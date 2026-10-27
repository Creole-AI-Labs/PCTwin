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
//!    SYSTEM (`PipeServer`). macOS and Linux: a socket only the mover's account can connect to
//!    (mode 0600, owned by the mover), inside a folder only the helper's own account (root) can
//!    write (mode 0711), so nothing in it can be swapped under the helper (`ChannelDir`,
//!    `SocketServer`); the mover's end checks the socket is served by root before it sends
//!    anything (`connect`).
//! 3. **Who is calling, on every connection** ([`Caller`]): the account, which is
//!    authoritative, and the process ID and program path, which are a hint for the record.
//!    Windows: `GetNamedPipeClientProcessId`, the process token's user and
//!    `QueryFullProcessImageNameW`. Linux: `SO_PEERCRED` and `/proc/<pid>/exe`. macOS:
//!    `LOCAL_PEERPID`, `LOCAL_PEERCRED` and `proc_pidpath`.
//!
//! **What the helper may decide on.** Only two facts are strong enough to authorise anything:
//! the caller's account, which the system records when the connection is made and which cannot
//! change afterwards, and the private channel itself (only the mover's account and SYSTEM or
//! root can reach it). The program path is read afterwards by process ID, so a caller that
//! starts another program, or ends and has its ID reused, can change it; it is
//! [`Caller::program_hint`], kept for the record and the report, never for allowing anything.
//! Tying a request to a signed program is a later step (ADR 0003: macOS `LOCAL_PEERTOKEN` with
//! a code-signing requirement, Linux `SO_PEERPIDFD`). Which account is the mover and which
//! commands exist are decided by the helper, not here.
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
//!   `os_open_process`, `os_process_token`, `os_client_token`, `os_token_user`,
//!   `os_image_path`, `os_volume_drive_type`, `os_native_machine`, `os_net_user_add`,
//!   `os_expire_password`, `os_create_dir_with_sddl`, `os_create_profile`,
//!   `os_set_process_sddl`, `os_windows_dir`, `os_machine_text`; and, added 27 October 2026
//!   for the join of 2.2 and 2.3 (awaiting the user's approval of the fence's two new members),
//!   `os_token_elevated` (whether this program is elevated: an app is never started from one)
//!   and `os_session_on_screen` (whether the helper's session is still on the screen: switching
//!   person ends the broker's grants); and, added 27 October 2026 after the second review
//!   (awaiting the user's approval), `os_win_verify_trust` (Windows' own signature check on a
//!   held file, replacing PowerShell, whose answer code in the person's folders could replace).
//! - macOS (`src/mac.rs`): `os_daemon_register`, `os_daemon_unregister`, `os_daemon_status`,
//!   `os_open_login_items`, `os_od_create_user`, `os_authorization_holds`.
//! - Linux: none. `pkexec` is started through the standard library, and `SO_PEERCRED` is read
//!   through rustix's safe wrapper.
//!
//! Every other line, including all the checks, is safe Rust. Each listed function gets the
//! fresh-context, line-by-line review the Security Design requires (part J).

use std::fmt;
use std::io;
use std::path::{Component, Path, PathBuf};

pub mod approval;
pub mod chip;
pub mod daemon;
pub mod holding;
pub mod signin;

#[cfg(windows)]
mod win;
#[cfg(windows)]
pub use win::{
    FileTrust, PipeClient, PipeName, PipeServer, SystemFolder, admin_only_folder, caller_of,
    clean_command, defender_scanner, minimal_env, program_folder, system_folder, verify_file_trust,
};

#[cfg(unix)]
mod unix;
#[cfg(unix)]
pub use unix::{ChannelDir, SocketServer, caller_of, connect, socket_name};

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
    /// The channel's folder is not as it must be: what is wrong with it.
    NotPrivate(String),
    /// The other end of the channel is not the account it must be (a socket or pipe planted by
    /// someone else). Nothing was sent.
    UnexpectedPeer,
    /// An account with that name already exists: it is never changed.
    AlreadyExists,
    /// The system refused: not run with an administrator's rights.
    AccessDenied,
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
            Error::NotPrivate(why) => write!(f, "the channel's folder is not safe: {why}"),
            Error::UnexpectedPeer => f.write_str("the other end of the channel is not the helper"),
            Error::AlreadyExists => f.write_str("an account with that name already exists"),
            Error::AccessDenied => f.write_str("only an administrator can do that"),
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
    /// The process ID the system recorded for the connection.
    pub process_id: u32,
    /// The account the process ran as when it connected. The only fact here to authorise on.
    pub account: Account,
    /// The program that process ID runs, read after the connection was made. A hint for the
    /// record and the report only: a process that starts another program, or ends and has its
    /// ID reused, changes it. Never authorise on it.
    pub program_hint: PathBuf,
}

