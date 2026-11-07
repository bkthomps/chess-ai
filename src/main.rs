//! Small loopback-only HTTP server; all chess decisions remain in Rust.
use chess_ai::{
    chess::{Move, Position},
    clock::Clock,
    engine::{search_with_cancel, time_budget, SearchResult},
    game::Game,
};
use std::{
    collections::HashMap,
    io::{Read, Write},
    net::{TcpListener, TcpStream},
    sync::{
        atomic::{AtomicBool, Ordering},
        Arc, Mutex,
    },
    thread,
    time::{Duration, Instant},
};

struct App {
    game: Game,
    human: Option<bool>,
    unlimited: bool,
    fixed_thinking: Duration,
    clock: Clock,
    clock_history: Vec<[Duration; 2]>,
    cancel: Arc<AtomicBool>,
    search_budget: Option<Duration>,
    revision: u64,
    thinking: bool,
    stats: Option<SearchResult>,
    adjudicated: Option<String>,
}
impl Default for App {
    fn default() -> Self {
        Self {
            game: Game::default(),
            human: Some(true),
            unlimited: false,
            fixed_thinking: Duration::from_secs(3),
            clock: Clock::new(Duration::from_secs(300), Duration::ZERO, Instant::now()),
            clock_history: vec![[Duration::from_secs(300); 2]],
            cancel: Arc::new(AtomicBool::new(false)),
            search_budget: None,
            revision: 0,
            thinking: false,
            stats: None,
            adjudicated: None,
        }
    }
}
impl App {
    fn cancel_search(&mut self) {
        self.cancel.store(true, Ordering::Relaxed);
        self.thinking = false;
    }
    fn tick(&mut self, now: Instant) {
        if self.unlimited {
            return;
        }
        if let Some(flagged) = self.clock.update(now) {
            if self.adjudicated.is_none() && self.game.result().is_none() {
                self.adjudicated = Some(chess_ai::game::timeout_result(
                    self.game.position(),
                    flagged,
                ));
                self.revision += 1;
                self.cancel_search();
            }
        }
    }
    fn apply_move(&mut self, m: Move, now: Instant) -> Result<(), String> {
        self.tick(now);
        if self.adjudicated.is_some() {
            return Err("The game has ended.".into());
        }
        self.game.play(m)?;
        // tick() already ruled out a flag at this timestamp.
        if !self.unlimited {
            self.clock
                .complete_move(now)
                .expect("clock checked before the move");
        }
        if self.game.result().is_some() {
            self.clock.stop(now);
        }
        self.clock_history.push(self.clock.remaining);
        self.revision += 1;
        Ok(())
    }
}

type Shared = Arc<Mutex<App>>;

fn main() -> std::io::Result<()> {
    let mut port = 7878u16;
    let mut args = std::env::args().skip(1);
    while let Some(arg) = args.next() {
        match arg.as_str() {
            "--port" => {
                port = args.next().and_then(|p| p.parse().ok()).unwrap_or_else(|| {
                    eprintln!("--port needs a number from 0 to 65535");
                    std::process::exit(2)
                });
            }
            "--help" | "-h" => {
                println!("Chess AI\n\nUsage: cargo run --release -- [--port 7878]\n\nOpen the printed address in your browser. No internet or dependencies required.\nOne local game is shared by tabs; Ctrl+C stops the server.");
                return Ok(());
            }
            _ => {
                eprintln!("Unknown option: {arg}. Use --help.");
                std::process::exit(2);
            }
        }
    }
    let listener = TcpListener::bind(("127.0.0.1", port))?;
    println!(
        "\n  Chess AI is ready.\n  Open http://{} in your browser.\n  Press Ctrl+C to quit.\n",
        listener.local_addr()?
    );
    let app = Arc::new(Mutex::new(App::default()));
    // Flags are adjudicated even when no browser is polling.
    let timer_app = app.clone();
    thread::spawn(move || loop {
        thread::sleep(Duration::from_millis(20));
        timer_app.lock().unwrap().tick(Instant::now());
    });
    for stream in listener.incoming() {
        match stream {
            Ok(stream) => {
                let app = app.clone();
                thread::spawn(move || {
                    let _ = serve(stream, app);
                });
            }
            Err(e) => eprintln!("Connection error: {e}"),
        }
    }
    Ok(())
}

