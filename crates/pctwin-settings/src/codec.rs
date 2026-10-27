//! Fixed tables and conversions between each system's own values and the neutral ones.
//!
//! Every table here is data checked on [`crate::schema::CHECKED_ON`]; nothing is guessed. A native
//! value that is not on a table is kept as the system's own name (`windows:...`) so the same
//! system can still take it back exactly, and another system gets a guided step instead.

use crate::schema::System;

/// Pointer speed: Windows' slider has 20 steps (1 to 20, `MouseSensitivity`).
pub fn windows_speed_to_neutral(step: i64) -> Option<i64> {
    if !(1..=20).contains(&step) {
        return None;
    }
    Some(((step - 1) * 1000 + 9) / 19)
}

/// The Windows step nearest to a neutral speed (0 to 1000).
pub fn neutral_to_windows_speed(p: i64) -> Option<i64> {
    if !(0..=1000).contains(&p) {
        return None;
    }
    Some((p * 19 + 500) / 1000 + 1)
}

/// Pointer speed on a Mac: `com.apple.mouse.scaling`, 0.0 to 3.0, kept to three decimals.
pub fn mac_speed_to_neutral(scaling: f64) -> Option<i64> {
    if !scaling.is_finite() || !(0.0..=3.0).contains(&scaling) {
        return None;
    }
    Some((scaling / 3.0 * 1000.0).round() as i64)
}

/// The Mac value for a neutral speed, as `defaults` writes it.
pub fn neutral_to_mac_speed(p: i64) -> Option<String> {
    if !(0..=1000).contains(&p) {
        return None;
    }
    Some(format!("{:.3}", p as f64 * 3.0 / 1000.0))
}

/// Pointer speed on GNOME: `org.gnome.desktop.peripherals.mouse speed`, -1.0 to 1.0.
pub fn gnome_speed_to_neutral(speed: f64) -> Option<i64> {
    if !speed.is_finite() || !(-1.0..=1.0).contains(&speed) {
        return None;
    }
    Some(((speed + 1.0) * 500.0).round() as i64)
}

pub fn neutral_to_gnome_speed(p: i64) -> Option<String> {
    if !(0..=1000).contains(&p) {
        return None;
    }
    Some(format!("{:.3}", p as f64 / 500.0 - 1.0))
}

/// The neutral speed nearest `p` that `system` can hold exactly: what its own slider would read
/// back after `p` is written. `None` when the system has no pointer speed PCTwin writes.
pub fn quantize_speed(system: System, p: i64) -> Option<i64> {
    match system {
        System::Windows => neutral_to_windows_speed(p).and_then(windows_speed_to_neutral),
        System::Mac => neutral_to_mac_speed(p)
            .and_then(|s| s.parse::<f64>().ok())
            .and_then(mac_speed_to_neutral),
        System::Gnome => neutral_to_gnome_speed(p)
            .and_then(|s| s.parse::<f64>().ok())
            .and_then(gnome_speed_to_neutral),
        System::Kde | System::OtherLinux => None,
    }
}

/// One keyboard layout under each system's own name.
pub struct Layout {
    pub neutral: &'static str,
    /// Windows keyboard layout ID (`HKCU\Keyboard Layout\Preload`).
    pub windows: &'static str,
    /// XKB layout, with `+variant` (GNOME `input-sources`).
    pub xkb: &'static str,
    /// macOS input source ID (`com.apple.HIToolbox AppleEnabledInputSources`).
    pub mac: &'static str,
}

