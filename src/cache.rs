//! Bounded, incrementally growing transposition storage. No OS-specific paths.
use super::{Entry, PackedEntry, BUCKET_SIZE, STALE_AGE};
use std::sync::{
    atomic::{AtomicUsize, Ordering},
    Mutex,
};

const PAGE_BUCKETS: usize = 1024;
const PAGE_BITS: u32 = 10;
const SHARDS: usize = 64;
const START_BYTES: usize = 4 * 1024 * 1024;
// Include page descriptors, their spare capacity, and allocation bookkeeping.
const PAGE_COST: usize = PAGE_BYTES + 256;
const PAGE_BYTES: usize = PAGE_BUCKETS * std::mem::size_of::<Bucket>();
const GROW_COLLISIONS: usize = 128;

/// One cache line holds four full keys, four six-byte payloads, and an epoch.
/// The epoch lets us retire stale entries on access without sweeping the table.
#[repr(align(64))]
#[derive(Clone, Copy, Default)]
pub(super) struct Bucket {
    pub(super) keys: [u64; BUCKET_SIZE],
    pub(super) entries: [PackedEntry; BUCKET_SIZE],
    epoch: u64,
}
impl Bucket {
    fn age_to(&mut self, epoch: u64) {
        if self.epoch == epoch {
            return;
        }
        let elapsed = epoch.wrapping_sub(self.epoch);
        for entry in &mut self.entries {
            if !entry.is_empty()
                && (elapsed >= u64::from(STALE_AGE)
                    || u64::from(entry.age(self.epoch as u8)).saturating_add(elapsed)
                        >= u64::from(STALE_AGE))
            {
                *entry = PackedEntry::default();
            }
        }
        self.epoch = epoch;
    }
    fn probe(&mut self, key: u64, epoch: u64) -> Option<Entry> {
        self.age_to(epoch);
        for (&stored, packed) in self.keys.iter().zip(&mut self.entries) {
            if stored == key {
                if let Some(entry) = packed.unpack() {
                    packed.refresh(epoch as u8);
                    return Some(entry);
                }
            }
        }
        None
    }
    // Return collision pressure even when a shallow incoming result is rejected.
    fn store(&mut self, key: u64, entry: Entry, epoch: u64) -> bool {
        let Some(packed) = PackedEntry::pack(entry, epoch as u8) else {
            return false;
        };
        self.age_to(epoch);
        let mut empty = None;
        let (mut victim, mut weakest) = (0, i16::MAX);
        for (index, (&stored, old)) in self.keys.iter().zip(&mut self.entries).enumerate() {
            if old.is_empty() {
                empty.get_or_insert(index);
            } else if stored == key {
                if packed.improves(*old) {
                    *old = packed;
                }
                old.refresh(epoch as u8);
                return false;
            } else {
                let rank = old.retention(epoch as u8);
                if rank < weakest {
                    victim = index;
                    weakest = rank;
                }
            }
        }
        let index = match empty {
            Some(index) => index,
            None if packed.retention(epoch as u8) >= weakest => victim,
            None => return true,
        };
        self.keys[index] = key;
        self.entries[index] = packed;
        empty.is_none()
    }
}

#[derive(Default)]
struct Shard {
    pages: Vec<Vec<Bucket>>,
    base: usize,
    split: usize,
    collisions: usize,
}
impl Shard {
    fn location(&self, key: u64, shard_bits: u32) -> (usize, usize) {
        let hash = (key >> shard_bits) as usize;
        let page_hash = hash >> PAGE_BITS;
        let mut page = page_hash & (self.base - 1);
        if page < self.split {
            page = page_hash & (self.base * 2 - 1);
        }
        (page, hash & (PAGE_BUCKETS - 1))
    }
    fn grow(&mut self, owner: &SharedTable) -> bool {
        self.collisions = 0;
        if !owner.reserve_page() {
            return false;
        }
        let mut page = Vec::new();
        if page.try_reserve_exact(PAGE_BUCKETS).is_err() || self.pages.try_reserve(1).is_err() {
            owner.allocated.fetch_sub(PAGE_COST, Ordering::Relaxed);
            return false;
        }
        page.resize(PAGE_BUCKETS, Bucket::default());
        if self.pages.is_empty() {
            self.base = 1;
        } else {
            // Linear hashing splits only one 64 KiB page. Existing addresses
            // remain valid without rehashing or copying the rest of the table.
            let old = &mut self.pages[self.split];
            for (offset, bucket) in old.iter_mut().enumerate() {
                bucket.age_to(owner.epoch);
                for slot in 0..BUCKET_SIZE {
                    let key = bucket.keys[slot];
                    let page_hash = (key >> (owner.shard_bits + PAGE_BITS)) as usize;
                    if !bucket.entries[slot].is_empty() && page_hash & self.base != 0 {
                        page[offset].keys[slot] = key;
                        page[offset].entries[slot] = bucket.entries[slot];
                        page[offset].epoch = owner.epoch;
                        bucket.entries[slot] = PackedEntry::default();
                    }
                }
            }
            self.split += 1;
            if self.split == self.base {
                self.base *= 2;
                self.split = 0;
            }
        }
        self.pages.push(page);
        true
    }
}

