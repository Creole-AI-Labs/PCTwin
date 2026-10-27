//! The neutral settings list: a fixed, versioned set of named settings, each with a typed,
//! bounded value (Phase 2.4 plan sections 3, 4 and 6; T24-001, T24-002).
//!
//! Nothing outside this list is ever read or written: there is no "any registry value" or "any
//! preference key" setting, so a request for an unknown key is refused before anything is
//! touched ([`SettingId::from_key`] answers `None`). Every value is checked against its setting's
//! shape ([`check_value`]) before it is planned, and again by the store before it is written.
//!
//! Values travel through the executor as [`SettingValue`]s under the setting's key, so the
//! executor's journal, compare-and-write undo and crash reconciliation cover them unchanged.

use pctwin_recipe::{SettingValue, text_ok};
use serde::{Deserialize, Serialize};

/// The version of this list. A capture made with a newer list is refused, never guessed at.
pub const SCHEMA_VERSION: u32 = 1;

/// The day every mapping in this list was last checked against the systems' own documentation
/// (Phase 2 adoption, change 5). Re-check before each release.
pub const CHECKED_ON: &str = "2026-10-27";

/// The longest path a wallpaper setting may hold.
pub const MAX_PATH: usize = 1024;

/// Why a setting or value was refused.
#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
pub enum SchemaError {
    #[error("PCTwin does not move the setting {0:?}")]
    UnknownSetting(String),
    #[error("the value for {0} has the wrong type")]
    WrongType(&'static str),
    #[error("the value for {0} is outside what the setting allows")]
    OutOfRange(&'static str),
    #[error("the value for {0} holds hidden or control characters")]
    HiddenText(&'static str),
}

/// The systems PCTwin reads and applies settings on. Linux desktops differ in where they keep
/// settings, so each supported desktop is its own system.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum System {
    Windows,
    Mac,
    /// GNOME (Ubuntu, Fedora Workstation): `gsettings`.
    Gnome,
    /// KDE Plasma: its own config tools (reading only for now).
    Kde,
    /// Any other Linux desktop: settings are guided.
    OtherLinux,
}

/// Every setting PCTwin moves. The list is closed: adding one is a code change with its own
/// tests and a new [`SCHEMA_VERSION`] if a value's meaning changes.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(try_from = "String", into = "&'static str")]
pub enum SettingId {
    /// Light, dark or automatic appearance (for apps; on Windows also see [`Self::TaskbarTheme`]).
    Theme,
    /// Windows only: the taskbar and Start menu's own light or dark.
    TaskbarTheme,
    /// High contrast. Accessibility: never switched on or off without the person (guided).
    HighContrast,
    /// Text size, as a percentage (100 to 225).
    TextSize,
    /// Fewer animations.
    ReduceMotion,
    /// The desktop picture: a file that arrived on the new laptop, by its full path there.
    Wallpaper,
    /// How the desktop picture fits the screen.
    WallpaperStyle,
    /// Pointer speed: the system slider's position, in thousandths (0 to 1000).
    PointerSpeed,
    /// Which mouse button is the main one. Can strand the person: has a safe way back.
    PrimaryButton,
    /// Scrolling direction follows the fingers ("natural").
    NaturalScroll,
    /// Keyboard layouts, in order (neutral names such as `us`, `fr`, `ar`).
    KeyboardLayouts,
    /// Display scaling, as a percentage. Guided: displays differ.
    DisplayScaling,
    /// The default web browser. The system asks the person itself: handed over, never set.
    DefaultBrowser,
    /// Which app opens a kind of file. Handed over, never set.
    FileHandlers,
    /// The time zone (an IANA name such as `Africa/Lagos`). Needs the administrator helper.
    TimeZone,
    /// Apps that start on sign-in. A separate choice, never moved as a hidden convenience.
    StartupApps,
    /// The default printer. The device must exist on the new laptop.
    DefaultPrinter,
}

/// How a setting is changed on the new laptop.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum Route {
    /// Written through the system's own settings store, compared and read back.
    Native,
    /// The system keeps this choice for the person to make: PCTwin opens the system's own page
    /// and reads the result back (default browser, file handlers).
    Handoff,
    /// The person changes it themselves; PCTwin shows the old value and opens the page.
    Guided,
    /// Needs the administrator helper's fixed action (time zone).
    Privileged,
    /// A separate opt-in the person ticks on its own (startup apps).
    OptIn,
}

/// Whether a change could leave the person unable to use the laptop.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum Risk {
    Safe,
    /// Applied only with a confirm-or-go-back window ([`crate::confirm`]).
    MayStrand,
}

