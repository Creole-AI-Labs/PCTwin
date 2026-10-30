//! The forgotten-things finder (Task List 2.5; plan section 11, "Forgotten-work finder").
//!
//! People forget the things that live outside their documents: their own fonts, mail
//! signatures, custom dictionaries, game saves kept in a game's own place, the folders a game
//! launcher installs games into, and the printers they added. The finder looks in the places a
//! signed catalogue names for each system ([`PlaceRow`], the `special-places` catalogue: data,
//! never code; every place in it is an example and a new one is a new row), and lists what the
//! system itself knows of printers ([`printers`]).
//!
//! It only looks: each place is found and counted from the file system's own facts (never
//! opening a file, never following a link, never reading a file kept online only, which is
//! counted apart), within a time and count limit, and each finding says whether it is already in
//! the move ([`Forgotten::moving`]) and the person's step ([`PlaceStep`]). A place that could not
//! be read is said ([`Finding::not_checked`]); nothing found is dropped, and finding nothing in
//! the known places never means nothing was forgotten.

use std::path::{Path, PathBuf};
use std::time::Instant;

use pctwin_catalogue::{Catalogue, Row};
use serde::{Deserialize, Serialize};

use crate::catalogue::Systems;
use crate::guard::{self, Kind};
use crate::roots::{DataRoot, RootKind, System};

/// A place the system names that is not one of the person's app data roots.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum Under {
    /// The person's home folder.
    Home,
    /// The person's Documents folder, wherever the system has it (the caller says where).
    Documents,
    /// Windows' folder for programs (`ProgramFiles`).
    ProgramFiles,
    /// Windows' folder for 32-bit programs (`ProgramFiles(x86)`).
    ProgramFilesX86,
    /// Windows' folder for programs' shared data (`ProgramData`).
    ProgramData,
}

/// What the person does about a place.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum PlaceStep {
    /// Plain files: copied with the move when the move takes their folder.
    Copy,
    /// Files the new laptop must install as well as hold (fonts): brought over, then installed.
    Install,
    /// A game launcher's library: the games come again through the launcher; a folder of games
    /// that is brought over is found and checked by the launcher's own "locate" and "verify".
    LocateVerify,
    /// Not files: added again on the new laptop (printers).
    AddAgain,
    /// Mail archives: copied, then the person opens each in the mail app on the new laptop
    /// (an archive is not part of the mail app until it is opened there).
    OpenInMailApp,
    /// Mail kept only on the old laptop: PCTwin carries it and writes it out on the new laptop
    /// in a standard mailbox format (the export); the person imports it in their mail app.
    ExportFirst,
}

/// One row of the `special-places` catalogue: a place on one kind of system, under one of the
/// person's app data roots (`root`) or a system folder (`under`), and what is kept there.
#[derive(Debug, Clone, PartialEq, Eq, Deserialize)]
pub struct PlaceRow {
    pub system: Systems,
    #[serde(default)]
    pub root: Option<RootKind>,
    #[serde(default)]
    pub under: Option<Under>,
    /// The place inside it: one to six plain names, `/` between them.
    pub path: String,
    /// What is kept there, as a short identifier (`fonts`, `game-saves`); the screens' words.
    pub kind: String,
    /// What is kept there, in plain English, for a kind the screens have no words for yet.
    pub name: String,
    pub step: PlaceStep,
    /// The maker's page for the step (a launcher's own help about finding installed games).
    #[serde(default)]
    pub page: Option<String>,
    /// When the row was last checked against the system's or maker's own documentation.
    pub checked_on: String,
}

fn plain_names(p: &str) -> bool {
    let parts: Vec<&str> = p.split('/').collect();
    !p.is_empty()
        && p.len() <= 255
        && parts.len() <= 6
        && parts
            .iter()
            .all(|n| !n.is_empty() && *n != "." && *n != ".." && !n.contains(['\\', ':', '\0']))
}

