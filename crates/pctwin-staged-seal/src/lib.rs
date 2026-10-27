//! The seal on `pctwin_maker::HelperStaged` (fourth review, small finding).
//!
//! `HelperStaged` vouches that a package is the administrator helper's own held copy, the only
//! file whose Windows Installer signature counts. It has [`Sealed`] as its supertrait, and
//! `deny.toml` lets only `pctwin-maker` (its tests) and `pctwin-helper` depend on this crate, so
//! no other code can implement it: a structural wall checked by `cargo deny check bans` on every
//! change, in place of a test that searched the sources for implementations.

/// Implemented only by the helper's staging (and the maker check's own tests).
pub trait Sealed {}
