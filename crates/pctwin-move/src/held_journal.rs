//! Other people's writes are journaled beside their files, never in the mover's own journal
//! (Security Design part C and G: "others' files may pass through PCTwin's engine, never shown
//! to the mover", decided 21 October 2026).
//!
//! The new laptop's change journal lives in the mover's own app data, where the mover can read
//! it after the move. A write into another person's holding place names their file, so it goes
//! to a journal kept inside that person's holding folder instead (`move.journal`). That folder is
//! the helper's (Windows: under Program Files, administrators only; macOS and Linux: under
//! root's staging folder): the mover's engine may write there during the move, as it writes the
//! files themselves, and the mover loses it at the hand-over, when the folder is locked to the
//! administrators or given to the person. The journal there is exactly as private as the files
//! it names, so it adds no new exposure, and no new cryptography is needed. The hand-over never
//! hands it over (it walks the place folders only) and removes it with the folder.
//!
//! [`SplitLedger`] is the one ledger the receiver writes through: entries for a holding place go
//! to that place's journal, everything else to the mover's.

use std::collections::BTreeMap;
use std::path::{Path, PathBuf};
use std::sync::{Arc, Mutex, PoisonError};

use pctwin_journal::{
    CheckedFrom, Entry, FileId, FillTag, Journal, JournalError, Landed, Ledger, PlannedWrite,
    Proof, Show,
};

/// The journal's file inside a person's holding folder.
pub const HELD_JOURNAL: &str = "move.journal";

/// Entry numbers of a holding place's journal carry the place's number above this many bits;
/// the mover's own have none there.
const SHIFT: u32 = 48;

/// The holding places of this move, by their destination label, as they are approved (the
/// helper makes them once the plan is approved, so they are added while the move runs).
#[derive(Debug, Clone, Default)]
pub struct HeldPlaces(Arc<Mutex<BTreeMap<String, PathBuf>>>);

impl HeldPlaces {
    /// Records that the destination `label` is the holding folder `folder`.
    pub fn hold(&self, label: &str, folder: &Path) {
        self.0
            .lock()
            .unwrap_or_else(PoisonError::into_inner)
            .insert(label.to_string(), folder.to_path_buf());
    }

    fn folder(&self, label: &str) -> Option<PathBuf> {
        self.0
            .lock()
            .unwrap_or_else(PoisonError::into_inner)
            .get(label)
            .cloned()
    }
}

/// The mover's journal for the mover's own writes, and one journal in each holding folder for
/// the writes into it.
pub struct SplitLedger<'a> {
    mine: &'a dyn Ledger,
    places: HeldPlaces,
    /// Each holding place's journal, opened at its first write, by its label.
    held: Mutex<Vec<(String, Journal)>>,
}

impl<'a> SplitLedger<'a> {
    pub fn new(mine: &'a dyn Ledger, places: HeldPlaces) -> Self {
        Self {
            mine,
            places,
            held: Mutex::new(Vec::new()),
        }
    }

    /// The journal for the holding place `label`, opened in its folder at its first write.
    fn place_for(&self, label: &str) -> Result<Option<usize>, JournalError> {
        let Some(folder) = self.places.folder(label) else {
            return Ok(None);
        };
        let mut held = self.held.lock().unwrap_or_else(PoisonError::into_inner);
        if let Some(n) = held.iter().position(|(l, _)| l == label) {
            return Ok(Some(n));
        }
        if held.len() + 1 >= 1 << (64 - SHIFT) {
            return Err(JournalError::Damaged("too many holding places".into()));
        }
        held.push((
            label.to_string(),
            Journal::open(&folder.join(HELD_JOURNAL))?,
        ));
        Ok(Some(held.len() - 1))
    }

    /// Runs `f` on the journal entry `id` belongs to, with its number there.
    fn on<R>(
        &self,
        id: u64,
        f: impl FnOnce(&dyn Ledger, u64) -> Result<R, JournalError>,
    ) -> Result<R, JournalError> {
        let (place, inner) = split(id);
        match place {
            None => f(self.mine, inner),
            Some(n) => {
                let held = self.held.lock().unwrap_or_else(PoisonError::into_inner);
                let (_, journal) = held
                    .get(n)
                    .ok_or_else(|| JournalError::Damaged("no such entry".into()))?;
                f(journal, inner)
            }
        }
    }
}

/// Entry `inner` of holding place `n` as the receiver sees it.
fn joined(n: usize, inner: u64) -> Result<u64, JournalError> {
    if inner >> SHIFT != 0 {
        return Err(JournalError::Damaged(
            "too many entries in one journal".into(),
        ));
    }
    Ok(((n as u64 + 1) << SHIFT) | inner)
}

