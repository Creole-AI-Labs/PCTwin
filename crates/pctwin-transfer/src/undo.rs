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

#[cfg(windows)]
use pctwin_gate::Left;
use pctwin_gate::{Destination, Destinations, Removed};
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
/// Why a copy an app keeps open (a database or mail store, or one with an app's working files
/// beside it) was kept.
pub const APP_KEEPS_OPEN: &str = "an app keeps this file open, so it was kept";
/// Why a copy was not removed where the system cannot say whether another program has it open.
pub const CANNOT_CHECK: &str =
    "PCTwin can't tell if another program is using this. Close your other programs, then remove";
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
/// just before, and are used up by this one pass (taken by value): a second pass asks again.
/// Only answers for this journal, and only while fresh, count
/// ([`crate::MAX_CONFIRMED_AGE`]). If it could not be asked, pass [`Confirmed::none`], and
/// nothing is removed. Once undo is closed (even part of the way), it
/// stops and says so ([`UndoReport::closed`]), with what it did before.
pub fn undo(
    journal: &Journal,
    table: &Destinations,
    originals: Confirmed,
) -> Result<UndoReport, JournalError> {
    undo_with(journal, table, originals, &UndoOptions::default())
}

/// [`undo`], with the person's answers and words ([`UndoOptions`]).
pub fn undo_with(
    journal: &Journal,
    table: &Destinations,
    originals: Confirmed,
    options: &UndoOptions<'_>,
) -> Result<UndoReport, JournalError> {
    let mut report = UndoReport::default();
    // Closing finishes every part-way undo itself, so after the close there may be nothing left
    // whose permit would be refused: the gate is read first.
    if journal.undo_gate()? == pctwin_journal::UndoGate::Closed {
        report.closed = true;
        return Ok(report);
    }
    match undo_all(journal, table, &originals, options, &mut report) {
        Err(JournalError::UndoClosed) => {
            report.closed = true;
            Ok(report)
        }
        Err(e) => Err(e),
        Ok(()) => Ok(report),
    }
}

/// What the person's app passes in for one undo.
#[derive(Debug, Clone, Copy)]
pub struct UndoOptions<'a> {
    /// The person confirmed, once for this undo, that their other programs are closed. Used only
    /// where the system cannot say whether another program has a file open
    /// ([`CANNOT_CHECK`]); never instead of asking it.
    pub others_closed: bool,
    /// The words added to the name of a file kept beside its own, in the person's language, from
    /// the reviewed translations ([`KEPT_WORDS`] in English).
    pub kept_words: &'a str,
    /// The byte length of the longest of those words in any language, so a name made with them
    /// always fits whatever the language.
    pub room_for_words: usize,
    /// How long finishing one removal left part-way may take ([`RESOLVE_WITHIN`]): a drive that
    /// stops answering leaves that file for another try ([`TOOK_TOO_LONG`]) instead of holding
    /// undo, or its close, for good.
    pub resolve_within: std::time::Duration,
}

impl Default for UndoOptions<'_> {
    fn default() -> Self {
        Self {
            others_closed: false,
            kept_words: KEPT_WORDS,
            room_for_words: KEPT_WORDS_ROOM,
            resolve_within: RESOLVE_WITHIN,
        }
    }
}

/// How long finishing one removal left part-way may take, by default.
pub const RESOLVE_WITHIN: std::time::Duration = std::time::Duration::from_secs(120);

/// The English words added to the name of a file kept beside its own ("Report (kept by PCTwin
/// undo).docx").
pub const KEPT_WORDS: &str = " (kept by PCTwin undo)";
/// Room kept for those words in any language, in bytes.
pub const KEPT_WORDS_ROOM: usize = 64;

