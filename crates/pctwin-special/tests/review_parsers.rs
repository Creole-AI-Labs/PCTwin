//! Regression tests from the fresh-context review of the parser process (review of 8d0ccea):
//! F1, capture never has SQLite open another app's database in this process; F2, a write-ahead
//! log can never make the reader grow a database past its bound.

mod common;

use std::path::{Path, PathBuf};
use std::sync::OnceLock;
use std::sync::atomic::{AtomicBool, AtomicUsize, Ordering};
use std::time::Duration;

use common::*;
use pctwin_catalogue::Catalogue;
use pctwin_grant::{CaptureEffect, Now, PlanSeal, RoleKeys, RootRef};
use pctwin_record::Owner;
use pctwin_special::capture::*;
use pctwin_special::catalogue::{SHIPPED, SpecialRow};
use pctwin_special::classify::{FolderReport, classify_all};
use pctwin_special::component::Component;
use pctwin_special::content::{
    ContentError, ContentReader, DbFacts, InThisProcess, JsonFacts, MAX_DB, apply_log,
};
use pctwin_special::roots::{System, find_folders, roots_of};

// ---------- A write-ahead log, built by the file format (sqlite.org/fileformat2.html 4.1) ----------

fn cks(s: (u32, u32), data: &[u8]) -> (u32, u32) {
    let (mut s0, mut s1) = s;
    for w in data.as_chunks::<8>().0 {
        s0 = s0
            .wrapping_add(u32::from_le_bytes(w[..4].try_into().unwrap()))
            .wrapping_add(s1);
        s1 = s1
            .wrapping_add(u32::from_le_bytes(w[4..].try_into().unwrap()))
            .wrapping_add(s0);
    }
    (s0, s1)
}

fn wal_with(version: u32, page: u32, frames: &[(u32, u32, Vec<u8>)]) -> Vec<u8> {
    let mut h = Vec::new();
    h.extend_from_slice(&0x377f_0682u32.to_be_bytes());
    h.extend_from_slice(&version.to_be_bytes());
    h.extend_from_slice(&page.to_be_bytes());
    h.extend_from_slice(&0u32.to_be_bytes());
    h.extend_from_slice(&0x1111_1111u32.to_be_bytes());
    h.extend_from_slice(&0x2222_2222u32.to_be_bytes());
    let mut s = cks((0, 0), &h);
    h.extend_from_slice(&s.0.to_be_bytes());
    h.extend_from_slice(&s.1.to_be_bytes());
    for (number, commit, data) in frames {
        assert_eq!(data.len(), page as usize);
        let mut f = Vec::new();
        f.extend_from_slice(&number.to_be_bytes());
        f.extend_from_slice(&commit.to_be_bytes());
        s = cks(s, &f);
        s = cks(s, data);
        f.extend_from_slice(&0x1111_1111u32.to_be_bytes());
        f.extend_from_slice(&0x2222_2222u32.to_be_bytes());
        f.extend_from_slice(&s.0.to_be_bytes());
        f.extend_from_slice(&s.1.to_be_bytes());
        h.extend_from_slice(&f);
        h.extend_from_slice(data);
    }
    h
}

fn wal(page: u32, frames: &[(u32, u32, Vec<u8>)]) -> Vec<u8> {
    wal_with(3_007_000, page, frames)
}

fn seal() -> pctwin_parse_seal::InChild {
    pctwin_parse_seal::in_child()
}

/// F2: a frame naming a page far past the bound is refused before anything is grown.
#[test]
fn a_log_frame_past_the_bound_is_refused_before_anything_grows() {
    let t = temp();
    let p = t.path().join("a.db");
    sqlite(&p, "PRAGMA page_size=4096; CREATE TABLE t(x);");
    let mut db = std::fs::read(&p).unwrap();
    let before = db.len();
    let log = wal(4096, &[(100_000, 1, db[..4096].to_vec())]);
    assert_eq!(
        apply_log(seal(), &mut db, &log),
        Err(ContentError::TooLarge)
    );
    assert_eq!(db.len(), before);
    assert!((db.capacity() as u64) < MAX_DB, "nothing was grown");
    // The largest page number there is, and a commit size past the bound: refused too.
    for (number, commit) in [(u32::MAX, 1), (1, u32::MAX)] {
        let log = wal(4096, &[(number, commit, db[..4096].to_vec())]);
        let mut d = db.clone();
        assert_eq!(apply_log(seal(), &mut d, &log), Err(ContentError::TooLarge));
        assert!((d.capacity() as u64) < MAX_DB);
    }
}

