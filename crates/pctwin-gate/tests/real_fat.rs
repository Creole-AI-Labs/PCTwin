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
//!
//! PCTWIN_FAT_REFUSES lists drive kinds (comma-separated) where the gate must refuse to name a
//! file safely, for now (exFAT on a Mac, until files can be filled in under their names there):
//! on those, each file is refused with nothing under a real name, rather than the test skipped.
// Tests make and remove files to test on; the no-file-changes fence is for the shipped code.
#![allow(clippy::disallowed_methods)]

use std::io::Write;
use std::path::{Path, PathBuf};

use pctwin_gate::{Destination, GateError, IncomingPath};

mod common;

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
    try_write(dest, p, bytes).unwrap()
}

fn try_write(dest: &Destination, p: &str, bytes: &[u8]) -> Result<String, GateError> {
    let mut file = dest.create_file(&IncomingPath::parse(p).unwrap(), bytes.len() as u64)?;
    file.write_all(bytes).unwrap();
    Ok(common::finish(file)?.final_path)
}

/// Whether the gate is to refuse naming files on this drive, for now (PCTWIN_FAT_REFUSES).
fn refuses(drive: &Drive) -> bool {
    std::env::var("PCTWIN_FAT_REFUSES")
        .unwrap_or_default()
        .split(',')
        .any(|k| k == drive.kind)
}

/// On a drive the gate refuses: a file is refused as unsafe to name, and nothing at all is left
/// under a real name (or, once waited for, under PCTwin's own).
fn assert_refused_cleanly(drive: &Drive, dir: &Path) {
    let dest = Destination::open(dir).unwrap();
    let r = try_write(&dest, "Notes/May?.txt", b"x");
    assert!(
        matches!(r, Err(GateError::NoSafeName)),
        "{}: {r:?}",
        drive.show()
    );
    let left = settled_names(dir);
    assert!(left.is_empty(), "{}: {left:?}", drive.show());
}

/// The names under `dir`, once a drive served by a program (FUSE) has let go of files PCTwin
/// closed a moment ago: it shows a removed file that was still open as `.fuse_hidden...` until
/// the last handle on it closes (PCTwin holds each new file for 10 ms). Waits up to 2 seconds.
fn settled_names(dir: &Path) -> Vec<String> {
    for _ in 0..40 {
        let names = names_under(dir);
        if !names.iter().any(|n| n.contains(".fuse_hidden")) {
            return names;
        }
        std::thread::sleep(std::time::Duration::from_millis(50));
    }
    names_under(dir)
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
        if refuses(drive) {
            return assert_refused_cleanly(drive, &dir);
        }

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
            settled_names(&dir),
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
        let left = settled_names(&dir);
        assert!(left.is_empty(), "{}: {left:?}", drive.show());
    });
}

