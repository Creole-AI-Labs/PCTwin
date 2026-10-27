//! Dead-laptop rescue, read only (Task List 2.6; acceptance T26-017 to T26-021). The drive is a
//! folder laid out as a dead laptop's drive (no administrator rights are needed to make one);
//! the drive's identity is given, as the system would report it.
// Tests make the fake drive's files.
#![allow(clippy::disallowed_methods)]

mod common;

use std::io;
use std::path::{Path, PathBuf};

use pctwin_orchestrate::rescue::{
    DriveIdentity, Lock, PersonIdentity, ReadOnlyLevel, Reinserted, RescueDrive, RescueError,
    check_not_target, identity_from_findmnt, identity_from_win32_volume, lock_from_error,
    people_on_drive, refuse_same_volume, reidentify, scan_rescued_drive, volume_of,
};
use pctwin_record::{FolderRole, Inclusion, Owner};
use pctwin_scan::{AccessFact, AccessReader, OtherSystem, ReadPlan, UnixMode};

fn touch(p: &Path, bytes: &[u8]) {
    std::fs::create_dir_all(p.parent().unwrap()).unwrap();
    std::fs::write(p, bytes).unwrap();
}

/// A dead Windows laptop's drive: Ada and Tunde, the Public and Default folders, and a profile
/// folder with no hive (a half-removed account).
fn windows_drive() -> tempfile::TempDir {
    let d = tempfile::tempdir().unwrap();
    let r = d.path();
    touch(&r.join("Windows/System32/config/SYSTEM"), b"");
    touch(&r.join("Users/Ada/NTUSER.DAT"), b"hive");
    touch(&r.join("Users/Ada/Documents/thesis.docx"), b"chapter one");
    touch(&r.join("Users/Ada/Pictures/2019/beach.jpg"), b"jpg");
    touch(&r.join("Users/Ada/Desktop/notes.txt"), b"n");
    touch(&r.join("Users/Tunde/NTUSER.DAT"), b"hive");
    touch(&r.join("Users/Tunde/Music/song.mp3"), b"mp3");
    touch(&r.join("Users/Public/Documents/shared.txt"), b"s");
    touch(&r.join("Users/Default/NTUSER.DAT"), b"hive");
    touch(&r.join("Users/Leftover/Documents/old.txt"), b"o");
    touch(&r.join("Users/.hidden/NTUSER.DAT"), b"hive");
    // Lower case: the listing's own order (ignoring case) is not the order PCTwin keeps.
    touch(&r.join("Users/bisi/Documents/b.txt"), b"b");
    d
}

/// Answers each hive's owner as the drive records it.
struct Owners(Vec<(&'static str, &'static str)>);

impl AccessReader for Owners {
    fn read(&mut self, path: &Path, _unix: Option<UnixMode>) -> AccessFact {
        let folder = path.parent().and_then(|p| p.file_name()).unwrap();
        match self.0.iter().find(|(f, _)| folder == *f) {
            Some((_, sid)) => AccessFact::Known(pctwin_scan::Access {
                everyone: false,
                people: Default::default(),
                owner: Some(sid.to_string()),
            }),
            None => AccessFact::Unknown {
                why: "not readable".into(),
            },
        }
    }
}

fn drive_id(serial: &str) -> DriveIdentity {
    DriveIdentity {
        volume_id: r"\\?\Volume{0f1e2d3c-0000-0000-0000-100000000000}\".into(),
        serial: Some(serial.into()),
        file_system: "NTFS".into(),
        total_bytes: 256_000_000_000,
    }
}

/// The identity of the drive `root` is on, with its real serial number, as the system's volume
/// list would give it.
fn real_id(root: &Path) -> DriveIdentity {
    drive_id(&volume_of(root).unwrap().to_string())
}

/// The drive at `root`, opened.
fn opened(root: &Path) -> RescueDrive {
    RescueDrive::open(root, &real_id(root)).unwrap()
}

