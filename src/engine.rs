//! Iterative-deepening negamax (minimax), alpha-beta pruning, and a Zobrist
//! transposition table. Root moves are searched in parallel, away from the UI.
use crate::chess::{random_key, Move, MoveState, Position};
use std::sync::{
    atomic::{AtomicBool, AtomicI32, AtomicU64, AtomicUsize, Ordering},
    mpsc, Arc, Mutex,
};
use std::thread;
use std::time::{Duration, Instant};

const MATE: i32 = 30_000;
const INF: i32 = 32_000;
const TABLE_SIZE: usize = 2_000_000;
const BUCKET_SIZE: usize = 4;
const GENERATION_MASK: u8 = 63;
const STALE_AGE: u8 = 32;
const SWEEP_PARTS: usize = 16;
const EXACT_BONUS: i16 = 2;
const AGE_PENALTY: i16 = 4;
// Retained scratch is capped independently of move count or search depth.
const MOVE_BUFFER_LIMIT: usize = 512 * 1024;
const BUFFER_PLIES: usize = 256 + 10;
const TABLE_SHARDS: usize = 1024;
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

/// Parallel arrays avoid padding each six-byte entry to an eight-byte key's
/// alignment. Each logical slot uses 8 + 6 bytes with aligned key accesses.
#[derive(Default)]
struct Table {
    keys: Vec<u64>,
    entries: Vec<PackedEntry>,
    generation: u8,
}
impl Table {
    fn resize(&mut self, len: usize) {
        assert!(len > 0 && len % BUCKET_SIZE == 0);
        self.keys.resize(len, 0);
        self.entries.resize(len, PackedEntry::default());
    }
    fn next_generation(&mut self) {
        self.generation = self.generation.wrapping_add(1) & GENERATION_MASK;
        // Visit every entry within 16 searches and retire unused entries at
        // age 32. This prevents a 64-generation wrap from making them young.
        let part = self.generation as usize % SWEEP_PARTS;
        let start = self.entries.len() * part / SWEEP_PARTS;
        let end = self.entries.len() * (part + 1) / SWEEP_PARTS;
        for entry in &mut self.entries[start..end] {
            if !entry.is_empty() && entry.age(self.generation) >= STALE_AGE {
                *entry = PackedEntry::default();
            }
        }
    }
    #[cfg(test)]
    fn bucket_start(&self, key: u64) -> usize {
        // The production size is constant, allowing a multiply/shift reduction
        // instead of a variable integer division. Small tables support tests.
        if self.keys.len() == TABLE_SIZE {
            key as usize % (TABLE_SIZE / BUCKET_SIZE) * BUCKET_SIZE
        } else {
            key as usize % (self.keys.len() / BUCKET_SIZE) * BUCKET_SIZE
        }
    }
    #[cfg(test)]
    fn probe(&mut self, key: u64) -> Option<Entry> {
        let start = self.bucket_start(key);
        self.probe_at(start, key)
    }
    fn probe_at(&mut self, start: usize, key: u64) -> Option<Entry> {
        let keys = &self.keys[start..start + BUCKET_SIZE];
        let entries = &mut self.entries[start..start + BUCKET_SIZE];
        for (&stored, packed) in keys.iter().zip(entries) {
            if stored == key {
                if let Some(entry) = packed.unpack() {
                    packed.refresh(self.generation);
                    return Some(entry);
                }
            }
        }
        None
    }
    #[cfg(test)]
    fn store(&mut self, key: u64, entry: Entry) {
        self.store_at(self.bucket_start(key), key, entry);
    }
    fn store_at(&mut self, start: usize, key: u64, entry: Entry) {
        let Some(packed) = PackedEntry::pack(entry, self.generation) else {
            return;
        };
        let mut empty = None;
        let mut victim = start;
        let mut weakest = i16::MAX;
        let keys = &self.keys[start..start + BUCKET_SIZE];
        let entries = &mut self.entries[start..start + BUCKET_SIZE];
        for (offset, (&stored, old)) in keys.iter().zip(entries).enumerate() {
            let index = start + offset;
            if old.is_empty() {
                empty.get_or_insert(index);
            } else if stored == key {
                // Keep deeper results for this exact position, and prefer an
                // exact score (or a tighter bound) when depth is equal.
                if packed.improves(*old) {
                    *old = packed;
                }
                old.refresh(self.generation);
                return;
            } else {
                let retention = old.retention(self.generation);
                if retention < weakest {
                    victim = index;
                    weakest = retention;
                }
            }
        }
        let index = match empty {
            Some(index) => index,
            None if packed.retention(self.generation) >= weakest => victim,
            None => return,
        };
        self.keys[index] = key;
        self.entries[index] = packed;
    }
}