/// Whether `name` may be a new account's name on every supported system: 1 to 20 of `a-z`,
/// `0-9`, `-` and `_`, starting with a letter (stricter than any one system, so the same name
/// works on all, and nothing in it can be read as an option or a path).
pub fn is_account_name(name: &str) -> bool {
    let mut chars = name.chars();
    matches!(chars.next(), Some('a'..='z'))
        && name.len() <= 20
        && chars.all(|c| matches!(c, 'a'..='z' | '0'..='9' | '-' | '_'))
}

/// Whether `full_name` may be shown as a new account's name: 1 to 64 characters, none of them a
/// control character, `:` or `,` (which separate fields in the systems' own account lists).
pub fn is_full_name(full_name: &str) -> bool {
    let count = full_name.chars().count();
    (1..=64).contains(&count)
        && !full_name.trim().is_empty()
        && full_name
            .chars()
            .all(|c| !c.is_control() && c != ':' && c != ',')
}

/// Makes a standard (never administrator) account `name`, shown as `full_name`, with the
/// temporary `password`, which must be changed at its first sign-in (Security Design part C,
/// `CreateAccount`). The password reaches the system in memory only, never on a command line.
/// An existing account is never changed ([`Error::AlreadyExists`]). Needs an administrator's
/// rights.
pub fn add_standard_account(name: &str, full_name: &str, password: &str) -> Result<Account, Error> {
    if !is_account_name(name) {
        return Err(Error::BadInput("an account name of a-z, 0-9, - and _"));
    }
    if !is_full_name(full_name) {
        return Err(Error::BadInput("a shown name of 1 to 64 plain characters"));
    }
    if password.len() < 12 || password.chars().any(char::is_control) {
        return Err(Error::BadInput(
            "a temporary password of at least 12 characters",
        ));
    }
    #[cfg(windows)]
    return win::add_standard_account(name, full_name, password)
        .and_then(|sid| Sid::parse(&sid))
        .map(Account::Sid);
    #[cfg(target_os = "linux")]
    return linux::add_standard_account(name, full_name, password).map(Account::Uid);
    #[cfg(target_os = "macos")]
    return mac::add_standard_account(name, full_name, password).map(Account::Uid);
    #[cfg(not(any(windows, target_os = "linux", target_os = "macos")))]
    Err(Error::NotSupported("making accounts"))
}

/// macOS: gives the new account `name` a Secure Token through the administrator's own approval
/// in the Mac's own window, its `password` given to the tool's prompt, never a command line;
/// `Ok(true)` only when the system then says the token is on. Elsewhere there is no such token:
/// `Ok(true)`.
pub fn secure_token_on(name: &str, password: &str) -> Result<bool, Error> {
    if !is_account_name(name) {
        return Err(Error::BadInput("an account name of a-z, 0-9, - and _"));
    }
    #[cfg(target_os = "macos")]
    return mac::secure_token_on(name, password);
    #[cfg(not(target_os = "macos"))]
    {
        let _ = password;
        Ok(true)
    }
}

