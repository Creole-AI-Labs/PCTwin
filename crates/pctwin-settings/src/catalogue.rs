//! The settings catalogue: which settings PCTwin moves is data, not code (the user's decision,
//! 27 October 2026).
//!
//! A catalogue is a versioned file of rows. Each row names one semantic setting (its ID, value
//! shape and bounds, route, risk, ownership domain) and, for each system, the generic mechanism
//! and place that holds it (a registry value, a Mac preference, a GNOME `gsettings` key, a KDE
//! config key) with a codec between the system's own values and the neutral one. App settings
//! are entries describing an app's settings files, their format and field rules. Code here is
//! only the mechanisms; adding a setting is adding a row.
//!
//! Discovery comes first (27 October 2026, "different people have different things"): every
//! setting in the person's own stores is found and, between the same kind of system, moved by
//! the generic rules whether or not a row exists ([`crate::found`], [`crate::generic`]). Rows
//! only enrich: translations between systems, better routes, exceptions and friendly names.
//!
//! A catalogue is checked whole when it is read ([`Catalogue::from_json`]), the same trust path
//! as recipes: the format number first (a newer one is refused), then every row. It is refused if
//! a writable place is in a store that holds security, start-up commands, protected choices or
//! organisation policy (T24-001, T24-003, T24-013, T24-023): a catalogue cannot make PCTwin write
//! there, whatever it says. The machine-wide registry is read-only. Pages opened for the person
//! use only the systems' own settings addresses. Signing and delivery through the signed recipe
//! channel (TUF) come with that channel; until then the starter catalogue is built in.

use std::collections::{BTreeMap, BTreeSet};

use pctwin_recipe::{SettingValue, SettingsRoot, key_ok, text_ok};
use pctwin_types::format::{Format, FormatError};
use serde::{Deserialize, Serialize};

use crate::value::{ValueType, plain_name};

/// The catalogue format this app reads and writes.
pub const FORMAT: u32 = 1;

pub const CATALOGUE_FORMAT: Format = Format {
    name: "settings catalogue",
    current: FORMAT,
    readable: &[],
};

/// The starter catalogue, built in until the signed recipe channel delivers catalogues.
pub const STARTER: &str = include_str!("../catalogue/settings.json");

/// The most rows, table rows and app entries a catalogue may hold.
const MAX_ROWS: usize = 2000;

/// The systems PCTwin reads and applies settings on. Linux desktops keep settings differently,
/// so each supported desktop is its own system.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum System {
    Windows,
    Mac,
    Gnome,
    Kde,
    OtherLinux,
}

impl System {
    pub const ALL: [System; 5] = [
        System::Windows,
        System::Mac,
        System::Gnome,
        System::Kde,
        System::OtherLinux,
    ];

    /// The column name tables use for this system's own names.
    pub fn column(self) -> &'static str {
        match self {
            System::Windows => "windows",
            System::Mac => "mac",
            System::Gnome | System::Kde | System::OtherLinux => "linux",
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
pub enum CatalogueError {
    #[error(transparent)]
    Format(#[from] FormatError),
    #[error("the settings catalogue is damaged: {0}")]
    Damaged(String),
    #[error("the settings catalogue row {row} is not allowed: {why}")]
    Row { row: String, why: String },
}

/// A setting's ID, such as `appearance.theme`: a dotted name, as recipes carry setting keys.
#[derive(Debug, Clone, PartialEq, Eq, Hash, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(try_from = "String", into = "String")]
pub struct SettingId(String);

impl SettingId {
    pub fn new(id: &str) -> Result<Self, CatalogueError> {
        let ok = key_ok(id)
            && id.contains('.')
            && !id.ends_with('.')
            && !id.contains("..")
            && id.bytes().all(|b| !b.is_ascii_uppercase());
        if ok {
            Ok(Self(id.to_string()))
        } else {
            Err(CatalogueError::Row {
                row: id.to_string(),
                why: "a setting ID is lowercase dotted words".into(),
            })
        }
    }

    pub fn as_str(&self) -> &str {
        &self.0
    }
}

impl TryFrom<String> for SettingId {
    type Error = CatalogueError;
    fn try_from(s: String) -> Result<Self, CatalogueError> {
        Self::new(&s)
    }
}

impl From<SettingId> for String {
    fn from(s: SettingId) -> Self {
        s.0
    }
}

/// How a setting is changed on the new laptop.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum Route {
    /// Through the system's settings store, compared and read back.
    #[default]
    Native,
    /// The system keeps this choice for the person: its own page is opened and the result read
    /// back (default browser).
    Handoff,
    /// The person changes it themselves on the page shown.
    Guided,
    /// Needs the administrator helper's fixed action (time zone).
    Privileged,
    /// Its own choice, unticked, shown separately (apps that start on sign-in).
    OptIn,
}

/// Whether a change could leave the person unable to use the laptop.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum Risk {
    #[default]
    Safe,
    /// Applied only with a confirm-or-go-back window ([`crate::confirm`]).
    MayStrand,
}

/// How a list setting is applied.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum Merge {
    /// The new value replaces the old.
    #[default]
    Replace,
    /// Items are only added after the new laptop's own, never taken away (keyboard layouts:
    /// nothing the person types with now disappears).
    AddOnly,
}

