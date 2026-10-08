//! Undo for files (Task List 2.3): what the move wrote and nobody changed goes to the system Trash
//! (never deleted); anything changed since is kept and said so; empty folders the move made are
//! removed; every step is recorded first, so undo cut short carries on safely and never takes a
//! file a person made later under the same name.

use std::cell::RefCell;
use std::path::{Path, PathBuf};

use pctwin_gate::{Approved, Destinations, temp_name};
use pctwin_journal::{Actor, FileId, Journal, Landed, Permission, PlannedWrite, Undo, UndoOutcome};
use pctwin_record::{ItemId, LaptopId};
use pctwin_scan::{Drive, FileSystem};
use pctwin_transfer::{
    Bin, CHANGED_SINCE, block_size_for, fingerprint_reader, recycle_bin_for, undo,
};

/// Stands in for the system Trash: moves files into a folder of its own, or refuses.
struct FakeBin {
    dir: tempfile::TempDir,
    refuse: Option<String>,
    fail_put: RefCell<u32>,
    taken: RefCell<Vec<PathBuf>>,
}

impl FakeBin {
    fn new() -> Self {
        Self {
            dir: tempfile::tempdir().unwrap(),
            refuse: None,
            fail_put: RefCell::new(0),
            taken: RefCell::new(Vec::new()),
        }
    }
}

impl Bin for FakeBin {
    fn can_take(&self, _: &Path) -> Result<(), String> {
        match &self.refuse {
            Some(why) => Err(why.clone()),
            None => Ok(()),
        }
    }
    fn put(&self, path: &Path) -> Result<(), String> {
        if *self.fail_put.borrow() > 0 {
            *self.fail_put.borrow_mut() -= 1;
            return Err("the file is open in another program".into());
        }
        let n = self.taken.borrow().len();
        std::fs::rename(path, self.dir.path().join(format!("{n}"))).map_err(|e| e.to_string())?;
        self.taken.borrow_mut().push(path.to_path_buf());
        Ok(())
    }
}

struct World {
    _dirs: Vec<tempfile::TempDir>,
    root: PathBuf,
    journal: Journal,
    table: Destinations,
}

fn world() -> World {
    let root_dir = tempfile::tempdir().unwrap();
    let jdir = tempfile::tempdir().unwrap();
    let root = root_dir.path().to_path_buf();
    let journal = Journal::open(&jdir.path().join("journal.redb")).unwrap();
    let mut table = Destinations::new();
    table.approve("me", Approved::MyFolders, &root).unwrap();
    World {
        _dirs: vec![root_dir, jdir],
        root,
        journal,
        table,
    }
}

impl World {
    /// A file the move wrote and committed at `stored` (making the folders `made`).
    fn moved(&self, n: u8, stored: &str, bytes: &[u8], made: &[&str]) -> u64 {
        let dest = self.table.get("me").unwrap();
        let place = dest.folder_identity("").unwrap().unwrap();
        let size = bytes.len() as u64;
        let id = self
            .journal
            .plan(&PlannedWrite {
                item: ItemId::from_hex(&format!("{n:02x}{}", "0".repeat(30))).unwrap(),
                source_laptop: LaptopId::from_hex("00112233445566778899aabbccddeeff").unwrap(),
                destination: "me".into(),
                path: stored.into(),
                size,
                actor: Actor {
                    acting_account: "1001".into(),
                    for_account: "1001".into(),
                    permission: Permission::OwnFolders,
                },
                block_size: block_size_for(size),
                source_modified_ns: None,
                place: Some(FileId {
                    volume: place.volume,
                    index: place.index,
                }),
            })
            .unwrap();
        let mut made_ids = Vec::new();
        for f in made {
            std::fs::create_dir_all(self.root.join(f)).unwrap();
            let fid = dest.folder_identity(f).unwrap().unwrap();
            made_ids.push((
                f.to_string(),
                Some(FileId {
                    volume: fid.volume,
                    index: fid.index,
                }),
            ));
        }
        if let Some(parent) = Path::new(stored).parent() {
            std::fs::create_dir_all(self.root.join(parent)).unwrap();
        }
        let folder = stored.rsplit_once('/').map_or("", |(f, _)| f);
        let temp = if folder.is_empty() {
            temp_name(&self.journal.temp_tag(id))
        } else {
            format!("{folder}/{}", temp_name(&self.journal.temp_tag(id)))
        };
        self.journal.staged(id, &temp, &made_ids).unwrap();
        let fp = fingerprint_reader(&mut &bytes[..], size, block_size_for(size))
            .unwrap()
            .unwrap();
        self.journal.verified(id, fp, None).unwrap();
        self.journal.applied(id, stored).unwrap();
        std::fs::write(self.root.join(stored), bytes).unwrap();
        let stat = dest.stat(stored).unwrap().unwrap();
        self.journal
            .committed(
                id,
                Landed {
                    size: stat.len,
                    modified_ns: stat.modified.map(|t| {
                        t.duration_since(std::time::UNIX_EPOCH).unwrap().as_nanos() as i64
                    }),
                    file: Some(FileId {
                        volume: stat.id.volume,
                        index: stat.id.index,
                    }),
                },
            )
            .unwrap();
        id
    }

