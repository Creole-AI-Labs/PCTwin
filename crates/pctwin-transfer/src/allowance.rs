use std::collections::HashMap;

use pctwin_record::{Approval, Inclusion, ItemId, ItemKind, Record, RecordError};

/// The least room a file has to grow since the plan was approved (a document saved again).
const MIN_FILE_ROOM: u64 = 1024 * 1024;
/// Extra room for the whole move, on top of a tenth of the plan's total.
const MOVE_ROOM: u64 = 64 * 1024 * 1024;

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
}

/// What the new laptop agreed to receive: the included files of exactly the approved revision of
/// the plan, each with its approved size. A file may grow a little since then (a tenth of its
/// size, at least 1 MiB); everything started together may not pass the plan's total plus a tenth
/// (and 64 MiB), so that room cannot be used over and over to fill the disk.
#[derive(Debug, Clone)]
pub struct Allowance {
    sizes: HashMap<ItemId, u64>,
    total_room: u64,
    /// The largest size each file was started with.
    started: HashMap<ItemId, u64>,
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

    /// Agrees to start `item` at `size` bytes, or says why not. A file started again (it changed
    /// while being read, or the connection dropped) counts once, at its largest size.
    pub fn admit(&mut self, item: ItemId, size: u64) -> Result<(), Refusal> {
        let approved = *self.sizes.get(&item).ok_or(Refusal::NotInPlan)?;
        let room = approved.saturating_add((approved / 10).max(MIN_FILE_ROOM));
        if size > room {
            return Err(Refusal::GrewTooMuch {
                approved,
                announced: size,
            });
        }
        let before = self.started.get(&item).copied().unwrap_or(0);
        if size > before {
            let total = self.started_total - before + size;
            if total > self.total_room {
                return Err(Refusal::OverTotal);
            }
            self.started.insert(item, size);
            self.started_total = total;
        }
        Ok(())
    }

    /// Bytes agreed to so far, each file counted once at its largest size.
    pub fn started_bytes(&self) -> u64 {
        self.started_total
    }
}
