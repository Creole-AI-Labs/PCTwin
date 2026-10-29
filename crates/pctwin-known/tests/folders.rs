//! The folders a remembered connection may use (Security Design Part T).

use std::path::{Path, PathBuf};

use pctwin_known::folders::{Places, open_plain_around, refused_on};
use pctwin_known::{FolderProblem, KnownFolder};

fn places(home: &Path) -> Places {
    Places {
        home: Some(home.to_path_buf()),
        appdata: Some(home.join("AppData/Roaming")),
        localappdata: Some(home.join("AppData/Local")),
        programdata: Some(home.join("ProgramData")),
        windir: Some(home.join("Windows")),
        programfiles: Some(home.join("Program Files")),
        programfilesx86: None,
        root: Some(home.join("root")),
    }
}

fn made(p: PathBuf) -> PathBuf {
    std::fs::create_dir_all(&p).unwrap();
    p
}

#[test]
fn startup_autostart_launch_agents_and_key_folders_are_refused() {
    let d = tempfile::tempdir().unwrap();
    let home = d.path();
    let p = places(home);
    let cases: &[(&str, &str, FolderProblem)] = &[
        (
            "windows",
            "AppData/Roaming/Microsoft/Windows/Start Menu/Programs/Startup",
            FolderProblem::RunsAtSignIn,
        ),
        (
            "windows",
            "ProgramData/Microsoft/Windows/Start Menu/Programs/StartUp",
            FolderProblem::RunsAtSignIn,
        ),
        ("windows", ".ssh", FolderProblem::HoldsKeys),
        ("windows", ".ssh/inner", FolderProblem::HoldsKeys),
        (
            "windows",
            "Documents/WindowsPowerShell",
            FolderProblem::ShellStartUp,
        ),
        ("windows", "Windows/System32", FolderProblem::System),
        ("macos", "Library/LaunchAgents", FolderProblem::RunsAtSignIn),
        (
            "macos",
            "root/Library/LaunchDaemons",
            FolderProblem::RunsAtSignIn,
        ),
        ("macos", "Library/Keychains", FolderProblem::HoldsKeys),
        ("linux", ".config/autostart", FolderProblem::RunsAtSignIn),
        ("linux", ".config/systemd/user", FolderProblem::RunsAtSignIn),
        ("linux", ".local/bin", FolderProblem::ShellStartUp),
        ("linux", ".gnupg", FolderProblem::HoldsKeys),
    ];
    for (system, rel, why) in cases {
        let f = made(home.join(rel));
        assert_eq!(refused_on(system, &f, &p), Some(*why), "{system} {rel}");
    }
}

/// A folder that holds such a place is refused too: a sent folder path could reach it.
#[test]
fn a_folder_holding_such_a_place_is_refused() {
    let d = tempfile::tempdir().unwrap();
    let home = d.path();
    let p = places(home);
    for (system, rel) in [
        ("linux", ""),
        ("linux", ".config"),
        ("macos", ""),
        ("macos", "Library"),
        ("windows", ""),
        ("windows", "AppData"),
        ("windows", "AppData/Roaming/Microsoft/Windows/Start Menu"),
    ] {
        let f = made(home.join(rel));
        assert!(refused_on(system, &f, &p).is_some(), "{system} {rel:?}");
    }
}

#[test]
fn everyday_folders_are_allowed() {
    let d = tempfile::tempdir().unwrap();
    let home = d.path();
    let p = places(home);
    for system in ["windows", "macos", "linux"] {
        for rel in ["Downloads", "Desktop/Photos", "Pictures"] {
            let f = made(home.join(rel));
            assert_eq!(refused_on(system, &f, &p), None, "{system} {rel}");
        }
    }
    // Documents holds the PowerShell profile folder, which a person sees by name before Save.
    assert_eq!(
        refused_on("windows", &made(home.join("Documents")), &p),
        None
    );
}

/// A link to a refused place is that place.
#[test]
fn a_link_to_a_refused_place_is_refused() {
    let d = tempfile::tempdir().unwrap();
    let home = d.path();
    let p = places(home);
    let ssh = made(home.join(".ssh"));
    let link = home.join("Downloads-link");
    #[cfg(windows)]
    {
        let made = std::process::Command::new("cmd")
            .args(["/C", "mklink", "/J"])
            .arg(&link)
            .arg(&ssh)
            .output()
            .unwrap();
        assert!(made.status.success());
    }
    #[cfg(unix)]
    std::os::unix::fs::symlink(&ssh, &link).unwrap();
    assert_eq!(
        refused_on(pctwin_known::folders::this_system(), &link, &p),
        Some(FolderProblem::HoldsKeys)
    );
    // And a link is never held as a folder.
    assert_eq!(
        KnownFolder::choose_with(&link, |_| None).err(),
        Some(FolderProblem::NotPlain)
    );
}

#[test]
fn a_remembered_folder_is_rechecked_every_time() {
    let d = tempfile::tempdir().unwrap();
    let inbox = made(d.path().join("Inbox"));
    let (known, _) = KnownFolder::choose_with(&inbox, |_| None).unwrap();
    assert!(known.reopen_with(|_| None).is_ok());
    // A refused place now (the catalogue grew).
    assert_eq!(
        known
            .reopen_with(|_| Some(FolderProblem::RunsAtSignIn))
            .err(),
        Some(FolderProblem::RunsAtSignIn)
    );
    // Swapped for another folder at the same path.
    std::fs::rename(&inbox, d.path().join("Inbox-moved")).unwrap();
    made(inbox.clone());
    assert_eq!(
        known.reopen_with(|_| None).err(),
        Some(FolderProblem::NotTheSame)
    );
    // Gone.
    std::fs::remove_dir(&inbox).unwrap();
    assert_eq!(
        known.reopen_with(|_| None).err(),
        Some(FolderProblem::NotPlain)
    );
    // A refused folder is never chosen.
    let other = made(d.path().join("Other"));
    assert_eq!(
        KnownFolder::choose_with(&other, |_| Some(FolderProblem::HoldsKeys)).err(),
        Some(FolderProblem::HoldsKeys)
    );
}

#[test]
fn nothing_swapped_opens_and_a_missing_folder_does_not() {
    let d = tempfile::tempdir().unwrap();
    let f = made(d.path().join("F"));
    assert!(open_plain_around(&f, || (), || ()).is_some());
    assert!(open_plain_around(&d.path().join("nope"), || (), || ()).is_none());
}