#[test]
fn names_fat_cannot_hold_are_changed_and_still_land() {
    if phase() == Phase::After {
        return;
    }
    each_drive(|drive| {
        let dir = fresh(&drive.path, "pctwin-fat-names");
        if refuses(drive) {
            return assert_refused_cleanly(drive, &dir);
        }
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
        if refuses(drive) {
            if phase != Phase::After {
                assert_refused_cleanly(drive, &fresh(&drive.path, "pctwin-fat-remount"));
            }
            return;
        }
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

/// FAT drives hold files of up to 4 GB less one byte, on every system: a bigger one is refused at
/// its start, before anything is made. exFAT has no such limit. (Windows reads the drive's type
/// from the volume list, for the volume with the folder's own number.)
#[test]
fn a_fat_drive_refuses_a_file_over_4_gb_at_its_start_on_every_system() {
    use pctwin_gate::FAT_LARGEST_FILE;
    if phase() == Phase::After {
        return;
    }
    each_drive(|drive| {
        let dir = fresh(&drive.path, "pctwin-fat-big");
        let dest = Destination::open(&dir).unwrap();
        let fat = !drive.kind.contains("exfat");
        let expected = fat.then_some(FAT_LARGEST_FILE);
        if cfg!(windows) {
            let kind = if fat {
                pctwin_gate::DriveKind::Fat
            } else {
                pctwin_gate::DriveKind::ExFat
            };
            assert_eq!(dest.profile().kind(), kind, "{}", drive.show());
        }
        assert_eq!(dest.largest_file(), expected, "{}", drive.show());
        let big = dest.create_file(
            &IncomingPath::parse("Films/big.mkv").unwrap(),
            FAT_LARGEST_FILE + 1,
        );
        if fat {
            assert!(
                matches!(big, Err(GateError::TooBigForDrive { largest }) if largest == FAT_LARGEST_FILE),
                "{}: {:?}",
                drive.show(),
                big.as_ref().err()
            );
            assert!(settled_names(&dir).is_empty(), "{}", drive.show());
            assert!(
                !dir.join("Films").exists(),
                "{}: a folder was made",
                drive.show()
            );
        } else {
            drop(big.unwrap());
        }
    });
}

/// A drive mounted inside an approved folder gets its own name rules, not the folder's: CI mounts
/// each drive under the runner's own folder (ext4 on Linux, APFS on a Mac), so a destination
/// there writes `?` as it is beside the drive, and as a lookalike inside it. (On Windows every
/// drive takes Windows' rules, so there is nothing to tell apart.)
#[cfg(unix)]
#[test]
fn a_drive_mounted_inside_the_approved_folder_gets_its_own_rules() {
    if phase() == Phase::After {
        return;
    }
    each_drive(|drive| {
        let mount = drive
            .path
            .file_name()
            .unwrap()
            .to_string_lossy()
            .into_owned();
        let parent = drive.path.parent().unwrap();
        let dest = Destination::open(parent).unwrap();
        let outside = format!("pctwin-nested-{}/May?.txt", drive.kind);
        let inside = format!("{mount}/pctwin-fat-nested/May?.txt");
        let _ = std::fs::remove_dir_all(parent.join(format!("pctwin-nested-{}", drive.kind)));
        let _ = std::fs::remove_dir_all(drive.path.join("pctwin-fat-nested"));
        if !refuses(drive) {
            assert_eq!(
                write(&dest, &inside, b"in"),
                format!("{mount}/pctwin-fat-nested/May\u{FF1F}.txt"),
                "{}",
                drive.show()
            );
        }
        assert_eq!(
            dest.folder_of(&IncomingPath::parse(&inside).unwrap()),
            format!("{mount}/pctwin-fat-nested"),
        );
        assert_eq!(write(&dest, &outside, b"out"), outside, "{}", drive.show());
        // A file too big for the drive inside is refused, though the folder's own drive takes it.
        let big = dest.create_file(
            &IncomingPath::parse(&format!("{mount}/big.bin")).unwrap(),
            pctwin_gate::FAT_LARGEST_FILE + 1,
        );
        let fat = !drive.kind.contains("exfat");
        assert_eq!(
            matches!(big, Err(GateError::TooBigForDrive { .. })),
            fat,
            "{}",
            drive.show()
        );
    });
}

/// PCTwin and another program make the same name at the same moment, many times, the way this
/// drive names files: exactly one of them gets it every time, and the other's file is never
/// touched or mixed in. Each round gives one side or the other a different head start (up to a
/// few times what making a file takes on this drive). Both sides winning proves little (the big
/// head starts decide those rounds), so the test fails unless some rounds went against the head
/// start: the side that started later still got the name, which happens only when the two really
/// overlap. The split is printed for the CI log.
#[test]
fn a_name_raced_for_is_never_shared() {
    use pctwin_gate::Claim;
    if phase() == Phase::After {
        return;
    }
    each_drive(|drive| {
        if refuses(drive) {
            return;
        }
        let dir = fresh(&drive.path, "pctwin-fat-race");
        let dest = Destination::open(&dir).unwrap();
        let naming = dest.check_drive().unwrap().naming;
        let ours = b"ours".to_vec();
        // How long making a file takes on this drive, the measure of each head start.
        let step = {
            let started = std::time::Instant::now();
            for i in 0..8 {
                std::fs::write(dir.join(format!("w{i}.tmp")), b"w").unwrap();
            }
            (started.elapsed() / 8).max(std::time::Duration::from_micros(20))
        };
        let wait = |d: std::time::Duration| {
            let until = std::time::Instant::now() + d;
            while std::time::Instant::now() < until {
                std::hint::spin_loop();
            }
        };
        let mut pending = None;
        let (mut won, mut lost, mut upsets) = (0, 0, 0);
        for round in 0..300u32 {
            let name = format!("r{round}.bin");
            let sealed = match pending.take() {
                Some(s) => s,
                None => {
                    let mut f = dest
                        .create_file(&IncomingPath::parse("x.bin").unwrap(), 4)
                        .unwrap();
                    f.write_all(&ours).unwrap();
                    common::seal(f).unwrap()
                }
            };
            let theirs = dir.join(&name);
            let barrier = std::sync::Arc::new(std::sync::Barrier::new(2));
            let b = std::sync::Arc::clone(&barrier);
            // A different head start each round, to one side or the other.
            let head = step * ((round * 7919) % 400) / 100;
            let they_wait = round % 2 == 0;
            let other = std::thread::spawn(move || {
                b.wait();
                if they_wait {
                    let until = std::time::Instant::now() + head;
                    while std::time::Instant::now() < until {
                        std::hint::spin_loop();
                    }
                }
                match std::fs::OpenOptions::new()
                    .write(true)
                    .create_new(true)
                    .open(&theirs)
                {
                    Ok(mut f) => {
                        f.write_all(b"theirs").unwrap();
                        true
                    }
                    Err(e) if e.kind() == std::io::ErrorKind::AlreadyExists => false,
                    Err(e) => panic!("{e}"),
                }
            });
            barrier.wait();
            if !they_wait {
                wait(head);
            }
            let claim = sealed.claim_as(&name);
            let they_won = other.join().unwrap();
            // The side that started later got the name.
            if !head.is_zero() && they_won == they_wait {
                upsets += 1;
            }
            let got = std::fs::read(dir.join(&name)).unwrap();
            match claim {
                Claim::Named(c) => {
                    assert!(!they_won, "{} round {round}: both got it", drive.show());
                    c.keep();
                    assert_eq!(got, ours, "{} round {round}", drive.show());
                    won += 1;
                }
                Claim::Taken(back) => {
                    assert!(they_won, "{} round {round}: nobody got it", drive.show());
                    assert_eq!(got, b"theirs", "{} round {round}", drive.show());
                    pending = Some(back);
                    lost += 1;
                }
                Claim::Failed(e, _) => panic!("{} round {round}: {e}", drive.show()),
            }
        }
        println!(
            "race on {}: names by {naming:?}; PCTwin got the name {won} times, the other program {lost}; {upsets} went against the head start",
            drive.show()
        );
        assert!(
            upsets > 0,
            "{}: the head start decided every round ({won} to {lost}): the two never really overlapped",
            drive.show()
        );
    });
}

/// A drive put in another's place at the same folder, with the very number the first had (Linux
/// gives a freed loop device to the next image): it is judged again, so it gets its own name
/// rules and limits, never the old drive's. CI mounts a vfat drive inside an approved folder,
/// names a file there, then puts ext4 on the same loop device at the same folder
/// (PCTWIN_SWAP_CMD) while the destination stays open.
#[cfg(target_os = "linux")]
#[test]
fn a_drive_put_in_another_s_place_at_the_same_folder_is_judged_again() {
    use std::os::unix::fs::MetadataExt;
    if phase() == Phase::After {
        return;
    }
    let Some(parent) = std::env::var_os("PCTWIN_SWAP_DIR").map(PathBuf::from) else {
        assert!(
            std::env::var_os("PCTWIN_REQUIRE_FAT").is_none(),
            "PCTWIN_SWAP_DIR is not set"
        );
        return;
    };
    let swap = std::env::var("PCTWIN_SWAP_CMD").expect("PCTWIN_SWAP_CMD with PCTWIN_SWAP_DIR");
    let drive = parent.join("drive");
    let number = std::fs::metadata(&drive).unwrap().dev();
    let dest = Destination::open(&parent).unwrap();
    // vfat: Windows' rules.
    assert_eq!(
        write(&dest, "drive/before/May?.txt", b"vfat"),
        "drive/before/May\u{FF1F}.txt"
    );
    let tested = dest.drives_tested();
    let done = std::process::Command::new("sh")
        .args(["-c", &swap])
        .status()
        .unwrap();
    assert!(done.success(), "the swap failed");
    assert_eq!(
        std::fs::metadata(&drive).unwrap().dev(),
        number,
        "the new drive has the old one's number"
    );
    // ext4: the system's own rules, so the drive was judged again.
    assert_eq!(
        write(&dest, "drive/after/May?.txt", b"ext4"),
        "drive/after/May?.txt"
    );
    // And it was tested again before its first file was named, not given the old drive's test.
    assert_eq!(
        dest.drives_tested(),
        tested + 1,
        "the new drive was tested again"
    );
}
