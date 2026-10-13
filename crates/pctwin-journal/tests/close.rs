//! Closing undo, complete by type (Security Design B, revised 9 October 2026): closing first
//! finishes every removal a crash or stop left part-way and records the close in the same step;
//! undo cannot close while any file it moved is still under a hidden name; anything that cannot be
//! finished leaves undo open, the wipe blocked, and nothing closed.

mod common;

use std::sync::{Arc, mpsc};
use std::time::Duration;

use common::*;
use pctwin_journal::{
    CloseError, Journal, JournalError, Pending, Resolved, Undo, UndoGate, UndoOutcome,
};

const LONG: Duration = Duration::from_secs(30);

fn done(outcome: UndoOutcome) -> Option<Undo> {
    Some(Undo::Done { outcome })
}

/// Records `stages` for fresh committed writes 1, 2, ...; returns their entry numbers.
fn with_stages(j: &Journal, stages: &[Vec<Undo>]) -> Vec<u64> {
    let permit = j.begin_undo().unwrap();
    stages
        .iter()
        .zip(1u8..)
        .map(|(steps, n)| {
            let id = committed(j, n);
            for s in steps {
                j.record_undo(&permit, id, s).unwrap();
            }
            id
        })
        .collect()
}

/// No stage that is not Done is on disk while the gate says closed: read from a fresh open.
fn closed_never_beside_a_hidden_stage(path: &std::path::Path, ids: &[u64]) {
    let j = Journal::open(path).unwrap();
    if j.undo_gate().unwrap() == UndoGate::Closed {
        for id in ids {
            if let Some(u) = j.undo_of(*id).unwrap() {
                assert!(u.is_done(), "closed beside {u:?}");
            }
        }
    }
}

#[test]
fn with_nothing_pending_close_asks_nothing_and_closes() {
    let (_d, _p, j) = journal();
    committed(&j, 1);
    let mut asked = 0;
    let token = j
        .close_undo(&mut |_, _| {
            asked += 1;
            Ok(Resolved::Removed)
        })
        .unwrap();
    assert_eq!(asked, 0);
    assert!(token.is_for(&j));
    assert_eq!(j.undo_gate().unwrap(), UndoGate::Closed);
}

#[test]
fn close_resolves_every_part_way_removal_and_records_them_with_the_close() {
    let (_d, path, j) = journal();
    let ids = with_stages(
        &j,
        &[
            vec![removing(1)],
            vec![removing(2), putting(2, "f2.txt")],
            vec![removing(3), salvaging(3, "f3 (kept).txt")],
            vec![
                removing(4),
                Undo::Done {
                    outcome: UndoOutcome::Deleted,
                },
            ],
        ],
    );
    let mut seen: Vec<Pending> = Vec::new();
    let token = j
        .close_undo(&mut |_, p| {
            seen.push(p.clone());
            Ok(match p.stage {
                Undo::Removing { .. } => Resolved::Removed,
                Undo::Putting { .. } => Resolved::Kept {
                    why: "put back".into(),
                },
                _ => Resolved::KeptAt {
                    at: "Docs/f3 (kept).txt".into(),
                    why: "may be incomplete".into(),
                },
            })
        })
        .unwrap();
    assert!(token.is_for(&j));
    // Only the three not done, each with its stage, destination and stored path.
    assert_eq!(
        seen,
        vec![
            Pending {
                id: ids[0],
                stage: removing(1),
                destination: "me".into(),
                path: "Docs/f1.txt".into(),
            },
            Pending {
                id: ids[1],
                stage: putting(2, "f2.txt"),
                destination: "me".into(),
                path: "Docs/f2.txt".into(),
            },
            Pending {
                id: ids[2],
                stage: salvaging(3, "f3 (kept).txt"),
                destination: "me".into(),
                path: "Docs/f3.txt".into(),
            },
        ]
    );
    drop(j);
    let j = Journal::open(&path).unwrap();
    assert_eq!(j.undo_gate().unwrap(), UndoGate::Closed);
    assert_eq!(j.undo_of(ids[0]).unwrap(), done(UndoOutcome::Deleted));
    assert_eq!(
        j.undo_of(ids[1]).unwrap(),
        done(UndoOutcome::Kept {
            why: "put back".into()
        })
    );
    assert_eq!(
        j.undo_of(ids[2]).unwrap(),
        done(UndoOutcome::KeptAt {
            at: "Docs/f3 (kept).txt".into(),
            why: "may be incomplete".into()
        })
    );
    assert_eq!(j.undo_of(ids[3]).unwrap(), done(UndoOutcome::Deleted));
}

