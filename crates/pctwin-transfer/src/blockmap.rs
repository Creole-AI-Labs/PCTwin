use std::collections::BTreeMap;
use std::ops::Range;

use crate::TransferError;

/// The most runs of done blocks one resume message carries (16 bytes each, so the message stays
/// well inside the link's 64 KiB limit). A map with more sends its earliest runs; blocks left out
/// are simply sent again and recognised as already there.
pub const MAX_TICKET_RUNS: usize = 2048;

/// A block number outside the file.
#[derive(Debug, Clone, Copy, PartialEq, Eq, thiserror::Error)]
#[error("block {0} is outside the file")]
pub struct BlockOutside(pub u64);

/// Which blocks of a file are done, kept as runs (start to end) that are joined when they touch,
/// so it stays small however a file's sections were split (the idea of aria2's control file).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct BlockMap {
    count: u64,
    /// Start to end of each run; runs never touch or overlap.
    runs: BTreeMap<u64, u64>,
    done: u64,
}

impl BlockMap {
    /// A file of `count` blocks with none done.
    pub fn new(count: u64) -> Self {
        Self {
            count,
            runs: BTreeMap::new(),
            done: 0,
        }
    }

    /// The file's number of blocks.
    pub fn count(&self) -> u64 {
        self.count
    }

    /// How many blocks are done.
    pub fn done_count(&self) -> u64 {
        self.done
    }

    pub fn is_full(&self) -> bool {
        self.done == self.count
    }

    pub fn contains(&self, block: u64) -> bool {
        self.runs
            .range(..=block)
            .next_back()
            .is_some_and(|(_, end)| *end > block)
    }

    /// Marks `block` done; `Ok(false)` if it already was.
    pub fn insert(&mut self, block: u64) -> Result<bool, BlockOutside> {
        if block >= self.count {
            return Err(BlockOutside(block));
        }
        if self.contains(block) {
            return Ok(false);
        }
        let joins_before = self
            .runs
            .range(..block)
            .next_back()
            .filter(|(_, end)| **end == block)
            .map(|(start, _)| *start);
        let joins_after = self.runs.remove(&(block + 1));
        self.runs.insert(
            joins_before.unwrap_or(block),
            joins_after.unwrap_or(block + 1),
        );
        self.done += 1;
        Ok(true)
    }

    /// The runs of done blocks, in order.
    pub fn runs(&self) -> impl Iterator<Item = Range<u64>> + '_ {
        self.runs.iter().map(|(s, e)| *s..*e)
    }

    /// The runs of blocks still needed, in order.
    pub fn missing(&self) -> impl Iterator<Item = Range<u64>> + '_ {
        let mut at = 0;
        let mut done = self.runs.iter();
        std::iter::from_fn(move || {
            loop {
                match done.next() {
                    Some((&s, &e)) => {
                        let gap = at..s;
                        at = e;
                        if !gap.is_empty() {
                            return Some(gap);
                        }
                    }
                    None => {
                        let gap = at..self.count;
                        at = self.count;
                        return (!gap.is_empty()).then_some(gap);
                    }
                }
            }
        })
    }

    /// One entry per block: done or not. For a map read from the other laptop, check its count
    /// matches the file first: the count decides how much memory this takes.
    pub fn to_bools(&self) -> Vec<bool> {
        let mut out = vec![false; usize::try_from(self.count).unwrap_or(0)];
        for r in self.runs() {
            for b in r {
                if let Some(d) = usize::try_from(b).ok().and_then(|i| out.get_mut(i)) {
                    *d = true;
                }
            }
        }
        out
    }

    /// The wire form: the block count, then up to [`MAX_TICKET_RUNS`] runs, earliest first.
    pub fn encode(&self) -> Vec<u8> {
        let sent: Vec<(u64, u64)> = self
            .runs
            .iter()
            .take(MAX_TICKET_RUNS)
            .map(|(s, e)| (*s, *e))
            .collect();
        let mut w = Vec::with_capacity(12 + 16 * sent.len());
        w.extend_from_slice(&self.count.to_be_bytes());
        // At most MAX_TICKET_RUNS, which fits in a u32.
        w.extend_from_slice(&u32::try_from(sent.len()).unwrap_or(0).to_be_bytes());
        for (s, e) in sent {
            w.extend_from_slice(&s.to_be_bytes());
            w.extend_from_slice(&e.to_be_bytes());
        }
        w
    }

    /// Reads a map, refusing runs that are out of order, empty, touching or overlapping, past the
    /// end of the file, too many, cut short, or followed by bytes left over.
    pub fn decode(bytes: &[u8]) -> Result<Self, TransferError> {
        let bad = |why: &str| TransferError::Damaged(format!("block map: {why}"));
        let count = u64::from_be_bytes(
            bytes
                .get(..8)
                .and_then(|b| b.try_into().ok())
                .ok_or_else(|| bad("cut short"))?,
        );
        let n = u32::from_be_bytes(
            bytes
                .get(8..12)
                .and_then(|b| b.try_into().ok())
                .ok_or_else(|| bad("cut short"))?,
        ) as usize;
        if n > MAX_TICKET_RUNS {
            return Err(bad("too many runs"));
        }
        if bytes.len() != 12 + 16 * n {
            return Err(bad("wrong length"));
        }
        let mut map = Self::new(count);
        let mut last_end: Option<u64> = None;
        for run in bytes[12..].as_chunks::<16>().0 {
            let (s, e) = run.split_at(8);
            let start = u64::from_be_bytes(s.try_into().map_err(|_| bad("cut short"))?);
            let end = u64::from_be_bytes(e.try_into().map_err(|_| bad("cut short"))?);
            if start >= end || end > count || last_end.is_some_and(|e| start <= e) {
                return Err(bad("runs do not add up"));
            }
            map.runs.insert(start, end);
            map.done += end - start;
            last_end = Some(end);
        }
        Ok(map)
    }
}
