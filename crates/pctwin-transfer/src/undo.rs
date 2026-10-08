//! Undoing the files a move wrote (Task List 2.3, Product Spec: "Undo reverses PCTwin's changes,
//! keeps any edits you made since, and tells you about anything that can't be reversed").
//!
//! For each file the move committed, newest first:
//! - still exactly as it landed (the same file, the same size and modified time, and then the same
//!   fingerprint): moved aside by its handle into PCTwin's own folder in that place, checked again
//!   there (where nobody else knows to look, so an edit made in between is caught), and only then
//!   handed to the system Trash or Recycle Bin, never deleted, so it can be brought back;
//! - changed since (or a different file now has its name): kept, with "you changed this since the
//!   move, so it was kept"; an edit caught after moving aside puts the file back where it was;
//! - moved or renamed since: kept, and said so; already gone: nothing to do.
//!
//! Each step is recorded in the journal first (where the file is about to be moved aside), so undo
//! cut short by a crash carries on safely and never takes a file a person made later under the
//! same name. Then the folders the move made are removed, deepest first, only if they are empty and
//! are still the folders it made. Nothing that could not be undone is hidden: every one is
//! reported with why.
//!
//! PCTwin's own folder (its name, in the person's language, is given by the app) is where the
//! system Trash puts a file back if the person restores it, and where a file would wait on a drive
//! with no Recycle Bin, should that be chosen; today such a file is kept where it is.

use std::path::Path;

use pctwin_gate::{Destination, Destinations, Moved, Stat};
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
/// The plain reason for a file (or its folder) moved or renamed since the move.
pub const MOVED_SINCE: &str = "it was moved or renamed since the move, so it was kept";
const NOT_REACHABLE: &str = "the place it is in could not be reached";
const NOT_SAME_PLACE: &str = "the place it is in is not the same folder as during the move";
const NOT_EMPTY: &str = "something is still in it";
const NOT_ITS_FOLDER: &str = "it is not the folder PCTwin made, so it was kept";
const CANNOT_TELL: &str = "PCTwin cannot tell it is the one it made, so it was kept";
/// Most names tried for a file moved aside when other files keep taking the free one first.
const MAX_ASIDE_TRIES: u32 = 16;

