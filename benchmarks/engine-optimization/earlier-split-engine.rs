//! Iterative-deepening negamax (minimax), alpha-beta pruning, and a Zobrist
//! transposition table. Root moves are searched in parallel, away from the UI.
use crate::chess::{random_key, Move, MoveState, Position};
use std::sync::{
    atomic::{AtomicBool, AtomicI32, AtomicU64, AtomicUsize, Ordering},
    mpsc, Arc, Mutex,
};
use std::thread;
use std::time::{Duration, Instant};

mod mobility {
    //! Pawn-safe pseudo-mobility, without move generation or heap scratch.
    #[cfg(test)]
    use crate::chess::Position;
    use crate::tables::DIRECTIONS;

    // Centipawns per destination, indexed by piece type: (middlegame, endgame).
    const WEIGHTS: [(i32, i32); 7] = [(0, 0), (0, 0), (4, 4), (5, 5), (2, 4), (1, 2), (0, 0)];
    const FILE_A: u64 = 0x0101_0101_0101_0101;
    const FILE_H: u64 = FILE_A << 7;

    struct Geometry {
        rays: [[u64; 8]; 64],
        knights: [u64; 64],
        kings: [u64; 64],
        lines: [[u64; 4]; 64],
    }
    const fn geometry() -> Geometry {
        let mut result = Geometry {
            rays: [[0; 8]; 64],
            knights: [0; 64],
            kings: [0; 64],
            lines: [[0; 4]; 64],
        };
        let steps: [(i32, i32); 8] = [
            (1, 2),
            (2, 1),
            (-1, 2),
            (-2, 1),
            (1, -2),
            (2, -1),
            (-1, -2),
            (-2, -1),
        ];
        let mut square = 0;
        while square < 64 {
            let (x, y) = (square as i32 % 8, square as i32 / 8);
            let mut ox = -1;
            while ox <= 1 {
                let mut oy = -1;
                while oy <= 1 {
                    let (nx, ny) = (x + ox, y + oy);
                    if (ox != 0 || oy != 0) && nx >= 0 && nx < 8 && ny >= 0 && ny < 8 {
                        result.kings[square] |= 1u64 << (ny * 8 + nx);
                    }
                    oy += 1;
                }
                ox += 1;
            }
            let mut d = 0;
            while d < 8 {
                let (dx, dy) = DIRECTIONS[d];
                let (mut nx, mut ny) = (x + dx, y + dy);
                while nx >= 0 && nx < 8 && ny >= 0 && ny < 8 {
                    result.rays[square][d] |= 1u64 << (ny * 8 + nx);
                    nx += dx;
                    ny += dy;
                }
                let (nx, ny) = (x + steps[d].0, y + steps[d].1);
                if nx >= 0 && nx < 8 && ny >= 0 && ny < 8 {
                    result.knights[square] |= 1u64 << (ny * 8 + nx);
                }
                d += 1;
            }
            let pairs = [(0, 1), (2, 3), (4, 7), (5, 6)];
            let mut line = 0;
            while line < 4 {
                let (a, b) = pairs[line];
                result.lines[square][line] =
                    result.rays[square][a] | result.rays[square][b] | (1u64 << square);
                line += 1;
            }
            square += 1;
        }
        result
    }
    static GEOMETRY: Geometry = geometry();

    fn nearest(bits: u64, direction: usize) -> usize {
        debug_assert_ne!(bits, 0);
        if matches!(direction, 0 | 2 | 4 | 5) {
            bits.trailing_zeros() as usize
        } else {
            63 - bits.leading_zeros() as usize
        }
    }

    // Subtraction borrows up to the first blocker in each direction. Reversing
    // the bits applies the same operation to the opposite half of the line.
    fn line_attacks(occupied: u64, from: usize, line: usize) -> u64 {
        let mask = GEOMETRY.lines[from][line];
        let blockers = occupied & mask;
        let piece = 1u64 << from;
        let forward = blockers.wrapping_sub(piece.wrapping_mul(2));
        let backward = blockers
            .reverse_bits()
            .wrapping_sub(piece.reverse_bits().wrapping_mul(2));
        (forward ^ backward.reverse_bits()) & mask
    }

    pub(super) fn pawn_attacks(pawns: [u64; 2]) -> [u64; 2] {
        [
            ((pawns[0] & !FILE_A) >> 9) | ((pawns[0] & !FILE_H) >> 7),
            ((pawns[1] & !FILE_A) << 7) | ((pawns[1] & !FILE_H) << 9),
        ]
    }

    pub(super) fn king_attacks(square: usize) -> u64 {
        GEOMETRY.kings[square]
    }

    pub(super) struct Mobility {
        occupied: u64,
        eligible: [u64; 2],
        pinned: [u64; 2],
        kings: [usize; 2],
    }

    impl Mobility {
        #[cfg(test)]
        pub(super) fn new(p: &Position) -> Self {
            let mut occupied = [0; 2];
            let mut pawns = [0; 2];
            let mut kings = [0; 2];
            let mut diagonal = [0; 2];
            let mut straight = [0; 2];
            for (s, &pc) in p.squares.iter().enumerate() {
                if pc == 0 {
                    continue;
                }
                let color = usize::from(pc > 0);
                occupied[color] |= 1u64 << s;
                match pc.abs() {
                    1 => pawns[color] |= 1u64 << s,
                    3 => diagonal[color] |= 1u64 << s,
                    4 => straight[color] |= 1u64 << s,
                    5 => {
                        diagonal[color] |= 1u64 << s;
                        straight[color] |= 1u64 << s;
                    }
                    6 => kings[color] = s,
                    _ => {}
                }
            }
            Self::from_masks(
                occupied,
                pawns,
                kings,
                diagonal,
                straight,
                pawn_attacks(pawns),
            )
        }

        pub(super) fn from_masks(
            occupied: [u64; 2],
            pawns: [u64; 2],
            kings: [usize; 2],
            diagonal: [u64; 2],
            straight: [u64; 2],
            pawn_attacks: [u64; 2],
        ) -> Self {
            let mut result = Self {
                occupied: occupied[0] | occupied[1],
                eligible: [
                    !(occupied[0] | pawn_attacks[1] | (1u64 << kings[1])),
                    !(occupied[1] | pawn_attacks[0] | (1u64 << kings[0])),
                ],
                pinned: [0; 2],
                kings,
            };
            for color in 0..2 {
                let active = occupied[color] & !(pawns[color] | (1u64 << kings[color]));
                if active == 0 {
                    continue;
                }
                for d in 0..8 {
                    let ray = GEOMETRY.rays[kings[color]][d];
                    let pinners = if d < 4 {
                        straight[1 - color]
                    } else {
                        diagonal[1 - color]
                    };
                    if ray & pinners == 0 {
                        continue;
                    }
                    let mut blockers = ray & result.occupied;
                    // The aligned enemy slider guarantees at least one blocker.
                    let first = nearest(blockers, d);
                    let first_bit = 1u64 << first;
                    if active & first_bit == 0 {
                        continue;
                    }
                    blockers &= !first_bit;
                    if blockers == 0 {
                        continue;
                    }
                    let second = nearest(blockers, d);
                    if pinners & (1u64 << second) != 0 {
                        result.pinned[color] |= first_bit;
                    }
                }
            }
            result
        }

        // Includes friendly blockers as defended squares. Mobility applies its
        // own destination exclusions after threats/king safety reuse this map.
        pub(super) fn attacks(&self, from: usize, pc: i8) -> u64 {
            let kind = pc.unsigned_abs() as usize;
            if !(2..=5).contains(&kind) {
                return 0;
            }
            let color = usize::from(pc > 0);
            let pinned = self.pinned[color] & (1u64 << from) != 0;
            if kind == 2 {
                return if pinned { 0 } else { GEOMETRY.knights[from] };
            }
            let lines = match kind {
                3 => 2..4,
                4 => 0..2,
                _ => 0..4,
            };
            let mut attacks = 0;
            for line in lines {
                attacks |= line_attacks(self.occupied, from, line);
            }
            if pinned {
                let king = self.kings[color];
                let dx = from as i32 % 8 - king as i32 % 8;
                let dy = from as i32 / 8 - king as i32 / 8;
                let (a, b) = if dx == 0 {
                    (2, 3)
                } else if dy == 0 {
                    (0, 1)
                } else if dx.signum() == dy.signum() {
                    (4, 7)
                } else {
                    (5, 6)
                };
                attacks &= GEOMETRY.rays[king][a] | GEOMETRY.rays[king][b] | (1u64 << king);
            }
            attacks
        }

        #[cfg(test)]
        pub(super) fn count(&self, _p: &Position, from: usize, pc: i8) -> i32 {
            if !(2..=5).contains(&pc.abs()) {
                return 0;
            }
            (self.attacks(from, pc) & self.eligible[usize::from(pc > 0)]).count_ones() as i32
        }

        pub(super) fn bonus_from_attacks(&self, pc: i8, attacks: u64) -> (i32, i32) {
            let count = (attacks & self.eligible[usize::from(pc > 0)]).count_ones() as i32
                * i32::from(pc.signum());
            let (mg, eg) = WEIGHTS[pc.unsigned_abs() as usize];
            (count * mg, count * eg)
        }
    }

    #[cfg(test)]
    pub(super) mod tests {
        use super::*;

        #[test]
        fn line_attacks_match_ray_walks_for_every_line_occupancy() {
            let pairs = [(0, 1), (2, 3), (4, 7), (5, 6)];
            for from in 0..64 {
                for (line, &(a, b)) in pairs.iter().enumerate() {
                    let squares: Vec<_> = crate::tables::RAYS[from][a]
                        .iter()
                        .chain(crate::tables::RAYS[from][b].iter())
                        .collect();
                    for subset in 0..(1usize << squares.len()) {
                        let mut occupied = 1u64 << from;
                        for (i, &square) in squares.iter().enumerate() {
                            if subset & (1 << i) != 0 {
                                occupied |= 1u64 << square;
                            }
                        }
                        let mut expected = 0;
                        for d in [a, b] {
                            for square in crate::tables::RAYS[from][d].iter() {
                                expected |= 1u64 << square;
                                if occupied & (1u64 << square) != 0 {
                                    break;
                                }
                            }
                        }
                        assert_eq!(
                            line_attacks(occupied, from, line),
                            expected,
                            "square {from}, line {line}, occupancy {occupied:#x}"
                        );
                    }
                }
            }
        }

        // Scalar oracle: enumerate all destination squares and walk paths with
        // coordinates, independently of the production attack tables and bit masks.
        pub(crate) fn reference_attacks(p: &Position, from: usize) -> u64 {
            let pc = p.squares[from];
            let kind = pc.abs();
            if !(1..=6).contains(&kind) {
                return 0;
            }
            let xy = |s: usize| (s as i32 % 8, s as i32 / 8);
            let (fx, fy) = xy(from);
            let king = p
                .squares
                .iter()
                .position(|&v| v == pc.signum() * 6)
                .unwrap();
            let (kx, ky) = xy(king);
            let mut pinned = false;
            for (s, &enemy) in p.squares.iter().enumerate() {
                let (x, y) = xy(s);
                let (dx, dy) = (x - kx, y - ky);
                let diagonal = dx.abs() == dy.abs();
                let straight = dx == 0 || dy == 0;
                if enemy.signum() != -pc.signum()
                    || !(enemy.abs() == 5 && (diagonal || straight)
                        || enemy.abs() == 3 && diagonal
                        || enemy.abs() == 4 && straight)
                    || (fx - kx) * dy != (fy - ky) * dx
                {
                    continue;
                }
                let (mut x, mut y) = (kx + dx.signum(), ky + dy.signum());
                let mut blockers = Vec::new();
                while (x, y) != xy(s) {
                    let at = (y * 8 + x) as usize;
                    if p.squares[at] != 0 {
                        blockers.push(at);
                    }
                    x += dx.signum();
                    y += dy.signum();
                }
                pinned |= blockers == [from];
            }
            let mut attacks = 0;
            for to in 0..64 {
                let (tx, ty) = xy(to);
                let (dx, dy) = (tx - fx, ty - fy);
                if to == from {
                    continue;
                }
                let geometry = match kind {
                    1 => dx.abs() == 1 && dy == i32::from(pc.signum()),
                    6 => dx.abs().max(dy.abs()) == 1,
                    2 => dx.abs() * dy.abs() == 2,
                    3 => dx.abs() == dy.abs(),
                    4 => dx == 0 || dy == 0,
                    5 => dx == 0 || dy == 0 || dx.abs() == dy.abs(),
                    _ => false,
                };
                if !geometry
                    || (2..=5).contains(&kind)
                        && pinned
                        && (kind == 2 || (tx - kx) * (fy - ky) != (ty - ky) * (fx - kx))
                {
                    continue;
                }
                if (3..=5).contains(&kind) {
                    let (mut x, mut y) = (fx + dx.signum(), fy + dy.signum());
                    let mut blocked = false;
                    while (x, y) != (tx, ty) {
                        blocked |= p.squares[(y * 8 + x) as usize] != 0;
                        x += dx.signum();
                        y += dy.signum();
                    }
                    if blocked {
                        continue;
                    }
                }
                attacks |= 1u64 << to;
            }
            attacks
        }

        pub(crate) fn reference_count(p: &Position, from: usize) -> i32 {
            let pc = p.squares[from];
            if !(2..=5).contains(&pc.abs()) {
                return 0;
            }
            let attacks = reference_attacks(p, from);
            (0..64)
                .filter(|&to| {
                    if attacks & (1u64 << to) == 0
                        || p.squares[to].signum() == pc.signum()
                        || p.squares[to].abs() == 6
                    {
                        return false;
                    }
                    !p.squares.iter().enumerate().any(|(s, &enemy)| {
                        enemy == -pc.signum()
                            && (s as i32 % 8 - to as i32 % 8).abs() == 1
                            && s as i32 / 8 - i32::from(pc.signum()) == to as i32 / 8
                    })
                })
                .count() as i32
        }

        pub(crate) fn reference_scores(p: &Position) -> (i32, i32) {
            let mut result = (0, 0);
            for (from, &pc) in p.squares.iter().enumerate() {
                let (mg, eg) = match pc.abs() {
                    2 => (4, 4),
                    3 => (5, 5),
                    4 => (2, 4),
                    5 => (1, 2),
                    _ => continue,
                };
                let n = reference_count(p, from) * i32::from(pc.signum());
                result.0 += n * mg;
                result.1 += n * eg;
            }
            result
        }

        #[test]
        fn pawn_pushes_free_bishops_and_queens_before_they_move() {
            let p = Position::default();
            let before = Mobility::new(&p);
            let e4 = p.play(
                p.legal_moves()
                    .into_iter()
                    .find(|m| m.to_string() == "e2e4")
                    .unwrap(),
            );
            let after = Mobility::new(&e4);
            assert_eq!(before.count(&p, 5, 3), 0);
            assert_eq!(after.count(&e4, 5, 3), 4);
            assert_eq!(before.count(&p, 3, 5), 0);
            assert_eq!(after.count(&e4, 3, 5), 4);
            let d4 = p.play(
                p.legal_moves()
                    .into_iter()
                    .find(|m| m.to_string() == "d2d4")
                    .unwrap(),
            );
            assert_eq!(Mobility::new(&d4).count(&d4, 2, 3), 4);
        }

        #[test]
        fn mobility_respects_pins_pawn_attacks_and_ray_blockers() {
            for (fen, from, expected) in [
                ("4r1k1/8/8/8/8/8/4N3/4K3 w - - 0 1", 12, 0),
                ("4r1k1/8/8/8/8/8/4R3/4K3 w - - 0 1", 12, 6),
                ("7k/8/7b/8/8/4B3/8/2K5 w - - 0 1", 20, 4),
                // d5 is defended; its pawn also excludes c4/e4. No ray passes d5.
                ("7k/8/4p3/3p4/3R4/8/8/K7 w - - 0 1", 27, 8),
            ] {
                let p = Position::from_fen(fen).unwrap();
                let m = Mobility::new(&p);
                assert_eq!(m.count(&p, from, p.squares[from]), expected, "{fen}");
                for (s, &pc) in p.squares.iter().enumerate() {
                    assert_eq!(m.count(&p, s, pc), reference_count(&p, s), "{fen} / {s}");
                }
            }
        }
    }
}

