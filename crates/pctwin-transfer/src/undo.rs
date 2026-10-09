//! Undoing the files a move wrote (Task List 2.3; Product Spec "Restore point and one-click undo";
//! Security Design B "Undo removes only the file it checked, through one handle", decided 8
//! October 2026).
//!
//! Undo is open only until the wipe of the old laptop starts; starting the wipe closes it for good
//! in the journal ([`Journal::close_undo`]), and after that undo refuses everything, even finishing
//! an undo a crash cut short. Until then every original is still on the old laptop, so undo removes
//! PCTwin's copies outright (no Recycle Bin), but only a copy:
//!
//! - whose original the old laptop confirms, read-only over the paired connection just before,
//!   it still has, unchanged ([`crate::check_originals`]); if it cannot be asked, nothing is
//!   removed ([`CONNECT_OLD_LAPTOP`]);
//! - that is, checked on the one handle it is removed through, the very file the move wrote, with
//!   one name, unchanged since (size, modified time, then the whole fingerprint read now)
//!   ([`pctwin_gate::Destination::remove_if_unchanged`]).
//!
//! A copy changed since, another file under its name, a file another program is using, a file
//! stored online only or a file with a second name is kept, with the reason. Each removal is
//! recorded in the journal before it happens, so undo cut short carries on safely. Then the
//! folders the move made are removed, deepest first, only if they are empty and are still the
//! folders it made. Nothing that could not be undone is hidden: every one is reported with why.

use pctwin_gate::{Destination, Destinations, Left, Removed};
use pctwin_journal::{
    Entry, FileId, Journal, JournalError, Landed, State, Undo, UndoOutcome, UndoPermit,
};
use pctwin_record::ItemId;

use crate::{Confirmed, OriginalNow, fingerprint_reader, original_unchanged};

/// What undo did with one thing.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Undone {
    /// The entry (for a file) or the entry that made it (for a folder).
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
    /// Undo is closed for good (the wipe of the old laptop started), so it stopped: what is
    /// listed was done before that, and nothing else was touched.
    pub closed: bool,
}

impl UndoReport {
    /// Everything that was not undone, with why.
    pub fn not_undone(&self) -> impl Iterator<Item = &Undone> {
        self.files.iter().chain(&self.folders).filter(|u| {
            // TODO(engineer): undo pipeline. KeptAt is needs-action too, so it is listed.
            matches!(
                u.outcome,
                UndoOutcome::Kept { .. } | UndoOutcome::KeptAt { .. } | UndoOutcome::NotDone { .. }
            )
        })
    }
}

/// The plain reason for a file changed since the move.
pub const CHANGED_SINCE: &str = "you changed this since the move, so it was kept";
/// The plain reason for a file (or its folder) moved or renamed since the move.
pub const MOVED_SINCE: &str = "it is no longer where the move put it (moved, renamed or removed since), so it was left as it is";
/// The plain reason when the old laptop could not confirm the original (not connected, or no
/// answer for it): nothing is removed.
pub const CONNECT_OLD_LAPTOP: &str = "Connect your old laptop to undo. Nothing was removed.";
/// The plain reason for a copy whose original on the old laptop has changed (final).
pub const ORIGINAL_CHANGED: &str =
    "the original on your old laptop has changed since the move, so this copy was kept";
/// The plain reason for a copy whose original the old laptop cannot find now (perhaps its drive
/// is not connected): kept for now, and asked again next time.
pub const ORIGINAL_NOT_FOUND: &str = "your old laptop cannot find the original right now (is its drive connected?), so this copy was kept for now; undo again once it is back";
/// The plain reason for a copy whose original PCTwin did not record when it was copied.
pub const ORIGINAL_UNKNOWN: &str = "PCTwin did not note which file the original was when it copied it, so it cannot check the original is still on your old laptop; this copy was kept";
/// The plain reason for a file another program is using.
pub const IN_USE: &str = "another program is using it, so it was kept; close it and undo again";
/// The plain reason for a file stored online only.
pub const ONLINE_ONLY: &str = "it is stored online only, so it was kept";
/// The plain reason for a file with a second name on the drive.
pub const SECOND_NAME: &str = "it has a second name on this drive, so it was kept";
const NO_SAFE_REMOVAL: &str =
    "this drive cannot remove it without risk to other files, so it was kept";
const NOT_REACHABLE: &str = "the place it is in could not be reached";
const NOT_SAME_PLACE: &str = "the place it is in is not the same folder as during the move";
const NOT_EMPTY: &str = "something is still in it";
const NOT_ITS_FOLDER: &str = "it is not the folder PCTwin made, so it was kept";
const CANNOT_TELL: &str = "PCTwin cannot tell it is the one it made, so it was kept";

