//! Shared by the gate's tests. A file gets its name only after it is read back from the drive and
//! checked against the sender's block fingerprints; tests about other things check it against
//! what was written to it (read through the gate's own handle), then go on as before.

#![allow(dead_code)]

use pctwin_gate::{Finished, GateError, IncomingFile, Sealed};

const BLOCK: u64 = 4096;

/// Reads `file` back and checks it against what was written to it. A file not fully arrived is
/// left for `seal` to refuse as before.
pub fn read_back(file: &mut IncomingFile<'_>) {
    let size = file.written();
    let mut hashes = Vec::new();
    let mut at = 0;
    while at < size {
        let mut buf = vec![0u8; BLOCK.min(size - at) as usize];
        if file.read_at(at, &mut buf).is_err() {
            return;
        }
        hashes.push(*blake3::hash(&buf).as_bytes());
        at += buf.len() as u64;
    }
    let _ = file.verify(BLOCK, &hashes);
}

/// [`IncomingFile::finish`] after the read-back.
pub fn finish(mut file: IncomingFile<'_>) -> Result<Finished, GateError> {
    read_back(&mut file);
    file.finish()
}

/// [`IncomingFile::seal`] after the read-back.
pub fn seal(mut file: IncomingFile<'_>) -> Result<Sealed<'_>, GateError> {
    read_back(&mut file);
    file.seal()
}