const MATE: i32 = 30_000;
const INF: i32 = 32_000;
#[cfg(test)]
const TABLE_SIZE: usize = 2_000_000;
pub const DEFAULT_MEMORY_LIMIT: usize = 100_000_000;
pub const MAX_MEMORY_LIMIT: usize = 1_000_000_000;
const MAX_SEARCH_THREADS: usize = 64;
const BUCKET_SIZE: usize = 4;
const GENERATION_MASK: u8 = 63;
const STALE_AGE: u8 = 32;
const EXACT_BONUS: i16 = 2;
const AGE_PENALTY: i16 = 4;
// Retained scratch is capped independently of move count or search depth.
const MOVE_BUFFER_LIMIT: usize = 512 * 1024;
const BUFFER_PLIES: usize = 256 + 10;
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
#[repr(u8)]
enum Bound {
    Exact,
    Lower,
    Upper,
}
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
struct Entry {
    depth: u8,
    score: i32,
    bound: Bound,
    best: Option<Move>,
}

/// Six bytes: a lossless score, packed move, depth, and a byte containing
/// two bound/occupancy bits plus a six-bit search generation.
#[derive(Clone, Copy, Default)]
#[repr(C)]
struct PackedEntry {
    score: i16,
    best: u16,
    depth: u8,
    flags: u8,
}
impl PackedEntry {
    fn pack(entry: Entry, generation: u8) -> Option<Self> {
        let best = entry.best.map_or(0, |m| {
            debug_assert!(m.from < 64 && m.to < 64 && m.from != m.to);
            debug_assert!([0, 2, 3, 4, 5].contains(&m.promotion));
            // Six bits per square and three for promotion. Zero represents
            // no move: a1-a1 without promotion is never a legal chess move.
            m.from as u16 | (m.to as u16) << 6 | (m.promotion as u16) << 12
        });
        Some(Self {
            // Unusual supplied positions can evaluate outside i16. Skip
            // caching those scores instead of truncating or clamping them.
            score: i16::try_from(entry.score).ok()?,
            best,
            depth: entry.depth,
            flags: (entry.bound as u8 + 1) | (generation & GENERATION_MASK) << 2,
        })
    }
    fn unpack(self) -> Option<Entry> {
        let bound = match self.flags & 3 {
            1 => Bound::Exact,
            2 => Bound::Lower,
            3 => Bound::Upper,
            _ => return None,
        };
        Some(Entry {
            score: i32::from(self.score),
            best: (self.best != 0).then_some(Move {
                from: (self.best & 63) as usize,
                to: ((self.best >> 6) & 63) as usize,
                promotion: ((self.best >> 12) & 7) as i8,
            }),
            depth: self.depth,
            bound,
        })
    }
    fn is_empty(self) -> bool {
        self.flags & 3 == 0
    }
    fn refresh(&mut self, generation: u8) {
        self.flags = self.flags & 3 | (generation & GENERATION_MASK) << 2;
    }
    fn age(self, generation: u8) -> u8 {
        generation.wrapping_sub(self.flags >> 2) & GENERATION_MASK
    }
    fn retention(self, generation: u8) -> i16 {
        let exact_bonus = if self.flags & 3 == 1 { EXACT_BONUS } else { 0 };
        i16::from(self.depth) + exact_bonus - i16::from(self.age(generation)) * AGE_PENALTY
    }
    fn improves(self, old: Self) -> bool {
        if self.depth != old.depth {
            return self.depth > old.depth;
        }
        match (self.flags & 3, old.flags & 3) {
            (1, _) => true,
            (_, 1) => false,
            (2, 2) => self.score >= old.score,
            (3, 3) => self.score <= old.score,
            _ => true,
        }
    }
}

#[path = "cache.rs"]
mod cache;
use cache::SharedTable;
#[cfg(test)]
use cache::TestTable as Table;
#[path = "ordering.rs"]
mod ordering;
use ordering::{MoveOrdering, ORDERING_BYTES};

/// Keep at least one logical CPU, and roughly a quarter on larger machines,
/// available to the browser and other applications. No OS-specific policy.
fn search_threads(cpus: usize) -> usize {
    cpus.saturating_sub(cpus.saturating_add(3) / 4)
        .clamp(1, MAX_SEARCH_THREADS)
}

#[derive(Default)]
struct WorkerStats {
    nodes: AtomicU64,
    hits: AtomicU64,
}

struct RootWork<'a> {
    position: &'a Position,
    moves: &'a [Move],
    depth: u8,
    ply: i32,
    next: AtomicUsize,
    alpha: AtomicI32,
    aborted: AtomicBool,
    scores: Vec<Mutex<Option<i32>>>,
}

/// Owned by the search worker and retained across pondering and computer moves.
/// Four-way buckets retain deeper, exact, and recently used results.
pub struct SearchCache {
    table: SharedTable,
    buffers: MoveBuffers,
    helpers: Vec<MoveBuffers>,
    threads: usize,
    single_cpu: bool,
}
impl Default for SearchCache {
    fn default() -> Self {
        Self::with_threads(search_threads(
            thread::available_parallelism().map_or(1, usize::from),
        ))
    }
}

#[derive(Default)]
struct MoveBuffers {
    slots: Vec<Vec<Move>>,
    retained_bytes: usize,
    ordering: MoveOrdering,
    evaluation: EvaluationCache,
}
impl MoveBuffers {
    fn take(&mut self, ply: usize) -> Vec<Move> {
        if ply >= BUFFER_PLIES {
            return Vec::new();
        }
        if self.slots.len() <= ply {
            self.slots.resize_with(ply + 1, Vec::new);
        }
        let mut moves = std::mem::take(&mut self.slots[ply]);
        self.retained_bytes -= moves.capacity() * std::mem::size_of::<Move>();
        moves.clear();
        moves
    }
    fn put(&mut self, ply: usize, mut moves: Vec<Move>) {
        moves.clear();
        let bytes = moves.capacity() * std::mem::size_of::<Move>();
        if ply < self.slots.len()
            && self.retained_bytes + bytes
                <= MOVE_BUFFER_LIMIT - ORDERING_BYTES - EVALUATION_CACHE_BYTES
        {
            self.retained_bytes += bytes;
            self.slots[ply] = moves;
        }
    }
}
#[derive(Clone, Debug)]
pub struct SearchResult {
    pub best: Option<Move>,
    pub score: i32,
    pub depth: u8,
    pub nodes: u64,
    pub hits: u64,
    pub elapsed: Duration,
    pub threads: usize,
    pub cache_bytes: usize,
    pub cache_limit: usize,
}
/// A coherent snapshot of one search, including an unfinished depth.
#[derive(Clone, Copy, Debug, Default)]
pub struct SearchProgress {
    pub evaluation: Option<i32>,
    pub depth: u8,
    pub searching_depth: u8,
    pub nodes: u64,
    pub hits: u64,
    pub elapsed: Duration,
    pub threads: usize,
    pub cache_bytes: usize,
    pub cache_limit: usize,
}
struct Search<'a> {
    table: &'a SharedTable,
    buffers: &'a mut MoveBuffers,
    keys: Vec<u64>,
    history_context: u64,
    deadline: Option<Instant>,
    nodes: u64,
    hits: u64,
    aborted: bool,
    cancelled: Arc<AtomicBool>,
    progress: Option<&'a Mutex<SearchProgress>>,
    started: Instant,
    published: Instant,
    depth: u8,
    searching_depth: u8,
    evaluation: Option<i32>,
    threads: usize,
    peers: &'a [WorkerStats],
    reporting: Option<&'a WorkerStats>,
    reported: (u64, u64),
    rest_at: Option<Instant>,
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
    SearchCache::default().search(position, history, budget, cancelled)
}

impl SearchCache {
    /// Choose an explicit search-thread limit (including the calling worker).
    /// Default reserves CPU capacity automatically; the RAM budget can lower
    /// the limit further to preserve at least 32 MiB for the cache.
    pub fn with_threads(threads: usize) -> Self {
        let thread_budget = (DEFAULT_MEMORY_LIMIT
            - 16 * 1024 * 1024
            - DEFAULT_MEMORY_LIMIT / 50
            - 32 * 1024 * 1024)
            / (2 * 1024 * 1024 + MOVE_BUFFER_LIMIT);
        Self::with_threads_and_memory(threads.min(thread_budget.max(1)), DEFAULT_MEMORY_LIMIT)
    }

    /// Bound the engine's total RAM budget, including conservative allowance for
    /// thread stacks, scratch, server state, and allocator overhead. Bytes are decimal.
    pub fn with_threads_and_memory(threads: usize, memory_bytes: usize) -> Self {
        let threads = threads.clamp(1, MAX_SEARCH_THREADS);
        let memory_bytes = memory_bytes.min(MAX_MEMORY_LIMIT);
        let reserve = 16 * 1024 * 1024 + threads * (2 * 1024 * 1024 + MOVE_BUFFER_LIMIT);
        // Freed pages can remain resident across new games and undos. Grow
        // allocator headroom smoothly for large budgets, reaching ~17% at 1 GB.
        let allocator_reserve =
            (memory_bytes / 50).max(memory_bytes.saturating_sub(500_000_000) / 3);
        let cache_limit = memory_bytes
            .saturating_sub(reserve)
            .saturating_sub(allocator_reserve);
        Self {
            table: SharedTable::with_limit(cache_limit),
            buffers: MoveBuffers::default(),
            helpers: Vec::new(),
            threads: threads.max(1),
            single_cpu: thread::available_parallelism().map_or(true, |n| n.get() == 1),
        }
    }

    pub fn search(
        &mut self,
        position: &Position,
        history: &[u64],
        budget: Duration,
        cancelled: Arc<AtomicBool>,
    ) -> SearchResult {
        self.run(position, history, Some(budget), cancelled, None)
    }

    pub fn search_with_progress(
        &mut self,
        position: &Position,
        history: &[u64],
        budget: Duration,
        cancelled: Arc<AtomicBool>,
        progress: &Mutex<SearchProgress>,
    ) -> SearchResult {
        self.run(position, history, Some(budget), cancelled, Some(progress))
    }

    /// Explore the human's possible moves and the computer's replies until
    /// cancelled. The caller must never play the resulting human-side move.
    pub fn ponder(
        &mut self,
        position: &Position,
        history: &[u64],
        cancelled: Arc<AtomicBool>,
    ) -> SearchResult {
        self.run(position, history, None, cancelled, None)
    }

    pub fn ponder_with_progress(
        &mut self,
        position: &Position,
        history: &[u64],
        cancelled: Arc<AtomicBool>,
        progress: &Mutex<SearchProgress>,
    ) -> SearchResult {
        self.run(position, history, None, cancelled, Some(progress))
    }

    fn run(
        &mut self,
        position: &Position,
        history: &[u64],
        budget: Option<Duration>,
        cancelled: Arc<AtomicBool>,
        progress: Option<&Mutex<SearchProgress>>,
    ) -> SearchResult {
        let start = Instant::now();
        let legal = position.legal_moves();
        let threads = self.threads.min(legal.len()).max(1);
        self.helpers.resize_with(threads - 1, MoveBuffers::default);
        let peers: Vec<_> = (1..threads).map(|_| WorkerStats::default()).collect();
        self.table.initialize();
        self.table.next_generation();
        self.buffers.ordering.begin_search();
        for helper in &mut self.helpers {
            helper.ordering.begin_search();
        }
        let mut s = Search {
            table: &self.table,
            buffers: &mut self.buffers,
            keys: history.to_vec(),
            history_context: 0,
            deadline: budget.map(|budget| start + budget),
            nodes: 0,
            hits: 0,
            aborted: false,
            cancelled,
            progress,
            started: start,
            published: start,
            depth: 0,
            searching_depth: 0,
            evaluation: None,
            threads,
            peers: &peers,
            reporting: None,
            reported: (0, 0),
            rest_at: self.single_cpu.then(|| start + Duration::from_millis(9)),
        };
        if s.keys.last() != Some(&position.repetition_key()) {
            s.keys.push(position.repetition_key());
        }
        s.history_context = reversible_context(position, &s.keys);
        let mut result = SearchResult {
            best: legal.first().copied(),
            score: 0,
            depth: 0,
            nodes: 0,
            hits: 0,
            elapsed: Duration::ZERO,
            threads,
            cache_bytes: s.table.bytes(),
            cache_limit: s.table.limit(),
        };
        if legal.is_empty() || s.is_draw(position) {
            result.best = None;
            s.publish(true);
            return result;
        }
        if legal.len() == 1 && budget.is_some() {
            // There is no decision to search: save the clock for later moves.
            result.elapsed = start.elapsed();
            s.publish(true);
            return result;
        }
        // A pondered child can now be the root. Only exact scores establish a
        // completed depth; bounds still help move ordering and alpha-beta cutoffs.
        if let Some(entry) = s.probe(s.table_key(position)) {
            if matches!(entry.bound, Bound::Exact) && entry.best.is_some_and(|m| legal.contains(&m))
            {
                result.best = entry.best;
                result.score = from_table(entry.score, 0);
                result.depth = entry.depth;
                s.hits += 1;
            }
        }
        s.depth = result.depth;
        s.evaluation = (result.depth > 0).then_some(if position.white {
            result.score
        } else {
            -result.score
        });
        s.searching_depth = result.depth.saturating_add(1);
        s.publish(true);
        let mut depth = result.depth.saturating_add(1);
        while !s.cancelled.load(Ordering::Relaxed)
            && s.deadline
                .map_or(true, |deadline| Instant::now() < deadline)
        {
            if result.score.abs() > MATE - 256 {
                break;
            }
            s.searching_depth = depth;
            let (score, best) = if threads > 1 && depth >= 4 {
                s.parallel_root(position, depth, &legal, &mut self.helpers)
            } else {
                s.negamax(position, depth, 0, -INF, INF)
            };
            if s.aborted {
                break;
            }
            result.best = best;
            result.score = score;
            result.depth = depth;
            s.depth = depth;
            s.evaluation = Some(if position.white { score } else { -score });
            s.searching_depth = depth.saturating_add(1);
            s.publish(false);
            if score.abs() > MATE - 256 {
                break;
            }
            // There is no depth target. Saturation protects the representation;
            // the deadline still controls termination at its maximum value.
            depth = depth.saturating_add(1);
        }
        (result.nodes, result.hits) = s.totals();
        result.elapsed = start.elapsed();
        result.cache_bytes = s.table.bytes();
        s.searching_depth = 0;
        s.publish(true);
        result
    }
}