    fn exists(&self, stored: &str) -> bool {
        self.root.join(stored).exists()
    }

    fn set_modified(&self, stored: &str, t: std::time::SystemTime) {
        let f = std::fs::OpenOptions::new()
            .write(true)
            .open(self.root.join(stored))
            .unwrap();
        f.set_modified(t).unwrap();
    }

    fn modified(&self, stored: &str) -> std::time::SystemTime {
        std::fs::metadata(self.root.join(stored))
            .unwrap()
            .modified()
            .unwrap()
    }
}

fn outcome_of(report: &pctwin_transfer::UndoReport, path: &str) -> UndoOutcome {
    report
        .files
        .iter()
        .chain(&report.folders)
        .find(|u| u.path == path)
        .unwrap_or_else(|| panic!("{path} not in {report:?}"))
        .outcome
        .clone()
}

fn kept_changed() -> UndoOutcome {
    UndoOutcome::Kept {
        why: CHANGED_SINCE.into(),
    }
}

#[test]
fn files_nobody_changed_go_to_the_trash_and_empty_folders_the_move_made_are_removed() {
    let w = world();
    let bin = FakeBin::new();
    std::fs::create_dir(w.root.join("Documents")).unwrap();
    let a = w.moved(
        1,
        "Documents/Tax/2026/a.pdf",
        b"aaaa",
        &["Documents/Tax", "Documents/Tax/2026"],
    );
    let r = undo(&w.journal, &w.table, &bin).unwrap();
    assert_eq!(
        outcome_of(&r, "Documents/Tax/2026/a.pdf"),
        UndoOutcome::Trashed
    );
    assert_eq!(outcome_of(&r, "Documents/Tax/2026"), UndoOutcome::Removed);
    assert_eq!(outcome_of(&r, "Documents/Tax"), UndoOutcome::Removed);
    assert!(!w.exists("Documents/Tax"));
    // A folder that was already there is the person's own: never touched.
    assert!(w.exists("Documents"));
    // Moved, never deleted: the bin has it, exactly as it was.
    assert_eq!(std::fs::read(bin.dir.path().join("0")).unwrap(), b"aaaa");
    assert_eq!(
        w.journal.undo_of(a).unwrap(),
        Some(Undo::Done {
            outcome: UndoOutcome::Trashed
        })
    );
    assert_eq!(r.not_undone().count(), 0);
}

