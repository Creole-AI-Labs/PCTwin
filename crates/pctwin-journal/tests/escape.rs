//! The consented escape for a drive that never comes back (decided 9 October 2026): undo cannot
//! close while a file it moved is under a hidden name, so the person may choose, once and with the
//! exact paths shown, "Leave these files; I'll remove them myself". The choice is bound to the one
//! offer it answers: a stale, reused or foreign answer is refused and records nothing.

mod common;

use common::*;
use pctwin_journal::{
    CloseError, EscapeItem, Journal, JournalError, Resolved, Undo, UndoGate, UndoOutcome,
};

/// Committed writes 1, 2, 3: a removal part-way, a put-back part-way and a salvage part-way (one
/// of them at the top of the destination), and 4 finished.
fn stuck(j: &Journal) -> Vec<u64> {
    let a = committed(j, 1);
    let b = committed_at(j, 2, "f2.txt");
    let c = committed(j, 3);
    let d = committed(j, 4);
    let permit = j.begin_undo().unwrap();
    j.record_undo(&permit, a, &removing(1)).unwrap();
    j.record_undo(&permit, b, &removing(2)).unwrap();
    j.record_undo(&permit, b, &putting(2, "f2.txt")).unwrap();
    j.record_undo(&permit, c, &removing(3)).unwrap();
    j.record_undo(&permit, c, &salvaging(3, "f3 (kept).txt"))
        .unwrap();
    j.record_undo(
        &permit,
        d,
        &Undo::Done {
            outcome: UndoOutcome::Deleted,
        },
    )
    .unwrap();
    vec![a, b, c, d]
}

#[test]
fn the_offer_shows_the_exact_hidden_path_of_every_part_way_file() {
    let (_d, _p, j) = journal();
    let ids = stuck(&j);
    let offer = j.offer_escape().unwrap();
    assert_eq!(
        offer.items,
        vec![
            EscapeItem {
                id: ids[0],
                destination: "me".into(),
                path: "Docs/.pctwin-undo-00000000000000000000000000000001".into(),
            },
            EscapeItem {
                id: ids[1],
                destination: "me".into(),
                path: ".pctwin-undo-00000000000000000000000000000002".into(),
            },
            EscapeItem {
                id: ids[2],
                destination: "me".into(),
                path: "Docs/.pctwin-salvage-00000000000000000000000000000003".into(),
            },
        ]
    );
}

#[test]
fn accepting_records_each_file_as_kept_at_its_path_and_then_close_goes_ahead() {
    let (_d, path, j) = journal();
    let ids = stuck(&j);
    // The drive never comes back.
    assert!(matches!(
        j.close_undo(&mut |_, _| Ok(Resolved::Unresolved {
            why: "the drive is not there".into()
        })),
        Err(CloseError::Unresolved { .. })
    ));
    let offer = j.offer_escape().unwrap();
    j.accept_escape(&offer.nonce).unwrap();
    for item in &offer.items {
        let Some(Undo::Done {
            outcome: UndoOutcome::KeptAt { at, why },
        }) = j.undo_of(item.id).unwrap()
        else {
            panic!("{:?}", j.undo_of(item.id));
        };
        assert_eq!(at, item.path);
        assert!(!why.is_empty());
    }
    assert_eq!(
        j.undo_of(ids[3]).unwrap(),
        Some(Undo::Done {
            outcome: UndoOutcome::Deleted
        })
    );
    let mut asked = false;
    j.close_undo(&mut |_, _| {
        asked = true;
        Ok(Resolved::Removed)
    })
    .unwrap();
    assert!(!asked);
    drop(j);
    assert_eq!(
        Journal::open(&path).unwrap().undo_gate().unwrap(),
        UndoGate::Closed
    );
}

#[test]
fn an_answer_works_once() {
    let (_d, _p, j) = journal();
    stuck(&j);
    let offer = j.offer_escape().unwrap();
    j.accept_escape(&offer.nonce).unwrap();
    assert!(matches!(
        j.accept_escape(&offer.nonce),
        Err(JournalError::EscapeRefused)
    ));
    // Even when nothing changed between the two answers (an offer of nothing).
    let (_d, _p, quiet) = journal();
    let offer = quiet.offer_escape().unwrap();
    assert!(offer.items.is_empty());
    quiet.accept_escape(&offer.nonce).unwrap();
    assert!(matches!(
        quiet.accept_escape(&offer.nonce),
        Err(JournalError::EscapeRefused)
    ));
}

