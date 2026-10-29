//! The Phase 2 deliverables made explicit (Task List 2.9) that need their own commands: the
//! screens' side of "Can I actually work?" checks ([`work`]), repair only what failed
//! ([`repair`]), changes during the move ([`changes`]), PCTwin's own update and uninstall waiting
//! for a safe stopping point ([`safe_stop`]) and the offline recovery kit ([`kit`]).
//!
//! The other 2.9 deliverables travel with the plan the person approves (`plan-view.json`): the
//! "before we change anything" preview's internet needs and busiest-moment room, the undo
//! explanation of every change and the compatibility record of every app
//! ([`crate::fullmove::RebuildPlanView`]).
//!
//! Every value sent is a word key the screens translate, a number, or a name to show exactly as
//! it is. No path the screens could act on, no command, no secret. Lists are bounded and what is
//! left out is counted.

pub mod changes;
#[cfg(test)]
pub mod examples;
pub mod kit;
pub mod repair;
pub mod safe_stop;
pub mod work;
