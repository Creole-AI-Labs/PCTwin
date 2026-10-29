//! Folders the system itself names as installed programs' places, so the sweep counts them as
//! programs instead of moving them as the person's files (found in the two-copy test, 29 October
//! 2026: a game from the Xbox app, in `C:\XboxGames`, was offered as "Other folder" and copied
//! into the new laptop's shared folder). Nothing here is a list of program names: each place
//! comes from the laptop's own records.
//!
//! - **Windows.** The Xbox app's game library: a `.GamingRoot` file at the top of a drive names
//!   the folders it installs games into. Every program in Windows' Uninstall lists (for
//!   everyone, both 64-bit and 32-bit, and the signed-in person's) that says where it is
//!   installed (`InstallLocation`).
//! - **Elsewhere:** none yet; programs there live in the system's own places, which the rules
//!   file already names.
//!
//! A place too broad to be one program's (a drive's top here; anything holding a person's home,
//! which the sweep checks) is never taken.

use std::path::{Component, Path, PathBuf};

use crate::volumes::Volume;

/// The installed programs' places on this laptop.
pub fn program_places(volumes: &[Volume]) -> Vec<PathBuf> {
    let mut places = Vec::new();
    for v in volumes {
        if let Ok(bytes) = std::fs::read(v.mount.join(".GamingRoot")) {
            places.extend(
                gaming_root_names(&bytes)
                    .into_iter()
                    .map(|n| v.mount.join(n)),
            );
        }
    }
    places.extend(install_locations());
    places.retain(|p| narrow_enough(p));
    places.sort();
    places.dedup();
    places
}

/// The folder names a `.GamingRoot` file holds: `RGBX`, a four-byte version, then names in
/// UTF-16 (little-endian), each ended by a zero. Only plain names (one folder, no separators,
/// not `.` or `..`) are kept; anything else in the file is not trusted.
pub fn gaming_root_names(bytes: &[u8]) -> Vec<String> {
    let Some(body) = bytes.strip_prefix(b"RGBX").and_then(|b| b.get(4..)) else {
        return Vec::new();
    };
    let units: Vec<u16> = body
        .as_chunks::<2>()
        .0
        .iter()
        .map(|c| u16::from_le_bytes(*c))
        .collect();
    units
        .split(|u| *u == 0)
        .filter_map(|name| String::from_utf16(name).ok())
        .filter(|name| plain_name(name))
        .collect()
}

fn plain_name(name: &str) -> bool {
    !name.is_empty()
        && name != "."
        && name != ".."
        && !name.contains(['/', '\\', ':'])
        && !name.chars().any(char::is_control)
}

/// At least one folder below a drive's top: a drive itself is never one program's place.
fn narrow_enough(path: &Path) -> bool {
    path.is_absolute() && path.components().any(|c| matches!(c, Component::Normal(_)))
}

#[cfg(windows)]
fn install_locations() -> Vec<PathBuf> {
    use winreg::RegKey;
    use winreg::enums::{
        HKEY_CURRENT_USER, HKEY_LOCAL_MACHINE, KEY_READ, KEY_WOW64_32KEY, KEY_WOW64_64KEY,
    };
    const UNINSTALL: &str = r"SOFTWARE\Microsoft\Windows\CurrentVersion\Uninstall";
    let lists = [
        (HKEY_LOCAL_MACHINE, KEY_READ | KEY_WOW64_64KEY),
        (HKEY_LOCAL_MACHINE, KEY_READ | KEY_WOW64_32KEY),
        (HKEY_CURRENT_USER, KEY_READ),
    ];
    let mut out = Vec::new();
    for (hive, flags) in lists {
        let Ok(list) = RegKey::predef(hive).open_subkey_with_flags(UNINSTALL, flags) else {
            continue;
        };
        for name in list.enum_keys().flatten() {
            let Ok(entry) = list.open_subkey_with_flags(&name, flags) else {
                continue;
            };
            if let Ok(at) = entry.get_value::<String, _>("InstallLocation") {
                // Written by each installer, sometimes in quotes or with a separator at the end.
                let at = at.trim().trim_matches('"').trim_end_matches(['\\', '/']);
                if !at.is_empty() {
                    out.push(PathBuf::from(at));
                }
            }
        }
    }
    out
}

#[cfg(not(windows))]
fn install_locations() -> Vec<PathBuf> {
    Vec::new()
}

#[cfg(test)]
mod tests {
    use super::*;

    fn file(names: &[&str]) -> Vec<u8> {
        let mut b = b"RGBX\x01\x00\x00\x00".to_vec();
        for n in names {
            for u in n.encode_utf16() {
                b.extend_from_slice(&u.to_le_bytes());
            }
            b.extend_from_slice(&[0, 0]);
        }
        b
    }

    /// The Xbox app's file names its game folders; only plain folder names are taken.
    #[test]
    fn a_gaming_root_names_its_folders_and_nothing_else() {
        assert_eq!(gaming_root_names(&file(&["XboxGames"])), ["XboxGames"]);
        assert_eq!(
            gaming_root_names(&file(&["Games", "..", "a\\b", "C:", ""])),
            ["Games"]
        );
        assert!(gaming_root_names(b"RGBX").is_empty());
        assert!(gaming_root_names(b"nope\x01\x00\x00\x00X\x00").is_empty());
    }

    /// A drive's top is never one program's place.
    #[test]
    fn a_whole_drive_is_too_broad() {
        let (root, game) = if cfg!(windows) {
            (r"C:\", r"C:\XboxGames")
        } else {
            ("/", "/opt/game")
        };
        assert!(!narrow_enough(Path::new(root)));
        assert!(narrow_enough(Path::new(game)));
        assert!(!narrow_enough(Path::new("relative/place")));
    }
}
