use std::collections::HashMap;
use std::sync::Arc;

use pctwin_journal::{DownloadMark, PartialKeep};
use pctwin_record::{Approval, Inclusion, Item, ItemId, ItemKind, LaptopId, Record, RecordError};

/// The least room a file has to grow since the plan was approved (a document saved again).
const MIN_FILE_ROOM: u64 = 1024 * 1024;
/// Extra room for the whole move, on top of a tenth of the plan's total.
const MOVE_ROOM: u64 = 64 * 1024 * 1024;
/// Most attempts at one file: a file that keeps changing while it is read is sent again a few
/// times, then reported, so it cannot be started over and over.
pub const MAX_ATTEMPTS: u32 = 4;

/// Where one approved file lands on the new laptop: the label of an approved destination and the
/// path inside it. Both laptops work it out from the approved plan with the same function (the
/// old laptop to say where it sends each file, the new laptop to check it), so the old laptop
/// never chooses it.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct LandingPlace {
    pub destination: String,
    pub path: String,
}

impl LandingPlace {
    pub fn new(destination: impl Into<String>, path: impl Into<String>) -> Self {
        Self {
            destination: destination.into(),
            path: path.into(),
        }
    }
}

/// How much bigger than approved a file may be when it is sent.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum SizeRoom {
    /// A move: a file may grow a little since the plan was approved (a document saved again), a
    /// tenth of its size and at least 1 MiB; everything started together may not pass the plan's
    /// total plus a tenth (and 64 MiB), so that room cannot be used over and over to fill the
    /// disk.
    #[default]
    Growth,
    /// No file may be even one byte bigger than approved, and everything together no more than
    /// the plan's total (Quick Send: the files were chosen a moment before they are sent).
    NoGrowth,
}

/// Why the new laptop will not start a file.
#[derive(Debug, Clone, Copy, PartialEq, Eq, thiserror::Error)]
pub enum Refusal {
    #[error("it is not part of the plan you approved")]
    NotInPlan,
    #[error(
        "the old laptop asked to put it somewhere other than the plan you approved, so it was not copied"
    )]
    NotWherePlanned,
    #[error(
        "it grew a lot since you approved the plan ({approved} bytes then, {announced} now), so it was not copied; check it and move it again"
    )]
    GrewTooMuch { approved: u64, announced: u64 },
    #[error(
        "it changed since it was chosen ({approved} bytes then, {announced} now), so it was not copied; send it again"
    )]
    Grew { approved: u64, announced: u64 },
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

/// One approved file: its approved size and where it lands, held at a fixed size: the
/// destination is shared by every file that lands in it, and the path inside it is kept as its
/// digest, never as a second copy of the path (a received list can hold a very great many long
/// paths; the scan inventory's memory count leaves room for exactly this entry per item,
/// `pctwin_record::inventory::PLAN_ROOM`).
#[derive(Debug, Clone)]
struct Planned {
    size: u64,
    destination: Arc<str>,
    path: [u8; 32],
}