/// F2: a log of another format version is no log; a log beside an empty database is no log, as
/// SQLite opens an empty database as empty.
#[test]
fn a_log_of_another_version_or_beside_an_empty_database_is_no_log() {
    let t = temp();
    let p = t.path().join("a.db");
    sqlite(&p, "PRAGMA page_size=4096; CREATE TABLE t(x);");
    let db = std::fs::read(&p).unwrap();
    let mut page2 = db[..4096].to_vec();
    page2[200] ^= 1;
    let mut d = db.clone();
    apply_log(
        seal(),
        &mut d,
        &wal_with(3_007_001, 4096, &[(1, 1, page2.clone())]),
    )
    .unwrap();
    assert_eq!(d, db, "another version: nothing laid over");
    let mut empty = Vec::new();
    apply_log(seal(), &mut empty, &wal(4096, &[(1, 1, page2)])).unwrap();
    assert!(empty.is_empty(), "an empty database stays empty");
}

/// F2 through the real parser process: a huge page number is "not checked", and the process
/// answers the next request.
#[test]
fn a_huge_page_number_through_the_parser_is_not_checked_and_it_answers_again() {
    let parser = pctwin_parse::testing::parser();
    let t = temp();
    let p = t.path().join("b.db");
    sqlite(&p, "PRAGMA page_size=65536; CREATE TABLE notes(body TEXT);");
    let db = std::fs::read(&p).unwrap();
    let log = wal(65_536, &[(0x8000_0000, 1, vec![0u8; 65_536])]);
    let r = parser.sqlite(&db, Some(&log), Duration::from_secs(5));
    assert_eq!(r, Err(ContentError::TooLarge));
    assert!(parser.sqlite(&db, None, Duration::from_secs(5)).is_ok());
}

// ---------- F1: capture reads databases only through the parser ----------

static OWN: InThisProcess = InThisProcess::new(pctwin_parse_seal::in_child());
static SQLITE_CALLS: AtomicUsize = AtomicUsize::new(0);
static REFUSE: AtomicBool = AtomicBool::new(false);

/// The in-process readers, watched: counts the databases asked about, and can refuse them all.
struct Watching;
impl ContentReader for Watching {
    fn json(&self, b: &[u8]) -> Result<JsonFacts, ContentError> {
        OWN.json(b)
    }
    fn sqlite(&self, db: &[u8], log: Option<&[u8]>, t: Duration) -> Result<DbFacts, ContentError> {
        SQLITE_CALLS.fetch_add(1, Ordering::SeqCst);
        if REFUSE.load(Ordering::SeqCst) {
            return Err(ContentError::Damaged("refused".into()));
        }
        OWN.sqlite(db, log, t)
    }
    fn snapshot(&self, db: &[u8], log: Option<&[u8]>) -> Result<Vec<u8>, ContentError> {
        SQLITE_CALLS.fetch_add(1, Ordering::SeqCst);
        if REFUSE.load(Ordering::SeqCst) {
            return Err(ContentError::Damaged("refused".into()));
        }
        OWN.snapshot(db, log)
    }
    fn add_on_manifest(&self, b: &[u8]) -> Result<bool, ContentError> {
        OWN.add_on_manifest(b)
    }
    fn emlx(&self, b: &[u8]) -> Result<pctwin_special::mail::EmlxFacts, ContentError> {
        OWN.emlx(b)
    }
}
static WATCHING: Watching = Watching;

fn keys() -> &'static RoleKeys {
    static K: OnceLock<RoleKeys> = OnceLock::new();
    K.get_or_init(|| RoleKeys::mint().unwrap())
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

#[test]
fn capture_never_reads_a_database_but_through_the_parser() {
    pctwin_special::content::install_reader(&WATCHING);
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
    let r = found(&home);
    let (f, c) = component(&r, "notes.db");
    let approved = || Approved {
        seal: PlanSeal::new(9, [3; 32]),
        component: c.id,
        root: RootRef::new(4),
        generation: generation(&f.folder.path(), &c.members),
        handle: 1,
    };
    let grant = || {
        keys()
            .capture
            .grant(
                PlanSeal::new(9, [3; 32]),
                vec![RootRef::new(4)],
                CaptureEffect::ReadPrivate,
                Duration::from_secs(60),
                Now::real(),
            )
            .unwrap()
    };
    let run = |w: &str| {
        let w = t.path().join(w);
        std::fs::create_dir(&w).unwrap();
        capture(
            grant(),
            &approved(),
            c,
            &f.folder.path(),
            &w,
            &Fixed(Holding::CannotTell),
            &Stop::new(),
            Now::real(),
        )
    };
    // With the parser answering: captured, and the parser was asked.
    let before = SQLITE_CALLS.load(Ordering::SeqCst);
    assert!(run("w1").is_ok());
    assert!(
        SQLITE_CALLS.load(Ordering::SeqCst) > before,
        "the parser made the copy"
    );
    // With the parser refusing every database: not captured. Nothing in this process read it.
    REFUSE.store(true, Ordering::SeqCst);
    let refused = run("w2");
    assert!(
        matches!(refused, Err(CaptureError::NotConsistent(_))),
        "{refused:?}"
    );
    REFUSE.store(false, Ordering::SeqCst);
    drop(app);
}
