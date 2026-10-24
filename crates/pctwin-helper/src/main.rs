//! The helper program. Started by the app through the system's administrator prompt, for one
//! move:
//!
//! `pctwin-helper --session <token> --mover <account> --approve <account>[,<account>...]
//!  --channel <name> [--dir <folder>] [--laptop <id>]`
//!
//! `--session -` reads the token from the first line of standard input instead, so it is never on
//! the command line, which other programs can read on Mac and Linux.
//!
//! It makes its private channel (Windows: the named pipe `<name>` only the mover's account can
//! open; Mac and Linux: the socket `<name>` in the folder `<folder>`, which it makes, owned by
//! the system and readable by no one else), accepts one connection, refuses it unless it comes
//! from the mover's account, serves that one session and ends. Anything it does not understand
//! on its command line ends it before the channel is made.
//!
//! On a Mac it is instead the launch daemon, `pctwin-helper --daemon`, started by the system; see
//! `pctwin_helper::opening`.

use std::collections::BTreeSet;
use std::process::ExitCode;

use pctwin_helper::Session;
use pctwin_helper::person::{ScanOnce, scan_people};
use pctwin_record::LaptopId;

struct Args {
    session: String,
    mover: String,
    approve: BTreeSet<String>,
    channel: String,
    dir: Option<String>,
    laptop: LaptopId,
}

fn parse(mut args: impl Iterator<Item = String>) -> Result<Args, String> {
    let (mut session, mut mover, mut approve, mut channel, mut dir, mut laptop) =
        (None, None, None, None, None, None);
    while let Some(flag) = args.next() {
        let value = args.next().ok_or(format!("{flag} needs a value"))?;
        let slot = match flag.as_str() {
            "--session" => &mut session,
            "--mover" => &mut mover,
            "--approve" => &mut approve,
            "--channel" => &mut channel,
            "--dir" => &mut dir,
            "--laptop" => &mut laptop,
            _ => return Err(format!("unknown {flag}")),
        };
        if slot.replace(value).is_some() {
            return Err(format!("{flag} given twice"));
        }
    }
    let mover = mover.ok_or("--mover is needed")?;
    let approve: BTreeSet<String> = approve
        .ok_or("--approve is needed")?
        .split(',')
        .filter(|a| !a.is_empty())
        .map(str::to_string)
        .collect();
    if approve.is_empty() || approve.contains(&mover) {
        return Err("--approve names the other people, never the mover".into());
    }
    Ok(Args {
        session: session_from(
            session.ok_or("--session is needed")?,
            std::io::stdin().lock(),
        )?,
        mover,
        approve,
        channel: channel.ok_or("--channel is needed")?,
        dir,
        laptop: match laptop {
            Some(hex) => LaptopId::from_hex(&hex).map_err(|_| "--laptop is not an ID")?,
            None => LaptopId::from_hex(&"0".repeat(32)).map_err(|_| "no laptop ID")?,
        },
    })
}

/// The session token: as given, or with `-`, the first line of `input` (at most 128 characters).
fn session_from(value: String, input: impl std::io::BufRead) -> Result<String, String> {
    use std::io::BufRead;
    if value != "-" {
        return Ok(value);
    }
    let mut line = String::new();
    input
        .take(130)
        .read_line(&mut line)
        .map_err(|_| "the session could not be read")?;
    let token = line.trim_end_matches(['\r', '\n']).to_string();
    if token.is_empty() || token.len() > 128 || token.chars().any(char::is_control) {
        return Err("no session was given".into());
    }
    Ok(token)
}

