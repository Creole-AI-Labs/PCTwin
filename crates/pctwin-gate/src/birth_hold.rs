//! Keeps every file and folder PCTwin makes open until a little after it was made (Linux and
//! macOS).
//!
//! A file is told apart from a later one in the same freed number by its birth time, and a drive
//! keeps birth times in ticks of a few milliseconds. While a handle is open the number cannot be
//! freed, so holding each new file and folder open until [`HOLD`] after it was made means nothing
//! made later can share both its number and its birth time. The handles wait in one bounded queue
//! that a single helper thread empties as each one's time passes; nothing sleeps per file, and a
//! full queue makes the next maker wait at most [`HOLD`].

use std::collections::VecDeque;
use std::sync::{Condvar, Mutex, OnceLock, PoisonError};
use std::time::{Duration, Instant};

/// How long past its birth each new file or folder is kept open: more than two ticks of the
/// coarsest birth-time clock measured (4 ms).
pub(crate) const HOLD: Duration = Duration::from_millis(10);

/// The most handles held at once.
const MOST: usize = 128;

struct Queue {
    held: Mutex<VecDeque<(Instant, std::fs::File)>>,
    changed: Condvar,
}

fn queue() -> &'static Queue {
    static QUEUE: OnceLock<&'static Queue> = OnceLock::new();
    QUEUE.get_or_init(|| {
        let q: &'static Queue = Box::leak(Box::new(Queue {
            held: Mutex::new(VecDeque::new()),
            changed: Condvar::new(),
        }));
        std::thread::Builder::new()
            .name("pctwin-birth-hold".into())
            .spawn(move || release_as_due(q))
            .map(drop)
            // Without the helper thread, `hold` lets each handle go at once: identity still has
            // the birth time, only without the extra guard.
            .ok();
        q
    })
}

fn release_as_due(q: &'static Queue) {
    let mut held = q.held.lock().unwrap_or_else(PoisonError::into_inner);
    loop {
        match held.front().map(|(due, _)| *due) {
            None => {
                held = q.changed.wait(held).unwrap_or_else(PoisonError::into_inner);
            }
            Some(due) => {
                let now = Instant::now();
                if now >= due {
                    drop(held.pop_front());
                    q.changed.notify_all();
                } else {
                    held = q
                        .changed
                        .wait_timeout(held, due - now)
                        .unwrap_or_else(PoisonError::into_inner)
                        .0;
                }
            }
        }
    }
}

/// Keeps `file` (a file or folder just made, opened by the caller) open until [`HOLD`] after
/// `born`. Best effort: a handle that cannot be copied is simply not held.
pub(crate) fn hold(file: &std::fs::File, born: Instant) {
    if !cfg!(unix) {
        return;
    }
    let Ok(copy) = file.try_clone() else {
        return;
    };
    let q = queue();
    let mut held = q.held.lock().unwrap_or_else(PoisonError::into_inner);
    while held.len() >= MOST {
        held = q
            .changed
            .wait_timeout(held, HOLD)
            .unwrap_or_else(PoisonError::into_inner)
            .0;
    }
    held.push_back((born + HOLD, copy));
    q.changed.notify_all();
}

/// Waits until every held handle is let go (at most [`HOLD`] plus the helper's turn), so a check
/// that needs PCTwin itself to have no other handle on a file (the write lease) is not fooled by
/// one still held here.
pub(crate) fn settle() {
    let q = queue();
    let mut held = q.held.lock().unwrap_or_else(PoisonError::into_inner);
    let until = Instant::now() + HOLD * 4;
    while !held.is_empty() && Instant::now() < until {
        held = q
            .changed
            .wait_timeout(held, HOLD)
            .unwrap_or_else(PoisonError::into_inner)
            .0;
    }
}

#[cfg(all(test, unix))]
mod tests {
    use super::*;

    /// A file made, held, removed and made again within the hold never gets the same number.
    #[test]
    fn a_file_made_again_at_once_gets_another_number() {
        use std::os::unix::fs::MetadataExt;
        let root = tempfile::tempdir().unwrap();
        let path = root.path().join("a");
        let mut reused = 0;
        for _ in 0..200 {
            let born = Instant::now();
            let first = std::fs::File::create(&path).unwrap();
            let ino = first.metadata().unwrap().ino();
            hold(&first, born);
            drop(first);
            std::fs::remove_file(&path).unwrap();
            let again = std::fs::File::create(&path).unwrap();
            if again.metadata().unwrap().ino() == ino {
                reused += 1;
            }
            std::fs::remove_file(&path).unwrap();
        }
        assert_eq!(reused, 0);
        settle();
        assert!(queue().held.lock().unwrap().is_empty());
    }
}
