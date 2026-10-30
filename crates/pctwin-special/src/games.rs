//! Games and game launchers (Task List 2.5; plan section 8; acceptance T25-038 to T25-041).
//!
//! **No game or launcher is in the code** (adoption changes 7 and 8). Three signed catalogues
//! say where things are, as data; the code reads per format and mechanism:
//!
//! - `game-saves` ([`SaveRow`]): where a game keeps its saves, written in the **Ludusavi manifest
//!   format** (paths with placeholders such as `<winAppData>`, `<home>`, `<base>`, each with its
//!   systems and stores; the stores that sync saves online, `cloud`), plus PCTwin's own fields:
//!   the date it was checked and the systems between which the save format is the same. These
//!   rows are PCTwin's own hand-written data, a few examples only.
//! - `game-launchers` ([`LauncherRow`]): where a launcher keeps its install records, which
//!   format reader reads them (Valve's text key-values, JSON, the Windows registry), which fields
//!   name a game, its ID and its folder, where the launcher marks a game's saves as synced
//!   online, and the launcher's own "verify" address and help page.
//! - `game-places` ([`GamePlaceRow`]): where the person's own save list is (a copy of the
//!   Ludusavi manifest kept by Ludusavi itself), places a store keeps protected for itself, and
//!   folders of add-ons (mods).
//!
//! **The Ludusavi manifest's own data is never shipped or downloaded**: its game data comes from
//! PCGamingWiki under a non-commercial licence (CC BY-NC-SA 3.0), not yet cleared (ADOPTION
//! change 6). PCTwin reads the **format**, applied to its own rows and to the person's own copy
//! when Ludusavi is on the old laptop (reading the person's own file is not redistribution).
//! [`fetch_manifest`] is the hook for fetching it, and refuses until the licence is cleared.
//!
//! What happens, honestly:
//! - **Saves** ([`find`]): each row's paths are filled in with the person's real folders on the
//!   old laptop and looked for (globs bounded, links never followed, files kept online never
//!   read). Saves inside an app data root are already in the move as app data ([`enrich`] marks
//!   them as games and adds their notes); saves moving with the person's own files are left to
//!   that; any other save place becomes its own component ([`components`]), kept on the new
//!   laptop in PCTwin's folder of things brought over (needs-action: put it back or let the game
//!   find it), since it lies outside the person's home.
//! - **Another kind of system** (T25-038): there is no general save conversion (Ludusavi's own
//!   documentation says so). A save is handed to the game on the new laptop only when its row
//!   says the format is the same on both systems and names the one place there; otherwise it is
//!   kept on the new laptop, needs-action, "may not work on this system".
//! - **Saves the launcher keeps online** (T25-039): never copied over the launcher's own copy.
//!   They are not proposed (the launcher brings them back); if the person ticks them, they are
//!   kept beside, never where the game reads; the old laptop's copy is only ever read.
//! - **Installed games** (T25-040): listed from the launcher's own records with name, folder and
//!   size; the person's step is the launcher's own locate and verify ([`open_verify`], opened as
//!   the person, never run by PCTwin); needs-action until the person confirms. The games' files
//!   are not copied unless the person chooses; entitlement stays with the store.
//! - **Store-protected saves**: warned, not copied, with the reason.
//! - **Programs in saves or add-ons** (T25-041): found by their content (never by name), listed
//!   apart, never copied as data and never turned on.
//!
//! Never: no game, launcher or network play is started as a check; no DRM or licence is
//! bypassed; the old laptop is only read.

use std::collections::BTreeMap;
use std::io::{Read, Seek, SeekFrom};
use std::path::{Path, PathBuf};
use std::time::Instant;

use pctwin_catalogue::{Catalogue, Row};
use pctwin_parse_seal::InChild;
use pctwin_record::Owner;
use pctwin_types::{ItemResult, Reason};
use serde::{Deserialize, Serialize};

use crate::api::{NotMovedItem, Plan, Planned};
use crate::catalogue::Systems;
use crate::category::{Category, Note};
use crate::classify::Placement;
use crate::component::{Component, Custody, Evidence};
use crate::content::ContentError;
use crate::gamefmt::{ManifestRead, RecordFormat, SaveEntry, Tree, Wanted};
use crate::guard::{self, Kind};
use crate::handoff::{Opened, Person, Step};
use crate::roots::{DataRoot, RootKind, System};

// ---------- the catalogues ----------

/// One row of the `game-saves` catalogue: a game's entry in the Ludusavi manifest format, and
/// PCTwin's own facts about it.
#[derive(Debug, Clone, PartialEq, Eq, Deserialize)]
pub struct SaveRow {
    #[serde(flatten)]
    pub entry: SaveEntry,
    /// The systems between which this game's saves are the same format (a save made on one
    /// works on another). Empty: no move between systems hands the saves to the game.
    #[serde(default)]
    pub same_format: Vec<System>,
    /// When the row was last checked against the game maker's own documentation.
    pub checked_on: String,
}

impl Row for SaveRow {
    const CATALOGUE: &'static str = "game-saves";
    const FIELDS: &'static [&'static str] = &[
        "name",
        "files",
        "registry",
        "installDir",
        "cloud",
        "steam",
        "gog",
        "id",
        "alias",
        "same_format",
        "checked_on",
    ];

    fn check(&self) -> Result<(), String> {
        let e = &self.entry;
        if e.name.trim().is_empty() || e.name.len() > 200 {
            return Err("a game's name is 1 to 200 characters".into());
        }
        if e.alias.is_some() {
            return Err("a row is a game's own entry, never another's alias".into());
        }
        if e.files.is_empty() {
            return Err("a row names at least one place".into());
        }
        if let Some(why) = e.files.keys().find_map(|t| template_ok(t).err()) {
            return Err(why);
        }
        if !e.install_dir.keys().all(|n| plain_name(n)) {
            return Err("an install folder is one plain name".into());
        }
        if !crate::catalogue::date_ok(&self.checked_on) {
            return Err("checked_on must be a date (YYYY-MM-DD)".into());
        }
        Ok(())
    }

    fn identity(&self) -> Option<String> {
        Some(self.entry.name.to_lowercase())
    }
}

/// Where a launcher's records are, and which of their fields mean what.
#[derive(Debug, Clone, PartialEq, Eq, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Records {
    /// One file, plain names inside each base (or library) folder.
    #[serde(default)]
    pub file: Option<String>,
    /// A folder, plain names inside each base (or library) folder...
    #[serde(default)]
    pub folder: Option<String>,
    /// ...and the names of its record files (`*` any text).
    #[serde(default)]
    pub files: Option<String>,
    /// Or a registry key (`HKEY_LOCAL_MACHINE/...`, `/` between keys), on Windows.
    #[serde(default)]
    pub registry: Option<String>,
    pub format: Format,
    /// Where in each record the fields are (names, one level each).
    #[serde(default)]
    pub at: Vec<String>,
    /// The value at `at` holds one record per child.
    #[serde(default)]
    pub each: bool,
    /// For libraries: the field holding a library folder.
    #[serde(default)]
    pub path: Option<String>,
    /// For games: the fields holding the game's name, its store ID and its install folder.
    #[serde(default)]
    pub name: Option<String>,
    #[serde(default)]
    pub id: Option<String>,
    #[serde(default)]
    pub install: Option<String>,
    /// The install field is a folder's name inside `<library>/<install_in>`; without it, a full
    /// path.
    #[serde(default)]
    pub install_in: Option<String>,
}

/// The format reader a launcher's records need.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum Format {
    Vdf,
    Json,
    Registry,
}

/// One row of the `game-launchers` catalogue.
#[derive(Debug, Clone, PartialEq, Eq, Deserialize)]
pub struct LauncherRow {
    /// The launcher's short name, as the save format names stores (`steam`, `epic`, `gog`).
    pub launcher: String,
    /// Its name for the person.
    pub name: String,
    pub system: Systems,
    /// Where the launcher itself is (templates such as `<winProgramFilesX86>/Steam`).
    #[serde(default)]
    pub base: Vec<String>,
    /// Where it lists its library folders, if it has several.
    #[serde(default)]
    pub libraries: Option<Records>,
    /// Its records of installed games.
    pub games: Records,
    /// A file or folder whose presence says the launcher syncs a game's saves online, inside
    /// its base (plain names, `<storeUserId>` and `<storeGameId>`).
    #[serde(default)]
    pub cloud_marker: Option<String>,
    /// The launcher's own address for checking a game's files (`<storeGameId>` filled in),
    /// opened as the person: `https://` or the launcher's own scheme (`steam://`).
    #[serde(default)]
    pub verify: Option<String>,
    /// The launcher's help page about finding and checking installed games.
    #[serde(default)]
    pub page: Option<String>,
    pub checked_on: String,
}

fn id_ok(s: &str) -> bool {
    !s.is_empty()
        && s.len() <= 40
        && s.bytes()
            .all(|b| b.is_ascii_lowercase() || b.is_ascii_digit() || b == b'-')
}

fn plain_name(n: &str) -> bool {
    !n.is_empty()
        && n.len() <= 255
        && n != "."
        && n != ".."
        && !n.contains(['/', '\\', ':', '\0'])
        && !n.chars().any(char::is_control)
}

fn plain_names(p: &str, most: usize) -> bool {
    let parts: Vec<&str> = p.split('/').collect();
    parts.len() <= most && parts.iter().all(|n| plain_name(n))
}

