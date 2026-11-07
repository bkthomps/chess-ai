//! Iterative-deepening negamax (minimax), alpha-beta pruning, and a Zobrist
//! transposition table. Search runs on a worker thread, away from the UI.
use crate::chess::{random_key, Move, Position};
use std::collections::HashMap;
use std::sync::{
    atomic::{AtomicBool, Ordering},
    Arc,
};
use std::time::{Duration, Instant};

const MATE: i32 = 30_000;
const INF: i32 = 32_000;
#[derive(Clone, Copy)]
enum Bound {
    Exact,
    Lower,
    Upper,
}
#[derive(Clone, Copy)]
struct Entry {
    depth: u8,
    score: i32,
    bound: Bound,
    best: Option<Move>,
}
#[derive(Clone, Debug)]
pub struct SearchResult {
    pub best: Option<Move>,
    pub score: i32,
    pub depth: u8,
    pub nodes: u64,
    pub hits: u64,
    pub elapsed: Duration,
}
struct Search {
    table: HashMap<u64, Entry>,
    keys: Vec<u64>,
    deadline: Instant,
    nodes: u64,
    hits: u64,
    aborted: bool,
    cancelled: Arc<AtomicBool>,
}

/// Allocate roughly 1/36 of the clock in the opening, increasing toward 1/18
/// as material comes off, plus 75% of the increment. Never spend over 15% of
/// usable time, and reserve up to 250ms for scheduling and move submission.
pub fn time_budget(position: &Position, remaining: Duration, increment: Duration) -> Duration {
    let phase: u32 = position
        .squares
        .iter()
        .map(|p| match p.abs() {
            2 | 3 => 1,
            4 => 2,
            5 => 4,
            _ => 0,
        })
        .sum();
    let moves_left = 18.0 + 18.0 * phase.min(24) as f64 / 24.0;
    let seconds = remaining.as_secs_f64();
    let usable = (seconds - (seconds * 0.1).min(0.25)).max(0.0);
    Duration::from_secs_f64(
        (usable / moves_left + increment.as_secs_f64() * 0.75).min(usable * 0.15),
    )
}

pub fn search(position: &Position, history: &[u64], budget: Duration) -> SearchResult {
    search_with_cancel(position, history, budget, Arc::new(AtomicBool::new(false)))
}

pub fn search_with_cancel(
    position: &Position,
    history: &[u64],
    budget: Duration,
    cancelled: Arc<AtomicBool>,
) -> SearchResult {
    let start = Instant::now();
    let mut s = Search {
        table: HashMap::new(),
        keys: history.to_vec(),
        deadline: start + budget,
        nodes: 0,
        hits: 0,
        aborted: false,
        cancelled,
    };
    if s.keys.last() != Some(&position.repetition_key()) {
        s.keys.push(position.repetition_key());
    }
    let legal = position.legal_moves();
    let mut result = SearchResult {
        best: legal.first().copied(),
        score: 0,
        depth: 0,
        nodes: 0,
        hits: 0,
        elapsed: Duration::ZERO,
    };
    if legal.is_empty() || s.is_draw(position) {
        result.best = None;
        return result;
    }
    if legal.len() == 1 {
        // There is no decision to search: save the clock for later moves.
        result.elapsed = start.elapsed();
        return result;
    }
    let mut depth = 1;
    while Instant::now() < s.deadline && !s.cancelled.load(Ordering::Relaxed) {
        let (score, best) = s.negamax(position, depth, 0, -INF, INF);
        if s.aborted {
            break;
        }
        result.best = best;
        result.score = score;
        result.depth = depth;
        if score.abs() > MATE - 256 {
            break;
        }
        // There is no depth target. Saturation protects the representation;
        // the deadline still controls termination at its maximum value.
        depth = depth.saturating_add(1);
    }
    result.nodes = s.nodes;
    result.hits = s.hits;
    result.elapsed = start.elapsed();
    result
}