/// Fine-grained locks keep full keys and packed entries coherent. A busy shard
/// is skipped: cache access must never make a search wait for another search.
/// The underlying slots retain their compact 14-byte representation.
#[derive(Default)]
struct SharedTable {
    shards: Vec<Mutex<Table>>,
    len: usize,
}
impl SharedTable {
    fn resize(&mut self, len: usize) {
        if self.len == len {
            return;
        }
        assert!(len > 0 && len % BUCKET_SIZE == 0);
        let buckets = len / BUCKET_SIZE;
        let count = buckets.min(TABLE_SHARDS);
        self.shards = (0..count)
            .map(|shard| {
                let mut table = Table::default();
                table.resize(((buckets - 1 - shard) / count + 1) * BUCKET_SIZE);
                Mutex::new(table)
            })
            .collect();
        self.len = len;
    }
    fn next_generation(&mut self) {
        for shard in &mut self.shards {
            shard
                .get_mut()
                .expect("search table poisoned")
                .next_generation();
        }
    }
    fn location(&self, key: u64) -> (usize, usize) {
        if self.len == TABLE_SIZE {
            let bucket = key as usize % (TABLE_SIZE / BUCKET_SIZE);
            return (bucket % TABLE_SHARDS, bucket / TABLE_SHARDS * BUCKET_SIZE);
        }
        let bucket = key as usize % (self.len / BUCKET_SIZE);
        let shard = bucket % self.shards.len();
        (shard, bucket / self.shards.len() * BUCKET_SIZE)
    }
    fn probe(&self, key: u64) -> Option<Entry> {
        let (shard, start) = self.location(key);
        self.shards[shard].try_lock().ok()?.probe_at(start, key)
    }
    fn store(&self, key: u64, entry: Entry) {
        let (shard, start) = self.location(key);
        if let Ok(mut table) = self.shards[shard].try_lock() {
            table.store_at(start, key, entry);
        }
    }
}