/// Whether `verify` is an address the launcher row may open: `https://`, or the launcher's own
/// scheme; `<storeGameId>` at most once.
fn verify_ok(verify: &str, launcher: &str) -> bool {
    let scheme_ok = crate::catalogue::page_ok(verify)
        || verify
            .strip_prefix(launcher)
            .and_then(|r| r.strip_prefix("://"))
            .is_some_and(|r| !r.is_empty() && !r.contains(char::is_whitespace));
    scheme_ok
        && verify.matches("<storeGameId>").count() <= 1
        && verify
            .replace("<storeGameId>", "")
            .find(['<', '>'])
            .is_none()
}

fn records_ok(r: &Records, games: bool) -> Result<(), String> {
    let places = [r.file.is_some(), r.folder.is_some(), r.registry.is_some()];
    if places.iter().filter(|p| **p).count() != 1 {
        return Err("records are in exactly one of a file, a folder or a registry key".into());
    }
    if r.folder.is_some() != r.files.is_some() {
        return Err("a folder of records names its files".into());
    }
    if r.registry.is_some() != (r.format == Format::Registry) {
        return Err("the registry is read only as the registry".into());
    }
    if let Some(k) = &r.registry {
        let hive = k.split('/').next().unwrap_or_default();
        if !matches!(hive, "HKEY_LOCAL_MACHINE" | "HKEY_CURRENT_USER") || !plain_names(k, 12) {
            return Err(
                "a registry key starts with HKEY_LOCAL_MACHINE or HKEY_CURRENT_USER".into(),
            );
        }
    }
    for p in [&r.file, &r.folder, &r.install_in].into_iter().flatten() {
        if !plain_names(p, 6) {
            return Err("a place inside a launcher's folder is plain names".into());
        }
    }
    if r.files.as_deref().is_some_and(|f| !plain_name(f)) {
        return Err("record files are named in one plain name".into());
    }
    let field = |f: &Option<String>| f.as_deref().is_some_and(|f| !f.is_empty() && f.len() <= 64);
    let fields_ok = if games {
        field(&r.name) && field(&r.install) && r.path.is_none()
    } else {
        field(&r.path) && r.name.is_none() && r.install.is_none() && r.id.is_none()
    };
    if !fields_ok || r.at.len() > crate::gamefmt::MAX_DEPTH {
        return Err("records name the fields they are read for".into());
    }
    Ok(())
}

impl Row for LauncherRow {
    const CATALOGUE: &'static str = "game-launchers";
    const FIELDS: &'static [&'static str] = &[
        "launcher",
        "name",
        "system",
        "base",
        "libraries",
        "games",
        "cloud_marker",
        "verify",
        "page",
        "checked_on",
    ];

    fn check(&self) -> Result<(), String> {
        if !id_ok(&self.launcher) {
            return Err("a launcher is named in lower-case letters, digits and dashes".into());
        }
        if self.name.is_empty() || self.name.len() > 100 {
            return Err("a name is 1 to 100 characters".into());
        }
        if let Some(why) = self.base.iter().find_map(|t| template_ok(t).err()) {
            return Err(why);
        }
        if self.games.registry.is_none() && self.base.is_empty() {
            return Err("records in folders need the launcher's folder".into());
        }
        if let Some(l) = &self.libraries {
            records_ok(l, false)?;
            if l.registry.is_some() {
                return Err("libraries are listed in a file".into());
            }
        }
        records_ok(&self.games, true)?;
        if let Some(m) = &self.cloud_marker {
            let plain = m
                .replace("<storeUserId>", "u")
                .replace("<storeGameId>", "g");
            if !plain_names(&plain, 8) || plain.contains(['<', '>', '*', '?']) {
                return Err("a cloud mark is plain names inside the launcher's folder".into());
            }
        }
        if self
            .verify
            .as_deref()
            .is_some_and(|v| !verify_ok(v, &self.launcher))
        {
            return Err("a verify address is https or the launcher's own scheme".into());
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
        Some(format!("{:?}/{}", self.system, self.launcher))
    }
}

/// What a `game-places` row names.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Deserialize, Serialize)]
#[serde(rename_all = "kebab-case")]
pub enum PlaceKind {
    /// The person's own save list in the Ludusavi manifest format (Ludusavi's own copy).
    SaveList,
    /// Saves a store keeps protected for itself: warned, never copied.
    StoreProtected,
    /// A game's add-ons (mods): their data kept, their programs listed apart.
    Mods,
}

/// One row of the `game-places` catalogue.
#[derive(Debug, Clone, PartialEq, Eq, Deserialize)]
pub struct GamePlaceRow {
    pub system: Systems,
    pub kind: PlaceKind,
    /// A template, as in the save format.
    pub path: String,
    /// What it is, for the person.
    pub name: String,
    /// The game whose add-ons these are (for `<base>`), by its row's name.
    #[serde(default)]
    pub game: Option<String>,
    pub checked_on: String,
}

impl Row for GamePlaceRow {
    const CATALOGUE: &'static str = "game-places";
    const FIELDS: &'static [&'static str] =
        &["system", "kind", "path", "name", "game", "checked_on"];

    fn check(&self) -> Result<(), String> {
        template_ok(&self.path)?;
        if self.name.is_empty() || self.name.len() > 100 {
            return Err("a name is 1 to 100 characters".into());
        }
        if (self.kind == PlaceKind::Mods) != self.game.is_some() {
            return Err("an add-on folder, and only that, names its game".into());
        }
        if !crate::catalogue::date_ok(&self.checked_on) {
            return Err("checked_on must be a date (YYYY-MM-DD)".into());
        }
        Ok(())
    }

    fn identity(&self) -> Option<String> {
        Some(format!(
            "{:?}/{:?}/{}",
            self.system,
            self.game.as_deref().map(str::to_lowercase),
            self.path.to_lowercase()
        ))
    }
}

/// The shipped catalogues (compiled in, covered by the app's own signature).
pub const SHIPPED_SAVES: &str = include_str!("../catalogue/game-saves.json");
pub const SHIPPED_LAUNCHERS: &str = include_str!("../catalogue/game-launchers.json");
pub const SHIPPED_PLACES: &str = include_str!("../catalogue/game-places.json");

/// The three game catalogues.
pub struct Catalogues {
    pub saves: Catalogue<SaveRow>,
    pub launchers: Catalogue<LauncherRow>,
    pub places: Catalogue<GamePlaceRow>,
}

impl Catalogues {
    pub fn shipped() -> Self {
        Self {
            saves: Catalogue::shipped(SHIPPED_SAVES),
            launchers: Catalogue::shipped(SHIPPED_LAUNCHERS),
            places: Catalogue::shipped(SHIPPED_PLACES),
        }
    }
}

// ---------- the move's own reader and plan ----------

static READER: std::sync::OnceLock<&'static dyn GameReader> = std::sync::OnceLock::new();

/// Installs the reader this program reads launcher records and save lists through, once (the
/// app at its start with its parser processes, tests with theirs). False when one was installed
/// already (that one stays).
pub fn install_reader(reader: &'static dyn GameReader) -> bool {
    READER.set(reader).is_ok()
}

/// No reader installed: nothing is read, every record is "not checked", never guessed.
struct NoReader;

impl GameReader for NoReader {
    fn record(&self, _: RecordFormat, _: &[u8]) -> Result<Tree, ContentError> {
        Err(ContentError::Damaged(NO_READER.into()))
    }
    fn save_manifest(&self, _: &[u8], _: &Wanted) -> Result<ManifestRead, ContentError> {
        Err(ContentError::Damaged(NO_READER.into()))
    }
}

const NO_READER: &str = "no parser process is installed to read it";

/// The installed reader, or one that reads nothing.
pub fn installed_reader() -> &'static dyn GameReader {
    READER.get().copied().unwrap_or(&NoReader)
}

/// Adds games to the move's plan (`plan`, made by [`crate::api::plan_within`] for the person
/// `owner` with home folder `home` on `from`): the saves the shipped rows and the person's own
/// save list name are found, the plan's app data that holds them is marked as games with their
/// notes ([`enrich`]), and saves no other part takes become components of their own
/// ([`components`]). `env` gives the person's folder values (only local folders), `moving` says
/// whether the move already takes a path with the person's own files.
#[allow(clippy::too_many_arguments)]
pub fn plan_games(
    plan: &mut Plan,
    owner: &Owner,
    home: &Path,
    documents: Option<PathBuf>,
    from: System,
    to: System,
    env: &dyn Fn(&str) -> Option<PathBuf>,
    moving: &dyn Fn(&Path) -> bool,
    until: Instant,
) -> GameReport {
    let places = Places::standard(from, home, documents, None, env);
    let roots = crate::roots::roots_of(from, home, &|_| None);
    let rows = Catalogues::shipped();
    let report = find(&FindInput {
        places: &places,
        to,
        roots: &roots,
        rows: &rows,
        reader: installed_reader(),
        registry: &SystemRegistry,
        moving,
        until,
    });
    enrich(plan, &report, from, to);
    plan.components.extend(components(owner, &report));
    report
}

// ---------- the licence hook ----------

/// Whether the Ludusavi manifest's data may be fetched: only once its licence is cleared.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct LicenceFlags {
    /// The founder cleared the PCGamingWiki data licence (CC BY-NC-SA 3.0) for PCTwin's use.
    pub ludusavi_data_cleared: bool,
}

