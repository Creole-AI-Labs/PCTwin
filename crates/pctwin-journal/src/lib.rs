//! PCTwin change journal (Task List 1.6; Engineering Plan: "write durable intent and undo records
//! before any change").
//!
//! Every write on the new laptop is one entry that moves, one durable step at a time, through
//! [`State::Planned`], [`State::Staged`] (its temporary `.pctwin-` file exists), [`State::Verified`]
//! (every byte arrived, checked, and flushed to disk), [`State::Applied`] (the real name it is
//! getting, recorded before it gets it) and [`State::Committed`] (finished; what it landed as is
//! kept for undo), or ends [`State::Existing`] (an identical file was already there, so nothing was
//! written) or [`State::Failed`] with the reason and the step it had reached. Steps cannot be
//! skipped or taken back.
//!
//! Each step is its own committed transaction in a [redb](https://docs.rs/redb) database: crash-safe
//! by default, with two checksummed commit slots, so a write torn by power loss is detected and the
//! last good commit is used. After a crash the journal holds exactly the steps already taken, and
//! [`recovery`] works out what each unfinished write needs instead of announcing success. A damaged
//! journal is reported, never trusted, and a journal from a newer PCTwin is refused.
//!
//! Each entry records the item, the old laptop it came from, the approved destination and path,
//! and who acted for whom with what permission. Each journal has its own random number, and each
//! write's temporary file is named after it and the entry ([`Journal::temp_tag`]), so the journal
//! always knows exactly which temporary file is its own, even one made just before a crash, and
//! never touches another's. Folders made for a write are recorded, so undo removes only those.
//!
//! Undo is open only until the wipe of the old laptop starts ([`Journal::close_undo`], one way and
//! for good). Every undo write needs an [`UndoRight`]: an [`UndoPermit`] taken while undo was
//! open, or the [`CloseTicket`] that only closing hands out. Closing waits (for a bounded time) for
//! every permit still held, finishes every removal left part-way, and records those and the close
//! in one transaction, so no file is being undone, and none is left under a hidden name, once the
//! wipe can be sent.

pub mod recovery;

use std::io;
use std::path::Path;
use std::sync::{Condvar, Mutex, PoisonError};
use std::time::{Duration, Instant};

use pctwin_record::{ItemId, LaptopId};
use redb::{Database, ReadableDatabase, ReadableTable, ReadableTableMetadata, TableDefinition};
use serde::{Deserialize, Serialize};

/// The journal's format. A journal with a higher number came from a newer PCTwin and is refused
/// ([`JournalError::NewerFormat`]), so an older PCTwin never reads records it does not know.
///
/// - **2**: undo removes a copy through the handle it was checked on (Windows) or by first moving
///   its name aside to a private name and checking it there (Linux and macOS): the undo stages
///   `Undo::Removing{file, dir_id, private}`, `Undo::Putting`, `Undo::Salvaging` and
///   `Undo::Done`, with `UndoOutcome::Deleted` and `UndoOutcome::KeptAt`; and every PCTwin that
///   reads it honours the undo gate in the `gate` table. (The undo stages changed shape on 9
///   October 2026 without a new number: format 2 had never shipped, so no journal on any laptop
///   holds the older shape. Once it ships, any change of shape needs a new number.) Raised because the gate came late in format 1: an earlier format-1 PCTwin ignored it
///   and could allow undo after the wipe, so it must refuse this journal instead.
/// - **1**: undo set copies aside or sent them to the Recycle Bin (`Undo::Aside`,
///   `UndoOutcome::Trashed`). Opening a format-1 journal that holds no undo record (of a file or
///   of a folder) upgrades it to 2 in the same transaction as the open: its entries, folders and
///   gate mean the same in both, and a missing `gate` table means undo is open (no PCTwin could
///   send a wipe then). One that holds any undo record is refused
///   ([`JournalError::OlderFormatWithUndo`]) and left unchanged, because those records cannot be
///   read in format 2 and guessing what a half-done undo did could remove the wrong thing. (No
///   format-1 journal was ever installed, so this is a guard, not a migration path.)
/// - Any other number (0) is reported damaged.
pub const FORMAT: u32 = 2;
/// The one older format that can still be opened (when it holds no undo record).
const FORMAT_1: u32 = 1;

const META: TableDefinition<&str, u32> = TableDefinition::new("meta");
/// The journal's own number, the next entry number, and how far clean-up has looked.
const COUNTERS: TableDefinition<&str, u64> = TableDefinition::new("counters");
const ENTRIES: TableDefinition<u64, &[u8]> = TableDefinition::new("entries");
/// Entries not yet finished, so a restart reads only those.
const UNFINISHED: TableDefinition<u64, ()> = TableDefinition::new("unfinished");
/// The unfinished entry for each file of each old laptop (`laptop/item` in hex).
const OPEN_ITEMS: TableDefinition<&str, u64> = TableDefinition::new("open-items");
/// Folders made for a write: (destination, stored folder) to who made it.
const FOLDERS: TableDefinition<(&str, &str), &[u8]> = TableDefinition::new("folders");
/// Entries whose temporary file could not be removed yet.
const LEFTOVERS: TableDefinition<u64, ()> = TableDefinition::new("leftovers");
/// Undo of each committed write, by entry: what was decided, then what happened.
const UNDO: TableDefinition<u64, &[u8]> = TableDefinition::new("undo");
/// Undo of each folder made for a write: (destination, stored folder) to what happened.
const UNDO_FOLDERS: TableDefinition<(&str, &str), &[u8]> = TableDefinition::new("undo-folders");
/// Each landed block's fingerprint, while its file is being received: (entry, block).
const BLOCKS: TableDefinition<(u64, u64), [u8; 32]> = TableDefinition::new("blocks");
/// One-way gates. `undo`: absent while undo is open, [`CLOSED`] once the wipe has started.
const GATE: TableDefinition<&str, u8> = TableDefinition::new("gate");
const UNDO_GATE: &str = "undo";
const CLOSED: u8 = 1;

const JOURNAL_ID: &str = "journal-id";
const NEXT_ID: &str = "next-id";
const SWEPT_UPTO: &str = "swept-upto";

/// Why the journal could not be used.
#[derive(Debug, thiserror::Error)]
pub enum JournalError {
    #[error("the move's record on this laptop is damaged: {0}")]
    Damaged(String),
    #[error("PCTwin is already running on this laptop")]
    InUse,
    #[error("the move's record was made by a newer PCTwin (format {found}); update PCTwin")]
    NewerFormat { found: u32 },
    /// A format-1 journal holding undo records that this PCTwin cannot read safely.
    #[error(
        "the move's record was made by an older PCTwin (format {found}) and holds an undo          this PCTwin cannot read; nothing was changed"
    )]
    OlderFormatWithUndo { found: u32 },
    #[error("no such entry in the move's record: {0}")]
    NoSuchEntry(u64),
    #[error("a step was taken out of order ({from} to {to})")]
    OutOfOrder {
        from: &'static str,
        to: &'static str,
    },
    #[error("the move's record could not be written: {0}")]
    Storage(String),
    /// The wipe of the old laptop has started, so nothing can be undone any more.
    #[error("undo is closed because the wipe of the old laptop has started")]
    UndoClosed,
    /// An undo permit or close ticket taken from another move's record.
    #[error("undo was allowed for another move's record, not this one")]
    OtherJournal,
    /// Undo is busy with files right now, or closing is under way; try again shortly.
    #[error("undo is busy with other files right now; try again in a moment")]
    Busy,
    /// The answer to "leave these files" was not for the list being shown now (an older list,
    /// one already answered, or another move's), so nothing was recorded.
    #[error("that list of files is out of date; look at it again and choose again")]
    EscapeRefused,
    /// A name in an undo record that is not one plain name in the copy's folder.
    #[error("not a plain file name: {0:?}")]
    BadName(String),
}

fn storage(e: impl std::fmt::Display) -> JournalError {
    JournalError::Storage(e.to_string())
}

fn damaged(e: impl std::fmt::Display) -> JournalError {
    JournalError::Damaged(e.to_string())
}

/// Another thread stopped with a panic while it held the undo count.
fn poisoned<T>(_: PoisonError<T>) -> JournalError {
    JournalError::Storage("undo stopped part way in another task; restart PCTwin".into())
}