#[test]
fn each_resolution_becomes_its_own_outcome() {
    let cases = [
        (Resolved::Removed, UndoOutcome::Deleted),
        (Resolved::AlreadyGone, UndoOutcome::AlreadyGone),
        (
            Resolved::Kept { why: "k".into() },
            UndoOutcome::Kept { why: "k".into() },
        ),
        (
            Resolved::KeptAt {
                at: "Docs/x".into(),
                why: "k".into(),
            },
            UndoOutcome::KeptAt {
                at: "Docs/x".into(),
                why: "k".into(),
            },
        ),
    ];
    for (resolved, outcome) in cases {
        let (_d, _p, j) = journal();
        let ids = with_stages(&j, &[vec![removing(1)]]);
        let mut once = Some(resolved);
        j.close_undo(&mut |_, _| Ok(once.take().unwrap())).unwrap();
        assert_eq!(j.undo_of(ids[0]).unwrap(), done(outcome));
    }
}

#[test]
fn one_unresolved_file_closes_nothing_writes_nothing_and_undo_carries_on() {
    let (_d, _p, j) = journal();
    let ids = with_stages(
        &j,
        &[vec![removing(1)], vec![removing(2)], vec![removing(3)]],
    );
    let err = j
        .close_undo(&mut |_, p| {
            Ok(if p.id == ids[1] {
                Resolved::Unresolved {
                    why: "the drive is not there".into(),
                }
            } else {
                Resolved::Removed
            })
        })
        .unwrap_err();
    let CloseError::Unresolved { items } = err else {
        panic!("{err:?}");
    };
    assert_eq!(items.len(), 1);
    assert_eq!(items[0].id, ids[1]);
    assert_eq!(items[0].destination, "me");
    assert_eq!(items[0].path, "Docs/f2.txt");
    assert_eq!(items[0].why, "the drive is not there");
    assert_eq!(j.undo_gate().unwrap(), UndoGate::Open);
    // Nothing written: the others are still as they were (resolving again is safe).
    for (id, n) in ids.iter().zip(1u8..) {
        assert_eq!(j.undo_of(*id).unwrap(), Some(removing(n)));
    }
    // Undo carries on.
    let permit = j.begin_undo().unwrap();
    drop(permit);
    // Once it can be resolved, closing goes ahead.
    j.close_undo(&mut |_, _| Ok(Resolved::AlreadyGone)).unwrap();
    assert_eq!(j.undo_gate().unwrap(), UndoGate::Closed);
}

#[test]
fn an_error_from_the_resolver_is_unresolved_too() {
    let (_d, _p, j) = journal();
    let ids = with_stages(&j, &[vec![removing(1)], vec![removing(2)]]);
    let err = j
        .close_undo(&mut |_, p| {
            if p.id == ids[0] {
                Err(std::io::Error::other("input/output error"))
            } else {
                Ok(Resolved::Removed)
            }
        })
        .unwrap_err();
    let CloseError::Unresolved { items } = err else {
        panic!("{err:?}");
    };
    assert_eq!(items.len(), 1);
    assert_eq!(items[0].id, ids[0]);
    assert!(items[0].why.contains("input/output error"));
    assert_eq!(j.undo_gate().unwrap(), UndoGate::Open);
    assert_eq!(j.undo_of(ids[1]).unwrap(), Some(removing(2)));
}

#[test]
fn every_unresolved_file_is_listed_not_just_the_first() {
    let (_d, _p, j) = journal();
    let ids = with_stages(&j, &[vec![removing(1)], vec![removing(2)]]);
    let Err(CloseError::Unresolved { items }) =
        j.close_undo(&mut |_, _| Ok(Resolved::Unresolved { why: "gone".into() }))
    else {
        panic!("closed");
    };
    let listed: Vec<u64> = items.iter().map(|i| i.id).collect();
    assert_eq!(listed, ids);
}

