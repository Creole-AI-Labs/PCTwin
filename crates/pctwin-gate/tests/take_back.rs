//! Sending again (quarantine design, step 2): blocks that read back wrong are taken back, so only
//! they can be written again; everything else the gate promises still holds. Taking back is the
//! only way a byte that arrived can be written twice, and it never lets a file be named without a
//! new read-back that matched.

use pctwin_gate::{Destination, FaultPlan, Flip, GateError, IncomingFile, IncomingPath};

fn path(s: &str) -> IncomingPath {
    IncomingPath::parse(s).unwrap()
}

fn content(len: usize) -> Vec<u8> {
    (0..len)
        .map(|i| (i as u8).wrapping_mul(31) ^ ((i / 4096) as u8))
        .collect()
}

fn hashes(data: &[u8], block: usize) -> Vec<[u8; 32]> {
    data.chunks(block)
        .map(|b| *blake3::hash(b).as_bytes())
        .collect()
}

fn written<'d>(dest: &'d Destination, name: &str, data: &[u8], block: usize) -> IncomingFile<'d> {
    let mut file = dest.create_file(&path(name), data.len() as u64).unwrap();
    for (i, b) in data.chunks(block).enumerate() {
        file.write_at((i * block) as u64, b).unwrap();
    }
    file
}

const BLOCK: usize = 4096;

#[test]
fn a_block_stored_wrong_is_taken_back_written_again_and_then_named() {
    let root = tempfile::tempdir_in(env!("CARGO_TARGET_TMPDIR")).unwrap();
    let dest = Destination::open(root.path())
        .unwrap()
        .with_faults(FaultPlan {
            flip: Some(Flip::OnWriteOnce),
            ..Default::default()
        });
    let data = content(3 * BLOCK + 100);
    let sent = hashes(&data, BLOCK);
    let mut file = written(&dest, "a.bin", &data, BLOCK);
    let first = file.verify(BLOCK as u64, &sent).unwrap();
    assert_eq!(first.bad_blocks, [0]);
    // Only the bad block is taken back, and only it may be written again.
    file.take_back(0, BLOCK as u64).unwrap();
    assert!(matches!(
        file.write_at(BLOCK as u64, &data[BLOCK..2 * BLOCK]),
        Err(GateError::Overlap)
    ));
    file.write_at(0, &data[..BLOCK]).unwrap();
    assert!(file.verify(BLOCK as u64, &sent).unwrap().passed());
    file.finish().unwrap();
    assert_eq!(std::fs::read(root.path().join("a.bin")).unwrap(), data);
}

#[test]
fn a_file_with_a_block_taken_back_is_never_named_until_it_is_written_and_checked_again() {
    let root = tempfile::tempdir_in(env!("CARGO_TARGET_TMPDIR")).unwrap();
    let dest = Destination::open(root.path()).unwrap();
    let data = content(2 * BLOCK);
    let sent = hashes(&data, BLOCK);
    let mut file = written(&dest, "b.bin", &data, BLOCK);
    assert!(file.verify(BLOCK as u64, &sent).unwrap().passed());
    // A good check no longer counts once a block is taken back.
    file.take_back(BLOCK as u64, BLOCK as u64).unwrap();
    assert!(matches!(
        file.verify(BLOCK as u64, &sent),
        Err(GateError::SizeMismatch { .. })
    ));
    assert!(matches!(file.seal(), Err(GateError::SizeMismatch { .. })));
    assert!(!root.path().join("b.bin").exists());
}

#[test]
fn a_taken_back_check_does_not_survive_to_the_seal() {
    let root = tempfile::tempdir_in(env!("CARGO_TARGET_TMPDIR")).unwrap();
    let dest = Destination::open(root.path()).unwrap();
    let data = content(2 * BLOCK);
    let sent = hashes(&data, BLOCK);
    let mut file = written(&dest, "c.bin", &data, BLOCK);
    assert!(file.verify(BLOCK as u64, &sent).unwrap().passed());
    file.take_back(0, BLOCK as u64).unwrap();
    file.write_at(0, &data[..BLOCK]).unwrap();
    // Written again but not read back again: no name.
    assert!(matches!(file.seal(), Err(GateError::NotReadBack)));
}

#[test]
fn only_bytes_that_arrived_and_inside_the_file_can_be_taken_back() {
    let root = tempfile::tempdir_in(env!("CARGO_TARGET_TMPDIR")).unwrap();
    let dest = Destination::open(root.path()).unwrap();
    let data = content(3 * BLOCK);
    let mut file = dest.create_file(&path("d.bin"), data.len() as u64).unwrap();
    file.write_at(0, &data[..BLOCK]).unwrap();
    // Not arrived yet.
    assert!(matches!(
        file.take_back(BLOCK as u64, BLOCK as u64),
        Err(GateError::NotArrived)
    ));
    // Partly arrived.
    assert!(matches!(
        file.take_back(BLOCK as u64 / 2, BLOCK as u64),
        Err(GateError::NotArrived)
    ));
    // Past the end.
    assert!(matches!(
        file.take_back(3 * BLOCK as u64, 1),
        Err(GateError::OutsideFile)
    ));
    assert!(matches!(
        file.take_back(u64::MAX, 2),
        Err(GateError::OutsideFile)
    ));
    // Nothing at all is fine and changes nothing.
    file.take_back(0, 0).unwrap();
    file.write_at(BLOCK as u64, &data[BLOCK..2 * BLOCK])
        .unwrap();
    file.write_at(2 * BLOCK as u64, &data[2 * BLOCK..]).unwrap();
    // A middle block of one joined part is taken back, splitting it in two; the rest stays.
    file.take_back(BLOCK as u64, BLOCK as u64).unwrap();
    assert!(matches!(
        file.write_at(0, &data[..BLOCK]),
        Err(GateError::Overlap)
    ));
    assert!(matches!(
        file.write_at(2 * BLOCK as u64, &data[2 * BLOCK..]),
        Err(GateError::Overlap)
    ));
    file.write_at(BLOCK as u64, &data[BLOCK..2 * BLOCK])
        .unwrap();
    assert!(
        file.verify(BLOCK as u64, &hashes(&data, BLOCK))
            .unwrap()
            .passed()
    );
}
