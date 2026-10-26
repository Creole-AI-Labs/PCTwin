//! Fault injection at every journal checkpoint for installs and settings (Task List 2.3; Phase 2
//! plan 2.3, section 10): the power going out, the disk filling up or the journal refusing a
//! write, between every pair of steps of a whole move. Each must end in a state recovery handles
//! honestly: nothing changed without its intent on disk, nothing installed twice, nothing
//! counted done that was not seen done, and undo afterwards puts the laptop back as it was.

mod common;

use std::collections::HashMap;

use common::{
    Faulty, Laptop, SCOPE, copy, install, journal, place, plan_of, recipe, removable, run_with, set,
};
use pctwin_executor::{RecoveredAs, recover, undo_effects};
use pctwin_journal::{
    EffectUndoOutcome, Found, ImportIntent, InstallIntent, Journal, JournalError, PlannedEffect,
    Receipt, SettingsFile,
};
use pctwin_recipe::{RecipeId, RecipeRef, SettingValue};
use pctwin_types::ItemStatus;

/// A laptop as it is before the move: one setting set, one settings file there.
fn before_move() -> Laptop {
    let mut l = Laptop::ok();
    l.settings
        .insert("theme".into(), SettingValue::Text("light".into()));
    let p = place("vlc");
    l.put_file(&p, "vlcrc", b"defaults");
    l.put_source(&p, "vlcrc", b"from the old laptop");
    l.put_source(&p, "extra.ini", b"more");
    l
}

/// A whole move: an app with its settings copied and a setting applied, and a second app.
fn the_move() -> pctwin_executor::Plan {
    plan_of(vec![
        removable(vec![
            install("A.B"),
            copy("vlc"),
            set("theme", SettingValue::Text("dark".into())),
        ]),
        removable(vec![install("C.D"), set("font", SettingValue::Int(14))]),
    ])
}

/// Journal writes in the whole move: the restore point answer, then intent, observation and
/// receipt for each of its five changes.
const WRITES: usize = 1 + 3 * 5;

fn installs(l: &Laptop) -> HashMap<String, usize> {
    let mut n = HashMap::new();
    for d in &l.done {
        if let Some(p) = d.strip_prefix("install ") {
            *n.entry(p.to_string()).or_default() += 1;
        }
    }
    n
}

fn changes(log: &[String]) -> usize {
    log.iter()
        .filter(|d| d.starts_with("install ") || d.starts_with("set ") || *d == "copy")
        .count()
}

