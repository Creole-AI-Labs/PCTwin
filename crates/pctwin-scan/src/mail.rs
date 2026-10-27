//! The mail set up on the old laptop, found first (Phase 2 adoption, changes 7 and 8), counted at
//! scan time for the after-move check ("You had 3 mail accounts: sign in to them on the new
//! laptop"). Only what mail stores record about their accounts is read, and the first bytes of a
//! file whose shape is checked; no mail is ever opened.
//!
//! Mail stores are found by general rules, with no list of mail programs:
//!
//! - **MAPI profiles** (Windows): every Office version's `...\Office\<version>\Outlook\Profiles`
//!   and the older `Windows Messaging Subsystem\Profiles`; each account is a key under a
//!   profile's account list ([`mapi_accounts_in`]);
//! - **profile stores**: any `profiles.ini` in the person's app data folders, up to two folders
//!   down; one whose profiles' `prefs.js` name mail servers is a mail store, whatever program
//!   made it ([`profile_mail_accounts_in`]); one naming none (a web browser's) is not mail;
//! - **the Mac's own mail folder** (`~/Library/Mail/V<n>`): one account per account folder;
//! - **mail by its shape** in the person's app data and mail folders: mbox files, maildir
//!   folders (`cur`, `new`, `tmp`), Outlook data files (`.pst`, `.ost`, checked by their first
//!   bytes), Outlook for Mac archives (`.olm`) and folders of saved messages (`.eml`)
//!   ([`shape_of`]).
//!
//! The `mail-clients` catalogue ([`MailClientRow`]) only adds a program's name and, for profile
//! stores, which server kinds are accounts. A store no row names is still reported (with the name
//! of the folder it was found in), never dropped ([`MailAccounts::not_yet_named`]).

use std::path::{Path, PathBuf};
use std::sync::OnceLock;

use pctwin_catalogue::{Catalogue, Row};
use serde::Deserialize;

/// How a mail store was found.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum StoreKind {
    /// MAPI profiles under one Office version (or the older Messaging key).
    MapiProfiles,
    /// A `profiles.ini` store whose profiles name mail servers.
    ProfileStore,
    /// The Mac's own mail folder.
    MacMailFolder,
    Mbox,
    Maildir,
    OutlookDataFile,
    OutlookOfflineCache,
    OutlookMacArchive,
    /// A folder holding saved messages.
    SavedMessages,
}

/// One mail store found.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct MailStore {
    pub kind: StoreKind,
    /// The name it was found under (the folder holding a profile store or a mail file, `Outlook`
    /// for MAPI profiles, `Mail` on a Mac): what the catalogue's rows match.
    pub found_name: String,
    /// The program's name, from the catalogue; `None`: found, not yet named.
    pub client: Option<String>,
    /// Accounts it records; `None` when it does not record accounts (a mail file).
    pub accounts: Option<u32>,
    /// Where it is (a registry key for MAPI profiles).
    pub at: PathBuf,
}

/// Every mail store found for one person.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct MailAccounts {
    pub stores: Vec<MailStore>,
}

impl MailAccounts {
    /// All accounts the stores record. MAPI profiles of several Office versions are copies made
    /// when Office was upgraded, so only the largest of them counts.
    pub fn total(&self) -> u32 {
        let mapi = self
            .stores
            .iter()
            .filter(|s| s.kind == StoreKind::MapiProfiles)
            .filter_map(|s| s.accounts)
            .max()
            .unwrap_or(0);
        self.stores
            .iter()
            .filter(|s| s.kind != StoreKind::MapiProfiles)
            .filter_map(|s| s.accounts)
            .fold(mapi, u32::saturating_add)
    }

    /// The stores no catalogue row names yet.
    pub fn not_yet_named(&self) -> impl Iterator<Item = &MailStore> {
        self.stores.iter().filter(|s| s.client.is_none())
    }
}

/// One row of the `mail-clients` catalogue.
#[derive(Debug, Clone, PartialEq, Eq, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub struct MailClientRow {
    pub name: String,
    /// The names its stores are found under, compared ignoring case and anything but letters
    /// and digits.
    pub found_as: Vec<String>,
    /// For a profile store: the server kinds that are accounts. Empty: the general rule (every
    /// kind but local folders, news and feeds).
    #[serde(default)]
    pub account_kinds: Vec<String>,
}