/// What allowed this write.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum Permission {
    /// The signed-in person writing into their own folders.
    OwnFolders,
    /// Writing into the shared folder for everyone on this laptop.
    SharedFolder,
    /// Writing into another person's account through the administrator helper.
    AdminHelper,
}

/// Who acted for whom, with what permission (accounts by their system ID on the new laptop).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Actor {
    pub acting_account: String,
    pub for_account: String,
    pub permission: Permission,
}

/// Which file or folder this is on its drive: the drive's number, the file's number on it (never
/// 0, which drives use for "no number"), and, where the drive keeps it, when the file was made.
/// Two names with the same identity are the same file. The birth time matters on Linux and macOS:
/// a drive there gives a freed number to the next new file at once, and only the birth time tells
/// the two apart. Kept as the raw fields, never folded together, so nothing can collide.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct FileId {
    pub volume: u64,
    pub index: std::num::NonZeroU64,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub born: Option<Born>,
}

/// When a file was made, as its drive keeps it: seconds since 1970 (negative before) and the
/// nanoseconds within that second.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct Born {
    pub secs: i64,
    pub nanos: u32,
}

impl Born {
    /// A birth time from the system's clock value.
    pub fn of(t: std::time::SystemTime) -> Self {
        match t.duration_since(std::time::UNIX_EPOCH) {
            Ok(d) => Self {
                secs: i64::try_from(d.as_secs()).unwrap_or(i64::MAX),
                nanos: d.subsec_nanos(),
            },
            Err(e) => {
                let d = e.duration();
                let secs = i64::try_from(d.as_secs()).unwrap_or(i64::MAX);
                if d.subsec_nanos() == 0 {
                    Self {
                        secs: -secs,
                        nanos: 0,
                    }
                } else {
                    Self {
                        secs: -secs - 1,
                        nanos: 1_000_000_000 - d.subsec_nanos(),
                    }
                }
            }
        }
    }
}

/// What is about to be written.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct PlannedWrite {
    pub item: ItemId,
    pub source_laptop: LaptopId,
    /// The approved destination's label.
    pub destination: String,
    /// The path inside it, as the old laptop sent it.
    pub path: String,
    pub size: u64,
    pub actor: Actor,
    /// The block size the file is sent in (its fingerprint is worked out over these blocks).
    pub block_size: u64,
    /// The original's modified time on the old laptop (nanoseconds since 1970), to continue after
    /// a restart only if the original has not changed.
    pub source_modified_ns: Option<i64>,
    /// The approved folder's identity when the write was planned: after a restart nothing is done
    /// in a folder that is not the same one (another drive under the same letter, say).
    pub place: Option<FileId>,
    /// The original's identity on the old laptop when it was read, so undo can ask the old laptop
    /// whether it still has that very file, unchanged, before removing the copy.
    #[serde(default)]
    pub source_file: Option<FileId>,
    /// How long a partly copied file of this move waits for the rest with nothing added to it,
    /// as the person chose before the move.
    #[serde(default)]
    pub partial_keep: PartialKeep,
}

/// How long a partly copied file waits for the rest with nothing added to it, picked before the
/// move (Product Spec, decided 8 October 2026): one of these, 30 days unless the person picks
/// another.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum PartialKeep {
    #[serde(rename = "3-days")]
    Days3,
    #[serde(rename = "7-days")]
    Days7,
    #[serde(rename = "14-days")]
    Days14,
    #[default]
    #[serde(rename = "30-days")]
    Days30,
    #[serde(rename = "60-days")]
    Days60,
    #[serde(rename = "90-days")]
    Days90,
}

impl PartialKeep {
    /// Every choice, shortest first.
    pub const ALL: [PartialKeep; 6] = [
        PartialKeep::Days3,
        PartialKeep::Days7,
        PartialKeep::Days14,
        PartialKeep::Days30,
        PartialKeep::Days60,
        PartialKeep::Days90,
    ];

    /// How many days.
    pub fn days(self) -> u32 {
        match self {
            PartialKeep::Days3 => 3,
            PartialKeep::Days7 => 7,
            PartialKeep::Days14 => 14,
            PartialKeep::Days30 => 30,
            PartialKeep::Days60 => 60,
            PartialKeep::Days90 => 90,
        }
    }

    /// The choice for exactly this many days, if it is one.
    pub fn from_days(days: u32) -> Option<Self> {
        Self::ALL.into_iter().find(|k| k.days() == days)
    }

    /// As a length of time.
    pub fn duration(self) -> std::time::Duration {
        std::time::Duration::from_secs(u64::from(self.days()) * 24 * 60 * 60)
    }
}

/// What a finished file landed as: undo compares against this to keep later edits.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct Landed {
    pub size: u64,
    /// Nanoseconds since 1970.
    pub modified_ns: Option<i64>,
    /// Which file it is, so undo never takes a different file made later under the same name.
    pub file: Option<FileId>,
}

/// How far a write got.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case", tag = "step")]
pub enum State {
    Planned,
    /// Its temporary file (stored path inside the destination) exists.
    Staged {
        temp: String,
    },
    /// Every byte arrived and was checked, the file did not change while it was read, and it is
    /// flushed to disk under its temporary name. `fingerprint` is the whole file's; `file` is
    /// which file it is on its drive, so after a crash only this very file is ever taken as it.
    Verified {
        temp: String,
        fingerprint: [u8; 32],
        file: Option<FileId>,
    },
    /// The real name it is getting, recorded before it gets it (never replacing another file).
    /// If something took that name first, it moves on to another name.
    Applied {
        temp: String,
        final_path: String,
        fingerprint: [u8; 32],
        file: Option<FileId>,
    },
    Committed {
        final_path: String,
        fingerprint: [u8; 32],
        landed: Landed,
    },
    /// An identical file was already at `stored_path`, so nothing was written. It is the person's
    /// own file: undo never touches it.
    Existing {
        stored_path: String,
    },
    /// It did not finish: why, and the step it had reached (for clean-up).
    Failed {
        why: String,
        reached: Box<State>,
    },
}

impl State {
    fn name(&self) -> &'static str {
        match self {
            State::Planned => "planned",
            State::Staged { .. } => "staged",
            State::Verified { .. } => "verified",
            State::Applied { .. } => "applied",
            State::Committed { .. } => "committed",
            State::Existing { .. } => "existing",
            State::Failed { .. } => "failed",
        }
    }

    /// Committed, existing and failed writes are finished.
    pub fn is_finished(&self) -> bool {
        matches!(
            self,
            State::Committed { .. } | State::Existing { .. } | State::Failed { .. }
        )
    }
}

/// One write and how far it got.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Entry {
    pub id: u64,
    pub write: PlannedWrite,
    pub state: State,
}

/// Undoing one committed write, recorded beside the write (the write's own record never changes:
/// it is the history of the move). Nothing is ever created under, or moved to, a name that is not
/// journaled first. Names (`private`, `to`, `temp`) are single names in the copy's own folder,
/// never paths.
///
/// The stages go in one direction: Removing (again with a new private name, if the first was
/// taken), then Putting (again for each candidate name) or Salvaging, then Done. A final Done is
/// never written over; [`UndoOutcome::NotDone`] ("not done this time") may start again with
/// Removing. Every stage before Done is about the same file.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case", tag = "step")]
pub enum Undo {
    /// About to remove this very file (by identity). On Windows, through the handle it was checked
    /// on. On Linux and macOS, by first moving its name, never replacing anything, to `private` in
    /// its folder (`dir_id`), then checking it there. Recorded first (for a group, with
    /// [`Journal::record_removals`]), so after a crash undo knows where the file may be.
    Removing {
        file: FileId,
        /// The identity of the folder the copy is in, so the private name is looked for only in
        /// that very folder.
        dir_id: FileId,
        /// The random private name the copy's name is moved to.
        private: String,
    },
    /// About to move what is under `private` back out to `to`: its own name, or a visible name
    /// beside it. Recorded before each rename tried.
    Putting {
        file: FileId,
        private: String,
        to: String,
    },
    /// About to make a salvage copy under the private name `temp` and publish it as `to` (a
    /// visible name beside the copy's), because the file changed while it was removed.
    Salvaging {
        file: FileId,
        temp: String,
        to: String,
    },
    /// Done; never looked at again, unless it could not be done this time.
    Done { outcome: UndoOutcome },
}