/// Which registry root.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum Hive {
    /// The person's own (`HKEY_CURRENT_USER`).
    #[default]
    User,
    /// The machine's (`HKEY_LOCAL_MACHINE`): only ever read.
    Machine,
}

/// The type a place's field holds natively.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum Kind {
    /// Registry `REG_DWORD`.
    Dword,
    /// Registry `REG_SZ` text.
    Sz,
    /// Registry `REG_SZ` holding a whole number (`MouseSensitivity`).
    SzInt,
    /// Every value name in the registry key (never the values): a list.
    ValueNames,
    /// The values named `1`, `2`, ... in order (`Keyboard Layout\Preload`): a list.
    NumberedValues,
    /// Mac preference or `gsettings` types.
    Bool,
    Int,
    Float,
    String,
    /// `gsettings` `a(ss)` input sources: the `xkb` ones, as a list.
    SourceList,
    /// A Mac preference array of dictionaries: field `item` of those whose `where_key` equals
    /// `where_value`, as a list.
    DictList,
}

impl Kind {
    /// Whether the kind gives a list.
    pub fn is_list(self) -> bool {
        matches!(
            self,
            Kind::ValueNames | Kind::NumberedValues | Kind::SourceList | Kind::DictList
        )
    }

    fn numeric(self) -> bool {
        matches!(self, Kind::Dword | Kind::SzInt | Kind::Int | Kind::Float)
    }
}

/// One field of a place.
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Field {
    pub name: String,
    pub kind: Kind,
    /// For [`Kind::DictList`]: the dictionary key whose text is the item.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub item: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub where_key: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub where_value: Option<String>,
}

/// One row of a [`Codec::Table`]: the native fields' values (JSON `null` for "not set",
/// `{"any": true}` for any value) and the neutral value they mean. `write` is what is written
/// for that neutral value when it differs from `native` (which must then have no `any`).
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct CodecRow {
    pub native: Vec<serde_json::Value>,
    pub neutral: serde_json::Value,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub write: Option<Vec<serde_json::Value>>,
}

/// Between a place's native fields and the neutral value.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(tag = "codec", rename_all = "kebab-case", deny_unknown_fields)]
pub enum Codec {
    /// One field, the same value.
    Same,
    /// A table of native field values and their neutral values, first match wins.
    Table { rows: Vec<CodecRow> },
    /// One numeric field, scaled: `native[0]..=native[1]` is `neutral[0]..=neutral[1]`.
    Linear {
        native: [f64; 2],
        neutral: [i64; 2],
        #[serde(default)]
        decimals: u8,
    },
    /// One text field holding a `file://` address of a full path.
    FileUri,
    /// One text field, looked up in a named table: native column to neutral column.
    Lookup {
        table: String,
        column: String,
        /// The native name may carry a per-install suffix after `-` or `.` (Firefox's ProgId).
        #[serde(default)]
        prefix: bool,
    },
    /// One list field, each item looked up; an item not in the table is kept as
    /// `<column>:<id>` so the same system can take it back exactly.
    ListLookup { table: String, column: String },
    /// One text field: the part before the first `sep` (`Name,winspool,Ne01:`). Read only.
    SplitFirst { sep: String },
    /// One numeric field: whether any bit of `mask` is set (`HighContrast` `Flags`). Read only.
    Flag { mask: i64 },
}

/// When a written setting takes effect.
#[derive(Debug, Clone, PartialEq, Eq, Default, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum Refresh {
    /// At once.
    #[default]
    Now,
    /// After the person signs out and in again.
    SignIn,
    /// After the system is told (Windows `WM_SETTINGCHANGE` with this area). Until PCTwin's
    /// fenced system call for it is approved, the same as [`Refresh::SignIn`].
    Broadcast(String),
}

/// A registry value whose presence means the organisation manages the setting.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct RegPolicy {
    #[serde(default)]
    pub hive: Hive,
    pub key: String,
    pub value: String,
}

/// A behaviour that needs its own code (each named, never a command from the catalogue).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum Special {
    /// The app a Mac opens for a URL scheme (LaunchServices), read only.
    MacUrlHandler(String),
    /// `xdg-settings get default-web-browser`, read only.
    XdgDefaultBrowser,
    /// The time zone `/etc/localtime` points at, read only.
    LocaltimeLink,
}

