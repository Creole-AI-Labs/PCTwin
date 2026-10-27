//! The third security review's rescue findings on Linux and macOS: a link on the rescued drive,
//! even one that stays on the drive, is never followed, and a folder is never read as a file.
// Tests make the fake drive's files and links.
#![allow(clippy::disallowed_methods)]
#![cfg(unix)]

use std::ffi::OsStr;
use std::path::Path;

use pctwin_orchestrate::rescue::{DriveIdentity, RescueDrive, scan_rescued_drive};
use pctwin_scan::{AccessFact, AccessReader, UnixMode};

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

fn id() -> DriveIdentity {
    DriveIdentity {
        volume_id: "2f4c-11aa".into(),
        serial: None,
        file_system: "ext4".into(),
        total_bytes: 1,
    }
}

/// A relative link that stays on the drive (Eve's Documents to Bob's) is not followed: Bob's
/// file is never Eve's, and is never read through Eve's folder.
#[test]
fn a_relative_link_on_the_drive_is_not_followed() {
    let drive = tempfile::tempdir().unwrap();
    let r = drive.path();
    touch(&r.join("etc/os-release"), b"ID=debian\n");
    touch(&r.join("home/bob/Documents/bob-private.txt"), b"bob");
    touch(&r.join("home/eve/Desktop/x.txt"), b"x");
    std::os::unix::fs::symlink("../bob/Documents", r.join("home/eve/Documents")).unwrap();
    std::os::unix::fs::symlink(
        "../../home/bob/Documents/bob-private.txt",
        r.join("home/eve/Desktop/y.txt"),
    )
    .unwrap();
    let scan = scan_rescued_drive(r, &id(), &mut NoOwners).unwrap();
    // Eve's linked Documents is said as unreadable, never listed as hers.
    assert!(
        scan.unreadable
            .iter()
            .any(|u| u.ends_with("home/eve/Documents")),
        "{:?}",
        scan.unreadable
    );
    let d = RescueDrive::open(r, &id()).unwrap();
    let through = ["home", "eve", "Documents", "bob-private.txt"].map(OsStr::new);
    assert!(d.open_file(&through).is_err());
    let link = ["home", "eve", "Desktop", "y.txt"].map(OsStr::new);
    assert!(d.open_file(&link).is_err());
    let folder = ["home", "eve", "Desktop"].map(OsStr::new);
    assert!(
        d.open_file(&folder).is_err(),
        "a folder is not a file to read"
    );
    let ok = ["home", "eve", "Desktop", "x.txt"].map(OsStr::new);
    assert!(d.open_file(&ok).is_ok());
}
