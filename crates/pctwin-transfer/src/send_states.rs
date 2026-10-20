//! The old laptop's per-file states, each change of step counted in the move meter. Kept apart
//! from the session so that nothing can change a state except through [`States::slot`]: the
//! states themselves are private to this module.

use crate::meter::{Group, MoveMeter, Step};
use crate::session::{SendJob, SendOutcome, SendState};

/// Sent-file states this many or fewer are checked against the meter after every answer in debug
/// builds (more are checked after each run only, so a debug build of the app stays quick).
const CHECKED_JOBS: usize = 256;

/// The old laptop's step for a file in this state (see [`Step`]).
fn step_of(state: &SendState) -> Step {
    match state {
        SendState::Waiting => Step::Waiting,
        SendState::Open { .. } => Step::Copying,
        // Sent whole, or found identical there: waiting for the new laptop to say it checked it.
        SendState::Ended { failure: None, .. } | SendState::Skipping => Step::Checking,
        // Failed here; only told to the new laptop so far.
        SendState::Ended {
            failure: Some(_), ..
        } => Step::Failed,
        SendState::Done(SendOutcome::Arrived) => Step::Copied,
        SendState::Done(SendOutcome::AlreadyThere) => Step::AlreadyThere,
        SendState::Done(SendOutcome::Failed(_)) => Step::Failed,
    }
}

/// Each job's state. Every change, of a field or of the whole state, goes through
/// [`slot`](Self::slot), and the meter counts the change when the slot is let go; or through
/// [`take`](Self::take) followed by a slot.
pub(crate) struct States {
    v: Vec<SendState>,
    /// Per job: its group in the meter, and the step the meter counts it in.
    group: Vec<Group>,
    counted: Vec<Step>,
    meter: MoveMeter,
}

impl States {
    pub(crate) fn new(jobs: &[SendJob], meter: MoveMeter) -> Self {
        meter.reset();
        let group = jobs.iter().map(|j| meter.plan(&j.destination, 0)).collect();
        Self {
            v: jobs.iter().map(|_| SendState::Waiting).collect(),
            group,
            counted: vec![Step::Waiting; jobs.len()],
            meter,
        }
    }

    pub(crate) fn len(&self) -> usize {
        self.v.len()
    }

    pub(crate) fn iter(&self) -> std::slice::Iter<'_, SendState> {
        self.v.iter()
    }

    /// To change `job`'s state; the meter counts the change when the slot is let go.
    pub(crate) fn slot(&mut self, job: usize) -> StateSlot<'_> {
        StateSlot { states: self, job }
    }

    /// Takes `job`'s state out to replace it (with [`slot`](Self::slot)), counted only then.
    pub(crate) fn take(&mut self, job: usize) -> SendState {
        std::mem::replace(&mut self.v[job], SendState::Waiting)
    }

    fn count(&mut self, job: usize) {
        let now = step_of(&self.v[job]);
        if now != self.counted[job] {
            self.meter.step(self.group[job], self.counted[job], now);
            self.counted[job] = now;
        }
    }

    /// In tests: every job is counted in the step its state is in (only a taken state, not yet
    /// put back, may differ, and none is held across a call to this).
    pub(crate) fn check(&self) {
        if cfg!(debug_assertions) && self.v.len() <= CHECKED_JOBS {
            debug_assert!(
                self.v
                    .iter()
                    .zip(&self.counted)
                    .all(|(s, c)| step_of(s) == *c),
                "a sent file changed step without the meter counting it"
            );
        }
    }
}

impl std::ops::Index<usize> for States {
    type Output = SendState;
    fn index(&self, job: usize) -> &SendState {
        &self.v[job]
    }
}

/// One job's state, open for a change (see [`States::slot`]).
pub(crate) struct StateSlot<'s> {
    states: &'s mut States,
    job: usize,
}

impl std::ops::Deref for StateSlot<'_> {
    type Target = SendState;
    fn deref(&self) -> &SendState {
        &self.states.v[self.job]
    }
}

impl std::ops::DerefMut for StateSlot<'_> {
    fn deref_mut(&mut self) -> &mut SendState {
        &mut self.states.v[self.job]
    }
}

impl Drop for StateSlot<'_> {
    fn drop(&mut self) {
        self.states.count(self.job);
    }
}