/// The people on a dead laptop's drive are found from the drive itself: each by the account ID
/// that owns their settings file, never by folder name; system folders are not people.
#[test]
fn people_are_found_from_the_drive_itself() {
    let d = windows_drive();
    let mut owners = Owners(vec![
        ("Ada", "S-1-5-21-1-2-3-1001"),
        ("Tunde", "S-1-5-21-1-2-3-1002"),
        ("Default", "S-1-5-18"),
        // Leftover has no settings file, so whatever its folder says is not read.
        ("Leftover", "S-1-5-21-1-2-3-1003"),
        (".hidden", "S-1-5-21-1-2-3-1004"),
    ]);
    let people = people_on_drive(&opened(d.path()), OtherSystem::Windows, &mut owners);
    let found: Vec<(&str, &PersonIdentity)> = people
        .iter()
        .map(|p| (p.label.as_str(), &p.identity))
        .collect();
    assert_eq!(
        found,
        [
            (
                "Ada",
                &PersonIdentity::Account("S-1-5-21-1-2-3-1001".into())
            ),
            ("Leftover", &PersonIdentity::FolderOnly),
            (
                "Tunde",
                &PersonIdentity::Account("S-1-5-21-1-2-3-1002".into())
            ),
            ("bisi", &PersonIdentity::FolderOnly),
        ]
    );
    // A person whose account cannot be read is still listed, keyed by their folder on this drive
    // only (never matched by name to anyone else).
    // Its exact name ("Leftover", unit by unit) and which folder it is.
    let exact = if cfg!(windows) {
        "folder:004c006500660074006f007600650072:"
    } else {
        "folder:4c6566746f766572:"
    };
    assert!(
        people[1].account_id().starts_with(exact),
        "{}",
        people[1].account_id()
    );
    assert_eq!(people[1].name, "Leftover");
    assert_eq!(people[0].account_id(), "S-1-5-21-1-2-3-1001");
    assert_eq!(people[0].home, d.path().join("Users").join("Ada"));
    // A hive owned by a system account says nothing about whose it is.
    let mut odd = Owners(vec![("Ada", "S-1-5-18"), ("Tunde", "BA")]);
    let people = people_on_drive(&opened(d.path()), OtherSystem::Windows, &mut odd);
    assert!(
        people
            .iter()
            .all(|p| p.identity == PersonIdentity::FolderOnly)
    );
}

/// The rescue scan: every person's folders become items under the drive's own laptop ID, read
/// with the careful plan (most important first, each file once), the person's documents and
/// photos ordered first; nothing on the drive changes.
#[test]
fn a_rescued_drive_becomes_an_ordinary_plan() {
    let d = windows_drive();
    let before = snapshot(d.path());
    let mut owners = Owners(vec![
        ("Ada", "S-1-5-21-1-2-3-1001"),
        ("Tunde", "S-1-5-21-1-2-3-1002"),
    ]);
    let id = real_id(d.path());
    let scan = scan_rescued_drive(d.path(), &id, &mut owners).unwrap();
    assert_eq!(scan.record.source_laptop, id.laptop_id());
    assert_eq!(scan.system, OtherSystem::Windows);
    assert_eq!(
        scan.read_plan,
        ReadPlan::Careful {
            most_important_first: true,
            read_each_file_once: true,
            stop_after_read_errors: 5
        }
    );
    let names: Vec<String> = scan
        .record
        .items
        .iter()
        .filter(|i| i.kind == pctwin_record::ItemKind::File)
        .map(|i| i.path.parts().last().unwrap().display().to_string())
        .collect();
    for want in [
        "thesis.docx",
        "beach.jpg",
        "notes.txt",
        "song.mp3",
        "old.txt",
    ] {
        assert!(names.contains(&want.to_string()), "{want} in {names:?}");
    }
    // The Public folder's things are everyone's.
    let shared = scan
        .record
        .items
        .iter()
        .find(|i| i.path.parts().last().unwrap().display() == "shared.txt")
        .unwrap();
    assert_eq!(shared.owner, Owner::Shared);
    assert_eq!(shared.place.role, FolderRole::Public);
    let thesis = scan
        .record
        .items
        .iter()
        .find(|i| i.path.parts().last().unwrap().display() == "thesis.docx")
        .unwrap();
    assert_eq!(
        thesis.owner,
        Owner::Person {
            account_id: "S-1-5-21-1-2-3-1001".into()
        }
    );
    assert_eq!(thesis.inclusion, Inclusion::Included);
    // Documents first, then photos, videos, music and the rest: the irreplaceable first.
    let first: Vec<String> = scan
        .record
        .personal
        .iter()
        .map(|id| {
            let i = scan.record.items.iter().find(|i| i.id == *id).unwrap();
            i.path.parts().last().unwrap().display().to_string()
        })
        .collect();
    use pctwin_scan::{Category, category};
    let rank = |c: Category| match c {
        Category::Document => 0,
        Category::Photo => 1,
        Category::Video => 2,
        Category::Music => 3,
        Category::Other => 4,
    };
    let ranks: Vec<u8> = first.iter().map(|n| rank(category(n))).collect();
    assert!(ranks.windows(2).all(|w| w[0] <= w[1]), "{first:?}");
    assert_eq!(first.len(), names.len(), "every file is in the order");
    assert_eq!(rank(category(&first[0])), 0);
    let pos = |n: &str| first.iter().position(|f| f == n).unwrap();
    assert!(pos("thesis.docx") < pos("beach.jpg"));
    assert!(pos("beach.jpg") < pos("song.mp3"));
    assert!(scan.record.validate().is_ok());
    // The source of each item is its file on the drive.
    let src = scan.source_of(thesis).unwrap();
    assert_eq!(std::fs::read(&src).unwrap(), b"chapter one");
    // ... read through the drive's own handles.
    let read = std::io::read_to_string(scan.open(&src).unwrap()).unwrap();
    assert_eq!(read, "chapter one");
    assert_eq!(snapshot(d.path()), before, "nothing on the drive changed");
    // The same drive scanned again gives the same laptop and item IDs.
    let again = scan_rescued_drive(d.path(), &id, &mut owners).unwrap();
    assert_eq!(again.record.items, scan.record.items);
}

