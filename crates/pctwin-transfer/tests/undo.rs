//! Undo for files (Task List 2.3; Security Design B "Undo removes only the file it checked,
//! through one handle", decided 8 October 2026): until the wipe starts, what the move wrote and
//! nobody changed is removed outright, but only once the old laptop confirms it still has the
//! original unchanged; anything changed since, in use, online only or with a second name is kept
//! and said so; empty folders the move made are removed; every removal is recorded first, so undo
//! cut short carries on safely; once the wipe starts, undo refuses everything. These tests never
//! use the person's real Recycle Bin.
// Tests make and remove files to test on; the no-file-changes fence is for the shipped code.
#![allow(clippy::disallowed_methods)]

mod common;

use std::collections::HashMap;
use std::path::{Path, PathBuf};

use pctwin_gate::{Approved, Destinations, temp_name};
use pctwin_journal::{Actor, FileId, Journal, Landed, Permission, PlannedWrite, Undo, UndoOutcome};
use pctwin_record::{ItemId, LaptopId};
use pctwin_transfer::{
    CHANGED_SINCE, CONNECT_OLD_LAPTOP, Confirmed, KEPT_AT_CLOSE, MOVED_SINCE, ORIGINAL_CHANGED,
    ORIGINAL_NOT_FOUND, ORIGINAL_UNKNOWN, OriginalNow, SECOND_NAME, UndoOptions, UndoReport,
    block_size_for, close_undo, fingerprint_reader, undo, undo_items,
};

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

fn item(n: u8) -> ItemId {
    ItemId::from_hex(&format!("{n:02x}{}", "0".repeat(30))).unwrap()
}

/// The original's identity and time on the old laptop, as recorded when it was read.
fn original(n: u8) -> (FileId, i64) {
    (
        FileId {
            volume: 7,
            index: std::num::NonZeroU64::new(u64::from(n)).unwrap(),
            born: None,
        },
        1_000 + i64::from(n),
    )
}