/// Where one system keeps a setting: a generic mechanism and its place.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(tag = "mechanism", rename_all = "kebab-case", deny_unknown_fields)]
pub enum Place {
    Registry {
        #[serde(default)]
        hive: Hive,
        key: String,
        fields: Vec<Field>,
        codec: Codec,
        #[serde(default)]
        read_only: bool,
        #[serde(default)]
        refresh: Refresh,
        #[serde(default)]
        policy: Vec<RegPolicy>,
    },
    MacPref {
        domain: String,
        fields: Vec<Field>,
        codec: Codec,
        #[serde(default)]
        read_only: bool,
        #[serde(default)]
        refresh: Refresh,
    },
    Gsettings {
        schema: String,
        fields: Vec<Field>,
        codec: Codec,
        #[serde(default)]
        read_only: bool,
    },
    KdeConfig {
        file: String,
        group: String,
        fields: Vec<Field>,
        codec: Codec,
        #[serde(default)]
        read_only: bool,
    },
    Special {
        special: Special,
        #[serde(default)]
        codec: Option<Codec>,
    },
}

impl Place {
    pub fn read_only(&self) -> bool {
        match self {
            Place::Registry { read_only, .. }
            | Place::MacPref { read_only, .. }
            | Place::Gsettings { read_only, .. }
            | Place::KdeConfig { read_only, .. } => *read_only,
            Place::Special { .. } => true,
        }
    }

    pub fn fields(&self) -> &[Field] {
        match self {
            Place::Registry { fields, .. }
            | Place::MacPref { fields, .. }
            | Place::Gsettings { fields, .. }
            | Place::KdeConfig { fields, .. } => fields,
            Place::Special { .. } => &[],
        }
    }

    pub fn codec(&self) -> Option<&Codec> {
        match self {
            Place::Registry { codec, .. }
            | Place::MacPref { codec, .. }
            | Place::Gsettings { codec, .. }
            | Place::KdeConfig { codec, .. } => Some(codec),
            Place::Special { codec, .. } => codec.as_ref(),
        }
    }

    pub fn refresh(&self) -> Refresh {
        match self {
            Place::Registry { refresh, .. } | Place::MacPref { refresh, .. } => refresh.clone(),
            _ => Refresh::Now,
        }
    }
}

/// The system's own page for a setting, opened for the person.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Page {
    /// `ms-settings:` or `x-apple.systempreferences:` only.
    pub uri: String,
    /// A query parameter naming the app (Windows' Default apps page:
    /// `registeredAppUser`), its value found in the registry's registered applications by the
    /// table name for the app.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub app_param: Option<AppParam>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct AppParam {
    pub name: String,
    pub table: String,
    pub column: String,
}

/// One semantic setting.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Row {
    pub id: SettingId,
    pub value: ValueType,
    #[serde(default)]
    pub route: Route,
    #[serde(default)]
    pub risk: Risk,
    /// The new laptop's stronger choice is never lowered without the person.
    #[serde(default)]
    pub accessibility: bool,
    #[serde(default)]
    pub merge: Merge,
    /// A table whose column for the new laptop's system must have the value (an app that exists
    /// there); the value is also an app that must be installed first.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub app_table: Option<String>,
    /// The value names a device that must exist on the new laptop.
    #[serde(default)]
    pub device: bool,
    #[serde(default)]
    pub places: BTreeMap<System, Place>,
    #[serde(default)]
    pub pages: BTreeMap<System, Page>,
    #[serde(default)]
    pub not_applicable: Vec<System>,
}

/// A table of names, one column per system (and `neutral`).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Table {
    pub columns: Vec<String>,
    pub rows: Vec<Vec<String>>,
}

impl Table {
    pub fn column(&self, name: &str) -> Option<usize> {
        self.columns.iter().position(|c| c == name)
    }

    /// The `to` column's cell of the first row whose `from` cell is `value` (ASCII letter case
    /// ignored; with `prefix`, the value may carry a suffix after `-` or `.`) and whose `to` cell
    /// is not empty. Empty cells never match.
    pub fn lookup(&self, from: &str, value: &str, to: &str, prefix: bool) -> Option<&str> {
        let (f, t) = (self.column(from)?, self.column(to)?);
        self.rows
            .iter()
            .find(|r| {
                let cell = r[f].as_str();
                !cell.is_empty()
                    && !r[t].is_empty()
                    && (cell.eq_ignore_ascii_case(value)
                        || (prefix
                            && value.len() > cell.len()
                            && value.is_char_boundary(cell.len())
                            && value[..cell.len()].eq_ignore_ascii_case(cell)
                            && value[cell.len()..].starts_with(['-', '.'])))
            })
            .map(|r| r[t].as_str())
    }
}

