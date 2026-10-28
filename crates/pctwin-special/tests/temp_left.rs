//! The old laptop is read-only: reading a live database (one with its log beside it) leaves no
//! copy in the temporary folder, and copies a stopped PCTwin left there are swept (sixth review,
//! finding 1). Synthetic data only, in folders this test makes; each read runs in a child process
//! whose temporary folder is the test's own.
use std::path::{Path, PathBuf};

const MARKER: &str = "SESSION-MARKER-7f3a9c";

fn files_under(p: &Path) -> Vec<PathBuf> {
    let mut out = Vec::new();
    for e in std::fs::read_dir(p).into_iter().flatten().flatten() {
        let path = e.path();
        if path.is_dir() {
            out.extend(files_under(&path));
        } else {
            out.push(path);
        }
    }
    out
}

/// The child: reads one database's tables, then its facts, and exits normally.
#[test]
fn child_reads_a_database() {
    let Some(db) = std::env::var_os("PCTWIN_TEST_DB") else {
        return;
    };
    pctwin_special::content::sqlite_tables(Path::new(&db)).unwrap();
    let until = std::time::Instant::now() + std::time::Duration::from_secs(30);
    pctwin_special::content::sqlite_facts(Path::new(&db), until).unwrap();
}

/// The child: sweeps leftover snapshots.
#[test]
fn child_sweeps() {
    if std::env::var_os("PCTWIN_TEST_SWEEP").is_none() {
        return;
    }
    println!("SWEPT {}", pctwin_special::content::sweep_snapshots());
}

fn child(test: &str, var: (&str, &Path), temp: &Path) -> String {
    let out = std::process::Command::new(std::env::current_exe().unwrap())
        .args([test, "--exact", "--nocapture", "--test-threads", "1"])
        .env(var.0, var.1)
        // The system's temporary folder: TMP / TEMP on Windows, TMPDIR on Mac and Linux.
        .env("TMP", temp)
        .env("TEMP", temp)
        .env("TMPDIR", temp)
        .output()
        .unwrap();
    assert!(out.status.success(), "{out:?}");
    String::from_utf8_lossy(&out.stdout).into_owned()
}

#[test]
fn reading_a_live_database_leaves_nothing_in_the_temporary_folder() {
    let base = tempfile::tempdir().unwrap();
    let temp = base.path().join("temp");
    std::fs::create_dir_all(&temp).unwrap();
    let db = base.path().join("Cookies.db");
    // A running app: write-ahead log, never folded back, the connection held open.
    let c = rusqlite::Connection::open(&db).unwrap();
    c.execute_batch(
        "PRAGMA journal_mode=WAL; PRAGMA wal_autocheckpoint=0;
         CREATE TABLE cookies(host_key TEXT, name TEXT, value TEXT);",
    )
    .unwrap();
    c.execute(
        "INSERT INTO cookies VALUES('bank.example','session',?1)",
        [MARKER],
    )
    .unwrap();
    let wal = PathBuf::from(format!("{}-wal", db.display()));
    assert!(
        std::fs::metadata(&wal).unwrap().len() > 0,
        "the log is beside it"
    );
    child("child_reads_a_database", ("PCTWIN_TEST_DB", &db), &temp);
    assert_eq!(
        files_under(&temp),
        Vec::<PathBuf>::new(),
        "nothing left behind"
    );
    drop(c);
}

#[test]
fn snapshots_a_stopped_pctwin_left_are_swept_and_nothing_else() {
    let base = tempfile::tempdir().unwrap();
    let temp = base.path().join("temp");
    let left = temp.join(format!(
        "{}abc123",
        pctwin_special::content::SNAPSHOT_PREFIX
    ));
    std::fs::create_dir_all(&left).unwrap();
    std::fs::write(left.join("db"), MARKER).unwrap();
    let other = temp.join("someone-elses");
    std::fs::create_dir_all(&other).unwrap();
    std::fs::write(other.join("keep"), "theirs").unwrap();
    let said = child("child_sweeps", ("PCTWIN_TEST_SWEEP", Path::new("1")), &temp);
    assert!(said.contains("SWEPT 1"), "{said}");
    assert!(!left.exists());
    assert!(
        other.join("keep").exists(),
        "only PCTwin's own snapshots are removed"
    );
}
