//! Two PCTwin instances never use one journal at once (round 4 test plan item 7): the second, in
//! another process, is refused while the first holds it, and a first that crashes lets go. redb
//! holds an operating-system lock on the file for as long as the database is open (byte-range
//! locks plus `flock` on Linux and macOS, `LockFileEx` on Windows), which the system drops when
//! the process ends, however it ends.

use std::io::{BufRead, BufReader};
use std::process::{Command, Stdio};
use std::sync::mpsc;
use std::time::Duration;

use pctwin_journal::{Journal, JournalError};

const HOLD: &str = "PCTWIN_TEST_JOURNAL_HOLD";
const HELD: &str = "PCTWIN-JOURNAL-HELD-3f9c";

/// Run only as the child of [`a_second_process_cannot_open_a_journal_in_use`]: opens the journal
/// it is given, says so, and holds it until killed.
#[test]
fn holder_child() {
    let Ok(path) = std::env::var(HOLD) else {
        return;
    };
    let _held = Journal::open(std::path::Path::new(&path)).unwrap();
    println!("{HELD}");
    let mut line = String::new();
    let _ = std::io::stdin().read_line(&mut line);
}

#[test]
fn a_second_process_cannot_open_a_journal_in_use() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("journal.redb");
    drop(Journal::open(&path).unwrap());
    let mut child = Command::new(std::env::current_exe().unwrap())
        .args(["holder_child", "--exact", "--nocapture", "--test-threads=1"])
        .env(HOLD, &path)
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::null())
        .spawn()
        .unwrap();
    let out = child.stdout.take().unwrap();
    let (tx, rx) = mpsc::channel();
    std::thread::spawn(move || {
        for line in BufReader::new(out).lines() {
            let Ok(line) = line else { break };
            if line.contains(HELD) {
                let _ = tx.send(());
            }
        }
    });
    if rx.recv_timeout(Duration::from_secs(60)).is_err() {
        let _ = child.kill();
        panic!("the child never opened the journal");
    }
    let refused = matches!(Journal::open(&path), Err(JournalError::InUse));
    // A crash, not a clean close: the system lets go of the lock when the process ends.
    child.kill().unwrap();
    child.wait().unwrap();
    assert!(refused, "a second instance opened a journal in use");
    let again = Journal::open(&path);
    assert!(again.is_ok(), "{again:?}");
}
