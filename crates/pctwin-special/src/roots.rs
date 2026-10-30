//! Where app data lives, found by general rules per system (ADOPTION change 8: discovery first).
//!
//! Every per-person app data root each system has ([`roots_of`]) is listed, and every folder in
//! it is found ([`find_folders`]). Each folder is tied to an app from 2.1's list only by the
//! system's own ownership facts ([`Tie`]): a packaged Windows app's family name, a Mac bundle ID
//! or its maker's Team ID on a group container, a Flatpak app ID, a Snap name; and, as a weaker
//! clue, a folder named the way the old laptop recorded the app or its maker. A folder no fact
//! ties is reported as found with no app, never dropped. No app is named here.
//!
//! The roots of the system's own caches are listed too, as caches, so the report can say they
//! were seen and why nothing in them moves.

use std::path::{Path, PathBuf};

use pctwin_record::{AppMaker, FoundApp, FoundWhere, Owner};
use serde::{Deserialize, Serialize};

/// Which system the laptop runs.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum System {
    Windows,
    Mac,
    Linux,
}

impl System {
    /// The system this program runs on.
    pub fn here() -> Self {
        if cfg!(windows) {
            System::Windows
        } else if cfg!(target_os = "macos") {
            System::Mac
        } else {
            System::Linux
        }
    }
}

/// A kind of app data root.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum RootKind {
    /// Windows `AppData\Roaming`.
    Roaming,
    /// Windows `AppData\Local`.
    Local,
    /// Windows `AppData\LocalLow`.
    LocalLow,
    /// Windows packaged apps' own folders (`AppData\Local\Packages`).
    Packages,
    /// Windows `Saved Games` (a folder the system names for games).
    SavedGames,
    /// Mac `~/Library/Application Support`.
    ApplicationSupport,
    /// Mac sandboxed apps' containers.
    Containers,
    /// Mac containers shared by one maker's apps.
    GroupContainers,
    /// Mac `~/Library/Preferences`.
    Preferences,
    /// Linux `$XDG_CONFIG_HOME` (`~/.config`).
    XdgConfig,
    /// Linux `$XDG_DATA_HOME` (`~/.local/share`).
    XdgData,
    /// Linux `$XDG_STATE_HOME` (`~/.local/state`).
    XdgState,
    /// Flatpak apps' own homes (`~/.var/app`).
    Flatpak,
    /// Snap apps' own homes (`~/snap`).
    Snap,
    /// A hidden folder in the home folder (`~/.name`), on any system.
    HomeDot,
    /// The system's own cache root: seen, never moved.
    SystemCache,
    /// Mac `~/Library/Mail/V<n>` (Apple Mail's own folder): never walked as app data and never
    /// landed as it is; only its raw message files are carried, and the new laptop writes the
    /// mail kept only on the old Mac as standard mailbox files ([`crate::applemail`]).
    MacMail,
}

impl RootKind {
    /// Whether this root is the system's own cache place.
    pub fn is_cache(self) -> bool {
        self == RootKind::SystemCache
    }
}

/// One app data root of one person.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DataRoot {
    pub kind: RootKind,
    pub path: PathBuf,
}

/// Every app data root a person with home folder `home` has on `system`, in the system's
/// standard places. `xdg` gives a Linux person's own `XDG_*_HOME` value when it is set (the
/// variable's name in, an absolute path out).
pub fn roots_of(
    system: System,
    home: &Path,
    xdg: &dyn Fn(&str) -> Option<PathBuf>,
) -> Vec<DataRoot> {
    let r = |kind, path: PathBuf| DataRoot { kind, path };
    match system {
        System::Windows => {
            let app = home.join("AppData");
            vec![
                r(RootKind::Roaming, app.join("Roaming")),
                r(RootKind::Local, app.join("Local")),
                r(RootKind::LocalLow, app.join("LocalLow")),
                r(RootKind::Packages, app.join("Local").join("Packages")),
                r(RootKind::SavedGames, home.join("Saved Games")),
                r(RootKind::HomeDot, home.to_path_buf()),
                r(RootKind::SystemCache, app.join("Local").join("Temp")),
            ]
        }
        System::Mac => {
            let lib = home.join("Library");
            vec![
                r(
                    RootKind::ApplicationSupport,
                    lib.join("Application Support"),
                ),
                r(RootKind::Containers, lib.join("Containers")),
                r(RootKind::GroupContainers, lib.join("Group Containers")),
                r(RootKind::Preferences, lib.join("Preferences")),
                r(RootKind::HomeDot, home.to_path_buf()),
                r(RootKind::SystemCache, lib.join("Caches")),
            ]
        }
        System::Linux => {
            let x = |var: &str, dflt: PathBuf| xdg(var).filter(|p| p.is_absolute()).unwrap_or(dflt);
            vec![
                r(
                    RootKind::XdgConfig,
                    x("XDG_CONFIG_HOME", home.join(".config")),
                ),
                r(
                    RootKind::XdgData,
                    x("XDG_DATA_HOME", home.join(".local").join("share")),
                ),
                r(
                    RootKind::XdgState,
                    x("XDG_STATE_HOME", home.join(".local").join("state")),
                ),
                r(RootKind::Flatpak, home.join(".var").join("app")),
                r(RootKind::Snap, home.join("snap")),
                r(RootKind::HomeDot, home.to_path_buf()),
                r(
                    RootKind::SystemCache,
                    x("XDG_CACHE_HOME", home.join(".cache")),
                ),
            ]
        }
    }
}