impl Row for MailClientRow {
    const CATALOGUE: &'static str = "mail-clients";
    const FIELDS: &'static [&'static str] = &["name", "found-as", "account-kinds"];
    fn check(&self) -> Result<(), String> {
        if self.name.trim().is_empty()
            || self.name.len() > 64
            || self.name.chars().any(char::is_control)
        {
            return Err("a name is 1 to 64 plain characters".into());
        }
        if self.found_as.is_empty() || self.found_as.iter().any(|f| squash(f).is_empty()) {
            return Err("a row is found as at least one name with letters or digits".into());
        }
        if self
            .account_kinds
            .iter()
            .any(|k| k.is_empty() || !k.bytes().all(|b| b.is_ascii_alphanumeric()))
        {
            return Err("an account kind is letters and digits".into());
        }
        Ok(())
    }
}

fn squash(name: &str) -> String {
    name.chars()
        .filter(char::is_ascii_alphanumeric)
        .map(|c| c.to_ascii_lowercase())
        .collect()
}

/// The catalogue compiled into this build.
pub fn shipped_mail_clients() -> &'static Catalogue<MailClientRow> {
    static ROWS: OnceLock<Catalogue<MailClientRow>> = OnceLock::new();
    ROWS.get_or_init(|| Catalogue::shipped(include_str!("../catalogues/mail-clients.json")))
}

/// Outlook keeps each account of each MAPI profile as a numbered key under this one.
const MAPI_ACCOUNTS_KEY: &str = "9375cff0413111d3b88a00104b2a6676\\";

/// MAPI accounts in `reg query <...>\Profiles /s` output: each key directly under an account
/// list (`...\9375CFF0...\00000002`), counted once.
pub fn mapi_accounts_in(reg_output: &str) -> u32 {
    let mut seen = std::collections::BTreeSet::new();
    for line in reg_output.lines() {
        let lower = line.trim().to_lowercase();
        let Some(at) = lower.find(MAPI_ACCOUNTS_KEY) else {
            continue;
        };
        let rest = &lower[at + MAPI_ACCOUNTS_KEY.len()..];
        if !rest.is_empty() && !rest.contains('\\') && rest.chars().all(|c| c.is_ascii_hexdigit()) {
            seen.insert(lower);
        }
    }
    u32::try_from(seen.len()).unwrap_or(u32::MAX)
}

/// The Office version keys in `reg query HKCU\Software\Microsoft\Office` output (`16.0`, `15.0`,
/// any other `<number>.<number>`), each as its whole key.
pub fn office_version_keys(reg_output: &str) -> Vec<String> {
    let is_version = |last: &str| {
        last.split_once('.').is_some_and(|(a, b)| {
            !a.is_empty()
                && !b.is_empty()
                && a.bytes().all(|c| c.is_ascii_digit())
                && b.bytes().all(|c| c.is_ascii_digit())
        })
    };
    reg_output
        .lines()
        .map(str::trim)
        .filter(|l| l.starts_with("HKEY_"))
        .filter(|l| l.rsplit('\\').next().is_some_and(is_version))
        .map(str::to_string)
        .collect()
}

/// Server kinds that are never a mail account in a profile store: local folders, news, feeds.
const NOT_ACCOUNTS: [&str; 4] = ["none", "nntp", "rss", "movemail"];

/// Mail servers in a profile's `prefs.js`: each `mail.server.serverN.type`. With `kinds` given,
/// only those count; otherwise every kind but local folders, news and feeds.
pub fn profile_mail_accounts_in(prefs: &str, kinds: &[String]) -> u32 {
    let count = prefs
        .lines()
        .filter_map(|l| {
            let rest = l.trim().strip_prefix("user_pref(\"mail.server.server")?;
            let (_, rest) = rest.split_once(".type\",")?;
            let kind = rest.trim().trim_end_matches(");").trim().trim_matches('"');
            Some(kind.to_ascii_lowercase())
        })
        .filter(|k| {
            if kinds.is_empty() {
                !k.is_empty() && !NOT_ACCOUNTS.contains(&k.as_str())
            } else {
                kinds.iter().any(|w| w.eq_ignore_ascii_case(k))
            }
        })
        .count();
    u32::try_from(count).unwrap_or(u32::MAX)
}

