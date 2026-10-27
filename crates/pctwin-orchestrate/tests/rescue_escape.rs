//! The third security review's rescue findings, as regression tests: a rescued drive's scan
//! never leaves the drive through a link, two people whose names differ only in characters that
//! do not survive conversion to text stay two people, and a target on the rescued drive is
//! refused however its path is written.
// Tests make the fake drive's files and links.
#![allow(clippy::disallowed_methods)]
#![cfg(windows)]

use std::path::{Path, PathBuf};

use pctwin_orchestrate::rescue::{
    DriveIdentity, RescueDrive, RescueError, check_not_target, people_on_drive, refuse_same_volume,
    scan_rescued_drive, volume_of,
};
use pctwin_scan::{AccessFact, AccessReader, OtherSystem, UnixMode};

struct NoOwners;
impl AccessReader for NoOwners {
    fn read(&mut self, _p: &Path, _u: Option<UnixMode>) -> AccessFact {
        AccessFact::Unknown { why: "x".into() }
    }
}

fn touch(p: &Path, b: &[u8]) {
    std::fs::create_dir_all(p.parent().unwrap()).unwrap();
    std::fs::write(p, b).unwrap();
}

fn junction(link: &Path, target: &Path) {
    std::fs::create_dir_all(link.parent().unwrap()).unwrap();
    let ok = std::process::Command::new("cmd")
        .args(["/d", "/c", "mklink", "/J"])
        .arg(link.to_string_lossy().replace('/', "\\"))
        .arg(target.to_string_lossy().replace('/', "\\"))
        .output()
        .unwrap();
    assert!(ok.status.success(), "{ok:?}");
}

/// The identity of the drive `root` is on, with its real serial number (as the system's volume
/// list would give it).
fn id_of(root: &Path) -> DriveIdentity {
    DriveIdentity {
        volume_id: r"\\?\Volume{1}\".into(),
        serial: Some(volume_of(root).unwrap().to_string()),
        file_system: "NTFS".into(),
        total_bytes: 1,
    }
}

/// A hostile drive whose person's Documents folder, and another person's whole home folder, are
/// links to a folder outside the drive (standing in for the new laptop's own folders).
#[test]
fn the_scan_never_leaves_the_drive_through_a_link() {
    let drive = tempfile::tempdir().unwrap();
    let outside = tempfile::tempdir().unwrap();
    touch(&outside.path().join("new-laptop-secret.txt"), b"secret");
    let r = drive.path();
    touch(&r.join("Windows/System32/config/SYSTEM"), b"");
    touch(&r.join("Users/Eve/Desktop/x.txt"), b"x");
    junction(&r.join("Users/Eve/Documents"), outside.path());
    junction(&r.join("Users/Mallory"), outside.path());

    let drive = RescueDrive::open(r, &id_of(r)).unwrap();
    let people = people_on_drive(&drive, OtherSystem::Windows, &mut NoOwners);
    assert!(
        people.iter().all(|p| p.label != "Mallory"),
        "a linked home folder is not a person on this drive"
    );
    let scan = scan_rescued_drive(r, &id_of(r), &mut NoOwners).unwrap();
    let drive_real = std::fs::canonicalize(r).unwrap();
    for item in &scan.record.items {
        let src = scan.source_of(item).unwrap();
        let real = std::fs::canonicalize(&src).unwrap();
        assert!(
            real.starts_with(&drive_real),
            "escaped: {src:?} is {real:?}"
        );
    }
    assert!(
        scan.record
            .items
            .iter()
            .all(|i| !format!("{:?}", i.path).contains("new-laptop-secret")),
        "nothing outside the drive is even listed"
    );
    // The linked folder is said as unreadable, not hidden.
    assert!(
        scan.unreadable
            .iter()
            .any(|u| u.ends_with(Path::new("Eve").join("Documents"))),
        "{:?}",
        scan.unreadable
    );
    // Reading goes through the drive's handles: a file of the plan opens, and the same file
    // named through the link does not, nor does anything off the drive.
    let x = scan
        .record
        .items
        .iter()
        .find(|i| scan.source_of(i).is_some_and(|p| p.ends_with("x.txt")))
        .unwrap();
    let f = scan.open(&scan.source_of(x).unwrap()).unwrap();
    assert_eq!(std::io::read_to_string(f).unwrap(), "x");
    let eve = r.join("Users").join("Eve");
    let through_link = eve.join("Documents").join("new-laptop-secret.txt");
    assert!(scan.open(&through_link).is_err());
    assert!(
        scan.open(&outside.path().join("new-laptop-secret.txt"))
            .is_err()
    );
    let dotdot = eve.join("..").join("Eve").join("Desktop").join("x.txt");
    assert!(scan.open(&dotdot).is_err());
    // A folder is not a file to read.
    assert!(scan.open(&eve.join("Desktop")).is_err());
}

