//! The fourth fresh-context review's rescue proofs (findings F6 and F5), kept as regression
//! tests: they failed on the code the review read and pass now.
// Tests make the fake drive's files.
#![allow(clippy::disallowed_methods)]

use std::path::Path;

use pctwin_orchestrate::rescue::{
    DriveIdentity, NOT_A_HOME_HERE, NOT_A_PLACE, NOT_LISTED, RescueDrive, home_places,
    people_on_drive_with, shipped_layouts, volume_of,
};
use pctwin_scan::{AccessFact, AccessReader, OtherSystem, UnixMode};

struct NoOwners;
impl AccessReader for NoOwners {
    fn read(&mut self, _p: &Path, _u: Option<UnixMode>) -> AccessFact {
        AccessFact::Unknown { why: "x".into() }
    }
}

fn touch(p: &Path) {
    std::fs::create_dir_all(p.parent().unwrap()).unwrap();
    std::fs::write(p, b"x").unwrap();
}

fn opened(r: &Path) -> RescueDrive {
    let id = DriveIdentity {
        volume_id: "v".into(),
        serial: Some(volume_of(r).unwrap().to_string()),
        file_system: "ext4".into(),
        total_bytes: 1,
    };
    RescueDrive::open(r, &id).unwrap()
}

/// A hostile drive's account list puts one account's home among the system's folders: the
/// folders beside it (`ssh` with the host keys, `systemd`, `docker`, `mysql`) were all read as
/// people. One account is one person at its own home.
#[test]
fn f6_one_account_is_one_person_at_its_own_home() {
    let d = tempfile::tempdir().unwrap();
    let r = d.path();
    touch(&r.join("home/ada/notes.txt"));
    touch(&r.join("etc/ssh/sshd_config"));
    touch(&r.join("etc/systemd/x.conf"));
    touch(&r.join("etc/x/mine.txt"));
    touch(&r.join("var/lib/docker/x"));
    touch(&r.join("var/lib/mysql/x"));
    touch(&r.join("var/lib/app/data"));
    std::fs::write(
        r.join("etc/passwd"),
        "root:x:0:0::/root:/bin/bash\n\
         ada:x:1000:1000::/home/ada:/bin/bash\n\
         evil:x:1001:1001::/etc/x:/bin/bash\n\
         svc:x:1002:1002::/var/lib/app:/bin/bash\n\
         rootish:x:1003:1003::/:/bin/bash\n\
         up:x:1004:1004::/home/../../x:/bin/bash\n\
         gone:x:1005:1005::/etc/whatever:/bin/bash\n\
         daemon:x:1006:1006::/var/lib/mysql:/usr/sbin/nologin\n",
    )
    .unwrap();
    // The drive also claims account 0 is a normal user.
    std::fs::write(r.join("etc/login.defs"), "UID_MIN 0\nUID_MAX 60000\n").unwrap();
    let drive = opened(r);
    let row = shipped_layouts()
        .rows()
        .iter()
        .find(|row| row.homes == [["home"]]);
    // Only the usual place is a folder of homes: no single account makes `etc` or `var/lib` one.
    assert_eq!(
        home_places(&drive, OtherSystem::Linux, row),
        [vec![std::ffi::OsString::from("home")]]
    );
    let found = people_on_drive_with(shipped_layouts(), &drive, OtherSystem::Linux, &mut NoOwners);
    let mut at: Vec<String> = found
        .people
        .iter()
        .map(|p| {
            p.at.iter()
                .map(|n| n.to_string_lossy().into_owned())
                .collect::<Vec<_>>()
                .join("/")
        })
        .collect();
    at.sort();
    assert_eq!(at, ["etc/x", "home/ada", "var/lib/app"]);
    // What could not be a home on this drive is said, never dropped.
    let said = |what: &str, why: &str| {
        found
            .not_read
            .iter()
            .any(|(p, w)| p.ends_with(what) && *w == why)
    };
    assert!(
        said("etc/whatever", NOT_A_HOME_HERE),
        "{:?}",
        found.not_read
    );
    assert!(
        found
            .not_read
            .iter()
            .filter(|(_, w)| *w == NOT_A_PLACE)
            .count()
            == 2,
        "{:?}",
        found.not_read
    );
}