/// The `XDG_*_HOME` names [`roots_of`] asks for.
pub const XDG_NAMES: [&str; 4] = [
    "XDG_CONFIG_HOME",
    "XDG_DATA_HOME",
    "XDG_STATE_HOME",
    "XDG_CACHE_HOME",
];

/// The largest `environment.d` or start-up file read.
const MAX_ENV_FILE: u64 = 64 * 1024;
/// The most `environment.d` files read.
const MAX_ENV_FILES: usize = 64;

/// A Linux person's own `XDG_*_HOME` settings, for the home folder `home` (CI review, decided 28
/// October 2026: each person's own, never the mover's for everyone).
///
/// - **The person PCTwin runs as** (`home` is `session_home`, this program's own home): this
///   session's values (`session`), which are theirs.
/// - **Anyone else**: their own declared settings, the systemd per-person environment files
///   `~/.config/environment.d/*.conf` (plain `NAME=value` lines, read in name order, a later line
///   winning; `$HOME` and `${HOME}` stand for their home). Only absolute results count.
///
/// The second answer lists that person's shell start-up files that mention one of these names: a
/// value set there is code PCTwin never runs, so where it points is not checked, and the report
/// says so ([`find_folders_for`]). A file that is a link or too large is never read.
pub fn xdg_of(
    home: &Path,
    session_home: Option<&Path>,
    session: &dyn Fn(&str) -> Option<PathBuf>,
) -> (std::collections::BTreeMap<String, PathBuf>, Vec<PathBuf>) {
    let mut out = std::collections::BTreeMap::new();
    if session_home == Some(home) {
        for n in XDG_NAMES {
            if let Some(p) = session(n).filter(|p| p.is_absolute()) {
                out.insert(n.to_string(), p);
            }
        }
        return (out, Vec::new());
    }
    let small = |p: &Path| -> Option<String> {
        let m = std::fs::symlink_metadata(p).ok()?;
        if !m.is_file() || m.len() > MAX_ENV_FILE {
            return None;
        }
        std::fs::read_to_string(p).ok()
    };
    let dir = home.join(".config").join("environment.d");
    let mut files: Vec<PathBuf> = std::fs::symlink_metadata(&dir)
        .ok()
        .filter(std::fs::Metadata::is_dir)
        .and_then(|_| std::fs::read_dir(&dir).ok())
        .map(|rd| {
            rd.flatten()
                .map(|e| e.path())
                .filter(|p| p.extension().is_some_and(|e| e == "conf"))
                .collect()
        })
        .unwrap_or_default();
    files.sort();
    files.truncate(MAX_ENV_FILES);
    let home_text = home.to_string_lossy();
    for f in files {
        let Some(text) = small(&f) else { continue };
        for line in text.lines().map(str::trim) {
            let Some((name, value)) = line.split_once('=') else {
                continue;
            };
            let name = name.trim();
            if line.starts_with('#') || !XDG_NAMES.contains(&name) {
                continue;
            }
            let value = value.trim().trim_matches('"');
            let value = if let Some(rest) = value.strip_prefix("${HOME}") {
                format!("{home_text}{rest}")
            } else if let Some(rest) = value.strip_prefix("$HOME") {
                format!("{home_text}{rest}")
            } else {
                value.to_string()
            };
            // Any other `$` is a value PCTwin does not know for this person: the standard place.
            let p = PathBuf::from(&value);
            if value.contains('$') || !p.is_absolute() {
                out.remove(name);
            } else {
                out.insert(name.to_string(), p);
            }
        }
    }
    let shell = [
        ".profile",
        ".bash_profile",
        ".bash_login",
        ".bashrc",
        ".zshenv",
        ".zprofile",
        ".zshrc",
        ".pam_environment",
    ];
    let mentioned = shell
        .iter()
        .map(|n| home.join(n))
        .filter(|p| small(p).is_some_and(|t| XDG_NAMES.iter().any(|n| t.contains(n))))
        .collect();
    (out, mentioned)
}