/// Privacy approvals and switched-off protections: noted, never copied.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum Protected {
    CameraAccess,
    MicrophoneAccess,
    LocationAccess,
    ScreenRecording,
    FilesAccess,
    FirewallOff,
    SmartScreenOff,
    GatekeeperOff,
    DownloadScanOff,
}

impl Protected {
    /// A protection that was off (kept on), rather than an approval (given again).
    pub fn is_protection_off(self) -> bool {
        matches!(
            self,
            Protected::FirewallOff
                | Protected::SmartScreenOff
                | Protected::GatekeeperOff
                | Protected::DownloadScanOff
        )
    }
}

/// How a protected thing is noticed.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "kebab-case", deny_unknown_fields)]
pub enum Probe {
    /// Any app under this consent key (or its `NonPackaged` key) has `Value` = `Allow`. Only
    /// whether any does is kept, never which app.
    RegistryConsent { key: String },
    /// A registry value equals this.
    RegistryEquals {
        #[serde(default)]
        hive: Hive,
        key: String,
        value: String,
        equals: serde_json::Value,
    },
    /// `spctl --status` says assessments are disabled.
    GatekeeperOff,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ProtectedRow {
    pub what: Protected,
    pub system: System,
    pub probe: Probe,
}

/// A store scanned for settings that have no row yet ("found, not yet supported").
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "mechanism", rename_all = "kebab-case", deny_unknown_fields)]
pub enum Store {
    Registry {
        #[serde(default)]
        hive: Hive,
        key: String,
    },
    MacPref {
        domain: String,
    },
    Gsettings {
        schema: String,
    },
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct DiscoverRow {
    pub system: System,
    pub store: Store,
}

/// Which role writes a file of an app's settings (one writer per domain, T24-017).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum Writer {
    /// 2.4: settings.
    Settings,
    /// 2.5: private data, and files that mix preferences with it.
    Data,
    /// 2.1: apps and their extensions or plugins (code).
    Install,
    /// Nobody in PCTwin (sign-ins, machine state, security approvals, caches).
    Nobody,
}

/// A settings file format.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum FileFormat {
    /// JSON with comments and trailing commas (VS Code).
    Jsonc,
    Json,
    Ini,
    Xml,
    /// Not read by 2.4 (a database, a binary): only its writer handles it.
    Opaque,
}

/// What happens to a field of an app's settings file.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum Rule {
    /// Copied (paths in it rewritten when the file says so).
    Copy,
    /// Left out: machine IDs, sockets, sign-ins, caches.
    Drop,
    /// Kept as data for the person to look at, not written: commands and programs to run.
    HoldBack,
    /// A path, rewritten through the move's item map (held back if it has no new place).
    RewritePath,
    /// A start-up hook: written switched off (`to`), the original kept for the person.
    Disable(serde_json::Value),
    /// Keyboard shortcuts across Mac and the others: `ctrl` and `cmd` swapped. An equivalent:
    /// the person confirms.
    SwapModifiers,
}

/// A rule for the fields whose path matches `at` (`/`-separated; `*` matches any one part; a
/// part may hold `*` within it, such as `*.path`; matching ignores letter case).
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct FieldRule {
    pub at: String,
    pub rule: Rule,
}

/// A path placeholder an app writes in its files (JetBrains' `$USER_HOME$`).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum MacroRoot {
    Home,
}

/// One file (or `*` pattern in its last part) of an app's settings.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct AppFile {
    pub root: SettingsRoot,
    pub path: String,
    pub format: FileFormat,
    pub writer: Writer,
    #[serde(default)]
    pub rules: Vec<FieldRule>,
    /// The rule for fields no rule matches.
    #[serde(default = "hold_back")]
    pub default: Rule,
    /// Whether text that is a full path is rewritten through the move's item map.
    #[serde(default)]
    pub rewrite_paths: bool,
    #[serde(default)]
    pub macros: BTreeMap<String, MacroRoot>,
    /// Rules that apply only between a Mac and another system.
    #[serde(default)]
    pub cross_os: Vec<FieldRule>,
}

fn hold_back() -> Rule {
    Rule::HoldBack
}

/// An app's settings: its files and the versions the rules were checked with.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct AppEntry {
    pub app: String,
    /// The oldest app version the rules were checked with (`major.minor`).
    pub min_version: String,
    /// A folder name pattern carrying the version (JetBrains `IntelliJIdea2025.2`): the part
    /// after this prefix is the version.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub versioned_folder: Option<String>,
    pub files: Vec<AppFile>,
}

/// A found setting that must not move for a given app, and why (the catalogue enriching the
/// generic rules). `store` and `name` are patterns (`*` matches anything; case ignored).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Exception {
    pub system: System,
    pub store: String,
    pub name: String,
    pub reason: String,
}

/// A friendly name for the settings in a store.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Label {
    pub system: System,
    pub store: String,
    pub label: String,
}

