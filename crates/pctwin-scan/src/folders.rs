use std::path::{Component, Path, PathBuf, Prefix};

use pctwin_record::{CloudProvider, FolderRole, Storage};
use serde::Serialize;

/// The special folders looked for, in the order they are reported.
const ROLES: [FolderRole; 8] = [
    FolderRole::Home,
    FolderRole::Desktop,
    FolderRole::Documents,
    FolderRole::Downloads,
    FolderRole::Pictures,
    FolderRole::Music,
    FolderRole::Videos,
    FolderRole::Public,
];

/// What is known about the laptop when deciding where a folder really is.
#[derive(Debug, Clone)]
pub struct Facts {
    pub home: PathBuf,
    /// The drive the system runs from (`C:`), on Windows.
    pub system_drive: Option<String>,
    /// Cloud storage folders found on the laptop.
    pub cloud_roots: Vec<(CloudProvider, PathBuf)>,
    /// macOS: iCloud keeps Desktop and Documents (their path does not change).
    pub icloud_desktop_and_documents: bool,
}

/// A special folder the system reported, where it really is.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct FoundFolder {
    pub role: FolderRole,
    /// The folder as the system reports it.
    pub path: PathBuf,
    pub storage: Storage,
    /// True when the folder is not where its plain name would put it (`home/Documents`).
    pub moved: bool,
}

/// The answer for one special folder.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(tag = "status", rename_all = "kebab-case")]
pub enum FolderLookup {
    Found(FoundFolder),
    /// The system has no such folder, or the one it names is not there.
    Missing {
        role: FolderRole,
    },
}

impl FolderLookup {
    pub fn role(&self) -> FolderRole {
        match self {
            Self::Found(f) => f.role,
            Self::Missing { role } => *role,
        }
    }
}

/// Where `path` really is. Cloud folders win over drives, and the innermost cloud folder wins.
pub fn classify(path: &Path, facts: &Facts) -> Storage {
    let cloud = facts
        .cloud_roots
        .iter()
        .filter(|(_, root)| is_within(path, root))
        .max_by_key(|(_, root)| root.components().count());
    if let Some((provider, _)) = cloud {
        return Storage::Cloud {
            provider: *provider,
        };
    }
    if let (Some(drive), Some(system)) = (drive_of(path), &facts.system_drive)
        && !drive.eq_ignore_ascii_case(system)
    {
        return Storage::OtherDrive { drive };
    }
    Storage::SystemDrive
}

/// The drive (`D:`) or network share (`\\server\share`) a Windows path is on.
fn drive_of(path: &Path) -> Option<String> {
    match path.components().next()? {
        Component::Prefix(p) => match p.kind() {
            Prefix::Disk(l) | Prefix::VerbatimDisk(l) => {
                Some(format!("{}:", char::from(l).to_ascii_uppercase()))
            }
            Prefix::UNC(server, share) | Prefix::VerbatimUNC(server, share) => Some(format!(
                r"\\{}\{}",
                server.to_string_lossy(),
                share.to_string_lossy()
            )),
            _ => None,
        },
        _ => None,
    }
}

/// Windows and (by default) macOS ignore case in names; Linux does not.
const IGNORES_CASE: bool = cfg!(any(windows, target_os = "macos"));

/// The parts of a path, compared the way the system compares them. `\\?\C:` and `C:` are one
/// drive.
fn parts(path: &Path) -> Vec<String> {
    path.components()
        .filter(|c| !matches!(c, Component::CurDir))
        .map(|c| {
            let text = match c {
                Component::Prefix(_) => drive_of(path).unwrap_or_default(),
                other => other.as_os_str().to_string_lossy().into_owned(),
            };
            if IGNORES_CASE {
                text.to_lowercase()
            } else {
                text
            }
        })
        .collect()
}