/// What the drive holds, with sizes and modified times, to show nothing changed.
fn snapshot(root: &Path) -> Vec<(PathBuf, u64, std::time::SystemTime)> {
    let mut out = Vec::new();
    let mut pending = vec![root.to_path_buf()];
    while let Some(dir) = pending.pop() {
        for e in std::fs::read_dir(&dir).unwrap() {
            let e = e.unwrap();
            // Through the file itself: a folder listing's copy of the details can lag behind.
            let m = std::fs::symlink_metadata(e.path()).unwrap();
            if m.is_dir() {
                pending.push(e.path());
            }
            out.push((e.path(), m.len(), m.modified().unwrap()));
        }
    }
    out.sort();
    out
}

/// A drive with no system on it is not taken for a laptop's drive.
#[test]
fn a_drive_without_a_system_is_refused() {
    let d = tempfile::tempdir().unwrap();
    touch(&d.path().join("photos/a.jpg"), b"x");
    let err = scan_rescued_drive(d.path(), &real_id(d.path()), &mut Owners(vec![])).unwrap_err();
    assert_eq!(err, RescueError::NoSystemFound);
    assert_eq!(
        err.to_string(),
        "This drive does not hold a laptop's system and home folders, so it is not read as a rescue drive. If it is an ordinary USB drive, plug it into the new laptop instead."
    );
}

/// T26-021: a drive taken out and put back is known by its own identity, never by its drive
/// letter: a different drive in the same letter is another drive.
#[test]
fn a_reinserted_drive_is_known_by_its_identity_not_its_letter() {
    let a = drive_id("1234-ABCD");
    assert_eq!(reidentify(&a, &a.clone()), Reinserted::Same);
    let mut other = a.clone();
    other.serial = Some("9999-0000".into());
    assert_eq!(reidentify(&a, &other), Reinserted::Different);
    for change in [
        |d: &mut DriveIdentity| d.volume_id.push('x'),
        |d: &mut DriveIdentity| d.file_system = "exFAT".into(),
        |d: &mut DriveIdentity| d.total_bytes += 1,
        |d: &mut DriveIdentity| d.serial = None,
    ] {
        let mut b = a.clone();
        change(&mut b);
        assert_eq!(reidentify(&a, &b), Reinserted::Different);
        assert_ne!(a.laptop_id(), b.laptop_id());
    }
    assert_eq!(a.laptop_id(), drive_id("1234-ABCD").laptop_id());
    // The system's answer, as PCTwin reads it (no drive letter in it).
    let text =
        "\\\\?\\Volume{0f1e2d3c-0000-0000-0000-100000000000}\\\t1234567890\tNTFS\t256060514304\r\n";
    let id = identity_from_win32_volume(text).unwrap();
    assert_eq!(
        id.volume_id,
        "\\\\?\\Volume{0f1e2d3c-0000-0000-0000-100000000000}\\"
    );
    assert_eq!(id.serial.as_deref(), Some("1234567890"));
    assert_eq!(id.file_system, "NTFS");
    assert_eq!(id.total_bytes, 256_060_514_304);
    assert_eq!(identity_from_win32_volume(""), None);
    assert_eq!(identity_from_win32_volume("a\tb\tc"), None);
    assert_eq!(identity_from_win32_volume("\t1\tNTFS\t5"), None);
    let no_serial = identity_from_win32_volume("V\t\tNTFS\t5").unwrap();
    assert_eq!(no_serial.serial, None);
    let id = identity_from_findmnt("2f4c-11aa ext4 500107862016\n").unwrap();
    assert_eq!(id.volume_id, "2f4c-11aa");
    assert_eq!(id.serial, None);
    assert_eq!(id.file_system, "ext4");
    assert_eq!(id.total_bytes, 500_107_862_016);
    assert_eq!(identity_from_findmnt(" ext4 5\n"), None);
    assert_eq!(identity_from_findmnt("u ext4 x\n"), None);
}

