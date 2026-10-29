//! Finishing a move (Task List 2.7 and the 2.9 deliverables "Can I actually work?", focused
//! repair and the offline recovery kit; Phase 2 plan 2.7 as adopted 26 October 2026).
//!
//! It answers three questions: what is usable on the new laptop, what still depends on the old
//! one, and what the person must do before handing the old one on.
//!
//! - [`ledger`]: one stable line per item with its result word and, kept apart, the evidence of
//!   each kind that real observations gave (plan section 3). A worker's "done" is never taken on
//!   its word: it is checked against the real state (the journal on disk, the 2.6 scheduler).
//! - [`checks`]: "Can I actually work?" checks the person picks: bounded, read only, never
//!   running anything; each result machine-checked, person-confirmed or still needs attention;
//!   what is missing becomes a focused repair request to its owner.
//! - [`repair`]: "Fix these" reruns only the unfinished items and what they need; final changes
//!   are a reviewed delta that never overwrites an edit on the new laptop; a returned laptop is a
//!   three-way comparison.
//! - [`goodbye`]: the before-wipe list: what is only on the old laptop, keys, licences, steps not
//!   finished, places not searched; each resolved by evidence or accepted as a specific loss;
//!   saved on the new laptop.
//! - [`handover`]: two separate requests (the mover's account, erasing the old laptop), each
//!   through one gate: exact device, the list clear and current, a fresh confirmation, and then
//!   undo closed for good. Both end at the system's own route; none is run here.
//! - [`cleanup`]: PCTwin's own temporary files, and its model and caches, each asked separately,
//!   by exact ownership.
//! - [`projects`]: what a file the person works with needs: told by its content first, then a
//!   generic reader the `project-files` catalogue names (its linked files and fonts).
//! - [`kit`]: the offline recovery kit, readable with no account, saying exactly what it holds.
//! - [`report`]: the shareable report, typed, private things left out unless chosen, and the
//!   check that refuses text holding anything left out; nothing is sent.
//!
//! Every word is a key: the screens hold the words in seven languages. Names the person knows
//! (their files, apps and places) are data, passed as they are.
//!
//! Nothing here changes a file on either laptop or starts a program: the crate's clippy fence
//! bans every way to write, rename or remove a file, to start a program and to open a network
//! connection. Saving PCTwin's own records is the one exception, in one place.

#![cfg_attr(not(test), forbid(clippy::disallowed_methods))]

pub mod checks;
pub mod cleanup;
pub mod goodbye;
pub mod handover;
pub mod kit;
pub mod ledger;
pub mod projects;
pub mod repair;
pub mod report;
mod store;
