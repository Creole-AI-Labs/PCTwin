//! The undo gate (Security Design B, decided 8 October 2026): undo is open only until the wipe of
//! the old laptop starts. Starting the wipe closes it for good in this journal, durably, before the
//! wipe can be sent; a cancelled wipe leaves it closed; once closed, undo refuses everything,
//! including finishing an undo a crash interrupted. Every undo write needs a permit taken while the
//! gate was open, and closing waits for every permit still held.

use std::sync::{Arc, mpsc};
use std::time::Duration;

use pctwin_journal::{
    Actor, FileId, Journal, JournalError, Landed, Permission, PlannedWrite, Undo, UndoGate,
    UndoOutcome,
};
use pctwin_record::{ItemId, LaptopId};

fn planned(n: u8) -> PlannedWrite {
    PlannedWrite {
        item: ItemId::from_hex(&format!("{n:02x}{}", "0".repeat(30))).unwrap(),
        source_laptop: LaptopId::from_hex("00112233445566778899aabbccddeeff").unwrap(),
        destination: "me".into(),
        path: format!("Docs/f{n}.txt"),
        size: 10,
        actor: Actor {
            acting_account: "1001".into(),
            for_account: "1001".into(),
            permission: Permission::OwnFolders,
        },
        block_size: 128 * 1024,
        source_modified_ns: None,
        place: None,
        source_file: None,
        partial_keep: Default::default(),
    }
}

/// A committed write in Docs (a folder it made), ready to be undone.
fn committed(j: &Journal, n: u8) -> u64 {
    let id = j.plan(&planned(n)).unwrap();
    j.staged(id, "Docs/.pctwin-t.part", &[("Docs".into(), None)])
        .unwrap();
    j.verified(id, [n; 32], None).unwrap();
    j.applied(id, &format!("Docs/f{n}.txt")).unwrap();
    j.committed(
        id,
        Landed {
            size: 10,
            modified_ns: None,
            file: Some(file()),
        },
    )
    .unwrap();
    id
}

fn file() -> FileId {
    FileId {
        volume: 3,
        index: 4,
    }
}

fn journal() -> (tempfile::TempDir, std::path::PathBuf, Journal) {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("journal.redb");
    let j = Journal::open(&path).unwrap();
    (dir, path, j)
}

#[test]
fn a_new_journal_is_open_for_undo() {
    let (_d, _p, j) = journal();
    assert_eq!(j.undo_gate().unwrap(), UndoGate::Open);
    let permit = j.begin_undo().unwrap();
    let id = committed(&j, 1);
    j.record_undo(&permit, id, &Undo::Removing { file: file() })
        .unwrap();
    assert_eq!(
        j.undo_of(id).unwrap(),
        Some(Undo::Removing { file: file() })
    );
}

#[test]
fn once_close_returns_the_journal_is_closed_on_disk_even_after_reopening() {
    let (_d, path, j) = journal();
    let token = j.close_undo().unwrap();
    assert!(token.is_for(&j));
    assert_eq!(j.undo_gate().unwrap(), UndoGate::Closed);
    drop(j);
    // As after the app stopped right after closing: the close is already on disk.
    let j = Journal::open(&path).unwrap();
    assert_eq!(j.undo_gate().unwrap(), UndoGate::Closed);
    assert!(matches!(j.begin_undo(), Err(JournalError::UndoClosed)));
}

#[test]
fn closing_twice_is_fine_and_stays_closed_so_the_wipe_can_be_sent_again() {
    let (_d, path, j) = journal();
    let first = j.close_undo().unwrap();
    let second = j.close_undo().unwrap();
    assert!(first.is_for(&j) && second.is_for(&j));
    drop(j);
    // A crash after the close and before the wipe was sent: closing again gives a token again.
    let j = Journal::open(&path).unwrap();
    let again = j.close_undo().unwrap();
    assert!(again.is_for(&j));
    assert_eq!(j.undo_gate().unwrap(), UndoGate::Closed);
}

#[test]
fn after_the_close_undo_refuses_everything_including_finishing_an_interrupted_undo() {
    let (_d, path, j) = journal();
    let id = committed(&j, 1);
    {
        let permit = j.begin_undo().unwrap();
        // About to delete; then the app stops.
        j.record_undo(&permit, id, &Undo::Removing { file: file() })
            .unwrap();
    }
    j.close_undo().unwrap();
    drop(j);
    let j = Journal::open(&path).unwrap();
    // The interrupted undo is still readable, so the app can say what may have happened...
    assert_eq!(
        j.undo_of(id).unwrap(),
        Some(Undo::Removing { file: file() })
    );
    // ...but nothing more is done: no permit, so no undo write of any kind.
    assert!(matches!(j.begin_undo(), Err(JournalError::UndoClosed)));
}

