//! Read-back before naming (quarantine design, detection): a received file is read back from the
//! drive through a second handle that skips the system's cache, each block checked over its real
//! length against the sender's fingerprints, and only a file that matched gets its real name.
//! Where the cache cannot be skipped the check still runs and says it came from memory.

use pctwin_gate::{
    CheckedFrom, Destination, FaultPlan, Flip, GateError, IncomingFile, IncomingPath, WhyMemory,
};

fn path(s: &str) -> IncomingPath {
    IncomingPath::parse(s).unwrap()
}

fn setup() -> (tempfile::TempDir, Destination) {
    let root = tempfile::tempdir().unwrap();
    let dest = Destination::open(root.path()).unwrap();
    (root, dest)
}

fn with_faults(plan: FaultPlan) -> (tempfile::TempDir, Destination) {
    let (root, dest) = setup();
    (root, dest.with_faults(plan))
}

/// Bytes that differ from block to block and from place to place.
fn content(len: usize) -> Vec<u8> {
    (0..len)
        .map(|i| (i as u8).wrapping_mul(31) ^ ((i / 4096) as u8))
        .collect()
}

/// The sender's fingerprint of each block, the last one over its real length.
fn hashes(data: &[u8], block: usize) -> Vec<[u8; 32]> {
    data.chunks(block)
        .map(|b| *blake3::hash(b).as_bytes())
        .collect()
}

/// Writes `data` block by block, one write per block, as a move does.
fn written<'d>(dest: &'d Destination, name: &str, data: &[u8], block: usize) -> IncomingFile<'d> {
    let mut file = dest.create_file(&path(name), data.len() as u64).unwrap();
    for (i, b) in data.chunks(block).enumerate() {
        file.write_at((i * block) as u64, b).unwrap();
    }
    file
}

fn names(dir: &std::path::Path) -> Vec<String> {
    let mut v: Vec<String> = std::fs::read_dir(dir)
        .unwrap()
        .map(|e| e.unwrap().file_name().to_string_lossy().into_owned())
        .collect();
    v.sort();
    v
}

/// This machine's own drives are read past the cache on Windows and Linux (the test folder is on
/// an ordinary local drive there); CI sets PCTWIN_REQUIRE_READ_BACK_DRIVE where it knows that.
fn expect_drive() -> bool {
    cfg!(windows) || std::env::var_os("PCTWIN_REQUIRE_READ_BACK_DRIVE").is_some()
}

/// Where a file of `len` bytes is read back from on such a drive: on Windows a file under 4 KiB
/// may live inside the drive's own records, which are read through memory, so it never counts as
/// read from the drive.
fn expected_from(len: usize) -> CheckedFrom {
    if cfg!(windows) && len < 4096 {
        CheckedFrom::Memory(WhyMemory::TooSmall)
    } else {
        CheckedFrom::Drive
    }
}

#[test]
fn every_size_reads_back_and_matches() {
    let (root, dest) = setup();
    for block in [512usize, 4096] {
        for len in [
            0usize,
            1,
            511,
            512,
            513,
            4095,
            4096,
            4097,
            3 * 4096 + 7,
            70_000,
        ] {
            let data = content(len);
            let name = format!("f-{block}-{len}.bin");
            let mut file = written(&dest, &name, &data, block);
            let check = file.verify(block as u64, &hashes(&data, block)).unwrap();
            assert!(
                check.passed(),
                "{len} bytes in blocks of {block}: {check:?}"
            );
            assert!(check.bad_blocks.is_empty());
            if expect_drive() {
                assert_eq!(check.from, expected_from(len), "{len} bytes");
            }
            let done = file.finish().unwrap();
            assert_eq!(done.final_path, name);
            assert_eq!(done.checked_from, Some(check.from));
            assert_eq!(std::fs::read(root.path().join(&name)).unwrap(), data);
        }
    }
}

#[test]
fn this_drive_is_read_past_the_cache() {
    let (root, dest) = setup();
    let found = dest.read_back_check();
    if expect_drive() {
        assert_eq!(found, CheckedFrom::Drive);
    }
    // The test leaves nothing behind.
    assert!(names(root.path()).is_empty());
}