#[test]
fn while_close_resolves_no_permit_is_handed_out_and_afterwards_one_is_again() {
    let (_d, _p, j) = journal();
    with_stages(&j, &[vec![removing(1)]]);
    let mut during = None;
    let _ = j.close_undo(&mut |_, _| {
        during = Some(matches!(j.begin_undo(), Err(JournalError::UndoClosed)));
        Ok(Resolved::Unresolved { why: "x".into() })
    });
    assert_eq!(during, Some(true));
    assert!(j.begin_undo().is_ok());
}

#[test]
fn the_resolver_journals_its_own_steps_with_the_close_ticket() {
    let (_d, _p, j) = journal();
    let ids = with_stages(&j, &[vec![removing(1)]]);
    j.close_undo(&mut |ticket, p| {
        // Put back, under a visible name beside it: journaled before the rename.
        j.record_undo(ticket, p.id, &putting(1, "f1 (kept).txt"))
            .map_err(std::io::Error::other)?;
        Ok(Resolved::KeptAt {
            at: "Docs/f1 (kept).txt".into(),
            why: "another file had its name".into(),
        })
    })
    .unwrap();
    assert_eq!(
        j.undo_of(ids[0]).unwrap(),
        done(UndoOutcome::KeptAt {
            at: "Docs/f1 (kept).txt".into(),
            why: "another file had its name".into()
        })
    );
}

#[test]
fn a_stage_started_during_close_that_was_never_resolved_blocks_the_close() {
    let (_d, path, j) = journal();
    let ids = with_stages(&j, &[vec![removing(1)]]);
    let other = committed(&j, 2);
    let err = j
        .close_undo(&mut |ticket, _| {
            // Something starts a new removal while closing: it is not in what was resolved.
            j.record_undo(ticket, other, &removing(2))
                .map_err(std::io::Error::other)?;
            Ok(Resolved::Removed)
        })
        .unwrap_err();
    let CloseError::Unresolved { items } = err else {
        panic!("{err:?}");
    };
    assert_eq!(items.iter().map(|i| i.id).collect::<Vec<_>>(), [other]);
    assert_eq!(j.undo_gate().unwrap(), UndoGate::Open);
    // The resolved one was not recorded either: that transaction never committed.
    assert_eq!(j.undo_of(ids[0]).unwrap(), Some(removing(1)));
    drop(j);
    closed_never_beside_a_hidden_stage(&path, &[ids[0], other]);
}

#[test]
fn a_resolver_that_panics_leaves_undo_open_and_closing_can_be_tried_again() {
    let (_d, _p, j) = journal();
    with_stages(&j, &[vec![removing(1)]]);
    let r = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
        let _ = j.close_undo(&mut |_, _| panic!("resolver crashed"));
    }));
    assert!(r.is_err());
    assert_eq!(j.undo_gate().unwrap(), UndoGate::Open);
    assert!(j.begin_undo().is_ok());
    j.close_undo(&mut |_, _| Ok(Resolved::Removed)).unwrap();
    assert_eq!(j.undo_gate().unwrap(), UndoGate::Closed);
}

#[test]
fn already_closed_gives_a_token_again_without_asking() {
    let (_d, path, j) = journal();
    j.close_undo(&mut |_, _| Ok(Resolved::Removed)).unwrap();
    drop(j);
    let j = Journal::open(&path).unwrap();
    let mut asked = false;
    let token = j
        .close_undo(&mut |_, _| {
            asked = true;
            Ok(Resolved::Removed)
        })
        .unwrap();
    assert!(!asked);
    assert!(token.is_for(&j));
}

#[test]
fn a_permit_held_past_the_deadline_is_busy_never_a_hang() {
    let (_d, _p, j) = journal();
    let held = j.begin_undo().unwrap();
    let started = std::time::Instant::now();
    let mut asked = false;
    let r = j.close_undo_within(Duration::from_millis(200), &mut |_, _| {
        asked = true;
        Ok(Resolved::Removed)
    });
    assert!(matches!(r, Err(CloseError::Busy)), "{r:?}");
    assert!(started.elapsed() < LONG);
    assert!(!asked);
    assert_eq!(j.undo_gate().unwrap(), UndoGate::Open);
    // Undo carries on: the held one and new ones.
    assert!(j.begin_undo().is_ok());
    drop(held);
    j.close_undo(&mut |_, _| Ok(Resolved::Removed)).unwrap();
}

