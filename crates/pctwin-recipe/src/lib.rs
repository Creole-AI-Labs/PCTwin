//! PCTwin setup recipes (engineering notes, building block 1: "recipes are the common unit").
//!
//! A [`Recipe`] says how one app is rebuilt on a new laptop: which catalog package installs it,
//! where its settings live, which settings are applied and how the result is checked. Live moves,
//! saved setup profiles and restores all use the same recipes.
//!
//! - Every step is one of a closed set of typed [`Step`]s. There is no step that runs a command,
//!   opens a link or downloads from an address: an app is installed only by its ID in a known
//!   [`Catalog`], whose own tool fetches and verifies it.
//! - Paths are [`RelPath`]s inside a named [`SettingsRoot`] (never absolute, never `..`), so a
//!   recipe cannot reach outside the app's own settings.
//! - Every step that changes the laptop has its own check ([`Step::check`]): the executor runs it
//!   right after the step, so a step is never counted done because some other check passed.
//! - Text a recipe holds (setting values, checklist entries) has no control characters and no
//!   hidden direction or zero-width marks. Adapters still pass values through typed system calls,
//!   never by writing them into a command or a file by hand.
//! - Every recipe is checked ([`Recipe::validate`]) when it is made and again when it is read, and
//!   every saved recipe carries [`FORMAT`] under the shared format rule.

use pctwin_types::format::{Format, FormatError};
use serde::{Deserialize, Serialize};

/// The recipe format this app writes and reads.
pub const FORMAT: u32 = 1;

/// Recipes in the shared format rule (no older formats yet).
pub const RECIPE_FORMAT: Format = Format {
    name: "setup recipe",
    current: FORMAT,
    readable: &[],
};

/// The longest ID, package ID, setting key or path part a recipe may hold.
const MAX_TEXT: usize = 200;
/// The most steps one recipe may hold.
const MAX_STEPS: usize = 200;

/// Why a recipe was refused.
#[derive(Debug, PartialEq, Eq, thiserror::Error)]
pub enum RecipeError {
    #[error(transparent)]
    Format(#[from] FormatError),
    #[error("this recipe is damaged: {0}")]
    Damaged(String),
    #[error("the recipe ID {0:?} is not allowed (lowercase letters, digits, `-` and `.` only)")]
    BadId(String),
    #[error("the package ID {0:?} is not allowed")]
    BadPackage(String),
    #[error("the setting key {0:?} is not allowed")]
    BadKey(String),
    #[error("the path {0:?} is not allowed (it must stay inside its settings folder)")]
    BadPath(String),
    #[error("a recipe version starts at 1")]
    BadVersion,
    #[error("a recipe must name at least one system it works on")]
    NoPlatform,
    #[error("a recipe must have between 1 and {MAX_STEPS} steps")]
    BadStepCount,
    #[error("a recipe must change something or give the person a checklist entry")]
    NothingToDo,
    #[error("a text value holds control characters or hidden marks")]
    BadText,
    #[error("the checklist key {0:?} is not allowed (lowercase letters, digits and `-` only)")]
    BadChecklistKey(String),
}

/// A recipe's stable ID, such as `org.mozilla.firefox`.
#[derive(Debug, Clone, PartialEq, Eq, Hash, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(try_from = "String", into = "String")]
pub struct RecipeId(String);

impl RecipeId {
    pub fn new(id: &str) -> Result<Self, RecipeError> {
        let ok = !id.is_empty()
            && id.len() <= MAX_TEXT
            && id
                .bytes()
                .all(|b| b.is_ascii_lowercase() || b.is_ascii_digit() || b == b'-' || b == b'.')
            && !id.starts_with(['.', '-'])
            && !id.contains("..");
        if ok {
            Ok(Self(id.to_string()))
        } else {
            Err(RecipeError::BadId(id.to_string()))
        }
    }

