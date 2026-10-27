//! Who writes what: one writer for each settings domain (Phase 2.4 plan section 3; T24-017).
//!
//! The settings role (2.4) and the private-data role (2.5) both work in an app's settings
//! folder. Each file or system setting has exactly one writer, fixed here; a role that is not a
//! domain's writer cannot take its lock, and two writers can never hold the same domain at once.
//! Files that mix preferences with private data (VS Code's `state.vscdb`, which also holds
//! extension sign-ins) belong to 2.5; 2.4 gives it the rewritten values to write (for example the
//! recent-files list) and never opens the file itself.

use std::collections::BTreeMap;
use std::sync::{Arc, Mutex};

use crate::schema::SettingId;

/// The roles that write on the new laptop.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub enum Writer {
    /// 2.4: named settings and the app settings files on its list.
    Settings,
    /// 2.5: private data, and files that mix preferences with it.
    Data,
    /// 2.1: apps and their extensions or plugins (executable code).
    Install,
    /// Nobody in PCTwin: the person, the system or the organisation (privacy approvals,
    /// security controls, sign-in state).
    Nobody,
}

/// The apps whose settings 2.4 maps.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub enum App {
    VsCode,
    JetBrains,
}

/// A settings domain: one system setting, or one file inside an app's settings folder.
#[derive(Debug, Clone, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub enum Domain {
    System(SettingId),
    /// A path inside the app's settings folder, `/`-separated.
    AppFile(App, String),
}

/// The one writer of a domain.
pub fn writer_of(domain: &Domain) -> Writer {
    match domain {
        Domain::System(_) => Writer::Settings,
        Domain::AppFile(App::VsCode, path) => vscode_writer(path),
        Domain::AppFile(App::JetBrains, path) => jetbrains_writer(path),
    }
}

/// VS Code's user folder (`Code/User/...`, `extensions/...`).
fn vscode_writer(path: &str) -> Writer {
    let p = path.to_ascii_lowercase();
    match p.as_str() {
        "user/settings.json" | "user/keybindings.json" | "user/tasks.json" => Writer::Settings,
        _ if p.starts_with("user/snippets/") => Writer::Settings,
        // Extension state, sign-ins and the recent list live together here.
        _ if p.starts_with("user/globalstorage/") || p.starts_with("user/workspacestorage/") => {
            Writer::Data
        }
        _ if p.starts_with("extensions/") => Writer::Install,
        _ => Writer::Nobody,
    }
}

/// A JetBrains IDE's config folder (`options/...`, `keymaps/...`).
fn jetbrains_writer(path: &str) -> Writer {
    let p = path.to_ascii_lowercase();
    if p.starts_with("plugins/") {
        return Writer::Install;
    }
    if p.starts_with("workspace/") || p.starts_with("projects/") {
        return Writer::Data;
    }
    if jetbrains_settings_file(path) {
        Writer::Settings
    } else {
        Writer::Nobody
    }
}

/// JetBrains options files that hold sign-ins, machine state, screen positions or a security
/// approval (trusted folders): never written by PCTwin.
pub const JETBRAINS_NEVER: &[&str] = &[
    "options/security.xml",
    "options/trusted-paths.xml",
    "options/jdk.table.xml",
    "options/updates.xml",
    "options/usage.statistics.xml",
    "options/window.state.xml",
    "options/github.xml",
    "options/gitlab.xml",
    "options/jetbrains.ai.xml",
    "options/remote-servers.xml",
];

/// The JetBrains config folders whose files 2.4 writes (`tools/` holds external tools, which
/// run commands: written as data, never switched on; see `apps::jetbrains`).
pub const JETBRAINS_FOLDERS: &[&str] = &[
    "options/",
    "codestyles/",
    "keymaps/",
    "colors/",
    "templates/",
    "filetemplates/",
    "inspection/",
    "quicklists/",
    "tools/",
];

/// Whether 2.4 writes this JetBrains config file: in one of its folders, not on the never list.
pub fn jetbrains_settings_file(path: &str) -> bool {
    let p = path.to_ascii_lowercase();
    !p.split('/')
        .any(|part| part.is_empty() || part == "." || part == "..")
        && JETBRAINS_FOLDERS
            .iter()
            .any(|f| p.starts_with(f) && p.len() > f.len())
        && !JETBRAINS_NEVER.contains(&p.as_str())
        && !p.ends_with(".key")
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

/// A held domain; released when dropped.
#[derive(Debug)]
pub struct DomainGuard {
    held: Arc<Mutex<BTreeMap<Domain, Writer>>>,
    domain: Domain,
}

impl DomainLocks {
    /// Takes `domain` for `writer`: only its one writer may, and only while nobody holds it.
    pub fn acquire(&self, domain: Domain, writer: Writer) -> Result<DomainGuard, LockError> {
        if writer == Writer::Nobody || writer_of(&domain) != writer {
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
