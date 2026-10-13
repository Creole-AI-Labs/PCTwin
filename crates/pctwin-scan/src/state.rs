use std::collections::BTreeMap;
use std::io::Write;
use std::path::{Path, PathBuf};

use pctwin_record::{FolderRole, ItemId, LaptopId, Record};
use serde::{Deserialize, Serialize};

use crate::scan::{Counts, Stamp};

/// The saved-scan format this app writes and reads.
pub const STATE_FORMAT: u32 = 1;

/// A file counts as "maybe changed" if it was modified this close to (or after) the start of the
/// scan it is compared with: an edit in the same clock tick can leave size and time unchanged
/// (git's "racy" rule). Two seconds covers FAT drives, which store time in two-second steps.
const RACY_MARGIN_NS: i64 = 2_000_000_000;

/// One special folder that was fully scanned.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct DoneFolder {
    pub role: FolderRole,
    /// The folder as the system reported it (as text).
    pub path: String,
    pub counts: Counts,
    pub links: u64,
    pub unreadable: Vec<String>,
}

/// The scan as saved: enough to resume after a restart and to tell later what changed.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ScanState {
    pub(crate) laptop: LaptopId,
    /// The signed-in person's account ID.
    pub(crate) person: String,
    /// When the scan started, in nanoseconds since 1970.
    pub(crate) started_ns: i64,
    pub(crate) finished: bool,
    pub(crate) done: Vec<DoneFolder>,
    pub(crate) record: Record,
    pub(crate) stamps: BTreeMap<ItemId, Stamp>,
}

#[derive(Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
struct Wire {
    format: u32,
    laptop: LaptopId,
    person: String,
    started_ns: i64,
    finished: bool,
    done: Vec<DoneFolder>,
    /// The move record in its own format, checked on load like any record.
    record: String,
    stamps: Vec<(ItemId, Stamp)>,
}

#[derive(Deserialize)]
struct FormatOnly {
    format: u32,
}

/// Why a saved scan was not used.
#[derive(Debug, thiserror::Error)]
pub enum StateError {
    #[error("the saved scan could not be read: {0}")]
    Io(#[from] std::io::Error),
    #[error("the saved scan is damaged: {0}")]
    Damaged(String),
    #[error("the saved scan was made by a newer version of PCTwin (format {found})")]
    NewerFormat { found: u32 },
}

impl ScanState {
    pub(crate) fn new(laptop: LaptopId, person: String) -> Self {
        Self {
            laptop,
            person,
            started_ns: now_ns(),
            finished: false,
            done: Vec::new(),
            record: Record::new(laptop),
            stamps: BTreeMap::new(),
        }
    }

    pub fn record(&self) -> &Record {
        &self.record
    }

    pub fn is_finished(&self) -> bool {
        self.finished
    }

    /// The folders already fully scanned.
    pub fn done_folders(&self) -> Vec<PathBuf> {
        self.done.iter().map(|d| PathBuf::from(&d.path)).collect()
    }

    pub fn stamps(&self) -> &BTreeMap<ItemId, Stamp> {
        &self.stamps
    }

    /// Saves to `path` so that a crash leaves either the previous save or this one, never a mix:
    /// written to a temporary file beside it, flushed to disk, then moved into place.
    ///
    /// The temporary file has a random name and is made only where nothing exists (never through
    /// a file or link someone left there), readable by this account alone on Mac and Linux. It is
    /// removed on every failure, since it is deleted when dropped unless moved into place.
    pub fn save(&self, path: &Path) -> std::io::Result<()> {
        let wire = Wire {
            format: STATE_FORMAT,
            laptop: self.laptop,
            person: self.person.clone(),
            started_ns: self.started_ns,
            finished: self.finished,
            done: self.done.clone(),
            record: String::from_utf8_lossy(&self.record.to_json()).into_owned(),
            stamps: self.stamps.iter().map(|(k, v)| (*k, *v)).collect(),
        };
        let bytes = serde_json::to_vec(&wire).map_err(std::io::Error::other)?;
        let name = path
            .file_name()
            .map(|n| n.to_string_lossy().into_owned())
            .unwrap_or_else(|| "scan".into());
        let dir = match path.parent() {
            Some(dir) if !dir.as_os_str().is_empty() => dir,
            _ => Path::new("."),
        };
        let mut temp = tempfile::Builder::new()
            .prefix(&format!(".{name}."))
            .suffix(".tmp")
            .tempfile_in(dir)?;
        temp.write_all(&bytes)?;
        temp.as_file().sync_all()?;
        replace(temp, path)?;
        // On Mac and Linux the folder's own record of the new name is flushed too.
        #[cfg(unix)]
        std::fs::File::open(dir)?.sync_all()?;
        Ok(())
    }

