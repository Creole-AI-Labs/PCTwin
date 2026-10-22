//! When a connection drops during a move, how long to wait before the next one, and when to tell
//! the person that nothing new has arrived (Task List, the reconnect loop). A connection that
//! moved something resets the wait; each one in a row that moved nothing waits twice as long, up
//! to a minute, and after two of them the screen says "Nothing new has arrived" with Stop. It
//! never gives up on its own: only the person stops a move.

use std::sync::{Arc, Mutex, PoisonError};
use std::time::{Duration, Instant};

use tokio::sync::Notify;

/// The first wait after a drop.
pub const FIRST_WAIT: Duration = Duration::from_secs(1);
/// The longest wait between connections.
pub const LONGEST_WAIT: Duration = Duration::from_secs(60);
/// Connections in a row that moved nothing before the screen says so.
pub const NOTHING_NEW_AFTER: u32 = 2;

/// What to do after a connection ended.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Next {
    /// How long to wait before connecting again.
    pub wait: Duration,
    /// Show "Nothing new has arrived" (with Stop): connections keep ending without progress.
    pub nothing_new: bool,
}

/// The reconnect loop's memory of how the last connections went.
#[derive(Debug, Clone, Default)]
pub struct Reconnect {
    idle_in_a_row: u32,
}

impl Reconnect {
    pub fn new() -> Self {
        Self::default()
    }

    /// A connection ended; `progressed` is whether it moved anything (bytes written, a file
    /// ended): the meter says.
    pub fn ended(&mut self, progressed: bool) -> Next {
        if progressed {
            self.idle_in_a_row = 0;
        } else {
            self.idle_in_a_row = self.idle_in_a_row.saturating_add(1);
        }
        let doublings = self.idle_in_a_row.min(10);
        let wait = FIRST_WAIT
            .saturating_mul(1u32 << doublings)
            .min(LONGEST_WAIT);
        Next {
            wait,
            nothing_new: self.idle_in_a_row >= NOTHING_NEW_AFTER,
        }
    }
}

/// How the connection between the laptops stands, as the screens show it (C09).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum LinkState {
    /// Connected (or not yet started): nothing to show.
    #[default]
    Connected,
    /// The connection dropped at `since` (when the other laptop was last heard from) and the move
    /// waits to carry on over a new one. `nothing_new`: connections keep ending without anything
    /// new arriving ([`NOTHING_NEW_AFTER`] in a row), so the screen says so, with Stop.
    Waiting { since: Instant, nothing_new: bool },
    /// The laptops can no longer find each other again without pairing again (the resume keys ran
    /// out of time or tries, or the old laptop could not listen for the new one): the move ended.
    Over,
}

#[derive(Debug, Default)]
struct ConnectionInner {
    state: Mutex<LinkState>,
    /// "Try now": the next try starts at once rather than after its wait.
    now: Notify,
}

/// The move's connection as the screens see it, and the person's "Try now". Cheap to clone; held
/// in [`crate::Live`].
#[derive(Debug, Clone, Default)]
pub struct Connection(Arc<ConnectionInner>);

impl Connection {
    pub fn state(&self) -> LinkState {
        *self.0.state.lock().unwrap_or_else(PoisonError::into_inner)
    }

    /// The person chose "Try now" while waiting: the new laptop tries at once. Nothing happens
    /// otherwise (a press while connected is not kept for a later drop).
    pub fn try_now(&self) {
        if matches!(self.state(), LinkState::Waiting { .. }) {
            self.0.now.notify_one();
        }
    }

    pub(crate) fn set(&self, state: LinkState) {
        *self.0.state.lock().unwrap_or_else(PoisonError::into_inner) = state;
    }

    /// Returns once the person chose "Try now" (at once if they did since the last try).
    pub(crate) async fn tried_now(&self) {
        self.0.now.notified().await;
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn idle_connections_wait_longer_and_progress_starts_again() {
        let mut r = Reconnect::new();
        // A drop after progress: back at once, nothing to say.
        assert_eq!(
            r.ended(true),
            Next {
                wait: FIRST_WAIT,
                nothing_new: false
            }
        );
        let waits: Vec<(u64, bool)> = (0..8)
            .map(|_| {
                let n = r.ended(false);
                (n.wait.as_secs(), n.nothing_new)
            })
            .collect();
        assert_eq!(
            waits,
            vec![
                (2, false),
                (4, true),
                (8, true),
                (16, true),
                (32, true),
                (60, true),
                (60, true),
                (60, true)
            ]
        );
        // Something arrived: the wait and the message go.
        assert_eq!(
            r.ended(true),
            Next {
                wait: FIRST_WAIT,
                nothing_new: false
            }
        );
    }
}