/// Why the manifest was not fetched.
#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
pub enum FetchError {
    #[error(
        "the Ludusavi manifest's data is under a non-commercial licence (PCGamingWiki, CC BY-NC-SA 3.0) not yet cleared for PCTwin: it is not fetched"
    )]
    LicenceNotCleared,
    #[error(
        "fetching is not built: once the licence is cleared, the data comes through PCTwin's signed catalogue channel"
    )]
    NotBuilt,
}

/// The hook for fetching the Ludusavi manifest. Off: it refuses until the licence is cleared,
/// and even then fetching is not built here (it would come through the signed catalogue
/// channel, never a direct download).
pub fn fetch_manifest(flags: LicenceFlags) -> Result<Vec<u8>, FetchError> {
    if !flags.ludusavi_data_cleared {
        return Err(FetchError::LicenceNotCleared);
    }
    Err(FetchError::NotBuilt)
}

// ---------- reading, in the parser process ----------

/// PCTwin's reading of launcher records and save lists, in the parser process (`pctwin-parse`'s
/// `Parser` and `Pool`). Err: it could not be read (why).
pub trait GameReader: Sync {
    fn record(&self, format: RecordFormat, bytes: &[u8]) -> Result<Tree, ContentError>;
    fn save_manifest(&self, bytes: &[u8], wanted: &Wanted) -> Result<ManifestRead, ContentError>;
}

/// The readers in this process: only for the parser process and tests (it takes an
/// [`InChild`]).
#[derive(Debug, Clone, Copy)]
pub struct InThisProcess(InChild);

impl InThisProcess {
    pub const fn new(seal: InChild) -> Self {
        Self(seal)
    }
}

impl GameReader for InThisProcess {
    fn record(&self, format: RecordFormat, bytes: &[u8]) -> Result<Tree, ContentError> {
        crate::gamefmt::record_tree(self.0, format, bytes)
    }
    fn save_manifest(&self, bytes: &[u8], wanted: &Wanted) -> Result<ManifestRead, ContentError> {
        crate::gamefmt::save_manifest(
            self.0,
            &mut std::io::Cursor::new(bytes),
            bytes.len() as u64,
            wanted,
        )
    }
}

/// The Windows registry, read only.
pub trait Registry {
    /// The key at `path` (`HKEY_.../a/b`) as a tree: each subkey a map of its values (as text)
    /// and its own subkeys, two levels deep. `Ok(None)`: not there.
    fn tree(&self, path: &str) -> Result<Option<Tree>, String>;
}

/// This laptop's registry (none on a Mac or Linux).
pub struct SystemRegistry;

/// The most subkeys and values read under one key.
const MAX_KEYS: usize = 2_000;

#[cfg(windows)]
impl Registry for SystemRegistry {
    fn tree(&self, path: &str) -> Result<Option<Tree>, String> {
        use winreg::RegKey;
        use winreg::enums::{HKEY_CURRENT_USER, HKEY_LOCAL_MACHINE, KEY_READ};
        let (hive, rest) = path.split_once('/').unwrap_or((path, ""));
        let hive = match hive {
            "HKEY_LOCAL_MACHINE" => HKEY_LOCAL_MACHINE,
            "HKEY_CURRENT_USER" => HKEY_CURRENT_USER,
            _ => return Err("not a registry hive PCTwin reads".into()),
        };
        let key =
            match RegKey::predef(hive).open_subkey_with_flags(rest.replace('/', "\\"), KEY_READ) {
                Ok(k) => k,
                Err(e) if e.kind() == std::io::ErrorKind::NotFound => return Ok(None),
                Err(e) => return Err(e.kind().to_string()),
            };
        fn read(key: &RegKey, depth: usize) -> Tree {
            use winreg::types::FromRegValue;
            let mut out = Vec::new();
            for (name, value) in key.enum_values().flatten().take(MAX_KEYS) {
                let text = match value.vtype {
                    winreg::enums::REG_DWORD => u32::from_reg_value(&value)
                        .map(|n| n.to_string())
                        .unwrap_or_default(),
                    _ => String::from_reg_value(&value).unwrap_or_default(),
                };
                if text.len() <= crate::gamefmt::MAX_TEXT {
                    out.push((name, Tree::Text(text)));
                }
            }
            if depth < 2 {
                for name in key.enum_keys().flatten().take(MAX_KEYS) {
                    if let Ok(sub) = key.open_subkey_with_flags(&name, KEY_READ) {
                        out.push((name, read(&sub, depth + 1)));
                    }
                }
            }
            Tree::Map(out)
        }
        Ok(Some(read(&key, 0)))
    }
}

#[cfg(not(windows))]
impl Registry for SystemRegistry {
    fn tree(&self, _: &str) -> Result<Option<Tree>, String> {
        Ok(None)
    }
}

// ---------- placeholders ----------

/// The person's folders on one laptop, by the save format's placeholder names (`winAppData`,
/// `xdgConfig`, ...; `home` always), and their account name.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Places {
    pub system: System,
    pub home: PathBuf,
    pub user: Option<String>,
    pub folders: BTreeMap<String, PathBuf>,
}

impl Places {
    /// The standard folders of a person with home folder `home` on `system`: `documents` where
    /// the system has their Documents, `env` the person's own environment values (absolute
    /// paths only are used).
    pub fn standard(
        system: System,
        home: &Path,
        documents: Option<PathBuf>,
        user: Option<String>,
        env: &dyn Fn(&str) -> Option<PathBuf>,
    ) -> Self {
        let mut folders = BTreeMap::new();
        let mut put = |name: &str, var: Option<&str>, fallback: Option<PathBuf>| {
            let from_env = var.and_then(env).filter(|p| p.is_absolute());
            if let Some(p) = from_env.or(fallback) {
                folders.insert(name.to_string(), p);
            }
        };
        match system {
            System::Windows => {
                let app = home.join("AppData");
                put("winAppData", Some("APPDATA"), Some(app.join("Roaming")));
                put(
                    "winLocalAppData",
                    Some("LOCALAPPDATA"),
                    Some(app.join("Local")),
                );
                put("winLocalAppDataLow", None, Some(app.join("LocalLow")));
                put(
                    "winDocuments",
                    None,
                    Some(documents.unwrap_or_else(|| home.join("Documents"))),
                );
                put("winPublic", Some("PUBLIC"), None);
                put("winProgramData", Some("ProgramData"), None);
                put("winDir", Some("WINDIR"), None);
                put("winProgramFiles", Some("ProgramFiles"), None);
                put("winProgramFilesX86", Some("ProgramFiles(x86)"), None);
            }
            System::Linux => {
                put(
                    "xdgData",
                    Some("XDG_DATA_HOME"),
                    Some(home.join(".local/share")),
                );
                put(
                    "xdgConfig",
                    Some("XDG_CONFIG_HOME"),
                    Some(home.join(".config")),
                );
            }
            System::Mac => {}
        }
        Self {
            system,
            home: home.to_path_buf(),
            user,
            folders,
        }
    }

    fn folder(&self, name: &str) -> Option<PathBuf> {
        if name == "home" {
            Some(self.home.clone())
        } else {
            self.folders.get(name).cloned()
        }
    }
}

/// Placeholders that start a path: the person's folders, and a game's install places.
const STARTS: &[&str] = &[
    "base",
    "root",
    "home",
    "winAppData",
    "winLocalAppData",
    "winLocalAppDataLow",
    "winDocuments",
    "winPublic",
    "winProgramData",
    "winDir",
    "winProgramFiles",
    "winProgramFilesX86",
    "xdgData",
    "xdgConfig",
];
/// Placeholders inside a path.
const INSIDE: &[&str] = &["game", "storeUserId", "storeGameId", "osUserName"];

/// Whether a template is one PCTwin reads: a starting placeholder, then names (with `*`, `?`,
/// `**` and the inside placeholders), never `.` or `..`.
pub fn template_ok(t: &str) -> Result<(), String> {
    let bad = |why: &str| Err(format!("{why}: {t}"));
    if t.len() > 1024 {
        return bad("a place longer than PCTwin reads");
    }
    let mut parts = t.split('/');
    let first = parts.next().unwrap_or_default();
    let Some(start) = first.strip_prefix('<').and_then(|f| f.strip_suffix('>')) else {
        return bad("a place starts with one of the person's folders");
    };
    if !STARTS.contains(&start) {
        return bad("a place starts with one of the person's folders");
    }
    let rest: Vec<&str> = parts.collect();
    for (i, p) in rest.iter().enumerate() {
        if p.is_empty() && i + 1 == rest.len() {
            continue;
        }
        let mut plain = (*p).to_string();
        for name in INSIDE {
            plain = plain.replace(&format!("<{name}>"), "x");
        }
        if plain.is_empty()
            || plain == "."
            || plain == ".."
            || plain.contains(['<', '>', '\\', ':', '\0'])
            || plain.chars().any(char::is_control)
        {
            return bad("a place is plain names");
        }
    }
    Ok(())
}

/// One part of a place to look for.
#[derive(Debug, Clone, PartialEq, Eq)]
enum Seg {
    Lit(String),
    Glob(String),
    /// `**`: any folders, or none.
    Deep,
}