impl Undo {
    fn name(&self) -> &'static str {
        match self {
            Undo::Removing { .. } => "removing",
            Undo::Putting { .. } => "putting",
            Undo::Salvaging { .. } => "salvaging",
            Undo::Done { .. } => "done",
        }
    }

    /// Whether this is the last stage (anything else is part-way, and closing must finish it).
    pub fn is_done(&self) -> bool {
        matches!(self, Undo::Done { .. })
    }

    /// The file a stage before Done is about.
    fn file(&self) -> Option<FileId> {
        match self {
            Undo::Removing { file, .. }
            | Undo::Putting { file, .. }
            | Undo::Salvaging { file, .. } => Some(*file),
            Undo::Done { .. } => None,
        }
    }

    /// The names this stage will create or move to, each one plain name in the copy's folder.
    fn names(&self) -> Vec<&str> {
        match self {
            Undo::Removing { private, .. } => vec![private],
            Undo::Putting { private, to, .. } => vec![private, to],
            Undo::Salvaging { temp, to, .. } => vec![temp, to],
            Undo::Done { .. } => vec![],
        }
    }

    /// The hidden name in the copy's folder where the file (or what was saved of it) may be.
    fn hidden_name(&self) -> Option<&str> {
        match self {
            Undo::Removing { private, .. } | Undo::Putting { private, .. } => Some(private),
            Undo::Salvaging { temp, .. } => Some(temp),
            Undo::Done { .. } => None,
        }
    }
}

/// Whether `next` may be recorded after `before` (none: not started).
fn may_follow(before: Option<&Undo>, next: &Undo) -> bool {
    match (before, next) {
        (Some(Undo::Done { outcome }), _) if outcome.is_final() => false,
        (None | Some(Undo::Done { .. }), Undo::Removing { .. } | Undo::Done { .. }) => true,
        (None | Some(Undo::Done { .. }), _) => false,
        (Some(before), next) => {
            let same_file = next.file().is_none_or(|f| before.file() == Some(f));
            same_file
                && matches!(
                    (before, next),
                    (
                        Undo::Removing { .. },
                        Undo::Removing { .. }
                            | Undo::Putting { .. }
                            | Undo::Salvaging { .. }
                            | Undo::Done { .. }
                    ) | (
                        Undo::Putting { .. },
                        Undo::Putting { .. } | Undo::Done { .. }
                    ) | (
                        Undo::Salvaging { .. },
                        Undo::Salvaging { .. } | Undo::Done { .. }
                    )
                )
        }
    }
}

/// An error unless `name` is one plain name (no folder, not `.` or `..`, nothing hidden in it).
fn plain_name(name: &str) -> Result<(), JournalError> {
    if name.is_empty() || name == "." || name == ".." || name.contains(['/', '\\', '\0']) {
        return Err(JournalError::BadName(name.into()));
    }
    Ok(())
}

/// How undoing one thing ended.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case", tag = "result")]
pub enum UndoOutcome {
    /// Deleted outright (not to the Recycle Bin): the original is still on the old laptop.
    Deleted,
    /// A folder PCTwin made, empty, removed.
    Removed,
    /// It was already gone.
    AlreadyGone,
    /// Kept, and why in plain words (changed since the move, in use, not empty).
    Kept { why: String },
    /// Kept at a visible stored path `at` inside the destination other than its own name (a name
    /// beside it marked as kept by PCTwin undo, or, if the person chose to leave them, where it
    /// was left), and why in plain words. The person needs to look at it (needs action).
    KeptAt { at: String, why: String },
    /// It could not be done this time, and why; undo tries it again next time.
    NotDone { why: String },
}

impl UndoOutcome {
    /// Whether undo is finished with it (only "not done this time" is tried again).
    pub fn is_final(&self) -> bool {
        !matches!(self, UndoOutcome::NotDone { .. })
    }

    /// Whether the person has something to look at or do: it was kept (where it was, or at
    /// another path) or could not be done this time.
    pub fn needs_action(&self) -> bool {
        matches!(
            self,
            UndoOutcome::Kept { .. } | UndoOutcome::KeptAt { .. } | UndoOutcome::NotDone { .. }
        )
    }
}

/// A folder made for a write: which entry made it, and its identity then.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct MadeFolder {
    pub destination: String,
    /// Its stored path inside the destination.
    pub folder: String,
    pub entry: u64,
    pub id: Option<FileId>,
}

/// Whether undo is still possible.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum UndoGate {
    /// The wipe has not started: files can be undone.
    Open,
    /// The wipe has started (even if it was cancelled later): undo refuses everything, for good.
    Closed,
}

/// Proof that undo is closed for good in a journal, on disk: only [`Journal::close_undo`] makes
/// one (its field is private, and it has no `Default` or `Clone`). The function that sends the
/// wipe to the old laptop is to take `&ClosedToken` and check [`ClosedToken::is_for`] its move's
/// journal, so no wipe can be sent while a file could still be undone. (No such function exists
/// yet.)
#[derive(Debug)]
pub struct ClosedToken {
    journal_id: u64,
}

impl ClosedToken {
    /// Whether this is the close of `journal` (by the journal's own number, kept on disk).
    pub fn is_for(&self, journal: &Journal) -> bool {
        self.journal_id == journal.journal_id
    }
}

mod sealed {
    /// Only this crate can say what an undo right is.
    pub trait Sealed {
        /// The journal that granted it.
        fn granted_by(&self) -> *const super::Journal;
    }
}

/// Leave to write undo records in one journal: an [`UndoPermit`] (taken while undo is open) or a
/// [`CloseTicket`] (handed out by [`Journal::close_undo`] alone, while it finishes what was left
/// part-way). Only this crate can make one, and each is checked against the journal it is used
/// on ([`JournalError::OtherJournal`]).
pub trait UndoRight: sealed::Sealed {}

impl sealed::Sealed for UndoPermit<'_> {
    fn granted_by(&self) -> *const Journal {
        self.journal
    }
}
impl UndoRight for UndoPermit<'_> {}

/// Leave to finish, while undo closes, the removals left part-way: only [`Journal::close_undo`]
/// makes one, and only lends it to its resolver for one call, so it cannot be kept, copied or
/// used once closing is over.
///
/// ```compile_fail
/// // Not made outside the journal.
/// let ticket = pctwin_journal::CloseTicket { journal: todo!() };
/// ```
///
/// ```compile_fail
/// // Not kept past the call it was lent for.
/// fn keep(j: &pctwin_journal::Journal) {
///     let mut kept = None;
///     let _ = j.close_undo(&mut |t, _| {
///         kept = Some(t);
///         Ok(pctwin_journal::Resolved::Removed)
///     });
/// }
/// ```
pub struct CloseTicket<'j> {
    journal: &'j Journal,
}

impl std::fmt::Debug for CloseTicket<'_> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str("CloseTicket(..)")
    }
}

impl sealed::Sealed for CloseTicket<'_> {
    fn granted_by(&self) -> *const Journal {
        self.journal
    }
}
impl UndoRight for CloseTicket<'_> {}

/// An undo left part-way (any stage but Done), for closing to finish.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Pending {
    /// The entry (a committed write).
    pub id: u64,
    pub stage: Undo,
    /// The approved destination's label.
    pub destination: String,
    /// The copy's stored path inside it (the name it landed under).
    pub path: String,
}

impl Pending {
    /// The copy's folder (stored path inside the destination; empty at its top).
    pub fn folder(&self) -> &str {
        self.path.rsplit_once('/').map_or("", |(f, _)| f)
    }

    /// The stored path of `name` in the copy's folder.
    pub fn in_folder(&self, name: &str) -> String {
        match self.folder() {
            "" => name.to_string(),
            folder => format!("{folder}/{name}"),
        }
    }
}

/// How closing finished one part-way undo; one for every [`Pending`], with nothing in between.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Resolved {
    /// The copy, proven to be the one checked and unchanged, was removed ([`UndoOutcome::Deleted`]).
    Removed,
    /// Nothing of it is left to remove.
    AlreadyGone,
    /// Kept under its own name, and why.
    Kept { why: String },
    /// Kept at another visible stored path `at` (a name beside it), and why: needs action.
    KeptAt { at: String, why: String },
    /// It could not be finished now (its drive is not there, say), and why: undo does not close.
    Unresolved { why: String },
}

