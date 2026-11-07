//! Move-generation and game-result regression positions.
//! Fixtures are checked in: the Java project is not needed to run these tests.
use chess_ai::{
    chess::Position,
    game::{Game, Outcome},
};

fn rows(text: &str) -> impl Iterator<Item = Vec<&str>> {
    text.lines()
        .filter(|l| !l.starts_with('#') && !l.is_empty())
        .map(|l| l.split('\t').collect())
}
#[derive(Default, Debug, PartialEq, Eq)]
struct Counts {
    nodes: u64,
    promotions: u64,
    castles: u64,
    ep: u64,
}
fn perft(p: &Position, depth: u8) -> Counts {
    let mut counts = Counts::default();
    for m in p.legal_moves() {
        let next = p.play(m);
        assert_eq!(next.hash, next.compute_hash(), "incremental hash after {m}");
        if depth == 1 {
            counts.nodes += 1;
            counts.promotions += u64::from(m.promotion != 0);
            counts.castles += u64::from(p.squares[m.from].abs() == 6 && m.from.abs_diff(m.to) == 2);
            counts.ep += u64::from(
                p.squares[m.from].abs() == 1 && p.ep == Some(m.to) && m.from % 8 != m.to % 8,
            );
        } else {
            let c = perft(&next, depth - 1);
            counts.nodes += c.nodes;
            counts.promotions += c.promotions;
            counts.castles += c.castles;
            counts.ep += c.ep;
        }
    }
    counts
}
fn perft_case(name: &str) {
    for r in rows(include_str!("fixtures/regression_perft.tsv")).filter(|r| r[0] == name) {
        let c = perft(&Position::from_fen(r[6]).unwrap(), r[1].parse().unwrap());
        assert_eq!(
            c.nodes,
            r[2].parse::<u64>().unwrap(),
            "{name} depth {}",
            r[1]
        );
        for (value, expected) in [(c.promotions, r[3]), (c.castles, r[4]), (c.ep, r[5])] {
            if expected != "-" {
                assert_eq!(
                    value,
                    expected.parse::<u64>().unwrap(),
                    "{name} depth {}: {c:?}",
                    r[1]
                );
            }
        }
    }
}
#[test]
fn regression_perft_start() {
    perft_case("position_1");
}
#[test]
fn regression_perft_kiwipete() {
    perft_case("position_2");
}
#[test]
fn regression_perft_endgame() {
    perft_case("position_3");
}
#[test]
fn regression_perft_promotions() {
    perft_case("position_4");
}
#[test]
fn regression_perft_mirrored() {
    perft_case("position_4_mirrored");
}
#[test]
fn regression_perft_tactics() {
    perft_case("position_5");
}
#[test]
fn regression_perft_middlegame() {
    perft_case("position_6");
}
#[test]
fn regression_smoke_moves() {
    for r in rows(include_str!("fixtures/regression_moves.tsv")) {
        assert_eq!(
            Position::from_fen(r[1]).unwrap().legal_moves().len(),
            r[0].parse::<usize>().unwrap(),
            "{}",
            r[1]
        );
    }
}
#[test]
fn regression_terminal_positions() {
    for r in rows(include_str!("fixtures/regression_terminal.tsv")) {
        let result = Game::new(Position::from_fen(r[1]).unwrap()).result();
        assert!(
            match r[0] {
                "checkmate" => matches!(result, Some(Outcome::Checkmate { .. })),
                _ => result == Some(Outcome::Stalemate),
            },
            "{}: {:?}",
            r[1],
            result
        );
    }
}
#[test]
fn regression_castling() {
    for r in rows(include_str!("fixtures/regression_castling.tsv")) {
        let p = Position::from_fen(r[2]).unwrap();
        assert_eq!(p.in_check(p.white), r[1] == "true", "{}", r[2]);
        assert_eq!(
            p.legal_moves()
                .iter()
                .filter(|m| p.squares[m.from].abs() == 6 && m.from.abs_diff(m.to) == 2)
                .count(),
            r[0].parse::<usize>().unwrap(),
            "{}",
            r[2]
        );
    }
}
#[test]
fn regression_material_draws() {
    for r in rows(include_str!("fixtures/regression_draws.tsv")) {
        assert_eq!(
            Game::new(Position::from_fen(r[1]).unwrap())
                .result()
                .is_some(),
            r[0] == "true",
            "{}",
            r[1]
        );
    }
}
#[test]
fn regression_promotion_choices() {
    for (fen, destinations) in [
        ("4k3/1P6/8/8/8/8/8/4K3 w - - 0 1", 1),
        ("2n1k3/1P6/8/8/8/8/8/4K3 w - - 0 1", 2),
        ("n3k3/1P6/8/8/8/8/8/4K3 w - - 0 1", 2),
        ("n1n1k3/1P6/8/8/8/8/8/4K3 w - - 0 1", 3),
        ("4k3/8/8/8/8/8/1p6/4K3 b - - 0 1", 1),
        ("4k3/8/8/8/8/8/1p6/2N1K3 b - - 0 1", 2),
        ("4k3/8/8/8/8/8/1p6/N3K3 b - - 0 1", 2),
        ("4k3/8/8/8/8/8/1p6/N1N1K3 b - - 0 1", 3),
    ] {
        let legal = Position::from_fen(fen).unwrap().legal_moves();
        assert_eq!(legal.len(), 5 + 4 * destinations);
        for piece in [2, 3, 4, 5] {
            assert_eq!(
                legal.iter().filter(|m| m.promotion == piece).count(),
                destinations
            );
        }
    }
}
