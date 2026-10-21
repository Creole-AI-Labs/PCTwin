use std::collections::BTreeMap;
use std::io::Write;
use std::path::{Path, PathBuf};

use pctwin_record::{FolderRole, ItemId, LaptopId, Record};
use pctwin_types::format::{Format, FormatError};
use serde::{Deserialize, Serialize};

use crate::scan::{Counts, Stamp};

/// The saved-scan format this app writes and reads.
pub const STATE_FORMAT: u32 = 1;

/// The saved scan's place in the shared format rule (no older formats yet).
pub const STATE_FORMAT_RULE: Format = Format {
    name: "saved scan",
    current: STATE_FORMAT,
    readable: &[],
};

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

/// Who gets the old laptop next, asked early (Product Spec: selling, family, keeping): it decides
/// what PCTwin offers once the move is done (wiping it for a new owner, leaving a family member's
/// own account, or nothing). Only ever the person's answer, never inferred.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum NextOwner {
    /// Sold or given to someone outside the household.
    Selling,
    /// Passed to someone in the family or household.
    Family,
    /// The person keeps it.
    Keeping,
    /// The person did not say yet.
    NotSure,
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
    pub(crate) next_owner: Option<NextOwner>,
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
    /// Missing in a save from before it was asked.
    #[serde(default)]
    next_owner: Option<NextOwner>,
}

/// [`Wire`] as it is saved: the same fields in the same order, borrowed, with the record written
/// into its text a piece at a time, so saving never holds the record's whole JSON.
#[derive(Serialize)]
struct SaveWire<'a> {
    format: u32,
    laptop: LaptopId,
    person: &'a str,
    started_ns: i64,
    finished: bool,
    done: &'a [DoneFolder],
    #[serde(serialize_with = "record_text")]
    record: &'a Record,
    #[serde(serialize_with = "stamp_pairs")]
    stamps: &'a BTreeMap<ItemId, Stamp>,
    next_owner: Option<NextOwner>,
}

fn record_text<S: serde::Serializer>(record: &&Record, s: S) -> Result<S::Ok, S::Error> {
    s.collect_str(&record.json_text())
}

fn stamp_pairs<S: serde::Serializer>(
    stamps: &&BTreeMap<ItemId, Stamp>,
    s: S,
) -> Result<S::Ok, S::Error> {
    s.collect_seq(stamps.iter())
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
            next_owner: None,
        }
    }

    /// Who gets the old laptop next, as the person said (`None`: not asked yet).
    pub fn next_owner(&self) -> Option<NextOwner> {
        self.next_owner
    }

    /// The person's answer to who gets the old laptop next; kept with the scan when it is saved.
    pub fn set_next_owner(&mut self, next: NextOwner) {
        self.next_owner = Some(next);
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
        let wire = SaveWire {
            format: STATE_FORMAT,
            laptop: self.laptop,
            person: &self.person,
            started_ns: self.started_ns,
            finished: self.finished,
            done: &self.done,
            record: &self.record,
            stamps: &self.stamps,
            next_owner: self.next_owner,
        };
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
        // Written a piece at a time: the whole save is never one text in memory.
        let mut out = std::io::BufWriter::new(&mut temp);
        serde_json::to_writer(&mut out, &wire).map_err(std::io::Error::other)?;
        out.flush()?;
        drop(out);
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
        STATE_FORMAT_RULE.check_json(&bytes).map_err(|e| match e {
            FormatError::Newer { found, .. } => StateError::NewerFormat { found },
            other => StateError::Damaged(other.to_string()),
        })?;
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
            next_owner: wire.next_owner,
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

#[cfg(test)]
mod saved_form {
    use super::*;
    use pctwin_record::{Inclusion, Item, ItemKind, ItemName, ItemPath, ManagedBy, Owner, Place};
    use pctwin_record::{Portability, Storage};

    /// Saving a piece at a time writes exactly what the whole-text form wrote, so scans saved by
    /// earlier versions and this one read alike.
    #[test]
    fn the_save_written_a_piece_at_a_time_is_the_same_bytes() {
        let laptop = LaptopId::from_hex("00112233445566778899aabbccddeeff").unwrap();
        let mut record = Record::new(laptop);
        let owner = Owner::Person {
            account_id: "S-1-5-21-7".into(),
        };
        let place = Place {
            role: FolderRole::Documents,
            storage: Storage::SystemDrive,
        };
        let path = ItemPath::new(vec![
            ItemName::from_text("Ré \"sumé\" \\ é\n"),
            ItemName::from_windows_wide(&[0xD800, 0x61]),
            ItemName::from_unix_bytes(&[b'x', 0xFF]),
        ])
        .unwrap();
        let id = ItemId::derive(&laptop, &owner, &place, &path);
        record.items.push(Item {
            id,
            kind: ItemKind::File,
            owner,
            place,
            path,
            size_bytes: 9,
            modified_ns: None,
            portability: Portability::Portable,
            managed_by: ManagedBy::Personal,
            download_bytes: None,
            inclusion: Inclusion::Included,
        });
        let done = vec![DoneFolder {
            role: FolderRole::Documents,
            path: r"C:\Users\Ada\Documents".into(),
            counts: Counts::default(),
            links: 2,
            unreadable: vec!["x".into()],
        }];
        let mut stamps = BTreeMap::new();
        stamps.insert(
            id,
            Stamp {
                size: 9,
                modified_ns: Some(-5),
            },
        );
        let whole = Wire {
            format: STATE_FORMAT,
            laptop,
            person: "Ada".into(),
            started_ns: 12,
            finished: true,
            done: done.clone(),
            record: String::from_utf8(record.to_json()).unwrap(),
            stamps: stamps.iter().map(|(k, v)| (*k, *v)).collect(),
            next_owner: None,
        };
        let pieces = SaveWire {
            format: STATE_FORMAT,
            laptop,
            person: "Ada",
            started_ns: 12,
            finished: true,
            done: &done,
            record: &record,
            stamps: &stamps,
            next_owner: None,
        };
        assert_eq!(
            serde_json::to_vec(&pieces).unwrap(),
            serde_json::to_vec(&whole).unwrap()
        );
    }
}
