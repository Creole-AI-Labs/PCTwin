//! Changes during the move (Task List 2.9): when an app updates, a destination folder changes or
//! a project is edited after the person approved, only the affected steps are held, checked
//! again, and asked about again; everything else carries on.
//!
//! Contract only (stub): what the screens send and get. The real rules follow in their own
//! commit.

use serde::Serialize;

pctwin_types::listed_enum! {
    /// What changed after the approval.
    #[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
    #[serde(rename_all = "kebab-case")]
    pub enum AfterWhy {
        /// An app on this laptop was updated, or installed, since the approval.
        AppUpdated,
        /// An app on this laptop was removed since the approval.
        AppRemoved,
        /// A folder things go to was moved, removed or replaced.
        DestinationChanged,
        /// A project or file was edited on the old laptop.
        ProjectEdited,
    }
}

pctwin_types::listed_enum! {
    /// The steps held for it, to be checked again.
    #[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize)]
    #[serde(rename_all = "kebab-case")]
    pub enum Recheck {
        Install,
        Settings,
        AppData,
        Files,
    }
}

/// One thing that changed after approval.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ChangedAfter {
    /// The app, folder or file, shown exactly as it is.
    pub name: String,
    pub why: AfterWhy,
    pub steps: Vec<Recheck>,
}

/// What changed after approval: only these are asked about again.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ChangesView {
    /// The fresh approval's version: an answer for another version is refused.
    pub generation: u64,
    pub lines: Vec<ChangedAfter>,
    pub more: u64,
    /// Steps held until the person answers.
    pub held: u64,
}

/// The event sent when something changed after approval.
// Sent once the real rules are wired (next commits).
#[allow(dead_code)]
pub const EVENT: &str = "move-changed";

/// What changed after approval and waits for the person (`None`: nothing).
#[tauri::command]
pub fn move_changes() -> Option<ChangesView> {
    None
}

/// The person's fresh answer for the held steps of `generation`: go ahead with them as they are
/// now (`approve`), or leave them out.
#[tauri::command]
pub fn move_reapprove(generation: u64, approve: bool) -> Result<(), crate::moving::MoveProblem> {
    let _ = (generation, approve);
    Err(crate::moving::MoveProblem::NotMoving)
}