#[test]
fn an_answer_to_an_older_offer_is_refused_and_records_nothing() {
    let (_d, _p, j) = journal();
    let ids = stuck(&j);
    let first = j.offer_escape().unwrap();
    let second = j.offer_escape().unwrap();
    assert_ne!(first.nonce, second.nonce);
    assert!(matches!(
        j.accept_escape(&first.nonce),
        Err(JournalError::EscapeRefused)
    ));
    assert_eq!(j.undo_of(ids[0]).unwrap(), Some(removing(1)));
}

#[test]
fn an_answer_is_refused_once_what_it_showed_has_changed() {
    let (_d, _p, j) = journal();
    let ids = stuck(&j);
    let offer = j.offer_escape().unwrap();
    // The drive came back for one file and undo moved it on: the list shown is out of date.
    {
        let permit = j.begin_undo().unwrap();
        j.record_undo(&permit, ids[0], &putting(1, "f1.txt"))
            .unwrap();
    }
    assert!(matches!(
        j.accept_escape(&offer.nonce),
        Err(JournalError::EscapeRefused)
    ));
    assert_eq!(j.undo_of(ids[0]).unwrap(), Some(putting(1, "f1.txt")));
    assert_eq!(j.undo_of(ids[1]).unwrap(), Some(putting(2, "f2.txt")));
    // So is one where a new file became part-way.
    let e = committed(&j, 5);
    let offer = j.offer_escape().unwrap();
    {
        let permit = j.begin_undo().unwrap();
        j.record_undo(&permit, e, &removing(5)).unwrap();
    }
    assert!(matches!(
        j.accept_escape(&offer.nonce),
        Err(JournalError::EscapeRefused)
    ));
    assert_eq!(j.undo_of(e).unwrap(), Some(removing(5)));
}

#[test]
fn an_answer_from_another_journal_or_a_made_up_one_is_refused() {
    let (_d1, _p1, mine) = journal();
    let (_d2, _p2, other) = journal();
    let ids = stuck(&mine);
    stuck(&other);
    let _mine = mine.offer_escape().unwrap();
    let theirs = other.offer_escape().unwrap();
    assert!(matches!(
        mine.accept_escape(&theirs.nonce),
        Err(JournalError::EscapeRefused)
    ));
    assert_eq!(mine.undo_of(ids[0]).unwrap(), Some(removing(1)));
    // With no offer at all.
    let (_d3, _p3, fresh) = journal();
    stuck(&fresh);
    assert!(matches!(
        fresh.accept_escape(&[0; 16]),
        Err(JournalError::EscapeRefused)
    ));
    assert!(matches!(
        fresh.accept_escape(&theirs.nonce),
        Err(JournalError::EscapeRefused)
    ));
}

#[test]
fn an_offer_does_not_survive_a_restart() {
    let (_d, path, j) = journal();
    stuck(&j);
    let offer = j.offer_escape().unwrap();
    drop(j);
    let j = Journal::open(&path).unwrap();
    assert!(matches!(
        j.accept_escape(&offer.nonce),
        Err(JournalError::EscapeRefused)
    ));
}

#[test]
fn nonces_are_random_each_time() {
    let (_d, _p, j) = journal();
    stuck(&j);
    let mut seen = std::collections::HashSet::new();
    for _ in 0..64 {
        let n = j.offer_escape().unwrap().nonce;
        assert_ne!(n, [0; 16]);
        assert!(seen.insert(n));
    }
}

#[test]
fn not_while_a_file_is_being_undone_or_a_close_is_running() {
    let (_d, _p, j) = journal();
    let ids = stuck(&j);
    let offer = j.offer_escape().unwrap();
    let held = j.begin_undo().unwrap();
    assert!(matches!(
        j.accept_escape(&offer.nonce),
        Err(JournalError::Busy)
    ));
    drop(held);
    let offer = j.offer_escape().unwrap();
    let mut during = None;
    let _ = j.close_undo(&mut |_, _| {
        during = Some(matches!(
            j.accept_escape(&offer.nonce),
            Err(JournalError::Busy)
        ));
        Ok(Resolved::Unresolved { why: "u".into() })
    });
    assert_eq!(during, Some(true));
    assert_eq!(j.undo_of(ids[0]).unwrap(), Some(removing(1)));
}

#[test]
fn after_the_close_there_is_nothing_to_leave() {
    let (_d, _p, j) = journal();
    stuck(&j);
    j.close_undo(&mut |_, _| Ok(Resolved::Removed)).unwrap();
    let offer = j.offer_escape().unwrap();
    assert!(offer.items.is_empty());
    assert!(matches!(
        j.accept_escape(&offer.nonce),
        Err(JournalError::UndoClosed)
    ));
}