/// [`roots_of`] and [`find_folders`] for the person with home folder `home` on `system`, with
/// that person's own `XDG_*_HOME` settings on Linux ([`xdg_of`]): a start-up file that may move
/// one of their folders elsewhere is said in the coverage as not checked, never silently assumed.
pub fn find_folders_for(
    system: System,
    owner: &Owner,
    home: &Path,
    apps: &[FoundApp],
    session_home: Option<&Path>,
    session: &dyn Fn(&str) -> Option<PathBuf>,
) -> (Vec<AppDataFolder>, Coverage) {
    let (xdg, mentioned) = if system == System::Linux {
        xdg_of(home, session_home, session)
    } else {
        Default::default()
    };
    let roots = roots_of(system, home, &|v| xdg.get(v).cloned());
    let (folders, mut cov) = find_folders(owner, home, &roots, apps);
    for f in mentioned {
        cov.unreadable.push((
            f,
            "this person's start-up file may keep app data somewhere else; PCTwin does not run it, so that place is not checked".into(),
        ));
    }
    (folders, cov)
}

/// How a folder was tied to an app.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum Tie {
    /// The system's own fact: the folder is the app's package, bundle, Flatpak or Snap home.
    Owned,
    /// The system's own fact about the maker: a group container of the maker's Team ID.
    MakersGroup,
    /// A clue only: the folder is named as the old laptop recorded the app or its maker.
    NamedLike,
}

/// One app data folder found.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AppDataFolder {
    pub root: RootKind,
    /// The root's path.
    pub root_path: PathBuf,
    /// The folder's path inside the root (`/` between folders; one or two names).
    pub rel: String,
    pub owner: Owner,
    /// The app it belongs to (its place in the app list), when a fact or clue ties it.
    pub app: Option<usize>,
    pub tie: Option<Tie>,
}

impl AppDataFolder {
    pub fn path(&self) -> PathBuf {
        self.rel
            .split('/')
            .fold(self.root_path.clone(), |p, part| p.join(part))
    }

    /// A key that names the folder for the person and the catalogue: the root kind and the path
    /// inside it.
    pub fn place(&self) -> String {
        format!(
            "{}/{}",
            serde_json::to_value(self.root)
                .ok()
                .and_then(|v| v.as_str().map(str::to_string))
                .unwrap_or_default(),
            self.rel
        )
    }
}

/// What was searched and what could not be.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct Coverage {
    /// Roots read.
    pub searched: Vec<(RootKind, PathBuf)>,
    /// Roots that do not exist on this laptop.
    pub absent: Vec<(RootKind, PathBuf)>,
    /// Places that could not be read, and why.
    pub unreadable: Vec<(PathBuf, String)>,
    /// Folders not looked into because they hold the system's own caches.
    pub caches: Vec<PathBuf>,
    /// Folders not looked into because they are links, kept online by a cloud app, or not
    /// folders at all, and why.
    pub skipped: Vec<(PathBuf, String)>,
}

fn not_looked_into(kind: crate::guard::Kind) -> Option<&'static str> {
    use crate::guard::Kind;
    match kind {
        Kind::Link => Some("a link to somewhere else: not followed"),
        Kind::OnlineOnly => Some("kept online by a cloud app: not downloaded or looked into"),
        Kind::NotAFile => Some("not a file or folder"),
        Kind::File | Kind::Folder => None,
    }
}

/// Why a start-up place is not moved.
pub const STARTS_AT_SIGN_IN: &str = "programs or commands that start when you sign in: PCTwin never turns these on for you; set up the ones you want yourself";

/// What a place in a person's home is to the system ([`TRUSTED_PLACES`]).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Trusted {
    /// The system or the desktop runs what is there at sign-in, or at every shell's start, or
    /// when a file of some kind is opened (T26-026: a moved setting must never switch a command
    /// on). Never found as app data, never landed.
    SignIn,
    /// The system trusts what is there: who may sign in remotely, the keys that sign and decrypt,
    /// Windows' own secrets. Found and moved (the person's own keys), but only ever kept aside,
    /// never landed where the system would trust it.
    Trust,
}

