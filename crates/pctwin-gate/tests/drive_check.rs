//! Each destination drive is tested once, on PCTwin's own probe files, for a way to give a
//! finished file its name that never replaces anything (security design notes, "Naming a
//! finished file never replaces anything", decided 15 October 2026). The result, never an error
//! number on its own, decides how files are named there.
// Tests make and read files to test on; the no-file-changes fence is for the shipped code.
#![allow(clippy::disallowed_methods)]

use std::io::Write;
use std::path::Path;

mod common;

use pctwin_gate::{
    Approved, Claim, Destination, Destinations, DriveKind, DriveProfile, GateError, IncomingPath,
    Naming,
};

fn names(dir: &Path) -> Vec<String> {
    let mut out: Vec<String> = std::fs::read_dir(dir)
        .unwrap()
        .map(|e| e.unwrap().file_name().to_string_lossy().into_owned())
        .collect();
    out.sort();
    out
}

/// The test machines' own drives (NTFS, ext4, APFS) all have a safe way to name a file at once,
/// and the test leaves nothing behind.
#[test]
fn an_ordinary_drive_names_files_at_once_and_the_test_leaves_nothing() {
    let root = tempfile::tempdir().unwrap();
    std::fs::write(root.path().join("mine.txt"), b"mine").unwrap();
    let dest = Destination::open(root.path()).unwrap();
    let check = dest.check_drive().unwrap();
    assert_eq!(check.naming, Naming::Atomic, "{check:?}");
    assert!(check.hard_links || check.no_replace_move, "{check:?}");
    assert!(!check.replaced_silently, "{check:?}");
    assert!(check.exclusive_create, "{check:?}");
    assert_eq!(names(root.path()), ["mine.txt"]);
    // Tested once: asking again gives the same answer without testing again.
    assert_eq!(dest.check_drive().unwrap(), check);
    assert_eq!(names(root.path()), ["mine.txt"]);
}

/// Probe files left by a crash part-way through a test are removed, but only when they hold the
/// test's own bytes: a file that merely has such a name is someone's, and stays.
#[test]
fn only_probe_files_holding_the_probe_s_own_bytes_are_cleared_away() {
    let root = tempfile::tempdir().unwrap();
    let dest = Destination::open(root.path()).unwrap();
    let (a, b) = pctwin_gate::probe_contents();
    std::fs::write(root.path().join(".pctwin-probe-0123abcd-a"), a).unwrap();
    std::fs::write(root.path().join(".pctwin-probe-0123abcd-b"), b).unwrap();
    // A silent replace leaves the first file's bytes under the second name.
    std::fs::write(root.path().join(".pctwin-probe-77-b"), a).unwrap();
    std::fs::write(
        root.path().join(".pctwin-probe-0123abcd-c"),
        b"not the probe's",
    )
    .unwrap();
    std::fs::write(root.path().join(".pctwin-probe-99-a"), [a, b"x"].concat()).unwrap();
    std::fs::create_dir(root.path().join(".pctwin-probe-5-a")).unwrap();
    // The other tests' probe files, in the same one list: the time test's two and the read-back
    // test's.
    let time = pctwin_gate::time_probe_contents();
    let read_back = pctwin_gate::read_back_probe_contents();
    std::fs::write(root.path().join(".pctwin-probe-0123abcd-t"), time).unwrap();
    std::fs::write(root.path().join(".pctwin-probe-0123abcd-u"), time).unwrap();
    std::fs::write(root.path().join(".pctwin-probe-0123abcd-r"), &read_back).unwrap();
    // A time probe's name holding a drive probe's bytes is not a time probe.
    std::fs::write(root.path().join(".pctwin-probe-66-u"), a).unwrap();
    // A probe file of a test running now (another handle on the drive): never disturbed.
    std::fs::write(root.path().join(".pctwin-probe-77-a"), a).unwrap();
    // Left by a crash: older than any test takes.
    let long_ago = std::time::SystemTime::now() - std::time::Duration::from_secs(3600);
    for leftover in [
        ".pctwin-probe-0123abcd-a",
        ".pctwin-probe-0123abcd-b",
        ".pctwin-probe-77-b",
        ".pctwin-probe-0123abcd-c",
        ".pctwin-probe-99-a",
        ".pctwin-probe-0123abcd-t",
        ".pctwin-probe-0123abcd-u",
        ".pctwin-probe-0123abcd-r",
        ".pctwin-probe-66-u",
    ] {
        std::fs::File::options()
            .write(true)
            .open(root.path().join(leftover))
            .unwrap()
            .set_modified(long_ago)
            .unwrap();
    }
    dest.check_drive().unwrap();
    assert_eq!(
        names(root.path()),
        [
            ".pctwin-probe-0123abcd-c",
            ".pctwin-probe-5-a",
            ".pctwin-probe-66-u",
            ".pctwin-probe-77-a",
            ".pctwin-probe-99-a"
        ]
    );
}

