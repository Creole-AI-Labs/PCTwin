//! The change journal (Task List 1.6, Engineering Plan: "write durable intent and undo records
//! before any change"): every write on the new laptop goes planned, staged, verified, applied,
//! committed, each step a durable transaction, so after a crash the app knows exactly how far each
//! file got, and a finished move can be undone.

use pctwin_journal::{
    Actor, FileId, Journal, JournalError, Landed, Permission, PlannedWrite, State,
};
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
        block_size: 128 * 1024,
        source_modified_ns: Some(1_780_000_000_000_000_000),
        place: Some(FileId {
            volume: 7,
            index: 9,
        }),
    }
}

fn landed() -> Landed {
    Landed {
        size: 1000,
        modified_ns: Some(1_790_000_000_000_000_000),
        file: Some(FileId {
            volume: 7,
            index: 11,
        }),
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
        j.staged(id, ".pctwin-77-1.part", &[]).unwrap();
        j.verified(id, [7; 32], None).unwrap();
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
        j.verified(id, [1; 32], None),
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
    j.staged(id, ".pctwin-1.part", &[]).unwrap();
    // Repeating a step.
    assert!(matches!(
        j.staged(id, ".pctwin-2.part", &[]),
        Err(JournalError::OutOfOrder { .. })
    ));
    j.verified(id, [1; 32], None).unwrap();
    j.applied(id, "Documents/f1.txt").unwrap();
    j.committed(id, landed()).unwrap();
    // A finished write stays finished.
    assert!(matches!(
        j.failed(id, "late"),
        Err(JournalError::OutOfOrder { .. })
    ));
    assert!(matches!(
        j.staged(id, ".pctwin-3.part", &[]),
        Err(JournalError::OutOfOrder { .. })
    ));
    // An unknown entry.
    assert!(matches!(
        j.staged(9999, ".x", &[]),
        Err(JournalError::NoSuchEntry(9999))
    ));
}

#[test]
fn a_write_can_fail_at_any_unfinished_step_and_says_why() {
    let (_d, j) = journal();
    let a = j.plan(&planned(1)).unwrap();
    j.failed(a, "not part of the plan").unwrap();
    let b = j.plan(&planned(2)).unwrap();
    j.staged(b, ".pctwin-b.part", &[]).unwrap();
    j.verified(b, [2; 32], None).unwrap();
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
    j.staged(id, ".pctwin-9.part", &[]).unwrap();
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
    j.staged(b, ".pctwin-b.part", &[]).unwrap();
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
        j.staged(a, ".pctwin-a.part", &[]).unwrap();
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

#[test]
fn each_journal_has_its_own_number_kept_across_reopening_and_naming_its_temporary_files() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("journal.redb");
    let tag = Journal::open(&path).unwrap().temp_tag(5);
    let again = Journal::open(&path).unwrap().temp_tag(5);
    assert_eq!(tag, again);
    let (hex, entry) = tag.split_once('-').unwrap();
    assert_eq!(hex.len(), 16);
    assert!(
        hex.chars()
            .all(|c| c.is_ascii_hexdigit() && !c.is_ascii_uppercase())
    );
    assert_eq!(entry, "5");
    let other = tempfile::tempdir().unwrap();
    let theirs = Journal::open(&other.path().join("journal.redb"))
        .unwrap()
        .temp_tag(5);
    assert_ne!(
        tag, theirs,
        "two journals never name a temporary file the same"
    );
}

#[test]
fn a_name_is_recorded_before_the_file_gets_it_and_can_move_on_if_taken() {
    let (_d, j) = journal();
    let id = j.plan(&planned(1)).unwrap();
    j.staged(id, "d/.pctwin-x.part", &[]).unwrap();
    assert!(matches!(
        j.applied(id, "d/f1.txt"),
        Err(JournalError::OutOfOrder { .. })
    ));
    j.verified(id, [3; 32], None).unwrap();
    j.applied(id, "d/f1.txt").unwrap();
    assert_eq!(
        j.entry(id).unwrap().unwrap().state,
        State::Applied {
            temp: "d/.pctwin-x.part".into(),
            final_path: "d/f1.txt".into(),
            fingerprint: [3; 32],
            file: None,
        }
    );
    // Something took that name first: another name, still before the file gets it.
    j.applied(id, "d/f1 (2).txt").unwrap();
    j.committed(id, landed()).unwrap();
    assert_eq!(
        j.entry(id).unwrap().unwrap().state,
        State::Committed {
            final_path: "d/f1 (2).txt".into(),
            fingerprint: [3; 32],
            landed: landed(),
        }
    );
    assert!(matches!(
        j.applied(id, "d/f1 (3).txt"),
        Err(JournalError::OutOfOrder { .. })
    ));
}

#[test]
fn an_identical_file_already_there_is_recorded_as_existing_never_as_written() {
    let (_d, j) = journal();
    let a = j.plan(&planned(1)).unwrap();
    j.existing(a, "Documents/f1.txt").unwrap();
    let b = j.plan(&planned(2)).unwrap();
    j.staged(b, ".pctwin-b.part", &[]).unwrap();
    j.existing(b, "Documents/f2.txt").unwrap();
    let c = j.plan(&planned(3)).unwrap();
    j.staged(c, ".pctwin-c.part", &[]).unwrap();
    j.verified(c, [1; 32], None).unwrap();
    assert!(matches!(
        j.existing(c, "x"),
        Err(JournalError::OutOfOrder { .. })
    ));
    assert_eq!(
        j.entry(a).unwrap().unwrap().state,
        State::Existing {
            stored_path: "Documents/f1.txt".into()
        }
    );
    assert!(j.entry(b).unwrap().unwrap().state.is_finished());
    assert!(matches!(
        j.failed(a, "late"),
        Err(JournalError::OutOfOrder { .. })
    ));
    let open: Vec<u64> = j.unfinished().unwrap().iter().map(|e| e.id).collect();
    assert_eq!(open, [c]);
}

#[test]
fn only_one_unfinished_write_of_a_file_at_a_time() {
    let (_d, j) = journal();
    let first = j.plan(&planned(1)).unwrap();
    j.staged(first, ".pctwin-1.part", &[]).unwrap();
    // The same file from another old laptop is a different file.
    let mut elsewhere = planned(1);
    elsewhere.source_laptop = LaptopId::from_hex("ffeeddccbbaa99887766554433221100").unwrap();
    let theirs = j.plan(&elsewhere).unwrap();
    let again = j.plan(&planned(1)).unwrap();
    match j.entry(first).unwrap().unwrap().state {
        State::Failed { why, reached } => {
            assert_eq!(why, "it was started again");
            assert_eq!(
                *reached,
                State::Staged {
                    temp: ".pctwin-1.part".into()
                }
            );
        }
        other => panic!("{other:?}"),
    }
    let open: Vec<u64> = j.unfinished().unwrap().iter().map(|e| e.id).collect();
    assert_eq!(open, [theirs, again]);
    // A finished write is history: starting the file again leaves it as it was.
    j.staged(again, ".pctwin-2.part", &[]).unwrap();
    j.verified(again, [1; 32], None).unwrap();
    j.applied(again, "f1.txt").unwrap();
    j.committed(again, landed()).unwrap();
    let later = j.plan(&planned(1)).unwrap();
    assert!(matches!(
        j.entry(again).unwrap().unwrap().state,
        State::Committed { .. }
    ));
    assert_eq!(j.entry(later).unwrap().unwrap().state, State::Planned);
}

#[test]
fn unfinished_writes_stay_listed_until_finished_however_they_finish() {
    let (_d, j) = journal();
    let ids: Vec<u64> = (1..=6).map(|n| j.plan(&planned(n)).unwrap()).collect();
    j.failed(ids[0], "x").unwrap();
    j.existing(ids[1], "f").unwrap();
    for id in [ids[2], ids[3]] {
        j.staged(id, ".pctwin-t.part", &[]).unwrap();
        j.verified(id, [0; 32], None).unwrap();
        j.applied(id, "f").unwrap();
    }
    j.committed(ids[2], landed()).unwrap();
    let open: Vec<u64> = j.unfinished().unwrap().iter().map(|e| e.id).collect();
    assert_eq!(open, [ids[3], ids[4], ids[5]]);
    let after: Vec<u64> = j
        .entries_after(ids[3])
        .unwrap()
        .iter()
        .map(|e| e.id)
        .collect();
    assert_eq!(after, [ids[4], ids[5]]);
}

#[test]
fn folders_made_for_a_write_are_recorded_once_by_whoever_made_them() {
    let (_d, j) = journal();
    let a = j.plan(&planned(1)).unwrap();
    let made = [
        (
            "Docs/New".to_string(),
            Some(FileId {
                volume: 1,
                index: 2,
            }),
        ),
        ("Docs/New/Deeper".to_string(), None),
    ];
    j.staged(a, "Docs/New/Deeper/.pctwin-a.part", &made)
        .unwrap();
    let b = j.plan(&planned(2)).unwrap();
    j.staged(
        b,
        "Docs/New/.pctwin-b.part",
        &[(
            "Docs/New".to_string(),
            Some(FileId {
                volume: 1,
                index: 99,
            }),
        )],
    )
    .unwrap();
    let mut folders = j.made_folders().unwrap();
    folders.sort_by(|x, y| x.folder.cmp(&y.folder));
    assert_eq!(folders.len(), 2);
    assert_eq!(folders[0].folder, "Docs/New");
    assert_eq!(folders[0].entry, a);
    assert_eq!(
        folders[0].id,
        Some(FileId {
            volume: 1,
            index: 2
        })
    );
    assert_eq!(folders[0].destination, "me");
    assert_eq!(folders[1].folder, "Docs/New/Deeper");
    assert_eq!(folders[1].id, None);
}

#[test]
fn clean_up_progress_and_leftovers_are_kept_across_reopening() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("journal.redb");
    {
        let j = Journal::open(&path).unwrap();
        assert_eq!(j.swept_upto().unwrap(), 0);
        assert!(j.leftovers().unwrap().is_empty());
        j.record_sweep(7, &[3, 5], &[]).unwrap();
    }
    let j = Journal::open(&path).unwrap();
    assert_eq!(j.swept_upto().unwrap(), 7);
    assert_eq!(j.leftovers().unwrap(), [3, 5]);
    j.record_sweep(9, &[8], &[3]).unwrap();
    assert_eq!(j.swept_upto().unwrap(), 9);
    assert_eq!(j.leftovers().unwrap(), [5, 8]);
}

