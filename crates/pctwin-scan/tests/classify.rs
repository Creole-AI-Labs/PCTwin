//! Where a special folder really is (Task List 1.4): on the system drive, another drive, a network
//! share, or inside a cloud storage folder. These rules are pure, so they are tested with made-up
//! laptops on each system.

use std::path::{Path, PathBuf};

use pctwin_record::{CloudProvider, Storage};
use pctwin_scan::{Facts, classify};

#[cfg(windows)]
fn other(drive: &str) -> Storage {
    Storage::OtherDrive {
        drive: drive.into(),
    }
}

fn cloud(provider: CloudProvider) -> Storage {
    Storage::Cloud { provider }
}

#[cfg(windows)]
fn windows_facts() -> Facts {
    Facts {
        home: PathBuf::from(r"C:\Users\Ada"),
        system_drive: Some("C:".into()),
        cloud_roots: vec![
            (
                CloudProvider::OneDrive,
                PathBuf::from(r"C:\Users\Ada\OneDrive"),
            ),
            (
                CloudProvider::OneDrive,
                PathBuf::from(r"C:\Users\Ada\OneDrive - Contoso"),
            ),
            (CloudProvider::Dropbox, PathBuf::from(r"E:\Dropbox")),
        ],
        icloud_desktop_and_documents: false,
    }
}

#[cfg(windows)]
#[test]
fn windows_folders_are_placed_by_drive_and_cloud_folder() {
    let f = windows_facts();
    let cases: Vec<(&str, Storage)> = vec![
        (r"C:\Users\Ada\Documents", Storage::SystemDrive),
        (r"c:\users\ada\documents", Storage::SystemDrive),
        (
            r"C:\Users\Ada\OneDrive\Documents",
            cloud(CloudProvider::OneDrive),
        ),
        (
            r"c:\USERS\ada\onedrive\Pictures",
            cloud(CloudProvider::OneDrive),
        ),
        (
            r"C:\Users\Ada\OneDrive - Contoso\Desktop",
            cloud(CloudProvider::OneDrive),
        ),
        (r"C:\Users\Ada\OneDrive", cloud(CloudProvider::OneDrive)),
        (r"D:\Documents", other("D:")),
        (r"d:\Documents", other("D:")),
        (r"E:\Dropbox\Docs", cloud(CloudProvider::Dropbox)),
        (r"E:\Other", other("E:")),
        (r"\\?\C:\Users\Ada\Documents", Storage::SystemDrive),
        (r"\\?\D:\Documents", other("D:")),
        (r"\\server\share\Ada\Documents", other(r"\\server\share")),
        (r"\\?\UNC\server\share\Ada", other(r"\\server\share")),
    ];
    for (path, expected) in cases {
        assert_eq!(classify(Path::new(path), &f), expected, "{path}");
    }
}

#[cfg(windows)]
#[test]
fn a_folder_merely_named_like_a_cloud_folder_is_not_inside_it() {
    let f = windows_facts();
    assert_eq!(
        classify(Path::new(r"C:\Users\Ada\OneDriveBackup\Docs"), &f),
        Storage::SystemDrive
    );
    assert_eq!(
        classify(Path::new(r"C:\Users\Ada\OneDrive - Contoso Old"), &f),
        Storage::SystemDrive
    );
}

#[cfg(windows)]
#[test]
fn without_a_known_system_drive_nothing_is_called_another_drive() {
    let mut f = windows_facts();
    f.system_drive = None;
    assert_eq!(
        classify(Path::new(r"D:\Documents"), &f),
        Storage::SystemDrive
    );
}

#[cfg(unix)]
fn unix_facts() -> Facts {
    Facts {
        home: PathBuf::from("/Users/ada"),
        system_drive: None,
        cloud_roots: vec![
            (
                CloudProvider::ICloud,
                PathBuf::from("/Users/ada/Library/Mobile Documents/com~apple~CloudDocs"),
            ),
            (
                CloudProvider::OneDrive,
                PathBuf::from("/Users/ada/Library/CloudStorage/OneDrive-Personal"),
            ),
            (CloudProvider::Dropbox, PathBuf::from("/Users/ada/Dropbox")),
        ],
        icloud_desktop_and_documents: false,
    }
}

#[cfg(unix)]
#[test]
fn unix_folders_are_placed_by_cloud_folder() {
    let f = unix_facts();
    let cases: Vec<(&str, Storage)> = vec![
        ("/Users/ada/Documents", Storage::SystemDrive),
        (
            "/Users/ada/Library/Mobile Documents/com~apple~CloudDocs/Documents",
            cloud(CloudProvider::ICloud),
        ),
        (
            "/Users/ada/Library/CloudStorage/OneDrive-Personal/Pictures",
            cloud(CloudProvider::OneDrive),
        ),
        ("/Users/ada/Dropbox/Work", cloud(CloudProvider::Dropbox)),
        ("/Users/ada/DropboxOld", Storage::SystemDrive),
    ];
    for (path, expected) in cases {
        assert_eq!(classify(Path::new(path), &f), expected, "{path}");
    }
    // Linux keeps case in names, so this is a different folder; a Mac disk ignores case.
    let other_case = classify(Path::new("/Users/ada/dropbox/Work"), &f);
    if cfg!(target_os = "macos") {
        assert_eq!(other_case, cloud(CloudProvider::Dropbox));
    } else {
        assert_eq!(other_case, Storage::SystemDrive);
    }
}

#[test]
fn the_longest_matching_cloud_folder_wins() {
    let home = std::env::temp_dir();
    let outer = home.join("Cloud");
    let inner = outer.join("Dropbox");
    let f = Facts {
        home: home.clone(),
        system_drive: None,
        cloud_roots: vec![
            (CloudProvider::OneDrive, outer),
            (CloudProvider::Dropbox, inner.clone()),
        ],
        icloud_desktop_and_documents: false,
    };
    assert_eq!(
        classify(&inner.join("x"), &f),
        cloud(CloudProvider::Dropbox)
    );
}
