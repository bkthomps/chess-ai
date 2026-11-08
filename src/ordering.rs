//! Quiet-move ordering, private to each search thread.
use super::{sort_moves, value, Move, Position, BUFFER_PLIES};

// The benchmark changes these constants only in temporary source copies to
// measure each heuristic independently. Production enables both.
const HISTORY_ENABLED: bool = true;
const KILLERS_ENABLED: bool = true;
const HISTORY_BOUND: i32 = 16_384;
type History = [[[i16; 64]; 64]; 2];
pub(super) const ORDERING_BYTES: usize =
    std::mem::size_of::<History>() + std::mem::size_of::<[[u16; 2]; BUFFER_PLIES]>();

pub(super) struct MoveOrdering {
    history: Box<History>,
    killers: [[u16; 2]; BUFFER_PLIES],
}
impl Default for MoveOrdering {
    fn default() -> Self {
        Self {
            history: Box::new([[[0; 64]; 64]; 2]),
            killers: [[0; 2]; BUFFER_PLIES],
        }
    }
}
impl MoveOrdering {
    pub(super) fn begin_search(&mut self) {
        self.killers.fill([0; 2]);
        // Keep useful learning across pondering/moves without allowing old
        // positions to dominate a new search indefinitely.
        for side in self.history.iter_mut() {
            for from in side {
                for score in from {
                    *score /= 2;
                }
            }
        }
    }
    pub(super) fn order(&self, p: &Position, moves: &mut [Move], best: Option<Move>, ply: usize) {
        let killers = self.killers.get(ply).copied().unwrap_or([0; 2]);
        let history = &self.history[usize::from(p.white)];
        sort_moves(moves, |m| {
            let capture = p.is_capture(m);
            let tactical = if capture {
                10_000 + value(p.squares[m.to]).max(100) * 10 - value(p.squares[m.from])
            } else {
                0
            } + value(m.promotion);
            if Some(m) == best {
                return 100_000 + tactical;
            }
            // Preserve capture/promotion priority and their original ordering.
            if capture || m.promotion != 0 {
                return 20_000 + tactical;
            }
            if KILLERS_ENABLED {
                let packed = pack_quiet(m);
                if killers[0] == packed {
                    return 19_000;
                }
                if killers[1] == packed {
                    return 18_000;
                }
            }
            if HISTORY_ENABLED {
                i32::from(history[m.from][m.to])
            } else {
                0
            }
        });
    }
    pub(super) fn cutoff(
        &mut self,
        p: &Position,
        winner: Move,
        ply: usize,
        depth: u8,
        earlier: &[Move],
    ) {
        if !is_quiet(p, winner) {
            return;
        }
        if KILLERS_ENABLED {
            if let Some(killers) = self.killers.get_mut(ply) {
                let packed = pack_quiet(winner);
                if killers[0] != packed {
                    killers[1] = killers[0];
                    killers[0] = packed;
                }
            }
        }
        if HISTORY_ENABLED {
            let bonus = (i32::from(depth) * i32::from(depth)).min(2048);
            let history = &mut self.history[usize::from(p.white)];
            update(&mut history[winner.from][winner.to], bonus);
            // Earlier quiet alternatives failed to produce this cutoff.
            for &m in earlier {
                if is_quiet(p, m) {
                    update(&mut history[m.from][m.to], -bonus);
                }
            }
        }
    }
}
fn is_quiet(p: &Position, m: Move) -> bool {
    m.promotion == 0 && !p.is_capture(m)
}
fn pack_quiet(m: Move) -> u16 {
    m.from as u16 | (m.to as u16) << 6
}
fn update(score: &mut i16, bonus: i32) {
    let old = i32::from(*score);
    // Gravity bounds the score while allowing fresh evidence to reverse it.
    *score = (old + bonus - old * bonus.abs() / HISTORY_BOUND) as i16;
}

#[cfg(test)]
mod tests {
    use super::*;

    fn mv(p: &Position, uci: &str) -> Move {
        p.legal_moves()
            .into_iter()
            .find(|m| m.to_string() == uci)
            .unwrap()
    }