/// Whether a profile's `prefs.js` names any mail server at all (what makes a profile store a mail
/// store).
pub fn names_mail_servers(prefs: &str) -> bool {
    prefs
        .lines()
        .any(|l| l.trim().starts_with("user_pref(\"mail.server.server"))
}

/// The profile folders a `profiles.ini` lists, only those inside its own folder.
pub fn profile_paths_in(ini: &str) -> Vec<PathBuf> {
    ini.lines()
        .filter_map(|l| l.trim().strip_prefix("Path="))
        .map(str::trim)
        .filter(|p| {
            !p.is_empty() && !p.contains("..") && !p.starts_with(['/', '\\']) && !p.contains(':')
        })
        .map(PathBuf::from)
        .collect()
}

/// The Mac's mail accounts: the account folders in its newest `V<n>` folder, never its own
/// `MailData`.
pub fn mac_mail_accounts_in(names: &[String]) -> u32 {
    let count = names
        .iter()
        .filter(|n| !n.starts_with('.') && *n != "MailData")
        .count();
    u32::try_from(count).unwrap_or(u32::MAX)
}

/// Whether the first bytes of a file named `name` are worth reading to tell its shape: only a
/// file whose name says it may be mail (an mbox file with no name ending is not looked for, so
/// that no other file is opened).
pub fn worth_a_look(name: &str) -> bool {
    let lower = name.to_ascii_lowercase();
    [".pst", ".ost", ".olm", ".mbox", ".mbx"]
        .iter()
        .any(|e| lower.ends_with(e))
}

/// The kind of mail a file or folder is by its shape: `name` its name, `children` (for a
/// folder) the names it directly holds, `head` (for a file) its first bytes. A folder holding
/// saved messages is told by its children.
pub fn shape_of(name: &str, children: Option<&[String]>, head: &[u8]) -> Option<StoreKind> {
    if let Some(children) = children {
        let has = |n: &str| children.iter().any(|c| c == n);
        if has("cur") && has("new") && has("tmp") {
            return Some(StoreKind::Maildir);
        }
        let eml = children
            .iter()
            .any(|c| c.to_ascii_lowercase().ends_with(".eml"));
        return eml.then_some(StoreKind::SavedMessages);
    }
    let lower = name.to_ascii_lowercase();
    // Outlook data files start "!BDN"; a file only named so is not one.
    if head.starts_with(b"!BDN") {
        if lower.ends_with(".pst") {
            return Some(StoreKind::OutlookDataFile);
        }
        if lower.ends_with(".ost") {
            return Some(StoreKind::OutlookOfflineCache);
        }
    }
    // An Outlook for Mac archive is a ZIP.
    if lower.ends_with(".olm") && head.starts_with(b"PK\x03\x04") {
        return Some(StoreKind::OutlookMacArchive);
    }
    // An mbox file starts with a "From " line.
    if head.starts_with(b"From ") && head.contains(&b'\n') {
        return Some(StoreKind::Mbox);
    }
    None
}

/// Names the found stores from the catalogue (a store no row names keeps `client: None`) and
/// counts a profile store's accounts by its row's kinds (the general rule without a row).
pub fn name_stores(
    rows: &Catalogue<MailClientRow>,
    found: Vec<(MailStore, Option<Vec<String>>)>,
) -> MailAccounts {
    let reading = rows.read(found, |(s, _), row| {
        let n = squash(&s.found_name);
        // Every row's names have letters or digits (checked on load): an empty one never matches.
        row.found_as.iter().any(|f| squash(f) == n)
    });
    let count = |prefs: &[String], kinds: &[String]| {
        prefs
            .iter()
            .map(|p| profile_mail_accounts_in(p, kinds))
            .fold(0, u32::saturating_add)
    };
    let mut stores = Vec::new();
    for ((mut s, prefs), row) in reading.known {
        s.client = Some(row.name.clone());
        if let Some(prefs) = prefs {
            s.accounts = Some(count(&prefs, &row.account_kinds));
        }
        stores.push(s);
    }
    for (mut s, prefs) in reading.found_not_supported {
        if let Some(prefs) = prefs {
            s.accounts = Some(count(&prefs, &[]));
        }
        stores.push(s);
    }
    MailAccounts { stores }
}

