//! Carrying out recovery after a restart (the plan): the journal decides
//! ([`pctwin_journal::recovery`]), this looks at the disk for it through the safety gate and does
//! what was decided, then cleans up the journal's own temporary files.
//!
//! Run it once at start, before anything is received. Running it again (or after a crash part of
//! the way through) is as safe as running it once: every step is recorded in the journal before or
//! as it happens, and each decision is made again from what the journal and the disk say.

use std::collections::HashSet;

use pctwin_gate::{
    Claim, Claimed, Destination, Destinations, FillFinish, FillRemoval, IncomingPath, temp_name,
};
use pctwin_journal::recovery::{self, Action, Look, Seen};
use pctwin_journal::{Born, Entry, FileId, Fill, Journal, JournalError, Landed, Proof, State};

use crate::fingerprint_reader;
use crate::naming::{Whole, commit_and_show, name_as, reads_back_whole, show_committed};

/// What recovery did.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct Recovered {
    /// Finished and proven: now committed.
    pub committed: Vec<u64>,
    /// Could not be finished, with why.
    pub failed: Vec<(u64, String)>,
    /// Partly copied files kept, to continue when the old laptop sends the rest.
    pub resumable: Vec<u64>,
    /// Left exactly as they were, because their place could not be looked at now.
    pub left: Vec<(u64, String)>,
    /// What the person should know about a write, in plain words, whatever became of it (a
    /// file PCTwin left at a name because it could not prove it its own, say).
    pub notes: Vec<(u64, String)>,
    /// Files filled in under their names, committed before a stop but not yet shown: now shown.
    pub shown: Vec<u64>,
    /// Temporary files removed.
    pub removed: u64,
}

/// Recovers every unfinished write in `journal`, writing only into the places in `table`.
pub fn recover(journal: &Journal, table: &Destinations) -> Result<Recovered, JournalError> {
    recover_with(journal, table, &|| {})
}

/// As [`recover`], with `after_planning` run once every decision is made and before any is
/// carried out (for tests, and fault injection, of what happens in between).
pub fn recover_with(
    journal: &Journal,
    table: &Destinations,
    after_planning: &dyn Fn(),
) -> Result<Recovered, JournalError> {
    let look = DiskLook { journal, table };
    let mut done = Recovered::default();
    // Copies committed while still hidden and tagged, then stopped before they were shown: shown
    // now (proven PCTwin's by their tag first), so none stays hidden.
    for (entry, show) in journal.unshown()? {
        let dest = match look.destination(&entry) {
            Ok(dest) => dest,
            Err(why) => {
                done.left.push((entry.id, why));
                continue;
            }
        };
        // After a restart: its bytes, read now, against the whole file's fingerprint.
        let State::Committed { fingerprint, .. } = &entry.state else {
            continue;
        };
        let whole = whole(&entry, fingerprint);
        let mut bytes = |f: &mut dyn std::io::Read| reads_back_whole(f, &whole);
        let check = pctwin_gate::ShowCheck::Bytes(&mut bytes);
        match show_committed(journal, dest, entry.id, &show, check)? {
            Some(note) => done.notes.push((entry.id, note)),
            None if journal.show_pending(entry.id)?.is_none() => done.shown.push(entry.id),
            None => done
                .left
                .push((entry.id, "the drive could not be reached to show it".into())),
        }
    }
    let unfinished = journal.unfinished()?;
    let decisions = recovery::plan(&unfinished, &look);
    after_planning();
    for decision in decisions {
        let Some(entry) = unfinished.iter().find(|e| e.id == decision.id) else {
            continue;
        };
        match decision.action {
            Action::Fail(why) => {
                journal.failed(entry.id, &why)?;
                done.failed.push((entry.id, why));
            }
            Action::Resume => done.resumable.push(entry.id),
            Action::Leave(why) => done.left.push((entry.id, why)),
            Action::Name | Action::Commit => {
                let finished = finish(journal, &look, entry, &decision.action, None)?;
                record_finish(journal, &mut done, entry.id, finished)?;
            }
            Action::NameNoting(note) => {
                // Kept for good in the journal, in the very step that goes back to the checked file.
                let finished = finish(journal, &look, entry, &Action::Name, Some(&note))?;
                done.notes.push((entry.id, note));
                record_finish(journal, &mut done, entry.id, finished)?;
            }
            Action::FinishFill => {
                let finished = finish_fill(journal, &look, entry)?;
                record_finish(journal, &mut done, entry.id, finished)?;
            }
            Action::RemoveFillThenName => {
                let finished = remove_fill_then_name(journal, &look, entry)?;
                record_finish(journal, &mut done, entry.id, finished)?;
            }
        }
    }
    let keep: HashSet<u64> = done
        .resumable
        .iter()
        .copied()
        .chain(done.left.iter().map(|(id, _)| *id))
        .collect();
    done.removed = sweep(journal, &look, &keep)?;
    Ok(done)
}