#[test]
fn a_block_that_differs_from_the_senders_is_reported_by_number() {
    let (_root, dest) = setup();
    let data = content(5 * 4096 + 100);
    let mut sent = hashes(&data, 4096);
    for bad in [0usize, 2, 5] {
        let mut file = written(&dest, &format!("x{bad}.bin"), &data, 4096);
        let good = sent[bad];
        sent[bad][7] ^= 1;
        let check = file.verify(4096, &sent).unwrap();
        sent[bad] = good;
        assert_eq!(check.bad_blocks, vec![bad as u64]);
        assert!(!check.passed());
    }
}

#[test]
fn a_byte_stored_wrong_once_is_caught_and_never_named() {
    let (root, dest) = with_faults(FaultPlan {
        flip: Some(Flip::OnWriteOnce),
        ..FaultPlan::default()
    });
    let data = content(3 * 4096 + 9);
    let mut file = written(&dest, "a.bin", &data, 4096);
    let check = file.verify(4096, &hashes(&data, 4096)).unwrap();
    assert_eq!(check.bad_blocks, vec![0]);
    // A failed check never lets the file have its name.
    assert!(matches!(file.seal(), Err(GateError::NotReadBack)));
    assert!(names(root.path()).is_empty(), "the damaged copy is removed");
    // Once only: the next file stores fine.
    let mut file = written(&dest, "b.bin", &data, 4096);
    assert!(file.verify(4096, &hashes(&data, 4096)).unwrap().passed());
    assert_eq!(file.finish().unwrap().final_path, "b.bin");
}

#[test]
fn a_drive_that_stores_every_write_wrong_fails_every_block() {
    let (root, dest) = with_faults(FaultPlan {
        flip: Some(Flip::OnEveryWrite),
        ..FaultPlan::default()
    });
    let data = content(4 * 4096 + 1);
    for name in ["a.bin", "b.bin"] {
        let mut file = written(&dest, name, &data, 4096);
        let check = file.verify(4096, &hashes(&data, 4096)).unwrap();
        assert_eq!(check.bad_blocks, vec![0, 1, 2, 3, 4]);
    }
    assert!(names(root.path()).is_empty());
}

#[test]
fn a_byte_wrong_only_when_read_back_is_caught_in_the_last_partial_block() {
    let len = 2 * 4096 + 3;
    let (root, dest) = with_faults(FaultPlan {
        flip: Some(Flip::OnRead { at: len as u64 - 1 }),
        ..FaultPlan::default()
    });
    let data = content(len);
    let mut file = written(&dest, "a.bin", &data, 4096);
    let check = file.verify(4096, &hashes(&data, 4096)).unwrap();
    assert_eq!(check.bad_blocks, vec![2]);
    // What is on the drive is right; only the read-back saw it wrong.
    let back = std::fs::read(root.path().join(file.temp_path())).unwrap();
    assert_eq!(back, data);
}

#[test]
fn no_name_without_a_read_back() {
    let (root, dest) = setup();
    let data = content(5000);
    let file = written(&dest, "a.bin", &data, 4096);
    assert!(matches!(file.seal(), Err(GateError::NotReadBack)));
    let file = written(&dest, "a.bin", &data, 4096);
    assert!(matches!(file.finish(), Err(GateError::NotReadBack)));
    assert!(names(root.path()).is_empty());
}

#[test]
fn a_failed_read_back_does_not_count_after_a_good_one_is_asked_again() {
    // Checked wrong first, then right: only the latest check counts.
    let (_root, dest) = setup();
    let data = content(9000);
    let mut file = written(&dest, "a.bin", &data, 4096);
    let mut wrong = hashes(&data, 4096);
    wrong[1][0] ^= 1;
    assert!(file.verify(4096, &hashes(&data, 4096)).unwrap().passed());
    assert!(!file.verify(4096, &wrong).unwrap().passed());
    assert!(matches!(file.seal(), Err(GateError::NotReadBack)));
}

