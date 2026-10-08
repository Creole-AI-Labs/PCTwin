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

pub mod recovery;

use std::path::Path;

use pctwin_record::{ItemId, LaptopId};
use redb::{Database, ReadableDatabase, ReadableTable, TableDefinition};
use serde::{Deserialize, Serialize};

/// The journal's format. A journal with a higher number came from a newer PCTwin.
pub const FORMAT: u32 = 1;

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
    #[error("no such entry in the move's record: {0}")]
    NoSuchEntry(u64),
    #[error("a step was taken out of order ({from} to {to})")]
    OutOfOrder {
        from: &'static str,
        to: &'static str,
    },
    #[error("the move's record could not be written: {0}")]
    Storage(String),
}

fn storage(e: impl std::fmt::Display) -> JournalError {
    JournalError::Storage(e.to_string())
}

fn damaged(e: impl std::fmt::Display) -> JournalError {
    JournalError::Damaged(e.to_string())
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

/// Which file or folder this is on its drive (the drive's number and the file's number on it):
/// two names with the same identity are the same file.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct FileId {
    pub volume: u64,
    pub index: u64,
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
/// it is the history of the move).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case", tag = "step")]
pub enum Undo {
    /// About to move this file (by identity) to the Trash: recorded first, so after a crash undo
    /// knows to check whether it already went.
    Moving { file: Option<FileId> },
    /// Done; never looked at again, unless it could not be done.
    Done { outcome: UndoOutcome },
}

/// How undoing one thing ended.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case", tag = "result")]
pub enum UndoOutcome {
    /// Moved to the system Trash or Recycle Bin, where the person can bring it back.
    Trashed,
    /// A folder PCTwin made, empty, removed.
    Removed,
    /// It was already gone.
    AlreadyGone,
    /// Kept, and why in plain words (changed since the move, not empty, no Recycle Bin there).
    Kept { why: String },
    /// It could not be done this time, and why; undo tries it again next time.
    NotDone { why: String },
}

impl UndoOutcome {
    /// Whether undo is finished with it (only "not done this time" is tried again).
    pub fn is_final(&self) -> bool {
        !matches!(self, UndoOutcome::NotDone { .. })
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

/// The change journal for a move on this laptop.
pub struct Journal {
    db: Database,
    /// This journal's own random number (names its temporary files).
    journal_id: u64,
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
        Ok(Self { db, journal_id })
    }

    /// Checks the format and makes every table; returns the journal's own number.
    fn prepare(db: &Database) -> Result<u64, JournalError> {
        let tx = db.begin_write().map_err(storage)?;
        let journal_id = {
            let mut meta = tx.open_table(META).map_err(storage)?;
            let found = meta.get("format").map_err(storage)?.map(|v| v.value());
            match found {
                Some(found) if found > FORMAT => {
                    return Err(JournalError::NewerFormat { found });
                }
                Some(_) => {}
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
            journal_id
        };
        tx.commit().map_err(storage)?;
        Ok(journal_id)
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

    /// Records a step of undoing entry `id` (a committed write).
    pub fn record_undo(&self, id: u64, undo: &Undo) -> Result<(), JournalError> {
        let tx = self.db.begin_write().map_err(storage)?;
        {
            let entries = tx.open_table(ENTRIES).map_err(storage)?;
            let entry = read_entry(&entries, id)?;
            if !matches!(entry.state, State::Committed { .. }) {
                return Err(JournalError::OutOfOrder {
                    from: entry.state.name(),
                    to: "undo",
                });
            }
            let mut table = tx.open_table(UNDO).map_err(storage)?;
            let bytes = serde_json::to_vec(undo).map_err(storage)?;
            table.insert(id, bytes.as_slice()).map_err(storage)?;
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

    /// Records how undoing a folder made for a write ended.
    pub fn record_folder_undo(
        &self,
        destination: &str,
        folder: &str,
        outcome: &UndoOutcome,
    ) -> Result<(), JournalError> {
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
