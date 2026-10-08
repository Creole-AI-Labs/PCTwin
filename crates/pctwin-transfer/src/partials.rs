//! Partly copied files kept to continue from (Task List 1.6, second review 9). A file the old
//! laptop stopped sending is kept so the move can carry on later without sending it again; it is
//! not kept forever: each move waits as long as the person chose before it (3 to 90 days, 30
//! unless they picked another). Like Windows' own background transfers (which give up on a transfer with no
//! progress for 90 days and remove its partial files), a partly copied file goes once nothing has
//! been added to it for too long, once partly copied files take more space than allowed (oldest
//! first), or once the move it belongs to, or its laptop, is gone. Its write then ends as failed,
//! so the file is simply sent again from the start next time. Nothing else is ever touched: only
//! PCTwin's own temporary files of writes still waiting for the rest of their bytes.
//!
//! Run it at start, after [`crate::recover`] and before anything is received, as recovery is.

use std::time::SystemTime;

use pctwin_gate::Destinations;
use pctwin_journal::{Entry, Journal, JournalError, State};

use crate::recovery::DiskLook;

/// Said of a partly copied file nothing was added to for too long.
pub const PARTIAL_TOO_OLD: &str = "the rest of this file did not arrive for a long time, so the part already copied was removed to free space; it is copied from the start next time";
/// Said of a partly copied file removed because such files took more space than allowed.
pub const PARTIAL_TOO_MUCH: &str = "partly copied files were taking more space than allowed, so this one, among the oldest, was removed; it is copied from the start next time";
/// Said of a partly copied file whose move, or laptop, is gone.
pub const PARTIAL_NOT_WANTED: &str = "the move it was part of was cancelled, or its laptop removed, so the part already copied was removed";

/// How long, and how much, partly copied files are kept.
pub struct KeepPartials<'a> {
    /// The time now.
    pub now: SystemTime,
    /// The most space all partly copied files may take together.
    pub max_bytes: u64,
    /// Whether a write is still wanted: false once its move is cancelled or its laptop removed.
    pub wanted: &'a dyn Fn(&Entry) -> bool,
}

/// One partly copied file kept.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Partial {
    pub id: u64,
    /// The space it takes.
    pub bytes: u64,
    /// When something was last added to it, if the drive says.
    pub modified: Option<SystemTime>,
}

/// What is kept of partly copied files, and what was removed.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct Partials {
    /// Kept to continue from.
    pub kept: Vec<Partial>,
    /// The space the kept ones take together.
    pub kept_bytes: u64,
    /// Writes ended now, with why (their partly copied files removed, or removed at the next start
    /// if their place cannot be reached now).
    pub ended: Vec<(u64, String)>,
    /// The space freed now.
    pub freed_bytes: u64,
    /// Kept, but not looked at, because their place cannot be reached now: with why.
    pub unseen: Vec<(u64, String)>,
}

/// The partly copied files kept, and the space they take. Changes nothing.
pub fn partials(journal: &Journal, table: &Destinations) -> Result<Partials, JournalError> {
    let look = DiskLook { journal, table };
    let mut out = Partials::default();
    for (entry, _) in waiting(journal)? {
        match seen(&look, &entry) {
            Ok(Some(p)) => {
                out.kept_bytes += p.bytes;
                out.kept.push(p);
            }
            Ok(None) => {}
            Err(why) => out.unseen.push((entry.id, why)),
        }
    }
    Ok(out)
}

/// Removes the partly copied files `keep` no longer allows, ending their writes as failed, and
/// says what is kept.
pub fn expire_partials(
    journal: &Journal,
    table: &Destinations,
    keep: &KeepPartials<'_>,
) -> Result<Partials, JournalError> {
    let look = DiskLook { journal, table };
    let mut out = Partials::default();
    let mut kept: Vec<(Partial, String)> = Vec::new();
    for (entry, temp) in waiting(journal)? {
        if !(keep.wanted)(&entry) {
            end(&look, &entry, &temp, PARTIAL_NOT_WANTED, &mut out)?;
            continue;
        }
        match seen(&look, &entry) {
            Ok(Some(p)) => {
                // Nothing added for longer than allowed. A time ahead of now (the clock was
                // changed) is not old.
                let old = p
                    .modified
                    .and_then(|m| keep.now.duration_since(m).ok())
                    .is_some_and(|age| age > entry.write.partial_keep.duration());
                if old {
                    end(&look, &entry, &temp, PARTIAL_TOO_OLD, &mut out)?;
                } else {
                    kept.push((p, temp));
                }
            }
            Ok(None) => {}
            Err(why) => out.unseen.push((entry.id, why)),
        }
    }
    // Over the space allowed: the oldest go first (one whose time is unknown counts as oldest).
    kept.sort_by_key(|(p, _)| (p.modified, p.id));
    let mut total: u64 = kept.iter().map(|(p, _)| p.bytes).sum();
    let mut kept = kept.into_iter();
    for (p, temp) in kept.by_ref() {
        if total <= keep.max_bytes {
            out.kept_bytes += p.bytes;
            out.kept.push(p);
            break;
        }
        total -= p.bytes;
        let Some(entry) = journal.entry(p.id)? else {
            continue;
        };
        end(&look, &entry, &temp, PARTIAL_TOO_MUCH, &mut out)?;
    }
    for (p, _) in kept {
        out.kept_bytes += p.bytes;
        out.kept.push(p);
    }
    Ok(out)
}

/// Unfinished writes still waiting for the rest of their bytes, with their temporary file.
fn waiting(journal: &Journal) -> Result<Vec<(Entry, String)>, JournalError> {
    Ok(journal
        .unfinished()?
        .into_iter()
        .filter_map(|e| match &e.state {
            State::Staged { temp } => {
                let temp = temp.clone();
                Some((e, temp))
            }
            _ => None,
        })
        .collect())
}

/// The partly copied file of `entry` as it is now, `None` if there is none, or why it cannot be
/// looked at.
fn seen(look: &DiskLook<'_>, entry: &Entry) -> Result<Option<Partial>, String> {
    let State::Staged { temp } = &entry.state else {
        return Ok(None);
    };
    let dest = look.destination(entry)?;
    Ok(dest
        .stat(temp)
        .map_err(|e| e.to_string())?
        .map(|s| Partial {
            id: entry.id,
            bytes: s.len,
            modified: s.modified,
        }))
}

/// Ends `entry`'s write as failed with `why`, then removes its partly copied file. Recorded first:
/// if the file cannot be removed now (or PCTwin stops first), the next start removes it.
fn end(
    look: &DiskLook<'_>,
    entry: &Entry,
    temp: &str,
    why: &str,
    out: &mut Partials,
) -> Result<(), JournalError> {
    look.journal.failed(entry.id, why)?;
    out.ended.push((entry.id, why.into()));
    let removed = look.destination(entry).ok().and_then(|dest| {
        let bytes = dest.stat(temp).ok().flatten().map_or(0, |s| s.len);
        dest.remove_temp(temp)
            .ok()
            .map(|gone| if gone { bytes } else { 0 })
    });
    match removed {
        Some(bytes) => out.freed_bytes += bytes,
        None => {
            let upto = look.journal.swept_upto()?;
            look.journal.record_sweep(upto, &[entry.id], &[])?;
        }
    }
    Ok(())
}
