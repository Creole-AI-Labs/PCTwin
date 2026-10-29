//! The folders a remembered connection may use (Security Design Part T): "folder for next time"
//! and permission 3's folder are each held by identity and path ([`KnownFolder`]), opened again
//! and rechecked at every exchange (same drive, same folder, not a link or reparse point), and may
//! never be a place that runs things at sign-in, when a shell starts, or holds keys, nor the
//! system's own folders. Those places are a catalogue (data: `catalogues/refused-folders.json`),
//! never a list in code.

use std::path::{Component, Path, PathBuf};
use std::sync::OnceLock;

use pctwin_gate::{Destination, FileId};
use serde::{Deserialize, Serialize};

/// Why a folder cannot be remembered or allowed.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, thiserror::Error)]
#[serde(rename_all = "kebab-case")]
pub enum FolderProblem {
    /// It runs things at sign-in (Startup, autostart, LaunchAgents), or holds such a place.
    #[error("this folder runs things when you sign in")]
    RunsAtSignIn,
    /// It runs things when a shell or a terminal starts, or holds such a place.
    #[error("this folder runs things when a terminal starts")]
    ShellStartUp,
    /// It holds keys (`.ssh`, `.gnupg`, the keychains), or holds such a place.
    #[error("this folder holds keys")]
    HoldsKeys,
    /// One of the system's own folders, or one that holds them.
    #[error("this is one of the system's own folders")]
    System,
    /// It is not a plain folder (a link, a junction or another reparse point), or not there.
    #[error("this is not a plain folder")]
    NotPlain,
    /// The folder at the remembered path is not the one remembered (moved, or another put there,
    /// or on another drive now).
    #[error("this is not the folder that was chosen")]
    NotTheSame,
}

#[derive(Deserialize)]
struct Catalogue {
    version: u32,
    places: Vec<Entry>,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct Entry {
    system: String,
    base: String,
    path: String,
    why: Why,
}

#[derive(Debug, Clone, Copy, Deserialize)]
#[serde(rename_all = "kebab-case")]
enum Why {
    RunsAtSignIn,
    ShellStartUp,
    HoldsKeys,
    System,
}

impl Why {
    fn problem(self) -> FolderProblem {
        match self {
            Why::RunsAtSignIn => FolderProblem::RunsAtSignIn,
            Why::ShellStartUp => FolderProblem::ShellStartUp,
            Why::HoldsKeys => FolderProblem::HoldsKeys,
            Why::System => FolderProblem::System,
        }
    }
}

const SHIPPED: &str = include_str!("../catalogues/refused-folders.json");

fn catalogue() -> &'static Catalogue {
    static CATALOGUE: OnceLock<Catalogue> = OnceLock::new();
    CATALOGUE.get_or_init(|| {
        // Shipped with the app and checked by the tests: a damaged one refuses every folder
        // (below), never none.
        serde_json::from_str(SHIPPED).unwrap_or(Catalogue {
            version: 0,
            places: Vec::new(),
        })
    })
}

/// Where the catalogue's bases are on this laptop.
#[derive(Debug, Clone, Default)]
pub struct Places {
    pub home: Option<PathBuf>,
    pub documents: Option<PathBuf>,
    pub appdata: Option<PathBuf>,
    pub localappdata: Option<PathBuf>,
    pub programdata: Option<PathBuf>,
    pub windir: Option<PathBuf>,
    pub programfiles: Option<PathBuf>,
    pub programfilesx86: Option<PathBuf>,
    pub root: Option<PathBuf>,
}

impl Places {
    /// This laptop's, from the system's own known-folder lookup (on Windows the shell's known
    /// folders, which follow a Documents moved into OneDrive or onto another drive), never from
    /// environment variables, which a program can leave out or point anywhere.
    pub fn of_this_system() -> Self {
        #[cfg(windows)]
        {
            use windows_sys::Win32::UI::Shell::{
                FOLDERID_Documents, FOLDERID_LocalAppData, FOLDERID_Profile, FOLDERID_ProgramData,
                FOLDERID_ProgramFiles, FOLDERID_ProgramFilesX86, FOLDERID_RoamingAppData,
                FOLDERID_Windows,
            };
            let known = dirs_sys::known_folder;
            Self {
                home: known(FOLDERID_Profile),
                documents: known(FOLDERID_Documents),
                appdata: known(FOLDERID_RoamingAppData),
                localappdata: known(FOLDERID_LocalAppData),
                programdata: known(FOLDERID_ProgramData),
                windir: known(FOLDERID_Windows),
                programfiles: known(FOLDERID_ProgramFiles),
                programfilesx86: known(FOLDERID_ProgramFilesX86),
                root: None,
            }
        }
        #[cfg(not(windows))]
        {
            Self {
                home: dirs_sys::home_dir(),
                root: Some(PathBuf::from("/")),
                ..Self::default()
            }
        }
    }

