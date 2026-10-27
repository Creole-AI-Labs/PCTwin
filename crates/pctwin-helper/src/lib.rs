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

// Fifth review, finding 6: staging into any folder is for tests; a release build with it is
// refused outright.
#[cfg(all(feature = "test-staging", not(debug_assertions)))]
compile_error!("the test-staging feature is for tests only, never a release build");

pub mod accepting;
pub mod account;
pub mod broker;
pub mod client;
pub mod lock;
pub mod newlaptop;
pub mod opening;
pub mod os_probes;
pub mod person;
pub mod protocol;
pub mod read;
pub mod session;

pub use client::{Acted, Client, ClientError, HandedOver, MadeAccount, Scanned, connect};
pub use protocol::{Answer, FromHelper, Handle, Refusal, Request, Secret};
pub use protocol::{MAX_SYSTEM_ASK, MAX_SYSTEM_ASKED, MAX_SYSTEM_PATH};
pub use read::Entry;
pub use session::Session;

/// Hardens the mover's engine (this process) against a person with normal rights reading its
/// memory, before it carries anyone else's things (Security Design part C): Linux, not dumpable;
/// Windows, an access list that lets the account only see, wait for and end it; macOS, the app's
/// hardened runtime, set when it is signed.
pub fn harden_engine() -> Result<(), String> {
    pctwin_helper_os::harden_this_process().map_err(|e| e.to_string())
}
