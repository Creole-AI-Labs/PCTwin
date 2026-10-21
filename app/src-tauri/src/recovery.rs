//! Picking up safely after a restart (X09). The new laptop's journal records every write before
//! and as it happens; if PCTwin stopped part way (a crash, a restart, the power), some writes are
//! unfinished. At start the screens ask whether there are any; the person then has them tidied
//! up: a copy that was finished and proven is put in place, a part-copied file is kept to carry on,
//! and the journal's own temporary files are removed. Nothing the person has is touched, and every
//! step is the engine's own recovery (`pctwin_transfer::recover`), safe to run again.
//!
//! The screens get counts only: the journal holds file names, other people's among them, and none
//! is shown.

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
    if !path.exists() {
        return Ok(Interrupted { unfinished: 0 });
    }
    let journal = pctwin_journal::Journal::open(&path).map_err(|_| MoveProblem::Busy)?;
    let unfinished = journal.unfinished().map_err(|_| MoveProblem::Failed)?.len()
        + journal.unshown().map_err(|_| MoveProblem::Failed)?.len();
    Ok(Interrupted {
        unfinished: count(unfinished),
    })
}

/// Tidies up what a move left part way, into this laptop's own places only.
#[tauri::command]
pub async fn recovery_run<R: Runtime>(app: AppHandle<R>) -> Result<Tidied, MoveProblem> {
    let path = journal_path(&app).ok_or(MoveProblem::Failed)?;
    if !path.exists() {
        return Ok(Tidied::default());
    }
    // Reading back files is slow work: off the screens' thread.
    tauri::async_runtime::spawn_blocking(move || {
        let journal = pctwin_journal::Journal::open(&path).map_err(|_| MoveProblem::Busy)?;
        let (table, _) = this_laptops_places()?;
        let done = pctwin_transfer::recover(&journal, &table).map_err(|_| MoveProblem::Failed)?;
        Ok(Tidied::from(&done))
    })
    .await
    .map_err(|_| MoveProblem::Failed)?
}

#[cfg(test)]
mod tests {
    use super::*;

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
