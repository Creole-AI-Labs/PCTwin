//! Files live while their app is open (Task List 2.5, decided 29 October 2026): found by kind, the
//! person asked to close only the app holding one, and a file still held never read.

use std::path::{Path, PathBuf};
use std::sync::Arc;
use std::sync::atomic::{AtomicUsize, Ordering};

use pctwin_fullmove::live::{
    HeldFile, LiveFile, LiveOpener, apps_to_close, find_live, held_now, until_closed,
};
use pctwin_record::ItemId;
use pctwin_special::capture::{Holders, Holding};
use pctwin_special::live::LiveKinds;
use pctwin_transfer::{FsOpener, InUse, Opener};

fn sqlite_bytes() -> Vec<u8> {
    let mut h = b"SQLite format 3\0".to_vec();
    h.resize(100, 0);
    h[16] = 0x10;
    h[18] = 1;
    h[19] = 1;
    h[21] = 64;
    h[22] = 32;
    h[23] = 32;
    h.resize(4096, 0);
    h
}

fn pst_bytes() -> Vec<u8> {
    let mut h = b"!BDN\0\0\0\0SM".to_vec();
    h.resize(2048, 0);
    h
}

fn id(n: u8) -> ItemId {
    ItemId::from_hex(&format!("{n:02x}").repeat(16)).unwrap()
}

/// Holds the files it is told, by the apps it is told, for a number of looks.
struct Fake {
    by: Vec<String>,
    looks_held: usize,
    looks: AtomicUsize,
    answer_when_free: Holding,
}

impl Fake {
    fn held_for(looks_held: usize, by: &[&str]) -> Self {
        Self {
            by: by.iter().map(|s| s.to_string()).collect(),
            looks_held,
            looks: AtomicUsize::new(0),
            answer_when_free: Holding::Free,
        }
    }
}

impl Holders for Fake {
    fn holding(&self, _files: &[PathBuf]) -> Holding {
        let n = self.looks.fetch_add(1, Ordering::SeqCst);
        if n < self.looks_held {
            Holding::Held {
                by: self.by.clone(),
            }
        } else {
            self.answer_when_free.clone()
        }
    }
}

fn live(dir: &Path, name: &str, n: u8) -> LiveFile {
    LiveFile {
        item: id(n),
        path: dir.join(name),
        name: name.into(),
        kind: "database".into(),
        sidecars: Vec::new(),
    }
}

#[test]
fn live_files_are_found_by_kind_never_by_name_alone() {
    let dir = tempfile::tempdir().unwrap();
    let d = dir.path();
    std::fs::write(d.join("money.db"), sqlite_bytes()).unwrap();
    std::fs::write(d.join("Outlook.pst"), pst_bytes()).unwrap();
    std::fs::write(d.join("Thumbs.db"), b"\xd0\xcf\x11\xe0\xa1\xb1\x1a\xe1rest").unwrap();
    std::fs::write(d.join("copy.sqlite.txt"), sqlite_bytes()).unwrap();
    std::fs::write(d.join("letter.docx"), b"PK\x03\x04").unwrap();
    let files = [
        "money.db",
        "Outlook.pst",
        "Thumbs.db",
        "copy.sqlite.txt",
        "letter.docx",
        "gone.db",
    ]
    .iter()
    .enumerate()
    .map(|(n, f)| (id(n as u8), d.join(f)));
    let found = find_live(files, LiveKinds::built_in());
    let names: Vec<(&str, &str)> = found
        .iter()
        .map(|f| (f.name.as_str(), f.kind.as_str()))
        .collect();
    assert_eq!(
        names,
        vec![("money.db", "database"), ("Outlook.pst", "mail-store")]
    );
    assert_eq!(found[0].item, id(0));
    assert_eq!(found[1].item, id(1));
    assert_eq!(found[1].path, d.join("Outlook.pst"));
}

#[test]
fn the_person_is_asked_until_the_app_is_closed_and_only_about_held_files() {
    let dir = tempfile::tempdir().unwrap();
    let files = [live(dir.path(), "money.db", 1)];
    let holders = Fake::held_for(2, &["Money Manager"]);
    let mut asked: Vec<Vec<HeldFile>> = Vec::new();
    let left = until_closed(&files, &holders, &mut |held| {
        asked.push(held.to_vec());
        true
    });
    assert!(left.is_empty(), "closed after two tries");
    assert_eq!(asked.len(), 2);
    assert_eq!(apps_to_close(&asked[0]), vec!["Money Manager".to_string()]);
    assert_eq!(asked[0][0].file, files[0]);
}

#[test]
fn going_on_without_them_leaves_the_held_files_with_their_apps() {
    let dir = tempfile::tempdir().unwrap();
    let files = [live(dir.path(), "a.db", 1), live(dir.path(), "b.pst", 2)];
    let holders = Fake::held_for(usize::MAX, &["Outlook", "Outlook"]);
    let mut times = 0;
    let left = until_closed(&files, &holders, &mut |_| {
        times += 1;
        false
    });
    assert_eq!(times, 1, "asked once, then gone on");
    assert_eq!(left.len(), 2);
    assert_eq!(apps_to_close(&left), vec!["Outlook".to_string()]);
}

#[test]
fn nothing_is_asked_when_nothing_holds_them_or_the_system_cannot_tell() {
    let dir = tempfile::tempdir().unwrap();
    let files = [live(dir.path(), "a.db", 1)];
    let free = Fake::held_for(0, &[]);
    assert!(held_now(&files, &free).is_empty());
    let cannot = Fake {
        answer_when_free: Holding::CannotTell,
        ..Fake::held_for(0, &[])
    };
    let left = until_closed(&files, &cannot, &mut |_| panic!("never asked"));
    assert!(left.is_empty());
}

