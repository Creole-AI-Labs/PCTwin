//! Old laptop, Windows: the saved Wi-Fi networks with their passwords (Task List 2.4, decided 20
//! October 2026). Windows gives a saved network's password in plain text only to an
//! administrator's elevated program, so the app asks the helper ([`crate::Request::WifiProfiles`])
//! when, and only when, the helper was started with `--wifi yes` through the system's own
//! prompt.
//!
//! Read with Windows' own tool, `netsh wlan export profile name="<network>" key=clear`, from
//! Windows' own folder (never a path from the environment or the app), once for each network the
//! person ticked and for no other (the fresh-context security review, finding 2: never every
//! saved network's password); each writes that network's profile to a file.
//! The files go into a fresh folder only the helper's account and the system may open, made so
//! from its first moment (`pctwin_helper_os::private_folder`); each file is read into memory that
//! is wiped when dropped and deleted at once; the folder is removed when reading ends, also on
//! failure. These are the only files the helper ever writes on the old laptop, and none outlives
//! the request. The profiles go to the app over the helper's private channel; the app keeps only
//! the networks its person ticked and parses them itself.

use std::path::{Path, PathBuf};

use zeroize::Zeroizing;

use crate::protocol::{MAX_WIFI_BYTES, MAX_WIFI_PROFILES, Refusal, Secret};

/// The prefix of the helper's Wi-Fi folders in its temporary folder.
pub const FOLDER_PREFIX: &str = "pctwin-helper-wifi-";

/// The most one profile file may hold.
pub const MAX_PROFILE: u64 = pctwin_types::limits::WIFI_PROFILE_BYTES;

/// A folder removed with everything in it when dropped (also on failure).
/// Used on Windows only (the export there); elsewhere only by the tests.
#[cfg_attr(not(windows), allow(dead_code))]
struct Folder(PathBuf);

impl Drop for Folder {
    fn drop(&mut self) {
        let _ = std::fs::remove_dir_all(&self.0);
    }
}

/// Every profile file in `folder`, each read into wiped memory and deleted from the disk before
/// the next is read; files too large, links and anything not a plain file are deleted unread.
/// Bounded: at most [`MAX_WIFI_PROFILES`] files and [`MAX_WIFI_BYTES`] in all (the rest are
/// deleted unread).
pub fn take_profiles(folder: &Path) -> Vec<Secret> {
    let mut out = Vec::new();
    let mut bytes = 0usize;
    let Ok(entries) = std::fs::read_dir(folder) else {
        return out;
    };
    for e in entries.flatten() {
        let path = e.path();
        let plain =
            std::fs::symlink_metadata(&path).is_ok_and(|m| m.is_file() && m.len() <= MAX_PROFILE);
        let read = if plain && out.len() < MAX_WIFI_PROFILES {
            std::fs::read(&path).ok().map(Zeroizing::new)
        } else {
            None
        };
        // Gone from the disk before anything else happens to it.
        let _ = std::fs::remove_file(&path);
        let Some(raw) = read else {
            continue;
        };
        let Ok(text) = std::str::from_utf8(&raw) else {
            continue;
        };
        let text = text.trim_start_matches('\u{feff}');
        if bytes + text.len() > MAX_WIFI_BYTES {
            continue;
        }
        bytes += text.len();
        out.push(Secret::new(Zeroizing::new(text.to_string())));
    }
    out
}

/// A fresh private folder in `parent` (a random name; never one that exists).
/// Used on Windows only (the export there); elsewhere only by the tests.
#[cfg_attr(not(windows), allow(dead_code))]
fn fresh_folder(parent: &Path) -> Result<Folder, Refusal> {
    for _ in 0..8 {
        let token = pctwin_helper_os::random_token().map_err(|_| Refusal::Unreadable)?;
        let name: String = token.chars().take(24).collect();
        let path = parent.join(format!("{FOLDER_PREFIX}{name}"));
        match pctwin_helper_os::private_folder(&path) {
            Ok(()) => return Ok(Folder(path)),
            Err(pctwin_helper_os::Error::NameTaken) => continue,
            Err(_) => return Err(Refusal::Unreadable),
        }
    }
    Err(Refusal::Unreadable)
}

