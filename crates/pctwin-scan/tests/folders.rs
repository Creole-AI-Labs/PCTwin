//! Finding each special folder through the system (Task List 1.4 and the 0.7 probe): the answer
//! must match what the system itself reports, folders moved elsewhere must be found where they
//! are, and a folder that isn't there is reported missing, never guessed.

use std::path::PathBuf;
use std::process::Command;

use pctwin_record::FolderRole;
use pctwin_scan::{FolderLookup, find_special_folders};

const ROLES: [FolderRole; 8] = [
    FolderRole::Home,
    FolderRole::Desktop,
    FolderRole::Documents,
    FolderRole::Downloads,
    FolderRole::Pictures,
    FolderRole::Music,
    FolderRole::Videos,
    FolderRole::Public,
];

fn lookup(found: &[FolderLookup], role: FolderRole) -> &FolderLookup {
    found.iter().find(|f| f.role() == role).unwrap()
}

fn found_path(found: &[FolderLookup], role: FolderRole) -> Option<PathBuf> {
    match lookup(found, role) {
        FolderLookup::Found(f) => Some(f.path.clone()),
        FolderLookup::Missing { .. } => None,
    }
}

#[test]
fn every_role_is_answered_once_and_found_folders_exist() {
    let found = find_special_folders();
    for role in ROLES {
        assert_eq!(
            found.iter().filter(|f| f.role() == role).count(),
            1,
            "{role:?}"
        );
    }
    assert_eq!(found.len(), ROLES.len());
    for f in &found {
        if let FolderLookup::Found(f) = f {
            assert!(f.path.is_dir(), "{:?} {}", f.role, f.path.display());
            assert!(f.path.is_absolute());
        }
    }
    assert_eq!(found_path(&found, FolderRole::Home), dirs_home());
}

fn dirs_home() -> Option<PathBuf> {
    std::env::var_os(if cfg!(windows) { "USERPROFILE" } else { "HOME" }).map(PathBuf::from)
}

/// Windows answers through its own known-folder lookup; .NET asks the same question a different
/// way, so the two must agree.
#[cfg(windows)]
#[test]
fn windows_folders_match_what_windows_reports() {
    let found = find_special_folders();
    for (role, name) in [
        (FolderRole::Desktop, "Desktop"),
        (FolderRole::Documents, "MyDocuments"),
        (FolderRole::Pictures, "MyPictures"),
        (FolderRole::Music, "MyMusic"),
        (FolderRole::Videos, "MyVideos"),
    ] {
        let out = Command::new("powershell")
            .args([
                "-NoProfile",
                "-NonInteractive",
                "-Command",
                &format!("[Environment]::GetFolderPath('{name}')"),
            ])
            .output()
            .unwrap();
        let reported = String::from_utf8_lossy(&out.stdout).trim().to_string();
        let ours = found_path(&found, role);
        if reported.is_empty() {
            assert_eq!(ours, None, "{role:?}");
        } else {
            let ours = ours.unwrap_or_else(|| panic!("{role:?} missing, Windows says {reported}"));
            assert!(
                ours.to_string_lossy().eq_ignore_ascii_case(&reported),
                "{role:?}: ours {} vs Windows {reported}",
                ours.display()
            );
        }
    }
}
