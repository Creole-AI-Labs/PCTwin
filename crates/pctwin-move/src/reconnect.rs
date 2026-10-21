//! When a connection drops during a move, how long to wait before the next one, and when to tell
//! the person that nothing new has arrived (Task List, the reconnect loop). A connection that
//! moved something resets the wait; each one in a row that moved nothing waits twice as long, up
//! to a minute, and after two of them the screen says "Nothing new has arrived" with Stop. It
//! never gives up on its own: only the person stops a move.

use std::time::Duration;

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