#[test]
fn a_file_that_is_not_live_is_read_as_before() {
    let dir = tempfile::tempdir().unwrap();
    let plain = dir.path().join("plain.txt");
    std::fs::write(&plain, b"hello").unwrap();
    let opener = LiveOpener::new(
        Arc::new(FsOpener),
        &[live(dir.path(), "money.db", 1)],
        Arc::new(Fake::held_for(usize::MAX, &["Money Manager"])),
    );
    let mut s = opener.open(&plain).unwrap();
    let mut text = String::new();
    std::io::Read::read_to_string(&mut s, &mut text).unwrap();
    assert_eq!(text, "hello");
    assert!(opener.held().is_empty());
}

fn is_in_use(e: &std::io::Error) -> bool {
    e.get_ref().is_some_and(|inner| inner.is::<InUse>())
}

#[cfg(windows)]
#[test]
fn a_live_file_open_for_writing_is_never_read_and_one_being_read_cannot_be_written() {
    let dir = tempfile::tempdir().unwrap();
    let f = live(dir.path(), "money.db", 1);
    std::fs::write(&f.path, sqlite_bytes()).unwrap();
    let opener = LiveOpener::new(
        Arc::new(FsOpener),
        std::slice::from_ref(&f),
        Arc::new(Fake::held_for(usize::MAX, &["Money Manager"])),
    );
    // Its app has it open for writing (sharing as an app does).
    let app = std::fs::OpenOptions::new()
        .read(true)
        .write(true)
        .open(&f.path)
        .unwrap();
    let e = opener.open(&f.path).err().expect("not read while held");
    assert!(is_in_use(&e), "{e}");
    assert_eq!(
        opener.held().get(&f.item),
        Some(&vec!["Money Manager".to_string()])
    );
    drop(app);
    // Closed: it is read, and while it is read no program can open it for writing.
    let reading = opener.open(&f.path).unwrap();
    assert!(
        std::fs::OpenOptions::new()
            .write(true)
            .open(&f.path)
            .is_err(),
        "writers are kept out while it is read"
    );
    drop(reading);
    assert!(
        std::fs::OpenOptions::new()
            .write(true)
            .open(&f.path)
            .is_ok()
    );
}

#[cfg(not(windows))]
#[test]
fn a_live_file_held_by_its_app_is_never_read() {
    let dir = tempfile::tempdir().unwrap();
    let f = live(dir.path(), "money.db", 1);
    std::fs::write(&f.path, sqlite_bytes()).unwrap();
    let opener = LiveOpener::new(
        Arc::new(FsOpener),
        std::slice::from_ref(&f),
        Arc::new(Fake::held_for(1, &["money-manager"])),
    );
    let e = opener.open(&f.path).err().expect("not read while held");
    assert!(is_in_use(&e), "{e}");
    assert_eq!(
        opener.held().get(&f.item),
        Some(&vec!["money-manager".to_string()])
    );
    // Closed since: read.
    assert!(opener.open(&f.path).is_ok());
}

/// A database's own files beside it (its write-ahead log and the like) are live with it: found,
/// asked about, and held while it is read, so the copy is whole.
#[test]
fn a_databases_own_files_beside_it_are_live_with_it() {
    let dir = tempfile::tempdir().unwrap();
    let d = dir.path();
    std::fs::write(d.join("money.db"), sqlite_bytes()).unwrap();
    std::fs::write(d.join("money.db-wal"), b"log").unwrap();
    std::fs::write(d.join("money.db-shm"), b"index").unwrap();
    std::fs::write(d.join("other.db-wal"), b"no database of its own").unwrap();
    let files = ["money.db", "money.db-wal", "money.db-shm", "other.db-wal"]
        .iter()
        .enumerate()
        .map(|(n, f)| (id(n as u8), d.join(f)));
    let found = find_live(files, LiveKinds::built_in());
    let mut names: Vec<&str> = found.iter().map(|f| f.name.as_str()).collect();
    names.sort();
    assert_eq!(names, ["money.db", "money.db-shm", "money.db-wal"]);
    let db = found.iter().find(|f| f.name == "money.db").unwrap();
    assert_eq!(
        db.sidecars,
        [d.join("money.db-wal"), d.join("money.db-shm")]
    );
}

#[cfg(windows)]
#[test]
fn a_database_whose_log_is_held_is_not_read_and_its_log_is_held_while_it_is() {
    let dir = tempfile::tempdir().unwrap();
    let d = dir.path();
    std::fs::write(d.join("money.db"), sqlite_bytes()).unwrap();
    std::fs::write(d.join("money.db-wal"), b"log").unwrap();
    let files = [(id(1), d.join("money.db")), (id(2), d.join("money.db-wal"))];
    let found = find_live(files, LiveKinds::built_in());
    let opener = LiveOpener::new(
        Arc::new(FsOpener),
        &found,
        Arc::new(Fake::held_for(usize::MAX, &["Money Manager"])),
    );
    // The app writes its log: the database is not read.
    let app = std::fs::OpenOptions::new()
        .read(true)
        .write(true)
        .open(d.join("money.db-wal"))
        .unwrap();
    let e = opener.open(&d.join("money.db")).err().expect("not read");
    assert!(is_in_use(&e), "{e}");
    drop(app);
    // Closed: read, and its log cannot be written meanwhile.
    let reading = opener.open(&d.join("money.db")).unwrap();
    assert!(
        std::fs::OpenOptions::new()
            .write(true)
            .open(d.join("money.db-wal"))
            .is_err()
    );
    drop(reading);
    assert!(
        std::fs::OpenOptions::new()
            .write(true)
            .open(d.join("money.db-wal"))
            .is_ok()
    );
}
