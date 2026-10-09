//! The write lease on Linux (Security Design 3B undo, decided 9 October 2026): granted only when
//! nothing else has the file open; a new open while it is held waits, is noticed, and does not
//! end PCTwin; PCTwin's own rename and removal keep it.
#![cfg(target_os = "linux")]

use std::io::{BufRead, BufReader, Write};
use std::os::fd::AsRawFd;
use std::path::{Path, PathBuf};
use std::process::{Child, Command, Stdio};
use std::time::{Duration, Instant};

use pctwin_lease::{Lease, release, still_held, write_lease};

/// What a helper process does with the file named in PCTWIN_LEASE_PATH, run as this same test
/// binary (`helper` below) so no outside tool is needed.
#[test]
fn helper() {
    let (Ok(role), Ok(path)) = (
        std::env::var("PCTWIN_LEASE_ROLE"),
        std::env::var("PCTWIN_LEASE_PATH"),
    ) else {
        return;
    };
    let ready = || {
        println!("ready");
        std::io::stdout().flush().unwrap();
    };
    match role.as_str() {
        "read" => {
            let _f = std::fs::File::open(&path).unwrap();
            ready();
            std::thread::sleep(Duration::from_secs(20));
        }
        "write" => {
            let _f = std::fs::OpenOptions::new()
                .append(true)
                .open(&path)
                .unwrap();
            ready();
            std::thread::sleep(Duration::from_secs(20));
        }
        "map" => {
            let f = std::fs::File::open(&path).unwrap();
            #[allow(unsafe_code, reason = "test helper: a mapping with its handle closed")]
            let p = unsafe {
                libc::mmap(
                    std::ptr::null_mut(),
                    4096,
                    libc::PROT_READ,
                    libc::MAP_SHARED,
                    f.as_raw_fd(),
                    0,
                )
            };
            assert_ne!(p, libc::MAP_FAILED);
            drop(f);
            ready();
            std::thread::sleep(Duration::from_secs(20));
        }
        "open-later" => {
            let t = Instant::now();
            let ok = std::fs::File::open(&path).is_ok();
            println!("opened {ok} after {}", t.elapsed().as_millis());
        }
        _ => panic!("unknown role"),
    }
}

fn spawn(role: &str, path: &Path, wait_ready: bool) -> Child {
    let mut child = Command::new(std::env::current_exe().unwrap())
        .args(["--exact", "helper", "--nocapture", "--test-threads=1"])
        .env("PCTWIN_LEASE_ROLE", role)
        .env("PCTWIN_LEASE_PATH", path)
        .stdout(Stdio::piped())
        .spawn()
        .unwrap();
    if wait_ready {
        let mut out = BufReader::new(child.stdout.take().unwrap());
        let mut line = String::new();
        loop {
            line.clear();
            assert!(out.read_line(&mut line).unwrap() > 0, "helper ended early");
            // The test runner may print its own "test helper ... " first on the same line.
            if line.trim_end().ends_with("ready") {
                break;
            }
        }
    }
    child
}

/// One test at a time: a process started by another test briefly shares every open handle (between
/// its start and running its program), which correctly counts as "another open" and would make
/// these results depend on timing.
fn one_at_a_time() -> std::sync::MutexGuard<'static, ()> {
    static ONE: std::sync::Mutex<()> = std::sync::Mutex::new(());
    ONE.lock()
        .unwrap_or_else(std::sync::PoisonError::into_inner)
}

fn fresh(dir: &Path, name: &str) -> PathBuf {
    let p = dir.join(name);
    std::fs::write(&p, [7u8; 8192]).unwrap();
    p
}

fn read_only(p: &Path) -> std::fs::File {
    std::fs::File::open(p).unwrap()
}

/// Leases switched off or a drive without them: skip (the rig runs these on ext4, XFS, btrfs).
fn leases_work(dir: &Path) -> bool {
    let p = fresh(dir, "probe");
    let f = read_only(&p);
    match write_lease(&f) {
        Lease::Held => {
            release(&f).unwrap();
            true
        }
        other => {
            eprintln!("leases not available here: {other:?}");
            false
        }
    }
}

#[test]
fn granted_only_when_nothing_else_has_the_file_open() {
    let _one = one_at_a_time();
    let dir = tempfile::tempdir().unwrap();
    if !leases_work(dir.path()) {
        return;
    }
    let p = fresh(dir.path(), "alone");
    let f = read_only(&p);
    assert!(matches!(write_lease(&f), Lease::Held));
    assert!(still_held(&f));
    release(&f).unwrap();
    assert!(!still_held(&f));
    for role in ["read", "write", "map"] {
        let p = fresh(dir.path(), role);
        let mut other = spawn(role, &p, true);
        let f = read_only(&p);
        let got = write_lease(&f);
        other.kill().unwrap();
        other.wait().unwrap();
        assert!(matches!(got, Lease::InUse), "{role}: {got:?}");
    }
    // PCTwin's own second handle counts too.
    let p = fresh(dir.path(), "twice");
    let f = read_only(&p);
    let _g = read_only(&p);
    assert!(matches!(write_lease(&f), Lease::InUse));
}

#[test]
fn a_new_open_while_held_waits_is_noticed_and_does_not_end_pctwin() {
    let _one = one_at_a_time();
    let dir = tempfile::tempdir().unwrap();
    if !leases_work(dir.path()) {
        return;
    }
    let p = fresh(dir.path(), "busy");
    let f = read_only(&p);
    assert!(matches!(write_lease(&f), Lease::Held));
    let opener = spawn("open-later", &p, false);
    let t = Instant::now();
    while still_held(&f) && t.elapsed() < Duration::from_secs(10) {
        std::thread::sleep(Duration::from_millis(1));
    }
    assert!(!still_held(&f), "the waiting open was not noticed");
    std::thread::sleep(Duration::from_millis(100));
    release(&f).unwrap();
    let out = opener.wait_with_output().unwrap();
    let said = String::from_utf8_lossy(&out.stdout);
    let waited: u64 = said
        .lines()
        .find_map(|l| l.split_once("opened true after ").map(|(_, ms)| ms))
        .expect("the other program opened it")
        .trim()
        .parse()
        .unwrap();
    assert!(
        waited >= 50,
        "it should have waited for the release ({waited} ms)"
    );
}

#[test]
fn pctwin_moving_and_removing_the_file_keeps_the_lease() {
    let _one = one_at_a_time();
    let dir = tempfile::tempdir().unwrap();
    if !leases_work(dir.path()) {
        return;
    }
    let p = fresh(dir.path(), "mine");
    let f = read_only(&p);
    assert!(matches!(write_lease(&f), Lease::Held));
    let private = dir.path().join(".pctwin-undo-x");
    std::fs::rename(&p, &private).unwrap();
    assert!(still_held(&f));
    std::fs::remove_file(&private).unwrap();
    assert!(still_held(&f));
    assert_eq!(f.metadata().unwrap().nlink(), 0);
}

use std::os::unix::fs::MetadataExt;