enum Finish {
    Committed,
    Failed(String),
    Left(String),
}

/// Records how finishing one write ended.
fn record_finish(
    journal: &Journal,
    done: &mut Recovered,
    id: u64,
    finished: Finish,
) -> Result<(), JournalError> {
    match finished {
        Finish::Committed => done.committed.push(id),
        Finish::Failed(why) => {
            journal.failed(id, &why)?;
            done.failed.push((id, why));
        }
        Finish::Left(why) => done.left.push((id, why)),
    }
    Ok(())
}

/// The checked temporary file of a write ready to be named: where it is, which file it was,
/// its whole fingerprint, and whether it is proven by its journal-given name and bytes alone
/// (on a drive whose identities change).
struct Checked<'e> {
    temp: &'e str,
    file: Option<FileId>,
    fingerprint: &'e [u8; 32],
    by_name: bool,
}

fn checked(entry: &Entry) -> Option<Checked<'_>> {
    match &entry.state {
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
        } => Some(Checked {
            temp,
            file: *file,
            fingerprint,
            by_name: false,
        }),
        State::Filling {
            temp,
            fingerprint,
            file,
            fill,
        }
        | State::Filled {
            temp,
            fingerprint,
            file,
            fill,
        } => Some(Checked {
            temp,
            file: *file,
            fingerprint,
            by_name: fill.proof == Proof::Token,
        }),
        _ => None,
    }
}

fn whole(entry: &Entry, fingerprint: &[u8; 32]) -> Whole {
    Whole {
        size: entry.write.size,
        block_size: entry.write.block_size,
        fingerprint: *fingerprint,
    }
}

/// Said when a filled-in file changed between recovery's look and its act.
const FILL_CHANGED_NOW: &str = "the copy was interrupted while it was being written under its name, and the file there changed while PCTwin was finishing it; it looks again next time";

