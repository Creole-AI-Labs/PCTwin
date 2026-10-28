//! How the parser process is locked down, said plainly and checked where it can be.
//!
//! In place on every system now (no new system calls):
//! - its own process, started by the app with no shell and no arguments, and an empty
//!   environment; its only ways in and out are the two pipes the app holds;
//! - it never opens a file, a socket or a program: it reads only the pieces of the one file the
//!   app hands it, asked for one at a time, and this crate's code is held to that by Clippy's
//!   `disallowed-methods` (`clippy.toml` beside this crate);
//! - every request is bounded in time and in the bytes it may ask for, and the app kills the
//!   process when it breaks a bound, stops or says anything out of turn, and starts a fresh one
//!   for the next request.
//!
//! Designed, behind [`confine_self`], not yet switched on: each needs a system call PCTwin has
//! not had approved (Security Design: the unsafe fence), so it is listed for the user's approval:
//! - Windows: started in an AppContainer with no capabilities (no network, no access to the
//!   person's files, low integrity), inside a job object that kills it with the app, allows it
//!   no child process and caps its memory;
//! - Linux: Landlock with no file rules (no file system at all) and a seccomp filter that
//!   refuses sockets and starting programs, after `no_new_privs`;
//! - macOS: a Seatbelt profile that denies everything but the two pipes (`sandbox_init`, which
//!   Apple marks deprecated but still honours; Chromium relies on it the same way).

use serde::{Deserialize, Serialize};

/// One wall around the parser process.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum Wall {
    /// A process of its own, started with no shell and no arguments.
    OwnProcess,
    /// Nothing of the app's environment reaches it.
    EmptyEnvironment,
    /// It reads only what the app hands it; it opens no file, socket or program itself.
    ReadsOnlyWhatItIsHanded,
    /// Each request is bounded in time and bytes; breaking a bound ends the process.
    BoundedRequests,
    /// The system refuses it the network.
    NoNetwork,
    /// The system refuses it the file system.
    NoFileSystem,
    /// The system refuses it starting another program.
    NoChildProcesses,
    /// It runs with fewer rights than the app (low integrity, or a sandbox profile).
    LowRights,
    /// The system caps its memory.
    MemoryCap,
}

/// The walls in place, and those designed but waiting for approval.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Confinement {
    pub in_place: Vec<Wall>,
    pub not_yet: Vec<Wall>,
}

/// The walls the system would add, each waiting for the user's approval of its system call.
pub const WAITING_FOR_APPROVAL: [Wall; 5] = [
    Wall::NoNetwork,
    Wall::NoFileSystem,
    Wall::NoChildProcesses,
    Wall::LowRights,
    Wall::MemoryCap,
];

/// Run first thing in the parser process: the seam where the system's own walls go once
/// approved. It reports only what is actually true of this process.
pub fn confine_self() -> Confinement {
    let mut in_place = vec![
        Wall::OwnProcess,
        Wall::ReadsOnlyWhatItIsHanded,
        Wall::BoundedRequests,
    ];
    if std::env::vars_os().next().is_none() {
        in_place.push(Wall::EmptyEnvironment);
    }
    Confinement {
        in_place,
        not_yet: WAITING_FOR_APPROVAL.to_vec(),
    }
}
