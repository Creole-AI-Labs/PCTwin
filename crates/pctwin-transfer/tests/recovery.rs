//! Recovery after a restart (Task List 1.6), on a real disk and journal. Each test builds exactly
//! what a crash at one step leaves behind (the journal at that step, the files as they were),
//! then checks recovery proves before it commits, never replaces or removes anything that is not
//! its own, and is as safe run twice as once.

use std::path::Path;

use pctwin_gate::{Approved, Destinations, temp_name};
use pctwin_journal::{Actor, Journal, PartialKeep, Permission, PlannedWrite, State};
use pctwin_record::{ItemId, LaptopId};
use pctwin_transfer::{
    KeepPartials, block_size_for, expire_partials, file_fingerprint, fingerprint_reader, partials,
    recover, recover_with,
};

struct World {
    _dirs: Vec<tempfile::TempDir>,
    root: std::path::PathBuf,
    journal: Journal,
    table: Destinations,
}

fn world() -> World {
    let root_dir = tempfile::tempdir().unwrap();
    let jdir = tempfile::tempdir().unwrap();
    let root = root_dir.path().to_path_buf();
    std::fs::create_dir(root.join("Docs")).unwrap();
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

fn data(n: usize) -> Vec<u8> {
    (0..n).map(|i| (i * 7 % 251) as u8).collect()
}

fn fp(bytes: &[u8]) -> [u8; 32] {
    fingerprint_reader(
        &mut &bytes[..],
        bytes.len() as u64,
        block_size_for(bytes.len() as u64),
    )
    .unwrap()
    .unwrap()
}

impl World {
    fn plan(&self, n: u8, size: usize) -> u64 {
        self.plan_kept(n, size, PartialKeep::default())
    }

    fn plan_kept(&self, n: u8, size: usize, keep: PartialKeep) -> u64 {
        let place = self
            .table
            .get("me")
            .unwrap()
            .folder_identity("")
            .unwrap()
            .unwrap();
        self.journal
            .plan(&PlannedWrite {
                item: ItemId::from_hex(&format!("{n:02x}{}", "0".repeat(30))).unwrap(),
                source_laptop: LaptopId::from_hex("00112233445566778899aabbccddeeff").unwrap(),
                destination: "me".into(),
                path: format!("Docs/f{n}.txt"),
                size: size as u64,
                actor: Actor {
                    acting_account: "1001".into(),
                    for_account: "1001".into(),
                    permission: Permission::OwnFolders,
                },
                block_size: block_size_for(size as u64),
                source_modified_ns: None,
                source_file: None,
                partial_keep: keep,
                place: Some(place),
            })
            .unwrap()
    }

    /// The stored path of entry `id`'s temporary file.
    fn temp(&self, id: u64) -> String {
        format!("Docs/{}", temp_name(&self.journal.temp_tag(id)))
    }

    fn put(&self, stored: &str, bytes: &[u8]) {
        std::fs::write(self.root.join(stored), bytes).unwrap();
    }

    fn read(&self, stored: &str) -> Option<Vec<u8>> {
        std::fs::read(self.root.join(stored)).ok()
    }

    fn staged(&self, n: u8, bytes: &[u8], on_disk: Option<&[u8]>) -> u64 {
        let id = self.plan(n, bytes.len());
        let temp = self.temp(id);
        if let Some(b) = on_disk {
            self.put(&temp, b);
            // PCTwin keeps each file and folder it makes open until 10 ms after it was made, so
            // nothing made later shares its number and birth time; this stands in for that.
            std::thread::sleep(std::time::Duration::from_millis(11));
        }
        self.journal.staged(id, &temp, &[]).unwrap();
        id
    }

    fn verified(&self, n: u8, bytes: &[u8], on_disk: Option<&[u8]>) -> u64 {
        let id = self.staged(n, bytes, on_disk);
        // The file it sealed: its temporary file as it is (or none, if there is none).
        let sealed = self
            .table
            .get("me")
            .unwrap()
            .stat(&self.temp(id))
            .unwrap()
            .map(|s| s.id);
        self.journal.verified(id, fp(bytes), sealed).unwrap();
        id
    }

    fn applied(&self, n: u8, bytes: &[u8], name: &str) -> u64 {
        let id = self.verified(n, bytes, Some(bytes));
        self.journal.applied(id, name).unwrap();
        id
    }

    fn state(&self, id: u64) -> State {
        self.journal.entry(id).unwrap().unwrap().state
    }

    fn names(&self) -> Vec<String> {
        names(&self.root.join("Docs"))
    }
}

fn names(dir: &Path) -> Vec<String> {
    let mut v: Vec<String> = std::fs::read_dir(dir)
        .unwrap()
        .map(|e| e.unwrap().file_name().to_string_lossy().into_owned())
        .collect();
    v.sort();
    v
}

fn committed_at(state: &State) -> Option<&str> {
    match state {
        State::Committed { final_path, .. } => Some(final_path),
        _ => None,
    }
}

#[test]
fn the_whole_file_fingerprint_is_its_blocks_in_order() {
    let bytes = data(300 * 1024);
    let bs = block_size_for(bytes.len() as u64);
    let blocks: Vec<[u8; 32]> = bytes
        .chunks(bs as usize)
        .map(|c| *blake3::hash(c).as_bytes())
        .collect();
    assert_eq!(
        fp(&bytes),
        file_fingerprint(bytes.len() as u64, bs, &blocks)
    );
    let mut swapped = blocks.clone();
    swapped.swap(0, 1);
    assert_ne!(
        fp(&bytes),
        file_fingerprint(bytes.len() as u64, bs, &swapped)
    );
    assert_ne!(
        file_fingerprint(bytes.len() as u64, bs, &blocks),
        file_fingerprint(bytes.len() as u64, bs * 2, &blocks)
    );
    assert_ne!(
        file_fingerprint(1, bs, &blocks),
        file_fingerprint(2, bs, &blocks)
    );
    // Not the plain BLAKE3 of anything: its own context, fixed for good, because journals keep it.
    assert_ne!(fp(b"abc"), *blake3::hash(b"abc").as_bytes());
    let mut known = blake3::Hasher::new_derive_key("PCTwin 2026-10-08 whole-file fingerprint v1");
    known.update(&3u64.to_le_bytes());
    known.update(&(128u64 * 1024).to_le_bytes());
    known.update(blake3::hash(b"abc").as_bytes());
    assert_eq!(fp(b"abc"), *known.finalize().as_bytes());
    // Exactly `size` bytes, or no fingerprint.
    let short = &bytes[..bytes.len() - 1];
    assert_eq!(
        fingerprint_reader(&mut &short[..], bytes.len() as u64, bs).unwrap(),
        None
    );
    let mut long = bytes.clone();
    long.push(0);
    assert_eq!(
        fingerprint_reader(&mut &long[..], bytes.len() as u64, bs).unwrap(),
        None
    );
    assert_eq!(fingerprint_reader(&mut &b""[..], 0, 0).unwrap(), None);
    // Only ever worked out the one way it was made: in the block size for its size.
    assert_eq!(
        fingerprint_reader(&mut &bytes[..], bytes.len() as u64, bs * 2).unwrap(),
        None
    );
    assert!(fingerprint_reader(&mut &b""[..], 0, bs).unwrap().is_some());
}

#[test]
fn a_write_planned_but_not_staged_is_failed_and_its_temporary_file_removed() {
    let w = world();
    let id = w.plan(1, 10);
    // Made just before the crash, before the journal recorded it.
    w.put(&w.temp(id), b"0123");
    let r = recover(&w.journal, &w.table).unwrap();
    assert_eq!(r.failed.len(), 1);
    assert_eq!(r.removed, 1);
    assert!(matches!(w.state(id), State::Failed { .. }));
    assert!(w.names().is_empty());
}

#[test]
fn a_partly_copied_file_is_kept_to_continue_and_one_that_vanished_is_failed() {
    let w = world();
    let bytes = data(1000);
    let kept = w.staged(1, &bytes, Some(&bytes[..400]));
    let gone = w.staged(2, &bytes, None);
    let r = recover(&w.journal, &w.table).unwrap();
    assert_eq!(r.resumable, [kept]);
    assert_eq!(r.failed.iter().map(|f| f.0).collect::<Vec<_>>(), [gone]);
    assert!(matches!(w.state(kept), State::Staged { .. }));
    assert_eq!(w.read(&w.temp(kept)).unwrap(), &bytes[..400]);
    // Kept across any number of restarts.
    let again = recover(&w.journal, &w.table).unwrap();
    assert_eq!(again.resumable, [kept]);
    assert_eq!(again.removed, 0);
    assert!(w.read(&w.temp(kept)).is_some());
}

#[test]
fn a_verified_file_is_proven_again_then_named_and_committed() {
    let w = world();
    let bytes = data(200 * 1024);
    let id = w.verified(1, &bytes, Some(&bytes));
    let r = recover(&w.journal, &w.table).unwrap();
    assert_eq!(r.committed, [id]);
    assert_eq!(w.names(), ["f1.txt"]);
    assert_eq!(w.read("Docs/f1.txt").unwrap(), bytes);
    let State::Committed {
        final_path,
        fingerprint,
        landed,
    } = w.state(id)
    else {
        panic!("not committed")
    };
    assert_eq!(final_path, "Docs/f1.txt");
    assert_eq!(fingerprint, fp(&bytes));
    let stat = w
        .table
        .get("me")
        .unwrap()
        .stat("Docs/f1.txt")
        .unwrap()
        .unwrap();
    assert_eq!(landed.size, bytes.len() as u64);
    assert_eq!(landed.file, Some(stat.id));
    assert!(landed.modified_ns.is_some());
}

#[test]
fn a_verified_file_that_changed_afterwards_is_never_committed() {
    let w = world();
    let bytes = data(5000);
    let mut changed = bytes.clone();
    changed[17] ^= 1;
    let id = w.verified(1, &bytes, Some(&changed));
    let r = recover(&w.journal, &w.table).unwrap();
    assert_eq!(r.failed.len(), 1);
    assert!(matches!(w.state(id), State::Failed { .. }));
    assert!(w.names().is_empty(), "its temporary file is removed");
}

#[test]
fn a_verified_file_never_replaces_a_person_s_file_with_its_name() {
    let w = world();
    let bytes = data(3000);
    w.put("Docs/f1.txt", b"mine");
    let id = w.verified(1, &bytes, Some(&bytes));
    recover(&w.journal, &w.table).unwrap();
    assert_eq!(committed_at(&w.state(id)), Some("Docs/f1 (2).txt"));
    assert_eq!(w.read("Docs/f1.txt").unwrap(), b"mine");
    assert_eq!(w.read("Docs/f1 (2).txt").unwrap(), bytes);
    assert_eq!(w.names(), ["f1 (2).txt", "f1.txt"]);
}

#[test]
fn a_file_named_just_before_the_crash_is_committed_once_under_that_name() {
    let w = world();
    let bytes = data(4000);
    let id = w.applied(1, &bytes, "Docs/f1.txt");
    std::fs::hard_link(w.root.join(w.temp(id)), w.root.join("Docs/f1.txt")).unwrap();
    let r = recover(&w.journal, &w.table).unwrap();
    assert_eq!(r.committed, [id]);
    assert_eq!(committed_at(&w.state(id)), Some("Docs/f1.txt"));
    assert_eq!(
        w.names(),
        ["f1.txt"],
        "one name, and the temporary one removed"
    );
}

#[test]
fn a_file_whose_temporary_name_was_already_removed_is_committed() {
    let w = world();
    let bytes = data(4000);
    let id = w.applied(1, &bytes, "Docs/f1.txt");
    std::fs::rename(w.root.join(w.temp(id)), w.root.join("Docs/f1.txt")).unwrap();
    recover(&w.journal, &w.table).unwrap();
    assert_eq!(committed_at(&w.state(id)), Some("Docs/f1.txt"));
    assert_eq!(w.read("Docs/f1.txt").unwrap(), bytes);
}

#[test]
fn a_name_lost_in_the_crash_is_given_again() {
    let w = world();
    let bytes = data(4000);
    let id = w.applied(1, &bytes, "Docs/f1.txt");
    recover(&w.journal, &w.table).unwrap();
    assert_eq!(committed_at(&w.state(id)), Some("Docs/f1.txt"));
    assert_eq!(w.names(), ["f1.txt"]);
}

#[test]
fn a_name_another_file_took_is_left_to_it_and_another_is_used() {
    let w = world();
    let bytes = data(4000);
    let id = w.applied(1, &bytes, "Docs/f1.txt");
    // Not this file, though identical: someone else's copy.
    w.put("Docs/f1.txt", &bytes);
    recover(&w.journal, &w.table).unwrap();
    assert_eq!(committed_at(&w.state(id)), Some("Docs/f1 (2).txt"));
    assert_eq!(w.names(), ["f1 (2).txt", "f1.txt"]);
}

#[test]
fn an_empty_file_at_its_name_is_never_taken() {
    let w = world();
    let bytes = data(4000);
    let id = w.applied(1, &bytes, "Docs/f1.txt");
    // A person's own empty file, made while the app was down.
    w.put("Docs/f1.txt", b"");
    let dest = w.table.get("me").unwrap();
    let theirs = dest.stat("Docs/f1.txt").unwrap().unwrap().id;
    recover(&w.journal, &w.table).unwrap();
    assert_eq!(committed_at(&w.state(id)), Some("Docs/f1 (2).txt"));
    assert_eq!(w.read("Docs/f1.txt").unwrap(), b"");
    assert_eq!(dest.stat("Docs/f1.txt").unwrap().unwrap().id, theirs);
    assert_eq!(w.read("Docs/f1 (2).txt").unwrap(), bytes);
}

#[test]
fn another_write_s_committed_empty_file_at_the_name_is_never_replaced() {
    let w = world();
    let bytes = data(4000);
    // A crashed after recording its name, before claiming it.
    let a = w.applied(1, &bytes, "Docs/f1.txt");
    // Meanwhile B (an empty file sent under the same name) landed there and was committed.
    w.put("Docs/f1.txt", b"");
    let b = w.plan(2, 0);
    let bt = w.temp(b);
    w.journal.staged(b, &bt, &[]).unwrap();
    w.journal.verified(b, fp(b""), None).unwrap();
    w.journal.applied(b, "Docs/f1.txt").unwrap();
    let dest = w.table.get("me").unwrap();
    let st = dest.stat("Docs/f1.txt").unwrap().unwrap();
    let landed_as = st.id;
    w.journal
        .committed(
            b,
            pctwin_journal::Landed {
                size: 0,
                modified_ns: None,
                file: Some(landed_as),
            },
        )
        .unwrap();
    recover(&w.journal, &w.table).unwrap();
    assert_eq!(w.read("Docs/f1.txt").unwrap(), b"");
    assert_eq!(dest.stat("Docs/f1.txt").unwrap().unwrap().id, st.id);
    assert_eq!(committed_at(&w.state(a)), Some("Docs/f1 (2).txt"));
}

#[test]
fn a_person_s_identical_copy_at_the_name_is_never_adopted_as_the_move_s_own() {
    let w = world();
    let bytes = data(4000);
    let id = w.applied(1, &bytes, "Docs/f1.txt");
    // The temporary file is gone and a copy with exactly its contents sits at the name: nothing
    // proves it is the file the move sealed, so it is not committed (undo would take it).
    std::fs::remove_file(w.root.join(w.temp(id))).unwrap();
    w.put("Docs/f1.txt", &bytes);
    let r = recover(&w.journal, &w.table).unwrap();
    assert!(r.committed.is_empty());
    assert!(matches!(w.state(id), State::Failed { .. }));
    assert_eq!(w.read("Docs/f1.txt").unwrap(), bytes);
}

#[test]
fn a_named_file_changed_afterwards_is_not_committed_and_is_kept_as_it_is() {
    let w = world();
    let bytes = data(4000);
    let id = w.applied(1, &bytes, "Docs/f1.txt");
    std::fs::remove_file(w.root.join(w.temp(id))).unwrap();
    w.put("Docs/f1.txt", b"edited by the person");
    let r = recover(&w.journal, &w.table).unwrap();
    assert_eq!(r.failed.len(), 1);
    assert!(matches!(w.state(id), State::Failed { .. }));
    assert_eq!(w.read("Docs/f1.txt").unwrap(), b"edited by the person");
}

#[test]
fn a_place_not_reachable_now_is_left_exactly_as_it_is_for_next_time() {
    let mut w = world();
    let bytes = data(4000);
    let id = w.verified(1, &bytes, Some(&bytes));
    let planned = w.plan(2, 10);
    w.put(&w.temp(planned), b"x");
    let full = std::mem::take(&mut w.table);
    let r = recover(&w.journal, &w.table).unwrap();
    assert_eq!(r.left.iter().map(|l| l.0).collect::<Vec<_>>(), [id]);
    // Nothing of a planned write can be trusted, but its file is cleaned up only once its place
    // can be reached.
    assert!(matches!(w.state(planned), State::Failed { .. }));
    assert!(matches!(w.state(id), State::Verified { .. }));
    assert_eq!(w.names().len(), 2);
    w.table = full;
    let r = recover(&w.journal, &w.table).unwrap();
    assert_eq!(r.committed, [id]);
    assert_eq!(r.removed, 1);
    assert_eq!(w.names(), ["f1.txt"]);
}

#[test]
fn a_different_folder_under_the_same_label_is_never_written_to() {
    let mut w = world();
    let bytes = data(4000);
    let id = w.verified(1, &bytes, Some(&bytes));
    let other = tempfile::tempdir().unwrap();
    std::fs::create_dir(other.path().join("Docs")).unwrap();
    std::fs::write(other.path().join(w.temp(id)), &bytes).unwrap();
    let mut table = Destinations::new();
    table
        .approve("me", Approved::MyFolders, other.path())
        .unwrap();
    let real = std::mem::replace(&mut w.table, table);
    let r = recover(&w.journal, &w.table).unwrap();
    assert_eq!(r.left.len(), 1);
    assert_eq!(names(&other.path().join("Docs")).len(), 1);
    assert!(matches!(w.state(id), State::Verified { .. }));
    w.table = real;
    assert_eq!(recover(&w.journal, &w.table).unwrap().committed, [id]);
}

#[test]
fn recovering_twice_is_as_safe_as_once() {
    let w = world();
    let bytes = data(4000);
    let a = w.verified(1, &bytes, Some(&bytes));
    let b = w.staged(2, &bytes, Some(&bytes[..100]));
    let c = w.applied(3, &bytes, "Docs/f3.txt");
    let d = w.plan(4, 10);
    w.put(&w.temp(d), b"x");
    w.put("Docs/mine.txt", b"mine");
    let first = recover(&w.journal, &w.table).unwrap();
    assert_eq!(first.committed, [a, c]);
    assert_eq!(first.resumable, [b]);
    let entries = w.journal.entries().unwrap();
    let disk = w.names();
    let second = recover(&w.journal, &w.table).unwrap();
    assert!(second.committed.is_empty());
    assert!(second.failed.is_empty());
    assert_eq!(second.resumable, [b]);
    assert_eq!(second.removed, 0);
    assert_eq!(w.journal.entries().unwrap(), entries);
    assert_eq!(w.names(), disk);
    assert_eq!(w.read("Docs/mine.txt").unwrap(), b"mine");
}

#[test]
fn a_recovery_cut_short_after_naming_is_finished_by_the_next_without_naming_twice() {
    let w = world();
    let bytes = data(4000);
    let id = w.verified(1, &bytes, Some(&bytes));
    // As the first recovery left it: the name recorded, the file linked, then a crash.
    w.journal.applied(id, "Docs/f1.txt").unwrap();
    std::fs::hard_link(w.root.join(w.temp(id)), w.root.join("Docs/f1.txt")).unwrap();
    recover(&w.journal, &w.table).unwrap();
    assert_eq!(committed_at(&w.state(id)), Some("Docs/f1.txt"));
    assert_eq!(w.names(), ["f1.txt"]);
}

#[test]
fn only_this_journal_s_own_temporary_files_are_ever_removed() {
    let w = world();
    let id = w.plan(1, 10);
    // Another PCTwin's (another person's journal) and a person's own file that looks similar.
    let theirs = format!("Docs/{}", temp_name("0123456789abcdef-1"));
    w.put(&theirs, b"theirs");
    w.put("Docs/.pctwin-notes.part", b"mine");
    w.put(&w.temp(id), b"ours");
    recover(&w.journal, &w.table).unwrap();
    assert_eq!(w.read(&theirs).unwrap(), b"theirs");
    assert_eq!(w.read("Docs/.pctwin-notes.part").unwrap(), b"mine");
    assert!(w.read(&w.temp(id)).is_none());
}

#[test]
fn a_temporary_name_left_behind_by_a_finished_write_is_cleaned_up_once() {
    let w = world();
    let bytes = data(4000);
    let id = w.applied(1, &bytes, "Docs/f1.txt");
    std::fs::hard_link(w.root.join(w.temp(id)), w.root.join("Docs/f1.txt")).unwrap();
    // Committed while another program held the temporary name, so it stayed.
    let stat = w
        .table
        .get("me")
        .unwrap()
        .stat("Docs/f1.txt")
        .unwrap()
        .unwrap();
    w.journal
        .committed(
            id,
            pctwin_journal::Landed {
                size: stat.len,
                modified_ns: None,
                file: None,
            },
        )
        .unwrap();
    let r = recover(&w.journal, &w.table).unwrap();
    assert_eq!(r.removed, 1);
    assert_eq!(w.names(), ["f1.txt"]);
    assert_eq!(w.read("Docs/f1.txt").unwrap(), bytes);
    assert_eq!(recover(&w.journal, &w.table).unwrap().removed, 0);
}

#[test]
fn a_temporary_file_in_a_place_not_reachable_is_cleaned_up_once_it_is() {
    let mut w = world();
    let id = w.plan(1, 10);
    w.put(&w.temp(id), b"x");
    let full = std::mem::take(&mut w.table);
    let r = recover(&w.journal, &w.table).unwrap();
    assert_eq!(r.failed.len(), 1);
    assert_eq!(r.removed, 0);
    assert!(w.read(&w.temp(id)).is_some());
    // Nothing else is kept, so only the record of what is left brings it back.
    w.table = full;
    let r = recover(&w.journal, &w.table).unwrap();
    assert_eq!(r.removed, 1);
    assert!(w.names().is_empty());
}

#[test]
fn a_named_file_edited_after_recovery_decided_is_never_committed() {
    let w = world();
    let small = data(1000);
    let id = w.applied(1, &small, "Docs/e1.txt");
    // Its real name is a second name of the sealed file, as the gate leaves it.
    std::fs::hard_link(w.root.join(w.temp(id)), w.root.join("Docs/e1.txt")).unwrap();
    let target = w.root.join("Docs/e1.txt");
    // Edited in place (same file, same size) after recovery decided to commit it.
    let r = recover_with(&w.journal, &w.table, &|| {
        use std::io::Write;
        let mut f = std::fs::OpenOptions::new()
            .write(true)
            .open(&target)
            .unwrap();
        f.write_all(&vec![b'X'; 1000]).unwrap();
    })
    .unwrap();
    assert!(r.committed.is_empty());
    assert!(matches!(w.state(id), State::Failed { .. }));
    assert_eq!(std::fs::read(&target).unwrap(), vec![b'X'; 1000]);
}

#[test]
fn a_checked_file_edited_after_recovery_decided_is_never_named_or_committed() {
    let w = world();
    let bytes = data(4000);
    let id = w.verified(1, &bytes, Some(&bytes));
    let temp = w.root.join(w.temp(id));
    let r = recover_with(&w.journal, &w.table, &|| {
        use std::io::Write;
        let mut f = std::fs::OpenOptions::new().write(true).open(&temp).unwrap();
        f.write_all(b"edited").unwrap();
    })
    .unwrap();
    assert!(r.committed.is_empty());
    assert!(matches!(w.state(id), State::Failed { .. }));
    assert!(!w.root.join("Docs/f1.txt").exists());
}

#[test]
fn a_checked_file_swapped_for_another_after_recovery_decided_is_never_named() {
    let w = world();
    let bytes = data(4000);
    let id = w.verified(1, &bytes, Some(&bytes));
    let temp = w.root.join(w.temp(id));
    // The same bytes, but another file.
    let r = recover_with(&w.journal, &w.table, &|| {
        std::fs::remove_file(&temp).unwrap();
        std::fs::write(&temp, &bytes).unwrap();
    })
    .unwrap();
    assert!(r.committed.is_empty());
    assert!(!w.root.join("Docs/f1.txt").exists());
}

#[test]
fn a_named_file_swapped_for_an_identical_copy_after_recovery_decided_is_never_committed() {
    let w = world();
    let bytes = data(1000);
    let id = w.applied(1, &bytes, "Docs/e1.txt");
    std::fs::hard_link(w.root.join(w.temp(id)), w.root.join("Docs/e1.txt")).unwrap();
    let target = w.root.join("Docs/e1.txt");
    let r = recover_with(&w.journal, &w.table, &|| {
        // The same bytes, but a person's own file now has the name.
        std::fs::remove_file(&target).unwrap();
        std::fs::write(&target, &bytes).unwrap();
    })
    .unwrap();
    assert!(r.committed.is_empty());
    assert!(matches!(w.state(id), State::Failed { .. }));
    assert_eq!(std::fs::read(&target).unwrap(), bytes);
}

// Partly copied files kept to continue from (second review 9): they do not stay forever.

const DAY: std::time::Duration = std::time::Duration::from_secs(24 * 60 * 60);

impl World {
    /// A partly copied file of `len` bytes, last written `ago` before `now`.
    fn partial(
        &self,
        n: u8,
        len: usize,
        now: std::time::SystemTime,
        ago: std::time::Duration,
    ) -> u64 {
        let id = self.staged(n, &data(len * 2), Some(&data(len)));
        let f = std::fs::File::options()
            .write(true)
            .open(self.root.join(self.temp(id)))
            .unwrap();
        f.set_modified(now - ago).unwrap();
        id
    }
}

fn rules<'a>(
    now: std::time::SystemTime,
    max_bytes: u64,
    wanted: &'a dyn Fn(&pctwin_journal::Entry) -> bool,
) -> KeepPartials<'a> {
    KeepPartials {
        now,
        max_bytes,
        wanted,
    }
}