pub const LAYOUTS: &[Layout] = &[
    Layout {
        neutral: "us",
        windows: "00000409",
        xkb: "us",
        mac: "com.apple.keylayout.US",
    },
    Layout {
        neutral: "us-intl",
        windows: "00020409",
        xkb: "us+intl",
        mac: "com.apple.keylayout.USInternational-PC",
    },
    Layout {
        neutral: "gb",
        windows: "00000809",
        xkb: "gb",
        mac: "com.apple.keylayout.British",
    },
    Layout {
        neutral: "fr",
        windows: "0000040c",
        xkb: "fr",
        mac: "com.apple.keylayout.French",
    },
    Layout {
        neutral: "de",
        windows: "00000407",
        xkb: "de",
        mac: "com.apple.keylayout.German",
    },
    Layout {
        neutral: "es",
        windows: "0000040a",
        xkb: "es",
        mac: "com.apple.keylayout.Spanish-ISO",
    },
    Layout {
        neutral: "it",
        windows: "00000410",
        xkb: "it",
        mac: "com.apple.keylayout.Italian",
    },
    Layout {
        neutral: "pt",
        windows: "00000816",
        xkb: "pt",
        mac: "com.apple.keylayout.Portuguese",
    },
    Layout {
        neutral: "br",
        windows: "00000416",
        xkb: "br",
        mac: "com.apple.keylayout.Brazilian",
    },
    Layout {
        neutral: "ar",
        windows: "00000401",
        xkb: "ara",
        mac: "com.apple.keylayout.Arabic",
    },
    Layout {
        neutral: "he",
        windows: "0000040d",
        xkb: "il",
        mac: "com.apple.keylayout.Hebrew",
    },
    Layout {
        neutral: "ru",
        windows: "00000419",
        xkb: "ru",
        mac: "com.apple.keylayout.Russian",
    },
];

/// The neutral name of a layout read on `system` (its own ID, as the system spells it).
/// Not on the table: kept as `<system>:<id>` so the same system can take it back.
pub fn layout_to_neutral(system: System, native: &str) -> Option<String> {
    let lower = native.to_ascii_lowercase();
    let found = LAYOUTS.iter().find(|l| match system {
        System::Windows => l.windows == lower,
        System::Mac => l.mac == native,
        System::Gnome | System::Kde | System::OtherLinux => l.xkb == native,
    });
    if let Some(l) = found {
        return Some(l.neutral.to_string());
    }
    let ok = !native.is_empty()
        && native.len() <= 100
        && native
            .chars()
            .all(|c| c.is_ascii_alphanumeric() || matches!(c, '.' | '_' | '-' | '+'));
    ok.then(|| format!("{}:{native}", prefix(system)))
}

/// The layout's ID on `system`: from the table, or the system's own ID if it was read there.
/// `None` when this system has no equivalent PCTwin knows (a guided step instead).
pub fn layout_from_neutral(system: System, neutral: &str) -> Option<String> {
    if let Some((p, id)) = neutral.split_once(':') {
        return (p == prefix(system)).then(|| id.to_string());
    }
    LAYOUTS.iter().find(|l| l.neutral == neutral).map(|l| {
        match system {
            System::Windows => l.windows,
            System::Mac => l.mac,
            System::Gnome | System::Kde | System::OtherLinux => l.xkb,
        }
        .to_string()
    })
}

fn prefix(system: System) -> &'static str {
    match system {
        System::Windows => "windows",
        System::Mac => "mac",
        System::Gnome | System::Kde | System::OtherLinux => "xkb",
    }
}

/// One browser under each system's own name.
pub struct Browser {
    pub neutral: &'static str,
    /// The start of the Windows ProgId its `https` choice names (Firefox and Vivaldi add a
    /// per-install suffix).
    pub windows_progid: &'static str,
    /// Its name in `HKCU\Software\RegisteredApplications`, for the Default apps page link.
    pub windows_registered: &'static str,
    /// The Mac bundle ID.
    pub mac: &'static str,
    /// Its `.desktop` file names (distribution, Snap, Flatpak).
    pub linux: &'static [&'static str],
}

