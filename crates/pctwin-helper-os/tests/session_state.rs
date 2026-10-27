//! The two system questions the join of 2.2 and 2.3 asks (27 October 2026): is this program
//! elevated (an app is never started from one, T22-020), and is this program's session the one on
//! the screen (switching person ends the broker's grants, T22-023).
//!
//! Second review (low finding): each answer is checked against what the system's own tools say
//! of this very program, not only printed. Notes from that review: a locked screen still counts
//! as on the screen (locking does not switch person, so it does not end the grants); and with
//! the administrator prompt turned off (UAC off) an administrator's programs run with full
//! rights, so the app reads as elevated and refuses to start apps: it fails safe.

/// Asked for real: both answer, and agree with what the system says otherwise.
#[test]
fn elevation_and_session_are_answered() {
    let elevated = pctwin_helper_os::this_process_elevated().expect("the system answers");
    #[cfg(unix)]
    assert_eq!(elevated, rustix::process::geteuid().is_root());
    #[cfg(windows)]
    {
        let on_screen = pctwin_helper_os::session_on_screen().expect("the system answers");
        eprintln!("elevated: {elevated}, on screen: {on_screen}");
        // Windows' own `whoami`: a program with full rights runs at the high integrity level
        // (S-1-16-12288) or above; one with a person's ordinary rights at medium (S-1-16-8192).
        let groups = system_tool("whoami.exe", &["/groups", "/fo", "csv", "/nh"]);
        let high = groups.contains("\"S-1-16-12288\"") || groups.contains("\"S-1-16-16384\"");
        let medium = groups.contains("\"S-1-16-8192\"");
        assert!(high != medium, "one integrity level: {groups}");
        assert_eq!(elevated, high, "{groups}");
        // Windows' own session list marks this program's session with ">"; it is on the screen
        // when its state is Active. A job run as a service (some CI runners) has no screen of
        // its own, and must say so.
        let sessions = system_tool("qwinsta.exe", &[]);
        if let Some(mine) = sessions.lines().find(|l| l.starts_with('>')) {
            assert_eq!(on_screen, mine.contains("Active"), "{sessions}");
        } else {
            assert!(!on_screen, "{sessions}");
        }
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

/// A Windows tool from `System32`, found and started with the system's own environment only.
#[cfg(windows)]
fn system_tool(name: &str, args: &[&str]) -> String {
    let program = pctwin_helper_os::system_folder(pctwin_helper_os::SystemFolder::Windows)
        .unwrap()
        .join("System32")
        .join(name);
    let out = pctwin_helper_os::clean_command(&program)
        .unwrap()
        .args(args)
        .output()
        .unwrap();
    String::from_utf8_lossy(&out.stdout).into_owned()
}