/// Finishes a file filled in under its real name, proven PCTwin's and whole: durable, read back,
/// recorded filled, given its time and shown, then committed; the temporary file then goes.
fn finish_fill(
    journal: &Journal,
    look: &DiskLook<'_>,
    entry: &Entry,
) -> Result<Finish, JournalError> {
    let dest = match look.destination(entry) {
        Ok(dest) => dest,
        Err(why) => return Ok(Finish::Left(why)),
    };
    let (temp, fingerprint, fill, filled): (&str, &[u8; 32], &Fill, bool) = match &entry.state {
        State::Filling {
            temp,
            fingerprint,
            fill,
            ..
        } => (temp, fingerprint, fill, false),
        State::Filled {
            temp,
            fingerprint,
            fill,
            ..
        } => (temp, fingerprint, fill, true),
        _ => return Ok(Finish::Failed(NOT_NAMED.into())),
    };
    let whole = whole(entry, fingerprint);
    // The original's modified time: the checked temporary file carries it; else the record.
    let modified = dest
        .stat(temp)
        .ok()
        .flatten()
        .and_then(|s| s.modified)
        .or_else(|| entry.write.source_modified_ns.and_then(time_of));
    // The creation date of PCTwin's copy: the checked temporary file's (else now), so the tag
    // never stays on the person's file.
    let created = dest
        .open_read(temp)
        .ok()
        .and_then(|f| f.metadata().ok())
        .and_then(|m| m.created().ok());
    let finished = dest.finish_fill(
        &fill.final_path,
        fill,
        |f| reads_back_whole(f, &whole),
        || {
            if filled {
                Ok(())
            } else {
                journal
                    .filled(entry.id)
                    .map_err(|e| std::io::Error::other(e.to_string()))
            }
        },
    );
    match finished {
        Ok(FillFinish::Finished(stat)) => {
            // Committed while still hidden and tagged, then shown with its real dates.
            let show = pctwin_gate::Show {
                fill: fill.clone(),
                created: created.map(Born::of),
                modified_ns: modified.map(crate::nanos),
            };
            let landed = Landed {
                size: stat.len,
                modified_ns: show.modified_ns,
                file: Some(stat.id),
            };
            commit_and_show(journal, dest, entry.id, landed, &show)?;
            let _ = dest.remove_temp(temp);
            Ok(Finish::Committed)
        }
        Ok(FillFinish::Gone | FillFinish::NotProven) => Ok(Finish::Left(FILL_CHANGED_NOW.into())),
        Err(e) => Ok(Finish::Left(e.to_string())),
    }
}

/// Removes PCTwin's own partial copy at the real name (proven again first), then names the
/// checked file again.
fn remove_fill_then_name(
    journal: &Journal,
    look: &DiskLook<'_>,
    entry: &Entry,
) -> Result<Finish, JournalError> {
    let dest = match look.destination(entry) {
        Ok(dest) => dest,
        Err(why) => return Ok(Finish::Left(why)),
    };
    let State::Filling { temp, fill, .. } = &entry.state else {
        return Ok(Finish::Failed(NOT_NAMED.into()));
    };
    let len = match dest.look(&fill.final_path) {
        Ok(Some(len)) => Some(len),
        Ok(None) => None,
        Err(e) => return Ok(Finish::Left(e.to_string())),
    };
    if let Some(len) = len {
        match dest.remove_fill(&fill.final_path, fill, temp, len) {
            Ok(FillRemoval::Removed | FillRemoval::Gone) => {}
            Ok(FillRemoval::NotProven) => return Ok(Finish::Left(FILL_CHANGED_NOW.into())),
            Err(e) => return Ok(Finish::Left(e.to_string())),
        }
    }
    finish(journal, look, entry, &Action::Name, None)
}

/// A time from nanoseconds since 1970.
fn time_of(ns: i64) -> Option<std::time::SystemTime> {
    let ns = u64::try_from(ns).ok()?;
    Some(std::time::UNIX_EPOCH + std::time::Duration::from_nanos(ns))
}

const NOT_NAMED: &str =
    "the copy was interrupted as it finished, and PCTwin could not give it its name afterwards";