/// A drive chosen both to read from and to write to is refused, by the volume each target is
/// really on (third security review, finding 2).
#[test]
fn a_drive_cannot_be_both_the_source_and_the_target() {
    let d = tempfile::tempdir().unwrap();
    let source = opened(d.path());
    let inside = d.path().join("Rescued");
    std::fs::create_dir_all(&inside).unwrap();
    assert_eq!(
        check_not_target(&source, std::slice::from_ref(&inside)),
        Err(RescueError::SourceIsTarget)
    );
    assert_eq!(check_not_target(&source, &[]), Ok(()));
    let v = source.volume();
    assert_eq!(refuse_same_volume(v, &[Some(v ^ 1)]), Ok(()));
    assert_eq!(
        refuse_same_volume(v, &[Some(v ^ 1), Some(v)]),
        Err(RescueError::SourceIsTarget)
    );
    assert_eq!(
        RescueError::SourceIsTarget.to_string(),
        "This drive is the one being rescued, so nothing can be put on it. Choose a folder on another drive."
    );
    assert_eq!(
        RescueError::TargetUnknown.to_string(),
        "PCTwin could not tell which drive this folder is on, so nothing is put there. Choose a folder on this laptop's own drive."
    );
}

/// T26-017: a drive locked with BitLocker gets the plain way to unlock it with Windows; PCTwin
/// never asks for the key, so it can never log or keep it.
#[test]
fn a_locked_drive_gets_plain_unlock_guidance() {
    let locked = io::Error::from_raw_os_error(0x8031_0000_u32 as i32);
    assert_eq!(lock_from_error(&locked), Some(Lock::BitLocker));
    assert_eq!(lock_from_error(&io::Error::from_raw_os_error(5)), None);
    assert_eq!(lock_from_error(&io::Error::other("x")), None);
    assert_eq!(
        Lock::BitLocker.words(),
        "This drive is locked with BitLocker. Unlock it in Windows first: open it in File Explorer and type its recovery key when Windows asks. The key is often saved in the owner's Microsoft account, at aka.ms/myrecoverykey. PCTwin never asks for the key and never keeps it."
    );
    assert_eq!(
        Lock::AppleChip.words(),
        "A Mac with Apple silicon or a T2 chip keeps its drive locked to that Mac, so taking the drive out does not work. If the Mac still starts in Recovery, use Share Disk with a cable. If it does not start at all, only an Apple repair centre can help."
    );
    assert_eq!(
        Lock::FileVault.words(),
        "This Mac drive is locked with FileVault. Unlock it on a Mac with the owner's login password or recovery key first. PCTwin never asks for either and never keeps them."
    );
}

/// T26-019: PCTwin only reads, but a drive the system has open for writing can still be changed
/// by the system itself (an access time, a journal it replays): only a drive the system mounted
/// read-only is called untouched.
#[test]
fn only_a_read_only_mount_is_called_untouched() {
    assert_eq!(
        ReadOnlyLevel::MountedReadOnly.words(),
        "The drive is connected read-only, so nothing on it can change."
    );
    assert_eq!(
        ReadOnlyLevel::ReadByPctwinOnly.words(),
        "PCTwin only reads this drive. The system itself may still note small details on it, such as when a folder was last opened. For a drive that must not change at all, connect it read-only first."
    );
    assert!(!ReadOnlyLevel::ReadByPctwinOnly.untouched());
    assert!(ReadOnlyLevel::MountedReadOnly.untouched());
    // A folder on this laptop's own (writable) drive is never called untouched.
    let d = tempfile::tempdir().unwrap();
    assert_eq!(
        pctwin_orchestrate::rescue::read_only_level(d.path()),
        ReadOnlyLevel::ReadByPctwinOnly
    );
}

/// On Windows, a real settings file's owner is read through the scan's own reader: the person
/// who made it, as an account ID.
#[cfg(windows)]
#[test]
fn a_real_hive_owner_is_read_as_an_account_id() {
    let d = windows_drive();
    let mut reader = pctwin_scan::system_reader();
    let people = people_on_drive(&opened(d.path()), OtherSystem::Windows, reader.as_mut());
    let ada = people.iter().find(|p| p.label == "Ada").unwrap();
    // This test made the file, so its owner is this account (or the administrators group when
    // run elevated, which says nothing about whose it is).
    match &ada.identity {
        PersonIdentity::Account(sid) => assert!(sid.starts_with("S-1-5-21-"), "{sid}"),
        PersonIdentity::FolderOnly => {}
    }
}

/// T26-020: hardware PCTwin cannot read gets an honest answer: a drive that cannot be read at
/// all says so, with the system's reason; a Mac whose drive is tied to its chip is told plainly.
#[test]
fn unreadable_hardware_is_said_honestly() {
    let d = tempfile::tempdir().unwrap();
    let gone = d.path().join("not-connected");
    match scan_rescued_drive(&gone, &drive_id("1"), &mut Owners(vec![])) {
        Err(RescueError::Unreadable(why)) => assert!(!why.is_empty()),
        other => panic!("{other:?}"),
    }
    assert_eq!(
        RescueError::Locked(Lock::AppleChip).to_string(),
        Lock::AppleChip.words()
    );
}
