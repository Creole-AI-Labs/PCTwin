//! PCTwin change journal (Task List 1.6; Engineering Plan: "write durable intent and undo records
//! before any change").
//!
//! Every write on the new laptop is one entry that moves, one durable step at a time, through
//! [`State::Planned`], [`State::Staged`] (its temporary `.pctwin-` file exists), [`State::Verified`]
//! (every byte arrived, checked, and flushed to disk), [`State::Applied`] (it has its real name) and
//! [`State::Committed`] (finished; what it landed as is kept for undo), or ends [`State::Failed`]
//! with the reason and the step it had reached. Steps cannot be skipped, repeated or taken back.
//!
//! Each step is its own committed transaction in a [redb](https://docs.rs/redb) database: crash-safe
//! by default, with two checksummed commit slots, so a write torn by power loss is detected and the
//! last good commit is used. After a crash the journal holds exactly the steps already taken; the
//! app reconciles anything unfinished instead of announcing success. A damaged journal is reported,
//! never trusted, and a journal from a newer PCTwin is refused.
//!
//! Each entry records the item, the old laptop it came from, the approved destination and path,
//! and who acted for whom with what permission.

use std::path::Path;

use pctwin_record::{ItemId, LaptopId};
use redb::{Database, ReadableDatabase, ReadableTable, TableDefinition};
use serde::{Deserialize, Serialize};

/// The journal's format. A journal with a higher number came from a newer PCTwin.
pub const FORMAT: u32 = 1;

const META: TableDefinition<&str, u32> = TableDefinition::new("meta");
const ENTRIES: TableDefinition<u64, &[u8]> = TableDefinition::new("entries");

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
}

/// What a finished file landed as: undo compares against this to keep later edits.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct Landed {
    pub size: u64,
    /// Nanoseconds since 1970.
    pub modified_ns: Option<i64>,
}

/// How far a write got.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case", tag = "step")]
pub enum State {
    Planned,
    /// Its temporary file (in the destination folder) exists.
    Staged {
        temp: String,
    },
    /// Every byte arrived and was checked, and the file was flushed to disk.
    Verified {
        temp: String,
        fingerprint: [u8; 32],
    },
    /// It has its real name (never replacing another file).
    Applied {
        final_path: String,
        fingerprint: [u8; 32],
    },
    Committed {
        final_path: String,
        fingerprint: [u8; 32],
        landed: Landed,
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
            State::Failed { .. } => "failed",
        }
    }

    /// Committed and failed writes are finished.
    pub fn is_finished(&self) -> bool {
        matches!(self, State::Committed { .. } | State::Failed { .. })
    }
}

/// One write and how far it got.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Entry {
    pub id: u64,
    pub write: PlannedWrite,
    pub state: State,
}

/// The change journal for a move on this laptop.
pub struct Journal {
    db: Database,
}

impl std::fmt::Debug for Journal {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str("Journal(..)")
    }
}

impl Journal {
    /// Opens the journal at `path`, creating it if there is none.
    pub fn open(path: &Path) -> Result<Self, JournalError> {
        let db = Database::create(path).map_err(|e| match e {
            redb::DatabaseError::DatabaseAlreadyOpen => JournalError::InUse,
            other => JournalError::Damaged(other.to_string()),
        })?;
        let journal = Self { db };
        journal.check_format()?;
        Ok(journal)
    }

    fn check_format(&self) -> Result<(), JournalError> {
        let tx = self.db.begin_write().map_err(storage)?;
        {
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
            tx.open_table(ENTRIES).map_err(storage)?;
        }
        tx.commit().map_err(storage)
    }

    /// Records a write about to start; returns its entry number (never reused).
    pub fn plan(&self, write: &PlannedWrite) -> Result<u64, JournalError> {
        let tx = self.db.begin_write().map_err(storage)?;
        let id = {
            let mut entries = tx.open_table(ENTRIES).map_err(storage)?;
            let id = entries
                .last()
                .map_err(storage)?
                .map_or(1, |(k, _)| k.value() + 1);
            let entry = Entry {
                id,
                write: write.clone(),
                state: State::Planned,
            };
            let bytes = serde_json::to_vec(&entry).map_err(storage)?;
            entries.insert(id, bytes.as_slice()).map_err(storage)?;
            id
        };
        tx.commit().map_err(storage)?;
        Ok(id)
    }

