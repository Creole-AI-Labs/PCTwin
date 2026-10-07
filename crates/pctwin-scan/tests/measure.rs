//! Measuring a folder without opening any file (Task List 1.4): how many files and bytes, which
//! files are only in the cloud (never downloaded by the scan), and what could not be read, listed
//! rather than hidden. Links are never followed.

use pctwin_scan::{Size, cloud_only_mac, cloud_only_windows, is_icloud_stub_name, measure};

#[test]
fn files_and_bytes_are_counted_through_every_folder() {
    let root = tempfile::tempdir().unwrap();
    std::fs::create_dir_all(root.path().join("a/b/c")).unwrap();
    std::fs::write(root.path().join("one.txt"), b"12345").unwrap();
    std::fs::write(root.path().join("a/two.txt"), b"123").unwrap();
    std::fs::write(root.path().join("a/b/c/three.bin"), vec![0u8; 1000]).unwrap();
    let m = measure(root.path());
    assert_eq!(m.files, 3);
    assert_eq!(m.folders, 3);
    assert_eq!(m.bytes, 1008);
    assert_eq!(m.cloud_only_files, 0);
    assert!(m.unreadable.is_empty(), "{:?}", m.unreadable);
}

#[test]
fn an_empty_folder_measures_nothing() {
    let root = tempfile::tempdir().unwrap();
    let m = measure(root.path());
    assert_eq!((m.files, m.folders, m.bytes), (0, 0, 0));
}

#[test]
fn a_missing_folder_is_reported_unreadable_not_empty() {
    let root = tempfile::tempdir().unwrap();
    let gone = root.path().join("gone");
    let m = measure(&gone);
    assert_eq!(m.unreadable, [gone]);
}

#[cfg(unix)]
#[test]
fn links_are_counted_as_links_and_never_followed() {
    let root = tempfile::tempdir().unwrap();
    let outside = tempfile::tempdir().unwrap();
    std::fs::write(outside.path().join("big.bin"), vec![0u8; 5000]).unwrap();
    std::os::unix::fs::symlink(outside.path(), root.path().join("link")).unwrap();
    std::fs::write(root.path().join("mine.txt"), b"x").unwrap();
    let m = measure(root.path());
    assert_eq!(m.files, 1);
    assert_eq!(m.bytes, 1);
    assert_eq!(m.links, 1);
}

#[cfg(windows)]
#[test]
fn junctions_are_counted_as_links_and_never_followed() {
    let root = tempfile::tempdir().unwrap();
    let outside = tempfile::tempdir().unwrap();
    std::fs::write(outside.path().join("big.bin"), vec![0u8; 5000]).unwrap();
    let ok = std::process::Command::new("cmd")
        .arg("/C")
        .arg("mklink")
        .arg("/J")
        .arg(root.path().join("link"))
        .arg(outside.path())
        .output()
        .unwrap()
        .status
        .success();
    assert!(ok, "could not make a junction");
    std::fs::write(root.path().join("mine.txt"), b"x").unwrap();
    let m = measure(root.path());
    assert_eq!(m.files, 1);
    assert_eq!(m.bytes, 1);
    assert_eq!(m.links, 1);
}

#[cfg(unix)]
#[test]
fn a_folder_that_cannot_be_read_is_listed_and_the_rest_still_counted() {
    use std::os::unix::fs::PermissionsExt;
    let root = tempfile::tempdir().unwrap();
    let locked = root.path().join("locked");
    std::fs::create_dir(&locked).unwrap();
    std::fs::write(locked.join("secret.txt"), b"hidden").unwrap();
    std::fs::write(root.path().join("open.txt"), b"seen").unwrap();
    std::fs::set_permissions(&locked, std::fs::Permissions::from_mode(0o000)).unwrap();
    let m = measure(root.path());
    let size = Size::of(root.path());
    std::fs::set_permissions(&locked, std::fs::Permissions::from_mode(0o755)).unwrap();
    // Running as root (some CI machines) can read everything; then nothing is unreadable.
    if std::fs::read_dir(&locked).is_ok() && m.unreadable.is_empty() && m.files == 2 {
        return;
    }
    assert_eq!(m.unreadable, [locked]);
    assert_eq!(m.files, 1);
    assert_eq!(m.bytes, 4);
    assert_eq!(size, Size::AtLeast { bytes: 4, files: 1 });
}

#[test]
fn windows_cloud_placeholders_are_recognised_by_their_attributes() {
    const NORMAL: u32 = 0x80;
    const ARCHIVE: u32 = 0x20;
    const OFFLINE: u32 = 0x1000;
    const RECALL_ON_OPEN: u32 = 0x4_0000;
    const PINNED: u32 = 0x8_0000;
    const UNPINNED: u32 = 0x10_0000;
    const RECALL_ON_DATA_ACCESS: u32 = 0x40_0000;
    assert!(!cloud_only_windows(NORMAL));
    assert!(!cloud_only_windows(ARCHIVE | PINNED));
    // A file kept on the laptop but allowed to be freed later is still here now.
    assert!(!cloud_only_windows(ARCHIVE | UNPINNED));
    assert!(cloud_only_windows(ARCHIVE | RECALL_ON_DATA_ACCESS));
    assert!(cloud_only_windows(RECALL_ON_OPEN));
    assert!(cloud_only_windows(OFFLINE));
    assert!(cloud_only_windows(UNPINNED | RECALL_ON_DATA_ACCESS));
}

#[test]
fn mac_files_only_in_icloud_are_recognised() {
    const UF_HIDDEN: u32 = 0x8000;
    const SF_DATALESS: u32 = 0x4000_0000;
    assert!(!cloud_only_mac(0));
    assert!(!cloud_only_mac(UF_HIDDEN));
    assert!(cloud_only_mac(SF_DATALESS));
    assert!(cloud_only_mac(SF_DATALESS | UF_HIDDEN));
    // Before macOS 14, an evicted file was replaced by a hidden stand-in named like this.
    assert!(is_icloud_stub_name(".Report.pdf.icloud"));
    assert!(is_icloud_stub_name(".notes.icloud"));
    assert!(!is_icloud_stub_name("Report.pdf.icloud"));
    assert!(!is_icloud_stub_name(".icloud"));
    assert!(!is_icloud_stub_name("..icloud"));
    assert!(!is_icloud_stub_name(".hidden.txt"));
}

#[test]
fn a_size_is_known_or_honestly_unknown() {
    let root = tempfile::tempdir().unwrap();
    std::fs::write(root.path().join("a"), b"abc").unwrap();
    assert_eq!(Size::of(root.path()), Size::Known { bytes: 3, files: 1 });
    assert_eq!(Size::of(&root.path().join("missing")), Size::Unreadable);
}