/// The shape of a setting's value.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Shape {
    /// One of a fixed list of words.
    Choice(&'static [&'static str]),
    Bool,
    Int {
        min: i64,
        max: i64,
    },
    /// A full path to a file on the new laptop.
    Path,
    /// A comma-separated list of plain names, at most `max` of them.
    List {
        max: usize,
    },
    /// One plain name (an app, a time zone, a printer).
    Name,
}

pub const THEMES: &[&str] = &["light", "dark", "auto"];
pub const BAR_THEMES: &[&str] = &["light", "dark"];
pub const WALLPAPER_STYLES: &[&str] = &["fill", "fit", "stretch", "tile", "center", "span"];
pub const BUTTONS: &[&str] = &["left", "right"];

impl SettingId {
    /// Every setting, in the order screens list them.
    pub const ALL: [SettingId; 17] = [
        SettingId::Theme,
        SettingId::TaskbarTheme,
        SettingId::HighContrast,
        SettingId::TextSize,
        SettingId::ReduceMotion,
        SettingId::Wallpaper,
        SettingId::WallpaperStyle,
        SettingId::PointerSpeed,
        SettingId::PrimaryButton,
        SettingId::NaturalScroll,
        SettingId::KeyboardLayouts,
        SettingId::DisplayScaling,
        SettingId::DefaultBrowser,
        SettingId::FileHandlers,
        SettingId::TimeZone,
        SettingId::StartupApps,
        SettingId::DefaultPrinter,
    ];

