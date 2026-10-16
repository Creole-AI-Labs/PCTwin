//! What each unfinished write needs after a restart (the plan: "reconcile partial work after
//! a crash"; engineering notes: "on restart, reconcile partial actions instead of announcing
//! success").
//!
//! This part only decides; it looks at the disk through [`Look`] and changes nothing, so every
//! case can be tested without a disk and a fault can be put between any two steps. The app carries
//! the decisions out. Deciding again after the decisions were (partly) carried out gives the same
//! or a later answer, so recovery interrupted by another crash is as safe as recovery run once.
//!
//! The rules (each write is proven before it is committed, never assumed):
//! - **Planned**: nothing of it can be trusted: failed.
//! - **Staged**: its partly copied file is kept to continue from, if it is still there; the blocks
//!   in it are checked again against their fingerprints before any is counted.
//! - **Verified**: its complete file is checked again against the whole file's fingerprint, then
//!   given its real name (never replacing anything) and committed.
//! - **Applied**: the name it was getting is looked at. Only its own file there is committed:
//!   the very file it sealed (by identity on its drive) with exactly its fingerprint. Any other
//!   file at that name, however alike (an empty one, or a copy of the same contents), is someone
//!   else's and is passed over for a new name; a lost name is given again from the temporary
//!   file. A file at the name that cannot be proven its own is never taken and never replaced.
//! - **Filling** and **Filled** (a drive with no safe rename: the real name was made only if
//!   free, tagged, and filled from the checked temporary file): the file at the real name is
//!   finished only when it is proven PCTwin's (its recorded identity where the drive keeps one,
//!   else the creation time PCTwin gave it) and is exactly the whole file; a partial copy proven
//!   PCTwin's, whose bytes are exactly the start of the checked temporary file, is removed and
//!   the file named again. Anything else at the real name is left as it is, and the person is
//!   told; the checked file is then named elsewhere. Where identities change, the checked
//!   temporary file is proven by its journal-given name with its whole fingerprint.
//! - Anything whose place cannot be looked at now (a drive not plugged in, a folder that is not
//!   the approved one) is left exactly as it is for next time.

use std::collections::HashSet;

use crate::{Born, Entry, FileId, Fill, Proof, State};

/// What recovery found at a stored path.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Seen {
    Missing,
    /// A regular file of this size.
    File {
        len: u64,
    },
    /// The place could not be looked at (why, in plain words).
    CannotLook(String),
}

/// How recovery looks at the disk, for one entry's destination.
pub trait Look {
    /// What is at the stored path `stored` in `entry`'s destination.
    fn look(&self, entry: &Entry, stored: &str) -> Seen;
    /// Whether the file at `stored` is exactly `entry`'s size with this fingerprint (reads all of
    /// it). `Err` if it could not be read.
    fn matches(&self, entry: &Entry, stored: &str, fingerprint: &[u8; 32]) -> Result<bool, String>;
    /// Which file is at `stored` (its identity on its drive), if a file is there.
    fn identity(&self, entry: &Entry, stored: &str) -> Result<Option<FileId>, String>;
    /// The stored path of `entry`'s temporary file in `folder`, or (no folder given) in the folder
    /// its path lands in. `None` if that cannot be worked out.
    fn temp_path(&self, entry: &Entry, folder: Option<&str>) -> Option<String>;
    /// The creation time of the file at `stored`, as its drive keeps it, if it keeps one.
    fn born(&self, entry: &Entry, stored: &str) -> Result<Option<Born>, String>;
    /// Whether the file at `stored` is marked hidden.
    fn hidden(&self, entry: &Entry, stored: &str) -> Result<bool, String>;
    /// Whether the file at `stored` is exactly `len` bytes, and they are exactly the first `len`
    /// bytes of the file at `temp`.
    fn same_start(&self, entry: &Entry, stored: &str, temp: &str, len: u64)
    -> Result<bool, String>;
}