#[test]
fn landed_blocks_are_checkpointed_while_a_file_is_received_and_cleared_once_it_is_whole() {
    let (_d, j) = journal();
    let a = j.plan(&planned(1)).unwrap();
    let b = j.plan(&planned(2)).unwrap();
    // Only while a file's temporary file exists.
    assert!(matches!(
        j.checkpoint(a, &[(0, [1; 32])], true),
        Err(JournalError::OutOfOrder { .. })
    ));
    j.staged(a, ".pctwin-a.part", &[]).unwrap();
    j.staged(b, ".pctwin-b.part", &[]).unwrap();
    j.checkpoint(a, &[(3, [3; 32]), (0, [1; 32])], false)
        .unwrap();
    j.checkpoint(a, &[(1, [2; 32]), (0, [1; 32])], true)
        .unwrap();
    j.checkpoint(b, &[(0, [9; 32])], true).unwrap();
    assert_eq!(
        j.blocks(a).unwrap(),
        [(0, [1; 32]), (1, [2; 32]), (3, [3; 32])]
    );
    // Whole: the fingerprint is all that is needed from here.
    j.verified(a, [7; 32], None).unwrap();
    assert!(j.blocks(a).unwrap().is_empty());
    assert!(matches!(
        j.checkpoint(a, &[(4, [4; 32])], true),
        Err(JournalError::OutOfOrder { .. })
    ));
    assert_eq!(j.blocks(b).unwrap(), [(0, [9; 32])]);
    j.failed(b, "x").unwrap();
    assert!(j.blocks(b).unwrap().is_empty());
}