/// A settings catalogue.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Catalogue {
    format: u32,
    /// Grows with every published change.
    pub revision: u64,
    /// The day its rows were last checked against the systems' documentation (adoption change 5).
    pub checked_on: String,
    #[serde(default)]
    pub tables: BTreeMap<String, Table>,
    pub settings: Vec<Row>,
    #[serde(default)]
    pub protected: Vec<ProtectedRow>,
    #[serde(default)]
    pub discover: Vec<DiscoverRow>,
    #[serde(default)]
    pub apps: Vec<AppEntry>,
    #[serde(default)]
    pub exceptions: Vec<Exception>,
    #[serde(default)]
    pub labels: Vec<Label>,
}

/// Registry stores no catalogue may make PCTwin write (prefixes, letter case ignored):
/// organisation policy, start-up commands, file and protocol choices the system protects, app
/// permissions, the command processor's start-up hook, sign-in and program-start hooks, the
/// environment (a changed `PATH` runs other programs).
pub const DENIED_REGISTRY: &[&str] = &[
    "software\\policies",
    "software\\microsoft\\windows\\currentversion\\policies",
    "software\\microsoft\\windows\\currentversion\\run",
    "software\\microsoft\\windows\\currentversion\\runonce",
    "software\\microsoft\\windows\\currentversion\\runservices",
    "software\\microsoft\\windows\\currentversion\\runservicesonce",
    "software\\microsoft\\windows\\currentversion\\explorer\\startupapproved",
    "software\\microsoft\\windows nt\\currentversion\\windows",
    "software\\microsoft\\windows\\currentversion\\explorer\\fileexts",
    "software\\microsoft\\windows\\currentversion\\capabilityaccessmanager",
    "software\\microsoft\\windows\\shell\\associations",
    "software\\microsoft\\windows nt\\currentversion\\winlogon",
    "software\\microsoft\\windows nt\\currentversion\\image file execution options",
    "software\\microsoft\\command processor",
    "software\\classes",
    "environment",
];

/// Mac preference domains no catalogue may make PCTwin write: sign-in hooks, protected
/// handler choices, the firewall and privacy databases, security settings.
pub const DENIED_MAC: &[&str] = &[
    "com.apple.loginwindow",
    "com.apple.launchservices",
    "com.apple.alf",
    "com.apple.tcc",
    "com.apple.security",
    "com.apple.systempolicy",
];

/// `gsettings` schemas no catalogue may make PCTwin write: lock-down and privacy, proxies,
/// screen locking, and custom shortcuts (which run commands).
pub const DENIED_GSETTINGS: &[&str] = &[
    "org.gnome.desktop.lockdown",
    "org.gnome.desktop.privacy",
    "org.gnome.desktop.screensaver",
    "org.gnome.system.proxy",
    "org.gnome.settings-daemon.plugins.media-keys",
    "org.gnome.desktop.app-folders",
];

/// Addresses a page may open: the systems' own settings pages only.
pub const PAGE_SCHEMES: &[&str] = &["ms-settings:", "x-apple.systempreferences:"];

impl Catalogue {
    /// The starter catalogue built into this app.
    pub fn starter() -> Result<Self, CatalogueError> {
        Self::from_json(STARTER.as_bytes())
    }

    /// Reads a catalogue: the format first (a newer one is refused safely), then every row.
    pub fn from_json(bytes: &[u8]) -> Result<Self, CatalogueError> {
        CATALOGUE_FORMAT.check_json(bytes)?;
        let c: Catalogue =
            serde_json::from_slice(bytes).map_err(|e| CatalogueError::Damaged(e.to_string()))?;
        c.validate()?;
        Ok(c)
    }

    pub fn row(&self, id: &str) -> Option<&Row> {
        self.settings.iter().find(|r| r.id.as_str() == id)
    }

    pub fn app(&self, app: &str) -> Option<&AppEntry> {
        self.apps.iter().find(|a| a.app == app)
    }

