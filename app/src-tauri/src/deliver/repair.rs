//! Repair only what failed (Task List 2.9; research RF10): "Fix these 3 items" in the finishing
//! flow. Only the items that did not finish are tried again; every successful result is kept;
//! what each retry needs is checked again first.
//!
//! Contract only (stub): what the screens send and get. The real repair follows in its own
//! commit.

use pctwin_fullmove::plan::{Kind, Title};
use pctwin_types::{ItemStatus, Reason};
use serde::Serialize;

pctwin_types::listed_enum! {
    /// What a retry needs, checked again before it runs.
    #[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize)]
    #[serde(rename_all = "kebab-case")]
    pub enum Prerequisite {
        /// The old laptop, paired and connected (for files and app data from it).
        OldLaptop,
        /// The internet (for an app's download).
        Internet,
        /// The system's administrator prompt, answered yes (for installing an app).
        Administrator,
        /// The system's own installer tool (winget, a Linux package tool).
        InstallerTool,
        /// Room on this laptop.
        Space,
        /// The app closed (for its data or settings).
        CloseApp,
    }
}

/// One item that did not finish.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct RepairLine {
    /// Its place in the offer (what the screens send back).
    pub line: u32,
    pub title: Title,
    pub kind: Kind,
    pub status: ItemStatus,
    pub reason: Option<Reason>,
    /// What a retry needs that is not there now (empty: it can be tried now).
    pub waits_for: Vec<Prerequisite>,
}

/// What "Fix these" can try again.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct RepairOffer {
    /// The offer's version: a retry of another version is refused.
    pub generation: u64,
    pub lines: Vec<RepairLine>,
    pub more: u64,
    /// Results that finished and are kept as they are.
    pub kept: u64,
}

/// What a retry did.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct RepairDone {
    pub generation: u64,
    /// Items that finished this time.
    pub fixed: u64,
    /// Items still not finished, each with what it waits for.
    pub lines: Vec<RepairLine>,
    pub more: u64,
    /// Results that finished before, kept as they were.
    pub kept: u64,
}

pctwin_types::listed_enum! {
    /// Why a retry did not start.
    #[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
    #[serde(rename_all = "kebab-case")]
    pub enum RepairProblem {
        /// Nothing picked is unfinished.
        NothingToRepair,
        /// The offer changed since the screens saw it.
        Stale,
        /// A move, an undo or another retry is going on.
        Busy,
        Failed,
    }
}

/// What "Fix these" can try again for the last move.
#[tauri::command]
pub fn repair_offer() -> RepairOffer {
    RepairOffer {
        generation: 0,
        lines: Vec::new(),
        more: 0,
        kept: 0,
    }
}

/// Tries again exactly the picked lines of the offer `generation`.
#[tauri::command]
pub async fn repair_run(generation: u64, lines: Vec<u32>) -> Result<RepairDone, RepairProblem> {
    let _ = (generation, lines);
    Err(RepairProblem::NothingToRepair)
}