/// Whether `name` may be put to Windows' tool as a network's name: plain text of 1 to 32
/// characters (a network's name is at most 32 bytes), without a quote, a control character or a
/// `%` (which the command line could expand), so `name="<name>"` names exactly that network.
pub fn name_ok(name: &str) -> bool {
    !name.is_empty()
        && name.chars().count() <= 32
        && name.len() <= 128
        && !name.contains(['"', '%'])
        && !name.chars().any(char::is_control)
        // A trailing backslash would escape the closing quote of `name="..."`.
        && !name.ends_with('\\')
}

/// What Windows' tool is told, once for each network named, in order: each always names its one
/// network (`name="..."`), never every network. `None` if any name is not plain text
/// ([`name_ok`]) or the folder has a quote or a control character.
pub fn export_args(names: &[String], folder: &str) -> Option<Vec<Vec<String>>> {
    if folder.contains('"') || folder.chars().any(char::is_control) {
        return None;
    }
    names
        .iter()
        .map(|n| {
            name_ok(n).then(|| {
                vec![
                    "wlan export profile".to_string(),
                    format!("name=\"{n}\""),
                    "key=clear".to_string(),
                    format!("folder=\"{folder}\""),
                ]
            })
        })
        .collect()
}

/// The saved Wi-Fi networks `names` (the ones the person ticked) with their passwords, and no
/// other (see the module notes): Windows.
#[cfg(windows)]
pub fn export_profiles(names: &[String]) -> Result<Vec<Secret>, Refusal> {
    if names.len() > MAX_WIFI_PROFILES {
        return Err(Refusal::TooMuch);
    }
    let netsh = pctwin_helper_os::system_folder(pctwin_helper_os::SystemFolder::Windows)
        .map_err(|_| Refusal::Unreadable)?
        .join("System32")
        .join("netsh.exe");
    let folder = fresh_folder(&std::env::temp_dir())?;
    let text = folder.0.to_str().ok_or(Refusal::Unreadable)?;
    let calls = export_args(names, text).ok_or(Refusal::NotPlain)?;
    let mut any = false;
    for args in calls {
        let mut command =
            pctwin_helper_os::clean_command(&netsh).map_err(|_| Refusal::Unreadable)?;
        {
            use std::os::windows::process::CommandExt;
            // netsh reads `name="..."` and `folder="..."` itself: both plain, without quotes.
            for a in &args {
                command.raw_arg(a);
            }
        }
        let ran = command
            .stdin(std::process::Stdio::null())
            .stdout(std::process::Stdio::null())
            .stderr(std::process::Stdio::null())
            .status()
            .map_err(|_| Refusal::Unreadable)?;
        any |= ran.success();
    }
    let profiles = take_profiles(&folder.0);
    drop(folder);
    if !any && profiles.is_empty() && !names.is_empty() {
        return Err(Refusal::Unreadable);
    }
    Ok(profiles)
}

/// Only Windows needs the helper for this (a Mac asks in macOS's own prompt, Linux in its own).
#[cfg(not(windows))]
pub fn export_profiles(_names: &[String]) -> Result<Vec<Secret>, Refusal> {
    Err(Refusal::NotApproved)
}

/// Removes Wi-Fi folders a crash left in `parent` (called when the helper starts).
pub fn sweep_leftovers(parent: &Path) -> usize {
    let Ok(entries) = std::fs::read_dir(parent) else {
        return 0;
    };
    let mut n = 0;
    for e in entries.flatten() {
        let ours = e.file_name().to_string_lossy().starts_with(FOLDER_PREFIX);
        let dir = e.file_type().is_ok_and(|t| t.is_dir() && !t.is_symlink());
        if ours && dir && std::fs::remove_dir_all(e.path()).is_ok() {
            n += 1;
        }
    }
    n
}

#[cfg(test)]
mod tests {
    use super::*;

    /// A name ending in a backslash would escape the closing quote of `name="..."`: refused.
    #[test]
    fn a_name_ending_in_a_backslash_is_refused() {
        assert!(!name_ok("Home\\"));
        assert!(name_ok("Home\\Office"));
        assert!(name_ok("Home"));
    }