/// Which journal an entry number belongs to (`None`: the mover's), and its number there.
fn split(id: u64) -> (Option<usize>, u64) {
    match id >> SHIFT {
        0 => (None, id),
        n => (Some((n - 1) as usize), id & ((1 << SHIFT) - 1)),
    }
}

impl Ledger for SplitLedger<'_> {
    fn temp_tag(&self, id: u64) -> String {
        self.on(id, |l, i| Ok(l.temp_tag(i)))
            .unwrap_or_else(|_| self.mine.temp_tag(id))
    }
    fn plan(&self, write: &PlannedWrite) -> Result<u64, JournalError> {
        match self.place_for(&write.destination)? {
            None => {
                let id = self.mine.plan(write)?;
                if id >> SHIFT != 0 {
                    return Err(JournalError::Damaged(
                        "too many entries in one journal".into(),
                    ));
                }
                Ok(id)
            }
            Some(n) => {
                let inner = self.on(joined(n, 0)?, |l, _| l.plan(write))?;
                joined(n, inner)
            }
        }
    }
    fn staged(
        &self,
        id: u64,
        temp: &str,
        made: &[(String, Option<FileId>)],
    ) -> Result<(), JournalError> {
        self.on(id, |l, i| l.staged(i, temp, made))
    }
    fn verified(
        &self,
        id: u64,
        fingerprint: [u8; 32],
        file: Option<FileId>,
    ) -> Result<(), JournalError> {
        self.on(id, |l, i| l.verified(i, fingerprint, file))
    }
    fn verified_as(
        &self,
        id: u64,
        fingerprint: [u8; 32],
        file: Option<FileId>,
        checked: Option<CheckedFrom>,
    ) -> Result<(), JournalError> {
        self.on(id, |l, i| l.verified_as(i, fingerprint, file, checked))
    }
    fn applied(&self, id: u64, final_path: &str) -> Result<(), JournalError> {
        self.on(id, |l, i| l.applied(i, final_path))
    }
    fn filling(
        &self,
        id: u64,
        final_path: &str,
        tag: FillTag,
        proof: Proof,
    ) -> Result<(), JournalError> {
        self.on(id, |l, i| l.filling(i, final_path, tag, proof))
    }
    fn fill_made(&self, id: u64, made: FileId) -> Result<(), JournalError> {
        self.on(id, |l, i| l.fill_made(i, made))
    }
    fn filled(&self, id: u64) -> Result<(), JournalError> {
        self.on(id, |l, i| l.filled(i))
    }
    fn fill_cleared(&self, id: u64, left: Option<&str>) -> Result<(), JournalError> {
        self.on(id, |l, i| l.fill_cleared(i, left))
    }
    fn committed_showing(&self, id: u64, landed: Landed, show: &Show) -> Result<(), JournalError> {
        self.on(id, |l, i| l.committed_showing(i, landed, show))
    }
    fn shown(&self, id: u64, left: Option<&str>, file: Option<FileId>) -> Result<(), JournalError> {
        self.on(id, |l, i| l.shown(i, left, file))
    }
    fn committed(&self, id: u64, landed: Landed) -> Result<(), JournalError> {
        self.on(id, |l, i| l.committed(i, landed))
    }
    fn existing(&self, id: u64, stored_path: &str) -> Result<(), JournalError> {
        self.on(id, |l, i| l.existing(i, stored_path))
    }
    fn failed(&self, id: u64, why: &str) -> Result<(), JournalError> {
        self.on(id, |l, i| l.failed(i, why))
    }
    fn kept_aside(
        &self,
        id: u64,
        why: &str,
        kept_path: &str,
        fingerprint: [u8; 32],
        landed: Landed,
    ) -> Result<(), JournalError> {
        self.on(id, |l, i| {
            l.kept_aside(i, why, kept_path, fingerprint, landed)
        })
    }
    fn checkpoint(
        &self,
        id: u64,
        blocks: &[(u64, [u8; 32])],
        durable: bool,
    ) -> Result<(), JournalError> {
        self.on(id, |l, i| l.checkpoint(i, blocks, durable))
    }
    fn blocks(&self, id: u64) -> Result<Vec<(u64, [u8; 32])>, JournalError> {
        self.on(id, |l, i| l.blocks(i))
    }
    fn unfinished(&self) -> Result<Vec<Entry>, JournalError> {
        let mut all = self.mine.unfinished()?;
        let held = self.held.lock().unwrap_or_else(PoisonError::into_inner);
        for (n, (_, journal)) in held.iter().enumerate() {
            for mut e in Ledger::unfinished(journal)? {
                e.id = joined(n, e.id)?;
                all.push(e);
            }
        }
        Ok(all)
    }
}