/// Names (if needed) and commits one entry recovery found complete.
fn finish(
    journal: &Journal,
    look: &DiskLook<'_>,
    entry: &Entry,
    action: &Action,
    left: Option<&str>,
) -> Result<Finish, JournalError> {
    let dest = match look.destination(entry) {
        Ok(dest) => dest,
        Err(why) => return Ok(Finish::Left(why)),
    };
    let Some(source) = checked(entry) else {
        return Ok(Finish::Failed(NOT_NAMED.into()));
    };
    // Proven again immediately before it is committed (not only when it was decided): the very
    // file it sealed, exactly its size and fingerprint, read now. A file filled in under its
    // name is a new file: the gate read it back whole just now, so its identity is what landed.
    let commit_if_proven = |stored: &str,
                            filled_in: bool,
                            show: Option<&pctwin_gate::Show>|
     -> Result<Finish, JournalError> {
        let sealed = if filled_in {
            match dest.stat(stored) {
                Ok(Some(stat)) => Some(stat.id),
                Ok(None) => None,
                Err(e) => return Ok(Finish::Left(e.to_string())),
            }
        } else {
            source.file
        };
        match prove(dest, entry, stored, sealed, source.fingerprint, false) {
            Proved::Proven(stat) => {
                match show {
                    // Filled in under its name: committed still hidden and tagged, then shown.
                    Some(show) => {
                        let landed = Landed {
                            size: stat.len,
                            modified_ns: show.modified_ns,
                            file: Some(stat.id),
                        };
                        commit_and_show(journal, dest, entry.id, landed, show)?;
                    }
                    None => journal.committed(
                        entry.id,
                        Landed {
                            size: stat.len,
                            modified_ns: stat.modified.map(crate::nanos),
                            file: Some(stat.id),
                        },
                    )?,
                }
                Ok(Finish::Committed)
            }
            Proved::Gone => Ok(Finish::Failed(
                "it was removed as soon as it was given its name, often by security software"
                    .into(),
            )),
            Proved::Changed => Ok(Finish::Failed(CHANGED_NOW.into())),
            Proved::CannotLook(why) => Ok(Finish::Left(why)),
        }
    };
    let (claimed, filled_in) = match (&entry.state, action) {
        (State::Applied { final_path, .. }, Action::Commit) => {
            return commit_if_proven(final_path, false, None);
        }
        (_, Action::Commit) => return Ok(Finish::Failed(NOT_NAMED.into())),
        (_, _) => {
            // A fill that is not finishing: nothing of PCTwin's is at its name any more (or what
            // is there is left as it is), so the write goes back to its checked file.
            if matches!(entry.state, State::Filling { .. } | State::Filled { .. }) {
                journal.fill_cleared(entry.id, left)?;
            }
            match name(journal, dest, entry, &source)? {
                Naming::Named(claimed, filled_in) => (claimed, filled_in),
                Naming::NotNow(why) => return Ok(Finish::Left(why)),
                Naming::Cannot(why) => return Ok(Finish::Failed(why)),
            }
        }
    };
    // Committed while the temporary name still holds the file, then the temporary name goes.
    let finish = commit_if_proven(&claimed.finished().final_path, filled_in, claimed.show())?;
    if matches!(finish, Finish::Committed) {
        claimed.keep();
    }
    Ok(finish)
}

/// Said of a file that changed after recovery last read it.
const CHANGED_NOW: &str = "the copy was interrupted as it finished, and the file changed before PCTwin could confirm it; it was kept as it is";

/// Said of a checked file that was removed after recovery last read it.
const GONE_NOW: &str = "the copy was interrupted as it finished, and the checked copy was removed before PCTwin could confirm it";

/// What proving a file just now found.
enum Proved {
    Proven(pctwin_gate::Stat),
    Gone,
    Changed,
    CannotLook(String),
}

/// Proves the file at `stored` is, right now, the very file sealed (`sealed`) with exactly the
/// write's size and `fingerprint`. `by_name`: a checked temporary file on a drive whose
/// identities change, proven by its journal-given name and its bytes alone.
fn prove(
    dest: &Destination,
    entry: &Entry,
    stored: &str,
    sealed: Option<FileId>,
    fingerprint: &[u8; 32],
    by_name: bool,
) -> Proved {
    let stat = match dest.stat(stored) {
        Ok(Some(stat)) => stat,
        Ok(None) => return Proved::Gone,
        Err(e) => return Proved::CannotLook(e.to_string()),
    };
    let same = by_name || sealed == Some(stat.id);
    if !same || stat.len != entry.write.size {
        return Proved::Changed;
    }
    // Read back from the drive past this laptop's cache where it allows, as on arrival.
    match dest
        .open_read_back(stored)
        .and_then(|mut f| fingerprint_reader(&mut f, entry.write.size, entry.write.block_size))
    {
        Ok(Some(found)) if &found == fingerprint => Proved::Proven(stat),
        Ok(_) => Proved::Changed,
        Err(e) => Proved::CannotLook(e.to_string()),
    }
}