pub(super) struct SharedTable {
    shards: Vec<Mutex<Shard>>,
    shard_bits: u32,
    epoch: u64,
    limit: usize,
    allocated: AtomicUsize,
}
impl Default for SharedTable {
    fn default() -> Self {
        Self::with_limit(START_BYTES + 256 * 64)
    }
}
impl SharedTable {
    fn reserve_page(&self) -> bool {
        let mut used = self.allocated.load(Ordering::Relaxed);
        loop {
            let Some(next) = used
                .checked_add(PAGE_COST)
                .filter(|&next| next <= self.limit)
            else {
                return false;
            };
            match self.allocated.compare_exchange_weak(
                used,
                next,
                Ordering::Relaxed,
                Ordering::Relaxed,
            ) {
                Ok(_) => return true,
                Err(current) => used = current,
            }
        }
    }
    pub(super) fn with_limit(limit: usize) -> Self {
        Self {
            shards: Vec::new(),
            shard_bits: 0,
            epoch: 0,
            limit,
            allocated: AtomicUsize::new(0),
        }
    }
    pub(super) fn initialize(&mut self) {
        if !self.shards.is_empty() || self.limit < PAGE_COST {
            return;
        }
        let pages = (self.limit / PAGE_COST).min(START_BYTES / PAGE_BYTES);
        let count = 1usize << pages.min(SHARDS).ilog2();
        self.shard_bits = count.ilog2();
        self.shards = (0..count).map(|_| Mutex::new(Shard::default())).collect();
        // Initialization is bounded to 4 MiB, independent of the memory cap.
        for index in 0..pages {
            let mut shard = self.shards[index % count].lock().expect("cache poisoned");
            if !shard.grow(self) {
                break;
            }
        }
    }
    pub(super) fn next_generation(&mut self) {
        self.epoch = self.epoch.wrapping_add(1);
    }
    pub(super) fn bytes(&self) -> usize {
        self.allocated.load(Ordering::Relaxed)
    }
    pub(super) fn limit(&self) -> usize {
        self.limit
    }
    pub(super) fn probe(&self, key: u64) -> Option<Entry> {
        if self.shards.is_empty() {
            return None;
        }
        let mut shard = self.shards[key as usize & (self.shards.len() - 1)]
            .try_lock()
            .ok()?;
        if shard.pages.is_empty() {
            return None;
        }
        let (page, bucket) = shard.location(key, self.shard_bits);
        shard.pages[page][bucket].probe(key, self.epoch)
    }
    pub(super) fn store(&self, key: u64, entry: Entry) {
        if self.shards.is_empty() {
            return;
        }
        if let Ok(mut shard) = self.shards[key as usize & (self.shards.len() - 1)].try_lock() {
            if shard.pages.is_empty() && !shard.grow(self) {
                return;
            }
            let (page, bucket) = shard.location(key, self.shard_bits);
            if shard.pages[page][bucket].store(key, entry, self.epoch) {
                shard.collisions += 1;
                if shard.collisions >= GROW_COLLISIONS {
                    shard.grow(self);
                }
            }
        }
    }
    #[cfg(test)]
    pub(super) fn resize(&mut self, slots: usize) {
        let pages = (slots * 16 + PAGE_BYTES - 1) / PAGE_BYTES;
        let limit = pages * PAGE_COST;
        if self.limit != limit || self.shards.is_empty() {
            *self = Self::with_limit(limit);
            self.initialize();
        }
    }
}

