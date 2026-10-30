//! Consistent capture (B25-03): a live database is captured through SQLite's own online backup
//! from a steady snapshot, never copied raw; a changed source is not the approved one; grants are
//! checked at every capture; the old laptop's files are only read.

mod common;

use std::path::{Path, PathBuf};
use std::sync::OnceLock;
use std::time::Duration;

use common::*;
use pctwin_catalogue::Catalogue;
use pctwin_grant::{CaptureEffect, Now, PlanSeal, RoleKeys, RootRef, SecretEffect};
use pctwin_record::Owner;
use pctwin_special::capture::*;
use pctwin_special::catalogue::{SHIPPED, SpecialRow};
use pctwin_special::classify::{FolderReport, classify_all};
use pctwin_special::component::{Component, Custody, Evidence, Preservation};
use pctwin_special::roots::{System, find_folders, roots_of};

fn keys() -> &'static RoleKeys {
    static K: OnceLock<RoleKeys> = OnceLock::new();
    K.get_or_init(|| RoleKeys::mint().expect("minted once in this test program"))
}

const MINUTE: Duration = Duration::from_secs(60);

fn seal() -> PlanSeal {
    PlanSeal::new(9, [3; 32])
}

fn root() -> RootRef {
    RootRef::new(4)
}

struct Fixed(Holding);
impl Holders for Fixed {
    fn holding(&self, _: &[PathBuf]) -> Holding {
        self.0.clone()
    }
}

fn found(home: &Path) -> Vec<FolderReport> {
    let owner = Owner::Person {
        account_id: "1001".into(),
    };
    let (folders, _) = find_folders(
        &owner,
        home,
        &roots_of(System::Windows, home, &|_| None),
        &[],
    );
    classify_all(
        folders,
        System::Windows,
        &Catalogue::<SpecialRow>::shipped(SHIPPED),
    )
}

fn component<'a>(r: &'a [FolderReport], member: &str) -> (&'a FolderReport, &'a Component) {
    for f in r {
        if let Some(c) = f
            .components
            .iter()
            .find(|c| c.members.iter().any(|m| m == member))
        {
            return (f, c);
        }
    }
    panic!("{member} not found")
}

fn approve(f: &FolderReport, c: &Component) -> Approved {
    Approved {
        seal: seal(),
        component: c.id,
        root: root(),
        generation: generation(&f.folder.path(), &c.members),
        handle: 1,
    }
}

fn grant() -> pctwin_grant::CaptureGrant {
    keys()
        .capture
        .grant(
            seal(),
            vec![root()],
            CaptureEffect::ReadPrivate,
            MINUTE,
            Now::real(),
        )
        .unwrap()
}

fn snapshot(dir: &Path) -> Vec<(PathBuf, Vec<u8>, std::time::SystemTime)> {
    let mut out = Vec::new();
    for e in std::fs::read_dir(dir).unwrap() {
        let e = e.unwrap();
        let m = e.metadata().unwrap();
        if m.is_file() {
            out.push((
                e.path(),
                std::fs::read(e.path()).unwrap(),
                m.modified().unwrap(),
            ));
        }
    }
    out.sort();
    out
}

fn rows(db: &Path) -> i64 {
    let c = rusqlite::Connection::open_with_flags(
        pctwin_special::content::immutable_uri(db),
        rusqlite::OpenFlags::SQLITE_OPEN_READ_ONLY | rusqlite::OpenFlags::SQLITE_OPEN_URI,
    )
    .unwrap();
    c.query_row("SELECT count(*) FROM notes", [], |r| r.get(0))
        .unwrap()
}

