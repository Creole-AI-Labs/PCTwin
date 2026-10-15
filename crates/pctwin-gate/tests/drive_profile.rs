//! The drive decides the name rules (security design notes, decided 15 October 2026): FAT,
//! exFAT, NTFS, network and unknown drives get Windows' rules on every system; only known POSIX
//! drives (APFS, HFS+, ext4, XFS, btrfs, ZFS, F2FS) keep the system's own. These run on every
//! system: the drive's type is given, not read from the test machine's disk.
// Tests make and read files to test on; the no-file-changes fence is for the shipped code.
#![allow(clippy::disallowed_methods)]

use std::io::Write;

use pctwin_gate::{
    Destination, DriveKind, DriveProfile, FAT_LARGEST_FILE, GateError, IncomingPath,
    MAX_COMPONENT_BYTES, NameChange, NameRules, Platform, convert_name,
};
use proptest::prelude::*;

const ALL_KINDS: [DriveKind; 13] = [
    DriveKind::Fat,
    DriveKind::ExFat,
    DriveKind::Ntfs,
    DriveKind::Apfs,
    DriveKind::HfsPlus,
    DriveKind::Ext4,
    DriveKind::Xfs,
    DriveKind::Btrfs,
    DriveKind::Zfs,
    DriveKind::F2fs,
    DriveKind::Memory,
    DriveKind::Network,
    DriveKind::Unknown,
];

/// The kinds that keep the system's own names on Linux and macOS.
fn posix(kind: DriveKind) -> bool {
    matches!(
        kind,
        DriveKind::Apfs
            | DriveKind::HfsPlus
            | DriveKind::Ext4
            | DriveKind::Xfs
            | DriveKind::Btrfs
            | DriveKind::Zfs
            | DriveKind::F2fs
            | DriveKind::Memory
    )
}

#[test]
fn every_drive_kind_gets_the_right_rules_on_every_system() {
    for kind in ALL_KINDS {
        // Windows itself refuses Windows-forbidden names on every drive.
        assert_eq!(
            DriveProfile::on(kind, Platform::Windows).name_rules(),
            NameRules::Windows,
            "{kind:?} on Windows"
        );
        for system in [Platform::Linux, Platform::MacOs] {
            let expected = if posix(kind) {
                NameRules::Posix
            } else {
                NameRules::Windows
            };
            assert_eq!(
                DriveProfile::on(kind, system).name_rules(),
                expected,
                "{kind:?} on {system:?}"
            );
        }
    }
}

#[test]
fn linux_drive_types_are_read_from_their_magic_numbers() {
    for (magic, kind) in [
        (0x4d44, DriveKind::Fat), // vfat and msdos alike
        (0x2011_BAB0, DriveKind::ExFat),
        (0x7366_746e, DriveKind::Ntfs), // ntfs3
        (0x5346_544e, DriveKind::Ntfs), // the old ntfs driver
        (0xEF53, DriveKind::Ext4),
        (0x5846_5342, DriveKind::Xfs),
        (0x9123_683E, DriveKind::Btrfs),
        (0x2FC1_2FC1, DriveKind::Zfs),
        (0xF2F5_2010, DriveKind::F2fs),
        (0x0102_1994, DriveKind::Memory),  // tmpfs
        (0x6969, DriveKind::Network),      // NFS
        (0xFF53_4D42, DriveKind::Network), // CIFS
        (0xFE53_4D42, DriveKind::Network), // SMB2
        // A drive served by a program (exfat-fuse, ntfs-3g): what it is cannot be told.
        (0x6573_5546, DriveKind::Unknown),
        (0x1234_5678, DriveKind::Unknown),
    ] {
        assert_eq!(DriveKind::from_linux_magic(magic), kind, "{magic:#x}");
    }
}

#[test]
fn mac_drive_types_are_read_from_their_names_loosely() {
    for (name, kind) in [
        ("apfs", DriveKind::Apfs),
        ("hfs", DriveKind::HfsPlus),
        ("msdos", DriveKind::Fat),
        ("MSDOS", DriveKind::Fat),
        // A newer system serving the same drive from a user-space module.
        ("fskit.msdos", DriveKind::Fat),
        ("exfat", DriveKind::ExFat),
        ("ExFAT", DriveKind::ExFat),
        ("fskit.exfat", DriveKind::ExFat),
        ("ntfs", DriveKind::Ntfs),
        ("smbfs", DriveKind::Network),
        ("nfs", DriveKind::Network),
        ("afpfs", DriveKind::Network),
        ("webdav", DriveKind::Network),
        ("macfuse", DriveKind::Unknown),
        ("", DriveKind::Unknown),
    ] {
        assert_eq!(DriveKind::from_macos_type(name), kind, "{name:?}");
    }
}