/// How naming a recovered file ended.
enum Naming<'d> {
    /// Named; and whether it was filled in under the name (a new file there).
    Named(Box<Claimed<'d>>, bool),
    /// Not now (the drive, say): kept, sealed, for the next start.
    NotNow(String),
    /// It cannot be: its temporary file is not one that can be named.
    Cannot(String),
}

/// Gives a verified file its real name, recording each name in the journal before the file gets
/// it. A file that cannot be named now is kept as it is, never thrown away.
fn name<'d>(
    journal: &Journal,
    dest: &'d Destination,
    entry: &Entry,
    source: &Checked<'_>,
) -> Result<Naming<'d>, JournalError> {
    let temp = source.temp;
    let reason = |e: &dyn std::fmt::Display| format!("{NOT_NAMED} ({e})");
    let sent = match IncomingPath::parse(&entry.write.path) {
        Ok(p) => p,
        Err(e) => return Ok(Naming::Cannot(reason(&e))),
    };
    let mut sealed = match dest.reopen_sealed(&sent, temp) {
        Ok(s) => s,
        Err(e) => return Ok(Naming::Cannot(reason(&e))),
    };
    // A copy filled in under its name gets the download marker its original had.
    sealed.keep_download_mark(entry.write.download_mark);
    // Still the very file it sealed, exactly as checked, read now, before it gets a real name.
    let (recorded, fingerprint) = (source.file, source.fingerprint);
    // A reopened sealed file is kept if dropped, so every early return below keeps it.
    match prove(dest, entry, temp, recorded, fingerprint, source.by_name) {
        Proved::Proven(_) => {}
        Proved::CannotLook(why) => return Ok(Naming::NotNow(why)),
        Proved::Gone => return Ok(Naming::Cannot(GONE_NOW.into())),
        Proved::Changed => return Ok(Naming::Cannot(CHANGED_NOW.into())),
    }
    // A name lost in the crash is free again, so it is the one found first.
    for _ in 0..MAX_NAME_TRIES {
        let next = match sealed.next_name() {
            Ok(n) => n,
            Err(e) => return Ok(Naming::NotNow(reason(&e))),
        };
        let named = match name_as(journal, entry.id, sealed, &next, whole(entry, fingerprint)) {
            Ok(named) => named,
            // The reopened checked file is kept when dropped.
            Err(broke) => return Err(broke.0),
        };
        match named.claim {
            Claim::Named(claimed) => {
                return Ok(Naming::Named(Box::new(claimed), named.filled_in));
            }
            Claim::Taken(back) => sealed = back,
            Claim::Failed(e, _) => return Ok(Naming::NotNow(reason(&e))),
        }
    }
    Ok(Naming::NotNow(NOT_NAMED.into()))
}

/// Most names tried when other programs keep taking the free one first.
const MAX_NAME_TRIES: u32 = 64;

/// Removes the journal's own temporary files no kept entry needs: those of entries written since
/// the last clean-up, and any that could not be removed last time. Returns how many it removed.
fn sweep(journal: &Journal, look: &DiskLook<'_>, keep: &HashSet<u64>) -> Result<u64, JournalError> {
    let before = journal.swept_upto()?;
    let mut candidates = journal.entries_after(before)?;
    let last = candidates.last().map_or(before, |e| e.id);
    let earlier = journal.leftovers()?;
    for id in &earlier {
        if *id <= before
            && let Some(e) = journal.entry(*id)?
        {
            candidates.push(e);
        }
    }
    let mut removed = 0;
    let mut leftovers = Vec::new();
    let mut cleared = Vec::new();
    let tidy_list = recovery::sweep(&candidates, keep, look);
    // Where a temporary file would be cannot be worked out now (its place is not reachable): it
    // is looked for again next time.
    for e in &candidates {
        if !keep.contains(&e.id) && !tidy_list.iter().any(|t| t.id == e.id) {
            leftovers.push(e.id);
        }
    }
    for tidy in tidy_list {
        let Some(entry) = candidates.iter().find(|e| e.id == tidy.id) else {
            continue;
        };
        match look
            .destination(entry)
            .map_err(|_| ())
            .and_then(|dest| dest.remove_temp(&tidy.temp).map_err(|_| ()))
        {
            Ok(gone) => {
                removed += u64::from(gone);
                cleared.push(tidy.id);
            }
            Err(()) => leftovers.push(tidy.id),
        }
    }
    let cleared: Vec<u64> = cleared
        .into_iter()
        .filter(|id| earlier.contains(id))
        .collect();
    journal.record_sweep(
        recovery::swept_upto(before, last, keep),
        &leftovers,
        &cleared,
    )?;
    Ok(removed)
}