/// What to do with one unfinished write.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Action {
    /// It cannot be finished: record it failed, with why in plain words.
    Fail(String),
    /// Its partly copied file is kept, to continue when the old laptop sends the rest.
    Resume,
    /// Its place cannot be looked at now: left exactly as it is for next time.
    Leave(String),
    /// Its checked, complete file gets its real name (the name applied, if still free, else a new
    /// one recorded first), then it is committed.
    Name,
    /// It has its real name and the file there is proven to be it: committed.
    Commit,
    /// The file filled in at its real name is proven PCTwin's and whole: made durable and
    /// checked again, given its modified time and shown, then committed.
    FinishFill,
    /// The file at its real name is PCTwin's own partial copy, proven: removed, then the checked
    /// temporary file is named again.
    RemoveFillThenName,
    /// The checked temporary file is named again (elsewhere: the file at the real name is not
    /// proven PCTwin's and is left as it is), and the person is told, in plain words, what was
    /// left there.
    NameNoting(String),
}

/// The decision for one entry.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Decision {
    pub id: u64,
    pub action: Action,
}

const BEFORE: &str = "the copy was interrupted before it began";
const PART_GONE: &str = "the copy was interrupted, and its partly copied file was gone afterwards";
const NOT_CONFIRMED: &str =
    "the copy was interrupted as it finished, and PCTwin could not confirm it afterwards";
const CHANGED: &str =
    "the copy was interrupted, and the file changed afterwards, so it could not be confirmed";
const CHANGED_KEPT: &str = "the copy was interrupted as it finished, and the file changed afterwards, so it could not be confirmed; it was kept as it is";
const LOST: &str = "the copy was interrupted as it finished, and the file was not found afterwards";
const FILL_LEFT: &str = "the copy was interrupted while it was being written under its name, and PCTwin could not confirm that the file now there is its own unfinished copy, so it left it as it is";
const FILL_LEFT_SAFE: &str = "the copy was interrupted while it was being written under its name, and the original's copy here could not be checked again, so PCTwin left its own unfinished copy as it is; that partly copied file (hidden, on drives that can hide files) is safe to delete";
const FILL_CHANGED: &str = "the copy was interrupted while it was being written under its name, and the unfinished copy there changed afterwards, so PCTwin left it as it is";

/// What was left at the real name, for the person: where, and whether it may be deleted.
fn left_at(stored: &str, why: &str) -> String {
    format!("{why}: {stored}")
}

fn leave(why: &str) -> Action {
    Action::Leave(format!(
        "the place it was going to could not be looked at ({why})"
    ))
}

/// What each unfinished entry needs, in order.
pub fn plan(unfinished: &[Entry], look: &impl Look) -> Vec<Decision> {
    unfinished
        .iter()
        .filter(|e| !e.state.is_finished())
        .map(|e| Decision {
            id: e.id,
            action: decide(e, look),
        })
        .collect()
}