    /// Checks everything the catalogue holds.
    pub fn validate(&self) -> Result<(), CatalogueError> {
        let damaged = |w: &str| Err(CatalogueError::Damaged(w.into()));
        if self.format != FORMAT {
            return damaged("wrong format");
        }
        if self.revision == 0 {
            return damaged("a catalogue revision starts at 1");
        }
        if !date_ok(&self.checked_on) {
            return damaged("the checked-on date is not a date");
        }
        if self.settings.len() > MAX_ROWS || self.apps.len() > MAX_ROWS {
            return damaged("too many rows");
        }
        for (name, t) in &self.tables {
            if !plain_name(name)
                || t.columns.is_empty()
                || t.rows.len() > MAX_ROWS
                || t.columns
                    .iter()
                    .enumerate()
                    .any(|(i, c)| !plain_name(c) || t.columns[..i].contains(c))
                || t.rows.iter().any(|r| {
                    r.len() != t.columns.len() || r.iter().any(|c| !text_ok(c) || c.len() > 256)
                })
            {
                return damaged(&format!("the table {name:?} is not well formed"));
            }
        }
        let mut seen = BTreeSet::new();
        for row in &self.settings {
            if !seen.insert(row.id.as_str()) {
                return Err(bad(row, "listed twice"));
            }
            self.check_row(row)?;
        }
        for p in &self.protected {
            match &p.probe {
                Probe::RegistryConsent { key } => reg_key(key).ok_or_else(|| {
                    CatalogueError::Damaged("a protected probe's key is not allowed".into())
                })?,
                Probe::RegistryEquals { key, value, .. } => {
                    reg_key(key).filter(|_| field_name(value)).ok_or_else(|| {
                        CatalogueError::Damaged("a protected probe is not allowed".into())
                    })?;
                }
                Probe::GatekeeperOff => {}
            }
        }
        for d in &self.discover {
            let ok = match &d.store {
                Store::Registry { key, .. } => reg_key(key).is_some(),
                Store::MacPref { domain } => mac_domain(domain),
                Store::Gsettings { schema } => gname(schema),
            };
            if !ok {
                return damaged("a store to look in is not allowed");
            }
        }
        let text = |t: &str, max: usize| !t.is_empty() && t.len() <= max && text_ok(t);
        if self
            .exceptions
            .iter()
            .any(|e| !text(&e.store, 255) || !text(&e.name, 255) || !text(&e.reason, 300))
            || self
                .labels
                .iter()
                .any(|l| !text(&l.store, 255) || !text(&l.label, 120))
        {
            return damaged("an exception or label is not plain text");
        }
        for a in &self.apps {
            crate::apps::check_entry(a).map_err(|why| CatalogueError::Row {
                row: a.app.clone(),
                why,
            })?;
        }
        Ok(())
    }

    fn check_row(&self, row: &Row) -> Result<(), CatalogueError> {
        if !row.value.sound() {
            return Err(bad(row, "its value shape is not sound"));
        }
        if let Some(t) = &row.app_table {
            let table = self
                .tables
                .get(t)
                .ok_or_else(|| bad(row, "names a missing table"))?;
            if table.column("neutral").is_none() {
                return Err(bad(row, "its app table has no neutral column"));
            }
        }
        for (system, place) in &row.places {
            if row.not_applicable.contains(system) {
                return Err(bad(row, "is both placed and not applicable"));
            }
            if !place.read_only() && row.route != Route::Native {
                return Err(bad(row, "only a native setting may be written"));
            }
            self.check_place(row, place)
                .map_err(|why| bad(row, &format!("{system:?}: {why}")))?;
        }
        for page in row.pages.values() {
            if !PAGE_SCHEMES.iter().any(|s| page.uri.starts_with(s))
                || page.uri.len() > 256
                || !page.uri.bytes().all(|b| b.is_ascii_graphic())
            {
                return Err(bad(row, "a page is not one of the systems' settings pages"));
            }
            if let Some(p) = &page.app_param {
                let ok = p.name.bytes().all(|b| b.is_ascii_alphanumeric())
                    && self
                        .tables
                        .get(&p.table)
                        .is_some_and(|t| t.column(&p.column).is_some());
                if !ok {
                    return Err(bad(row, "a page's app parameter is not well formed"));
                }
            }
        }
        Ok(())
    }

