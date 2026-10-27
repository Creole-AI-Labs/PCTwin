//! A change that could strand the person (the main mouse button) is applied in a move scope of
//! its own, then kept only if the person confirms it within a short time; otherwise it goes
//! back by itself through the same journaled undo (T24-009). Going back is compare-and-write,
//! so a choice the person made in the meantime is kept.
//!
//! The confirmation the person sees (keyboard and screen-reader usable, T24-026) is a 2.8
//! screen; this is the rule behind it.

use std::time::{Duration, Instant};

use pctwin_journal::Scope;

/// How long the person has to confirm.
pub const CONFIRM_WITHIN: Duration = Duration::from_secs(20);

/// What to do now.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Decision {
    /// Still waiting for the person.
    Wait,
    /// Confirmed: keep it.
    Keep,
    /// Not confirmed in time: put it back (undo this scope).
    GoBack,
}

/// One confirm-or-go-back window.
#[derive(Debug, Clone)]
pub struct ConfirmWindow {
    started: Instant,
    within: Duration,
    confirmed: bool,
}

impl ConfirmWindow {
    pub fn start(now: Instant) -> Self {
        Self {
            started: now,
            within: CONFIRM_WITHIN,
            confirmed: false,
        }
    }

    /// The person confirmed. Only counts inside the window.
    pub fn confirm(&mut self, now: Instant) -> bool {
        if now.saturating_duration_since(self.started) < self.within {
            self.confirmed = true;
        }
        self.confirmed
    }

    pub fn decide(&self, now: Instant) -> Decision {
        if self.confirmed {
            Decision::Keep
        } else if now.saturating_duration_since(self.started) >= self.within {
            Decision::GoBack
        } else {
            Decision::Wait
        }
    }
}

/// The move scope a risky setting is applied in: its own, derived from the move's and the
/// setting's key, so undoing it touches nothing else.
pub fn scope_for(move_scope: Scope, key: &str) -> Scope {
    let mut input = move_scope.0.to_vec();
    input.extend_from_slice(key.as_bytes());
    let d = blake3::derive_key("PCTwin risky setting scope 2026-10-27 v1", &input);
    let mut s = [0u8; 16];
    s.copy_from_slice(&d[..16]);
    Scope(s)
}