/// Whether `path` is `root` or inside it, whole name by whole name (`OneDriveOld` is not inside
/// `OneDrive`).
pub(crate) fn is_within(path: &Path, root: &Path) -> bool {
    let (p, r) = (parts(path), parts(root));
    !r.is_empty() && p.len() >= r.len() && p[..r.len()] == r[..]
}

pub(crate) fn same_place(a: &Path, b: &Path) -> bool {
    parts(a) == parts(b)
}

/// Facts about this laptop, read from the system.
pub fn facts_from_system() -> Facts {
    let home = dirs::home_dir().unwrap_or_default();
    let mut cloud_roots = Vec::new();
    let mut add = |provider, path: PathBuf| {
        if path.is_dir() {
            cloud_roots.push((provider, real(&path)));
        }
    };
    if cfg!(windows) {
        for var in ["OneDrive", "OneDriveConsumer", "OneDriveCommercial"] {
            if let Some(path) = std::env::var_os(var) {
                add(CloudProvider::OneDrive, PathBuf::from(path));
            }
        }
        // Each signed-in OneDrive account (Personal, Business1, ...) records its folder here;
        // the environment variables above are only set when the person signs in.
        for path in registry_values(r"HKCU\Software\Microsoft\OneDrive\Accounts", "UserFolder") {
            add(CloudProvider::OneDrive, path);
        }
        add(CloudProvider::ICloud, home.join("iCloudDrive"));
        for base in [dirs::data_local_dir(), dirs::data_dir()]
            .into_iter()
            .flatten()
        {
            for path in dropbox_folders(&base.join("Dropbox").join("info.json")) {
                add(CloudProvider::Dropbox, path);
            }
        }
    } else if cfg!(target_os = "macos") {
        add(
            CloudProvider::ICloud,
            home.join("Library/Mobile Documents/com~apple~CloudDocs"),
        );
        if let Ok(entries) = std::fs::read_dir(home.join("Library/CloudStorage")) {
            for entry in entries.flatten() {
                let name = entry.file_name().to_string_lossy().into_owned();
                let provider = if name.starts_with("OneDrive") {
                    CloudProvider::OneDrive
                } else if name.starts_with("Dropbox") {
                    CloudProvider::Dropbox
                } else if name.starts_with("GoogleDrive") {
                    CloudProvider::GoogleDrive
                } else {
                    CloudProvider::Other
                };
                add(provider, entry.path());
            }
        }
    }
    if !cfg!(windows) {
        for path in dropbox_folders(&home.join(".dropbox").join("info.json")) {
            add(CloudProvider::Dropbox, path);
        }
        add(CloudProvider::Dropbox, home.join("Dropbox"));
    }
    Facts {
        icloud_desktop_and_documents: icloud_desktop_and_documents(&home),
        system_drive: if cfg!(windows) {
            std::env::var("SystemDrive").ok()
        } else {
            None
        },
        home,
        cloud_roots,
    }
}

/// Every `name` value under `key` and its subkeys, read with the system's own `reg` tool.
fn registry_values(key: &str, name: &str) -> Vec<PathBuf> {
    let Ok(out) = std::process::Command::new("reg")
        .args(["query", key, "/s", "/v", name])
        .output()
    else {
        return Vec::new();
    };
    parse_reg_values(&String::from_utf8_lossy(&out.stdout), name)
}

/// Reads `    UserFolder    REG_SZ    C:\Users\Ada\OneDrive` lines from `reg query` output.
fn parse_reg_values(output: &str, name: &str) -> Vec<PathBuf> {
    output
        .lines()
        .filter_map(|line| {
            let rest = line.trim_start().strip_prefix(name)?;
            let rest = rest.trim_start();
            let value = rest
                .strip_prefix("REG_EXPAND_SZ")
                .or_else(|| rest.strip_prefix("REG_SZ"))?;
            let value = value.trim();
            (!value.is_empty()).then(|| PathBuf::from(value))
        })
        .collect()
}

