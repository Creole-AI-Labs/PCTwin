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