/// Every place in a person's home that the system runs at sign-in or trusts, home-relative as
/// each system lays out a home by default (`/` between names; compared ignoring letter case). One
/// table, the system's own rules (the deny region, never a catalogue row): the old laptop's scan
/// leaves out what is here ([`starts_at_sign_in`]), the new laptop never places app data here
/// (`classify::placement_of`), and the landing refuses to write here or into a folder holding
/// one of these (`land`), whoever asks.
pub const TRUSTED_PLACES: &[(&str, Trusted)] = &[
    // Linux: the freedesktop autostart folder, systemd's per-person services and environment,
    // KDE's sign-in scripts and environment, D-Bus services and desktop entries (run when asked
    // for, or to open a file), and shell start-up folders.
    (".config/autostart", Trusted::SignIn),
    (".config/autostart-scripts", Trusted::SignIn),
    (".config/systemd", Trusted::SignIn),
    (".config/environment.d", Trusted::SignIn),
    (".config/plasma-workspace", Trusted::SignIn),
    (".config/fish", Trusted::SignIn),
    (".local/share/systemd", Trusted::SignIn),
    (".local/share/dbus-1", Trusted::SignIn),
    (".local/share/applications", Trusted::SignIn),
    (".kde/Autostart", Trusted::SignIn),
    (".kde/env", Trusted::SignIn),
    (".kde4/Autostart", Trusted::SignIn),
    (".kde4/env", Trusted::SignIn),
    (".bashrc.d", Trusted::SignIn),
    (".profile.d", Trusted::SignIn),
    (".zshrc.d", Trusted::SignIn),
    // macOS: what launchd starts at sign-in, and the sign-in window's own hooks.
    ("Library/LaunchAgents", Trusted::SignIn),
    ("Library/Preferences/loginwindow.plist", Trusted::SignIn),
    (
        "Library/Preferences/com.apple.loginwindow.plist",
        Trusted::SignIn,
    ),
    // Windows: the Start menu (its Startup folder) and the rest of Windows' own per-person tree.
    ("AppData/Roaming/Microsoft/Windows", Trusted::SignIn),
    ("AppData/Local/Microsoft/Windows", Trusted::SignIn),
    // Trusted: remote sign-in and keys, certificates, Windows' own protected secrets.
    (".ssh", Trusted::Trust),
    (".gnupg", Trusted::Trust),
    (".pki", Trusted::Trust),
    ("AppData/Roaming/Microsoft/Protect", Trusted::Trust),
    ("AppData/Roaming/Microsoft/Credentials", Trusted::Trust),
    ("AppData/Roaming/Microsoft/Crypto", Trusted::Trust),
    (
        "AppData/Roaming/Microsoft/SystemCertificates",
        Trusted::Trust,
    ),
    ("AppData/Local/Microsoft/Credentials", Trusted::Trust),
];

/// The names of a home-relative path, for comparing (letter case ignored).
fn names_of(path: &str) -> Vec<String> {
    path.split(['/', '\\'])
        .filter(|p| !p.is_empty() && *p != ".")
        .map(str::to_lowercase)
        .collect()
}

/// What the place at the home-relative `path` (or anything in it) is to the system: the place
/// itself or a folder inside one ([`TRUSTED_PLACES`]), or, with `holding`, also a folder that
/// holds one (landing `.config` whole would land its `autostart`). `None`: none of these.
pub fn trusted_place(path: &str, holding: bool) -> Option<Trusted> {
    let have = names_of(path);
    TRUSTED_PLACES.iter().find_map(|(place, what)| {
        let want = names_of(place);
        let within = have.len() >= want.len() && have[..want.len()] == want[..];
        let holds = holding && want.len() > have.len() && want[..have.len()] == have[..];
        (within || holds).then_some(*what)
    })
}

/// Where each root is in a home, as each system lays it out by default (home-relative).
pub fn default_places(kind: RootKind) -> &'static [&'static str] {
    match kind {
        RootKind::Roaming => &["AppData/Roaming"],
        RootKind::Local => &["AppData/Local"],
        RootKind::LocalLow => &["AppData/LocalLow"],
        RootKind::Packages => &["AppData/Local/Packages"],
        RootKind::SavedGames => &["Saved Games"],
        RootKind::ApplicationSupport => &["Library/Application Support"],
        RootKind::Containers => &["Library/Containers"],
        RootKind::GroupContainers => &["Library/Group Containers"],
        RootKind::Preferences => &["Library/Preferences"],
        RootKind::XdgConfig => &[".config"],
        RootKind::XdgData => &[".local/share"],
        RootKind::XdgState => &[".local/state"],
        RootKind::Flatpak => &[".var/app"],
        RootKind::Snap => &["snap"],
        RootKind::HomeDot => &[""],
        RootKind::SystemCache => &["AppData/Local/Temp", "Library/Caches", ".cache"],
        RootKind::MacMail => &["Library/Mail"],
    }
}

/// What `folder` (one or two names) in `root` is to the system, wherever that root is in a home
/// ([`trusted_place`]).
pub fn trusted_in(root: RootKind, folder: &str, holding: bool) -> Option<Trusted> {
    default_places(root).iter().find_map(|base| {
        let path = if base.is_empty() {
            folder.to_string()
        } else {
            format!("{base}/{folder}")
        };
        trusted_place(&path, holding)
    })
}

/// Whether `name` in `root` is a place the system or the desktop runs from at sign-in (T26-026:
/// a moved setting must never switch a command on): [`TRUSTED_PLACES`]' sign-in places, and
/// anything inside one.
pub fn starts_at_sign_in(root: RootKind, name: &str) -> bool {
    trusted_in(root, name, false) == Some(Trusted::SignIn)
}

/// Lower-case letters and digits only: how names are compared with what the old laptop recorded.
fn squash(s: &str) -> String {
    s.chars()
        .filter(char::is_ascii_alphanumeric)
        .map(|c| c.to_ascii_lowercase())
        .collect()
}