/// What fills a game's own placeholders.
#[derive(Debug, Clone, Default)]
struct GameCtx {
    /// The game's install folder (`<base>`), its library (`<root>`) and store ID.
    install: Option<PathBuf>,
    root: Option<PathBuf>,
    store_id: Option<String>,
    /// `<game>`: the install folder's name, else the entry's install folder name, else its name.
    game: String,
}

/// A template with the person's folders filled in: the folder it starts at and the parts
/// below. `None`: it needs something this laptop does not have (another system's folder, the
/// game's install folder when the game is not installed).
fn expand(t: &str, places: &Places, ctx: &GameCtx) -> Option<(PathBuf, Vec<Seg>)> {
    template_ok(t).ok()?;
    let mut parts = t.split('/');
    let start = parts.next()?.trim_start_matches('<').trim_end_matches('>');
    let base = match start {
        "base" => ctx.install.clone()?,
        "root" => ctx.root.clone()?,
        other => places.folder(other)?,
    };
    let mut segs = Vec::new();
    for p in parts {
        if p.is_empty() {
            continue;
        }
        let mut s = p.to_string();
        if s.contains("<game>") {
            if !plain_name(&ctx.game) {
                return None;
            }
            s = s.replace("<game>", &ctx.game);
        }
        if s.contains("<storeGameId>") {
            s = s.replace("<storeGameId>", ctx.store_id.as_deref()?);
        }
        s = s.replace("<storeUserId>", "*");
        let user = places
            .user
            .as_deref()
            .filter(|u| plain_name(u))
            .unwrap_or("*");
        s = s.replace("<osUserName>", user);
        segs.push(if s == "**" {
            Seg::Deep
        } else if s.contains(['*', '?']) {
            Seg::Glob(s)
        } else {
            Seg::Lit(s)
        });
    }
    Some((base, segs))
}

/// Whether `name` matches the pattern `pat` (`*` any text, `?` one character).
fn glob(pat: &str, name: &str, fold: bool) -> bool {
    let fold_s = |s: &str| {
        if fold {
            s.to_lowercase()
        } else {
            s.to_string()
        }
    };
    let (p, n): (Vec<char>, Vec<char>) = (
        fold_s(pat).chars().collect(),
        fold_s(name).chars().collect(),
    );
    let (mut pi, mut ni, mut star, mut mark) = (0, 0, None, 0);
    while ni < n.len() {
        if pi < p.len() && (p[pi] == '?' || p[pi] == n[ni]) {
            pi += 1;
            ni += 1;
        } else if pi < p.len() && p[pi] == '*' {
            star = Some(pi);
            mark = ni;
            pi += 1;
        } else if let Some(s) = star {
            pi = s + 1;
            mark += 1;
            ni = mark;
        } else {
            return false;
        }
    }
    while pi < p.len() && p[pi] == '*' {
        pi += 1;
    }
    pi == p.len()
}

/// Names compare ignoring case on Windows and a Mac (their usual file systems), not on Linux.
fn folds(system: System) -> bool {
    system != System::Linux
}

// ---------- looking ----------

/// The most files looked at per game, and the deepest.
const MAX_FILES: usize = 20_000;
const MAX_WALK_DEPTH: usize = 24;

/// What a look found so far.
#[derive(Debug, Default)]
struct Look {
    /// Places matched (files or folders).
    hits: Vec<PathBuf>,
    files: usize,
    online_only: u64,
    complete: bool,
}

fn look(dir: &Path, segs: &[Seg], fold: bool, depth: usize, until: Instant, out: &mut Look) {
    if depth > MAX_WALK_DEPTH || out.files >= MAX_FILES || Instant::now() >= until {
        out.complete = false;
        return;
    }
    let Some((first, rest)) = segs.split_first() else {
        out.hits.push(dir.to_path_buf());
        out.files += 1;
        return;
    };
    match first {
        Seg::Lit(name) => {
            let p = dir.join(name);
            match std::fs::symlink_metadata(&p).map(|m| guard::kind_of(&m)) {
                Ok(Kind::Folder) => look(&p, rest, fold, depth + 1, until, out),
                Ok(Kind::File) if rest.is_empty() => look(&p, rest, fold, depth + 1, until, out),
                Ok(Kind::OnlineOnly) => out.online_only += 1,
                Ok(Kind::Link) => out.complete = false,
                _ => {}
            }
        }
        Seg::Glob(pat) => {
            let Ok(entries) = std::fs::read_dir(dir) else {
                return;
            };
            for e in entries.flatten() {
                let name = e.file_name().to_string_lossy().into_owned();
                if !glob(pat, &name, fold) {
                    continue;
                }
                let p = e.path();
                match std::fs::symlink_metadata(&p).map(|m| guard::kind_of(&m)) {
                    Ok(Kind::Folder) => look(&p, rest, fold, depth + 1, until, out),
                    Ok(Kind::File) if rest.is_empty() => {
                        look(&p, rest, fold, depth + 1, until, out)
                    }
                    Ok(Kind::OnlineOnly) => out.online_only += 1,
                    Ok(Kind::Link) => out.complete = false,
                    _ => {}
                }
            }
        }
        Seg::Deep => {
            look(dir, rest, fold, depth + 1, until, out);
            let Ok(entries) = std::fs::read_dir(dir) else {
                return;
            };
            for e in entries.flatten() {
                let p = e.path();
                match std::fs::symlink_metadata(&p).map(|m| guard::kind_of(&m)) {
                    Ok(Kind::Folder) => look(&p, segs, fold, depth + 1, until, out),
                    Ok(Kind::Link) => out.complete = false,
                    _ => {}
                }
            }
        }
    }
}

/// The files in and below `p` (itself, if a file), never through a link.
fn files_of(p: &Path, depth: usize, until: Instant, out: &mut Vec<(PathBuf, u64)>, c: &mut Count) {
    if depth > MAX_WALK_DEPTH || out.len() >= MAX_FILES || Instant::now() >= until {
        c.complete = false;
        return;
    }
    let Ok(m) = std::fs::symlink_metadata(p) else {
        c.complete = false;
        return;
    };
    match guard::kind_of(&m) {
        Kind::File => out.push((p.to_path_buf(), m.len())),
        Kind::Folder => match std::fs::read_dir(p) {
            Ok(entries) => {
                for e in entries {
                    match e {
                        Ok(e) => files_of(&e.path(), depth + 1, until, out, c),
                        Err(_) => c.complete = false,
                    }
                }
            }
            Err(_) => c.complete = false,
        },
        Kind::OnlineOnly => c.online_only += 1,
        Kind::Link => c.complete = false,
        Kind::NotAFile => {}
    }
}

#[derive(Debug, Clone, Copy, Default)]
struct Count {
    online_only: u64,
    complete: bool,
}

/// Whether the file at `p` is a program, by its content (never its name).
fn is_program(p: &Path, len: u64) -> bool {
    let Ok(mut f) = guard::open(p) else {
        return false;
    };
    let mut head = vec![0u8; crate::sniff::HEAD];
    let n = (&mut f)
        .take(crate::sniff::HEAD as u64)
        .read(&mut head)
        .unwrap_or(0);
    head.truncate(n);
    let mut tail = Vec::new();
    if len > crate::sniff::TAIL as u64
        && f.seek(SeekFrom::End(-(crate::sniff::TAIL as i64))).is_ok()
    {
        let _ = (&mut f)
            .take(crate::sniff::TAIL as u64)
            .read_to_end(&mut tail);
    }
    crate::sniff::shape(&head, &tail, len) == crate::sniff::Shape::Executable
}

/// `p` inside `base`, `/` between folders.
fn rel(p: &Path, base: &Path) -> Option<String> {
    let r = p.strip_prefix(base).ok()?;
    let parts: Vec<String> = r
        .components()
        .map(|c| c.as_os_str().to_string_lossy().into_owned())
        .collect();
    (!parts.is_empty()).then(|| parts.join("/"))
}

// ---------- launchers ----------

/// An installed game a launcher lists.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct InstalledGame {
    pub launcher: String,
    pub launcher_name: String,
    pub name: String,
    pub id: Option<String>,
    /// Its install folder.
    pub folder: PathBuf,
    /// The library it is in (`<root>`).
    pub library: PathBuf,
    pub files: u64,
    pub bytes: u64,
    /// Every file was counted.
    pub complete: bool,
    /// The launcher marks its saves as kept online.
    pub cloud: bool,
    /// The launcher's own verify address for it, opened as the person.
    pub verify: Option<String>,
    pub page: Option<String>,
    /// Its files are copied only if the person chooses (never by default).
    pub copy_files: bool,
    pub result: ItemResult,
}

fn launcher_text(t: &Tree, field: &str) -> Option<String> {
    let s = t.get(field)?.text()?.trim();
    (!s.is_empty() && s.len() <= 200 && !s.chars().any(char::is_control)).then(|| s.to_string())
}

fn store_id_ok(s: &str) -> bool {
    !s.is_empty()
        && s.len() <= 64
        && s.bytes()
            .all(|b| b.is_ascii_alphanumeric() || matches!(b, b'.' | b'_' | b'-'))
}

/// A full path a launcher record names: absolute, with no `.` or `..`, and not a drive's top.
fn record_path(s: &str) -> Option<PathBuf> {
    let p = PathBuf::from(s);
    let normal = p
        .components()
        .filter(|c| matches!(c, std::path::Component::Normal(_)))
        .count();
    let odd = p.components().any(|c| {
        matches!(
            c,
            std::path::Component::ParentDir | std::path::Component::CurDir
        )
    });
    (p.is_absolute() && !odd && normal >= 1 && !s.contains('\0')).then_some(p)
}

