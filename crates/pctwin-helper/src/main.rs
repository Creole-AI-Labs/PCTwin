//! The helper program. Started by the app through the system's administrator prompt, for one
//! move:
//!
//! `pctwin-helper --session <token> --mover <account> --approve <account>[,<account>...]
//!  --channel <name> [--dir <folder>] [--laptop <id>] [--plan <generation>:<digest>]`
//!
//! `--plan` (Phase 2.2, Windows for now) starts the permission broker for the approved plan with
//! that generation and digest: they are on the command line so the system's administrator prompt
//! starts the helper for that plan and no other. With `--plan`, `--approve` may be left out (a
//! move of one person that needs an administrator only for installs, settings or a restore
//! point).
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
    plan: Option<pctwin_helper::broker::LaunchSeal>,
    /// `--wifi yes`: the administrator approved reading this laptop's saved Wi-Fi networks with
    /// their passwords (old laptop, Windows), once.
    wifi: bool,
}

fn parse(mut args: impl Iterator<Item = String>) -> Result<Args, String> {
    let (mut session, mut mover, mut approve, mut channel, mut dir, mut laptop, mut plan) =
        (None, None, None, None, None, None, None);
    let mut wifi = None;
    while let Some(flag) = args.next() {
        let value = args.next().ok_or(format!("{flag} needs a value"))?;
        let slot = match flag.as_str() {
            "--session" => &mut session,
            "--mover" => &mut mover,
            "--approve" => &mut approve,
            "--channel" => &mut channel,
            "--dir" => &mut dir,
            "--laptop" => &mut laptop,
            "--plan" => &mut plan,
            "--wifi" => &mut wifi,
            _ => return Err(format!("unknown {flag}")),
        };
        if slot.replace(value).is_some() {
            return Err(format!("{flag} given twice"));
        }
    }
    let mover = mover.ok_or("--mover is needed")?;
    let plan = match plan {
        Some(p) => Some(
            pctwin_helper::broker::LaunchSeal::parse(&p).ok_or("--plan is not a plan's seal")?,
        ),
        None => None,
    };
    let wifi = match wifi.as_deref() {
        None => false,
        Some("yes") => true,
        Some(_) => return Err("--wifi takes only yes".into()),
    };
    let approve: BTreeSet<String> = match (approve, plan) {
        (Some(list), _) => {
            let approve: BTreeSet<String> = list
                .split(',')
                .filter(|a| !a.is_empty())
                .map(str::to_string)
                .collect();
            if approve.is_empty() || approve.contains(&mover) {
                return Err("--approve names the other people, never the mover".into());
            }
            approve
        }
        // A move of one person with an approved plan, or one whose administrator approved only
        // the saved Wi-Fi networks: nobody else's files are read.
        (None, Some(_)) => BTreeSet::new(),
        (None, None) if wifi => BTreeSet::new(),
        (None, None) => return Err("--approve is needed".into()),
    };
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
        plan,
        wifi,
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
    // The change journal asks the system through the helper's own probe (third review, finding 8).
    pctwin_helper::os_probes::install();
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
    use std::sync::atomic::{AtomicBool, Ordering};
    if args.dir.is_some() {
        return Err("--dir is not used on Windows".into());
    }
    let mover = Sid::parse(&args.mover).map_err(|e| e.to_string())?;
    let name = PipeName::parse(&args.channel).map_err(|e| e.to_string())?;
    let pipe = PipeServer::create(&name, &mover).map_err(|e| e.to_string())?;
    // The mover connects within seconds of the helper starting. Nobody in time (the app went
    // away, or the copy that started this one was stopped): it ends, and never holds the
    // one-helper lock with nobody behind it (found by the two-accounts lab).
    let connected = std::sync::Arc::new(AtomicBool::new(false));
    let watch = std::sync::Arc::clone(&connected);
    std::thread::spawn(move || {
        std::thread::sleep(std::time::Duration::from_secs(CONNECT_WAITS_SECS));
        if !watch.load(Ordering::Acquire) {
            eprintln!("pctwin-helper: nobody connected in time");
            std::process::exit(1);
        }
    });
    pipe.accept().map_err(|e| e.to_string())?;
    connected.store(true, Ordering::Release);
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
    // Phase 2.2: the permission broker, for the plan named on the command line. Its intents and
    // outcomes go to the helper's own journal (2.3), in its administrators-only folder, sealed
    // with a key the system keeps for the administrator; if that journal cannot be opened,
    // nothing can be recorded first, so the broker changes nothing.
    let keys = pctwin_grant::RoleKeys::mint().ok_or("the role keys were already taken")?;
    let helper_journal = match args.plan {
        Some(_) => pctwin_helper_os::admin_only_folder("journal")
            .ok()
            .and_then(|dir| pctwin_journal::Journal::open(&dir.join("helper.redb")).ok()),
        None => None,
    };
    let mut real_journal = helper_journal
        .as_ref()
        .map(|j| pctwin_helper::broker::journal::JournalIntents::new(j.broker(&keys.broker)));
    let mut no_journal = pctwin_helper::broker::NoJournal;
    let journal: &mut dyn pctwin_helper::broker::IntentJournal = match real_journal.as_mut() {
        Some(j) => j,
        None => &mut no_journal,
    };
    let (mut authority, mut adapters) = (
        pctwin_helper::broker::SessionElevation,
        pctwin_helper::broker::BuiltInAdapters,
    );
    let clock = pctwin_grant::Now::real;
    // A crash's Wi-Fi folder (it holds passwords in plain text) is gone before anything else.
    pctwin_helper::wifi::sweep_leftovers(&std::env::temp_dir());
    let mut wifi = pctwin_helper::wifi::export_profiles;
    let session = Session::new(args.session.clone(), args.approve.clone(), &mut scanner)
        .with_account_maker(&mut maker)
        .with_new_laptop(&mut system);
    let session = if args.wifi {
        session.with_wifi(&mut wifi)
    } else {
        session
    };
    let mut session = match args.plan {
        Some(seal) => {
            let packages = pctwin_helper::broker::staging::HelperFolder::admin_only("packages")?;
            let broker = pctwin_helper::broker::Broker::new(
                seal,
                &args.session,
                pctwin_helper::broker::staging::Staging::new(packages),
                &mut authority,
                journal,
                &mut adapters,
                &clock,
            );
            // Sleep, hibernation and switching person end the broker's grants (T22-023); signing
            // out ends the helper itself.
            let _watcher = pctwin_helper::broker::watch::start(broker.revoker());
            session.with_broker(broker)
        }
        None => session,
    };
    session.serve(&mut stream).map_err(|e| e.to_string())
}

