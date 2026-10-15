//! The gate on real FAT and exFAT drives (Task List 1.3, gate follow-ups): drives without hard
//! links, where a finished file gets its name by a move the drive itself refuses if the name is
//! taken, and where names differing only in letter case are the same name. CI makes real
//! drives (loop-mounted images and FUSE on Linux, disk images on a Mac, virtual disks on Windows)
//! and lists them in PCTWIN_FAT_DIRS, separated by `|`, each as `kind=folder` (a bare folder is
//! kind "unknown"); with PCTWIN_REQUIRE_FAT set, a missing list fails instead of skipping.
//! Elsewhere this file has nothing to test.
//!
//! CI runs the file twice with the drives unmounted and mounted again in between:
//! PCTWIN_FAT_PHASE=before, then =after. Unset, the phase is "single" (a local run). Tests that
//! need to survive the remount leave notes in the folder named by PCTWIN_FAT_STATE.
// Tests make and remove files to test on; the no-file-changes fence is for the shipped code.
#![allow(clippy::disallowed_methods)]

use std::io::Write;
use std::path::{Path, PathBuf};

use pctwin_gate::{Destination, IncomingPath};

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
enum Phase {
    Before,
    After,
    Single,
}

fn phase() -> Phase {
    match std::env::var("PCTWIN_FAT_PHASE").as_deref() {
        Ok("before") => Phase::Before,
        Ok("after") => Phase::After,
        Ok("") | Err(_) => Phase::Single,
        Ok(other) => panic!("PCTWIN_FAT_PHASE must be before or after, not {other:?}"),
    }
}

/// One real drive: what kind it is (for messages) and its folder.
#[derive(Clone, Debug)]
struct Drive {
    kind: String,
    path: PathBuf,
}

impl Drive {
    fn show(&self) -> String {
        format!("{} drive at {}", self.kind, self.path.display())
    }
}

fn drives() -> Vec<Drive> {
    match std::env::var("PCTWIN_FAT_DIRS") {
        Ok(list) => list
            .split('|')
            .filter(|s| !s.is_empty())
            .map(|entry| match entry.split_once('=') {
                Some((kind, path)) => Drive {
                    kind: kind.to_string(),
                    path: PathBuf::from(path),
                },
                None => Drive {
                    kind: "unknown".to_string(),
                    path: PathBuf::from(entry),
                },
            })
            .collect(),
        Err(_) => {
            assert!(
                std::env::var_os("PCTWIN_REQUIRE_FAT").is_none(),
                "PCTWIN_REQUIRE_FAT is set but PCTWIN_FAT_DIRS is not"
            );
            Vec::new()
        }
    }
}

/// Runs `check` on every drive and fails once, at the end, naming each drive that failed and why:
/// one bad drive must not hide the result for the others.
fn each_drive(check: impl Fn(&Drive)) {
    let mut failures = Vec::new();
    for drive in drives() {
        let run = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| check(&drive)));
        if let Err(cause) = run {
            let why = cause
                .downcast_ref::<String>()
                .cloned()
                .or_else(|| cause.downcast_ref::<&str>().map(|s| (*s).to_string()))
                .unwrap_or_else(|| "failed".to_string());
            failures.push(format!("[{}] {}", drive.kind, why.replace('\n', " | ")));
        }
    }
    assert!(
        failures.is_empty(),
        "failed on {} drive(s):\n{}",
        failures.len(),
        failures.join("\n")
    );
}

fn write(dest: &Destination, p: &str, bytes: &[u8]) -> String {
    let mut file = dest
        .create_file(&IncomingPath::parse(p).unwrap(), bytes.len() as u64)
        .unwrap();
    file.write_all(bytes).unwrap();
    file.finish().unwrap().final_path
}

/// A fresh, empty folder on the drive for one test.
fn fresh(drive: &Path, name: &str) -> PathBuf {
    let dir = drive.join(name);
    let _ = std::fs::remove_dir_all(&dir);
    std::fs::create_dir_all(&dir).unwrap();
    dir
}

/// Every name in `dir` and below, apart from the Mac's own `._` attribute files.
fn names_under(dir: &Path) -> Vec<String> {
    let mut out = Vec::new();
    for e in std::fs::read_dir(dir).unwrap() {
        let e = e.unwrap();
        let name = e.file_name().to_string_lossy().into_owned();
        if name.starts_with("._") {
            continue;
        }
        if e.file_type().unwrap().is_dir() {
            out.extend(
                names_under(&e.path())
                    .into_iter()
                    .map(|n| format!("{name}/{n}")),
            );
        } else {
            out.push(name);
        }
    }
    out.sort();
    out
}