impl Resolved {
    /// What it ends as, if it ended.
    fn outcome(self) -> Result<UndoOutcome, String> {
        match self {
            Resolved::Removed => Ok(UndoOutcome::Deleted),
            Resolved::AlreadyGone => Ok(UndoOutcome::AlreadyGone),
            Resolved::Kept { why } => Ok(UndoOutcome::Kept { why }),
            Resolved::KeptAt { at, why } => Ok(UndoOutcome::KeptAt { at, why }),
            Resolved::Unresolved { why } => Err(why),
        }
    }
}

/// A part-way undo closing could not finish: which, where, and why.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct UnresolvedItem {
    pub id: u64,
    pub destination: String,
    /// The copy's stored path inside the destination.
    pub path: String,
    pub why: String,
}

/// Why undo did not close. In every case nothing was closed, the wipe cannot be sent, and undo
/// carries on as before.
#[derive(Debug, thiserror::Error)]
pub enum CloseError {
    /// Files were still being undone when the wait ran out, or another close is under way.
    #[error("files are still being undone; try again in a moment")]
    Busy,
    /// Some files moved part-way could not be finished (each listed with why).
    #[error("PCTwin could not finish tidying {} files, so the old laptop was not wiped", items.len())]
    Unresolved { items: Vec<UnresolvedItem> },
    #[error(transparent)]
    Journal(#[from] JournalError),
}

/// An offer to leave the files still part-way where they are, for the person to remove
/// themselves ([`Journal::offer_escape`]): the exact paths to show, and the one answer that
/// accepts exactly this list ([`Journal::accept_escape`]).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct EscapeOffer {
    /// 128 random bits from the operating system, new for each offer.
    pub nonce: [u8; 16],
    pub items: Vec<EscapeItem>,
}

/// A file the person would be leaving: the exact stored path of its hidden name.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct EscapeItem {
    pub id: u64,
    pub destination: String,
    /// Stored path inside the destination.
    pub path: String,
}

/// The offer this journal is waiting on an answer for, with what it showed.
struct StoredOffer {
    nonce: [u8; 16],
    shown: Vec<Pending>,
}

/// Why the person's choice was recorded.
const LEFT_BY_CHOICE: &str = "you chose to leave it here and remove it yourself";

/// How long closing waits by default for files still being undone.
pub const CLOSE_WAIT: Duration = Duration::from_secs(10);

/// Leave to undo while undo is open: taken per file with [`Journal::begin_undo`], needed for every
/// undo write, and given back when dropped. Many can be held at once (files undone side by side);
/// [`Journal::close_undo`] waits until none is held.
pub struct UndoPermit<'j> {
    journal: &'j Journal,
}

impl std::fmt::Debug for UndoPermit<'_> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str("UndoPermit(..)")
    }
}

impl Drop for UndoPermit<'_> {
    fn drop(&mut self) {
        // Given back even if another thread panicked while holding the count's lock: the count is
        // a plain number, still right, and a permit never given back would make closing wait
        // forever.
        let mut undo = self
            .journal
            .undo
            .lock()
            .unwrap_or_else(PoisonError::into_inner);
        undo.permits = undo.permits.saturating_sub(1);
        drop(undo);
        self.journal.undo_free.notify_all();
    }
}

/// Undo permits held in this process, whether closing has begun, and whether a close is running.
#[derive(Default)]
struct UndoUse {
    permits: usize,
    closing: bool,
    closer: bool,
}

/// The change journal for a move on this laptop.
pub struct Journal {
    db: Database,
    /// This journal's own random number (names its temporary files).
    journal_id: u64,
    /// Undo permits held and whether closing has begun. A count and a flag rather than a
    /// read-write lock: the systems' read-write locks differ in whether a waiting writer holds
    /// back new readers, and here no permit may ever be handed out once closing has begun.
    undo: Mutex<UndoUse>,
    /// Woken when a permit is given back.
    undo_free: Condvar,
    /// The offer to leave part-way files that an answer is awaited for (kept in memory only: an
    /// offer never outlives the PCTwin that showed it).
    escape: Mutex<Option<StoredOffer>>,
}

impl std::fmt::Debug for Journal {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str("Journal(..)")
    }
}

fn open_key(write: &PlannedWrite) -> String {
    format!("{}/{}", write.source_laptop.to_hex(), write.item.to_hex())
}

impl Journal {
    /// Opens the journal at `path`, creating it if there is none.
    pub fn open(path: &Path) -> Result<Self, JournalError> {
        let db = Database::create(path).map_err(|e| match e {
            redb::DatabaseError::DatabaseAlreadyOpen => JournalError::InUse,
            other => JournalError::Damaged(other.to_string()),
        })?;
        let journal_id = Self::prepare(&db)?;
        Ok(Self {
            db,
            journal_id,
            undo: Mutex::new(UndoUse::default()),
            undo_free: Condvar::new(),
            escape: Mutex::new(None),
        })
    }

    /// Checks the format and makes every table; returns the journal's own number.
    fn prepare(db: &Database) -> Result<u64, JournalError> {
        let tx = db.begin_write().map_err(storage)?;
        let journal_id = {
            let mut meta = tx.open_table(META).map_err(storage)?;
            let found = meta.get("format").map_err(storage)?.map(|v| v.value());
            match found {
                Some(FORMAT) => {}
                Some(found) if found > FORMAT => {
                    return Err(JournalError::NewerFormat { found });
                }
                Some(FORMAT_1) => {
                    // Upgraded only if no undo record exists; returning drops the transaction
                    // uncommitted, so a refused journal is left exactly as it was.
                    let files = tx.open_table(UNDO).map_err(storage)?;
                    let folders = tx.open_table(UNDO_FOLDERS).map_err(storage)?;
                    if !files.is_empty().map_err(storage)?
                        || !folders.is_empty().map_err(storage)?
                    {
                        return Err(JournalError::OlderFormatWithUndo { found: FORMAT_1 });
                    }
                    meta.insert("format", FORMAT).map_err(storage)?;
                }
                Some(other) => return Err(damaged(format!("unknown format {other}"))),
                None => {
                    meta.insert("format", FORMAT).map_err(storage)?;
                }
            }
            let mut counters = tx.open_table(COUNTERS).map_err(storage)?;
            let existing = counters
                .get(JOURNAL_ID)
                .map_err(storage)?
                .map(|v| v.value());
            let journal_id = match existing {
                Some(id) => id,
                None => {
                    let mut bytes = [0u8; 8];
                    getrandom::fill(&mut bytes).map_err(storage)?;
                    let id = u64::from_le_bytes(bytes);
                    counters.insert(JOURNAL_ID, id).map_err(storage)?;
                    id
                }
            };
            if counters.get(NEXT_ID).map_err(storage)?.is_none() {
                counters.insert(NEXT_ID, 1).map_err(storage)?;
            }
            tx.open_table(ENTRIES).map_err(storage)?;
            tx.open_table(UNFINISHED).map_err(storage)?;
            tx.open_table(OPEN_ITEMS).map_err(storage)?;
            tx.open_table(FOLDERS).map_err(storage)?;
            tx.open_table(LEFTOVERS).map_err(storage)?;
            tx.open_table(BLOCKS).map_err(storage)?;
            tx.open_table(UNDO).map_err(storage)?;
            tx.open_table(UNDO_FOLDERS).map_err(storage)?;
            tx.open_table(GATE).map_err(storage)?;
            journal_id
        };
        tx.commit().map_err(storage)?;
        Ok(journal_id)
    }

    /// This journal's own number: random, chosen when it was first made, and kept as long as its
    /// file is kept. Something taken from one journal (a check of the originals, say) names this
    /// number so it is never used against another journal.
    pub fn instance(&self) -> u64 {
        self.journal_id
    }

    /// The tag of entry `id`'s temporary file: this journal's number and the entry's, so the
    /// name is known before the file exists and differs from every other journal's.
    pub fn temp_tag(&self, id: u64) -> String {
        format!("{:016x}-{id}", self.journal_id)
    }