/// macOS: whether iCloud "Desktop & Documents Folders" is on. The folders keep their usual path
/// (`~/Documents`), so the path alone cannot show it. Finder's own setting says so; failing that,
/// iCloud Drive holds a link named Documents pointing at the Documents folder.
fn icloud_desktop_and_documents(home: &Path) -> bool {
    if !cfg!(target_os = "macos") {
        return false;
    }
    let setting = std::process::Command::new("defaults")
        .args(["read", "com.apple.finder", "FXICloudDriveDesktop"])
        .output()
        .ok()
        .filter(|o| o.status.success())
        .map(|o| String::from_utf8_lossy(&o.stdout).trim().to_string());
    if setting.as_deref() == Some("1") {
        return true;
    }
    let link = home.join("Library/Mobile Documents/com~apple~CloudDocs/Documents");
    std::fs::symlink_metadata(&link).is_ok_and(|m| m.file_type().is_symlink())
        && same_place(&real(&link), &real(&home.join("Documents")))
}

/// The folders Dropbox's own settings file lists (`{"personal": {"path": ...}, "business": ...}`).
fn dropbox_folders(info: &Path) -> Vec<PathBuf> {
    let Ok(text) = std::fs::read(info) else {
        return Vec::new();
    };
    let Ok(value) = serde_json::from_slice::<serde_json::Value>(&text) else {
        return Vec::new();
    };
    value
        .as_object()
        .into_iter()
        .flat_map(|accounts| accounts.values())
        .filter_map(|account| account.get("path")?.as_str().map(PathBuf::from))
        .collect()
}

/// On Mac and Linux, follows links so a folder linked into iCloud is placed by where it really
/// is. On Windows the known-folder answer is used as given, so a drive letter stays a drive letter.
fn real(path: &Path) -> PathBuf {
    if cfg!(windows) {
        path.to_path_buf()
    } else {
        std::fs::canonicalize(path).unwrap_or_else(|_| path.to_path_buf())
    }
}

/// Where the folder would be if it had never been moved or renamed.
fn plain_place(role: FolderRole, facts: &Facts) -> PathBuf {
    let home = &facts.home;
    match role {
        FolderRole::Home => home.clone(),
        FolderRole::Desktop => home.join("Desktop"),
        FolderRole::Documents => home.join("Documents"),
        FolderRole::Downloads => home.join("Downloads"),
        FolderRole::Pictures => home.join("Pictures"),
        FolderRole::Music => home.join("Music"),
        FolderRole::Videos if cfg!(target_os = "macos") => home.join("Movies"),
        FolderRole::Videos => home.join("Videos"),
        FolderRole::Public => match (&facts.system_drive, cfg!(windows)) {
            (Some(drive), true) => PathBuf::from(format!(r"{drive}\Users\Public")),
            _ => home.join("Public"),
        },
        _ => home.clone(),
    }
}

/// What the system says about one special folder.
#[derive(Debug)]
enum Answer {
    At(PathBuf),
    /// Deliberately switched off (Linux user-dirs points it at the home folder).
    SwitchedOff,
    /// The system has nothing to say (Linux without user-dirs settings).
    NotSet,
}

fn reported(role: FolderRole, facts: &Facts) -> Answer {
    match system_path(role) {
        Some(p) if role != FolderRole::Home && same_place(&p, &facts.home) => Answer::SwitchedOff,
        Some(p) => Answer::At(p),
        // The dirs library reports a switched-off folder the same way as an unset one, so the
        // Linux settings file is read to tell them apart.
        None if switched_off_on_linux(role, &facts.home) => Answer::SwitchedOff,
        None => Answer::NotSet,
    }
}