/// What one unfinished entry needs.
pub fn decide(entry: &Entry, look: &impl Look) -> Action {
    let size = entry.write.size;
    // Checks the file at `stored` is exactly this write: `Ok(true)` proven, `Ok(false)` not.
    let proven = |stored: &str, len: u64, fingerprint: &[u8; 32]| -> Result<bool, Action> {
        if len != size {
            return Ok(false);
        }
        look.matches(entry, stored, fingerprint)
            .map_err(|why| leave(&why))
    };
    // Whether the file at `stored` is the very file it sealed. Without a recorded identity (a drive
    // that gives none) nothing can be proven its own.
    let own = |stored: &str, file: Option<FileId>| -> Result<bool, Action> {
        let Some(file) = file else {
            return Ok(false);
        };
        look.identity(entry, stored)
            .map(|found| found == Some(file))
            .map_err(|why| leave(&why))
    };
    match &entry.state {
        State::Planned => Action::Fail(BEFORE.into()),
        State::Staged { temp } => match look.look(entry, temp) {
            Seen::Missing => Action::Fail(PART_GONE.into()),
            Seen::CannotLook(why) => leave(&why),
            Seen::File { len } if len <= size => Action::Resume,
            Seen::File { .. } => Action::Fail(CHANGED.into()),
        },
        State::Verified {
            temp,
            fingerprint,
            file,
        } => match look.look(entry, temp) {
            Seen::Missing => Action::Fail(NOT_CONFIRMED.into()),
            Seen::CannotLook(why) => leave(&why),
            Seen::File { len } => match own(temp, *file).and_then(|mine| {
                if mine {
                    proven(temp, len, fingerprint)
                } else {
                    Ok(false)
                }
            }) {
                Ok(true) => Action::Name,
                Ok(false) => Action::Fail(CHANGED.into()),
                Err(left) => left,
            },
        },
        State::Applied {
            temp,
            final_path,
            fingerprint,
            file,
        } => {
            let at_name = match look.look(entry, final_path) {
                Seen::CannotLook(why) => return leave(&why),
                Seen::Missing => None,
                Seen::File { len } => Some(len),
            };
            // Its own file at the name: the very file it sealed, then exactly its fingerprint.
            if let Some(len) = at_name {
                match own(final_path, *file) {
                    Err(left) => return left,
                    Ok(true) => {
                        return match proven(final_path, len, fingerprint) {
                            Ok(true) => Action::Commit,
                            Ok(false) => Action::Fail(CHANGED_KEPT.into()),
                            Err(left) => left,
                        };
                    }
                    // Someone else's file, however alike: never taken, never replaced.
                    Ok(false) => {}
                }
            }
            // Not named yet (or the name was lost, or taken): name it from its temporary file.
            match look.look(entry, temp) {
                Seen::CannotLook(why) => leave(&why),
                Seen::Missing if at_name.is_some() => Action::Fail(NOT_CONFIRMED.into()),
                Seen::Missing => Action::Fail(LOST.into()),
                Seen::File { len } => match own(temp, *file).and_then(|mine| {
                    if mine {
                        proven(temp, len, fingerprint)
                    } else {
                        Ok(false)
                    }
                }) {
                    Ok(true) => Action::Name,
                    Ok(false) => Action::Fail(CHANGED.into()),
                    Err(left) => left,
                },
            }
        }
        State::Filling {
            temp,
            fingerprint,
            file,
            fill,
        } => decide_fill(entry, look, temp, fingerprint, *file, fill, false),
        State::Filled {
            temp,
            fingerprint,
            file,
            fill,
        } => decide_fill(entry, look, temp, fingerprint, *file, fill, true),
        State::Committed { .. } | State::Existing { .. } | State::Failed { .. } => {
            Action::Fail("already finished".into())
        }
    }
}

/// A file filled in under its real name (see the module notes). `filled`: it was recorded
/// complete, durable and read back.
fn decide_fill(
    entry: &Entry,
    look: &impl Look,
    temp: &str,
    fingerprint: &[u8; 32],
    file: Option<FileId>,
    fill: &Fill,
    filled: bool,
) -> Action {
    let size = entry.write.size;
    let at_name = match look.look(entry, &fill.final_path) {
        Seen::CannotLook(why) => return leave(&why),
        Seen::Missing => None,
        Seen::File { len } => Some(len),
    };
    // The checked temporary file, still exactly as checked: its identity where the drive keeps
    // identities, else its journal-given name; then its whole fingerprint.
    let source = || -> Result<bool, Action> {
        match look.look(entry, temp) {
            Seen::CannotLook(why) => Err(leave(&why)),
            Seen::Missing => Ok(false),
            Seen::File { len } => {
                if len != size {
                    return Ok(false);
                }
                let mine = fill.proof == Proof::Token
                    || match file {
                        Some(file) => look
                            .identity(entry, temp)
                            .map_err(|why| leave(&why))?
                            .is_some_and(|found| found == file),
                        None => false,
                    };
                if !mine {
                    return Ok(false);
                }
                look.matches(entry, temp, fingerprint)
                    .map_err(|why| leave(&why))
            }
        }
    };
    let Some(len) = at_name else {
        // Nothing at the real name (never made, or lost since): named again from the checked file.
        return match source() {
            Ok(true) => Action::Name,
            Ok(false) => Action::Fail(LOST.into()),
            Err(left) => left,
        };
    };
    let ours = match fill_is_ours(entry, look, fill) {
        Ok(true) => true,
        // Stopped between making the name and tagging it: made hidden, nothing in it yet, and
        // nothing recorded as made (the tag comes before that). Only PCTwin's own exclusively made
        // file can be at the name then, and an empty file holds nothing to lose.
        Ok(false) if !filled && len == 0 && fill.made.is_none() && fill.tag.hidden => {
            match look.hidden(entry, &fill.final_path) {
                Ok(hidden) => hidden,
                Err(why) => return leave(&why),
            }
        }
        Ok(false) => false,
        Err(left) => return left,
    };
    if !ours {
        // Someone else's file, or one nothing proves is PCTwin's: never touched.
        return match source() {
            Ok(true) => Action::NameNoting(left_at(&fill.final_path, FILL_LEFT)),
            Ok(false) => Action::Fail(left_at(&fill.final_path, FILL_LEFT)),
            Err(left) => left,
        };
    }
    if len == size {
        match look.matches(entry, &fill.final_path, fingerprint) {
            Ok(true) => return Action::FinishFill,
            Ok(false) => {}
            Err(why) => return leave(&why),
        }
    }
    if filled {
        // Complete and checked when it was recorded filled, different now: changed by someone.
        return Action::Fail(CHANGED_KEPT.into());
    }
    match source() {
        Ok(true) => {}
        Ok(false) => return Action::Fail(left_at(&fill.final_path, FILL_LEFT_SAFE)),
        Err(left) => return left,
    }
    match look.same_start(entry, &fill.final_path, temp, len) {
        Ok(true) if len <= size => Action::RemoveFillThenName,
        Ok(_) => Action::NameNoting(left_at(&fill.final_path, FILL_CHANGED)),
        Err(why) => leave(&why),
    }
}

