//! Who writes what: one writer for each settings domain, as the catalogue declares it (Phase
//! 2.4 plan section 3; T24-017).
//!
//! Every catalogue setting is written by the settings role (2.4). Each file of an app's settings
//! names its writer in the app's catalogue entry: 2.4 for settings files, 2.5 for files that mix
//! preferences with private data (VS Code's `state.vscdb`), 2.1 for code (extensions, plugins),
//! nobody for sign-ins, machine state and caches. A role that is not a domain's writer cannot take
//! its lock, and two holders can never hold one domain at once.

use std::collections::BTreeMap;
use std::sync::{Arc, Mutex};

use crate::apps::path_matches;
use crate::catalogue::{Catalogue, SettingId, Writer};

/// A settings domain.
#[derive(Debug, Clone, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub enum Domain {
    Setting(SettingId),
    /// A file of an app's settings, by its path inside the app's settings root.
    AppFile {
        app: String,
        path: String,
    },
}

/// The one writer of a domain, from the catalogue. Unknown domains have none.
pub fn writer_of(cat: &Catalogue, domain: &Domain) -> Writer {
    match domain {
        Domain::Setting(id) => {
            if cat.row(id.as_str()).is_some() {
                Writer::Settings
            } else {
                Writer::Nobody
            }
        }
        Domain::AppFile { app, path } => cat
            .app(app)
            .and_then(|a| a.files.iter().find(|f| path_matches(&f.path, path)))
            .map(|f| f.writer)
            .unwrap_or(Writer::Nobody),
    }
}

#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
pub enum LockError {
    #[error("this part belongs to another part of PCTwin")]
    NotTheWriter,
    #[error("this part is being changed already")]
    Busy,
}

/// The locks of every domain on the new laptop, shared by the roles.
#[derive(Debug, Clone, Default)]
pub struct DomainLocks {
    held: Arc<Mutex<BTreeMap<Domain, Writer>>>,
}

/// A held domain; given back when dropped.
#[derive(Debug)]
pub struct DomainGuard {
    held: Arc<Mutex<BTreeMap<Domain, Writer>>>,
    domain: Domain,
}

impl DomainLocks {
    /// Takes `domain` for `writer`: only its one writer may, and only while nobody holds it.
    pub fn acquire(
        &self,
        cat: &Catalogue,
        domain: Domain,
        writer: Writer,
    ) -> Result<DomainGuard, LockError> {
        if writer == Writer::Nobody || writer_of(cat, &domain) != writer {
            return Err(LockError::NotTheWriter);
        }
        let mut held = self.held.lock().unwrap_or_else(|p| p.into_inner());
        if held.contains_key(&domain) {
            return Err(LockError::Busy);
        }
        held.insert(domain.clone(), writer);
        Ok(DomainGuard {
            held: Arc::clone(&self.held),
            domain,
        })
    }
}

impl Drop for DomainGuard {
    fn drop(&mut self) {
        let mut held = self.held.lock().unwrap_or_else(|p| p.into_inner());
        held.remove(&self.domain);
    }
}