#[test]
fn a_file_changed_since_the_move_is_kept_and_says_so() {
    let w = world();
    let bin = FakeBin::new();
    w.moved(1, "edited.txt", b"draft", &[]);
    w.moved(2, "touched.txt", b"same", &[]);
    w.moved(3, "sneaky.txt", b"1234", &[]);
    // Edited and saved: new contents and size.
    std::fs::write(w.root.join("edited.txt"), b"final version").unwrap();
    // Opened and saved without a change: only the time moved.
    w.set_modified(
        "touched.txt",
        w.modified("touched.txt") + std::time::Duration::from_secs(60),
    );
    // Same size and time, other contents: the fingerprint tells.
    let t = w.modified("sneaky.txt");
    {
        use std::io::Write;
        let mut f = std::fs::OpenOptions::new()
            .write(true)
            .open(w.root.join("sneaky.txt"))
            .unwrap();
        f.write_all(b"4321").unwrap();
    }
    w.set_modified("sneaky.txt", t);
    let r = undo(&w.journal, &w.table, &bin).unwrap();
    for p in ["edited.txt", "touched.txt", "sneaky.txt"] {
        assert_eq!(outcome_of(&r, p), kept_changed(), "{p}");
        assert!(w.exists(p), "{p}");
    }
    assert_eq!(
        std::fs::read(w.root.join("edited.txt")).unwrap(),
        b"final version"
    );
    assert_eq!(r.not_undone().count(), 3);
    assert!(bin.taken.borrow().is_empty());
}

#[test]
fn a_file_made_again_under_the_same_name_is_not_the_one_the_move_wrote() {
    let w = world();
    let bin = FakeBin::new();
    w.moved(1, "a.txt", b"same", &[]);
    let t = w.modified("a.txt");
    // Saved by a program that writes a new file and swaps it in: same contents, size and time.
    std::fs::remove_file(w.root.join("a.txt")).unwrap();
    std::fs::write(w.root.join("a.txt"), b"same").unwrap();
    w.set_modified("a.txt", t);
    let r = undo(&w.journal, &w.table, &bin).unwrap();
    // Only when the drive gives each new file a new identity (all the drives tests run on).
    assert_eq!(outcome_of(&r, "a.txt"), kept_changed());
    assert!(w.exists("a.txt"));
}

#[test]
fn a_file_already_gone_is_reported_and_undo_goes_newest_first() {
    let w = world();
    let bin = FakeBin::new();
    w.moved(1, "first.txt", b"1", &[]);
    w.moved(2, "second.txt", b"2", &[]);
    std::fs::remove_file(w.root.join("first.txt")).unwrap();
    let r = undo(&w.journal, &w.table, &bin).unwrap();
    let order: Vec<&str> = r.files.iter().map(|u| u.path.as_str()).collect();
    assert_eq!(order, ["second.txt", "first.txt"]);
    assert_eq!(outcome_of(&r, "first.txt"), UndoOutcome::AlreadyGone);
    assert_eq!(outcome_of(&r, "second.txt"), UndoOutcome::Trashed);
}

#[test]
fn a_folder_with_something_in_it_is_kept() {
    let w = world();
    let bin = FakeBin::new();
    w.moved(1, "New/a.txt", b"a", &["New"]);
    std::fs::write(w.root.join("New/mine.txt"), b"mine").unwrap();
    let r = undo(&w.journal, &w.table, &bin).unwrap();
    assert_eq!(outcome_of(&r, "New/a.txt"), UndoOutcome::Trashed);
    assert!(matches!(outcome_of(&r, "New"), UndoOutcome::Kept { .. }));
    assert_eq!(std::fs::read(w.root.join("New/mine.txt")).unwrap(), b"mine");
}

#[test]
fn a_folder_made_again_by_the_person_is_never_removed() {
    let w = world();
    let bin = FakeBin::new();
    w.moved(1, "New/a.txt", b"a", &["New"]);
    std::fs::remove_file(w.root.join("New/a.txt")).unwrap();
    std::fs::remove_dir(w.root.join("New")).unwrap();
    std::fs::create_dir(w.root.join("New")).unwrap();
    let r = undo(&w.journal, &w.table, &bin).unwrap();
    assert!(matches!(outcome_of(&r, "New"), UndoOutcome::Kept { .. }));
    assert!(w.exists("New"));
}

