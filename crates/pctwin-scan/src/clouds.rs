//! Cloud storage folders, found on the laptop first (Phase 2 adoption, changes 7 and 8).
//!
//! Every sync app's folder is found by general rules, per system, with no list of apps:
//!
//! - **Windows**: every sync root registered with the system's Cloud Files service
//!   (`HKLM\...\Explorer\SyncRootManager\<provider>!<SID>!<account>\UserSyncRoots`, the
//!   documented registration every modern sync app makes), for the signed-in person's SID;
//! - **macOS**: every folder in `~/Library/CloudStorage` (where Apple's File Provider puts every
//!   sync app's folder, named `<provider>-<account>`);
//! - **Linux**: every FUSE mount inside the home folder (rclone, onedriver and the like).
//!
//! The `cloud-providers` catalogue ([`ProviderRow`]) only **enriches** what was found: the ID and
//! name to show, which accounts are work or school ones, and extra routes to folders a provider
//! records in its own settings (an environment value, a registry value, a JSON settings file, a
//! folder in the home). A folder found with no row is still found: it gets an ID made from the
//! name the system gave it ([`CloudProvider::from_found_name`]) and is reported as found, not yet
//! named ([`FoundCloud::named`]); it is never dropped.

use std::path::{Path, PathBuf};
use std::sync::OnceLock;

use pctwin_catalogue::{Catalogue, Row};
use pctwin_record::CloudProvider;
use serde::Deserialize;

/// The system a row or route applies to.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum System {
    Windows,
    Mac,
    Linux,
}

impl System {
    pub fn this() -> Self {
        if cfg!(windows) {
            Self::Windows
        } else if cfg!(target_os = "macos") {
            Self::Mac
        } else {
            Self::Linux
        }
    }
}

/// A folder a route's path starts from.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum Base {
    Home,
    /// Windows `%LOCALAPPDATA%`; elsewhere the system's local data folder.
    LocalAppData,
    /// Windows `%APPDATA%`; elsewhere the system's data folder.
    AppData,
}

/// One extra way to a provider's folders, by mechanism. The set of mechanisms is code; which
/// provider uses which is data.
#[derive(Debug, Clone, PartialEq, Eq, Deserialize)]
#[serde(rename_all = "kebab-case", deny_unknown_fields)]
pub enum Route {
    /// An environment value holding a folder (`OneDriveCommercial`). `work`: a work or school
    /// account's folder.
    Env {
        system: System,
        name: String,
        #[serde(default)]
        work: bool,
    },
    /// Every value `value` under a registry key of the person's own (`HKCU\...`) and its
    /// subkeys; the subkey is the account.
    Registry {
        system: System,
        key: String,
        value: String,
    },
    /// A JSON settings file holding `{"<account>": {"path": "<folder>"}}`.
    JsonAccounts {
        system: System,
        base: Base,
        file: Vec<String>,
    },
    /// A folder below a base folder.
    Folder {
        system: System,
        base: Base,
        path: Vec<String>,
    },
}

impl Route {
    fn system(&self) -> System {
        match self {
            Route::Env { system, .. }
            | Route::Registry { system, .. }
            | Route::JsonAccounts { system, .. }
            | Route::Folder { system, .. } => *system,
        }
    }
}

/// One row of the `cloud-providers` catalogue.
#[derive(Debug, Clone, PartialEq, Eq, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub struct ProviderRow {
    /// The ID records carry ([`CloudProvider`]).
    pub id: CloudProvider,
    /// Its name, to show.
    pub name: String,
    /// The names the system knows it by (a sync root's provider name, a CloudStorage folder's
    /// first part), compared ignoring case and anything but letters and digits.
    #[serde(default)]
    pub found_as: Vec<String>,
    /// When given, the accounts that are personal (compared ignoring case; `""` is "no account
    /// named"; `Personal` also covers `Personal|<id>`): every other account is a work or school
    /// one.
    #[serde(default)]
    pub personal_accounts: Option<Vec<String>>,
    /// Account names starting with one of these (ignoring case) are work or school accounts.
    #[serde(default)]
    pub work_accounts: Vec<String>,
    #[serde(default)]
    pub routes: Vec<Route>,
}

fn plain_part(n: &str) -> bool {
    !n.is_empty() && n != "." && n != ".." && !n.contains(['/', '\\', '\0']) && n.len() <= 255
}

