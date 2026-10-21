//! PCTwin's administrator helper (Security Design part C, "The helper in Release 1"): a separate
//! program started with the system's own administrator prompt, only when someone besides the
//! mover is moving. On the old laptop it does two things, read-only: [`Request::ScanPerson`]
//! finds an approved person's files and keeps them in its own plan, handing the app their
//! totals, a handle per file and the files as record items, which travel in the move record and
//! are never shown to the mover (decided 21 October 2026); [`Request::ReadBlocks`] reads a file
//! of that plan, by handle.
//!
//! It never writes, deletes or changes permissions on the old laptop, never follows a link,
//! never recalls a file stored online only, never takes a path from the app, serves one app for
//! one move and ends with it. Its private channel and the check of who is calling are in
//! `pctwin-helper-os`; everything here is safe Rust.

pub mod client;
pub mod lock;
pub mod person;
pub mod protocol;
pub mod read;
pub mod session;

pub use client::{Client, ClientError, Scanned};
pub use protocol::{Answer, FromHelper, Handle, Refusal, Request};
pub use read::Entry;
pub use session::Session;
