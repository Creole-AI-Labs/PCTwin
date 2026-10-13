//! macOS: the system's list of programs that have a file open (Security Design 3B undo bullet).
//! macOS cannot prove that no other program can still write to a file, so undo never removes a
//! copy there (strengthened 13 October 2026); this list only says whether the copy is kept
//! because it is "in use" or because PCTwin "cannot check". It works by path, as a check, never
//! as the proof.

use std::io;
use std::path::Path;

/// How many times the system's list is asked for when it may have been cut short.
const LIST_TRIES: usize = 3;

/// Whether any program other than PCTwin has the file at `path` open, in any way: one that only
/// watches it for changes counts too (no filter is asked for, so the list is exactly every
/// program with the file open). The system's list is an Apple interface without a promise, so a
/// failure to read it is an error and the caller keeps the file. The list is made in a space
/// sized for every program running; a list that fills that space may have been cut short
/// (programs started meanwhile), so it is asked for again, and if it stays full, that is an error
/// too.
pub(crate) fn others_have_it_open(path: &Path) -> io::Result<bool> {
    let me = std::process::id();
    for _ in 0..LIST_TRIES {
        let pids = libproc::processes::pids_by_path(path, false, false)?;
        if pids.iter().any(|p| *p != me) {
            return Ok(true);
        }
        if pids.len() < pids.capacity() {
            return Ok(false);
        }
    }
    Err(io::Error::other(
        "the list of programs with the file open may be incomplete",
    ))
}