impl Row for ProviderRow {
    const CATALOGUE: &'static str = "cloud-providers";
    const FIELDS: &'static [&'static str] = &[
        "id",
        "name",
        "found-as",
        "personal-accounts",
        "work-accounts",
        "routes",
    ];
    fn check(&self) -> Result<(), String> {
        if self.name.trim().is_empty()
            || self.name.len() > 64
            || self.name.chars().any(char::is_control)
        {
            return Err("a name is 1 to 64 plain characters".into());
        }
        if self.id == CloudProvider::Other {
            return Err("`other` is for clouds no row names".into());
        }
        if self.found_as.iter().any(|f| squash(f).is_empty()) {
            return Err("a found-as name has letters or digits".into());
        }
        for r in &self.routes {
            let ok = match r {
                Route::Env { name, .. } => {
                    !name.is_empty()
                        && name.len() <= 64
                        && name.bytes().all(|b| b.is_ascii_alphanumeric() || b == b'_')
                }
                Route::Registry { key, value, .. } => {
                    // Only the person's own settings, and only reading.
                    key.starts_with("HKCU\\")
                        && key.len() <= 255
                        && !key.contains(['"', '\0', '\n'])
                        && !value.is_empty()
                        && value.len() <= 64
                        && !value.contains(['"', '\0', '\n', '\\'])
                }
                Route::JsonAccounts { file: p, .. } | Route::Folder { path: p, .. } => {
                    !p.is_empty() && p.len() <= 16 && p.iter().all(|n| plain_part(n))
                }
            };
            if !ok {
                return Err("a route breaks the rules".into());
            }
        }
        Ok(())
    }
}

/// Letters and digits only, lower case: how found names are compared with a row's.
fn squash(name: &str) -> String {
    name.chars()
        .filter(char::is_ascii_alphanumeric)
        .map(|c| c.to_ascii_lowercase())
        .collect()
}

impl ProviderRow {
    /// Whether an account (as the system names it) is a work or school one.
    pub fn is_work(&self, account: &str) -> bool {
        let account = account.trim().to_lowercase();
        if let Some(personal) = &self.personal_accounts {
            // `Personal`, or `Personal` and more after a mark (`Personal|<id>`, as a sync root
            // names it), never `PersonalStuff`.
            return !personal.iter().any(|p| {
                let p = p.to_lowercase();
                account == p
                    || (!p.is_empty()
                        && account
                            .strip_prefix(&p)
                            .and_then(|rest| rest.chars().next())
                            .is_some_and(|c| !c.is_alphanumeric()))
            });
        }
        self.work_accounts
            .iter()
            .any(|w| !w.is_empty() && account.starts_with(&w.to_lowercase()))
    }

    fn knows(&self, found_name: &str) -> bool {
        let found = squash(found_name);
        !found.is_empty() && self.found_as.iter().any(|f| squash(f) == found)
    }
}

/// The catalogue compiled into this build.
pub fn shipped() -> &'static Catalogue<ProviderRow> {
    static ROWS: OnceLock<Catalogue<ProviderRow>> = OnceLock::new();
    ROWS.get_or_init(|| Catalogue::shipped(include_str!("../catalogues/cloud-providers.json")))
}

/// A provider's name to show, from the shipped catalogue; `None` for one no row names.
pub fn provider_name(provider: CloudProvider) -> Option<&'static str> {
    provider_name_in(shipped(), provider)
}

/// [`provider_name`] in a given catalogue.
pub fn provider_name_in(rows: &Catalogue<ProviderRow>, provider: CloudProvider) -> Option<&str> {
    let reading = rows.read([provider], |p, row| row.id == *p);
    reading.known.first().map(|(_, row)| row.name.as_str())
}

/// How a cloud folder was found.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum How {
    /// A sync root registered with Windows' Cloud Files service.
    SyncRoot,
    /// A folder in macOS's `~/Library/CloudStorage`.
    CloudStorageFolder,
    /// A FUSE mount inside the home folder (Linux).
    Mount,
    /// A route a catalogue row gave.
    Route,
}

/// One cloud folder found on the laptop.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct FoundCloud {
    pub provider: CloudProvider,
    pub root: PathBuf,
    /// A work or school account's folder.
    pub work: bool,
    /// A catalogue row names its provider. `false`: found, not yet named (reported so).
    pub named: bool,
    pub how: How,
    /// The name the system gave its provider (for the "found, not yet named" report).
    pub found_name: String,
}

/// A cloud folder as a discovery mechanism saw it, before the catalogue is read.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Seen {
    /// The provider's name as the system gave it.
    pub provider: String,
    /// The account part, when the system gives one.
    pub account: String,
    pub root: PathBuf,
    pub how: How,
}

