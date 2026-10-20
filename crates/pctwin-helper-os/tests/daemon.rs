//! macOS's launch daemon, through a stand-in for Service Management: every answer is typed.
//! (Registration itself needs a Developer ID signature, which CI builds do not have.)

use pctwin_helper_os::Error;
use pctwin_helper_os::daemon::{
    DaemonService, DaemonStart, DaemonState, HELPER_PLIST, start_daemon_with, stop_daemon_with,
};
use std::cell::{Cell, RefCell};

struct StandIn {
    register_fails: bool,
    state_after: DaemonState,
    asked: RefCell<Vec<String>>,
    opened: Cell<bool>,
}

impl StandIn {
    fn new(register_fails: bool, state_after: DaemonState) -> Self {
        StandIn {
            register_fails,
            state_after,
            asked: RefCell::new(vec![]),
            opened: Cell::new(false),
        }
    }
}

impl DaemonService for StandIn {
    fn register(&self, plist: &str) -> Result<(), Error> {
        self.asked.borrow_mut().push(format!("register {plist}"));
        if self.register_fails {
            Err(Error::Os {
                call: "SMAppService.register",
                code: 1,
            })
        } else {
            Ok(())
        }
    }
    fn unregister(&self, plist: &str) -> Result<(), Error> {
        self.asked.borrow_mut().push(format!("unregister {plist}"));
        Ok(())
    }
    fn status(&self, _plist: &str) -> DaemonState {
        self.state_after
    }
    fn open_approval_settings(&self) {
        self.opened.set(true);
    }
}

#[test]
fn enabled_is_running() {
    let s = StandIn::new(false, DaemonState::Enabled);
    assert_eq!(
        start_daemon_with(&s, HELPER_PLIST).unwrap(),
        DaemonStart::Running
    );
    assert_eq!(*s.asked.borrow(), [format!("register {HELPER_PLIST}")]);
}

#[test]
fn not_yet_approved_is_a_typed_answer_even_when_register_errs() {
    for fails in [false, true] {
        let s = StandIn::new(fails, DaemonState::RequiresApproval);
        assert_eq!(
            start_daemon_with(&s, HELPER_PLIST).unwrap(),
            DaemonStart::NeedsApproval
        );
        assert!(!s.opened.get(), "settings open only when the app asks");
    }
}

#[test]
fn a_missing_daemon_or_failed_register_is_an_error() {
    let s = StandIn::new(false, DaemonState::NotFound);
    assert!(matches!(
        start_daemon_with(&s, HELPER_PLIST),
        Err(Error::BadInput(_))
    ));
    let s = StandIn::new(true, DaemonState::NotRegistered);
    assert!(matches!(
        start_daemon_with(&s, HELPER_PLIST),
        Err(Error::Os { code: 1, .. })
    ));
    let s = StandIn::new(false, DaemonState::Unknown(7));
    assert!(start_daemon_with(&s, HELPER_PLIST).is_err());
}

#[test]
fn bad_names_never_reach_the_system() {
    let s = StandIn::new(false, DaemonState::Enabled);
    assert!(start_daemon_with(&s, "../evil.plist").is_err());
    assert!(stop_daemon_with(&s, "x/y.plist").is_err());
    assert!(s.asked.borrow().is_empty());
}

#[test]
fn stopping_ends_not_registered() {
    let s = StandIn::new(false, DaemonState::NotRegistered);
    assert!(stop_daemon_with(&s, HELPER_PLIST).is_ok());
    let s = StandIn::new(false, DaemonState::Enabled);
    assert!(
        stop_daemon_with(&s, HELPER_PLIST).is_err(),
        "still on is not stopped"
    );
}

/// The real Service Management, read only: an unsigned test program asking about the helper's
/// daemon gets a state back, not a crash.
#[cfg(target_os = "macos")]
#[test]
#[ignore = "needs PCTWIN_HELPER_OS_ADMIN_TESTS=1 (CI); reads Service Management"]
fn the_real_status_read_is_typed() {
    use pctwin_helper_os::daemon::SystemDaemons;
    assert!(
        std::env::var("PCTWIN_HELPER_OS_ADMIN_TESTS").as_deref() == Ok("1"),
        "set PCTWIN_HELPER_OS_ADMIN_TESTS=1 (CI only)"
    );
    let state = SystemDaemons.status(HELPER_PLIST);
    println!("Service Management says: {state:?}");
    assert_ne!(state, DaemonState::Enabled, "nothing registered it here");
}

#[cfg(not(target_os = "macos"))]
#[test]
fn elsewhere_daemons_are_not_supported() {
    assert!(matches!(
        pctwin_helper_os::daemon::start_daemon(HELPER_PLIST),
        Err(Error::NotSupported(_))
    ));
}