/// The record trees of `r` inside `dir` (or the registry).
fn trees(
    r: &Records,
    dir: Option<&Path>,
    reader: &dyn GameReader,
    registry: &dyn Registry,
    fold: bool,
    not_checked: &mut Vec<(String, String)>,
) -> Vec<Tree> {
    let format = match r.format {
        Format::Vdf => RecordFormat::Vdf,
        Format::Json => RecordFormat::Json,
        Format::Registry => {
            let key = r.registry.as_deref().unwrap_or_default();
            return match registry.tree(key) {
                Ok(Some(t)) => vec![t],
                Ok(None) => Vec::new(),
                Err(why) => {
                    not_checked.push((key.to_string(), why));
                    Vec::new()
                }
            };
        }
    };
    let Some(dir) = dir else {
        return Vec::new();
    };
    let mut paths = Vec::new();
    if let Some(f) = &r.file {
        paths.push(f.split('/').fold(dir.to_path_buf(), |p, n| p.join(n)));
    } else if let (Some(folder), Some(pat)) = (&r.folder, &r.files) {
        let folder = folder.split('/').fold(dir.to_path_buf(), |p, n| p.join(n));
        if let Ok(entries) = std::fs::read_dir(&folder) {
            for e in entries.flatten().take(MAX_KEYS) {
                if glob(pat, &e.file_name().to_string_lossy(), fold) {
                    paths.push(e.path());
                }
            }
        }
        paths.sort();
    }
    let mut out = Vec::new();
    for p in paths {
        match std::fs::symlink_metadata(&p).map(|m| guard::kind_of(&m)) {
            Ok(Kind::File) => {}
            Err(e) if e.kind() == std::io::ErrorKind::NotFound => continue,
            _ => {
                not_checked.push((p.display().to_string(), "not a plain file".into()));
                continue;
            }
        }
        match guard::read_all(&p, crate::gamefmt::MAX_RECORD)
            .map_err(|e| e.to_string())
            .and_then(|b| reader.record(format, &b).map_err(|e| e.to_string()))
        {
            Ok(t) => out.push(t),
            Err(why) => not_checked.push((p.display().to_string(), why)),
        }
    }
    out
}

/// The records in `trees` (at `r.at`, one per child when `r.each`).
fn records<'t>(r: &Records, trees: &'t [Tree]) -> Vec<&'t Tree> {
    let mut out = Vec::new();
    for t in trees {
        let Some(at) = t.at(&r.at) else { continue };
        if r.each {
            out.extend(at.children().iter().map(|(_, c)| c));
        } else {
            out.push(at);
        }
    }
    out
}

fn bases(row: &LauncherRow, places: &Places) -> Vec<PathBuf> {
    let mut out = Vec::new();
    for t in &row.base {
        let Some((base, segs)) = expand(t, places, &GameCtx::default()) else {
            continue;
        };
        let mut l = Look {
            complete: true,
            ..Look::default()
        };
        look(
            &base,
            &segs,
            folds(places.system),
            0,
            Instant::now() + std::time::Duration::from_secs(5),
            &mut l,
        );
        for h in l.hits {
            if h.is_dir() && !out.contains(&h) {
                out.push(h);
            }
        }
    }
    out
}

/// The games `row`'s launcher lists on this laptop.
fn installed(
    row: &LauncherRow,
    places: &Places,
    reader: &dyn GameReader,
    registry: &dyn Registry,
    until: Instant,
    not_checked: &mut Vec<(String, String)>,
) -> Vec<InstalledGame> {
    let fold = folds(places.system);
    let bases = bases(row, places);
    // Library folders: those the launcher lists, else its own folders.
    let mut libraries: Vec<PathBuf> = Vec::new();
    if let Some(l) = &row.libraries {
        for b in &bases {
            let t = trees(l, Some(b), reader, registry, fold, not_checked);
            for rec in records(l, &t) {
                let Some(p) = l
                    .path
                    .as_deref()
                    .and_then(|f| launcher_text(rec, f))
                    .and_then(|s| record_path(&s))
                else {
                    continue;
                };
                if p.is_dir() && !libraries.contains(&p) {
                    libraries.push(p);
                }
            }
        }
    }
    if libraries.is_empty() {
        libraries = bases.clone();
    }
    let mut found: Vec<(Tree, PathBuf)> = Vec::new();
    if row.games.registry.is_some() {
        for t in trees(&row.games, None, reader, registry, fold, not_checked) {
            found.push((t, PathBuf::new()));
        }
    } else {
        for lib in &libraries {
            for t in trees(&row.games, Some(lib), reader, registry, fold, not_checked) {
                found.push((t, lib.clone()));
            }
        }
    }
    let mut out: Vec<InstalledGame> = Vec::new();
    for (tree, lib) in &found {
        for rec in records(&row.games, std::slice::from_ref(tree)) {
            let g = &row.games;
            let Some(name) = g.name.as_deref().and_then(|f| launcher_text(rec, f)) else {
                continue;
            };
            let id =
                g.id.as_deref()
                    .and_then(|f| launcher_text(rec, f))
                    .filter(|s| store_id_ok(s));
            let Some(install) = g.install.as_deref().and_then(|f| launcher_text(rec, f)) else {
                continue;
            };
            let (folder, library) = match &g.install_in {
                Some(inside) => {
                    if !plain_name(&install) {
                        not_checked.push((name.clone(), "its folder is not a plain name".into()));
                        continue;
                    }
                    let dir = inside.split('/').fold(lib.clone(), |p, n| p.join(n));
                    (dir.join(&install), lib.clone())
                }
                None => match record_path(&install) {
                    Some(p) => {
                        let library = p.parent().map(Path::to_path_buf).unwrap_or_default();
                        (p, library)
                    }
                    None => {
                        not_checked.push((name.clone(), "its folder is not a full path".into()));
                        continue;
                    }
                },
            };
            match std::fs::symlink_metadata(&folder).map(|m| guard::kind_of(&m)) {
                Ok(Kind::Folder) => {}
                _ => {
                    not_checked.push((
                        name.clone(),
                        "the launcher lists it, but its folder is not on this laptop".into(),
                    ));
                    continue;
                }
            }
            if out.iter().any(|o| o.folder == folder) {
                continue;
            }
            let mut files = Vec::new();
            let mut c = Count {
                complete: true,
                ..Count::default()
            };
            files_of(&folder, 0, until, &mut files, &mut c);
            let cloud = match (&row.cloud_marker, &id) {
                (Some(m), Some(id)) => bases.iter().any(|b| {
                    let ctx = GameCtx {
                        root: Some(b.clone()),
                        store_id: Some(id.clone()),
                        ..GameCtx::default()
                    };
                    let t = format!("<root>/{m}");
                    expand(&t, places, &ctx).is_some_and(|(base, segs)| {
                        let mut l = Look {
                            complete: true,
                            ..Look::default()
                        };
                        look(&base, &segs, fold, 0, until, &mut l);
                        !l.hits.is_empty()
                    })
                }),
                _ => false,
            };
            let verify = match (&row.verify, &id) {
                (Some(v), Some(id)) => Some(v.replace("<storeGameId>", id)),
                (Some(v), None) if !v.contains("<storeGameId>") => Some(v.clone()),
                _ => None,
            };
            out.push(InstalledGame {
                launcher: row.launcher.clone(),
                launcher_name: row.name.clone(),
                name,
                id,
                folder,
                library,
                files: files.len() as u64,
                bytes: files.iter().map(|(_, n)| n).sum(),
                complete: c.complete,
                cloud,
                verify,
                page: row.page.clone(),
                copy_files: false,
                result: ItemResult::because(Reason::FinishByHand),
            });
        }
    }
    out
}

/// Opens the launcher's own verify (or its help page) for `g`, as the person: the launcher
/// checks the game's files itself. Never "done": the person confirms.
pub fn open_verify(person: &dyn Person, g: &InstalledGame) -> Opened {
    match g.verify.as_deref().or(g.page.as_deref()) {
        Some(url) => match person.open_page(url) {
            Ok(()) => Opened::Opened,
            Err(e) => Opened::NotOpened(e.kind().to_string()),
        },
        None => Opened::InWords,
    }
}

/// The result of an installed game once the person says whether the launcher found and
/// checked it: needs-action until then.
pub fn installed_result(confirmed: Option<bool>) -> ItemResult {
    match confirmed {
        Some(true) => ItemResult::done(pctwin_types::ItemStatus::Reinstalled)
            .unwrap_or_else(|_| ItemResult::because(Reason::FinishByHand)),
        Some(false) => ItemResult::skipped_by_user(),
        None => ItemResult::because(Reason::FinishByHand),
    }
}

// ---------- saves ----------

/// Where a found save goes.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum Route {
    /// Inside an app data root: already in the move as app data ([`enrich`] marks it).
    WithAppData,
    /// Inside the person's home, in a folder the move already takes with their own files.
    WithFiles,
    /// Its own component ([`components`]).
    Own,
}

/// What a found place is.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum Found {
    Saves,
    Mods,
}

/// Where a row came from.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum Source {
    /// PCTwin's own signed `game-saves` rows.
    Pctwin,
    /// The person's own save list (Ludusavi's copy of its manifest).
    PersonsList,
}