fn start_engine(app: &Shared) {
    let mut a = app.lock().unwrap();
    let started = Instant::now();
    a.tick(started);
    if a.thinking
        || a.adjudicated.is_some()
        || a.human.is_none()
        || a.human == Some(a.game.position().white)
        || a.game.result().is_some()
    {
        return;
    }
    let position = a.game.position().clone();
    let budget = if a.unlimited {
        a.fixed_thinking
    } else {
        time_budget(
            &position,
            a.clock.remaining[usize::from(!position.white)],
            a.clock.increment,
        )
    };
    a.cancel = Arc::new(AtomicBool::new(false));
    let cancel = a.cancel.clone();
    a.thinking = true;
    let (keys, revision) = (a.game.keys.clone(), a.revision);
    let shared = app.clone();
    thread::spawn(move || {
        let available = budget.saturating_sub(started.elapsed());
        let result = search_with_cancel(&position, &keys, available, cancel);
        let mut a = shared.lock().unwrap();
        let now = Instant::now();
        a.tick(now);
        if a.revision == revision && a.thinking {
            if let Some(m) = result.best {
                let _ = a.apply_move(m, now);
            }
            a.search_budget = Some(budget);
            a.stats = Some(result);
            a.thinking = false;
        }
    });
}

fn serve(mut stream: TcpStream, app: Shared) -> std::io::Result<()> {
    stream.set_read_timeout(Some(Duration::from_secs(3)))?;
    stream.set_write_timeout(Some(Duration::from_secs(3)))?;
    let port = stream.local_addr()?.port();
    serve_connection(&mut stream, app, port)
}

