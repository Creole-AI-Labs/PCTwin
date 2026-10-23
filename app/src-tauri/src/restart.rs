//! "Waiting for a restart" (Product Spec, "Restarts and updates"): while a move is under way,
//! PCTwin opens by itself when the person signs in, so after a restart (an update, the power, a
//! crash) the move is picked up from the start page: Resume your move, which tidies up safely
//! (X09) and pairs again (an approval lasts one session, Security Design part C). Once there is
//! nothing left to pick up, PCTwin no longer opens by itself.
//!
//! Through Tauri's own autostart plugin: the system's sign-in list for this account (Windows, the
//! Run key; a Mac, a launch agent; Linux, an autostart entry). Best effort: if the system refuses,
//! the move is no less safe, and the person opens PCTwin themselves.

use tauri::plugin::TauriPlugin;
use tauri::{AppHandle, Runtime};
use tauri_plugin_autostart::{MacosLauncher, ManagerExt};

/// The plugin, with nothing the screens may call (no capability grants its commands).
pub fn plugin<R: Runtime>() -> TauriPlugin<R> {
    tauri_plugin_autostart::init(MacosLauncher::LaunchAgent, None)
}

/// A move has begun: PCTwin opens at the next sign-in until it is over.
pub fn while_moving<R: Runtime>(app: &AppHandle<R>) {
    let _ = app.autolaunch().enable();
}

/// Nothing is left to pick up: PCTwin no longer opens by itself.
pub fn nothing_left<R: Runtime>(app: &AppHandle<R>) {
    if app.autolaunch().is_enabled().unwrap_or(true) {
        let _ = app.autolaunch().disable();
    }
}