/// A file linked in place of a person's file is not read either (when this account may make a
/// file link at all).
#[test]
fn a_file_link_is_not_read() {
    let drive = tempfile::tempdir().unwrap();
    let outside = tempfile::tempdir().unwrap();
    let r = drive.path();
    touch(&r.join("Windows/System32/config/SYSTEM"), b"");
    touch(&r.join("Users/Eve/Desktop/x.txt"), b"x");
    let secret = outside.path().join("secret.txt");
    touch(&secret, b"secret");
    let linked = r.join("Users").join("Eve").join("Desktop").join("y.txt");
    if std::os::windows::fs::symlink_file(&secret, &linked).is_ok() {
        let d = RescueDrive::open(r, &id_of(r)).unwrap();
        let names = ["Users", "Eve", "Desktop", "y.txt"].map(std::ffi::OsStr::new);
        assert!(d.open_file(&names).is_err());
        let names = ["Users", "Eve", "Desktop", "x.txt"].map(std::ffi::OsStr::new);
        assert!(d.open_file(&names).is_ok());
    }
}

/// A folder that is not the drive the identity names is refused, and so is an identity with no
/// serial number.
#[test]
fn only_the_named_drive_is_read() {
    let drive = tempfile::tempdir().unwrap();
    let r = drive.path();
    touch(&r.join("Windows/System32/config/SYSTEM"), b"");
    let mut other = id_of(r);
    other.serial = Some((volume_of(r).unwrap() ^ 1).to_string());
    assert!(matches!(
        RescueDrive::open(r, &other),
        Err(RescueError::NotTheDrive)
    ));
    other.serial = None;
    assert!(matches!(
        scan_rescued_drive(r, &other, &mut NoOwners),
        Err(RescueError::NotTheDrive)
    ));
    // The serial as `vol` prints it is the same drive.
    let n = volume_of(r).unwrap();
    other.serial = Some(format!("{:04X}-{:04X}", (n >> 16) & 0xFFFF, n & 0xFFFF));
    assert!(RescueDrive::open(r, &other).is_ok());
}

/// Two folder-only people whose names differ only in unpaired UTF-16 halves stay two people.
#[test]
fn two_people_never_merge_through_lossy_names() {
    use std::ffi::OsString;
    use std::os::windows::ffi::OsStringExt;
    let drive = tempfile::tempdir().unwrap();
    let r = drive.path();
    touch(&r.join("Windows/System32/config/SYSTEM"), b"");
    let a = OsString::from_wide(&[0x41, 0xD800]);
    let b = OsString::from_wide(&[0x41, 0xD801]);
    touch(&r.join("Users").join(&a).join("Documents/a.txt"), b"a");
    touch(&r.join("Users").join(&b).join("Documents/b.txt"), b"b");
    let d = RescueDrive::open(r, &id_of(r)).unwrap();
    let people = people_on_drive(&d, OtherSystem::Windows, &mut NoOwners);
    assert_eq!(people.len(), 2);
    assert_ne!(people[0].account_id(), people[1].account_id());
    assert_ne!(people[0].name, people[1].name);
    // Their things stay apart in the plan.
    let scan = scan_rescued_drive(r, &id_of(r), &mut NoOwners).unwrap();
    let owners: std::collections::BTreeSet<_> = scan
        .record
        .items
        .iter()
        .map(|i| format!("{:?}", i.owner))
        .collect();
    assert_eq!(owners.len(), 2, "{owners:?}");
}