    /// The setting's key, as recipes, the journal and the executor carry it.
    pub fn key(self) -> &'static str {
        match self {
            SettingId::Theme => "appearance.theme",
            SettingId::TaskbarTheme => "appearance.taskbar-theme",
            SettingId::HighContrast => "accessibility.high-contrast",
            SettingId::TextSize => "accessibility.text-size",
            SettingId::ReduceMotion => "accessibility.reduce-motion",
            SettingId::Wallpaper => "desktop.wallpaper",
            SettingId::WallpaperStyle => "desktop.wallpaper-style",
            SettingId::PointerSpeed => "pointer.speed",
            SettingId::PrimaryButton => "pointer.primary-button",
            SettingId::NaturalScroll => "pointer.natural-scroll",
            SettingId::KeyboardLayouts => "keyboard.layouts",
            SettingId::DisplayScaling => "display.scaling",
            SettingId::DefaultBrowser => "apps.default-browser",
            SettingId::FileHandlers => "apps.file-handlers",
            SettingId::TimeZone => "time.zone",
            SettingId::StartupApps => "apps.startup",
            SettingId::DefaultPrinter => "devices.default-printer",
        }
    }

    /// The setting with this key; `None` for anything not on the list (T24-001).
    pub fn from_key(key: &str) -> Option<Self> {
        Self::ALL.into_iter().find(|s| s.key() == key)
    }

    /// The shape its value must have.
    pub fn shape(self) -> Shape {
        match self {
            SettingId::Theme => Shape::Choice(THEMES),
            SettingId::TaskbarTheme => Shape::Choice(BAR_THEMES),
            SettingId::HighContrast | SettingId::ReduceMotion | SettingId::NaturalScroll => {
                Shape::Bool
            }
            SettingId::TextSize => Shape::Int { min: 100, max: 225 },
            SettingId::DisplayScaling => Shape::Int { min: 100, max: 500 },
            SettingId::PointerSpeed => Shape::Int { min: 0, max: 1000 },
            SettingId::Wallpaper => Shape::Path,
            SettingId::WallpaperStyle => Shape::Choice(WALLPAPER_STYLES),
            SettingId::PrimaryButton => Shape::Choice(BUTTONS),
            SettingId::KeyboardLayouts => Shape::List { max: 16 },
            SettingId::FileHandlers => Shape::List { max: 32 },
            SettingId::StartupApps => Shape::List { max: 64 },
            SettingId::DefaultBrowser | SettingId::TimeZone | SettingId::DefaultPrinter => {
                Shape::Name
            }
        }
    }

    /// How it is changed, whatever the system.
    pub fn route(self) -> Route {
        match self {
            SettingId::DefaultBrowser | SettingId::FileHandlers => Route::Handoff,
            SettingId::HighContrast | SettingId::DisplayScaling | SettingId::DefaultPrinter => {
                Route::Guided
            }
            SettingId::TimeZone => Route::Privileged,
            SettingId::StartupApps => Route::OptIn,
            _ => Route::Native,
        }
    }

    /// Whether changing it could strand the person.
    pub fn risk(self) -> Risk {
        match self {
            SettingId::PrimaryButton => Risk::MayStrand,
            _ => Risk::Safe,
        }
    }

    /// An accessibility setting: the new laptop's own stronger choice is never lowered without
    /// the person (plan section 4: "preserve destination accessibility needs").
    pub fn accessibility(self) -> bool {
        matches!(
            self,
            SettingId::HighContrast | SettingId::TextSize | SettingId::ReduceMotion
        )
    }
}

impl From<SettingId> for &'static str {
    fn from(s: SettingId) -> Self {
        s.key()
    }
}

impl TryFrom<String> for SettingId {
    type Error = SchemaError;
    fn try_from(s: String) -> Result<Self, SchemaError> {
        SettingId::from_key(&s).ok_or(SchemaError::UnknownSetting(s))
    }
}

/// Where a setting can be applied natively on a system, or why not.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Support {
    /// Through the system's settings store, compared and read back.
    Native,
    /// The person does it on the system's own page.
    Guided,
    /// The system has no such setting.
    NotApplicable,
}

/// Whether `system` can take `id` natively. Settings whose [`Route`] is not native are never
/// [`Support::Native`] anywhere.
pub fn support(system: System, id: SettingId) -> Support {
    use SettingId as S;
    use Support::{Guided, Native, NotApplicable};
    if id.route() != Route::Native {
        return match (system, id) {
            (System::Mac | System::Gnome | System::Kde | System::OtherLinux, S::TaskbarTheme) => {
                NotApplicable
            }
            _ => Guided,
        };
    }
    match system {
        System::Windows => match id {
            S::Theme
            | S::TaskbarTheme
            | S::TextSize
            | S::Wallpaper
            | S::WallpaperStyle
            | S::PointerSpeed
            | S::PrimaryButton => Native,
            // Animation effects and the touchpad's direction have no documented per-user value
            // PCTwin could write and read back; keyboard layouts are added in Settings, so no
            // layout the person still needs is ever taken away.
            _ => Guided,
        },
        System::Mac => match id {
            S::Theme | S::PointerSpeed | S::NaturalScroll => Native,
            S::TaskbarTheme | S::TextSize => NotApplicable,
            // The desktop picture needs a system call PCTwin does not make; reduce motion is
            // protected by the system; the main button and layouts are set in System Settings.
            _ => Guided,
        },
        System::Gnome => match id {
            S::Theme
            | S::TextSize
            | S::ReduceMotion
            | S::Wallpaper
            | S::WallpaperStyle
            | S::PointerSpeed
            | S::PrimaryButton
            | S::NaturalScroll
            | S::KeyboardLayouts => Native,
            S::TaskbarTheme => NotApplicable,
            _ => Guided,
        },
        System::Kde | System::OtherLinux => match id {
            S::TaskbarTheme => NotApplicable,
            _ => Guided,
        },
    }
}