pub const BROWSERS: &[Browser] = &[
    Browser {
        neutral: "firefox",
        windows_progid: "FirefoxURL",
        windows_registered: "Firefox",
        mac: "org.mozilla.firefox",
        linux: &[
            "firefox.desktop",
            "firefox_firefox.desktop",
            "org.mozilla.firefox.desktop",
        ],
    },
    Browser {
        neutral: "chrome",
        windows_progid: "ChromeHTML",
        windows_registered: "Google Chrome",
        mac: "com.google.chrome",
        linux: &["google-chrome.desktop", "com.google.Chrome.desktop"],
    },
    Browser {
        neutral: "edge",
        windows_progid: "MSEdgeHTM",
        windows_registered: "Microsoft Edge",
        mac: "com.microsoft.edgemac",
        linux: &["microsoft-edge.desktop", "com.microsoft.Edge.desktop"],
    },
    Browser {
        neutral: "brave",
        windows_progid: "BraveHTML",
        windows_registered: "Brave",
        mac: "com.brave.browser",
        linux: &["brave-browser.desktop", "com.brave.Browser.desktop"],
    },
    Browser {
        neutral: "opera",
        windows_progid: "OperaStable",
        windows_registered: "Opera Stable",
        mac: "com.operasoftware.opera",
        linux: &["opera.desktop", "com.opera.Opera.desktop"],
    },
    Browser {
        neutral: "vivaldi",
        windows_progid: "VivaldiHTM",
        windows_registered: "Vivaldi",
        mac: "com.vivaldi.vivaldi",
        linux: &["vivaldi-stable.desktop", "com.vivaldi.Vivaldi.desktop"],
    },
    Browser {
        neutral: "safari",
        windows_progid: "",
        windows_registered: "",
        mac: "com.apple.safari",
        linux: &[],
    },
];

/// The neutral browser a system's own choice names; `None` for one not on the table.
pub fn browser_to_neutral(system: System, native: &str) -> Option<&'static str> {
    BROWSERS
        .iter()
        .find(|b| match system {
            System::Windows => {
                !b.windows_progid.is_empty()
                    && (native == b.windows_progid
                        || native
                            .strip_prefix(b.windows_progid)
                            .is_some_and(|rest| rest.starts_with(['-', '.'])))
            }
            System::Mac => b.mac.eq_ignore_ascii_case(native),
            System::Gnome | System::Kde | System::OtherLinux => b.linux.contains(&native),
        })
        .map(|b| b.neutral)
}

/// Whether `system` has the neutral browser at all (Safari has no Windows or Linux version).
pub fn browser_exists_on(system: System, neutral: &str) -> bool {
    BROWSERS.iter().any(|b| {
        b.neutral == neutral
            && match system {
                System::Windows => !b.windows_progid.is_empty(),
                System::Mac => !b.mac.is_empty(),
                System::Gnome | System::Kde | System::OtherLinux => !b.linux.is_empty(),
            }
    })
}

/// Windows time zone names and their IANA names (Unicode CLDR `windowsZones.xml`, the "001"
/// territory rows). A zone not here is reported as not readable, never guessed.
pub const WINDOWS_ZONES: &[(&str, &str)] = &[
    ("UTC", "Etc/UTC"),
    ("GMT Standard Time", "Europe/London"),
    ("Greenwich Standard Time", "Atlantic/Reykjavik"),
    ("W. Europe Standard Time", "Europe/Berlin"),
    ("Romance Standard Time", "Europe/Paris"),
    ("Central Europe Standard Time", "Europe/Budapest"),
    ("Russian Standard Time", "Europe/Moscow"),
    ("Turkey Standard Time", "Europe/Istanbul"),
    ("W. Central Africa Standard Time", "Africa/Lagos"),
    ("South Africa Standard Time", "Africa/Johannesburg"),
    ("E. Africa Standard Time", "Africa/Nairobi"),
    ("Egypt Standard Time", "Africa/Cairo"),
    ("Morocco Standard Time", "Africa/Casablanca"),
    ("Eastern Standard Time", "America/New_York"),
    ("Central Standard Time", "America/Chicago"),
    ("Mountain Standard Time", "America/Denver"),
    ("Pacific Standard Time", "America/Los_Angeles"),
    ("SA Pacific Standard Time", "America/Bogota"),
    ("E. South America Standard Time", "America/Sao_Paulo"),
    ("Argentina Standard Time", "America/Buenos_Aires"),
    ("India Standard Time", "Asia/Kolkata"),
    ("Pakistan Standard Time", "Asia/Karachi"),
    ("Bangladesh Standard Time", "Asia/Dhaka"),
    ("SE Asia Standard Time", "Asia/Bangkok"),
    ("Singapore Standard Time", "Asia/Singapore"),
    ("China Standard Time", "Asia/Shanghai"),
    ("Korea Standard Time", "Asia/Seoul"),
    ("Tokyo Standard Time", "Asia/Tokyo"),
    ("Arabian Standard Time", "Asia/Dubai"),
    ("Arab Standard Time", "Asia/Riyadh"),
    ("Israel Standard Time", "Asia/Jerusalem"),
    ("Iran Standard Time", "Asia/Tehran"),
    ("AUS Eastern Standard Time", "Australia/Sydney"),
];

