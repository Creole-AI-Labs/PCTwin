//! The few system calls PCTwin's administrator helper needs, on Windows, macOS and Linux.
//!
//! The helper does the jobs that need administrator rights during a move of more than one person
//! (Security Design, part C). This crate gives it three things and nothing more:
//!
//! 1. **Starting it with the system's own prompt.** Windows: `ShellExecuteExW` with the verb
//!    `runas`, the UAC prompt ([`launch_elevated`]). Linux: `pkexec`, which asks through polkit
//!    under the policy in `packaging/linux` ([`launch_elevated`]). macOS: the helper is a launch
//!    daemon the app carries, switched on through `SMAppService` and approved by an administrator
//!    in System Settings ([`daemon`]). A refused prompt is a typed answer
//!    ([`Elevation::Refused`], [`Ended::Refused`], [`daemon::DaemonStart::NeedsApproval`]), never
//!    a crash.
//! 2. **A private channel only the mover can reach.** Windows: a named pipe with a random name,
//!    made first (`FILE_FLAG_FIRST_PIPE_INSTANCE`), refusing other computers
//!    (`PIPE_REJECT_REMOTE_CLIENTS`), whose access list admits exactly the mover's account and
//!    SYSTEM (`PipeServer`). macOS and Linux: a Unix socket inside a folder only the mover's
//!    account can enter, mode 0700 (`PrivateDir`, `SocketServer`).
//! 3. **Who is calling, on every connection** ([`Caller`]): the process ID, the account and the
//!    program's path. Windows: `GetNamedPipeClientProcessId`, the process token's user and
//!    `QueryFullProcessImageNameW`. Linux: `SO_PEERCRED` and `/proc/<pid>/exe`. macOS:
//!    `LOCAL_PEERPID`, `LOCAL_PEERCRED` and `proc_pidpath`.
//!
//! What is *allowed* (which account is the mover, which program may call, which commands exist)
//! is decided by the helper, not here: this crate reports facts and refuses only what the system
//! itself refuses.
//!
//! # The second and last place PCTwin calls the system unsafely
//!
//! The workspace forbids `unsafe` outright. The first exception is `fn fcntl_int` in
//! `pctwin-lease`. This crate is the second and last (decided by the user, 20 October 2026,
//! Security Design part C, "How the helper talks to the system"): some of these calls have no
//! safe binding anywhere. Each one sits in its own short function whose name starts `os_`, with
//! exactly one `unsafe` block, one `#[expect(unsafe_code)]` and a `SAFETY` comment, and
//! `scripts/unsafe-fence.sh` lists exactly those functions; an `unsafe` anywhere else in this
//! crate, or a second one inside a listed function, fails CI. They are:
//!
//! - Windows (`src/win.rs`): `os_shell_execute_runas`, `os_wait`, `os_create_pipe`,
//!   `os_access_list`, `os_accept`, `os_client_process_id`, `os_server_process_id`,
//!   `os_open_process`, `os_token_user`, `os_image_path`.
//! - macOS (`src/mac.rs`): `os_daemon_register`, `os_daemon_unregister`, `os_daemon_status`,
//!   `os_open_login_items`.
//! - Linux: none. `pkexec` is started through the standard library, and `SO_PEERCRED` is read
//!   through rustix's safe wrapper.
//!
//! Every other line, including all the checks, is safe Rust. Each listed function gets the
//! fresh-context, line-by-line review the Security Design requires (part J).

use std::fmt;
use std::io;
use std::path::{Path, PathBuf};

pub mod daemon;

#[cfg(windows)]
mod win;
#[cfg(windows)]
pub use win::{PipeClient, PipeName, PipeServer, caller_of};

#[cfg(unix)]
mod unix;
#[cfg(unix)]
pub use unix::{PrivateDir, SocketServer, caller_of, socket_name};

#[cfg(target_os = "linux")]
mod linux;

#[cfg(target_os = "macos")]
mod mac;