/// The first argument of the helper restarted with a clean environment.
/// Windows: how long the helper waits for the mover to connect.
#[cfg(windows)]
const CONNECT_WAITS_SECS: u64 = 120;

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
    // Windows itself adds PROCESSOR_ARCHITECTURE to a process it starts on some machines (found
    // on Windows on Arm CI runners): allowed once, and only naming the architecture this program
    // was built for, which is what Windows gives it (second review: any other value restarts).
    let mut own_seen = false;
    let have: Vec<(std::ffi::OsString, std::ffi::OsString)> = have
        .iter()
        .filter(|(k, v)| {
            let own = !own_seen
                && k.to_string_lossy()
                    .eq_ignore_ascii_case("PROCESSOR_ARCHITECTURE")
                && v.as_os_str() == own_architecture();
            own_seen |= own;
            !own
        })
        .cloned()
        .collect();
    if norm(&have) == norm(want) {
        Start::Run
    } else if restarted {
        Start::Refuse
    } else {
        Start::Restart
    }
}

/// The PROCESSOR_ARCHITECTURE Windows gives a program built for this architecture.
#[cfg(any(windows, test))]
fn own_architecture() -> &'static str {
    match std::env::consts::ARCH {
        "x86_64" => "AMD64",
        "aarch64" => "ARM64",
        "x86" => "x86",
        // Nothing else is built for Windows: never matches a value Windows gives.
        _ => "",
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
        Ok(Start::Refuse) => {
            let want = pctwin_helper_os::minimal_env().unwrap_or_default();
            eprintln!(
                "pctwin-helper: its environment could not be made clean (names that differ: {})",
                names_that_differ(&have, &want).join(", ")
            );
            Err(ExitCode::FAILURE)
        }
        Err(e) => {
            eprintln!("pctwin-helper: its environment could not be made clean: {e}");
            Err(ExitCode::FAILURE)
        }
    }
}

