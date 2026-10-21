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

use std::collections::BTreeSet;
use std::process::ExitCode;

use pctwin_helper::Session;
use pctwin_helper::person::scan_person;
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
    let args = match parse(std::env::args().skip(1)) {
        Ok(a) => a,
        Err(why) => {
            eprintln!("pctwin-helper: {why}");
            return ExitCode::from(2);
        }
    };
    // One move at a time: held until this program ends.
    let _lock = match pctwin_helper::lock::one_helper(&pctwin_helper::lock::lock_path()) {
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
    let mut scanner = move |account: &str| scan_person(laptop, account);
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
    let laptop = args.laptop;
    let mut scanner = move |account: &str| scan_person(laptop, account);
    let mut maker = pctwin_helper::account::create_account;
    let mut system = pctwin_helper::newlaptop::System {
        mover: Account::Uid(mover),
    };
    Session::new(args.session.clone(), args.approve.clone(), &mut scanner)
        .with_account_maker(&mut maker)
        .with_new_laptop(&mut system)
        .serve(&mut stream)
        .map_err(|e| e.to_string())
}

#[cfg(test)]
mod tests {
    use super::*;

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