fn main() -> ExitCode {
    // Mac and Linux: nothing this helper makes is ever open to others, not even for a moment
    // before its own mode is set.
    #[cfg(unix)]
    rustix::process::umask(rustix::fs::Mode::from_raw_mode(0o077));
    let raw: Vec<String> = std::env::args().skip(1).collect();
    // Windows: the helper runs with an administrator's rights, so it runs only with the minimal
    // environment worked out from Windows itself; everything it starts then inherits only that
    // (found by the fresh-context review: PowerShell, whoami, cmd and reg inherited the starter's).
    #[cfg(windows)]
    let raw = match clean_start(raw) {
        Ok(raw) => raw,
        Err(code) => return code,
    };
    // On a Mac, launchd starts the daemon with no arguments (its property list names the program
    // only, as Service Management requires): that, or --daemon, is daemon mode (found by the
    // fresh-context review: the daemon could never have started).
    if raw == ["--daemon"] || (cfg!(target_os = "macos") && raw.is_empty()) {
        return daemon_main();
    }
    let args = match parse(raw.into_iter()) {
        Ok(a) => a,
        Err(why) => {
            eprintln!("pctwin-helper: {why}");
            return ExitCode::from(2);
        }
    };
    // One move at a time: held until this program ends.
    let _lock = match pctwin_helper::lock::lock_path()
        .and_then(|path| pctwin_helper::lock::one_helper(&path))
    {
        Ok(l) => l,
        Err(why) => {
            eprintln!("pctwin-helper: {why}");
            return ExitCode::FAILURE;
        }
    };
    // Sign-in hand-overs that are done (their held folder gone) are cleared away first.
    if let Some(root) = pctwin_helper_os::holding::staging_root() {
        pctwin_helper_os::signin::clean(&root);
    }
    match run(&args) {
        Ok(()) => ExitCode::SUCCESS,
        Err(why) => {
            eprintln!("pctwin-helper: {why}");
            ExitCode::FAILURE
        }
    }
}

#[cfg(windows)]
fn run(args: &Args) -> Result<(), String> {
    use pctwin_helper_os::{Account, PipeName, PipeServer, Sid, caller_of};
    if args.dir.is_some() {
        return Err("--dir is not used on Windows".into());
    }
    let mover = Sid::parse(&args.mover).map_err(|e| e.to_string())?;
    let name = PipeName::parse(&args.channel).map_err(|e| e.to_string())?;
    let pipe = PipeServer::create(&name, &mover).map_err(|e| e.to_string())?;
    pipe.accept().map_err(|e| e.to_string())?;
    let caller = caller_of(&pipe).map_err(|e| e.to_string())?;
    if caller.account != Account::Sid(mover.clone()) {
        return Err("a connection from another account was refused".into());
    }
    let laptop = args.laptop;
    // One sweep for every approved person, on the first ScanPerson.
    let mut once = ScanOnce::new(args.approve.clone(), move |people: &BTreeSet<String>| {
        scan_people(laptop, people)
    });
    let mut scanner = move |account: &str| once.scan(account);
    let mut stream = pipe.stream();
    let mut maker = pctwin_helper::account::create_account;
    let mut system = pctwin_helper::newlaptop::System {
        mover: Account::Sid(mover.clone()),
    };
    Session::new(args.session.clone(), args.approve.clone(), &mut scanner)
        .with_account_maker(&mut maker)
        .with_new_laptop(&mut system)
        .serve(&mut stream)
        .map_err(|e| e.to_string())
}

/// The first argument of the helper restarted with a clean environment.
#[cfg(windows)]
const CLEAN_START: &str = "--clean-start";

/// What the helper does with the environment it was started with.
#[cfg(any(windows, test))]
#[derive(Debug, PartialEq, Eq)]
enum Start {
    /// Exactly the minimal environment: carry on.
    Run,
    /// Anything else: start again, once, with only the minimal environment.
    Restart,
    /// Already restarted and still not clean: never run (nothing loops; a planted
    /// --clean-start only makes the helper refuse).
    Refuse,
}

