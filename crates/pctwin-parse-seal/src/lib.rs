//! The seal on PCTwin's parsers of untrusted files (Phase 2 adoption, change 1).
//!
//! The parsers that read the inside of a file from another laptop or a download (a program's
//! signature, an installer package, a font's names, an archive's index, a database's header)
//! run only in PCTwin's locked-down parser process (`pctwin-parse`). Each such parser takes an
//! [`InChild`] as its first argument, and `deny.toml` lets only the parser process and the
//! parsers' own crates (for their tests) depend on this crate, so no other code can call a
//! parser in its own process: a wall checked by `cargo deny check bans` on every change.

/// Proof that the caller may run a parser in its own process: the parser process itself, or a
/// parser crate's own tests.
#[derive(Debug, Clone, Copy)]
pub struct InChild(());

/// The proof, for those allowed to depend on this crate.
pub const fn in_child() -> InChild {
    InChild(())
}
