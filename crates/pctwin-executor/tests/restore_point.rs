//! The Windows restore point as a second safety net (Task List 2.3; Product Spec, "Restore point
//! and one-click undo"): asked for once per move, before its first install or setting, with the
//! honest answer kept in the journal. Asking itself is the administrator helper's work (2.2);
//! here only the rule and the record.

mod common;
#[allow(unused_imports)]
use common::{keys, rebuild};

use common::{Faulty, Laptop, SCOPE, install, journal, plan_of, recipe, removable, run_with, set};
use pctwin_journal::{RestorePoint, Scope};
use pctwin_recipe::{Arch, Check, ChecklistKey, Os, Platform, SettingValue, Step};
use pctwin_types::ItemStatus;

#[test]
fn it_is_asked_once_before_the_first_change_and_its_answer_kept() {
    let (_d, path, j) = journal();
    let answer = RestorePoint::EarlierExists {
        at: 1_793_000_000,
        number: 12,
    };
    let mut laptop = Laptop {
        restore_answer: Some(answer.clone()),
        ..Laptop::ok()
    };
    let plan = plan_of(vec![
        removable(vec![install("A.B")]),
        recipe(vec![set("k", SettingValue::Int(1))]),
    ]);
    let run = run_with(&plan, &mut laptop, &rebuild(&j));
    assert_eq!(run.restore_point, Some(answer.clone()));
    assert_eq!(laptop.restore_asked, 1);
    assert_eq!(laptop.done[0], "restore point", "before any change");
    assert_eq!(j.restore_point(SCOPE).unwrap(), Some(answer.clone()));
    // The same move run again (after a restart): not asked again.
    drop(j);
    let j = pctwin_journal::Journal::open(&path).unwrap();
    let run = run_with(&plan, &mut laptop, &rebuild(&j));
    assert_eq!(laptop.restore_asked, 1);
    assert_eq!(run.restore_point, Some(answer));
    // Another move asks for its own.
    plan.run(&mut laptop, &rebuild(&j), Scope([9; 16]), &common::actor());
    assert_eq!(laptop.restore_asked, 2);
}

#[test]
fn every_honest_answer_is_kept_as_it_was_given() {
    for answer in [
        RestorePoint::Made {
            at: 1_793_000_000,
            number: 41,
        },
        RestorePoint::NotMade {
            why: "System Restore is turned off on this laptop".into(),
        },
        RestorePoint::NotWindows,
    ] {
        let (_d, _p, j) = journal();
        let mut laptop = Laptop {
            restore_answer: Some(answer.clone()),
            ..Laptop::ok()
        };
        let plan = plan_of(vec![recipe(vec![set("k", SettingValue::Int(1))])]);
        let run = run_with(&plan, &mut laptop, &rebuild(&j));
        // A safety net that could not be made does not stop the move: the journal is the
        // first one.
        assert_eq!(run.results[0].1.status(), ItemStatus::Converted);
        assert_eq!(j.restore_point(SCOPE).unwrap(), Some(answer));
    }
}

#[test]
fn a_move_that_changes_nothing_asks_for_none() {
    let (_d, _p, j) = journal();
    let mut laptop = Laptop::ok();
    let only_checks_and_lists = plan_of(vec![recipe(vec![
        Step::Verify {
            check: Check::SettingIs {
                key: "k".into(),
                value: SettingValue::Int(1),
            },
        },
        Step::Checklist {
            what: ChecklistKey::new("sign-in").unwrap(),
        },
    ])]);
    run_with(&only_checks_and_lists, &mut laptop, &rebuild(&j));
    let mut mac = Laptop {
        platform: Some(Platform {
            os: Os::Macos,
            arch: Arch::Aarch64,
        }),
        ..Laptop::ok()
    };
    run_with(
        &plan_of(vec![removable(vec![install("A.B")])]),
        &mut mac,
        &rebuild(&j),
    );
    assert_eq!(laptop.restore_asked + mac.restore_asked, 0);
    assert_eq!(j.restore_point(SCOPE).unwrap(), None);
}

#[test]
fn an_answer_that_cannot_be_written_down_stops_the_move_before_any_change() {
    let (_d, _p, j) = journal();
    let ledger = Faulty::new(&j, 0);
    let mut laptop = Laptop::ok();
    let run = run_with(
        &plan_of(vec![removable(vec![install("A.B")])]),
        &mut laptop,
        &ledger,
    );
    assert!(run.stopped.is_some());
    assert_eq!(laptop.done, ["restore point"]);
    assert_eq!(run.results[0].1.status(), ItemStatus::Failed);
}
