//! Undoing the files a move wrote (Task List 2.3, Product Spec: "Undo reverses PCTwin's changes,
//! keeps any edits you made since, and tells you about anything that can't be reversed").
//!
//! For each file the move committed, newest first:
//! - still exactly as it landed (the same file, the same size and modified time, and then the same
//!   fingerprint): moved to the system Trash or Recycle Bin, never deleted, so it can be brought
//!   back;
//! - changed since (or a different file now has its name): kept, with "you changed this since the
//!   move, so it was kept";
//! - already gone: nothing to do.
//!
//! Each step is recorded in the journal first, so undo cut short by a crash carries on safely and
//! never takes a file a person made later under the same name. Then the folders the move made are
//! removed, deepest first, only if they are empty and are still the folders it made. Nothing that
//! could not be undone is hidden: every one is reported with why.

use std::path::Path;

use pctwin_gate::{Destination, Destinations};
use pctwin_journal::{Entry, FileId, Journal, JournalError, State, Undo, UndoOutcome};

use crate::fingerprint_reader;

/// Where undone files go: the system Trash. A trait so tests (and fault injection) can stand in.
pub trait Bin {
    /// Whether a file at `path` can be moved to the Trash at all (`Err`: why not, in plain
    /// words). Checked before anything is recorded or moved.
    fn can_take(&self, path: &Path) -> Result<(), String>;
    /// Moves the file at `path` to the Trash. Never deletes it: if it cannot be moved, it stays.
    fn put(&self, path: &Path) -> Result<(), String>;
}

/// What undo did with one thing.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Undone {
    /// The entry (for a file) or 0 for a folder.
    pub entry: u64,
    pub destination: String,
    /// Its stored path inside the destination.
    pub path: String,
    pub outcome: UndoOutcome,
}

/// What undo did, and everything it could not undo.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct UndoReport {
    pub files: Vec<Undone>,
    pub folders: Vec<Undone>,
}

impl UndoReport {
    /// Everything that was not undone, with why.
    pub fn not_undone(&self) -> impl Iterator<Item = &Undone> {
        self.files.iter().chain(&self.folders).filter(|u| {
            matches!(
                u.outcome,
                UndoOutcome::Kept { .. } | UndoOutcome::NotDone { .. }
            )
        })
    }
}

/// The plain reason for a file changed since the move.
pub const CHANGED_SINCE: &str = "you changed this since the move, so it was kept";
const NOT_REACHABLE: &str = "the place it is in could not be reached";
const NOT_SAME_PLACE: &str = "the place it is in is not the same folder as during the move";
const NOT_EMPTY: &str = "something is still in it, so it was kept";
const NOT_ITS_FOLDER: &str = "it is not the folder PCTwin made, so it was kept";

/// Undoes every file the move in `journal` committed, then the folders it made.
pub fn undo(
    journal: &Journal,
    table: &Destinations,
    bin: &dyn Bin,
) -> Result<UndoReport, JournalError> {
    let mut report = UndoReport::default();
    let mut entries = journal.entries()?;
    entries.retain(|e| matches!(e.state, State::Committed { .. }));
    entries.reverse();
    for entry in &entries {
        let earlier = journal.undo_of(entry.id)?;
        if let Some(Undo::Done { outcome }) = &earlier
            && outcome.is_final()
        {
            continue;
        }
        let outcome = undo_file(journal, table, bin, entry, earlier.as_ref())?;
        let State::Committed { final_path, .. } = &entry.state else {
            continue;
        };
        report.files.push(Undone {
            entry: entry.id,
            destination: entry.write.destination.clone(),
            path: final_path.clone(),
            outcome,
        });
    }
    let mut folders = journal.made_folders()?;
    // Deepest first, so a folder's own made folders go before it.
    folders.sort_by_key(|f| std::cmp::Reverse(f.folder.matches('/').count()));
    for made in folders {
        if journal
            .folder_undo_of(&made.destination, &made.folder)?
            .is_some_and(|o| o.is_final())
        {
            continue;
        }
        let outcome = match place(table, &made.destination, None) {
            Err(why) => UndoOutcome::NotDone { why },
            Ok(dest) => match dest.folder_identity(&made.folder) {
                Err(e) => UndoOutcome::NotDone { why: e.to_string() },
                Ok(None) => UndoOutcome::AlreadyGone,
                Ok(Some(now)) if made.id.is_some_and(|id| !same(id, now)) => UndoOutcome::Kept {
                    why: NOT_ITS_FOLDER.into(),
                },
                Ok(Some(_)) => match dest.remove_empty_folder(&made.folder) {
                    Ok(true) => UndoOutcome::Removed,
                    Ok(false) => UndoOutcome::Kept {
                        why: NOT_EMPTY.into(),
                    },
                    Err(e) => UndoOutcome::NotDone { why: e.to_string() },
                },
            },
        };
        journal.record_folder_undo(&made.destination, &made.folder, &outcome)?;
        report.folders.push(Undone {
            entry: made.entry,
            destination: made.destination,
            path: made.folder,
            outcome,
        });
    }
    Ok(report)
}

fn same(a: FileId, b: pctwin_gate::FileId) -> bool {
    a.volume == b.volume && a.index == b.index
}

/// The approved place `label`, if it is still the folder it was (`place`).
fn place<'t>(
    table: &'t Destinations,
    label: &str,
    place: Option<FileId>,
) -> Result<&'t Destination, String> {
    let dest = table.get(label).map_err(|_| NOT_REACHABLE.to_string())?;
    if let Some(place) = place {
        let now = dest
            .folder_identity("")
            .map_err(|e| e.to_string())?
            .ok_or_else(|| NOT_REACHABLE.to_string())?;
        if !same(place, now) {
            return Err(NOT_SAME_PLACE.into());
        }
    }
    Ok(dest)
}