#[test]
fn a_restart_between_writing_and_checking_still_checks() {
    let (_root, dest) = setup();
    let data = content(10_000);
    let len = data.len() as u64;
    let reopened = |checked_before: bool| {
        let mut file = written(&dest, "d/a.bin", &data, 4096);
        if checked_before {
            assert!(file.verify(4096, &hashes(&data, 4096)).unwrap().passed());
        }
        let temp = file.temp_path();
        file.persist();
        let mut again = dest.reopen_incoming(&path("d/a.bin"), &temp, len).unwrap();
        again.count_arrived(0, len).unwrap();
        again
    };
    // A check before the restart does not carry over: the reopened file is never named unchecked.
    assert!(matches!(reopened(true).seal(), Err(GateError::NotReadBack)));
    let mut again = reopened(false);
    assert!(again.verify(4096, &hashes(&data, 4096)).unwrap().passed());
    assert_eq!(again.finish().unwrap().final_path, "d/a.bin");
}

#[test]
fn the_wrong_number_of_fingerprints_or_no_block_size_is_refused() {
    let (_root, dest) = setup();
    let data = content(9000);
    let mut file = written(&dest, "a.bin", &data, 4096);
    let sent = hashes(&data, 4096);
    assert!(matches!(
        file.verify(4096, &sent[..2]),
        Err(GateError::BadBlockList)
    ));
    let mut more = sent.clone();
    more.push([0; 32]);
    assert!(matches!(
        file.verify(4096, &more),
        Err(GateError::BadBlockList)
    ));
    assert!(matches!(
        file.verify(0, &sent),
        Err(GateError::BadBlockList)
    ));
    // Before every byte has arrived there is nothing to check.
    let mut part = dest.create_file(&path("p.bin"), 10).unwrap();
    part.write_at(0, b"12345").unwrap();
    assert!(matches!(
        part.verify(4096, &hashes(b"1234567890", 4096)),
        Err(GateError::SizeMismatch { .. })
    ));
}

#[test]
fn where_the_cache_cannot_be_skipped_it_still_checks_and_says_so() {
    let (_root, dest) = with_faults(FaultPlan {
        uncached_refused: true,
        ..FaultPlan::default()
    });
    assert_eq!(
        dest.read_back_check(),
        CheckedFrom::Memory(WhyMemory::CannotSkipCache)
    );
    let data = content(4097);
    let mut file = written(&dest, "a.bin", &data, 512);
    let check = file.verify(512, &hashes(&data, 512)).unwrap();
    assert!(check.passed());
    assert_eq!(check.from, CheckedFrom::Memory(WhyMemory::CannotSkipCache));
    let done = file.finish().unwrap();
    assert_eq!(
        done.checked_from,
        Some(CheckedFrom::Memory(WhyMemory::CannotSkipCache))
    );
    // Said plainly, for "more details".
    let said = CheckedFrom::Memory(WhyMemory::CannotSkipCache).to_string();
    assert!(said.contains("memory, not the drive"), "{said}");
}

#[test]
fn from_memory_still_catches_a_damaged_block() {
    let (_root, dest) = with_faults(FaultPlan {
        uncached_refused: true,
        flip: Some(Flip::OnWriteOnce),
        ..FaultPlan::default()
    });
    let data = content(3000);
    let mut file = written(&dest, "a.bin", &data, 1024);
    let check = file.verify(1024, &hashes(&data, 1024)).unwrap();
    assert_eq!(check.bad_blocks, vec![0]);
    assert_eq!(check.from, CheckedFrom::Memory(WhyMemory::CannotSkipCache));
}

#[test]
fn a_finished_file_reads_back_exactly() {
    let (_root, dest) = setup();
    for len in [0usize, 1, 4096, 65_536, 65_537, 200_000] {
        let data = content(len);
        let name = format!("r{len}.bin");
        let mut file = written(&dest, &name, &data, 4096);
        assert!(file.verify(4096, &hashes(&data, 4096)).unwrap().passed());
        let stored = file.finish().unwrap().final_path;
        let mut reader = dest.open_read_back(&stored).unwrap();
        if expect_drive() {
            assert_eq!(reader.from(), expected_from(len), "{len} bytes");
        }
        let mut back = Vec::new();
        std::io::Read::read_to_end(&mut reader, &mut back).unwrap();
        assert_eq!(back, data, "{len} bytes");
    }
}