    fn base(&self, name: &str) -> Option<&Path> {
        match name {
            "home" => self.home.as_deref(),
            "documents" => self.documents.as_deref(),
            "appdata" => self.appdata.as_deref(),
            "localappdata" => self.localappdata.as_deref(),
            "programdata" => self.programdata.as_deref(),
            "windir" => self.windir.as_deref(),
            "programfiles" => self.programfiles.as_deref(),
            "programfilesx86" => self.programfilesx86.as_deref(),
            "root" => self.root.as_deref(),
            _ => None,
        }
    }
}

/// This system's name in the catalogue.
pub fn this_system() -> &'static str {
    if cfg!(windows) {
        "windows"
    } else if cfg!(target_os = "macos") {
        "macos"
    } else {
        "linux"
    }
}

/// `path` in one form for comparing: the longest part that exists resolved by the system (links
/// followed, so a link to a refused place is that place), the rest added as written, each part
/// in lower case where the system ignores case.
fn comparable(path: &Path) -> Vec<String> {
    let mut existing = path.to_path_buf();
    let mut rest = Vec::new();
    let resolved = loop {
        if let Ok(real) = std::fs::canonicalize(&existing) {
            break real;
        }
        match (existing.file_name(), existing.parent()) {
            (Some(name), Some(parent)) => {
                rest.push(name.to_os_string());
                existing = parent.to_path_buf();
            }
            _ => break existing,
        }
    };
    let fold = cfg!(any(windows, target_os = "macos"));
    let mut parts: Vec<String> = Vec::new();
    for c in resolved
        .components()
        .chain(rest.iter().rev().map(|r| Component::Normal(r.as_os_str())))
    {
        let text = match c {
            Component::Prefix(p) => {
                // `\\?\C:` and `C:` are one drive.
                let s = p.as_os_str().to_string_lossy().into_owned();
                s.trim_start_matches(r"\\?\").to_string()
            }
            Component::RootDir => continue,
            Component::CurDir => continue,
            Component::ParentDir => {
                parts.pop();
                continue;
            }
            Component::Normal(n) => n.to_string_lossy().into_owned(),
        };
        parts.push(if fold { text.to_lowercase() } else { text });
    }
    parts
}

/// Why `folder` may not be a destination on `system` with these `places`: it is a catalogue
/// place, inside one, or holds one (a sent folder's paths could reach it). A place whose base
/// cannot be resolved refuses too: what is there cannot be told. `None`: it may.
pub fn refused_on(system: &str, folder: &Path, places: &Places) -> Option<FolderProblem> {
    let cat = catalogue();
    if cat.version != 1 || cat.places.is_empty() {
        return Some(FolderProblem::System);
    }
    let chosen = comparable(folder);
    for entry in cat.places.iter().filter(|e| e.system == system) {
        let Some(base) = places.base(&entry.base) else {
            return Some(entry.why.problem());
        };
        let mut place = base.to_path_buf();
        for part in entry.path.split('/').filter(|p| !p.is_empty()) {
            place.push(part);
        }
        let place = comparable(&place);
        let inside = chosen.starts_with(&place);
        let holds = place.starts_with(&chosen);
        if inside || holds {
            return Some(entry.why.problem());
        }
    }
    None
}

/// As [`refused_on`] for this laptop.
pub fn refused(folder: &Path) -> Option<FolderProblem> {
    refused_on(this_system(), folder, &Places::of_this_system())
}

/// A folder held by its path and by what it is (its identity when it was first held: the drive
/// and the folder on it), so that a folder put at that path later, or a link put there, is never
/// taken for it.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct KnownFolder {
    pub path: PathBuf,
    pub identity: FileId,
}

impl KnownFolder {
    /// The plain folder at `path`, to remember: refused when it is not a plain folder or is a
    /// catalogue place.
    pub fn choose(path: &Path) -> Result<(Self, Destination), FolderProblem> {
        Self::choose_with(path, refused)
    }

    /// As [`choose`](Self::choose), with the catalogue check given (tests).
    pub fn choose_with(
        path: &Path,
        refused: impl Fn(&Path) -> Option<FolderProblem>,
    ) -> Result<(Self, Destination), FolderProblem> {
        let (dest, identity) = destination_with(path, None, refused)?;
        Ok((
            Self {
                path: path.to_path_buf(),
                identity,
            },
            dest,
        ))
    }

    /// Opened again for this exchange, only while it is still that very folder, plain, and not a
    /// catalogue place (the catalogue may have grown since).
    pub fn reopen(&self) -> Result<Destination, FolderProblem> {
        self.reopen_with(refused)
    }

    /// As [`reopen`](Self::reopen), with the catalogue check given (tests).
    pub fn reopen_with(
        &self,
        refused: impl Fn(&Path) -> Option<FolderProblem>,
    ) -> Result<Destination, FolderProblem> {
        destination_with(&self.path, Some(self.identity), refused).map(|(dest, _)| dest)
    }
}

