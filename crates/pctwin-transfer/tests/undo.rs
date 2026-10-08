//! Undo for files (Task List 2.3): what the move wrote and nobody changed goes to the system Trash
//! (never deleted); anything changed since is kept and said so; empty folders the move made are
//! removed; every step is recorded first, so undo cut short carries on safely and never takes a
//! file a person made later under the same name.

use std::cell::RefCell;
use std::path::{Path, PathBuf};

use pctwin_gate::{Approved, Destination, Destinations, temp_name};
use pctwin_journal::{Actor, FileId, Journal, Landed, Permission, PlannedWrite, Undo, UndoOutcome};
use pctwin_record::{ItemId, LaptopId};
use pctwin_scan::{Drive, FileSystem};
/// PCTwin's own folder for files on their way to the Trash (the app gives it in the person's
/// language).
const ASIDE: &str = "Undone by PCTwin";

use pctwin_transfer::{
    Bin, CHANGED_SINCE, block_size_for, fingerprint_reader, recycle_bin_for, undo,
};

/// Stands in for the system Trash: moves files into a folder of its own, or refuses.
struct FakeBin {
    dir: tempfile::TempDir,
    refuse: Option<String>,
    fail_put: RefCell<u32>,
    taken: RefCell<Vec<PathBuf>>,
    apart: bool,
}