fn undo_all(
    journal: &Journal,
    table: &Destinations,
    originals: &Confirmed,
    options: &UndoOptions<'_>,
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
        let outcome = undo_file(journal, table, originals, options, &entry)?;
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
            report.files[at].outcome = undo_file(journal, table, originals, options, &entry)?;
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
    options: &UndoOptions<'_>,
    entry: &Entry,
) -> Result<UndoOutcome, JournalError> {
    let State::Committed { final_path, .. } = &entry.state else {
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
    // Not recorded: tried again next time.
    let not_done = |why: String| Ok(UndoOutcome::NotDone { why });
    let dest = match place(table, &entry.write.destination, entry.write.place) {
        Ok(dest) => dest,
        Err(why) => return not_done(why),
    };
    // Which file it is: the one that landed (never removed without knowing).
    let Some(file) = landed_file(entry) else {
        return done(&permit, kept(CANNOT_TELL));
    };
    // A removal was under way when PCTwin stopped: what is on the disk decides first, so a copy
    // that is already gone is never reported as kept, and nothing is left under a hidden name.
    if let Some(stage) = journal.undo_of(entry.id)?.filter(|u| !u.is_done()) {
        match finish_part_way(journal, &permit, dest, entry, &stage, options)? {
            PartWay::Ended(outcome) => return done(&permit, outcome),
            PartWay::BackForNow(why) => return not_done(why.into()),
            PartWay::Unresolved(why) => return not_done(why),
            // A put-back finished with the copy itself under its name: it starts over, as if
            // never tried (never "changed since" for having been put back).
            PartWay::StillThere if matches!(stage, Undo::Putting { .. }) => {
                journal.record_undo(
                    &permit,
                    entry.id,
                    &Undo::Done {
                        outcome: UndoOutcome::NotDone {
                            why: BACK_UNDER_ITS_NAME.into(),
                        },
                    },
                )?;
            }
            PartWay::StillThere => {}
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
    match remove_copy(journal, &permit, dest, entry, final_path, file, options)? {
        Ok(outcome) => done(&permit, outcome),
        Err(why) => not_done(why),
    }
}

fn kept(why: &str) -> UndoOutcome {
    UndoOutcome::Kept { why: why.into() }
}

/// The identity of the file that landed for `entry`, if recorded.
fn landed_file(entry: &Entry) -> Option<FileId> {
    match &entry.state {
        State::Committed { landed, .. } => landed.file,
        _ => None,
    }
}

/// The check "the very file the move wrote, unchanged" for `entry`, through the held handle.
fn check_for(entry: &Entry) -> impl FnOnce(&mut std::fs::File) -> std::io::Result<bool> + '_ {
    move |f| match &entry.state {
        State::Committed {
            fingerprint,
            landed,
            ..
        } => unchanged(f, landed, fingerprint, entry.write.block_size),
        _ => Ok(false),
    }
}

/// How a copy that was looked at for removal ended, or why it was not done this time.
fn outcome_of(removed: Removed) -> Result<UndoOutcome, String> {
    Ok(match removed {
        Removed::Removed => UndoOutcome::Deleted,
        Removed::Gone => kept(MOVED_SINCE),
        Removed::NotThatFile | Removed::Changed => kept(CHANGED_SINCE),
        Removed::Linked => kept(SECOND_NAME),
        Removed::CloudOnly => kept(ONLINE_ONLY),
        Removed::Unsupported => kept(NO_SAFE_REMOVAL),
        Removed::AppKeepsOpen => kept(APP_KEEPS_OPEN),
        Removed::KeptBeside { at } => UndoOutcome::KeptAt {
            at,
            why: KEPT_BESIDE.into(),
        },
        Removed::Salvaged { at } => UndoOutcome::KeptAt {
            at,
            why: SAVED_AGAIN.into(),
        },
        Removed::InUse => return Err(IN_USE.into()),
        Removed::CannotCheck => return Err(CANNOT_CHECK.into()),
    })
}

/// Why a copy whose name was taken while undo had moved it aside was kept, under a new name.
pub const KEPT_BESIDE: &str =
    "it changed during undo and its name was taken meanwhile, so it was kept under a new name";
/// Why a copy written to while undo removed it was saved again, under a new name.
pub const SAVED_AGAIN: &str =
    "a program changed it while undo removed it, so it was saved again under a new name";
/// Why a copy undo was saving again when PCTwin stopped needs a look.
pub const SAVE_CUT_SHORT: &str = "undo stopped while saving changes made to this file during undo; if you changed it then, check it";
/// Why a copy saved again under a new name needs a look: undo stopped while saving it, so it may
/// not hold everything.
pub const SAVED_MAYBE_INCOMPLETE: &str = "a program changed it while undo removed it, and undo stopped while saving it again under a new name, so the saved copy may be incomplete; check it";
/// Why a copy was not undone this time: its drive did not answer in time.
pub const TOOK_TOO_LONG: &str = "its drive did not answer in time, so it was left as it is for now; check the drive is connected and working, then undo again";
/// Internal: a copy put back under its own name, to be looked at again from the start.
const BACK_UNDER_ITS_NAME: &str = "it is back under its own name, to be looked at again";
/// Why a copy was not undone: its folder is not on the drive right now.
pub const FOLDER_MISSING: &str =
    "its folder cannot be found on this drive right now (is the drive connected?)";
/// Why a copy undo had not finished with when it closed was kept.
pub const KEPT_AT_CLOSE: &str = "undo was closed before this copy could be removed, so it was kept";

/// Windows: removes the checked copy through the handle it was checked on, recorded just before.
#[cfg(windows)]
fn remove_copy(
    journal: &Journal,
    permit: &UndoPermit<'_>,
    dest: &Destination,
    entry: &Entry,
    final_path: &str,
    file: FileId,
    _options: &UndoOptions<'_>,
) -> Result<Result<UndoOutcome, String>, JournalError> {
    // Recorded only when the removal really happens, just before it (a journal that cannot be
    // written stops it, with nothing removed).
    let failed_record = std::cell::Cell::new(None);
    let removed = dest.remove_if_unchanged(permit, final_path, file, check_for(entry), || {
        let folder = final_path.rsplit_once('/').map_or("", |(f, _)| f);
        let dir_id = dest.folder_identity(folder)?.ok_or_else(|| {
            std::io::Error::new(std::io::ErrorKind::NotFound, "its folder is not there")
        })?;
        // Removal by handle leaves nothing under another name; the name only fills the record.
        let removing = Undo::Removing {
            file,
            dir_id,
            private: format!(".pctwin-undo-{:032x}", file.index.get()),
        };
        journal
            .record_undo(permit, entry.id, &removing)
            .map_err(|e| {
                let message = e.to_string();
                failed_record.set(Some(e));
                std::io::Error::other(message)
            })
    });
    if let Some(e) = failed_record.take() {
        return Err(e);
    }
    Ok(removed.map_err(|e| e.to_string()).and_then(outcome_of))
}

/// Linux and macOS: removes the checked copy by moving its name aside first and checking it
/// there, each step recorded before it happens, and its folder flushed before it is recorded as
/// done (Security Design 3B, decided 9 October 2026).
#[cfg(unix)]
fn remove_copy(
    journal: &Journal,
    permit: &UndoPermit<'_>,
    dest: &Destination,
    entry: &Entry,
    final_path: &str,
    file: FileId,
    options: &UndoOptions<'_>,
) -> Result<Result<UndoOutcome, String>, JournalError> {
    use pctwin_gate::Check;
    let copy = match dest.check_copy(final_path, file, check_for(entry)) {
        Ok(Check::Ready(copy)) => copy,
        Ok(Check::Done(r)) => return Ok(outcome_of(r)),
        Err(e) => return Ok(Err(e.to_string())),
    };
    let dir_id = copy.dir_id();
    let private = match pctwin_gate::private_name() {
        Ok(p) => p,
        Err(e) => return Ok(Err(e.to_string())),
    };
    // Recorded first: after a crash, undo knows the one hidden name the copy may be under.
    journal.record_undo(
        permit,
        entry.id,
        &Undo::Removing {
            file,
            dir_id,
            private: private.clone(),
        },
    )?;
    let failed_record = std::cell::Cell::new(None);
    let mut record = |step: pctwin_gate::Step<'_>| {
        journal
            .record_undo(permit, entry.id, &stage_of(file, dir_id, step))
            .map_err(|e| {
                let message = e.to_string();
                failed_record.set(Some(e));
                std::io::Error::other(message)
            })
    };
    let mut cx = pctwin_gate::Context {
        kept_words: options.kept_words,
        room_for_words: options.room_for_words,
        others_closed: options.others_closed,
        journal: &mut record,
    };
    // The gate flushes the folder it acted in before it answers, so the outcome is on the disk
    // before the journal says it is done.
    let removed = dest.remove_checked(permit, *copy, &private, &mut cx);
    if let Some(e) = failed_record.take() {
        return Err(e);
    }
    match removed {
        Ok(r) => Ok(outcome_of(r)),
        // Not recorded as done: the next undo finishes it from the disk.
        Err(e) => Ok(Err(e.to_string())),
    }
}

/// The journal's record of one step of a removal on Linux and macOS, taken before it happens.
#[cfg(unix)]
fn stage_of(file: FileId, dir_id: FileId, step: pctwin_gate::Step<'_>) -> Undo {
    use pctwin_gate::Step;
    match step {
        Step::NewPrivate { private } => Undo::Removing {
            file,
            dir_id,
            private: private.into(),
        },
        Step::Putting { private, to } => Undo::Putting {
            file,
            dir_id,
            private: private.into(),
            to: to.into(),
        },
        Step::Salvaging { temp, to, complete } => Undo::Salvaging {
            file,
            dir_id,
            temp: temp.into(),
            to: to.into(),
            complete,
        },
    }
}

/// How finishing a removal left part-way ended.
enum PartWay {
    /// Finished: how it ended.
    Ended(UndoOutcome),
    /// Nothing was moved, or it was put back: the copy itself is under its name, to be looked at
    /// as usual.
    StillThere,
    /// Back under its own name for now, for another try (why).
    #[cfg_attr(
        windows,
        expect(
            dead_code,
            reason = "only moving aside, on Linux and macOS, puts a copy back"
        )
    )]
    BackForNow(&'static str),
    /// Not finished: something may still be under a hidden name (why).
    Unresolved(String),
}