/// The app (by its place in `apps`) whose own facts tie a folder `name` in a root of `kind`.
fn owned_by(kind: RootKind, name: &str, apps: &[FoundApp]) -> Option<(usize, Tie)> {
    let eq = |a: &str| a.eq_ignore_ascii_case(name);
    let stem = name.strip_suffix(".plist").unwrap_or(name);
    for (i, app) in apps.iter().enumerate() {
        let hit = match (&app.found, kind) {
            (FoundWhere::WindowsPackage { family_name, .. }, RootKind::Packages) => eq(family_name),
            (
                FoundWhere::MacBundle { bundle_id, .. },
                RootKind::Containers | RootKind::ApplicationSupport,
            ) => eq(bundle_id),
            (FoundWhere::MacBundle { bundle_id, .. }, RootKind::Preferences) => {
                stem.eq_ignore_ascii_case(bundle_id)
                    || stem
                        .to_ascii_lowercase()
                        .starts_with(&format!("{}.", bundle_id.to_ascii_lowercase()))
            }
            (FoundWhere::Flatpak { app_id, .. }, RootKind::Flatpak) => eq(app_id),
            (FoundWhere::Snap { name: snap }, RootKind::Snap) => eq(snap),
            _ => false,
        };
        if hit {
            return Some((i, Tie::Owned));
        }
    }
    if kind == RootKind::GroupContainers {
        let team = name.split('.').next().unwrap_or("");
        for (i, app) in apps.iter().enumerate() {
            if let AppMaker::MacTeam { team_id } = &app.maker
                && !team_id.is_empty()
                && team.eq_ignore_ascii_case(team_id)
            {
                return Some((i, Tie::MakersGroup));
            }
        }
    }
    None
}

/// The app a folder `name` is named like, as the old laptop recorded it (its name, its maker's
/// recorded name, or its package name). A clue, never proof.
fn named_like(name: &str, apps: &[FoundApp]) -> Option<usize> {
    let n = squash(name);
    if n.len() < 3 {
        return None;
    }
    apps.iter().position(|app| {
        let package = match &app.found {
            FoundWhere::Deb { package } | FoundWhere::Rpm { package } => Some(package.as_str()),
            _ => None,
        };
        [Some(app.name.as_str()), app.publisher.as_deref(), package]
            .into_iter()
            .flatten()
            .any(|recorded| squash(recorded) == n)
    })
}

fn listing(dir: &Path, cov: &mut Coverage) -> Vec<(String, std::fs::Metadata)> {
    let entries = match std::fs::read_dir(dir) {
        Ok(e) => e,
        Err(e) => {
            cov.unreadable
                .push((dir.to_path_buf(), e.kind().to_string()));
            return Vec::new();
        }
    };
    let mut out = Vec::new();
    for e in entries {
        let Ok(e) = e else { continue };
        let Ok(name) = e.file_name().into_string() else {
            cov.unreadable
                .push((e.path(), "a name the system cannot spell".into()));
            continue;
        };
        if let Ok(m) = std::fs::symlink_metadata(e.path()) {
            out.push((name, m));
        }
    }
    out.sort_by(|a, b| a.0.cmp(&b.0));
    out
}

/// The folder names the other roots of `roots` take inside the home folder (so the home's own
/// hidden folders do not list them twice).
fn inside_home(home: &Path, roots: &[DataRoot]) -> Vec<String> {
    roots
        .iter()
        .filter(|r| r.kind != RootKind::HomeDot)
        .filter_map(|r| r.path.strip_prefix(home).ok())
        .filter_map(|p| p.components().next())
        .map(|c| c.as_os_str().to_string_lossy().to_ascii_lowercase())
        .collect()
}