#[test]
fn only_fat_has_a_largest_file() {
    for kind in ALL_KINDS {
        let expected = (kind == DriveKind::Fat).then_some(FAT_LARGEST_FILE);
        assert_eq!(DriveProfile::of(kind).largest_file(), expected, "{kind:?}");
    }
    assert_eq!(FAT_LARGEST_FILE, 4 * 1024 * 1024 * 1024 - 1);
}

/// The old `Platform` stays a way to name rules, so existing callers keep working.
#[test]
fn a_system_still_names_its_own_rules() {
    assert_eq!(NameRules::from(Platform::Windows), NameRules::Windows);
    assert_eq!(NameRules::from(Platform::MacOs), NameRules::Posix);
    assert_eq!(NameRules::from(Platform::Linux), NameRules::Posix);
    assert_eq!(
        convert_name("May?.txt", NameRules::Windows).name,
        convert_name("May?.txt", Platform::Windows).name
    );
}

fn write(dest: &Destination, path: &str, bytes: &[u8]) -> Result<String, GateError> {
    let mut file = dest.create_file(&IncomingPath::parse(path).unwrap(), bytes.len() as u64)?;
    file.write_all(bytes).unwrap();
    Ok(file.finish()?.final_path)
}

/// On a FAT drive a Mac or Linux laptop would otherwise write `?` and `:` as they are, which
/// the drive refuses (Linux) or Windows cannot open later. The drive's rules apply, whatever the
/// system.
#[test]
fn a_fat_drive_gets_windows_names_on_every_system() {
    let root = tempfile::tempdir().unwrap();
    let dest = Destination::open(root.path())
        .unwrap()
        .with_profile(DriveProfile::of(DriveKind::Fat));
    let landed = write(&dest, "Notes: May/May?.txt", b"x").unwrap();
    assert_eq!(landed, "Notes\u{FF1A} May/May\u{FF1F}.txt");
    assert_eq!(std::fs::read(root.path().join(&landed)).unwrap(), b"x");
    assert_eq!(
        dest.folder_of(&IncomingPath::parse("a:b/c.txt").unwrap()),
        "a\u{FF1A}b"
    );
    assert_eq!(
        dest.find(&IncomingPath::parse("Notes: May/May?.txt").unwrap()),
        Some((landed, 1))
    );
}

/// On a known POSIX drive on Linux or macOS, names stay as they were sent.
#[cfg(unix)]
#[test]
fn a_posix_drive_keeps_the_system_s_names() {
    let root = tempfile::tempdir().unwrap();
    let dest = Destination::open(root.path())
        .unwrap()
        .with_profile(DriveProfile::of(DriveKind::Ext4));
    assert_eq!(write(&dest, "a:b/c?.txt", b"x").unwrap(), "a:b/c?.txt");
}

/// The drive this test machine's temporary folder is on is read when it is opened, and a
/// file lands there as before.
#[test]
fn the_drive_type_is_read_when_a_destination_opens() {
    let root = tempfile::tempdir().unwrap();
    let dest = Destination::open(root.path()).unwrap();
    let kind = dest.profile().kind();
    if cfg!(target_os = "macos") {
        assert_eq!(kind, DriveKind::Apfs);
    }
    if cfg!(target_os = "linux") {
        assert!(posix(kind), "{kind:?}");
    }
    assert_eq!(write(&dest, "a.txt", b"x").unwrap(), "a.txt");
}