    /// Records a write about to start; returns its entry number (never reused). A write of the
    /// same file from the same old laptop still unfinished is ended as failed in the same step
    /// (one attempt at a file at a time); its temporary file is cleaned up by recovery.
    pub fn plan(&self, write: &PlannedWrite) -> Result<u64, JournalError> {
        let tx = self.db.begin_write().map_err(storage)?;
        let id = {
            let mut counters = tx.open_table(COUNTERS).map_err(storage)?;
            let id = counters
                .get(NEXT_ID)
                .map_err(storage)?
                .map(|v| v.value())
                .ok_or_else(|| damaged("no next entry number"))?;
            counters.insert(NEXT_ID, id + 1).map_err(storage)?;
            let mut open = tx.open_table(OPEN_ITEMS).map_err(storage)?;
            let key = open_key(write);
            let earlier = open.get(key.as_str()).map_err(storage)?.map(|v| v.value());
            open.insert(key.as_str(), id).map_err(storage)?;
            drop(open);
            if let Some(earlier) = earlier {
                let mut entries = tx.open_table(ENTRIES).map_err(storage)?;
                let mut old = read_entry(&entries, earlier)?;
                if !old.state.is_finished() {
                    old.state = State::Failed {
                        why: "it was started again".into(),
                        reached: Box::new(old.state.clone()),
                    };
                    write_entry(&mut entries, &old)?;
                    let mut unfinished = tx.open_table(UNFINISHED).map_err(storage)?;
                    unfinished.remove(earlier).map_err(storage)?;
                    let mut blocks = tx.open_table(BLOCKS).map_err(storage)?;
                    blocks
                        .retain_in((earlier, 0)..=(earlier, u64::MAX), |_, _| false)
                        .map_err(storage)?;
                }
            }
            let entry = Entry {
                id,
                write: write.clone(),
                state: State::Planned,
            };
            let mut entries = tx.open_table(ENTRIES).map_err(storage)?;
            write_entry(&mut entries, &entry)?;
            let mut unfinished = tx.open_table(UNFINISHED).map_err(storage)?;
            unfinished.insert(id, ()).map_err(storage)?;
            id
        };
        tx.commit().map_err(storage)?;
        Ok(id)
    }

    /// Its temporary file (stored path `temp`) exists; `made` are the folders made for it (stored
    /// paths, with their identity). A folder already recorded as made keeps its first record.
    pub fn staged(
        &self,
        id: u64,
        temp: &str,
        made: &[(String, Option<FileId>)],
    ) -> Result<(), JournalError> {
        self.step(
            id,
            "staged",
            |s| match s {
                State::Planned => Some(State::Staged { temp: temp.into() }),
                _ => None,
            },
            |tx, entry| {
                let mut folders = tx.open_table(FOLDERS).map_err(storage)?;
                for (folder, folder_id) in made {
                    let key = (entry.write.destination.as_str(), folder.as_str());
                    if folders.get(key).map_err(storage)?.is_none() {
                        let record = MadeFolder {
                            destination: entry.write.destination.clone(),
                            folder: folder.clone(),
                            entry: id,
                            id: *folder_id,
                        };
                        let bytes = serde_json::to_vec(&record).map_err(storage)?;
                        folders.insert(key, bytes.as_slice()).map_err(storage)?;
                    }
                }
                Ok(())
            },
        )
    }

    /// Blocks of a file being received have landed (block number and fingerprint), so after a
    /// restart it can continue from them. Only checkpoints, written in batches: a `durable` one is
    /// on disk when this returns; others are made durable by the next durable step, and if the
    /// laptop stops first they are simply lost (those blocks are sent again). After a restart every
    /// block is checked against its fingerprint in the file before it counts, because the file's
    /// bytes may not have reached the disk even when the checkpoint did.
    pub fn checkpoint(
        &self,
        id: u64,
        blocks: &[(u64, [u8; 32])],
        durable: bool,
    ) -> Result<(), JournalError> {
        let mut tx = self.db.begin_write().map_err(storage)?;
        if !durable {
            tx.set_durability(redb::Durability::None).map_err(storage)?;
        }
        {
            let entries = tx.open_table(ENTRIES).map_err(storage)?;
            let entry = read_entry(&entries, id)?;
            if !matches!(entry.state, State::Staged { .. }) {
                return Err(JournalError::OutOfOrder {
                    from: entry.state.name(),
                    to: "checkpoint",
                });
            }
            let mut table = tx.open_table(BLOCKS).map_err(storage)?;
            for (block, hash) in blocks {
                table.insert((id, *block), hash).map_err(storage)?;
            }
        }
        tx.commit().map_err(storage)
    }

    /// The blocks checkpointed for entry `id`, in order (claims only: check each against the file).
    pub fn blocks(&self, id: u64) -> Result<Vec<(u64, [u8; 32])>, JournalError> {
        let tx = self.db.begin_read().map_err(storage)?;
        let table = tx.open_table(BLOCKS).map_err(storage)?;
        let mut out = Vec::new();
        for row in table.range((id, 0)..=(id, u64::MAX)).map_err(storage)? {
            let (k, v) = row.map_err(storage)?;
            out.push((k.value().1, v.value()));
        }
        Ok(out)
    }

    /// Every byte arrived and was checked, the file did not change while it was read, and it was
    /// flushed to disk; `fingerprint` is the whole file's, `file` its identity on its drive.
    pub fn verified(
        &self,
        id: u64,
        fingerprint: [u8; 32],
        file: Option<FileId>,
    ) -> Result<(), JournalError> {
        self.step(
            id,
            "verified",
            |s| match s {
                State::Staged { temp } => Some(State::Verified {
                    temp: temp.clone(),
                    fingerprint,
                    file,
                }),
                _ => None,
            },
            |_, _| Ok(()),
        )
    }

    /// The real name it is getting, `final_path` inside the destination: recorded before the file
    /// gets it. Called again with another name if something took this one first.
    pub fn applied(&self, id: u64, final_path: &str) -> Result<(), JournalError> {
        self.step(
            id,
            "applied",
            |s| match s {
                State::Verified {
                    temp,
                    fingerprint,
                    file,
                }
                | State::Applied {
                    temp,
                    fingerprint,
                    file,
                    ..
                } => Some(State::Applied {
                    temp: temp.clone(),
                    final_path: final_path.into(),
                    fingerprint: *fingerprint,
                    file: *file,
                }),
                _ => None,
            },
            |_, _| Ok(()),
        )
    }

    /// Finished, under the name it was applied with, as `landed`.
    pub fn committed(&self, id: u64, landed: Landed) -> Result<(), JournalError> {
        self.step(
            id,
            "committed",
            |s| match s {
                State::Applied {
                    final_path,
                    fingerprint,
                    ..
                } => Some(State::Committed {
                    final_path: final_path.clone(),
                    fingerprint: *fingerprint,
                    landed,
                }),
                _ => None,
            },
            |_, _| Ok(()),
        )
    }

    /// An identical file was already at `stored_path`, so nothing was written.
    pub fn existing(&self, id: u64, stored_path: &str) -> Result<(), JournalError> {
        self.step(
            id,
            "existing",
            |s| match s {
                State::Planned | State::Staged { .. } => Some(State::Existing {
                    stored_path: stored_path.into(),
                }),
                _ => None,
            },
            |_, _| Ok(()),
        )
    }

    /// It did not finish, because of `why`.
    pub fn failed(&self, id: u64, why: &str) -> Result<(), JournalError> {
        self.step(
            id,
            "failed",
            |s| {
                (!s.is_finished()).then(|| State::Failed {
                    why: why.into(),
                    reached: Box::new(s.clone()),
                })
            },
            |_, _| Ok(()),
        )
    }

    /// Takes one step as its own committed transaction, if `next` allows it from where it is, with
    /// `also` recorded in the same transaction. A step that finishes the write takes it off the
    /// unfinished lists.
    fn step(
        &self,
        id: u64,
        to: &'static str,
        next: impl FnOnce(&State) -> Option<State>,
        also: impl FnOnce(&redb::WriteTransaction, &Entry) -> Result<(), JournalError>,
    ) -> Result<(), JournalError> {
        let tx = self.db.begin_write().map_err(storage)?;
        {
            let mut entries = tx.open_table(ENTRIES).map_err(storage)?;
            let mut entry = read_entry(&entries, id)?;
            entry.state = next(&entry.state).ok_or(JournalError::OutOfOrder {
                from: entry.state.name(),
                to,
            })?;
            write_entry(&mut entries, &entry)?;
            drop(entries);
            // Block checkpoints are only for continuing a file not yet complete.
            if !matches!(entry.state, State::Staged { .. }) {
                let mut blocks = tx.open_table(BLOCKS).map_err(storage)?;
                blocks
                    .retain_in((id, 0)..=(id, u64::MAX), |_, _| false)
                    .map_err(storage)?;
            }
            if entry.state.is_finished() {
                let mut unfinished = tx.open_table(UNFINISHED).map_err(storage)?;
                unfinished.remove(id).map_err(storage)?;
                let mut open = tx.open_table(OPEN_ITEMS).map_err(storage)?;
                let key = open_key(&entry.write);
                let current = open.get(key.as_str()).map_err(storage)?.map(|v| v.value());
                if current == Some(id) {
                    open.remove(key.as_str()).map_err(storage)?;
                }
            }
            also(&tx, &entry)?;
        }
        tx.commit().map_err(storage)
    }

