//! Undo stages on Linux and macOS ("move aside first, then check", decided 9 October 2026): every
//! name PCTwin creates or moves to is journaled first (Removing before the move aside, Putting
//! before any rename out of the private name, Salvaging before a salvage copy), Done is final, and
//! removals and their Done records are written in groups, one durable transaction each.

mod common;

use common::*;
use pctwin_journal::{Journal, JournalError, Undo, UndoOutcome};

#[test]
fn every_stage_round_trips_through_the_record_in_its_exact_shape() {
    let cases = [
        (
            removing(1),
            r#"{"step":"removing","file":{"volume":3,"index":101},"dir_id":{"volume":3,"index":2},"private":".pctwin-undo-00000000000000000000000000000001"}"#,
        ),
        (
            putting(1, "f1.txt"),
            r#"{"step":"putting","file":{"volume":3,"index":101},"dir_id":{"volume":3,"index":2},"private":".pctwin-undo-00000000000000000000000000000001","to":"f1.txt"}"#,
        ),
        (
            salvaging(1, "f1 (kept).txt"),
            r#"{"step":"salvaging","file":{"volume":3,"index":101},"dir_id":{"volume":3,"index":2},"temp":".pctwin-salvage-00000000000000000000000000000001","to":"f1 (kept).txt"}"#,
        ),
        (
            Undo::Done {
                outcome: UndoOutcome::KeptAt {
                    at: "Docs/f1 (kept).txt".into(),
                    why: "it changed while it was removed".into(),
                },
            },
            r#"{"step":"done","outcome":{"result":"kept-at","at":"Docs/f1 (kept).txt","why":"it changed while it was removed"}}"#,
        ),
        (
            Undo::Done {
                outcome: UndoOutcome::Deleted,
            },
            r#"{"step":"done","outcome":{"result":"deleted"}}"#,
        ),
    ];
    for (undo, text) in cases {
        assert_eq!(serde_json::to_string(&undo).unwrap(), text);
        assert_eq!(serde_json::from_str::<Undo>(text).unwrap(), undo);
    }
}

#[test]
fn a_copy_kept_at_a_visible_path_is_final_and_needs_action() {
    let kept_at = UndoOutcome::KeptAt {
        at: "Docs/f1 (kept).txt".into(),
        why: "w".into(),
    };
    assert!(kept_at.is_final());
    assert!(kept_at.needs_action());
    assert!(UndoOutcome::NotDone { why: "w".into() }.needs_action());
    assert!(UndoOutcome::Kept { why: "w".into() }.needs_action());
    assert!(!UndoOutcome::Deleted.needs_action());
    assert!(!UndoOutcome::AlreadyGone.needs_action());
    assert!(!UndoOutcome::Removed.needs_action());
}

#[test]
fn only_done_is_done() {
    assert!(!removing(1).is_done());
    assert!(!putting(1, "a").is_done());
    assert!(!salvaging(1, "a").is_done());
    assert!(
        Undo::Done {
            outcome: UndoOutcome::AlreadyGone
        }
        .is_done()
    );
}

#[test]
fn a_group_of_removals_is_written_together_and_read_back_after_reopening() {
    let (_d, path, j) = journal();
    let ids: Vec<u64> = (1..=64).map(|n| committed(&j, n)).collect();
    {
        let permit = j.begin_undo().unwrap();
        let items: Vec<(u64, Undo)> = ids
            .iter()
            .zip(1u8..)
            .map(|(id, n)| (*id, removing(n)))
            .collect();
        j.record_removals(&permit, &items).unwrap();
    }
    drop(j);
    let j = Journal::open(&path).unwrap();
    for (id, n) in ids.iter().zip(1u8..) {
        assert_eq!(j.undo_of(*id).unwrap(), Some(removing(n)));
    }
}

