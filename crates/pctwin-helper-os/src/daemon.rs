//! macOS: the helper as a launch daemon the app carries, switched on through Service Management
//! (`SMAppService`) for the length of a move and off at its end.
//!
//! The first time, macOS does not start it until an administrator allows it in System Settings,
//! Login Items: [`start_daemon`] then answers [`DaemonStart::NeedsApproval`], and the app may
//! open that page ([`DaemonService::open_approval_settings`]) and ask again.
//!
//! Registration needs the app signed with a Developer ID; CI builds are unsigned, so CI checks
//! this logic through a stand-in [`DaemonService`] and reads the real state only.

use crate::Error;

/// The daemon's property list, as the app bundle carries it in `Contents/Library/LaunchDaemons`
/// (from `packaging/macos`).
pub const HELPER_PLIST: &str = "com.creoleailabs.pctwin.helper.plist";

/// A launch daemon's state, as Service Management reports it.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum DaemonState {
    NotRegistered,
    Enabled,
    /// An administrator must allow it in System Settings, Login Items.
    RequiresApproval,
    /// The app does not carry that property list.
    NotFound,
    Unknown(isize),
}

impl DaemonState {
    /// From Apple's numbering (`SMAppServiceStatus`).
    pub fn from_raw(raw: isize) -> DaemonState {
        match raw {
            0 => DaemonState::NotRegistered,
            1 => DaemonState::Enabled,
            2 => DaemonState::RequiresApproval,
            3 => DaemonState::NotFound,
            other => DaemonState::Unknown(other),
        }
    }
}

/// What became of switching the daemon on.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum DaemonStart {
    /// It is on; launchd runs it.
    Running,
    /// An administrator has not allowed it yet (or said no): nothing runs.
    NeedsApproval,
}

/// Service Management, or a stand-in for it.
pub trait DaemonService {
    fn register(&self, plist: &str) -> Result<(), Error>;
    fn unregister(&self, plist: &str) -> Result<(), Error>;
    fn status(&self, plist: &str) -> DaemonState;
    fn open_approval_settings(&self);
}

#[cfg(target_os = "macos")]
pub use crate::mac::SystemDaemons;

fn check_plist_name(plist: &str) -> Result<(), Error> {
    let stem = plist.strip_suffix(".plist").unwrap_or("");
    if stem.is_empty()
        || stem.starts_with('.')
        || !stem
            .bytes()
            .all(|b| b.is_ascii_alphanumeric() || b == b'.' || b == b'-')
    {
        return Err(Error::BadInput("not a property list name"));
    }
    Ok(())
}

/// Switches the daemon on. The state after asking decides the answer, not the call's own
/// result: macOS reports "needs approval" as an error from the call and as a state.
pub fn start_daemon_with(service: &impl DaemonService, plist: &str) -> Result<DaemonStart, Error> {
    check_plist_name(plist)?;
    let registered = service.register(plist);
    match service.status(plist) {
        DaemonState::Enabled => Ok(DaemonStart::Running),
        DaemonState::RequiresApproval => Ok(DaemonStart::NeedsApproval),
        DaemonState::NotFound => Err(Error::BadInput("the app does not carry this launch daemon")),
        DaemonState::NotRegistered | DaemonState::Unknown(_) => {
            Err(registered.err().unwrap_or(Error::Os {
                call: "SMAppService.register",
                code: -1,
            }))
        }
    }
}

/// Switches the daemon off; already off is fine.
pub fn stop_daemon_with(service: &impl DaemonService, plist: &str) -> Result<(), Error> {
    check_plist_name(plist)?;
    let unregistered = service.unregister(plist);
    match service.status(plist) {
        DaemonState::NotRegistered | DaemonState::NotFound => Ok(()),
        _ => unregistered.and(Err(Error::Os {
            call: "SMAppService.unregister",
            code: -1,
        })),
    }
}

/// [`start_daemon_with`] through the system (macOS only).
pub fn start_daemon(plist: &str) -> Result<DaemonStart, Error> {
    #[cfg(target_os = "macos")]
    {
        start_daemon_with(&SystemDaemons, plist)
    }
    #[cfg(not(target_os = "macos"))]
    {
        let _ = plist;
        Err(Error::NotSupported(
            "launch daemons are macOS only; elsewhere use launch_elevated",
        ))
    }
}

/// [`stop_daemon_with`] through the system (macOS only).
pub fn stop_daemon(plist: &str) -> Result<(), Error> {
    #[cfg(target_os = "macos")]
    {
        stop_daemon_with(&SystemDaemons, plist)
    }
    #[cfg(not(target_os = "macos"))]
    {
        let _ = plist;
        Err(Error::NotSupported(
            "launch daemons are macOS only; elsewhere use launch_elevated",
        ))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn plist_names_are_plain() {
        assert!(check_plist_name(HELPER_PLIST).is_ok());
        for bad in [
            "",
            ".plist",
            "../x.plist",
            "a/b.plist",
            "x",
            "x.plist/",
            ".x.plist",
        ] {
            assert!(check_plist_name(bad).is_err(), "{bad}");
        }
    }

    #[test]
    fn states_follow_apples_numbers() {
        assert_eq!(DaemonState::from_raw(0), DaemonState::NotRegistered);
        assert_eq!(DaemonState::from_raw(1), DaemonState::Enabled);
        assert_eq!(DaemonState::from_raw(2), DaemonState::RequiresApproval);
        assert_eq!(DaemonState::from_raw(3), DaemonState::NotFound);
        assert_eq!(DaemonState::from_raw(9), DaemonState::Unknown(9));
    }
}