/// Finishes, from what is on the disk, a removal the journal recorded part of the way (`stage`),
/// with an open undo permit or the ticket undo's close lends. Safe to run again.
///
/// The drive's part runs on a worker thread with a deadline (`options.resolve_within`; Security
/// Design 3B: "each resolution runs in a worker with a deadline"), because a drive that stops
/// answering blocks whatever asks it, for good. The worker gets a lent right and asks this thread
/// to journal each of its steps; if it does not finish in time, this file is
/// [`PartWay::Unresolved`] ([`TOOK_TOO_LONG`]), the lent right ends (the gate then refuses every
/// rename and removal it would still try), and the file stays busy for any other try until the
/// worker has ended.
fn finish_part_way(
    journal: &Journal,
    right: &impl pctwin_journal::UndoRight,
    dest: &Destination,
    entry: &Entry,
    stage: &Undo,
    options: &UndoOptions<'_>,
) -> Result<PartWay, JournalError> {
    let Some(key) = hidden_name(stage) else {
        return Ok(PartWay::StillThere);
    };
    let dest = match dest.try_clone() {
        Ok(dest) => dest,
        Err(e) => return Ok(PartWay::Unresolved(e.to_string())),
    };
    let (entry, stage) = (entry.clone(), stage.clone());
    let (others_closed, kept_words, room_for_words) = (
        options.others_closed,
        options.kept_words.to_string(),
        options.room_for_words,
    );
    let ended = within(
        journal,
        right,
        entry.id,
        key,
        options.resolve_within,
        move |lent, record| {
            let options = UndoOptions {
                others_closed,
                kept_words: &kept_words,
                room_for_words,
                resolve_within: RESOLVE_WITHIN,
            };
            on_the_disk(record, lent, &dest, &entry, &stage, &options)
        },
    )?;
    Ok(ended.unwrap_or_else(|| PartWay::Unresolved(TOOK_TOO_LONG.into())))
}