#[test]
fn a_file_longer_on_the_drive_than_sent_does_not_pass() {
    let (root, dest) = setup();
    let data = content(5000);
    let mut file = written(&dest, "a.bin", &data, 4096);
    let temp = root.path().join(file.temp_path());
    // Another program adds a byte to the end of the temporary file.
    let mut other = std::fs::OpenOptions::new()
        .append(true)
        .open(&temp)
        .unwrap();
    std::io::Write::write_all(&mut other, b"!").unwrap();
    drop(other);
    let check = file.verify(4096, &hashes(&data, 4096)).unwrap();
    assert!(check.too_long);
    assert!(!check.passed());
}

#[test]
fn a_drive_that_tested_fine_but_refuses_later_is_still_checked_and_says_so() {
    let (_root, dest) = setup();
    let tested = dest.read_back_check();
    // The drive's answer was remembered at the start; now it refuses reads past the cache.
    let dest = dest.with_faults(FaultPlan {
        uncached_refused: true,
        ..FaultPlan::default()
    });
    assert_eq!(dest.read_back_check(), tested, "tested once, remembered");
    let data = content(6000);
    let mut file = written(&dest, "a.bin", &data, 4096);
    let check = file.verify(4096, &hashes(&data, 4096)).unwrap();
    assert!(check.passed());
    assert_eq!(check.from, CheckedFrom::Memory(WhyMemory::CannotSkipCache));
}

#[test]
fn a_drive_that_reads_files_through_memory_anyway_never_says_drive() {
    let (_root, dest) = with_faults(FaultPlan {
        served_from_memory: true,
        ..FaultPlan::default()
    });
    let data = content(200_000);
    let mut file = written(&dest, "a.bin", &data, 4096);
    let check = file.verify(4096, &hashes(&data, 4096)).unwrap();
    assert!(check.passed());
    assert_eq!(check.from, CheckedFrom::Memory(WhyMemory::NotShown));
    let done = file.finish().unwrap();
    assert_eq!(
        done.checked_from,
        Some(CheckedFrom::Memory(WhyMemory::NotShown))
    );
}

#[test]
fn a_different_file_put_at_its_place_is_never_checked_in_its_stead() {
    let (root, dest) = setup();
    let data = content(20_000);
    let mut file = written(&dest, "a.bin", &data, 4096);
    let temp = root.path().join(file.temp_path());
    // Another program swaps in a file of the very same bytes under the temporary name.
    let aside = root.path().join("aside.bin");
    std::fs::rename(&temp, &aside).unwrap();
    std::fs::write(&temp, &data).unwrap();
    assert!(file.verify(4096, &hashes(&data, 4096)).is_err());
}

/// The case the first review found: Windows reads a compressed folder's files through memory
/// even when asked not to, so a file there is never said to be read from the drive.
#[cfg(windows)]
#[test]
fn a_file_in_a_compressed_folder_is_said_to_be_checked_from_memory() {
    use std::os::windows::fs::MetadataExt;
    const COMPRESSED: u32 = 0x0000_0800;
    let root = tempfile::tempdir().unwrap();
    let folder = root.path().join("packed");
    std::fs::create_dir(&folder).unwrap();
    let packed = std::process::Command::new("compact")
        .args(["/c", "/q"])
        .arg(&folder)
        .output()
        .is_ok_and(|o| o.status.success());
    let compressed = std::fs::metadata(&folder).unwrap().file_attributes() & COMPRESSED != 0;
    if !(packed && compressed) {
        // A drive that cannot compress (FAT, exFAT, ReFS) has no such case to test.
        eprintln!("this drive cannot compress folders; nothing to test here");
        return;
    }
    let dest = Destination::open(&folder).unwrap();
    let data = content(300_000);
    let mut file = written(&dest, "a.bin", &data, 4096);
    let check = file.verify(4096, &hashes(&data, 4096)).unwrap();
    assert!(check.passed());
    assert_eq!(check.from, CheckedFrom::Memory(WhyMemory::StoredSpecially));
}
