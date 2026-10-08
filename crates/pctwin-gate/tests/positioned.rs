//! Writing a file in sections (Security Design A, "Lanes share one plan"): a big file's sections
//! arrive on different lanes and are written at their place. The gate keeps exactly which bytes
//! have arrived: nothing outside the announced size, nothing written twice, and a file with a hole
//! is never finished. Space for the whole file can be reserved first, so a full disk shows at the
//! start, not at the end.

use std::io::Write;

use pctwin_gate::{Destination, GateError, IncomingPath, MAX_FILE_PARTS};
use proptest::prelude::*;

fn path(s: &str) -> IncomingPath {
    IncomingPath::parse(s).unwrap()
}

fn bytes(n: usize) -> Vec<u8> {
    (0..n).map(|i| (i % 251) as u8).collect()
}

/// Only the finished file itself: no temporary `.pctwin-` file left behind.
fn names(dir: &std::path::Path) -> Vec<String> {
    let mut v: Vec<String> = std::fs::read_dir(dir)
        .unwrap()
        .map(|e| e.unwrap().file_name().to_string_lossy().into_owned())
        .collect();
    v.sort();
    v
}

#[test]
fn sections_written_in_any_order_make_the_whole_file() {
    let root = tempfile::tempdir().unwrap();
    let dest = Destination::open(root.path()).unwrap();
    let data = bytes(1000);
    let mut f = dest.create_file(&path("video.mp4"), 1000).unwrap();
    f.write_at(600, &data[600..]).unwrap();
    f.write_at(0, &data[..300]).unwrap();
    f.write_at(300, &data[300..600]).unwrap();
    assert_eq!(f.written(), 1000);
    let done = f.finish().unwrap();
    assert_eq!(
        std::fs::read(root.path().join(done.final_path)).unwrap(),
        data
    );
    assert_eq!(names(root.path()), ["video.mp4"]);
}

#[test]
fn nothing_is_written_twice() {
    let root = tempfile::tempdir().unwrap();
    let dest = Destination::open(root.path()).unwrap();
    let mut f = dest.create_file(&path("a.bin"), 100).unwrap();
    f.write_at(20, &[1; 30]).unwrap(); // 20..50
    for (offset, len) in [(20, 30), (10, 11), (49, 5), (25, 5), (0, 100), (19, 2)] {
        assert!(
            matches!(f.write_at(offset, &vec![9; len]), Err(GateError::Overlap)),
            "{offset}+{len}"
        );
    }
    assert_eq!(f.written(), 30);
    // Right next to it on both sides is fine.
    f.write_at(0, &[2; 20]).unwrap();
    f.write_at(50, &[3; 50]).unwrap();
    let done = f.finish().unwrap();
    let got = std::fs::read(root.path().join(done.final_path)).unwrap();
    assert_eq!(&got[..20], &[2; 20]);
    assert_eq!(&got[20..50], &[1; 30], "refused writes changed nothing");
    assert_eq!(&got[50..], &[3; 50]);
}

#[test]
fn nothing_is_written_outside_the_announced_size() {
    let root = tempfile::tempdir().unwrap();
    let dest = Destination::open(root.path()).unwrap();
    let mut f = dest.create_file(&path("a.bin"), 100).unwrap();
    for (offset, len) in [
        (90, 11),
        (100, 1),
        (101, 0),
        (u64::MAX, 1),
        (u64::MAX - 2, 5),
    ] {
        assert!(
            matches!(
                f.write_at(offset, &vec![0; len]),
                Err(GateError::OutsideFile)
            ),
            "{offset}+{len}"
        );
    }
    assert_eq!(f.written(), 0);
    // An empty write at the very end is harmless.
    f.write_at(100, &[]).unwrap();
}

#[test]
fn a_file_with_a_hole_is_never_finished_and_leaves_nothing_behind() {
    let root = tempfile::tempdir().unwrap();
    let dest = Destination::open(root.path()).unwrap();
    let mut f = dest.create_file(&path("a.bin"), 100).unwrap();
    f.write_at(0, &[1; 40]).unwrap();
    f.write_at(60, &[1; 40]).unwrap();
    assert!(matches!(
        f.finish(),
        Err(GateError::SizeMismatch {
            announced: 100,
            received: 80
        })
    ));
    assert!(names(root.path()).is_empty());
}

#[test]
fn writing_in_order_still_works_and_mixes_with_sections() {
    let root = tempfile::tempdir().unwrap();
    let dest = Destination::open(root.path()).unwrap();
    let data = bytes(100);
    let mut f = dest.create_file(&path("a.bin"), 100).unwrap();
    f.write_at(70, &data[70..]).unwrap();
    f.write_all(&data[..30]).unwrap();
    f.write_all(&data[30..70]).unwrap();
    // The in-order writer may not run into a section already there.
    assert!(f.write_all(&[0]).is_err());
    let done = f.finish().unwrap();
    assert_eq!(
        std::fs::read(root.path().join(done.final_path)).unwrap(),
        data
    );
}

