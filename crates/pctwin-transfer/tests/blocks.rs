//! Moving one file in checked blocks (Task List 1.5): block sizes follow the file's size, every
//! block carries its fingerprint and is checked on arrival, compression is used only when it helps,
//! a dropped connection resumes from the exact block, and a file that changes while it is being
//! read is never finished.

use std::io::Write;
use std::path::Path;

use pctwin_gate::{Destination, IncomingPath};
use pctwin_transfer::{
    Assembly, Block, FileSender, MAX_BLOCK, MIN_BLOCK, TransferError, block_size_for,
};

fn send_all(sender: &mut FileSender, assembly: &mut Assembly<'_>) {
    while let Some(block) = sender.next_block().unwrap() {
        let wire = block.encode();
        let back = Block::decode(&wire, sender.header().block_size).unwrap();
        assembly.accept(back).unwrap();
    }
}

fn mixed_content(len: usize) -> Vec<u8> {
    // Half text (compresses well), half pseudo-random (does not).
    let mut out = Vec::with_capacity(len);
    let mut x: u32 = 12345;
    while out.len() < len {
        if (out.len() / 65536) % 2 == 0 {
            out.extend_from_slice(b"the quick brown fox jumps over the lazy dog ");
        } else {
            x ^= x << 13;
            x ^= x >> 17;
            x ^= x << 5;
            out.extend_from_slice(&x.to_le_bytes());
        }
    }
    out.truncate(len);
    out
}

fn source(dir: &Path, name: &str, bytes: &[u8]) -> std::path::PathBuf {
    let p = dir.join(name);
    std::fs::write(&p, bytes).unwrap();
    p
}

#[test]
fn block_sizes_follow_the_file_size() {
    assert_eq!(block_size_for(0), MIN_BLOCK);
    assert_eq!(block_size_for(1_000_000), MIN_BLOCK);
    assert_eq!(block_size_for(250 * 1024 * 1024), MIN_BLOCK);
    // Past about 2,000 blocks the size doubles.
    assert_eq!(block_size_for(256 * 1024 * 1024 + 1), 256 * 1024);
    assert_eq!(block_size_for(1024 * 1024 * 1024), 1024 * 1024);
    assert_eq!(block_size_for(500 * 1024 * 1024 * 1024), MAX_BLOCK);
    for len in [1u64, 5_000_000, 3 << 30, 40 << 30] {
        let size = block_size_for(len);
        assert!(size.is_power_of_two() && (MIN_BLOCK..=MAX_BLOCK).contains(&size));
    }
}

#[test]
fn a_file_arrives_exactly_and_gets_its_name_only_when_complete() {
    let src = tempfile::tempdir().unwrap();
    let dst = tempfile::tempdir().unwrap();
    let bytes = mixed_content(3 * 1024 * 1024 + 17);
    let path = source(src.path(), "report.bin", &bytes);

    let mut sender = FileSender::open(&path, None, true).unwrap();
    let header = sender.header().clone();
    assert_eq!(header.size, bytes.len() as u64);
    let dest = Destination::open(dst.path()).unwrap();
    let mut assembly = Assembly::start(
        &dest,
        &IncomingPath::parse("Documents/report.bin").unwrap(),
        header,
    )
    .unwrap();
    send_all(&mut sender, &mut assembly);
    assert!(!dst.path().join("Documents/report.bin").exists());
    let done = assembly.finish(sender.finish().unwrap()).unwrap();
    assert_eq!(done.final_path, "Documents/report.bin");
    assert_eq!(
        std::fs::read(dst.path().join("Documents/report.bin")).unwrap(),
        bytes
    );
}

#[test]
fn an_empty_file_arrives_too() {
    let src = tempfile::tempdir().unwrap();
    let dst = tempfile::tempdir().unwrap();
    let path = source(src.path(), "empty.txt", b"");
    let mut sender = FileSender::open(&path, None, true).unwrap();
    assert!(sender.next_block().unwrap().is_none());
    let dest = Destination::open(dst.path()).unwrap();
    let assembly = Assembly::start(
        &dest,
        &IncomingPath::parse("empty.txt").unwrap(),
        sender.header().clone(),
    )
    .unwrap();
    assembly.finish(sender.finish().unwrap()).unwrap();
    assert_eq!(std::fs::read(dst.path().join("empty.txt")).unwrap(), b"");
}

#[test]
fn compression_is_used_only_when_it_helps() {
    let src = tempfile::tempdir().unwrap();
    let text = vec![b'a'; 200_000];
    let noise = mixed_content(400_000)[65536..65536 + 60_000].to_vec();
    let t = source(src.path(), "t.txt", &text);
    let n = source(src.path(), "n.bin", &noise);

    let mut s = FileSender::open(&t, None, true).unwrap();
    let b = s.next_block().unwrap().unwrap();
    assert!(b.is_compressed());
    assert!(b.encode().len() < 10_000);

    let mut s = FileSender::open(&n, None, true).unwrap();
    assert!(!s.next_block().unwrap().unwrap().is_compressed());

    // Already-compressed formats (photos, videos, archives) are never tried.
    let mut s = FileSender::open(&t, None, false).unwrap();
    assert!(!s.next_block().unwrap().unwrap().is_compressed());
}

