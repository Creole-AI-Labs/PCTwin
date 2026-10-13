//! Waiting on something that may never answer, without waiting forever (Security Design 3B, undo
//! bullet: "each resolution runs in a worker with a deadline"). Apple's file coordination waits
//! on other apps to save and let go, and an app that never answers would keep undo waiting for
//! good; so the coordination is held by a worker thread, and undo's own steps run on the caller's
//! thread only once it is held, within the deadline. The worker itself never acts on the drive.

use std::io;
use std::sync::mpsc;
use std::time::Duration;

/// What the worker tells the caller.
enum Said {
    /// It holds what it was asked to hold, until told to let go.
    Holding,
    /// It ended (with an error if it could not get hold).
    Ended(io::Result<()>),
}

/// Runs `then` on this thread while `hold`, run on a worker thread, holds something: `hold` is
/// given `inside`, which it calls once it holds it, and it keeps holding until `inside` returns,
/// which is once `then` has finished. If it does not get hold within `wait`, `then` never runs
/// and the answer is `Ok(None)`: the worker is left to end on its own, and whatever it gets hold
/// of later it lets go at once. If `hold` ends with an error first, that error is returned.
#[cfg_attr(
    all(not(target_os = "macos"), not(test)),
    expect(dead_code, reason = "only macOS coordinates with other apps")
)]
pub(crate) fn while_held<R>(
    wait: Duration,
    hold: impl FnOnce(&dyn Fn()) -> io::Result<()> + Send + 'static,
    then: impl FnOnce() -> R,
) -> io::Result<Option<R>> {
    let (said, hears) = mpsc::channel::<Said>();
    let (let_go, told) = mpsc::channel::<()>();
    std::thread::Builder::new()
        .name("pctwin-undo-hold".into())
        .spawn(move || {
            let holding = said.clone();
            let inside = move || {
                // Only if the caller is still waiting; then held until it is done (or gone).
                if holding.send(Said::Holding).is_ok() {
                    let _ = told.recv();
                }
            };
            let ended = hold(&inside);
            let _ = said.send(Said::Ended(ended));
        })?;
    let r = match hears.recv_timeout(wait) {
        Ok(Said::Holding) => Ok(Some(then())),
        Ok(Said::Ended(Err(e))) => Err(e),
        Ok(Said::Ended(Ok(()))) => Err(io::Error::other(
            "it ended without getting hold of the file",
        )),
        Err(mpsc::RecvTimeoutError::Timeout | mpsc::RecvTimeoutError::Disconnected) => Ok(None),
    };
    // Lets go: the worker's `inside` returns now, or at once if it gets there later.
    drop(let_go);
    r
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::Arc;
    use std::sync::atomic::{AtomicBool, Ordering};
    use std::time::Instant;

    /// Something that never answers: the caller's part never runs, and the caller is back within
    /// the deadline, not when (if ever) the other side answers.
    #[test]
    fn a_hold_that_never_comes_runs_nothing_and_returns_in_time() {
        let ran = AtomicBool::new(false);
        let start = Instant::now();
        let r = while_held(
            Duration::from_millis(100),
            |inside| {
                std::thread::sleep(Duration::from_secs(3));
                inside();
                Ok(())
            },
            || ran.store(true, Ordering::SeqCst),
        )
        .unwrap();
        assert!(r.is_none());
        assert!(!ran.load(Ordering::SeqCst));
        assert!(
            start.elapsed() < Duration::from_secs(2),
            "{:?}",
            start.elapsed()
        );
    }

    /// Once held, the caller's part runs while it is held, and the hold ends only after it.
    #[test]
    fn the_callers_part_runs_while_held_and_the_hold_ends_after_it() {
        let released = Arc::new(AtomicBool::new(false));
        let r2 = Arc::clone(&released);
        let (ended_tx, ended_rx) = mpsc::channel();
        let r = while_held(
            Duration::from_secs(10),
            move |inside| {
                inside();
                r2.store(true, Ordering::SeqCst);
                let _ = ended_tx.send(());
                Ok(())
            },
            || {
                std::thread::sleep(Duration::from_millis(50));
                released.load(Ordering::SeqCst)
            },
        )
        .unwrap();
        assert_eq!(r, Some(false), "released before the caller's part finished");
        ended_rx.recv_timeout(Duration::from_secs(5)).unwrap();
        assert!(released.load(Ordering::SeqCst));
    }

    /// A hold that fails says why, and nothing runs.
    #[test]
    fn a_hold_that_fails_says_so_and_runs_nothing() {
        let r = while_held(
            Duration::from_secs(10),
            |_| Err(io::Error::other("refused")),
            || panic!("must not run"),
        );
        assert_eq!(r.unwrap_err().to_string(), "refused");
        let r = while_held(
            Duration::from_secs(10),
            |_| Ok(()),
            || panic!("must not run"),
        );
        assert!(r.is_err());
    }

    /// A hold that comes late, after the caller gave up, is let go at once.
    #[test]
    fn a_hold_that_comes_late_is_let_go_at_once() {
        let (done_tx, done_rx) = mpsc::channel();
        let r = while_held(
            Duration::from_millis(50),
            move |inside| {
                std::thread::sleep(Duration::from_millis(300));
                let start = Instant::now();
                inside();
                let _ = done_tx.send(start.elapsed());
                Ok(())
            },
            || (),
        )
        .unwrap();
        assert!(r.is_none());
        let held_for = done_rx.recv_timeout(Duration::from_secs(5)).unwrap();
        assert!(held_for < Duration::from_secs(1), "{held_for:?}");
    }
}
