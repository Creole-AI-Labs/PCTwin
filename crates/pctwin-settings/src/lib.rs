//! PCTwin settings and personalisation (Task List 2.4; Phase 2.4 plan, as adopted on 26 October
//! 2026, and the user's two decisions of 27 October 2026).
//!
//! **Discovery first.** Different people have different things, so no list decides what moves:
//! - [`found`] finds every setting in the person's own settings stores (the person's registry
//!   trees and personalisation keys on Windows, every preferences domain on a Mac, every value the
//!   person set in GNOME's dconf);
//! - [`classify`] decides, by the KIND of value and never by app name, what may not move
//!   (security, privacy and policy, credentials, machine and account identities, caches, window
//!   places, device paths, programs to start, paths to things not moved) and says why;
//! - [`generic`] plans the rest: between the same kind of system copied exactly through the
//!   mechanisms, journaled by the executor with compare-and-write undo; anything that cannot move
//!   is "found, not moved" with its reason, never dropped silently.
//!
//! **The catalogue enriches.** [`catalogue`] is versioned data (format number, every row checked
//! when read; signed delivery with the recipe channel later): translations between systems
//! ([`map`], with one declared outcome each: exact, equivalent, guided, unsupported,
//! not-applicable), better routes (the default browser's handoff, add-only keyboard layouts),
//! exceptions and friendly names. [`codec`] and [`mechanism`] are the generic machinery rows use.
//!
//! [`store`] reads and writes both on one laptop; [`ownership`] keeps one writer per domain.
//!
//! **Nothing that could run or fetch is written (fourth review, finding F1).** Writing is scoped
//! to the one allow-list discovery reads ([`found::Address::writable`]); the general rule
//! ([`classify::run_or_fetch`]) keeps back, by the value's shape and the key's kind and never by
//! app name, every value that is or holds a path or address in any spelling (another computer,
//! a web or app address, an environment variable, a relative place, a key named like a file), a
//! command line, and anything a catalogue action-hint row marks as run or loaded; only one full
//! local path to data is pointed at the moved file. Every write is checked again on the new
//! laptop ([`generic::may_write`]).
//!
//! **What is still not covered (the 2.4 plan's residual, written down on purpose).** Copying a
//! third-party app's settings generically can carry an app-specific command that no general
//! rule recognises: a plain word such as `calc` that one app happens to run, a value it builds
//! into a command line itself, or bytes it decodes into a path. The classifier keeps back
//! everything run-able it can see; the report says the rest moved as data, and the catalogue's
//! action-hint and exception rows close such cases one app at a time as they are found.

pub mod apps;
pub mod catalogue;
pub mod classify;
pub mod codec;
pub mod confirm;
pub mod files;
pub mod found;
pub mod generic;
pub mod gvariant;
pub mod handoff;
pub mod map;
pub mod mechanism;
pub mod ownership;
pub mod plist;
pub mod registry;
pub mod runner;
pub mod store;
pub mod value;
pub mod wifi;

pub use catalogue::{Catalogue, Route, SettingId, System};
pub use map::{
    Action, Arrival, Capture, Choice, Choices, Outcome, PathMap, People, PlanError, Planned,
    Reading, SettingsPlan, Target, plan,
};
pub use store::{SettingsAdapter, Store};