/// The time test (asked when a drive's type cannot be read) clears the same leftovers in its own
/// folder, which the drive test may never visit, by the same rules: only probe files holding a
/// probe's own bytes, only when older than any test takes, never a folder or someone's file.
#[test]
fn the_time_test_clears_leftover_probe_files_by_the_same_rules() {
    let root = tempfile::tempdir().unwrap();
    let dest = Destination::open(root.path()).unwrap();
    let (a, _) = pctwin_gate::probe_contents();
    let time = pctwin_gate::time_probe_contents();
    let read_back = pctwin_gate::read_back_probe_contents();
    std::fs::write(root.path().join(".pctwin-probe-1a-t"), time).unwrap();
    std::fs::write(root.path().join(".pctwin-probe-1a-u"), time).unwrap();
    std::fs::write(root.path().join(".pctwin-probe-1a-a"), a).unwrap();
    std::fs::write(root.path().join(".pctwin-probe-1a-r"), &read_back).unwrap();
    std::fs::write(root.path().join(".pctwin-probe-2b-u"), b"theirs").unwrap();
    std::fs::create_dir(root.path().join(".pctwin-probe-3c-t")).unwrap();
    // Young: a time test running now.
    std::fs::write(root.path().join(".pctwin-probe-4d-u"), time).unwrap();
    let long_ago = std::time::SystemTime::now() - std::time::Duration::from_secs(3600);
    for leftover in [
        ".pctwin-probe-1a-t",
        ".pctwin-probe-1a-u",
        ".pctwin-probe-1a-a",
        ".pctwin-probe-1a-r",
        ".pctwin-probe-2b-u",
    ] {
        std::fs::File::options()
            .write(true)
            .open(root.path().join(leftover))
            .unwrap()
            .set_modified(long_ago)
            .unwrap();
    }
    assert_eq!(dest.time_test_kind(), None);
    assert_eq!(
        names(root.path()),
        [
            ".pctwin-probe-2b-u",
            ".pctwin-probe-3c-t",
            ".pctwin-probe-4d-u"
        ]
    );
}

/// Another program that writes into the time test's probe files while it runs (a sync tool, a
/// virus scanner repairing a file) makes them its own: the time test never removes such a file,
/// since it removes only a regular file holding exactly the probe's own bytes, and never one
/// put in the probe's place, nor a folder.
#[test]
fn a_time_probe_another_program_wrote_into_is_never_removed() {
    let root = tempfile::tempdir().unwrap();
    let dest = Destination::open(root.path()).unwrap();
    let dir = root.path().to_path_buf();
    let mut seen = Vec::new();
    // Just before each probe is removed: the first is written into, the second replaced by a
    // folder.
    let kind = dest.time_test_kind_with(|name| {
        let path = dir.join(name);
        if seen.is_empty() {
            std::fs::write(&path, b"their own now").unwrap();
        } else {
            std::fs::remove_file(&path).unwrap();
            std::fs::create_dir(&path).unwrap();
        }
        seen.push(name.to_string());
    });
    assert_eq!(kind, None);
    assert_eq!(seen.len(), 2, "{seen:?}");
    assert_eq!(std::fs::read(dir.join(&seen[0])).unwrap(), b"their own now");
    assert!(dir.join(&seen[1]).is_dir());
    assert_eq!(names(&dir), {
        let mut both = seen.clone();
        both.sort();
        both
    });
}

fn sealed_file<'d>(dest: &'d Destination, path: &str, bytes: &[u8]) -> pctwin_gate::Sealed<'d> {
    let mut file = dest
        .create_file(&IncomingPath::parse(path).unwrap(), bytes.len() as u64)
        .unwrap();
    file.write_all(bytes).unwrap();
    common::seal(file).unwrap()
}

