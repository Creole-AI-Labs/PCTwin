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
    dest.check_drive().unwrap();
    assert_eq!(
        names(root.path()),
        [
            ".pctwin-probe-0123abcd-c",
            ".pctwin-probe-5-a",
            ".pctwin-probe-99-a"
        ]
    );
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