#[test]
fn a_group_with_one_bad_item_writes_nothing_at_all() {
    let (_d, _p, j) = journal();
    let a = committed(&j, 1);
    let b = committed(&j, 2);
    // Not committed: cannot be undone.
    let unfinished = j.plan(&planned(3)).unwrap();
    let permit = j.begin_undo().unwrap();
    assert!(matches!(
        j.record_removals(&permit, &[(a, removing(1)), (unfinished, removing(3))]),
        Err(JournalError::OutOfOrder { .. })
    ));
    // Not a removal.
    assert!(matches!(
        j.record_removals(&permit, &[(a, removing(1)), (b, putting(2, "f2.txt"))]),
        Err(JournalError::OutOfOrder { .. })
    ));
    // No such entry.
    assert!(matches!(
        j.record_removals(&permit, &[(a, removing(1)), (999, removing(2))]),
        Err(JournalError::NoSuchEntry(999))
    ));
    assert_eq!(j.undo_of(a).unwrap(), None);
    assert_eq!(j.undo_of(b).unwrap(), None);
    // A put-back is never part of a group of removals, even where it could follow.
    j.record_undo(&permit, b, &removing(2)).unwrap();
    assert!(matches!(
        j.record_removals(&permit, &[(b, putting(2, "f2.txt"))]),
        Err(JournalError::OutOfOrder { .. })
    ));
    assert_eq!(j.undo_of(b).unwrap(), Some(removing(2)));
}

#[test]
fn the_done_records_of_a_group_are_written_together() {
    let (_d, path, j) = journal();
    let a = committed(&j, 1);
    let b = committed(&j, 2);
    let c = committed(&j, 3);
    let permit = j.begin_undo().unwrap();
    j.record_removals(
        &permit,
        &[(a, removing(1)), (b, removing(2)), (c, removing(3))],
    )
    .unwrap();
    let kept_at = UndoOutcome::KeptAt {
        at: "Docs/f2 (kept).txt".into(),
        why: "w".into(),
    };
    j.finish_undos(
        &permit,
        &[
            (a, UndoOutcome::Deleted),
            (b, kept_at.clone()),
            (c, UndoOutcome::AlreadyGone),
        ],
    )
    .unwrap();
    drop(permit);
    drop(j);
    let j = Journal::open(&path).unwrap();
    let done = |o| Some(Undo::Done { outcome: o });
    assert_eq!(j.undo_of(a).unwrap(), done(UndoOutcome::Deleted));
    assert_eq!(j.undo_of(b).unwrap(), done(kept_at));
    assert_eq!(j.undo_of(c).unwrap(), done(UndoOutcome::AlreadyGone));
}

#[test]
fn a_done_group_with_one_bad_item_writes_nothing_at_all() {
    let (_d, _p, j) = journal();
    let a = committed(&j, 1);
    let b = committed(&j, 2);
    let permit = j.begin_undo().unwrap();
    j.record_removals(&permit, &[(a, removing(1))]).unwrap();
    j.finish_undos(&permit, &[(b, UndoOutcome::Deleted)])
        .unwrap();
    // b is already finally done: that cannot be written over, so a is not finished either.
    assert!(matches!(
        j.finish_undos(
            &permit,
            &[(a, UndoOutcome::Deleted), (b, UndoOutcome::AlreadyGone)]
        ),
        Err(JournalError::OutOfOrder { .. })
    ));
    assert_eq!(j.undo_of(a).unwrap(), Some(removing(1)));
}