#[test]
fn there_is_no_way_back_a_cancelled_wipe_leaves_undo_closed() {
    // No reopening call exists; opening the journal again never reopens it either.
    let (_d, path, j) = journal();
    j.close_undo().unwrap();
    drop(j);
    for _ in 0..3 {
        let j = Journal::open(&path).unwrap();
        assert_eq!(j.undo_gate().unwrap(), UndoGate::Closed);
    }
}

/// Long enough for any working close; a broken one fails the test instead of hanging it.
const LONG: Duration = Duration::from_secs(30);

/// Closes `j` on its own thread; `Some(token is for j)` if it finished within [`LONG`].
fn close_soon(j: &Arc<Journal>) -> Option<bool> {
    let (tx, rx) = mpsc::channel();
    let jj = Arc::clone(j);
    std::thread::spawn(move || {
        let ok = jj.close_undo().unwrap().is_for(&jj);
        let _ = tx.send(ok);
    });
    rx.recv_timeout(LONG).ok()
}

#[test]
fn a_held_permit_makes_closing_wait_until_it_is_dropped() {
    let (_d, _p, j) = journal();
    let id = committed(&j, 1);
    let j = Arc::new(j);
    let (taken_tx, taken_rx) = mpsc::channel::<()>();
    let (release_tx, release_rx) = mpsc::channel::<()>();
    let (closing_tx, closing_rx) = mpsc::channel::<()>();
    let (closed_tx, closed_rx) = mpsc::channel::<bool>();
    // An undo of one file in progress on another thread.
    let undoer = {
        let j = Arc::clone(&j);
        std::thread::spawn(move || {
            let permit = j.begin_undo().unwrap();
            taken_tx.send(()).unwrap();
            release_rx.recv().unwrap();
            // Still allowed to finish recording this file: closing waits for it.
            j.record_undo(
                &permit,
                id,
                &Undo::Done {
                    outcome: UndoOutcome::Deleted,
                },
            )
            .unwrap();
        })
    };
    taken_rx.recv_timeout(LONG).unwrap();
    {
        let j = Arc::clone(&j);
        std::thread::spawn(move || {
            closing_tx.send(()).unwrap();
            let ok = j.close_undo().unwrap().is_for(&j);
            let _ = closed_tx.send(ok);
        });
    }
    closing_rx.recv_timeout(LONG).unwrap();
    // Closing has been asked for and cannot finish while the permit is held.
    assert!(
        closed_rx.recv_timeout(Duration::from_millis(300)).is_err(),
        "closing finished while a file was still being undone"
    );
    assert_eq!(j.undo_gate().unwrap(), UndoGate::Open);
    release_tx.send(()).unwrap();
    // Once it is given back, closing finishes, and the file's last record came first.
    assert!(
        closed_rx
            .recv_timeout(LONG)
            .expect("closing never finished")
    );
    undoer.join().unwrap();
    assert_eq!(j.undo_gate().unwrap(), UndoGate::Closed);
    assert_eq!(
        j.undo_of(id).unwrap(),
        Some(Undo::Done {
            outcome: UndoOutcome::Deleted
        })
    );
}

#[test]
fn once_closing_is_waiting_no_new_permit_is_handed_out() {
    // Whatever the system's lock fairness, a permit asked for after closing began is refused at
    // once (it neither waits for the close nor slips in ahead of it).
    let (_d, _p, j) = journal();
    let j = Arc::new(j);
    let (taken_tx, taken_rx) = mpsc::channel::<()>();
    let (release_tx, release_rx) = mpsc::channel::<()>();
    let (closed_tx, closed_rx) = mpsc::channel::<()>();
    {
        let j = Arc::clone(&j);
        std::thread::spawn(move || {
            let _held = j.begin_undo().unwrap();
            taken_tx.send(()).unwrap();
            let _ = release_rx.recv();
        });
    }
    taken_rx.recv_timeout(LONG).unwrap();
    {
        let j = Arc::clone(&j);
        std::thread::spawn(move || {
            j.close_undo().unwrap();
            let _ = closed_tx.send(());
        });
    }
    // As soon as closing has begun, permits are refused.
    let mut refused = false;
    for _ in 0..20_000 {
        match j.begin_undo() {
            Err(JournalError::UndoClosed) => {
                refused = true;
                break;
            }
            Ok(p) => drop(p),
            Err(other) => panic!("{other:?}"),
        }
        std::thread::yield_now();
    }
    assert!(
        refused,
        "a permit was still handed out while closing waited"
    );
    // Refused while closing is still waiting for the held one, not just after it closed.
    assert_eq!(j.undo_gate().unwrap(), UndoGate::Open, "not closed yet");
    assert!(matches!(j.begin_undo(), Err(JournalError::UndoClosed)));
    release_tx.send(()).unwrap();
    closed_rx
        .recv_timeout(LONG)
        .expect("closing never finished");
    assert!(matches!(j.begin_undo(), Err(JournalError::UndoClosed)));
}