/// Why a call could not be done.
#[derive(Debug)]
#[non_exhaustive]
pub enum Error {
    /// This system does it another way (macOS starts the helper as a launch daemon, not with a
    /// prompt-and-run), or not at all yet.
    NotSupported(&'static str),
    /// The input was refused before anything was asked of the system.
    BadInput(&'static str),
    /// The system refused or failed a call. `code` is the Windows HRESULT, the Unix errno or the
    /// macOS `NSError` code.
    Os { call: &'static str, code: i64 },
    /// Reading or writing failed.
    Io(io::Error),
    /// The channel's name already exists: someone else made it first. Never joined.
    NameTaken,
    /// The calling process ended before it could be checked.
    CallerGone,
    /// The folder meant for the private channel is not private: what is wrong with it.
    NotPrivate(String),
}

impl fmt::Display for Error {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Error::NotSupported(what) => write!(f, "not supported on this system: {what}"),
            Error::BadInput(what) => write!(f, "refused input: {what}"),
            Error::Os { call, code } => write!(f, "{call} failed (code {code:#x})"),
            Error::Io(e) => write!(f, "{e}"),
            Error::NameTaken => f.write_str("the channel's name is already taken"),
            Error::CallerGone => f.write_str("the caller ended before it could be checked"),
            Error::NotPrivate(why) => write!(f, "the channel's folder is not private: {why}"),
        }
    }
}

impl std::error::Error for Error {}

impl From<io::Error> for Error {
    fn from(e: io::Error) -> Self {
        Error::Io(e)
    }
}

/// A Windows account's security identifier in its string form (`S-1-5-21-...`), checked.
///
/// It is written into the pipe's access list, so only the strict form is accepted: `S-1-`, the
/// identifier authority, then one to fifteen sub-authorities, all decimal, each fitting its size.
#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub struct Sid(String);

impl Sid {
    /// The local SYSTEM account.
    pub const SYSTEM: &'static str = "S-1-5-18";

    /// Checks `s` and keeps it.
    pub fn parse(s: &str) -> Result<Sid, Error> {
        const BAD: Error = Error::BadInput("not an account identifier (S-1-...)");
        let rest = s.strip_prefix("S-1-").ok_or(BAD)?;
        let mut parts = rest.split('-');
        let authority = parts.next().ok_or(BAD)?;
        if !is_decimal(authority) || authority.parse::<u64>().map_or(true, |a| a >= 1 << 48) {
            return Err(BAD);
        }
        let mut n = 0;
        for sub in parts {
            if !is_decimal(sub) || sub.parse::<u32>().is_err() {
                return Err(BAD);
            }
            n += 1;
        }
        if !(1..=15).contains(&n) {
            return Err(BAD);
        }
        Ok(Sid(s.to_owned()))
    }

    /// The identifier as text.
    pub fn as_str(&self) -> &str {
        &self.0
    }
}

impl fmt::Display for Sid {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(&self.0)
    }
}

fn is_decimal(s: &str) -> bool {
    !s.is_empty() && s.len() <= 15 && s.bytes().all(|b| b.is_ascii_digit())
}

/// A system account: a Windows security identifier or a Unix user ID.
#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub enum Account {
    Sid(Sid),
    Uid(u32),
}

/// Who is on the other end of a connection, as the system reports it.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Caller {
    /// The process ID.
    pub process_id: u32,
    /// The account the process runs as.
    pub account: Account,
    /// The full path of the program the process runs.
    pub program: PathBuf,
}

/// The account this process runs as.
pub fn current_account() -> Result<Account, Error> {
    #[cfg(windows)]
    {
        win::current_sid().map(Account::Sid)
    }
    #[cfg(unix)]
    {
        Ok(Account::Uid(rustix::process::getuid().as_raw()))
    }
}

