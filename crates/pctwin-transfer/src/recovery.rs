//! Carrying out recovery after a restart (Task List 1.6): the journal decides
//! ([`pctwin_journal::recovery`]), this looks at the disk for it through the safety gate and does
//! what was decided, then cleans up the journal's own temporary files.
//!
//! Run it once at start, before anything is received. Running it again (or after a crash part of
//! the way through) is as safe as running it once: every step is recorded in the journal before or
//! as it happens, and each decision is made again from what the journal and the disk say.

use std::collections::HashSet;

use pctwin_gate::{Claimed, Destination, Destinations, IncomingPath, temp_name};
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
    let look = DiskLook { journal, table };
    let mut done = Recovered::default();
    let unfinished = journal.unfinished()?;
    for decision in recovery::plan(&unfinished, &look) {
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
            Action::Name | Action::TakeReservation | Action::Commit => {
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
    let final_path = match (&entry.state, action) {
        (State::Applied { final_path, .. }, Action::Commit) => final_path.clone(),
        (_, Action::Commit) => return Ok(Finish::Failed(NOT_NAMED.into())),
        (_, _) => match name(journal, dest, entry, action)? {
            Ok(claimed) => claimed.keep().final_path,
            Err(why) => return Ok(Finish::Failed(why)),
        },
    };
    match dest.stat(&final_path) {
        Ok(Some(stat)) => {
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
        // It has its name in the journal; the next recovery looks again.
        Ok(None) => Ok(Finish::Left("its file could not be found just now".into())),
        Err(e) => Ok(Finish::Left(e.to_string())),
    }
}

/// Gives a verified file its real name, recording each name in the journal before the file gets
/// it. `Err` is why it could not be named.
fn name<'d>(
    journal: &Journal,
    dest: &'d Destination,
    entry: &Entry,
    action: &Action,
) -> Result<Result<Claimed<'d>, String>, JournalError> {
    let (temp, applied) = match &entry.state {
        State::Verified { temp, .. } => (temp, None),
        State::Applied {
            temp, final_path, ..
        } => (temp, Some(final_path)),
        _ => return Ok(Err(NOT_NAMED.into())),
    };
    let reason = |e: &dyn std::fmt::Display| format!("{NOT_NAMED} ({e})");
    let sent = match IncomingPath::parse(&entry.write.path) {
        Ok(p) => p,
        Err(e) => return Ok(Err(reason(&e))),
    };
    let mut sealed = match dest.reopen_sealed(&sent, temp) {
        Ok(s) => s,
        Err(e) => return Ok(Err(reason(&e))),
    };
    if let Some(final_path) = applied
        && matches!(action, Action::TakeReservation)
    {
        return Ok(sealed.take_reservation(final_path).map_err(|e| reason(&e)));
    }
    // A name lost in the crash is free again, so it is the one found first.
    for _ in 0..MAX_NAME_TRIES {
        let next = match sealed.next_name() {
            Ok(n) => n,
            Err(e) => return Ok(Err(reason(&e))),
        };
        journal.applied(entry.id, &next)?;
        match sealed.claim_as(&next) {
            Ok(Ok(claimed)) => return Ok(Ok(claimed)),
            Ok(Err(back)) => sealed = back,
            Err(e) => return Ok(Err(reason(&e))),
        }
    }
    Ok(Err(NOT_NAMED.into()))
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
struct DiskLook<'a> {
    journal: &'a Journal,
    table: &'a Destinations,
}

impl DiskLook<'_> {
    fn destination(&self, entry: &Entry) -> Result<&Destination, String> {
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

    fn same_file(&self, entry: &Entry, a: &str, b: &str) -> Result<bool, String> {
        let dest = self.destination(entry)?;
        let a = dest.stat(a).map_err(|e| e.to_string())?;
        let b = dest.stat(b).map_err(|e| e.to_string())?;
        Ok(matches!((a, b), (Some(a), Some(b)) if a.id == b.id))
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
