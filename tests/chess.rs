use chess_ai::{
    chess::{Move, Position},
    engine::search,
    game::{Game, Outcome},
};
use std::time::Duration;
fn find(p: &Position, s: &str) -> Move {
    p.legal_moves()
        .into_iter()
        .find(|m| m.to_string() == s)
        .unwrap_or_else(|| panic!("Missing move {}", s))
}
fn perft(p: &Position, d: u8) -> u64 {
    if d == 0 {
        return 1;
    }
    p.legal_moves()
        .into_iter()
        .map(|m| perft(&p.play(m), d - 1))
        .sum()
}
#[test]
fn starting_perft() {
    let p = Position::default();
    assert_eq!(perft(&p, 1), 20);
    assert_eq!(perft(&p, 2), 400);
    assert_eq!(perft(&p, 3), 8902);
    assert_eq!(perft(&p, 4), 197281);
}
#[test]
fn kiwipete_perft() {
    let p =
        Position::from_fen("r3k2r/p1ppqpb1/bn2pnp1/3PN3/1p2P3/2N2Q1p/PPPBBPPP/R3K2R w KQkq - 0 1")
            .unwrap();
    assert_eq!(perft(&p, 1), 48);
    assert_eq!(perft(&p, 2), 2039);
    assert_eq!(perft(&p, 3), 97862);
}
#[test]
fn endgame_perft() {
    let p = Position::from_fen("8/2p5/3p4/KP5r/1R3p1k/8/4P1P1/8 w - - 0 1").unwrap();
    assert_eq!(perft(&p, 3), 2812);
}
#[test]
fn incremental_hashes() {
    fn walk(p: &Position, d: u8) {
        assert_eq!(p.hash, p.compute_hash());
        if d > 0 {
            for m in p.legal_moves() {
                walk(&p.play(m), d - 1)
            }
        }
    }
    walk(&Position::default(), 3);
    walk(
        &Position::from_fen("r3k2r/P6P/8/3pP3/8/8/p6p/R3K2R w KQkq d6 0 1").unwrap(),
        2,
    );
}
#[test]
fn castling_moves_rook_and_removes_rights() {
    let p = Position::from_fen("r3k2r/8/8/8/8/8/8/R3K2R w KQkq - 0 1").unwrap();
    let p = p.play(find(&p, "e1g1"));
    assert_eq!(p.squares[5], 4);
    assert_eq!(p.squares[6], 6);
    assert_eq!(p.squares[7], 0);
    assert_eq!(p.castling, 12);
    assert_eq!(p.hash, p.compute_hash());
}
#[test]
fn cannot_castle_through_check() {
    let p = Position::from_fen("4kr2/8/8/8/8/8/8/R3K2R w KQ - 0 1").unwrap();
    assert!(!p.legal_moves().iter().any(|m| m.to_string() == "e1g1"));
    assert!(p.legal_moves().iter().any(|m| m.to_string() == "e1c1"));
}
#[test]
fn en_passant_and_pin() {
    let p = Position::from_fen("4k3/8/8/3pP3/8/8/8/4K3 w - d6 0 1").unwrap();
    let next = p.play(find(&p, "e5d6"));
    assert_eq!(next.squares[35], 0);
    assert_eq!(next.squares[43], 1);
    assert_eq!(next.halfmove, 0);
    let pinned = Position::from_fen("k3r3/8/8/3pP3/8/8/8/4K3 w - d6 0 1").unwrap();
    assert!(!pinned.legal_moves().iter().any(|m| m.to_string() == "e5d6"));
}
#[test]
fn promotion_has_four_choices() {
    let p = Position::from_fen("4k3/P7/8/8/8/8/8/4K3 w - - 0 1").unwrap();
    assert_eq!(p.legal_moves().iter().filter(|m| m.from == 48).count(), 4);
    assert_eq!(p.play(find(&p, "a7a8n")).squares[56], 2);
}
#[test]
fn fools_mate_and_undo() {
    let mut g = Game::default();
    for s in ["f2f3", "e7e5", "g2g4", "d8h4"] {
        let m = find(g.position(), s);
        g.play(m).unwrap();
    }
    assert_eq!(
        g.result(),
        Some(Outcome::Checkmate {
            winner_white: false
        })
    );
    assert_eq!(g.notation.last().unwrap(), "Qh4#");
    assert!(g.undo());
    assert_eq!(g.result(), None);
}
#[test]
fn repetition_and_identity() {
    let mut g = Game::default();
    for s in [
        "g1f3", "g8f6", "f3g1", "f6g8", "g1f3", "g8f6", "f3g1", "f6g8",
    ] {
        g.play(find(g.position(), s)).unwrap();
    }
    assert_eq!(g.result(), Some(Outcome::Repetition));
    let a = Position::from_fen("4k3/8/8/3p4/8/8/8/4K3 w - d6 0 1").unwrap();
    let b = Position::from_fen("4k3/8/8/3p4/8/8/8/4K3 w - - 0 1").unwrap();
    assert_ne!(a.hash, b.hash);
    assert_eq!(a.repetition_key(), b.repetition_key());
}
#[test]
fn draws() {
    let p = Position::from_fen("7k/5Q2/6K1/8/8/8/8/8 b - - 0 1").unwrap();
    assert_eq!(Game::new(p).result(), Some(Outcome::Stalemate));
    let p = Position::from_fen("4k3/8/8/8/8/8/8/3RK3 w - - 100 51").unwrap();
    assert_eq!(Game::new(p).result(), Some(Outcome::FiftyMove));
    let p = Position::from_fen("4k3/8/8/8/8/8/8/3BK3 w - - 0 1").unwrap();
    assert_eq!(Game::new(p).result(), Some(Outcome::InsufficientMaterial));
}
#[test]
fn engine_finds_mate() {
    let mut g = Game::default();
    for s in ["f2f3", "e7e5", "g2g4"] {
        g.play(find(g.position(), s)).unwrap();
    }
    let r = search(g.position(), &g.keys, Duration::from_secs(3));
    assert_eq!(r.best.unwrap().to_string(), "d8h4");
    assert!(r.score > 29000);
}
#[test]
fn engine_returns_legal_move_and_draw() {
    let p = Position::default();
    let r = search(&p, &[p.repetition_key()], Duration::from_secs(2));
    assert!(p.legal_moves().contains(&r.best.unwrap()));
    assert!(r.depth >= 2);
    // An ordinary position should keep deepening until its budget is spent.
    assert!(r.elapsed >= Duration::from_secs(2));
    assert!(r.elapsed < Duration::from_secs(5));
    let p = Position::from_fen("4k3/8/8/8/8/8/8/4K3 w - - 0 1").unwrap();
    assert!(search(&p, &[p.repetition_key()], Duration::from_secs(1))
        .best
        .is_none());
}

#[test]
fn engine_zero_budget_returns_a_legal_fallback() {
    let p = Position::default();
    let r = search(&p, &[p.repetition_key()], Duration::ZERO);
    assert!(p.legal_moves().contains(&r.best.unwrap()));
    assert_eq!(r.depth, 0);
}