/// A game's saves (or add-ons) found in one place.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SaveFound {
    pub game: String,
    pub what: Found,
    pub source: Source,
    /// The folder its files are inside.
    pub base: PathBuf,
    /// Its files, inside `base`, `/` between folders: never a program.
    pub members: Vec<String>,
    pub bytes: u64,
    /// Programs found among them (by content): listed apart, never copied or turned on.
    pub programs: Vec<String>,
    pub online_only: u64,
    pub complete: bool,
    pub tags: Vec<String>,
    pub route: Route,
    /// The launcher that keeps these saves online, if it does (and the game came from it).
    pub cloud: Option<String>,
    /// Where it goes on the new laptop.
    pub placement: Placement,
    pub notes: Vec<Note>,
}

/// A place a store keeps protected for itself: warned, not copied.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Protected {
    pub name: String,
    pub place: PathBuf,
    pub files: u64,
    pub bytes: u64,
}

/// The person's own save list, as read.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SaveListRead {
    pub place: PathBuf,
    pub entries: usize,
    pub unreadable: u64,
    pub more: bool,
}

/// What the games finder found, and what it could not check.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct GameReport {
    pub installed: Vec<InstalledGame>,
    pub saves: Vec<SaveFound>,
    pub protected: Vec<Protected>,
    pub save_list: Option<SaveListRead>,
    /// Registry saves a row names: not moved by this part (the settings part reads the
    /// registry), listed with their game.
    pub registry: Vec<(String, String)>,
    pub not_checked: Vec<(String, String)>,
}

/// Everything [`find`] needs.
pub struct FindInput<'a> {
    /// The person's folders on the old laptop.
    pub places: &'a Places,
    /// The new laptop's system.
    pub to: System,
    /// The person's app data roots on the old laptop ([`crate::roots::roots_of`]).
    pub roots: &'a [DataRoot],
    pub rows: &'a Catalogues,
    pub reader: &'a dyn GameReader,
    pub registry: &'a dyn Registry,
    /// Whether the move already takes a path with the person's own files.
    pub moving: &'a dyn Fn(&Path) -> bool,
    pub until: Instant,
}

fn os_word(s: System) -> &'static str {
    match s {
        System::Windows => "windows",
        System::Mac => "mac",
        System::Linux => "linux",
    }
}

/// Whether a path's conditions let it apply on `system` (a store condition is left to the
/// path itself: it is looked for, and found or not).
fn applies(rule: &crate::gamefmt::FileRule, system: System) -> bool {
    rule.when.is_empty()
        || rule
            .when
            .iter()
            .any(|w| w.os.as_deref().is_none_or(|o| o == os_word(system)))
}

/// The root kind and folder on `to` a template names, with the parts left after it (globs).
fn on_new(t: &str, to: System) -> Option<(RootKind, String, Vec<String>)> {
    template_ok(t).ok()?;
    let mut parts: Vec<&str> = t.split('/').filter(|p| !p.is_empty()).collect();
    let start = parts
        .remove(0)
        .trim_start_matches('<')
        .trim_end_matches('>');
    let (root, skip) = match (to, start) {
        (System::Windows, "winAppData") => (RootKind::Roaming, 0),
        (System::Windows, "winLocalAppData") => (RootKind::Local, 0),
        (System::Windows, "winLocalAppDataLow") => (RootKind::LocalLow, 0),
        (System::Windows, "home") if parts.first() == Some(&"Saved Games") => {
            (RootKind::SavedGames, 1)
        }
        (System::Linux, "xdgData") => (RootKind::XdgData, 0),
        (System::Linux, "xdgConfig") => (RootKind::XdgConfig, 0),
        (System::Mac, "home")
            if parts.first() == Some(&"Library")
                && parts.get(1) == Some(&"Application Support") =>
        {
            (RootKind::ApplicationSupport, 2)
        }
        (_, "home")
            if parts
                .first()
                .is_some_and(|p| p.starts_with('.') && p.len() > 1) =>
        {
            (RootKind::HomeDot, 0)
        }
        _ => return None,
    };
    let rest = &parts[skip..];
    let lit = rest
        .iter()
        .take_while(|p| !p.contains(['*', '?', '<']))
        .count();
    if lit == 0 {
        return None;
    }
    Some((
        root,
        rest[..lit].join("/"),
        rest[lit..].iter().map(|s| (*s).to_string()).collect(),
    ))
}

/// The parts of a template after its literal folder (as written).
fn tail_of(t: &str) -> Vec<String> {
    t.split('/')
        .skip(1)
        .filter(|p| !p.is_empty())
        .skip_while(|p| !p.contains(['*', '?', '<']))
        .map(str::to_string)
        .collect()
}

/// Where a save found by `from_rule` of `row` goes on `to`: the game's one place there when the
/// row says the format is the same, else kept (preserved only).
fn translate(row: Option<&SaveRow>, from_rule: &str, from: System, to: System) -> Placement {
    let Some(row) = row.filter(|r| r.same_format.contains(&from) && r.same_format.contains(&to))
    else {
        return Placement::PreservedOnly;
    };
    let from_rule_tags = row
        .entry
        .files
        .get(from_rule)
        .map(|r| r.tags.clone())
        .unwrap_or_default();
    let tail = tail_of(from_rule);
    let candidates: Vec<(RootKind, String)> = row
        .entry
        .files
        .iter()
        .filter(|(_, r)| applies(r, to) && r.tags == from_rule_tags)
        .filter_map(|(t, _)| on_new(t, to))
        .filter(|(_, _, rest)| *rest == tail)
        .map(|(root, folder, _)| (root, folder))
        .collect();
    match candidates.as_slice() {
        [(root, folder)] => Placement::Translated {
            root: *root,
            folder: folder.clone(),
        },
        _ => Placement::PreservedOnly,
    }
}

struct Entry<'a> {
    entry: &'a SaveEntry,
    row: Option<&'a SaveRow>,
    source: Source,
}

/// Finds games, their saves and add-ons on the old laptop.
pub fn find(i: &FindInput<'_>) -> GameReport {
    let mut report = GameReport::default();
    let system = i.places.system;
    let fold = folds(system);
    // Installed games, from each launcher's own records.
    for row in i
        .rows
        .launchers
        .rows()
        .iter()
        .filter(|r| r.system.covers(system))
    {
        let games = installed(
            row,
            i.places,
            i.reader,
            i.registry,
            i.until,
            &mut report.not_checked,
        );
        report.installed.extend(games);
    }
    // PCTwin's own rows, then the person's own save list for installed games no row knows.
    let mut entries: Vec<Entry<'_>> = i
        .rows
        .saves
        .rows()
        .iter()
        .map(|r| Entry {
            entry: &r.entry,
            row: Some(r),
            source: Source::Pctwin,
        })
        .collect();
    let persons = persons_list(i, &mut report);
    for e in &persons {
        if !entries
            .iter()
            .any(|x| x.entry.name.eq_ignore_ascii_case(&e.name))
        {
            entries.push(Entry {
                entry: e,
                row: None,
                source: Source::PersonsList,
            });
        }
    }
    let mut claimed = std::collections::BTreeSet::new();
    for e in &entries {
        let game = installed_for(e.entry, &report.installed);
        let ctx = ctx_for(e.entry, game);
        let cloud = game.and_then(|g| {
            (g.cloud || e.entry.cloud.get(&g.launcher) == Some(&true))
                .then(|| g.launcher_name.clone())
        });
        for (template, rule) in &e.entry.files {
            if !applies(rule, system) {
                continue;
            }
            let Some((base, segs)) = expand(template, i.places, &ctx) else {
                continue;
            };
            let placement = if system == i.to {
                Placement::PreservedOnly
            } else {
                translate(e.row, template, system, i.to)
            };
            if let Some(s) = found_at(
                i,
                &e.entry.name,
                Found::Saves,
                e.source,
                &base,
                &segs,
                fold,
                &rule.tags,
                &mut claimed,
            ) {
                report.saves.push(SaveFound {
                    cloud: cloud.clone(),
                    placement,
                    notes: Vec::new(),
                    ..s
                });
            }
        }
        for (key, rule) in &e.entry.registry {
            if system == System::Windows && applies(rule, system) && game.is_some() {
                report.registry.push((e.entry.name.clone(), key.clone()));
            }
        }
    }
    // Places rows: store-protected saves and add-on folders.
    for row in i
        .rows
        .places
        .rows()
        .iter()
        .filter(|r| r.system.covers(system))
    {
        match row.kind {
            PlaceKind::SaveList => {}
            PlaceKind::StoreProtected => {
                let Some((base, segs)) = expand(&row.path, i.places, &GameCtx::default()) else {
                    continue;
                };
                let mut l = Look {
                    complete: true,
                    ..Look::default()
                };
                look(&base, &segs, fold, 0, i.until, &mut l);
                for h in l.hits {
                    let mut files = Vec::new();
                    let mut c = Count {
                        complete: true,
                        ..Count::default()
                    };
                    files_of(&h, 0, i.until, &mut files, &mut c);
                    if files.is_empty() {
                        continue;
                    }
                    report.protected.push(Protected {
                        name: row.name.clone(),
                        place: h,
                        files: files.len() as u64,
                        bytes: files.iter().map(|(_, n)| n).sum(),
                    });
                }
            }
            PlaceKind::Mods => {
                let name = row.game.clone().unwrap_or_default();
                let entry = entries
                    .iter()
                    .find(|e| e.entry.name.eq_ignore_ascii_case(&name))
                    .map(|e| e.entry);
                let game = entry.and_then(|e| installed_for(e, &report.installed));
                let ctx = entry.map_or_else(
                    || GameCtx {
                        game: name.clone(),
                        ..GameCtx::default()
                    },
                    |e| ctx_for(e, game),
                );
                let Some((base, segs)) = expand(&row.path, i.places, &ctx) else {
                    continue;
                };
                if let Some(s) = found_at(
                    i,
                    &name,
                    Found::Mods,
                    Source::Pctwin,
                    &base,
                    &segs,
                    fold,
                    &[],
                    &mut claimed,
                ) {
                    report.saves.push(s);
                }
            }
        }
    }
    // Saves in a protected place are the store's, never copied.
    let protected: Vec<PathBuf> = report.protected.iter().map(|p| p.place.clone()).collect();
    report
        .saves
        .retain(|s| !protected.iter().any(|p| s.base.starts_with(p)));
    for s in &mut report.saves {
        s.route = route_of(&s.base, i);
        s.notes = notes_of(s, system, i.to, &entries);
        said_for_route(&mut s.notes, s.route);
    }
    report
}