/// Most folders and files looked at while finding mail by its shape (the rest are not).
pub const MAX_LOOKED_AT: usize = 50_000;
/// How far below each app data folder mail is looked for by its shape.
const SHAPE_DEPTH: usize = 4;

/// The mail stores of the person whose home is `home` (the signed-in person: MAPI profiles are
/// read from their own registry).
pub fn mail_accounts(home: &Path) -> MailAccounts {
    let mut found: Vec<(MailStore, Option<Vec<String>>)> = Vec::new();
    if cfg!(windows) {
        found.extend(mapi_stores().into_iter().map(|s| (s, None)));
    }
    let bases = app_data_bases(home);
    for base in &bases {
        found.extend(profile_stores(base));
    }
    if cfg!(target_os = "macos")
        && let Some(s) = mac_mail_store(home)
    {
        found.push((s, None));
    }
    // A store already found is not looked through again for its mail files.
    let skip: Vec<PathBuf> = found.iter().map(|(s, _)| s.at.clone()).collect();
    let mut walk = Walk {
        looked: 0,
        found: Vec::new(),
    };
    for base in &bases {
        walk.below(base, 0, &skip);
    }
    found.extend(walk.found.into_iter().map(|s| (s, None)));
    name_stores(shipped_mail_clients(), found)
}

/// The person's app data folders, where programs keep their stores.
fn app_data_bases(home: &Path) -> Vec<PathBuf> {
    let mut out = Vec::new();
    if cfg!(windows) {
        out.push(home.join("AppData").join("Roaming"));
        out.push(home.join("AppData").join("Local"));
    } else if cfg!(target_os = "macos") {
        out.push(home.join("Library").join("Application Support"));
        out.push(home.join("Library").join("Thunderbird"));
        out.push(home.join("Library").join("Mail"));
    } else {
        // Programs keep their stores in hidden folders of the home.
        if let Ok(entries) = std::fs::read_dir(home) {
            for e in entries.flatten() {
                let name = e.file_name().to_string_lossy().into_owned();
                if name.starts_with('.') && e.file_type().is_ok_and(|t| t.is_dir()) {
                    out.push(e.path());
                }
            }
        }
        out.push(home.join("Mail"));
        out.push(home.join("mail"));
    }
    out
}

fn reg(args: &[&str]) -> Option<String> {
    let root = std::env::var_os("SystemRoot").unwrap_or_else(|| r"C:\Windows".into());
    let out = std::process::Command::new(Path::new(&root).join(r"System32\reg.exe"))
        .args(args)
        .stdin(std::process::Stdio::null())
        .output()
        .ok()?;
    out.status
        .success()
        .then(|| String::from_utf8_lossy(&out.stdout).into_owned())
}

/// MAPI profile stores of every Office version, and the older Messaging key.
fn mapi_stores() -> Vec<MailStore> {
    let mut keys: Vec<String> = reg(&["query", r"HKCU\Software\Microsoft\Office"])
        .map(|o| office_version_keys(&o))
        .unwrap_or_default()
        .into_iter()
        .map(|v| format!(r"{v}\Outlook\Profiles"))
        .collect();
    keys.push(
        r"HKCU\Software\Microsoft\Windows NT\CurrentVersion\Windows Messaging Subsystem\Profiles"
            .to_string(),
    );
    keys.into_iter()
        .filter_map(|key| {
            // No profile there: the key is missing, and that is nothing to claim either way.
            let out = reg(&["query", &key, "/s"])?;
            Some(MailStore {
                kind: StoreKind::MapiProfiles,
                found_name: "Outlook".into(),
                client: None,
                accounts: Some(mapi_accounts_in(&out)),
                at: PathBuf::from(key),
            })
        })
        .collect()
}