/// `have` is clean when it holds exactly `want` (names compared as Windows does, without case).
#[cfg(any(windows, test))]
fn start_decision(
    have: &[(std::ffi::OsString, std::ffi::OsString)],
    want: &[(std::ffi::OsString, std::ffi::OsString)],
    restarted: bool,
) -> Start {
    let norm = |pairs: &[(std::ffi::OsString, std::ffi::OsString)]| {
        let mut v: Vec<(String, std::ffi::OsString)> = pairs
            .iter()
            .map(|(k, v)| (k.to_string_lossy().to_uppercase(), v.clone()))
            .collect();
        v.sort();
        v
    };
    if norm(have) == norm(want) {
        Start::Run
    } else if restarted {
        Start::Refuse
    } else {
        Start::Restart
    }
}

/// Windows: carries on with `raw` when the environment is exactly the minimal one; otherwise
/// starts again clean (and ends as that copy ends), or refuses.
#[cfg(windows)]
fn clean_start(mut raw: Vec<String>) -> Result<Vec<String>, ExitCode> {
    let restarted = raw.first().is_some_and(|a| a == CLEAN_START);
    if restarted {
        raw.remove(0);
    }
    let have: Vec<(std::ffi::OsString, std::ffi::OsString)> = std::env::vars_os().collect();
    match pctwin_helper_os::minimal_env().map(|want| start_decision(&have, &want, restarted)) {
        Ok(Start::Run) => Ok(raw),
        Ok(Start::Restart) => Err(restart_clean(&raw)),
        Ok(Start::Refuse) | Err(_) => {
            eprintln!("pctwin-helper: its environment could not be made clean");
            Err(ExitCode::FAILURE)
        }
    }
}

/// Starts this program again, as it was started, with only the minimal environment, and ends
/// as it ends.
#[cfg(windows)]
fn restart_clean(raw: &[String]) -> ExitCode {
    let run = || -> Result<i32, String> {
        let exe = std::env::current_exe().map_err(|e| e.to_string())?;
        let status = pctwin_helper_os::clean_command(&exe)
            .map_err(|e| e.to_string())?
            .arg(CLEAN_START)
            .args(raw)
            .status()
            .map_err(|e| e.to_string())?;
        Ok(status.code().unwrap_or(1))
    };
    match run() {
        Ok(0) => ExitCode::SUCCESS,
        Ok(code) => ExitCode::from(u8::try_from(code).unwrap_or(1)),
        Err(why) => {
            eprintln!("pctwin-helper: starting again with a clean environment: {why}");
            ExitCode::FAILURE
        }
    }
}

#[cfg(unix)]
fn run(args: &Args) -> Result<(), String> {
    use pctwin_helper_os::{Account, ChannelDir, SocketServer};
    let mover: u32 = args.mover.parse().map_err(|_| "--mover is not a user ID")?;
    let dir = args.dir.as_ref().ok_or("--dir is needed here")?;
    let dir = ChannelDir::create(std::path::Path::new(dir)).map_err(|e| e.to_string())?;
    let server = SocketServer::bind(&dir, &args.channel, mover).map_err(|e| e.to_string())?;
    let (mut stream, caller) = server.accept().map_err(|e| e.to_string())?;
    if caller.account != Account::Uid(mover) {
        return Err("a connection from another account was refused".into());
    }
    serve_unix(
        &mut stream,
        mover,
        args.session.clone(),
        args.approve.clone(),
        args.laptop,
    )
}

/// Mac and Linux: one session for the mover `mover` on `stream`.
#[cfg(unix)]
fn serve_unix(
    stream: &mut std::os::unix::net::UnixStream,
    mover: u32,
    session: String,
    approve: BTreeSet<String>,
    laptop: LaptopId,
) -> Result<(), String> {
    let mut once = ScanOnce::new(approve.clone(), move |people: &BTreeSet<String>| {
        scan_people(laptop, people)
    });
    let mut scanner = move |account: &str| once.scan(account);
    let mut maker = pctwin_helper::account::create_account;
    let mut system = pctwin_helper::newlaptop::System {
        mover: pctwin_helper_os::Account::Uid(mover),
    };
    Session::new(session, approve, &mut scanner)
        .with_account_maker(&mut maker)
        .with_new_laptop(&mut system)
        .serve(stream)
        .map_err(|e| e.to_string())
}