    pub fn as_str(&self) -> &str {
        &self.0
    }
}

impl TryFrom<String> for RecipeId {
    type Error = RecipeError;
    fn try_from(s: String) -> Result<Self, RecipeError> {
        Self::new(&s)
    }
}

impl From<RecipeId> for String {
    fn from(id: RecipeId) -> Self {
        id.0
    }
}

/// A recipe and the exact version of it a plan was approved with.
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct RecipeRef {
    pub id: RecipeId,
    pub version: u32,
}

/// The package catalogs PCTwin installs from. Each one fetches and checks its own packages.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum Catalog {
    Winget,
    MicrosoftStore,
    Homebrew,
    MacAppStore,
    Flathub,
    Snap,
    Apt,
    Dnf,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum Os {
    Windows,
    Macos,
    Linux,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum Arch {
    X86_64,
    Aarch64,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Platform {
    pub os: Os,
    pub arch: Arch,
}

/// Where an app keeps its settings, named rather than spelled out, so the OS adapter decides the
/// real folder for the person it is working for.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum SettingsRoot {
    /// Windows `%APPDATA%`, macOS `~/Library/Application Support`, Linux `~/.config`.
    Config,
    /// Windows `%LOCALAPPDATA%`, Linux `~/.local/share` (macOS keeps these under [`Self::Config`]).
    LocalData,
    /// macOS `~/Library/Preferences`.
    Preferences,
    /// The person's home folder (only for dotfiles an app reads from there).
    Home,
}

/// A path inside a [`SettingsRoot`]: one or more plain names, never absolute, never `.` or `..`.
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(try_from = "String", into = "String")]
pub struct RelPath(String);

impl RelPath {
    pub fn new(path: &str) -> Result<Self, RecipeError> {
        let bad = || RecipeError::BadPath(path.to_string());
        if path.is_empty() || path.len() > MAX_TEXT * 4 {
            return Err(bad());
        }
        for part in path.split('/') {
            let ok = !part.is_empty()
                && part.len() <= MAX_TEXT
                && !part.ends_with(['.', ' '])
                && !part.contains(['\\', ':', '\0'])
                && text_ok(part)
                && !is_device_name(part);
            if !ok {
                return Err(bad());
            }
        }
        Ok(Self(path.to_string()))
    }

    pub fn parts(&self) -> impl Iterator<Item = &str> {
        self.0.split('/')
    }
}

impl TryFrom<String> for RelPath {
    type Error = RecipeError;
    fn try_from(s: String) -> Result<Self, RecipeError> {
        Self::new(&s)
    }
}

impl From<RelPath> for String {
    fn from(p: RelPath) -> Self {
        p.0
    }
}

/// Whether text is free of characters that are invisible or change how text reads: controls,
/// line and paragraph separators, direction marks and overrides, and every default-ignorable code
/// point (Unicode's own list: zero-width spaces, soft hyphens, invisible tag characters and the
/// rest). Two kinds are kept because real words need them: the zero-width joiner and non-joiner
/// (Hindi, Persian and other scripts spell with them) and the variation selectors (emoji).
pub fn text_ok(text: &str) -> bool {
    !text.chars().any(hidden)
}

/// Characters [`text_ok`] refuses. The default-ignorable ranges are Unicode's
/// `Default_Ignorable_Code_Point` property (DerivedCoreProperties.txt); the other format
/// characters are Unicode's category Cf outside that property.
fn hidden(c: char) -> bool {
    if matches!(c, '\u{200C}' | '\u{200D}' | '\u{FE00}'..='\u{FE0F}' | '\u{E0100}'..='\u{E01EF}') {
        return false;
    }
    c.is_control()
        || matches!(
            c,
            // Default_Ignorable_Code_Point
            '\u{00AD}'
                | '\u{034F}'
                | '\u{061C}'
                | '\u{115F}'..='\u{1160}'
                | '\u{17B4}'..='\u{17B5}'
                | '\u{180B}'..='\u{180F}'
                | '\u{200B}'..='\u{200F}'
                | '\u{202A}'..='\u{202E}'
                | '\u{2060}'..='\u{206F}'
                | '\u{3164}'
                | '\u{FFA0}'
                | '\u{FFF0}'..='\u{FFF8}'
                | '\u{1BCA0}'..='\u{1BCA3}'
                | '\u{1D173}'..='\u{1D17A}'
                | '\u{E0000}'..='\u{E0FFF}'
                // Line and paragraph separators (Zl, Zp)
                | '\u{2028}'..='\u{2029}'
                // Other format characters (Cf)
                | '\u{0600}'..='\u{0605}'
                | '\u{06DD}'
                | '\u{070F}'
                | '\u{0890}'..='\u{0891}'
                | '\u{08E2}'
                | '\u{FEFF}'
                | '\u{FFF9}'..='\u{FFFB}'
                | '\u{110BD}'
                | '\u{110CD}'
                | '\u{13430}'..='\u{1343F}'
        )
}

/// A checklist entry: the key of a message every language has, so the person sees translated
/// words made ahead of time, never text a recipe wrote. Lowercase letters, digits and `-`.
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(try_from = "String", into = "String")]
pub struct ChecklistKey(String);

