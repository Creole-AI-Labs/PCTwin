//! macOS: before undo moves a copy aside, it announces the removal through Apple's file
//! coordination, so apps showing the file save it and let go, and then asks the system which
//! programs have the file open (decided 9 October 2026, Security Design 3B undo bullet). A file
//! any other program has open, in any way, is kept. If either cannot be asked, nothing is removed.
//!
//! Both work by path: they are a courtesy to other programs and a check, never the proof. The
//! proof that PCTwin removes the very file it checked, unchanged, stays with the held handle and
//! the private name (see `unix_undo`).

use std::cell::{Cell, RefCell};
use std::io;
use std::path::Path;
use std::ptr::NonNull;

use objc2::AllocAnyThread;
use objc2::rc::Retained;
use objc2_foundation::{
    NSError, NSFileCoordinator, NSFileCoordinatorWritingOptions, NSString, NSURL,
};

/// Runs `then` while macOS file coordination holds the file at `path` for deleting: every app
/// presenting it has been told, has saved, and has let go. An error if coordination fails.
pub(crate) fn coordinated_for_deleting<R>(path: &Path, then: impl FnOnce() -> R) -> io::Result<R> {
    let text = path
        .to_str()
        .ok_or_else(|| io::Error::new(io::ErrorKind::InvalidInput, "path is not text"))?;
    let url = NSURL::fileURLWithPath(&NSString::from_str(text));
    let coordinator = NSFileCoordinator::initWithFilePresenter(NSFileCoordinator::alloc(), None);
    let then = Cell::new(Some(then));
    let result = RefCell::new(None);
    let accessor = block2::RcBlock::new(|_url: NonNull<NSURL>| {
        if let Some(f) = then.take() {
            *result.borrow_mut() = Some(f());
        }
    });
    let mut error: Option<Retained<NSError>> = None;
    coordinator.coordinateWritingItemAtURL_options_error_byAccessor(
        &url,
        NSFileCoordinatorWritingOptions::ForDeleting,
        Some(&mut error),
        &accessor,
    );
    if let Some(e) = error {
        return Err(io::Error::other(format!(
            "file coordination refused: {}",
            e.localizedDescription()
        )));
    }
    result
        .take()
        .ok_or_else(|| io::Error::other("file coordination did not run"))
}

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
