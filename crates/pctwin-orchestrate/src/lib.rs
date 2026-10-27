//! More move types (Task List 2.6): the planning that sits above one plain move.
//!
//! - [`cloud`]: files in a personal cloud folder are honest by default. A file stored online only
//!   is "safe in OneDrive, comes back when you sign in" (needs action, never copied); a file whose
//!   contents are on the laptop is copied like any other, and counts as only on this laptop
//!   unless PCTwin can tell it is uploaded; work or school folders stay left out; a sync app's own
//!   sign-in state is never copied.
//! - [`local_copy`]: with a person's consent, their online-only files are also kept on the new
//!   laptop: brought down on the old laptop in batches that fit its space, sent, then made online
//!   only again, with every effect on the cloud app recorded and said plainly.
//! - [`capacity`]: a smaller new laptop. Every real volume is budgeted with everything that lands
//!   on it; what does not fit is proposed (left in the cloud, an approved offload drive, or left on
//!   the old laptop), never dropped silently; an offload drive that is not connected is never
//!   counted as on the new laptop.
//! - [`topology`]: several people and several old laptops. People are known by their account on
//!   one laptop, never by equal names or numbers; clashing names from two laptops land in their
//!   own "From <laptop>" folders; each laptop keeps its own record, journal, undo and session
//!   number; a change after approval holds only the affected items.
//! - [`rescue`]: a dead laptop's drive, read only: the drive known by its own identity, its people
//!   found from the drive itself, locked drives explained, a weak drive read carefully, and the
//!   result an ordinary plan.
//! - [`jobs`]: the plan that orders the work: child jobs with their own scopes, cycles found,
//!   bounded scheduling that never drops a job, wrong results refused, exact partial reports.
//!
//! Nothing here changes a file on the old laptop's drives: the crate's clippy fence bans every
//! way to make, write, rename or remove one. The one effect on the old laptop is the cloud app's
//! own, asked for only with the person's consent: a file brought down to keep a copy is made
//! online only again.

#![cfg_attr(not(test), forbid(clippy::disallowed_methods))]

pub mod capacity;
pub mod cloud;
pub mod jobs;
pub mod local_copy;
pub mod rescue;
pub mod topology;