/// Checks a value against its setting's shape (T24-002): the wrong type, a value out of range,
/// a word not on the list, a relative path or hidden characters are refused.
pub fn check_value(id: SettingId, value: &SettingValue) -> Result<(), SchemaError> {
    let key = id.key();
    match (id.shape(), value) {
        (Shape::Choice(words), SettingValue::Text(t)) => {
            if words.contains(&t.as_str()) {
                Ok(())
            } else {
                Err(SchemaError::OutOfRange(key))
            }
        }
        (Shape::Bool, SettingValue::Bool(_)) => Ok(()),
        (Shape::Int { min, max }, SettingValue::Int(n)) => {
            if (min..=max).contains(n) {
                Ok(())
            } else {
                Err(SchemaError::OutOfRange(key))
            }
        }
        (Shape::Path, SettingValue::Text(t)) => {
            if !text_ok(t) {
                Err(SchemaError::HiddenText(key))
            } else if t.len() > MAX_PATH || !full_path(t) {
                Err(SchemaError::OutOfRange(key))
            } else {
                Ok(())
            }
        }
        (Shape::List { max }, SettingValue::Text(t)) => {
            if !text_ok(t) {
                return Err(SchemaError::HiddenText(key));
            }
            if t.is_empty() {
                return Ok(());
            }
            let items: Vec<&str> = t.split(',').collect();
            if items.len() > max || !items.iter().all(|i| plain_name(i)) {
                return Err(SchemaError::OutOfRange(key));
            }
            Ok(())
        }
        (Shape::Name, SettingValue::Text(t)) => {
            if !text_ok(t) {
                Err(SchemaError::HiddenText(key))
            } else if plain_name(t) || (id == SettingId::DefaultPrinter && printer_name(t)) {
                Ok(())
            } else {
                Err(SchemaError::OutOfRange(key))
            }
        }
        _ => Err(SchemaError::WrongType(key)),
    }
}

/// A plain name: letters (any script), digits and `. _ - + / : =`, 1 to 128 characters, no spaces.
/// Covers app IDs (`org.mozilla.firefox`), layouts (`us`, `windows:00000409`), time zones
/// (`America/Argentina/Buenos_Aires`) and handler pairs (`pdf=firefox`).
pub fn plain_name(s: &str) -> bool {
    let n = s.chars().count();
    (1..=128).contains(&n)
        && !s.starts_with(['-', '.', '/'])
        && !s.contains("..")
        && s.chars().all(|c| {
            c.is_ascii_alphanumeric()
                || matches!(c, '.' | '_' | '-' | '+' | '/' | ':' | '=')
                // Other scripts spell with combining marks and joiners; hidden marks are
                // refused separately (`text_ok`).
                || (!c.is_ascii() && !c.is_whitespace() && !c.is_control())
        })
}

/// A printer's name as the system shows it: may hold spaces, never a path separator.
fn printer_name(s: &str) -> bool {
    let n = s.chars().count();
    (1..=256).contains(&n) && !s.contains(['\\', '/', ',']) && s.trim() == s
}

/// A full path on any of the systems: `C:\...` or `/...`, never with a `..` part, never a
/// network share or a device path.
pub fn full_path(p: &str) -> bool {
    let windows = {
        let b = p.as_bytes();
        b.len() > 3 && b[0].is_ascii_alphabetic() && b[1] == b':' && b[2] == b'\\'
    };
    let unix = p.starts_with('/') && !p.starts_with("//");
    (windows || unix)
        && !p.contains('\0')
        && p.split(['/', '\\']).all(|part| part != ".." && part != ".")
}