#[test]
fn putting_and_salvaging_follow_only_a_removal_of_the_same_file() {
    let (_d, _p, j) = journal();
    let a = committed(&j, 1);
    let permit = j.begin_undo().unwrap();
    // Nothing was moved aside yet: there is nothing to put back or salvage.
    assert!(matches!(
        j.record_undo(&permit, a, &putting(1, "f1.txt")),
        Err(JournalError::OutOfOrder { .. })
    ));
    assert!(matches!(
        j.record_undo(&permit, a, &salvaging(1, "f1 (kept).txt")),
        Err(JournalError::OutOfOrder { .. })
    ));
    j.record_undo(&permit, a, &removing(1)).unwrap();
    // Another file's stage cannot follow this file's.
    assert!(matches!(
        j.record_undo(&permit, a, &putting(2, "f1.txt")),
        Err(JournalError::OutOfOrder { .. })
    ));
    // Nor one that says it is in another folder.
    let mut elsewhere = dir();
    elsewhere.index = std::num::NonZeroU64::new(9).unwrap();
    for stage in [putting(1, "f1.txt"), salvaging(1, "f1 (kept).txt")] {
        let moved = match stage {
            Undo::Putting {
                file, private, to, ..
            } => Undo::Putting {
                file,
                dir_id: elsewhere,
                private,
                to,
            },
            Undo::Salvaging { file, temp, to, .. } => Undo::Salvaging {
                file,
                dir_id: elsewhere,
                temp,
                to,
            },
            other => other,
        };
        assert!(matches!(
            j.record_undo(&permit, a, &moved),
            Err(JournalError::OutOfOrder { .. })
        ));
    }
    assert!(matches!(
        j.record_undo(&permit, a, &removing(2)),
        Err(JournalError::OutOfOrder { .. })
    ));
    // A new private name for the same file (the first was taken) is journaled again.
    let again = Undo::Removing {
        file: file(1),
        dir_id: dir(),
        private: ".pctwin-undo-ffff".into(),
    };
    j.record_undo(&permit, a, &again).unwrap();
    j.record_undo(&permit, a, &putting(1, "f1.txt")).unwrap();
    // Each candidate name is journaled again before it is tried.
    j.record_undo(&permit, a, &putting(1, "f1 (kept).txt"))
        .unwrap();
    // Putting back never turns into a salvage or a new removal.
    assert!(matches!(
        j.record_undo(&permit, a, &salvaging(1, "x")),
        Err(JournalError::OutOfOrder { .. })
    ));
    assert!(matches!(
        j.record_undo(&permit, a, &removing(1)),
        Err(JournalError::OutOfOrder { .. })
    ));
    j.record_undo(
        &permit,
        a,
        &Undo::Done {
            outcome: UndoOutcome::KeptAt {
                at: "Docs/f1 (kept).txt".into(),
                why: "w".into(),
            },
        },
    )
    .unwrap();
}

#[test]
fn a_salvage_follows_a_removal_and_ends_done() {
    let (_d, _p, j) = journal();
    let a = committed(&j, 1);
    let permit = j.begin_undo().unwrap();
    j.record_undo(&permit, a, &removing(1)).unwrap();
    j.record_undo(&permit, a, &salvaging(1, "f1 (kept).txt"))
        .unwrap();
    assert!(matches!(
        j.record_undo(&permit, a, &putting(1, "f1.txt")),
        Err(JournalError::OutOfOrder { .. })
    ));
    j.record_undo(
        &permit,
        a,
        &Undo::Done {
            outcome: UndoOutcome::KeptAt {
                at: "Docs/f1 (kept).txt".into(),
                why: "w".into(),
            },
        },
    )
    .unwrap();
}

#[test]
fn a_final_done_is_never_written_over_but_not_done_this_time_is_tried_again() {
    let (_d, _p, j) = journal();
    let a = committed(&j, 1);
    let b = committed(&j, 2);
    let permit = j.begin_undo().unwrap();
    j.record_undo(
        &permit,
        a,
        &Undo::Done {
            outcome: UndoOutcome::Deleted,
        },
    )
    .unwrap();
    for next in [
        removing(1),
        Undo::Done {
            outcome: UndoOutcome::AlreadyGone,
        },
    ] {
        assert!(matches!(
            j.record_undo(&permit, a, &next),
            Err(JournalError::OutOfOrder { .. })
        ));
    }
    j.record_undo(
        &permit,
        b,
        &Undo::Done {
            outcome: UndoOutcome::NotDone {
                why: "in use".into(),
            },
        },
    )
    .unwrap();
    j.record_undo(&permit, b, &removing(2)).unwrap();
    assert_eq!(j.undo_of(b).unwrap(), Some(removing(2)));
}