#[test]
fn a_damaged_block_is_refused() {
    let src = tempfile::tempdir().unwrap();
    let path = source(src.path(), "a.bin", &mixed_content(300_000));
    let mut sender = FileSender::open(&path, None, true).unwrap();
    let block_size = sender.header().block_size;
    let mut wire = sender.next_block().unwrap().unwrap().encode();
    let last = wire.len() - 1;
    wire[last] ^= 0xFF;
    assert!(matches!(
        Block::decode(&wire, block_size),
        Err(TransferError::Damaged(_))
    ));
    assert!(matches!(
        Block::decode(&wire[..5], block_size),
        Err(TransferError::Damaged(_))
    ));
}

#[test]
fn a_block_larger_than_announced_is_refused_however_it_is_packed() {
    let src = tempfile::tempdir().unwrap();
    // Highly compressible: a small packed block that would expand to far more than allowed.
    let path = source(src.path(), "zeros.bin", &vec![0u8; 4 * 1024 * 1024]);
    let mut sender = FileSender::open(&path, None, true).unwrap();
    let wire = sender.next_block().unwrap().unwrap().encode();
    // The receiver expects blocks of at most 1 KiB here.
    assert!(matches!(
        Block::decode(&wire, 1024),
        Err(TransferError::Damaged(_))
    ));
}

#[test]
fn blocks_arrive_in_any_order_and_one_sent_twice_is_written_once() {
    let src = tempfile::tempdir().unwrap();
    let dst = tempfile::tempdir().unwrap();
    let bytes = mixed_content(400_000);
    let path = source(src.path(), "a.bin", &bytes);
    let mut sender = FileSender::open(&path, None, true).unwrap();
    let dest = Destination::open(dst.path()).unwrap();
    let mut assembly = Assembly::start(
        &dest,
        &IncomingPath::parse("a.bin").unwrap(),
        sender.header().clone(),
    )
    .unwrap();
    let b0 = sender.next_block().unwrap().unwrap();
    let b1 = sender.next_block().unwrap().unwrap();
    // Sections on different lanes: block 1 may come first.
    assert_eq!(assembly.accept(b1.clone()).unwrap().block, 1);
    assert_eq!(assembly.accept(b0.clone()).unwrap().block, 0);
    // Sent again (it was left off a resume message): confirmed, not written twice.
    assert_eq!(assembly.accept(b0).unwrap().block, 0);
    assert_eq!(assembly.resume_ticket().done.done_count(), 2);
    send_all(&mut sender, &mut assembly);
    assembly.finish(sender.finish().unwrap()).unwrap();
    assert_eq!(std::fs::read(dst.path().join("a.bin")).unwrap(), bytes);
}

#[test]
fn a_block_outside_the_file_is_refused() {
    let src = tempfile::tempdir().unwrap();
    let dst = tempfile::tempdir().unwrap();
    let path = source(src.path(), "a.bin", &mixed_content(400_000));
    let sender = FileSender::open(&path, None, true).unwrap();
    let count = sender.header().block_count;
    let dest = Destination::open(dst.path()).unwrap();
    let mut assembly = Assembly::start(
        &dest,
        &IncomingPath::parse("a.bin").unwrap(),
        sender.header().clone(),
    )
    .unwrap();
    // A correctly fingerprinted block numbered just past the end.
    let data = vec![7u8; 1000];
    let mut wire = vec![1u8, 0];
    wire.extend_from_slice(&count.to_be_bytes());
    wire.extend_from_slice(&1000u32.to_be_bytes());
    wire.extend_from_slice(blake3::hash(&data).as_bytes());
    wire.extend_from_slice(&data);
    let block = Block::decode(&wire, MIN_BLOCK).unwrap();
    assert!(matches!(
        assembly.accept(block),
        Err(TransferError::Damaged(_))
    ));
    assert_eq!(assembly.resume_ticket().done.done_count(), 0);
}

#[test]
fn a_file_missing_blocks_is_not_finished() {
    let src = tempfile::tempdir().unwrap();
    let dst = tempfile::tempdir().unwrap();
    let path = source(src.path(), "a.bin", &mixed_content(400_000));
    let mut sender = FileSender::open(&path, None, true).unwrap();
    let dest = Destination::open(dst.path()).unwrap();
    let mut assembly = Assembly::start(
        &dest,
        &IncomingPath::parse("a.bin").unwrap(),
        sender.header().clone(),
    )
    .unwrap();
    assembly
        .accept(sender.next_block().unwrap().unwrap())
        .unwrap();
    let trailer = {
        while sender.next_block().unwrap().is_some() {}
        sender.finish().unwrap()
    };
    assert!(matches!(
        assembly.finish(trailer),
        Err(TransferError::Incomplete { .. })
    ));
    let left: Vec<_> = std::fs::read_dir(dst.path()).unwrap().collect();
    assert!(left.is_empty(), "{left:?}");
}

