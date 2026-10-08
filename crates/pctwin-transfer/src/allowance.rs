use std::collections::HashMap;

use pctwin_record::{Approval, Inclusion, ItemId, ItemKind, Record, RecordError};

/// The least room a file has to grow since the plan was approved (a document saved again).
const MIN_FILE_ROOM: u64 = 1024 * 1024;
/// Extra room for the whole move, on top of a tenth of the plan's total.
const MOVE_ROOM: u64 = 64 * 1024 * 1024;
/// Most attempts at one file: a file that keeps changing while it is read is sent again a few
/// times, then reported, so it cannot be started over and over.
pub const MAX_ATTEMPTS: u32 = 4;

/// Why the new laptop will not start a file.
#[derive(Debug, Clone, Copy, PartialEq, Eq, thiserror::Error)]
pub enum Refusal {
    #[error("it is not part of the plan you approved")]
    NotInPlan,
    #[error(
        "it grew a lot since you approved the plan ({approved} bytes then, {announced} now), so it was not copied; check it and move it again"
    )]
    GrewTooMuch { approved: u64, announced: u64 },
    #[error("the old laptop sent more than the plan you approved")]
    OverTotal,
    #[error("it has already been moved")]
    AlreadyMoved,
    #[error("the old laptop started it twice at once")]
    AlreadyStarted,
    #[error(
        "it kept changing while it was being copied; close the program using it and move it again"
    )]
    KeptChanging,
}

/// Where one approved file is in the move.
#[derive(Debug, Clone, Copy, Default)]
struct Progress {
    /// The largest size it was started with.
    largest: u64,
    attempts: u32,
    live: bool,
    landed: bool,
}

/// What the new laptop agreed to receive: the included files of exactly the approved revision of
/// the plan, each with its approved size. A file may grow a little since then (a tenth of its
/// size, at least 1 MiB); everything started together may not pass the plan's total plus a tenth
/// (and 64 MiB), so that room cannot be used over and over to fill the disk.
#[derive(Debug, Clone)]
pub struct Allowance {
    sizes: HashMap<ItemId, u64>,
    total_room: u64,
    /// Each file's attempts so far.
    started: HashMap<ItemId, Progress>,
    started_total: u64,
}

impl Allowance {
    /// The allowance for `record`, only if `approval` is for exactly this revision.
    pub fn from_record(record: &Record, approval: &Approval) -> Result<Self, RecordError> {
        record.check(approval)?;
        let sizes: HashMap<ItemId, u64> = record
            .items
            .iter()
            .filter(|i| i.inclusion == Inclusion::Included && i.kind == ItemKind::File)
            .map(|i| (i.id, i.size_bytes))
            .collect();
        let total = sizes.values().fold(0u64, |t, s| t.saturating_add(*s));
        Ok(Self {
            sizes,
            total_room: total.saturating_add(total / 10).saturating_add(MOVE_ROOM),
            started: HashMap::new(),
            started_total: 0,
        })
    }

    /// Agrees to start an attempt at `item` of `size` bytes, or says why not. One attempt at a
    /// file runs at a time; a file that landed is spent; a file gets at most [`MAX_ATTEMPTS`]
    /// attempts. Its bytes count once toward the total, at its largest size. Call
    /// [`ended`](Self::ended) when the attempt ends.
    pub fn admit(&mut self, item: ItemId, size: u64) -> Result<(), Refusal> {
        let approved = *self.sizes.get(&item).ok_or(Refusal::NotInPlan)?;
        let progress = self.started.get(&item).copied().unwrap_or_default();
        if progress.landed {
            return Err(Refusal::AlreadyMoved);
        }
        if progress.live {
            return Err(Refusal::AlreadyStarted);
        }
        if progress.attempts >= MAX_ATTEMPTS {
            return Err(Refusal::KeptChanging);
        }
        let room = approved.saturating_add((approved / 10).max(MIN_FILE_ROOM));
        if size > room {
            return Err(Refusal::GrewTooMuch {
                approved,
                announced: size,
            });
        }
        let before = progress.largest;
        let mut total = self.started_total;
        if size > before {
            total = total - before + size;
            if total > self.total_room {
                return Err(Refusal::OverTotal);
            }
        }
        self.started_total = total;
        self.started.insert(
            item,
            Progress {
                largest: before.max(size),
                attempts: progress.attempts + 1,
                live: true,
                landed: false,
            },
        );
        Ok(())
    }

    /// The attempt at `item` ended: it `landed` (or was found already there), or not.
    pub fn ended(&mut self, item: ItemId, landed: bool) {
        if let Some(p) = self.started.get_mut(&item) {
            p.live = false;
            p.landed |= landed;
        }
    }

    /// Bytes agreed to so far, each file counted once at its largest size.
    pub fn started_bytes(&self) -> u64 {
        self.started_total
    }
}
