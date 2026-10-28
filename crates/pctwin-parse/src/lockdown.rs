//! How the parser process is walled in, said plainly, and only what is really in place
//! (Phase 2 adoption, change 1; the system's walls approved by the user, 28 October 2026).
//!
//! On every system:
//! - its own process, started by the app with no shell and no arguments, and an empty
//!   environment; its only ways in and out are the two pipes the app holds;
//! - it never opens a file, a socket or a program itself: it reads only the pieces of the one
//!   file the app hands it, and this crate's code is held to that by Clippy's
//!   `disallowed-methods` (`clippy.toml` beside this crate);
//! - every request is bounded in time and in bytes; the app ends the process when it breaks a
//!   bound, stops or speaks out of turn, and starts a fresh one for the next request.
//!
//! The system's own walls:
//! - **Windows** (made by the app's side before the parser runs a single instruction:
//!   `pctwin_helper_os::contain`): an AppContainer with no capabilities (no network, none of the
//!   person's files, low integrity); a restricted token (administrators deny-only, privileges
//!   removed), so a parser the administrator helper starts never has an administrator's rights;
//!   no programs of its own (the child-process policy); a job that ends it with the app, allows
//!   one process and caps its memory.
//! - **Linux** (by the parser itself, first thing): `no_new_privs`, Landlock with every file
//!   access handled and no rule (no file system at all), and a seccomp filter refusing `socket`,
//!   `connect`, `execve` and `execveat`.
//! - **macOS** (by the parser itself, first thing): a Seatbelt profile that denies everything
//!   (`sandbox_init`, which Apple marks deprecated but still honours, as Chromium relies on).
//!
//! What the parser reports ([`confine_self`]) is checked, never assumed: its own token on
//! Windows, the walls' own answers on Linux and macOS, and on every system a real attempt to
//! start a program (and, on Linux and macOS, to open a file and a socket), each of which must
//! fail. The app's side adds only what it set itself and the system accepted (the memory cap).

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
    /// The system refuses it the person's files (Windows: the system's own folders every app may
    /// read stay readable).
    NoPersonalFiles,
    /// The system refuses it starting another program.
    NoChildProcesses,
    /// It runs with fewer rights than the app (Windows: low integrity in a container, not
    /// elevated; macOS: a sandbox profile).
    LowRights,
    /// The system caps its memory.
    MemoryCap,
}

/// The walls in place, and those not in place on this system.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Confinement {
    pub in_place: Vec<Wall>,
    pub not_yet: Vec<Wall>,
}

/// The system's walls, whether or not this system has them in place.
pub const SYSTEM_WALLS: [Wall; 6] = [
    Wall::NoNetwork,
    Wall::NoFileSystem,
    Wall::NoPersonalFiles,
    Wall::NoChildProcesses,
    Wall::LowRights,
    Wall::MemoryCap,
];

impl Confinement {
    /// Adds a wall the app's side put in place itself (the system accepted it).
    pub fn add(&mut self, wall: Wall) {
        if !self.in_place.contains(&wall) {
            self.in_place.push(wall);
        }
        self.not_yet.retain(|w| *w != wall);
    }

    fn from(in_place: Vec<Wall>) -> Self {
        let not_yet = SYSTEM_WALLS
            .iter()
            .copied()
            .filter(|w| !in_place.contains(w))
            .collect();
        Self { in_place, not_yet }
    }
}

/// The memory the app's side lets one parser process use (a database, its log laid over, and
/// SQLite's own copy of it read from memory).
pub const MEMORY_CAP: usize = 1536 * 1024 * 1024;

/// Run first thing in the parser process: puts up the walls the parser puts up itself (Linux,
/// macOS), then reports what is really in place, each checked.
pub fn confine_self() -> Confinement {
    let mut in_place = vec![
        Wall::OwnProcess,
        Wall::ReadsOnlyWhatItIsHanded,
        Wall::BoundedRequests,
    ];
    if std::env::vars_os().next().is_none() {
        in_place.push(Wall::EmptyEnvironment);
    }
    in_place.extend(system_walls());
    if cannot_start_a_program() {
        in_place.push(Wall::NoChildProcesses);
    }
    Confinement::from(in_place)
}