impl Search<'_> {
    /// Search the principal move first to establish a useful alpha bound, then
    /// let workers claim the remaining root moves. Null-window probes avoid
    /// fully exploring moves that cannot improve the current best result.
    fn parallel_root(
        &mut self,
        position: &Position,
        depth: u8,
        legal: &[Move],
        helpers: &mut [MoveBuffers],
    ) -> (i32, Option<Move>) {
        if self.expired() {
            return (0, None);
        }
        self.parallel_moves(position, depth, 0, legal, helpers)
    }

    /// Use the same workers while the leading root branch is being resolved.
    /// Phases finish before their parent starts workers, so splitting never
    /// multiplies the configured thread count or retained thread buffers.
    fn parallel_first(
        &mut self,
        position: &Position,
        depth: u8,
        ply: i32,
        helpers: &mut [MoveBuffers],
    ) -> (i32, Option<Move>) {
        if self.expired() {
            return (0, None);
        }
        if let Some(entry) = self.probe(self.table_key(position)) {
            if entry.depth >= depth && matches!(entry.bound, Bound::Exact) {
                self.hits += 1;
                return (from_table(entry.score, ply), entry.best);
            }
        }
        let mut moves = self.buffers.take(ply as usize);
        let state = position.legal_moves_into(&mut moves);
        let result = if !state.has_legal {
            (if state.in_check { -MATE + ply } else { 0 }, None)
        } else if self.is_draw(position) {
            (0, None)
        } else {
            self.parallel_moves(position, depth, ply, &moves, helpers)
        };
        self.buffers.put(ply as usize, moves);
        result
    }

    fn parallel_moves(
        &mut self,
        position: &Position,
        depth: u8,
        ply: i32,
        legal: &[Move],
        helpers: &mut [MoveBuffers],
    ) -> (i32, Option<Move>) {
        let key = self.table_key(position);
        let mut moves = legal.to_vec();
        let cached = self.probe(key).and_then(|entry| entry.best);
        self.buffers
            .ordering
            .order(position, &mut moves, cached, ply as usize);
        let first = position.play(moves[0]);
        let previous = self.push_position(&first);
        let first_score = if ply < 2 && depth >= 5 && !helpers.is_empty() {
            -self.parallel_first(&first, depth - 1, ply + 1, helpers).0
        } else {
            -self.negamax(&first, depth - 1, ply + 1, -INF, INF).0
        };
        self.pop_position(previous);
        if self.aborted {
            return (0, None);
        }
        let work = RootWork {
            position,
            moves: &moves,
            depth,
            ply,
            next: AtomicUsize::new(1),
            alpha: AtomicI32::new(first_score),
            aborted: AtomicBool::new(false),
            scores: (0..moves.len()).map(|_| Mutex::new(None)).collect(),
        };
        thread::scope(|scope| {
            let mut handles = Vec::new();
            let (done_tx, done_rx) = mpsc::channel();
            for (index, (buffers, stats)) in helpers
                .iter_mut()
                .zip(self.peers)
                .take(moves.len().saturating_sub(1))
                .enumerate()
            {
                let mut worker = Search {
                    table: self.table,
                    buffers,
                    keys: self.keys.clone(),
                    history_context: self.history_context,
                    deadline: self.deadline,
                    nodes: 0,
                    hits: 0,
                    aborted: false,
                    cancelled: self.cancelled.clone(),
                    progress: None,
                    started: self.started,
                    published: self.published,
                    depth: self.depth,
                    searching_depth: depth,
                    evaluation: None,
                    threads: self.threads,
                    peers: &[],
                    reporting: Some(stats),
                    reported: (0, 0),
                    rest_at: None,
                };
                let work = &work;
                let done = done_tx.clone();
                // If the OS cannot create a helper, the remaining workers
                // still finish every root move through the shared work queue.
                if let Ok(handle) = thread::Builder::new()
                    .name(format!("chess-search-{}", index + 1))
                    .spawn_scoped(scope, move || {
                        worker.root_jobs(work);
                        worker.flush_stats();
                        let _ = done.send(());
                    })
                {
                    handles.push(handle);
                }
            }
            drop(done_tx);
            self.root_jobs(&work);
            for _ in 0..handles.len() {
                loop {
                    match done_rx.recv_timeout(Duration::from_millis(10)) {
                        Ok(()) | Err(mpsc::RecvTimeoutError::Disconnected) => break,
                        Err(mpsc::RecvTimeoutError::Timeout) => self.publish(false),
                    }
                }
            }
            for handle in handles {
                handle.join().expect("search helper panicked");
            }
        });
        if work.aborted.load(Ordering::Relaxed) {
            self.aborted = true;
            return (0, None);
        }
        let (mut score, mut best) = (first_score, moves[0]);
        for (index, result) in work.scores.iter().enumerate().skip(1) {
            let value = result
                .lock()
                .expect("root score poisoned")
                .expect("unfinished root move");
            if value > score {
                score = value;
                best = moves[index];
            }
        }
        self.table.store(
            key,
            Entry {
                depth,
                score: to_table(score, ply),
                best: Some(best),
                bound: Bound::Exact,
            },
        );
        (score, Some(best))
    }

    fn root_jobs(&mut self, work: &RootWork<'_>) {
        while !work.aborted.load(Ordering::Relaxed) {
            let index = work.next.fetch_add(1, Ordering::Relaxed);
            let Some(&m) = work.moves.get(index) else {
                break;
            };
            let next = work.position.play(m);
            let previous = self.push_position(&next);
            let alpha = work.alpha.load(Ordering::Relaxed);
            let mut score = -self
                .negamax(&next, work.depth - 1, work.ply + 1, -alpha - 1, -alpha)
                .0;
            if !self.aborted && score > alpha {
                score = -self
                    .negamax(&next, work.depth - 1, work.ply + 1, -INF, -alpha)
                    .0;
            }
            self.pop_position(previous);
            if self.aborted {
                work.aborted.store(true, Ordering::Relaxed);
                break;
            }
            work.alpha.fetch_max(score, Ordering::Relaxed);
            // A failed null-window probe supplies only an upper bound. It
            // must never replace a proven best move, even on an equal score.
            *work.scores[index].lock().expect("root score poisoned") =
                Some(if score > alpha { score } else { -INF });
        }
    }

    fn totals(&self) -> (u64, u64) {
        self.peers
            .iter()
            .fold((self.nodes, self.hits), |(nodes, hits), peer| {
                (
                    nodes + peer.nodes.load(Ordering::Relaxed),
                    hits + peer.hits.load(Ordering::Relaxed),
                )
            })
    }
    fn flush_stats(&mut self) {
        if let Some(stats) = self.reporting {
            stats
                .nodes
                .fetch_add(self.nodes - self.reported.0, Ordering::Relaxed);
            stats
                .hits
                .fetch_add(self.hits - self.reported.1, Ordering::Relaxed);
            self.reported = (self.nodes, self.hits);
        }
    }
    fn publish(&mut self, force: bool) {
        if let Some(progress) = self.progress {
            let now = Instant::now();
            if force || now.duration_since(self.published) >= Duration::from_millis(150) {
                // Never wait for a reader, and never take the game's mutex.
                if let Ok(mut snapshot) = progress.try_lock() {
                    let (nodes, hits) = self.totals();
                    *snapshot = SearchProgress {
                        evaluation: self.evaluation,
                        depth: self.depth,
                        searching_depth: self.searching_depth,
                        nodes,
                        hits,
                        elapsed: now.duration_since(self.started),
                        threads: self.threads,
                        cache_bytes: self.table.bytes(),
                        cache_limit: self.table.limit(),
                    };
                    self.published = now;
                }
            }
        }
    }
    fn probe(&mut self, key: u64) -> Option<Entry> {
        self.table.probe(key)
    }
    fn expired(&mut self) -> bool {
        self.nodes += 1;
        if self.cancelled.load(Ordering::Relaxed)
            || (self.nodes == 1 || self.nodes & 31 == 0)
                && self
                    .deadline
                    .is_some_and(|deadline| Instant::now() >= deadline)
        {
            self.aborted = true;
        }
        // Only sample publication time every 1024 nodes. Most nodes perform
        // no extra clock read, synchronization, or allocation for live status.
        if self.nodes & 1023 == 0 {
            if let Some(rest_at) = self.rest_at {
                if Instant::now() >= rest_at && !self.aborted {
                    thread::sleep(Duration::from_millis(1));
                    self.rest_at = Some(Instant::now() + Duration::from_millis(9));
                }
            }
            self.flush_stats();
            self.publish(false);
        }
        self.aborted
    }
    fn is_draw(&self, p: &Position) -> bool {
        p.halfmove >= 100 || p.insufficient_material() || {
            let key = *self.keys.last().expect("current position key");
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
        p.hash ^ self.history_context.rotate_left(17) ^ random_key(20_000 + p.halfmove as u64)
    }
    fn push_position(&mut self, p: &Position) -> u64 {
        let previous = self.history_context;
        let key = p.repetition_key();
        let context = random_key(key);
        self.history_context = if p.halfmove == 0 {
            context
        } else {
            previous.wrapping_add(context)
        };
        self.keys.push(key);
        previous
    }
    fn pop_position(&mut self, previous: u64) {
        self.keys.pop();
        self.history_context = previous;
    }
    fn negamax(
        &mut self,
        p: &Position,
        depth: u8,
        ply: i32,
        alpha: i32,
        beta: i32,
    ) -> (i32, Option<Move>) {
        if self.expired() {
            return (0, None);
        }
        // Cache keys include draw history and the halfmove clock, and entries
        // are stored only for nonterminal positions. Check proven results
        // before spending time generating the same moves again.
        let (key, entry) = if depth > 0 {
            let key = self.table_key(p);
            let entry = self.probe(key);
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
            (key, entry)
        } else {
            (0, None)
        };
        let mut moves = self.buffers.take(ply as usize);
        let state = if depth == 0 {
            p.tactical_moves_into(&mut moves)
        } else {
            p.legal_moves_into(&mut moves)
        };
        let result = self.negamax_moves(
            p,
            depth,
            ply,
            (alpha, beta),
            &mut moves,
            (key, entry, state),
        );
        self.buffers.put(ply as usize, moves);
        result
    }
    fn negamax_moves(
        &mut self,
        p: &Position,
        depth: u8,
        ply: i32,
        window: (i32, i32),
        moves: &mut [Move],
        cached: (u64, Option<Entry>, MoveState),
    ) -> (i32, Option<Move>) {
        let (mut alpha, beta) = window;
        let (key, entry, state) = cached;
        if !state.has_legal {
            return (if state.in_check { -MATE + ply } else { 0 }, None);
        }
        if self.is_draw(p) {
            return (0, None);
        }
        if depth == 0 {
            return (
                self.quiescence_moves(p, moves, state, ply, 0, (alpha, beta)),
                None,
            );
        }
        let original_alpha = alpha;
        self.buffers
            .ordering
            .order(p, moves, entry.and_then(|e| e.best), ply as usize);
        let mut best = None;
        let mut value = -INF;
        for (index, &m) in moves.iter().enumerate() {
            let next = p.play(m);
            let previous = self.push_position(&next);
            // The first move establishes a score with the full window. Later
            // moves first need only prove whether they can improve alpha.
            // A probe between alpha and beta needs full-window verification;
            // a fail-high already proves a cutoff. Null-window nodes already
            // have the narrowest useful window, so do not probe twice there.
            let score = if index == 0 || beta - alpha == 1 {
                -self.negamax(&next, depth - 1, ply + 1, -beta, -alpha).0
            } else {
                let probe = -self
                    .negamax(&next, depth - 1, ply + 1, -alpha - 1, -alpha)
                    .0;
                if !self.aborted && probe > alpha && probe < beta {
                    -self.negamax(&next, depth - 1, ply + 1, -beta, -alpha).0
                } else {
                    probe
                }
            };
            self.pop_position(previous);
            if self.aborted {
                return (0, None);
            }
            if score > value {
                value = score;
                best = Some(m);
            }
            alpha = alpha.max(score);
            if alpha >= beta {
                self.buffers
                    .ordering
                    .cutoff(p, m, ply as usize, depth, &moves[..index]);
                break;
            }
        }
        self.table.store(
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
        (value, best)
    }
    fn quiescence(&mut self, p: &Position, ply: i32, qply: u8, alpha: i32, beta: i32) -> i32 {
        let mut moves = self.buffers.take(ply as usize);
        let state = p.tactical_moves_into(&mut moves);
        let result = self.quiescence_moves(p, &mut moves, state, ply, qply, (alpha, beta));
        self.buffers.put(ply as usize, moves);
        result
    }
    fn quiescence_moves(
        &mut self,
        p: &Position,
        moves: &mut [Move],
        state: MoveState,
        ply: i32,
        qply: u8,
        window: (i32, i32),
    ) -> i32 {
        let (mut alpha, beta) = window;
        if self.expired() {
            return 0;
        }
        if !state.has_legal {
            return if state.in_check { -MATE + ply } else { 0 };
        }
        // At qply zero negamax has already checked this exact position.
        if qply != 0 && self.is_draw(p) {
            return 0;
        }
        if qply >= 10 {
            return self.buffers.evaluation.get(p);
        }
        if !state.in_check {
            let stand = self.buffers.evaluation.get(p);
            if stand >= beta {
                return stand;
            }
            alpha = alpha.max(stand);
        }
        if state.in_check {
            self.buffers.ordering.order(p, moves, None, ply as usize);
        } else {
            order(p, moves, None);
        }
        for &m in moves.iter() {
            let next = p.play(m);
            let previous = self.push_position(&next);
            let score = -self.quiescence(&next, ply + 1, qply + 1, -beta, -alpha);
            self.pop_position(previous);
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
fn reversible_context(p: &Position, keys: &[u64]) -> u64 {
    keys.iter()
        .rev()
        .take(p.halfmove as usize + 1)
        .fold(0u64, |context, &key| context.wrapping_add(random_key(key)))
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
const fn value(piece: i8) -> i32 {
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
    let priority = |m: Move| {
        (if Some(m) == best { 100_000 } else { 0 }
            + if p.is_capture(m) {
                10_000 + value(p.squares[m.to]).max(100) * 10 - value(p.squares[m.from])
            } else {
                0
            }
            + value(m.promotion))
    };
    sort_moves(moves, priority);
}
fn sort_moves(moves: &mut [Move], priority: impl Fn(Move) -> i32) {
    // Equal priorities retain their generated order without heap scratch.
    let mut priorities = [0; 64];
    if moves.len() <= priorities.len() {
        for (score, &m) in priorities.iter_mut().zip(moves.iter()) {
            *score = priority(m);
        }
        for index in 1..moves.len() {
            let m = moves[index];
            let score = priorities[index];
            let mut target = index;
            while target > 0 && priorities[target - 1] < score {
                moves[target] = moves[target - 1];
                priorities[target] = priorities[target - 1];
                target -= 1;
            }
            moves[target] = m;
            priorities[target] = score;
        }
        return;
    }
    // Unusually large move lists still have no size limit or heap scratch.
    for index in 1..moves.len() {
        let m = moves[index];
        let score = priority(m);
        let mut target = index;
        while target > 0 && priority(moves[target - 1]) < score {
            moves[target] = moves[target - 1];
            target -= 1;
        }
        moves[target] = m;
    }
}
const fn square_score(pc: i8, s: usize, endgame: bool) -> i16 {
    if pc == 0 {
        return 0;
    }
    let x = (s % 8) as i32;
    let y = if pc > 0 { s / 8 } else { 7 - s / 8 } as i32;
    let center = 6 - (2 * x - 7).abs() - (2 * y - 7).abs();
    let positional = match pc.abs() {
        1 => y * 8 + center * 2,
        2 => center * 10,
        3 => center * 5,
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
    ((if pc > 0 { 1 } else { -1 }) * (value(pc) + positional)) as i16
}
const fn evaluation_table() -> [[i16; 64]; 13] {
    let mut scores = [[0; 64]; 13];
    let mut piece = 0;
    while piece < 13 {
        let mut square = 0;
        while square < 64 {
            scores[piece][square] = square_score(piece as i8 - 6, square, false);
            square += 1;
        }
        piece += 1;
    }
    scores
}
const fn king_deltas() -> [[i16; 64]; 2] {
    let mut scores = [[0; 64]; 2];
    let mut color = 0;
    while color < 2 {
        let piece = if color == 1 { 6 } else { -6 };
        let mut square = 0;
        while square < 64 {
            scores[color][square] =
                square_score(piece, square, true) - square_score(piece, square, false);
            square += 1;
        }
        color += 1;
    }
    scores
}
// Phase totals 24 in the starting position. Pawns and kings carry no phase.
const FULL_PHASE: i32 = 24;
const PHASE_WEIGHTS: [i32; 7] = [0, 0, 1, 1, 2, 4, 0];

static SQUARE_SCORES: [[i16; 64]; 13] = evaluation_table();
static KING_ENDGAME_DELTAS: [[i16; 64]; 2] = king_deltas();

// Pawn-safe room on files c-f, ranks 2-4 (White) / 5-7 (Black).
// Each square counts once, plus once if up to three ranks behind a friendly
// pawn. Overlapping pawn support does not multiply that second credit.
const SPACE_WHITE: u64 = 0x0000_0000_3c3c_3c00;
const SPACE_MASKS: [u64; 2] = [SPACE_WHITE << 24, SPACE_WHITE];
const SPACE_UNIT: i32 = 4;

fn central_space(occupied: [u64; 2], pawns: [u64; 2], pawn_attacks: [u64; 2]) -> i32 {
    if pawns[0] | pawns[1] == 0 {
        return 0;
    }
    let behind = [
        (pawns[0] << 8) | (pawns[0] << 16) | (pawns[0] << 24),
        (pawns[1] >> 8) | (pawns[1] >> 16) | (pawns[1] >> 24),
    ];
    let mut units = [0; 2];
    for color in 0..2 {
        // Friendly non-pawn pieces can occupy this usable space. Enemy pieces,
        // friendly pawns, and enemy pawn attacks exclude a square.
        let safe =
            SPACE_MASKS[color] & !(pawns[color] | occupied[1 - color] | pawn_attacks[1 - color]);
        units[color] = safe.count_ones() as i32 + (safe & behind[color]).count_ones() as i32;
    }
    SPACE_UNIT * (units[1] - units[0])
}

// Static pawn-only terms, returned as signed (middlegame, endgame) centipawns.
// These initial weights are untuned. Rank tables are from each pawn's own side.
const ISOLATED_PAWN: (i32, i32) = (12, 18);
const DOUBLED_PAWN: (i32, i32) = (10, 16);
const BACKWARD_PAWN: (i32, i32) = (8, 12);
const CONNECTED_PAWN: (i32, i32) = (6, 10);
const PASSED_PAWN: [(i32, i32); 8] = [
    (0, 0),
    (0, 5),
    (4, 10),
    (10, 20),
    (20, 40),
    (35, 70),
    (60, 120),
    (0, 0),
];
const PAWN_FILE_A: u64 = 0x0101_0101_0101_0101;
const PAWN_FILE_H: u64 = PAWN_FILE_A << 7;

fn pawn_neighbors(bits: u64) -> u64 {
    ((bits & !PAWN_FILE_A) >> 1) | ((bits & !PAWN_FILE_H) << 1)
}
fn pawn_north_fill(mut bits: u64) -> u64 {
    bits |= bits << 8;
    bits |= bits << 16;
    bits | (bits << 32)
}
fn pawn_south_fill(mut bits: u64) -> u64 {
    bits |= bits >> 8;
    bits |= bits >> 16;
    bits | (bits >> 32)
}

fn pawn_structure(pawns: [u64; 2], attacks: [u64; 2]) -> (i32, i32) {
    if pawns[0] | pawns[1] == 0 {
        return (0, 0);
    }
    let rear = [
        pawn_north_fill(pawns[0] << 8),
        pawn_south_fill(pawns[1] >> 8),
    ];
    let forward_span = [
        pawn_south_fill(pawns[0] >> 8),
        pawn_north_fill(pawns[1] << 8),
    ];
    let support_span = [
        pawn_neighbors(forward_span[0]),
        pawn_neighbors(forward_span[1]),
    ];
    let all_pawns = pawns[0] | pawns[1];
    let mut total = (0, 0);
    for color in 0..2 {
        let own = pawns[color];
        let downward = if color == 1 {
            rear[color]
        } else {
            forward_span[color]
        };
        let files = (own | downward) & 0xff;
        let occupied_files = files * PAWN_FILE_A;
        let isolated = own & !pawn_neighbors(occupied_files);
        let doubled = own.count_ones() as i32 - files.count_ones() as i32;
        let connected = own & (attacks[color] | pawn_neighbors(own));
        let stops = if color == 1 { own << 8 } else { own >> 8 };
        // No backward penalty for isolated pawns or pawn-blocked advances.
        // The support span includes possible future support from adjacent pawns
        // on the same rank or behind, without simulating their moves.
        let unsafe_stops = stops & attacks[1 - color] & !all_pawns & !support_span[color];
        let backward = (if color == 1 {
            unsafe_stops >> 8
        } else {
            unsafe_stops << 8
        }) & !isolated;
        let blockers = forward_span[1 - color] | pawn_neighbors(forward_span[1 - color]);
        // Only the frontmost pawn on a friendly file can receive a passer bonus.
        let mut passed = own & !rear[color] & !blockers;
        let sign = if color == 1 { 1 } else { -1 };
        let mut mg = connected.count_ones() as i32 * CONNECTED_PAWN.0
            - isolated.count_ones() as i32 * ISOLATED_PAWN.0
            - doubled * DOUBLED_PAWN.0
            - backward.count_ones() as i32 * BACKWARD_PAWN.0;
        let mut eg = connected.count_ones() as i32 * CONNECTED_PAWN.1
            - isolated.count_ones() as i32 * ISOLATED_PAWN.1
            - doubled * DOUBLED_PAWN.1
            - backward.count_ones() as i32 * BACKWARD_PAWN.1;
        while passed != 0 {
            let square = passed.trailing_zeros() as usize;
            passed &= passed - 1;
            let rank = if color == 1 {
                square / 8
            } else {
                7 - square / 8
            };
            mg += PASSED_PAWN[rank].0;
            eg += PASSED_PAWN[rank].1;
        }
        total.0 += sign * mg;
        total.1 += sign * eg;
    }
    total
}

// Original, untuned (middlegame, endgame) weights. These positional
// estimates do not establish tactical safety, legal mobility, or pawn races.
const KNIGHT_OUTPOST: (i32, i32) = (20, 12);
const BISHOP_OUTPOST: (i32, i32) = (12, 8);
const MINOR_BEHIND_PAWN: (i32, i32) = (8, 4);
const BISHOP_PAWN_COLOR: (i32, i32) = (2, 3);
const ROOK_SEMI_OPEN: (i32, i32) = (12, 8);
const ROOK_OPEN: (i32, i32) = (24, 12);

fn piece_pawn_relationships(
    pawns: [u64; 2],
    attacks: [u64; 2],
    knights: [u64; 2],
    diagonal: [u64; 2],
    straight: [u64; 2],
) -> (i32, i32) {
    // Pawnless open-file bonuses apply too; do not return early without pawns.
    let challenges = [
        pawn_neighbors(pawn_south_fill(pawns[0] >> 8)),
        pawn_neighbors(pawn_north_fill(pawns[1] << 8)),
    ];
    let mut total = (0, 0);
    for color in 0..2 {
        let bishops = diagonal[color] & !straight[color];
        let rooks = straight[color] & !diagonal[color];
        let minors = knights[color] | bishops;
        // Relative ranks 4–6. Including current enemy pawn attacks in the
        // challenge span also excludes squares already attacked by a pawn.
        let ranks = if color == 1 {
            0x0000_ffff_ff00_0000
        } else {
            0x0000_00ff_ffff_0000
        };
        let outposts = ranks & attacks[color] & !challenges[1 - color];
        let behind = if color == 1 {
            pawns[color] >> 8
        } else {
            pawns[color] << 8
        };
        let n = (knights[color] & outposts).count_ones() as i32;
        let b = (bishops & outposts).count_ones() as i32;
        let sheltered = (minors & behind).count_ones() as i32;
        let mut mg = n * KNIGHT_OUTPOST.0 + b * BISHOP_OUTPOST.0 + sheltered * MINOR_BEHIND_PAWN.0;
        let mut eg = n * KNIGHT_OUTPOST.1 + b * BISHOP_OUTPOST.1 + sheltered * MINOR_BEHIND_PAWN.1;
        let light = 0x55aa_55aa_55aa_55aa;
        let same_color = (bishops & light).count_ones() * (pawns[color] & light).count_ones()
            + (bishops & !light).count_ones() * (pawns[color] & !light).count_ones();
        mg -= same_color as i32 * BISHOP_PAWN_COLOR.0;
        eg -= same_color as i32 * BISHOP_PAWN_COLOR.1;
        let own_files = pawn_north_fill(pawn_south_fill(pawns[color]));
        let enemy_files = pawn_north_fill(pawn_south_fill(pawns[1 - color]));
        let open = rooks & !own_files;
        let semi_count = (open & enemy_files).count_ones() as i32;
        let open_count = (open & !enemy_files).count_ones() as i32;
        mg += semi_count * ROOK_SEMI_OPEN.0 + open_count * ROOK_OPEN.0;
        eg += semi_count * ROOK_SEMI_OPEN.1 + open_count * ROOK_OPEN.1;
        let sign = if color == 1 { 1 } else { -1 };
        total.0 += sign * mg;
        total.1 += sign * eg;
    }
    total
}

// Static tactical guidance, not a proof of a profitable exchange. Weights
// are original and untuned; each target receives its largest applicable term.
const HANGING: [(i32, i32); 7] = [
    (0, 0),
    (6, 10),
    (16, 20),
    (16, 20),
    (24, 30),
    (40, 50),
    (0, 0),
];
const PAWN_THREAT: [(i32, i32); 7] = [
    (0, 0),
    (0, 0),
    (24, 32),
    (24, 32),
    (40, 48),
    (60, 72),
    (0, 0),
];
const LOWER_THREAT: [(i32, i32); 7] = [(0, 0), (0, 0), (0, 0), (0, 0), (20, 24), (32, 40), (0, 0)];

struct Pressure {
    all: [u64; 2],
    non_king: [u64; 2],
    minor: [u64; 2],
    rook: [u64; 2],
    king_units: [i32; 2],
    zones: [u64; 2],
}
impl Pressure {
    fn new(pawn_attacks: [u64; 2], kings: [usize; 2]) -> Self {
        let adjacent = [
            mobility::king_attacks(kings[0]),
            mobility::king_attacks(kings[1]),
        ];
        let zones = [
            adjacent[0] | (1u64 << kings[0]),
            adjacent[1] | (1u64 << kings[1]),
        ];
        Self {
            all: [pawn_attacks[0] | adjacent[0], pawn_attacks[1] | adjacent[1]],
            non_king: pawn_attacks,
            minor: [0; 2],
            rook: [0; 2],
            king_units: [
                (pawn_attacks[0] & zones[1]).count_ones() as i32,
                (pawn_attacks[1] & zones[0]).count_ones() as i32,
            ],
            zones,
        }
    }
    fn add(&mut self, pc: i8, attacks: u64) {
        let color = usize::from(pc > 0);
        let kind = pc.unsigned_abs() as usize;
        self.all[color] |= attacks;
        self.non_king[color] |= attacks;
        if kind == 2 || kind == 3 {
            self.minor[color] |= attacks;
        }
        if kind == 4 {
            self.rook[color] |= attacks;
        }
        let weight = [0, 0, 2, 2, 3, 5, 0][kind];
        self.king_units[color] +=
            weight * (attacks & self.zones[1 - color]).count_ones().min(4) as i32;
    }
}

fn threats(
    p: &Position,
    occupied: [u64; 2],
    kings: [usize; 2],
    pawn_attacks: [u64; 2],
    pressure: &Pressure,
) -> (i32, i32) {
    let mut total = (0, 0);
    for color in 0..2 {
        let enemy = 1 - color;
        let mut targets = occupied[color] & pressure.all[enemy] & !(1u64 << kings[color]);
        while targets != 0 {
            let square = targets.trailing_zeros() as usize;
            targets &= targets - 1;
            let bit = 1u64 << square;
            let kind = p.squares[square].unsigned_abs() as usize;
            let mut bonus = if pressure.all[color] & bit == 0 {
                HANGING[kind]
            } else {
                (0, 0)
            };
            if pawn_attacks[enemy] & bit != 0 {
                bonus.0 = bonus.0.max(PAWN_THREAT[kind].0);
                bonus.1 = bonus.1.max(PAWN_THREAT[kind].1);
            }
            let lower = if kind == 4 {
                pressure.minor[enemy]
            } else if kind == 5 {
                pressure.minor[enemy] | pressure.rook[enemy]
            } else {
                0
            };
            if lower & bit != 0 {
                bonus.0 = bonus.0.max(LOWER_THREAT[kind].0);
                bonus.1 = bonus.1.max(LOWER_THREAT[kind].1);
            }
            let sign = if color == 1 { -1 } else { 1 };
            total.0 += sign * bonus.0;
            total.1 += sign * bonus.1;
        }
    }
    total
}

// Middlegame-only danger, additionally scaled by opposing attacking material.
fn king_safety(
    kings: [usize; 2],
    pawns: [u64; 2],
    knights: [u64; 2],
    diagonal: [u64; 2],
    straight: [u64; 2],
    pressure: &Pressure,
) -> i32 {
    let mut total = 0;
    for color in 0..2 {
        let enemy = 1 - color;
        let queens = diagonal[enemy] & straight[enemy];
        let bishops = diagonal[enemy] & !straight[enemy];
        let rooks = straight[enemy] & !diagonal[enemy];
        let attack_material = (knights[enemy].count_ones()
            + bishops.count_ones()
            + 2 * rooks.count_ones()
            + 4 * queens.count_ones())
        .min(12) as i32;
        if attack_material == 0 {
            continue;
        }
        let units = pressure.king_units[enemy];
        let weak = (pressure.zones[color] & pressure.all[enemy] & !pressure.non_king[color])
            .count_ones() as i32;
        let mut danger = 2 * units + units * units / 16 + 4 * weak;
        let file = (kings[color] % 8) as i32;
        let rank = (kings[color] / 8) as i32;
        let forward = if color == 1 { 1 } else { -1 };
        let home_rank = if color == 1 { rank } else { 7 - rank };
        for x in (file - 1).max(0)..=(file + 1).min(7) {
            let file_mask = PAWN_FILE_A << x;
            if straight[enemy] != 0 && pawns[color] & file_mask == 0 {
                danger += 6;
                if pawns[enemy] & file_mask == 0 {
                    danger += 4;
                }
            }
            let (mut shield, mut storm) = (0, 0);
            for distance in 1..=3 {
                let y = rank + forward * distance;
                if !(0..8).contains(&y) {
                    break;
                }
                let bit = 1u64 << (8 * y + x);
                if shield == 0 && pawns[color] & bit != 0 {
                    shield = distance;
                }
                if storm == 0 && pawns[enemy] & bit != 0 {
                    storm = distance;
                }
            }
            if home_rank <= 2 {
                danger += [12, 0, 4, 8][shield as usize];
            }
            danger += [0, 12, 8, 4][storm as usize];
        }
        let penalty = danger.min(300) * attack_material / 12;
        total += if color == 1 { -penalty } else { penalty };
    }
    total
}

// Per-thread static evaluations. Validate the full key and all piece squares;
// clocks/history remain exclusively the responsibility of search draw handling.
const EVALUATION_CACHE_ENTRIES: usize = 4096;
#[derive(Clone)]
struct EvaluationEntry {
    key: u64,
    squares: [i8; 64],
    white_score: i32,
}
const EVALUATION_CACHE_BYTES: usize =
    EVALUATION_CACHE_ENTRIES * std::mem::size_of::<EvaluationEntry>();
struct EvaluationCache {
    entries: Vec<EvaluationEntry>,
}
impl Default for EvaluationCache {
    fn default() -> Self {
        Self {
            entries: vec![
                EvaluationEntry {
                    key: 0,
                    squares: [0; 64],
                    white_score: 0
                };
                EVALUATION_CACHE_ENTRIES
            ],
        }
    }
}
impl EvaluationCache {
    fn get(&mut self, p: &Position) -> i32 {
        let entry = &mut self.entries[p.hash as usize & (EVALUATION_CACHE_ENTRIES - 1)];
        if entry.key != p.hash || entry.squares != p.squares {
            let score = evaluate(p);
            *entry = EvaluationEntry {
                key: p.hash,
                squares: p.squares,
                white_score: if p.white { score } else { -score },
            };
        }
        if p.white {
            entry.white_score
        } else {
            -entry.white_score
        }
    }
}

fn evaluate(p: &Position) -> i32 {
    let (mut score, mut endgame_delta, mut phase) = (0, 0, 0);
    let mut bishops = [0, 0];
    let mut occupied = [0; 2];
    let mut pawns = [0; 2];
    let mut kings = [0; 2];
    let mut diagonal = [0; 2];
    let mut knights = [0; 2];
    let mut straight = [0; 2];
    for (square, &piece) in p.squares.iter().enumerate() {
        if piece == 0 {
            continue;
        }
        score += i32::from(SQUARE_SCORES[(piece + 6) as usize][square]);
        phase += PHASE_WEIGHTS[piece.unsigned_abs() as usize];
        let color = usize::from(piece > 0);
        occupied[color] |= 1u64 << square;
        match piece.abs() {
            1 => pawns[color] |= 1u64 << square,
            2 => knights[color] |= 1u64 << square,
            3 => {
                bishops[usize::from(piece < 0)] += 1;
                diagonal[color] |= 1u64 << square;
            }
            4 => straight[color] |= 1u64 << square,
            5 => {
                diagonal[color] |= 1u64 << square;
                straight[color] |= 1u64 << square;
            }
            6 => {
                kings[color] = square;
                endgame_delta += i32::from(KING_ENDGAME_DELTAS[color][square]);
            }
            _ => {}
        }
    }
    let pawn_attacks = mobility::pawn_attacks(pawns);
    let (pawn_mg, pawn_eg) = pawn_structure(pawns, pawn_attacks);
    score += pawn_mg;
    endgame_delta += pawn_eg - pawn_mg;
    let mut pressure = Pressure::new(pawn_attacks, kings);
    if phase != 0 {
        let (piece_mg, piece_eg) =
            piece_pawn_relationships(pawns, pawn_attacks, knights, diagonal, straight);
        score += piece_mg;
        endgame_delta += piece_eg - piece_mg;
        let space = central_space(occupied, pawns, pawn_attacks);
        score += space;
        endgame_delta -= space; // Space is a middlegame-only term.
        let mobility = mobility::Mobility::from_masks(
            occupied,
            pawns,
            kings,
            diagonal,
            straight,
            pawn_attacks,
        );
        let mut active = (occupied[0] | occupied[1])
            & !(pawns[0] | pawns[1] | (1u64 << kings[0]) | (1u64 << kings[1]));
        while active != 0 {
            let square = active.trailing_zeros() as usize;
            active &= active - 1;
            let piece = p.squares[square];
            let attacks = mobility.attacks(square, piece);
            let (mg, eg) = mobility.bonus_from_attacks(piece, attacks);
            pressure.add(piece, attacks);
            score += mg;
            endgame_delta += eg - mg;
        }
    }
    let (threat_mg, threat_eg) = threats(p, occupied, kings, pawn_attacks, &pressure);
    let safety = king_safety(kings, pawns, knights, diagonal, straight, &pressure);
    score += threat_mg + safety;
    endgame_delta += threat_eg - threat_mg - safety;
    // Promotions can exceed the initial material; clamp before blending.
    // Blend all phase-dependent terms once to preserve color symmetry.
    score += endgame_delta * (FULL_PHASE - phase.min(FULL_PHASE)) / FULL_PHASE;
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

#[cfg(test)]
mod tests {
    use super::*;
    use crate::game::Game;
    use std::{sync::mpsc, thread};

    #[test]
    fn evaluation_blends_king_activity_through_material_exchanges() {
        // All non-king material cancels by color. White's central king versus
        // Black's back-rank king scores -30 in the middlegame and +48 at phase 0.
        for (fen, phase, expected) in [
            ("4k3/8/8/8/4K3/8/8/8 w - - 0 1", 0, 48),
            ("r2qk3/8/8/8/4K3/8/8/R2Q4 w - - 0 1", 12, 9),
            ("rnbqkbnr/8/8/8/4K3/8/8/RNBQ1BNR w - - 0 1", 24, -30),
            // Promoted material must never extrapolate beyond the middlegame.
            ("rqbqkbqr/8/8/8/4K3/8/8/RQBQ1BQR w - - 0 1", 24, -30),
        ] {
            let mut p = Position::from_fen(fen).unwrap();
            let (mg, eg) = mobility::tests::reference_scores(&p);
            let ((threat_mg, threat_eg), safety) = reference_tactical_terms(&p);
            assert_eq!((-30 * phase + 48 * (24 - phase)) / 24, expected);
            let combined = -30
                + mg
                + threat_mg
                + safety
                + (78 + eg - mg + threat_eg - threat_mg - safety) * (24 - phase) / 24;
            assert_eq!(evaluate(&p), combined, "{fen}");
            p.white = false;
            assert_eq!(evaluate(&p), -combined);
        }
    }

    #[test]
    fn automatic_thread_budget_preserves_cpu_headroom() {
        for (cpus, threads) in [
            (0, 1),
            (1, 1),
            (2, 1),
            (3, 2),
            (4, 3),
            (8, 6),
            (16, 12),
            (64, 48),
            (128, 64),
        ] {
            assert_eq!(search_threads(cpus), threads);
            if cpus > 1 {
                assert!(threads < cpus);
                assert!(threads * 4 <= cpus * 3);
            }
        }
    }

    #[test]
    fn memory_budgets_reserve_thread_and_server_headroom() {
        for threads in [1, 6, 64] {
            let mut previous = 0;
            for bytes in [
                100_000_000,
                250_000_000,
                500_000_000,
                MAX_MEMORY_LIMIT,
                usize::MAX,
            ] {
                let cache = SearchCache::with_threads_and_memory(threads, bytes);
                let reserve = 16 * 1024 * 1024 + threads * (2 * 1024 * 1024 + MOVE_BUFFER_LIMIT);
                let capped = bytes.min(MAX_MEMORY_LIMIT);
                assert!(cache.table.limit() <= capped.saturating_sub(reserve));
                assert!(cache.table.limit() >= previous);
                previous = cache.table.limit();
                if capped == MAX_MEMORY_LIMIT {
                    assert!(cache.table.limit() <= capped.saturating_sub(reserve + capped / 6));
                }
                assert_eq!(cache.table.bytes(), 0);
            }
        }
        let mut cache = SearchCache::with_threads_and_memory(1, 0);
        let p = Position::default();
        let result = cache.search(
            &p,
            &[p.repetition_key()],
            Duration::from_millis(10),
            Arc::default(),
        );
        assert_eq!(result.cache_bytes, 0);
        assert!(p.legal_moves().contains(&result.best.unwrap()));
    }

    #[test]
    fn default_memory_budget_is_100_mb_and_keeps_cache_space_on_many_cpus() {
        assert_eq!(DEFAULT_MEMORY_LIMIT, 100_000_000);
        let cache = SearchCache::with_threads(6);
        assert_eq!(cache.threads, 6);
        assert_eq!(cache.table.limit(), 65_494_144);
        let many = SearchCache::with_threads(64);
        assert_eq!(many.threads, 18);
        assert!(many.table.limit() >= 32 * 1024 * 1024);
        assert!(many.table.limit() < DEFAULT_MEMORY_LIMIT);
    }

    #[test]
    fn shared_table_keeps_compact_slots_and_coherent_concurrent_entries() {
        let mut table = SharedTable::default();
        table.resize(TABLE_SIZE);
        table.next_generation();
        assert!(table.bytes() <= table.limit());
        assert_eq!(std::mem::size_of::<cache::Bucket>(), 64);
        thread::scope(|scope| {
            for worker in 0..4 {
                let table = &table;
                scope.spawn(move || {
                    for step in 0..2000 {
                        // All writers contend for one bucket. Busy accesses
                        // may be skipped, but a returned full key must match.
                        let key = (worker * 2000 + step) << 24;
                        let score = (key % 20_000) as i32 - 10_000;
                        table.store(
                            key,
                            Entry {
                                depth: 5,
                                score,
                                best: None,
                                bound: Bound::Exact,
                            },
                        );
                        if let Some(entry) = table.probe(key) {
                            assert_eq!(entry.score, score);
                            assert_eq!(entry.depth, 5);
                        }
                    }
                });
            }
        });
    }

    #[test]
    fn principal_variation_search_verifies_later_improvements_and_preserves_bounds() {
        for fen in [
            "rnbqkbnr/pppppppp/8/8/8/8/PPPPPPPP/RNBQKBNR w KQkq - 0 1",
            "8/2p5/3p4/KP5r/1R3p1k/8/4P1P1/8 w - - 0 1",
        ] {
            let p = Position::from_fen(fen).unwrap();
            let keys = [p.repetition_key()];
            // Independently resolve every root alternative with a full window.
            // Deliberately search the worst one first to force PV replacement.
            let scores: Vec<_> = p
                .legal_moves()
                .into_iter()
                .map(|m| {
                    let child = p.play(m);
                    let child_keys = [keys[0], child.repetition_key()];
                    let score =
                        -at_depth(&mut SearchCache::with_threads(1), &child, &child_keys, 2, 1).0;
                    (m, score)
                })
                .collect();
            let &(worst, low) = scores.iter().min_by_key(|(_, score)| score).unwrap();
            let expected = scores.iter().map(|(_, score)| *score).max().unwrap();
            assert!(low < expected);
            for (alpha, beta, bound) in [
                (-INF, INF, Bound::Exact),
                (expected, expected + 1, Bound::Upper),
                (low - 2, low - 1, Bound::Lower),
                (low - 1, expected, Bound::Lower),
            ] {
                let mut cache = SearchCache::with_threads(1);
                cache.table.initialize();
                let mut search = test_search(&mut cache, &p, &keys);
                let key = search.table_key(&p);
                search.table.store(
                    key,
                    Entry {
                        depth: 0,
                        score: 0,
                        best: Some(worst),
                        bound: Bound::Upper,
                    },
                );
                let (score, best) = search.negamax(&p, 3, 0, alpha, beta);
                assert!(!search.aborted);
                let entry = search.probe(key).unwrap();
                assert_eq!(entry.bound, bound, "{fen} / {alpha}..{beta}");
                match bound {
                    Bound::Exact => {
                        assert_eq!(score, expected);
                        assert_ne!(best, Some(worst));
                        assert!(scores.contains(&(best.unwrap(), expected)));
                    }
                    Bound::Upper => assert!(expected <= score && score <= alpha),
                    Bound::Lower => assert!(beta <= score && score <= expected),
                }
                // A cached probe/cutoff bound must not masquerade as an exact
                // result when the same position gets a wider search window.
                let (score, best) = search.negamax(&p, 3, 0, -INF, INF);
                assert_eq!(score, expected);
                assert!(scores.contains(&(best.unwrap(), expected)));
                assert_eq!(search.probe(key).unwrap().bound, Bound::Exact);
            }
        }
    }

    #[test]
    fn leading_branch_splits_preserve_scores_mates_draws_and_history() {
        for (fen, depth) in [
            (
                "rnbqkbnr/pppppppp/8/8/8/8/PPPPPPPP/RNBQKBNR w KQkq - 0 1",
                6,
            ),
            ("8/5pk1/2p3p1/1p1pP2p/1P1P1P1P/2P3K1/8/8 w - - 0 1", 7),
            (
                "rnbqkbnr/pppp1ppp/8/4p3/6P1/5P2/PPPPP2P/RNBQKBNR b KQkq g3 0 2",
                6,
            ),
            ("4k3/8/8/8/8/8/4R3/4K3 w - - 99 1", 6),
            ("7k/5K2/8/6Q1/8/8/8/8 b - - 0 1", 6),
        ] {
            let p = Position::from_fen(fen).unwrap();
            let keys = [p.repetition_key()];
            let expected = at_depth(&mut SearchCache::with_threads(1), &p, &keys, depth, 0).0;
            let mut cache = SearchCache::with_threads(3);
            cache.table.initialize();
            let mut helpers = [MoveBuffers::default(), MoveBuffers::default()];
            let peers = [WorkerStats::default(), WorkerStats::default()];
            let mut search = test_search(&mut cache, &p, &keys);
            search.threads = 3;
            search.peers = &peers;
            let (score, best) = search.parallel_root(&p, depth, &p.legal_moves(), &mut helpers);
            assert!(!search.aborted);
            assert_eq!(score, expected, "{fen}");
            assert_eq!(search.keys, keys);
            assert_eq!(search.history_context, reversible_context(&p, &keys));
            let child = p.play(best.unwrap());
            let verified = -at_depth(
                &mut SearchCache::with_threads(1),
                &child,
                &[keys[0], child.repetition_key()],
                depth - 1,
                1,
            )
            .0;
            assert_eq!(score, verified, "unproven best move after splitting: {fen}");
            let entry = search.probe(search.table_key(&p)).unwrap();
            assert_eq!(entry.bound, Bound::Exact);
            assert_eq!(entry.score, score);
            assert_eq!(entry.depth, depth);
        }
    }

    #[test]
    fn parallel_search_matches_serial_scores_and_returns_a_proven_best_move() {
        let mut helper_nodes = 0;
        for (fen, depth) in [
            (
                "rnbqkbnr/pppppppp/8/8/8/8/PPPPPPPP/RNBQKBNR w KQkq - 0 1",
                4,
            ),
            (
                "r3k2r/p1ppqpb1/bn2pnp1/3PN3/1p2P3/2N2Q1p/PPPBBPPP/R3K2R w KQkq - 0 1",
                3,
            ),
            ("8/2p5/3p4/KP5r/1R3p1k/8/4P1P1/8 w - - 0 1", 4),
            (
                "rnbqk2r/pp1Pbppp/2p5/8/2B5/8/PPP1NnPP/RNBQK2R w KQkq - 1 8",
                3,
            ),
            (
                "rnbqkbnr/pppp1ppp/8/4p3/6P1/5P2/PPPPP2P/RNBQKBNR b KQkq g3 0 2",
                2,
            ),
        ] {
            let p = Position::from_fen(fen).unwrap();
            let keys = [p.repetition_key()];
            let expected = at_depth(&mut SearchCache::with_threads(1), &p, &keys, depth, 0);
            for _ in 0..3 {
                let mut cache = SearchCache::with_threads(3);
                cache.table.resize(TABLE_SIZE);
                cache.table.next_generation();
                let mut helpers = [MoveBuffers::default(), MoveBuffers::default()];
                let peers = [WorkerStats::default(), WorkerStats::default()];
                let mut search = test_search(&mut cache, &p, &keys);
                search.peers = &peers;
                search.threads = 3;
                let (score, best) = search.parallel_root(&p, depth, &p.legal_moves(), &mut helpers);
                assert!(!search.aborted);
                assert_eq!(score, expected.0, "{fen}");
                let child = p.play(best.unwrap());
                let child_keys = [keys[0], child.repetition_key()];
                let verified = -at_depth(
                    &mut SearchCache::with_threads(1),
                    &child,
                    &child_keys,
                    depth - 1,
                    1,
                )
                .0;
                assert_eq!(
                    score, verified,
                    "an unproven bound became the best move: {fen}"
                );
                helper_nodes += peers
                    .iter()
                    .map(|peer| peer.nodes.load(Ordering::Relaxed))
                    .sum::<u64>();
                // The root score is cached only after all root moves finish.
                let cached = search.probe(search.table_key(&p)).unwrap();
                assert!(matches!(cached.bound, Bound::Exact));
                assert_eq!(cached.depth, depth);
                assert_eq!(cached.score, score);
            }
        }
        // A small tree can finish before a helper is scheduled. Require helper
        // work across the corpus, rather than on every fast cutoff position.
        assert!(helper_nodes > 0);
    }

    #[test]
    fn interrupted_parallel_iteration_keeps_the_last_completed_result() {
        let p = Position::default();
        let mut cache = SearchCache::with_threads(3);
        cache.table.resize(TABLE_SIZE);
        cache.table.next_generation();
        let keys = [p.repetition_key()];
        let mut helpers = [MoveBuffers::default(), MoveBuffers::default()];
        let peers = [WorkerStats::default(), WorkerStats::default()];
        let mut search = test_search(&mut cache, &p, &keys);
        search.peers = &peers;
        let completed = search.negamax(&p, 2, 0, -INF, INF);
        let key = search.table_key(&p);
        search.deadline = Some(Instant::now() + Duration::from_millis(20));
        search.parallel_root(&p, 8, &p.legal_moves(), &mut helpers);
        assert!(search.aborted);
        let entry = search.probe(key).unwrap();
        assert_eq!(entry.depth, 2);
        assert_eq!(entry.best, completed.1);
        assert_eq!(entry.score, completed.0);
    }

    #[test]
    #[ignore = "parallel fixed-depth benchmark; run with --release --ignored --nocapture"]
    fn benchmark_parallel_depth() {
        let threads = SearchCache::default().threads;
        let cases = [
            (
                "opening",
                "rnbqkbnr/pppppppp/8/8/8/8/PPPPPPPP/RNBQKBNR w KQkq - 0 1",
                7,
            ),
            (
                "middlegame",
                "r3k2r/p1ppqpb1/bn2pnp1/3PN3/1p2P3/2N2Q1p/PPPBBPPP/R3K2R w KQkq - 0 1",
                5,
            ),
            ("endgame", "8/2p5/3p4/KP5r/1R3p1k/8/4P1P1/8 w - - 0 1", 8),
            (
                "tactics",
                "rnbqk2r/pp1Pbppp/2p5/8/2B5/8/PPP1NnPP/RNBQK2R w KQkq - 1 8",
                6,
            ),
            (
                "pawns",
                "8/5pk1/2p3p1/1p1pP2p/1P1P1P1P/2P3K1/8/8 w - - 0 1",
                9,
            ),
        ];
        println!("case,threads,depth,score,best,median_ms,nodes");
        for (name, fen, depth) in cases {
            let p = Position::from_fen(fen).unwrap();
            let keys = [p.repetition_key()];
            let mut expected_score = None;
            for limit in [1, threads] {
                let mut runs = Vec::new();
                for repetition in 0..4 {
                    let mut cache = SearchCache::with_threads(limit);
                    cache.table.resize(TABLE_SIZE);
                    cache.table.next_generation();
                    let mut helpers: Vec<_> = (1..limit).map(|_| MoveBuffers::default()).collect();
                    let peers: Vec<_> = (1..limit).map(|_| WorkerStats::default()).collect();
                    let mut search = test_search(&mut cache, &p, &keys);
                    search.threads = limit;
                    search.peers = &peers;
                    let started = Instant::now();
                    let mut result = (0, None);
                    for d in 1..=depth {
                        result = if limit > 1 && d >= 4 {
                            search.parallel_root(&p, d, &p.legal_moves(), &mut helpers)
                        } else {
                            search.negamax(&p, d, 0, -INF, INF)
                        };
                    }
                    if let Some(score) = expected_score {
                        assert_eq!(result.0, score, "{name}");
                    }
                    expected_score = Some(result.0);
                    if repetition > 0 {
                        runs.push((
                            started.elapsed(),
                            result.0,
                            result.1.unwrap(),
                            search.totals().0,
                        ));
                    }
                }
                runs.sort_by_key(|r| r.0);
                let (elapsed, score, best, nodes) = runs[1];
                println!(
                    "{name},{limit},{depth},{score},{best},{:.3},{nodes}",
                    elapsed.as_secs_f64() * 1000.0
                );
            }
        }
    }

    #[test]
    fn move_buffer_pool_reuses_storage_and_respects_its_memory_cap() {
        let mut buffers = MoveBuffers::default();
        let mut moves = buffers.take(0);
        moves.reserve(48);
        let original = moves.as_ptr();
        buffers.put(0, moves);
        let moves = buffers.take(0);
        assert_eq!(moves.as_ptr(), original);
        assert_eq!(buffers.retained_bytes, 0);
        buffers.put(0, moves);
        let mut oversized = buffers.take(1);
        oversized.reserve(MOVE_BUFFER_LIMIT / std::mem::size_of::<Move>() + 1);
        buffers.put(1, oversized);
        assert_eq!(buffers.slots[1].capacity(), 0);
        for ply in 0..BUFFER_PLIES {
            let mut moves = buffers.take(ply);
            moves.reserve(128);
            buffers.put(ply, moves);
        }
        assert_eq!(
            buffers.evaluation.entries.capacity() * std::mem::size_of::<EvaluationEntry>(),
            EVALUATION_CACHE_BYTES
        );
        assert!(
            buffers.retained_bytes + ORDERING_BYTES + EVALUATION_CACHE_BYTES <= MOVE_BUFFER_LIMIT
        );
        assert_eq!(
            buffers.retained_bytes,
            buffers
                .slots
                .iter()
                .map(|v| v.capacity() * std::mem::size_of::<Move>())
                .sum()
        );
        let beyond_pool = buffers.take(BUFFER_PLIES);
        buffers.put(BUFFER_PLIES, beyond_pool);
        assert!(buffers.slots.len() <= BUFFER_PLIES);
    }

    #[test]
    fn static_evaluation_cache_handles_collisions_turns_and_special_moves() {
        let mut cache = EvaluationCache::default();
        for fen in [
            "rnbqkbnr/pppppppp/8/8/8/8/PPPPPPPP/RNBQKBNR w KQkq - 0 1",
            "r3k2r/P6P/8/3pP3/8/8/p6p/R3K2R w KQkq d6 0 1",
            "8/2p5/3p4/KP5r/1R3p1k/8/4P1P1/8 w - - 0 1",
        ] {
            let p = Position::from_fen(fen).unwrap();
            let expected = reference_evaluate(&p);
            for m in p.legal_moves() {
                assert_eq!(cache.get(&p), expected);
                let mut child = p.play(m);
                // Force both index collisions and identical full hashes with
                // different boards. Neither can return a stale static score.
                child.hash = p.hash ^ EVALUATION_CACHE_ENTRIES as u64;
                assert_eq!(cache.get(&child), reference_evaluate(&child));
                child.hash = p.hash;
                assert_eq!(cache.get(&child), reference_evaluate(&child));
                child.white = !child.white;
                assert_eq!(cache.get(&child), reference_evaluate(&child));
                assert_eq!(cache.get(&p), expected);
            }
            let mut same_board = p.clone();
            same_board.halfmove = 100;
            same_board.fullmove = 71;
            same_board.castling = 0;
            same_board.ep = None;
            assert_eq!(cache.get(&same_board), expected);
        }
    }

    #[test]
    fn incremental_history_and_cached_evaluation_match_recomputation() {
        for seed in [
            "rnbqkbnr/pppppppp/8/8/8/8/PPPPPPPP/RNBQKBNR w KQkq - 0 1",
            "r3k2r/P6P/8/3pP3/8/8/p6p/R3K2R w KQkq d6 0 1",
            "8/2p5/3p4/KP5r/1R3p1k/8/4P1P1/8 w - - 0 1",
        ] {
            let mut p = Position::from_fen(seed).unwrap();
            let mut cache = SearchCache::default();
            let mut search = test_search(&mut cache, &p, &[p.repetition_key()]);
            let mut random = 17;
            for _ in 0..500 {
                assert_eq!(
                    pawn_structure_for_position(&p),
                    reference_pawn_structure(&p),
                    "{:?}",
                    p.squares
                );
                assert_eq!(
                    relationships_for_position(&p),
                    reference_piece_pawn_relationships(&p)
                );
                assert_eq!(
                    tactical_terms_for_position(&p),
                    reference_tactical_terms(&p)
                );
                assert_eq!(evaluate(&p), reference_evaluate(&p));
                assert_eq!(search.buffers.evaluation.get(&p), reference_evaluate(&p));
                assert_eq!(space_for_position(&p), reference_space(&p));
                let mut mirrored = p.clone();
                for (s, &pc) in p.squares.iter().enumerate() {
                    mirrored.squares[s ^ 56] = -pc;
                }
                // Evaluation ignores the stale hash/rights in this mirrored copy.
                assert_eq!(evaluate(&mirrored), -evaluate(&p));
                assert_eq!(
                    search.buffers.evaluation.get(&mirrored),
                    reference_evaluate(&mirrored)
                );
                let mobility = mobility::Mobility::new(&p);
                for (s, &pc) in p.squares.iter().enumerate() {
                    assert_eq!(
                        mobility.count(&p, s, pc),
                        mobility::tests::reference_count(&p, s)
                    );
                    if (2..=5).contains(&pc.abs()) {
                        assert_eq!(
                            mobility.attacks(s, pc),
                            mobility::tests::reference_attacks(&p, s)
                        );
                    }
                }
                assert_eq!(search.history_context, reversible_context(&p, &search.keys));
                assert_eq!(
                    search.table_key(&p),
                    p.hash
                        ^ reversible_context(&p, &search.keys).rotate_left(17)
                        ^ random_key(20_000 + p.halfmove as u64)
                );
                let legal = p.legal_moves();
                if legal.is_empty() {
                    break;
                }
                random = random_key(random);
                let next = p.play(legal[random as usize % legal.len()]);
                let previous = search.push_position(&next);
                assert_eq!(
                    search.history_context,
                    reversible_context(&next, &search.keys)
                );
                search.pop_position(previous);
                assert_eq!(search.history_context, reversible_context(&p, &search.keys));
                search.push_position(&next);
                p = next;
            }
        }
    }

    #[test]
    #[ignore = "fixed-work runtime benchmark; run with --release --ignored --nocapture"]
    fn benchmark_fixed_depth() {
        let cases = [
            (
                "opening",
                "rnbqkbnr/pppppppp/8/8/8/8/PPPPPPPP/RNBQKBNR w KQkq - 0 1",
                5,
            ),
            (
                "middlegame",
                "r3k2r/p1ppqpb1/bn2pnp1/3PN3/1p2P3/2N2Q1p/PPPBBPPP/R3K2R w KQkq - 0 1",
                4,
            ),
            ("endgame", "8/2p5/3p4/KP5r/1R3p1k/8/4P1P1/8 w - - 0 1", 6),
            (
                "tactics",
                "rnbqk2r/pp1Pbppp/2p5/8/2B5/8/PPP1NnPP/RNBQK2R w KQkq - 1 8",
                4,
            ),
            (
                "pawns",
                "8/5pk1/2p3p1/1p1pP2p/1P1P1P1P/2P3K1/8/8 w - - 0 1",
                6,
            ),
        ];
        println!("case,depth,nodes,hits,score,best,median_ms,scratch_bytes");
        for (name, fen, depth) in cases {
            let p = Position::from_fen(fen).unwrap();
            let mut times = Vec::new();
            let mut expected = None;
            let mut scratch = 0;
            for repetition in 0..8 {
                let mut cache = SearchCache::default();
                cache.table.resize(TABLE_SIZE);
                cache.table.next_generation();
                let mut search = test_search(&mut cache, &p, &[p.repetition_key()]);
                let start = Instant::now();
                let (score, best) = search.negamax(&p, depth, 0, -INF, INF);
                let elapsed = start.elapsed();
                let result = (search.nodes, search.hits, score, best);
                if let Some(expected) = expected {
                    assert_eq!(result, expected);
                }
                expected = Some(result);
                if repetition != 0 {
                    times.push(elapsed);
                }
                scratch = cache.buffers.retained_bytes;
            }
            times.sort();
            let (nodes, hits, score, best) = expected.unwrap();
            println!(
                "{name},{depth},{nodes},{hits},{score},{},{:.3},{scratch}",
                best.unwrap(),
                times[times.len() / 2].as_secs_f64() * 1000.0
            );
        }
    }

    #[test]
    fn move_order_keeps_cache_capture_promotion_priority_and_quiet_ties() {
        let p = Position::from_fen("4k3/P7/8/3p4/4P3/8/8/4K3 w - - 0 1").unwrap();
        let legal = p.legal_moves();
        let mut moves: Vec<_> = [
            "e4e5", "e1d1", "a7a8n", "e4d5", "a7a8q", "e1f1", "a7a8r", "a7a8b",
        ]
        .iter()
        .map(|uci| *legal.iter().find(|m| m.to_string() == *uci).unwrap())
        .collect();
        let cached = moves[5];
        order(&p, &mut moves, Some(cached));
        assert_eq!(
            moves.iter().map(ToString::to_string).collect::<Vec<_>>(),
            ["e1f1", "e4d5", "a7a8q", "a7a8r", "a7a8b", "a7a8n", "e4e5", "e1d1"]
        );
    }

    #[test]
    fn move_order_preserves_lists_larger_than_the_stack_buffer() {
        let p = Position::from_fen("k7/8/8/8/1Q1Q1Q1Q/Q1Q1Q1Q1/8/7K w - - 0 1").unwrap();
        let mut moves = p.legal_moves();
        assert!(moves.len() > 64);
        let mut original = moves.clone();
        let cached = *moves.last().unwrap();
        order(&p, &mut moves, Some(cached));
        assert_eq!(moves[0], cached);
        moves.sort_unstable_by_key(|m| (m.from, m.to, m.promotion));
        original.sort_unstable_by_key(|m| (m.from, m.to, m.promotion));
        assert_eq!(moves, original);
    }

    #[test]
    fn compact_cache_layout() {
        use std::mem::size_of;
        assert_eq!(size_of::<PackedEntry>(), 6);
        let mut table = Table::default();
        table.resize(TABLE_SIZE);
        let bytes = table.buckets.capacity() * size_of::<cache::Bucket>();
        assert_eq!(TABLE_SIZE, 2_000_000);
        assert_eq!(bytes, 32_000_000);
        println!(
            "Cache: 16 bytes per slot including shared epoch, {bytes} bytes for {TABLE_SIZE} slots"
        );
    }

    #[test]
    fn packed_entries_round_trip_moves_promotions_scores_and_bounds() {
        // Include every square pair and promotion, so the move encoding cannot
        // alias an ordinary move, promotion, or the no-move sentinel.
        for from in 0..64 {
            for to in 0..64 {
                if from == to {
                    continue;
                }
                for promotion in [0, 2, 3, 4, 5] {
                    let entry = Entry {
                        depth: u8::MAX,
                        score: -MATE,
                        bound: Bound::Exact,
                        best: Some(Move {
                            from,
                            to,
                            promotion,
                        }),
                    };
                    for generation in [0, 1, GENERATION_MASK] {
                        let packed = PackedEntry::pack(entry, generation).unwrap();
                        assert_eq!(packed.unpack(), Some(entry));
                        assert_eq!(packed.age(generation), 0);
                    }
                }
            }
        }
        for score in [i16::MIN as i32, -MATE, -1, 0, 1, MATE, i16::MAX as i32] {
            for bound in [Bound::Exact, Bound::Lower, Bound::Upper] {
                for depth in [0, 1, u8::MAX] {
                    let entry = Entry {
                        depth,
                        score,
                        bound,
                        best: None,
                    };
                    assert_eq!(
                        PackedEntry::pack(entry, GENERATION_MASK).unwrap().unpack(),
                        Some(entry)
                    );
                }
            }
        }
    }

    #[test]
    fn compact_table_checks_full_keys_and_never_truncates_scores() {
        let mut table = Table::default();
        table.resize(BUCKET_SIZE);
        assert_eq!(table.probe(0), None);
        let entry = Entry {
            depth: 7,
            score: MATE - 5,
            bound: Bound::Lower,
            best: Some(Move {
                from: 0,
                to: 63,
                promotion: 0,
            }),
        };
        table.store(0, entry);
        assert_eq!(table.probe(0), Some(entry));
        // These keys collide and share their low bits. Match all 64 key bits.
        let other_key = 1 << 63;
        assert_eq!(table.probe(other_key), None);
        table.store(other_key, entry);
        assert_eq!(table.probe(0), Some(entry));
        assert_eq!(table.probe(other_key), Some(entry));
        for score in [i32::MIN, i16::MIN as i32 - 1, i16::MAX as i32 + 1, i32::MAX] {
            table.store(1, Entry { score, ..entry });
            assert_eq!(table.probe(1), None);
            assert_eq!(table.probe(0), Some(entry));
            assert_eq!(table.probe(other_key), Some(entry));
        }
    }

    fn cache_entry(depth: u8, bound: Bound) -> Entry {
        Entry {
            depth,
            bound,
            score: 17,
            best: Some(Move {
                from: 1,
                to: 18,
                promotion: 0,
            }),
        }
    }

    #[test]
    fn colliding_positions_share_a_bucket_and_keep_the_strongest_results() {
        let mut table = Table::default();
        table.resize(BUCKET_SIZE);
        for (key, entry) in [
            (0, cache_entry(8, Bound::Exact)),
            (1, cache_entry(8, Bound::Lower)),
            (2, cache_entry(9, Bound::Upper)),
            (3, cache_entry(10, Bound::Exact)),
        ] {
            table.store(key, entry);
        }
        // No same-generation shallow result can displace these deep entries.
        table.store(4, cache_entry(1, Bound::Exact));
        assert_eq!(table.probe(4), None);
        for key in 0..4 {
            assert!(table.probe(key).is_some());
        }
        // At equal depth an exact score wins over a bound. The other exact
        // depth-eight result survives even though it was inserted first.
        table.store(4, cache_entry(8, Bound::Exact));
        assert_eq!(table.probe(1), None);
        assert_eq!(table.probe(0), Some(cache_entry(8, Bound::Exact)));
        assert_eq!(table.probe(4), Some(cache_entry(8, Bound::Exact)));
        assert!(table.probe(2).is_some());
        assert!(table.probe(3).is_some());
    }

    #[test]
    fn same_position_keeps_deep_exact_results_and_tightens_bounds() {
        let mut table = Table::default();
        table.resize(BUCKET_SIZE);
        let deep = cache_entry(8, Bound::Exact);
        table.store(0, deep);
        table.store(0, cache_entry(7, Bound::Exact));
        table.store(0, cache_entry(8, Bound::Lower));
        assert_eq!(table.probe(0), Some(deep));
        let deeper_bound = cache_entry(9, Bound::Lower);
        table.store(0, deeper_bound);
        table.store(
            0,
            Entry {
                score: 16,
                ..deeper_bound
            },
        );
        assert_eq!(table.probe(0), Some(deeper_bound));
        table.store(
            0,
            Entry {
                score: 18,
                ..deeper_bound
            },
        );
        assert_eq!(table.probe(0).unwrap().score, 18);
        let exact = cache_entry(9, Bound::Exact);
        table.store(0, exact);
        assert_eq!(table.probe(0), Some(exact));
        let upper = cache_entry(9, Bound::Upper);
        table.store(1, upper);
        table.store(1, Entry { score: 18, ..upper });
        assert_eq!(table.probe(1), Some(upper));
        table.store(1, Entry { score: 16, ..upper });
        assert_eq!(table.probe(1).unwrap().score, 16);
        // A hole before an existing position must not create duplicate entries.
        table.buckets[0].entries[0] = PackedEntry::default();
        table.store(1, cache_entry(10, Bound::Exact));
        assert_eq!(table.probe(1), Some(cache_entry(10, Bound::Exact)));
        assert_eq!(
            (0..BUCKET_SIZE)
                .filter(
                    |&i| table.buckets[0].keys[i] == 1 && !table.buckets[0].entries[i].is_empty()
                )
                .count(),
            1
        );
    }

    #[test]
    fn eviction_ages_unused_entries_and_refreshes_looked_up_positions() {
        let mut table = Table::default();
        table.resize(BUCKET_SIZE);
        for key in 0..4 {
            table.store(key, cache_entry(10, Bound::Exact));
        }
        for _ in 0..3 {
            table.next_generation();
        }
        assert!(table.probe(0).is_some());
        table.store(4, cache_entry(5, Bound::Lower));
        assert!(table.probe(0).is_some());
        assert_eq!(table.probe(1), None);
        assert_eq!(table.probe(4), Some(cache_entry(5, Bound::Lower)));
    }

    #[test]
    fn generation_wrap_cannot_resurrect_unused_entries() {
        let mut table = Table::default();
        table.resize(BUCKET_SIZE);
        table.store(0, cache_entry(255, Bound::Exact));
        table.store(1, cache_entry(9, Bound::Exact));
        for _ in 0..128 {
            table.next_generation();
            assert_eq!(table.probe(1), Some(cache_entry(9, Bound::Exact)));
        }
        assert_eq!(table.probe(0), None);
        assert_eq!(table.generation, 128);
    }

    // Independent square/coordinate oracle. No production pressure masks,
    // king geometry, danger helper, or threat weight constants are used here.
    fn reference_tactical_terms(p: &Position) -> ((i32, i32), i32) {
        let kings = [
            p.squares.iter().position(|&pc| pc == -6).unwrap(),
            p.squares.iter().position(|&pc| pc == 6).unwrap(),
        ];
        let in_zone = |square: usize, king: usize| {
            (square as i32 % 8 - king as i32 % 8).abs() <= 1
                && (square as i32 / 8 - king as i32 / 8).abs() <= 1
        };
        // Slot zero holds the union; other slots hold attacks by piece kind.
        let mut maps = [[[false; 64]; 7]; 2];
        let mut units = [0; 2];
        for (s, &pc) in p.squares.iter().enumerate() {
            if pc == 0 {
                continue;
            }
            let c = usize::from(pc > 0);
            let kind = pc.unsigned_abs() as usize;
            let raw = mobility::tests::reference_attacks(p, s);
            let mut hits = 0;
            for (to, _) in p.squares.iter().enumerate() {
                if raw & (1u64 << to) != 0 {
                    maps[c][0][to] = true;
                    maps[c][kind][to] = true;
                    if in_zone(to, kings[1 - c]) {
                        hits += 1;
                    }
                }
            }
            units[c] += match kind {
                2 | 3 => 2 * hits.min(4),
                4 => 3 * hits.min(4),
                5 => 5 * hits.min(4),
                _ => 0,
            };
        }
        for c in 0..2 {
            units[c] += (0..64)
                .filter(|&to| maps[c][1][to] && in_zone(to, kings[1 - c]))
                .count() as i32;
        }
        let mut threats = (0, 0);
        for (s, &pc) in p.squares.iter().enumerate() {
            if pc == 0 || pc.abs() == 6 {
                continue;
            }
            let c = usize::from(pc > 0);
            let enemy = 1 - c;
            let k = pc.unsigned_abs() as usize;
            if !maps[enemy][0][s] {
                continue;
            }
            let mut options = vec![(0, 0)];
            if !maps[c][0][s] {
                options.push([(0, 0), (6, 10), (16, 20), (16, 20), (24, 30), (40, 50)][k]);
            }
            if maps[enemy][1][s] {
                options.push([(0, 0), (0, 0), (24, 32), (24, 32), (40, 48), (60, 72)][k]);
            }
            if (k == 4 || k == 5)
                && (maps[enemy][2][s] || maps[enemy][3][s] || k == 5 && maps[enemy][4][s])
            {
                options.push(if k == 4 { (20, 24) } else { (32, 40) });
            }
            let sign = -i32::from(pc.signum());
            threats.0 += sign * options.iter().map(|x| x.0).max().unwrap();
            threats.1 += sign * options.iter().map(|x| x.1).max().unwrap();
        }
        let mut safety = 0;
        for c in 0..2 {
            let sign = if c == 1 { 1i8 } else { -1 };
            let (x, y) = (kings[c] as i32 % 8, kings[c] as i32 / 8);
            let forward = i32::from(sign);
            let enemy_material = p
                .squares
                .iter()
                .filter(|v| v.signum() == -sign)
                .map(|v| match v.abs() {
                    2 | 3 => 1,
                    4 => 2,
                    5 => 4,
                    _ => 0,
                })
                .sum::<i32>()
                .min(12);
            if enemy_material == 0 {
                continue;
            }
            let weak = (0..64)
                .filter(|&to| {
                    in_zone(to, kings[c])
                        && maps[1 - c][0][to]
                        && !(1..=5).any(|kind| maps[c][kind][to])
                })
                .count() as i32;
            let mut danger = 2 * units[1 - c] + units[1 - c] * units[1 - c] / 16 + 4 * weak;
            let pawns: Vec<_> = p
                .squares
                .iter()
                .enumerate()
                .filter(|(_, v)| v.abs() == 1)
                .map(|(s, &v)| (s as i32 % 8, s as i32 / 8, v))
                .collect();
            let enemy_sliders = p
                .squares
                .iter()
                .any(|v| v.signum() == -sign && matches!(v.abs(), 4 | 5));
            for file in (x - 1).max(0)..=(x + 1).min(7) {
                if enemy_sliders && !pawns.iter().any(|&(a, _, v)| a == file && v == sign) {
                    danger += if pawns.iter().any(|&(a, _, v)| a == file && v == -sign) {
                        6
                    } else {
                        10
                    };
                }
                let nearest = |owner| {
                    pawns
                        .iter()
                        .filter(|&&(a, b, v)| {
                            a == file && v == owner && (1..=3).contains(&((b - y) * forward))
                        })
                        .map(|&(_, b, _)| (b - y) * forward)
                        .min()
                };
                if (if sign > 0 { y } else { 7 - y }) <= 2 {
                    danger += match nearest(sign) {
                        Some(1) => 0,
                        Some(2) => 4,
                        Some(3) => 8,
                        _ => 12,
                    };
                }
                danger += match nearest(-sign) {
                    Some(1) => 12,
                    Some(2) => 8,
                    Some(3) => 4,
                    _ => 0,
                };
            }
            safety -= i32::from(sign) * danger.min(300) * enemy_material / 12;
        }
        (threats, safety)
    }

    fn tactical_terms_for_position(p: &Position) -> ((i32, i32), i32) {
        let (mut occupied, mut pawns, mut knights, mut diagonal, mut straight, mut kings) =
            ([0; 2], [0; 2], [0; 2], [0; 2], [0; 2], [0; 2]);
        for (s, &pc) in p.squares.iter().enumerate() {
            if pc == 0 {
                continue;
            }
            let c = usize::from(pc > 0);
            occupied[c] |= 1u64 << s;
            match pc.abs() {
                1 => pawns[c] |= 1u64 << s,
                2 => knights[c] |= 1u64 << s,
                3 => diagonal[c] |= 1u64 << s,
                4 => straight[c] |= 1u64 << s,
                5 => {
                    diagonal[c] |= 1u64 << s;
                    straight[c] |= 1u64 << s;
                }
                6 => kings[c] = s,
                _ => {}
            }
        }
        let pawn_attacks = mobility::pawn_attacks(pawns);
        let mobility = mobility::Mobility::from_masks(
            occupied,
            pawns,
            kings,
            diagonal,
            straight,
            pawn_attacks,
        );
        let mut pressure = Pressure::new(pawn_attacks, kings);
        for (s, &pc) in p.squares.iter().enumerate() {
            if (2..=5).contains(&pc.abs()) {
                pressure.add(pc, mobility.attacks(s, pc));
            }
        }
        (
            threats(p, occupied, kings, pawn_attacks, &pressure),
            king_safety(kings, pawns, knights, diagonal, straight, &pressure),
        )
    }

    #[test]
    fn threats_count_targets_once_and_distinguish_defense_and_cheaper_attacks() {
        for (fen, expected) in [
            ("4k3/8/8/n7/8/8/8/R3K3 w - - 0 1", (16, 20)),
            ("4k3/8/8/n7/8/P7/8/R3K3 w - - 0 1", (0, 0)),
            ("k3r3/8/8/8/5p2/8/4N3/4K3 w - - 0 1", (0, 0)),
            ("4k3/8/1p6/n7/8/8/8/R3K3 w - - 0 1", (0, 0)),
            ("7k/4p3/3r4/8/2N5/8/8/K7 w - - 0 1", (20, 24)),
            ("7k/8/4p3/3n4/4P3/8/8/K7 w - - 0 1", (24, 32)),
            ("7k/8/8/3p4/4P3/4K3/8/8 w - - 0 1", (6, 10)),
            ("8/8/4k3/3q4/4P3/5K2/8/8 w - - 0 1", (60, 72)),
            ("7k/8/8/R3n3/8/8/8/4R2K w - - 0 1", (16, 20)),
        ] {
            let p = Position::from_fen(fen).unwrap();
            let actual = tactical_terms_for_position(&p);
            assert_eq!(actual.0, expected, "{fen}");
            assert_eq!(actual, reference_tactical_terms(&p), "{fen}");
            let mut mirrored = p.clone();
            for (s, &pc) in p.squares.iter().enumerate() {
                mirrored.squares[s ^ 56] = -pc;
            }
            assert_eq!(
                tactical_terms_for_position(&mirrored),
                ((-actual.0 .0, -actual.0 .1), -actual.1)
            );
        }
    }

    #[test]
    fn king_safety_rewards_shields_accounts_for_storms_and_scales_with_material() {
        for (fen, expected) in [
            ("q5k1/8/8/8/8/8/5PPP/6K1 w - - 0 1", -5),
            ("q5k1/8/8/8/8/8/5P1P/6K1 w - - 0 1", -18),
            ("q5k1/8/8/8/8/8/8/6K1 w - - 0 1", -33),
            ("n5k1/8/8/8/8/8/8/6K1 w - - 0 1", -3),
            ("6k1/8/8/8/8/8/8/6K1 w - - 0 1", 0),
            ("q5k1/8/8/8/8/6p1/5PPP/6K1 w - - 0 1", -12),
            ("6kq/8/8/8/8/8/8/K7 w - - 0 1", -26),
            ("7k/8/8/8/8/8/qqqqqqqq/4K3 w - - 0 1", -300),
        ] {
            let p = Position::from_fen(fen).unwrap();
            let actual = tactical_terms_for_position(&p);
            assert_eq!(actual.1, expected, "{fen}");
            assert_eq!(actual, reference_tactical_terms(&p), "{fen}");
            let mut mirrored = p.clone();
            for (s, &pc) in p.squares.iter().enumerate() {
                mirrored.squares[s ^ 56] = -pc;
            }
            assert_eq!(
                tactical_terms_for_position(&mirrored),
                ((-actual.0 .0, -actual.0 .1), -actual.1)
            );
        }
        assert_eq!(
            tactical_terms_for_position(&Position::default()),
            ((0, 0), 0)
        );
    }

    fn reference_piece_pawn_relationships(p: &Position) -> (i32, i32) {
        let mut total = (0, 0);
        for (s, &pc) in p.squares.iter().enumerate() {
            if !matches!(pc.abs(), 2..=4) {
                continue;
            }
            let sign = pc.signum();
            let direction = i32::from(sign);
            let (x, y) = (s as i32 % 8, s as i32 / 8);
            let own: Vec<_> = p
                .squares
                .iter()
                .enumerate()
                .filter_map(|(s, &v)| (v == sign).then_some((s as i32 % 8, s as i32 / 8)))
                .collect();
            let enemy: Vec<_> = p
                .squares
                .iter()
                .enumerate()
                .filter_map(|(s, &v)| (v == -sign).then_some((s as i32 % 8, s as i32 / 8)))
                .collect();
            let mut bonus = (0, 0);
            if pc.abs() == 4 {
                if !own.iter().any(|&(a, _)| a == x) {
                    bonus = if enemy.iter().any(|&(a, _)| a == x) {
                        (12, 8)
                    } else {
                        (24, 12)
                    };
                }
            } else {
                let rank = if sign > 0 { y + 1 } else { 8 - y };
                let supported = own
                    .iter()
                    .any(|&(a, b)| (a - x).abs() == 1 && b + direction == y);
                let challenge = enemy
                    .iter()
                    .any(|&(a, b)| (a - x).abs() == 1 && (b - y) * direction >= 1);
                if (4..=6).contains(&rank) && supported && !challenge {
                    bonus = if pc.abs() == 2 { (20, 12) } else { (12, 8) };
                }
                if own.contains(&(x, y + direction)) {
                    bonus.0 += 8;
                    bonus.1 += 4;
                }
                if pc.abs() == 3 {
                    let same = own
                        .iter()
                        .filter(|&&(a, b)| (a + b) % 2 == (x + y) % 2)
                        .count() as i32;
                    bonus.0 -= same * 2;
                    bonus.1 -= same * 3;
                }
            }
            total.0 += direction * bonus.0;
            total.1 += direction * bonus.1;
        }
        total
    }

    fn relationships_for_position(p: &Position) -> (i32, i32) {
        let (mut pawns, mut knights, mut diagonal, mut straight) = ([0; 2], [0; 2], [0; 2], [0; 2]);
        for (s, &pc) in p.squares.iter().enumerate() {
            let color = usize::from(pc > 0);
            match pc.abs() {
                1 => pawns[color] |= 1u64 << s,
                2 => knights[color] |= 1u64 << s,
                3 => diagonal[color] |= 1u64 << s,
                4 => straight[color] |= 1u64 << s,
                5 => {
                    diagonal[color] |= 1u64 << s;
                    straight[color] |= 1u64 << s;
                }
                _ => {}
            }
        }
        piece_pawn_relationships(
            pawns,
            mobility::pawn_attacks(pawns),
            knights,
            diagonal,
            straight,
        )
    }

    #[test]
    fn piece_pawn_terms_cover_outposts_pawn_colors_shelter_and_rook_files() {
        for (fen, expected) in [
            ("4k3/8/8/4N3/3P4/8/8/4K3 w - - 0 1", (20, 12)),
            ("4k3/8/4P3/4N3/3P4/8/8/4K3 w - - 0 1", (28, 16)),
            ("4k3/5p2/8/4N3/3P4/8/8/4K3 w - - 0 1", (0, 0)),
            ("4k3/8/3p4/4N3/3P4/8/8/4K3 w - - 0 1", (0, 0)),
            ("4k3/8/8/4Np2/3P4/8/8/4K3 w - - 0 1", (20, 12)),
            ("4k3/8/8/4N3/8/8/8/4K3 w - - 0 1", (0, 0)),
            ("4k3/8/8/8/8/4N3/3P4/4K3 w - - 0 1", (0, 0)),
            ("4k3/8/8/8/4N3/3P4/8/4K3 w - - 0 1", (20, 12)),
            ("4k3/8/4N3/3P4/8/8/8/4K3 w - - 0 1", (20, 12)),
            ("4k3/4N3/3P4/8/8/8/8/4K3 w - - 0 1", (0, 0)),
            ("4k3/8/8/N7/1P6/8/8/4K3 w - - 0 1", (20, 12)),
            ("4k3/8/8/N6p/1P6/8/8/4K3 w - - 0 1", (20, 12)),
            ("4k3/8/8/4B3/3P4/8/8/4K3 w - - 0 1", (10, 5)),
            ("4k3/8/4P3/4B3/3P4/8/8/4K3 w - - 0 1", (18, 9)),
            ("4k3/8/8/4B3/8/8/4P3/4K3 w - - 0 1", (0, 0)),
            ("4k3/8/8/4B3/8/8/3P4/4K3 w - - 0 1", (-2, -3)),
            ("4k3/8/8/8/8/8/8/R3K3 w - - 0 1", (24, 12)),
            ("4k3/p7/8/8/8/8/8/R3K3 w - - 0 1", (12, 8)),
            ("4k3/p7/8/8/8/8/P7/R3K3 w - - 0 1", (0, 0)),
            ("4k3/8/8/8/8/8/8/Q3K3 w - - 0 1", (0, 0)),
        ] {
            let p = Position::from_fen(fen).unwrap();
            assert_eq!(relationships_for_position(&p), expected, "{fen}");
            assert_eq!(
                relationships_for_position(&p),
                reference_piece_pawn_relationships(&p),
                "{fen}"
            );
            let mut mirrored = p.clone();
            for (s, &pc) in p.squares.iter().enumerate() {
                mirrored.squares[s ^ 56] = -pc;
            }
            assert_eq!(
                relationships_for_position(&mirrored),
                (-expected.0, -expected.1),
                "{fen}"
            );
        }
        assert_eq!(relationships_for_position(&Position::default()), (0, 0));
    }

    // Coordinate/piece-list oracle independent of production fills and masks.
    fn reference_pawn_structure(p: &Position) -> (i32, i32) {
        let mut total = (0, 0);
        for sign in [-1i8, 1] {
            let own: Vec<_> = p
                .squares
                .iter()
                .enumerate()
                .filter_map(|(s, &pc)| (pc == sign).then_some((s as i32 % 8, s as i32 / 8)))
                .collect();
            let enemy: Vec<_> = p
                .squares
                .iter()
                .enumerate()
                .filter_map(|(s, &pc)| (pc == -sign).then_some((s as i32 % 8, s as i32 / 8)))
                .collect();
            let forward = i32::from(sign);
            let mut score = (0, 0);
            for &(x, y) in &own {
                let isolated = !own.iter().any(|&(a, _)| (a - x).abs() == 1);
                let connected = own
                    .iter()
                    .any(|&(a, b)| (a - x).abs() == 1 && (b == y || b + forward == y));
                let stop = (x, y + forward);
                let blocked = own.contains(&stop) || enemy.contains(&stop);
                let threatened = enemy
                    .iter()
                    .any(|&(a, b)| (a - x).abs() == 1 && b - forward == stop.1);
                let can_support = own
                    .iter()
                    .any(|&(a, b)| (a - x).abs() == 1 && (b - y) * forward <= 0);
                let backward = !isolated && !blocked && threatened && !can_support;
                if isolated {
                    score.0 -= 12;
                    score.1 -= 18;
                }
                if connected {
                    score.0 += 6;
                    score.1 += 10;
                }
                if backward {
                    score.0 -= 8;
                    score.1 -= 12;
                }
                let frontmost = !own.iter().any(|&(a, b)| a == x && (b - y) * forward > 0);
                let passed = frontmost
                    && !enemy
                        .iter()
                        .any(|&(a, b)| (a - x).abs() <= 1 && (b - y) * forward > 0);
                if passed {
                    let rank = if sign > 0 { y } else { 7 - y } as usize;
                    score.0 += [0, 0, 4, 10, 20, 35, 60, 0][rank];
                    score.1 += [0, 5, 10, 20, 40, 70, 120, 0][rank];
                }
            }
            for file in 0..8 {
                let n = own.iter().filter(|&&(x, _)| x == file).count() as i32;
                score.0 -= (n - 1).max(0) * 10;
                score.1 -= (n - 1).max(0) * 16;
            }
            total.0 += i32::from(sign) * score.0;
            total.1 += i32::from(sign) * score.1;
        }
        total
    }

    fn pawn_structure_for_position(p: &Position) -> (i32, i32) {
        let mut pawns = [0; 2];
        for (s, &pc) in p.squares.iter().enumerate() {
            if pc.abs() == 1 {
                pawns[usize::from(pc > 0)] |= 1u64 << s;
            }
        }
        pawn_structure(pawns, mobility::pawn_attacks(pawns))
    }

    #[test]
    fn pawn_structure_distinguishes_weaknesses_connections_and_frontmost_passers() {
        for (fen, expected) in [
            ("4k3/8/8/8/8/8/8/4K3 w - - 0 1", (0, 0)),
            ("4k3/8/8/8/4P3/8/8/4K3 w - - 0 1", (-2, 2)),
            ("4k3/8/8/8/8/8/P7/4K3 w - - 0 1", (-12, -13)),
            ("4k3/8/8/8/8/P7/P7/4K3 w - - 0 1", (-30, -42)),
            ("4k3/8/8/8/8/8/PP6/4K3 w - - 0 1", (12, 30)),
            ("4k3/8/8/8/1P6/P7/8/4K3 w - - 0 1", (20, 40)),
            ("4k3/8/8/3p4/5P2/4P3/8/4K3 w - - 0 1", (20, 36)),
            ("4k3/8/8/8/8/PP6/P7/4K3 w - - 0 1", (10, 24)),
            ("4k3/8/8/3p4/4pP2/4P3/8/4K3 w - - 0 1", (18, 32)),
            ("4k3/8/8/3p4/8/4P3/8/4K3 w - - 0 1", (0, 0)),
            ("4k3/8/8/8/3pP3/8/8/4K3 w - - 0 1", (-10, -20)),
        ] {
            let p = Position::from_fen(fen).unwrap();
            assert_eq!(
                pawn_structure_for_position(&p),
                reference_pawn_structure(&p),
                "{fen}"
            );
            assert_eq!(pawn_structure_for_position(&p), expected, "{fen}");
            let mut mirrored = p.clone();
            for (s, &pc) in p.squares.iter().enumerate() {
                mirrored.squares[s ^ 56] = -pc;
            }
            assert_eq!(
                pawn_structure_for_position(&mirrored),
                (-expected.0, -expected.1)
            );
        }
        assert_eq!(pawn_structure_for_position(&Position::default()), (0, 0));
    }

    fn reference_space(p: &Position) -> i32 {
        if !p.squares.iter().any(|pc| pc.abs() == 1) {
            return 0;
        }
        let mut total = 0;
        for sign in [-1i8, 1] {
            for rank in 1..=3 {
                let y = if sign > 0 { rank } else { 7 - rank };
                for file in 2..=5 {
                    let at = (y * 8 + file) as usize;
                    let pc = p.squares[at];
                    if pc == sign || pc != 0 && pc.signum() != sign {
                        continue;
                    }
                    let attacked = p.squares.iter().enumerate().any(|(s, &enemy)| {
                        enemy == -sign
                            && (s as i32 % 8 - file).abs() == 1
                            && s as i32 / 8 + i32::from(enemy.signum()) == y
                    });
                    if attacked {
                        continue;
                    }
                    let behind = (1..=3).any(|distance| {
                        let ahead = y + distance * i32::from(sign);
                        (0..8).contains(&ahead) && p.squares[(ahead * 8 + file) as usize] == sign
                    });
                    total += i32::from(sign) * 4 * (1 + i32::from(behind));
                }
            }
        }
        total
    }

    fn space_for_position(p: &Position) -> i32 {
        let mut occupied = [0; 2];
        let mut pawns = [0; 2];
        for (square, &pc) in p.squares.iter().enumerate() {
            if pc != 0 {
                let color = usize::from(pc > 0);
                occupied[color] |= 1u64 << square;
                if pc.abs() == 1 {
                    pawns[color] |= 1u64 << square;
                }
            }
        }
        central_space(occupied, pawns, mobility::pawn_attacks(pawns))
    }

    #[test]
    fn central_space_rewards_pawn_room_and_excludes_enemy_control() {
        for (fen, expected) in [
            ("4k3/8/8/8/8/8/8/4K3 w - - 0 1", 0),
            ("4k3/8/8/8/3B4/8/P7/4K3 w - - 0 1", 0),
            ("4k3/8/8/8/3b4/8/P7/4K3 w - - 0 1", -4),
            ("4k3/8/8/4p3/8/8/8/4K3 w - - 0 1", -12),
            ("4k3/8/2P5/8/8/8/8/4K3 w - - 0 1", 16),
            ("4k3/2P5/8/8/8/8/8/4K3 w - - 0 1", 8),
            ("4k3/8/8/2P5/2P5/8/8/4K3 w - - 0 1", 16),
            ("4k3/8/8/8/P7/8/8/4K3 w - - 0 1", 0),
        ] {
            let p = Position::from_fen(fen).unwrap();
            assert_eq!(space_for_position(&p), expected, "{fen}");
            assert_eq!(space_for_position(&p), reference_space(&p));
            let mut mirrored = p.clone();
            for (square, &pc) in p.squares.iter().enumerate() {
                mirrored.squares[square ^ 56] = -pc;
            }
            assert_eq!(space_for_position(&mirrored), -expected);
        }
        // A pawn ending has a nonzero raw territory imbalance, but its phase
        // must remove the entire space term from the actual evaluation.
        let ending = Position::from_fen("4k3/8/8/8/4P3/8/8/4K3 w - - 0 1").unwrap();
        assert_eq!(space_for_position(&ending), 12);
        assert_eq!(evaluate(&ending), 134); // 132 material/position + 2 endgame pawn structure.
        let p = Position::default();
        assert_eq!(space_for_position(&p), 0);
        for (uci, expected) in [("e2e4", 16), ("d2d4", 16), ("e2e3", 4), ("b1c3", 0)] {
            let m = p
                .legal_moves()
                .into_iter()
                .find(|m| m.to_string() == uci)
                .unwrap();
            assert_eq!(space_for_position(&p.play(m)), expected, "{uci}");
        }
    }

    // Arithmetic reference independent of the compiled score tables.
    fn reference_evaluate(p: &Position) -> i32 {
        let mut phase = 0;
        let mut king_delta = 0;
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
                    let middle = -center * 5 + if y == 0 && (x == 6 || x == 2) { 35 } else { 0 };
                    king_delta += i32::from(pc.signum()) * (center * 8 - middle);
                    middle
                }
                _ => 0,
            };
            phase += match pc.abs() {
                2 | 3 => 1,
                4 => 2,
                5 => 4,
                _ => 0,
            };
            score += pc.signum() as i32 * (value(pc) + positional);
        }
        let (mg, eg) = mobility::tests::reference_scores(p);
        let space = reference_space(p);
        let (pawn_mg, pawn_eg) = reference_pawn_structure(p);
        let (piece_mg, piece_eg) = reference_piece_pawn_relationships(p);
        let ((threat_mg, threat_eg), safety) = reference_tactical_terms(p);
        score += mg + space + pawn_mg + piece_mg + threat_mg + safety;
        score += (king_delta + eg - mg - space + pawn_eg - pawn_mg + piece_eg - piece_mg
            + threat_eg
            - threat_mg
            - safety)
            * (24 - phase.min(24))
            / 24;
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

    // Complete a known depth deterministically, using the same search as ponder.
    fn at_depth(
        cache: &mut SearchCache,
        p: &Position,
        keys: &[u64],
        depth: u8,
        ply: i32,
    ) -> (i32, Option<Move>) {
        cache.table.resize(TABLE_SIZE);
        cache.table.next_generation();
        let mut search = test_search(cache, p, keys);
        search.searching_depth = depth;
        search.negamax(p, depth, ply, -INF, INF)
    }

    fn test_search<'a>(cache: &'a mut SearchCache, p: &Position, keys: &[u64]) -> Search<'a> {
        Search {
            table: &cache.table,
            buffers: &mut cache.buffers,
            keys: keys.to_vec(),
            history_context: reversible_context(p, keys),
            deadline: None,
            nodes: 0,
            hits: 0,
            aborted: false,
            cancelled: Arc::new(AtomicBool::new(false)),
            progress: None,
            started: Instant::now(),
            published: Instant::now(),
            depth: 0,
            searching_depth: 0,
            evaluation: None,
            threads: 1,
            peers: &[],
            reporting: None,
            reported: (0, 0),
            rest_at: None,
        }
    }

    #[test]
    fn pondered_reply_is_available_even_without_a_move_budget() {
        let p = Position::default();
        let mut cache = SearchCache::default();
        let (_, human_move) = at_depth(&mut cache, &p, &[p.repetition_key()], 3, 0);
        let child = p.play(human_move.unwrap());
        let keys = [p.repetition_key(), child.repetition_key()];
        let cached = cache.search(&child, &keys, Duration::ZERO, Arc::default());
        let expected = at_depth(&mut SearchCache::default(), &child, &keys, 2, 0);
        assert_eq!(cached.depth, 2);
        assert_eq!(cached.score, expected.0);
        assert!(child.legal_moves().contains(&cached.best.unwrap()));
        assert_eq!(cached.hits, 1);
        assert_eq!(cached.nodes, 0);

        // The cache must not transfer scores into a different repetition context
        // or fifty-move clock, even though the board is identical.
        let mut different_clock = child.clone();
        different_clock.halfmove += 1;
        let result = cache.search(&different_clock, &keys, Duration::ZERO, Arc::default());
        assert_eq!(result.depth, 0);
        assert_eq!(result.hits, 0);
        // A pawn move resets the history window; use a reversible child instead.
        let knight = p.play(
            p.legal_moves()
                .into_iter()
                .find(|m| m.to_string() == "g1f3")
                .unwrap(),
        );
        let knight_keys = [p.repetition_key(), knight.repetition_key()];
        at_depth(&mut cache, &knight, &knight_keys, 2, 0);
        let result = cache.search(
            &knight,
            &[123, knight.repetition_key()],
            Duration::ZERO,
            Arc::default(),
        );
        assert_eq!(result.depth, 0);
        assert_eq!(result.hits, 0);
    }

    #[test]
    fn cached_mate_distance_is_relative_to_the_new_root() {
        let mut game = Game::default();
        for uci in ["f2f3", "e7e5", "g2g4"] {
            let m = game
                .position()
                .legal_moves()
                .into_iter()
                .find(|m| m.to_string() == uci)
                .unwrap();
            game.play(m).unwrap();
        }
        let mut cache = SearchCache::default();
        let (score, _) = at_depth(&mut cache, game.position(), &game.keys, 1, 3);
        let result = cache.search(game.position(), &game.keys, Duration::ZERO, Arc::default());
        assert_eq!(score, MATE - 4);
        assert_eq!(result.score, MATE - 1);
        assert_eq!(result.best.unwrap().to_string(), "d8h4");
    }

    #[test]
    fn pondering_keeps_searching_until_cancelled() {
        let p = Position::default();
        let cancel = Arc::new(AtomicBool::new(false));
        let worker_cancel = cancel.clone();
        let (started_tx, started_rx) = mpsc::channel();
        let (done_tx, done_rx) = mpsc::channel();
        let worker = thread::spawn(move || {
            started_tx.send(()).unwrap();
            let result = SearchCache::default().ponder(&p, &[p.repetition_key()], worker_cancel);
            done_tx.send(result).unwrap();
        });
        started_rx.recv_timeout(Duration::from_secs(2)).unwrap();
        let before_cancel = done_rx.recv_timeout(Duration::from_millis(30));
        cancel.store(true, Ordering::Relaxed);
        let result = done_rx.recv_timeout(Duration::from_secs(2)).unwrap();
        worker.join().unwrap();
        assert!(matches!(
            before_cancel,
            Err(mpsc::RecvTimeoutError::Timeout)
        ));
        assert!(Position::default()
            .legal_moves()
            .contains(&result.best.unwrap()));
    }

    #[test]
    fn live_progress_advances_during_pondering_and_finishes_coherently() {
        let progress = Arc::new(Mutex::new(SearchProgress::default()));
        let cancel = Arc::new(AtomicBool::new(false));
        let worker_progress = progress.clone();
        let worker_cancel = cancel.clone();
        let worker = thread::spawn(move || {
            let p = Position::default();
            SearchCache::default().ponder_with_progress(
                &p,
                &[p.repetition_key()],
                worker_cancel,
                &worker_progress,
            )
        });
        let deadline = Instant::now() + Duration::from_secs(3);
        let mut snapshots = Vec::new();
        while Instant::now() < deadline && snapshots.len() < 2 {
            let snapshot = *progress.lock().unwrap();
            if snapshot.nodes > snapshots.last().map_or(0, |s: &SearchProgress| s.nodes) {
                snapshots.push(snapshot);
            }
            thread::sleep(Duration::from_millis(5));
        }
        cancel.store(true, Ordering::Relaxed);
        let result = worker.join().unwrap();
        assert_eq!(snapshots.len(), 2, "No live progress while pondering");
        assert!(snapshots[1].elapsed > snapshots[0].elapsed);
        assert!(snapshots[1].searching_depth > snapshots[1].depth);
        let final_snapshot = *progress.lock().unwrap();
        assert_eq!(final_snapshot.nodes, result.nodes);
        assert_eq!(final_snapshot.hits, result.hits);
        assert_eq!(final_snapshot.depth, result.depth);
        assert_eq!(final_snapshot.searching_depth, 0);
    }
}