#[test]
fn scattered_tiny_writes_cannot_fill_memory() {
    let root = tempfile::tempdir().unwrap();
    let dest = Destination::open(root.path()).unwrap();
    let parts = MAX_FILE_PARTS as u64;
    let mut f = dest.create_file(&path("a.bin"), 2 * parts + 10).unwrap();
    // One byte at every other place: each write is a separate part.
    for i in 0..parts {
        f.write_at(2 * i, &[1]).unwrap();
    }
    assert!(matches!(
        f.write_at(2 * parts + 2, &[1]),
        Err(GateError::TooScattered)
    ));
    // Filling a gap joins two parts into one, so it is always allowed.
    f.write_at(1, &[1]).unwrap();
    f.write_at(2 * parts + 2, &[1]).unwrap();
    assert_eq!(f.written(), parts + 2);
}

#[test]
fn parts_that_touch_are_joined_however_they_arrive() {
    let root = tempfile::tempdir().unwrap();
    let dest = Destination::open(root.path()).unwrap();
    let n = 3 * MAX_FILE_PARTS as u64;
    // Forwards, one byte at a time.
    let mut f = dest.create_file(&path("a.bin"), n).unwrap();
    for i in 0..n {
        f.write_at(i, &[1]).unwrap();
    }
    f.finish().unwrap();
    // Backwards, one byte at a time.
    let mut f = dest.create_file(&path("b.bin"), n).unwrap();
    for i in (0..n).rev() {
        f.write_at(i, &[1]).unwrap();
    }
    f.finish().unwrap();
}

#[test]
fn space_for_the_whole_file_can_be_reserved_first() {
    let root = tempfile::tempdir().unwrap();
    let dest = Destination::open(root.path()).unwrap();
    let data = bytes(5000);
    let mut f = dest.create_file(&path("a.bin"), 5000).unwrap();
    // Some drives cannot reserve space (then the copy simply goes ahead); none may refuse it.
    let reserved = f.reserve().unwrap();
    eprintln!("reserved on this drive: {reserved}");
    f.write_at(2500, &data[2500..]).unwrap();
    f.write_at(0, &data[..2500]).unwrap();
    let done = f.finish().unwrap();
    assert_eq!(
        std::fs::read(root.path().join(done.final_path)).unwrap(),
        data
    );
}

#[test]
fn a_file_too_big_for_the_disk_is_refused_at_the_start_and_leaves_nothing() {
    let root = tempfile::tempdir().unwrap();
    let dest = Destination::open(root.path()).unwrap();
    // 1 PiB: more than any test machine has free.
    let mut f = dest.create_file(&path("huge.bin"), 1 << 50).unwrap();
    assert!(!matches!(f.reserve(), Ok(true)));
    drop(f);
    assert!(names(root.path()).is_empty());
}

#[test]
fn an_unfinished_reserved_file_is_removed() {
    let root = tempfile::tempdir().unwrap();
    let dest = Destination::open(root.path()).unwrap();
    let mut f = dest.create_file(&path("a.bin"), 4096).unwrap();
    let _ = f.reserve().unwrap();
    f.write_at(0, &[1; 100]).unwrap();
    drop(f);
    assert!(names(root.path()).is_empty());
}

proptest! {
    /// Any mix of writes: the gate's count is exactly the bytes covered (each counted once), and
    /// the file finishes only when every byte from 0 to the announced size is covered.
    #[test]
    fn the_count_is_exactly_the_bytes_covered(
        size in 0u64..400,
        writes in proptest::collection::vec((0u64..450, 0usize..120), 0..40),
    ) {
        let root = tempfile::tempdir().unwrap();
        let dest = Destination::open(root.path()).unwrap();
        let mut f = dest.create_file(&path("p.bin"), size).unwrap();
        let mut covered = vec![false; size as usize];
        for (offset, len) in writes {
            let end = offset + len as u64;
            let outside = end > size;
            let overlaps = !outside && covered[offset as usize..end as usize].iter().any(|c| *c);
            let r = f.write_at(offset, &vec![7; len]);
            if outside {
                prop_assert!(matches!(r, Err(GateError::OutsideFile)));
            } else if overlaps {
                prop_assert!(matches!(r, Err(GateError::Overlap)));
            } else {
                prop_assert!(r.is_ok());
                for c in &mut covered[offset as usize..end as usize] {
                    *c = true;
                }
            }
            prop_assert_eq!(f.written(), covered.iter().filter(|c| **c).count() as u64);
        }
        let full = covered.iter().all(|c| *c);
        prop_assert_eq!(f.finish().is_ok(), full);
    }
}