#[test]
fn the_space_partly_copied_files_take_is_shown() {
    let w = world();
    let now = std::time::SystemTime::now();
    let a = w.partial(1, 1000, now, DAY);
    let b = w.partial(2, 3000, now, 2 * DAY);
    let p = partials(&w.journal, &w.table).unwrap();
    let mut kept: Vec<(u64, u64)> = p.kept.iter().map(|k| (k.id, k.bytes)).collect();
    kept.sort_unstable();
    assert_eq!(kept, [(a, 1000), (b, 3000)]);
    assert_eq!(p.kept_bytes, 4000);
    assert!(p.ended.is_empty() && p.unseen.is_empty());
    // Looking changes nothing.
    assert!(matches!(w.state(a), State::Staged { .. }));
    assert!(w.read(&w.temp(a)).is_some());
}

#[test]
fn a_partly_copied_file_not_added_to_for_too_long_is_removed_and_its_copy_ended() {
    let w = world();
    let now = std::time::SystemTime::now();
    let old = w.partial(1, 1000, now, 31 * DAY);
    let recent = w.partial(2, 2000, now, 29 * DAY);
    // Exactly at the limit: still kept.
    let edge = w.partial(3, 500, now, 30 * DAY);
    let p = expire_partials(&w.journal, &w.table, &rules(now, u64::MAX, &|_| true)).unwrap();
    assert_eq!(p.ended.iter().map(|e| e.0).collect::<Vec<_>>(), [old]);
    assert_eq!(p.freed_bytes, 1000);
    assert!(matches!(w.state(old), State::Failed { .. }));
    assert!(w.read(&w.temp(old)).is_none());
    for id in [recent, edge] {
        assert!(matches!(w.state(id), State::Staged { .. }));
        assert!(w.read(&w.temp(id)).is_some());
    }
    assert_eq!(p.kept_bytes, 2500);
    // Run again: nothing more to do.
    let again = expire_partials(&w.journal, &w.table, &rules(now, u64::MAX, &|_| true)).unwrap();
    assert!(again.ended.is_empty());
    assert_eq!(again.kept_bytes, 2500);
}