/// The digest a planned path is kept as (BLAKE3 in its own named mode).
fn path_digest(path: &str) -> [u8; 32] {
    let mut h = blake3::Hasher::new_derive_key("pctwin 2026-10-21 landing path");
    h.update(path.as_bytes());
    h.finalize().into()
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
/// the plan, each with its approved size and the one place it lands (see [`LandingPlace`]). The
/// old laptop's word is never enough for where a file goes, how big it may be, or how cautious
/// the new laptop's system is with it: a file announced anywhere else is refused, its size may
/// grow only as the [`SizeRoom`] allows, and it is marked at least as strictly as the marker
/// floor (see [`with_mark_floor`](Self::with_mark_floor)).
#[derive(Debug, Clone)]
pub struct Allowance {
    /// The old laptop the plan is for.
    source_laptop: LaptopId,
    files: HashMap<ItemId, Planned>,
    /// The plan's total, every approved file at its approved size.
    total: u64,
    room: SizeRoom,
    /// The least download marker every file gets, whatever the old laptop says.
    mark_floor: Option<DownloadMark>,
    /// Each file's attempts so far.
    started: HashMap<ItemId, Progress>,
    started_total: u64,
    /// How long a partly copied file of this move waits for the rest, as the person chose.
    partial_keep: PartialKeep,
}

impl Allowance {
    /// The allowance for `record`, only if `approval` is for exactly this revision. `landing`
    /// says where each included file lands; it must be the very function the old laptop uses to
    /// say where it sends them. A file it gives no place (`None`) is not received at all. A move
    /// has [`SizeRoom::Growth`] and no marker floor unless set.
    pub fn from_record(
        record: &Record,
        approval: &Approval,
        landing: impl Fn(&Item) -> Option<LandingPlace>,
    ) -> Result<Self, RecordError> {
        record.check(approval)?;
        // Room for every item at once (counted in the inventory's memory limit), so the map
        // never grows and moves while it is filled.
        let mut files: HashMap<ItemId, Planned> = HashMap::with_capacity(record.items.len());
        let mut destinations: HashMap<String, Arc<str>> = HashMap::new();
        for i in &record.items {
            if i.inclusion != Inclusion::Included || i.kind != ItemKind::File {
                continue;
            }
            let Some(place) = landing(i) else {
                continue;
            };
            let path = path_digest(&place.path);
            let destination = destinations
                .entry(place.destination)
                .or_insert_with_key(|d| Arc::from(d.as_str()))
                .clone();
            files.insert(
                i.id,
                Planned {
                    size: i.size_bytes,
                    destination,
                    path,
                },
            );
        }
        let total = files.values().fold(0u64, |t, f| t.saturating_add(f.size));
        Ok(Self {
            source_laptop: record.source_laptop,
            files,
            total,
            room: SizeRoom::default(),
            mark_floor: None,
            started: HashMap::new(),
            started_total: 0,
            partial_keep: PartialKeep::default(),
        })
    }

    /// With this much room for files to grow (a move has [`SizeRoom::Growth`]).
    pub fn with_size_room(mut self, room: SizeRoom) -> Self {
        self.room = room;
        self
    }

    /// With a download marker every file gets at least, however the old laptop described it:
    /// the copy keeps the stricter of the two. A move has none (the old laptop is the person's
    /// own, and its markers travel as they are); files from someone else's laptop get
    /// [`DownloadMark::Internet`] at least.
    pub fn with_mark_floor(mut self, floor: DownloadMark) -> Self {
        self.mark_floor = Some(floor);
        self
    }

    /// The marker a file gets that the old laptop described as marked `sent`: the stricter of
    /// that and the floor.
    pub fn mark_for(&self, sent: Option<DownloadMark>) -> Option<DownloadMark> {
        sent.max(self.mark_floor)
    }

    /// The destination `item` lands in, if it is in the plan (the path inside it is checked with
    /// [`check_place`](Self::check_place)).
    pub fn destination(&self, item: ItemId) -> Option<&str> {
        self.files.get(&item).map(|f| &*f.destination)
    }

    /// Every approved file: its id, the destination it lands in, and its approved size.
    pub(crate) fn planned(&self) -> impl Iterator<Item = (ItemId, &str, u64)> {
        self.files
            .iter()
            .map(|(id, f)| (*id, &*f.destination, f.size))
    }

    /// Whether `item` is in the plan and announced exactly where the plan says it lands
    /// (`destination` and `path` inside it), or why not. Nothing about the attempt is counted.
    pub fn check_place(&self, item: ItemId, destination: &str, path: &str) -> Result<(), Refusal> {
        let planned = self.files.get(&item).ok_or(Refusal::NotInPlan)?;
        if &*planned.destination != destination || planned.path != path_digest(path) {
            return Err(Refusal::NotWherePlanned);
        }
        Ok(())
    }

    /// Agrees to start an attempt at `item` of `size` bytes at `path` inside `destination`, or
    /// says why not. It must be exactly where the plan says it lands; one attempt at a file runs
    /// at a time; a file that landed is spent; a file gets at most [`MAX_ATTEMPTS`] attempts.
    /// Its bytes count once toward the total, at its largest size. Call
    /// [`ended`](Self::ended) when the attempt ends.
    pub fn admit(
        &mut self,
        item: ItemId,
        destination: &str,
        path: &str,
        size: u64,
    ) -> Result<(), Refusal> {
        self.check_place(item, destination, path)?;
        let approved = self.files.get(&item).map_or(0, |f| f.size);
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
        let room = match self.room {
            SizeRoom::Growth => approved.saturating_add((approved / 10).max(MIN_FILE_ROOM)),
            SizeRoom::NoGrowth => approved,
        };
        if size > room {
            return Err(match self.room {
                SizeRoom::Growth => Refusal::GrewTooMuch {
                    approved,
                    announced: size,
                },
                SizeRoom::NoGrowth => Refusal::Grew {
                    approved,
                    announced: size,
                },
            });
        }
        let total_room = match self.room {
            SizeRoom::Growth => self
                .total
                .saturating_add(self.total / 10)
                .saturating_add(MOVE_ROOM),
            SizeRoom::NoGrowth => self.total,
        };
        let before = progress.largest;
        let mut total = self.started_total;
        if size > before {
            total = total - before + size;
            if total > total_room {
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

    /// With the keep-time the person chose for partly copied files of this move.
    pub fn with_partial_keep(mut self, keep: PartialKeep) -> Self {
        self.partial_keep = keep;
        self
    }

    /// How long a partly copied file of this move waits for the rest.
    pub fn partial_keep(&self) -> PartialKeep {
        self.partial_keep
    }

    /// The old laptop the plan is for.
    pub fn source_laptop(&self) -> LaptopId {
        self.source_laptop
    }

    /// Bytes agreed to so far, each file counted once at its largest size.
    pub fn started_bytes(&self) -> u64 {
        self.started_total
    }
}

#[cfg(test)]
mod room {
    use super::*;

    /// A planned file's entry fits the room the inventory's memory count leaves for it: the map
    /// is made with room for every item at once, at most about 2.3 slots an entry, plus a control
    /// byte each, and holds nothing else per file.
    #[test]
    fn a_planned_file_fits_the_room_the_inventory_leaves_for_it() {
        let entry = std::mem::size_of::<(ItemId, Planned)>() + 1;
        assert!(entry * 23 / 10 < pctwin_record::inventory::PLAN_ROOM as usize);
    }
}