/// The hidden name a stage before Done is about (for telling one part-way removal from another).
fn hidden_name(stage: &Undo) -> Option<&str> {
    match stage {
        Undo::Removing { private, .. } | Undo::Putting { private, .. } => Some(private),
        Undo::Salvaging { temp, .. } => Some(temp),
        Undo::Done { .. } => None,
    }
}

/// The hidden names whose part-way removal a worker is finishing right now (one worker each).
static BUSY: std::sync::Mutex<Vec<String>> = std::sync::Mutex::new(Vec::new());

/// Marks a hidden name busy for as long as it lives (moved into the worker).
struct Busy(String);

impl Busy {
    fn claim(key: &str) -> Option<Busy> {
        let mut busy = BUSY
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner);
        if busy.iter().any(|k| k == key) {
            return None;
        }
        busy.push(key.to_string());
        Some(Busy(key.to_string()))
    }
}

impl Drop for Busy {
    fn drop(&mut self) {
        let mut busy = BUSY
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner);
        busy.retain(|k| *k != self.0);
    }
}

/// What a worker asks of the thread that lent it its right.
enum Ask<T> {
    /// Journal this step of entry's undo, and answer.
    Record(Undo, std::sync::mpsc::Sender<Result<(), JournalError>>),
    /// The work ended.
    Ended(Result<T, JournalError>),
}