    #[test]
    fn quiet_cutoffs_reward_winners_penalize_failed_moves_and_keep_two_killers() {
        let p = Position::default();
        let (a, b, c) = (mv(&p, "a2a3"), mv(&p, "b2b3"), mv(&p, "c2c3"));
        let mut order = MoveOrdering::default();
        order.cutoff(&p, b, 4, 5, &[a]);
        assert!(order.history[1][b.from][b.to] > 0);
        assert!(order.history[1][a.from][a.to] < 0);
        order.cutoff(&p, c, 4, 5, &[]);
        order.cutoff(&p, c, 4, 5, &[]);
        assert_eq!(order.killers[4], [pack_quiet(c), pack_quiet(b)]);
        let mut moves = vec![a, b, c];
        order.order(&p, &mut moves, None, 4);
        assert_eq!(moves, [c, b, a]);
        order.order(&p, &mut moves, Some(a), 4);
        assert_eq!(moves[0], a);
        // Other plies still benefit from history, with no killer override.
        order.order(&p, &mut moves, None, 5);
        assert_eq!(moves, [c, b, a]);
        let old = order.history[1][c.from][c.to];
        order.begin_search();
        assert_eq!(order.killers[4], [0; 2]);
        assert_eq!(order.history[1][c.from][c.to], old / 2);
    }

    #[test]
    fn tactical_moves_keep_priority_and_never_become_quiet_killers() {
        let p = Position::from_fen("4k3/P7/8/3p4/4P3/8/8/4K3 w - - 0 1").unwrap();
        let (capture, promotion, quiet) = (mv(&p, "e4d5"), mv(&p, "a7a8n"), mv(&p, "e1f1"));
        let mut order = MoveOrdering::default();
        order.cutoff(&p, capture, 0, 6, &[]);
        order.cutoff(&p, promotion, 0, 6, &[]);
        assert_eq!(order.killers[0], [0; 2]);
        order.cutoff(&p, quiet, 0, 6, &[]);
        let mut moves = vec![quiet, promotion, capture];
        order.order(&p, &mut moves, None, 0);
        assert_eq!(moves, [capture, promotion, quiet]);
        // A checking non-capture is still quiet under this engine's definition.
        let checking = Position::from_fen("4k3/8/8/8/8/8/R7/4K3 w - - 0 1").unwrap();
        let check = mv(&checking, "a2e2");
        assert!(checking.play(check).in_check(false));
        order.cutoff(&checking, check, 1, 4, &[]);
        assert_eq!(order.killers[1][0], pack_quiet(check));
    }

    #[test]
    fn history_is_bounded_color_specific_and_supports_plies_beyond_retained_killers() {
        let p = Position::default();
        let m = mv(&p, "g1f3");
        let mut order = MoveOrdering::default();
        for _ in 0..10_000 {
            order.cutoff(&p, m, BUFFER_PLIES + 1, 255, &[]);
        }
        let score = order.history[1][m.from][m.to];
        assert!(i32::from(score) <= HISTORY_BOUND && score > 0);
        assert_eq!(order.history[0][m.from][m.to], 0);
        let other = mv(&p, "a2a3");
        for _ in 0..10_000 {
            order.cutoff(&p, other, BUFFER_PLIES + 1, 255, &[m]);
        }
        let score = order.history[1][m.from][m.to];
        assert!(i32::from(score) >= -HISTORY_BOUND && score < 0);
        let mut moves = p.legal_moves();
        let mut expected = moves.clone();
        order.order(&p, &mut moves, None, BUFFER_PLIES + 1);
        moves.sort_by_key(|m| (m.from, m.to, m.promotion));
        expected.sort_by_key(|m| (m.from, m.to, m.promotion));
        assert_eq!(moves, expected);
    }

    #[test]
    fn fresh_order_matches_legacy_and_learning_preserves_large_legal_move_lists() {
        for fen in [
            "rnbqkbnr/pppppppp/8/8/8/8/PPPPPPPP/RNBQKBNR w KQkq - 0 1",
            "r3k2r/P6P/8/3pP3/8/8/p6p/R3K2R w KQkq d6 0 1",
            "k7/8/8/8/1Q1Q1Q1Q/Q1Q1Q1Q1/8/7K w - - 0 1",
        ] {
            let p = Position::from_fen(fen).unwrap();
            let legal = p.legal_moves();
            let mut order = MoveOrdering::default();
            for cached in [None, legal.first().copied(), legal.last().copied()] {
                let mut old = legal.clone();
                let mut new = legal.clone();
                crate::engine::order(&p, &mut old, cached);
                order.order(&p, &mut new, cached, 0);
                assert_eq!(new, old);
            }
            for &m in &legal {
                order.cutoff(&p, m, 0, 8, &[]);
            }
            let mut learned = legal.clone();
            order.order(&p, &mut learned, None, 0);
            let mut expected = legal;
            learned.sort_by_key(|m| (m.from, m.to, m.promotion));
            expected.sort_by_key(|m| (m.from, m.to, m.promotion));
            assert_eq!(learned, expected);
        }
    }
}