/// The installed game an entry is for: by store ID, else by install folder name, else by name.
fn installed_for<'g>(e: &SaveEntry, installed: &'g [InstalledGame]) -> Option<&'g InstalledGame> {
    let id_of = |store: &str| match store {
        "steam" => e.steam.map(|s| s.id),
        "gog" => e.gog.map(|s| s.id),
        _ => None,
    };
    installed
        .iter()
        .find(|g| {
            g.id.as_deref()
                .and_then(|i| i.parse::<u64>().ok())
                .is_some_and(|i| id_of(&g.launcher) == Some(i))
        })
        .or_else(|| {
            installed.iter().find(|g| {
                g.folder.file_name().is_some_and(|n| {
                    e.install_dir
                        .keys()
                        .any(|d| n.to_string_lossy().eq_ignore_ascii_case(d))
                })
            })
        })
        .or_else(|| {
            installed
                .iter()
                .find(|g| g.name.eq_ignore_ascii_case(&e.name))
        })
}

fn ctx_for(e: &SaveEntry, g: Option<&InstalledGame>) -> GameCtx {
    let game = g
        .and_then(|g| g.folder.file_name())
        .map(|n| n.to_string_lossy().into_owned())
        .or_else(|| e.install_dir.keys().next().cloned())
        .unwrap_or_else(|| e.name.clone());
    GameCtx {
        install: g.map(|g| g.folder.clone()),
        root: g.map(|g| g.library.clone()),
        store_id: g.and_then(|g| g.id.clone()),
        game,
    }
}

/// The files at one place, gathered (never one already claimed by an earlier place).
#[allow(clippy::too_many_arguments)]
fn found_at(
    i: &FindInput<'_>,
    game: &str,
    what: Found,
    source: Source,
    base: &Path,
    segs: &[Seg],
    fold: bool,
    tags: &[String],
    claimed: &mut std::collections::BTreeSet<PathBuf>,
) -> Option<SaveFound> {
    let mut l = Look {
        complete: true,
        ..Look::default()
    };
    look(base, segs, fold, 0, i.until, &mut l);
    if l.hits.is_empty() && l.online_only == 0 {
        return None;
    }
    // The folder the files are inside: the place's literal start, or a folder matched whole.
    let lit = segs.iter().take_while(|s| matches!(s, Seg::Lit(_))).count();
    let mut inside = segs[..lit].iter().fold(base.to_path_buf(), |p, s| match s {
        Seg::Lit(n) => p.join(n),
        _ => p,
    });
    if lit == segs.len() && l.hits.len() == 1 && !l.hits[0].is_dir() {
        inside = l.hits[0].parent().map(Path::to_path_buf).unwrap_or(inside);
    }
    let mut files = Vec::new();
    let mut c = Count {
        complete: l.complete,
        online_only: l.online_only,
    };
    for h in &l.hits {
        files_of(h, 0, i.until, &mut files, &mut c);
    }
    let mut members = Vec::new();
    let mut programs = Vec::new();
    let mut bytes = 0;
    for (p, len) in files {
        if claimed.contains(&p) {
            continue;
        }
        let Some(r) = rel(&p, &inside) else { continue };
        if is_program(&p, len) {
            programs.push(r);
        } else {
            members.push(r);
            bytes += len;
        }
        claimed.insert(p);
    }
    if members.is_empty() && programs.is_empty() && c.online_only == 0 {
        return None;
    }
    members.sort();
    programs.sort();
    Some(SaveFound {
        game: game.to_string(),
        what,
        source,
        base: inside,
        members,
        bytes,
        programs,
        online_only: c.online_only,
        complete: c.complete,
        tags: tags.to_vec(),
        route: Route::Own,
        cloud: None,
        placement: Placement::PreservedOnly,
        notes: Vec::new(),
    })
}

fn route_of(base: &Path, i: &FindInput<'_>) -> Route {
    if i.roots
        .iter()
        .any(|r| !r.kind.is_cache() && base.starts_with(&r.path) && base != r.path)
    {
        Route::WithAppData
    } else if base.starts_with(&i.places.home) && (i.moving)(base) {
        Route::WithFiles
    } else {
        Route::Own
    }
}

/// Saves among the person's own files move with them: their programs are not held back there, so
/// the note says what really happens (2.5 security review, 30 October 2026).
fn said_for_route(notes: &mut [Note], route: Route) {
    if route == Route::WithFiles {
        for n in notes.iter_mut() {
            if *n == Note::ModProgramsHeld {
                *n = Note::ModProgramsAsFiles;
            }
        }
    }
}

