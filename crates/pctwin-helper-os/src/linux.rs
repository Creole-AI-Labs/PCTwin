//! Linux: starting the helper through pkexec (polkit), and who is calling on the helper's
//! socket. No unsafe code here: pkexec is started through the standard library and SO_PEERCRED
//! is read through rustix's safe wrapper.

use crate::{Account, Caller, Error, LaunchError, LaunchRequest, Process};
use std::os::unix::net::UnixStream;
use std::process::{Command, Stdio};

/// pkexec by its full path, never looked up on the search path.
pub(crate) const PKEXEC: &str = "/usr/bin/pkexec";

pub(crate) fn start_pkexec(request: &LaunchRequest) -> Result<Process, LaunchError> {
    let child = Command::new(PKEXEC)
        .arg("--disable-internal-agent")
        .arg(request.program())
        .args(request.args())
        .stdin(Stdio::null())
        .spawn()
        .map_err(|e| {
            if e.kind() == std::io::ErrorKind::NotFound {
                LaunchError::Failed(Error::NotSupported("pkexec is not installed"))
            } else {
                LaunchError::Failed(e.into())
            }
        })?;
    Ok(Process::from_pkexec(child))
}

pub(crate) fn peer(stream: &UnixStream) -> Result<Caller, Error> {
    let cred = rustix::net::sockopt::socket_peercred(stream).map_err(|e| Error::Io(e.into()))?;
    let pid = cred.pid.as_raw_nonzero().get();
    let process_id = u32::try_from(pid).map_err(|_| Error::CallerGone)?;
    let program = std::fs::read_link(format!("/proc/{pid}/exe")).map_err(|e| {
        if e.kind() == std::io::ErrorKind::NotFound {
            Error::CallerGone
        } else {
            Error::Io(e)
        }
    })?;
    Ok(Caller {
        process_id,
        account: Account::Uid(cred.uid.as_raw()),
        program,
    })
}