/// Every `profiles.ini` in `base` or up to two folders down whose profiles name mail servers.
fn profile_stores(base: &Path) -> Vec<(MailStore, Option<Vec<String>>)> {
    let mut out = Vec::new();
    let mut dirs = vec![(base.to_path_buf(), 0usize)];
    while let Some((dir, depth)) = dirs.pop() {
        if let Ok(text) = std::fs::read_to_string(dir.join("profiles.ini")) {
            let prefs: Vec<String> = profile_paths_in(&text)
                .into_iter()
                .filter_map(|p| std::fs::read_to_string(dir.join(p).join("prefs.js")).ok())
                .collect();
            if prefs.iter().any(|p| names_mail_servers(p)) {
                out.push((
                    MailStore {
                        kind: StoreKind::ProfileStore,
                        found_name: name_of(&dir),
                        client: None,
                        accounts: None,
                        at: dir.clone(),
                    },
                    Some(prefs),
                ));
            }
            continue;
        }
        if depth < 2
            && let Ok(entries) = std::fs::read_dir(&dir)
        {
            for e in entries.flatten() {
                if e.file_type().is_ok_and(|t| t.is_dir()) {
                    dirs.push((e.path(), depth + 1));
                }
            }
        }
    }
    out
}

fn name_of(p: &Path) -> String {
    p.file_name()
        .map(|n| n.to_string_lossy().into_owned())
        .unwrap_or_default()
}

fn mac_mail_store(home: &Path) -> Option<MailStore> {
    let mail = home.join("Library/Mail");
    // A privacy setting may refuse this: then nothing is claimed.
    let newest = std::fs::read_dir(&mail)
        .ok()?
        .flatten()
        .filter_map(|e| {
            let name = e.file_name().to_string_lossy().into_owned();
            name.strip_prefix('V')?.parse::<u32>().ok()
        })
        .max()?;
    let at = mail.join(format!("V{newest}"));
    let names: Vec<String> = std::fs::read_dir(&at)
        .ok()?
        .flatten()
        .filter(|e| e.file_type().is_ok_and(|t| t.is_dir()))
        .map(|e| e.file_name().to_string_lossy().into_owned())
        .collect();
    Some(MailStore {
        kind: StoreKind::MacMailFolder,
        found_name: "Mail".into(),
        client: None,
        accounts: Some(mac_mail_accounts_in(&names)),
        at: mail,
    })
}

/// Finding mail by its shape, a bounded number of entries in all.
pub struct Walk {
    pub looked: usize,
    pub found: Vec<MailStore>,
}

impl Walk {
    /// Mail by its shape in and below `dir` (never through a link, never inside `skip`).
    pub fn below(&mut self, dir: &Path, depth: usize, skip: &[PathBuf]) {
        if skip.iter().any(|s| dir.starts_with(s)) {
            return;
        }
        let Ok(entries) = std::fs::read_dir(dir) else {
            return;
        };
        let entries: Vec<_> = entries.flatten().collect();
        let names: Vec<String> = entries
            .iter()
            .map(|e| e.file_name().to_string_lossy().into_owned())
            .collect();
        if let Some(kind) = shape_of("", Some(&names), &[]) {
            self.found.push(MailStore {
                kind,
                found_name: if kind == StoreKind::Maildir {
                    dir.parent().map(name_of).unwrap_or_default()
                } else {
                    name_of(dir)
                },
                client: None,
                accounts: None,
                at: dir.to_path_buf(),
            });
            if kind == StoreKind::Maildir {
                return;
            }
        }
        for (e, name) in entries.iter().zip(&names) {
            self.looked += 1;
            if self.looked > MAX_LOOKED_AT {
                return;
            }
            let Ok(t) = e.file_type() else { continue };
            if t.is_symlink() {
                continue;
            }
            if t.is_dir() {
                if depth < SHAPE_DEPTH {
                    self.below(&e.path(), depth + 1, skip);
                }
            } else if t.is_file() && worth_a_look(name) {
                let mut head = [0u8; 64];
                let n = std::fs::File::open(e.path())
                    .and_then(|mut f| std::io::Read::read(&mut f, &mut head))
                    .unwrap_or(0);
                if let Some(kind) = shape_of(name, None, &head[..n]) {
                    self.found.push(MailStore {
                        kind,
                        found_name: name_of(dir),
                        client: None,
                        accounts: None,
                        at: e.path(),
                    });
                }
            }
        }
    }
}