    /// Its temporary file `temp` exists.
    pub fn staged(&self, id: u64, temp: &str) -> Result<(), JournalError> {
        self.step(id, "staged", |s| match s {
            State::Planned => Some(State::Staged { temp: temp.into() }),
            _ => None,
        })
    }

    /// Every byte arrived, was checked, and was flushed to disk; `fingerprint` is the whole file's.
    pub fn verified(&self, id: u64, fingerprint: [u8; 32]) -> Result<(), JournalError> {
        self.step(id, "verified", |s| match s {
            State::Staged { temp } => Some(State::Verified {
                temp: temp.clone(),
                fingerprint,
            }),
            _ => None,
        })
    }

    /// It has its real name, `final_path` inside the destination.
    pub fn applied(&self, id: u64, final_path: &str) -> Result<(), JournalError> {
        self.step(id, "applied", |s| match s {
            State::Verified { fingerprint, .. } => Some(State::Applied {
                final_path: final_path.into(),
                fingerprint: *fingerprint,
            }),
            _ => None,
        })
    }

    /// Finished, as `landed`.
    pub fn committed(&self, id: u64, landed: Landed) -> Result<(), JournalError> {
        self.step(id, "committed", |s| match s {
            State::Applied {
                final_path,
                fingerprint,
            } => Some(State::Committed {
                final_path: final_path.clone(),
                fingerprint: *fingerprint,
                landed,
            }),
            _ => None,
        })
    }

    /// It did not finish, because of `why`.
    pub fn failed(&self, id: u64, why: &str) -> Result<(), JournalError> {
        self.step(id, "failed", |s| {
            (!s.is_finished()).then(|| State::Failed {
                why: why.into(),
                reached: Box::new(s.clone()),
            })
        })
    }

    /// Takes one step as its own committed transaction, if `next` allows it from where it is.
    fn step(
        &self,
        id: u64,
        to: &'static str,
        next: impl FnOnce(&State) -> Option<State>,
    ) -> Result<(), JournalError> {
        let tx = self.db.begin_write().map_err(storage)?;
        {
            let mut entries = tx.open_table(ENTRIES).map_err(storage)?;
            let mut entry: Entry = {
                let found = entries
                    .get(id)
                    .map_err(storage)?
                    .ok_or(JournalError::NoSuchEntry(id))?;
                serde_json::from_slice(found.value())
                    .map_err(|e| JournalError::Damaged(e.to_string()))?
            };
            entry.state = next(&entry.state).ok_or(JournalError::OutOfOrder {
                from: entry.state.name(),
                to,
            })?;
            let bytes = serde_json::to_vec(&entry).map_err(storage)?;
            entries.insert(id, bytes.as_slice()).map_err(storage)?;
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
            .map(|v| {
                serde_json::from_slice(v.value()).map_err(|e| JournalError::Damaged(e.to_string()))
            })
            .transpose()
    }

    /// Every entry, in the order planned.
    pub fn entries(&self) -> Result<Vec<Entry>, JournalError> {
        let tx = self.db.begin_read().map_err(storage)?;
        let entries = tx.open_table(ENTRIES).map_err(storage)?;
        let mut out = Vec::new();
        for row in entries.iter().map_err(storage)? {
            let (_, v) = row.map_err(storage)?;
            out.push(
                serde_json::from_slice(v.value())
                    .map_err(|e| JournalError::Damaged(e.to_string()))?,
            );
        }
        Ok(out)
    }

    /// Entries not yet committed or failed, in the order planned: what recovery looks at.
    pub fn unfinished(&self) -> Result<Vec<Entry>, JournalError> {
        Ok(self
            .entries()?
            .into_iter()
            .filter(|e| !e.state.is_finished())
            .collect())
    }
}
