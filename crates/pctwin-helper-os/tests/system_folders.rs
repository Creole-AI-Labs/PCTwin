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

/// Programs the elevated helper starts get nothing of its environment but the few values worked
/// out from Windows' own folder (a planted COR_PROFILER or SystemRoot never reaches them).
#[test]
fn programs_started_by_the_helper_get_only_the_minimal_environment() {
    use pctwin_helper_os::{clean_command, minimal_env};
    // cargo test always sets this in the parent.
    assert!(std::env::var_os("CARGO_PKG_NAME").is_some());
    let windows = system_folder(SystemFolder::Windows).unwrap();
    let out = clean_command(&windows.join("System32").join("cmd.exe"))
        .unwrap()
        .args(["/c", "set"])
        .output()
        .unwrap();
    let seen = String::from_utf8_lossy(&out.stdout).to_uppercase();
    assert!(!seen.contains("CARGO_PKG_NAME="), "{seen}");
    for (key, _) in minimal_env().unwrap() {
        let key = key.to_string_lossy().to_uppercase();
        assert!(seen.contains(&format!("{key}=")), "{key} missing: {seen}");
    }
}

/// Second review, low finding: a disk can be asked, without an administrator, whether it can be
/// unplugged; this laptop's first disk (inside it) says it cannot.
#[cfg(windows)]
#[test]
fn the_laptops_own_disk_says_it_cannot_be_unplugged() {
    let h = pctwin_helper_os::disk_hotplug(0).expect("disk 0 answers");
    assert!(!h.device_hotplug, "{h:?}");
    assert!(!h.media_removable, "{h:?}");
    assert!(pctwin_helper_os::disk_hotplug(9999).is_err());
}
