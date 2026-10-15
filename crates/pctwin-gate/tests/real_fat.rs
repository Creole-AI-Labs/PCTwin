//! The gate on real FAT and exFAT drives (Task List 1.3, gate follow-ups): drives without hard
//! links, where a finished file gets its name by a move the drive itself refuses if the name is
//! taken, and where names differing only in letter case are the same name. CI makes real
//! drives (a loop-mounted image on Linux, a disk image on a Mac) and lists their folders in
//! PCTWIN_FAT_DIRS, separated by `|`; with PCTWIN_REQUIRE_FAT set, a missing list fails instead
//! of skipping. Elsewhere this file has nothing to test.
// Tests make and remove files to test on; the no-file-changes fence is for the shipped code.
#![allow(clippy::disallowed_methods)]

use std::io::Write;
use std::path::{Path, PathBuf};

use pctwin_gate::{Destination, IncomingPath};

fn drives() -> Vec<PathBuf> {
    match std::env::var("PCTWIN_FAT_DIRS") {
        Ok(list) => list
            .split('|')
            .filter(|s| !s.is_empty())
            .map(PathBuf::from)
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
    for drive in drives() {
        let dir = fresh(&drive, "pctwin-fat-landing");
        // This really is a drive without hard links, so the move fallback is what runs.
        std::fs::write(dir.join("probe"), b"x").unwrap();
        assert!(
            std::fs::hard_link(dir.join("probe"), dir.join("probe2")).is_err(),
            "{} has hard links: not the drive this test is for",
            drive.display()
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
            drive.display()
        );
    }
}

#[test]
fn an_unfinished_file_leaves_nothing_behind() {
    for drive in drives() {
        let dir = fresh(&drive, "pctwin-fat-unfinished");
        let dest = Destination::open(&dir).unwrap();
        {
            let mut file = dest
                .create_file(&IncomingPath::parse("big.bin").unwrap(), 100)
                .unwrap();
            file.write_all(&[0u8; 40]).unwrap();
            // Dropped part way.
        }
        assert!(names_under(&dir).is_empty(), "{:?}", names_under(&dir));
    }
}

#[test]
fn names_fat_cannot_hold_are_changed_and_still_land() {
    for drive in drives() {
        let dir = fresh(&drive, "pctwin-fat-names");
        let dest = Destination::open(&dir).unwrap();
        let landed = write(&dest, "Notes/May?.txt", b"x");
        assert_ne!(landed, "Notes/May?.txt");
        assert_eq!(std::fs::read(dir.join(&landed)).unwrap(), b"x");
    }
}

/// Reports, never asserts: what a Mac drive does with creation times, renames and file identity,
/// through std and rustix (CI job `macos-drive-probe`; fixtures/macos/drive_caps.c covers the
/// rest, such as the hidden flag, which needs chflags and so unsafe code). PCTWIN_FAT_PHASE is
/// `before` (make files and set times) or `after` (the drive was ejected and attached again).
#[cfg(target_os = "macos")]
#[test]
#[ignore = "a probe for CI's macos-drive-probe job; it prints facts and checks nothing"]
fn macos_drive_probe() {
    use std::fs::{File, FileTimes};
    use std::os::macos::fs::FileTimesExt;
    use std::os::unix::fs::MetadataExt;
    use std::time::{Duration, UNIX_EPOCH};

    use rustix::fs::{CWD, RenameFlags, fstatvfs, renameat_with};

    let before = std::env::var("PCTWIN_FAT_PHASE").as_deref() == Ok("before");
    // 2001-02-03 04:05:06.00 and .07 UTC, then 2026-10-15 12:34:57.123456789 UTC.
    let times = [
        (981_173_106, 0),
        (981_173_106, 70_000_000),
        (1_792_067_697, 123_456_789),
    ];
    for drive in drives() {
        let dir = drive.join("pctwin-rust-probe");
        if before {
            let _ = std::fs::remove_dir_all(&dir);
            std::fs::create_dir_all(&dir).unwrap();
        }
        println!(
            "rust: ==== {} ({})",
            drive.display(),
            if before { "before" } else { "after" }
        );
        for (i, &(s, ns)) in times.iter().enumerate() {
            let path = dir.join(format!("crt_{i}"));
            let want = UNIX_EPOCH + Duration::new(s, ns);
            if before {
                let file = File::create(&path).unwrap();
                let set = file.set_times(FileTimes::new().set_created(want));
                println!("rust: set_created crt_{i}: {set:?}");
            }
            let meta = std::fs::metadata(&path).unwrap();
            let got = meta
                .created()
                .map(|t| t.duration_since(UNIX_EPOCH).unwrap());
            let fsid = fstatvfs(File::open(&path).unwrap()).map(|v| v.f_fsid);
            println!(
                "rust: crt_{i} set={s}.{ns:09} got={got:?} exact={} ino={} dev={} fsid={fsid:x?}",
                got.as_ref().is_ok_and(|g| *g == Duration::new(s, ns)),
                meta.ino(),
                meta.dev()
            );
        }
        if before {
            for (flag, name) in [
                (RenameFlags::NOREPLACE, "NOREPLACE"),
                (RenameFlags::EXCHANGE, "EXCHANGE"),
            ] {
                std::fs::write(dir.join("r_a"), b"A").unwrap();
                std::fs::write(dir.join("r_b"), b"B").unwrap();
                let rc = renameat_with(CWD, dir.join("r_a"), CWD, dir.join("r_b"), flag);
                let b = std::fs::read(dir.join("r_b")).unwrap_or_default();
                println!(
                    "rust: renameat_with({name}) onto an existing name: {rc:?}; r_b now {:?}",
                    String::from_utf8_lossy(&b)
                );
            }
            let free = renameat_with(
                CWD,
                dir.join("r_a"),
                CWD,
                dir.join("r_free"),
                RenameFlags::NOREPLACE,
            );
            println!("rust: renameat_with(NOREPLACE) onto a free name: {free:?}");
        }
    }
}