/// Finds every app data folder of `owner` in `roots` (from [`roots_of`] for the person's `home`),
/// ties each to an app in `apps` where the system's facts or the recorded names allow, and says
/// what was searched. Never follows a link. Windows' `Roaming`, `Local` and `LocalLow` often hold
/// a maker's folder with its apps inside: a maker's folder whose name ties no app is looked into
/// once more, and each folder inside that does tie an app is its own app data folder.
pub fn find_folders(
    owner: &Owner,
    home: &Path,
    roots: &[DataRoot],
    apps: &[FoundApp],
) -> (Vec<AppDataFolder>, Coverage) {
    let mut cov = Coverage::default();
    let mut out = Vec::new();
    let taken = inside_home(home, roots);
    for root in roots {
        let Ok(meta) = std::fs::symlink_metadata(&root.path) else {
            cov.absent.push((root.kind, root.path.clone()));
            continue;
        };
        if let Some(why) = not_looked_into(crate::guard::kind_of(&meta)) {
            cov.skipped.push((root.path.clone(), why.to_string()));
            continue;
        }
        if !meta.is_dir() {
            cov.absent.push((root.kind, root.path.clone()));
            continue;
        }
        if root.kind.is_cache() {
            cov.caches.push(root.path.clone());
            continue;
        }
        cov.searched.push((root.kind, root.path.clone()));
        for (name, m) in listing(&root.path, &mut cov) {
            if let Some(why) = not_looked_into(crate::guard::kind_of(&m)) {
                // A home holds many links of its own; only those that would have been app
                // folders are reported.
                if root.kind != RootKind::HomeDot || name.starts_with('.') {
                    cov.skipped.push((root.path.join(&name), why.to_string()));
                }
                continue;
            }
            let is_dir = m.is_dir();
            match root.kind {
                // Only hidden folders of the home itself, and not the other roots.
                RootKind::HomeDot => {
                    if !is_dir || !name.starts_with('.') || name.len() < 2 {
                        continue;
                    }
                    if taken.contains(&name.to_ascii_lowercase()) {
                        continue;
                    }
                }
                // Preferences are files (one property list per app); everything else is a folder.
                RootKind::Preferences => {
                    if !m.is_file() {
                        continue;
                    }
                }
                _ if !is_dir => continue,
                _ => {}
            }
            // What the system runs at sign-in (T26-026): never landed where it would run.
            if starts_at_sign_in(root.kind, &name) {
                cov.skipped
                    .push((root.path.join(&name), STARTS_AT_SIGN_IN.to_string()));
                continue;
            }
            // `Local\Packages` is a root of its own; `Local\Temp` is the cache root.
            if root.kind == RootKind::Local
                && roots
                    .iter()
                    .any(|r| r.kind != RootKind::Local && r.path == root.path.join(&name))
            {
                continue;
            }
            let folder = |rel: String, tied: Option<(usize, Tie)>| AppDataFolder {
                root: root.kind,
                root_path: root.path.clone(),
                rel,
                owner: owner.clone(),
                app: tied.as_ref().map(|t| t.0),
                tie: tied.map(|t| t.1),
            };
            let tied = owned_by(root.kind, &name, apps)
                .or_else(|| named_like(&name, apps).map(|i| (i, Tie::NamedLike)));
            let maker_folder = matches!(
                root.kind,
                RootKind::Roaming | RootKind::Local | RootKind::LocalLow
            );
            if tied.is_none() && maker_folder {
                // A maker's folder with its apps inside: each tied app's folder on its own.
                let inner: Vec<(String, usize)> = listing(&root.path.join(&name), &mut cov)
                    .into_iter()
                    .filter(|(_, m)| m.is_dir() && !m.file_type().is_symlink())
                    .filter_map(|(n, _)| named_like(&n, apps).map(|i| (n, i)))
                    .collect();
                if !inner.is_empty() {
                    for (n, i) in inner {
                        out.push(folder(format!("{name}/{n}"), Some((i, Tie::NamedLike))));
                    }
                    continue;
                }
            }
            out.push(folder(name, tied));
        }
    }
    (out, cov)
}

#[cfg(test)]
mod tests {
    use super::*;
    use pctwin_record::{AppArch, AppSystem, PackageSigning};

    fn app(name: &str, publisher: Option<&str>, found: FoundWhere, maker: AppMaker) -> FoundApp {
        FoundApp {
            name: name.into(),
            version: None,
            publisher: publisher.map(str::to_string),
            owner: Owner::Shared,
            system: AppSystem::Windows,
            found,
            arch: AppArch::default(),
            maker,
            web: None,
            update_feed: None,
            installers: Vec::new(),
            notes: Vec::new(),
        }
    }

    fn me() -> Owner {
        Owner::Person {
            account_id: "1000".into(),
        }
    }

    fn mk(p: &Path) {
        std::fs::create_dir_all(p).unwrap();
    }

    #[test]
    fn every_system_lists_its_own_roots_and_its_cache() {
        let home = Path::new("/h");
        let none = |_: &str| None;
        let w = roots_of(System::Windows, home, &none);
        assert!(w.iter().any(|r| r.kind == RootKind::Packages
            && r.path == home.join("AppData").join("Local").join("Packages")));
        assert_eq!(w.iter().filter(|r| r.kind.is_cache()).count(), 1);
        let m = roots_of(System::Mac, home, &none);
        assert!(m.iter().any(|r| r.kind == RootKind::GroupContainers));
        assert!(
            m.iter().any(|r| r.kind == RootKind::SystemCache
                && r.path == home.join("Library").join("Caches"))
        );
        let elsewhere = std::env::temp_dir().join("conf");
        let set = |v: &str| (v == "XDG_CONFIG_HOME").then(|| elsewhere.clone());
        let l = roots_of(System::Linux, home, &set);
        assert!(
            l.iter()
                .any(|r| r.kind == RootKind::XdgConfig && r.path == elsewhere)
        );
        assert!(l.iter().any(|r| r.kind == RootKind::Flatpak));
        // A relative value is not trusted.
        let rel = |_: &str| Some(PathBuf::from("relative"));
        assert!(
            roots_of(System::Linux, home, &rel)
                .iter()
                .all(|r| r.path.starts_with(home))
        );
        assert_eq!(
            System::here(),
            if cfg!(windows) {
                System::Windows
            } else if cfg!(target_os = "macos") {
                System::Mac
            } else {
                System::Linux
            }
        );
    }