impl FakeBin {
    fn new() -> Self {
        Self {
            dir: tempfile::tempdir().unwrap(),
            refuse: None,
            fail_put: RefCell::new(0),
            taken: RefCell::new(Vec::new()),
            apart: true,
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
    fn holds(&self, path: &Path) -> Option<bool> {
        let want = std::fs::canonicalize(path.parent()?)
            .ok()?
            .join(path.file_name()?);
        Some(self.taken.borrow().iter().any(|p| {
            p.parent()
                .and_then(|d| std::fs::canonicalize(d).ok())
                .zip(p.file_name())
                .is_some_and(|(d, n)| d.join(n) == want)
        }))
    }
    fn tells_files_apart(&self, _: &Path) -> bool {
        self.apart
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
    let r = undo(&w.journal, &w.table, &bin, ASIDE).unwrap();
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
    let r = undo(&w.journal, &w.table, &bin, ASIDE).unwrap();
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
    let r = undo(&w.journal, &w.table, &bin, ASIDE).unwrap();
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
    let r = undo(&w.journal, &w.table, &bin, ASIDE).unwrap();
    let order: Vec<&str> = r.files.iter().map(|u| u.path.as_str()).collect();
    assert_eq!(order, ["second.txt", "first.txt"]);
    assert_eq!(
        outcome_of(&r, "first.txt"),
        UndoOutcome::Kept {
            why: pctwin_transfer::MOVED_SINCE.into()
        }
    );
    assert_eq!(outcome_of(&r, "second.txt"), UndoOutcome::Trashed);
}

#[test]
fn a_folder_with_something_in_it_is_kept_and_looked_at_again_next_time() {
    let w = world();
    let bin = FakeBin::new();
    w.moved(1, "New/a.txt", b"a", &["New"]);
    std::fs::write(w.root.join("New/mine.txt"), b"mine").unwrap();
    let r = undo(&w.journal, &w.table, &bin, ASIDE).unwrap();
    assert_eq!(outcome_of(&r, "New/a.txt"), UndoOutcome::Trashed);
    assert!(matches!(outcome_of(&r, "New"), UndoOutcome::NotDone { .. }));
    assert_eq!(std::fs::read(w.root.join("New/mine.txt")).unwrap(), b"mine");
    // Once it is empty, the next undo removes it.
    std::fs::remove_file(w.root.join("New/mine.txt")).unwrap();
    let r = undo(&w.journal, &w.table, &bin, ASIDE).unwrap();
    assert_eq!(outcome_of(&r, "New"), UndoOutcome::Removed);
}

#[test]
fn a_folder_made_again_by_the_person_is_never_removed() {
    let w = world();
    let bin = FakeBin::new();
    w.moved(1, "New/a.txt", b"a", &["New"]);
    std::fs::remove_file(w.root.join("New/a.txt")).unwrap();
    std::fs::remove_dir(w.root.join("New")).unwrap();
    std::fs::create_dir(w.root.join("New")).unwrap();
    let r = undo(&w.journal, &w.table, &bin, ASIDE).unwrap();
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
    let r = undo(&w.journal, &w.table, &bin, ASIDE).unwrap();
    assert!(r.files.is_empty());
    assert!(w.exists("mine.txt"));
}

#[test]
fn where_the_trash_cannot_take_a_file_it_is_kept_with_why() {
    let w = world();
    let mut bin = FakeBin::new();
    bin.refuse = Some("this drive has no Recycle Bin, so it was kept".into());
    w.moved(1, "a.txt", b"a", &[]);
    let r = undo(&w.journal, &w.table, &bin, ASIDE).unwrap();
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
    let r = undo(&w.journal, &w.table, &bin, ASIDE).unwrap();
    assert!(matches!(
        outcome_of(&r, "a.txt"),
        UndoOutcome::NotDone { .. }
    ));
    assert!(w.exists("a.txt"));
    let r = undo(&w.journal, &w.table, &bin, ASIDE).unwrap();
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
    undo(&w.journal, &w.table, &bin, ASIDE).unwrap();
    // The person makes a new a.txt after undo: a second undo never takes it.
    std::fs::write(w.root.join("a.txt"), b"a").unwrap();
    let again = undo(&w.journal, &w.table, &bin, ASIDE).unwrap();
    assert!(again.files.is_empty(), "{again:?}");
    assert!(again.folders.is_empty(), "{again:?}");
    assert!(w.exists("a.txt"));
    assert_eq!(bin.taken.borrow().len(), 1);
}

#[test]
fn undo_cut_short_carries_on_from_where_it_was_without_taking_another_file() {
    let w = world();
    let bin = FakeBin::new();
    let a = w.moved(1, "a.txt", b"a", &[]);
    let b = w.moved(2, "b.txt", b"b", &[]);
    let c = w.moved(3, "c.txt", b"c", &[]);
    let id = |p: &str| {
        let s = w.table.get("me").unwrap().stat(p).unwrap().unwrap();
        FileId {
            volume: s.id.volume,
            index: s.id.index,
        }
    };
    let aside = |name: &str| format!("{ASIDE}/{name}");
    // a.txt: recorded as going aside, then the crash came before it moved.
    w.journal
        .record_undo(
            a,
            &Undo::Aside {
                file: Some(id("a.txt")),
                at: aside("a.txt"),
                staging: None,
                made: Vec::new(),
            },
        )
        .unwrap();
    // b.txt: recorded, moved aside, then the crash; the person made a new b.txt since.
    w.journal
        .record_undo(
            b,
            &Undo::Aside {
                file: Some(id("b.txt")),
                at: aside("b.txt"),
                staging: None,
                // Moved aside into PCTwin's folder, made for it.
                made: vec![ASIDE.to_string()],
            },
        )
        .unwrap();
    std::fs::create_dir(w.root.join(ASIDE)).unwrap();
    std::fs::rename(w.root.join("b.txt"), w.root.join(aside("b.txt"))).unwrap();
    std::fs::write(w.root.join("b.txt"), b"b").unwrap();
    // c.txt: recorded, moved aside and handed to the Trash, then the crash; a new c.txt since.
    w.journal
        .record_undo(
            c,
            &Undo::Aside {
                file: Some(id("c.txt")),
                at: aside("c.txt"),
                staging: None,
                made: Vec::new(),
            },
        )
        .unwrap();
    std::fs::rename(w.root.join("c.txt"), bin.dir.path().join("earlier")).unwrap();
    std::fs::write(w.root.join("c.txt"), b"c").unwrap();
    let r = undo(&w.journal, &w.table, &bin, ASIDE).unwrap();
    assert_eq!(outcome_of(&r, "a.txt"), UndoOutcome::Trashed);
    assert_eq!(outcome_of(&r, "b.txt"), UndoOutcome::Trashed);
    assert_eq!(outcome_of(&r, "c.txt"), kept_changed());
    // The person's new files are never taken; PCTwin's folder is gone once empty.
    assert_eq!(std::fs::read(w.root.join("b.txt")).unwrap(), b"b");
    assert_eq!(std::fs::read(w.root.join("c.txt")).unwrap(), b"c");
    assert!(!w.root.join(ASIDE).exists());
    assert_eq!(bin.taken.borrow().len(), 2);
}

#[test]
fn an_edit_made_while_a_file_is_aside_puts_it_back_where_it_was() {
    let w = world();
    let bin = FakeBin::new();
    let a = w.moved(1, "Docs/a.txt", b"draft", &[]);
    let s = w
        .table
        .get("me")
        .unwrap()
        .stat("Docs/a.txt")
        .unwrap()
        .unwrap();
    let at = format!("{ASIDE}/Docs/a.txt");
    w.journal
        .record_undo(
            a,
            &Undo::Aside {
                file: Some(FileId {
                    volume: s.id.volume,
                    index: s.id.index,
                }),
                at: at.clone(),
                staging: None,
                made: vec![ASIDE.to_string(), format!("{ASIDE}/Docs")],
            },
        )
        .unwrap();
    std::fs::create_dir_all(w.root.join(format!("{ASIDE}/Docs"))).unwrap();
    std::fs::rename(w.root.join("Docs/a.txt"), w.root.join(&at)).unwrap();
    // A program that had it open writes to it while it is aside.
    {
        use std::io::Write;
        let mut f = std::fs::OpenOptions::new()
            .append(true)
            .open(w.root.join(&at))
            .unwrap();
        f.write_all(b" and more").unwrap();
    }
    let r = undo(&w.journal, &w.table, &bin, ASIDE).unwrap();
    assert_eq!(outcome_of(&r, "Docs/a.txt"), kept_changed());
    assert_eq!(
        std::fs::read(w.root.join("Docs/a.txt")).unwrap(),
        b"draft and more"
    );
    assert!(!w.root.join(ASIDE).exists());
    assert!(bin.taken.borrow().is_empty());
}

#[test]
fn a_place_not_reachable_now_is_tried_again_later() {
    let mut w = world();
    let bin = FakeBin::new();
    let a = w.moved(1, "a.txt", b"a", &[]);
    let full = std::mem::take(&mut w.table);
    let r = undo(&w.journal, &w.table, &bin, ASIDE).unwrap();
    assert!(matches!(
        outcome_of(&r, "a.txt"),
        UndoOutcome::NotDone { .. }
    ));
    assert_eq!(w.journal.undo_of(a).unwrap(), None);
    w.table = full;
    let r = undo(&w.journal, &w.table, &bin, ASIDE).unwrap();
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
    if cfg!(any(windows, target_os = "macos")) {
        assert!(ok("c:\\users\\me\\a.txt"));
    }
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
                    Some(Undo::Aside { .. })
                )
            })
            .count();
        assert_eq!(going, 1, "recorded as going before it goes");
        // Only ever from PCTwin's own folder.
        assert!(
            path.components()
                .any(|c| c.as_os_str() == std::ffi::OsStr::new(ASIDE)),
            "{path:?}"
        );
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
    let r = undo(&w.journal, &w.table, &bin, ASIDE).unwrap();
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
    let r = undo(&w.journal, &w.table, &bin, ASIDE).unwrap();
    assert!(matches!(
        outcome_of(&r, "a.txt"),
        UndoOutcome::NotDone { .. }
    ));
    assert!(other.path().join("a.txt").exists());
    w.table = real;
    assert_eq!(
        outcome_of(&undo(&w.journal, &w.table, &bin, ASIDE).unwrap(), "a.txt"),
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
    let r = undo(&w.journal, &w.table, &bin, ASIDE).unwrap();
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
    undo(&w.journal, &w.table, &bin, ASIDE).unwrap();
    assert!(outside.path().join("victim.txt").exists());
    assert!(bin.taken.borrow().is_empty());
}

/// Lets the person save an edit into the file between undo checking it and moving it aside.
struct EditingBin {
    inner: FakeBin,
}

impl Bin for EditingBin {
    fn can_take(&self, path: &Path) -> Result<(), String> {
        use std::io::Write;
        let mut f = std::fs::OpenOptions::new().append(true).open(path).unwrap();
        f.write_all(b" + my new edit").unwrap();
        Ok(())
    }
    fn put(&self, path: &Path) -> Result<(), String> {
        self.inner.put(path)
    }
}

#[test]
fn an_edit_saved_after_the_check_is_never_trashed() {
    let w = world();
    w.moved(1, "a.txt", b"original", &[]);
    let bin = EditingBin {
        inner: FakeBin::new(),
    };
    let r = undo(&w.journal, &w.table, &bin, ASIDE).unwrap();
    assert_eq!(outcome_of(&r, "a.txt"), kept_changed());
    assert_eq!(
        std::fs::read(w.root.join("a.txt")).unwrap(),
        b"original + my new edit"
    );
    assert!(bin.inner.taken.borrow().is_empty());
    assert!(!w.root.join(ASIDE).exists());
}

/// Swaps the file for the person's new work between undo checking it and moving it.
struct SwapBin {
    inner: FakeBin,
}

impl Bin for SwapBin {
    fn can_take(&self, path: &Path) -> Result<(), String> {
        std::fs::rename(path, path.with_extension("orig-elsewhere")).unwrap();
        std::fs::write(path, b"the person's new work").unwrap();
        Ok(())
    }
    fn put(&self, path: &Path) -> Result<(), String> {
        self.inner.put(path)
    }
}

#[test]
fn a_file_swapped_in_after_the_check_is_never_trashed() {
    let w = world();
    w.moved(1, "f.txt", b"ours", &[]);
    let bin = SwapBin {
        inner: FakeBin::new(),
    };
    let r = undo(&w.journal, &w.table, &bin, ASIDE).unwrap();
    assert_eq!(outcome_of(&r, "f.txt"), kept_changed());
    assert_eq!(
        std::fs::read(w.root.join("f.txt")).unwrap(),
        b"the person's new work"
    );
    assert!(bin.inner.taken.borrow().is_empty());
}

#[test]
fn a_file_or_folder_moved_or_renamed_since_is_kept_and_said_so() {
    let w = world();
    let bin = FakeBin::new();
    w.moved(1, "A/b.txt", b"bbbb", &["A"]);
    w.moved(2, "C/d.txt", b"dddd", &["C"]);
    w.moved(3, "E/f.txt", b"ffff", &["E"]);
    // A renamed, d.txt renamed inside C, f.txt deleted.
    std::fs::rename(w.root.join("A"), w.root.join("A-renamed")).unwrap();
    std::fs::rename(w.root.join("C/d.txt"), w.root.join("C/d2.txt")).unwrap();
    std::fs::remove_file(w.root.join("E/f.txt")).unwrap();
    // Another file of the same size there is not it.
    std::fs::write(w.root.join("E/g.txt"), b"gggg").unwrap();
    let r = undo(&w.journal, &w.table, &bin, ASIDE).unwrap();
    let moved = UndoOutcome::Kept {
        why: pctwin_transfer::MOVED_SINCE.into(),
    };
    assert_eq!(outcome_of(&r, "A/b.txt"), moved);
    assert_eq!(outcome_of(&r, "C/d.txt"), moved);
    assert_eq!(outcome_of(&r, "E/f.txt"), moved);
    assert!(w.exists("A-renamed/b.txt"));
    assert!(w.exists("C/d2.txt"));
    assert_eq!(
        r.not_undone().count(),
        3 + 2,
        "the files A/b.txt, C/d.txt and E/f.txt, and folders C and E (not empty)"
    );
}

#[test]
fn a_folder_whose_identity_was_never_known_is_never_removed() {
    let w = world();
    let bin = FakeBin::new();
    // Recorded without an identity (it could not be read when made).
    let id = w.moved(1, "X/a.txt", b"a", &[]);
    let _ = id;
    std::fs::create_dir(w.root.join("Y")).unwrap();
    let e = w.journal.plan(&pctwin_journal::PlannedWrite {
        item: ItemId::from_hex(&format!("0e{}", "0".repeat(30))).unwrap(),
        source_laptop: LaptopId::from_hex("00112233445566778899aabbccddeeff").unwrap(),
        destination: "me".into(),
        path: "Y/z.txt".into(),
        size: 1,
        actor: Actor {
            acting_account: "1001".into(),
            for_account: "1001".into(),
            permission: Permission::OwnFolders,
        },
        block_size: block_size_for(1),
        source_modified_ns: None,
        place: None,
    });
    w.journal
        .staged(e.unwrap(), "Y/.pctwin-x.part", &[("Y".to_string(), None)])
        .unwrap();
    let r = undo(&w.journal, &w.table, &bin, ASIDE).unwrap();
    assert!(matches!(outcome_of(&r, "Y"), UndoOutcome::Kept { .. }));
    assert!(w.root.join("Y").is_dir());
}

#[test]
fn a_file_whose_identity_was_never_known_is_kept_and_said_so() {
    let w = world();
    let bin = FakeBin::new();
    let id = w
        .journal
        .plan(&pctwin_journal::PlannedWrite {
            item: ItemId::from_hex(&format!("0d{}", "0".repeat(30))).unwrap(),
            source_laptop: LaptopId::from_hex("00112233445566778899aabbccddeeff").unwrap(),
            destination: "me".into(),
            path: "n.txt".into(),
            size: 1,
            actor: Actor {
                acting_account: "1001".into(),
                for_account: "1001".into(),
                permission: Permission::OwnFolders,
            },
            block_size: block_size_for(1),
            source_modified_ns: None,
            place: None,
        })
        .unwrap();
    w.journal.staged(id, ".pctwin-n.part", &[]).unwrap();
    let fp = fingerprint_reader(&mut &b"n"[..], 1, block_size_for(1))
        .unwrap()
        .unwrap();
    w.journal.verified(id, fp, None).unwrap();
    w.journal.applied(id, "n.txt").unwrap();
    std::fs::write(w.root.join("n.txt"), b"n").unwrap();
    w.journal
        .committed(
            id,
            Landed {
                size: 1,
                modified_ns: None,
                file: None,
            },
        )
        .unwrap();
    let r = undo(&w.journal, &w.table, &bin, ASIDE).unwrap();
    let UndoOutcome::Kept { why } = outcome_of(&r, "n.txt") else {
        panic!("not kept")
    };
    assert!(why.contains("cannot tell"), "{why}");
    assert!(w.exists("n.txt"));
}

#[test]
fn a_drive_is_matched_by_whole_folder_names_not_by_the_start_of_a_name() {
    let drives = [
        drive("C:\\", FileSystem::Ntfs, false),
        drive("C:\\mnt\\usb\\", FileSystem::ExFat, true),
    ];
    let ok = |p: &str| recycle_bin_for(&drives, Path::new(p)).is_ok();
    // "usb2" is a folder on C:, not on the stick mounted at "usb".
    assert!(ok("C:\\mnt\\usb2\\a.txt"));
    assert!(!ok("C:\\mnt\\usb\\a.txt"));
    if cfg!(any(windows, target_os = "macos")) {
        assert!(!ok("C:\\mnt\\USB\\a.txt"));
    }
}

#[test]
fn a_recycle_bin_set_to_delete_or_too_small_never_gets_the_file() {
    use pctwin_transfer::{BIN_TURNED_OFF, BinSettings, TOO_BIG_FOR_BIN, bin_keeps};
    let normal = BinSettings::default();
    assert!(bin_keeps(&normal, 1 << 40).is_ok());
    let off = BinSettings {
        turned_off: true,
        ..BinSettings::default()
    };
    assert_eq!(bin_keeps(&off, 1), Err(BIN_TURNED_OFF.to_string()));
    let nuke = BinSettings {
        deletes_at_once: true,
        ..BinSettings::default()
    };
    assert_eq!(bin_keeps(&nuke, 1), Err(BIN_TURNED_OFF.to_string()));
    let small = BinSettings {
        max_bytes: Some(1000),
        ..BinSettings::default()
    };
    assert!(bin_keeps(&small, 1000).is_ok());
    assert_eq!(bin_keeps(&small, 1001), Err(TOO_BIG_FOR_BIN.to_string()));
    // Settings that cannot be read for certain (two drives share a number): kept.
    let unknown = BinSettings {
        unknown: true,
        ..BinSettings::default()
    };
    assert_eq!(
        bin_keeps(&unknown, 1),
        Err(pctwin_transfer::BIN_UNKNOWN.to_string())
    );
}

#[test]
fn a_drive_without_its_own_settings_has_the_recycle_bin_windows_gives_by_default() {
    use pctwin_transfer::default_bin_bytes;
    const GB: u64 = 1024 * 1024 * 1024;
    // A tenth of the first 40 GB, a twentieth of the rest.
    assert_eq!(default_bin_bytes(10 * GB), GB);
    assert_eq!(default_bin_bytes(40 * GB), 4 * GB);
    assert_eq!(default_bin_bytes(240 * GB), 4 * GB + 10 * GB);
    assert_eq!(default_bin_bytes(0), 0);
}

/// Reads this laptop's real Recycle Bin settings (nothing is put in the Recycle Bin).
#[cfg(windows)]
#[test]
fn the_real_recycle_bin_settings_are_read_and_answered_in_plain_words() {
    use pctwin_transfer::{BIN_TURNED_OFF, NO_RECYCLE_BIN, SystemBin, TOO_BIG_FOR_BIN};
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("probe.txt");
    std::fs::write(&path, b"x").unwrap();
    let real = std::fs::canonicalize(&path).unwrap();
    match SystemBin::new().can_take(&real) {
        Ok(()) => {}
        Err(why) => assert!(
            [NO_RECYCLE_BIN, BIN_TURNED_OFF, TOO_BIG_FOR_BIN].contains(&why.as_str()),
            "{why}"
        ),
    }
    assert!(path.exists());
}

#[test]
fn a_lookalike_put_where_a_file_was_going_aside_is_never_trashed() {
    let w = world();
    let bin = FakeBin::new();
    let e = w.moved(1, "Documents/a.pdf", b"aaaa", &[]);
    let ours = w
        .table
        .get("me")
        .unwrap()
        .stat("Documents/a.pdf")
        .unwrap()
        .unwrap()
        .id;
    // Cut short after "about to move aside to <at>" was recorded...
    let at = format!("{ASIDE}/Documents/a.pdf");
    w.journal
        .record_undo(
            e,
            &Undo::Aside {
                file: Some(FileId {
                    volume: ours.volume,
                    index: ours.index,
                }),
                at: at.clone(),
                staging: None,
                made: Vec::new(),
            },
        )
        .unwrap();
    // ...then someone puts their own identical-looking file at that name.
    std::fs::create_dir_all(w.root.join(ASIDE).join("Documents")).unwrap();
    std::fs::write(w.root.join(&at), b"aaaa").unwrap();
    let mtime = w.modified("Documents/a.pdf");
    w.set_modified(&at, mtime);
    let r = undo(&w.journal, &w.table, &bin, ASIDE).unwrap();
    // The move's own file is undone; the lookalike is never touched.
    assert_eq!(outcome_of(&r, "Documents/a.pdf"), UndoOutcome::Trashed);
    assert_eq!(std::fs::read(w.root.join(&at)).unwrap(), b"aaaa");
    assert!(!w.exists("Documents/a.pdf"));
    assert_eq!(bin.taken.borrow().len(), 1);
    assert_ne!(bin.taken.borrow()[0], w.root.join(&at));
}

/// Swaps in the person's own byte-identical copy (same modified time) at every moment of undo.
#[test]
fn an_identical_copy_swapped_in_during_undo_is_never_trashed() {
    use std::sync::Arc;
    use std::sync::atomic::{AtomicBool, Ordering};
    for i in 0..80u64 {
        let w = world();
        let bin = FakeBin::new();
        w.moved(1, "Documents/a.pdf", b"aaaa", &[]);
        let f = w.root.join("Documents/a.pdf");
        let ours = w
            .table
            .get("me")
            .unwrap()
            .stat("Documents/a.pdf")
            .unwrap()
            .unwrap()
            .id;
        let mtime = std::fs::metadata(&f).unwrap().modified().unwrap();
        let p = w.root.join("Documents/person-copy.tmp");
        std::fs::write(&p, b"aaaa").unwrap();
        std::fs::OpenOptions::new()
            .write(true)
            .open(&p)
            .unwrap()
            .set_modified(mtime)
            .unwrap();
        let theirs = w
            .table
            .get("me")
            .unwrap()
            .stat("Documents/person-copy.tmp")
            .unwrap()
            .unwrap()
            .id;
        let go = Arc::new(AtomicBool::new(false));
        let go2 = go.clone();
        let (p2, f2) = (p.clone(), f.clone());
        let t = std::thread::spawn(move || {
            while !go2.load(Ordering::Acquire) {}
            let t0 = std::time::Instant::now();
            while t0.elapsed() < std::time::Duration::from_micros(i * 50) {
                std::hint::spin_loop();
            }
            let _ = std::fs::rename(&p2, &f2);
        });
        go.store(true, Ordering::Release);
        let r = undo(&w.journal, &w.table, &bin, ASIDE).unwrap();
        t.join().unwrap();
        if matches!(r.files[0].outcome, UndoOutcome::Trashed) {
            let dest = Destination::open(bin.dir.path()).unwrap();
            let trashed = dest.stat("0").unwrap().unwrap().id;
            assert_eq!(
                trashed, ours,
                "iteration {i}: the person's own copy was trashed"
            );
            assert_ne!(trashed, theirs);
        }
    }
}

#[cfg(windows)]
#[test]
fn a_junction_or_file_in_the_way_of_pctwin_s_folder_never_blocks_or_misleads_undo() {
    let w = world();
    let bin = FakeBin::new();
    let outside = tempfile::tempdir().unwrap();
    w.moved(1, "Documents/a.pdf", b"aaaa", &[]);
    let made = std::process::Command::new("cmd")
        .args(["/C", "mklink", "/J"])
        .arg(w.root.join(ASIDE))
        .arg(outside.path())
        .output()
        .unwrap();
    assert!(made.status.success());
    let r = undo(&w.journal, &w.table, &bin, ASIDE).unwrap();
    assert_eq!(outcome_of(&r, "Documents/a.pdf"), UndoOutcome::Trashed);
    assert_eq!(std::fs::read_dir(outside.path()).unwrap().count(), 0);
    // Its numbered folder, made for the move, is gone again.
    assert!(!w.root.join(format!("{ASIDE} (2)")).exists());
}

#[test]
fn a_file_put_where_pctwin_s_folder_would_be_never_blocks_undo() {
    let w = world();
    let bin = FakeBin::new();
    w.moved(1, "a.txt", b"a", &[]);
    std::fs::write(w.root.join(ASIDE), b"someone's file").unwrap();
    let r = undo(&w.journal, &w.table, &bin, ASIDE).unwrap();
    assert_eq!(outcome_of(&r, "a.txt"), UndoOutcome::Trashed);
    assert_eq!(
        std::fs::read(w.root.join(ASIDE)).unwrap(),
        b"someone's file"
    );
}

#[test]
fn a_person_s_own_empty_folder_inside_pctwin_s_folder_is_never_tidied_away() {
    let w = world();
    let bin = FakeBin::new();
    w.moved(1, "Docs/a.txt", b"a", &[]);
    std::fs::create_dir_all(w.root.join(format!("{ASIDE}/Docs"))).unwrap();
    std::fs::create_dir_all(w.root.join(format!("{ASIDE}/Mine"))).unwrap();
    let r = undo(&w.journal, &w.table, &bin, ASIDE).unwrap();
    assert_eq!(outcome_of(&r, "Docs/a.txt"), UndoOutcome::Trashed);
    // Folders that were there before are not PCTwin's to remove.
    assert!(w.root.join(format!("{ASIDE}/Docs")).is_dir());
    assert!(w.root.join(format!("{ASIDE}/Mine")).is_dir());
}

#[test]
fn a_file_moved_to_another_folder_is_kept_and_said_so() {
    let w = world();
    let bin = FakeBin::new();
    w.moved(1, "Documents/Tax/a.pdf", b"aaaa", &["Documents/Tax"]);
    std::fs::create_dir_all(w.root.join("Archive")).unwrap();
    std::fs::rename(
        w.root.join("Documents/Tax/a.pdf"),
        w.root.join("Archive/a.pdf"),
    )
    .unwrap();
    let r = undo(&w.journal, &w.table, &bin, ASIDE).unwrap();
    assert_eq!(
        outcome_of(&r, "Documents/Tax/a.pdf"),
        UndoOutcome::Kept {
            why: pctwin_transfer::MOVED_SINCE.into()
        }
    );
    assert!(w.exists("Archive/a.pdf"));
}

#[test]
fn undo_cut_short_after_the_trash_took_a_file_says_trashed_only_if_the_trash_has_it() {
    let w = world();
    let bin = FakeBin::new();
    let a = w.moved(1, "a.txt", b"a", &[]);
    let b = w.moved(2, "b.txt", b"b", &[]);
    let id_of = |p: &str| {
        let s = w.table.get("me").unwrap().stat(p).unwrap().unwrap();
        FileId {
            volume: s.id.volume,
            index: s.id.index,
        }
    };
    for (entry, name) in [(a, "a.txt"), (b, "b.txt")] {
        let at = format!("{ASIDE}/{name}");
        w.journal
            .record_undo(
                entry,
                &Undo::Aside {
                    file: Some(id_of(name)),
                    at: at.clone(),
                    staging: None,
                    made: Vec::new(),
                },
            )
            .unwrap();
        std::fs::create_dir_all(w.root.join(ASIDE)).unwrap();
        std::fs::rename(w.root.join(name), w.root.join(&at)).unwrap();
    }
    // a.txt: the Trash took it, then the crash. b.txt: the person moved it out of PCTwin's
    // folder themselves.
    bin.put(&w.root.join(format!("{ASIDE}/a.txt"))).unwrap();
    std::fs::rename(
        w.root.join(format!("{ASIDE}/b.txt")),
        w.root.join("b-mine.txt"),
    )
    .unwrap();
    let r = undo(&w.journal, &w.table, &bin, ASIDE).unwrap();
    assert_eq!(outcome_of(&r, "a.txt"), UndoOutcome::Trashed);
    assert_eq!(
        outcome_of(&r, "b.txt"),
        UndoOutcome::Kept {
            why: pctwin_transfer::MOVED_SINCE.into()
        }
    );
    assert!(w.exists("b-mine.txt"));
}

#[test]
fn on_a_drive_that_cannot_tell_files_apart_nothing_is_undone_by_guess() {
    let w = world();
    let mut bin = FakeBin::new();
    bin.apart = false;
    w.moved(1, "a.txt", b"a", &[]);
    let r = undo(&w.journal, &w.table, &bin, ASIDE).unwrap();
    assert!(
        matches!(outcome_of(&r, "a.txt"), UndoOutcome::Kept { why } if why.contains("cannot tell"))
    );
    assert!(w.exists("a.txt"));
}

#[test]
fn only_fat_and_exfat_drives_count_as_not_telling_files_apart() {
    let drives = [
        drive("/", FileSystem::Ext4, false),
        drive("/media/stick", FileSystem::Fat32, true),
        drive("/media/card", FileSystem::ExFat, true),
    ];
    assert!(pctwin_transfer::tells_files_apart(
        &drives,
        Path::new("/home/a.txt")
    ));
    assert!(!pctwin_transfer::tells_files_apart(
        &drives,
        Path::new("/media/stick/a.txt")
    ));
    assert!(!pctwin_transfer::tells_files_apart(
        &drives,
        Path::new("/media/card/a.txt")
    ));
    assert!(pctwin_transfer::tells_files_apart(
        &drives,
        Path::new("/media/cards/a.txt")
    ));
}

#[cfg(windows)]
#[test]
fn a_file_left_in_its_staging_folder_by_a_crash_is_picked_up_again() {
    let w = world();
    let bin = FakeBin::new();
    let e = w.moved(1, "Docs/a.txt", b"aaaa", &[]);
    let s = w
        .table
        .get("me")
        .unwrap()
        .stat("Docs/a.txt")
        .unwrap()
        .unwrap();
    let staging = format!("Docs/{}", pctwin_gate::staging_name(&w.journal.temp_tag(e)));
    w.journal
        .record_undo(
            e,
            &Undo::Aside {
                file: Some(FileId {
                    volume: s.id.volume,
                    index: s.id.index,
                }),
                at: format!("{ASIDE}/Docs/a.txt"),
                staging: Some(staging.clone()),
                made: Vec::new(),
            },
        )
        .unwrap();
    // The crash came between moving it into its staging folder and naming it aside.
    std::fs::create_dir(w.root.join(&staging)).unwrap();
    std::fs::rename(
        w.root.join("Docs/a.txt"),
        w.root.join(format!("{staging}/a.txt")),
    )
    .unwrap();
    let r = undo(&w.journal, &w.table, &bin, ASIDE).unwrap();
    assert_eq!(outcome_of(&r, "Docs/a.txt"), UndoOutcome::Trashed);
    assert_eq!(names_in(&w.root.join("Docs")), Vec::<String>::new());
    assert_eq!(bin.taken.borrow().len(), 1);
}

fn names_in(dir: &Path) -> Vec<String> {
    let mut v: Vec<String> = std::fs::read_dir(dir)
        .unwrap()
        .map(|e| e.unwrap().file_name().to_string_lossy().into_owned())
        .collect();
    v.sort();
    v
}

/// The app stops (here: a panic) at the very moment the Trash is asked to take the file.
struct StopsAtPut;

impl Bin for StopsAtPut {
    fn can_take(&self, _: &Path) -> Result<(), String> {
        Ok(())
    }
    fn put(&self, _: &Path) -> Result<(), String> {
        panic!("the app stopped");
    }
}

#[test]
fn undo_cut_short_at_the_trash_tidies_the_folders_it_made_on_the_next_run() {
    let w = world();
    w.moved(1, "Docs/Tax/a.txt", b"a", &[]);
    let stopped = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
        undo(&w.journal, &w.table, &StopsAtPut, ASIDE)
    }));
    assert!(stopped.is_err());
    // Left aside, in folders made for it.
    assert!(w.root.join(format!("{ASIDE}/Docs/Tax/a.txt")).is_file());
    let bin = FakeBin::new();
    let r = undo(&w.journal, &w.table, &bin, ASIDE).unwrap();
    assert_eq!(outcome_of(&r, "Docs/Tax/a.txt"), UndoOutcome::Trashed);
    assert!(!w.root.join(ASIDE).exists());
}