#[test]
fn identical_files_that_were_already_there_are_never_touched() {
    let w = world();
    let bin = FakeBin::new();
    let id = w
        .journal
        .plan(&PlannedWrite {
            item: ItemId::from_hex(&format!("09{}", "0".repeat(30))).unwrap(),
            source_laptop: LaptopId::from_hex("00112233445566778899aabbccddeeff").unwrap(),
            destination: "me".into(),
            path: "mine.txt".into(),
            size: 4,
            actor: Actor {
                acting_account: "1001".into(),
                for_account: "1001".into(),
                permission: Permission::OwnFolders,
            },
            block_size: block_size_for(4),
            source_modified_ns: None,
            place: None,
        })
        .unwrap();
    std::fs::write(w.root.join("mine.txt"), b"mine").unwrap();
    w.journal.existing(id, "mine.txt").unwrap();
    let r = undo(&w.journal, &w.table, &bin).unwrap();
    assert!(r.files.is_empty());
    assert!(w.exists("mine.txt"));
}

#[test]
fn where_the_trash_cannot_take_a_file_it_is_kept_with_why() {
    let w = world();
    let mut bin = FakeBin::new();
    bin.refuse = Some("this drive has no Recycle Bin, so it was kept".into());
    w.moved(1, "a.txt", b"a", &[]);
    let r = undo(&w.journal, &w.table, &bin).unwrap();
    assert_eq!(
        outcome_of(&r, "a.txt"),
        UndoOutcome::Kept {
            why: "this drive has no Recycle Bin, so it was kept".into()
        }
    );
    assert!(w.exists("a.txt"));
}

#[test]
fn a_file_the_trash_could_not_take_this_time_is_tried_again_next_time() {
    let w = world();
    let bin = FakeBin::new();
    *bin.fail_put.borrow_mut() = 1;
    w.moved(1, "a.txt", b"a", &[]);
    let r = undo(&w.journal, &w.table, &bin).unwrap();
    assert!(matches!(
        outcome_of(&r, "a.txt"),
        UndoOutcome::NotDone { .. }
    ));
    assert!(w.exists("a.txt"));
    let r = undo(&w.journal, &w.table, &bin).unwrap();
    assert_eq!(outcome_of(&r, "a.txt"), UndoOutcome::Trashed);
    assert!(!w.exists("a.txt"));
}

#[test]
fn undoing_twice_is_as_safe_as_once() {
    let w = world();
    let bin = FakeBin::new();
    w.moved(1, "a.txt", b"a", &["New"]);
    w.moved(2, "b.txt", b"b", &[]);
    std::fs::write(w.root.join("b.txt"), b"changed").unwrap();
    undo(&w.journal, &w.table, &bin).unwrap();
    // The person makes a new a.txt after undo: a second undo never takes it.
    std::fs::write(w.root.join("a.txt"), b"a").unwrap();
    let again = undo(&w.journal, &w.table, &bin).unwrap();
    assert!(again.files.is_empty(), "{again:?}");
    assert!(again.folders.is_empty(), "{again:?}");
    assert!(w.exists("a.txt"));
    assert_eq!(bin.taken.borrow().len(), 1);
}

#[test]
fn undo_cut_short_after_recording_carries_on_without_taking_another_file() {
    let w = world();
    let bin = FakeBin::new();
    let a = w.moved(1, "a.txt", b"a", &[]);
    let b = w.moved(2, "b.txt", b"b", &[]);
    let stat = |p: &str| {
        let s = w.table.get("me").unwrap().stat(p).unwrap().unwrap();
        FileId {
            volume: s.id.volume,
            index: s.id.index,
        }
    };
    // a.txt: recorded as going, then the crash came before it went.
    w.journal
        .record_undo(
            a,
            &Undo::Moving {
                file: Some(stat("a.txt")),
            },
        )
        .unwrap();
    // b.txt: recorded as going, it went, then the crash came; the person made a new b.txt since.
    w.journal
        .record_undo(
            b,
            &Undo::Moving {
                file: Some(stat("b.txt")),
            },
        )
        .unwrap();
    std::fs::rename(w.root.join("b.txt"), bin.dir.path().join("earlier")).unwrap();
    std::fs::write(w.root.join("b.txt"), b"b").unwrap();
    let r = undo(&w.journal, &w.table, &bin).unwrap();
    assert_eq!(outcome_of(&r, "a.txt"), UndoOutcome::Trashed);
    assert!(matches!(outcome_of(&r, "b.txt"), UndoOutcome::Kept { .. }));
    assert!(w.exists("b.txt"));
}