#[test]
fn the_power_going_out_after_any_journal_write_is_recovered_honestly_and_undone_fully() {
    let start = before_move();
    for writes in 0..=WRITES {
        let (_d, path, j) = journal();
        let mut laptop = before_move();
        {
            // The journal accepts `writes` writes, then the power goes out: nothing after.
            let ledger = Faulty::new(&j, writes);
            let run = run_with(&the_move(), &mut laptop, &ledger);
            assert_eq!(run.stopped.is_some(), writes < WRITES, "writes {writes}");
            // An item is claimed done only when every one of its changes has its receipt, done.
            let effects = j.effects().unwrap();
            for (item, r) in &run.results {
                if r.status() == ItemStatus::Reinstalled {
                    let its: Vec<_> = effects.iter().filter(|e| e.planned.item == *item).collect();
                    assert!(!its.is_empty());
                    for e in its {
                        assert!(
                            matches!(
                                e.state,
                                pctwin_journal::EffectState::Receipted {
                                    receipt: Receipt::Done,
                                    ..
                                }
                            ),
                            "writes {writes}: claimed done without its receipt"
                        );
                    }
                }
            }
        }
        drop(j);
        // Restart: the journal holds exactly the steps already taken.
        let j = Journal::open(&path).unwrap();
        let recovered = recover(&j, &mut laptop).unwrap();
        for r in &recovered {
            assert!(
                matches!(r.outcome, RecoveredAs::Receipt(_)),
                "writes {writes}: {r:?}"
            );
        }
        assert!(
            j.unfinished_effects().unwrap().is_empty(),
            "writes {writes}"
        );
        // The move run again finishes, never installing anything twice.
        let run = run_with(&the_move(), &mut laptop, &j);
        assert_eq!(run.stopped, None, "writes {writes}");
        for (_, r) in &run.results {
            assert_eq!(r.status(), ItemStatus::Reinstalled, "writes {writes}");
        }
        for (pkg, n) in installs(&laptop) {
            assert_eq!(n, 1, "writes {writes}: {pkg} installed {n} times");
        }
        // Asked once per move; again only when its answer never reached the journal.
        let most = if writes == 0 { 2 } else { 1 };
        assert!(laptop.restore_asked <= most, "writes {writes}");
        assert!(j.restore_point(SCOPE).unwrap().is_some(), "writes {writes}");
        // Undo puts the laptop back as it was before the move.
        let permit = j.begin_undo().unwrap();
        for u in undo_effects(&j, &permit, &mut laptop, SCOPE).unwrap() {
            assert!(!u.outcome.needs_action(), "writes {writes}: {u:?}");
        }
        assert_eq!(laptop.settings, start.settings, "writes {writes}");
        assert_eq!(laptop.files, start.files, "writes {writes}");
        assert!(laptop.installed.is_empty(), "writes {writes}");
    }
}

#[test]
fn a_full_disk_or_a_refused_write_stops_every_change_after_it() {
    for writes in 0..WRITES {
        let (_d, _p, j) = journal();
        let ledger = Faulty::new(&j, writes);
        let mut laptop = Laptop {
            log: Some(ledger.log.clone()),
            ..before_move()
        };
        let run = run_with(&the_move(), &mut laptop, &ledger);
        assert!(run.stopped.is_some());
        let log = ledger.log.borrow();
        let refused = log
            .iter()
            .position(|e| e.ends_with("refused"))
            .expect("a write was refused");
        assert_eq!(
            changes(&log[refused..]),
            0,
            "writes {writes}: a change after the refusal: {log:?}"
        );
        // Every change made has its intent written before it.
        let intents = log[..refused].iter().filter(|e| *e == "intend").count();
        assert!(
            changes(&log[..refused]) <= intents,
            "writes {writes}: {log:?}"
        );
    }
}

#[test]
fn an_installer_still_running_is_never_started_again() {
    let (_d, path, j) = journal();
    let mut laptop = Laptop::ok();
    {
        // The power goes out after the install's intent; the installer goes on running.
        let ledger = Faulty::new(&j, 2);
        run_with(
            &plan_of(vec![removable(vec![install("A.B")])]),
            &mut laptop,
            &ledger,
        );
    }
    drop(j);
    let j = Journal::open(&path).unwrap();
    laptop.installed.clear();
    laptop.installing.insert("A.B".into());
    laptop.done.clear();
    let run = run_with(
        &plan_of(vec![removable(vec![install("A.B")])]),
        &mut laptop,
        &j,
    );
    assert!(matches!(
        &run.recovered[0].outcome,
        RecoveredAs::Left(why) if why == pctwin_executor::STILL_INSTALLING
    ));
    assert_eq!(run.results[0].1.status(), ItemStatus::Failed);
    assert!(installs(&laptop).is_empty(), "{:?}", laptop.done);
    // If whether it runs cannot be told, it is not started either.
    laptop.installing.clear();
    laptop.running_unknown = true;
    let run = run_with(
        &plan_of(vec![removable(vec![install("A.B")])]),
        &mut laptop,
        &j,
    );
    assert!(matches!(run.recovered[0].outcome, RecoveredAs::Left(_)));
    assert!(installs(&laptop).is_empty());
    // Once it has finished, recovery finds it, and running again installs nothing.
    laptop.running_unknown = false;
    laptop.installed.insert("A.B".into(), "1.0".into());
    let run = run_with(
        &plan_of(vec![removable(vec![install("A.B")])]),
        &mut laptop,
        &j,
    );
    assert_eq!(
        run.recovered[0].outcome,
        RecoveredAs::Receipt(Receipt::Done)
    );
    assert_eq!(run.results[0].1.status(), ItemStatus::Reinstalled);
    assert!(installs(&laptop).is_empty());
}