#[test]
fn when_partly_copied_files_take_more_than_allowed_the_oldest_go_first() {
    let w = world();
    let now = std::time::SystemTime::now();
    let oldest = w.partial(1, 1000, now, 3 * DAY);
    let middle = w.partial(2, 2000, now, 2 * DAY);
    let newest = w.partial(3, 3000, now, DAY);
    let p = expire_partials(&w.journal, &w.table, &rules(now, 3500, &|_| true)).unwrap();
    let mut ended: Vec<u64> = p.ended.iter().map(|e| e.0).collect();
    ended.sort_unstable();
    assert_eq!(ended, [oldest, middle]);
    assert_eq!(p.freed_bytes, 3000);
    assert_eq!(p.kept_bytes, 3000);
    assert!(matches!(w.state(newest), State::Staged { .. }));
    assert!(w.read(&w.temp(newest)).is_some());
}

#[test]
fn partly_copied_files_that_take_exactly_the_space_allowed_are_all_kept() {
    let w = world();
    let now = std::time::SystemTime::now();
    w.partial(1, 1000, now, 2 * DAY);
    w.partial(2, 2000, now, DAY);
    let p = expire_partials(&w.journal, &w.table, &rules(now, 3000, &|_| true)).unwrap();
    assert!(p.ended.is_empty());
    assert_eq!(p.kept_bytes, 3000);
}