/// Turns what the mechanisms saw into found clouds: a row that knows the provider's name gives
/// its ID and work rule; one with no row gets an ID from its own name, and is found all the same.
pub fn name_seen(rows: &Catalogue<ProviderRow>, seen: Vec<Seen>) -> Vec<FoundCloud> {
    let reading = rows.read(seen, |s, row| row.knows(&s.provider));
    let mut out: Vec<FoundCloud> = reading
        .known
        .into_iter()
        .map(|(s, row)| FoundCloud {
            provider: row.id,
            work: row.is_work(&s.account),
            named: true,
            how: s.how,
            found_name: s.provider,
            root: s.root,
        })
        .collect();
    out.extend(reading.found_not_supported.into_iter().map(|s| FoundCloud {
        provider: CloudProvider::from_found_name(&s.provider),
        // PCTwin cannot tell a work account of a service it does not know: the person's own,
        // so it is copied rather than left out.
        work: false,
        named: false,
        how: s.how,
        found_name: s.provider,
        root: s.root,
    }));
    out
}

/// Windows: the sync roots in `reg query HKLM\...\SyncRootManager /s` output, for the person
/// with SID `sid` (both the key's middle part and the value's name must be it).
pub fn sync_roots_from_reg(output: &str, sid: &str) -> Vec<Seen> {
    let mut out = Vec::new();
    let mut current: Option<(String, String, String)> = None;
    for line in output.lines() {
        let line = line.trim_end_matches('\r');
        if line.starts_with("HKEY_") {
            current = None;
            let mut parts = line.trim().rsplit('\\');
            if parts.next() != Some("UserSyncRoots") {
                continue;
            }
            let Some(id) = parts.next() else { continue };
            let mut fields = id.splitn(3, '!');
            let (Some(provider), Some(key_sid), Some(account)) =
                (fields.next(), fields.next(), fields.next())
            else {
                continue;
            };
            if !provider.is_empty() {
                current = Some((
                    provider.to_string(),
                    key_sid.to_string(),
                    account.to_string(),
                ));
            }
            continue;
        }
        let Some((provider, key_sid, account)) = &current else {
            continue;
        };
        let rest = line.trim_start();
        let Some(rest) = rest.strip_prefix(sid) else {
            continue;
        };
        if !rest.starts_with([' ', '\t']) || key_sid != sid {
            continue;
        }
        let rest = rest.trim_start();
        let Some(value) = rest
            .strip_prefix("REG_EXPAND_SZ")
            .or_else(|| rest.strip_prefix("REG_SZ"))
        else {
            continue;
        };
        let value = value.trim();
        if !value.is_empty() {
            out.push(Seen {
                provider: provider.clone(),
                account: account.clone(),
                root: PathBuf::from(value),
                how: How::SyncRoot,
            });
        }
    }
    out
}

/// macOS: a `~/Library/CloudStorage` folder's name split into provider and account
/// (`OneDrive-Personal`, `GoogleDrive-ada@example.com`, `Dropbox (Contoso)`, `Box-Box`).
pub fn cloud_storage_name(name: &str) -> (String, String) {
    let cut = name.find(['-', ' ', '(']).unwrap_or(name.len());
    let (provider, rest) = name.split_at(cut);
    let account = rest.trim_start_matches(['-', ' ']).trim();
    (provider.to_string(), account.to_string())
}

/// Linux: FUSE mounts inside `home` from `/proc/self/mounts` text: the mount point and the
/// file system's own name (`fuse.rclone` gives `rclone`).
pub fn fuse_mounts_in(mounts: &str, home: &Path) -> Vec<Seen> {
    mounts
        .lines()
        .filter_map(|l| {
            let mut f = l.split_whitespace();
            let (_, point, kind) = (f.next()?, f.next()?, f.next()?);
            let tool = kind.strip_prefix("fuse.")?;
            // The mounts file writes a space in a path as \040.
            let point = PathBuf::from(point.replace("\\040", " "));
            (point != home && point.starts_with(home)).then(|| Seen {
                provider: tool.to_string(),
                account: String::new(),
                root: point,
                how: How::Mount,
            })
        })
        .collect()
}

/// The folders a row's routes lead to on this laptop, each with whether it is a work account.
fn route_folders(
    row: &ProviderRow,
    system: System,
    home: &Path,
    read: &dyn RouteReader,
) -> Vec<(PathBuf, bool)> {
    let base = |b: Base| -> Option<PathBuf> {
        match b {
            Base::Home => Some(home.to_path_buf()),
            Base::LocalAppData => read.local_app_data(),
            Base::AppData => read.app_data(),
        }
    };
    let join = |b: PathBuf, parts: &[String]| parts.iter().fold(b, |p, n| p.join(n));
    let mut out = Vec::new();
    for route in row.routes.iter().filter(|r| r.system() == system) {
        match route {
            Route::Env { name, work, .. } => {
                if let Some(p) = read.env(name) {
                    out.push((p, *work));
                }
            }
            Route::Registry { key, value, .. } => {
                for (account, p) in read.registry(key, value) {
                    out.push((p, row.is_work(&account)));
                }
            }
            Route::JsonAccounts { base: b, file, .. } => {
                if let Some(b) = base(*b) {
                    for (account, p) in read.json_accounts(&join(b, file)) {
                        out.push((p, row.is_work(&account)));
                    }
                }
            }
            Route::Folder { base: b, path, .. } => {
                if let Some(b) = base(*b) {
                    out.push((join(b, path), false));
                }
            }
        }
    }
    out
}

