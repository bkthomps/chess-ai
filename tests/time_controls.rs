use chess_ai::{
    chess::Position,
    engine::{search_with_cancel, time_budget},
    game::timeout_result,
};
use std::{
    sync::{atomic::AtomicBool, Arc},
    time::Duration,
};

#[test]
fn timeout_material_is_checked_for_the_potential_winner() {
    for (fen, flagged_white, expected) in [
        ("4k3/8/8/8/8/8/8/3QK3 w - - 0 1", true, "Draw on time"),
        (
            "4k3/8/8/8/8/8/8/3QK3 b - - 0 1",
            false,
            "White wins on time",
        ),
        ("4k3/8/8/8/8/8/8/3qK3 w - - 0 1", true, "Black wins on time"),
        (
            "4k3/8/8/8/8/8/8/2NNK3 b - - 0 1",
            false,
            "White wins on time",
        ),
        ("4k3/8/8/8/8/8/8/3NK3 b - - 0 1", false, "Draw on time"),
        ("4k3/8/8/8/8/8/8/3BK3 b - - 0 1", false, "Draw on time"),
        (
            "4k3/8/8/8/8/8/8/2BNK3 b - - 0 1",
            false,
            "White wins on time",
        ),
        (
            "4k3/8/8/8/8/8/8/2BBK3 b - - 0 1",
            false,
            "White wins on time",
        ),
        // Opposing material can enable mate even with only a single minor.
        (
            "4k3/p7/8/8/8/8/8/3NK3 b - - 0 1",
            false,
            "White wins on time",
        ),
        (
            "4k3/p7/8/8/8/8/8/3BK3 b - - 0 1",
            false,
            "White wins on time",
        ),
        ("4k3/8/8/8/8/8/8/1B1BK3 b - - 0 1", false, "Draw on time"),
        ("b3k3/8/8/8/8/8/8/1B2K3 b - - 0 1", false, "Draw on time"),
        (
            "1b2k3/8/8/8/8/8/8/1B2K3 b - - 0 1",
            false,
            "White wins on time",
        ),
        ("q3k3/8/8/8/8/8/8/3NK3 b - - 0 1", false, "Draw on time"),
        (
            "r3k3/8/8/8/8/8/8/3NK3 b - - 0 1",
            false,
            "White wins on time",
        ),
    ] {
        assert!(
            timeout_result(&Position::from_fen(fen).unwrap(), flagged_white).starts_with(expected),
            "{fen}"
        );
    }
}

#[test]
fn engine_allocates_time_without_spending_its_clock() {
    let opening = Position::default();
    let endgame = Position::from_fen("4k3/p7/8/8/8/8/P7/4K3 w - - 0 1").unwrap();
    let minute = Duration::from_secs(60);
    let opening_budget = time_budget(&opening, minute, Duration::ZERO);
    assert!(opening_budget > Duration::from_secs(1) && opening_budget < Duration::from_secs(2));
    assert!(time_budget(&opening, minute, Duration::from_secs(1)) > opening_budget);
    assert!(time_budget(&endgame, minute, Duration::ZERO) > opening_budget);
    assert!(time_budget(&opening, Duration::from_secs(5), Duration::ZERO) < opening_budget);
    for remaining in [
        Duration::ZERO,
        Duration::from_millis(1),
        Duration::from_millis(10),
        Duration::from_secs(1),
        minute,
    ] {
        let budget = time_budget(&opening, remaining, Duration::from_secs(600));
        assert!(budget <= remaining.mul_f64(0.15));
        if !remaining.is_zero() {
            assert!(budget < remaining);
        }
    }
}

#[test]
fn cancelled_search_returns_without_spending_the_time_allowance() {
    let p = Position::default();
    let result = search_with_cancel(
        &p,
        &[p.repetition_key()],
        Duration::from_secs(30),
        Arc::new(AtomicBool::new(true)),
    );
    assert!(result.elapsed < Duration::from_millis(500));
    assert_eq!(result.depth, 0);
    assert!(p.legal_moves().contains(&result.best.unwrap()));
}

#[test]
fn forced_move_does_not_spend_the_search_budget() {
    let p = Position::from_fen("7r/8/8/8/8/5k2/8/7K w - - 0 1").unwrap();
    let legal = p.legal_moves();
    assert_eq!(legal.len(), 1);
    let result = chess_ai::engine::search(&p, &[p.repetition_key()], Duration::from_secs(30));
    assert_eq!(result.best, Some(legal[0]));
    assert!(result.elapsed < Duration::from_millis(500));
}