    fn check_place(&self, row: &Row, place: &Place) -> Result<(), String> {
        let fields = place.fields();
        for f in fields {
            if !field_name(&f.name) {
                return Err(format!("the field name {:?} is not allowed", f.name));
            }
            let dict = f.kind == Kind::DictList;
            if dict != (f.item.is_some() && f.where_key.is_some() && f.where_value.is_some()) {
                return Err("a dictionary list needs item, where-key and where-value".into());
            }
        }
        let kinds_ok = |allowed: &[Kind]| fields.iter().all(|f| allowed.contains(&f.kind));
        match place {
            Place::Registry {
                hive,
                key,
                read_only,
                policy,
                ..
            } => {
                let k = reg_key(key).map(|_| key.to_ascii_lowercase());
                let Some(k) = k else {
                    return Err("the registry key is not allowed".into());
                };
                if !kinds_ok(&[
                    Kind::Dword,
                    Kind::Sz,
                    Kind::SzInt,
                    Kind::ValueNames,
                    Kind::NumberedValues,
                ]) {
                    return Err("a field kind does not belong to the registry".into());
                }
                if !read_only {
                    if *hive == Hive::Machine {
                        return Err("the machine's registry is only ever read".into());
                    }
                    if DENIED_REGISTRY
                        .iter()
                        .any(|d| k == *d || k.starts_with(&format!("{d}\\")))
                    {
                        return Err("this registry store may never be written".into());
                    }
                    if fields.iter().any(|f| f.kind.is_list()) {
                        return Err("a list of registry values is only ever read".into());
                    }
                }
                if policy
                    .iter()
                    .any(|p| reg_key(&p.key).is_none() || !field_name(&p.value))
                {
                    return Err("a policy probe is not allowed".into());
                }
            }
            Place::MacPref {
                domain, read_only, ..
            } => {
                if !mac_domain(domain) {
                    return Err("the preferences domain is not allowed".into());
                }
                if !kinds_ok(&[
                    Kind::Bool,
                    Kind::Int,
                    Kind::Float,
                    Kind::String,
                    Kind::DictList,
                ]) {
                    return Err("a field kind does not belong to Mac preferences".into());
                }
                let d = domain.to_ascii_lowercase();
                if !read_only
                    && (DENIED_MAC
                        .iter()
                        .any(|x| d == *x || d.starts_with(&format!("{x}.")))
                        || fields.iter().any(|f| f.kind.is_list()))
                {
                    return Err("this preferences domain may never be written".into());
                }
            }
            Place::Gsettings {
                schema, read_only, ..
            } => {
                if !gname(schema) || !fields.iter().all(|f| gname(&f.name)) {
                    return Err("the schema or key is not allowed".into());
                }
                if !kinds_ok(&[
                    Kind::Bool,
                    Kind::Int,
                    Kind::Float,
                    Kind::String,
                    Kind::SourceList,
                ]) {
                    return Err("a field kind does not belong to gsettings".into());
                }
                if !read_only
                    && DENIED_GSETTINGS
                        .iter()
                        .any(|x| schema == x || schema.starts_with(&format!("{x}.")))
                {
                    return Err("this schema may never be written".into());
                }
            }
            Place::KdeConfig {
                file,
                group,
                read_only,
                ..
            } => {
                let file_ok = !file.is_empty()
                    && file.len() <= 64
                    && file
                        .bytes()
                        .all(|b| b.is_ascii_alphanumeric() || matches!(b, b'.' | b'_' | b'-'))
                    && !file.starts_with('.');
                if !file_ok || !field_name(group) || group.contains(['[', ']']) {
                    return Err("the KDE file or group is not allowed".into());
                }
                if !kinds_ok(&[Kind::String, Kind::Bool, Kind::Int]) || !read_only {
                    return Err("KDE settings are only read for now".into());
                }
            }
            Place::Special { .. } => {}
        }
        self.check_codec(row, place)
    }

    fn check_codec(&self, row: &Row, place: &Place) -> Result<(), String> {
        let fields = place.fields();
        let Some(codec) = place.codec() else {
            return match place {
                Place::Special { .. } => Ok(()),
                _ => Err("a place needs a codec".into()),
            };
        };
        let one = |numeric: Option<bool>| -> Result<(), String> {
            if let Place::Special { .. } = place {
                return Ok(());
            }
            if fields.len() != 1 {
                return Err("this codec takes exactly one field".into());
            }
            match numeric {
                Some(true) if !fields[0].kind.numeric() => Err("a scale needs a number".into()),
                Some(false) if fields[0].kind.numeric() || fields[0].kind.is_list() => {
                    Err("this codec needs text".into())
                }
                _ => Ok(()),
            }
        };
        match codec {
            Codec::Same => {
                one(None)?;
                if fields.first().is_some_and(|f| f.kind.is_list())
                    && !matches!(row.value, ValueType::List { .. })
                {
                    return Err("a list field needs a list setting".into());
                }
                Ok(())
            }
            Codec::Table { rows } => {
                if rows.is_empty() || fields.is_empty() || fields.iter().any(|f| f.kind.is_list()) {
                    return Err("a table needs rows and plain fields".into());
                }
                for r in rows {
                    if r.native.len() != fields.len()
                        || r.write.as_ref().is_some_and(|w| w.len() != fields.len())
                    {
                        return Err("a table row has the wrong number of values".into());
                    }
                    let any = r.native.iter().any(is_any);
                    if any && r.write.is_none() && !place.read_only() {
                        return Err("a row matching any value needs what to write".into());
                    }
                    if r.write.as_ref().is_some_and(|w| w.iter().any(is_any)) {
                        return Err("what to write cannot be any value".into());
                    }
                    if !r
                        .native
                        .iter()
                        .chain(r.write.iter().flatten())
                        .all(literal_ok)
                    {
                        return Err("a table value is not plain".into());
                    }
                    let v = json_to_setting(&r.neutral).ok_or("a neutral value is not plain")?;
                    row.value
                        .check(row.id.as_str(), &v)
                        .map_err(|e| e.to_string())?;
                }
                Ok(())
            }
            Codec::Linear {
                native, neutral, ..
            } => {
                one(Some(true))?;
                let ok = native.iter().all(|n| n.is_finite())
                    && native[0] != native[1]
                    && neutral[0] != neutral[1]
                    && matches!(row.value, ValueType::Int { .. });
                ok.then_some(())
                    .ok_or_else(|| "the scale is not sound".into())
            }
            Codec::FileUri => {
                one(Some(false))?;
                (row.value == ValueType::FilePath)
                    .then_some(())
                    .ok_or_else(|| "a file address needs a file setting".into())
            }
            Codec::Lookup { table, column, .. } => {
                one(Some(false))?;
                self.table_has(table, column)
            }
            Codec::ListLookup { table, column } => {
                if fields.len() != 1 || !fields[0].kind.is_list() {
                    return Err("a list lookup takes one list field".into());
                }
                if !matches!(row.value, ValueType::List { .. }) {
                    return Err("a list lookup needs a list setting".into());
                }
                self.table_has(table, column)
            }
            Codec::Flag { mask } => {
                one(Some(true))?;
                if *mask <= 0 || !place.read_only() || row.value != ValueType::Bool {
                    return Err("a flag is read only, needs a mask and a true/false setting".into());
                }
                Ok(())
            }
            Codec::SplitFirst { sep } => {
                one(Some(false))?;
                if sep.is_empty() || !place.read_only() {
                    return Err("a split is read only and needs a separator".into());
                }
                Ok(())
            }
        }
    }

