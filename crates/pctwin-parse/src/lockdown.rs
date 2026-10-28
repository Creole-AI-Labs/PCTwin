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
//! Designed, behind [`confine_self`], switched off: each needs system calls the user has not
//! approved yet (Security Design: the unsafe fence). Exactly what each would need:
//! - **Windows, started by the app or the helper** (a new fenced function in
//!   `pctwin-helper-os`, or the crates named): an AppContainer with no capabilities, so no
//!   network, no access to the person's files, low integrity: `CreateAppContainerProfile`,
//!   `DeriveAppContainerSidFromAppContainerName`, `InitializeProcThreadAttributeList`,
//!   `UpdateProcThreadAttribute(PROC_THREAD_ATTRIBUTE_SECURITY_CAPABILITIES)`,
//!   `CreateProcessW` with `EXTENDED_STARTUPINFO_PRESENT` (crate `rappct`, MIT, wraps these);
//!   and a job object: `CreateJobObjectW`, `SetInformationJobObject`
//!   (`JOB_OBJECT_LIMIT_KILL_ON_JOB_CLOSE`, `JOB_OBJECT_LIMIT_ACTIVE_PROCESS` = 1,
//!   `JOB_OBJECT_LIMIT_PROCESS_MEMORY`), `AssignProcessToJobObject` (crate `win32job`, MIT or
//!   Apache-2.0). From the elevated helper, also a restricted token (`CreateRestrictedToken` with
//!   the administrators group made deny-only), so the parser never runs with an administrator's
//!   rights.
//! - **Linux, in the parser itself at its start**: `prctl(PR_SET_NO_NEW_PRIVS)`, Landlock
//!   (`landlock_create_ruleset`, `landlock_restrict_self` with every file access handled and no
//!   rule: crate `landlock`, MIT or Apache-2.0), and a seccomp filter refusing `socket`,
//!   `connect`, `execve` and `execveat` (crate `seccompiler`, Apache-2.0).
//! - **macOS, in the parser itself at its start**: `sandbox_init` with a profile that denies
//!   everything but reading and writing the two pipes it was started with (Apple marks it
//!   deprecated but still honours it; Chromium relies on it the same way). One fenced function,
//!   no new crate.

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
