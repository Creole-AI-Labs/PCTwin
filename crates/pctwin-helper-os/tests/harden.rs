//! The mover's engine, hardened while it carries other people's things (Security Design part C):
//! another program run by the same person cannot read its memory. A copy of this test program
//! stands in for the engine; a second copy, not hardened, shows the same look works on an
//! ordinary process, so the check is real.

#[cfg(any(windows, target_os = "linux"))]
use std::io::{BufRead, BufReader};
#[cfg(any(windows, target_os = "linux"))]
use std::process::{Child, Command, Stdio};

/// Run in a copy of this program: hardens itself when asked, says so, and waits to be ended.
#[test]
#[ignore = "run by the test below, in a process of its own"]
fn stand_in_engine() {
    if std::env::var("PCTWIN_STAND_IN").is_err() {
        return;
    }
    if std::env::var("PCTWIN_HARDEN").as_deref() == Ok("1") {
        pctwin_helper_os::harden_this_process().unwrap();
    }
    println!("ready");
    std::thread::sleep(std::time::Duration::from_secs(60));
}

#[cfg(any(windows, target_os = "linux"))]
fn start(harden: bool) -> Child {
    let mut child = Command::new(std::env::current_exe().unwrap())
        .args(["--exact", "stand_in_engine", "--ignored", "--nocapture"])
        .env("PCTWIN_STAND_IN", "1")
        .env("PCTWIN_HARDEN", if harden { "1" } else { "0" })
        .stdout(Stdio::piped())
        .spawn()
        .unwrap();
    let out = child.stdout.take().unwrap();
    let ready = BufReader::new(out)
        .lines()
        .map_while(Result::ok)
        .any(|l| l.contains("ready"));
    assert!(ready, "the stand-in started");
    child
}

/// Whether this account can read the process's memory. (macOS: the hardened runtime, set when the
/// app is signed, keeps it unreadable; nothing to look at in an unsigned test build.)
#[cfg(any(windows, target_os = "linux"))]
fn can_read(pid: u32) -> bool {
    #[cfg(windows)]
    {
        // Listing a process's modules reads its memory; one that cannot be read lists none
        // (.NET says nothing), and every running program has at least its own.
        let probe = format!(
            "try {{ if ((Get-Process -Id {pid}).Modules.Count -gt 0) {{ exit 0 }} else {{ exit 1 }} }} catch {{ exit 1 }}"
        );
        Command::new("powershell")
            .args(["-NoProfile", "-NonInteractive", "-Command", &probe])
            .status()
            .unwrap()
            .success()
    }
    #[cfg(target_os = "linux")]
    {
        use std::io::{Read, Seek, SeekFrom};
        let Ok(mut mem) = std::fs::File::open(format!("/proc/{pid}/mem")) else {
            return false;
        };
        let maps = std::fs::read_to_string(format!("/proc/{pid}/maps")).unwrap_or_default();
        let Some(start) = maps
            .lines()
            .next()
            .and_then(|l| l.split('-').next())
            .and_then(|a| u64::from_str_radix(a, 16).ok())
        else {
            return false;
        };
        mem.seek(SeekFrom::Start(start)).is_ok() && mem.read(&mut [0u8; 1]).is_ok()
    }
}

#[test]
#[cfg(any(windows, target_os = "linux"))]
fn the_same_person_cannot_read_the_hardened_engine() {
    let mut plain = start(false);
    let mut hardened = start(true);
    let (read_plain, read_hardened) = (can_read(plain.id()), can_read(hardened.id()));
    let _ = plain.kill();
    let _ = hardened.kill();
    let _ = plain.wait();
    let _ = hardened.wait();
    assert!(
        read_plain,
        "an ordinary process can be read: the look works"
    );
    // The threat is the mover with ordinary rights. An administrator can open any process (CI's
    // Windows runners run as one), so there this test cannot make its claim; the two-accounts
    // lab (fixtures/everyone) makes it as a real ordinary account.
    if windows_admin() {
        eprintln!(
            "run as an administrator: the hardened engine is checked by the two-accounts lab"
        );
        return;
    }
    assert!(!read_hardened, "the hardened engine cannot be read");
}

/// Windows: whether this test runs with administrator rights (`net session` succeeds only then).
#[cfg(any(windows, target_os = "linux"))]
fn windows_admin() -> bool {
    cfg!(windows)
        && Command::new("net")
            .arg("session")
            .stdout(Stdio::null())
            .stderr(Stdio::null())
            .status()
            .is_ok_and(|s| s.success())
}