/// The Linux user-dirs key for a role (`XDG_<KEY>_DIR`).
fn user_dirs_key(role: FolderRole) -> Option<&'static str> {
    Some(match role {
        FolderRole::Desktop => "DESKTOP",
        FolderRole::Documents => "DOCUMENTS",
        FolderRole::Downloads => "DOWNLOAD",
        FolderRole::Pictures => "PICTURES",
        FolderRole::Music => "MUSIC",
        FolderRole::Videos => "VIDEOS",
        FolderRole::Public => "PUBLICSHARE",
        _ => return None,
    })
}

fn switched_off_on_linux(role: FolderRole, home: &Path) -> bool {
    if cfg!(any(windows, target_os = "macos")) {
        return false;
    }
    let Some(key) = user_dirs_key(role) else {
        return false;
    };
    let config = std::env::var_os("XDG_CONFIG_HOME")
        .map(PathBuf::from)
        .filter(|p| p.is_absolute())
        .unwrap_or_else(|| home.join(".config"));
    std::fs::read(config.join("user-dirs.dirs"))
        .is_ok_and(|bytes| switched_off_in(&String::from_utf8_lossy(&bytes), key, home))
}

/// Whether the user-dirs text points `XDG_<key>_DIR` at the home folder, which the user-dirs
/// rules define as "switched off".
fn switched_off_in(text: &str, key: &str, home: &Path) -> bool {
    let name = format!("XDG_{key}_DIR");
    text.lines().any(|line| {
        let Some((k, v)) = line.split_once('=') else {
            return false;
        };
        if k.trim() != name {
            return false;
        }
        let v = v.trim().trim_matches('"');
        v == "$HOME" || v == "$HOME/" || same_place(Path::new(v), home)
    })
}

fn system_path(role: FolderRole) -> Option<PathBuf> {
    match role {
        FolderRole::Home => dirs::home_dir(),
        FolderRole::Desktop => dirs::desktop_dir(),
        FolderRole::Documents => dirs::document_dir(),
        FolderRole::Downloads => dirs::download_dir(),
        FolderRole::Pictures => dirs::picture_dir(),
        FolderRole::Music => dirs::audio_dir(),
        FolderRole::Videos => dirs::video_dir(),
        FolderRole::Public => dirs::public_dir(),
        _ => None,
    }
}

/// Where each special folder of the signed-in person really is.
pub fn find_special_folders() -> Vec<FolderLookup> {
    let facts = facts_from_system();
    ROLES
        .iter()
        .map(|&role| look_up(role, reported(role, &facts), &facts))
        .collect()
}

fn look_up(role: FolderRole, answer: Answer, facts: &Facts) -> FolderLookup {
    let plain = plain_place(role, facts);
    let path = match answer {
        Answer::At(p) => Some(p),
        Answer::SwitchedOff => None,
        // Not configured at all (Linux without user-dirs): the plain name, only if it exists.
        Answer::NotSet => Some(plain.clone()),
    };
    match path {
        Some(path) if path.is_dir() => {
            let storage = storage_of(role, &path, facts);
            FolderLookup::Found(FoundFolder {
                role,
                moved: !same_place(&path, &plain),
                storage,
                path,
            })
        }
        _ => FolderLookup::Missing { role },
    }
}

fn storage_of(role: FolderRole, path: &Path, facts: &Facts) -> Storage {
    if matches!(role, FolderRole::Desktop | FolderRole::Documents)
        && facts.icloud_desktop_and_documents
    {
        return Storage::Cloud {
            provider: CloudProvider::ICloud,
        };
    }
    let storage = classify(&real(path), facts);
    #[cfg(unix)]
    if storage == Storage::SystemDrive
        && let Some(mount) = other_device(path, &facts.home)
    {
        return Storage::OtherDrive {
            drive: mount.to_string_lossy().into_owned(),
        };
    }
    storage
}