/// A drive found to have no way at all (not even a name made only if free) is never written
/// with a guess: the file is not named, and comes back sealed and whole.
#[test]
fn a_drive_with_no_safe_way_never_names_a_file() {
    let root = tempfile::tempdir().unwrap();
    let dest = Destination::open(root.path())
        .unwrap()
        .with_profile(DriveProfile::of(DriveKind::Unknown).with_naming(Naming::NoSafeMethod));
    assert_eq!(dest.check_drive().unwrap().naming, Naming::NoSafeMethod);
    let sealed = sealed_file(&dest, "a.txt", b"abc");
    let temp = sealed.temp_path();
    let name = sealed.next_name().unwrap();
    let Claim::Failed(e, back) = sealed.claim_as(&name) else {
        panic!("named on a drive with no safe way");
    };
    assert!(matches!(e, GateError::NoSafeName), "{e:?}");
    assert!(!root.path().join("a.txt").exists());
    assert_eq!(back.temp_path(), temp);
    assert_eq!(std::fs::read(root.path().join(&temp)).unwrap(), b"abc");
}

/// Such a drive is refused when it is chosen, with the reason.
#[test]
fn a_drive_with_no_safe_way_is_refused_when_chosen() {
    let root = tempfile::tempdir().unwrap();
    let mut table = Destinations::new();
    let refused = table.approve_with_profile(
        "usb",
        Approved::ChosenDrive,
        root.path(),
        DriveProfile::of(DriveKind::Unknown).with_naming(Naming::NoSafeMethod),
    );
    assert!(matches!(refused, Err(GateError::NoSafeName)), "{refused:?}");
    assert!(table.get("usb").is_err());
    // An ordinary drive is tested and approved.
    table
        .approve("home", Approved::MyFolders, root.path())
        .unwrap();
    assert_eq!(
        table.get("home").unwrap().check_drive().unwrap().naming,
        Naming::Atomic
    );
    assert!(names(root.path()).is_empty(), "{:?}", names(root.path()));
}

/// A drive with only a no-replace move (no hard links, as FAT and exFAT) names whole files by
/// that move, on every system.
#[test]
fn a_drive_without_hard_links_names_by_a_move_that_never_replaces() {
    let root = tempfile::tempdir().unwrap();
    std::fs::write(root.path().join("a.txt"), b"theirs").unwrap();
    let dest = Destination::open(root.path()).unwrap().with_profile(
        DriveProfile::of(DriveKind::ExFat)
            .with_naming(Naming::Atomic)
            .without_hard_links(),
    );
    let sealed = sealed_file(&dest, "a.txt", b"ours");
    assert_eq!(sealed.claim().unwrap().keep().final_path, "a (2).txt");
    assert_eq!(std::fs::read(root.path().join("a.txt")).unwrap(), b"theirs");
    assert_eq!(
        std::fs::read(root.path().join("a (2).txt")).unwrap(),
        b"ours"
    );
    assert_eq!(names(root.path()), ["a (2).txt", "a.txt"]);
}

/// A link put in place of a time probe is never removed, even one pointing at a file that holds
/// the probe's bytes: only a regular file at the name itself is PCTwin's probe.
#[cfg(unix)]
#[test]
fn a_link_put_in_place_of_a_time_probe_is_never_removed() {
    let root = tempfile::tempdir().unwrap();
    let dest = Destination::open(root.path()).unwrap();
    let dir = root.path().to_path_buf();
    std::fs::write(dir.join("t"), pctwin_gate::time_probe_contents()).unwrap();
    let mut linked = Vec::new();
    dest.time_test_kind_with(|name| {
        let path = dir.join(name);
        std::fs::remove_file(&path).unwrap();
        std::os::unix::fs::symlink("t", &path).unwrap();
        linked.push(name.to_string());
    });
    assert_eq!(linked.len(), 2);
    for name in &linked {
        assert!(
            std::fs::symlink_metadata(dir.join(name))
                .unwrap()
                .file_type()
                .is_symlink(),
            "{name}: the link was removed"
        );
    }
    assert_eq!(
        std::fs::read(dir.join("t")).unwrap(),
        pctwin_gate::time_probe_contents()
    );
}
