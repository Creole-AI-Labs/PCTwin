//! macOS: who is calling on the helper's socket, and switching the helper's launch daemon on and
//! off through SMAppService.
//!
//! The four `os_daemon_*` / `os_open_login_items` functions are the only unsafe code here, one
//! block each, listed in scripts/unsafe-fence.sh: the Service Management bindings mark every
//! method unsafe. Reading the caller needs none (nix and libproc wrap it safely).

use crate::daemon::{DaemonService, DaemonState};
use crate::{Account, Caller, Error};
use nix::sys::socket::{getsockopt, sockopt};
use objc2_foundation::NSString;
use objc2_service_management::{SMAppService, SMAppServiceStatus};
use std::os::unix::net::UnixStream;
use std::path::PathBuf;

pub(crate) fn peer(stream: &UnixStream) -> Result<Caller, Error> {
    let errno = |e: nix::errno::Errno| Error::Io(std::io::Error::from_raw_os_error(e as i32));
    let pid = getsockopt(stream, sockopt::LocalPeerPid).map_err(errno)?;
    let cred = getsockopt(stream, sockopt::LocalPeerCred).map_err(errno)?;
    let process_id = u32::try_from(pid).map_err(|_| Error::CallerGone)?;
    let program = libproc::proc_pid::pidpath(pid).map_err(|_| Error::CallerGone)?;
    Ok(Caller {
        process_id,
        account: Account::Uid(cred.uid()),
        program_hint: PathBuf::from(program),
    })
}

/// Registers the launch daemon whose property list the app carries in
/// `Contents/Library/LaunchDaemons/<plist>`. Errors come back as the `NSError` code and text.
fn os_daemon_register(plist: &str) -> Result<(), (i64, String)> {
    let name = NSString::from_str(plist);
    // SAFETY: `name` is a valid NSString for the call; the service object is retained by the
    // binding and released on drop; the error, if any, is a retained NSError read only here.
    #[expect(
        unsafe_code,
        reason = "the SMAppService bindings mark every method unsafe"
    )]
    let r = unsafe { SMAppService::daemonServiceWithPlistName(&name).registerAndReturnError() };
    r.map_err(|e| (e.code() as i64, e.localizedDescription().to_string()))
}

/// Unregisters it (the move has ended).
fn os_daemon_unregister(plist: &str) -> Result<(), (i64, String)> {
    let name = NSString::from_str(plist);
    // SAFETY: as in os_daemon_register.
    #[expect(
        unsafe_code,
        reason = "the SMAppService bindings mark every method unsafe"
    )]
    let r = unsafe { SMAppService::daemonServiceWithPlistName(&name).unregisterAndReturnError() };
    r.map_err(|e| (e.code() as i64, e.localizedDescription().to_string()))
}

/// Its state: 0 not registered, 1 enabled, 2 needs an administrator's approval, 3 not found.
fn os_daemon_status(plist: &str) -> isize {
    let name = NSString::from_str(plist);
    // SAFETY: `name` is a valid NSString for the call; `status` only reads.
    #[expect(
        unsafe_code,
        reason = "the SMAppService bindings mark every method unsafe"
    )]
    let r = unsafe { SMAppService::daemonServiceWithPlistName(&name).status() };
    r.0
}

/// Opens System Settings at Login Items, where an administrator allows the daemon.
fn os_open_login_items() {
    // SAFETY: a class method with no arguments and no result.
    #[expect(
        unsafe_code,
        reason = "the SMAppService bindings mark every method unsafe"
    )]
    unsafe {
        SMAppService::openSystemSettingsLoginItems()
    };
}

/// The system's Service Management.
#[derive(Clone, Copy, Debug, Default)]
pub struct SystemDaemons;

impl DaemonService for SystemDaemons {
    fn register(&self, plist: &str) -> Result<(), Error> {
        os_daemon_register(plist).map_err(|(code, _)| Error::Os {
            call: "SMAppService.register",
            code,
        })
    }

    fn unregister(&self, plist: &str) -> Result<(), Error> {
        os_daemon_unregister(plist).map_err(|(code, _)| Error::Os {
            call: "SMAppService.unregister",
            code,
        })
    }

    fn status(&self, plist: &str) -> DaemonState {
        DaemonState::from_raw(os_daemon_status(plist))
    }

    fn open_approval_settings(&self) {
        os_open_login_items();
    }
}

// The raw states, as Apple numbers them.
const _: () = {
    assert!(SMAppServiceStatus::NotRegistered.0 == 0);
    assert!(SMAppServiceStatus::Enabled.0 == 1);
    assert!(SMAppServiceStatus::RequiresApproval.0 == 2);
    assert!(SMAppServiceStatus::NotFound.0 == 3);
};