    /// One entry, if there is one with this number.
    pub fn entry(&self, id: u64) -> Result<Option<Entry>, JournalError> {
        let tx = self.db.begin_read().map_err(storage)?;
        let entries = tx.open_table(ENTRIES).map_err(storage)?;
        entries
            .get(id)
            .map_err(storage)?
            .map(|v| serde_json::from_slice(v.value()).map_err(damaged))
            .transpose()
    }

    /// Every entry, in the order planned.
    pub fn entries(&self) -> Result<Vec<Entry>, JournalError> {
        self.entries_after(0)
    }

    /// Every entry numbered above `after`, in the order planned.
    pub fn entries_after(&self, after: u64) -> Result<Vec<Entry>, JournalError> {
        let tx = self.db.begin_read().map_err(storage)?;
        let entries = tx.open_table(ENTRIES).map_err(storage)?;
        let mut out = Vec::new();
        for row in entries.range(after.saturating_add(1)..).map_err(storage)? {
            let (_, v) = row.map_err(storage)?;
            out.push(serde_json::from_slice(v.value()).map_err(damaged)?);
        }
        Ok(out)
    }

    /// Entries not yet finished, in the order planned: what recovery looks at. Only these are
    /// read, however long the journal is.
    pub fn unfinished(&self) -> Result<Vec<Entry>, JournalError> {
        let tx = self.db.begin_read().map_err(storage)?;
        let unfinished = tx.open_table(UNFINISHED).map_err(storage)?;
        let entries = tx.open_table(ENTRIES).map_err(storage)?;
        let mut out = Vec::new();
        for row in unfinished.iter().map_err(storage)? {
            let (k, _) = row.map_err(storage)?;
            out.push(read_entry(&entries, k.value())?);
        }
        Ok(out)
    }

    /// The folders made for writes, in no particular order.
    pub fn made_folders(&self) -> Result<Vec<MadeFolder>, JournalError> {
        let tx = self.db.begin_read().map_err(storage)?;
        let folders = tx.open_table(FOLDERS).map_err(storage)?;
        let mut out = Vec::new();
        for row in folders.iter().map_err(storage)? {
            let (_, v) = row.map_err(storage)?;
            out.push(serde_json::from_slice(v.value()).map_err(damaged)?);
        }
        Ok(out)
    }

    /// Whether undo is still open, as recorded on disk.
    pub fn undo_gate(&self) -> Result<UndoGate, JournalError> {
        let tx = self.db.begin_read().map_err(storage)?;
        let gate = tx.open_table(GATE).map_err(storage)?;
        match gate.get(UNDO_GATE).map_err(storage)?.map(|v| v.value()) {
            None => Ok(UndoGate::Open),
            Some(CLOSED) => Ok(UndoGate::Closed),
            Some(other) => Err(damaged(format!("unknown undo gate {other}"))),
        }
    }