#[test]
fn a_dropped_connection_resumes_from_the_exact_block() {
    let src = tempfile::tempdir().unwrap();
    let dst = tempfile::tempdir().unwrap();
    let bytes = mixed_content(1024 * 1024);
    let path = source(src.path(), "big.bin", &bytes);
    let dest = Destination::open(dst.path()).unwrap();
    let mut first = FileSender::open(&path, None, true).unwrap();
    let mut assembly = Assembly::start(
        &dest,
        &IncomingPath::parse("big.bin").unwrap(),
        first.header().clone(),
    )
    .unwrap();
    for _ in 0..3 {
        assembly
            .accept(first.next_block().unwrap().unwrap())
            .unwrap();
    }
    drop(first); // the connection drops

    let ticket = assembly.resume_ticket();
    assert_eq!(ticket.done.done_count(), 3);
    let mut again = FileSender::open(&path, Some(&ticket), true).unwrap();
    let next = again.next_block().unwrap().unwrap();
    assert_eq!(next.index(), 3);
    assembly.accept(next).unwrap();
    send_all(&mut again, &mut assembly);
    assembly.finish(again.finish().unwrap()).unwrap();
    assert_eq!(std::fs::read(dst.path().join("big.bin")).unwrap(), bytes);
}

#[test]
fn a_resume_sends_only_the_blocks_still_missing_wherever_they_are() {
    let src = tempfile::tempdir().unwrap();
    let dst = tempfile::tempdir().unwrap();
    let bytes = mixed_content(2 * 1024 * 1024);
    let path = source(src.path(), "big.bin", &bytes);
    let dest = Destination::open(dst.path()).unwrap();
    let mut first = FileSender::open(&path, None, true).unwrap();
    let count = first.header().block_count;
    assert!(count >= 10);
    let mut assembly = Assembly::start(
        &dest,
        &IncomingPath::parse("big.bin").unwrap(),
        first.header().clone(),
    )
    .unwrap();
    // Two sections were under way: blocks 0 to 2, and 6 and 7.
    let all: Vec<Block> = std::iter::from_fn(|| first.next_block().unwrap()).collect();
    for b in [0, 1, 2, 6, 7] {
        assembly.accept(all[b].clone()).unwrap();
    }
    drop(first);
    let ticket = assembly.resume_ticket();
    let mut again = FileSender::open(&path, Some(&ticket), true).unwrap();
    let mut sent = Vec::new();
    while let Some(b) = again.next_block().unwrap() {
        sent.push(b.index());
        assembly.accept(b).unwrap();
    }
    let expected: Vec<u64> = (3..6).chain(8..count).collect();
    assert_eq!(sent, expected);
    assembly.finish(again.finish().unwrap()).unwrap();
    assert_eq!(std::fs::read(dst.path().join("big.bin")).unwrap(), bytes);
}

#[test]
fn a_resume_ticket_that_does_not_fit_the_file_is_refused_without_crashing() {
    let src = tempfile::tempdir().unwrap();
    let dst = tempfile::tempdir().unwrap();
    let path = source(src.path(), "big.bin", &mixed_content(1024 * 1024));
    let dest = Destination::open(dst.path()).unwrap();
    let first = FileSender::open(&path, None, true).unwrap();
    let assembly = Assembly::start(
        &dest,
        &IncomingPath::parse("big.bin").unwrap(),
        first.header().clone(),
    )
    .unwrap();
    let good = assembly.resume_ticket();
    // The old laptop decides the block size itself; a ticket naming another is refused.
    for block_size in [0, 1, good.block_size / 2, good.block_size * 2, u64::MAX] {
        let mut t = good.clone();
        t.block_size = block_size;
        assert!(
            FileSender::open(&path, Some(&t), true).is_err(),
            "{block_size}"
        );
    }
    // A map for a different number of blocks.
    let mut t = good.clone();
    t.done = pctwin_transfer::BlockMap::new(good.done.count() + 1);
    assert!(FileSender::open(&path, Some(&t), true).is_err());
    assert!(FileSender::open(&path, Some(&good), true).is_ok());
}