    /// Loads a saved scan. The format is checked first, so a newer save is refused safely even if
    /// its shape changed; the record inside is checked like any record.
    pub fn load(path: &Path) -> Result<Self, StateError> {
        let bytes = std::fs::read(path)?;
        let FormatOnly { format } =
            serde_json::from_slice(&bytes).map_err(|e| StateError::Damaged(e.to_string()))?;
        if format > STATE_FORMAT {
            return Err(StateError::NewerFormat { found: format });
        }
        if format != STATE_FORMAT {
            return Err(StateError::Damaged(format!("unknown format {format}")));
        }
        let wire: Wire =
            serde_json::from_slice(&bytes).map_err(|e| StateError::Damaged(e.to_string()))?;
        let record = Record::from_json(wire.record.as_bytes())
            .map_err(|e| StateError::Damaged(e.to_string()))?;
        Ok(Self {
            laptop: wire.laptop,
            person: wire.person,
            started_ns: wire.started_ns,
            finished: wire.finished,
            done: wire.done,
            record,
            stamps: wire.stamps.into_iter().collect(),
        })
    }
}

/// Moves the temporary file onto `to`. Windows refuses for a moment while another program
/// (antivirus, search indexing) has the target open, so it tries again briefly. On failure the
/// temporary file is dropped, which removes it.
fn replace(mut temp: tempfile::NamedTempFile, to: &Path) -> std::io::Result<()> {
    for _ in 1..20 {
        match temp.persist(to) {
            Ok(_) => return Ok(()),
            Err(e) if e.error.kind() == std::io::ErrorKind::PermissionDenied => {
                temp = e.file;
                std::thread::sleep(std::time::Duration::from_millis(50));
            }
            Err(e) => return Err(e.error),
        }
    }
    temp.persist(to).map(drop).map_err(|e| e.error)
}

fn now_ns() -> i64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| i64::try_from(d.as_nanos()).unwrap_or(i64::MAX))
        .unwrap_or(0)
}

/// What changed between two scans, by item ID.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct Changes {
    pub added: Vec<ItemId>,
    pub removed: Vec<ItemId>,
    pub changed: Vec<ItemId>,
    pub unchanged: Vec<ItemId>,
}

/// Compares a later scan with an earlier one. A file is unchanged only when its size and modified
/// time match and it was not modified right around the earlier scan.
pub fn changes_between(before: &ScanState, after: &ScanState) -> Changes {
    let mut changes = Changes::default();
    let before_ids: BTreeMap<ItemId, ()> = before.record.items.iter().map(|i| (i.id, ())).collect();
    for item in &after.record.items {
        if !before_ids.contains_key(&item.id) {
            changes.added.push(item.id);
            continue;
        }
        match (before.stamps.get(&item.id), after.stamps.get(&item.id)) {
            (Some(old), Some(new)) => {
                let racy = old
                    .modified_ns
                    .is_none_or(|m| m >= before.started_ns.saturating_sub(RACY_MARGIN_NS));
                if old != new || racy {
                    changes.changed.push(item.id);
                } else {
                    changes.unchanged.push(item.id);
                }
            }
            // Folders and left-out items have no stamp; one that became a file (or the other
            // way round) has changed.
            (None, None) => changes.unchanged.push(item.id),
            _ => changes.changed.push(item.id),
        }
    }
    let after_ids: BTreeMap<ItemId, ()> = after.record.items.iter().map(|i| (i.id, ())).collect();
    changes.removed = before
        .record
        .items
        .iter()
        .filter(|i| !after_ids.contains_key(&i.id))
        .map(|i| i.id)
        .collect();
    changes
}
