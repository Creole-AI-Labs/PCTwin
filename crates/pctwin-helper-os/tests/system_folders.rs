//! Windows: the elevated helper finds the system's folders by asking Windows, never from the
//! environment, which whoever starts it sets (Security Design part C). A copy of this test program
//! runs with every folder variable pointing at a planted folder; it must still find the real ones.
#![cfg(windows)]

use pctwin_helper_os::{SystemFolder, system_folder};
use std::process::Command;

const PLANTED: &str = r"C:\pctwin-planted";

/// Run in a copy of this program, with the environment planted: the folders found are not the
/// planted one.
#[test]
#[ignore = "run by the test below, in a process of its own"]
fn found_with_a_planted_environment() {
    if std::env::var("PCTWIN_PLANTED").is_err() {
        return;
    }
    // ProgramFiles is set again by Windows for each new process (to match its bitness), so it
    // cannot be planted this way; the others can.
    for var in ["SystemRoot", "windir", "SystemDrive", "ProgramData"] {
        assert_eq!(std::env::var(var).unwrap(), PLANTED, "{var} is planted");
    }
    for which in [SystemFolder::Windows, SystemFolder::ProgramFiles] {
        let found = system_folder(which).unwrap();
        assert!(found.is_absolute(), "{which:?}: {}", found.display());
        assert!(
            !found.starts_with(PLANTED),
            "{which:?} came from the environment: {}",
            found.display()
        );
        assert!(found.is_dir(), "{which:?}: {}", found.display());
    }
    // Windows is where the system's own programs are.
    let windows = system_folder(SystemFolder::Windows).unwrap();
    assert!(windows.join("System32").join("reg.exe").is_file());
}

#[test]
fn the_system_folders_never_come_from_the_environment() {
    let mut run = Command::new(std::env::current_exe().unwrap());
    run.args([
        "--exact",
        "found_with_a_planted_environment",
        "--ignored",
        "--nocapture",
    ])
    .env("PCTWIN_PLANTED", "1");
    for var in [
        "SystemRoot",
        "windir",
        "SystemDrive",
        "ProgramFiles",
        "ProgramData",
    ] {
        run.env(var, PLANTED);
    }
    let out = run.output().unwrap();
    let text = String::from_utf8_lossy(&out.stdout);
    assert!(
        out.status.success() && text.contains("1 passed"),
        "{text}\n{}",
        String::from_utf8_lossy(&out.stderr)
    );
}