#[test]
fn a_file_started_again_loses_the_checkpoints_of_its_earlier_attempt() {
    let (_d, j) = journal();
    let first = j.plan(&planned(1)).unwrap();
    j.staged(first, ".pctwin-1.part", &[]).unwrap();
    j.checkpoint(first, &[(0, [1; 32])], true).unwrap();
    let again = j.plan(&planned(1)).unwrap();
    assert!(j.blocks(first).unwrap().is_empty());
    assert!(j.blocks(again).unwrap().is_empty());
}

#[test]
fn checkpoints_written_durably_survive_reopening() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("journal.redb");
    let id = {
        let j = Journal::open(&path).unwrap();
        let id = j.plan(&planned(1)).unwrap();
        j.staged(id, ".pctwin-1.part", &[]).unwrap();
        j.checkpoint(id, &[(5, [5; 32])], true).unwrap();
        id
    };
    let j = Journal::open(&path).unwrap();
    assert_eq!(j.blocks(id).unwrap(), [(5, [5; 32])]);
}

#[test]
fn undo_is_recorded_beside_a_committed_write_and_never_changes_the_write() {
    use pctwin_journal::{Undo, UndoOutcome};
    let (_d, j) = journal();
    let id = j.plan(&planned(1)).unwrap();
    // Only a committed write can be undone.
    assert!(matches!(
        j.record_undo(
            id,
            &Undo::Aside {
                file: None,
                at: "x".into(),
                staging: None,
                made: Vec::new(),
            }
        ),
        Err(JournalError::OutOfOrder { .. })
    ));
    j.staged(id, "Docs/.pctwin-t.part", &[("Docs".into(), None)])
        .unwrap();
    j.verified(id, [1; 32], None).unwrap();
    j.applied(id, "Docs/f1.txt").unwrap();
    j.committed(id, landed()).unwrap();
    let before = j.entry(id).unwrap().unwrap();
    assert_eq!(j.undo_of(id).unwrap(), None);
    let moving = Undo::Aside {
        file: Some(FileId {
            volume: 1,
            index: 2,
        }),
        at: "Undone/Docs/f1.txt".into(),
        staging: Some("Docs/.pctwin-move-t".into()),
        made: vec!["Undone".into()],
    };
    j.record_undo(id, &moving).unwrap();
    assert_eq!(j.undo_of(id).unwrap(), Some(moving));
    let done = Undo::Done {
        outcome: UndoOutcome::Trashed,
    };
    j.record_undo(id, &done).unwrap();
    assert_eq!(j.undo_of(id).unwrap(), Some(done));
    assert_eq!(j.entry(id).unwrap().unwrap(), before);
    // Folders: only those the move made.
    assert!(
        j.record_folder_undo("me", "Elsewhere", &UndoOutcome::Removed)
            .is_err()
    );
    j.record_folder_undo("me", "Docs", &UndoOutcome::Removed)
        .unwrap();
    assert_eq!(
        j.folder_undo_of("me", "Docs").unwrap(),
        Some(UndoOutcome::Removed)
    );
    assert_eq!(j.folder_undo_of("me", "Other").unwrap(), None);
    // Only "not done this time" is tried again.
    assert!(UndoOutcome::Trashed.is_final());
    assert!(UndoOutcome::Kept { why: "x".into() }.is_final());
    assert!(UndoOutcome::AlreadyGone.is_final());
    assert!(UndoOutcome::Removed.is_final());
    assert!(!UndoOutcome::NotDone { why: "x".into() }.is_final());
}