#[test]
fn a_place_not_reachable_now_is_tried_again_later() {
    let mut w = world();
    let bin = FakeBin::new();
    let a = w.moved(1, "a.txt", b"a", &[]);
    let full = std::mem::take(&mut w.table);
    let r = undo(&w.journal, &w.table, &bin).unwrap();
    assert!(matches!(
        outcome_of(&r, "a.txt"),
        UndoOutcome::NotDone { .. }
    ));
    assert_eq!(w.journal.undo_of(a).unwrap(), None);
    w.table = full;
    let r = undo(&w.journal, &w.table, &bin).unwrap();
    assert_eq!(outcome_of(&r, "a.txt"), UndoOutcome::Trashed);
}

fn drive(mount: &str, fs: FileSystem, removable: bool) -> Drive {
    Drive {
        name: "d".into(),
        mount: PathBuf::from(mount),
        file_system: fs,
        removable,
        read_only: false,
        total_bytes: 1,
        free_bytes: 1,
        is_system: false,
        other_system: None,
    }
}

#[test]
fn on_windows_only_a_fixed_ntfs_or_refs_drive_counts_as_having_a_recycle_bin() {
    let drives = [
        drive("C:\\", FileSystem::Ntfs, false),
        drive("D:\\", FileSystem::ReFs, false),
        drive("E:\\", FileSystem::ExFat, true),
        drive("F:\\", FileSystem::Ntfs, true),
        drive("G:\\", FileSystem::Fat32, false),
        // A USB stick mounted inside a fixed drive's folder.
        drive("C:\\mnt\\usb\\", FileSystem::ExFat, true),
    ];
    let ok = |p: &str| recycle_bin_for(&drives, Path::new(p)).is_ok();
    assert!(ok("C:\\Users\\me\\a.txt"));
    assert!(ok("\\\\?\\C:\\Users\\me\\a.txt"));
    assert!(ok("c:\\users\\me\\a.txt"));
    assert!(ok("D:\\a.txt"));
    assert!(!ok("E:\\a.txt"), "a USB stick");
    assert!(!ok("F:\\a.txt"), "a removable NTFS drive");
    assert!(!ok("G:\\a.txt"), "FAT has no Recycle Bin PCTwin trusts");
    assert!(!ok("H:\\a.txt"), "not a known drive (a network drive, say)");
    assert!(
        !ok("C:\\mnt\\usb\\a.txt"),
        "the stick, not the drive it hangs from"
    );
    assert!(!ok("\\\\server\\share\\a.txt"));
}

/// The real system Trash, only on the build machines (never into a person's own Trash when the
/// tests run on their laptop).
#[test]
fn the_system_trash_takes_a_file_and_does_not_delete_it() {
    if std::env::var_os("CI").is_none() {
        eprintln!("skipped: runs on the build machines only");
        return;
    }
    let dir = tempfile::tempdir_in(std::env::current_dir().unwrap()).unwrap();
    let path = dir
        .path()
        .join(format!("pctwin-undo-test-{}.txt", std::process::id()));
    std::fs::write(&path, b"undo me").unwrap();
    let bin = pctwin_transfer::SystemBin::new();
    let real = std::fs::canonicalize(&path).unwrap();
    match bin.can_take(&real) {
        Ok(()) => {
            bin.put(&real).unwrap();
            assert!(!path.exists());
        }
        // A build machine whose drive has no Recycle Bin: the file must stay.
        Err(_) => assert!(path.exists()),
    }
}

