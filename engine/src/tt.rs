//! Fixed-size transposition table keyed by Zobrist hash.

use crate::moves::Move;

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum Bound {
    Exact,
    Lower, // fail-high: true score >= stored
    Upper, // fail-low: true score <= stored
}

#[derive(Clone, Copy)]
pub struct Entry {
    pub key: u64,
    pub best: Move,
    pub score: i32,
    pub depth: u8,
    pub bound: Bound,
}

pub struct TranspositionTable {
    entries: Vec<Entry>,
    mask: usize,
}

impl TranspositionTable {
    /// Allocate a table of roughly `size_mb` megabytes (rounded down to a power
    /// of two number of entries).
    pub fn new(size_mb: usize) -> TranspositionTable {
        let entry_size = std::mem::size_of::<Entry>();
        let count = ((size_mb * 1024 * 1024) / entry_size).max(1024);
        let count = count.next_power_of_two();
        let empty = Entry {
            key: 0,
            best: Move(0),
            score: 0,
            depth: 0,
            bound: Bound::Exact,
        };
        TranspositionTable {
            entries: vec![empty; count],
            mask: count - 1,
        }
    }

    pub fn clear(&mut self) {
        for e in self.entries.iter_mut() {
            e.key = 0;
        }
    }

    #[inline]
    pub fn probe(&self, key: u64) -> Option<&Entry> {
        let e = &self.entries[(key as usize) & self.mask];
        if e.key == key {
            Some(e)
        } else {
            None
        }
    }

    pub fn store(&mut self, key: u64, depth: u8, score: i32, bound: Bound, best: Move) {
        let slot = &mut self.entries[(key as usize) & self.mask];
        // Depth-preferred replacement; always replace a different position.
        if slot.key != key || depth >= slot.depth {
            *slot = Entry {
                key,
                best,
                score,
                depth,
                bound,
            };
        }
    }
}