impl Row for PlaceRow {
    const CATALOGUE: &'static str = "special-places";
    const FIELDS: &'static [&'static str] = &[
        "system",
        "root",
        "under",
        "path",
        "kind",
        "name",
        "step",
        "page",
        "checked_on",
    ];

    fn check(&self) -> Result<(), String> {
        if self.root.is_some() == self.under.is_some() {
            return Err(
                "a place is under exactly one of an app data root or a system folder".into(),
            );
        }
        if self.root.is_some_and(RootKind::is_cache) {
            return Err("a place is never in the system's cache".into());
        }
        if !plain_names(&self.path) {
            return Err("the place must be one to six plain names".into());
        }
        let id_ok = !self.kind.is_empty()
            && self.kind.len() <= 40
            && self
                .kind
                .bytes()
                .all(|b| b.is_ascii_lowercase() || b.is_ascii_digit() || b == b'-');
        if !id_ok {
            return Err("a kind is lower-case letters, digits and dashes".into());
        }
        if self.name.is_empty() || self.name.len() > 100 {
            return Err("a name is 1 to 100 characters".into());
        }
        if self.step == PlaceStep::AddAgain {
            return Err("places hold files; what is added again is read from the system".into());
        }
        if self
            .page
            .as_deref()
            .is_some_and(|u| !crate::catalogue::page_ok(u))
        {
            return Err("a page must be an https address".into());
        }
        if !crate::catalogue::date_ok(&self.checked_on) {
            return Err("checked_on must be a date (YYYY-MM-DD)".into());
        }
        Ok(())
    }

    fn identity(&self) -> Option<String> {
        Some(format!(
            "{:?}/{:?}/{:?}/{}",
            self.system,
            self.root,
            self.under,
            self.path.to_lowercase()
        ))
    }
}

/// The shipped `special-places` catalogue (compiled in, covered by the app's own signature).
pub const SHIPPED: &str = include_str!("../catalogue/special-places.json");

/// One place found, counted.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Forgotten {
    pub kind: String,
    pub name: String,
    /// Where it is (inside the home folder when it is there, `/` between folders).
    pub place: String,
    /// Files found there (not counting those kept online only).
    pub files: u64,
    pub bytes: u64,
    /// Files kept online only: counted, never read or downloaded.
    pub online_only: u64,
    /// Every file was counted (no limit or link stopped the count).
    pub complete: bool,
    /// The move already takes this place (the plan proposes the folder it is in).
    pub moving: bool,
    pub step: PlaceStep,
    pub page: Option<String>,
}

/// A printer the system lists.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Printer {
    pub name: String,
    /// Shared from another computer or the network (its connection is made again by name).
    pub network: bool,
}

/// What the finder found, and what it could not check.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Finding {
    pub found: Vec<Forgotten>,
    pub printers: Vec<Printer>,
    /// Places or lists that could not be read, and why.
    pub not_checked: Vec<(String, String)>,
}

/// The most files counted in one place.
pub const MAX_COUNT: u64 = 200_000;
/// The deepest a place is counted.
const MAX_DEPTH: usize = 24;

struct Count {
    files: u64,
    bytes: u64,
    online_only: u64,
    complete: bool,
}

fn count(dir: &Path, depth: usize, until: Instant, c: &mut Count) {
    // The time is checked before each entry below, so a folder is never begun past it.
    if depth > MAX_DEPTH {
        c.complete = false;
        return;
    }
    let Ok(entries) = std::fs::read_dir(dir) else {
        c.complete = false;
        return;
    };
    for e in entries {
        if c.files + c.online_only >= MAX_COUNT || Instant::now() >= until {
            c.complete = false;
            return;
        }
        let Ok(e) = e else {
            c.complete = false;
            continue;
        };
        let Ok(m) = std::fs::symlink_metadata(e.path()) else {
            c.complete = false;
            continue;
        };
        match guard::kind_of(&m) {
            Kind::File => {
                c.files += 1;
                c.bytes += m.len();
            }
            Kind::Folder => count(&e.path(), depth + 1, until, c),
            Kind::OnlineOnly => c.online_only += 1,
            // A link is never followed: what it leads to is not counted.
            Kind::Link => c.complete = false,
            Kind::NotAFile => {}
        }
    }
}

