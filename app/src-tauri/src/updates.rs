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
///
/// The download can take minutes, and a move, a rebuild or an undo may start meanwhile: so it is
/// only downloaded first, and installed only if PCTwin is still at a safe point right then, with
/// every step kept from starting from that moment on ([`install_when_safe`]).
#[tauri::command]
pub async fn update_install<R: Runtime>(app: AppHandle<R>) -> Result<(), MoveProblem> {
    let gate = crate::deliver::safe_stop::gate();
    let underway = || app.state::<AppMoving>().underway();
    if !may_update(underway(), &gate.view()) {
        return Err(MoveProblem::Busy);
    }
    let update = app
        .updater()
        .map_err(|_| MoveProblem::Failed)?
        .check()
        .await
        .map_err(|_| MoveProblem::Failed)?
        .ok_or(MoveProblem::NotMoving)?;
    // The signature is checked here, and again just before anything is installed.
    let bytes = update
        .download(|_, _| {}, || {})
        .await
        .map_err(|_| MoveProblem::Failed)?;
    let _staged = install_when_safe(gate, underway, || {
        update.install(&bytes).map_err(|_| MoveProblem::Failed)
    })?;
    // Windows: the installer has taken over and this app has already closed. Elsewhere: no step
    // starts before the new build opens.
    app.restart()
}

/// Installs (`install`) only if nothing is going on right now (`underway`: a move), keeping every
/// step from starting from the check on: the hold is kept by the caller until PCTwin restarts,
/// and let go if the install fails.
fn install_when_safe(
    gate: &'static crate::deliver::safe_stop::SafeStop,
    underway: impl Fn() -> bool,
    install: impl FnOnce() -> Result<(), MoveProblem>,
) -> Result<crate::deliver::safe_stop::Staged, MoveProblem> {
    let staged = gate.begin_update().ok_or(MoveProblem::Busy)?;
    if underway() {
        return Err(MoveProblem::Busy);
    }
    install()?;
    Ok(staged)
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

    /// Fresh-context review F2: the gate is checked again right before the install, and no step
    /// can start from then on; a step that started during the download stops the install, and a
    /// failed install lets steps start again.
    #[test]
    fn the_install_checks_again_and_keeps_steps_from_starting() {
        use crate::deliver::safe_stop::SafeStop;
        let gate: &'static SafeStop = Box::leak(Box::new(SafeStop::new()));
        // A rebuild started while the update downloaded: nothing is installed.
        let step = gate.hold(BusyWith::Rebuilding);
        let mut ran = false;
        let r = install_when_safe(
            gate,
            || false,
            || {
                ran = true;
                Ok(())
            },
        );
        assert!(matches!(r, Err(MoveProblem::Busy)));
        assert!(!ran);
        drop(step);
        // A move under way: nothing is installed, and steps may start again.
        let r = install_when_safe(gate, || true, || Ok(()));
        assert!(matches!(r, Err(MoveProblem::Busy)));
        assert!(gate.try_hold(BusyWith::Moving).is_some());
        // While installing, no step can start; after a failure they can.
        let r = install_when_safe(
            gate,
            || false,
            || {
                assert!(gate.try_hold(BusyWith::Undoing).is_none());
                Err(MoveProblem::Failed)
            },
        );
        assert!(matches!(r, Err(MoveProblem::Failed)));
        assert!(gate.try_hold(BusyWith::Undoing).is_some());
        // A good install keeps steps from starting until PCTwin restarts.
        let staged = install_when_safe(gate, || false, || Ok(())).ok();
        assert!(staged.is_some());
        assert!(gate.try_hold(BusyWith::Undoing).is_none());
        drop(staged);
    }
}