/// The items whose originals undo needs the old laptop to confirm first: every committed file not
/// yet finished with. Ask with [`crate::check_originals`], then pass the answers to [`undo`].
pub fn undo_items(journal: &Journal) -> Result<Vec<ItemId>, JournalError> {
    let mut items = Vec::new();
    for entry in committed(journal)? {
        if !finished(journal, &entry)? {
            items.push(entry.write.item);
        }
    }
    Ok(items)
}

/// Undoes every file the move in `journal` committed, newest first, then the folders it made.
/// `originals` are the old laptop's answers for [`undo_items`], from [`crate::check_originals`]
/// just before: only answers for this journal, and only while fresh, count
/// ([`crate::MAX_CONFIRMED_AGE`]). If it could not be asked, pass [`Confirmed::none`], and
/// nothing is removed. Once undo is closed (even part of the way), it
/// stops and says so ([`UndoReport::closed`]), with what it did before.
pub fn undo(
    journal: &Journal,
    table: &Destinations,
    originals: &Confirmed,
) -> Result<UndoReport, JournalError> {
    let mut report = UndoReport::default();
    // TODO(engineer): undo pipeline. Closing now finishes every part-way undo itself, so after
    // the close there may be nothing left whose permit would be refused: read the gate first.
    if journal.undo_gate()? == pctwin_journal::UndoGate::Closed {
        report.closed = true;
        return Ok(report);
    }
    match undo_all(journal, table, originals, &mut report) {
        Err(JournalError::UndoClosed) => {
            report.closed = true;
            Ok(report)
        }
        Err(e) => Err(e),
        Ok(()) => Ok(report),
    }
}

fn undo_all(
    journal: &Journal,
    table: &Destinations,
    originals: &Confirmed,
    report: &mut UndoReport,
) -> Result<(), JournalError> {
    let mut in_use = Vec::new();
    for entry in committed(journal)? {
        if finished(journal, &entry)? {
            continue;
        }
        let State::Committed { final_path, .. } = &entry.state else {
            continue;
        };
        let outcome = undo_file(journal, table, originals, &entry)?;
        if outcome == (UndoOutcome::NotDone { why: IN_USE.into() }) {
            in_use.push(report.files.len());
        }
        report.files.push(Undone {
            entry: entry.id,
            destination: entry.write.destination.clone(),
            path: final_path.clone(),
            outcome,
        });
    }
    // Files that were in use get one more try, once everything else is done.
    for at in in_use {
        let id = report.files[at].entry;
        if let Some(entry) = journal.entry(id)? {
            report.files[at].outcome = undo_file(journal, table, originals, &entry)?;
        }
    }
    undo_folders(journal, table, report)
}

/// Committed files, newest first.
fn committed(journal: &Journal) -> Result<Vec<Entry>, JournalError> {
    let mut entries = journal.entries()?;
    entries.retain(|e| matches!(e.state, State::Committed { .. }));
    entries.reverse();
    Ok(entries)
}

/// Whether undo is finished with this file (only "not done this time" is tried again).
fn finished(journal: &Journal, entry: &Entry) -> Result<bool, JournalError> {
    Ok(matches!(
        journal.undo_of(entry.id)?,
        Some(Undo::Done { outcome }) if outcome.is_final()
    ))
}