#[test]
fn a_late_step_from_a_replaced_worker_is_refused() {
    let (_d, path, j) = journal();
    let mut laptop = Laptop::ok();
    {
        let ledger = Faulty::new(&j, 2);
        run_with(
            &plan_of(vec![removable(vec![install("A.B")])]),
            &mut laptop,
            &ledger,
        );
    }
    drop(j);
    let j = Journal::open(&path).unwrap();
    let got = recover(&j, &mut laptop).unwrap();
    let id = got[0].id;
    // The old worker comes back with its own observation and receipt: refused, nothing changed.
    let before = j.effect(id).unwrap();
    assert!(matches!(
        j.installs()
            .observed(id, &Found::App(pctwin_journal::Presence::Absent), false),
        Err(JournalError::OutOfOrder { .. })
    ));
    assert!(matches!(
        j.installs().receipt(
            id,
            &Receipt::Failed {
                reason: pctwin_types::Reason::InstallFailed
            }
        ),
        Err(JournalError::OutOfOrder { .. })
    ));
    assert_eq!(j.effect(id).unwrap(), before);
}

#[test]
fn putting_back_settings_files_stopped_part_way_is_finished_by_looking_again() {
    let (_d, path, j) = journal();
    let mut laptop = before_move();
    let p = place("vlc");
    run_with(&plan_of(vec![recipe(vec![copy("vlc")])]), &mut laptop, &j);
    let id = j.effects().unwrap()[0].id;
    // Undo began: its record is on disk and one of two files was put back; then the power went.
    {
        let permit = j.begin_undo().unwrap();
        j.settings()
            .record_undo(&permit, id, &pctwin_journal::EffectUndo::Reversing)
            .unwrap();
    }
    laptop.put_file(&p, "vlcrc", b"defaults");
    drop(j);
    let j = Journal::open(&path).unwrap();
    let permit = j.begin_undo().unwrap();
    let got = undo_effects(&j, &permit, &mut laptop, SCOPE).unwrap();
    assert_eq!(got[0].outcome, EffectUndoOutcome::Restored);
    assert_eq!(laptop.file(&p, "vlcrc").unwrap(), b"defaults");
    assert_eq!(laptop.file(&p, "extra.ini"), None);
}

#[test]
fn a_putting_back_that_fails_part_way_keeps_exactly_what_is_left_for_next_time() {
    let (_d, _p, j) = journal();
    let mut laptop = before_move();
    let p = place("vlc");
    run_with(&plan_of(vec![recipe(vec![copy("vlc")])]), &mut laptop, &j);

    laptop.puts_before_failing = Some(1);
    let permit = j.begin_undo().unwrap();
    let got = undo_effects(&j, &permit, &mut laptop, SCOPE).unwrap();
    assert!(matches!(got[0].outcome, EffectUndoOutcome::NotDone { .. }));
    assert_eq!(
        laptop
            .done
            .iter()
            .filter(|d| d.starts_with("restore file"))
            .count(),
        2,
        "one put back, one failed"
    );
    // Next time, what is left is finished; what was already put back is left as it is.
    laptop.puts_before_failing = None;
    laptop.done.clear();
    let got = undo_effects(&j, &permit, &mut laptop, SCOPE).unwrap();
    assert_eq!(got[0].outcome, EffectUndoOutcome::Restored);
    assert_eq!(
        laptop
            .done
            .iter()
            .filter(|d| d.starts_with("restore file"))
            .count(),
        1,
        "only what was left: {:?}",
        laptop.done
    );
    assert_eq!(laptop.file(&p, "vlcrc").unwrap(), b"defaults");
    assert_eq!(laptop.file(&p, "extra.ini"), None);
}

