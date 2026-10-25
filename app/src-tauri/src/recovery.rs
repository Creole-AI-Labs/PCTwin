//! Picking up safely after a restart (X09). The new laptop's journal records every write before
//! and as it happens; if PCTwin stopped part way (a crash, a restart, the power), some writes are
//! unfinished. At start the screens ask whether there are any; the person then has them tidied
//! up: a copy that was finished and proven is put in place, a part-copied file is kept to carry on,
//! and the journal's own temporary files are removed. Nothing the person has is touched, and every
//! step is the engine's own recovery (`pctwin_transfer::recover`), safe to run again.
//!
//! The screens get counts only: the journal holds file names, and none is shown.
//!
//! Other people's writes are journaled in their holding folders (`pctwin_move::held_journal`).
//! The holding places of each move are remembered here (labels, accounts and folders, never a
//! file name), and every one the mover may still open is picked up by the same rules; one the
//! mover may no longer open (handed over, or locked for it) is left to its hand-over, which never
//! hands over an unfinished file.

use serde::Serialize;
use tauri::{AppHandle, Runtime};

use crate::moving::{MoveProblem, journal_path, this_laptops_places};

/// What is left from a move that stopped part way.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Interrupted {
    /// Writes the journal has not finished: none means there is nothing to pick up.
    pub unfinished: u64,
}

/// What tidying up did, counted.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Tidied {
    /// Finished and proven copies now in place.
    pub put_in_place: u64,
    /// Part-copied files kept, to carry on from when the laptops are paired again.
    pub kept_to_continue: u64,
    /// Writes that could not be finished; the next move copies them again.
    pub not_finished: u64,
    /// Left as they were, because their place could not be looked at now (a drive unplugged).
    pub left_for_later: u64,
}

fn count(n: usize) -> u64 {
    u64::try_from(n).unwrap_or(u64::MAX)
}

impl Tidied {
    /// This, with what picking up the holding places did.
    fn and_held(self, held: pctwin_move::held_journal::HeldTidied) -> Self {
        Self {
            put_in_place: self.put_in_place.saturating_add(held.put_in_place),
            kept_to_continue: self.kept_to_continue.saturating_add(held.kept_to_continue),
            not_finished: self.not_finished.saturating_add(held.not_finished),
            left_for_later: self
                .left_for_later
                .saturating_add(held.left_for_later)
                .saturating_add(held.left_to_hand_over),
        }
    }
}

/// Where the holding places of the moves on this laptop are remembered.
fn held_places_path<R: Runtime>(app: &AppHandle<R>) -> Option<std::path::PathBuf> {
    journal_path(app).map(|j| j.with_file_name("held-places.json"))
}

/// The holding places remembered at `path` (none if there is no file, or it is not PCTwin's).
fn read_held(path: &std::path::Path) -> Vec<pctwin_move::held_journal::HeldPlace> {
    std::fs::read(path)
        .ok()
        .filter(|b| b.len() <= 1 << 20)
        .and_then(|b| serde_json::from_slice(&b).ok())
        .unwrap_or_default()
}

/// Writes the holding places `places` to `path` whole (a new file, then renamed into place).
fn write_held(
    path: &std::path::Path,
    places: &[pctwin_move::held_journal::HeldPlace],
) -> std::io::Result<()> {
    if let Some(dir) = path.parent() {
        std::fs::create_dir_all(dir)?;
    }
    let tmp = path.with_extension("json.part");
    std::fs::write(
        &tmp,
        serde_json::to_vec(places).map_err(std::io::Error::other)?,
    )?;
    std::fs::rename(&tmp, path)
}

/// Remembers a holding place before anything lands in it, so a restart picks its writes up.
pub(crate) fn remember_held<R: Runtime>(
    app: &AppHandle<R>,
    place: pctwin_move::held_journal::HeldPlace,
) -> Result<(), MoveProblem> {
    let path = held_places_path(app).ok_or(MoveProblem::Failed)?;
    remember_at(&path, place).map_err(|_| MoveProblem::Failed)
}

fn remember_at(
    path: &std::path::Path,
    place: pctwin_move::held_journal::HeldPlace,
) -> std::io::Result<()> {
    let mut places = read_held(path);
    if !places.contains(&place) {
        places.push(place);
    }
    write_held(path, &places)
}

/// Forgets the holding places whose folder is gone (handed over and emptied).
fn forget_gone(path: &std::path::Path) {
    let places = read_held(path);
    let kept: Vec<_> = places
        .iter()
        .filter(|p| {
            !matches!(std::fs::symlink_metadata(&p.folder),
                Err(e) if e.kind() == std::io::ErrorKind::NotFound)
        })
        .cloned()
        .collect();
    if kept.len() != places.len() {
        let _ = write_held(path, &kept);
    }
}

impl From<&pctwin_transfer::Recovered> for Tidied {
    fn from(r: &pctwin_transfer::Recovered) -> Self {
        Self {
            put_in_place: count(r.committed.len() + r.shown.len()),
            kept_to_continue: count(r.resumable.len()),
            not_finished: count(r.failed.len()),
            left_for_later: count(r.left.len()),
        }
    }
}