/// Whether the file at the real name is the one PCTwin made: its recorded identity, on a drive
/// that keeps identities; or the creation time PCTwin gave it at once, where it gave one (on any
/// drive, since the identity is recorded only just after the file is made).
fn fill_is_ours(entry: &Entry, look: &impl Look, fill: &Fill) -> Result<bool, Action> {
    if fill.proof == Proof::StableId
        && let Some(made) = fill.made
        && look
            .identity(entry, &fill.final_path)
            .map_err(|why| leave(&why))?
            == Some(made)
    {
        return Ok(true);
    }
    if !matches!(fill.proof, Proof::StableId | Proof::Token) {
        return Ok(false);
    }
    let Some(tag) = fill.tag.born else {
        return Ok(false);
    };
    Ok(look
        .born(entry, &fill.final_path)
        .map_err(|why| leave(&why))?
        == Some(tag))
}

/// A temporary file to remove in clean-up.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Tidy {
    pub id: u64,
    pub destination: String,
    /// Its stored path inside the destination.
    pub temp: String,
}

/// Where `entry`'s temporary file is (or would be, if one was made).
pub fn temp_of(entry: &Entry, look: &impl Look) -> Option<String> {
    fn at(entry: &Entry, state: &State, look: &impl Look) -> Option<String> {
        let folder_of = |stored: &str| stored.rsplit_once('/').map_or("", |(f, _)| f).to_string();
        match state {
            State::Staged { temp }
            | State::Verified { temp, .. }
            | State::Applied { temp, .. }
            | State::Filling { temp, .. }
            | State::Filled { temp, .. } => Some(temp.clone()),
            State::Committed { final_path, .. } => {
                look.temp_path(entry, Some(&folder_of(final_path)))
            }
            State::Planned | State::Existing { .. } => look.temp_path(entry, None),
            State::Failed { reached, .. } => at(entry, reached, look),
        }
    }
    at(entry, &entry.state, look)
}

/// The temporary files to remove: those of `candidates` except entries in `keep` (kept to resume
/// or left for next time). Only names the journal itself gave are ever listed.
pub fn sweep(candidates: &[Entry], keep: &HashSet<u64>, look: &impl Look) -> Vec<Tidy> {
    candidates
        .iter()
        .filter(|e| !keep.contains(&e.id))
        .filter_map(|e| {
            temp_of(e, look).map(|temp| Tidy {
                id: e.id,
                destination: e.write.destination.clone(),
                temp,
            })
        })
        .collect()
}

/// How far clean-up has looked once this one is done: up to just below the first entry kept (it
/// is looked at again once it finishes), else up to the last entry there is.
pub fn swept_upto(before: u64, last: u64, keep: &HashSet<u64>) -> u64 {
    match keep.iter().min() {
        Some(first_kept) => first_kept.saturating_sub(1).min(last.max(before)),
        None => last.max(before),
    }
}