/// One holding place of a move, as the new laptop remembers it to pick its writes up after a
/// restart: its destination label, the account it is held for, and its folder. None of it names
/// a file (the folder is the helper's `<staging>/<random>/p<n>`), so it may be kept in the mover's
/// own app data.
#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
#[serde(deny_unknown_fields)]
pub struct HeldPlace {
    pub label: String,
    pub account: String,
    pub folder: PathBuf,
}

/// What picking up the holding places' writes came to, counted only (their names stay in their
/// journals).
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct HeldTidied {
    /// Finished and proven copies now in place in a holding folder.
    pub put_in_place: u64,
    /// Part-copied files kept.
    pub kept_to_continue: u64,
    /// Writes that could not be finished (their temporary files removed).
    pub not_finished: u64,
    /// Writes left as they were, because their place could not be looked at.
    pub left_for_later: u64,
    /// Holding places left to their hand-over: the mover may no longer open them (they were
    /// handed over or locked for it), or they are gone.
    pub left_to_hand_over: u64,
}

fn count(n: usize) -> u64 {
    u64::try_from(n).unwrap_or(u64::MAX)
}

/// How a holding place's journal is reached now.
enum Reach {
    /// Its journal, to pick up from.
    Open(Journal),
    /// Nothing was ever written there, or the folder is gone (emptied by its hand-over).
    Nothing,
    /// The mover may not open it any more (it was handed over or locked for it), or another
    /// PCTwin has it open: left to the hand-over.
    Left,
}

fn reach(place: &HeldPlace) -> Reach {
    let at = place.folder.join(HELD_JOURNAL);
    match std::fs::symlink_metadata(&at) {
        Ok(meta) if meta.is_file() => match Journal::open(&at) {
            Ok(journal) => Reach::Open(journal),
            Err(_) => Reach::Left,
        },
        // A link or anything else at the journal's name is never opened.
        Ok(_) => Reach::Left,
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => Reach::Nothing,
        // Not even looked at: the folder is closed to the mover now.
        Err(_) => Reach::Left,
    }
}

/// How many writes the holding places' journals have not finished, of those the mover may still
/// open (a count only).
pub fn unfinished_held(places: &[HeldPlace]) -> u64 {
    places
        .iter()
        .map(|p| match reach(p) {
            Reach::Open(journal) => count(
                journal.unfinished().map_or(0, |u| u.len())
                    + journal.unshown().map_or(0, |u| u.len()),
            ),
            _ => 0,
        })
        .sum()
}

/// Picks up what a stopped move left in each holding place the mover may still open, by the
/// same rules as the mover's own journal (pctwin-transfer's recovery): a finished and proven copy
/// is named, a part-copied one kept, anything else failed and its temporary file removed, and
/// writing only into that holding folder. A place the mover may no longer open is left to its
/// hand-over, which never hands over a temporary file. Only counts come back: the names and the
/// reasons, which name files, stay in the person's journal.
pub fn recover_held(places: &[HeldPlace]) -> HeldTidied {
    let mut done = HeldTidied::default();
    for place in places {
        let journal = match reach(place) {
            Reach::Open(journal) => journal,
            Reach::Nothing => continue,
            Reach::Left => {
                done.left_to_hand_over += 1;
                continue;
            }
        };
        let mut table = pctwin_gate::Destinations::new();
        if table
            .approve_holding(&place.label, &place.account, &place.folder)
            .is_err()
        {
            done.left_to_hand_over += 1;
            continue;
        }
        match pctwin_transfer::recover(&journal, &table) {
            Ok(r) => {
                done.put_in_place += count(r.committed.len() + r.shown.len());
                done.kept_to_continue += count(r.resumable.len());
                done.not_finished += count(r.failed.len());
                done.left_for_later += count(r.left.len());
            }
            Err(_) => done.left_to_hand_over += 1,
        }
    }
    done
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn entry_numbers_say_whose_journal_they_are_in() {
        assert_eq!(split(7), (None, 7));
        let id = joined(0, 7).unwrap();
        assert_ne!(id, 7);
        assert_eq!(split(id), (Some(0), 7));
        assert_eq!(split(joined(3, 1).unwrap()), (Some(3), 1));
        assert!(joined(0, 1 << SHIFT).is_err());
    }
}