#[test]
fn a_name_that_is_not_a_plain_name_in_the_folder_is_refused() {
    let (_d, _p, j) = journal();
    let a = committed(&j, 1);
    let permit = j.begin_undo().unwrap();
    for bad in ["", ".", "..", "x/y", "x\\y", "a\0b"] {
        let undo = Undo::Removing {
            file: file(1),
            dir_id: dir(),
            private: bad.into(),
        };
        assert!(
            matches!(
                j.record_undo(&permit, a, &undo),
                Err(JournalError::BadName(_))
            ),
            "{bad:?}"
        );
        assert!(matches!(
            j.record_removals(&permit, &[(a, undo)]),
            Err(JournalError::BadName(_))
        ));
    }
    j.record_undo(&permit, a, &removing(1)).unwrap();
    for bad in ["", "..", "a/b"] {
        assert!(matches!(
            j.record_undo(
                &permit,
                a,
                &Undo::Putting {
                    file: file(1),
                    dir_id: dir(),
                    private: ".pctwin-undo-1".into(),
                    to: bad.into()
                }
            ),
            Err(JournalError::BadName(_))
        ));
        assert!(matches!(
            j.record_undo(
                &permit,
                a,
                &Undo::Salvaging {
                    file: file(1),
                    dir_id: dir(),
                    temp: bad.into(),
                    to: "f1 (kept).txt".into()
                }
            ),
            Err(JournalError::BadName(_))
        ));
    }
    assert_eq!(j.undo_of(a).unwrap(), Some(removing(1)));
}

#[test]
fn rights_from_another_journal_are_refused_by_every_undo_write() {
    let (_d1, _p1, mine) = journal();
    let (_d2, _p2, other) = journal();
    let a = committed(&mine, 1);
    let theirs = other.begin_undo().unwrap();
    assert!(matches!(
        mine.record_undo(&theirs, a, &removing(1)),
        Err(JournalError::OtherJournal)
    ));
    assert!(matches!(
        mine.record_removals(&theirs, &[(a, removing(1))]),
        Err(JournalError::OtherJournal)
    ));
    assert!(matches!(
        mine.finish_undos(&theirs, &[(a, UndoOutcome::Deleted)]),
        Err(JournalError::OtherJournal)
    ));
    assert!(matches!(
        mine.record_folder_undo(&theirs, "me", "Docs", &UndoOutcome::Removed),
        Err(JournalError::OtherJournal)
    ));
    drop(theirs);
    // A close ticket of another journal is refused too.
    let b = committed(&other, 2);
    {
        let permit = other.begin_undo().unwrap();
        other.record_undo(&permit, b, &removing(2)).unwrap();
    }
    let mut refused = Vec::new();
    other
        .close_undo(&mut |ticket, _| {
            refused.push(matches!(
                mine.record_undo(ticket, a, &removing(1)),
                Err(JournalError::OtherJournal)
            ));
            refused.push(matches!(
                mine.record_removals(ticket, &[(a, removing(1))]),
                Err(JournalError::OtherJournal)
            ));
            refused.push(matches!(
                mine.finish_undos(ticket, &[(a, UndoOutcome::Deleted)]),
                Err(JournalError::OtherJournal)
            ));
            refused.push(matches!(
                mine.record_folder_undo(ticket, "me", "Docs", &UndoOutcome::Removed),
                Err(JournalError::OtherJournal)
            ));
            Ok(pctwin_journal::Resolved::AlreadyGone)
        })
        .unwrap();
    assert_eq!(refused, [true; 4]);
    assert_eq!(mine.undo_of(a).unwrap(), None);
    assert_eq!(mine.folder_undo_of("me", "Docs").unwrap(), None);
}
