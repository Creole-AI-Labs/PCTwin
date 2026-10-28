//! PCTwin's locked-down parser process (Phase 2 adoption, change 1; Security Design: trust
//! nothing that arrives).
//!
//! The insides of files from a download or another laptop are read only here, in a separate
//! process the app starts ([`Parser`]): a Windows program's or installer package's signature, a
//! Flatpak reference, the old laptop's own copy of an installer, a font's family names, a ZIP
//! archive's index and a SQLite database's header. The app asks typed requests ([`wire`]) about
//! a file it holds open, and hands over only the pieces the parser asks for; the parser opens
//! nothing itself. A parser that stops, takes too long, asks for too much or answers out of turn
//! is ended, and the file is "could not be checked" ([`NotChecked`]), never a pass.
//!
//! The readers themselves live in their own crates (`pctwin-maker`, `pctwin-readiness`) and
//! each takes a `pctwin_parse_seal::InChild`, which only this crate and those crates' own tests
//! may make (`deny.toml`): no other code can run them in its own process.
//!
//! [`lockdown`] says how the process is walled in on each system, and what waits for approval.

pub mod child;
pub mod client;
pub mod lockdown;
#[cfg(feature = "test-support")]
pub mod testing;
pub mod wire;

pub use client::{Limits, NotChecked, PARSER_PROGRAM, Parser};
pub use lockdown::{Confinement, Wall};