#[test]
fn a_partly_copied_file_whose_move_or_laptop_is_gone_is_removed() {
    let w = world();
    let now = std::time::SystemTime::now();
    let gone = w.partial(1, 1000, now, DAY);
    let wanted = w.partial(2, 1000, now, DAY);
    let p = expire_partials(
        &w.journal,
        &w.table,
        &rules(now, u64::MAX, &|e| e.id != gone),
    )
    .unwrap();
    assert_eq!(p.ended.iter().map(|e| e.0).collect::<Vec<_>>(), [gone]);
    assert!(w.read(&w.temp(gone)).is_none());
    assert!(matches!(w.state(gone), State::Failed { .. }));
    assert!(matches!(w.state(wanted), State::Staged { .. }));
}

#[test]
fn a_partly_copied_file_whose_place_cannot_be_reached_is_kept_and_said() {
    let w = world();
    let now = std::time::SystemTime::now();
    let id = w.partial(1, 1000, now, 365 * DAY);
    // Its place is not approved now (a drive unplugged, say): its age cannot be looked at.
    let none = Destinations::new();
    let p = expire_partials(&w.journal, &none, &rules(now, 0, &|_| true)).unwrap();
    assert_eq!(p.unseen.iter().map(|e| e.0).collect::<Vec<_>>(), [id]);
    assert!(p.ended.is_empty());
    assert!(matches!(w.state(id), State::Staged { .. }));
    assert!(w.read(&w.temp(id)).is_some());
}

