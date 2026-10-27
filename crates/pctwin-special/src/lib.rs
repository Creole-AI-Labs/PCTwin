//! Phase 2.5, special categories (Task List 2.5; the 2.5 plan in `docs/plans/PCTwin_Phase_2_Bots/`,
//! as adopted with its `ADOPTION.md`, changes 1 to 8).
//!
//! **The move calls [`api`]**: [`api::plan`] on the old laptop (a person's home and 2.1's app list
//! in; components with class, step and plain-words reason out), [`api::approve`] and
//! [`api::capture_planned`] there, then [`api::apply`] on the new laptop (through the safety gate
//! and the effect journal, result word out), [`api::undo`], [`api::result_without_landing`] and
//! [`api::before_wipe`]. That API is stable: it changes only by adding.
//!
//! **No hard-coded apps.** Every app, folder and file name in the plans is an example. Code
//! exists per mechanism only:
//! - [`roots`]: every per-person app data root each system has, and each folder there tied to an
//!   app by the system's own ownership facts; unowned folders are still found;
//! - [`sniff`] and [`content`]: what each file is, from its bytes and inside (database headers
//!   and table shapes, key armour, protected blobs, mail formats, model weights, caches), never
//!   from its name;
//! - [`classify`]: components (files that only work together move whole) with their custody
//!   ([`component::Custody`]: P, S, E, D or unresolved) and evidence; nothing found is dropped;
//! - [`catalogue`]: the signed `special` catalogue only enriches (names, export routes, typed path
//!   fix-ups, cross-system places, stricter exceptions); with no rows everything is classified;
//! - [`capture`]: consistent capture (SQLite's online backup from a steady snapshot; stores only
//!   while their app is closed), on the old laptop, read-only;
//! - [`land`]: landing through the safety gate and the journal, never over an existing folder,
//!   secrets owner-only, crash reconcile, undo;
//! - [`select`] and [`export`]: secrets chosen one by one; a readable password export kept
//!   owner-only and removed only after a verified import;
//! - [`handoff`]: the person's step for each component, pages and apps opened as the person, the
//!   app's own export tool only from a signed row and only if the maker check passes.
//!
//! **Never**: no app-bound encryption, DPAPI or keychain is bypassed; no cookie, session or token
//! is copied; nothing is sent, connected, launched or run as a check; the old laptop is only read.

pub mod api;
pub mod capture;
pub mod catalogue;
pub mod classify;
pub mod component;
pub mod content;
pub mod export;
pub mod handoff;
pub mod land;
pub mod roots;
pub mod select;
pub mod sniff;