/// Keep at least one logical CPU, and roughly a quarter on larger machines,
/// available to the browser and other applications. No OS-specific policy.
fn search_threads(cpus: usize) -> usize {
    cpus.saturating_sub(cpus.saturating_add(3) / 4).max(1)
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
        if ply < self.slots.len() && self.retained_bytes + bytes <= MOVE_BUFFER_LIMIT {
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
}
/// A coherent snapshot of one search, including an unfinished depth.
#[derive(Clone, Copy, Debug, Default)]
pub struct SearchProgress {
    pub depth: u8,
    pub searching_depth: u8,
    pub nodes: u64,
    pub hits: u64,
    pub elapsed: Duration,
    pub threads: usize,
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
    /// Default reserves CPU capacity automatically; one is useful for comparisons.
    pub fn with_threads(threads: usize) -> Self {
        Self {
            table: SharedTable::default(),
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
        self.table.resize(TABLE_SIZE);
        self.table.next_generation();
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
        let key = self.table_key(position);
        let mut moves = legal.to_vec();
        order(
            position,
            &mut moves,
            self.probe(key).and_then(|entry| entry.best),
        );
        let first = position.play(moves[0]);
        let previous = self.push_position(&first);
        let first_score = -self.negamax(&first, depth - 1, 1, -INF, INF).0;
        self.pop_position(previous);
        if self.aborted {
            return (0, None);
        }
        let work = RootWork {
            position,
            moves: &moves,
            depth,
            next: AtomicUsize::new(1),
            alpha: AtomicI32::new(first_score),
            aborted: AtomicBool::new(false),
            scores: (0..moves.len()).map(|_| Mutex::new(None)).collect(),
        };
        thread::scope(|scope| {
            let mut handles = Vec::new();
            let (done_tx, done_rx) = mpsc::channel();
            for (index, (buffers, stats)) in helpers.iter_mut().zip(self.peers).enumerate() {
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
                score: to_table(score, 0),
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
            let mut score = -self.negamax(&next, work.depth - 1, 1, -alpha - 1, -alpha).0;
            if !self.aborted && score > alpha {
                score = -self.negamax(&next, work.depth - 1, 1, -INF, -alpha).0;
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
            if force || now.duration_since(self.published) >= Duration::from_millis(250) {
                // Never wait for a reader, and never take the game's mutex.
                if let Ok(mut snapshot) = progress.try_lock() {
                    let (nodes, hits) = self.totals();
                    *snapshot = SearchProgress {
                        depth: self.depth,
                        searching_depth: self.searching_depth,
                        nodes,
                        hits,
                        elapsed: now.duration_since(self.started),
                        threads: self.threads,
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
        order(p, moves, entry.and_then(|e| e.best));
        let mut best = None;
        let mut value = -INF;
        for &m in moves.iter() {
            let next = p.play(m);
            let previous = self.push_position(&next);
            let score = -self.negamax(&next, depth - 1, ply + 1, -beta, -alpha).0;
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
            return evaluate(p);
        }
        if !state.in_check {
            let stand = evaluate(p);
            if stand >= beta {
                return stand;
            }
            alpha = alpha.max(stand);
        }
        order(p, moves, None);
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
    // Most moves share the quiet-move priority. Stable insertion keeps their
    // existing order without allocating cached keys or a sorting buffer.
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
static SQUARE_SCORES: [[i16; 64]; 13] = evaluation_table();
static KING_ENDGAME_DELTAS: [[i16; 64]; 2] = king_deltas();

fn evaluate(p: &Position) -> i32 {
    let (mut score, mut endgame_delta, mut majors) = (0, 0, 0);
    let mut bishops = [0, 0];
    for (square, &piece) in p.squares.iter().enumerate() {
        if piece == 0 {
            continue;
        }
        score += i32::from(SQUARE_SCORES[(piece + 6) as usize][square]);
        match piece.abs() {
            3 => bishops[usize::from(piece < 0)] += 1,
            4 | 5 => majors += 1,
            6 => endgame_delta += i32::from(KING_ENDGAME_DELTAS[usize::from(piece > 0)][square]),
            _ => {}
        }
    }
    if majors <= 2 {
        score += endgame_delta;
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

#[cfg(test)]
mod tests {
    use super::*;
    use crate::game::Game;
    use std::{sync::mpsc, thread};

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
        ] {
            assert_eq!(search_threads(cpus), threads);
            if cpus > 1 {
                assert!(threads < cpus);
                assert!(threads * 4 <= cpus * 3);
            }
        }
    }

    #[test]
    fn shared_table_keeps_compact_slots_and_coherent_concurrent_entries() {
        let mut table = SharedTable::default();
        table.resize(TABLE_SIZE);
        table.next_generation();
        let mut bytes = 0;
        let mut slots = 0;
        for shard in &mut table.shards {
            let shard = shard.get_mut().unwrap();
            slots += shard.keys.len();
            bytes +=
                shard.keys.len() * 8 + shard.entries.len() * std::mem::size_of::<PackedEntry>();
        }
        assert_eq!(slots, TABLE_SIZE);
        assert_eq!(bytes, 28_000_000);
        thread::scope(|scope| {
            for worker in 0..4 {
                let table = &table;
                scope.spawn(move || {
                    for step in 0..2000 {
                        // All writers contend for one bucket. Busy accesses
                        // may be skipped, but a returned full key must match.
                        let key = (worker * 2000 + step) * (TABLE_SIZE / BUCKET_SIZE) as u64;
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
    fn parallel_search_matches_serial_scores_and_returns_a_proven_best_move() {
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
                if depth >= 3 {
                    assert!(peers
                        .iter()
                        .any(|peer| peer.nodes.load(Ordering::Relaxed) > 0));
                }
                // The root score is cached only after all root moves finish.
                let cached = search.probe(search.table_key(&p)).unwrap();
                assert!(matches!(cached.bound, Bound::Exact));
                assert_eq!(cached.depth, depth);
                assert_eq!(cached.score, score);
            }
        }
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
        assert!(buffers.retained_bytes <= MOVE_BUFFER_LIMIT);
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
                assert_eq!(evaluate(&p), reference_evaluate(&p));
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
        let bytes = table.keys.capacity() * size_of::<u64>()
            + table.entries.capacity() * size_of::<PackedEntry>();
        assert_eq!(TABLE_SIZE, 2_000_000);
        assert_eq!(bytes, 28_000_000);
        println!("Cache: 14 bytes per slot, {bytes} allocated bytes for {TABLE_SIZE} slots");
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
        table.entries[0] = PackedEntry::default();
        table.store(1, cache_entry(10, Bound::Exact));
        assert_eq!(table.probe(1), Some(cache_entry(10, Bound::Exact)));
        assert_eq!(
            (0..BUCKET_SIZE)
                .filter(|&i| table.keys[i] == 1 && !table.entries[i].is_empty())
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
        assert_eq!(table.generation, 0);
    }

    // Original arithmetic evaluator, independent of the compiled score tables.
    fn reference_evaluate(p: &Position) -> i32 {
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
