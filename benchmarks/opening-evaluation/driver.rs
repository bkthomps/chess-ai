// Appended only to temporary engine copies by study.py.
pub fn opening_study(path: &str, depth: u8) {
    use std::io::{self, BufRead};
    for line in io::BufReader::new(std::fs::File::open(path).unwrap()).lines() {
        let line = line.unwrap();
        let (id, sequence) = line.split_once('\t').unwrap();
        let mut p = Position::default();
        let mut keys = vec![p.repetition_key()];
        for san in sequence.split_whitespace() {
            let legal = p.legal_moves();
            let m = *legal.iter().find(|&&m| p.san(m, &legal) == san)
                .unwrap_or_else(|| panic!("illegal SAN {san} in {sequence}"));
            p = p.play(m);
            keys.push(p.repetition_key());
        }
        let mut board = String::new();
        for rank in (0..8).rev() {
            if rank < 7 { board.push('/'); }
            let mut empty = 0;
            for file in 0..8 {
                let pc = p.squares[rank * 8 + file];
                if pc == 0 { empty += 1; continue; }
                if empty > 0 { board.push(char::from(b'0' + empty)); empty = 0; }
                let piece = b" PNBRQK"[pc.unsigned_abs() as usize] as char;
                board.push(if pc > 0 {piece} else {piece.to_ascii_lowercase()});
            }
            if empty > 0 { board.push(char::from(b'0' + empty)); }
        }
        let mut rights = String::new();
        for (bit, letter) in [(1, 'K'), (2, 'Q'), (4, 'k'), (8, 'q')] {
            if p.castling & bit != 0 { rights.push(letter); }
        }
        if rights.is_empty() { rights.push('-'); }
        let ep = p.ep.map_or_else(|| "-".to_string(), crate::chess::square_name);
        let fen = format!("{board} {} {rights} {ep} {} {}", if p.white {"w"} else {"b"}, p.halfmove, p.fullmove);
        let static_score = if p.white { evaluate(&p) } else { -evaluate(&p) };
        let mut cache = SearchCache::with_threads_and_memory(1, 100_000_000);
        cache.table.initialize();
        cache.table.next_generation();
        let started = Instant::now();
        let mut search = Search {
            table: &cache.table, buffers: &mut cache.buffers, keys,
            history_context: 0, deadline: Some(started + Duration::from_secs(120)),
            nodes: 0, hits: 0, aborted: false, cancelled: Arc::default(),
            progress: None, started, published: started, depth: 0,
            searching_depth: 0, evaluation: None, threads: 1, peers: &[],
            reporting: None, reported: (0, 0), rest_at: None,
        };
        search.history_context = reversible_context(&p, &search.keys);
        let mut result = (0, None);
        for d in 1..=depth {
            result = search.negamax(&p, d, 0, -INF, INF);
            assert!(!search.aborted, "incomplete depth {d}: {sequence}");
        }
        let score = if p.white {result.0} else {-result.0};
        println!("{{\"id\":\"{id}\",\"fen\":\"{fen}\",\"static_cp\":{static_score},\"score_cp\":{score},\"depth\":{depth},\"best\":\"{}\",\"nodes\":{},\"hits\":{},\"cache_bytes\":{},\"cache_limit\":{}}}", result.1.map_or_else(String::new, |m| m.to_string()), search.nodes, search.hits, search.table.bytes(),search.table.limit());
    }
}