// The transport is generic so the exact HTTP handler can also be exercised
// without opening a socket.
fn serve_connection(
    mut stream: &mut (impl Read + Write),
    app: Shared,
    port: u16,
) -> std::io::Result<()> {
    let mut bytes = Vec::new();
    let mut buf = [0; 1024];
    while !bytes.windows(4).any(|w| w == b"\r\n\r\n") {
        let n = stream.read(&mut buf)?;
        if n == 0 {
            return Ok(());
        }
        bytes.extend_from_slice(&buf[..n]);
        if bytes.len() > 8192 {
            return respond(
                &mut stream,
                400,
                "application/json",
                b"{\"error\":\"Request too large\"}",
            );
        }
    }
    let text = match std::str::from_utf8(&bytes) {
        Ok(t) => t,
        Err(_) => {
            return respond(
                &mut stream,
                400,
                "application/json",
                b"{\"error\":\"Invalid request\"}",
            )
        }
    };
    let mut lines = text.split("\r\n");
    let request: Vec<_> = lines.next().unwrap_or("").split_whitespace().collect();
    if request.len() != 3 {
        return respond(
            &mut stream,
            400,
            "application/json",
            b"{\"error\":\"Invalid request\"}",
        );
    }
    let (method, target) = (request[0], request[1]);
    let headers: HashMap<_, _> = lines
        .filter_map(|l| l.split_once(':'))
        .map(|(k, v)| (k.to_ascii_lowercase(), v.trim()))
        .collect();
    // Reject DNS rebinding and cross-origin mutations. The browser sends a
    // dedicated header on API writes; no CORS permission is ever granted.
    let valid_hosts = [format!("127.0.0.1:{port}"), format!("localhost:{port}")];
    if !headers
        .get("host")
        .is_some_and(|h| valid_hosts.iter().any(|v| v == h))
    {
        return respond(
            &mut stream,
            403,
            "application/json",
            b"{\"error\":\"Use the local address printed in the terminal\"}",
        );
    }
    if method == "POST"
        && (headers.get("x-chess-request") != Some(&"1")
            || headers.get("origin").is_some_and(|origin| {
                !valid_hosts.iter().any(|h| format!("http://{h}") == *origin)
            }))
    {
        return respond(
            &mut stream,
            403,
            "application/json",
            b"{\"error\":\"Invalid request origin\"}",
        );
    }
    let (path, query) = target.split_once('?').unwrap_or((target, ""));
    let params: HashMap<_, _> = query.split('&').filter_map(|p| p.split_once('=')).collect();
    if method == "GET" {
        let resource: Option<(&str, &[u8])> = match path {
            "/" => Some((
                "text/html; charset=utf-8",
                include_bytes!("../web/index.html"),
            )),
            "/style.css" => Some((
                "text/css; charset=utf-8",
                include_bytes!("../web/style.css"),
            )),
            "/app.js" => Some((
                "text/javascript; charset=utf-8",
                include_bytes!("../web/app.js"),
            )),
            "/favicon.svg" => Some(("image/svg+xml", include_bytes!("../web/favicon.svg"))),
            _ => None,
        };
        if let Some((kind, body)) = resource {
            return respond(&mut stream, 200, kind, body);
        }
    }
    let answer: Result<String, String> = match (method, path) {
        ("GET", "/api/state") => Ok(state_json(&mut app.lock().unwrap())),
        ("POST", "/api/new") => (|| {
            let mode = params.get("mode").copied().unwrap_or("");
            let color = params.get("color").copied().unwrap_or("");
            let unlimited = match params.get("unlimited").copied() {
                Some("true") => true,
                Some("false") | None => false,
                _ => return Err("Choose a valid time control.".into()),
            };
            let (base, increment) = if unlimited {
                (0, 0)
            } else {
                (
                    number(&params, "base_seconds")?,
                    number(&params, "increment_seconds")?,
                )
            };
            let fixed_thinking = if unlimited && mode == "one" {
                let seconds = params
                    .get("thinking_seconds")
                    .and_then(|s| s.parse::<f64>().ok())
                    .ok_or("Enter a positive thinking time in seconds.")?;
                let duration = Duration::try_from_secs_f64(seconds)
                    .map_err(|_| "Enter a positive, representable thinking time in seconds.")?;
                if duration.is_zero() || Instant::now().checked_add(duration).is_none() {
                    return Err("Enter a positive, representable thinking time in seconds.".into());
                }
                duration
            } else {
                Duration::from_secs(3)
            };
            if !["one", "two"].contains(&mode)
                || !["white", "black"].contains(&color)
                || !unlimited && !(1..=10_800).contains(&base)
                || increment > 600
            {
                return Err("Choose a valid mode, color, base time (up to 180 minutes), and increment (0–600 seconds).".into());
            }
            {
                let mut a = app.lock().unwrap();
                a.cancel_search();
                a.game = Game::default();
                a.unlimited = unlimited;
                a.fixed_thinking = fixed_thinking;
                a.human = if mode == "two" {
                    None
                } else {
                    Some(color == "white")
                };
                let now = Instant::now();
                a.clock = Clock::new(
                    Duration::from_secs(base),
                    Duration::from_secs(increment),
                    now,
                );
                if !unlimited {
                    a.clock.start(true, now);
                }
                a.clock_history = vec![a.clock.remaining];
                a.search_budget = None;
                a.revision += 1;
                a.thinking = false;
                a.stats = None;
                a.adjudicated = None;
            }
            start_engine(&app);
            Ok(state_json(&mut app.lock().unwrap()))
        })(),
        ("POST", "/api/move") => (|| {
            let from = number(&params, "from")?;
            let to = number(&params, "to")?;
            let promotion = number(&params, "promotion")?;
            let revision = number(&params, "revision")?;
            if from >= 64 || to >= 64 || ![0, 2, 3, 4, 5].contains(&promotion) {
                return Err("Invalid move coordinates.".into());
            }
            {
                let mut a = app.lock().unwrap();
                a.tick(Instant::now());
                if a.adjudicated.is_some() {
                    return Err("The game has ended.".into());
                }
                if a.revision != revision {
                    return Err("The board changed. Try your move again.".into());
                }
                if a.thinking || a.human.is_some_and(|c| c != a.game.position().white) {
                    return Err("The computer is thinking.".into());
                }
                a.apply_move(
                    Move {
                        from: from as usize,
                        to: to as usize,
                        promotion: promotion as i8,
                    },
                    Instant::now(),
                )?;
            }
            start_engine(&app);
            Ok(state_json(&mut app.lock().unwrap()))
        })(),
        ("POST", "/api/undo") => {
            {
                let mut a = app.lock().unwrap();
                a.tick(Instant::now());
                if a.game.undo() {
                    a.clock_history.pop();
                    while a.human.is_some_and(|c| c != a.game.position().white)
                        && !a.game.moves.is_empty()
                    {
                        a.game.undo();
                        a.clock_history.pop();
                    }
                    a.revision += 1;
                    a.cancel_search();
                    let snapshot = *a.clock_history.last().unwrap();
                    let white = a.game.position().white;
                    if !a.unlimited {
                        a.clock.restore(snapshot, white, Instant::now());
                    }
                    a.search_budget = None;
                    a.stats = None;
                    a.adjudicated = None;
                }
            }
            start_engine(&app);
            Ok(state_json(&mut app.lock().unwrap()))
        }
        ("POST", "/api/resign") => {
            let mut a = app.lock().unwrap();
            a.tick(Instant::now());
            if a.adjudicated.is_none() && a.game.result().is_none() {
                let loser = a.human.unwrap_or(a.game.position().white);
                a.adjudicated = Some(format!(
                    "{} wins by resignation",
                    if loser { "Black" } else { "White" }
                ));
                a.revision += 1;
                a.cancel_search();
                a.clock.stop(Instant::now());
            }
            Ok(state_json(&mut a))
        }
        _ => {
            return respond(
                &mut stream,
                404,
                "application/json",
                b"{\"error\":\"Not found\"}",
            )
        }
    };
    match answer {
        Ok(body) => respond(&mut stream, 200, "application/json", body.as_bytes()),
        Err(e) => respond(
            &mut stream,
            400,
            "application/json",
            format!("{{\"error\":{}}}", quote(&e)).as_bytes(),
        ),
    }
}
fn number(params: &HashMap<&str, &str>, key: &str) -> Result<u64, String> {
    params
        .get(key)
        .and_then(|s| s.parse().ok())
        .ok_or_else(|| format!("Missing or invalid {key}."))
}
fn quote(s: &str) -> String {
    let mut out = String::from("\"");
    for c in s.chars() {
        match c {
            '"' => out.push_str("\\\""),
            '\\' => out.push_str("\\\\"),
            '\n' => out.push_str("\\n"),
            '\r' => out.push_str("\\r"),
            c if c.is_control() => out.push_str(&format!("\\u{:04x}", c as u32)),
            c => out.push(c),
        }
    }
    out.push('"');
    out
}
fn state_json(a: &mut App) -> String {
    a.tick(Instant::now());
    let p: &Position = a.game.position();
    let legal = p.legal_moves();
    let result = a.adjudicated.clone().or_else(|| {
        chess_ai::game::outcome(p, &a.game.keys, &legal).map(|r| r.label().to_string())
    });
    let board = p
        .squares
        .iter()
        .map(i8::to_string)
        .collect::<Vec<_>>()
        .join(",");
    let moves = if result.is_none() {
        legal
            .iter()
            .map(|m| format!("[{},{},{}]", m.from, m.to, m.promotion))
            .collect::<Vec<_>>()
            .join(",")
    } else {
        String::new()
    };
    let notation = a
        .game
        .notation
        .iter()
        .map(|s| quote(s))
        .collect::<Vec<_>>()
        .join(",");
    let last = a
        .game
        .moves
        .last()
        .map_or("null".into(), |m| format!("[{},{}]", m.from, m.to));
    let stats = a.stats.as_ref().map_or("null".into(), |s| {
        format!(
            "{{\"depth\":{},\"nodes\":{},\"hits\":{},\"millis\":{},\"budget_millis\":{}}}",
            s.depth,
            s.nodes,
            s.hits,
            s.elapsed.as_millis(),
            a.search_budget.unwrap_or_default().as_millis()
        )
    });
    let clocks = format!("{{\"enabled\":{},\"white_ms\":{},\"black_ms\":{},\"running\":{},\"active_white\":{},\"base_seconds\":{},\"increment_seconds\":{}}}",
        !a.unlimited, a.clock.remaining[0].as_millis(), a.clock.remaining[1].as_millis(), a.clock.active.is_some(),
        a.clock.active.map_or("null", |c| if c { "true" } else { "false" }), a.clock.base.as_secs(), a.clock.increment.as_secs());
    format!("{{\"board\":[{board}],\"white\":{},\"human\":{},\"clocks\":{clocks},\"thinking_seconds\":{},\"revision\":{},\"thinking\":{},\"check\":{},\"result\":{},\"legal\":[{moves}],\"notation\":[{notation}],\"last\":{last},\"stats\":{stats}}}",
        p.white, a.human.map_or("null", |c| if c { "true" } else { "false" }), a.fixed_thinking.as_secs_f64(), a.revision, a.thinking, p.in_check(p.white), result.map_or("null".into(), |r| quote(&r)))
}
fn respond(stream: &mut impl Write, status: u16, kind: &str, body: &[u8]) -> std::io::Result<()> {
    let reason = match status {
        200 => "OK",
        400 => "Bad Request",
        403 => "Forbidden",
        _ => "Not Found",
    };
    write!(stream, "HTTP/1.1 {status} {reason}\r\nContent-Type: {kind}\r\nContent-Length: {}\r\nConnection: close\r\nCache-Control: no-store\r\nX-Content-Type-Options: nosniff\r\nContent-Security-Policy: default-src 'self'; img-src 'self' data:; style-src 'self'; script-src 'self'; frame-ancestors 'none'; base-uri 'none'\r\n\r\n", body.len())?;
    stream.write_all(body)
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Cursor;
    use std::time::Instant;

    struct Connection {
        input: Cursor<Vec<u8>>,
        output: Vec<u8>,
    }
    impl Read for Connection {
        fn read(&mut self, buf: &mut [u8]) -> std::io::Result<usize> {
            self.input.read(buf)
        }
    }
    impl Write for Connection {
        fn write(&mut self, buf: &[u8]) -> std::io::Result<usize> {
            self.output.write(buf)
        }
        fn flush(&mut self) -> std::io::Result<()> {
            Ok(())
        }
    }
    fn shared() -> Shared {
        Arc::new(Mutex::new(App::default()))
    }
    fn raw(app: &Shared, request: &str) -> (u16, String) {
        let mut conn = Connection {
            input: Cursor::new(request.as_bytes().to_vec()),
            output: vec![],
        };
        serve_connection(&mut conn, app.clone(), 7878).unwrap();
        let response = String::from_utf8(conn.output).unwrap();
        let (head, body) = response.split_once("\r\n\r\n").unwrap();
        let status = head.split_whitespace().nth(1).unwrap().parse().unwrap();
        let length: usize = head
            .lines()
            .find_map(|l| l.strip_prefix("Content-Length: "))
            .unwrap()
            .parse()
            .unwrap();
        assert_eq!(body.len(), length);
        (status, body.to_string())
    }
    fn request(app: &Shared, method: &str, target: &str, expected: u16) -> String {
        let (status, body) = raw(app, &format!("{method} {target} HTTP/1.1\r\nHost: 127.0.0.1:7878\r\nOrigin: http://127.0.0.1:7878\r\nX-Chess-Request: 1\r\n\r\n"));
        assert_eq!(status, expected, "{target}: {body}");
        body
    }
    fn new_game(app: &Shared, mode: &str, color: &str) {
        request(
            app,
            "POST",
            &format!("/api/new?mode={mode}&color={color}&base_seconds=6&increment_seconds=0"),
            200,
        );
    }
    fn play(app: &Shared, uci: &str) -> String {
        let a = app.lock().unwrap();
        let m = a
            .game
            .position()
            .legal_moves()
            .into_iter()
            .find(|m| m.to_string() == uci)
            .unwrap();
        let revision = a.revision;
        drop(a);
        request(
            app,
            "POST",
            &format!(
                "/api/move?from={}&to={}&promotion={}&revision={revision}",
                m.from, m.to, m.promotion
            ),
            200,
        )
    }
    fn wait_for_engine(app: &Shared) {
        let deadline = Instant::now() + Duration::from_secs(5);
        while app.lock().unwrap().thinking {
            assert!(Instant::now() < deadline, "Engine did not finish");
            thread::sleep(Duration::from_millis(5));
        }
    }
    #[test]
    fn bundled_assets_and_initial_state() {
        let app = shared();
        for target in ["/", "/style.css", "/app.js", "/favicon.svg"] {
            assert!(!request(&app, "GET", target, 200).is_empty());
        }
        let body = request(&app, "GET", "/api/state", 200);
        assert!(body.contains("\"white\":true"));
        assert!(body.contains("\"notation\":[]"));
        request(&app, "GET", "/missing", 404);
    }
    #[test]
    fn local_play_validation_and_undo() {
        let app = shared();
        new_game(&app, "two", "white");
        let revision = app.lock().unwrap().revision;
        let body = play(&app, "e2e4");
        assert!(body.contains("\"notation\":[\"e4\"]"));
        request(
            &app,
            "POST",
            &format!("/api/move?from=11&to=27&promotion=0&revision={revision}"),
            400,
        );
        let revision = app.lock().unwrap().revision;
        for (from, to, promotion) in [(52, 28, 0), (64, 28, 0), (52, 44, 6)] {
            request(
                &app,
                "POST",
                &format!("/api/move?from={from}&to={to}&promotion={promotion}&revision={revision}"),
                400,
            );
        }
        request(&app, "POST", "/api/undo", 200);
        assert!(app.lock().unwrap().game.moves.is_empty());
    }
    #[test]
    fn mate_and_resignation_stop_moves() {
        let app = shared();
        new_game(&app, "two", "white");
        for uci in ["f2f3", "e7e5", "g2g4", "d8h4"] {
            play(&app, uci);
        }
        let body = request(&app, "GET", "/api/state", 200);
        assert!(body.contains("Black wins by checkmate"));
        assert!(body.contains("\"legal\":[]"));
        request(&app, "POST", "/api/undo", 200);
        assert!(app.lock().unwrap().game.result().is_none());
        assert!(request(&app, "POST", "/api/resign", 200).contains("White wins by resignation"));
        let rev = app.lock().unwrap().revision;
        request(
            &app,
            "POST",
            &format!("/api/move?from=51&to=43&promotion=0&revision={rev}"),
            400,
        );
    }
    #[test]
    fn solo_white_and_black_and_takeback() {
        let app = shared();
        new_game(&app, "one", "white");
        play(&app, "e2e4");
        wait_for_engine(&app);
        {
            let a = app.lock().unwrap();
            assert_eq!(a.game.moves.len(), 2);
            assert!(a.game.position().white);
            assert!(a.stats.as_ref().unwrap().nodes > 0);
        }
        request(&app, "POST", "/api/undo", 200);
        assert!(app.lock().unwrap().game.moves.is_empty());
        new_game(&app, "one", "black");
        wait_for_engine(&app);
        {
            let a = app.lock().unwrap();
            assert_eq!(a.game.moves.len(), 1);
            assert!(!a.game.position().white);
        }
    }
    #[test]
    fn promotion_is_validated_and_notated() {
        let app = shared();
        {
            let mut a = app.lock().unwrap();
            a.human = None;
            a.game = Game::new(Position::from_fen("4k3/P7/8/8/8/8/8/4K3 w - - 0 1").unwrap());
        }
        request(
            &app,
            "POST",
            "/api/move?from=48&to=56&promotion=0&revision=0",
            400,
        );
        play(&app, "a7a8n");
        let a = app.lock().unwrap();
        assert_eq!(a.game.position().squares[56], 2);
        assert_eq!(a.game.notation[0], "a8=N");
    }
    #[test]
    fn untrusted_and_malformed_requests() {
        let app = shared();
        for request in [
            "GET /api/state HTTP/1.1\r\nHost: example.com:7878\r\n\r\n",
            "POST /api/undo HTTP/1.1\r\nHost: 127.0.0.1:7878\r\n\r\n",
            "POST /api/undo HTTP/1.1\r\nHost: 127.0.0.1:7878\r\nX-Chess-Request: 1\r\nOrigin: http://example.com\r\n\r\n",
        ] { assert_eq!(raw(&app, request).0, 403); }
        assert_eq!(raw(&app, "GET\r\n\r\n").0, 400);
        request(
            &app,
            "POST",
            "/api/new?mode=one&color=white&base_seconds=0&increment_seconds=0",
            400,
        );
        request(&app, "POST", "/api/move?from=nope", 400);
    }
    #[test]
    fn undo_while_thinking_cancels_the_pending_move() {
        let app = shared();
        new_game(&app, "one", "white");
        play(&app, "e2e4");
        request(&app, "POST", "/api/undo", 200);
        thread::sleep(Duration::from_millis(300));
        let a = app.lock().unwrap();
        assert!(a.game.moves.is_empty());
        assert!(!a.thinking);
    }
    #[test]
    fn new_game_discards_an_old_engine_result() {
        let app = shared();
        new_game(&app, "one", "black");
        new_game(&app, "two", "white");
        thread::sleep(Duration::from_millis(300));
        let a = app.lock().unwrap();
        assert_eq!(a.human, None);
        assert!(!a.thinking);
        assert!(a.game.moves.is_empty());
    }
    #[test]
    fn clock_controls_are_validated_for_both_modes() {
        let app = shared();
        for mode in ["one", "two"] {
            for (base, increment) in [(60, 0), (120, 1), (300, 0), (600, 5), (90, 3)] {
                let body = request(&app, "POST", &format!("/api/new?mode={mode}&color=white&base_seconds={base}&increment_seconds={increment}"), 200);
                assert!(body.contains(&format!("\"base_seconds\":{base}")));
                assert!(body.contains(&format!("\"increment_seconds\":{increment}")));
                let a = app.lock().unwrap();
                assert_eq!(a.clock.active, Some(true));
                assert_eq!(a.clock.base, Duration::from_secs(base));
                assert_eq!(a.clock.increment, Duration::from_secs(increment));
            }
        }
        for query in [
            "base_seconds=0&increment_seconds=0",
            "base_seconds=10801&increment_seconds=0",
            "base_seconds=60&increment_seconds=601",
            "base_seconds=60&increment_seconds=-1",
            "base_seconds=60",
        ] {
            request(
                &app,
                "POST",
                &format!("/api/new?mode=two&color=white&{query}"),
                400,
            );
        }
    }
    #[test]
    fn legal_move_awards_increment_and_undo_restores_both_clocks() {
        let app = shared();
        request(
            &app,
            "POST",
            "/api/new?mode=two&color=white&base_seconds=120&increment_seconds=1",
            200,
        );
        {
            let mut a = app.lock().unwrap();
            a.clock.start(true, Instant::now() - Duration::from_secs(2));
        }
        play(&app, "e2e4");
        {
            let a = app.lock().unwrap();
            assert_eq!(a.clock.active, Some(false));
            assert!(a.clock.remaining[0] <= Duration::from_secs(119));
            assert!(a.clock.remaining[0] > Duration::from_millis(118_900));
        }
        request(&app, "POST", "/api/undo", 200);
        let a = app.lock().unwrap();
        assert_eq!(a.clock.active, Some(true));
        assert!(a.clock.remaining[0] > Duration::from_millis(119_900));
        assert_eq!(a.clock.remaining[1], Duration::from_secs(120));
    }
    #[test]
    fn expired_clock_ends_game_and_late_move_does_not_get_increment() {
        let app = shared();
        request(
            &app,
            "POST",
            "/api/new?mode=two&color=white&base_seconds=60&increment_seconds=5",
            200,
        );
        let revision;
        {
            let mut a = app.lock().unwrap();
            revision = a.revision;
            a.clock
                .start(true, Instant::now() - Duration::from_secs(61));
        }
        request(
            &app,
            "POST",
            &format!("/api/move?from=12&to=28&promotion=0&revision={revision}"),
            400,
        );
        let body = request(&app, "GET", "/api/state", 200);
        assert!(body.contains("Black wins on time"));
        assert!(body.contains("\"legal\":[]"));
        let a = app.lock().unwrap();
        assert_eq!(a.clock.remaining[0], Duration::ZERO);
        assert_eq!(a.clock.active, None);
        assert!(a.game.moves.is_empty());
    }
    #[test]
    fn flag_against_bare_king_draws_and_stops_the_clocks() {
        let app = shared();
        {
            let mut a = app.lock().unwrap();
            a.game = Game::new(Position::from_fen("4k3/8/8/8/8/8/8/3QK3 w - - 0 1").unwrap());
            a.clock
                .start(true, Instant::now() - Duration::from_secs(301));
            a.tick(Instant::now());
            assert!(a.adjudicated.as_ref().unwrap().starts_with("Draw on time"));
            let times = a.clock.remaining;
            a.tick(Instant::now() + Duration::from_secs(100));
            assert_eq!(a.clock.remaining, times);
        }
        assert!(request(&app, "GET", "/api/state", 200).contains("Draw on time"));
    }
    #[test]
    fn checkmate_takes_precedence_and_freezes_the_clock() {
        let app = shared();
        new_game(&app, "two", "white");
        for uci in ["f2f3", "e7e5", "g2g4", "d8h4"] {
            play(&app, uci);
        }
        let mut a = app.lock().unwrap();
        let times = a.clock.remaining;
        a.tick(Instant::now() + Duration::from_secs(1000));
        assert_eq!(a.clock.active, None);
        assert_eq!(a.clock.remaining, times);
        assert!(a.adjudicated.is_none());
        assert!(a.game.result().unwrap().label().contains("checkmate"));
    }
    #[test]
    fn timeout_cancels_pending_engine_result() {
        let app = shared();
        new_game(&app, "one", "black");
        {
            let mut a = app.lock().unwrap();
            a.clock.remaining[0] = Duration::ZERO;
            a.tick(Instant::now());
            assert!(!a.thinking);
            assert!(a.cancel.load(Ordering::Relaxed));
        }
        thread::sleep(Duration::from_millis(300));
        let a = app.lock().unwrap();
        assert!(a.game.moves.is_empty());
        assert_eq!(a.adjudicated.as_deref(), Some("Black wins on time"));
    }
    #[test]
    fn unlimited_game_never_flags_including_after_undo() {
        let app = shared();
        let body = request(
            &app,
            "POST",
            "/api/new?mode=two&color=white&unlimited=true",
            200,
        );
        assert!(body.contains("\"enabled\":false"));
        assert!(body.contains("\"running\":false"));
        play(&app, "e2e4");
        request(&app, "POST", "/api/undo", 200);
        let mut a = app.lock().unwrap();
        a.tick(Instant::now() + Duration::from_secs(1_000_000));
        assert!(a.unlimited);
        assert_eq!(a.clock.active, None);
        assert!(a.adjudicated.is_none());
        assert!(a.game.moves.is_empty());
    }
    #[test]
    fn unlimited_thinking_time_accepts_presets_and_custom_values() {
        let app = shared();
        for seconds in [1.0, 2.0, 3.0, 4.0, 5.0, 0.125, 12.5, 1000.0] {
            let body = request(
                &app,
                "POST",
                &format!("/api/new?mode=one&color=white&unlimited=true&thinking_seconds={seconds}"),
                200,
            );
            assert!(body.contains(&format!("\"thinking_seconds\":{seconds}")));
            let a = app.lock().unwrap();
            assert_eq!(a.fixed_thinking, Duration::from_secs_f64(seconds));
            assert_eq!(a.clock.active, None);
        }
        for seconds in ["0", "-1", "NaN", "inf", "1e100", "nope", ""] {
            request(
                &app,
                "POST",
                &format!("/api/new?mode=one&color=white&unlimited=true&thinking_seconds={seconds}"),
                400,
            );
        }
        request(
            &app,
            "POST",
            "/api/new?mode=one&color=white&unlimited=true",
            400,
        );
        request(
            &app,
            "POST",
            "/api/new?mode=two&color=white&unlimited=maybe",
            400,
        );
    }
    #[test]
    fn unlimited_computer_uses_the_selected_budget_and_undo_stays_untimed() {
        let app = shared();
        request(
            &app,
            "POST",
            "/api/new?mode=one&color=black&unlimited=true&thinking_seconds=0.05",
            200,
        );
        wait_for_engine(&app);
        {
            let a = app.lock().unwrap();
            assert_eq!(a.search_budget, Some(Duration::from_millis(50)));
            assert!(a.stats.as_ref().unwrap().nodes > 0);
            assert_eq!(a.game.moves.len(), 1);
            assert_eq!(a.clock.active, None);
        }
        // Taking back Black's first decision replays the computer's opening.
        request(&app, "POST", "/api/undo", 200);
        wait_for_engine(&app);
        let a = app.lock().unwrap();
        assert_eq!(a.clock.active, None);
        assert!(a.adjudicated.is_none());
    }
    #[test]
    fn switching_between_unlimited_and_timed_games_restores_the_right_policy() {
        let app = shared();
        request(
            &app,
            "POST",
            "/api/new?mode=two&color=white&unlimited=true",
            200,
        );
        new_game(&app, "two", "white");
        {
            let a = app.lock().unwrap();
            assert!(!a.unlimited);
            assert_eq!(a.clock.active, Some(true));
        }
        request(
            &app,
            "POST",
            "/api/new?mode=two&color=white&unlimited=true",
            200,
        );
        assert_eq!(app.lock().unwrap().clock.active, None);
    }
}
