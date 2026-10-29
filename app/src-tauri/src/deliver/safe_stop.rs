//! PCTwin's own update and uninstall wait for the engine's safe stopping points (Task List 2.9,
//! "Changes during the move").
//!
//! Contract only (stub): the state and the view. The real gate follows in its own commit.

use serde::Serialize;

pctwin_types::listed_enum! {
    /// What PCTwin is in the middle of, that an update or uninstall waits for.
    #[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize)]
    #[serde(rename_all = "kebab-case")]
    pub enum BusyWith {
        /// Moving files (stopping the move is a safe point: what is copied and checked stays).
        Moving,
        /// Installing apps and putting settings in place.
        Rebuilding,
        /// Reading or placing app data.
        AppData,
        /// Undoing a move.
        Undoing,
    }
}

/// Whether PCTwin may be updated or removed now, and if not, what it waits for.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct SafeStopView {
    pub safe: bool,
    pub busy: Vec<BusyWith>,
}

/// The event sent when PCTwin reaches, or leaves, a safe stopping point.
// Sent once the real rules are wired (next commits).
#[allow(dead_code)]
pub const EVENT: &str = "safe-stop";

/// Whether PCTwin may be updated or removed now.
#[tauri::command]
pub fn safe_stop() -> SafeStopView {
    SafeStopView {
        safe: true,
        busy: Vec::new(),
    }
}