#[test]
fn permits_are_shared_so_files_can_be_undone_side_by_side() {
    let (_d, _p, j) = journal();
    let ia = committed(&j, 1);
    let ib = committed(&j, 2);
    let j = Arc::new(j);
    let a = j.begin_undo().unwrap();
    let b = j.begin_undo().unwrap();
    std::thread::scope(|s| {
        s.spawn(|| {
            let p = j.begin_undo().unwrap();
            j.record_undo(&p, ia, &Undo::Removing { file: file() })
                .unwrap();
        });
    });
    j.record_undo(&b, ib, &Undo::Removing { file: file() })
        .unwrap();
    drop((a, b));
    assert_eq!(close_soon(&j), Some(true), "closing never finished");
}

#[test]
fn a_permit_from_another_journal_is_refused() {
    let (_d1, _p1, mine) = journal();
    let (_d2, _p2, other) = journal();
    let id = committed(&mine, 1);
    mine.close_undo().unwrap();
    let theirs = other.begin_undo().unwrap();
    assert!(matches!(
        mine.record_undo(&theirs, id, &Undo::Removing { file: file() }),
        Err(JournalError::OtherJournal)
    ));
    assert!(matches!(
        mine.record_folder_undo(&theirs, "me", "Docs", &UndoOutcome::Removed),
        Err(JournalError::OtherJournal)
    ));
    assert_eq!(mine.undo_of(id).unwrap(), None);
    assert_eq!(mine.folder_undo_of("me", "Docs").unwrap(), None);
    // And a close token says which journal it closed.
    let token = mine.close_undo().unwrap();
    assert!(token.is_for(&mine));
    assert!(!token.is_for(&other));
}

#[test]
fn a_permit_still_records_folder_undo_while_open() {
    let (_d, _p, j) = journal();
    committed(&j, 1);
    let permit = j.begin_undo().unwrap();
    j.record_folder_undo(&permit, "me", "Docs", &UndoOutcome::Removed)
        .unwrap();
    assert_eq!(
        j.folder_undo_of("me", "Docs").unwrap(),
        Some(UndoOutcome::Removed)
    );
}

#[test]
fn an_unknown_gate_value_is_reported_as_damaged_and_undo_stays_shut() {
    let (_d, path, j) = journal();
    drop(j);
    {
        let db = redb::Database::open(&path).unwrap();
        let tx = db.begin_write().unwrap();
        {
            let gate: redb::TableDefinition<&str, u8> = redb::TableDefinition::new("gate");
            let mut t = tx.open_table(gate).unwrap();
            t.insert("undo", 7).unwrap();
        }
        tx.commit().unwrap();
    }
    let j = Journal::open(&path).unwrap();
    assert!(matches!(j.undo_gate(), Err(JournalError::Damaged(_))));
    assert!(matches!(j.begin_undo(), Err(JournalError::Damaged(_))));
}

#[test]
fn removing_and_deleted_round_trip_through_the_record() {
    let removing = Undo::Removing { file: file() };
    let text = serde_json::to_string(&removing).unwrap();
    assert_eq!(text, r#"{"step":"removing","file":{"volume":3,"index":4}}"#);
    assert_eq!(serde_json::from_str::<Undo>(&text).unwrap(), removing);
    let deleted = Undo::Done {
        outcome: UndoOutcome::Deleted,
    };
    let text = serde_json::to_string(&deleted).unwrap();
    assert_eq!(text, r#"{"step":"done","outcome":{"result":"deleted"}}"#);
    assert_eq!(serde_json::from_str::<Undo>(&text).unwrap(), deleted);
    assert!(UndoOutcome::Deleted.is_final());
}