    /// Leave to undo one file, while undo is open; [`JournalError::UndoClosed`] once the wipe has
    /// started (also after a restart, and also to finish an undo a crash interrupted), or as soon
    /// as closing has begun. Give it back (drop it) when that file is done. Never call
    /// [`Journal::close_undo`] on a thread holding one: it would wait for itself.
    pub fn begin_undo(&self) -> Result<UndoPermit<'_>, JournalError> {
        let mut undo = self.undo.lock().map_err(poisoned)?;
        if undo.closing {
            return Err(JournalError::UndoClosed);
        }
        // Read while holding the count, so closing cannot slip in between.
        if self.undo_gate()? == UndoGate::Closed {
            return Err(JournalError::UndoClosed);
        }
        undo.permits += 1;
        Ok(UndoPermit { journal: self })
    }

    /// Closes undo for good, as the wipe of the old laptop starts, waiting at most [`CLOSE_WAIT`]
    /// for files still being undone. See [`Journal::close_undo_within`].
    pub fn close_undo(
        &self,
        resolve: &mut dyn FnMut(&CloseTicket<'_>, &Pending) -> io::Result<Resolved>,
    ) -> Result<ClosedToken, CloseError> {
        self.close_undo_within(CLOSE_WAIT, resolve)
    }

    /// Closes undo for good, as the wipe of the old laptop starts:
    ///
    /// 1. From now no permit is handed out.
    /// 2. It waits up to `wait` for every file still being undone; if one is still held then,
    ///    [`CloseError::Busy`] (it never hangs).
    /// 3. It asks `resolve` to finish each undo left part-way ([`Pending`]: every stage but
    ///    Done), lending it a [`CloseTicket`] for journaling its own steps (a Putting before a
    ///    rename, say). `resolve` must have every change it made on the drive flushed (the folder
    ///    included) before it returns.
    /// 4. If every one was finished, it records all their Done records and the close in one
    ///    transaction, and in that same transaction refuses to close if any stage but Done is
    ///    still there (one begun while closing, say). The token is returned only once that is on
    ///    disk (redb's default, immediate durability).
    ///
    /// Any [`Resolved::Unresolved`], or an error from `resolve`, is [`CloseError::Unresolved`]
    /// with every such file listed: nothing is recorded (each rule of the resolver can simply be
    /// run again), nothing is closed, the wipe cannot be sent, and undo carries on. If the
    /// drive never comes back, the person may choose to leave those files
    /// ([`Journal::offer_escape`]). Only a failure to write the close itself keeps undo shut in
    /// this PCTwin until it restarts (whether it reached the disk is then not known; the disk
    /// decides at the next start).
    ///
    /// Already closed, it returns a token again, asking nothing, so after a crash between the
    /// close and the wipe the wipe can be sent again. There is no way to reopen undo. Never call
    /// it on a thread holding a permit: it would wait for itself until `wait` runs out.
    pub fn close_undo_within(
        &self,
        wait: Duration,
        resolve: &mut dyn FnMut(&CloseTicket<'_>, &Pending) -> io::Result<Resolved>,
    ) -> Result<ClosedToken, CloseError> {
        let token = ClosedToken {
            journal_id: self.journal_id,
        };
        {
            let mut undo = self.undo.lock().map_err(poisoned)?;
            if undo.closer {
                return Err(CloseError::Busy);
            }
            if self.undo_gate()? == UndoGate::Closed {
                undo.closing = true;
                return Ok(token);
            }
            undo.closing = true;
            undo.closer = true;
            let deadline = Instant::now().checked_add(wait);
            while undo.permits > 0 {
                let left = deadline.map(|d| d.saturating_duration_since(Instant::now()));
                if left == Some(Duration::ZERO) {
                    undo.closing = false;
                    undo.closer = false;
                    return Err(CloseError::Busy);
                }
                undo = match left {
                    Some(left) => self.undo_free.wait_timeout(undo, left).map_err(poisoned)?.0,
                    None => self.undo_free.wait(undo).map_err(poisoned)?,
                };
            }
        }
        // From here closing is under way with the count's lock free (a permit asked for is
        // refused at once, not kept waiting); whatever way this ends, `reopen` says what is left.
        let mut reopen = Reopen {
            journal: self,
            closing_stays: false,
        };
        let pending = self.pending()?;
        let ticket = CloseTicket { journal: self };
        let mut finished = Vec::with_capacity(pending.len());
        let mut stuck = Vec::new();
        for p in &pending {
            match resolve(&ticket, p)
                .map_err(|e| e.to_string())
                .and_then(Resolved::outcome)
            {
                Ok(outcome) => finished.push((p.id, outcome)),
                Err(why) => stuck.push(UnresolvedItem {
                    id: p.id,
                    destination: p.destination.clone(),
                    path: p.path.clone(),
                    why,
                }),
            }
        }
        if !stuck.is_empty() {
            return Err(CloseError::Unresolved { items: stuck });
        }
        let tx = self.db.begin_write().map_err(storage)?;
        {
            let entries = tx.open_table(ENTRIES).map_err(storage)?;
            let mut table = tx.open_table(UNDO).map_err(storage)?;
            for (id, outcome) in finished {
                put_undo(&entries, &mut table, id, &Undo::Done { outcome })?;
            }
            // Closed is never on disk beside a stage that is not Done: checked in this very
            // transaction, so nothing written meanwhile can slip past.
            let left = pending_in(&entries, &table)?;
            if !left.is_empty() {
                let items = left
                    .into_iter()
                    .map(|p| UnresolvedItem {
                        id: p.id,
                        destination: p.destination,
                        path: p.path,
                        why: "it was still being undone while undo was closing".into(),
                    })
                    .collect();
                // Returning drops the transaction uncommitted: nothing of it is written.
                return Err(CloseError::Unresolved { items });
            }
            let mut gate = tx.open_table(GATE).map_err(storage)?;
            gate.insert(UNDO_GATE, CLOSED).map_err(storage)?;
        }
        // If the commit fails, it is not known whether the close reached the disk: undo stays
        // shut in this PCTwin, and the disk decides at the next start.
        reopen.closing_stays = true;
        tx.commit().map_err(storage)?;
        Ok(token)
    }

    /// Every undo left part-way (any stage but Done), in entry order.
    fn pending(&self) -> Result<Vec<Pending>, JournalError> {
        let tx = self.db.begin_read().map_err(storage)?;
        let entries = tx.open_table(ENTRIES).map_err(storage)?;
        let table = tx.open_table(UNDO).map_err(storage)?;
        pending_in(&entries, &table)
    }

    /// Offers to leave every file still part-way where it is, for the person to remove
    /// themselves, when a drive holding one never comes back and undo cannot close: the exact
    /// stored path of each hidden name to show, and a new random nonce. Only the newest offer can
    /// be accepted, once; it is kept in memory only.
    pub fn offer_escape(&self) -> Result<EscapeOffer, JournalError> {
        let shown = self.pending()?;
        let mut nonce = [0u8; 16];
        getrandom::fill(&mut nonce).map_err(storage)?;
        let items = shown.iter().map(escape_item).collect();
        *self.escape.lock().map_err(poisoned)? = Some(StoredOffer { nonce, shown });
        Ok(EscapeOffer { nonce, items })
    }

    /// The person chose "Leave these files; I'll remove them myself" for the offer with this
    /// nonce: records each file it showed as kept where it is ([`UndoOutcome::KeptAt`] at its
    /// exact path), in one transaction, after which undo can close. Works once, only for the
    /// newest offer of this journal, and only while what it showed is exactly what is still
    /// part-way; otherwise [`JournalError::EscapeRefused`] and nothing is recorded (offer again).
    /// [`JournalError::Busy`] while a file is being undone or undo is closing;
    /// [`JournalError::UndoClosed`] once closed.
    pub fn accept_escape(&self, nonce: &[u8; 16]) -> Result<(), JournalError> {
        // Held throughout, so no permit can be taken and no close can start meanwhile.
        let undo = self.undo.lock().map_err(poisoned)?;
        if self.undo_gate()? == UndoGate::Closed {
            return Err(JournalError::UndoClosed);
        }
        if undo.closing || undo.permits > 0 {
            return Err(JournalError::Busy);
        }
        // Taken whatever the answer: an offer is answered once.
        let offer = self.escape.lock().map_err(poisoned)?.take();
        let Some(offer) = offer.filter(|o| o.nonce == *nonce) else {
            return Err(JournalError::EscapeRefused);
        };
        let tx = self.db.begin_write().map_err(storage)?;
        {
            let entries = tx.open_table(ENTRIES).map_err(storage)?;
            let mut table = tx.open_table(UNDO).map_err(storage)?;
            if pending_in(&entries, &table)? != offer.shown {
                return Err(JournalError::EscapeRefused);
            }
            for p in &offer.shown {
                let outcome = UndoOutcome::KeptAt {
                    at: escape_item(p).path,
                    why: LEFT_BY_CHOICE.into(),
                };
                put_undo(&entries, &mut table, p.id, &Undo::Done { outcome })?;
            }
        }
        tx.commit().map_err(storage)?;
        drop(undo);
        Ok(())
    }

    /// An error unless `right` was granted by this journal.
    fn check_right(&self, right: &impl UndoRight) -> Result<(), JournalError> {
        if std::ptr::eq(sealed::Sealed::granted_by(right), self) {
            Ok(())
        } else {
            Err(JournalError::OtherJournal)
        }
    }

    /// Records a step of undoing entry `id` (a committed write), with leave from this journal:
    /// for the single steps (a Putting before each rename tried, a Salvaging, or one Done).
    pub fn record_undo(
        &self,
        right: &impl UndoRight,
        id: u64,
        undo: &Undo,
    ) -> Result<(), JournalError> {
        self.write_undos(right, std::iter::once((id, undo.clone())))
    }

    /// Records a group of removals about to happen (each an [`Undo::Removing`] with its private
    /// name already chosen) in one durable transaction, before any of them is moved aside: about
    /// 64 at a time keeps a long undo to one flush per group. All or nothing.
    pub fn record_removals(
        &self,
        right: &impl UndoRight,
        items: &[(u64, Undo)],
    ) -> Result<(), JournalError> {
        if let Some((_, other)) = items
            .iter()
            .find(|(_, u)| !matches!(u, Undo::Removing { .. }))
        {
            return Err(JournalError::OutOfOrder {
                from: other.name(),
                to: "removing",
            });
        }
        self.write_undos(right, items.iter().cloned())
    }

    /// Records how a group of undos ended, in one durable transaction. All or nothing.
    ///
    /// Call it only once every folder the group touched has been flushed to disk: a Done on disk
    /// before its folder could hide a removal or rename that a power cut then took back.
    pub fn finish_undos(
        &self,
        right: &impl UndoRight,
        items: &[(u64, UndoOutcome)],
    ) -> Result<(), JournalError> {
        self.write_undos(
            right,
            items.iter().map(|(id, outcome)| {
                (
                    *id,
                    Undo::Done {
                        outcome: outcome.clone(),
                    },
                )
            }),
        )
    }

    /// Writes undo records in one transaction, each checked to follow the last.
    fn write_undos(
        &self,
        right: &impl UndoRight,
        items: impl Iterator<Item = (u64, Undo)>,
    ) -> Result<(), JournalError> {
        self.check_right(right)?;
        let tx = self.db.begin_write().map_err(storage)?;
        {
            let entries = tx.open_table(ENTRIES).map_err(storage)?;
            let mut table = tx.open_table(UNDO).map_err(storage)?;
            for (id, undo) in items {
                put_undo(&entries, &mut table, id, &undo)?;
            }
        }
        tx.commit().map_err(storage)
    }

    /// How far undoing entry `id` got, if it was started.
    pub fn undo_of(&self, id: u64) -> Result<Option<Undo>, JournalError> {
        let tx = self.db.begin_read().map_err(storage)?;
        let table = tx.open_table(UNDO).map_err(storage)?;
        table
            .get(id)
            .map_err(storage)?
            .map(|v| serde_json::from_slice(v.value()).map_err(damaged))
            .transpose()
    }

    /// Records how undoing a folder made for a write ended, with leave from this journal.
    pub fn record_folder_undo(
        &self,
        right: &impl UndoRight,
        destination: &str,
        folder: &str,
        outcome: &UndoOutcome,
    ) -> Result<(), JournalError> {
        self.check_right(right)?;
        let tx = self.db.begin_write().map_err(storage)?;
        {
            let made = tx.open_table(FOLDERS).map_err(storage)?;
            if made.get((destination, folder)).map_err(storage)?.is_none() {
                return Err(JournalError::Damaged(
                    "undo of a folder PCTwin did not make".into(),
                ));
            }
            let mut table = tx.open_table(UNDO_FOLDERS).map_err(storage)?;
            let bytes = serde_json::to_vec(outcome).map_err(storage)?;
            table
                .insert((destination, folder), bytes.as_slice())
                .map_err(storage)?;
        }
        tx.commit().map_err(storage)
    }

    /// How undoing a folder made for a write ended, if it was undone.
    pub fn folder_undo_of(
        &self,
        destination: &str,
        folder: &str,
    ) -> Result<Option<UndoOutcome>, JournalError> {
        let tx = self.db.begin_read().map_err(storage)?;
        let table = tx.open_table(UNDO_FOLDERS).map_err(storage)?;
        table
            .get((destination, folder))
            .map_err(storage)?
            .map(|v| serde_json::from_slice(v.value()).map_err(damaged))
            .transpose()
    }

    /// Entries up to this number have had their temporary files cleaned up.
    pub fn swept_upto(&self) -> Result<u64, JournalError> {
        let tx = self.db.begin_read().map_err(storage)?;
        let counters = tx.open_table(COUNTERS).map_err(storage)?;
        Ok(counters
            .get(SWEPT_UPTO)
            .map_err(storage)?
            .map_or(0, |v| v.value()))
    }

    /// Records a clean-up in one transaction: entries up to `upto` are swept, `leftovers` still
    /// have a temporary file that could not be removed (looked at again next time), and `cleared`
    /// no longer do.
    pub fn record_sweep(
        &self,
        upto: u64,
        leftovers: &[u64],
        cleared: &[u64],
    ) -> Result<(), JournalError> {
        let tx = self.db.begin_write().map_err(storage)?;
        {
            let mut counters = tx.open_table(COUNTERS).map_err(storage)?;
            counters.insert(SWEPT_UPTO, upto).map_err(storage)?;
            let mut left = tx.open_table(LEFTOVERS).map_err(storage)?;
            for id in cleared {
                left.remove(*id).map_err(storage)?;
            }
            for id in leftovers {
                left.insert(*id, ()).map_err(storage)?;
            }
        }
        tx.commit().map_err(storage)
    }

    /// Entries whose temporary file could not be removed at the last clean-up.
    pub fn leftovers(&self) -> Result<Vec<u64>, JournalError> {
        let tx = self.db.begin_read().map_err(storage)?;
        let left = tx.open_table(LEFTOVERS).map_err(storage)?;
        let mut out = Vec::new();
        for row in left.iter().map_err(storage)? {
            out.push(row.map_err(storage)?.0.value());
        }
        Ok(out)
    }
}

/// The steps a write is recorded through, as the receiving side uses them. [`Journal`] is the real
/// one; the receiver takes any, so a test can make the record fail at any step (fault injection).
pub trait Ledger {
    /// See [`Journal::temp_tag`].
    fn temp_tag(&self, id: u64) -> String;
    fn plan(&self, write: &PlannedWrite) -> Result<u64, JournalError>;
    fn staged(
        &self,
        id: u64,
        temp: &str,
        made: &[(String, Option<FileId>)],
    ) -> Result<(), JournalError>;
    fn verified(
        &self,
        id: u64,
        fingerprint: [u8; 32],
        file: Option<FileId>,
    ) -> Result<(), JournalError>;
    fn applied(&self, id: u64, final_path: &str) -> Result<(), JournalError>;
    fn committed(&self, id: u64, landed: Landed) -> Result<(), JournalError>;
    fn existing(&self, id: u64, stored_path: &str) -> Result<(), JournalError>;
    fn failed(&self, id: u64, why: &str) -> Result<(), JournalError>;
    fn checkpoint(
        &self,
        id: u64,
        blocks: &[(u64, [u8; 32])],
        durable: bool,
    ) -> Result<(), JournalError>;
    fn blocks(&self, id: u64) -> Result<Vec<(u64, [u8; 32])>, JournalError>;
    fn unfinished(&self) -> Result<Vec<Entry>, JournalError>;
}

impl Ledger for Journal {
    fn temp_tag(&self, id: u64) -> String {
        Journal::temp_tag(self, id)
    }
    fn plan(&self, write: &PlannedWrite) -> Result<u64, JournalError> {
        Journal::plan(self, write)
    }
    fn staged(
        &self,
        id: u64,
        temp: &str,
        made: &[(String, Option<FileId>)],
    ) -> Result<(), JournalError> {
        Journal::staged(self, id, temp, made)
    }
    fn verified(
        &self,
        id: u64,
        fingerprint: [u8; 32],
        file: Option<FileId>,
    ) -> Result<(), JournalError> {
        Journal::verified(self, id, fingerprint, file)
    }
    fn applied(&self, id: u64, final_path: &str) -> Result<(), JournalError> {
        Journal::applied(self, id, final_path)
    }
    fn committed(&self, id: u64, landed: Landed) -> Result<(), JournalError> {
        Journal::committed(self, id, landed)
    }
    fn existing(&self, id: u64, stored_path: &str) -> Result<(), JournalError> {
        Journal::existing(self, id, stored_path)
    }
    fn failed(&self, id: u64, why: &str) -> Result<(), JournalError> {
        Journal::failed(self, id, why)
    }
    fn checkpoint(
        &self,
        id: u64,
        blocks: &[(u64, [u8; 32])],
        durable: bool,
    ) -> Result<(), JournalError> {
        Journal::checkpoint(self, id, blocks, durable)
    }
    fn blocks(&self, id: u64) -> Result<Vec<(u64, [u8; 32])>, JournalError> {
        Journal::blocks(self, id)
    }
    fn unfinished(&self) -> Result<Vec<Entry>, JournalError> {
        Journal::unfinished(self)
    }
}

/// Records `undo` for entry `id` (a committed write) if it may follow what is there.
fn put_undo(
    entries: &impl ReadableTable<u64, &'static [u8]>,
    table: &mut redb::Table<'_, u64, &'static [u8]>,
    id: u64,
    undo: &Undo,
) -> Result<(), JournalError> {
    let entry = read_entry(entries, id)?;
    if !matches!(entry.state, State::Committed { .. }) {
        return Err(JournalError::OutOfOrder {
            from: entry.state.name(),
            to: "undo",
        });
    }
    for name in undo.names() {
        plain_name(name)?;
    }
    let before: Option<Undo> = table
        .get(id)
        .map_err(storage)?
        .map(|v| serde_json::from_slice(v.value()).map_err(damaged))
        .transpose()?;
    if !may_follow(before.as_ref(), undo) {
        return Err(JournalError::OutOfOrder {
            from: before.as_ref().map_or("none", Undo::name),
            to: undo.name(),
        });
    }
    let bytes = serde_json::to_vec(undo).map_err(storage)?;
    table.insert(id, bytes.as_slice()).map_err(storage)?;
    Ok(())
}

/// Every undo left part-way (any stage but Done) in these tables, in entry order.
fn pending_in(
    entries: &impl ReadableTable<u64, &'static [u8]>,
    table: &impl ReadableTable<u64, &'static [u8]>,
) -> Result<Vec<Pending>, JournalError> {
    let mut out = Vec::new();
    for row in table.iter().map_err(storage)? {
        let (k, v) = row.map_err(storage)?;
        let stage: Undo = serde_json::from_slice(v.value()).map_err(damaged)?;
        if stage.is_done() {
            continue;
        }
        let id = k.value();
        let entry = read_entry(entries, id)?;
        let State::Committed { final_path, .. } = entry.state else {
            return Err(damaged(format!(
                "undo of entry {id}, which is not finished"
            )));
        };
        out.push(Pending {
            id,
            stage,
            destination: entry.write.destination,
            path: final_path,
        });
    }
    Ok(out)
}

/// What the person would be leaving for one part-way undo: its hidden name's exact path.
fn escape_item(p: &Pending) -> EscapeItem {
    EscapeItem {
        id: p.id,
        destination: p.destination.clone(),
        path: p.in_folder(p.stage.hidden_name().unwrap_or_default()),
    }
}

/// Ends a close under way: unless the close was being written, undo carries on (also if the
/// resolver panicked); either way the next close may start.
struct Reopen<'j> {
    journal: &'j Journal,
    closing_stays: bool,
}

impl Drop for Reopen<'_> {
    fn drop(&mut self) {
        let mut undo = self
            .journal
            .undo
            .lock()
            .unwrap_or_else(PoisonError::into_inner);
        undo.closer = false;
        if !self.closing_stays {
            undo.closing = false;
        }
    }
}

fn read_entry(
    entries: &impl ReadableTable<u64, &'static [u8]>,
    id: u64,
) -> Result<Entry, JournalError> {
    let found = entries
        .get(id)
        .map_err(storage)?
        .ok_or(JournalError::NoSuchEntry(id))?;
    serde_json::from_slice(found.value()).map_err(damaged)
}

fn write_entry(
    entries: &mut redb::Table<'_, u64, &'static [u8]>,
    entry: &Entry,
) -> Result<(), JournalError> {
    let bytes = serde_json::to_vec(entry).map_err(storage)?;
    entries
        .insert(entry.id, bytes.as_slice())
        .map_err(storage)?;
    Ok(())
}