/// 32 hexadecimal characters from the system's random source, for channel names.
pub fn random_token() -> Result<String, Error> {
    let mut bytes = [0u8; 16];
    getrandom::fill(&mut bytes).map_err(|e| Error::Os {
        call: "getrandom",
        code: i64::from(e.raw_os_error().unwrap_or(-1)),
    })?;
    Ok(bytes.iter().map(|b| format!("{b:02x}")).collect())
}

/// What became of asking to start the helper with administrator rights.
#[derive(Debug)]
pub enum Elevation {
    /// The helper was started. On Linux the polkit prompt is still open at this point: if it is
    /// not given, the process ends with [`Ended::Refused`] or [`Ended::NotAuthorized`].
    Started(Process),
    /// The person said no at the system's prompt (Windows), so nothing was started.
    Refused,
}

/// How a started program ended.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Ended {
    /// It ran and exited with this code.
    Exited(i32),
    /// It was ended by a signal (macOS, Linux), or Windows could not say how.
    Killed,
    /// The administrator prompt was dismissed or the password refused (pkexec's exit 126).
    Refused,
    /// No one could be asked or the account may not do this (pkexec's exit 127: no polkit
    /// agent, not authorised, or the program not named in the policy).
    NotAuthorized,
}

/// A started program: the helper, or any program a [`Launcher`] starts.
#[derive(Debug)]
pub struct Process {
    inner: ProcessInner,
}

#[derive(Debug)]
enum ProcessInner {
    #[cfg(windows)]
    Handle {
        handle: std::os::windows::io::OwnedHandle,
        id: u32,
    },
    Child {
        child: std::process::Child,
        through_pkexec: bool,
    },
}

impl Process {
    /// A program started through the standard library.
    pub fn from_child(child: std::process::Child) -> Process {
        Process::wrap(child, false)
    }

    /// A program started as `pkexec <program> ...`: its exits 126 and 127 are pkexec's answers
    /// ([`Ended::Refused`], [`Ended::NotAuthorized`]). The helper itself never exits with them.
    pub fn from_pkexec(child: std::process::Child) -> Process {
        Process::wrap(child, true)
    }

    fn wrap(child: std::process::Child, through_pkexec: bool) -> Process {
        Process {
            inner: ProcessInner::Child {
                child,
                through_pkexec,
            },
        }
    }

    #[cfg(windows)]
    pub(crate) fn from_handle(handle: std::os::windows::io::OwnedHandle, id: u32) -> Process {
        Process {
            inner: ProcessInner::Handle { handle, id },
        }
    }

    /// The process ID.
    pub fn id(&self) -> u32 {
        match &self.inner {
            #[cfg(windows)]
            ProcessInner::Handle { id, .. } => *id,
            ProcessInner::Child { child, .. } => child.id(),
        }
    }

    /// Waits until it ends.
    pub fn wait(&mut self) -> Result<Ended, Error> {
        match &mut self.inner {
            #[cfg(windows)]
            ProcessInner::Handle { handle, .. } => {
                win::wait(handle, None).map(|e| e.unwrap_or(Ended::Killed))
            }
            ProcessInner::Child {
                child,
                through_pkexec,
            } => Ok(ended(child.wait()?, *through_pkexec)),
        }
    }

    /// How it ended, or `None` while it still runs.
    pub fn try_wait(&mut self) -> Result<Option<Ended>, Error> {
        match &mut self.inner {
            #[cfg(windows)]
            ProcessInner::Handle { handle, .. } => win::wait(handle, Some(0)),
            ProcessInner::Child {
                child,
                through_pkexec,
            } => Ok(child.try_wait()?.map(|s| ended(s, *through_pkexec))),
        }
    }
}

fn ended(status: std::process::ExitStatus, through_pkexec: bool) -> Ended {
    match status.code() {
        Some(code) => exit_code(code, through_pkexec),
        None => Ended::Killed,
    }
}