/// The one check every place received files may be written to passes, whichever way it was
/// chosen (the folder for this receive, the one chosen this session, Downloads, the folder for
/// next time, permission 3's): not a catalogue place nor inside or holding one, a plain folder,
/// and, when `expect` says what it was, still that very folder. Opened and held for the write.
pub fn destination(
    path: &Path,
    expect: Option<FileId>,
) -> Result<(Destination, FileId), FolderProblem> {
    destination_with(path, expect, refused)
}

/// As [`destination`], with the catalogue check given (tests).
pub fn destination_with(
    path: &Path,
    expect: Option<FileId>,
    refused: impl Fn(&Path) -> Option<FolderProblem>,
) -> Result<(Destination, FileId), FolderProblem> {
    if let Some(problem) = refused(path) {
        return Err(problem);
    }
    let (dest, identity) = open_plain(path).ok_or(FolderProblem::NotPlain)?;
    if expect.is_some_and(|e| e != identity) {
        return Err(FolderProblem::NotTheSame);
    }
    Ok((dest, identity))
}

/// Whether `path` is itself a plain folder: not a symbolic link, junction or other reparse point.
pub fn plain_folder(path: &Path) -> bool {
    let Ok(meta) = std::fs::symlink_metadata(path) else {
        return false;
    };
    #[cfg(windows)]
    {
        use std::os::windows::fs::MetadataExt;
        const FILE_ATTRIBUTE_REPARSE_POINT: u32 = 0x400;
        if meta.file_attributes() & FILE_ATTRIBUTE_REPARSE_POINT != 0 {
            return false;
        }
    }
    meta.is_dir() && !meta.file_type().is_symlink()
}

/// Opens the plain folder at `path` and says what it is. `None` when it is not a plain folder,
/// cannot be opened, or stopped being a plain folder while it was opened.
pub fn open_plain(path: &Path) -> Option<(Destination, FileId)> {
    open_plain_around(path, || (), || ())
}

/// [`open_plain`], with what happens just before and just after the open (for tests, which swap
/// a link in and out there). The open follows a link, so what it opened must be the very folder
/// that a second open of the path, which never follows one, finds there.
pub fn open_plain_around(
    path: &Path,
    before: impl FnOnce(),
    after: impl FnOnce(),
) -> Option<(Destination, FileId)> {
    if !plain_folder(path) {
        return None;
    }
    before();
    let dest = Destination::open(path).ok()?;
    after();
    let identity = dest.folder_identity("").ok()??;
    (plain_identity(path)? == identity).then_some((dest, identity))
}

/// What the plain folder at `path` is, from an open that never follows a link: `None` when a
/// link, junction or anything but a plain folder is there.
pub fn plain_identity(path: &Path) -> Option<FileId> {
    #[cfg(windows)]
    let folder = {
        use std::os::windows::fs::{MetadataExt, OpenOptionsExt};
        const FILE_READ_ATTRIBUTES: u32 = 0x0080;
        const FILE_FLAG_BACKUP_SEMANTICS: u32 = 0x0200_0000;
        const FILE_FLAG_OPEN_REPARSE_POINT: u32 = 0x0020_0000;
        const FILE_ATTRIBUTE_REPARSE_POINT: u32 = 0x400;
        let folder = std::fs::OpenOptions::new()
            .access_mode(FILE_READ_ATTRIBUTES)
            .custom_flags(FILE_FLAG_BACKUP_SEMANTICS | FILE_FLAG_OPEN_REPARSE_POINT)
            .open(path)
            .ok()?;
        let meta = folder.metadata().ok()?;
        if !meta.is_dir() || meta.file_attributes() & FILE_ATTRIBUTE_REPARSE_POINT != 0 {
            return None;
        }
        folder
    };
    #[cfg(unix)]
    let folder = {
        use rustix::fs::{Mode, OFlags};
        std::fs::File::from(
            rustix::fs::open(
                path,
                OFlags::RDONLY | OFlags::NOFOLLOW | OFlags::DIRECTORY | OFlags::CLOEXEC,
                Mode::empty(),
            )
            .ok()?,
        )
    };
    pctwin_gate::file_identity(&folder).ok()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_shipped_catalogue_reads_and_names_every_system() {
        let cat: Catalogue = serde_json::from_str(SHIPPED).unwrap();
        assert_eq!(cat.version, 1);
        for system in ["windows", "macos", "linux"] {
            assert!(cat.places.iter().any(|p| p.system == system), "{system}");
        }
        let known = [
            "home",
            "documents",
            "appdata",
            "localappdata",
            "programdata",
            "windir",
            "programfiles",
            "programfilesx86",
            "root",
        ];
        for p in &cat.places {
            assert!(known.contains(&p.base.as_str()), "{}", p.base);
        }
    }
}