/// Runs `work` on a worker thread, within `wait`, for entry `id`, about the hidden name `key`:
/// `work` gets a right lent from `right` and a way to journal its steps (journaled by this
/// thread, with `right`, before the worker goes on). `Ok(None)` if it did not end in time, or if
/// another worker is still busy with `key`: then the lent right has ended (the gate refuses
/// whatever the worker would still do), and nothing it asks to journal is journaled any more.
fn within<T: Send + 'static>(
    journal: &Journal,
    right: &impl pctwin_journal::UndoRight,
    id: u64,
    key: &str,
    wait: std::time::Duration,
    work: impl FnOnce(
        &pctwin_journal::LentRight,
        &dyn Fn(&Undo) -> Result<(), JournalError>,
    ) -> Result<T, JournalError>
    + Send
    + 'static,
) -> Result<Option<T>, JournalError> {
    use std::sync::mpsc;
    let Some(busy) = Busy::claim(key) else {
        return Ok(None);
    };
    let deadline = std::time::Instant::now() + wait;
    let (lending, lent) = right.lend();
    let (asks, asked) = mpsc::channel::<Ask<T>>();
    std::thread::Builder::new()
        .name("pctwin-undo-finish".into())
        .spawn(move || {
            let _busy = busy;
            let ask = asks.clone();
            let record = move |stage: &Undo| -> Result<(), JournalError> {
                let gone = || JournalError::Storage("undo stopped here: it took too long".into());
                let (answer, answered) = mpsc::channel();
                ask.send(Ask::Record(stage.clone(), answer))
                    .map_err(|_| gone())?;
                answered.recv().map_err(|_| gone())?
            };
            let ended = work(&lent, &record);
            let _ = asks.send(Ask::Ended(ended));
        })
        .map_err(|e| JournalError::Storage(e.to_string()))?;
    loop {
        let left = deadline.saturating_duration_since(std::time::Instant::now());
        match asked.recv_timeout(left) {
            Ok(Ask::Record(stage, answer)) => {
                let _ = answer.send(journal.record_undo(right, id, &stage));
            }
            Ok(Ask::Ended(ended)) => return ended.map(Some),
            Err(_) => {
                // Ended here, before anything else: from now the worker can act on nothing.
                drop(lending);
                return Ok(None);
            }
        }
    }
}

