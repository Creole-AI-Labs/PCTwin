//! The change journal (Task List 1.6, Engineering Plan: "write durable intent and undo records
//! before any change"): every write on the new laptop goes planned, staged, verified, applied,
//! committed, each step a durable transaction, so after a crash the app knows exactly how far each
//! file got, and a finished move can be undone.

use pctwin_journal::{Actor, Journal, JournalError, Landed, Permission, PlannedWrite, State};
use pctwin_record::{ItemId, LaptopId};

fn item(n: u8) -> ItemId {
    ItemId::from_hex(&format!("{n:02x}{}", "0".repeat(30))).unwrap()
}

fn laptop() -> LaptopId {
    LaptopId::from_hex("00112233445566778899aabbccddeeff").unwrap()
}

fn planned(n: u8) -> PlannedWrite {
    PlannedWrite {
        item: item(n),
        source_laptop: laptop(),
        destination: "me".into(),
        path: format!("Documents/f{n}.txt"),
        size: 1000 + u64::from(n),
        actor: Actor {
            acting_account: "1001".into(),
            for_account: "1001".into(),
            permission: Permission::OwnFolders,
        },
    }
}

fn landed() -> Landed {
    Landed {
        size: 1000,
        modified_ns: Some(1_790_000_000_000_000_000),
    }
}

fn journal() -> (tempfile::TempDir, Journal) {
    let dir = tempfile::tempdir().unwrap();
    let j = Journal::open(&dir.path().join("journal.redb")).unwrap();
    (dir, j)
}

#[test]
fn a_write_goes_through_every_step_and_is_read_back_after_reopening() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("journal.redb");
    let id = {
        let j = Journal::open(&path).unwrap();
        let id = j.plan(&planned(1)).unwrap();
        j.staged(id, ".pctwin-77-1.part").unwrap();
        j.verified(id, [7; 32]).unwrap();
        j.applied(id, "Documents/f1.txt").unwrap();
        j.committed(id, landed()).unwrap();
        id
    };
    let j = Journal::open(&path).unwrap();
    let e = j.entry(id).unwrap().unwrap();
    assert_eq!(e.write, planned(1));
    assert_eq!(
        e.state,
        State::Committed {
            final_path: "Documents/f1.txt".into(),
            fingerprint: [7; 32],
            landed: landed(),
        }
    );
    assert!(j.unfinished().unwrap().is_empty());
}

#[test]
fn every_entry_says_who_acted_for_whom_with_what_permission() {
    let (_d, j) = journal();
    let mut w = planned(2);
    w.actor = Actor {
        acting_account: "1001".into(),
        for_account: "1002".into(),
        permission: Permission::AdminHelper,
    };
    let id = j.plan(&w).unwrap();
    let e = j.entry(id).unwrap().unwrap();
    assert_eq!(e.write.item, item(2));
    assert_eq!(e.write.source_laptop, laptop());
    assert_eq!(e.write.destination, "me");
    assert_eq!(e.write.actor.for_account, "1002");
    assert_eq!(e.write.actor.permission, Permission::AdminHelper);
}

#[test]
fn steps_cannot_be_skipped_repeated_or_undone() {
    let (_d, j) = journal();
    let id = j.plan(&planned(1)).unwrap();
    // Skipping ahead.
    assert!(matches!(
        j.verified(id, [1; 32]),
        Err(JournalError::OutOfOrder { .. })
    ));
    assert!(matches!(
        j.applied(id, "x"),
        Err(JournalError::OutOfOrder { .. })
    ));
    assert!(matches!(
        j.committed(id, landed()),
        Err(JournalError::OutOfOrder { .. })
    ));
    j.staged(id, ".pctwin-1.part").unwrap();
    // Repeating a step.
    assert!(matches!(
        j.staged(id, ".pctwin-2.part"),
        Err(JournalError::OutOfOrder { .. })
    ));
    j.verified(id, [1; 32]).unwrap();
    j.applied(id, "Documents/f1.txt").unwrap();
    j.committed(id, landed()).unwrap();
    // A finished write stays finished.
    assert!(matches!(
        j.failed(id, "late"),
        Err(JournalError::OutOfOrder { .. })
    ));
    assert!(matches!(
        j.staged(id, ".pctwin-3.part"),
        Err(JournalError::OutOfOrder { .. })
    ));
    // An unknown entry.
    assert!(matches!(
        j.staged(9999, ".x"),
        Err(JournalError::NoSuchEntry(9999))
    ));
}