/// Looks at the disk for recovery, through the gate, only inside approved places that are still
/// the folders they were when the write was planned.
pub(crate) struct DiskLook<'a> {
    pub(crate) journal: &'a Journal,
    pub(crate) table: &'a Destinations,
}

impl DiskLook<'_> {
    pub(crate) fn destination(&self, entry: &Entry) -> Result<&Destination, String> {
        let dest = self
            .table
            .get(&entry.write.destination)
            .map_err(|_| "it is not one of this laptop's approved places now".to_string())?;
        if let Some(place) = entry.write.place {
            let now = dest.folder_identity("").map_err(|e| e.to_string())?;
            if now.map(|id| (id.volume, id.index)) != Some((place.volume, place.index)) {
                return Err("it is not the same folder as when the copy began".into());
            }
        }
        Ok(dest)
    }
}

impl Look for DiskLook<'_> {
    fn look(&self, entry: &Entry, stored: &str) -> Seen {
        match self.destination(entry) {
            Err(why) => Seen::CannotLook(why),
            Ok(dest) => match dest.look(stored) {
                Ok(None) => Seen::Missing,
                Ok(Some(len)) => Seen::File { len },
                Err(e) => Seen::CannotLook(e.to_string()),
            },
        }
    }

    fn matches(&self, entry: &Entry, stored: &str, fingerprint: &[u8; 32]) -> Result<bool, String> {
        let dest = self.destination(entry)?;
        // The same reader as on arrival: from the drive, past the cache where it allows.
        let mut file = dest.open_read_back(stored).map_err(|e| e.to_string())?;
        let found = fingerprint_reader(&mut file, entry.write.size, entry.write.block_size)
            .map_err(|e| e.to_string())?;
        Ok(found.as_ref() == Some(fingerprint))
    }

    fn identity(&self, entry: &Entry, stored: &str) -> Result<Option<FileId>, String> {
        let dest = self.destination(entry)?;
        Ok(dest.stat(stored).map_err(|e| e.to_string())?.map(|s| s.id))
    }

    fn hidden(&self, entry: &Entry, stored: &str) -> Result<bool, String> {
        let dest = self.destination(entry)?;
        dest.hidden(stored).map_err(|e| e.to_string())
    }

    fn born(&self, entry: &Entry, stored: &str) -> Result<Option<Born>, String> {
        let dest = self.destination(entry)?;
        dest.born(stored).map_err(|e| e.to_string())
    }

    fn same_start(
        &self,
        entry: &Entry,
        stored: &str,
        temp: &str,
        len: u64,
    ) -> Result<bool, String> {
        let dest = self.destination(entry)?;
        dest.same_start(stored, temp, len)
            .map_err(|e| e.to_string())
    }

    fn temp_path(&self, entry: &Entry, folder: Option<&str>) -> Option<String> {
        let folder = match folder {
            Some(f) => f.to_string(),
            None => {
                let dest = self.destination(entry).ok()?;
                dest.folder_of(&IncomingPath::parse(&entry.write.path).ok()?)
            }
        };
        let name = temp_name(&self.journal.temp_tag(entry.id));
        Some(if folder.is_empty() {
            name
        } else {
            format!("{folder}/{name}")
        })
    }
}