/// The drive's part of [`finish_part_way`], on the worker: each step journaled through `record`
/// before it happens, every rename and removal under the lent `right`.
fn on_the_disk(
    record: &dyn Fn(&Undo) -> Result<(), JournalError>,
    right: &pctwin_journal::LentRight,
    dest: &Destination,
    entry: &Entry,
    stage: &Undo,
    options: &UndoOptions<'_>,
) -> Result<PartWay, JournalError> {
    let State::Committed { final_path, .. } = &entry.state else {
        return Ok(PartWay::Unresolved("it was not finished".into()));
    };
    #[cfg(windows)]
    {
        let _ = (record, options);
        // Removal by handle is all or nothing: the copy is under its name or it is gone.
        let Undo::Removing { file, .. } = stage else {
            return Ok(PartWay::Unresolved(
                "an undo step Windows never takes".into(),
            ));
        };
        Ok(match dest.resume_removal(right, final_path, *file) {
            Ok(Left::Here) => PartWay::StillThere,
            Ok(Left::Gone) => PartWay::Ended(UndoOutcome::AlreadyGone),
            Err(e) => PartWay::Unresolved(e.to_string()),
        })
    }
    #[cfg(unix)]
    {
        use pctwin_gate::Resolution;
        let (Some(file), Some(dir_id)) = (stage_file(stage), stage.dir_id()) else {
            return Ok(PartWay::StillThere);
        };
        let failed_record = std::cell::Cell::new(None);
        let mut journal = |step: pctwin_gate::Step<'_>| {
            record(&stage_of(file, dir_id, step)).map_err(|e| {
                let message = e.to_string();
                failed_record.set(Some(e));
                std::io::Error::other(message)
            })
        };
        let mut cx = pctwin_gate::Context {
            kept_words: options.kept_words,
            room_for_words: options.room_for_words,
            others_closed: options.others_closed,
            journal: &mut journal,
        };
        let resolved = match stage {
            Undo::Removing { private, .. } => dest.resolve_removing(
                right,
                final_path,
                file,
                dir_id,
                private,
                check_for(entry),
                &mut cx,
            ),
            Undo::Putting { private, to, .. } => {
                dest.resolve_putting(right, final_path, file, dir_id, private, to, &mut cx)
            }
            Undo::Salvaging {
                temp, to, complete, ..
            } => dest.resolve_salvaging(right, final_path, dir_id, temp, to, *complete, &mut cx),
            Undo::Done { .. } => return Ok(PartWay::StillThere),
        };
        if let Some(e) = failed_record.take() {
            return Err(e);
        }
        let salvaging = matches!(stage, Undo::Salvaging { .. });
        let putting = matches!(stage, Undo::Putting { .. });
        let kept_at = |at: String, why: &str| {
            PartWay::Ended(UndoOutcome::KeptAt {
                at,
                why: why.into(),
            })
        };
        Ok(match resolved {
            Err(e) => PartWay::Unresolved(e.to_string()),
            Ok(Resolution::Removed) => PartWay::Ended(UndoOutcome::Deleted),
            // The copy was removed, but the bytes being saved again are not there.
            Ok(Resolution::AlreadyGone) if salvaging => PartWay::Ended(kept(SAVE_CUT_SHORT)),
            // What was being put back was moved on since.
            Ok(Resolution::AlreadyGone) if putting => PartWay::Ended(kept(MOVED_SINCE)),
            Ok(Resolution::AlreadyGone) => PartWay::Ended(UndoOutcome::AlreadyGone),
            Ok(Resolution::StillThere) => PartWay::StillThere,
            // Something that is not the copy, or the copy changed since.
            Ok(Resolution::Home) => PartWay::Ended(kept(CHANGED_SINCE)),
            Ok(Resolution::InUse) => PartWay::BackForNow(IN_USE),
            Ok(Resolution::CannotCheck) => PartWay::BackForNow(CANNOT_CHECK),
            Ok(Resolution::KeptAt { at }) => kept_at(at, KEPT_BESIDE),
            Ok(Resolution::SavedAgain { at }) => kept_at(at, SAVED_AGAIN),
            Ok(Resolution::SavedMaybeIncomplete { at }) => kept_at(at, SAVED_MAYBE_INCOMPLETE),
            Ok(Resolution::FolderMissing) => PartWay::Unresolved(FOLDER_MISSING.into()),
        })
    }
}

/// The file a stage before Done is about.
#[cfg(unix)]
fn stage_file(stage: &Undo) -> Option<FileId> {
    match stage {
        Undo::Removing { file, .. } | Undo::Putting { file, .. } | Undo::Salvaging { file, .. } => {
            Some(*file)
        }
        Undo::Done { .. } => None,
    }
}