/// The volume ID of drive C:, as Windows lists it.
fn c_volume_guid() -> String {
    let out = std::process::Command::new("mountvol")
        .args([r"C:\", "/L"])
        .output()
        .unwrap();
    String::from_utf8_lossy(&out.stdout).trim().to_owned()
}

/// A target folder on the rescued drive is refused however its path is written.
#[test]
fn a_target_on_the_rescued_drive_is_refused_however_it_is_written() {
    let guid = c_volume_guid();
    assert!(guid.starts_with(r"\\?\Volume{"), "{guid}");
    let drive = tempfile::tempdir().unwrap();
    let inside = drive.path().join("Rescued");
    std::fs::create_dir_all(&inside).unwrap();
    // The drive's root folder, in the test the system drive.
    let rest = inside
        .strip_prefix(r"C:\")
        .expect("the test folder is on C:");
    let rest = rest.to_string_lossy();
    let source = RescueDrive::open(drive.path(), &id_of(drive.path())).unwrap();
    for t in [
        format!(r"C:\{rest}"),
        format!(r"c:/{rest}"),
        format!(r"\\?\C:\{rest}"),
        format!(r"\\.\C:\{rest}"),
        format!(r"\\localhost\C$\{rest}"),
        format!(r"{guid}{rest}"),
        // A folder that does not exist yet is judged by the nearest one that does.
        format!(r"\\?\C:\{rest}\new\deeper"),
    ] {
        assert_eq!(
            check_not_target(&source, &[PathBuf::from(&t)]),
            Err(RescueError::SourceIsTarget),
            "{t}"
        );
    }
    // A folder whose drive cannot be told is refused too, never let through.
    assert_eq!(
        check_not_target(
            &source,
            &[PathBuf::from(r"\\no-such-pctwin-host.invalid\share\x")]
        ),
        Err(RescueError::TargetUnknown)
    );
}

/// The rule itself: every target's volume known, none the rescued drive's.
#[test]
fn the_target_rule() {
    assert_eq!(refuse_same_volume(7, &[Some(8), Some(9)]), Ok(()));
    assert_eq!(refuse_same_volume(7, &[]), Ok(()));
    assert_eq!(
        refuse_same_volume(7, &[Some(8), Some(7)]),
        Err(RescueError::SourceIsTarget)
    );
    assert_eq!(
        refuse_same_volume(7, &[Some(8), None]),
        Err(RescueError::TargetUnknown)
    );
}

/// A link that stays on the drive is still a link: one person's Documents linked to another
/// person's never puts the other's things in the first person's plan, and is never read through.
#[test]
fn a_link_inside_the_drive_never_mixes_two_people() {
    let drive = tempfile::tempdir().unwrap();
    let r = drive.path();
    touch(&r.join("Windows/System32/config/SYSTEM"), b"");
    touch(&r.join("Users/Bob/Documents/bob-private.txt"), b"bob");
    touch(&r.join("Users/Eve/Desktop/x.txt"), b"x");
    junction(
        &r.join("Users/Eve/Documents"),
        &r.join("Users").join("Bob").join("Documents"),
    );
    let scan = scan_rescued_drive(r, &id_of(r), &mut NoOwners).unwrap();
    let eve = scan
        .people
        .iter()
        .find(|p| p.label == "Eve")
        .unwrap()
        .account_id();
    for item in &scan.record.items {
        if format!("{:?}", item.path).contains("bob-private") {
            assert_ne!(
                item.owner,
                pctwin_record::Owner::Person {
                    account_id: eve.clone()
                },
                "Bob's file listed as Eve's"
            );
        }
    }
    let d = scan.drive();
    let names = ["Users", "Eve", "Documents", "bob-private.txt"].map(std::ffi::OsStr::new);
    assert!(d.open_file(&names).is_err());
    let names = ["Users", "Bob", "Documents", "bob-private.txt"].map(std::ffi::OsStr::new);
    assert!(d.open_file(&names).is_ok());
}

/// Every part is one name: never "..", ".", a separator or a drive.
#[test]
fn every_part_is_one_name() {
    let drive = tempfile::tempdir().unwrap();
    let r = drive.path();
    touch(&r.join("Users/Eve/Desktop/x.txt"), b"x");
    let d = RescueDrive::open(r, &id_of(r)).unwrap();
    let ok = ["Users", "Eve", "Desktop", "x.txt"].map(std::ffi::OsStr::new);
    assert!(d.open_file(&ok).is_ok());
    for bad in [
        &["Users", "..", "Users", "Eve", "Desktop", "x.txt"][..],
        &["Users", ".", "Eve", "Desktop", "x.txt"][..],
        &["Users", r"Eve\Desktop", "x.txt"][..],
        &["Users", "Eve/Desktop", "x.txt"][..],
        &["Users", "Eve", "Desktop", r"C:\x.txt"][..],
        &[][..],
    ] {
        let names: Vec<&std::ffi::OsStr> = bad.iter().map(std::ffi::OsStr::new).collect();
        assert!(d.open_file(&names).is_err(), "{bad:?}");
    }
}

/// Every folder at the top of a person's home is read, not only the usual six; apps' own data
/// and account state are counted but never moved; a folder that leads off the drive is said as
/// unreadable.
#[test]
fn every_folder_in_a_home_is_read() {
    let drive = tempfile::tempdir().unwrap();
    let outside = tempfile::tempdir().unwrap();
    let r = drive.path();
    touch(&r.join("Windows/System32/config/SYSTEM"), b"");
    touch(&r.join("Users/Eve/Projects/a.txt"), b"project");
    touch(&r.join("Users/Eve/Saved Games/a.txt"), b"game");
    touch(&r.join("Users/Eve/Documents/thesis.docx"), b"thesis");
    touch(
        &r.join("Users/Eve/AppData/Local/Microsoft/Credentials/blob"),
        b"key",
    );
    touch(&r.join("Users/Eve/.ssh/id_ed25519"), b"key");
    // An app's own data, which is neither hidden nor a known place: counted, not moved.
    touch(
        &r.join("Users/Eve/AppData/Roaming/SomeApp/settings.ini"),
        b"x=1",
    );
    // A credential store in the person's own folder: never moved.
    touch(&r.join("Users/Eve/Documents/Cookies"), b"db");
    touch(&r.join("Users/Eve/notes.txt"), b"top-level file");
    junction(&r.join("Users/Eve/Elsewhere"), outside.path());
    let scan = scan_rescued_drive(r, &id_of(r), &mut NoOwners).unwrap();
    let find = |want: &[&str]| {
        scan.record
            .items
            .iter()
            .find(|i| {
                let parts: Vec<String> = i
                    .path
                    .parts()
                    .iter()
                    .map(|p| p.display().to_string())
                    .collect();
                parts == want
            })
            .unwrap_or_else(|| panic!("{want:?} not in the plan"))
    };
    let project = find(&["Projects", "a.txt"]);
    let game = find(&["Saved Games", "a.txt"]);
    assert_eq!(project.inclusion, pctwin_record::Inclusion::Included);
    assert_eq!(game.inclusion, pctwin_record::Inclusion::Included);
    assert_ne!(project.id, game.id, "two folders' files never share an ID");
    let read = std::io::read_to_string(scan.open(&scan.source_of(project).unwrap()).unwrap());
    assert_eq!(read.unwrap(), "project");
    assert_eq!(
        find(&["thesis.docx"]).inclusion,
        pctwin_record::Inclusion::Included
    );
    for kept in [
        &["AppData", "Local", "Microsoft", "Credentials", "blob"][..],
        &[".ssh", "id_ed25519"][..],
        &["AppData", "Roaming", "SomeApp", "settings.ini"][..],
        &["Cookies"][..],
    ] {
        assert!(
            matches!(
                find(kept).inclusion,
                pctwin_record::Inclusion::LeftOut { .. }
            ),
            "{kept:?}"
        );
    }
    assert!(
        scan.unreadable
            .iter()
            .any(|u| u.ends_with(Path::new("Eve").join("Elsewhere"))),
        "{:?}",
        scan.unreadable
    );
}