    #[test]
    fn profiles_are_taken_whole_and_gone_from_the_disk() {
        let dir = tempfile::tempdir().unwrap();
        std::fs::write(dir.path().join("Wi-Fi-Home.xml"), "\u{feff}<WLANProfile/>").unwrap();
        std::fs::write(
            dir.path().join("Wi-Fi-Cafe.xml"),
            "<WLANProfile>c</WLANProfile>",
        )
        .unwrap();
        // Too large, or not text: deleted unread.
        std::fs::write(
            dir.path().join("big.xml"),
            vec![b'a'; MAX_PROFILE as usize + 1],
        )
        .unwrap();
        std::fs::write(dir.path().join("bad.xml"), [0xff, 0xfe, 0x00]).unwrap();
        let mut got: Vec<String> = take_profiles(dir.path())
            .iter()
            .map(|s| s.as_str().to_string())
            .collect();
        got.sort();
        assert_eq!(got, ["<WLANProfile/>", "<WLANProfile>c</WLANProfile>"]);
        assert_eq!(std::fs::read_dir(dir.path()).unwrap().count(), 0);
    }

    /// The fresh-context security review, finding 2: Windows' tool is asked for each ticked
    /// network by its name, and only for those; a name that is not plain text stops it before
    /// anything runs; nothing ever asks for every network.
    #[test]
    fn only_the_ticked_networks_are_asked_for_by_name() {
        let names = vec!["Home".to_string(), "Caf\u{e9} 5G".to_string()];
        let calls = export_args(&names, r"C:\Temp\pctwin-helper-wifi-x").unwrap();
        assert_eq!(calls.len(), 2);
        for (call, name) in calls.iter().zip(&names) {
            assert_eq!(call[1], format!("name=\"{name}\""));
            assert_eq!(call.iter().filter(|a| a.starts_with("name=")).count(), 1);
            assert!(call.contains(&"key=clear".to_string()));
        }
        assert_eq!(export_args(&[], "x"), Some(vec![]));
        for bad in ["", "a\"b", "a%PATH%", "line\nbreak", &"x".repeat(33)] {
            assert_eq!(export_args(&[bad.to_string()], "x"), None, "{bad:?}");
        }
        assert_eq!(export_args(&names, "a\"b"), None);
    }

    /// Windows' own tool, for real: whatever this laptop has (a runner without Wi-Fi has none),
    /// nothing of it is left on the disk, and each profile is the system's own kind.
    #[cfg(windows)]
    #[test]
    fn the_real_tool_leaves_nothing_behind() {
        let before = sweep_leftovers(&std::env::temp_dir());
        let _ = before;
        match export_profiles(&["PCTwin lab, not here".to_string()]) {
            Ok(profiles) => {
                assert!(profiles.iter().all(|p| p.as_str().contains("WLANProfile")));
            }
            Err(why) => assert_eq!(why, Refusal::Unreadable),
        }
        let left = std::fs::read_dir(std::env::temp_dir())
            .unwrap()
            .flatten()
            .filter(|e| e.file_name().to_string_lossy().starts_with(FOLDER_PREFIX))
            .count();
        assert_eq!(left, 0);
    }

    #[test]
    fn a_folder_a_crash_left_is_swept_and_nothing_else() {
        let dir = tempfile::tempdir().unwrap();
        std::fs::create_dir(dir.path().join(format!("{FOLDER_PREFIX}abc"))).unwrap();
        std::fs::create_dir(dir.path().join("pctwin-other")).unwrap();
        std::fs::write(dir.path().join(format!("{FOLDER_PREFIX}file")), "x").unwrap();
        assert_eq!(sweep_leftovers(dir.path()), 1);
        assert!(dir.path().join("pctwin-other").exists());
        assert!(dir.path().join(format!("{FOLDER_PREFIX}file")).exists());
    }

    #[test]
    fn a_fresh_folder_is_private_and_removed_when_done() {
        let dir = tempfile::tempdir().unwrap();
        let path = {
            let f = fresh_folder(dir.path()).unwrap();
            assert!(f.0.is_dir());
            assert!(
                f.0.file_name()
                    .unwrap()
                    .to_string_lossy()
                    .starts_with(FOLDER_PREFIX)
            );
            #[cfg(unix)]
            {
                use std::os::unix::fs::PermissionsExt;
                let mode = std::fs::metadata(&f.0).unwrap().permissions().mode();
                assert_eq!(mode & 0o077, 0);
            }
            f.0.clone()
        };
        assert!(!path.exists());
    }
}
