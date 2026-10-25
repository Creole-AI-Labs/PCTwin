//! Linux: starting the helper through pkexec (polkit), and who is calling on the helper's
//! socket. No unsafe code here: pkexec is started through the standard library and SO_PEERCRED
//! is read through rustix's safe wrapper.

use crate::{Account, Caller, Error, LaunchError, LaunchRequest, Process};
use std::os::unix::net::UnixStream;
use std::process::{Command, Stdio};

/// pkexec by its full path, never looked up on the search path.
pub(crate) const PKEXEC: &str = "/usr/bin/pkexec";

pub(crate) fn start_pkexec(request: &LaunchRequest) -> Result<Process, LaunchError> {
    use std::io::Write;
    let mut child = Command::new(PKEXEC)
        .arg("--disable-internal-agent")
        .arg(request.program())
        .args(request.args())
        .stdin(if request.input().is_some() {
            Stdio::piped()
        } else {
            Stdio::null()
        })
        .spawn()
        .map_err(|e| {
            if e.kind() == std::io::ErrorKind::NotFound {
                LaunchError::Failed(Error::NotSupported("pkexec is not installed"))
            } else {
                LaunchError::Failed(e.into())
            }
        })?;
    // pkexec hands its standard input to the program it starts once the prompt is answered: the
    // secret waits in the pipe until then, and the pipe is closed after it.
    if let (Some(input), Some(mut stdin)) = (request.input(), child.stdin.take()) {
        stdin
            .write_all(input)
            .map_err(|e| LaunchError::Failed(e.into()))?;
    }
    Ok(Process::from_pkexec(child))
}

pub(crate) fn peer(stream: &UnixStream) -> Result<Caller, Error> {
    let cred = rustix::net::sockopt::socket_peercred(stream).map_err(|e| Error::Io(e.into()))?;
    let pid = cred.pid.as_raw_nonzero().get();
    let process_id = u32::try_from(pid).map_err(|_| Error::CallerGone)?;
    // Only for the record: a process another account may not look into (the app looking at the
    // hardened root helper) gives no path, and that never stops the connection.
    let program = match std::fs::read_link(format!("/proc/{pid}/exe")) {
        Ok(p) => p,
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => return Err(Error::CallerGone),
        Err(_) => std::path::PathBuf::new(),
    };
    Ok(Caller {
        process_id,
        account: Account::Uid(cred.uid.as_raw()),
        program_hint: program,
    })
}

/// The system's account tools by their full paths, never looked up on the search path.
const USERADD: &str = "/usr/sbin/useradd";
const CHPASSWD: &str = "/usr/sbin/chpasswd";
const CHAGE: &str = "/usr/bin/chage";

fn run(program: &str, args: &[&str], input: Option<&[u8]>) -> Result<(), Error> {
    use std::io::Write;
    let mut child = Command::new(program)
        .args(args)
        .stdin(if input.is_some() {
            Stdio::piped()
        } else {
            Stdio::null()
        })
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .spawn()
        .map_err(|e| {
            if e.kind() == std::io::ErrorKind::NotFound {
                Error::NotSupported("the system's account tools are not installed")
            } else {
                Error::Io(e)
            }
        })?;
    if let (Some(bytes), Some(mut stdin)) = (input, child.stdin.take()) {
        stdin.write_all(bytes)?;
    }
    let status = child.wait()?;
    match status.code() {
        Some(0) => Ok(()),
        // useradd: 9, the name is taken.
        Some(9) if program == USERADD => Err(Error::AlreadyExists),
        Some(code) => Err(Error::Os {
            call: "an account tool",
            code: i64::from(code),
        }),
        None => Err(Error::Os {
            call: "an account tool",
            code: -1,
        }),
    }
}

/// Linux: a standard account with its home folder (`useradd`), its password set through
/// `chpasswd`'s input (never a command line), then marked to be changed at the first sign-in
/// (`chage -d 0`). Returns its user ID. If a step after `useradd` fails, the account it made is
/// taken away again by its user ID (and its home, only if `useradd` made it), never left
/// half-made.
pub(crate) fn add_standard_account(
    name: &str,
    full_name: &str,
    password: &str,
) -> Result<u32, Error> {
    let home_was_there = default_home(name).exists();
    run(
        USERADD,
        &["--create-home", "--comment", full_name, "--", name],
        None,
    )?;
    let uid = uid_of(name)?;
    crate::finish_or_undo(
        uid,
        || {
            let mut line = format!("{name}:{password}\n").into_bytes();
            let set = run(CHPASSWD, &[], Some(&line));
            line.iter_mut().for_each(|b| *b = 0);
            set?;
            run(CHAGE, &["-d", "0", "--", name], None)
        },
        |made| remove_made_account(name, made, home_was_there),
    )
}

/// Where `useradd` makes `name`'s home: its default base folder (`useradd -D`, `HOME=`), or
/// `/home`.
fn default_home(name: &str) -> std::path::PathBuf {
    let base = Command::new(USERADD)
        .arg("-D")
        .stdin(Stdio::null())
        .stderr(Stdio::null())
        .output()
        .ok()
        .and_then(|out| {
            String::from_utf8_lossy(&out.stdout)
                .lines()
                .find_map(|l| l.strip_prefix("HOME=").map(str::to_string))
        })
        .filter(|b| b.starts_with('/'))
        .unwrap_or_else(|| "/home".to_string());
    std::path::Path::new(&base).join(name)
}

const USERDEL: &str = "/usr/sbin/userdel";

/// Takes away the account `name` PCTwin just made: only if it still has the user ID `uid` it was
/// made with (never another account of that name), and its home folder only if `useradd` made it
/// (it was not there before, and it is that account's).
fn remove_made_account(name: &str, uid: u32, home_was_there: bool) -> Result<(), Error> {
    use std::os::unix::fs::MetadataExt;
    if !crate::is_the_made_account(uid_of(name).ok(), uid, false) {
        return Err(Error::BadInput("the account is not the one PCTwin made"));
    }
    let home = crate::holding::passwd_entry(uid).map(|(_, home)| home).ok();
    run(USERDEL, &["--", name], None)?;
    if let Some(home) = home
        && std::fs::symlink_metadata(&home)
            .is_ok_and(|m| crate::is_the_made_home(home_was_there, m.is_dir(), m.uid(), uid))
        && home == default_home(name)
    {
        std::fs::remove_dir_all(&home)?;
    }
    Ok(())
}

/// The user ID the system gave `name`, from its own list of accounts.
fn uid_of(name: &str) -> Result<u32, Error> {
    let passwd = std::fs::read_to_string("/etc/passwd")?;
    passwd
        .lines()
        .filter_map(|l| {
            let mut f = l.split(':');
            let (n, _, uid) = (f.next()?, f.next()?, f.next()?);
            (n == name).then(|| uid.parse().ok()).flatten()
        })
        .next()
        .ok_or(Error::NotSupported(
            "the new account was not in the system's list",
        ))
}