/// Undoes one committed file; records and returns how it ended.
fn undo_file(
    journal: &Journal,
    table: &Destinations,
    bin: &dyn Bin,
    entry: &Entry,
    earlier: Option<&Undo>,
) -> Result<UndoOutcome, JournalError> {
    let State::Committed {
        final_path,
        fingerprint,
        landed,
    } = &entry.state
    else {
        return Ok(UndoOutcome::NotDone {
            why: "it was not finished".into(),
        });
    };
    let done = |outcome: UndoOutcome| -> Result<UndoOutcome, JournalError> {
        journal.record_undo(
            entry.id,
            &Undo::Done {
                outcome: outcome.clone(),
            },
        )?;
        Ok(outcome)
    };
    let dest = match place(table, &entry.write.destination, entry.write.place) {
        Ok(dest) => dest,
        // Not recorded: tried again next time.
        Err(why) => return Ok(UndoOutcome::NotDone { why }),
    };
    let now = match dest.stat(final_path) {
        Ok(Some(now)) => now,
        Ok(None) => return done(UndoOutcome::AlreadyGone),
        Err(e) => return Ok(UndoOutcome::NotDone { why: e.to_string() }),
    };
    // Which file must be at the name: the one recorded as about to go (after a crash), else the
    // one that landed.
    let expected = match earlier {
        Some(Undo::Moving { file }) => *file,
        _ => landed.file,
    };
    if expected.is_some_and(|id| !same(id, now.id)) {
        // Another file has the name now (edited by saving a new copy, or made again): kept.
        return done(UndoOutcome::Kept {
            why: CHANGED_SINCE.into(),
        });
    }
    if now.len != landed.size || now.modified.map(crate::nanos) != landed.modified_ns {
        return done(UndoOutcome::Kept {
            why: CHANGED_SINCE.into(),
        });
    }
    let unchanged = dest
        .open_read(final_path)
        .and_then(|mut f| fingerprint_reader(&mut f, landed.size, entry.write.block_size));
    match unchanged {
        Ok(Some(found)) if &found == fingerprint => {}
        Ok(_) => {
            return done(UndoOutcome::Kept {
                why: CHANGED_SINCE.into(),
            });
        }
        Err(e) => return Ok(UndoOutcome::NotDone { why: e.to_string() }),
    }
    let path = match dest.ambient_path(final_path, now.id) {
        Ok(path) => path,
        Err(e) => return Ok(UndoOutcome::NotDone { why: e.to_string() }),
    };
    if let Err(why) = bin.can_take(&path) {
        return done(UndoOutcome::Kept { why });
    }
    journal.record_undo(
        entry.id,
        &Undo::Moving {
            file: Some(FileId {
                volume: now.id.volume,
                index: now.id.index,
            }),
        },
    )?;
    match bin.put(&path) {
        Ok(()) => done(UndoOutcome::Trashed),
        Err(why) => done(UndoOutcome::NotDone { why }),
    }
}

/// The plain reason for a drive whose Trash cannot be trusted to keep a file.
pub const NO_RECYCLE_BIN: &str = "this drive has no Recycle Bin, so it was kept";

/// Whether the Recycle Bin of the drive `path` is on can be trusted to keep a file, among `drives`
/// (as the scan lists them). Windows deletes outright, without a Recycle Bin, on removable drives,
/// FAT and exFAT drives and network drives, so only a fixed NTFS or ReFS drive counts.
pub fn recycle_bin_for(drives: &[pctwin_scan::Drive], path: &Path) -> Result<(), String> {
    use pctwin_scan::FileSystem;
    let text = path.to_string_lossy();
    // A verbatim path (`\\?\C:\...`, as canonical paths are on Windows) names the same drive.
    let text = text.strip_prefix("\\\\?\\").unwrap_or(&text).to_lowercase();
    let drive = drives
        .iter()
        .filter(|d| {
            let mount = d.mount.to_string_lossy().to_lowercase();
            !mount.is_empty() && text.starts_with(&mount)
        })
        .max_by_key(|d| d.mount.as_os_str().len());
    match drive {
        Some(d)
            if !d.removable
                && !d.read_only
                && matches!(d.file_system, FileSystem::Ntfs | FileSystem::ReFs) =>
        {
            Ok(())
        }
        _ => Err(NO_RECYCLE_BIN.into()),
    }
}

/// The system Trash (Recycle Bin on Windows) through the `trash` crate. It never deletes: what it
/// cannot move stays where it is. On Windows a file is offered only on a drive whose Recycle Bin
/// can be trusted ([`recycle_bin_for`]); on macOS Finder is not asked (it would need the person's
/// permission to control Finder), the system's file manager is.
pub struct SystemBin {
    drives: Vec<pctwin_scan::Drive>,
}

impl SystemBin {
    /// Looks at the laptop's drives once.
    pub fn new() -> Self {
        Self {
            drives: pctwin_scan::list_drives(),
        }
    }
}

impl Default for SystemBin {
    fn default() -> Self {
        Self::new()
    }
}

impl Bin for SystemBin {
    fn can_take(&self, path: &Path) -> Result<(), String> {
        if cfg!(windows) {
            recycle_bin_for(&self.drives, path)
        } else {
            Ok(())
        }
    }

    fn put(&self, path: &Path) -> Result<(), String> {
        #[cfg(target_os = "macos")]
        let result = {
            use trash::macos::{DeleteMethod, TrashContextExtMacos};
            let mut context = trash::TrashContext::default();
            context.set_delete_method(DeleteMethod::NsFileManager);
            context.delete(path)
        };
        #[cfg(not(target_os = "macos"))]
        let result = trash::delete(path);
        result.map_err(|e| format!("it could not be moved to the Trash ({e})"))
    }
}