impl Search {
    fn expired(&mut self) -> bool {
        self.nodes += 1;
        if self.cancelled.load(Ordering::Relaxed)
            || (self.nodes == 1 || self.nodes & 31 == 0) && Instant::now() >= self.deadline
        {
            self.aborted = true;
        }
        self.aborted
    }
    fn is_draw(&self, p: &Position) -> bool {
        p.halfmove >= 100 || p.insufficient_material() || {
            let key = p.repetition_key();
            self.keys
                .iter()
                .rev()
                .take(p.halfmove as usize + 1)
                .filter(|&&k| k == key)
                .count()
                >= 3
        }
    }
    // Include the reversible history and clock: identical boards with different
    // repetition/fifty-move contexts must not share cached scores.
    fn table_key(&self, p: &Position) -> u64 {
        let mut context = 0u64;
        for key in self.keys.iter().rev().take(p.halfmove as usize + 1) {
            context = context.wrapping_add(random_key(*key));
        }
        p.hash ^ context.rotate_left(17) ^ random_key(20_000 + p.halfmove as u64)
    }
    fn negamax(
        &mut self,
        p: &Position,
        depth: u8,
        ply: i32,
        mut alpha: i32,
        beta: i32,
    ) -> (i32, Option<Move>) {
        if self.expired() {
            return (0, None);
        }
        let mut moves = p.legal_moves();
        if moves.is_empty() {
            return (if p.in_check(p.white) { -MATE + ply } else { 0 }, None);
        }
        if self.is_draw(p) {
            return (0, None);
        }
        if depth == 0 {
            return (self.quiescence(p, moves, ply, 0, alpha, beta), None);
        }
        let key = self.table_key(p);
        let original_alpha = alpha;
        let entry = self.table.get(&key).copied();
        if let Some(e) = entry {
            if e.depth >= depth {
                let score = from_table(e.score, ply);
                if matches!(e.bound, Bound::Exact)
                    || matches!(e.bound, Bound::Lower) && score >= beta
                    || matches!(e.bound, Bound::Upper) && score <= alpha
                {
                    self.hits += 1;
                    return (score, e.best);
                }
            }
        }
        order(p, &mut moves, entry.and_then(|e| e.best));
        let mut best = None;
        let mut value = -INF;
        for m in moves {
            let next = p.play(m);
            self.keys.push(next.repetition_key());
            let score = -self.negamax(&next, depth - 1, ply + 1, -beta, -alpha).0;
            self.keys.pop();
            if self.aborted {
                return (0, None);
            }
            if score > value {
                value = score;
                best = Some(m);
            }
            alpha = alpha.max(score);
            if alpha >= beta {
                break;
            }
        }
        if self.table.len() < 200_000 {
            self.table.insert(
                key,
                Entry {
                    depth,
                    score: to_table(value, ply),
                    best,
                    bound: if value <= original_alpha {
                        Bound::Upper
                    } else if value >= beta {
                        Bound::Lower
                    } else {
                        Bound::Exact
                    },
                },
            );
        }
        (value, best)
    }
    fn quiescence(
        &mut self,
        p: &Position,
        mut moves: Vec<Move>,
        ply: i32,
        qply: u8,
        mut alpha: i32,
        beta: i32,
    ) -> i32 {
        if self.expired() {
            return 0;
        }
        if moves.is_empty() {
            return if p.in_check(p.white) { -MATE + ply } else { 0 };
        }
        if self.is_draw(p) {
            return 0;
        }
        let check = p.in_check(p.white);
        if qply >= 10 {
            return evaluate(p);
        }
        if !check {
            let stand = evaluate(p);
            if stand >= beta {
                return stand;
            }
            alpha = alpha.max(stand);
            moves.retain(|&m| p.is_capture(m) || m.promotion != 0);
        }
        order(p, &mut moves, None);
        for m in moves {
            let next = p.play(m);
            self.keys.push(next.repetition_key());
            let score =
                -self.quiescence(&next, next.legal_moves(), ply + 1, qply + 1, -beta, -alpha);
            self.keys.pop();
            if self.aborted {
                return 0;
            }
            if score >= beta {
                return score;
            }
            alpha = alpha.max(score);
        }
        alpha
    }
}
fn to_table(s: i32, ply: i32) -> i32 {
    if s > MATE - 256 {
        s + ply
    } else if s < -MATE + 256 {
        s - ply
    } else {
        s
    }
}
fn from_table(s: i32, ply: i32) -> i32 {
    if s > MATE - 256 {
        s - ply
    } else if s < -MATE + 256 {
        s + ply
    } else {
        s
    }
}
fn value(piece: i8) -> i32 {
    match piece.abs() {
        1 => 100,
        2 => 320,
        3 => 335,
        4 => 500,
        5 => 900,
        _ => 0,
    }
}
fn order(p: &Position, moves: &mut [Move], best: Option<Move>) {
    moves.sort_by_cached_key(|&m| {
        -(if Some(m) == best { 100_000 } else { 0 }
            + if p.is_capture(m) {
                10_000 + value(p.squares[m.to]).max(100) * 10 - value(p.squares[m.from])
            } else {
                0
            }
            + value(m.promotion))
    });
}
fn evaluate(p: &Position) -> i32 {
    let endgame = p
        .squares
        .iter()
        .filter(|&&pc| [4, 5].contains(&pc.abs()))
        .count()
        <= 2;
    let mut score = 0;
    let mut bishops = [0, 0];
    for (s, &pc) in p.squares.iter().enumerate() {
        if pc == 0 {
            continue;
        }
        let x = (s % 8) as i32;
        let y = if pc > 0 { s / 8 } else { 7 - s / 8 } as i32;
        let center = 6 - (2 * x - 7).abs() - (2 * y - 7).abs();
        let positional = match pc.abs() {
            1 => y * 8 + center * 2,
            2 => center * 10,
            3 => {
                bishops[usize::from(pc < 0)] += 1;
                center * 5
            }
            4 => {
                if y == 6 {
                    20
                } else {
                    0
                }
            }
            5 => center * 2,
            6 => {
                if endgame {
                    center * 8
                } else {
                    -center * 5 + if y == 0 && (x == 6 || x == 2) { 35 } else { 0 }
                }
            }
            _ => 0,
        };
        score += pc.signum() as i32 * (value(pc) + positional);
    }
    if bishops[0] >= 2 {
        score += 30;
    }
    if bishops[1] >= 2 {
        score -= 30;
    }
    if p.white {
        score
    } else {
        -score
    }
}