/// Two accounts living in the same folder show the drive keeps homes there: every folder in it
/// is a person's (nothing found is dropped).
#[test]
fn f6_a_folder_holding_more_than_one_account_is_a_folder_of_homes() {
    let d = tempfile::tempdir().unwrap();
    let r = d.path();
    touch(&r.join("srv/people/ada/a.txt"));
    touch(&r.join("srv/people/bo/b.txt"));
    touch(&r.join("srv/people/cy/c.txt"));
    std::fs::create_dir_all(r.join("etc")).unwrap();
    std::fs::write(
        r.join("etc/passwd"),
        "ada:x:1000:1000::/srv/people/ada:/bin/bash\nbo:x:1001:1001::/srv/people/bo:/bin/bash\n",
    )
    .unwrap();
    let drive = opened(r);
    let places = home_places(&drive, OtherSystem::Linux, None);
    assert!(
        places.contains(&vec!["srv".into(), "people".into()]),
        "{places:?}"
    );
    let found = people_on_drive_with(shipped_layouts(), &drive, OtherSystem::Linux, &mut NoOwners);
    let labels: Vec<&str> = found.people.iter().map(|p| p.label.as_str()).collect();
    assert_eq!(labels, ["ada", "bo", "cy"]);
}

/// A folder of homes that cannot be opened, and a home that is a link, are reported (they were a
/// bare `continue`).
#[test]
fn f5_a_home_that_is_a_link_or_a_folder_that_cannot_be_opened_is_reported() {
    let d = tempfile::tempdir().unwrap();
    let r = d.path();
    touch(&r.join("Windows/System32/config/SYSTEM"));
    touch(&r.join("Users/Ada/NTUSER.DAT"));
    let outside = tempfile::tempdir().unwrap();
    touch(&outside.path().join("NTUSER.DAT"));
    let linked = r.join("Users").join("Mallory");
    #[cfg(windows)]
    let made = std::os::windows::fs::symlink_dir(outside.path(), &linked).is_ok()
        || std::process::Command::new("cmd")
            .args(["/c", "mklink", "/J"])
            .arg(&linked)
            .arg(outside.path())
            .output()
            .is_ok_and(|o| o.status.success());
    #[cfg(unix)]
    let made = std::os::unix::fs::symlink(outside.path(), &linked).is_ok();
    if !made {
        return;
    }
    let drive = opened(r);
    let found = people_on_drive_with(
        shipped_layouts(),
        &drive,
        OtherSystem::Windows,
        &mut NoOwners,
    );
    let labels: Vec<&str> = found.people.iter().map(|p| p.label.as_str()).collect();
    assert_eq!(labels, ["Ada"]);
    assert!(
        found
            .not_read
            .iter()
            .any(|(p, w)| p.ends_with("Mallory") && *w == NOT_A_HOME_HERE),
        "{:?}",
        found.not_read
    );
    // A layout row naming a folder of homes that is there but is not a folder (one that is not
    // there at all is not a finding).
    touch(&r.join("NotThere"));
    let rows = pctwin_catalogue::Catalogue::shipped(
        r#"{"format":1,"catalogue":"rescue-layouts","version":1,"rows":[
        {"system":"windows","homes":[["Users"],["NotThere"]]}]}"#,
    );
    let found = people_on_drive_with(&rows, &drive, OtherSystem::Windows, &mut NoOwners);
    assert!(
        found
            .not_read
            .iter()
            .any(|(p, w)| p.ends_with("NotThere") && *w == NOT_LISTED),
        "{:?}",
        found.not_read
    );
}