    fn table_has(&self, table: &str, column: &str) -> Result<(), String> {
        let t = self.tables.get(table).ok_or("names a missing table")?;
        if t.column(column).is_none() || t.column("neutral").is_none() {
            return Err("names a missing table column".into());
        }
        Ok(())
    }
}

fn bad(row: &Row, why: &str) -> CatalogueError {
    CatalogueError::Row {
        row: row.id.as_str().to_string(),
        why: why.to_string(),
    }
}

fn is_any(v: &serde_json::Value) -> bool {
    v.as_object()
        .is_some_and(|o| o.len() == 1 && o.get("any") == Some(&serde_json::Value::Bool(true)))
}

fn literal_ok(v: &serde_json::Value) -> bool {
    match v {
        serde_json::Value::Null | serde_json::Value::Bool(_) => true,
        serde_json::Value::Number(n) => n.as_f64().is_some_and(f64::is_finite),
        serde_json::Value::String(s) => s.len() <= 256 && text_ok(s),
        serde_json::Value::Object(_) => is_any(v),
        serde_json::Value::Array(_) => false,
    }
}

/// A neutral JSON value as a setting value: text, a whole number or true/false.
pub fn json_to_setting(v: &serde_json::Value) -> Option<SettingValue> {
    match v {
        serde_json::Value::String(s) => Some(SettingValue::Text(s.clone())),
        serde_json::Value::Bool(b) => Some(SettingValue::Bool(*b)),
        serde_json::Value::Number(n) => n.as_i64().map(SettingValue::Int),
        _ => None,
    }
}

/// A registry key path relative to its hive: plain parts joined by `\`, no `..`.
fn reg_key(k: &str) -> Option<()> {
    let ok = !k.is_empty()
        && k.len() <= 255
        && text_ok(k)
        && !k.starts_with('\\')
        && !k.ends_with('\\')
        && k.split('\\')
            .all(|p| !p.is_empty() && p != "." && p != ".." && !p.contains('/'));
    ok.then_some(())
}

/// A field (value or key) name: printable, at most 255 bytes.
fn field_name(n: &str) -> bool {
    !n.is_empty() && n.len() <= 255 && text_ok(n) && n.trim() == n
}

/// A Mac preferences domain: `NSGlobalDomain` or a reverse-DNS name (with one `/` for a
/// LaunchServices-style subdomain).
fn mac_domain(d: &str) -> bool {
    !d.is_empty()
        && d.len() <= 128
        && !d.starts_with(['.', '-', '/'])
        && !d.contains("..")
        && d.matches('/').count() <= 1
        && d.bytes()
            .all(|b| b.is_ascii_alphanumeric() || matches!(b, b'.' | b'-' | b'_' | b'/'))
}

/// A `gsettings` schema or key name.
fn gname(s: &str) -> bool {
    !s.is_empty()
        && s.len() <= 128
        && !s.starts_with(['.', '-'])
        && s.bytes()
            .all(|b| b.is_ascii_alphanumeric() || matches!(b, b'.' | b'-'))
}

fn date_ok(d: &str) -> bool {
    let b = d.as_bytes();
    b.len() == 10
        && b[4] == b'-'
        && b[7] == b'-'
        && b.iter()
            .enumerate()
            .all(|(i, c)| i == 4 || i == 7 || c.is_ascii_digit())
}