#[test]
fn an_unwanted_partly_copied_file_in_a_place_not_reachable_now_is_removed_once_it_is() {
    let w = world();
    let now = std::time::SystemTime::now();
    let id = w.partial(1, 1000, now, DAY);
    let none = Destinations::new();
    let p = expire_partials(&w.journal, &none, &rules(now, u64::MAX, &|_| false)).unwrap();
    assert_eq!(p.ended.iter().map(|e| e.0).collect::<Vec<_>>(), [id]);
    assert_eq!(p.freed_bytes, 0);
    assert!(matches!(w.state(id), State::Failed { .. }));
    assert!(w.read(&w.temp(id)).is_some());
    // Once the place is back, the next start removes it.
    recover(&w.journal, &w.table).unwrap();
    assert!(w.read(&w.temp(id)).is_none());
}

#[test]
fn a_partly_copied_file_written_in_the_future_by_the_clock_is_kept() {
    let w = world();
    let now = std::time::SystemTime::now();
    let id = w.staged(1, &data(2000), Some(&data(1000)));
    std::fs::File::options()
        .write(true)
        .open(w.root.join(w.temp(id)))
        .unwrap()
        .set_modified(now + 10 * DAY)
        .unwrap();
    let p = expire_partials(&w.journal, &w.table, &rules(now, u64::MAX, &|_| true)).unwrap();
    assert!(p.ended.is_empty());
    assert!(matches!(w.state(id), State::Staged { .. }));
}