/// T25-008: a database whose latest committed rows are only in its write-ahead log, while the
/// app keeps it open.
#[test]
fn a_live_database_is_captured_whole_through_sqlites_own_backup() {
    pctwin_special::content::install_reader(&OWN_READERS);
    let t = temp();
    let home = home_of(&t);
    let dir = home.join("AppData/Roaming/Live App");
    std::fs::create_dir_all(&dir).unwrap();
    let db = dir.join("notes.db");
    let app = rusqlite::Connection::open(&db).unwrap();
    app.execute_batch(
        "PRAGMA journal_mode=WAL; PRAGMA wal_autocheckpoint=0;
         CREATE TABLE notes(body TEXT);
         INSERT INTO notes VALUES('one'),('two'),('three');",
    )
    .unwrap();
    // The newest rows are in the log, not yet in the database file.
    assert!(wal_len(&db) > 0);

    let r = found(&home);
    let (f, c) = component(&r, "notes.db");
    assert!(
        c.members.contains(&"notes.db-wal".to_string()),
        "the log travels with it"
    );
    let approved = approve(f, c);
    let before = snapshot(&dir);

    // A raw copy of the database file alone misses what is in the log: the hazard.
    let raw = t.path().join("raw.db");
    std::fs::copy(&db, &raw).unwrap();
    assert_ne!(
        rusqlite::Connection::open(&raw)
            .and_then(|c| c.query_row("SELECT count(*) FROM notes", [], |r| r.get::<_, i64>(0)))
            .unwrap_or(-1),
        3
    );

    // On Windows the system says the app still holds it: capture waits for the app to close.
    if cfg!(windows) {
        let w = t.path().join("w0");
        std::fs::create_dir(&w).unwrap();
        let held = capture(
            grant(),
            &approved,
            c,
            &f.folder.path(),
            &w,
            &SystemHolders,
            &Stop::new(),
            Now::real(),
        );
        assert!(
            matches!(held, Err(CaptureError::AwaitingClose { .. })),
            "{held:?}"
        );
    }

    // With the app's writer quiet (the system could not say; the files stay steady), the online
    // backup gives one complete database with every committed row.
    let w = t.path().join("w1");
    std::fs::create_dir(&w).unwrap();
    let cap = capture(
        grant(),
        &approved,
        c,
        &f.folder.path(),
        &w,
        &Fixed(Holding::CannotTell),
        &Stop::new(),
        Now::real(),
    )
    .unwrap();
    assert_eq!(cap.method, Method::OnlineBackup);
    assert_eq!(cap.preservation, Preservation::Captured);
    assert!(cap.closed_unconfirmed);
    assert_eq!(cap.folded, vec!["notes.db-wal".to_string()]);
    assert_eq!(cap.files.len(), 1);
    assert_eq!(rows(&w.join("notes.db")), 3);
    assert!(!w.join("notes.db-wal").exists());
    assert_eq!(
        cap.files[0].hash,
        *blake3::hash(&std::fs::read(w.join("notes.db")).unwrap()).as_bytes()
    );
    // The old laptop's files were only read: same bytes, same times.
    assert_eq!(snapshot(&dir), before);
    drop(app);
}

fn wal_len(db: &Path) -> u64 {
    std::fs::metadata(pctwin_special::content::wal_of(db))
        .map(|m| m.len())
        .unwrap_or(0)
}

/// T25-009: the source changed after approval: not the approved generation.
#[test]
fn a_source_changed_after_approval_is_not_captured() {
    pctwin_special::content::install_reader(&OWN_READERS);
    let t = temp();
    let home = home_of(&t);
    let dir = home.join("AppData/Roaming/Journal");
    sqlite(
        &dir.join("j.db"),
        "CREATE TABLE notes(body TEXT); INSERT INTO notes VALUES('a');",
    );
    let r = found(&home);
    let (f, c) = component(&r, "j.db");
    let approved = approve(f, c);
    std::thread::sleep(Duration::from_millis(20));
    let conn = rusqlite::Connection::open(dir.join("j.db")).unwrap();
    conn.execute("INSERT INTO notes VALUES('b')", []).unwrap();
    drop(conn);
    let w = t.path().join("w");
    std::fs::create_dir(&w).unwrap();
    let got = capture(
        grant(),
        &approved,
        c,
        &f.folder.path(),
        &w,
        &Fixed(Holding::Free),
        &Stop::new(),
        Now::real(),
    );
    assert_eq!(got.unwrap_err(), CaptureError::ChangedSinceApproval);
    // Approved again, it is captured, with the new row.
    let again = approve(f, c);
    let cap = capture(
        grant(),
        &again,
        c,
        &f.folder.path(),
        &w,
        &Fixed(Holding::Free),
        &Stop::new(),
        Now::real(),
    )
    .unwrap();
    assert_eq!(cap.method, Method::OnlineBackup);
    assert_eq!(rows(&w.join("j.db")), 2);
}