fn undo_folders(
    journal: &Journal,
    table: &Destinations,
    report: &mut UndoReport,
) -> Result<(), JournalError> {
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
        // Held while the folder is looked at and removed, so undo cannot be closed in between.
        let permit = journal.begin_undo()?;
        let outcome = match (place(table, &made.destination, None), made.id) {
            (Err(why), _) => UndoOutcome::NotDone { why },
            // Never removed without knowing it is the very folder it made.
            (Ok(_), None) => UndoOutcome::Kept {
                why: CANNOT_TELL.into(),
            },
            (Ok(dest), Some(id)) => match dest.folder_identity(&made.folder) {
                Err(e) => UndoOutcome::NotDone { why: e.to_string() },
                Ok(None) => UndoOutcome::AlreadyGone,
                Ok(Some(now)) if id != now => UndoOutcome::Kept {
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
        journal.record_folder_undo(&permit, &made.destination, &made.folder, &outcome)?;
        report.folders.push(Undone {
            entry: made.entry,
            destination: made.destination,
            path: made.folder,
            outcome,
        });
    }
    Ok(())
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
        if place != now {
            return Err(NOT_SAME_PLACE.into());
        }
    }
    Ok(dest)
}

/// Whether the file held open is exactly what landed: size and modified time, then the whole
/// fingerprint, all read through the handle.
fn unchanged(
    file: &mut std::fs::File,
    landed: &Landed,
    fingerprint: &[u8; 32],
    block_size: u64,
) -> std::io::Result<bool> {
    let meta = file.metadata()?;
    if meta.len() != landed.size || meta.modified().ok().map(crate::nanos) != landed.modified_ns {
        return Ok(false);
    }
    Ok(fingerprint_reader(file, landed.size, block_size)?.as_ref() == Some(fingerprint))
}

/// Undoes one committed file; records and returns how it ended.
fn undo_file(
    journal: &Journal,
    table: &Destinations,
    originals: &Confirmed,
    entry: &Entry,
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
    // Held for this one file: undo cannot be closed while it is being undone, and nothing is
    // done once it is closed.
    let permit = journal.begin_undo()?;
    let done =
        |permit: &UndoPermit<'_>, outcome: UndoOutcome| -> Result<UndoOutcome, JournalError> {
            journal.record_undo(
                permit,
                entry.id,
                &Undo::Done {
                    outcome: outcome.clone(),
                },
            )?;
            Ok(outcome)
        };
    let kept = |why: &str| UndoOutcome::Kept { why: why.into() };
    // Not recorded: tried again next time.
    let not_done = |why: String| Ok(UndoOutcome::NotDone { why });
    let dest = match place(table, &entry.write.destination, entry.write.place) {
        Ok(dest) => dest,
        Err(why) => return not_done(why),
    };
    // Which file it is: the one that landed (never removed without knowing).
    let Some(file) = landed.file else {
        return done(&permit, kept(CANNOT_TELL));
    };
    let gate_file = file;
    // A removal was under way when PCTwin stopped: what is on the disk decides first, so a copy
    // that is already gone is never reported as kept.
    if matches!(journal.undo_of(entry.id)?, Some(Undo::Removing { .. })) {
        match dest.resume_removal(&permit, final_path, gate_file) {
            Ok(Left::Here) => {}
            Ok(Left::Gone) => return done(&permit, UndoOutcome::AlreadyGone),
            Ok(Left::Stranded { at }) => {
                return done(
                    &permit,
                    UndoOutcome::Kept {
                        why: format!("it was kept in {at}"),
                    },
                );
            }
            Err(e) => return not_done(e.to_string()),
        }
    }
    // The old laptop still has the original, unchanged, or nothing is removed. Only an original
    // that is there but different is final; one it cannot find now is asked about again.
    if entry.write.source_file.is_none() || entry.write.source_modified_ns.is_none() {
        return done(&permit, kept(ORIGINAL_UNKNOWN));
    }
    match originals.answer(journal, &entry.write.item) {
        None | Some(OriginalNow::CannotLook) => return not_done(CONNECT_OLD_LAPTOP.into()),
        Some(OriginalNow::Missing) => return not_done(ORIGINAL_NOT_FOUND.into()),
        Some(now) if !original_unchanged(&entry.write, now) => {
            return done(&permit, kept(ORIGINAL_CHANGED));
        }
        Some(_) => {}
    }
    // Recorded only when the removal really happens, just before it (a journal that cannot be
    // written stops it, with nothing removed).
    let failed_record = std::cell::Cell::new(None);
    let removed = dest.remove_if_unchanged(
        &permit,
        final_path,
        gate_file,
        |f| unchanged(f, landed, fingerprint, entry.write.block_size),
        || {
            // TODO(engineer): undo pipeline. Placeholder until the group commit and the
            // move-aside pipeline land: the folder's identity is read now (no removal without
            // it), and the private name is the gate's current one for this file.
            let folder = final_path.rsplit_once('/').map_or("", |(f, _)| f);
            let dir_id = dest.folder_identity(folder)?.ok_or_else(|| {
                std::io::Error::new(std::io::ErrorKind::NotFound, "its folder is not there")
            })?;
            let removing = Undo::Removing {
                file,
                dir_id,
                private: pctwin_gate::undo_name(file),
            };
            journal
                .record_undo(&permit, entry.id, &removing)
                .map_err(|e| {
                    let message = e.to_string();
                    failed_record.set(Some(e));
                    std::io::Error::other(message)
                })
        },
    );
    if let Some(e) = failed_record.take() {
        return Err(e);
    }
    let outcome = match removed {
        Ok(Removed::Removed) => UndoOutcome::Deleted,
        Ok(Removed::Gone) => kept(MOVED_SINCE),
        Ok(Removed::NotThatFile | Removed::Changed) => kept(CHANGED_SINCE),
        Ok(Removed::Linked) => kept(SECOND_NAME),
        Ok(Removed::CloudOnly) => kept(ONLINE_ONLY),
        Ok(Removed::Unsupported) => kept(NO_SAFE_REMOVAL),
        Ok(Removed::Stranded { at }) => UndoOutcome::Kept {
            why: format!("it was kept in {at}"),
        },
        Ok(Removed::InUse) => return not_done(IN_USE.into()),
        Err(e) => return not_done(e.to_string()),
    };
    done(&permit, outcome)
}