impl ChecklistKey {
    pub fn new(key: &str) -> Result<Self, RecipeError> {
        let ok = !key.is_empty()
            && key.len() <= 80
            && key
                .bytes()
                .all(|b| b.is_ascii_lowercase() || b.is_ascii_digit() || b == b'-')
            && !key.starts_with('-');
        if ok {
            Ok(Self(key.to_string()))
        } else {
            Err(RecipeError::BadChecklistKey(key.to_string()))
        }
    }

    pub fn as_str(&self) -> &str {
        &self.0
    }
}

impl TryFrom<String> for ChecklistKey {
    type Error = RecipeError;
    fn try_from(s: String) -> Result<Self, RecipeError> {
        Self::new(&s)
    }
}

impl From<ChecklistKey> for String {
    fn from(k: ChecklistKey) -> Self {
        k.0
    }
}

/// Windows device names (`CON`, `nul.txt`, `COM1`, `LPT²`), which open a device, not a file.
fn is_device_name(part: &str) -> bool {
    let stem = part
        .split('.')
        .next()
        .unwrap_or("")
        .trim_end()
        .to_uppercase();
    if matches!(
        stem.as_str(),
        "CON" | "PRN" | "AUX" | "NUL" | "CONIN$" | "CONOUT$"
    ) {
        return true;
    }
    let mut chars = stem.chars();
    let head: String = chars.by_ref().take(3).collect();
    let rest: Vec<char> = chars.collect();
    (head == "COM" || head == "LPT")
        && rest.len() == 1
        && (rest[0].is_ascii_digit() || matches!(rest[0], '\u{B9}' | '\u{B2}' | '\u{B3}'))
}

/// A place inside an app's settings.
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct SettingsPlace {
    pub root: SettingsRoot,
    pub path: RelPath,
}

/// A setting's value: only plain data, never code.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum SettingValue {
    Bool(bool),
    Int(i64),
    Text(String),
}

/// How a recipe checks that the app works on the new laptop.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case", deny_unknown_fields)]
pub enum Check {
    /// The catalog reports the package installed.
    PackageInstalled { catalog: Catalog, package: String },
    /// A settings file exists where it was put.
    SettingsPresent { place: SettingsPlace },
    /// The settings at `to` hold exactly what `from` held on the old laptop.
    SettingsMatch {
        from: SettingsPlace,
        to: SettingsPlace,
    },
    /// A named setting holds this value.
    SettingIs { key: String, value: SettingValue },
}

/// One step of a recipe. The set is closed: nothing here can run a command or name an address.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case", deny_unknown_fields)]
pub enum Step {
    /// Install a package by its ID in a catalog.
    Install { catalog: Catalog, package: String },
    /// Copy settings from the old laptop's place to the new laptop's place.
    CopySettings {
        from: SettingsPlace,
        to: SettingsPlace,
    },
    /// Set one named setting.
    ApplySetting { key: String, value: SettingValue },
    /// Check the result.
    Verify { check: Check },
    /// This part cannot be done automatically: the person gets a checklist entry instead.
    Checklist { what: ChecklistKey },
}

/// How one app is rebuilt on a new laptop.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Recipe {
    format: u32,
    pub id: RecipeId,
    pub version: u32,
    pub platforms: Vec<Platform>,
    pub steps: Vec<Step>,
}

impl Step {
    /// The check of what this step changed, run right after it; `None` for steps that change
    /// nothing.
    pub fn check(&self) -> Option<Check> {
        match self {
            Step::Install { catalog, package } => Some(Check::PackageInstalled {
                catalog: *catalog,
                package: package.clone(),
            }),
            Step::CopySettings { from, to } => Some(Check::SettingsMatch {
                from: from.clone(),
                to: to.clone(),
            }),
            Step::ApplySetting { key, value } => Some(Check::SettingIs {
                key: key.clone(),
                value: value.clone(),
            }),
            Step::Verify { .. } | Step::Checklist { .. } => None,
        }
    }
}

