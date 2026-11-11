// Diagnostic-only code appended to a temporary engine copy.
pub fn preference_diagnostic(reply: &str, target_depth: u8, no_table: bool) {
    let mut p = Position::default();
    let mut keys = vec![p.repetition_key()];
    for san in ["e4", reply] {
        let moves = p.legal_moves();
        let m = *moves.iter().find(|&&m| p.san(m, &moves) == san).unwrap();
        p = p.play(m);
        keys.push(p.repetition_key());
    }
    let mut cache = SearchCache::with_threads_and_memory(1, if no_table { 0 } else { 100_000_000 });
    cache.table.initialize();
    cache.table.next_generation();
    let started = Instant::now();
    let mut s = Search {
        table: &cache.table, buffers: &mut cache.buffers,
        history_context: reversible_context(&p, &keys), keys,
        deadline: Some(started + Duration::from_secs(120)), nodes: 0, hits: 0,
        aborted: false, cancelled: Arc::default(), progress: None,
        started, published: started, depth: 0, searching_depth: 0,
        evaluation: None, threads: 1, peers: &[], reporting: None,
        reported: (0, 0), rest_at: None,
    };
    let mut result = (0, None);
    for depth in 1..=target_depth {
        result = s.negamax(&p, depth, 0, -INF, INF);
        assert!(!s.aborted, "diagnostic exceeded deadline");
    }
    let nodes = s.nodes;
    let hits = s.hits;
    let original_keys = s.keys.clone();
    let original_context = s.history_context;
    let mut line = Vec::new();
    let mut steps = Vec::new();
    let mut previous = Vec::new();
    let mut position = p.clone();
    let mut ply = 0;
    for depth in (1..=target_depth).rev() {
        let (score, best) = s.negamax(&position, depth, ply, -INF, INF);
        let Some(m) = best else { break };
        let legal = position.legal_moves();
        assert!(legal.contains(&m));
        line.push(position.san(m, &legal));
        steps.push(if position.white { score } else { -score });
        position = position.play(m);
        previous.push(s.push_position(&position));
        ply += 1;
    }
    // Resolve the quiescent continuation with exact full windows. A stand-pat
    // result is a search bound, not a legal quiet move; stop when it wins.
    let mut qply = 0;
    while qply < 10 {
        let mut tactical = Vec::new();
        let state = position.tactical_moves_into(&mut tactical);
        if !state.has_legal || s.is_draw(&position) { break; }
        let score = s.quiescence(&position, ply, qply, -INF, INF);
        let stand = s.buffers.evaluation.get(&position);
        if !state.in_check && score == stand { break; }
        let mut chosen = None;
        for m in tactical {
            let child = position.play(m);
            let old = s.push_position(&child);
            let value = -s.quiescence(&child, ply + 1, qply + 1, -INF, INF);
            s.pop_position(old);
            if value == score { chosen = Some(m); break; }
        }
        let Some(m) = chosen else { break };
        line.push(position.san(m, &position.legal_moves()));
        steps.push(if position.white { score } else { -score });
        position = position.play(m);
        previous.push(s.push_position(&position));
        ply += 1;
        qply += 1;
    }
    let (leaf_white_score, terms) = preference_eval_detail(&position);
    let quiet_d4 = position.legal_moves().into_iter()
        .find(|m| position.white && m.from == 11 && m.to == 27)
        .map(|m| {
            let mut tactical = Vec::new();
            position.tactical_moves_into(&mut tactical);
            let child = position.play(m);
            format!("{{\"legal\":true,\"in_quiescence_moves\":{},\"white_static_after\":{}}}",
                    tactical.contains(&m), preference_eval_detail(&child).0)
        }).unwrap_or_else(|| "null".to_string());
    for old in previous.into_iter().rev() { s.pop_position(old); }
    assert_eq!(s.keys, original_keys);
    assert_eq!(s.history_context, original_context);
    let root_score = if p.white { result.0 } else { -result.0 };
    let squares: Vec<String> = position.squares.iter().map(|x| x.to_string()).collect();
    println!("{{\"reply\":\"{reply}\",\"depth_after_reply\":{target_depth},\"no_table\":{no_table},\"score_cp\":{root_score},\"nodes\":{nodes},\"hits\":{hits},\"pv\":{:?},\"pv_white_scores\":{:?},\"q_plies\":{qply},\"leaf_white_score\":{leaf_white_score},\"quiet_d4\":{quiet_d4},\"leaf_terms\":{:?},\"leaf_squares\":[{}],\"cache_bytes\":{}}}",line,steps,terms,squares.join(","),s.table.bytes());
}