/// Hardens this process against a person with normal rights reading its memory (Security Design
/// part C: the mover's engine while it carries other people's things in a move of several
/// people). Linux: not dumpable, so no debugger can attach, `/proc/<pid>/mem` is closed and no
/// core file is written. Windows: an access list that lets the account only see the process,
/// wait for it and end it. macOS: the app's hardened runtime without `get-task-allow`, set when
/// the app is signed, so nothing is done here.
pub fn harden_this_process() -> Result<(), Error> {
    #[cfg(windows)]
    {
        win::harden_process()
    }
    #[cfg(target_os = "linux")]
    {
        rustix::process::set_dumpable_behavior(rustix::process::DumpableBehavior::NotDumpable)
            .map_err(|e| Error::Io(e.into()))
    }
    #[cfg(not(any(windows, target_os = "linux")))]
    {
        Ok(())
    }
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

/// Whether this program runs with an administrator's full rights: Windows, its token is elevated;
/// Mac and Linux, it runs as root. (The 2.7 launch check refuses to start an app from such a
/// program: an app is always started as the person; acceptance case T22-020.)
pub fn this_process_elevated() -> Result<bool, Error> {
    #[cfg(windows)]
    {
        win::this_process_elevated()
    }
    #[cfg(unix)]
    {
        Ok(rustix::process::geteuid().is_root())
    }
}

/// Windows: whether this program's own session is the one on the screen and keyboard (after the
/// person switches to another account, it is not). Elsewhere it cannot be told here: `Err`.
pub fn session_on_screen() -> Result<bool, Error> {
    #[cfg(windows)]
    {
        win::session_on_screen()
    }
    #[cfg(not(windows))]
    {
        Err(Error::BadInput(
            "this system cannot say which session is on the screen",
        ))
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
    /// pkexec's exit 127: the account is not authorised, or there was no polkit agent to ask.
    /// (A program the policy does not name is not refused for that: polkit falls back to its
    /// generic administrator action and still asks.)
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
#[derive(Clone, PartialEq, Eq)]
pub struct LaunchRequest {
    program: PathBuf,
    args: Vec<String>,
    /// Given to the program on its standard input (Linux: a secret kept off the command line,
    /// which other programs can read). Not used on Windows, where an elevated program's command
    /// line cannot be read by a program that is not.
    input: Option<Vec<u8>>,
}

impl std::fmt::Debug for LaunchRequest {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("LaunchRequest")
            .field("program", &self.program)
            .field("args", &self.args)
            .field("input", &self.input.as_ref().map(|_| "(hidden)"))
            .finish()
    }
}

/// Linux: where the package installs the helper programs (`pctwin-helper`, `pctwin-handover`),
/// named by the polkit policy (packaging/linux).
pub const LINUX_HELPER_DIR: &str = "/usr/lib/pctwin";

impl LaunchRequest {
    /// Checks the program and arguments.
    ///
    /// The path is first checked as text: a full path (on Windows a drive path, never a network
    /// share, a device path or a bare name the system would search for), with no `.` or `..`
    /// part and, on Windows, no `:` after the drive (an alternate data stream). It is then
    /// resolved by the system through an open handle (links, junctions, mapped and substituted
    /// drives and short 8.3 names all resolved), and the resolved path must be an existing file
    /// on this computer: on Windows a fixed drive, never a network, removable or optical one.
    /// The resolved path is the one started. No argument may hold a NUL character.
    pub fn new(program: &Path, args: &[&str]) -> Result<LaunchRequest, Error> {
        if program.to_string_lossy().contains('\0') || args.iter().any(|a| a.contains('\0')) {
            return Err(Error::BadInput(
                "a NUL character in the program or arguments",
            ));
        }
        check_program_text(program)?;
        let program = resolve_local_program(program)?;
        Ok(LaunchRequest {
            program,
            args: args.iter().map(|a| (*a).to_owned()).collect(),
            input: None,
        })
    }

    /// The same request, with `input` given on the program's standard input.
    pub fn with_input(mut self, input: &[u8]) -> LaunchRequest {
        self.input = Some(input.to_vec());
        self
    }

    pub fn input(&self) -> Option<&[u8]> {
        self.input.as_deref()
    }

    /// The installed helper program `file_name`, started with `args`: on Linux in
    /// [`LINUX_HELPER_DIR`], the one place its polkit policy lets `pkexec` run it from (the app
    /// itself is in /usr/bin); elsewhere beside the app (Windows: in Program Files; a Mac: in the
    /// app's Contents/MacOS).
    pub fn installed_helper(file_name: &str, args: &[&str]) -> Result<LaunchRequest, Error> {
        if cfg!(target_os = "linux") {
            let mut parts = Path::new(file_name).components();
            let plain =
                matches!(parts.next(), Some(Component::Normal(_))) && parts.next().is_none();
            if !plain {
                return Err(Error::BadInput("not a plain file name"));
            }
            LaunchRequest::new(&Path::new(LINUX_HELPER_DIR).join(file_name), args)
        } else {
            LaunchRequest::beside_current_exe(file_name, args)
        }
    }

    /// The program `file_name` in the folder this program was started from (the app's own
    /// install folder), checked as [`new`](Self::new) checks any path.
    pub fn beside_current_exe(file_name: &str, args: &[&str]) -> Result<LaunchRequest, Error> {
        let mut parts = Path::new(file_name).components();
        let plain = matches!(parts.next(), Some(Component::Normal(_))) && parts.next().is_none();
        if !plain || file_name.contains(':') {
            return Err(Error::BadInput("not a plain file name"));
        }
        let exe = std::env::current_exe()?;
        let folder = exe
            .parent()
            .ok_or(Error::BadInput("this program has no folder"))?;
        LaunchRequest::new(&folder.join(file_name), args)
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

/// The text checks, before the system is asked anything.
fn check_program_text(p: &Path) -> Result<(), Error> {
    if !is_local_absolute(p) {
        return Err(Error::BadInput(
            "the program must be a full path on this computer",
        ));
    }
    // Read from the text itself: `Path::components` quietly drops a `.` in the middle.
    let text = p.to_string_lossy();
    let separators: &[char] = if cfg!(windows) { &['/', '\\'] } else { &['/'] };
    if text
        .split(separators)
        .any(|part| part == "." || part == "..")
    {
        return Err(Error::BadInput("the program's path may not hold . or .."));
    }
    for c in p.components() {
        match c {
            Component::Normal(n) if cfg!(windows) && n.to_string_lossy().contains(':') => {
                return Err(Error::BadInput(
                    "the program's path may not name a data stream",
                ));
            }
            _ => {}
        }
    }
    Ok(())
}

#[cfg(windows)]
fn is_local_absolute(p: &Path) -> bool {
    use std::path::Prefix;
    let mut c = p.components();
    matches!(c.next(), Some(Component::Prefix(pre)) if matches!(pre.kind(), Prefix::Disk(_)))
        && matches!(c.next(), Some(Component::RootDir))
}

#[cfg(not(windows))]
fn is_local_absolute(p: &Path) -> bool {
    p.is_absolute()
}

/// Resolves the path through the system and checks what it really names.
#[cfg(windows)]
fn resolve_local_program(p: &Path) -> Result<PathBuf, Error> {
    let resolved = std::fs::canonicalize(p)?;
    let local = win::local_fixed_drive_path(&resolved)?;
    if !std::fs::metadata(&local)?.is_file() {
        return Err(Error::BadInput("the program is not a file"));
    }
    Ok(local)
}

/// Resolves the path through the system and checks what it really names. (Whether a Unix path
/// lies on a network file system is not checked here; pkexec starts only the exact path the
/// polkit policy names, and macOS starts only the program inside the signed app bundle.)
#[cfg(not(windows))]
fn resolve_local_program(p: &Path) -> Result<PathBuf, Error> {
    let resolved = std::fs::canonicalize(p)?;
    if !std::fs::metadata(&resolved)?.is_file() {
        return Err(Error::BadInput("the program is not a file"));
    }
    Ok(resolved)
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

/// After an account's record is made (`made`, its user ID or SID): the rest of making it
/// (`rest`: its password, its home, its first-sign-in rule). If any of that fails, the account
/// PCTwin itself just made is taken away again (`undo`, given that very ID, never another
/// account), so it is never left half-made for a retry to find "already there"; the first
/// failure is what is reported.
pub fn finish_or_undo<T: Clone>(
    made: T,
    rest: impl FnOnce() -> Result<(), Error>,
    undo: impl FnOnce(T) -> Result<(), Error>,
) -> Result<T, Error> {
    match rest() {
        Ok(()) => Ok(made),
        Err(failed) => {
            let _ = undo(made.clone());
            Err(failed)
        }
    }
}

/// Whether the account record now at the name PCTwin just made (its user ID as the system lists
/// it, `None` when it has none) is the one made with `made`, so it may be taken away.
/// `unset_is_ours`: the record was made in this very call, and making it failed before its user
/// ID was set (it can only be that record: none was at the name before, and making one at a taken
/// name fails).
#[cfg(any(unix, test))]
pub(crate) fn is_the_made_account(listed: Option<u32>, made: u32, unset_is_ours: bool) -> bool {
    match listed {
        Some(uid) => uid == made,
        None => unset_is_ours,
    }
}

/// Whether a home folder may be taken away with the account PCTwin just made (`made`): only if
/// PCTwin's own step made it (it was not there before), it is a folder, and it is that account's.
#[cfg(any(unix, test))]
pub(crate) fn is_the_made_home(was_there: bool, is_folder: bool, owner: u32, made: u32) -> bool {
    !was_there && is_folder && owner == made
}

/// macOS: whether an Open Directory step that failed (`step`, as `os_od_create_user` names it)
/// came after the user record was made, so a record of PCTwin's own is left to take away.
#[cfg(any(target_os = "macos", test))]
pub(crate) fn od_step_left_a_record(step: &str) -> bool {
    matches!(step, "attribute" | "password" | "save")
}

/// macOS: the user ID in a user record as `dscl . -read` prints it whole, `None` when it has none.
#[cfg(any(target_os = "macos", test))]
pub(crate) fn listed_uid(record: &str) -> Option<u32> {
    record
        .lines()
        .find_map(|l| l.strip_prefix("UniqueID:"))
        .and_then(|u| u.trim().parse().ok())
}

/// Windows: the SID of the account PCTwin just made, to take it away by after a later step
/// failed: the one the failed step read, or else the one the system gives the name just made.
#[cfg(any(windows, test))]
pub(crate) fn made_sid(
    read: Option<String>,
    by_name: impl FnOnce() -> Option<String>,
) -> Option<String> {
    read.or_else(by_name)
}

/// Refuses `program` unless only the administrators can change it, or the folders it is in: on
/// Windows it must be inside Program Files (only administrators can change anything there); on
/// macOS and Linux it, and every folder above it, must be root's, and nobody else may write to
/// them. `program` is resolved first, so no link is left in it. For a program that runs with an
/// administrator's rights, or as each person who signs in: one the mover could replace would run
/// as them.
pub fn only_administrators_can_change(program: &Path) -> Result<(), Error> {
    #[cfg(windows)]
    return win::only_from_program_files(program);
    #[cfg(unix)]
    {
        use std::os::unix::fs::MetadataExt;
        let real = std::fs::canonicalize(program)?;
        let mut at = Some(real.as_path());
        while let Some(path) = at {
            let meta = std::fs::symlink_metadata(path)?;
            if !root_only(meta.uid(), meta.mode()) {
                return Err(Error::NotPrivate(format!(
                    "{} can be changed by an account other than root",
                    path.display()
                )));
            }
            at = path.parent();
        }
        Ok(())
    }
    #[cfg(not(any(windows, unix)))]
    {
        let _ = program;
        Err(Error::NotSupported("checking who can change a program"))
    }
}

/// Whether a file or folder owned by `owner` with the permission bits `mode` can be changed by
/// root alone: root's, and no write for its group or anyone else.
#[cfg(any(unix, test))]
pub(crate) fn root_only(owner: u32, mode: u32) -> bool {
    owner == 0 && mode & 0o022 == 0
}

/// A system tool run so that it, and anything it starts, can be ended together (on Unix in a
/// process group of its own: a tool that hands its window to another process cannot leave that
/// window open after PCTwin stopped waiting). Whatever way it is left, early return or error
/// included, the whole group is ended and the tool reaped.
pub struct ToolRun {
    child: Option<std::process::Child>,
}

impl ToolRun {
    /// Starts `command` in a process group of its own.
    pub fn spawn(command: &mut std::process::Command) -> io::Result<Self> {
        #[cfg(unix)]
        std::os::unix::process::CommandExt::process_group(command, 0);
        Ok(Self {
            child: Some(command.spawn()?),
        })
    }

    /// The tool's input, if it was made a pipe (once).
    pub fn stdin(&mut self) -> Option<std::process::ChildStdin> {
        self.child.as_mut()?.stdin.take()
    }

    /// The tool's process ID.
    pub fn id(&self) -> Option<u32> {
        self.child.as_ref().map(std::process::Child::id)
    }

    /// Waits for the tool to end, at most `limit`: `true` if it ended by itself. Either way its
    /// whole group is then ended and it is reaped (a window it handed on never stays open).
    pub fn wait_at_most(&mut self, limit: std::time::Duration) -> io::Result<bool> {
        let until = std::time::Instant::now() + limit;
        let ended = loop {
            let Some(child) = self.child.as_mut() else {
                break Ok(true);
            };
            match child.try_wait() {
                Ok(Some(_)) => break Ok(true),
                Ok(None) if std::time::Instant::now() >= until => break Ok(false),
                Ok(None) => std::thread::sleep(std::time::Duration::from_millis(50)),
                Err(e) => break Err(e),
            }
        };
        self.end();
        ended
    }

    /// Ends the tool's whole group, then the tool, and reaps it.
    fn end(&mut self) {
        if let Some(mut child) = self.child.take() {
            #[cfg(unix)]
            if let Some(pid) = i32::try_from(child.id())
                .ok()
                .and_then(rustix::process::Pid::from_raw)
            {
                let _ = rustix::process::kill_process_group(pid, rustix::process::Signal::KILL);
            }
            let _ = child.kill();
            let _ = child.wait();
        }
    }
}

impl Drop for ToolRun {
    fn drop(&mut self) {
        self.end();
    }
}

#[cfg(test)]
mod tests {
    /// A later step of making an account fails: the account just made (by its own user ID) is
    /// taken away and the failure reported; when every step works nothing is taken away.
    #[test]
    fn an_account_half_made_is_taken_away_again() {
        let undone = std::cell::Cell::new(None);
        let failed = finish_or_undo(
            612,
            || Err(Error::BadInput("its home could not be made")),
            |uid| {
                undone.set(Some(uid));
                Ok(())
            },
        );
        assert!(matches!(failed, Err(Error::BadInput(_))));
        assert_eq!(undone.get(), Some(612));
        undone.set(None);
        let made = finish_or_undo(
            613,
            || Ok(()),
            |uid| {
                undone.set(Some(uid));
                Ok(())
            },
        );
        assert_eq!(made.unwrap(), 613);
        assert_eq!(undone.get(), None);
        // Taking it away failing too still reports the first failure.
        let both = finish_or_undo(
            614,
            || Err(Error::BadInput("first")),
            |_| Err(Error::BadInput("second")),
        );
        assert!(matches!(both, Err(Error::BadInput("first"))));
        // A Windows account goes by its SID, taken away by that SID.
        let by_sid = std::cell::RefCell::new(None);
        let failed = finish_or_undo(
            "S-1-5-21-1-2-3-1001".to_string(),
            || Err(Error::BadInput("its password could not be marked")),
            |sid| {
                *by_sid.borrow_mut() = Some(sid);
                Ok(())
            },
        );
        assert!(failed.is_err());
        assert_eq!(by_sid.borrow().as_deref(), Some("S-1-5-21-1-2-3-1001"));
    }

    /// Only the very account and home PCTwin just made are ever taken away.
    #[test]
    fn only_what_pctwin_made_is_taken_away() {
        // The record at the name: taken away only if it has the user ID it was made with, or,
        // made in this very call, before its user ID was set.
        assert!(is_the_made_account(Some(612), 612, false));
        assert!(!is_the_made_account(Some(613), 612, false));
        assert!(!is_the_made_account(Some(613), 612, true));
        assert!(!is_the_made_account(None, 612, false));
        assert!(is_the_made_account(None, 612, true));
        // The home: only one PCTwin's step made, a folder, and that account's.
        assert!(is_the_made_home(false, true, 612, 612));
        assert!(!is_the_made_home(true, true, 612, 612));
        assert!(!is_the_made_home(false, false, 612, 612));
        assert!(!is_the_made_home(false, true, 0, 612));
        // macOS: a record is left only by a step after it was made.
        for step in ["attribute", "password", "save"] {
            assert!(od_step_left_a_record(step), "{step}");
        }
        for step in ["session", "node", "record"] {
            assert!(!od_step_left_a_record(step), "{step}");
        }
        assert_eq!(
            listed_uid("RecordName: bea\nUniqueID: 612\nUserShell: /bin/zsh\n"),
            Some(612)
        );
        assert_eq!(listed_uid("RecordName: bea\nUserShell: /bin/zsh\n"), None);
        // Windows: the SID the failed step read, else the one the new name has.
        assert_eq!(
            made_sid(Some("S-1-5-21-1".into()), || Some("S-1-5-21-2".into())).as_deref(),
            Some("S-1-5-21-1")
        );
        assert_eq!(
            made_sid(None, || Some("S-1-5-21-2".into())).as_deref(),
            Some("S-1-5-21-2")
        );
        assert_eq!(made_sid(None, || None), None);
    }

    /// A program anyone but root (or, on Windows, the administrators) can change is refused.
    #[test]
    fn a_program_others_can_change_is_refused() {
        assert!(root_only(0, 0o755));
        assert!(root_only(0, 0o700));
        assert!(!root_only(0, 0o775));
        assert!(!root_only(0, 0o757));
        assert!(!root_only(0, 0o1777));
        assert!(!root_only(501, 0o755));
        // A program in a temporary folder: the user's own, and on Unix in a folder anyone may
        // write to.
        let dir = tempfile::tempdir().unwrap();
        let program = dir.path().join("pctwin-handover");
        std::fs::write(&program, b"").unwrap();
        assert!(only_administrators_can_change(&program).is_err());
        assert!(only_administrators_can_change(&dir.path().join("missing")).is_err());
        // A system program, root's all the way up.
        #[cfg(unix)]
        assert!(only_administrators_can_change(Path::new("/bin/sh")).is_ok());
    }

    fn long_tool() -> std::process::Command {
        if cfg!(windows) {
            let mut c = std::process::Command::new("ping");
            c.args(["-n", "30", "127.0.0.1"])
                .stdout(std::process::Stdio::null());
            c
        } else {
            let mut c = std::process::Command::new("sleep");
            c.arg("30");
            c
        }
    }

    /// A tool that never ends is ended at the limit; one that ends by itself is waited for.
    #[test]
    fn a_tool_is_waited_for_at_most_its_limit() {
        let mut long = ToolRun::spawn(&mut long_tool()).unwrap();
        let started = std::time::Instant::now();
        assert!(
            !long
                .wait_at_most(std::time::Duration::from_millis(500))
                .unwrap()
        );
        assert!(started.elapsed() < std::time::Duration::from_secs(10));
        let mut quick = if cfg!(windows) {
            let mut c = std::process::Command::new("cmd");
            c.args(["/c", "exit"]);
            c
        } else {
            std::process::Command::new("true")
        };
        let mut quick = ToolRun::spawn(&mut quick).unwrap();
        assert!(
            quick
                .wait_at_most(std::time::Duration::from_secs(10))
                .unwrap()
        );
    }

    /// A tool that started another process (as a tool may hand its window on): at the limit
    /// both are gone, not only the tool.
    #[cfg(unix)]
    #[test]
    fn what_a_tool_started_is_ended_with_it() {
        let dir = tempfile::tempdir().unwrap();
        let pidfile = dir.path().join("grandchild");
        let mut tool = ToolRun::spawn(std::process::Command::new("sh").args([
            "-c",
            &format!("sleep 30 & echo $! > '{}'; wait", pidfile.display()),
        ]))
        .unwrap();
        let mut grandchild = None;
        for _ in 0..100 {
            if let Some(pid) = std::fs::read_to_string(&pidfile)
                .ok()
                .and_then(|t| t.trim().parse::<i32>().ok())
            {
                grandchild = rustix::process::Pid::from_raw(pid);
                break;
            }
            std::thread::sleep(std::time::Duration::from_millis(20));
        }
        let grandchild = grandchild.expect("the tool started its sleeper");
        assert!(rustix::process::test_kill_process(grandchild).is_ok());
        assert!(
            !tool
                .wait_at_most(std::time::Duration::from_millis(300))
                .unwrap()
        );
        // Ended: gone, or (where nothing reaps orphans, as in a container) only a zombie.
        let ended = |pid: rustix::process::Pid| {
            rustix::process::test_kill_process(pid).is_err()
                || std::fs::read_to_string(format!("/proc/{}/stat", pid.as_raw_nonzero()))
                    .is_ok_and(|stat| {
                        stat.rsplit(')')
                            .next()
                            .is_some_and(|rest| rest.trim_start().starts_with('Z'))
                    })
        };
        let mut gone = false;
        for _ in 0..100 {
            if ended(grandchild) {
                gone = true;
                break;
            }
            std::thread::sleep(std::time::Duration::from_millis(20));
        }
        assert!(gone, "what the tool started is still running");
    }

    /// Left without waiting (an early return), a tool is still ended and reaped.
    #[test]
    fn a_tool_left_early_is_ended() {
        let tool = ToolRun::spawn(&mut long_tool()).unwrap();
        let pid = tool.id().unwrap();
        drop(tool);
        #[cfg(unix)]
        {
            let pid = rustix::process::Pid::from_raw(i32::try_from(pid).unwrap()).unwrap();
            assert!(rustix::process::test_kill_process(pid).is_err());
        }
        #[cfg(not(unix))]
        let _ = pid;
    }

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
