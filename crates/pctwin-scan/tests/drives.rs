//! Drives on the old laptop (Task List 1.4): every real drive with its type and what that type can
//! keep, and never assuming the running system owns every drive: another installed system is found
//! by its own marker files and read only through rescue, with permission.

use std::path::Path;

use pctwin_scan::{CaseRule, FileSystem, OtherSystem, is_real_mount, list_drives, other_system};

#[test]
fn file_system_names_from_every_system_are_recognised() {
    for (name, fs) in [
        ("NTFS", FileSystem::Ntfs),
        ("ntfs3", FileSystem::Ntfs),
        ("ReFS", FileSystem::ReFs),
        ("FAT32", FileSystem::Fat32),
        ("vfat", FileSystem::Fat32),
        ("msdos", FileSystem::Fat32),
        ("exFAT", FileSystem::ExFat),
        ("exfat", FileSystem::ExFat),
        ("apfs", FileSystem::Apfs),
        ("hfs", FileSystem::HfsPlus),
        ("ext4", FileSystem::Ext4),
        ("ext3", FileSystem::Ext4),
        ("btrfs", FileSystem::Btrfs),
        ("xfs", FileSystem::Xfs),
    ] {
        assert_eq!(FileSystem::from_name(name), fs, "{name}");
    }
    assert_eq!(
        FileSystem::from_name("zfs"),
        FileSystem::Other("zfs".into())
    );
}

#[test]
fn each_drive_type_says_what_it_can_keep() {
    let fat = FileSystem::Fat32.keeps();
    assert_eq!(fat.largest_file, Some(4 * 1024 * 1024 * 1024 - 1));
    assert!(!fat.permissions && !fat.links && !fat.extra_streams);
    assert_eq!(fat.case, CaseRule::IgnoresCase);
    assert_eq!(fat.time_step_ns, 2_000_000_000);

    let exfat = FileSystem::ExFat.keeps();
    assert_eq!(exfat.largest_file, None);
    assert!(!exfat.permissions && !exfat.links);
    assert_eq!(exfat.time_step_ns, 10_000_000);

    let ntfs = FileSystem::Ntfs.keeps();
    assert!(ntfs.permissions && ntfs.links && ntfs.extra_streams);
    assert_eq!(ntfs.case, CaseRule::IgnoresCase);
    assert_eq!(ntfs.time_step_ns, 100);

    let ext4 = FileSystem::Ext4.keeps();
    assert!(ext4.permissions && ext4.links && !ext4.extra_streams);
    assert_eq!(ext4.case, CaseRule::KeepsCase);

    let apfs = FileSystem::Apfs.keeps();
    assert!(apfs.permissions && apfs.links);
    assert_eq!(apfs.case, CaseRule::IgnoresCase);

    // Unknown types promise nothing.
    let unknown = FileSystem::Other("zfs".into()).keeps();
    assert!(!unknown.permissions && !unknown.links && !unknown.extra_streams);
    assert_eq!(unknown.case, CaseRule::Unknown);
}

#[test]
fn another_installed_system_is_found_by_its_marker_files() {
    let make = |rel: &str| {
        let root = tempfile::tempdir().unwrap();
        let p = root.path().join(rel);
        std::fs::create_dir_all(p.parent().unwrap()).unwrap();
        std::fs::write(p, b"x").unwrap();
        root
    };
    let win = make("Windows/System32/config/SYSTEM");
    assert_eq!(other_system(win.path()), Some(OtherSystem::Windows));
    let mac = make("System/Library/CoreServices/SystemVersion.plist");
    assert_eq!(other_system(mac.path()), Some(OtherSystem::MacOs));
    let linux = make("etc/os-release");
    assert_eq!(other_system(linux.path()), Some(OtherSystem::Linux));
    let data = make("Photos/holiday.jpg");
    assert_eq!(other_system(data.path()), None);
    // A folder merely named Windows is not a system.
    let lookalike = make("Windows/readme.txt");
    assert_eq!(other_system(lookalike.path()), None);
    let half = make("Windows/System32/notes.txt");
    assert_eq!(other_system(half.path()), None);
}

#[test]
fn system_and_temporary_mounts_are_not_drives() {
    for (fs, mount) in [
        ("ext4", "/"),
        ("ext4", "/home"),
        ("vfat", "/boot/efi"),
        ("exfat", "/media/ada/USB"),
        ("apfs", "/"),
        ("apfs", "/Volumes/Backup"),
        ("NTFS", "C:\\"),
        ("FAT32", "E:\\"),
    ] {
        assert!(is_real_mount(fs, Path::new(mount)), "{fs} {mount}");
    }
    for (fs, mount) in [
        ("tmpfs", "/run"),
        ("proc", "/proc"),
        ("sysfs", "/sys"),
        ("devtmpfs", "/dev"),
        ("squashfs", "/snap/core/123"),
        ("squashfs", "/mnt/disk-image"),
        ("overlay", "/var/lib/docker/overlay2/x/merged"),
        ("efivarfs", "/sys/firmware/efi/efivars"),
        ("apfs", "/System/Volumes/VM"),
        ("apfs", "/System/Volumes/Preboot"),
        ("devfs", "/dev"),
        ("autofs", "/net"),
    ] {
        assert!(!is_real_mount(fs, Path::new(mount)), "{fs} {mount}");
    }
}

#[test]
fn this_laptop_has_exactly_one_system_drive_and_knows_its_type() {
    let drives = list_drives();
    let system: Vec<_> = drives.iter().filter(|d| d.is_system).collect();
    assert_eq!(system.len(), 1, "{drives:#?}");
    assert!(
        !matches!(system[0].file_system, FileSystem::Other(_)),
        "{:?}",
        system[0]
    );
    assert!(system[0].total_bytes > 0);
    // The running system's own drive is never reported as "another system".
    assert_eq!(system[0].other_system, None);
}