#[test]
fn a_file_changed_since_the_drop_starts_again() {
    let src = tempfile::tempdir().unwrap();
    let dst = tempfile::tempdir().unwrap();
    let path = source(src.path(), "big.bin", &mixed_content(1024 * 1024));
    let dest = Destination::open(dst.path()).unwrap();
    let mut first = FileSender::open(&path, None, true).unwrap();
    let mut assembly = Assembly::start(
        &dest,
        &IncomingPath::parse("big.bin").unwrap(),
        first.header().clone(),
    )
    .unwrap();
    assembly
        .accept(first.next_block().unwrap().unwrap())
        .unwrap();
    drop(first);
    std::fs::OpenOptions::new()
        .append(true)
        .open(&path)
        .unwrap()
        .write_all(b"more")
        .unwrap();
    assert!(matches!(
        FileSender::open(&path, Some(&assembly.resume_ticket()), true),
        Err(TransferError::ChangedSince)
    ));
}

#[test]
fn a_file_changed_while_being_read_is_never_finished() {
    let src = tempfile::tempdir().unwrap();
    let dst = tempfile::tempdir().unwrap();
    let path = source(src.path(), "live.bin", &mixed_content(600_000));
    let mut sender = FileSender::open(&path, None, true).unwrap();
    let dest = Destination::open(dst.path()).unwrap();
    let mut assembly = Assembly::start(
        &dest,
        &IncomingPath::parse("live.bin").unwrap(),
        sender.header().clone(),
    )
    .unwrap();
    assembly
        .accept(sender.next_block().unwrap().unwrap())
        .unwrap();
    // Another program writes to the file mid-read (same size, new time).
    std::thread::sleep(std::time::Duration::from_millis(20));
    let f = std::fs::OpenOptions::new().write(true).open(&path).unwrap();
    f.set_modified(std::time::SystemTime::now() + std::time::Duration::from_secs(5))
        .unwrap();
    drop(f);
    while let Some(b) = sender.next_block().unwrap() {
        assembly.accept(b).unwrap();
    }
    let trailer = sender.finish().unwrap();
    assert!(trailer.changed_while_read());
    assert!(matches!(
        assembly.finish(trailer),
        Err(TransferError::ChangedWhileRead)
    ));
    assert!(!dst.path().join("live.bin").exists());
}

proptest::proptest! {
    #![proptest_config(proptest::prelude::ProptestConfig::with_cases(2000))]
    /// Whatever arrives, decoding never crashes and never yields a block longer than allowed.
    #[test]
    fn decoding_anything_never_crashes(bytes in proptest::collection::vec(proptest::prelude::any::<u8>(), 0..400)) {
        if let Ok(block) = Block::decode(&bytes, 256) {
            proptest::prop_assert!(block.encode().len() <= 46 + 256);
        }
    }

    /// A real block with any single byte changed is refused, or still gives the exact original.
    #[test]
    fn any_changed_byte_is_caught(position in 0usize..2000, flip in 1u8..=255) {
        let src = tempfile::tempdir().unwrap();
        let path = source(src.path(), "a.txt", &vec![b'q'; 1500]);
        let mut sender = FileSender::open(&path, None, true).unwrap();
        let first = sender.next_block().unwrap().unwrap();
        let original = Block::decode(&first.encode(), MIN_BLOCK).unwrap();
        let mut wire = first.encode();
        let i = position % wire.len();
        // Changing the stated index alone still gives a valid block for another place, which the
        // receiver's order check refuses; every other byte must be caught here.
        if !(2..10).contains(&i) {
            wire[i] ^= flip;
            // Either refused, or (for a byte in the packed form that does not change what it unpacks
            // to) exactly the original contents: damaged data can never get through.
            if let Ok(block) = Block::decode(&wire, MIN_BLOCK) {
                proptest::prop_assert_eq!(block, original.clone());
            }
        }
    }
}

#[test]
fn a_genuine_block_of_the_wrong_length_for_its_place_is_refused() {
    let src = tempfile::tempdir().unwrap();
    let dst = tempfile::tempdir().unwrap();
    let path = source(src.path(), "a.bin", &mixed_content(400_000));
    let sender = FileSender::open(&path, None, true).unwrap();
    let dest = Destination::open(dst.path()).unwrap();
    let mut assembly = Assembly::start(
        &dest,
        &IncomingPath::parse("a.bin").unwrap(),
        sender.header().clone(),
    )
    .unwrap();
    // A hostile sender: a correctly fingerprinted block 0 that is only 10 bytes long.
    let data = b"0123456789";
    let mut wire = vec![1u8, 0];
    wire.extend_from_slice(&0u64.to_be_bytes());
    wire.extend_from_slice(&10u32.to_be_bytes());
    wire.extend_from_slice(blake3::hash(data).as_bytes());
    wire.extend_from_slice(data);
    let block = Block::decode(&wire, MIN_BLOCK).unwrap();
    assert!(matches!(
        assembly.accept(block),
        Err(TransferError::Damaged(_))
    ));
    assert_eq!(assembly.resume_ticket().done.done_count(), 0);
}