/// Closes undo for good, as the wipe of the old laptop starts: first every removal left part-way
/// is finished from the disk (nothing is left under a hidden name), then the close is recorded
/// with them, all at once. A copy undo had not finished with stays, and is said to be kept. If
/// one cannot be finished (its drive is not connected, say), nothing is closed and each is listed
/// ([`pctwin_journal::CloseError::Unresolved`]): the person can connect it and try again, or
/// choose to leave it ([`Journal::offer_escape`]).
pub fn close_undo(
    journal: &Journal,
    table: &Destinations,
    options: &UndoOptions<'_>,
) -> Result<pctwin_journal::ClosedToken, pctwin_journal::CloseError> {
    use pctwin_journal::Resolved;
    journal.close_undo(&mut |ticket, pending| {
        let Some(entry) = journal.entry(pending.id).map_err(std::io::Error::other)? else {
            return Ok(Resolved::Unresolved {
                why: "the journal has no record of it".into(),
            });
        };
        let dest = match place(table, &entry.write.destination, entry.write.place) {
            Ok(dest) => dest,
            Err(why) => return Ok(Resolved::Unresolved { why }),
        };
        let part_way = finish_part_way(journal, ticket, dest, &entry, &pending.stage, options)
            .map_err(std::io::Error::other)?;
        Ok(match part_way {
            PartWay::Ended(UndoOutcome::Deleted | UndoOutcome::Removed) => Resolved::Removed,
            PartWay::Ended(UndoOutcome::AlreadyGone) => Resolved::AlreadyGone,
            PartWay::Ended(UndoOutcome::KeptAt { at, why }) => Resolved::KeptAt { at, why },
            PartWay::Ended(UndoOutcome::Kept { why } | UndoOutcome::NotDone { why }) => {
                Resolved::Kept { why }
            }
            PartWay::StillThere | PartWay::BackForNow(_) => Resolved::Kept {
                why: KEPT_AT_CLOSE.into(),
            },
            PartWay::Unresolved(why) => Resolved::Unresolved { why },
        })
    })
}

#[cfg(test)]
mod tests {
    //! The deadline on finishing a part-way removal (Part J, finding 8), shown with work that
    //! stands in for a drive that stops answering.
    use super::*;
    use pctwin_journal::{Actor, Landed, Permission, PlannedWrite, UndoRight};
    use std::sync::mpsc;
    use std::time::{Duration, Instant};

    fn journal() -> (tempfile::TempDir, Journal) {
        let dir = tempfile::tempdir().unwrap();
        let j = Journal::open(&dir.path().join("journal.redb")).unwrap();
        (dir, j)
    }

    fn file() -> FileId {
        FileId {
            volume: 3,
            index: std::num::NonZeroU64::new(5).unwrap(),
            born: None,
        }
    }

    fn committed(j: &Journal) -> u64 {
        let id = j
            .plan(&PlannedWrite {
                item: ItemId::from_hex(&"01".repeat(16)).unwrap(),
                source_laptop: pctwin_record::LaptopId::from_hex(&"02".repeat(16)).unwrap(),
                destination: "me".into(),
                path: "a.txt".into(),
                size: 1,
                actor: Actor {
                    acting_account: "1".into(),
                    for_account: "1".into(),
                    permission: Permission::OwnFolders,
                },
                block_size: crate::block_size_for(1),
                source_modified_ns: None,
                source_file: None,
                partial_keep: Default::default(),
                place: None,
            })
            .unwrap();
        j.staged(id, ".pctwin-t.part", &[]).unwrap();
        j.verified(id, [1; 32], None).unwrap();
        j.applied(id, "a.txt").unwrap();
        j.committed(
            id,
            Landed {
                size: 1,
                modified_ns: None,
                file: Some(file()),
            },
        )
        .unwrap();
        id
    }

    fn removing(n: u8) -> Undo {
        Undo::Removing {
            file: file(),
            dir_id: file(),
            private: format!(".pctwin-undo-{n:032x}"),
        }
    }