#[test]
fn a_write_can_fail_at_any_unfinished_step_and_says_why() {
    let (_d, j) = journal();
    let a = j.plan(&planned(1)).unwrap();
    j.failed(a, "not part of the plan").unwrap();
    let b = j.plan(&planned(2)).unwrap();
    j.staged(b, ".pctwin-b.part").unwrap();
    j.verified(b, [2; 32]).unwrap();
    j.failed(b, "disk full").unwrap();
    for (id, why) in [(a, "not part of the plan"), (b, "disk full")] {
        match j.entry(id).unwrap().unwrap().state {
            State::Failed { why: w, .. } => assert_eq!(w, why),
            other => panic!("{other:?}"),
        }
    }
    // Failed is final.
    assert!(matches!(
        j.failed(a, "again"),
        Err(JournalError::OutOfOrder { .. })
    ));
}

#[test]
fn a_failed_write_keeps_the_step_it_reached_for_clean_up() {
    let (_d, j) = journal();
    let id = j.plan(&planned(1)).unwrap();
    j.staged(id, ".pctwin-9.part").unwrap();
    j.failed(id, "the old laptop went away").unwrap();
    match j.entry(id).unwrap().unwrap().state {
        State::Failed { reached, .. } => assert_eq!(
            *reached,
            State::Staged {
                temp: ".pctwin-9.part".into()
            }
        ),
        other => panic!("{other:?}"),
    }
}

#[test]
fn unfinished_writes_are_listed_in_the_order_they_were_planned() {
    let (_d, j) = journal();
    let a = j.plan(&planned(1)).unwrap();
    let b = j.plan(&planned(2)).unwrap();
    let c = j.plan(&planned(3)).unwrap();
    j.staged(b, ".pctwin-b.part").unwrap();
    j.failed(c, "skipped").unwrap();
    let ids: Vec<u64> = j.unfinished().unwrap().iter().map(|e| e.id).collect();
    assert_eq!(ids, [a, b]);
    let all: Vec<u64> = j.entries().unwrap().iter().map(|e| e.id).collect();
    assert_eq!(all, [a, b, c]);
}

#[test]
fn a_crash_leaves_the_journal_at_the_last_step_taken() {
    // Each step is its own committed transaction, so the app stopping anywhere leaves exactly the
    // steps already taken.
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("journal.redb");
    let (a, b) = {
        let j = Journal::open(&path).unwrap();
        let a = j.plan(&planned(1)).unwrap();
        j.staged(a, ".pctwin-a.part").unwrap();
        let b = j.plan(&planned(2)).unwrap();
        (a, b)
    };
    let j = Journal::open(&path).unwrap();
    assert_eq!(
        j.entry(a).unwrap().unwrap().state,
        State::Staged {
            temp: ".pctwin-a.part".into()
        }
    );
    assert_eq!(j.entry(b).unwrap().unwrap().state, State::Planned);
    // New entries never reuse a number.
    let c = j.plan(&planned(3)).unwrap();
    assert!(c > b);
}
#[test]
fn a_damaged_journal_is_reported_not_trusted() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("journal.redb");
    std::fs::write(&path, b"this is not a journal at all, just some bytes").unwrap();
    assert!(matches!(
        Journal::open(&path),
        Err(JournalError::Damaged(_))
    ));
}

#[test]
fn only_one_copy_of_the_app_uses_the_journal_at_a_time() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("journal.redb");
    let _first = Journal::open(&path).unwrap();
    assert!(matches!(Journal::open(&path), Err(JournalError::InUse)));
}

#[test]
fn a_journal_from_a_newer_pctwin_is_refused_safely() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("journal.redb");
    drop(Journal::open(&path).unwrap());
    {
        // As a newer PCTwin would leave it: its format number in the "meta" table.
        let db = redb::Database::open(&path).unwrap();
        let tx = db.begin_write().unwrap();
        {
            let meta: redb::TableDefinition<&str, u32> = redb::TableDefinition::new("meta");
            let mut t = tx.open_table(meta).unwrap();
            t.insert("format", pctwin_journal::FORMAT + 1).unwrap();
        }
        tx.commit().unwrap();
    }
    assert!(matches!(
        Journal::open(&path),
        Err(JournalError::NewerFormat { .. })
    ));
}