// The replacement-policy tests use tiny tables and the same production buckets.
#[cfg(test)]
#[derive(Default)]
pub(super) struct TestTable {
    pub(super) buckets: Vec<Bucket>,
    pub(super) generation: u64,
}
#[cfg(test)]
impl TestTable {
    pub(super) fn resize(&mut self, slots: usize) {
        self.buckets.resize(slots / BUCKET_SIZE, Bucket::default());
    }
    pub(super) fn next_generation(&mut self) {
        self.generation = self.generation.wrapping_add(1);
    }
    pub(super) fn probe(&mut self, key: u64) -> Option<Entry> {
        let index = key as usize % self.buckets.len();
        self.buckets[index].probe(key, self.generation)
    }
    pub(super) fn store(&mut self, key: u64, entry: Entry) {
        let index = key as usize % self.buckets.len();
        self.buckets[index].store(key, entry, self.generation);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::engine::Bound;

    fn entry(score: i32) -> Entry {
        Entry {
            depth: 8,
            score,
            best: None,
            bound: Bound::Exact,
        }
    }

    #[test]
    fn incremental_page_splits_preserve_keys_scores_and_ages() {
        let mut table = SharedTable::with_limit(256 * PAGE_COST);
        table.initialize();
        table.next_generation();
        let mut keys = Vec::new();
        for tag in 0..4 {
            for offset in 0..PAGE_BUCKETS {
                let key = ((tag * PAGE_BUCKETS + offset) as u64) << table.shard_bits;
                table.store(key, entry((offset + tag) as i32));
                keys.push((key, entry((offset + tag) as i32)));
            }
        }
        for _ in 0..8 {
            {
                let mut shard = table.shards[0].lock().unwrap();
                let before = shard.pages.len();
                assert!(shard.grow(&table));
                assert_eq!(shard.pages.len(), before + 1);
            }
            for &(key, expected) in &keys {
                assert_eq!(table.probe(key), Some(expected));
            }
            assert!(table.bytes() <= table.limit());
        }
        for _ in 0..128 {
            table.next_generation();
        }
        assert!(keys.iter().all(|&(key, _)| table.probe(key).is_none()));
    }

    #[test]
    fn concurrent_growth_never_exceeds_the_shared_budget() {
        let limit = 80 * PAGE_COST;
        let mut table = SharedTable::with_limit(limit);
        table.initialize();
        std::thread::scope(|scope| {
            for worker in 0..8 {
                let table = &table;
                scope.spawn(move || {
                    for tag in 0..30_000 {
                        let key = ((tag as u64) << (table.shard_bits + PAGE_BITS)) | worker;
                        table.store(key, entry(tag % 1000));
                        if let Some(e) = table.probe(key) {
                            assert_eq!(e.score, tag % 1000);
                        }
                        assert!(table.bytes() <= limit);
                    }
                });
            }
        });
        assert!(table.bytes() > START_BYTES);
        let pages: usize = table
            .shards
            .iter()
            .map(|s| s.lock().unwrap().pages.len())
            .sum();
        assert_eq!(table.bytes(), pages * PAGE_COST);
        assert!(table.bytes() <= limit);
    }

    #[test]
    fn large_limits_do_not_preallocate_or_scan_large_tables() {
        for limit in [100_000_000, 250_000_000, 500_000_000, 1_000_000_000] {
            let mut table = SharedTable::with_limit(limit);
            assert_eq!(table.bytes(), 0);
            table.initialize();
            let initial = table.bytes();
            assert_eq!(initial, START_BYTES / PAGE_BYTES * PAGE_COST);
            table.store(0, entry(42));
            // An untouched bucket must expire correctly even after many wraps
            // of its compact six-bit entry age. Advancing epochs is O(1).
            for _ in 0..65_536 {
                table.next_generation();
            }
            assert_eq!(table.bytes(), initial);
            assert_eq!(table.probe(0), None);
        }
    }

    #[test]
    fn insufficient_cache_budget_falls_back_to_uncached_search() {
        let mut table = SharedTable::with_limit(PAGE_COST - 1);
        table.initialize();
        table.store(0, entry(42));
        assert_eq!(table.probe(0), None);
        assert_eq!(table.bytes(), 0);
    }
}