/// pkexec's own answers, kept apart from the helper's exit codes.
fn exit_code(code: i32, through_pkexec: bool) -> Ended {
    match code {
        126 if through_pkexec => Ended::Refused,
        127 if through_pkexec => Ended::NotAuthorized,
        c => Ended::Exited(c),
    }
}

/// A checked request to start a program with administrator rights.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct LaunchRequest {
    program: PathBuf,
    args: Vec<String>,
}

impl LaunchRequest {
    /// Checks the program and arguments. The program must be a full path on this computer (on
    /// Windows a drive path, never a network share or a bare name the system would search
    /// for); no argument may hold a NUL character.
    pub fn new(program: &Path, args: &[&str]) -> Result<LaunchRequest, Error> {
        if !is_local_absolute(program) {
            return Err(Error::BadInput(
                "the program must be a full path on this computer",
            ));
        }
        if program.to_string_lossy().contains('\0') || args.iter().any(|a| a.contains('\0')) {
            return Err(Error::BadInput(
                "a NUL character in the program or arguments",
            ));
        }
        Ok(LaunchRequest {
            program: program.to_path_buf(),
            args: args.iter().map(|a| (*a).to_owned()).collect(),
        })
    }

    /// The program to start.
    pub fn program(&self) -> &Path {
        &self.program
    }

    /// Its arguments.
    pub fn args(&self) -> &[String] {
        &self.args
    }

    /// The arguments as one Windows command line, quoted so the program's standard parser
    /// (`CommandLineToArgvW`, the C and Rust runtimes) gives back exactly [`args`](Self::args).
    pub fn windows_parameters(&self) -> String {
        let quoted: Vec<String> = self.args.iter().map(|a| quote_windows_arg(a)).collect();
        quoted.join(" ")
    }
}

#[cfg(windows)]
fn is_local_absolute(p: &Path) -> bool {
    use std::path::{Component, Prefix};
    let mut c = p.components();
    matches!(c.next(), Some(Component::Prefix(pre)) if matches!(pre.kind(), Prefix::Disk(_)))
        && matches!(c.next(), Some(Component::RootDir))
}

#[cfg(not(windows))]
fn is_local_absolute(p: &Path) -> bool {
    p.is_absolute()
}

/// Quotes one argument for a Windows command line: unchanged when it holds no space, tab, line
/// break or quote; otherwise in quotes, with every quote escaped and the backslashes before a
/// quote (or before the closing quote) doubled.
fn quote_windows_arg(arg: &str) -> String {
    if !arg.is_empty() && !arg.contains([' ', '\t', '\n', '\x0b', '"']) {
        return arg.to_owned();
    }
    let mut out = String::with_capacity(arg.len() + 2);
    out.push('"');
    let mut backslashes = 0usize;
    for ch in arg.chars() {
        match ch {
            '\\' => backslashes += 1,
            '"' => {
                out.extend(std::iter::repeat_n('\\', backslashes * 2 + 1));
                out.push('"');
                backslashes = 0;
            }
            _ => {
                out.extend(std::iter::repeat_n('\\', backslashes));
                out.push(ch);
                backslashes = 0;
            }
        }
    }
    out.extend(std::iter::repeat_n('\\', backslashes * 2));
    out.push('"');
    out
}

/// Why a [`Launcher`] did not start the program.
#[derive(Debug)]
pub enum LaunchError {
    /// The person said no at the system's prompt.
    Cancelled,
    /// Anything else.
    Failed(Error),
}

/// Starts a program with administrator rights. [`SystemLauncher`] asks the system; tests and
/// other callers may put another in its place.
pub trait Launcher {
    fn start(&self, request: &LaunchRequest) -> Result<Process, LaunchError>;
}

/// The system's own prompt: UAC on Windows, polkit through `/usr/bin/pkexec` on Linux. macOS
/// has none of this kind; there the helper is a launch daemon ([`daemon`]).
#[derive(Clone, Copy, Debug, Default)]
pub struct SystemLauncher;