#[test]
fn files_land_whole_with_numbered_clashes_and_nothing_left_behind() {
    if phase() == Phase::After {
        return;
    }
    each_drive(|drive| {
        let dir = fresh(&drive.path, "pctwin-fat-landing");
        // This really is a drive without hard links, so the move fallback is what runs.
        std::fs::write(dir.join("probe"), b"x").unwrap();
        assert!(
            std::fs::hard_link(dir.join("probe"), dir.join("probe2")).is_err(),
            "{} has hard links: not the drive this test is for",
            drive.show()
        );
        std::fs::remove_file(dir.join("probe")).unwrap();

        let dest = Destination::open(&dir).unwrap();
        assert_eq!(write(&dest, "Photos/a.jpg", b"one"), "Photos/a.jpg");
        assert_eq!(write(&dest, "Photos/a.jpg", b"two"), "Photos/a (2).jpg");
        // FAT keeps one name for both spellings: the second is numbered, never written over.
        assert_eq!(write(&dest, "Photos/A.JPG", b"three"), "Photos/A (3).JPG");
        assert_eq!(std::fs::read(dir.join("Photos/a.jpg")).unwrap(), b"one");
        assert_eq!(std::fs::read(dir.join("Photos/a (2).jpg")).unwrap(), b"two");
        assert_eq!(
            std::fs::read(dir.join("Photos/A (3).JPG")).unwrap(),
            b"three"
        );
        assert_eq!(
            names_under(&dir),
            ["Photos/A (3).JPG", "Photos/a (2).jpg", "Photos/a.jpg"],
            "no temporary file left on {}",
            drive.show()
        );
    });
}

#[test]
fn an_unfinished_file_leaves_nothing_behind() {
    if phase() == Phase::After {
        return;
    }
    each_drive(|drive| {
        let dir = fresh(&drive.path, "pctwin-fat-unfinished");
        let dest = Destination::open(&dir).unwrap();
        {
            let mut file = dest
                .create_file(&IncomingPath::parse("big.bin").unwrap(), 100)
                .unwrap();
            file.write_all(&[0u8; 40]).unwrap();
            // Dropped part way.
        }
        assert!(
            names_under(&dir).is_empty(),
            "{}: {:?}",
            drive.show(),
            names_under(&dir)
        );
    });
}

#[test]
fn names_fat_cannot_hold_are_changed_and_still_land() {
    if phase() == Phase::After {
        return;
    }
    each_drive(|drive| {
        let dir = fresh(&drive.path, "pctwin-fat-names");
        let dest = Destination::open(&dir).unwrap();
        let landed = write(&dest, "Notes/May?.txt", b"x");
        assert_ne!(landed, "Notes/May?.txt", "{}", drive.show());
        assert_eq!(
            std::fs::read(dir.join(&landed)).unwrap(),
            b"x",
            "{}",
            drive.show()
        );
    });
}

/// The bytes the remount test writes: more than one cluster, and different at every position.
fn remount_bytes() -> Vec<u8> {
    (0..9000u32)
        .map(|i| (i.wrapping_mul(31) % 251) as u8)
        .collect()
}

/// The contract the later real-drive tests rely on: what the gate finished before the drive was
/// unmounted is on the drive, whole, when it is mounted again at the same path.
#[test]
fn a_finished_file_is_still_there_after_the_drive_is_mounted_again() {
    let phase = phase();
    let state = std::env::var_os("PCTWIN_FAT_STATE").map(PathBuf::from);
    let bytes = remount_bytes();
    each_drive(|drive| {
        let note = state
            .as_ref()
            .map(|s| s.join(format!("remount-{}.txt", drive.kind)));
        let dir = drive.path.join("pctwin-fat-remount");
        if phase != Phase::After {
            let dir = fresh(&drive.path, "pctwin-fat-remount");
            let dest = Destination::open(&dir).unwrap();
            let landed = write(&dest, "Kept/remount.bin", &bytes);
            if let Some(note) = &note {
                std::fs::write(note, &landed).unwrap();
            }
            if phase == Phase::Single {
                assert_eq!(std::fs::read(dir.join(&landed)).unwrap(), bytes);
            }
        } else {
            let note = note.expect("PCTWIN_FAT_STATE must be set in the after phase");
            let landed = std::fs::read_to_string(&note).unwrap_or_else(|e| {
                panic!("{}: no note from the before phase ({e})", drive.show())
            });
            let back = std::fs::read(dir.join(&landed)).unwrap_or_else(|e| {
                panic!("{}: {landed} is gone after the remount ({e})", drive.show())
            });
            assert_eq!(back, bytes, "{}: {landed} came back changed", drive.show());
        }
    });
}
