//! The offline recovery kit (Task List 2.9; Engineering Plan A02): the move record kept, readable
//! with no account, and saying exactly what it holds and what it does not.
//!
//! Contract only (stub): what the screens send and get. The real kit follows in its own commit.

use serde::Serialize;

use crate::moving::MoveProblem;

pctwin_types::listed_enum! {
    /// One part a kit holds, or does not.
    #[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
    #[serde(rename_all = "kebab-case")]
    pub enum KitPart {
        /// What was picked to move, and where each thing went.
        MoveRecord,
        /// The result of every item, and how each was checked.
        Results,
        /// What is still only on the old laptop.
        BeforeWipeList,
        /// The files themselves (never in a kit: instructions cannot bring back missing files).
        FileData,
        /// The steps to set apps and settings up again.
        SetupSteps,
        /// A tool that restores anything.
        RecoveryTool,
    }
}

/// A kit saved where the person chose.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct KitSaved {
    /// The folder, shown exactly as it is.
    pub folder: String,
    /// The files written there, by name.
    pub files: Vec<String>,
    pub holds: Vec<KitPart>,
    pub lacks: Vec<KitPart>,
}

/// Saves the recovery kit of the move `id` (an ID from My PCTwin history) in a folder the person
/// picks. `None`: they picked nowhere.
#[tauri::command]
pub async fn recovery_kit_save(id: String) -> Result<Option<KitSaved>, MoveProblem> {
    let _ = id;
    Err(MoveProblem::Failed)
}