/// Checks, as it takes each file, that the journal already says the file is going.
struct RecordFirst<'j> {
    inner: FakeBin,
    journal: &'j Journal,
    entries: Vec<u64>,
}

impl Bin for RecordFirst<'_> {
    fn can_take(&self, path: &Path) -> Result<(), String> {
        self.inner.can_take(path)
    }
    fn put(&self, path: &Path) -> Result<(), String> {
        let going = self
            .entries
            .iter()
            .filter(|id| {
                matches!(
                    self.journal.undo_of(**id).unwrap(),
                    Some(Undo::Moving { .. })
                )
            })
            .count();
        assert_eq!(going, 1, "recorded as going before it goes");
        self.inner.put(path)
    }
}

#[test]
fn each_file_is_recorded_as_going_before_it_goes() {
    let w = world();
    let a = w.moved(1, "a.txt", b"a", &[]);
    let b = w.moved(2, "b.txt", b"b", &[]);
    let bin = RecordFirst {
        inner: FakeBin::new(),
        journal: &w.journal,
        entries: vec![a, b],
    };
    let r = undo(&w.journal, &w.table, &bin).unwrap();
    assert_eq!(r.files.len(), 2);
    assert_eq!(bin.inner.taken.borrow().len(), 2);
}

#[test]
fn a_different_folder_under_the_same_label_is_never_undone() {
    let mut w = world();
    let bin = FakeBin::new();
    w.moved(1, "a.txt", b"a", &[]);
    let other = tempfile::tempdir().unwrap();
    std::fs::copy(w.root.join("a.txt"), other.path().join("a.txt")).unwrap();
    let mut table = Destinations::new();
    table
        .approve("me", Approved::MyFolders, other.path())
        .unwrap();
    let real = std::mem::replace(&mut w.table, table);
    let r = undo(&w.journal, &w.table, &bin).unwrap();
    assert!(matches!(
        outcome_of(&r, "a.txt"),
        UndoOutcome::NotDone { .. }
    ));
    assert!(other.path().join("a.txt").exists());
    w.table = real;
    assert_eq!(
        outcome_of(&undo(&w.journal, &w.table, &bin).unwrap(), "a.txt"),
        UndoOutcome::Trashed
    );
}

#[test]
fn a_file_in_a_folder_spelled_otherwise_on_the_disk_is_still_undone() {
    let w = world();
    let bin = FakeBin::new();
    // The person's own "Docs"; the old laptop sent "docs/a.txt". On drives that ignore capital
    // letters the file lands in "Docs", and is recorded as sent.
    std::fs::create_dir(w.root.join("Docs")).unwrap();
    let case_blind = w.root.join("docs").exists();
    w.moved(1, "docs/a.txt", b"aaaa", &[]);
    let r = undo(&w.journal, &w.table, &bin).unwrap();
    if case_blind {
        assert_eq!(outcome_of(&r, "docs/a.txt"), UndoOutcome::Trashed);
        assert!(!w.root.join("Docs/a.txt").exists());
        assert!(w.root.join("Docs").exists(), "the person's folder stays");
    }
}

#[cfg(windows)]
#[test]
fn a_junction_put_in_place_of_a_folder_is_never_followed_by_undo() {
    let w = world();
    let outside = tempfile::tempdir().unwrap();
    std::fs::write(outside.path().join("victim.txt"), b"outside-original").unwrap();
    w.moved(1, "jn/victim.txt", b"outside-original", &["jn"]);
    std::fs::remove_dir_all(w.root.join("jn")).unwrap();
    let made = std::process::Command::new("cmd")
        .args(["/C", "mklink", "/J"])
        .arg(w.root.join("jn"))
        .arg(outside.path())
        .output()
        .unwrap();
    assert!(made.status.success());
    let bin = FakeBin::new();
    undo(&w.journal, &w.table, &bin).unwrap();
    assert!(outside.path().join("victim.txt").exists());
    assert!(bin.taken.borrow().is_empty());
}
