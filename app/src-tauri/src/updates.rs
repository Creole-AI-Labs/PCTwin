//! Updates: when PCTwin opens, it asks the update address in tauri.conf.json whether a newer
//! build is out, and the start page offers it. Installing downloads the new build, checks its
//! signature against the update key built into this app (tauri.conf.json `plugins.updater.pubkey`;
//! anything not signed with our key is refused), installs it and opens PCTwin again.
//!
//! The screens never reach the updater itself: they may only ask whether there is an update and
//! ask for it to be installed, here. Never during a move: a move picks up after a restart, but an
//! update is not worth breaking one for.
//!
//! For internal test builds (decided 26 October 2026). Before the public launch the same flow is
//! wrapped in TUF (Security Design part E): signed metadata with expiry and rollback protection.

use serde::Serialize;
use tauri::{AppHandle, Manager, Runtime};
use tauri_plugin_updater::UpdaterExt;

use crate::moving::{AppMoving, MoveProblem};

/// The plugin, with nothing the screens may call (no capability grants its commands).
pub fn plugin<R: Runtime>() -> tauri::plugin::TauriPlugin<R, tauri_plugin_updater::Config> {
    tauri_plugin_updater::Builder::new().build()
}

/// A newer build, as the start page shows it.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct UpdateView {
    pub version: String,
}

/// Whether a newer build is out. `None` when there is none, or when it cannot be told (no
/// network, the address cannot be reached, no build for this system): the app simply carries on.
#[tauri::command]
pub async fn update_check<R: Runtime>(app: AppHandle<R>) -> Option<UpdateView> {
    let update = app.updater().ok()?.check().await.ok()??;
    Some(UpdateView {
        version: update.version,
    })
}

/// Downloads the newer build, checks its signature, installs it and opens PCTwin again. Refused
/// while a move is under way or PCTwin is in the middle of a step that must not be cut part way
/// (rebuilding, app data, an undo: `Busy`; the screens say what it waits for with `safe_stop`);
/// `NotMoving` when there is no update after all.
#[tauri::command]
pub async fn update_install<R: Runtime>(app: AppHandle<R>) -> Result<(), MoveProblem> {
    if !may_update(
        app.state::<AppMoving>().underway(),
        &crate::deliver::safe_stop::gate().view(),
    ) {
        return Err(MoveProblem::Busy);
    }
    let update = app
        .updater()
        .map_err(|_| MoveProblem::Failed)?
        .check()
        .await
        .map_err(|_| MoveProblem::Failed)?
        .ok_or(MoveProblem::NotMoving)?;
    // The signature is checked before anything is installed; a bad one fails here.
    update
        .download_and_install(|_, _| {}, || {})
        .await
        .map_err(|_| MoveProblem::Failed)?;
    app.restart()
}

/// Whether an update may start: no move under way, and PCTwin at a safe stopping point.
fn may_update(underway: bool, safe: &crate::deliver::safe_stop::SafeStopView) -> bool {
    !underway && safe.safe
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::deliver::safe_stop::{BusyWith, SafeStopView};

    #[test]
    fn an_update_waits_for_the_move_and_every_step_that_must_not_be_cut() {
        let safe = SafeStopView {
            safe: true,
            busy: vec![],
        };
        let undoing = SafeStopView {
            safe: false,
            busy: vec![BusyWith::Undoing],
        };
        assert!(may_update(false, &safe));
        assert!(!may_update(true, &safe));
        assert!(!may_update(false, &undoing));
        assert!(!may_update(true, &undoing));
    }
}