/// Whether a move stopped part way on this laptop. No journal yet: nothing to pick up (and none is
/// made by asking).
#[tauri::command]
pub fn recovery_check<R: Runtime>(app: AppHandle<R>) -> Result<Interrupted, MoveProblem> {
    let path = journal_path(&app).ok_or(MoveProblem::Failed)?;
    let held = held_places_path(&app)
        .map(|p| read_held(&p))
        .unwrap_or_default();
    let in_held = pctwin_move::held_journal::unfinished_held(&held);
    if !path.exists() {
        if in_held == 0 {
            crate::restart::nothing_left(&app);
        }
        return Ok(Interrupted {
            unfinished: in_held,
        });
    }
    let journal = pctwin_journal::Journal::open(&path).map_err(|_| MoveProblem::Busy)?;
    let unfinished = count(
        journal.unfinished().map_err(|_| MoveProblem::Failed)?.len()
            + journal.unshown().map_err(|_| MoveProblem::Failed)?.len(),
    )
    .saturating_add(in_held);
    // Nothing to pick up: PCTwin need not open by itself at sign-in any more.
    if unfinished == 0 {
        crate::restart::nothing_left(&app);
    }
    Ok(Interrupted { unfinished })
}

/// Tidies up what a move left part way, into this laptop's own places only.
#[tauri::command]
pub async fn recovery_run<R: Runtime>(app: AppHandle<R>) -> Result<Tidied, MoveProblem> {
    let path = journal_path(&app).ok_or(MoveProblem::Failed)?;
    let held_path = held_places_path(&app).ok_or(MoveProblem::Failed)?;
    // Reading back files is slow work: off the screens' thread.
    tauri::async_runtime::spawn_blocking(move || {
        let mine = if path.exists() {
            let journal = pctwin_journal::Journal::open(&path).map_err(|_| MoveProblem::Busy)?;
            let (table, _) = this_laptops_places()?;
            let done =
                pctwin_transfer::recover(&journal, &table).map_err(|_| MoveProblem::Failed)?;
            Tidied::from(&done)
        } else {
            Tidied::default()
        };
        // Other people's writes, in each holding place the mover may still open; counts only.
        let held = pctwin_move::held_journal::recover_held(&read_held(&held_path));
        forget_gone(&held_path);
        Ok(mine.and_held(held))
    })
    .await
    .map_err(|_| MoveProblem::Failed)?
}

/// "Your things are ready" (M32): the note a hand-over left for the person signed in here, read
/// once and removed, so it is shown once. `None`: there is none.
#[tauri::command]
pub fn things_ready<R: Runtime>(app: AppHandle<R>) -> Option<pctwin_move::handover::ReadyNote> {
    let path = journal_path(&app)?.with_file_name("things-ready.json");
    let bytes = std::fs::read(&path).ok()?;
    let _ = std::fs::remove_file(&path);
    pctwin_move::handover::ReadyNote::read(&bytes)
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Holding places are remembered once each, read back whole, and forgotten once gone.
    #[test]
    fn holding_places_are_remembered_and_forgotten_when_gone() {
        let dir = tempfile::tempdir().unwrap();
        let at = dir.path().join("held-places.json");
        let folder = dir.path().join("p1");
        std::fs::create_dir_all(&folder).unwrap();
        let place = pctwin_move::held_journal::HeldPlace {
            label: "for-1".into(),
            account: "1001".into(),
            folder: folder.clone(),
        };
        remember_at(&at, place.clone()).unwrap();
        remember_at(&at, place.clone()).unwrap();
        assert_eq!(read_held(&at), vec![place.clone()]);
        forget_gone(&at);
        assert_eq!(read_held(&at), vec![place]);
        std::fs::remove_dir(&folder).unwrap();
        forget_gone(&at);
        assert!(read_held(&at).is_empty());
        // Anything else at the name is no list at all.
        std::fs::write(&at, b"not json").unwrap();
        assert!(read_held(&at).is_empty());
    }

    #[test]
    fn only_counts_reach_the_screens() {
        let r = pctwin_transfer::Recovered {
            committed: vec![1, 2],
            failed: vec![(3, "C:/Users/Bea/secret.txt could not be read".into())],
            resumable: vec![4],
            left: vec![(5, "D:/Photos is not plugged in".into())],
            notes: vec![(6, "left at C:/Users/Bea/x".into())],
            shown: vec![7],
            removed: 9,
        };
        let tidied = Tidied::from(&r);
        assert_eq!(
            tidied,
            Tidied {
                put_in_place: 3,
                kept_to_continue: 1,
                not_finished: 1,
                left_for_later: 1,
            }
        );
        let text = serde_json::to_string(&tidied).unwrap();
        assert!(!text.contains("Bea") && !text.contains("Photos"), "{text}");
    }
}