pub fn windows_zone_to_iana(windows: &str) -> Option<&'static str> {
    WINDOWS_ZONES
        .iter()
        .find(|(w, _)| *w == windows)
        .map(|(_, i)| *i)
}

pub fn iana_to_windows_zone(iana: &str) -> Option<&'static str> {
    WINDOWS_ZONES
        .iter()
        .find(|(_, i)| *i == iana)
        .map(|(w, _)| *w)
}

/// Windows `WallpaperStyle` (and `TileWallpaper`) values for each neutral style.
pub fn windows_style(neutral: &str) -> Option<(&'static str, &'static str)> {
    Some(match neutral {
        "fill" => ("10", "0"),
        "fit" => ("6", "0"),
        "stretch" => ("2", "0"),
        "tile" => ("0", "1"),
        "center" => ("0", "0"),
        "span" => ("22", "0"),
        _ => return None,
    })
}

pub fn windows_style_to_neutral(style: &str, tile: &str) -> Option<&'static str> {
    Some(match (style, tile) {
        ("10", _) => "fill",
        ("6", _) => "fit",
        ("2", _) => "stretch",
        ("0", "1") => "tile",
        ("0", _) => "center",
        ("22", _) => "span",
        _ => return None,
    })
}

/// GNOME `picture-options` for each neutral style.
pub fn gnome_style(neutral: &str) -> Option<&'static str> {
    Some(match neutral {
        "fill" => "zoom",
        "fit" => "scaled",
        "stretch" => "stretched",
        "tile" => "wallpaper",
        "center" => "centered",
        "span" => "spanned",
        _ => return None,
    })
}

pub fn gnome_style_to_neutral(native: &str) -> Option<&'static str> {
    ["fill", "fit", "stretch", "tile", "center", "span"]
        .into_iter()
        .find(|n| gnome_style(n) == Some(native))
}

/// A Unix path as a `file://` address, every byte outside the unreserved set and `/` encoded.
pub fn file_uri(path: &str) -> Option<String> {
    if !path.starts_with('/') {
        return None;
    }
    let mut out = String::from("file://");
    for b in path.bytes() {
        if b.is_ascii_alphanumeric() || matches!(b, b'/' | b'-' | b'.' | b'_' | b'~') {
            out.push(char::from(b));
        } else {
            out.push_str(&format!("%{b:02X}"));
        }
    }
    Some(out)
}

/// The path a local `file://` address names; `None` for any other address or broken encoding.
pub fn path_of_file_uri(uri: &str) -> Option<String> {
    let rest = uri.strip_prefix("file://")?;
    if !rest.starts_with('/') {
        return None;
    }
    let bytes = rest.as_bytes();
    let mut out = Vec::with_capacity(bytes.len());
    let mut i = 0;
    while i < bytes.len() {
        if bytes[i] == b'%' {
            let hex = std::str::from_utf8(bytes.get(i + 1..i + 3)?).ok()?;
            out.push(u8::from_str_radix(hex, 16).ok()?);
            i += 3;
        } else {
            out.push(bytes[i]);
            i += 1;
        }
    }
    let s = String::from_utf8(out).ok()?;
    (!s.contains('\0')).then_some(s)
}