/// Where a place is, for the person: inside `home` when it is there.
fn shown(path: &Path, home: &Path) -> String {
    let rel = path.strip_prefix(home).unwrap_or(path);
    rel.components()
        .map(|p| p.as_os_str().to_string_lossy().into_owned())
        .collect::<Vec<_>>()
        .join("/")
}

/// [`find_places`], and the printers the system lists ([`printers`]).
pub fn find(
    system: System,
    home: &Path,
    roots: &[DataRoot],
    under: &dyn Fn(Under) -> Option<PathBuf>,
    rows: &Catalogue<PlaceRow>,
    moving: &dyn Fn(&Path) -> bool,
    until: Instant,
) -> Finding {
    let mut out = find_places(system, home, roots, under, rows, moving, until);
    match printers() {
        Ok(p) => out.printers = p,
        Err(why) => out.not_checked.push(("printers".into(), why)),
    }
    out
}

/// Finds the places of `rows` on `system` for the person with home folder `home`, app data
/// roots `roots` (from [`crate::roots::roots_of`]), and system folders `under` (the caller says
/// where Documents and the programs' folders are); `moving` says whether the plan already takes
/// a path. Everything is counted by `until`; what was not is said.
pub fn find_places(
    system: System,
    home: &Path,
    roots: &[DataRoot],
    under: &dyn Fn(Under) -> Option<PathBuf>,
    rows: &Catalogue<PlaceRow>,
    moving: &dyn Fn(&Path) -> bool,
    until: Instant,
) -> Finding {
    let mut out = Finding::default();
    for row in rows.rows().iter().filter(|r| r.system.covers(system)) {
        let base = match (row.root, row.under) {
            (Some(kind), _) => roots
                .iter()
                .find(|r| r.kind == kind)
                .map(|r| r.path.clone()),
            (None, Some(u)) => under(u),
            (None, None) => None,
        };
        let Some(base) = base else { continue };
        let path = row.path.split('/').fold(base, |p, n| p.join(n));
        let meta = match std::fs::symlink_metadata(&path) {
            Ok(m) => m,
            // Not on this laptop: nothing to say.
            Err(e) if e.kind() == std::io::ErrorKind::NotFound => continue,
            Err(e) => {
                out.not_checked
                    .push((shown(&path, home), e.kind().to_string()));
                continue;
            }
        };
        match guard::kind_of(&meta) {
            Kind::Folder => {}
            Kind::Link => {
                out.not_checked.push((
                    shown(&path, home),
                    "a link to somewhere else: not followed".into(),
                ));
                continue;
            }
            Kind::OnlineOnly => {
                out.not_checked.push((
                    shown(&path, home),
                    "kept online by a cloud app: not downloaded or looked into".into(),
                ));
                continue;
            }
            Kind::File | Kind::NotAFile => continue,
        }
        let mut c = Count {
            files: 0,
            bytes: 0,
            online_only: 0,
            complete: true,
        };
        count(&path, 0, until, &mut c);
        if c.files == 0 && c.online_only == 0 && c.complete {
            continue;
        }
        out.found.push(Forgotten {
            kind: row.kind.clone(),
            name: row.name.clone(),
            place: shown(&path, home),
            files: c.files,
            bytes: c.bytes,
            online_only: c.online_only,
            complete: c.complete,
            moving: moving(&path),
            step: row.step,
            page: row.page.clone(),
        });
    }
    out
}

/// The most printers listed.
pub const MAX_PRINTERS: usize = 200;

/// The printers this laptop's system lists for the person: on Windows, the system's printer
/// list and the person's own connections to shared printers (the registry, read only); on a Mac
/// and Linux, the print system's list of destinations (`lpstat -e`, in the C locale so its output
/// is never translated). Only names are read.
pub fn printers() -> Result<Vec<Printer>, String> {
    let mut out = printers_here()?;
    out.sort_by_key(|p| p.name.to_lowercase());
    out.dedup_by(|a, b| a.name.eq_ignore_ascii_case(&b.name));
    out.retain(|p| !p.name.trim().is_empty() && p.name.len() <= 256);
    out.truncate(MAX_PRINTERS);
    Ok(out)
}