/// The names (never the values) in `have` or `want` that are not the same in both.
#[cfg(windows)]
fn names_that_differ(
    have: &[(std::ffi::OsString, std::ffi::OsString)],
    want: &[(std::ffi::OsString, std::ffi::OsString)],
) -> Vec<String> {
    let find = |pairs: &[(std::ffi::OsString, std::ffi::OsString)], name: &str| {
        pairs
            .iter()
            .find(|(k, _)| k.to_string_lossy().eq_ignore_ascii_case(name))
            .map(|(_, v)| v.clone())
    };
    let mut names: Vec<String> = have
        .iter()
        .chain(want)
        .map(|(k, _)| k.to_string_lossy().to_uppercase())
        .filter(|name| find(have, name) != find(want, name))
        .collect();
    names.sort();
    names.dedup();
    names
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
    if args.plan.is_some() {
        // The broker's own folder and the system's approval per action are built for Windows
        // first; on Mac and Linux it is refused before anything starts.
        return Err("--plan is not supported on this system yet".into());
    }
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
    use pctwin_helper::accepting::{Accepted, Deadlined, NotAccepted, OPENING_TIME, first_opening};
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
    // Callers are taken as they come, each with a few seconds in all to open a move and at most
    // one waiting per account, until a move is opened or the time above runs out: a refused
    // caller never uses up the daemon, and callers that say nothing (or say it slowly) never
    // queue the real mover behind them (second fresh-context review).
    let accept = move || {
        // The listener's own failure ends the daemon; one connection's lets that caller go.
        let stream = server
            .accept_connection()
            .map_err(|e| NotAccepted::of(&e))?;
        // Who called: a caller whose account cannot be read is let go, the door stays open.
        let caller =
            pctwin_helper_os::caller_of(&stream).map_err(|e| NotAccepted::Caller(e.to_string()))?;
        // Anyone the system cannot name by user ID is never served (as root is not).
        let uid = match caller.account {
            Account::Uid(uid) => uid,
            _ => 0,
        };
        let cutter = stream.try_clone().ok();
        Ok(Accepted {
            stream,
            uid,
            cut: Box::new(move || {
                if let Some(c) = cutter {
                    let _ = c.shutdown(std::net::Shutdown::Both);
                }
            }),
        })
    };
    let open_move = |stream: &mut Deadlined<std::os::unix::net::UnixStream>, uid: u32| {
        let mut holds =
            |a: &approval::Approval| approval::approval_holds(a).map_err(|e| e.to_string());
        open(stream, &uid.to_string(), &mut holds)
    };
    let log = |uid: u32, why: &NotOpened| match why {
        NotOpened::NotApproved => eprintln!("pctwin-helper: account {uid}: not approved"),
        NotOpened::Malformed(why) | NotOpened::Channel(why) => {
            eprintln!("pctwin-helper: account {uid}: {why}");
        }
    };
    let (mut stream, mover, o) = first_opening(accept, open_move, OPENING_TIME, log)?;
    opened.store(true, Ordering::Release);
    stream.set_read_timeout(None).map_err(|e| e.to_string())?;
    serve_unix(&mut stream, mover, o.session, o.approve, o.laptop)
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
        // Windows' own PROCESSOR_ARCHITECTURE, naming an architecture: still clean.
        let mut arm = same.clone();
        arm.push(pair("PROCESSOR_ARCHITECTURE", own_architecture()));
        assert_eq!(start_decision(&arm, &want, true), Start::Run);
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
            vec![
                pair("SystemRoot", r"C:\Windows"),
                pair("TEMP", r"C:\Windows\Temp"),
                pair("PROCESSOR_ARCHITECTURE", r"C:\Evil"),
            ],
            // Another architecture than this program's, or Windows' own value twice.
            vec![
                pair("SystemRoot", r"C:\Windows"),
                pair("TEMP", r"C:\Windows\Temp"),
                pair(
                    "PROCESSOR_ARCHITECTURE",
                    if own_architecture() == "x86" {
                        "AMD64"
                    } else {
                        "x86"
                    },
                ),
            ],
            vec![
                pair("SystemRoot", r"C:\Windows"),
                pair("TEMP", r"C:\Windows\Temp"),
                pair("PROCESSOR_ARCHITECTURE", own_architecture()),
                pair("processor_architecture", own_architecture()),
            ],
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
        assert!(a.plan.is_none());
    }

    /// Phase 2.2: the approved plan's seal on the command line; with it, nobody else need be
    /// approved, and a broken seal ends the helper before anything starts.
    #[test]
    fn a_plan_is_read_from_the_command_line_strictly() {
        let seal = format!("3:{}", "ab".repeat(32));
        let base = ["--session", "t", "--mover", "1000", "--channel", "c"];
        let mut with_plan = base.to_vec();
        with_plan.extend(["--plan", &seal]);
        let a = args(&with_plan).unwrap();
        assert!(a.approve.is_empty());
        let p = a.plan.unwrap();
        assert_eq!(p.generation, 3);
        assert_eq!(p.digest.to_string(), "ab".repeat(32));
        // Without a plan, someone must be approved.
        assert!(args(&base).is_err());
        // With both, the approved people are still checked.
        let mut both = with_plan.clone();
        both.extend(["--approve", "1000"]);
        assert!(args(&both).is_err());
        let mut both = with_plan.clone();
        both.extend(["--approve", "1001"]);
        assert_eq!(args(&both).unwrap().approve.len(), 1);
        for bad in ["3", "x:00", &format!("3:{}", "ab".repeat(31))] {
            let mut broken = base.to_vec();
            broken.extend(["--plan", bad]);
            assert!(args(&broken).is_err(), "{bad}");
        }
        let mut twice = with_plan.clone();
        twice.extend(["--plan", &seal]);
        assert!(args(&twice).is_err());
    }

    /// The saved Wi-Fi networks (2.4): read only by a helper the administrator started for
    /// them, `--wifi yes`, alone or beside other people; nothing else is taken for yes.
    #[test]
    fn wifi_is_read_from_the_command_line_strictly() {
        let base = ["--session", "t", "--mover", "1000", "--channel", "c"];
        assert!(
            !args(&[&base[..], &["--approve", "1001"]].concat())
                .unwrap()
                .wifi
        );
        let alone = args(&[&base[..], &["--wifi", "yes"]].concat()).unwrap();
        assert!(alone.wifi);
        assert!(alone.approve.is_empty());
        let both = args(&[&base[..], &["--wifi", "yes", "--approve", "1001"]].concat()).unwrap();
        assert!(both.wifi);
        assert_eq!(both.approve.len(), 1);
        // Still never the mover approved for themselves.
        assert!(args(&[&base[..], &["--wifi", "yes", "--approve", "1000"]].concat()).is_err());
        for bad in ["no", "true", "YES", ""] {
            assert!(
                args(&[&base[..], &["--wifi", bad]].concat()).is_err(),
                "{bad}"
            );
        }
        assert!(args(&[&base[..], &["--wifi", "yes", "--wifi", "yes"]].concat()).is_err());
    }
}
