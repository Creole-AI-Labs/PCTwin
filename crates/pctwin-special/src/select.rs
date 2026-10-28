//! What moves: the person's selection (plan section 10: explicit categories, never a default
//! "copy all hidden folders" switch).
//!
//! [`Selection::default_for`] proposes every portable, non-secret component and nothing else. A
//! secret (a private key, a password database, a key store, saved passwords) moves only when the
//! person picks it by its identity ([`Selection::choose_secret`]); there is no way to pick "all
//! secrets". Machine-bound, service-held and unresolved components are never proposed for
//! copying: they appear as steps (sign in, re-enrol, relink) and, where a labelled copy is kept
//! (a server mailbox's offline copy), as that.

use std::collections::BTreeSet;

use crate::classify::FolderReport;
use crate::component::{Component, ComponentId, Custody};

/// The components chosen to move.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct Selection {
    chosen: BTreeSet<ComponentId>,
}

/// Why a component could not be chosen.
#[derive(Debug, Clone, Copy, PartialEq, Eq, thiserror::Error)]
pub enum NotChosen {
    #[error("no such component was found")]
    Unknown,
    #[error("it is not a secret: it is chosen with everything else")]
    NotSecret,
    #[error("it cannot be copied: it is bound to the old laptop, held by a service, or unknown")]
    NotCopyable,
}

fn all(reports: &[FolderReport]) -> impl Iterator<Item = &Component> {
    reports.iter().flat_map(|r| r.components.iter())
}

fn copyable(c: &Component) -> bool {
    matches!(c.custody, Custody::Portable | Custody::NeedsExport)
        || (c.custody == Custody::ServiceHeld && c.preserve)
}

impl Selection {
    /// The proposal: every copyable component that is not secret.
    pub fn default_for(reports: &[FolderReport]) -> Self {
        Self {
            chosen: all(reports)
                .filter(|c| copyable(c) && !c.secret)
                .map(|c| c.id)
                .collect(),
        }
    }

    /// The person picks one secret, by its identity.
    pub fn choose_secret(
        &mut self,
        reports: &[FolderReport],
        id: ComponentId,
    ) -> Result<(), NotChosen> {
        let c = all(reports)
            .find(|c| c.id == id)
            .ok_or(NotChosen::Unknown)?;
        if !c.secret {
            return Err(NotChosen::NotSecret);
        }
        if !copyable(c) {
            return Err(NotChosen::NotCopyable);
        }
        self.chosen.insert(id);
        Ok(())
    }

    /// The person leaves one out.
    pub fn leave_out(&mut self, id: ComponentId) {
        self.chosen.remove(&id);
    }

    pub fn contains(&self, id: ComponentId) -> bool {
        self.chosen.contains(&id)
    }

    pub fn chosen(&self) -> impl Iterator<Item = &ComponentId> {
        self.chosen.iter()
    }
}