/// A file bigger than a FAT drive can hold is refused at its start, before anything is made,
/// in plain words, never part-way through.
#[test]
fn a_file_too_big_for_a_fat_drive_is_refused_before_anything_is_made() {
    let root = tempfile::tempdir().unwrap();
    let dest = Destination::open(root.path())
        .unwrap()
        .with_profile(DriveProfile::of(DriveKind::Fat));
    assert_eq!(dest.largest_file(), Some(FAT_LARGEST_FILE));
    let path = IncomingPath::parse("Films/big.mkv").unwrap();
    let Err(e) = dest.create_file(&path, FAT_LARGEST_FILE + 1) else {
        panic!("a file over 4 GB was accepted for FAT32");
    };
    assert!(
        matches!(e, GateError::TooBigForDrive { largest } if largest == FAT_LARGEST_FILE),
        "{e:?}"
    );
    assert!(e.to_string().contains("4 GB"), "{e}");
    assert_eq!(std::fs::read_dir(root.path()).unwrap().count(), 0);
    // Exactly the largest is fine (nothing is written here: it is only started).
    drop(dest.create_file(&path, FAT_LARGEST_FILE).unwrap());
    // exFAT has no such limit.
    let exfat = Destination::open(root.path())
        .unwrap()
        .with_profile(DriveProfile::of(DriveKind::ExFat));
    assert_eq!(exfat.largest_file(), None);
    drop(exfat.create_file(&path, FAT_LARGEST_FILE + 1).unwrap());
}

/// On a drive that tells capital letters apart, a numbered clash of `A.jpg` starts at 2, not
/// after the numbers `a.jpg` used; on one that does not, both spellings share one count, since
/// they are one name there.
#[cfg(unix)]
#[test]
fn clash_numbers_follow_the_drive_s_case_rule() {
    let root = tempfile::tempdir().unwrap();
    let dest = Destination::open(root.path())
        .unwrap()
        .with_profile(DriveProfile::of(DriveKind::Ext4));
    assert_eq!(write(&dest, "a.jpg", b"1").unwrap(), "a.jpg");
    assert_eq!(write(&dest, "a.jpg", b"2").unwrap(), "a (2).jpg");
    assert_eq!(write(&dest, "a.jpg", b"3").unwrap(), "a (3).jpg");
    assert_eq!(write(&dest, "A.jpg", b"4").unwrap(), "A.jpg");
    assert_eq!(write(&dest, "A.jpg", b"5").unwrap(), "A (2).jpg");
}

/// The words a person sees for a name the drive will not take, and for a file too big for it.
#[test]
fn drive_refusals_are_said_plainly() {
    assert_eq!(
        GateError::NameRejected.to_string(),
        "this drive won't accept this file's name"
    );
    assert_eq!(
        GateError::TooBigForDrive {
            largest: FAT_LARGEST_FILE
        }
        .to_string(),
        "this file is too big for this drive, which holds files of up to 4 GB each"
    );
}

/// Linux's FAT driver (`vfat_is_used_badchars`, `vfat_striptail_len`): it refuses control
/// characters and `* ? < > | " : / \`, and strips trailing dots and spaces (a name that is then
/// empty is refused). exFAT and Windows refuse the same set. A name passing this lands under
/// exactly that name.
fn fat_takes(name: &str) -> bool {
    !name.is_empty()
        && name != "."
        && name != ".."
        && name.encode_utf16().count() <= 255
        && !name
            .chars()
            .any(|c| (c as u32) < 0x20 || "*?<>|\":/\\".contains(c))
        && !name.ends_with('.')
        && !name.ends_with(' ')
}

proptest! {
    #[test]
    fn every_name_converted_for_a_fat_drive_is_one_fat_takes(s in "\\PC{1,80}") {
        for system in [Platform::Linux, Platform::MacOs, Platform::Windows] {
            for kind in [DriveKind::Fat, DriveKind::ExFat, DriveKind::Unknown, DriveKind::Network] {
                let rules = DriveProfile::on(kind, system).name_rules();
                let c = convert_name(&s, rules);
                prop_assert!(fat_takes(&c.name), "{kind:?} on {system:?}: {:?} -> {:?}", s, c.name);
                prop_assert!(c.name.len() <= MAX_COMPONENT_BYTES);
            }
        }
    }

    #[test]
    fn names_fat_refuses_are_reported_as_changed(
        stem in "[a-z]{1,8}",
        bad in prop::sample::select(vec!['*', '?', '<', '>', '|', '"', ':', '\\', '\u{1}', '\u{1f}']),
    ) {
        let c = convert_name(&format!("{stem}{bad}.txt"), DriveProfile::of(DriveKind::Fat).name_rules());
        prop_assert!(c.changes.contains(&NameChange::ForbiddenCharacters));
        prop_assert!(fat_takes(&c.name));
    }
}