impl Launcher for SystemLauncher {
    fn start(&self, request: &LaunchRequest) -> Result<Process, LaunchError> {
        #[cfg(windows)]
        {
            win::start_runas(request)
        }
        #[cfg(target_os = "linux")]
        {
            linux::start_pkexec(request)
        }
        #[cfg(not(any(windows, target_os = "linux")))]
        {
            let _ = request;
            Err(LaunchError::Failed(Error::NotSupported(
                "macOS starts the helper as a launch daemon (pctwin_helper_os::daemon)",
            )))
        }
    }
}

/// Starts `program` with administrator rights through the system's own prompt.
pub fn launch_elevated(program: &Path, args: &[&str]) -> Result<Elevation, Error> {
    launch_elevated_with(&SystemLauncher, program, args)
}

/// [`launch_elevated`] through the given launcher.
pub fn launch_elevated_with(
    launcher: &impl Launcher,
    program: &Path,
    args: &[&str],
) -> Result<Elevation, Error> {
    let request = LaunchRequest::new(program, args)?;
    match launcher.start(&request) {
        Ok(process) => Ok(Elevation::Started(process)),
        Err(LaunchError::Cancelled) => Ok(Elevation::Refused),
        Err(LaunchError::Failed(e)) => Err(e),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn sid_accepts_only_the_strict_form() {
        for good in [
            "S-1-5-18",
            "S-1-5-21-3623811015-3361044348-30300820-1013",
            "S-1-5-32-544",
            "S-1-281474976710655-1",
            "S-1-5-1-2-3-4-5-6-7-8-9-10-11-12-13-14-15",
        ] {
            assert!(Sid::parse(good).is_ok(), "{good}");
        }
        for bad in [
            "",
            "S-1-",
            "S-1-5",
            "S-1-5-",
            "S-2-5-18",
            "s-1-5-18",
            "S-1-5-18)(A;;GA;;;WD",
            "S-1-5-18;",
            "S-1-5--18",
            "S-1-5-+18",
            "S-1-5-4294967296",
            "S-1-281474976710656-1",
            "S-1-5-1-2-3-4-5-6-7-8-9-10-11-12-13-14-15-16",
            "WD",
            "SY",
        ] {
            assert!(Sid::parse(bad).is_err(), "{bad:?} must be refused");
        }
    }

    #[test]
    fn quoting_follows_the_windows_rules() {
        let cases: &[(&str, &str)] = &[
            ("plain", "plain"),
            ("", "\"\""),
            ("two words", "\"two words\""),
            ("tab\there", "\"tab\there\""),
            ("say \"hi\"", "\"say \\\"hi\\\"\""),
            ("C:\\path\\", "C:\\path\\"),
            ("C:\\my path\\", "\"C:\\my path\\\\\""),
            ("a\\\"b", "\"a\\\\\\\"b\""),
            ("a\\b c", "\"a\\b c\""),
        ];
        for (arg, want) in cases {
            assert_eq!(quote_windows_arg(arg), *want, "{arg:?}");
        }
    }

    #[test]
    fn pkexec_answers_are_kept_apart_from_exit_codes() {
        assert_eq!(exit_code(126, true), Ended::Refused);
        assert_eq!(exit_code(127, true), Ended::NotAuthorized);
        assert_eq!(exit_code(126, false), Ended::Exited(126));
        assert_eq!(exit_code(127, false), Ended::Exited(127));
        assert_eq!(exit_code(0, true), Ended::Exited(0));
        assert_eq!(exit_code(1, true), Ended::Exited(1));
    }

    #[test]
    fn random_tokens_are_32_hex_and_differ() {
        let a = random_token().unwrap();
        let b = random_token().unwrap();
        assert_eq!(a.len(), 32);
        assert!(
            a.bytes()
                .all(|c| c.is_ascii_hexdigit() && !c.is_ascii_uppercase())
        );
        assert_ne!(a, b);
    }
}