/// On Mac and Linux, the mount point of `path` when it is on a different device from the home
/// folder.
#[cfg(unix)]
fn other_device(path: &Path, home: &Path) -> Option<PathBuf> {
    use std::os::unix::fs::MetadataExt;
    let dev = |p: &Path| std::fs::metadata(p).ok().map(|m| m.dev());
    let here = dev(path)?;
    if dev(home)? == here {
        return None;
    }
    let mut mount = real(path);
    while let Some(parent) = mount.parent() {
        if dev(parent) != Some(here) {
            break;
        }
        mount = parent.to_path_buf();
    }
    Some(mount)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn facts(home: &Path) -> Facts {
        Facts {
            home: home.to_path_buf(),
            system_drive: None,
            cloud_roots: Vec::new(),
            icloud_desktop_and_documents: false,
        }
    }

    #[test]
    fn a_folder_the_system_names_but_that_is_not_there_is_missing() {
        let home = tempfile::tempdir().unwrap();
        std::fs::create_dir(home.path().join("Documents")).unwrap();
        let f = facts(home.path());
        let missing = FolderLookup::Missing {
            role: FolderRole::Documents,
        };
        // The plain Documents folder exists, but the system says Documents is elsewhere.
        let named = home.path().join("Gone");
        assert_eq!(
            look_up(FolderRole::Documents, Answer::At(named), &f),
            missing
        );
        // Switched off: no such folder, even though the plain one exists.
        assert_eq!(
            look_up(FolderRole::Documents, Answer::SwitchedOff, &f),
            missing
        );
        // Not configured at all: the plain folder, because it exists.
        assert!(matches!(
            look_up(FolderRole::Documents, Answer::NotSet, &f),
            FolderLookup::Found(FoundFolder { moved: false, .. })
        ));
        assert_eq!(
            look_up(FolderRole::Pictures, Answer::NotSet, &f),
            FolderLookup::Missing {
                role: FolderRole::Pictures
            }
        );
    }

    #[test]
    fn a_user_dirs_entry_pointing_at_home_means_switched_off() {
        let home = Path::new("/home/ada");
        let text = "# written by xdg-user-dirs-update\nXDG_DESKTOP_DIR=\"$HOME/\"\nXDG_MUSIC_DIR=\"$HOME\"\nXDG_VIDEOS_DIR=\"/home/ada\"\nXDG_DOCUMENTS_DIR=\"$HOME/Dokumente\"\n";
        assert!(switched_off_in(text, "DESKTOP", home));
        assert!(switched_off_in(text, "MUSIC", home));
        assert!(switched_off_in(text, "VIDEOS", home));
        assert!(!switched_off_in(text, "DOCUMENTS", home));
        // Not mentioned at all is "not set", not "switched off".
        assert!(!switched_off_in(text, "PICTURES", home));
        assert!(!switched_off_in(
            "XDG_DESKTOP_DIRX=\"$HOME/\"",
            "DESKTOP",
            home
        ));
    }

    #[test]
    fn onedrive_folders_are_read_from_reg_output() {
        let output = "\r\nHKEY_CURRENT_USER\\Software\\Microsoft\\OneDrive\\Accounts\\Personal\r\n    UserFolder    REG_SZ    C:\\Users\\Ada\\OneDrive\r\n\r\nHKEY_CURRENT_USER\\Software\\Microsoft\\OneDrive\\Accounts\\Business1\r\n    UserFolder    REG_EXPAND_SZ    C:\\Users\\Ada\\OneDrive - Contoso\r\n    UserFolderOld    REG_SZ    C:\\Old\r\n    UserFolder    REG_SZ    \r\nEnd of search: 3 match(es) found.\r\n";
        assert_eq!(
            parse_reg_values(output, "UserFolder"),
            [
                PathBuf::from(r"C:\Users\Ada\OneDrive"),
                PathBuf::from(r"C:\Users\Ada\OneDrive - Contoso"),
            ]
        );
        assert!(
            parse_reg_values(
                "ERROR: The system was unable to find the specified registry key or value.",
                "UserFolder"
            )
            .is_empty()
        );
    }
}