/// Package IDs as the catalogs spell them (`Mozilla.Firefox`, `firefox`, `org.mozilla.firefox`,
/// `9NBLGGH4NNS1`): letters, digits and `.`, `-`, `_`, `+`, `@`; no spaces, slashes or `:`, so
/// they cannot carry an address or a command.
fn package_ok(p: &str) -> bool {
    !p.is_empty()
        && p.len() <= MAX_TEXT
        && p.bytes()
            .all(|b| b.is_ascii_alphanumeric() || matches!(b, b'.' | b'-' | b'_' | b'+' | b'@'))
        && !p.starts_with(['.', '-', '@'])
        && !looks_like_a_file(p)
}

/// Some package tools read a name ending like a package file as a file to install from
/// (`dnf install foo.rpm`, `brew install foo.rb`), so such names are refused.
fn looks_like_a_file(p: &str) -> bool {
    let lower = p.to_ascii_lowercase();
    [
        ".rpm",
        ".deb",
        ".snap",
        ".rb",
        ".msi",
        ".msix",
        ".appx",
        ".exe",
        ".pkg",
        ".dmg",
        ".flatpak",
        ".flatpakref",
        ".appimage",
        ".tar",
        ".gz",
        ".zip",
    ]
    .iter()
    .any(|ext| lower.ends_with(ext))
}

/// Setting keys: a dotted name such as `browser.startup.page`.
fn key_ok(k: &str) -> bool {
    !k.is_empty()
        && !k.starts_with(['-', '.'])
        && k.len() <= MAX_TEXT
        && k.bytes()
            .all(|b| b.is_ascii_alphanumeric() || matches!(b, b'.' | b'-' | b'_'))
}

fn check_package(package: &str) -> Result<(), RecipeError> {
    if package_ok(package) {
        Ok(())
    } else {
        Err(RecipeError::BadPackage(package.to_string()))
    }
}

fn check_setting(key: &str, value: &SettingValue) -> Result<(), RecipeError> {
    if !key_ok(key) {
        return Err(RecipeError::BadKey(key.to_string()));
    }
    if let SettingValue::Text(t) = value {
        if t.len() > MAX_TEXT * 20 {
            return Err(RecipeError::Damaged("a setting value is too long".into()));
        }
        if !text_ok(t) {
            return Err(RecipeError::BadText);
        }
    }
    Ok(())
}

impl Recipe {
    /// A new recipe, checked.
    pub fn new(
        id: RecipeId,
        version: u32,
        platforms: Vec<Platform>,
        steps: Vec<Step>,
    ) -> Result<Self, RecipeError> {
        let r = Self {
            format: FORMAT,
            id,
            version,
            platforms,
            steps,
        };
        r.validate()?;
        Ok(r)
    }

    pub fn reference(&self) -> RecipeRef {
        RecipeRef {
            id: self.id.clone(),
            version: self.version,
        }
    }

    /// Whether the recipe works on a platform.
    pub fn supports(&self, platform: Platform) -> bool {
        self.platforms.contains(&platform)
    }

    /// Checks everything a recipe holds.
    pub fn validate(&self) -> Result<(), RecipeError> {
        if self.version == 0 {
            return Err(RecipeError::BadVersion);
        }
        if self.platforms.is_empty() {
            return Err(RecipeError::NoPlatform);
        }
        if self.steps.is_empty() || self.steps.len() > MAX_STEPS {
            return Err(RecipeError::BadStepCount);
        }
        for step in &self.steps {
            match step {
                Step::Install { package, .. } => check_package(package)?,
                Step::ApplySetting { key, value } => check_setting(key, value)?,
                Step::Verify { check } => match check {
                    Check::PackageInstalled { package, .. } => check_package(package)?,
                    Check::SettingIs { key, value } => check_setting(key, value)?,
                    Check::SettingsPresent { .. } | Check::SettingsMatch { .. } => {}
                },
                Step::Checklist { .. } => {}
                Step::CopySettings { .. } => {}
            }
        }
        // A recipe made only of checks would report an app rebuilt that nothing rebuilt.
        if self.steps.iter().all(|s| matches!(s, Step::Verify { .. })) {
            return Err(RecipeError::NothingToDo);
        }
        Ok(())
    }

    pub fn to_json(&self) -> Vec<u8> {
        serde_json::to_vec(self).unwrap_or_default()
    }

    /// Reads a saved recipe: the format first (a newer one is refused safely), then every field.
    pub fn from_json(bytes: &[u8]) -> Result<Self, RecipeError> {
        RECIPE_FORMAT.check_json(bytes)?;
        let r: Self =
            serde_json::from_slice(bytes).map_err(|e| RecipeError::Damaged(e.to_string()))?;
        r.validate()?;
        Ok(r)
    }
}