    /// Work that ends in time: each step it asks for is journaled (by this thread, with the real
    /// right) before it goes on, and its answer comes back.
    #[test]
    fn work_that_ends_in_time_has_its_steps_journaled_first() {
        let (_d, j) = journal();
        let a = committed(&j);
        let permit = j.begin_undo().unwrap();
        let r = within(
            &j,
            &permit,
            a,
            "key-in-time",
            Duration::from_secs(10),
            |lent, record| {
                assert!(lent.in_force());
                record(&removing(1))?;
                Ok(7)
            },
        )
        .unwrap();
        assert_eq!(r, Some(7));
        assert_eq!(j.undo_of(a).unwrap(), Some(removing(1)));
    }

    /// Work that does not end in time (a drive that stopped answering): the caller is back at
    /// the deadline, the file is not finished, and the late worker can act on nothing: its right
    /// has ended and nothing it asks to journal is journaled.
    #[test]
    fn work_that_takes_too_long_is_given_up_and_acts_on_nothing_after() {
        let (_d, j) = journal();
        let a = committed(&j);
        let permit = j.begin_undo().unwrap();
        let (late_tx, late_rx) = mpsc::channel();
        let start = Instant::now();
        let r = within(
            &j,
            &permit,
            a,
            "key-too-long",
            Duration::from_millis(100),
            move |lent, record| {
                std::thread::sleep(Duration::from_millis(600));
                let in_force = lent.in_force();
                let recorded = record(&removing(2)).is_ok();
                let _ = late_tx.send((in_force, recorded));
                Ok(())
            },
        )
        .unwrap();
        assert_eq!(r, None);
        assert!(
            start.elapsed() < Duration::from_millis(500),
            "{:?}",
            start.elapsed()
        );
        let (in_force, recorded) = late_rx.recv_timeout(Duration::from_secs(5)).unwrap();
        assert!(!in_force, "a late worker's right has ended");
        assert!(!recorded, "a late worker's step is never journaled");
        assert_eq!(j.undo_of(a).unwrap(), None);
    }

    /// While a worker is still busy with a hidden name (given up on, but not ended), no second
    /// worker starts on it; once it has ended, the name can be finished again.
    #[test]
    fn a_hidden_name_is_never_finished_by_two_workers_at_once() {
        let (_d, j) = journal();
        let a = committed(&j);
        let permit = j.begin_undo().unwrap();
        let (go_tx, go_rx) = mpsc::channel::<()>();
        let (ended_tx, ended_rx) = mpsc::channel::<()>();
        let first = within(
            &j,
            &permit,
            a,
            "key-busy",
            Duration::from_millis(50),
            move |_, _| {
                let _ = go_rx.recv();
                let _ = ended_tx.send(());
                Ok(())
            },
        )
        .unwrap();
        assert_eq!(first, None);
        let ran = std::sync::Arc::new(std::sync::atomic::AtomicBool::new(false));
        let ran2 = std::sync::Arc::clone(&ran);
        let start = Instant::now();
        let second = within(
            &j,
            &permit,
            a,
            "key-busy",
            Duration::from_secs(10),
            move |_, _| {
                ran2.store(true, std::sync::atomic::Ordering::SeqCst);
                Ok(())
            },
        )
        .unwrap();
        assert_eq!(second, None);
        assert!(start.elapsed() < Duration::from_secs(1));
        assert!(!ran.load(std::sync::atomic::Ordering::SeqCst));
        go_tx.send(()).unwrap();
        ended_rx.recv_timeout(Duration::from_secs(5)).unwrap();
        // The busy mark goes as the worker's thread ends, just after it said so.
        let deadline = Instant::now() + Duration::from_secs(5);
        let third = loop {
            let r = within(
                &j,
                &permit,
                a,
                "key-busy",
                Duration::from_secs(10),
                |_, _| Ok(3),
            )
            .unwrap();
            if r.is_some() || Instant::now() > deadline {
                break r;
            }
            std::thread::sleep(Duration::from_millis(10));
        };
        assert_eq!(third, Some(3));
    }
}