fn notes_of(s: &SaveFound, from: System, to: System, entries: &[Entry<'_>]) -> Vec<Note> {
    let mut out = Vec::new();
    let entry = entries
        .iter()
        .find(|e| e.entry.name.eq_ignore_ascii_case(&s.game));
    if s.cloud.is_some() {
        out.push(Note::SavesInLauncherCloud);
    } else if entry.is_some_and(|e| e.entry.cloud.values().any(|v| *v)) {
        out.push(Note::CloudSavesMayDiffer);
    }
    if from != to && s.placement == Placement::PreservedOnly {
        out.push(Note::SaveMayNotWorkHere);
    }
    if !s.programs.is_empty() {
        out.push(Note::ModProgramsHeld);
    }
    out
}

/// The person's own save list, read for the installed games (Ludusavi's own copy of its manifest,
/// where a `game-places` row says it is).
fn persons_list(i: &FindInput<'_>, report: &mut GameReport) -> Vec<SaveEntry> {
    if report.installed.is_empty() {
        return Vec::new();
    }
    let wanted = Wanted {
        names: report.installed.iter().map(|g| g.name.clone()).collect(),
        steam: ids(&report.installed, "steam"),
        gog: ids(&report.installed, "gog"),
    };
    let fold = folds(i.places.system);
    for row in i
        .rows
        .places
        .rows()
        .iter()
        .filter(|r| r.kind == PlaceKind::SaveList && r.system.covers(i.places.system))
    {
        let Some((base, segs)) = expand(&row.path, i.places, &GameCtx::default()) else {
            continue;
        };
        let mut l = Look {
            complete: true,
            ..Look::default()
        };
        look(&base, &segs, fold, 0, i.until, &mut l);
        let Some(path) = l.hits.into_iter().find(|h| h.is_file()) else {
            continue;
        };
        let read = guard::read_all(&path, crate::gamefmt::MAX_MANIFEST)
            .map_err(|e| e.to_string())
            .and_then(|b| {
                i.reader
                    .save_manifest(&b, &wanted)
                    .map_err(|e| e.to_string())
            });
        match read {
            Ok(r) => {
                report.save_list = Some(SaveListRead {
                    place: path,
                    entries: r.entries.len(),
                    unreadable: r.unreadable,
                    more: r.more,
                });
                return r
                    .entries
                    .into_iter()
                    .filter(|e| e.alias.is_none())
                    .collect();
            }
            Err(why) => report.not_checked.push((path.display().to_string(), why)),
        }
    }
    Vec::new()
}

fn ids(installed: &[InstalledGame], launcher: &str) -> Vec<u64> {
    installed
        .iter()
        .filter(|g| g.launcher == launcher)
        .filter_map(|g| g.id.as_deref()?.parse().ok())
        .collect()
}

// ---------- into the move ----------

fn why_of(s: &SaveFound) -> String {
    let what = match s.what {
        Found::Saves => format!("saves of {}", s.game),
        Found::Mods => format!("add-ons of {}", s.game),
    };
    match (&s.cloud, &s.placement) {
        (Some(l), _) => format!(
            "{what}: {l} keeps them online and brings them back; not copied unless you choose, and then kept beside, never over the launcher's copy"
        ),
        (None, Placement::PreservedOnly) => format!(
            "{what}: kept on the new laptop in PCTwin's folder of things brought over; put them back where the game looks, or let the game find them"
        ),
        (None, _) => format!("{what}: copied to where the game keeps them on the new laptop"),
    }
}

/// The components the move takes for found saves and add-ons that no other part takes: each
/// kept on the new laptop (preserved only, needs-action) unless its row hands it to the game
/// there; saves the launcher keeps online are not proposed.
pub fn components(owner: &Owner, report: &GameReport) -> Vec<Planned> {
    report
        .saves
        .iter()
        .filter(|s| s.route == Route::Own && !s.members.is_empty())
        .map(|s| {
            let place = s.base.display().to_string();
            let component = Component::new(
                owner.clone(),
                &place,
                s.members.clone(),
                Custody::Portable,
                false,
                vec![Evidence::Catalogue {
                    row: s.game.clone(),
                }],
            );
            let placement = if s.cloud.is_some() {
                Placement::PreservedOnly
            } else {
                s.placement.clone()
            };
            Planned {
                app: s.game.clone(),
                component,
                base: s.base.clone(),
                step: Step::None,
                why: why_of(s),
                proposed: s.cloud.is_none(),
                placement,
                // A game's own save place outside the app data roots is filed with the games
                // root kind (the root is a label here; the component's place is its own).
                root: RootKind::SavedGames,
                category: Category::Game,
                notes: s.notes.clone(),
            }
        })
        .collect()
}

/// Marks the move's app data components that hold found saves: their category is games and
/// they carry the saves' notes; saves the launcher keeps online are not proposed and, if
/// chosen, kept beside; between different systems they are handed to the game only through
/// their row (else kept, "may not work on this system"); programs and store-protected files
/// among them are taken out and listed as not moved, with why.
pub fn enrich(plan: &mut Plan, report: &GameReport, from: System, to: System) {
    let mut kept = Vec::new();
    for mut p in std::mem::take(&mut plan.components) {
        let abs: Vec<PathBuf> = p.component.members.iter().map(|m| p.base.join(m)).collect();
        // Store-protected files and programs found in saves: out, listed.
        let out_of = |a: &PathBuf| -> Option<&'static str> {
            if report.protected.iter().any(|x| a.starts_with(&x.place)) {
                Some(
                    "saves a store keeps protected for itself: they come back from the store once you sign in there; not copied",
                )
            } else if report
                .saves
                .iter()
                .any(|s| s.programs.iter().any(|r| s.base.join(r) == *a))
            {
                Some(
                    "a program among a game's saves or add-ons: listed, never copied or turned on; get it again from its maker",
                )
            } else {
                None
            }
        };
        let mut left = Vec::new();
        for (m, a) in p.component.members.iter().zip(&abs) {
            match out_of(a) {
                Some(why) => plan.not_moved.push(NotMovedItem {
                    app: p.app.clone(),
                    rel: m.clone(),
                    why: why.to_string(),
                    count: 1,
                }),
                None => left.push(m.clone()),
            }
        }
        if left.is_empty() {
            continue;
        }
        if left.len() != p.component.members.len() {
            let c = &p.component;
            let mut nc = Component::new(
                c.owner.clone(),
                &c.place,
                left,
                c.custody,
                c.secret,
                c.evidence.clone(),
            );
            nc.needs = c.needs.clone();
            nc.preserve = c.preserve;
            p.component = nc;
        }
        let saves: Vec<&SaveFound> = report
            .saves
            .iter()
            .filter(|s| {
                s.route == Route::WithAppData
                    && p.component
                        .members
                        .iter()
                        .any(|m| p.base.join(m).starts_with(&s.base))
            })
            .collect();
        if !saves.is_empty() && !p.component.secret {
            p.category = Category::Game;
            for s in &saves {
                for n in &s.notes {
                    if !p.notes.contains(n) {
                        p.notes.push(n.clone());
                    }
                }
            }
            if saves.iter().any(|s| s.cloud.is_some()) {
                p.proposed = false;
                p.placement = Placement::PreservedOnly;
            } else if from != to {
                p.placement = enriched_placement(&p, &saves);
                if p.placement == Placement::PreservedOnly
                    && !p.notes.contains(&Note::SaveMayNotWorkHere)
                {
                    p.notes.push(Note::SaveMayNotWorkHere);
                }
            }
        }
        kept.push(p);
    }
    plan.components = kept;
}

/// Between systems: the component's own place on the new laptop, when all its files are in one
/// found save place handed to the game there; else kept (preserved only).
fn enriched_placement(p: &Planned, saves: &[&SaveFound]) -> Placement {
    let [s] = saves else {
        return Placement::PreservedOnly;
    };
    let all_inside = p
        .component
        .members
        .iter()
        .all(|m| p.base.join(m).starts_with(&s.base));
    let Placement::Translated { root, folder } = &s.placement else {
        return Placement::PreservedOnly;
    };
    let Some(inside) = rel(&s.base, &p.base) else {
        // The component is the save place itself.
        return if all_inside && s.base == p.base {
            s.placement.clone()
        } else {
            Placement::PreservedOnly
        };
    };
    match folder.strip_suffix(&format!("/{inside}")) {
        Some(f) if all_inside && !f.is_empty() => Placement::Translated {
            root: *root,
            folder: f.to_string(),
        },
        _ => Placement::PreservedOnly,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn globs_match_as_the_format_means() {
        assert!(glob("*.sav", "slot1.SAV", true));
        assert!(!glob("*.sav", "slot1.SAV", false));
        assert!(glob("appmanifest_*.acf", "appmanifest_70.acf", true));
        assert!(glob("a?c", "abc", false));
        assert!(!glob("a?c", "abbc", false));
        assert!(glob("*", "", false));
        assert!(!glob("x*", "y", false));
    }

    #[test]
    fn templates_are_checked() {
        assert!(template_ok("<winAppData>/Game/Saves").is_ok());
        assert!(template_ok("<base>/userdata/<storeUserId>/*.sav").is_ok());
        assert!(template_ok("<home>/**/x").is_ok());
        for bad in [
            "C:/Games/x",
            "<unknown>/x",
            "<home>/../x",
            "<home>/./x",
            "<home>//x",
            "<home>/a<b>",
            "<home>/a:b",
            "x/<home>",
        ] {
            assert!(template_ok(bad).is_err(), "{bad}");
        }
    }

    #[test]
    fn templates_name_their_place_on_the_new_system() {
        assert_eq!(
            on_new("<xdgData>/Game/Saves", System::Linux),
            Some((RootKind::XdgData, "Game/Saves".into(), vec![]))
        );
        assert_eq!(
            on_new("<home>/Library/Application Support/Game/*.sav", System::Mac),
            Some((
                RootKind::ApplicationSupport,
                "Game".into(),
                vec!["*.sav".into()]
            ))
        );
        assert_eq!(
            on_new("<winAppData>/Game", System::Windows),
            Some((RootKind::Roaming, "Game".into(), vec![]))
        );
        assert_eq!(on_new("<winAppData>/Game", System::Linux), None);
        assert_eq!(on_new("<base>/Saves", System::Linux), None);
        assert_eq!(on_new("<xdgData>/*/x", System::Linux), None);
    }

    #[test]
    fn verify_addresses_are_the_launchers_own() {
        assert!(verify_ok("steam://validate/<storeGameId>", "steam"));
        assert!(verify_ok("https://example.org/verify", "x"));
        assert!(!verify_ok("steam://validate/<storeGameId>", "epic"));
        assert!(!verify_ok("file:///c/x", "file2"));
        assert!(!verify_ok("steam://<storeGameId>/<storeGameId>", "steam"));
        assert!(!verify_ok("steam://run/<other>", "steam"));
        assert!(!verify_ok("steam://", "steam"));
    }

    #[test]
    fn fetching_the_manifest_refuses_until_its_licence_is_cleared() {
        assert_eq!(
            fetch_manifest(LicenceFlags::default()),
            Err(FetchError::LicenceNotCleared)
        );
        assert_eq!(
            fetch_manifest(LicenceFlags {
                ludusavi_data_cleared: true
            }),
            Err(FetchError::NotBuilt)
        );
    }

    #[test]
    fn the_shipped_catalogues_read_whole() {
        let c = Catalogues::shipped();
        assert!(c.saves.report().is_clean(), "{:?}", c.saves.report());
        assert!(
            c.launchers.report().is_clean(),
            "{:?}",
            c.launchers.report()
        );
        assert!(c.places.report().is_clean(), "{:?}", c.places.report());
        assert!(c.saves.rows().len() >= 3);
        assert!(c.launchers.rows().len() >= 3);
    }

    #[test]
    fn installed_games_need_the_person_until_they_confirm() {
        assert_eq!(
            installed_result(None),
            ItemResult::because(Reason::FinishByHand)
        );
        assert_eq!(installed_result(Some(false)), ItemResult::skipped_by_user());
        assert_eq!(
            installed_result(Some(true)).status(),
            pctwin_types::ItemStatus::Reinstalled
        );
    }
}

#[cfg(test)]
mod route_notes {
    use super::*;

    #[test]
    fn programs_in_saves_among_the_persons_files_are_said_to_move_as_files() {
        let mut with_files = vec![Note::CloudSavesMayDiffer, Note::ModProgramsHeld];
        said_for_route(&mut with_files, Route::WithFiles);
        assert_eq!(
            with_files,
            [Note::CloudSavesMayDiffer, Note::ModProgramsAsFiles]
        );
        for route in [Route::WithAppData, Route::Own] {
            let mut held = vec![Note::ModProgramsHeld];
            said_for_route(&mut held, route);
            assert_eq!(held, [Note::ModProgramsHeld], "{route:?}");
        }
    }
}