impl World {
    /// A file the move wrote and committed at `stored` (making the folders `made`).
    fn moved(&self, n: u8, stored: &str, bytes: &[u8], made: &[&str]) -> u64 {
        let dest = self.table.get("me").unwrap();
        let place = dest.folder_identity("").unwrap().unwrap();
        let size = bytes.len() as u64;
        let (source_file, source_ns) = original(n);
        let id = self
            .journal
            .plan(&PlannedWrite {
                item: item(n),
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
                source_modified_ns: Some(source_ns),
                source_file: Some(source_file),
                partial_keep: Default::default(),
                place: Some(place),
            })
            .unwrap();
        let mut made_ids = Vec::new();
        for f in made {
            std::fs::create_dir_all(self.root.join(f)).unwrap();
            let fid = dest.folder_identity(f).unwrap().unwrap();
            made_ids.push((f.to_string(), Some(fid)));
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
        // PCTwin keeps each file and folder it makes open until 10 ms after it was made, so
        // nothing made later shares its number and birth time; this stands in for that.
        std::thread::sleep(std::time::Duration::from_millis(11));
        let stat = dest.stat(stored).unwrap().unwrap();
        self.journal
            .committed(
                id,
                Landed {
                    size: stat.len,
                    modified_ns: stat.modified.map(|t| {
                        t.duration_since(std::time::UNIX_EPOCH).unwrap().as_nanos() as i64
                    }),
                    file: Some(stat.id),
                },
            )
            .unwrap();
        id
    }

    /// The old laptop's answers: every original still there, unchanged.
    fn confirmed(&self) -> HashMap<ItemId, OriginalNow> {
        self.journal
            .entries()
            .unwrap()
            .into_iter()
            .map(|e| {
                (
                    e.write.item,
                    OriginalNow::Present {
                        size: e.write.size,
                        modified_ns: e.write.source_modified_ns,
                        file: e.write.source_file,
                    },
                )
            })
            .collect()
    }

    /// The answers as the old laptop gives them, through the real check.
    fn token(&self, answers: HashMap<ItemId, OriginalNow>) -> Confirmed {
        common::confirmed_blocking(&self.journal, answers.into_iter().collect())
    }

    fn undo(&self) -> UndoReport {
        undo(&self.journal, &self.table, self.token(self.confirmed())).unwrap()
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

    /// The record of a removal about to happen, for a copy at the top of the destination.
    fn removing(&self, entry: u64) -> Undo {
        let file = self.file_of(entry);
        Undo::Removing {
            file,
            dir_id: self
                .table
                .get("me")
                .unwrap()
                .folder_identity("")
                .unwrap()
                .unwrap(),
            private: format!(".pctwin-undo-{:032x}", file.index.get()),
        }
    }

    /// Closes undo, finishing anything part-way with `resolved`.
    fn close(&self, resolved: pctwin_journal::Resolved) {
        self.journal
            .close_undo(&mut |_, _| Ok(resolved.clone()))
            .unwrap();
    }

    /// Undo's own names left anywhere in the destination.
    #[cfg(unix)]
    fn hidden(&self) -> Vec<String> {
        let mut found = Vec::new();
        let mut dirs = vec![self.root.clone()];
        while let Some(d) = dirs.pop() {
            for e in std::fs::read_dir(&d).unwrap() {
                let e = e.unwrap();
                let name = e.file_name().to_string_lossy().into_owned();
                if name.starts_with(".pctwin-undo-") || name.starts_with(".pctwin-salvage-") {
                    found.push(name);
                } else if e.file_type().unwrap().is_dir() {
                    dirs.push(e.path());
                }
            }
        }
        found
    }

    fn file_of(&self, entry: u64) -> FileId {
        match self.journal.entry(entry).unwrap().unwrap().state {
            pctwin_journal::State::Committed { landed, .. } => landed.file.unwrap(),
            other => panic!("{other:?}"),
        }
    }
}

fn outcome_of(report: &UndoReport, path: &str) -> UndoOutcome {
    report
        .files
        .iter()
        .chain(&report.folders)
        .find(|u| u.path == path)
        .unwrap_or_else(|| panic!("{path} not in {report:?}"))
        .outcome
        .clone()
}

fn kept(why: &str) -> UndoOutcome {
    UndoOutcome::Kept { why: why.into() }
}

fn names_in(dir: &Path) -> Vec<String> {
    let mut v: Vec<String> = std::fs::read_dir(dir)
        .unwrap()
        .map(|e| e.unwrap().file_name().to_string_lossy().into_owned())
        .collect();
    v.sort();
    v
}

#[test]
fn files_nobody_changed_are_removed_and_empty_folders_the_move_made_are_removed() {
    let w = world();
    std::fs::create_dir(w.root.join("Documents")).unwrap();
    let a = w.moved(
        1,
        "Documents/Tax/2026/a.pdf",
        b"aaaa",
        &["Documents/Tax", "Documents/Tax/2026"],
    );
    assert_eq!(undo_items(&w.journal).unwrap(), [item(1)]);
    let r = w.undo();
    assert_eq!(
        outcome_of(&r, "Documents/Tax/2026/a.pdf"),
        UndoOutcome::Deleted
    );
    assert_eq!(outcome_of(&r, "Documents/Tax/2026"), UndoOutcome::Removed);
    assert_eq!(outcome_of(&r, "Documents/Tax"), UndoOutcome::Removed);
    assert!(!w.exists("Documents/Tax"));
    // A folder that was already there is the person's own: never touched.
    assert!(w.exists("Documents"));
    assert_eq!(
        w.journal.undo_of(a).unwrap(),
        Some(Undo::Done {
            outcome: UndoOutcome::Deleted
        })
    );
    assert_eq!(r.not_undone().count(), 0);
    // Nothing left to ask the old laptop about.
    assert!(undo_items(&w.journal).unwrap().is_empty());
}

#[test]
fn without_the_old_laptop_nothing_is_removed() {
    let w = world();
    w.moved(1, "a.txt", b"a", &[]);
    w.moved(2, "New/b.txt", b"b", &["New"]);
    let r = undo(&w.journal, &w.table, Confirmed::none(&w.journal)).unwrap();
    for p in ["a.txt", "New/b.txt"] {
        assert_eq!(
            outcome_of(&r, p),
            UndoOutcome::NotDone {
                why: CONNECT_OLD_LAPTOP.into()
            },
            "{p}"
        );
        assert!(w.exists(p));
    }
    // Asked again next time.
    assert_eq!(undo_items(&w.journal).unwrap().len(), 2);
    // Once the old laptop answers, the next undo removes them.
    let r = w.undo();
    assert_eq!(outcome_of(&r, "a.txt"), UndoOutcome::Deleted);
    assert_eq!(outcome_of(&r, "New/b.txt"), UndoOutcome::Deleted);
    assert_eq!(outcome_of(&r, "New"), UndoOutcome::Removed);
}

#[test]
fn an_original_the_old_laptop_could_not_confirm_keeps_its_copy() {
    let w = world();
    w.moved(1, "unanswered.txt", b"1", &[]);
    w.moved(2, "unseen.txt", b"2", &[]);
    w.moved(3, "fine.txt", b"3", &[]);
    let mut answers = w.confirmed();
    answers.remove(&item(1));
    answers.insert(item(2), OriginalNow::CannotLook);
    let r = undo(&w.journal, &w.table, w.token(answers)).unwrap();
    for p in ["unanswered.txt", "unseen.txt"] {
        assert_eq!(
            outcome_of(&r, p),
            UndoOutcome::NotDone {
                why: CONNECT_OLD_LAPTOP.into()
            },
            "{p}"
        );
        assert!(w.exists(p));
    }
    assert_eq!(outcome_of(&r, "fine.txt"), UndoOutcome::Deleted);
}

#[test]
fn a_copy_whose_original_changed_on_the_old_laptop_is_kept() {
    let w = world();
    w.moved(1, "edited.txt", b"1", &[]);
    w.moved(2, "replaced.txt", b"2", &[]);
    w.moved(3, "gone.txt", b"3", &[]);
    let mut answers = w.confirmed();
    let (file1, ns1) = original(1);
    answers.insert(
        item(1),
        OriginalNow::Present {
            size: 1,
            modified_ns: Some(ns1 + 1),
            file: Some(file1),
        },
    );
    let (_, ns2) = original(2);
    answers.insert(
        item(2),
        OriginalNow::Present {
            size: 1,
            modified_ns: Some(ns2),
            file: Some(FileId {
                volume: 7,
                index: std::num::NonZeroU64::new(99).unwrap(),
                born: None,
            }),
        },
    );
    answers.insert(item(3), OriginalNow::Missing);
    let r = undo(&w.journal, &w.table, w.token(answers)).unwrap();
    for p in ["edited.txt", "replaced.txt"] {
        assert_eq!(outcome_of(&r, p), kept(ORIGINAL_CHANGED), "{p}");
        assert!(w.exists(p), "{p}");
    }
    // Not found right now (its drive unplugged, say): kept for now, asked again next time.
    assert_eq!(
        outcome_of(&r, "gone.txt"),
        UndoOutcome::NotDone {
            why: ORIGINAL_NOT_FOUND.into()
        }
    );
    assert!(w.exists("gone.txt"));
}

/// One answer "not found" while the old laptop's drive was unplugged for a moment is not the
/// last word: once it is back, the next undo removes the copy.
#[test]
fn an_original_not_found_for_a_moment_does_not_keep_the_copy_for_good() {
    let w = world();
    w.moved(1, "a.txt", b"a", &[]);
    let mut glitch = w.confirmed();
    glitch.insert(item(1), OriginalNow::Missing);
    let r = undo(&w.journal, &w.table, w.token(glitch)).unwrap();
    assert!(matches!(
        outcome_of(&r, "a.txt"),
        UndoOutcome::NotDone { .. }
    ));
    assert_eq!(undo_items(&w.journal).unwrap(), [item(1)]);
    assert_eq!(outcome_of(&w.undo(), "a.txt"), UndoOutcome::Deleted);
}

/// A copy whose original was never noted (an older record, or read from a snapshot) is kept,
/// and said why in its own words.
#[test]
fn a_copy_whose_original_was_never_noted_is_kept_and_said_so() {
    let w = world();
    let place = w
        .table
        .get("me")
        .unwrap()
        .folder_identity("")
        .unwrap()
        .unwrap();
    let id = w
        .journal
        .plan(&PlannedWrite {
            item: item(6),
            source_laptop: LaptopId::from_hex("00112233445566778899aabbccddeeff").unwrap(),
            destination: "me".into(),
            path: "a.txt".into(),
            size: 1,
            actor: Actor {
                acting_account: "1001".into(),
                for_account: "1001".into(),
                permission: Permission::OwnFolders,
            },
            block_size: block_size_for(1),
            source_modified_ns: Some(1),
            source_file: None,
            partial_keep: Default::default(),
            place: Some(place),
        })
        .unwrap();
    w.journal.staged(id, "x", &[]).unwrap();
    w.journal.verified(id, [0; 32], None).unwrap();
    w.journal.applied(id, "a.txt").unwrap();
    std::fs::write(w.root.join("a.txt"), b"a").unwrap();
    let st = w.table.get("me").unwrap().stat("a.txt").unwrap().unwrap();
    w.journal
        .committed(
            id,
            Landed {
                size: 1,
                modified_ns: None,
                file: Some(st.id),
            },
        )
        .unwrap();
    let r = w.undo();
    assert_eq!(outcome_of(&r, "a.txt"), kept(ORIGINAL_UNKNOWN));
    assert!(w.exists("a.txt"));
}

/// Cut short after the copy was removed, then the old laptop says its original changed: the copy
/// is gone, and the report says so (decided from the disk first), never "kept".
#[test]
fn a_removal_that_landed_before_a_crash_is_reported_gone_whatever_the_old_laptop_says() {
    let w = world();
    let a = w.moved(1, "a.txt", b"a", &[]);
    {
        let permit = w.journal.begin_undo().unwrap();
        w.journal.record_undo(&permit, a, &w.removing(a)).unwrap();
    }
    std::fs::remove_file(w.root.join("a.txt")).unwrap();
    for answer in [OriginalNow::Missing, OriginalNow::CannotLook] {
        let mut answers = w.confirmed();
        answers.insert(item(1), answer);
        let r = undo(&w.journal, &w.table, w.token(answers)).unwrap();
        if let Some(u) = r.files.first() {
            assert_eq!(u.outcome, UndoOutcome::AlreadyGone);
        }
    }
    assert_eq!(
        w.journal.undo_of(a).unwrap(),
        Some(Undo::Done {
            outcome: UndoOutcome::AlreadyGone
        })
    );
}

#[test]
fn a_file_changed_since_the_move_is_kept_and_says_so() {
    let w = world();
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
    // Added to at the end, then given back its time: what landed is still at the start.
    w.moved(4, "grown.txt", b"start", &[]);
    let t = w.modified("grown.txt");
    {
        use std::io::Write;
        let mut f = std::fs::OpenOptions::new()
            .append(true)
            .open(w.root.join("grown.txt"))
            .unwrap();
        f.write_all(b" and more of mine").unwrap();
    }
    w.set_modified("grown.txt", t);
    let r = w.undo();
    for p in ["edited.txt", "touched.txt", "sneaky.txt", "grown.txt"] {
        assert_eq!(outcome_of(&r, p), kept(CHANGED_SINCE), "{p}");
        assert!(w.exists(p), "{p}");
    }
    assert_eq!(
        std::fs::read(w.root.join("edited.txt")).unwrap(),
        b"final version"
    );
    assert_eq!(r.not_undone().count(), 4);
}

#[test]
fn a_file_made_again_under_the_same_name_is_not_the_one_the_move_wrote() {
    let w = world();
    w.moved(1, "a.txt", b"same", &[]);
    let t = w.modified("a.txt");
    // Saved by a program that writes a new file and swaps it in: same contents, size and time.
    std::fs::remove_file(w.root.join("a.txt")).unwrap();
    std::fs::write(w.root.join("a.txt"), b"same").unwrap();
    w.set_modified("a.txt", t);
    let r = w.undo();
    assert_eq!(outcome_of(&r, "a.txt"), kept(CHANGED_SINCE));
    assert!(w.exists("a.txt"));
}

#[test]
fn a_file_already_gone_is_reported_and_undo_goes_newest_first() {
    let w = world();
    w.moved(1, "first.txt", b"1", &[]);
    w.moved(2, "second.txt", b"2", &[]);
    std::fs::remove_file(w.root.join("first.txt")).unwrap();
    let r = w.undo();
    let order: Vec<&str> = r.files.iter().map(|u| u.path.as_str()).collect();
    assert_eq!(order, ["second.txt", "first.txt"]);
    assert_eq!(outcome_of(&r, "first.txt"), kept(MOVED_SINCE));
    assert_eq!(outcome_of(&r, "second.txt"), UndoOutcome::Deleted);
}

#[test]
fn a_file_moved_to_another_folder_is_kept_and_said_so() {
    let w = world();
    w.moved(1, "a.txt", b"a", &[]);
    std::fs::create_dir(w.root.join("Elsewhere")).unwrap();
    std::fs::rename(w.root.join("a.txt"), w.root.join("Elsewhere/a.txt")).unwrap();
    let r = w.undo();
    assert_eq!(outcome_of(&r, "a.txt"), kept(MOVED_SINCE));
    assert!(w.exists("Elsewhere/a.txt"));
}

#[test]
fn a_folder_with_something_in_it_is_kept_and_looked_at_again_next_time() {
    let w = world();
    w.moved(1, "New/a.txt", b"a", &["New"]);
    std::fs::write(w.root.join("New/mine.txt"), b"mine").unwrap();
    let r = w.undo();
    assert_eq!(outcome_of(&r, "New/a.txt"), UndoOutcome::Deleted);
    assert!(matches!(outcome_of(&r, "New"), UndoOutcome::NotDone { .. }));
    assert_eq!(std::fs::read(w.root.join("New/mine.txt")).unwrap(), b"mine");
    // Once it is empty, the next undo removes it.
    std::fs::remove_file(w.root.join("New/mine.txt")).unwrap();
    let r = w.undo();
    assert_eq!(outcome_of(&r, "New"), UndoOutcome::Removed);
}

#[test]
fn a_folder_made_again_by_the_person_is_never_removed() {
    let w = world();
    w.moved(1, "New/a.txt", b"a", &["New"]);
    std::fs::remove_file(w.root.join("New/a.txt")).unwrap();
    std::fs::remove_dir(w.root.join("New")).unwrap();
    std::fs::create_dir(w.root.join("New")).unwrap();
    let r = w.undo();
    assert!(matches!(outcome_of(&r, "New"), UndoOutcome::Kept { .. }));
    assert!(w.exists("New"));
}

#[test]
fn identical_files_that_were_already_there_are_never_touched() {
    let w = world();
    std::fs::write(w.root.join("mine.txt"), b"mine").unwrap();
    let id = w
        .journal
        .plan(&PlannedWrite {
            item: item(9),
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
            source_file: None,
            partial_keep: Default::default(),
            place: None,
        })
        .unwrap();
    w.journal.existing(id, "mine.txt").unwrap();
    let r = w.undo();
    assert!(r.files.is_empty());
    assert!(undo_items(&w.journal).unwrap().is_empty());
    assert_eq!(std::fs::read(w.root.join("mine.txt")).unwrap(), b"mine");
}

#[test]
fn undoing_twice_is_as_safe_as_once() {
    let w = world();
    w.moved(1, "a.txt", b"a", &[]);
    let first = w.undo();
    assert_eq!(outcome_of(&first, "a.txt"), UndoOutcome::Deleted);
    // The person makes a new file under the same name afterwards: never touched.
    std::fs::write(w.root.join("a.txt"), b"a").unwrap();
    let second = w.undo();
    assert!(second.files.is_empty());
    assert!(w.exists("a.txt"));
}

#[test]
fn once_the_wipe_starts_undo_refuses_everything() {
    let w = world();
    w.moved(1, "a.txt", b"a", &[]);
    w.moved(2, "New/b.txt", b"b", &["New"]);
    w.close(pctwin_journal::Resolved::Kept {
        why: "undo was cut short".into(),
    });
    let r = undo(&w.journal, &w.table, w.token(w.confirmed())).unwrap();
    assert!(
        r.closed && r.files.is_empty() && r.folders.is_empty(),
        "{r:?}"
    );
    assert!(w.exists("a.txt") && w.exists("New/b.txt"));
}

/// Closed part of the way (the wipe started while undo ran): it stops, says so, and reports
/// what it did before.
#[cfg(windows)]
#[test]
fn undo_closed_part_of_the_way_stops_and_reports_what_it_did() {
    let w = world();
    w.moved(1, "first.txt", b"1", &[]);
    w.moved(2, "second.txt", b"2", &[]);
    // The newest goes first, and is held by another program for a moment: undo is on it when
    // the wipe starts.
    let holder = std::fs::OpenOptions::new()
        .write(true)
        .open(w.root.join("second.txt"))
        .unwrap();
    let r = std::thread::scope(|s| {
        let undoing = s.spawn(|| w.undo());
        std::thread::sleep(std::time::Duration::from_millis(100));
        w.close(pctwin_journal::Resolved::Kept {
            why: "undo was cut short".into(),
        });
        undoing.join().unwrap()
    });
    drop(holder);
    assert!(r.closed, "{r:?}");
    assert_eq!(r.files.len(), 1, "{r:?}");
    assert_eq!(r.files[0].path, "second.txt");
    assert!(w.exists("first.txt"), "nothing after the close");
}

#[test]
fn an_undo_cut_short_is_not_finished_after_the_wipe_starts() {
    let w = world();
    let a = w.moved(1, "a.txt", b"a", &[]);
    // Cut short just after recording that it was about to remove it.
    {
        let permit = w.journal.begin_undo().unwrap();
        w.journal.record_undo(&permit, a, &w.removing(a)).unwrap();
    }
    // Closing finishes it from the disk: nothing was moved, so the copy stays, said kept.
    close_undo(&w.journal, &w.table, &UndoOptions::default()).unwrap();
    assert_eq!(
        w.journal.undo_of(a).unwrap(),
        Some(Undo::Done {
            outcome: UndoOutcome::Kept {
                why: KEPT_AT_CLOSE.into()
            }
        })
    );
    let r = undo(&w.journal, &w.table, w.token(w.confirmed())).unwrap();
    assert!(r.closed && r.files.is_empty(), "{r:?}");
    assert!(w.exists("a.txt"));
}

#[test]
fn undo_cut_short_before_removing_finishes_next_time() {
    let w = world();
    let a = w.moved(1, "a.txt", b"a", &[]);
    {
        let permit = w.journal.begin_undo().unwrap();
        w.journal.record_undo(&permit, a, &w.removing(a)).unwrap();
    }
    let r = w.undo();
    assert_eq!(outcome_of(&r, "a.txt"), UndoOutcome::Deleted);
    assert!(!w.exists("a.txt"));
}

/// Linux and macOS: PCTwin stopped after moving the copy's name aside. The next undo finds it
/// under exactly the recorded private name, proves it is the copy, unchanged, and removes it.
#[cfg(unix)]
#[test]
fn undo_cut_short_after_moving_aside_is_finished_from_the_disk() {
    let w = world();
    let a = w.moved(1, "a.txt", b"a", &[]);
    let stage = w.removing(a);
    let Undo::Removing { private, .. } = &stage else {
        unreachable!()
    };
    {
        let permit = w.journal.begin_undo().unwrap();
        w.journal.record_undo(&permit, a, &stage).unwrap();
    }
    std::fs::rename(w.root.join("a.txt"), w.root.join(private)).unwrap();
    let r = w.undo();
    assert_eq!(outcome_of(&r, "a.txt"), UndoOutcome::Deleted, "{r:?}");
    assert!(w.hidden().is_empty(), "{:?}", w.hidden());
    assert!(!w.exists("a.txt"));
}

/// Linux and macOS: a copy changed while it was under its private name goes back under its own
/// name, and is said to be kept.
#[cfg(unix)]
#[test]
fn a_copy_changed_while_moved_aside_goes_back_under_its_name() {
    let w = world();
    let a = w.moved(1, "a.txt", b"a", &[]);
    let stage = w.removing(a);
    let Undo::Removing { private, .. } = &stage else {
        unreachable!()
    };
    {
        let permit = w.journal.begin_undo().unwrap();
        w.journal.record_undo(&permit, a, &stage).unwrap();
    }
    std::fs::rename(w.root.join("a.txt"), w.root.join(private)).unwrap();
    std::fs::write(w.root.join(private), b"edited").unwrap();
    let r = w.undo();
    assert_eq!(
        outcome_of(&r, "a.txt"),
        UndoOutcome::Kept {
            why: CHANGED_SINCE.into()
        },
        "{r:?}"
    );
    assert_eq!(std::fs::read(w.root.join("a.txt")).unwrap(), b"edited");
    assert!(w.hidden().is_empty(), "{:?}", w.hidden());
}

/// Linux and macOS: closing undo (the wipe starts) finishes a removal cut short after the move
/// aside, so nothing is left under a hidden name once undo is gone.
#[cfg(unix)]
#[test]
fn closing_finishes_a_removal_cut_short_after_moving_aside() {
    let w = world();
    let a = w.moved(1, "a.txt", b"a", &[]);
    let stage = w.removing(a);
    let Undo::Removing { private, .. } = &stage else {
        unreachable!()
    };
    {
        let permit = w.journal.begin_undo().unwrap();
        w.journal.record_undo(&permit, a, &stage).unwrap();
    }
    std::fs::rename(w.root.join("a.txt"), w.root.join(private)).unwrap();
    close_undo(&w.journal, &w.table, &UndoOptions::default()).unwrap();
    assert_eq!(
        w.journal.undo_of(a).unwrap(),
        Some(Undo::Done {
            outcome: UndoOutcome::Deleted
        })
    );
    assert!(w.hidden().is_empty(), "{:?}", w.hidden());
    assert!(!w.exists("a.txt"));
}

/// Linux and macOS: a salvage PCTwin stopped while copying is never said to be "saved again": it
/// is named visibly and said to be possibly incomplete; one the journal recorded complete is
/// "saved again" (Part J, finding 1).
#[cfg(unix)]
#[test]
fn a_salvage_cut_short_is_said_to_be_maybe_incomplete_never_saved_again() {
    use std::os::unix::fs::PermissionsExt;
    for (complete, mode) in [(false, 0o644), (false, 0o000), (true, 0o644)] {
        let w = world();
        let a = w.moved(1, "a.txt", b"a", &[]);
        let stage = w.removing(a);
        let Undo::Removing { file, dir_id, .. } = stage.clone() else {
            unreachable!()
        };
        let temp = format!(".pctwin-salvage-{}", "1".repeat(32));
        let to = "a (kept by PCTwin undo).txt";
        {
            let permit = w.journal.begin_undo().unwrap();
            w.journal.record_undo(&permit, a, &stage).unwrap();
            w.journal
                .record_undo(
                    &permit,
                    a,
                    &Undo::Salvaging {
                        file,
                        dir_id,
                        temp: temp.clone(),
                        to: to.into(),
                        complete,
                    },
                )
                .unwrap();
        }
        // The copy was removed, then PCTwin stopped while saving what was written to it.
        std::fs::remove_file(w.root.join("a.txt")).unwrap();
        std::fs::write(w.root.join(&temp), b"part of the late byt").unwrap();
        std::fs::set_permissions(w.root.join(&temp), std::fs::Permissions::from_mode(mode))
            .unwrap();
        let r = w.undo();
        let why = if complete {
            pctwin_transfer::SAVED_AGAIN
        } else {
            pctwin_transfer::SAVED_MAYBE_INCOMPLETE
        };
        assert_eq!(
            outcome_of(&r, "a.txt"),
            UndoOutcome::KeptAt {
                at: to.into(),
                why: why.into()
            },
            "complete {complete}, mode {mode:o}: {r:?}"
        );
        assert_eq!(
            std::fs::read(w.root.join(to)).unwrap(),
            b"part of the late byt"
        );
        assert!(w.hidden().is_empty(), "{:?}", w.hidden());
    }
}

/// Linux: a copy put back under its name because another program had it open is tried again
/// later and removed once it is free; it is never called "changed" for having been put back
/// (Part J, finding 2).
#[cfg(target_os = "linux")]
#[test]
fn a_copy_put_back_because_it_was_in_use_is_removed_once_free() {
    let w = world();
    let a = w.moved(1, "a.txt", b"a", &[]);
    let stage = w.removing(a);
    let Undo::Removing { private, .. } = &stage else {
        unreachable!()
    };
    {
        let permit = w.journal.begin_undo().unwrap();
        w.journal.record_undo(&permit, a, &stage).unwrap();
    }
    // PCTwin stopped after moving it aside; a program has it open.
    std::fs::rename(w.root.join("a.txt"), w.root.join(private)).unwrap();
    let holder = std::fs::File::open(w.root.join(private)).unwrap();
    let r = w.undo();
    assert_eq!(
        outcome_of(&r, "a.txt"),
        UndoOutcome::NotDone {
            why: pctwin_transfer::IN_USE.into()
        },
        "{r:?}"
    );
    assert!(w.exists("a.txt"));
    assert!(w.hidden().is_empty(), "{:?}", w.hidden());
    drop(holder);
    let r = w.undo();
    assert_eq!(outcome_of(&r, "a.txt"), UndoOutcome::Deleted, "{r:?}");
    assert!(!w.exists("a.txt"));
}

/// Linux and macOS: a put-back cut short (recorded, not yet done) of the copy itself ends with
/// the copy under its name and looked at again as usual, so it is removed (Part J, finding 2).
#[cfg(unix)]
#[test]
fn a_put_back_of_the_copy_cut_short_is_finished_and_the_copy_removed() {
    let w = world();
    let a = w.moved(1, "a.txt", b"a", &[]);
    let stage = w.removing(a);
    let Undo::Removing {
        file,
        dir_id,
        private,
    } = stage.clone()
    else {
        unreachable!()
    };
    {
        let permit = w.journal.begin_undo().unwrap();
        w.journal.record_undo(&permit, a, &stage).unwrap();
        w.journal
            .record_undo(
                &permit,
                a,
                &Undo::Putting {
                    file,
                    dir_id,
                    private: private.clone(),
                    to: "a.txt".into(),
                },
            )
            .unwrap();
    }
    std::fs::rename(w.root.join("a.txt"), w.root.join(&private)).unwrap();
    let r = w.undo();
    assert_eq!(outcome_of(&r, "a.txt"), UndoOutcome::Deleted, "{r:?}");
    assert!(w.hidden().is_empty(), "{:?}", w.hidden());
}

/// Every undo on Linux and macOS leaves no name of its own behind.
#[cfg(unix)]
#[test]
fn undo_leaves_no_hidden_names() {
    let w = world();
    w.moved(1, "a.txt", b"a", &[]);
    w.moved(2, "New/b.txt", b"b", &["New"]);
    let r = w.undo();
    assert_eq!(outcome_of(&r, "a.txt"), UndoOutcome::Deleted, "{r:?}");
    assert!(w.hidden().is_empty(), "{:?}", w.hidden());
}

#[test]
fn undo_cut_short_after_removing_says_it_is_gone_and_claims_nothing_more() {
    let w = world();
    let a = w.moved(1, "a.txt", b"a", &[]);
    {
        let permit = w.journal.begin_undo().unwrap();
        w.journal.record_undo(&permit, a, &w.removing(a)).unwrap();
    }
    std::fs::remove_file(w.root.join("a.txt")).unwrap();
    let r = w.undo();
    assert_eq!(outcome_of(&r, "a.txt"), UndoOutcome::AlreadyGone);
}

#[test]
fn a_file_is_recorded_as_being_removed_only_when_it_really_is() {
    let w = world();
    let a = w.moved(1, "a.txt", b"a", &[]);
    // Kept open by another program for the whole undo: never attempted, so nothing recorded.
    #[cfg(windows)]
    {
        let _holder = std::fs::OpenOptions::new()
            .write(true)
            .open(w.root.join("a.txt"))
            .unwrap();
        let r = w.undo();
        assert_eq!(
            outcome_of(&r, "a.txt"),
            UndoOutcome::NotDone {
                why: pctwin_transfer::IN_USE.into()
            }
        );
        assert_eq!(w.journal.undo_of(a).unwrap(), None);
        assert!(w.exists("a.txt"));
    }
    // Once nobody holds it, the next undo removes it.
    let r = w.undo();
    assert_eq!(outcome_of(&r, "a.txt"), UndoOutcome::Deleted);
    assert_eq!(
        w.journal.undo_of(a).unwrap(),
        Some(Undo::Done {
            outcome: UndoOutcome::Deleted
        })
    );
}

#[test]
fn a_file_with_a_second_name_is_kept_and_said_so() {
    let w = world();
    w.moved(1, "a.txt", b"a", &[]);
    std::fs::hard_link(w.root.join("a.txt"), w.root.join("also-a.txt")).unwrap();
    let r = w.undo();
    assert_eq!(outcome_of(&r, "a.txt"), kept(SECOND_NAME));
    assert!(w.exists("a.txt") && w.exists("also-a.txt"));
}

#[test]
fn a_place_not_reachable_now_is_tried_again_later() {
    let mut w = world();
    w.moved(1, "a.txt", b"a", &[]);
    let real = std::mem::replace(&mut w.table, Destinations::new());
    let r = w.undo();
    assert!(matches!(
        outcome_of(&r, "a.txt"),
        UndoOutcome::NotDone { .. }
    ));
    w.table = real;
    assert_eq!(outcome_of(&w.undo(), "a.txt"), UndoOutcome::Deleted);
}

#[test]
fn a_different_folder_under_the_same_label_is_never_undone() {
    let mut w = world();
    w.moved(1, "a.txt", b"a", &[]);
    let other = tempfile::tempdir().unwrap();
    std::fs::copy(w.root.join("a.txt"), other.path().join("a.txt")).unwrap();
    let mut table = Destinations::new();
    table
        .approve("me", Approved::MyFolders, other.path())
        .unwrap();
    let real = std::mem::replace(&mut w.table, table);
    let r = w.undo();
    assert!(matches!(
        outcome_of(&r, "a.txt"),
        UndoOutcome::NotDone { .. }
    ));
    assert!(other.path().join("a.txt").exists());
    w.table = real;
    assert_eq!(outcome_of(&w.undo(), "a.txt"), UndoOutcome::Deleted);
}

#[test]
fn a_file_in_a_folder_spelled_otherwise_on_the_disk_is_still_undone() {
    let w = world();
    std::fs::create_dir(w.root.join("Docs")).unwrap();
    let case_blind = w.root.join("docs").exists();
    w.moved(1, "docs/a.txt", b"aaaa", &[]);
    let r = w.undo();
    if case_blind {
        assert_eq!(outcome_of(&r, "docs/a.txt"), UndoOutcome::Deleted);
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
    w.undo();
    assert!(outside.path().join("victim.txt").exists());
}

#[test]
fn a_folder_whose_identity_was_never_known_is_never_removed() {
    let w = world();
    w.moved(1, "X/a.txt", b"a", &[]);
    // Recorded as made, but without an identity (it could not be read when made).
    std::fs::create_dir(w.root.join("Y")).unwrap();
    let e = w
        .journal
        .plan(&PlannedWrite {
            item: item(14),
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
            source_file: None,
            partial_keep: Default::default(),
            place: None,
        })
        .unwrap();
    w.journal
        .staged(e, "Y/.pctwin-x.part", &[("Y".to_string(), None)])
        .unwrap();
    let r = w.undo();
    assert!(matches!(outcome_of(&r, "Y"), UndoOutcome::Kept { .. }));
    assert!(w.root.join("Y").is_dir());
}

#[test]
fn a_file_whose_identity_was_never_known_is_kept_and_said_so() {
    let w = world();
    let place = w
        .table
        .get("me")
        .unwrap()
        .folder_identity("")
        .unwrap()
        .unwrap();
    let id = w
        .journal
        .plan(&PlannedWrite {
            item: item(5),
            source_laptop: LaptopId::from_hex("00112233445566778899aabbccddeeff").unwrap(),
            destination: "me".into(),
            path: "a.txt".into(),
            size: 1,
            actor: Actor {
                acting_account: "1001".into(),
                for_account: "1001".into(),
                permission: Permission::OwnFolders,
            },
            block_size: block_size_for(1),
            source_modified_ns: Some(1),
            source_file: Some(FileId {
                volume: 1,
                index: std::num::NonZeroU64::new(1).unwrap(),
                born: None,
            }),
            partial_keep: Default::default(),
            place: Some(place),
        })
        .unwrap();
    w.journal.staged(id, "x", &[]).unwrap();
    w.journal.verified(id, [0; 32], None).unwrap();
    w.journal.applied(id, "a.txt").unwrap();
    std::fs::write(w.root.join("a.txt"), b"a").unwrap();
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
    let r = w.undo();
    assert!(matches!(outcome_of(&r, "a.txt"), UndoOutcome::Kept { .. }));
    assert!(w.exists("a.txt"));
}

/// The person saves a new version over the file (writing another and putting it over the name, as
/// editors do) while undo runs: the new version is never lost, whatever undo reports.
#[test]
fn a_new_version_saved_during_undo_is_never_lost() {
    use std::sync::Arc;
    use std::sync::atomic::{AtomicBool, Ordering};
    let trials = if std::env::var_os("CI").is_some() {
        200
    } else {
        40
    };
    for n in 0..trials {
        let w = world();
        w.moved(1, "a.txt", b"copy", &[]);
        std::fs::write(w.root.join("new.tmp"), format!("edit {n}")).unwrap();
        let go = Arc::new(AtomicBool::new(false));
        let r = std::thread::scope(|s| {
            let go2 = Arc::clone(&go);
            let root = w.root.clone();
            let saver = s.spawn(move || {
                while !go2.load(Ordering::Acquire) {
                    std::hint::spin_loop();
                }
                for _ in 0..100_000 {
                    if std::fs::rename(root.join("new.tmp"), root.join("a.txt")).is_ok() {
                        return;
                    }
                    std::thread::yield_now();
                }
                panic!("the save never landed");
            });
            go.store(true, Ordering::Release);
            let r = w.undo();
            saver.join().unwrap();
            r
        });
        assert_eq!(
            std::fs::read_to_string(w.root.join("a.txt")).unwrap(),
            format!("edit {n}"),
            "trial {n}: {r:?}"
        );
        assert_eq!(names_in(&w.root), ["a.txt"], "trial {n}: {r:?}");
    }
}

#[cfg(windows)]
#[test]
fn a_file_stored_online_only_is_kept_and_said_so() {
    let w = world();
    w.moved(1, "a.txt", b"a", &[]);
    let ok = std::process::Command::new("attrib")
        .arg("+O")
        .arg(w.root.join("a.txt"))
        .status()
        .is_ok_and(|s| s.success());
    assert!(ok);
    let r = w.undo();
    assert_eq!(outcome_of(&r, "a.txt"), kept(pctwin_transfer::ONLINE_ONLY));
    assert!(w.exists("a.txt"));
}