/// `pctwin-helper --daemon`: the launch daemon on a Mac (Security Design part C, "The
/// administrator's approval on a Mac"). Started by the system, as root, when the app switches it
/// on; it serves the first move opened with an administrator's approval, then ends.
fn daemon_main() -> ExitCode {
    #[cfg(target_os = "macos")]
    let r = daemon();
    #[cfg(not(target_os = "macos"))]
    let r: Result<(), String> = Err("--daemon is for a Mac only".into());
    match r {
        Ok(()) => ExitCode::SUCCESS,
        Err(why) => {
            eprintln!("pctwin-helper: {why}");
            ExitCode::FAILURE
        }
    }
}

#[cfg(target_os = "macos")]
fn daemon() -> Result<(), String> {
    use pctwin_helper::opening::{NotOpened, open};
    use pctwin_helper_os::daemon::{CHANNEL_DIR, CHANNEL_SOCKET, WAITS_SECS};
    use pctwin_helper_os::{Account, ChannelDir, SocketServer, approval};
    use std::os::unix::fs::{MetadataExt, PermissionsExt};
    use std::path::Path;
    use std::sync::atomic::{AtomicBool, Ordering};
    // The folder above the channel, which also holds the lock: root's own plain folder, which
    // others may only pass through. Checked before the lock is taken in it (second fresh-context
    // review), so nothing in a folder someone else made is ever opened.
    let dir = Path::new(CHANNEL_DIR);
    let above = dir.parent().ok_or("no folder above the channel")?;
    std::fs::create_dir_all(above).map_err(|e| format!("the channel's folder: {e}"))?;
    let meta = std::fs::symlink_metadata(above).map_err(|e| e.to_string())?;
    if !meta.is_dir() || meta.file_type().is_symlink() || meta.uid() != 0 {
        return Err(format!(
            "{} is not root's own plain folder",
            above.display()
        ));
    }
    std::fs::set_permissions(above, std::fs::Permissions::from_mode(0o755))
        .map_err(|e| e.to_string())?;
    let _lock =
        pctwin_helper::lock::lock_path().and_then(|p| pctwin_helper::lock::one_helper(&p))?;
    if let Some(root) = pctwin_helper_os::holding::staging_root() {
        pctwin_helper_os::signin::clean(&root);
    }
    approval::define_move_right().map_err(|e| format!("PCTwin's right: {e}"))?;
    // Left by an earlier run: only root can make anything here.
    if std::fs::symlink_metadata(dir).is_ok() {
        std::fs::remove_dir_all(dir).map_err(|e| format!("the old channel: {e}"))?;
    }
    let channel = ChannelDir::create(dir).map_err(|e| e.to_string())?;
    let server =
        SocketServer::bind_for_anyone(&channel, CHANNEL_SOCKET).map_err(|e| e.to_string())?;
    // Nobody opens a move in time (the Mac started up with the daemon still on, or the app went
    // away): it ends.
    let opened = std::sync::Arc::new(AtomicBool::new(false));
    let watch = std::sync::Arc::clone(&opened);
    std::thread::spawn(move || {
        std::thread::sleep(std::time::Duration::from_secs(WAITS_SECS));
        if !watch.load(Ordering::Acquire) {
            std::process::exit(0);
        }
    });
    // Connections are taken one at a time until a move is opened, or the time above runs out:
    // a refused caller never uses up the daemon, and one that says nothing is let go after a few
    // seconds, so it cannot hold the move's place (found by the fresh-context review).
    loop {
        let (mut stream, caller) = server.accept().map_err(|e| e.to_string())?;
        let Account::Uid(mover) = caller.account else {
            continue;
        };
        if mover == 0 {
            continue;
        }
        if stream
            .set_read_timeout(Some(std::time::Duration::from_secs(10)))
            .is_err()
        {
            continue;
        }
        let mut holds =
            |a: &approval::Approval| approval::approval_holds(a).map_err(|e| e.to_string());
        match open(&mut stream, &mover.to_string(), &mut holds) {
            Ok(o) => {
                opened.store(true, Ordering::Release);
                stream.set_read_timeout(None).map_err(|e| e.to_string())?;
                return serve_unix(&mut stream, mover, o.session, o.approve, o.laptop);
            }
            Err(NotOpened::NotApproved) => {
                eprintln!("pctwin-helper: account {mover}: not approved");
            }
            Err(NotOpened::Malformed(why) | NotOpened::Channel(why)) => {
                eprintln!("pctwin-helper: account {mover}: {why}");
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_helper_runs_only_with_exactly_the_minimal_environment() {
        let pair = |k: &str, v: &str| (std::ffi::OsString::from(k), std::ffi::OsString::from(v));
        let want = vec![
            pair("SystemRoot", r"C:\Windows"),
            pair("TEMP", r"C:\Windows\Temp"),
        ];
        // The same, names in any case and any order: run.
        let same = vec![
            pair("TEMP", r"C:\Windows\Temp"),
            pair("SYSTEMROOT", r"C:\Windows"),
        ];
        assert_eq!(start_decision(&same, &want, false), Start::Run);
        assert_eq!(start_decision(&same, &want, true), Start::Run);
        // Anything planted (a profiler, a changed SystemRoot, one value missing): start again
        // clean, and if that was already the clean start, never run.
        for bad in [
            vec![
                pair("SystemRoot", r"C:\Windows"),
                pair("TEMP", r"C:\Windows\Temp"),
                pair("COR_PROFILER", "x"),
            ],
            vec![
                pair("SystemRoot", r"C:\Evil"),
                pair("TEMP", r"C:\Windows\Temp"),
            ],
            vec![pair("SystemRoot", r"C:\Windows")],
        ] {
            assert_eq!(
                start_decision(&bad, &want, false),
                Start::Restart,
                "{bad:?}"
            );
            assert_eq!(start_decision(&bad, &want, true), Start::Refuse, "{bad:?}");
        }
    }

    fn args(list: &[&str]) -> Result<Args, String> {
        parse(list.iter().map(|s| s.to_string()))
    }

    #[test]
    fn a_session_given_as_a_dash_is_read_from_the_input_and_never_from_the_command_line() {
        assert_eq!(session_from("abc".into(), &b""[..]).unwrap(), "abc");
        assert_eq!(
            session_from("-".into(), &b"0123abcd\n"[..]).unwrap(),
            "0123abcd"
        );
        assert_eq!(
            session_from("-".into(), &b"0123abcd"[..]).unwrap(),
            "0123abcd"
        );
        assert!(session_from("-".into(), &b"\n"[..]).is_err());
        assert!(session_from("-".into(), &b""[..]).is_err());
        let long = format!("{}\n", "a".repeat(200));
        assert!(session_from("-".into(), long.as_bytes()).is_err());
        assert!(session_from("-".into(), &b"ab\x07cd\n"[..]).is_err());
    }

    #[test]
    fn the_command_line_is_read_strictly() {
        let good = [
            "--session",
            "t",
            "--mover",
            "1000",
            "--approve",
            "1001,1002",
            "--channel",
            "c",
        ];
        let a = args(&good).unwrap();
        assert_eq!(a.approve.len(), 2);
        assert!(args(&["--session", "t"]).is_err());
        // The mover is never approved for themselves, nobody approved is refused.
        assert!(
            args(&[
                "--session",
                "t",
                "--mover",
                "1000",
                "--approve",
                "1000",
                "--channel",
                "c"
            ])
            .is_err()
        );
        assert!(
            args(&[
                "--session",
                "t",
                "--mover",
                "1000",
                "--approve",
                ",",
                "--channel",
                "c"
            ])
            .is_err()
        );
        let mut extra = good.to_vec();
        extra.extend(["--run", "cmd"]);
        assert!(args(&extra).is_err());
        let mut twice = good.to_vec();
        twice.extend(["--mover", "0"]);
        assert!(args(&twice).is_err());
    }
}
