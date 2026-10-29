//! "Can I actually work?" checks on the screens (Task List 2.9; Product Spec R17).
//!
//! Contract only (stub): what the screens send and get. The checks themselves are
//! `pctwin_readiness::checks`; the real wiring follows in its own commit.

use serde::{Deserialize, Serialize};

/// What the person adds a check for.
#[derive(Debug, Clone, PartialEq, Eq, Deserialize)]
#[serde(tag = "kind", rename_all = "kebab-case", deny_unknown_fields)]
pub enum WorkPick {
    /// A file they work with, picked in the system's own file picker: a project whose linked
    /// files and fonts are looked up, an archive whose index is read, or a database whose header
    /// is read (told by its content, never its name).
    File {},
    /// A font they need, by its family name.
    Font { family: String },
    /// A task they do themselves and say whether it worked.
    Task { task: String },
}

pctwin_types::listed_enum! {
    /// What a check looks at.
    #[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
    #[serde(rename_all = "kebab-case")]
    pub enum WorkWhat {
        /// A project's linked files.
        LinkedFiles,
        /// A font.
        Font,
        /// An archive opens.
        Archive,
        /// A database opens.
        Database,
        /// A task the person does.
        Task,
    }
}

pctwin_types::listed_enum! {
    /// The three labels every result carries.
    #[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
    #[serde(rename_all = "kebab-case")]
    pub enum WorkLabel {
        MachineChecked,
        PersonConfirmed,
        StillNeedsAttention,
    }
}

pctwin_types::listed_enum! {
    /// Where one check got to.
    #[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
    #[serde(rename_all = "kebab-case")]
    pub enum WorkState {
        /// Added, not run yet.
        NotRunYet,
        /// Everything it looked at is there and whole.
        AllThere,
        /// Something it needs is missing (named in `missing`).
        Missing,
        /// The file is not whole, or not what it should be.
        Damaged,
        /// The file is not there at all.
        NotThere,
        /// Waits for the person to try it and say.
        WaitingForYou,
        /// The person said it did not work.
        YouSaidNo,
        /// It links to files outside the project's folder: not looked up.
        OutsideProject,
        /// The file could not be read (no permission, say).
        CouldNotRead,
        /// PCTwin cannot check this kind of file yet: found, not yet supported.
        NotSupported,
    }
}

/// One check and its latest result.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct WorkCheck {
    /// Its place in the list (for answering or removing it).
    pub index: u32,
    pub what: WorkWhat,
    /// The file, font or task, shown exactly as it is.
    pub name: String,
    pub label: WorkLabel,
    pub state: WorkState,
    /// What is missing (file or font names, shown exactly as they are), bounded.
    pub missing: Vec<String>,
    pub missing_more: u64,
    /// Kinds of file found and not checked (a font file PCTwin cannot read the names of).
    pub not_checked: Vec<String>,
    /// How much was looked at: `checked` of `of` (equal when all of it).
    pub checked: u64,
    pub of: u64,
}

/// The person's checks.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct WorkChecks {
    /// The list's version: an answer or removal for another version is refused.
    pub generation: u64,
    pub checks: Vec<WorkCheck>,
    pub more: u64,
}

pctwin_types::listed_enum! {
    /// Why a check could not be added, answered or removed.
    #[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
    #[serde(rename_all = "kebab-case")]
    pub enum WorkProblem {
        /// The person chose no file in the picker.
        NotChosen,
        /// Too many checks at once.
        TooMany,
        /// No such check.
        NoSuchCheck,
        /// The list changed since the screens saw it.
        Stale,
        /// A font or task name that is empty or too long.
        BadName,
        /// The answer is for another kind of check.
        WrongKind,
        Failed,
    }
}

fn empty() -> WorkChecks {
    WorkChecks {
        generation: 0,
        checks: Vec::new(),
        more: 0,
    }
}

/// The person's checks, with their latest results.
#[tauri::command]
pub fn work_checks() -> WorkChecks {
    empty()
}

/// Adds a check.
#[tauri::command]
pub async fn work_check_add(pick: WorkPick) -> Result<WorkChecks, WorkProblem> {
    let _ = pick;
    Err(WorkProblem::Failed)
}

/// Runs every check that PCTwin can do by reading (never running anything).
#[tauri::command]
pub fn work_check_run() -> Result<WorkChecks, WorkProblem> {
    Ok(empty())
}

/// The person's answer to a task check: it worked, or it did not.
#[tauri::command]
pub fn work_check_answer(
    index: u32,
    generation: u64,
    worked: bool,
) -> Result<WorkChecks, WorkProblem> {
    let _ = (index, generation, worked);
    Err(WorkProblem::NoSuchCheck)
}

/// Removes a check.
#[tauri::command]
pub fn work_check_remove(index: u32, generation: u64) -> Result<WorkChecks, WorkProblem> {
    let _ = (index, generation);
    Err(WorkProblem::NoSuchCheck)
}