/// Whether starting a program (this one, which would end at once) is refused.
fn cannot_start_a_program() -> bool {
    let Ok(me) = std::env::current_exe() else {
        return false;
    };
    #[expect(
        clippy::disallowed_methods,
        reason = "the one program start the parser tries: to prove it is refused"
    )]
    let tried = std::process::Command::new(me)
        .arg("--must-not-start")
        .stdin(std::process::Stdio::null())
        .stdout(std::process::Stdio::null())
        .stderr(std::process::Stdio::null())
        .spawn();
    match tried {
        Err(_) => true,
        Ok(mut child) => {
            let _ = child.kill();
            let _ = child.wait();
            false
        }
    }
}

/// Whether a network socket is refused (Linux, macOS: opening one at all).
#[cfg(any(target_os = "linux", target_os = "macos"))]
fn cannot_open_a_socket() -> bool {
    #[expect(
        clippy::disallowed_methods,
        reason = "the one socket the parser tries: to prove it is refused"
    )]
    let tried = std::net::TcpListener::bind("127.0.0.1:0");
    tried.is_err()
}

/// Whether opening a file every account can read is refused.
#[cfg(any(target_os = "linux", target_os = "macos"))]
fn cannot_open_a_file() -> bool {
    #[expect(
        clippy::disallowed_methods,
        reason = "the one file the parser tries: to prove it is refused"
    )]
    let tried = std::fs::File::open("/etc/hosts");
    tried.is_err()
}

/// Windows: the walls the app's side put up, read back from this process's own token.
#[cfg(windows)]
fn system_walls() -> Vec<Wall> {
    let Ok(token) = rappct::token::query_current_process_token() else {
        return Vec::new();
    };
    if !token.is_appcontainer || !token.capability_sids.is_empty() {
        return Vec::new();
    }
    let mut walls = vec![Wall::NoNetwork, Wall::NoPersonalFiles];
    if pctwin_helper_os::this_process_elevated().is_ok_and(|elevated| !elevated) {
        walls.push(Wall::LowRights);
    }
    walls
}

/// Linux: no_new_privs, Landlock with no rule, and the seccomp filter, each then checked.
#[cfg(target_os = "linux")]
fn system_walls() -> Vec<Wall> {
    use landlock::{ABI, Access, AccessFs, Ruleset, RulesetAttr, RulesetStatus};
    let mut walls = Vec::new();
    // Landlock (it sets no_new_privs first, which the seccomp filter needs too).
    let landlocked = Ruleset::default()
        .handle_access(AccessFs::from_all(ABI::V5))
        .and_then(|r| r.create())
        .and_then(|r| r.restrict_self())
        .is_ok_and(|s| s.ruleset == RulesetStatus::FullyEnforced);
    let filtered = seccomp().is_ok();
    if landlocked && cannot_open_a_file() {
        walls.push(Wall::NoFileSystem);
        walls.push(Wall::NoPersonalFiles);
    }
    if filtered && cannot_open_a_socket() {
        walls.push(Wall::NoNetwork);
    }
    walls
}

/// The seccomp filter: `socket`, `connect`, `execve` and `execveat` refused with EPERM, on every
/// thread.
#[cfg(target_os = "linux")]
fn seccomp() -> Result<(), String> {
    use seccompiler::{BpfProgram, SeccompAction, SeccompFilter, TargetArch};
    let refused = [
        libc::SYS_socket,
        libc::SYS_connect,
        libc::SYS_execve,
        libc::SYS_execveat,
    ];
    let rules = refused.iter().map(|s| (*s, Vec::new())).collect();
    let arch: TargetArch = std::env::consts::ARCH
        .try_into()
        .map_err(|e: seccompiler::BackendError| e.to_string())?;
    let filter = SeccompFilter::new(
        rules,
        SeccompAction::Allow,
        SeccompAction::Errno(libc::EPERM as u32),
        arch,
    )
    .map_err(|e| e.to_string())?;
    let program: BpfProgram = filter
        .try_into()
        .map_err(|e: seccompiler::BackendError| e.to_string())?;
    seccompiler::apply_filter_all_threads(&program).map_err(|e| e.to_string())
}

/// macOS: the Seatbelt profile, then checked.
#[cfg(target_os = "macos")]
fn system_walls() -> Vec<Wall> {
    if pctwin_helper_os::sandbox_parser().is_err() {
        return Vec::new();
    }
    let mut walls = vec![Wall::LowRights];
    if cannot_open_a_file() {
        walls.push(Wall::NoFileSystem);
        walls.push(Wall::NoPersonalFiles);
    }
    if cannot_open_a_socket() {
        walls.push(Wall::NoNetwork);
    }
    walls
}

/// Other systems: none.
#[cfg(not(any(windows, target_os = "linux", target_os = "macos")))]
fn system_walls() -> Vec<Wall> {
    Vec::new()
}