#[test]
fn an_earlier_version_whose_bytes_are_missing_is_never_promised_back() {
    let (_d, path, j) = journal();
    let mut laptop = before_move();
    let p = place("vlc");
    run_with(&plan_of(vec![recipe(vec![copy("vlc")])]), &mut laptop, &j);
    drop(j);
    {
        let db = redb::Database::open(&path).unwrap();
        let tx = db.begin_write().unwrap();
        tx.open_table(redb::TableDefinition::<[u8; 32], &[u8]>::new("kept-files"))
            .unwrap()
            .remove(pctwin_journal::content_hash(b"defaults"))
            .unwrap();
        tx.commit().unwrap();
    }
    let j = Journal::open(&path).unwrap();
    let permit = j.begin_undo().unwrap();
    laptop.done.clear();
    let got = undo_effects(&j, &permit, &mut laptop, SCOPE).unwrap();
    assert!(matches!(&got[0].outcome, EffectUndoOutcome::Kept { .. }));
    assert_eq!(laptop.file(&p, "vlcrc").unwrap(), b"from the old laptop");
    assert!(laptop.done.is_empty(), "nothing was changed");
}

fn planned() -> PlannedEffect {
    PlannedEffect {
        scope: SCOPE,
        item: plan_of(vec![recipe(vec![set("k", SettingValue::Int(1))])]).items()[0].item,
        recipe: RecipeRef {
            id: RecipeId::new("x").unwrap(),
            version: 1,
        },
        step: 0,
        actor: common::actor(),
    }
}

#[test]
fn a_licence_or_service_change_is_never_claimed_undone_here() {
    let (_d, _p, j) = journal();
    let id = j
        .installs()
        .intend(
            &planned(),
            &InstallIntent::Licence {
                identity: "activation 2291".into(),
            },
        )
        .unwrap();
    j.installs()
        .observed(id, &Found::External { happened: true }, false)
        .unwrap();
    j.installs().receipt(id, &Receipt::Done).unwrap();
    let mut laptop = Laptop::ok();
    let permit = j.begin_undo().unwrap();
    let got = undo_effects(&j, &permit, &mut laptop, SCOPE).unwrap();
    assert!(
        matches!(&got[0].outcome, EffectUndoOutcome::Kept { why } if why.contains("only they"))
    );
    assert!(laptop.done.is_empty());
}

#[test]
fn imports_belong_to_their_own_role_and_are_left_to_it() {
    let (_d, _p, j) = journal();
    let id = j
        .imports()
        .intend(
            &planned(),
            &ImportIntent::Files {
                to: place("Mail"),
                before: vec![SettingsFile {
                    path: "inbox".into(),
                    bytes: b"x".to_vec(),
                }],
            },
        )
        .unwrap();
    let mut laptop = Laptop::ok();
    // Recovery here does not look at it, and undo here does not touch it.
    assert!(recover(&j, &mut laptop).unwrap().is_empty());
    assert_eq!(j.unfinished_effects().unwrap().len(), 1);
    let permit = j.begin_undo().unwrap();
    assert!(
        undo_effects(&j, &permit, &mut laptop, SCOPE)
            .unwrap()
            .is_empty()
    );
    assert_eq!(j.effect_undo_of(id).unwrap(), None);
    // And a change to the same place waits until the import is accounted for.
    laptop.put_source(&place("Mail"), "inbox", b"y");
    let run = run_with(&plan_of(vec![recipe(vec![copy("Mail")])]), &mut laptop, &j);
    assert_eq!(run.results[0].1.status(), ItemStatus::Failed);
    assert!(!laptop.done.contains(&"copy".to_string()));
}
