//! Clean-up of PCTwin's own files (Task List 2.7 "Source cleanup and PCTwin model/cache removal,
//! each asked separately"; Security Design A "Clean exit" and C "Removing our own cache or model
//! ... is a separate step the user approves"; screen R07; acceptance T27-015).
//!
//! Two questions, asked separately: PCTwin's temporary files, and PCTwin's model and caches. Each
//! is decided by exact ownership: only what PCTwin recorded making is offered for removal.
//! Something that merely looks like PCTwin's (by its name or folder), and anything that is the
//! person's selected work (a model they chose to keep), stays. Nothing here removes anything: the
//! [`Plan`] says exactly what would go, its digest binds the person's confirmation to exactly
//! that, and the caller removes only the plan's entries, checked again.

use pctwin_record::ItemId;
use serde::{Deserialize, Serialize};

/// One of the two clean-up questions.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum What {
    TemporaryFiles,
    ModelAndCache,
}

/// How PCTwin knows a file or folder is its own.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum Ownership {
    /// PCTwin recorded making it, and it is still the very one recorded.
    RecordedByPctwin,
    /// It only looks like PCTwin's (by its name or folder).
    LooksTemporary,
}

/// One thing found.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Entry {
    /// Its name, as shown.
    pub name: String,
    pub what: What,
    pub bytes: u64,
    pub ownership: Ownership,
    /// The person's selected work it is, if it is.
    pub item: Option<ItemId>,
    /// A program has it open, or a move is using it.
    pub in_use: bool,
}

impl Entry {
    fn removable(&self) -> bool {
        self.ownership == Ownership::RecordedByPctwin && self.item.is_none() && !self.in_use
    }
}

/// What one question would remove, and what it keeps.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Plan {
    pub what: What,
    pub removes: Vec<Entry>,
    pub keeps: Vec<Entry>,
}

impl Plan {
    /// The plan for `what` from everything `found` (entries of the other question are left out).
    pub fn of(what: What, found: &[Entry]) -> Self {
        let (removes, keeps) = found
            .iter()
            .filter(|e| e.what == what)
            .cloned()
            .partition(Entry::removable);
        Self {
            what,
            removes,
            keeps,
        }
    }

    /// Binds the person's confirmation to exactly this plan.
    pub fn digest(&self) -> [u8; 32] {
        let mut h = blake3::Hasher::new();
        h.update(b"pctwin/v1/cleanup-plan\0");
        h.update(&serde_json::to_vec(self).expect("serialises"));
        *h.finalize().as_bytes()
    }
}

/// Why a clean-up was refused. Nothing was removed.
#[derive(Debug, Clone, Copy, PartialEq, Eq, thiserror::Error)]
pub enum CleanupError {
    #[error("what was confirmed is not what is there now")]
    Changed,
    #[error("there is nothing of PCTwin's to remove")]
    Nothing,
}

/// The entries to remove now: the plan made from what is found now must be exactly the one the
/// person confirmed (`seen`), and not empty.
pub fn approve(
    what: What,
    found_now: &[Entry],
    seen: [u8; 32],
) -> Result<Vec<Entry>, CleanupError> {
    let plan = Plan::of(what, found_now);
    if plan.digest() != seen {
        return Err(CleanupError::Changed);
    }
    if plan.removes.is_empty() {
        return Err(CleanupError::Nothing);
    }
    Ok(plan.removes)
}