#[test]
fn expiry_never_touches_checked_files_or_the_persons_own() {
    let w = world();
    let now = std::time::SystemTime::now();
    let bytes = data(4000);
    let checked = w.verified(1, &bytes, Some(&bytes));
    std::fs::File::options()
        .write(true)
        .open(w.root.join(w.temp(checked)))
        .unwrap()
        .set_modified(now - 365 * DAY)
        .unwrap();
    w.put("Docs/mine.txt", b"mine");
    let p = expire_partials(&w.journal, &w.table, &rules(now, 0, &|_| false)).unwrap();
    assert!(p.ended.is_empty() && p.kept.is_empty());
    assert!(matches!(w.state(checked), State::Verified { .. }));
    assert_eq!(w.read(&w.temp(checked)).unwrap(), bytes);
    assert_eq!(w.read("Docs/mine.txt").unwrap(), b"mine");
}

#[test]
fn each_move_waits_as_long_as_the_person_chose_before_it() {
    let w = world();
    let now = std::time::SystemTime::now();
    let ago = |id: u64, d: std::time::Duration| {
        std::fs::File::options()
            .write(true)
            .open(w.root.join(w.temp(id)))
            .unwrap()
            .set_modified(now - d)
            .unwrap();
    };
    let stage = |n: u8, keep: PartialKeep| {
        let id = w.plan_kept(n, 2000, keep);
        w.put(&w.temp(id), &data(1000));
        w.journal.staged(id, &w.temp(id), &[]).unwrap();
        id
    };
    // Three days chosen: four days with nothing added is too long.
    let short = stage(1, PartialKeep::Days3);
    ago(short, 4 * DAY);
    // Ninety days chosen: sixty days is not.
    let long = stage(2, PartialKeep::Days90);
    ago(long, 60 * DAY);
    // Thirty days unless the person picked another.
    let usual = stage(3, PartialKeep::default());
    ago(usual, 31 * DAY);
    let p = expire_partials(&w.journal, &w.table, &rules(now, u64::MAX, &|_| true)).unwrap();
    let mut ended: Vec<u64> = p.ended.iter().map(|e| e.0).collect();
    ended.sort_unstable();
    assert_eq!(ended, [short, usual]);
    assert!(matches!(w.state(long), State::Staged { .. }));
}

#[test]
fn the_keep_time_choices_are_exactly_the_ones_offered() {
    let days: Vec<u32> = PartialKeep::ALL.iter().map(|k| k.days()).collect();
    assert_eq!(days, [3, 7, 14, 30, 60, 90]);
    assert_eq!(PartialKeep::default().days(), 30);
    assert_eq!(PartialKeep::from_days(14), Some(PartialKeep::Days14));
    assert_eq!(PartialKeep::from_days(10), None);
    assert_eq!(PartialKeep::Days7.duration(), 7 * DAY);
}
