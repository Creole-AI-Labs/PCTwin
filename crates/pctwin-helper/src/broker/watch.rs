//! Ending the broker's grants when the person's session changes (acceptance case T22-023:
//! sleep, sign-out and switching user end every grant; the move asks again afterwards).
//!
//! The helper watches for itself; it does not wait to be told by the app, which may be the one
//! that went away. A watcher thread looks every [`TICK`]:
//!
//! - **Sleep and hibernation**, on every system: the watcher's own looks stop while the laptop
//!   sleeps, so a gap between two looks far longer than [`TICK`] on either clock (the one that only
//!   goes forward, or the calendar) means the laptop slept (or the calendar was changed, which is
//!   treated the same way). Both clocks are read, because the forward-only clock does not count
//!   sleep on every system.
//! - **Switching user** (Windows): the session the helper runs in is no longer the one shown on
//!   the screen and keyboard (`WTSQuerySessionInformationW`, `WTSConnectState` not active).
//! - **Sign-out**: the system ends every program of the session, the helper with it, and the
//!   broker with the helper (nothing of a grant is kept on disk). Also on Mac and Linux.
//!
//! Each ends the session's grants through the broker's [`Revoker`]: nothing new starts in this
//! session, ever (an installer already running finishes and reports what really happened).
//!
//! What does not end the grants, by design (second review, 27 October 2026): **locking the
//! screen**. A locked session is still the active one on the screen and keyboard (nobody else is
//! using the laptop), so grants given before the lock stay until the lease or the session's
//! deadline runs out, sleep, sign-out or a switch of user. And with the administrator prompt
//! turned off (UAC off), an administrator's programs run with full rights: the app then reads as
//! elevated and refuses to start apps (T22-020), which fails safe rather than open.
//!
//! Not yet proven: switching user on Mac and Linux (fast user switching keeps the helper's
//! session alive there, and no safe system call for it is wired yet); the sleep check on a real
//! laptop going to sleep (proven here with the clocks given to the watcher).

use std::time::{Duration, Instant, SystemTime};

use super::Revoker;

/// How often the watcher looks.
pub const TICK: Duration = Duration::from_secs(2);
/// A gap between two looks longer than this, on either clock, means the laptop slept.
pub const SLEPT_AFTER: Duration = Duration::from_secs(30);

/// Why the grants were ended.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Ended {
    /// The laptop slept (or the calendar was changed).
    Slept,
    /// Another person's session is now on the screen.
    SwitchedAway,
}

/// One look of the watcher: two readings of the clocks, and whether the helper's session is
/// still the one on the screen (`None`: this system cannot tell).
#[derive(Debug, Clone, Copy)]
pub struct Look {
    pub mono: Instant,
    pub wall: SystemTime,
    pub on_screen: Option<bool>,
}

/// What the watcher decides from the look before and this one.
pub fn judge(before: &Look, now: &Look) -> Option<Ended> {
    if now.on_screen == Some(false) {
        return Some(Ended::SwitchedAway);
    }
    let mono_gap = now.mono.saturating_duration_since(before.mono);
    let wall_gap = now.wall.duration_since(before.wall);
    match wall_gap {
        // The calendar went back.
        Err(_) => Some(Ended::Slept),
        Ok(wall) if wall > SLEPT_AFTER || mono_gap > SLEPT_AFTER => Some(Ended::Slept),
        Ok(_) => None,
    }
}

/// This look, from the system.
pub fn look_now() -> Look {
    Look {
        mono: Instant::now(),
        wall: SystemTime::now(),
        on_screen: on_screen(),
    }
}

#[cfg(windows)]
fn on_screen() -> Option<bool> {
    pctwin_helper_os::session_on_screen().ok()
}

#[cfg(not(windows))]
fn on_screen() -> Option<bool> {
    None
}

/// Watches until the grants are ended (or `stop` is revoked by something else), looking with
/// `look` every `tick`. Returns why it ended them, or `None` if they were ended elsewhere.
pub fn watch(
    revoker: &Revoker,
    tick: Duration,
    mut look: impl FnMut() -> Look,
    mut wait: impl FnMut(Duration),
) -> Option<Ended> {
    let mut before = look();
    loop {
        if revoker.is_revoked() {
            return None;
        }
        wait(tick);
        let now = look();
        if let Some(why) = judge(&before, &now) {
            revoker.revoke();
            return Some(why);
        }
        before = now;
    }
}

/// Starts the watcher on its own thread for the broker's `revoker`.
pub fn start(revoker: Revoker) -> std::thread::JoinHandle<Option<Ended>> {
    std::thread::spawn(move || watch(&revoker, TICK, look_now, std::thread::sleep))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn look(base: &(Instant, SystemTime), mono: u64, wall: i64, on_screen: Option<bool>) -> Look {
        let wall = if wall >= 0 {
            base.1 + Duration::from_secs(wall as u64)
        } else {
            base.1 - Duration::from_secs(wall.unsigned_abs())
        };
        Look {
            mono: base.0 + Duration::from_secs(mono),
            wall,
            on_screen,
        }
    }

    /// T22-023: what ends the grants, and what does not.
    #[test]
    fn sleep_and_switching_person_end_the_grants() {
        let base = (Instant::now(), SystemTime::now());
        let before = look(&base, 100, 100, Some(true));
        let cases = [
            // An ordinary tick, a slow one: nothing.
            (look(&base, 102, 102, Some(true)), None),
            (look(&base, 129, 129, None), None),
            // A gap on either clock: the laptop slept (or the calendar was changed).
            (look(&base, 102, 3_700, Some(true)), Some(Ended::Slept)),
            (look(&base, 3_700, 102, Some(true)), Some(Ended::Slept)),
            (look(&base, 102, 40, Some(true)), Some(Ended::Slept)),
            // Another person's session is on the screen.
            (
                look(&base, 102, 102, Some(false)),
                Some(Ended::SwitchedAway),
            ),
        ];
        for (now, want) in cases {
            assert_eq!(judge(&before, &now), want, "{now:?}");
        }
    }

    /// The watcher ends the grants through the broker's revoker as soon as it sees a change, and
    /// stops when they were ended elsewhere.
    #[test]
    fn the_watcher_revokes_at_once_and_stops_when_revoked_elsewhere() {
        let base = (Instant::now(), SystemTime::now());
        let looks = [
            look(&base, 0, 0, Some(true)),
            look(&base, 2, 2, Some(true)),
            look(&base, 4, 4, Some(true)),
            // The laptop slept for an hour.
            look(&base, 6, 3_606, Some(true)),
        ];
        let r = Revoker::default();
        let mut n = 0;
        let mut waits = 0;
        let why = watch(
            &r,
            TICK,
            || {
                n += 1;
                looks[n - 1]
            },
            |d| {
                assert_eq!(d, TICK);
                waits += 1;
            },
        );
        assert_eq!(why, Some(Ended::Slept));
        assert!(r.is_revoked());
        assert_eq!(waits, 3);
        // Ended elsewhere (the app's stop): the watcher just stops.
        let r = Revoker::default();
        r.revoke();
        assert_eq!(watch(&r, TICK, || looks[0], |_| {}), None);
    }

    /// The real look on this machine: the clocks move on, and on Windows the test's own session
    /// is on the screen (a service-run CI job may have none: then it cannot be told).
    #[test]
    fn the_real_look_reads_both_clocks() {
        let a = look_now();
        let b = look_now();
        assert!(b.mono >= a.mono);
        let want = if b.on_screen == Some(false) {
            Some(Ended::SwitchedAway)
        } else {
            None
        };
        assert_eq!(judge(&a, &b), want);
    }
}
