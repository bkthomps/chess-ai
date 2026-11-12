//! Small local and LAN HTTP server; all chess decisions remain in Rust.
use chess_ai::{
    chess::{Move, Position},
    clock::Clock,
    engine::{time_budget, SearchCache, SearchProgress, SearchResult},
    game::Game,
};
use std::{
    collections::{hash_map::RandomState, HashMap},
    hash::{BuildHasher, Hasher},
    io::{Read, Write},
    net::{Ipv4Addr, TcpListener, TcpStream, UdpSocket},
    sync::{
        atomic::{AtomicBool, Ordering},
        mpsc::{self, Sender},
        Arc, Mutex,
    },
    thread,
    time::{Duration, Instant, SystemTime, UNIX_EPOCH},
};

struct Lan {
    invite: String,
    host: String,
    guest: String,
    joined: bool,
    host_left: bool,
    host_white: bool,
    address: Option<Ipv4Addr>,
}
impl Lan {
    fn player(&self, token: &str) -> Option<bool> {
        if token == self.host {
            Some(self.host_white)
        } else if !self.host_left && self.joined && token == self.guest {
            Some(!self.host_white)
        } else {
            None
        }
    }
}
fn seat_token() -> String {
    // Use randomly keyed standard-library hashers for opaque seat identifiers.
    let mut token = String::new();
    for _ in 0..2 {
        token.push_str(&format!(
            "{:016x}",
            RandomState::new().build_hasher().finish()
        ));
    }
    token
}
fn invitation_token() -> String {
    let words = include_str!("invite_words.txt").split_ascii_whitespace();
    let count = words.clone().count();
    let mut token = String::new();
    for i in 0..4 {
        if i > 0 {
            token.push('.');
        }
        let index = (RandomState::new().build_hasher().finish() % count as u64) as usize;
        token.push_str(words.clone().nth(index).unwrap());
    }
    token
}
fn lan_address() -> Option<Ipv4Addr> {
    let socket = UdpSocket::bind((Ipv4Addr::UNSPECIFIED, 0)).ok()?;
    // UDP connect selects an interface without transmitting any packets.
    socket.connect((Ipv4Addr::new(192, 0, 2, 1), 80)).ok()?;
    match socket.local_addr().ok()?.ip() {
        std::net::IpAddr::V4(ip) if ip.is_private() || ip.is_link_local() => Some(ip),
        _ => None,
    }
}
fn authorized(a: &App, path: &str, token: &str, local: bool) -> bool {
    match path {
        "/api/new" => local && a.lan.as_ref().map_or(true, |lan| lan.host == token),
        "/api/leave" => local && a.lan.as_ref().is_some_and(|lan| lan.host == token),
        "/api/undo" => local && a.lan.is_none(),
        "/api/move" | "/api/resign" => {
            if let Some(lan) = &a.lan {
                !lan.host_left
                    && lan.player(token).is_some()
                    && (path != "/api/move"
                        || lan.joined && lan.player(token) == Some(a.game.position().white))
            } else {
                local
            }
        }
        _ => true,
    }
}

fn viewer_json(a: &mut App, token: &str, port: u16) -> String {
    let mut json = state_json(a);
    json.pop();
    let player = a.lan.as_ref().and_then(|lan| lan.player(token));
    let waiting = a.lan.as_ref().is_some_and(|lan| !lan.joined);
    let link = a
        .lan
        .as_ref()
        .filter(|lan| lan.host == token)
        .and_then(|lan| {
            lan.address
                .map(|ip| quote(&format!("http://{ip}:{port}/join={}", lan.invite)))
        })
        .unwrap_or_else(|| "null".into());
    json.push_str(&format!(
        ",\"lan\":{},\"player\":{},\"is_host\":{},\"host_left\":{},\"waiting\":{waiting},\"join_url\":{link}}}",
        a.lan.is_some(),
        player.map_or("null", |white| if white { "true" } else { "false" }),
        a.lan.as_ref().is_some_and(|lan| lan.host == token),
        a.lan.as_ref().is_some_and(|lan| lan.host_left)
    ));
    json
}

struct App {
    game: Game,
    started_at: Option<u64>,
    human: Option<bool>,
    ai_vs_ai: bool,
    lan: Option<Lan>,
    unlimited: bool,
    fixed_thinking: Duration,
    clock: Clock,
    clock_history: Vec<[Duration; 2]>,
    cancel: Arc<AtomicBool>,
    search_budget: Option<Duration>,
    revision: u64,
    thinking: bool,
    pondering: bool,
    worker: Option<Sender<EngineTask>>,
    cache_epoch: u64,
    stats: Option<SearchResult>,
    analysis: Option<Analysis>,
    adjudicated: Option<String>,
}
impl Default for App {
    fn default() -> Self {
        Self {
            game: Game::default(),
            started_at: game_start_time(),
            human: Some(true),
            ai_vs_ai: false,
            lan: None,
            unlimited: false,
            fixed_thinking: Duration::from_secs(3),
            clock: Clock::new(Duration::from_secs(300), Duration::ZERO, Instant::now()),
            clock_history: vec![[Duration::from_secs(300); 2]],
            cancel: Arc::new(AtomicBool::new(false)),
            search_budget: None,
            revision: 0,
            thinking: false,
            pondering: false,
            worker: None,
            cache_epoch: 0,
            stats: None,
            analysis: None,
            adjudicated: None,
        }
    }
}
fn game_start_time() -> Option<u64> {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .ok()
        .map(|d| d.as_secs())
}
impl App {
    fn cancel_search(&mut self) {
        self.cancel.store(true, Ordering::Relaxed);
        self.thinking = false;
        self.pondering = false;
    }
    fn release_cache(&mut self) {
        self.cancel_search();
        if let Some(worker) = &self.worker {
            let _ = worker.send(EngineTask::Release);
        }
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
                self.release_cache();
            }
        }
    }
    fn apply_move(&mut self, m: Move, now: Instant) -> Result<(), String> {
        self.tick(now);
        if self.adjudicated.is_some() {
            return Err("The game has ended.".into());
        }
        self.game.play(m)?;
        self.cancel_search();
        // tick() already ruled out a flag at this timestamp.
        if !self.unlimited {
            self.clock
                .complete_move(now)
                .expect("clock checked before the move");
        }
        if self.game.result().is_some() {
            self.clock.stop(now);
            self.release_cache();
        }
        self.clock_history.push(self.clock.remaining);
        self.revision += 1;
        Ok(())
    }
}

