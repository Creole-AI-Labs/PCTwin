//! The two system questions the join of 2.2 and 2.3 asks (27 October 2026): is this program
//! elevated (an app is never started from one, T22-020), and is this program's session the one on
//! the screen (switching person ends the broker's grants, T22-023).

/// Asked for real: both answer, and agree with what the system says otherwise.
#[test]
fn elevation_and_session_are_answered() {
    let elevated = pctwin_helper_os::this_process_elevated().expect("the system answers");
    #[cfg(unix)]
    assert_eq!(elevated, rustix::process::geteuid().is_root());
    #[cfg(windows)]
    {
        // A test run from a person's own desktop is on the screen; a job run as a service (some
        // CI runners) has no screen of its own, and says so rather than guessing.
        let on_screen = pctwin_helper_os::session_on_screen().expect("the system answers");
        eprintln!("elevated: {elevated}, on screen: {on_screen}");
    }
    #[cfg(not(windows))]
    {
        let _ = elevated;
        assert!(
            pctwin_helper_os::session_on_screen().is_err(),
            "cannot be told here"
        );
    }
}