/// T25-004 and T25-005 at run time; the compiler refuses a 2.1 grant outright (doc tests in
/// `pctwin-grant` and `capture_secret`).
#[test]
fn every_capture_checks_its_grant() {
    pctwin_special::content::install_reader(&OWN_READERS);
    let t = temp();
    let home = home_of(&t);
    ssh_folder(&home.join(".ssh"));
    put(&home.join("AppData/Roaming/Plain/data.json"), br#"{"a":1}"#);
    let r = found(&home);
    let (f, plain) = component(&r, "data.json");
    let approved = approve(f, plain);
    let w = t.path().join("w");
    std::fs::create_dir(&w).unwrap();
    let base = f.folder.path();
    let try_with = |g: pctwin_grant::CaptureGrant, now: Now| {
        capture(
            g,
            &approved,
            plain,
            &base,
            &w,
            &Fixed(Holding::Free),
            &Stop::new(),
            now,
        )
    };
    let k = &keys().capture;
    let denied = |r: Result<Capture, CaptureError>| matches!(r, Err(CaptureError::Denied(_)));
    assert!(denied(try_with(
        k.grant(
            seal(),
            vec![root()],
            CaptureEffect::Import,
            MINUTE,
            Now::real()
        )
        .unwrap(),
        Now::real()
    )));
    assert!(denied(try_with(
        k.grant(
            PlanSeal::new(10, [3; 32]),
            vec![root()],
            CaptureEffect::ReadPrivate,
            MINUTE,
            Now::real()
        )
        .unwrap(),
        Now::real()
    )));
    assert!(denied(try_with(
        k.grant(
            seal(),
            vec![RootRef::new(5)],
            CaptureEffect::ReadPrivate,
            MINUTE,
            Now::real()
        )
        .unwrap(),
        Now::real()
    )));
    let later = Now::at(
        std::time::Instant::now() + Duration::from_secs(120),
        std::time::SystemTime::now() + Duration::from_secs(120),
    );
    assert!(denied(try_with(grant(), later)));
    assert!(try_with(grant(), Now::real()).is_ok());

    // A secret needs a secret grant naming exactly it (the mail-archive grant is refused).
    let (sf, key) = component(&r, "id_ed25519");
    assert!(key.secret);
    let key_ok = approve(sf, key);
    let w2 = t.path().join("w2");
    std::fs::create_dir(&w2).unwrap();
    let sbase = sf.folder.path();
    let got = capture(
        grant(),
        &key_ok,
        key,
        &sbase,
        &w2,
        &Fixed(Holding::Free),
        &Stop::new(),
        Now::real(),
    );
    assert!(denied(got));
    let other = k
        .secret(
            seal(),
            vec![approved.object()],
            SecretEffect::Read,
            MINUTE,
            Now::real(),
        )
        .unwrap();
    assert!(denied(capture_secret(
        other,
        &key_ok,
        key,
        &sbase,
        &w2,
        &Fixed(Holding::Free),
        &Stop::new(),
        Now::real()
    )));
    let landing = k
        .secret(
            seal(),
            vec![key_ok.object()],
            SecretEffect::Land,
            MINUTE,
            Now::real(),
        )
        .unwrap();
    assert!(denied(capture_secret(
        landing,
        &key_ok,
        key,
        &sbase,
        &w2,
        &Fixed(Holding::Free),
        &Stop::new(),
        Now::real()
    )));
    // A secret grant does not reach an ordinary component either.
    let for_plain = k
        .secret(
            seal(),
            vec![approved.object()],
            SecretEffect::Read,
            MINUTE,
            Now::real(),
        )
        .unwrap();
    assert!(denied(capture_secret(
        for_plain,
        &approved,
        plain,
        &base,
        &w2,
        &Fixed(Holding::Free),
        &Stop::new(),
        Now::real()
    )));
    let right = k
        .secret(
            seal(),
            vec![key_ok.object()],
            SecretEffect::Read,
            MINUTE,
            Now::real(),
        )
        .unwrap();
    let cap = capture_secret(
        right,
        &key_ok,
        key,
        &sbase,
        &w2,
        &Fixed(Holding::Free),
        &Stop::new(),
        Now::real(),
    )
    .unwrap();
    assert_eq!(cap.preservation, Preservation::Captured);
    // The secret grant is bound to the approved generation: after a change it no longer fits.
    let changed = Approved {
        generation: Generation {
            digest: [0; 32],
            ..key_ok.generation.clone()
        },
        ..key_ok.clone()
    };
    assert_ne!(changed.object(), key_ok.object());
}

#[test]
fn bound_and_service_held_components_are_never_captured() {
    pctwin_special::content::install_reader(&OWN_READERS);
    let t = temp();
    let home = home_of(&t);
    chromium_profile(&home.join("AppData/Local/Chromey/User Data"));
    let mut ost = b"!BDN\0\0\0\0SO\x24\0".to_vec();
    ost.extend_from_slice(&[0; 64]);
    put(&home.join("AppData/Local/Mail Client/box.ost"), &ost);
    let r = found(&home);
    let w = t.path().join("w");
    std::fs::create_dir(&w).unwrap();
    for member in ["User Data/Default/Cookies", "User Data/Local State"] {
        let (f, c) = component(&r, member);
        assert_eq!(c.custody, Custody::MachineBound);
        let got = capture(
            grant(),
            &approve(f, c),
            c,
            &f.folder.path(),
            &w,
            &Fixed(Holding::Free),
            &Stop::new(),
            Now::real(),
        );
        assert!(matches!(got, Err(CaptureError::Denied(_))), "{member}");
    }
    let (f, account) = r
        .iter()
        .flat_map(|f| f.components.iter().map(move |c| (f, c)))
        .find(|(_, c)| c.evidence.contains(&Evidence::ServiceAccount))
        .unwrap();
    assert!(matches!(
        capture(
            grant(),
            &approve(f, account),
            account,
            &f.folder.path(),
            &w,
            &Fixed(Holding::Free),
            &Stop::new(),
            Now::real()
        ),
        Err(CaptureError::Denied(_))
    ));
    // T25-014: a server mailbox's offline copy is service-held, yet a labelled copy is kept,
    // because items that never reached the server cannot be ruled out.
    let (f, c) = component(&r, "box.ost");
    assert_eq!((c.custody, c.preserve), (Custody::ServiceHeld, true));
    let cap = capture(
        grant(),
        &approve(f, c),
        c,
        &f.folder.path(),
        &w,
        &Fixed(Holding::CannotTell),
        &Stop::new(),
        Now::real(),
    )
    .unwrap();
    assert_eq!(cap.preservation, Preservation::PreservedOnly);
    assert_eq!(cap.method, Method::PreservedOnly);
    assert_eq!(
        c.custody,
        Custody::ServiceHeld,
        "the result stays sign in, never copied"
    );
}

#[test]
fn a_store_whose_app_may_be_open_is_kept_labelled_never_copied() {
    pctwin_special::content::install_reader(&OWN_READERS);
    let t = temp();
    let home = home_of(&t);
    let d = home.join("AppData/Roaming/Store App/db");
    put(&d.join("CURRENT"), b"MANIFEST-000003\n");
    put(&d.join("MANIFEST-000003"), b"manifest");
    put(&d.join("000004.log"), b"log records");
    let r = found(&home);
    let (f, c) = component(&r, "db/CURRENT");
    assert!(c.evidence.contains(&Evidence::LiveStore));
    let run = |h: Holding, name: &str| {
        let w = t.path().join(name);
        std::fs::create_dir(&w).unwrap();
        capture(
            grant(),
            &approve(f, c),
            c,
            &f.folder.path(),
            &w,
            &Fixed(h),
            &Stop::new(),
            Now::real(),
        )
    };
    let unsure = run(Holding::CannotTell, "a").unwrap();
    assert_eq!(unsure.preservation, Preservation::PreservedOnly);
    let closed = run(Holding::Free, "b").unwrap();
    assert_eq!(
        (closed.method, closed.preservation),
        (Method::SteadyCopy, Preservation::Captured)
    );
    assert_eq!(closed.files.len(), 3);
    assert_eq!(
        run(
            Holding::Held {
                by: vec!["app".into()]
            },
            "c"
        )
        .unwrap_err(),
        CaptureError::AwaitingClose {
            by: vec!["app".into()]
        }
    );
    // The person may still keep it as it is, labelled.
    let w = t.path().join("d");
    std::fs::create_dir(&w).unwrap();
    let kept = preserve_as_is(
        grant(),
        &approve(f, c),
        c,
        &f.folder.path(),
        &w,
        &Stop::new(),
        Now::real(),
    )
    .unwrap();
    assert_eq!(kept.preservation, Preservation::PreservedOnly);
}

#[test]
fn files_named_by_their_hash_are_checked_and_a_mismatch_is_partial() {
    pctwin_special::content::install_reader(&OWN_READERS);
    let t = temp();
    let home = home_of(&t);
    let hashes = model_library(&home.join(".runner"), false);
    // Damage one blob: its bytes no longer match its name.
    std::fs::write(
        home.join(format!(".runner/models/blobs/sha256-{}", hashes[1])),
        b"tampered",
    )
    .unwrap();
    let r = found(&home);
    let (f, c) = component(&r, &format!("models/blobs/sha256-{}", hashes[0]));
    let w = t.path().join("w");
    std::fs::create_dir(&w).unwrap();
    let cap = capture(
        grant(),
        &approve(f, c),
        c,
        &f.folder.path(),
        &w,
        &Fixed(Holding::Free),
        &Stop::new(),
        Now::real(),
    )
    .unwrap();
    assert_eq!(cap.preservation, Preservation::Partial);
    assert!(cap.evidence.contains(&Evidence::HashMismatch));
}

#[test]
fn stopping_keeps_nothing_partly_read_and_keeps_the_private_folder() {
    pctwin_special::content::install_reader(&OWN_READERS);
    let t = temp();
    let home = home_of(&t);
    game_saves(&home.join("Saved Games/G"));
    let r = found(&home);
    let (f, c) = component(&r, "settings.ini");
    let w = t.path().join("w");
    std::fs::create_dir(&w).unwrap();
    let stop = Stop::new();
    stop.stop();
    let got = capture(
        grant(),
        &approve(f, c),
        c,
        &f.folder.path(),
        &w,
        &Fixed(Holding::Free),
        &stop,
        Now::real(),
    );
    assert_eq!(got.unwrap_err(), CaptureError::Stopped);
    assert!(w.is_dir(), "the private folder stays");
    assert_eq!(std::fs::read_dir(&w).unwrap().count(), 0);
}

/// Changes the first file of a component once its copy has begun (it shows in the work folder),
/// while the second, large file is still to be read: each file alone reads steady, but the two
/// were not read at one moment.
struct ChangesFirst {
    first: PathBuf,
    copied: PathBuf,
    writer: std::sync::Mutex<Option<std::thread::JoinHandle<()>>>,
}

impl Holders for ChangesFirst {
    fn holding(&self, _: &[PathBuf]) -> Holding {
        let (first, copied) = (self.first.clone(), self.copied.clone());
        *self.writer.lock().unwrap() = Some(std::thread::spawn(move || {
            let until = std::time::Instant::now() + MINUTE;
            while !copied.exists() && std::time::Instant::now() < until {
                std::thread::yield_now();
            }
            std::fs::write(&first, b"the index changed meanwhile").unwrap();
        }));
        Holding::Free
    }
}

/// A component carried raw (Apple Mail's index and its log) is read steady as a whole: a file
/// that changed after it was copied, while the rest was still read, refuses the capture.
#[test]
fn files_carried_raw_that_were_not_read_at_one_moment_are_not_captured() {
    let t = temp();
    let base = t.path().join("V10");
    put(&base.join("MailData/Envelope Index"), b"db");
    put(
        &base.join("MailData/Envelope Index-wal"),
        &vec![b'w'; 64 * 1024 * 1024],
    );
    let members = vec![
        "MailData/Envelope Index".to_string(),
        "MailData/Envelope Index-wal".to_string(),
    ];
    let c = Component::new(
        Owner::Person {
            account_id: "501".into(),
        },
        "Mail/V10/MailData",
        members.clone(),
        Custody::NeedsExport,
        false,
        vec![Evidence::MailStore, Evidence::CarriedRaw],
    );
    let approved = Approved {
        seal: seal(),
        component: c.id,
        root: root(),
        generation: generation(&base, &members),
        handle: 1,
    };
    let w = t.path().join("w");
    std::fs::create_dir(&w).unwrap();
    let app = ChangesFirst {
        first: base.join("MailData/Envelope Index"),
        copied: w.join("MailData").join("Envelope Index"),
        writer: Default::default(),
    };
    let got = capture(
        grant(),
        &approved,
        &c,
        &base,
        &w,
        &app,
        &Stop::new(),
        Now::real(),
    );
    app.writer.lock().unwrap().take().unwrap().join().unwrap();
    assert_eq!(got.unwrap_err(), CaptureError::KeptChanging);
}

/// Starts an app writing the file the moment capture asks whether anything holds it (after the
/// approved generation was checked), as an app that wakes up mid-move would.
struct WakesUp {
    file: PathBuf,
    done: std::sync::Arc<std::sync::atomic::AtomicBool>,
    writer: std::sync::Mutex<Option<std::thread::JoinHandle<()>>>,
}

impl Holders for WakesUp {
    fn holding(&self, _: &[PathBuf]) -> Holding {
        let (file, done) = (self.file.clone(), self.done.clone());
        let (started_tx, started_rx) = std::sync::mpsc::channel();
        *self.writer.lock().unwrap() = Some(std::thread::spawn(move || {
            use std::io::Write;
            let mut h = std::fs::OpenOptions::new()
                .append(true)
                .open(&file)
                .unwrap();
            let _ = started_tx.send(());
            // Appends without pause, each write growing the file, until capture is over.
            while !done.load(std::sync::atomic::Ordering::SeqCst) {
                h.write_all(b"x").unwrap();
            }
        }));
        started_rx.recv().unwrap();
        Holding::Free
    }
}

#[test]
fn a_file_that_keeps_changing_is_not_captured() {
    pctwin_special::content::install_reader(&OWN_READERS);
    let t = temp();
    let home = home_of(&t);
    let file = home.join("AppData/Roaming/Busy/log.txt");
    // Large enough that copying it takes long while the app keeps appending.
    put(&file, &vec![b'a'; 32 * 1024 * 1024]);
    let r = found(&home);
    let (f, c) = component(&r, "log.txt");
    let approved = approve(f, c);
    let app = WakesUp {
        file: file.clone(),
        done: Default::default(),
        writer: Default::default(),
    };
    let w = t.path().join("w");
    std::fs::create_dir(&w).unwrap();
    let got = capture(
        grant(),
        &approved,
        c,
        &f.folder.path(),
        &w,
        &app,
        &Stop::new(),
        Now::real(),
    );
    app.done.store(true, std::sync::atomic::Ordering::SeqCst);
    app.writer.lock().unwrap().take().unwrap().join().unwrap();
    assert_eq!(got.unwrap_err(), CaptureError::KeptChanging);
    assert_eq!(
        std::fs::read_dir(&w).unwrap().count(),
        0,
        "nothing partly read is kept"
    );
}

/// PCTwin's readers of files' contents, run in the test's own process (only a parser crate's
/// tests may).
static OWN_READERS: pctwin_special::content::InThisProcess =
    pctwin_special::content::InThisProcess::new(pctwin_parse_seal::in_child());