    #[test]
    fn folders_are_tied_by_the_systems_own_facts_and_others_are_still_found() {
        let dir = tempfile::tempdir().unwrap();
        let home = dir.path();
        let apps = vec![
            app(
                "Notes App",
                Some("Synthetic Maker"),
                FoundWhere::WindowsPackage {
                    family_name: "SyntheticMaker.NotesApp_abc123".into(),
                    signing: PackageSigning::Store,
                },
                AppMaker::WindowsPackage {
                    publisher: "CN=Synthetic".into(),
                },
            ),
            app(
                "Mail Thing",
                Some("Synthetic Org"),
                FoundWhere::WindowsEntry {
                    view: pctwin_record::RegistryView::Machine,
                    key: "k".into(),
                    product_code: None,
                },
                AppMaker::Unsigned,
            ),
            app(
                "Mac Editor",
                None,
                FoundWhere::MacBundle {
                    bundle_id: "org.example.editor".into(),
                    folder: "Mac Editor.app".into(),
                    app_store_id: None,
                },
                AppMaker::MacTeam {
                    team_id: "ABCDE12345".into(),
                },
            ),
        ];
        let roots = roots_of(System::Windows, home, &|_| None);
        let app_data = home.join("AppData");
        mk(&app_data.join("Local/Packages/SyntheticMaker.NotesApp_abc123/LocalState"));
        mk(&app_data.join("Roaming/Synthetic Org"));
        mk(&app_data.join("Roaming/Big Vendor/Mail Thing"));
        mk(&app_data.join("Roaming/Big Vendor/Unrelated"));
        mk(&app_data.join("Roaming/Nobody Knows"));
        mk(&app_data.join("Local/Temp/junk"));
        mk(&app_data.join("LocalLow/Game Studio/Game"));
        mk(&home.join(".ssh"));
        mk(&home.join("Saved Games/Some Game"));
        std::fs::write(app_data.join("Roaming/loose.txt"), b"x").unwrap();
        let (found, cov) = find_folders(&me(), home, &roots, &apps);
        let by = |rel: &str| found.iter().find(|f| f.rel == rel).cloned();
        let pkg = by("SyntheticMaker.NotesApp_abc123").unwrap();
        assert_eq!(
            (pkg.root, pkg.app, pkg.tie),
            (RootKind::Packages, Some(0), Some(Tie::Owned))
        );
        // The maker's recorded name: a clue.
        assert_eq!(by("Synthetic Org").unwrap().tie, Some(Tie::NamedLike));
        // A maker's folder with an app's folder inside.
        let inner = by("Big Vendor/Mail Thing").unwrap();
        assert_eq!((inner.app, inner.tie), (Some(1), Some(Tie::NamedLike)));
        assert_eq!(
            inner.path(),
            app_data
                .join("Roaming")
                .join("Big Vendor")
                .join("Mail Thing")
        );
        assert!(
            by("Big Vendor").is_none(),
            "its tied app's folder stands for it"
        );
        // Found, with no app: still listed.
        let nobody = by("Nobody Knows").unwrap();
        assert_eq!((nobody.app, nobody.tie), (None, None));
        assert_eq!(nobody.place(), "roaming/Nobody Knows");
        assert!(by("Game Studio").is_some());
        assert!(by(".ssh").is_some_and(|f| f.root == RootKind::HomeDot));
        assert!(by("Some Game").is_some_and(|f| f.root == RootKind::SavedGames));
        assert!(by("loose.txt").is_none());
        // The home's own roots are not listed again as hidden folders; Packages and Temp are not
        // folders of Local.
        assert!(by("AppData").is_none() && by("Packages").is_none() && by("Temp").is_none());
        assert_eq!(cov.caches, vec![app_data.join("Local").join("Temp")]);
        assert!(cov.searched.iter().any(|(k, _)| *k == RootKind::Roaming));
        assert!(cov.unreadable.is_empty());

        // On a Mac: bundle ID, its preferences file, and the maker's group container.
        let mac = tempfile::tempdir().unwrap();
        let lib = mac.path().join("Library");
        mk(&lib.join("Containers/org.example.editor/Data"));
        mk(&lib.join("Group Containers/ABCDE12345.shared"));
        mk(&lib.join("Group Containers/ZZZZZ99999.other"));
        mk(&lib.join("Preferences"));
        std::fs::write(
            lib.join("Preferences/org.example.editor.plist"),
            b"bplist00",
        )
        .unwrap();
        std::fs::write(
            lib.join("Preferences/org.example.editor.helper.plist"),
            b"bplist00",
        )
        .unwrap();
        std::fs::write(
            lib.join("Preferences/org.example.editorx.plist"),
            b"bplist00",
        )
        .unwrap();
        let roots = roots_of(System::Mac, mac.path(), &|_| None);
        let (found, cov) = find_folders(&me(), mac.path(), &roots, &apps);
        let by = |rel: &str| found.iter().find(|f| f.rel == rel).cloned();
        assert_eq!(by("org.example.editor").unwrap().tie, Some(Tie::Owned));
        assert_eq!(by("ABCDE12345.shared").unwrap().tie, Some(Tie::MakersGroup));
        assert_eq!(by("ZZZZZ99999.other").unwrap().tie, None);
        assert_eq!(by("org.example.editor.plist").unwrap().app, Some(2));
        assert_eq!(by("org.example.editor.helper.plist").unwrap().app, Some(2));
        assert_eq!(by("org.example.editorx.plist").unwrap().app, None);
        assert!(
            cov.absent
                .iter()
                .any(|(k, _)| *k == RootKind::ApplicationSupport)
        );
    }

