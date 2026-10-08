//! Carrying out recovery after a restart (Task List 1.6): the journal decides
//! ([`pctwin_journal::recovery`]), this looks at the disk for it through the safety gate and does
//! what was decided, then cleans up the journal's own temporary files.
//!
//! Run it once at start, before anything is received. Running it again (or after a crash part of
//! the way through) is as safe as running it once: every step is recorded in the journal before or
//! as it happens, and each decision is made again from what the journal and the disk say.

use std::collections::HashSet;

use pctwin_gate::{Claim, Claimed, Destination, Destinations, IncomingPath, temp_name};
use pctwin_journal::recovery::{self, Action, Look, Seen};
use pctwin_journal::{Entry, FileId, Journal, JournalError, Landed, State};

use crate::fingerprint_reader;

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
                match finish(journal, &look, entry, &decision.action)? {
                    Finish::Committed => done.committed.push(entry.id),
                    Finish::Failed(why) => {
                        journal.failed(entry.id, &why)?;
                        done.failed.push((entry.id, why));
                    }
                    Finish::Left(why) => done.left.push((entry.id, why)),
                }
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

const NOT_NAMED: &str =
    "the copy was interrupted as it finished, and PCTwin could not give it its name afterwards";

/// Names (if needed) and commits one entry recovery found complete.
fn finish(
    journal: &Journal,
    look: &DiskLook<'_>,
    entry: &Entry,
    action: &Action,
) -> Result<Finish, JournalError> {
    let dest = match look.destination(entry) {
        Ok(dest) => dest,
        Err(why) => return Ok(Finish::Left(why)),
    };
    // Proven again immediately before it is committed (not only when it was decided): the very
    // file it sealed, exactly its size and fingerprint, read now.
    let commit_if_proven = |stored: &str| -> Result<Finish, JournalError> {
        let (sealed, fingerprint) = match &entry.state {
            State::Verified {
                file, fingerprint, ..
            }
            | State::Applied {
                file, fingerprint, ..
            } => (*file, fingerprint),
            _ => return Ok(Finish::Failed(NOT_NAMED.into())),
        };
        match prove(dest, entry, stored, sealed, fingerprint) {
            Proof::Proven(stat) => {
                journal.committed(
                    entry.id,
                    Landed {
                        size: stat.len,
                        modified_ns: stat.modified.map(crate::nanos),
                        file: Some(FileId {
                            volume: stat.id.volume,
                            index: stat.id.index,
                        }),
                    },
                )?;
                Ok(Finish::Committed)
            }
            Proof::Gone => Ok(Finish::Failed(
                "it was removed as soon as it was given its name, often by security software"
                    .into(),
            )),
            Proof::Changed => Ok(Finish::Failed(CHANGED_NOW.into())),
            Proof::CannotLook(why) => Ok(Finish::Left(why)),
        }
    };
    let claimed = match (&entry.state, action) {
        (State::Applied { final_path, .. }, Action::Commit) => {
            return commit_if_proven(final_path);
        }
        (_, Action::Commit) => return Ok(Finish::Failed(NOT_NAMED.into())),
        (_, _) => match name(journal, dest, entry)? {
            Naming::Named(claimed) => claimed,
            Naming::NotNow(why) => return Ok(Finish::Left(why)),
            Naming::Cannot(why) => return Ok(Finish::Failed(why)),
        },
    };
    // Committed while the temporary name still holds the file, then the temporary name goes.
    let finish = commit_if_proven(&claimed.finished().final_path)?;
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
enum Proof {
    Proven(pctwin_gate::Stat),
    Gone,
    Changed,
    CannotLook(String),
}

/// Proves the file at `stored` is, right now, the very file sealed (`sealed`) with exactly the
/// write's size and `fingerprint`.
fn prove(
    dest: &Destination,
    entry: &Entry,
    stored: &str,
    sealed: Option<FileId>,
    fingerprint: &[u8; 32],
) -> Proof {
    let stat = match dest.stat(stored) {
        Ok(Some(stat)) => stat,
        Ok(None) => return Proof::Gone,
        Err(e) => return Proof::CannotLook(e.to_string()),
    };
    let same = sealed.is_some_and(|f| f.volume == stat.id.volume && f.index == stat.id.index);
    if !same || stat.len != entry.write.size {
        return Proof::Changed;
    }
    match dest
        .open_read(stored)
        .and_then(|mut f| fingerprint_reader(&mut f, entry.write.size, entry.write.block_size))
    {
        Ok(Some(found)) if &found == fingerprint => Proof::Proven(stat),
        Ok(_) => Proof::Changed,
        Err(e) => Proof::CannotLook(e.to_string()),
    }
}

/// How naming a recovered file ended.
enum Naming<'d> {
    Named(Claimed<'d>),
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
) -> Result<Naming<'d>, JournalError> {
    let temp = match &entry.state {
        State::Verified { temp, .. } | State::Applied { temp, .. } => temp,
        _ => return Ok(Naming::Cannot(NOT_NAMED.into())),
    };
    let reason = |e: &dyn std::fmt::Display| format!("{NOT_NAMED} ({e})");
    let sent = match IncomingPath::parse(&entry.write.path) {
        Ok(p) => p,
        Err(e) => return Ok(Naming::Cannot(reason(&e))),
    };
    let mut sealed = match dest.reopen_sealed(&sent, temp) {
        Ok(s) => s,
        Err(e) => return Ok(Naming::Cannot(reason(&e))),
    };
    // Still the very file it sealed, exactly as checked, read now, before it gets a real name.
    let (recorded, fingerprint) = match &entry.state {
        State::Verified {
            file, fingerprint, ..
        }
        | State::Applied {
            file, fingerprint, ..
        } => (*file, fingerprint),
        _ => return Ok(Naming::Cannot(NOT_NAMED.into())),
    };
    // A reopened sealed file is kept if dropped, so every early return below keeps it.
    match prove(dest, entry, temp, recorded, fingerprint) {
        Proof::Proven(_) => {}
        Proof::CannotLook(why) => return Ok(Naming::NotNow(why)),
        Proof::Gone => return Ok(Naming::Cannot(GONE_NOW.into())),
        Proof::Changed => return Ok(Naming::Cannot(CHANGED_NOW.into())),
    }
    // A name lost in the crash is free again, so it is the one found first.
    for _ in 0..MAX_NAME_TRIES {
        let next = match sealed.next_name() {
            Ok(n) => n,
            Err(e) => return Ok(Naming::NotNow(reason(&e))),
        };
        journal.applied(entry.id, &next)?;
        match sealed.claim_as(&next) {
            Claim::Named(claimed) => return Ok(Naming::Named(claimed)),
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
        let mut file = dest.open_read(stored).map_err(|e| e.to_string())?;
        let found = fingerprint_reader(&mut file, entry.write.size, entry.write.block_size)
            .map_err(|e| e.to_string())?;
        Ok(found.as_ref() == Some(fingerprint))
    }

    fn identity(&self, entry: &Entry, stored: &str) -> Result<Option<FileId>, String> {
        let dest = self.destination(entry)?;
        Ok(dest
            .stat(stored)
            .map_err(|e| e.to_string())?
            .map(|s| FileId {
                volume: s.id.volume,
                index: s.id.index,
            }))
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