impl Drop for App {
    fn drop(&mut self) {
        self.cancel_search();
    }
}

type Shared = Arc<Mutex<App>>;

enum EngineTask {
    Analyze(EngineJob),
    Release,
}

struct Analysis {
    progress: Arc<Mutex<SearchProgress>>,
    budget: Option<Duration>,
    pondering: bool,
    revision: u64,
}

struct EngineJob {
    position: Position,
    keys: Vec<u64>,
    // None means pondering, which uses the human's turn rather than a budget.
    budget: Option<Duration>,
    started: Instant,
    cancel: Arc<AtomicBool>,
    revision: u64,
    cache_epoch: u64,
    progress: Arc<Mutex<SearchProgress>>,
}

fn main() -> std::io::Result<()> {
    let mut port = 7878u16;
    let mut open_browser = false;
    let mut args = std::env::args().skip(1);
    while let Some(arg) = args.next() {
        match arg.as_str() {
            "--open" => open_browser = true,
            "--port" => {
                port = args.next().and_then(|p| p.parse().ok()).unwrap_or_else(|| {
                    eprintln!("--port needs a number from 0 to 65535");
                    std::process::exit(2)
                });
            }
            "--help" | "-h" => {
                println!("Chess AI\n\nUsage: cargo run --release -- [--port 7878] [--open]\n\n--open opens the local address in your default browser.\nOne game is shared by tabs; LAN mode accepts an invited opponent on your network; Ctrl+C stops the server.");
                return Ok(());
            }
            _ => {
                eprintln!("Unknown option: {arg}. Use --help.");
                std::process::exit(2);
            }
        }
    }
    let listener = TcpListener::bind(("0.0.0.0", port))?;
    if open_browser {
        let url = format!("http://127.0.0.1:{}", listener.local_addr()?.port());
        thread::spawn(move || {
            #[cfg(target_os = "windows")]
            let result = std::process::Command::new("cmd")
                .args(["/c", "start", "", &url])
                .status();
            #[cfg(target_os = "macos")]
            let result = std::process::Command::new("open").arg(&url).status();
            #[cfg(not(any(target_os = "windows", target_os = "macos")))]
            let result = std::process::Command::new("xdg-open").arg(&url).status();
            if !matches!(result, Ok(status) if status.success()) {
                eprintln!("Could not open the browser automatically. Open {url} manually.");
            }
        });
    }
    println!(
        "\n  Chess AI is ready.\n  Open http://127.0.0.1:{} in your browser.\n  Press Ctrl+C to quit.\n",
        listener.local_addr()?.port()
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
    queue_engine(app, &mut a);
}

fn queue_engine(app: &Shared, a: &mut App) {
    let started = Instant::now();
    a.tick(started);
    if a.thinking
        || a.pondering
        || a.adjudicated.is_some()
        || (a.human.is_none() && !a.ai_vs_ai)
        || a.game.result().is_some()
    {
        return;
    }
    let position = a.game.position().clone();
    let pondering = !a.ai_vs_ai && a.human == Some(position.white);
    let budget = if pondering {
        None
    } else if a.unlimited {
        Some(a.fixed_thinking)
    } else {
        Some(time_budget(
            &position,
            a.clock.remaining[usize::from(!position.white)],
            a.clock.increment,
        ))
    };
    a.cancel = Arc::new(AtomicBool::new(false));
    a.thinking = !pondering;
    a.pondering = pondering;
    let progress = Arc::new(Mutex::new(SearchProgress::default()));
    a.analysis = Some(Analysis {
        progress: progress.clone(),
        budget,
        pondering,
        revision: a.revision,
    });
    if a.worker.is_none() {
        let (sender, receiver) = mpsc::channel::<EngineTask>();
        // The worker must not keep an abandoned game (or a test) alive.
        let weak = Arc::downgrade(app);
        thread::spawn(move || {
            let mut cache = SearchCache::default();
            let mut epoch = None;
            while let Ok(mut task) = receiver.recv() {
                // Rapid new games/undos may enqueue several obsolete positions.
                for newer in receiver.try_iter() {
                    task = newer;
                }
                let job = match task {
                    EngineTask::Analyze(job) => job,
                    EngineTask::Release => {
                        cache = SearchCache::default();
                        epoch = None;
                        continue;
                    }
                };
                if job.cancel.load(Ordering::Relaxed) {
                    continue;
                }
                if epoch != Some(job.cache_epoch) {
                    cache = SearchCache::default();
                    epoch = Some(job.cache_epoch);
                }
                let result = match job.budget {
                    Some(budget) => cache.search_move_with_progress(
                        &job.position,
                        &job.keys,
                        budget.saturating_sub(job.started.elapsed()),
                        job.cancel.clone(),
                        &job.progress,
                    ),
                    None => cache.ponder_with_progress(
                        &job.position,
                        &job.keys,
                        job.cancel.clone(),
                        &job.progress,
                    ),
                };
                // Always leave a final snapshot, even if a reader coincided
                // with the engine's last nonblocking publication.
                *job.progress.lock().unwrap() = SearchProgress {
                    evaluation: (result.depth > 0).then_some(if job.position.white {
                        result.score
                    } else {
                        -result.score
                    }),
                    depth: result.depth,
                    searching_depth: 0,
                    nodes: result.nodes,
                    hits: result.hits,
                    elapsed: result.elapsed,
                    threads: result.threads,
                    cache_bytes: result.cache_bytes,
                    cache_limit: result.cache_limit,
                };
                let Some(shared) = weak.upgrade() else {
                    break;
                };
                {
                    let mut a = shared.lock().unwrap();
                    let now = Instant::now();
                    a.tick(now);
                    if a.revision != job.revision
                        || !Arc::ptr_eq(&a.cancel, &job.cancel)
                        || job.cancel.load(Ordering::Relaxed)
                    {
                        continue;
                    }
                    a.thinking = false;
                    a.pondering = false;
                    if job.budget.is_none() {
                        // A completed ponder only warms the cache, never plays.
                        continue;
                    }
                    if let Some(m) = result.best {
                        let _ = a.apply_move(m, now);
                    }
                    a.search_budget = job.budget;
                    a.stats = Some(result);
                    // Start the next turn (or solo pondering) on the same worker.
                    queue_engine(&shared, &mut a);
                }
            }
        });
        a.worker = Some(sender);
    }
    a.worker
        .as_ref()
        .unwrap()
        .send(EngineTask::Analyze(EngineJob {
            position,
            keys: a.game.keys.clone(),
            budget,
            started,
            cancel: a.cancel.clone(),
            revision: a.revision,
            cache_epoch: a.cache_epoch,
            progress,
        }))
        .unwrap();
}

fn serve(mut stream: TcpStream, app: Shared) -> std::io::Result<()> {
    stream.set_read_timeout(Some(Duration::from_secs(3)))?;
    stream.set_write_timeout(Some(Duration::from_secs(3)))?;
    let port = stream.local_addr()?.port();
    let local = stream.peer_addr()?.ip().is_loopback();
    serve_connection(&mut stream, app, port, local)
}

// The transport is generic so the exact HTTP handler can also be exercised
// without opening a socket.
fn serve_connection(
    mut stream: &mut (impl Read + Write),
    app: Shared,
    port: u16,
    local: bool,
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
    let host = headers.get("host").copied().unwrap_or("");
    let valid_host = host == format!("localhost:{port}")
        || host.rsplit_once(':').is_some_and(|(ip, p)| {
            p.parse::<u16>() == Ok(port)
                && ip
                    .parse::<Ipv4Addr>()
                    .is_ok_and(|ip| ip.is_loopback() || ip.is_private() || ip.is_link_local())
        });
    if !valid_host
        || method == "POST"
            && (headers.get("x-chess-request") != Some(&"1")
                || headers
                    .get("origin")
                    .is_some_and(|origin| **origin != format!("http://{host}")))
    {
        return respond(
            &mut stream,
            403,
            "application/json",
            b"{\"error\":\"Invalid request origin or address\"}",
        );
    }
    let (path, query) = target.split_once('?').unwrap_or((target, ""));
    let params: HashMap<_, _> = query.split('&').filter_map(|p| p.split_once('=')).collect();
    if method == "GET" {
        let resource: Option<(&str, &[u8])> = match path {
            p if p == "/" || p.starts_with("/join=") => Some((
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
    let mut token = params.get("player").copied().unwrap_or("").to_string();
    if method == "POST" {
        let a = app.lock().unwrap();
        let denied = !authorized(&a, path, &token, local);
        if denied {
            return respond(
                &mut stream,
                403,
                "application/json",
                b"{\"error\":\"This action is unavailable for this player.\"}",
            );
        }
    }
    let answer: Result<String, String> = match (method, path) {
        ("POST", "/api/join") => (|| {
            let mut a = app.lock().unwrap();
            if (a.adjudicated.is_some() || a.game.result().is_some())
                && a.lan.as_ref().and_then(|lan| lan.player(&token)).is_none()
            {
                return Err("This game has ended.".into());
            }
            let lan = a.lan.as_mut().ok_or("No LAN game is waiting.")?;
            if lan.host_left {
                return Err("This invitation is invalid or already used.".into());
            }
            if lan.player(&token).is_none() {
                if lan.joined || params.get("invite").copied() != Some(lan.invite.as_str()) {
                    return Err("This invitation is invalid or already used.".into());
                }
                lan.joined = true;
                token = lan.guest.clone();
                a.revision += 1;
                if !a.unlimited {
                    a.clock.start(true, Instant::now());
                }
            }
            let mut json = viewer_json(&mut a, &token, port);
            json.pop();
            Ok(format!("{json},\"player_token\":{}}}", quote(&token)))
        })(),
        ("GET", "/api/state") => {
            let mut a = app.lock().unwrap();
            let invite_required =
                !local && a.lan.as_ref().and_then(|lan| lan.player(&token)).is_none();
            let mut json = viewer_json(&mut a, &token, port);
            json.pop();
            Ok(format!("{json},\"invite_required\":{invite_required}}}"))
        }
        ("POST", "/api/leave") => {
            let mut a = app.lock().unwrap();
            if !authorized(&a, path, &token, local) {
                return respond(
                    &mut stream,
                    403,
                    "application/json",
                    b"{\"error\":\"This action is unavailable for this player.\"}",
                );
            }
            a.lan.as_mut().unwrap().host_left = true;
            a.clock.stop(Instant::now());
            a.revision += 1;
            Ok(viewer_json(&mut a, &token, port))
        }
        ("POST", "/api/new") => (|| {
            let mode = params.get("mode").copied().unwrap_or("");
            let color = params.get("color").copied().unwrap_or("");
            let two_player_mode = params.get("two_player_mode").copied().unwrap_or("same");
            if !["lan", "same", "ai"].contains(&two_player_mode)
                || mode != "two" && two_player_mode != "same"
            {
                return Err("Choose a valid two-player mode.".into());
            }
            let ai_vs_ai = mode == "two" && two_player_mode == "ai";
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
            let fixed_thinking = if unlimited && (mode == "one" || ai_vs_ai) {
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
                || !["random", "white", "black"].contains(&color)
                || !unlimited && !(1..=10_800).contains(&base)
                || increment > 600
            {
                return Err("Choose a valid mode, color, base time (up to 180 minutes), and increment (0–600 seconds).".into());
            }
            {
                let mut a = app.lock().unwrap();
                if !authorized(&a, path, &token, local) {
                    return Err("This action is unavailable for this player.".into());
                }
                let white = match color {
                    "random" => RandomState::new().build_hasher().finish() & 1 == 0,
                    "white" => true,
                    _ => false,
                };
                a.cancel_search();
                a.cache_epoch += 1;
                a.game = Game::default();
                a.started_at = game_start_time();
                a.unlimited = unlimited;
                a.fixed_thinking = fixed_thinking;
                a.ai_vs_ai = ai_vs_ai;
                a.lan = if mode == "two" && two_player_mode == "lan" {
                    token = seat_token();
                    Some(Lan {
                        invite: invitation_token(),
                        host: token.clone(),
                        guest: seat_token(),
                        joined: false,
                        host_left: false,
                        host_white: white,
                        address: lan_address(),
                    })
                } else {
                    None
                };
                a.human = if mode == "two" { None } else { Some(white) };
                if a.human.is_none() && !a.ai_vs_ai {
                    a.release_cache();
                }
                let now = Instant::now();
                a.clock = Clock::new(
                    Duration::from_secs(base),
                    Duration::from_secs(increment),
                    now,
                );
                if !unlimited && a.lan.is_none() {
                    a.clock.start(true, now);
                }
                a.clock_history = vec![a.clock.remaining];
                a.search_budget = None;
                a.revision += 1;
                a.thinking = false;
                a.stats = None;
                a.analysis = None;
                a.adjudicated = None;
            }
            start_engine(&app);
            let mut json = viewer_json(&mut app.lock().unwrap(), &token, port);
            json.pop();
            Ok(format!("{json},\"player_token\":{}}}", quote(&token)))
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
                if !authorized(&a, path, &token, local) {
                    return Err("This action is unavailable for this player.".into());
                }
                a.tick(Instant::now());
                if a.adjudicated.is_some() {
                    return Err("The game has ended.".into());
                }
                if a.revision != revision {
                    return Err("The board changed. Try your move again.".into());
                }
                if a.ai_vs_ai || a.thinking || a.human.is_some_and(|c| c != a.game.position().white)
                {
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
            Ok(viewer_json(&mut app.lock().unwrap(), &token, port))
        })(),
        ("POST", "/api/undo") => {
            {
                let mut a = app.lock().unwrap();
                if !authorized(&a, path, &token, local) {
                    return respond(
                        &mut stream,
                        403,
                        "application/json",
                        b"{\"error\":\"This action is unavailable for this player.\"}",
                    );
                }
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
                    a.cache_epoch += 1;
                    let snapshot = *a.clock_history.last().unwrap();
                    let white = a.game.position().white;
                    if !a.unlimited {
                        a.clock.restore(snapshot, white, Instant::now());
                    }
                    a.search_budget = None;
                    a.stats = None;
                    a.analysis = None;
                    a.adjudicated = None;
                }
            }
            start_engine(&app);
            Ok(viewer_json(&mut app.lock().unwrap(), &token, port))
        }
        ("POST", "/api/resign") => {
            let mut a = app.lock().unwrap();
            if !authorized(&a, path, &token, local) {
                return respond(
                    &mut stream,
                    403,
                    "application/json",
                    b"{\"error\":\"This action is unavailable for this player.\"}",
                );
            }
            a.tick(Instant::now());
            if a.ai_vs_ai {
                return respond(
                    &mut stream,
                    400,
                    "application/json",
                    b"{\"error\":\"Resignation is unavailable in AI vs. AI.\"}",
                );
            }
            if a.adjudicated.is_none() && a.game.result().is_none() {
                let loser = a
                    .lan
                    .as_ref()
                    .and_then(|lan| lan.player(&token))
                    .or(a.human)
                    .unwrap_or(a.game.position().white);
                a.adjudicated = Some(format!(
                    "{} wins by resignation",
                    if loser { "Black" } else { "White" }
                ));
                a.revision += 1;
                a.release_cache();
                a.clock.stop(Instant::now());
            }
            Ok(viewer_json(&mut a, &token, port))
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
    let opening = quote(a.game.opening_name());
    let last = a
        .game
        .moves
        .last()
        .map_or("null".into(), |m| format!("[{},{}]", m.from, m.to));
    let stats = a.stats.as_ref().map_or("null".into(), |s| {
        format!(
            "{{\"depth\":{},\"nodes\":{},\"hits\":{},\"millis\":{},\"threads\":{},\"cache_bytes\":{},\"cache_limit\":{},\"budget_millis\":{}}}",
            s.depth,
            s.nodes,
            s.hits,
            s.elapsed.as_millis(),
            s.threads,
            s.cache_bytes,
            s.cache_limit,
            a.search_budget.unwrap_or_default().as_millis()
        )
    });
    let clocks = format!("{{\"enabled\":{},\"white_ms\":{},\"black_ms\":{},\"running\":{},\"active_white\":{},\"base_seconds\":{},\"increment_seconds\":{}}}",
        !a.unlimited, a.clock.remaining[0].as_millis(), a.clock.remaining[1].as_millis(), a.clock.active.is_some(),
        a.clock.active.map_or("null", |c| if c { "true" } else { "false" }), a.clock.base.as_secs(), a.clock.increment.as_secs());
    let analysis = a.analysis.as_ref().map_or("null".into(), |analysis| {
        let s = *analysis.progress.lock().unwrap();
        let active = (a.thinking || a.pondering) && analysis.revision == a.revision;
        let evaluation = s.evaluation.map_or("null".into(), |score| score.to_string());
        let budget = analysis.budget.map_or("null".into(), |b| b.as_millis().to_string());
        format!("{{\"revision\":{},\"pondering\":{},\"active\":{},\"depth\":{},\"searching_depth\":{},\"evaluation\":{evaluation},\"nodes\":{},\"hits\":{},\"millis\":{},\"threads\":{},\"cache_bytes\":{},\"cache_limit\":{},\"budget_millis\":{budget}}}",
            analysis.revision, analysis.pondering, active, s.depth, s.searching_depth, s.nodes, s.hits, s.elapsed.as_millis(), s.threads, s.cache_bytes, s.cache_limit)
    });
    format!("{{\"board\":[{board}],\"started_at\":{},\"white\":{},\"human\":{},\"ai_vs_ai\":{},\"clocks\":{clocks},\"thinking_seconds\":{},\"revision\":{},\"thinking\":{},\"pondering\":{},\"check\":{},\"result\":{},\"legal\":[{moves}],\"notation\":[{notation}],\"opening\":{opening},\"last\":{last},\"stats\":{stats},\"analysis\":{analysis}}}",
        a.started_at.map_or("null".to_string(), |t| t.to_string()), p.white, a.human.map_or("null", |c| if c { "true" } else { "false" }), a.ai_vs_ai, a.fixed_thinking.as_secs_f64(), a.revision, a.thinking, a.pondering, p.in_check(p.white), result.map_or("null".into(), |r| quote(&r)))
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
        raw_peer(app, request, true)
    }
    fn raw_peer(app: &Shared, request: &str, local: bool) -> (u16, String) {
        let mut conn = Connection {
            input: Cursor::new(request.as_bytes().to_vec()),
            output: vec![],
        };
        serve_connection(&mut conn, app.clone(), 7878, local).unwrap();
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
    fn ai_vs_ai_alternates_without_pondering_and_cancels_on_replacement() {
        let app = shared();
        let body = request(
            &app,
            "POST",
            "/api/new?mode=two&two_player_mode=ai&color=white&unlimited=true&thinking_seconds=0.05",
            200,
        );
        assert!(body.contains("\"ai_vs_ai\":true"));
        let deadline = Instant::now() + Duration::from_secs(5);
        loop {
            let a = app.lock().unwrap();
            assert!(!a.pondering);
            assert!(a.thinking);
            let analysis = a.analysis.as_ref().unwrap();
            assert!(!analysis.pondering);
            assert_eq!(analysis.budget, Some(Duration::from_millis(50)));
            assert_eq!(analysis.revision, a.revision);
            assert_eq!(a.game.position().white, a.game.moves.len() % 2 == 0);
            if a.game.moves.len() >= 2 {
                break;
            }
            drop(a);
            assert!(Instant::now() < deadline, "AIs did not alternate turns");
            thread::sleep(Duration::from_millis(5));
        }
        {
            let a = app.lock().unwrap();
            let target = format!(
                "/api/move?from=12&to=28&promotion=0&revision={}",
                a.revision
            );
            drop(a);
            request(&app, "POST", &target, 400);
        }
        request(&app, "POST", "/api/resign", 400);
        // Hold the next search long enough to inspect undo before another move.
        let moves_before_undo = {
            let mut a = app.lock().unwrap();
            a.fixed_thinking = Duration::from_secs(3);
            a.cancel_search();
            queue_engine(&app, &mut a);
            a.game.moves.len()
        };
        request(&app, "POST", "/api/undo", 200);
        let old_cancel = {
            let a = app.lock().unwrap();
            assert!(a.thinking && !a.pondering);
            assert_eq!(a.game.moves.len(), moves_before_undo - 1);
            assert_eq!(a.game.position().white, a.game.moves.len() % 2 == 0);
            assert_eq!(a.clock_history.len(), a.game.moves.len() + 1);
            a.cancel.clone()
        };
        new_game(&app, "two", "white");
        assert!(old_cancel.load(Ordering::Relaxed));
        thread::sleep(Duration::from_millis(80));
        let a = app.lock().unwrap();
        assert!(!a.ai_vs_ai && !a.thinking && !a.pondering);
        assert!(a.game.moves.is_empty());
    }

    #[test]
    fn ai_vs_ai_stops_at_terminal_positions_and_timeout() {
        for fen in [
            "7k/6Q1/6K1/8/8/8/8/8 b - - 0 1",
            "7k/5Q2/6K1/8/8/8/8/8 b - - 0 1",
            "7k/8/6K1/8/8/8/8/8 w - - 0 1",
            "7k/8/6K1/8/8/8/8/R7 w - - 100 1",
        ] {
            let app = shared();
            {
                let mut a = app.lock().unwrap();
                a.ai_vs_ai = true;
                a.human = None;
                a.game = Game::new(Position::from_fen(fen).unwrap());
            }
            start_engine(&app);
            let a = app.lock().unwrap();
            assert!(a.game.result().is_some());
            assert!(!a.thinking && !a.pondering);
        }
        let app = shared();
        new_game(&app, "two", "white");
        {
            let mut a = app.lock().unwrap();
            a.ai_vs_ai = true;
            a.clock.remaining = [Duration::ZERO; 2];
        }
        start_engine(&app);
        let a = app.lock().unwrap();
        assert!(a.adjudicated.is_some());
        assert!(!a.thinking && !a.pondering);
    }

    #[test]
    fn bundled_assets_and_initial_state() {
        let app = shared();
        for target in [
            "/",
            "/join=maple.river.otter.moon",
            "/style.css",
            "/app.js",
            "/favicon.svg",
        ] {
            assert!(!request(&app, "GET", target, 200).is_empty());
        }
        let body = request(&app, "GET", "/api/state", 200);
        assert!(body.contains("\"white\":true"));
        assert!(body.contains("\"notation\":[]"));
        assert!(body.contains("\"opening\":\"Starting position\""));
        request(&app, "GET", "/openings.js", 404);
        request(&app, "GET", "/missing", 404);
    }
    #[test]
    fn opening_name_tracks_moves_deviations_undo_and_new_game() {
        let app = shared();
        new_game(&app, "two", "white");
        assert!(play(&app, "b2b4").contains("\"opening\":\"Polish Opening\""));
        let variation = "\"opening\":\"Polish Opening: Karniewski Variation\"";
        assert!(play(&app, "g8h6").contains(variation));
        assert!(play(&app, "a2a3").contains(variation));
        assert!(request(&app, "POST", "/api/undo", 200).contains(variation));
        assert!(request(&app, "POST", "/api/undo", 200).contains("\"opening\":\"Polish Opening\""));
        new_game(&app, "two", "white");
        assert!(
            request(&app, "GET", "/api/state", 200).contains("\"opening\":\"Starting position\"")
        );
    }
    #[test]
    fn random_color_is_assigned_once_for_solo_and_lan() {
        for mode in ["one", "two"] {
            let app = shared();
            let lan_param = if mode == "two" {
                "&two_player_mode=lan"
            } else {
                ""
            };
            let body = request(
                &app,
                "POST",
                &format!("/api/new?mode={mode}&color=random&base_seconds=60&increment_seconds=0{lan_param}"),
                200,
            );
            let (white, token) = {
                let a = app.lock().unwrap();
                if let Some(lan) = &a.lan {
                    assert_eq!(lan.player(&lan.host), Some(lan.host_white));
                    assert!(body.contains(&format!("\"player\":{}", lan.host_white)));
                    assert_eq!(a.clock.active, None);
                    (lan.host_white, lan.host.clone())
                } else {
                    let white = a.human.unwrap();
                    assert!(body.contains(&format!("\"human\":{white}")));
                    (white, String::new())
                }
            };
            let state = request(&app, "GET", &format!("/api/state?player={token}"), 200);
            let field = if mode == "two" { "player" } else { "human" };
            assert!(state.contains(&format!("\"{field}\":{white}")));
        }
    }

    #[test]
    fn lan_seats_clocks_reconnect_and_authority() {
        for host_white in [true, false] {
            let color = if host_white { "white" } else { "black" };
            let app = shared();
            let body = request(
            &app,
            "POST",
            &format!("/api/new?mode=two&two_player_mode=lan&color={color}&base_seconds=60&increment_seconds=1"),
            200,
        );
            assert!(body.contains("\"waiting\":true"));
            assert!(body.contains(&format!("\"player\":{host_white}")));
            assert!(body.contains("\"is_host\":true"));
            let (host, guest, invite) = {
                let a = app.lock().unwrap();
                assert_eq!(a.clock.active, None);
                assert!(!a.thinking && !a.pondering);
                let lan = a.lan.as_ref().unwrap();
                (lan.host.clone(), lan.guest.clone(), lan.invite.clone())
            };
            let white = if host_white { &host } else { &guest };
            let black = if host_white { &guest } else { &host };
            let words: Vec<_> = invite.split('.').collect();
            assert_eq!(words.len(), 4);
            assert!(words
                .iter()
                .all(|word| !word.is_empty() && word.bytes().all(|b| b.is_ascii_lowercase())));
            let remote = |path: &str| {
                raw_peer(&app, &format!("POST {path} HTTP/1.1\r\nHost: 192.168.1.10:7878\r\nOrigin: http://192.168.1.10:7878\r\nX-Chess-Request: 1\r\n\r\n"), false)
            };
            let remote_state = |token: &str| {
                raw_peer(
                    &app,
                    &format!(
                        "GET /api/state?player={token} HTTP/1.1\r\nHost: 192.168.1.10:7878\r\n\r\n"
                    ),
                    false,
                )
                .1
            };
            assert!(remote_state("").contains("\"invite_required\":true"));
            assert!(remote_state("invalid").contains("\"invite_required\":true"));
            assert!(request(&app, "GET", "/api/state", 200).contains("\"invite_required\":false"));
            let rev = app.lock().unwrap().revision;
            request(
                &app,
                "POST",
                &format!("/api/move?from=12&to=28&promotion=0&revision={rev}&player={host}"),
                403,
            );
            assert_eq!(remote("/api/join?invite=wrong").0, 400);
            let (status, joined) = remote(&format!("/api/join?invite={invite}"));
            assert_eq!(status, 200);
            assert!(joined.contains(&format!("\"player_token\":\"{guest}\"")));
            assert!(joined.contains(&format!("\"player\":{}", !host_white)));
            assert!(joined.contains("\"is_host\":false"));
            assert!(!joined.contains(&host));
            assert!(!joined.contains(&invite));
            assert_eq!(app.lock().unwrap().clock.active, Some(true));
            assert_eq!(remote(&format!("/api/join?invite={invite}")).0, 400);
            assert_eq!(remote(&format!("/api/join?player={guest}")).0, 200);
            assert!(remote_state(&guest).contains("\"invite_required\":false"));
            let rev = app.lock().unwrap().revision;
            assert_eq!(
                remote(&format!(
                    "/api/move?from=12&to=28&promotion=0&revision={rev}&player={black}"
                ))
                .0,
                403
            );
            request(
                &app,
                "POST",
                &format!("/api/move?from=12&to=28&promotion=0&revision={rev}&player={white}"),
                200,
            );
            let rev = app.lock().unwrap().revision;
            assert_eq!(
                remote(&format!(
                    "/api/move?from=52&to=36&promotion=0&revision={rev}&player={black}"
                ))
                .0,
                200
            );
            request(&app, "POST", &format!("/api/undo?player={host}"), 403);
            assert_eq!(remote(&format!("/api/new?player={guest}")).0, 403);
            request(
                &app,
                "POST",
                "/api/new?mode=two&color=white&unlimited=true",
                403,
            );
            assert_eq!(remote("/api/resign").0, 403);
            assert_eq!(remote(&format!("/api/resign?player={guest}")).0, 200);
            assert_eq!(
                app.lock().unwrap().adjudicated.as_deref(),
                Some(if host_white {
                    "White wins by resignation"
                } else {
                    "Black wins by resignation"
                })
            );
            request(
                &app,
                "POST",
                &format!(
                "/api/new?mode=two&two_player_mode=same&color=white&unlimited=true&player={host}"
            ),
                200,
            );
            assert_eq!(
                remote("/api/move?from=12&to=28&promotion=0&revision=0").0,
                403
            );
            let spoof = "POST /api/join HTTP/1.1\r\nHost: 192.168.1.10:7878\r\nOrigin: http://192.168.1.11:7878\r\nX-Chess-Request: 1\r\n\r\n";
            assert_eq!(raw_peer(&app, spoof, false).0, 403);
        }
    }

    #[test]
    fn lan_host_leaving_revokes_guest_and_invitation() {
        let app = shared();
        request(
            &app,
            "POST",
            "/api/new?mode=two&two_player_mode=lan&color=white&base_seconds=60&increment_seconds=0",
            200,
        );
        let (host, guest, invite) = {
            let a = app.lock().unwrap();
            let lan = a.lan.as_ref().unwrap();
            (lan.host.clone(), lan.guest.clone(), lan.invite.clone())
        };
        request(&app, "POST", &format!("/api/join?invite={invite}"), 200);
        request(&app, "POST", &format!("/api/leave?player={guest}"), 403);
        let remote_leave = format!("POST /api/leave?player={host} HTTP/1.1\r\nHost: 192.168.1.10:7878\r\nX-Chess-Request: 1\r\n\r\n");
        assert_eq!(raw_peer(&app, &remote_leave, false).0, 403);
        request(&app, "POST", &format!("/api/leave?player={host}"), 200);
        assert_eq!(app.lock().unwrap().clock.active, None);
        let state = request(&app, "GET", &format!("/api/state?player={guest}"), 200);
        assert!(state.contains("\"host_left\":true"));
        assert!(state.contains("\"player\":null"));
        request(&app, "POST", &format!("/api/join?invite={invite}"), 400);
        request(&app, "POST", &format!("/api/join?player={guest}"), 400);
        request(
            &app,
            "POST",
            &format!("/api/move?from=52&to=36&promotion=0&player={guest}"),
            403,
        );
        request(
            &app,
            "POST",
            &format!(
                "/api/new?mode=two&two_player_mode=lan&color=white&unlimited=true&player={host}"
            ),
            200,
        );
        let state = request(&app, "GET", &format!("/api/state?player={guest}"), 200);
        assert!(state.contains("\"host_left\":false"));
        assert!(state.contains("\"player\":null"));
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
    fn game_start_time_is_shared_and_survives_moves_and_undo() {
        let app = shared();
        new_game(&app, "two", "white");
        app.lock().unwrap().started_at = Some(1_700_000_000);
        let expected = "\"started_at\":1700000000";
        assert!(request(&app, "GET", "/api/state", 200).contains(expected));
        assert!(play(&app, "e2e4").contains(expected));
        assert!(request(&app, "POST", "/api/undo", 200).contains(expected));
        new_game(&app, "two", "white");
        assert_ne!(app.lock().unwrap().started_at, Some(1_700_000_000));
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
    fn undo_after_resignation_restarts_pondering_with_a_fresh_cache() {
        let app = shared();
        new_game(&app, "one", "white");
        play(&app, "e2e4");
        wait_for_engine(&app);
        request(&app, "POST", "/api/resign", 200);
        assert!(!app.lock().unwrap().pondering);
        request(&app, "POST", "/api/undo", 200);
        let a = app.lock().unwrap();
        assert!(a.game.moves.is_empty());
        assert!(a.adjudicated.is_none());
        assert!(a.pondering);
        assert!(!a.cancel.load(Ordering::Relaxed));
    }

    #[test]
    fn a_new_game_can_replace_a_pending_cache_release() {
        let app = shared();
        new_game(&app, "one", "black");
        request(&app, "POST", "/api/resign", 200);
        new_game(&app, "one", "black");
        wait_for_engine(&app);
        let a = app.lock().unwrap();
        assert_eq!(a.game.moves.len(), 1);
        assert!(a.adjudicated.is_none());
        assert!(a.pondering);
    }
    #[test]
    fn pondering_preserves_the_human_turn_and_computer_clock() {
        let app = shared();
        new_game(&app, "one", "white");
        let (cancel, black_time, revision) = {
            let a = app.lock().unwrap();
            assert!(a.pondering);
            assert!(!a.thinking);
            (a.cancel.clone(), a.clock.remaining[1], a.revision)
        };
        thread::sleep(Duration::from_millis(30));
        let body = request(&app, "GET", "/api/state", 200);
        assert!(body.contains("\"pondering\":true"));
        {
            let a = app.lock().unwrap();
            assert!(a.game.moves.is_empty());
            assert_eq!(a.revision, revision);
            assert_eq!(a.clock.remaining[1], black_time);
            assert!(a.clock.remaining[0] < black_time);
        }
        play(&app, "e2e4");
        assert!(cancel.load(Ordering::Relaxed));
        wait_for_engine(&app);
        let a = app.lock().unwrap();
        assert_eq!(a.game.moves.len(), 2);
        assert!(a.pondering);
        assert!(!a.thinking);
        assert!(a.game.position().white);
    }

    #[test]
    fn pondering_stops_on_replacement_resignation_timeout_and_drop() {
        let app = shared();
        new_game(&app, "one", "white");
        let cancel = app.lock().unwrap().cancel.clone();
        new_game(&app, "two", "white");
        assert!(cancel.load(Ordering::Relaxed));
        assert!(!app.lock().unwrap().pondering);

        new_game(&app, "one", "white");
        let cancel = app.lock().unwrap().cancel.clone();
        request(&app, "POST", "/api/resign", 200);
        assert!(cancel.load(Ordering::Relaxed));
        assert!(!app.lock().unwrap().pondering);

        new_game(&app, "one", "white");
        let cancel = app.lock().unwrap().cancel.clone();
        {
            let mut a = app.lock().unwrap();
            a.clock.remaining[0] = Duration::ZERO;
            a.tick(Instant::now());
            assert!(!a.pondering);
            assert!(a.adjudicated.is_some());
        }
        assert!(cancel.load(Ordering::Relaxed));

        new_game(&app, "one", "white");
        let cancel = app.lock().unwrap().cancel.clone();
        drop(app);
        // An in-flight completion may briefly hold the last strong reference.
        let deadline = Instant::now() + Duration::from_secs(2);
        while !cancel.load(Ordering::Relaxed) {
            assert!(Instant::now() < deadline, "Dropped app kept pondering");
            thread::sleep(Duration::from_millis(1));
        }
    }

    #[test]
    fn undo_cancels_pondering_and_starts_fresh_analysis() {
        let app = shared();
        new_game(&app, "one", "white");
        play(&app, "e2e4");
        wait_for_engine(&app);
        let (cancel, epoch) = {
            let a = app.lock().unwrap();
            assert!(a.pondering);
            (a.cancel.clone(), a.cache_epoch)
        };
        request(&app, "POST", "/api/undo", 200);
        assert!(cancel.load(Ordering::Relaxed));
        let a = app.lock().unwrap();
        assert!(a.pondering);
        assert!(!a.thinking);
        assert!(a.game.moves.is_empty());
        assert_eq!(a.cache_epoch, epoch + 1);
        assert!(a.stats.is_none());
        assert!(!a.cancel.load(Ordering::Relaxed));
    }

    #[test]
    fn live_analysis_resets_on_a_move_and_cannot_leak_into_a_new_game() {
        let app = shared();
        new_game(&app, "one", "white");
        let ponder_progress = app
            .lock()
            .unwrap()
            .analysis
            .as_ref()
            .unwrap()
            .progress
            .clone();
        play(&app, "e2e4");
        {
            let a = app.lock().unwrap();
            let analysis = a.analysis.as_ref().unwrap();
            assert!(!Arc::ptr_eq(&ponder_progress, &analysis.progress));
            assert!(!analysis.pondering);
            assert!(analysis.budget.is_some());
            assert_eq!(analysis.revision, a.revision);
        }
        new_game(&app, "two", "white");
        // A late publication only updates its abandoned snapshot.
        ponder_progress.lock().unwrap().nodes = 123_456;
        let body = request(&app, "GET", "/api/state", 200);
        assert!(body.contains("\"analysis\":null"));
        assert!(body.contains("\"stats\":null"));
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