    #[test]
    fn linux_flatpak_and_snap_homes_belong_to_their_apps() {
        let dir = tempfile::tempdir().unwrap();
        let home = dir.path();
        let apps = vec![
            app(
                "Viewer",
                None,
                FoundWhere::Flatpak {
                    app_id: "org.example.Viewer".into(),
                    remote: "r".into(),
                    remote_url: None,
                    system_wide: false,
                },
                AppMaker::Unsigned,
            ),
            app(
                "Snappy",
                None,
                FoundWhere::Snap {
                    name: "snappy".into(),
                },
                AppMaker::Unsigned,
            ),
            app(
                "Tool",
                None,
                FoundWhere::Deb {
                    package: "synthetic-tool".into(),
                },
                AppMaker::Unsigned,
            ),
        ];
        mk(&home.join(".var/app/org.example.Viewer/config"));
        mk(&home.join("snap/snappy/common"));
        mk(&home.join(".config/synthetic-tool"));
        mk(&home.join(".cache/thing"));
        let (found, cov) = find_folders(
            &me(),
            home,
            &roots_of(System::Linux, home, &|_| None),
            &apps,
        );
        let by = |rel: &str| found.iter().find(|f| f.rel == rel).cloned();
        assert_eq!(by("org.example.Viewer").unwrap().app, Some(0));
        assert_eq!(by("snappy").unwrap().app, Some(1));
        assert_eq!(by("synthetic-tool").unwrap().tie, Some(Tie::NamedLike));
        // The other roots inside the home are not hidden app folders.
        assert!(by(".config").is_none() && by(".var").is_none() && by(".cache").is_none());
        assert!(by(".local").is_none());
        assert_eq!(cov.caches.len(), 1);
    }

    /// CI review (28 October 2026): every person's app data is looked for where their own
    /// settings put it, never where the mover's session does.
    #[test]
    fn each_person_s_own_xdg_settings_not_the_mover_s() {
        let base = tempfile::tempdir().unwrap();
        let mine = base.path().join("me");
        let theirs = base.path().join("them");
        mk(&mine);
        mk(&theirs.join(".config").join("environment.d"));
        let session_conf = base.path().join("session-conf");
        let session = |v: &str| (v == "XDG_CONFIG_HOME").then(|| session_conf.clone());
        // The mover's own home: this session's values.
        let (x, shell) = xdg_of(&mine, Some(&mine), &session);
        assert_eq!(x.get("XDG_CONFIG_HOME"), Some(&session_conf));
        assert!(shell.is_empty());
        // Another person: never the session's; their own environment.d, later files winning,
        // $HOME as theirs, anything unknown left at the standard place.
        std::fs::write(
            theirs.join(".config/environment.d/10-a.conf"),
            "# mine
XDG_DATA_HOME=/nowhere
XDG_STATE_HOME=$XDG_DATA_HOME/state
PATH=/bin
",
        )
        .unwrap();
        std::fs::write(
            theirs.join(".config/environment.d/20-b.conf"),
            "XDG_DATA_HOME=\"${HOME}/data\"
XDG_CACHE_HOME=relative
",
        )
        .unwrap();
        let (x, shell) = xdg_of(&theirs, Some(&mine), &session);
        assert_eq!(x.get("XDG_CONFIG_HOME"), None);
        assert_eq!(x.get("XDG_DATA_HOME"), Some(&theirs.join("data")));
        assert_eq!(x.get("XDG_STATE_HOME"), None);
        assert_eq!(x.get("XDG_CACHE_HOME"), None);
        assert!(shell.is_empty());
        // A value set in their shell start-up file is code PCTwin never runs: said, not checked.
        std::fs::write(
            theirs.join(".profile"),
            "export XDG_CONFIG_HOME=$HOME/cfg
",
        )
        .unwrap();
        let (_, shell) = xdg_of(&theirs, Some(&mine), &session);
        assert_eq!(shell, vec![theirs.join(".profile")]);
        mk(&theirs.join("data").join("someapp"));
        let (folders, cov) =
            find_folders_for(System::Linux, &me(), &theirs, &[], Some(&mine), &session);
        assert!(
            folders
                .iter()
                .any(|f| f.root == RootKind::XdgData && f.root_path == theirs.join("data"))
        );
        assert!(
            folders
                .iter()
                .all(|f| !f.root_path.starts_with(&session_conf)),
            "the mover's own settings are never used for another person"
        );
        assert!(
            cov.unreadable
                .iter()
                .any(|(p, _)| *p == theirs.join(".profile"))
        );
    }
}