/// What the routes read from the system (a test gives its own).
pub trait RouteReader {
    fn env(&self, name: &str) -> Option<PathBuf>;
    /// `(subkey, folder)` for every `value` under `key` and its subkeys.
    fn registry(&self, key: &str, value: &str) -> Vec<(String, PathBuf)>;
    /// `(account, folder)` from a JSON settings file.
    fn json_accounts(&self, file: &Path) -> Vec<(String, PathBuf)>;
    fn local_app_data(&self) -> Option<PathBuf>;
    fn app_data(&self) -> Option<PathBuf>;
}

/// Every cloud folder: what the system's own mechanisms show (`seen`), named by the catalogue,
/// then every row's routes. A folder found twice is kept once (the first way it was found), and
/// is a work account's if any way said so. Only folders that exist (`exists`) are kept.
pub fn find_clouds(
    rows: &Catalogue<ProviderRow>,
    system: System,
    home: &Path,
    seen: Vec<Seen>,
    read: &dyn RouteReader,
    exists: &dyn Fn(&Path) -> bool,
) -> Vec<FoundCloud> {
    let mut all = name_seen(rows, seen);
    for row in rows.rows() {
        for (root, work) in route_folders(row, system, home, read) {
            all.push(FoundCloud {
                provider: row.id,
                root,
                work,
                named: true,
                how: How::Route,
                found_name: row.name.clone(),
            });
        }
    }
    let mut out: Vec<FoundCloud> = Vec::new();
    for f in all {
        if !exists(&f.root) {
            continue;
        }
        if let Some(same) = out
            .iter_mut()
            .find(|o| crate::folders::same_place(&o.root, &f.root))
        {
            same.work |= f.work;
            continue;
        }
        out.push(f);
    }
    out
}

/// Reads routes from this laptop.
pub struct SystemRoutes;

impl RouteReader for SystemRoutes {
    fn env(&self, name: &str) -> Option<PathBuf> {
        std::env::var_os(name).map(PathBuf::from)
    }
    fn registry(&self, key: &str, value: &str) -> Vec<(String, PathBuf)> {
        crate::folders::registry_values(key, value)
    }
    fn json_accounts(&self, file: &Path) -> Vec<(String, PathBuf)> {
        crate::folders::json_accounts(file)
    }
    fn local_app_data(&self) -> Option<PathBuf> {
        dirs::data_local_dir()
    }
    fn app_data(&self) -> Option<PathBuf> {
        dirs::data_dir()
    }
}

/// What the system's own mechanisms show on this laptop.
pub fn seen_on_this_laptop(home: &Path) -> Vec<Seen> {
    let mut seen = Vec::new();
    if cfg!(windows) {
        let root = std::env::var_os("SystemRoot").unwrap_or_else(|| r"C:\Windows".into());
        let system32 = Path::new(&root).join("System32");
        let sid = std::process::Command::new(system32.join("whoami.exe"))
            .args(["/user", "/fo", "csv", "/nh"])
            .stdin(std::process::Stdio::null())
            .output()
            .ok()
            .and_then(|o| crate::census::sid_from_whoami(&String::from_utf8_lossy(&o.stdout)));
        if let Some(sid) = sid
            && let Ok(out) = std::process::Command::new(system32.join("reg.exe"))
                .args([
                    "query",
                    r"HKLM\SOFTWARE\Microsoft\Windows\CurrentVersion\Explorer\SyncRootManager",
                    "/s",
                ])
                .stdin(std::process::Stdio::null())
                .output()
        {
            seen.extend(sync_roots_from_reg(
                &String::from_utf8_lossy(&out.stdout),
                &sid,
            ));
        }
    } else if cfg!(target_os = "macos") {
        if let Ok(entries) = std::fs::read_dir(home.join("Library/CloudStorage")) {
            for entry in entries.flatten() {
                if !entry.file_type().is_ok_and(|t| t.is_dir()) {
                    continue;
                }
                let name = entry.file_name().to_string_lossy().into_owned();
                let (provider, account) = cloud_storage_name(&name);
                seen.push(Seen {
                    provider,
                    account,
                    root: entry.path(),
                    how: How::CloudStorageFolder,
                });
            }
        }
    } else if let Ok(mounts) = std::fs::read_to_string("/proc/self/mounts") {
        seen.extend(fuse_mounts_in(&mounts, home));
    }
    seen
}
