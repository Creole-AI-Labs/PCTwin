use std::ops::Range;

/// The smallest section a big file is cut into. A range is split only when at least twice this
/// much is left to send (the aria2 rule), so each half is worth a lane.
pub const MIN_SECTION_BYTES: u64 = 32 * 1024 * 1024;

/// Why a confirmation was refused.
#[derive(Debug, Clone, Copy, PartialEq, Eq, thiserror::Error)]
pub enum SectionError {
    #[error("a block was confirmed out of order")]
    OutOfOrder,
    #[error("that lane is not sending this file")]
    NoSection,
}

/// One lane's run of blocks: sent in order from `sent`, confirmed in order from `confirmed`, up to
/// (not including) `end`.
#[derive(Debug, Clone)]
struct Section {
    lane: u32,
    end: u32,
    sent: u32,
    confirmed: u32,
}

/// Which lane sends which blocks of one file (on the old laptop). Every section runs in order on
/// one lane. A free lane first takes the earliest blocks nobody is sending; when there are none,
/// it takes the back half of the section with the most left to send, measured from what was
/// already sent so blocks on their way stay with their lane. Done blocks are kept block by block
/// (as aria2's control file does), so any pattern of splits, dropped lanes and resumes works out.
#[derive(Debug, Clone)]
pub struct FileSections {
    done: Vec<bool>,
    done_count: u32,
    block_bytes: u64,
    sections: Vec<Section>,
}

impl FileSections {
    /// A file of `blocks` blocks of about `block_bytes` each; `done` marks blocks the new laptop
    /// already confirmed (for a resumed file), missing entries meaning not done.
    pub fn new(blocks: u32, block_bytes: u64, done: &[bool]) -> Self {
        let done: Vec<bool> = (0..blocks as usize)
            .map(|i| done.get(i).copied().unwrap_or(false))
            .collect();
        let done_count = u32::try_from(done.iter().filter(|d| **d).count()).unwrap_or(blocks);
        Self {
            done,
            done_count,
            block_bytes,
            sections: Vec::new(),
        }
    }

    /// How many blocks are confirmed.
    pub fn done_count(&self) -> u32 {
        self.done_count
    }

    /// Whether every block is confirmed.
    pub fn is_done(&self) -> bool {
        self.done.iter().all(|d| *d)
    }

    /// Gives `lane` a run of this file to send, or `None` when there is nothing worth splitting
    /// off. A lane works on one section of a file at a time.
    pub fn claim(&mut self, lane: u32) -> Option<Range<u32>> {
        if self.sections.iter().any(|s| s.lane == lane) {
            return None;
        }
        if let Some(gap) = self.first_gap() {
            self.sections.push(Section {
                lane,
                end: gap.end,
                sent: gap.start,
                confirmed: gap.start,
            });
            return Some(gap);
        }
        let biggest = self
            .sections
            .iter_mut()
            .max_by_key(|s| (s.end - s.sent, std::cmp::Reverse(s.sent)))?;
        let left = biggest.end - biggest.sent;
        if u64::from(left) * self.block_bytes < 2 * MIN_SECTION_BYTES {
            return None;
        }
        let mid = biggest.sent + left / 2;
        let end = biggest.end;
        biggest.end = mid;
        self.sections.push(Section {
            lane,
            end,
            sent: mid,
            confirmed: mid,
        });
        Some(mid..end)
    }

    /// The next block `lane` should send, if its section has any left.
    pub fn next_block(&mut self, lane: u32) -> Option<u32> {
        let s = self
            .sections
            .iter_mut()
            .find(|s| s.lane == lane && s.sent < s.end)?;
        let b = s.sent;
        s.sent += 1;
        Some(b)
    }

    /// The new laptop confirmed `block` from `lane`. Confirmations must come in the order the lane
    /// sent its blocks; a lane's section ends once every block in it is confirmed.
    pub fn confirmed(&mut self, lane: u32, block: u32) -> Result<(), SectionError> {
        let i = self
            .sections
            .iter()
            .position(|s| s.lane == lane)
            .ok_or(SectionError::NoSection)?;
        let s = &mut self.sections[i];
        if block != s.confirmed || s.confirmed >= s.sent {
            return Err(SectionError::OutOfOrder);
        }
        s.confirmed += 1;
        if let Some(d) = self.done.get_mut(block as usize) {
            *d = true;
            self.done_count += 1;
        }
        if s.confirmed == s.end {
            self.sections.remove(i);
        }
        Ok(())
    }

    /// `lane` dropped: blocks it sent but did not get confirmed go back to be sent again.
    pub fn lane_lost(&mut self, lane: u32) {
        self.sections.retain(|s| s.lane != lane);
    }

    /// The earliest run of blocks that is neither done nor part of any lane's section.
    fn first_gap(&self) -> Option<Range<u32>> {
        let mut taken = vec![false; self.done.len()];
        for s in &self.sections {
            for t in &mut taken[s.confirmed as usize..s.end as usize] {
                *t = true;
            }
        }
        let free = |i: usize| !self.done[i] && !taken[i];
        let start = (0..self.done.len()).find(|&i| free(i))?;
        let end = (start..self.done.len())
            .find(|&i| !free(i))
            .unwrap_or(self.done.len());
        Some(u32::try_from(start).ok()?..u32::try_from(end).ok()?)
    }
}