/// Undoes every file the move in `journal` committed, then the folders it made. `aside` is the
/// name of PCTwin's own folder (in the person's language) made in each place for files on their
/// way to the Trash.
pub fn undo(
    journal: &Journal,
    table: &Destinations,
    bin: &dyn Bin,
    aside: &str,
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
        let outcome = undo_file(journal, table, bin, aside, entry, earlier.as_ref())?;
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
        let outcome = match (place(table, &made.destination, None), made.id) {
            (Err(why), _) => UndoOutcome::NotDone { why },
            // Never removed without knowing it is the very folder it made.
            (Ok(_), None) => UndoOutcome::Kept {
                why: CANNOT_TELL.into(),
            },
            (Ok(dest), Some(id)) => match dest.folder_identity(&made.folder) {
                Err(e) => UndoOutcome::NotDone { why: e.to_string() },
                Ok(None) => UndoOutcome::AlreadyGone,
                Ok(Some(now)) if !same(id, now) => UndoOutcome::Kept {
                    why: NOT_ITS_FOLDER.into(),
                },
                Ok(Some(_)) => match dest.remove_empty_folder(&made.folder) {
                    Ok(true) => UndoOutcome::Removed,
                    // Looked at again next time: what is in it may be undone by then.
                    Ok(false) => UndoOutcome::NotDone {
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

/// What a committed file is checked against.
struct Landing<'e> {
    final_path: &'e str,
    fingerprint: &'e [u8; 32],
    size: u64,
    modified_ns: Option<i64>,
    block_size: u64,
}

impl Landing<'_> {
    /// Whether the file at `stored` (as `now`) has exactly what landed: size, modified time, then
    /// fingerprint. `Err` if it could not be read.
    fn unchanged(&self, dest: &Destination, stored: &str, now: &Stat) -> Result<bool, String> {
        if now.len != self.size || now.modified.map(crate::nanos) != self.modified_ns {
            return Ok(false);
        }
        match dest
            .open_read(stored)
            .and_then(|mut f| fingerprint_reader(&mut f, self.size, self.block_size))
        {
            Ok(found) => Ok(found.as_ref() == Some(self.fingerprint)),
            Err(e) => Err(e.to_string()),
        }
    }
}

/// Undoes one committed file; records and returns how it ended.
fn undo_file(
    journal: &Journal,
    table: &Destinations,
    bin: &dyn Bin,
    aside: &str,
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
    let not_done = |why: String| Ok(UndoOutcome::NotDone { why });
    let dest = match place(table, &entry.write.destination, entry.write.place) {
        Ok(dest) => dest,
        // Not recorded: tried again next time.
        Err(why) => return not_done(why),
    };
    // Which file it is: the one that landed (never moved without knowing).
    let Some(expected) = landed.file else {
        return done(UndoOutcome::Kept {
            why: CANNOT_TELL.into(),
        });
    };
    let landing = Landing {
        final_path,
        fingerprint,
        size: landed.size,
        modified_ns: landed.modified_ns,
        block_size: entry.write.block_size,
    };
    // Cut short after moving it aside: carry on from there.
    if let Some(Undo::Aside { at, .. }) = earlier {
        match dest.stat(at) {
            Ok(Some(now)) => {
                return finish_aside(journal, dest, bin, aside, entry.id, &landing, at, &now);
            }
            Ok(None) => {}
            Err(e) => return not_done(e.to_string()),
        }
    }
    let now = match dest.stat(final_path) {
        Ok(Some(now)) => now,
        Ok(None) => return done(missing(dest, final_path, expected, landed.size)),
        Err(e) => return not_done(e.to_string()),
    };
    // Another file has the name now (saved by writing a new copy, or made again): kept.
    if !same(expected, now.id) {
        return done(UndoOutcome::Kept {
            why: CHANGED_SINCE.into(),
        });
    }
    match landing.unchanged(dest, final_path, &now) {
        Ok(true) => {}
        Ok(false) => {
            return done(UndoOutcome::Kept {
                why: CHANGED_SINCE.into(),
            });
        }
        Err(why) => return not_done(why),
    }
    // Whether this drive's Trash can take it at all, before anything moves. (A drive with no
    // Recycle Bin keeps the file where it is; moving it aside there instead would go here.)
    let path = match dest.ambient_path(final_path, now.id) {
        Ok(path) => path,
        Err(e) => return not_done(e.to_string()),
    };
    if let Err(why) = bin.can_take(&path) {
        return done(UndoOutcome::Kept { why });
    }
    // Moved aside by its handle into PCTwin's own folder, where it is recorded to go first.
    for _ in 0..MAX_ASIDE_TRIES {
        let at = match dest.free_name_at(&format!("{aside}/{final_path}")) {
            Ok(at) => at,
            Err(e) => return not_done(e.to_string()),
        };
        journal.record_undo(
            entry.id,
            &Undo::Aside {
                file: Some(expected),
                at: at.clone(),
            },
        )?;
        match dest.move_file(final_path, &at, now.id) {
            Ok(Moved::Moved) => {
                let Ok(Some(there)) = dest.stat(&at) else {
                    return not_done("it could not be found after moving it aside".into());
                };
                return finish_aside(journal, dest, bin, aside, entry.id, &landing, &at, &there);
            }
            Ok(Moved::Taken) => continue,
            // Swapped for another file since it was checked: that file is the person's.
            Ok(Moved::NotThatFile) => {
                return done(UndoOutcome::Kept {
                    why: CHANGED_SINCE.into(),
                });
            }
            Err(e) => return not_done(e.to_string()),
        }
    }
    not_done("PCTwin's own folder kept having its names taken".into())
}

/// The file is aside at `at` (as `now`): checked again there, then handed to the Trash; changed
/// in between (an edit through a program that had it open), or refused by the Trash, it goes back
/// where it was.
#[allow(clippy::too_many_arguments)]
fn finish_aside(
    journal: &Journal,
    dest: &Destination,
    bin: &dyn Bin,
    aside: &str,
    id: u64,
    landing: &Landing<'_>,
    at: &str,
    now: &Stat,
) -> Result<UndoOutcome, JournalError> {
    let done = |outcome: UndoOutcome| -> Result<UndoOutcome, JournalError> {
        journal.record_undo(
            id,
            &Undo::Done {
                outcome: outcome.clone(),
            },
        )?;
        Ok(outcome)
    };
    let back = |then: UndoOutcome| -> Result<UndoOutcome, JournalError> {
        match dest.move_file(at, landing.final_path, now.id) {
            Ok(Moved::Moved) => {
                tidy_aside(dest, aside, at);
                done(then)
            }
            // Its name was taken meanwhile: it stays in PCTwin's folder, and is said so.
            _ => done(UndoOutcome::Kept {
                why: format!("it was kept in {at}"),
            }),
        }
    };
    match landing.unchanged(dest, at, now) {
        Ok(true) => {}
        Ok(false) => {
            return back(UndoOutcome::Kept {
                why: CHANGED_SINCE.into(),
            });
        }
        Err(why) => return back(UndoOutcome::NotDone { why }),
    }
    let path = match dest.ambient_path(at, now.id) {
        Ok(path) => path,
        Err(e) => return back(UndoOutcome::NotDone { why: e.to_string() }),
    };
    match bin.put(&path) {
        Ok(()) => {
            tidy_aside(dest, aside, at);
            done(UndoOutcome::Trashed)
        }
        Err(why) => back(UndoOutcome::NotDone { why }),
    }
}

/// Removes the folders of PCTwin's own folder that `at` was in, from the deepest, while empty.
fn tidy_aside(dest: &Destination, aside: &str, at: &str) {
    let mut folder = at;
    while let Some((parent, _)) = folder.rsplit_once('/') {
        if !matches!(dest.remove_empty_folder(parent), Ok(true)) || parent == aside {
            break;
        }
        folder = parent;
    }
}

/// The file is not at its name: moved or renamed since (kept, and said so), or gone.
fn missing(dest: &Destination, final_path: &str, expected: FileId, len: u64) -> UndoOutcome {
    let folder = final_path.rsplit_once('/').map_or("", |(f, _)| f);
    let file = pctwin_gate::FileId {
        volume: expected.volume,
        index: expected.index,
    };
    match dest.folder_identity(folder) {
        // Its folder is not there: moved or renamed (or removed) with what is in it.
        Ok(None) => UndoOutcome::Kept {
            why: MOVED_SINCE.into(),
        },
        Ok(Some(_)) => match dest.find_in_folder(folder, file, len) {
            Ok(Some(_)) => UndoOutcome::Kept {
                why: MOVED_SINCE.into(),
            },
            Ok(None) => UndoOutcome::AlreadyGone,
            Err(e) => UndoOutcome::NotDone { why: e.to_string() },
        },
        Err(e) => UndoOutcome::NotDone { why: e.to_string() },
    }
}

/// The plain reason for a drive whose Trash cannot be trusted to keep a file.
pub const NO_RECYCLE_BIN: &str = "this drive has no Recycle Bin, so it was kept";

/// Whether the Recycle Bin of the drive `path` is on can be trusted to keep a file, among `drives`
/// (as the scan lists them). Windows deletes outright, without a Recycle Bin, on removable drives,
/// FAT and exFAT drives and network drives, so only a fixed NTFS or ReFS drive counts. The drive is
/// the one whose mount point holds the path, part by part (`C:\mnt\usb` is not `C:\mnt\usb2`).
pub fn recycle_bin_for(drives: &[pctwin_scan::Drive], path: &Path) -> Result<(), String> {
    use pctwin_scan::FileSystem;
    // A verbatim path (`\\?\C:\...`, as canonical paths are on Windows) names the same drive.
    let parts = |p: &Path| -> Vec<String> {
        let text = p.to_string_lossy();
        let text = text.strip_prefix("\\\\?\\").unwrap_or(&text).to_lowercase();
        text.split(['\\', '/'])
            .filter(|c| !c.is_empty())
            .map(str::to_string)
            .collect()
    };
    let file = parts(path);
    let drive = drives
        .iter()
        .filter(|d| {
            let mount = parts(&d.mount);
            !mount.is_empty() && file.len() > mount.len() && file.starts_with(&mount)
        })
        .max_by_key(|d| parts(&d.mount).len());
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

/// What this laptop's settings say about the Recycle Bin a file would go to.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct BinSettings {
    /// "Do not move deleted files to the Recycle Bin" is set for this person or this laptop.
    pub turned_off: bool,
    /// The drive's Recycle Bin is set to delete files at once.
    pub deletes_at_once: bool,
    /// The most the drive's Recycle Bin holds; a bigger file would be deleted at once.
    pub max_bytes: Option<u64>,
}

/// The plain reason for a Recycle Bin set to delete files instead of keeping them.
pub const BIN_TURNED_OFF: &str =
    "Windows is set to delete files instead of using the Recycle Bin, so it was kept";
/// The plain reason for a file too big for its drive's Recycle Bin.
pub const TOO_BIG_FOR_BIN: &str = "it is too big for this drive's Recycle Bin, so it was kept";

/// Whether, by these settings, a file of `size` bytes would be kept by the Recycle Bin (Windows
/// deletes it outright otherwise, even when asked to allow undo).
pub fn bin_keeps(settings: &BinSettings, size: u64) -> Result<(), String> {
    if settings.turned_off || settings.deletes_at_once {
        return Err(BIN_TURNED_OFF.into());
    }
    if settings.max_bytes.is_some_and(|max| size > max) {
        return Err(TOO_BIG_FOR_BIN.into());
    }
    Ok(())
}

/// Reads the Recycle Bin settings for the drive with volume number `volume` (Windows only): the
/// policy that turns it off, and the drive's own "delete at once" and size settings (kept per
/// drive under the drive's volume name, matched to the drive by its volume number).
#[cfg(windows)]
fn bin_settings(volume: u64) -> BinSettings {
    use winreg::RegKey;
    use winreg::enums::{HKEY_CURRENT_USER, HKEY_LOCAL_MACHINE};
    let on = |key: &RegKey, name: &str| key.get_value::<u32, _>(name).is_ok_and(|v| v != 0);
    let policy = r"Software\Microsoft\Windows\CurrentVersion\Policies\Explorer";
    let mut settings = BinSettings {
        turned_off: [HKEY_CURRENT_USER, HKEY_LOCAL_MACHINE].iter().any(|root| {
            RegKey::predef(*root)
                .open_subkey(policy)
                .is_ok_and(|k| on(&k, "NoRecycleFiles"))
        }),
        ..BinSettings::default()
    };
    let volumes = r"Software\Microsoft\Windows\CurrentVersion\Explorer\BitBucket\Volume";
    let Ok(all) = RegKey::predef(HKEY_CURRENT_USER).open_subkey(volumes) else {
        return settings;
    };
    for name in all.enum_keys().flatten() {
        let root = std::path::PathBuf::from(format!(r"\\?\Volume{name}\"));
        let same_drive = open_volume(&root)
            .and_then(|f| winapi_util::file::information(&f))
            .is_ok_and(|info| info.volume_serial_number() == volume);
        if !same_drive {
            continue;
        }
        if let Ok(key) = all.open_subkey(&name) {
            settings.deletes_at_once = on(&key, "NukeOnDelete");
            settings.max_bytes = key
                .get_value::<u32, _>("MaxCapacity")
                .ok()
                .map(|mb| u64::from(mb) * 1024 * 1024);
        }
        break;
    }
    settings
}

#[cfg(windows)]
fn open_volume(root: &Path) -> std::io::Result<std::fs::File> {
    use std::os::windows::fs::OpenOptionsExt;
    std::fs::OpenOptions::new()
        .read(true)
        .custom_flags(0x0200_0000)
        .open(root)
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
        #[cfg(windows)]
        {
            recycle_bin_for(&self.drives, path)?;
            let file = std::fs::File::open(path).map_err(|e| e.to_string())?;
            let size = file.metadata().map_err(|e| e.to_string())?.len();
            let volume = winapi_util::file::information(&file)
                .map_err(|e| e.to_string())?
                .volume_serial_number();
            bin_keeps(&bin_settings(volume), size)
        }
        #[cfg(not(windows))]
        {
            let _ = (&self.drives, path);
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
        result.map_err(|e| format!("it could not be moved to the Trash ({e})"))?;
        // Taken only if it really left its place.
        if path.symlink_metadata().is_ok() {
            return Err("the Trash did not take it".into());
        }
        Ok(())
    }
}