/// A shared printer's connection key (`,,server,printer`) as its name (`\\server\printer`).
pub fn connection_name(key: &str) -> Option<String> {
    let rest = key.strip_prefix(",,")?;
    let (server, printer) = rest.split_once(',')?;
    if server.is_empty() || printer.is_empty() {
        return None;
    }
    Some(format!("\\\\{server}\\{}", printer.replace(',', "\\")))
}

#[cfg(windows)]
fn printers_here() -> Result<Vec<Printer>, String> {
    use winreg::RegKey;
    use winreg::enums::{HKEY_CURRENT_USER, HKEY_LOCAL_MACHINE, KEY_READ};
    let mut out = Vec::new();
    let system = RegKey::predef(HKEY_LOCAL_MACHINE)
        .open_subkey_with_flags(r"SYSTEM\CurrentControlSet\Control\Print\Printers", KEY_READ)
        .map_err(|e| e.kind().to_string())?;
    for name in system.enum_keys().flatten().take(MAX_PRINTERS) {
        out.push(Printer {
            name,
            network: false,
        });
    }
    // The person's own connections; none is an ordinary answer.
    if let Ok(conns) =
        RegKey::predef(HKEY_CURRENT_USER).open_subkey_with_flags(r"Printers\Connections", KEY_READ)
    {
        for key in conns.enum_keys().flatten().take(MAX_PRINTERS) {
            if let Some(name) = connection_name(&key) {
                out.retain(|p| !p.name.eq_ignore_ascii_case(&name));
                out.push(Printer {
                    name,
                    network: true,
                });
            }
        }
    }
    Ok(out)
}

/// The destinations `lpstat -e` lists, one name per line.
pub fn lpstat_names(out: &[u8]) -> Vec<String> {
    String::from_utf8_lossy(out)
        .lines()
        .map(str::trim)
        .filter(|l| !l.is_empty() && !l.contains(char::is_whitespace))
        .map(str::to_string)
        .collect()
}

#[cfg(not(windows))]
fn printers_here() -> Result<Vec<Printer>, String> {
    use std::process::Stdio;
    let lpstat = ["/usr/bin/lpstat", "/bin/lpstat"]
        .into_iter()
        .map(Path::new)
        .find(|p| p.is_file())
        .ok_or_else(|| "the print system's list is not on this laptop".to_string())?;
    let out = pctwin_types::process::quiet(lpstat)
        .arg("-e")
        .env_clear()
        .env("LC_ALL", "C")
        .stdin(Stdio::null())
        .stderr(Stdio::null())
        .output()
        .map_err(|e| e.kind().to_string())?;
    if !out.status.success() {
        return Err("the print system did not answer".into());
    }
    let bounded = &out.stdout[..out.stdout.len().min(64 * 1024)];
    Ok(lpstat_names(bounded)
        .into_iter()
        .map(|name| Printer {
            name,
            network: false,
        })
        .collect())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn shared_printer_connections_read_as_their_names() {
        assert_eq!(
            connection_name(",,print-server,Office Colour").as_deref(),
            Some(r"\\print-server\Office Colour")
        );
        assert_eq!(
            connection_name(",,srv,floor,2").as_deref(),
            Some(r"\\srv\floor\2")
        );
        assert_eq!(connection_name("Local Printer"), None);
        assert_eq!(connection_name(",,,x"), None);
        assert_eq!(connection_name(",,srv,"), None);
        assert_eq!(connection_name(",,srv"), None);
    }

    #[test]
    fn lpstat_lines_are_names_only() {
        assert_eq!(
            lpstat_names(b"Office_Laser\n\n  Home-Inkjet \nnot a name\n"),
            vec!["Office_Laser".to_string(), "Home-Inkjet".to_string()]
        );
    }
}