#[test]
fn a_permit_given_back_before_the_deadline_lets_close_go_ahead() {
    let (_d, _p, j) = journal();
    let id = committed(&j, 1);
    let j = Arc::new(j);
    let (taken_tx, taken_rx) = mpsc::channel::<()>();
    let undoer = {
        let j = Arc::clone(&j);
        std::thread::spawn(move || {
            let permit = j.begin_undo().unwrap();
            j.record_undo(&permit, id, &removing(1)).unwrap();
            taken_tx.send(()).unwrap();
            std::thread::sleep(Duration::from_millis(200));
            // Still allowed to finish this file: closing waits for it.
            j.finish_undos(&permit, &[(id, UndoOutcome::Deleted)])
                .unwrap();
        })
    };
    taken_rx.recv_timeout(LONG).unwrap();
    let mut asked = 0;
    j.close_undo_within(LONG, &mut |_, _| {
        asked += 1;
        Ok(Resolved::Removed)
    })
    .unwrap();
    undoer.join().unwrap();
    assert_eq!(asked, 0, "the file finished before close looked");
    assert_eq!(j.undo_of(id).unwrap(), done(UndoOutcome::Deleted));
}

#[test]
fn a_second_close_while_one_is_resolving_is_busy() {
    let (_d, _p, j) = journal();
    with_stages(&j, &[vec![removing(1)]]);
    let j = Arc::new(j);
    let (in_tx, in_rx) = mpsc::channel::<()>();
    let (go_tx, go_rx) = mpsc::channel::<()>();
    let first = {
        let j = Arc::clone(&j);
        std::thread::spawn(move || {
            j.close_undo(&mut |_, _| {
                in_tx.send(()).unwrap();
                go_rx.recv().unwrap();
                Ok(Resolved::Removed)
            })
            .map(|t| t.is_for(&j))
        })
    };
    in_rx.recv_timeout(LONG).unwrap();
    assert!(matches!(
        j.close_undo(&mut |_, _| Ok(Resolved::Removed)),
        Err(CloseError::Busy)
    ));
    go_tx.send(()).unwrap();
    assert!(first.join().unwrap().unwrap());
    assert_eq!(j.undo_gate().unwrap(), UndoGate::Closed);
}

/// Many random mixes of stages and resolutions: whatever happens, Closed is never on disk beside
/// a stage that is not Done, and a close that succeeds leaves every stage Done.
#[test]
fn closed_is_never_on_disk_beside_a_stage_that_is_not_done() {
    let mut seed: u64 = 0x9e37_79b9_7f4a_7c15;
    let mut next = move |n: u64| {
        seed ^= seed << 13;
        seed ^= seed >> 7;
        seed ^= seed << 17;
        seed % n
    };
    let (mut closed, mut refused) = (0, 0);
    for _ in 0..40 {
        let (_d, path, j) = journal();
        let count = 1 + next(5) as u8;
        let stages: Vec<Vec<Undo>> = (1..=count)
            .map(|n| match next(4) {
                0 => vec![removing(n)],
                1 => vec![removing(n), putting(n, "x")],
                2 => vec![removing(n), salvaging(n, "y")],
                _ => vec![],
            })
            .collect();
        let ids = with_stages(&j, &stages);
        let plan: Vec<u64> = (0..count).map(|_| next(6)).collect();
        let mut k = 0;
        let r = j.close_undo(&mut |_, _| {
            let pick = plan[k % plan.len()];
            k += 1;
            match pick {
                0 => Err(std::io::Error::other("no drive")),
                1 => Ok(Resolved::Unresolved { why: "u".into() }),
                2 => Ok(Resolved::AlreadyGone),
                3 => Ok(Resolved::Kept { why: "k".into() }),
                4 => Ok(Resolved::KeptAt {
                    at: "Docs/z".into(),
                    why: "k".into(),
                }),
                _ => Ok(Resolved::Removed),
            }
        });
        let gate = j.undo_gate().unwrap();
        assert_eq!(r.is_ok(), gate == UndoGate::Closed, "{r:?}");
        if r.is_ok() {
            closed += 1
        } else {
            refused += 1
        }
        drop(j);
        closed_never_beside_a_hidden_stage(&path, &ids);
    }
    assert!(
        closed > 0 && refused > 0,
        "closed {closed}, refused {refused}"
    );
}
