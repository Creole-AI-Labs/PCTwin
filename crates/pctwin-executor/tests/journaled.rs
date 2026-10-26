//! Every changing step is journaled in the order System Design S14 sets (intent durable, effect,
//! effect observed, receipt durable), and a change a crash interrupted is reconciled by asking
//! the laptop what is there now: an installer is never run again blindly, and nothing is counted
//! done that was not seen done.

mod common;
#[allow(unused_imports)]
use common::{keys, rebuild};

use common::{
    Faulty, Laptop, SCOPE, copy, install, journal, place, plan_of, removable, run_with, set,
};
use pctwin_executor::{RecoveredAs, recover};
use pctwin_journal::{
    EffectLedger, EffectState, Found, InstallIntent, Journal, KeptFile, PlannedEffect, Presence,
    Receipt, SettingsIntent, StoredEffect, content_hash,
};
use pctwin_recipe::{Catalog, RecipeId, RecipeRef, SettingValue};
use pctwin_types::{ItemStatus, Reason};

#[test]
fn an_install_is_recorded_before_it_runs_and_observed_after() {
    let (_d, _p, j) = journal();
    let ledger = Faulty::new(&j, usize::MAX);
    let mut laptop = Laptop {
        log: Some(ledger.log.clone()),
        ..Laptop::ok()
    };
    let plan = plan_of(vec![removable(vec![install("VideoLAN.VLC")])]);
    let run = run_with(&plan, &mut laptop, &ledger);
    assert_eq!(run.results[0].1.status(), ItemStatus::Reinstalled);
    assert_eq!(run.stopped, None);
    assert_eq!(
        *ledger.log.borrow(),
        [
            "restore point",
            "restore-point",
            "intend",
            "install VideoLAN.VLC",
            "observed",
            "check",
            "receipt"
        ]
    );
    let e = &j.effects_of(SCOPE).unwrap()[0];
    assert_eq!(e.planned.step, 0);
    assert!(e.planned.recipe.id.as_str().starts_with('x'));
    let StoredEffect::Install { before, undo, .. } = &e.effect else {
        panic!("not an install");
    };
    assert_eq!(*before, Presence::Absent);
    assert!(undo.removable(), "the recipe's declaration is kept with it");
    assert_eq!(
        e.state,
        EffectState::Receipted {
            found: Found::App(Presence::Present {
                version: Some("1.0".into())
            }),
            after_restart: false,
            receipt: Receipt::Done,
        }
    );
}

#[test]
fn an_app_already_there_is_never_installed_again_and_is_recorded_as_there_before() {
    let (_d, _p, j) = journal();
    let mut laptop = Laptop::ok();
    laptop.installed.insert("VideoLAN.VLC".into(), "3.0".into());
    let plan = plan_of(vec![removable(vec![install("VideoLAN.VLC")])]);
    let run = run_with(&plan, &mut laptop, &rebuild(&j));
    assert_eq!(run.results[0].1.status(), ItemStatus::Reinstalled);
    assert!(!laptop.done.iter().any(|d| d.starts_with("install")));
    let StoredEffect::Install { before, .. } = &j.effects().unwrap()[0].effect else {
        panic!("not an install");
    };
    assert_eq!(
        *before,
        Presence::Present {
            version: Some("3.0".into())
        }
    );
}

#[test]
fn a_setting_keeps_its_value_from_before_or_not_set() {
    let (_d, _p, j) = journal();
    let mut laptop = Laptop::ok();
    laptop
        .settings
        .insert("theme".into(), SettingValue::Text("light".into()));
    let plan = plan_of(vec![common::recipe(vec![
        set("theme", SettingValue::Text("dark".into())),
        set("font", SettingValue::Int(14)),
    ])]);
    let run = run_with(&plan, &mut laptop, &rebuild(&j));
    assert_eq!(run.results[0].1.status(), ItemStatus::Converted);
    let effects = j.effects().unwrap();
    assert!(matches!(
        &effects[0].effect,
        StoredEffect::ApplySetting { before: Some(SettingValue::Text(t)), .. } if t == "light"
    ));
    assert!(matches!(
        &effects[1].effect,
        StoredEffect::ApplySetting { before: None, .. }
    ));
    assert_eq!(effects[1].planned.step, 1);
}

#[test]
fn a_settings_copy_keeps_every_file_that_was_there_whole_and_records_what_it_wrote() {
    let (_d, _p, j) = journal();
    let mut laptop = Laptop::ok();
    let p = place("vlc");
    laptop.put_file(&p, "vlcrc", b"defaults");
    laptop.put_source(&p, "vlcrc", b"old laptop");
    laptop.put_source(&p, "extra.ini", b"more");
    let plan = plan_of(vec![common::recipe(vec![copy("vlc")])]);
    assert_eq!(
        run_with(&plan, &mut laptop, &rebuild(&j)).results[0]
            .1
            .status(),
        ItemStatus::Converted
    );
    let e = &j.effects().unwrap()[0];
    let StoredEffect::CopySettings { before, .. } = &e.effect else {
        panic!("not a copy");
    };
    assert_eq!(
        *before,
        vec![KeptFile {
            path: "vlcrc".into(),
            hash: content_hash(b"defaults"),
            size: 8,
        }]
    );
    assert_eq!(j.kept_bytes(&before[0].hash).unwrap(), b"defaults");
    let EffectState::Receipted {
        found: Found::Files(files),
        ..
    } = &e.state
    else {
        panic!("not receipted");
    };
    let mut written: Vec<(&str, [u8; 32])> =
        files.iter().map(|f| (f.path.as_str(), f.hash)).collect();
    written.sort();
    assert_eq!(
        written,
        [
            ("extra.ini", content_hash(b"more")),
            ("vlcrc", content_hash(b"old laptop"))
        ]
    );
}

#[test]
fn nothing_is_changed_when_what_is_there_cannot_be_read_first() {
    for blind in ["app", "setting", "files"] {
        let (_d, _p, j) = journal();
        let mut laptop = Laptop::ok();
        laptop.blind.insert(blind);
        let plan = plan_of(vec![common::recipe(vec![
            install("A.B"),
            set("k", SettingValue::Bool(true)),
            copy("c"),
        ])]);
        let plan = match blind {
            "app" => plan,
            "setting" => plan_of(vec![common::recipe(vec![set(
                "k",
                SettingValue::Bool(true),
            )])]),
            _ => plan_of(vec![common::recipe(vec![copy("c")])]),
        };
        let run = run_with(&plan, &mut laptop, &rebuild(&j));
        assert_eq!(run.results[0].1.reason(), Some(Reason::NotVerified));
        assert!(laptop.done.is_empty(), "{blind}: {:?}", laptop.done);
        assert!(j.effects().unwrap().is_empty());
        assert_eq!(run.stopped, None, "the journal itself is fine");
    }
}

#[test]
fn settings_too_large_to_keep_stop_the_copy_before_it_changes_anything() {
    let (_d, _p, j) = journal();
    let mut laptop = Laptop::ok();
    let p = place("big");
    laptop.put_file(&p, "data", &vec![0u8; pctwin_journal::MAX_KEPT_FILE + 1]);
    laptop.put_source(&p, "data", b"x");
    let plan = plan_of(vec![common::recipe(vec![copy("big")])]);
    let run = run_with(&plan, &mut laptop, &rebuild(&j));
    assert_eq!(run.results[0].1.reason(), Some(Reason::NotVerified));
    assert_eq!(laptop.done, ["restore point"], "nothing else changed");
    assert_eq!(run.stopped, None);
}

#[test]
fn when_the_journal_cannot_be_written_nothing_more_is_changed_and_the_run_says_so() {
    // Writes allowed before the journal fails: 0 (the restore point answer), 1 (the first
    // intent), 2 (its observation), 3 (its receipt).
    for writes in 0..4 {
        let (_d, _p, j) = journal();
        let ledger = Faulty::new(&j, writes);
        let mut laptop = Laptop::ok();
        let plan = plan_of(vec![
            removable(vec![install("A.B")]),
            removable(vec![install("C.D")]),
        ]);
        let run = run_with(&plan, &mut laptop, &ledger);
        assert_eq!(run.stopped.as_deref(), Some(pctwin_executor::STOPPED));
        assert_eq!(run.results.len(), 2, "every item has a result");
        for (_, r) in &run.results {
            assert_eq!(r.status(), ItemStatus::Failed, "never claimed done");
        }
        let installs: Vec<&String> = laptop
            .done
            .iter()
            .filter(|d| d.starts_with("install"))
            .collect();
        if writes <= 1 {
            assert!(installs.is_empty(), "no intent, no change");
        } else {
            assert_eq!(installs, ["install A.B"], "the second app is never started");
        }
    }
}

fn planned(step: u32) -> PlannedEffect {
    PlannedEffect {
        scope: SCOPE,
        item: plan_of(vec![removable(vec![install("A.B")])]).items()[0].item,
        recipe: RecipeRef {
            id: RecipeId::new("x").unwrap(),
            version: 1,
        },
        step,
        actor: common::actor(),
    }
}

fn intend_install(j: &Journal) -> u64 {
    j.installs(&keys().install)
        .intend(
            &planned(0),
            &InstallIntent::App {
                catalog: Catalog::Winget,
                package: "A.B".into(),
                before: Presence::Absent,
                undo: Default::default(),
            },
        )
        .unwrap()
}

#[test]
fn after_a_crash_an_install_that_happened_is_found_and_never_run_again() {
    let (_d, path, j) = journal();
    let id = intend_install(&j);
    drop(j); // the power went out after the install ran
    let j = common::open(&path).unwrap();
    let mut laptop = Laptop::ok();
    laptop.installed.insert("A.B".into(), "1.0".into());
    let got = recover(&rebuild(&j), &mut laptop).unwrap();
    assert_eq!(got.len(), 1);
    assert_eq!(got[0].outcome, RecoveredAs::Receipt(Receipt::Done));
    assert!(!laptop.done.iter().any(|d| d.starts_with("install")));
    assert!(matches!(
        j.effect(id).unwrap().unwrap().state,
        EffectState::Receipted {
            after_restart: true,
            receipt: Receipt::Done,
            ..
        }
    ));
    assert!(j.unfinished_effects().unwrap().is_empty());
}

#[test]
fn after_a_crash_an_install_that_did_not_happen_fails_and_is_not_run_again() {
    let (_d, path, j) = journal();
    let id = intend_install(&j);
    drop(j);
    let j = common::open(&path).unwrap();
    let mut laptop = Laptop::ok();
    let got = recover(&rebuild(&j), &mut laptop).unwrap();
    assert_eq!(
        got[0].outcome,
        RecoveredAs::Receipt(Receipt::Failed {
            reason: Reason::NotVerified
        })
    );
    assert!(laptop.done.iter().all(|d| d == "check"));
    assert!(matches!(
        j.effect(id).unwrap().unwrap().state,
        EffectState::Receipted {
            after_restart: true,
            ..
        }
    ));
}

#[test]
fn after_a_crash_what_cannot_be_looked_at_is_left_for_next_time() {
    let (_d, path, j) = journal();
    let id = intend_install(&j);
    drop(j);
    let j = common::open(&path).unwrap();
    let mut laptop = Laptop::ok();
    laptop.blind.insert("app");
    let got = recover(&rebuild(&j), &mut laptop).unwrap();
    assert!(matches!(&got[0].outcome, RecoveredAs::Left(why) if !why.is_empty()));
    assert_eq!(j.effect(id).unwrap().unwrap().state, EffectState::Intended);
    // Next time it can be looked at.
    laptop.blind.clear();
    laptop.installed.insert("A.B".into(), "1.0".into());
    let got = recover(&rebuild(&j), &mut laptop).unwrap();
    assert_eq!(got[0].outcome, RecoveredAs::Receipt(Receipt::Done));
    assert!(
        recover(&rebuild(&j), &mut laptop).unwrap().is_empty(),
        "done once"
    );
}

#[test]
fn after_a_crash_an_observation_without_a_receipt_gets_its_receipt_from_what_was_seen() {
    let (_d, path, j) = journal();
    let id = j
        .settings(&keys().setting)
        .intend(
            &planned(0),
            &SettingsIntent::Apply {
                key: "k".into(),
                before: None,
                value: SettingValue::Bool(true),
            },
        )
        .unwrap();
    j.settings(&keys().setting)
        .observed(id, &Found::Setting(Some(SettingValue::Bool(false))), false)
        .unwrap();
    drop(j);
    let j = common::open(&path).unwrap();
    // Whatever the laptop says now, the recorded observation decides: it was not set as wanted.
    let mut laptop = Laptop::ok();
    laptop.settings.insert("k".into(), SettingValue::Bool(true));
    let got = recover(&rebuild(&j), &mut laptop).unwrap();
    assert_eq!(
        got[0].outcome,
        RecoveredAs::Receipt(Receipt::Failed {
            reason: Reason::NotVerified
        })
    );
    assert!(matches!(
        j.effect(id).unwrap().unwrap().state,
        EffectState::Receipted {
            after_restart: false,
            ..
        }
    ));
}

#[test]
fn a_run_reconciles_first_and_running_again_finishes_without_installing_twice() {
    let (_d, path, j) = journal();
    let plan = plan_of(vec![removable(vec![
        install("A.B"),
        set("k", SettingValue::Int(1)),
    ])]);
    let mut laptop = Laptop::ok();
    {
        // The journal fails after the install's intent: the install runs, nothing more.
        let ledger = Faulty::new(&j, 2);
        let run = run_with(&plan, &mut laptop, &ledger);
        assert!(run.stopped.is_some());
    }
    drop(j);
    let j = common::open(&path).unwrap();
    let run = run_with(&plan, &mut laptop, &rebuild(&j));
    assert_eq!(run.recovered.len(), 1);
    assert_eq!(
        run.recovered[0].outcome,
        RecoveredAs::Receipt(Receipt::Done)
    );
    assert_eq!(run.results[0].1.status(), ItemStatus::Reinstalled);
    assert_eq!(
        laptop
            .done
            .iter()
            .filter(|d| d.starts_with("install"))
            .count(),
        1,
        "installed once: {:?}",
        laptop.done
    );
    // Two install effects: the first (absent before), the second (there before, nothing run).
    let effects = j.effects_of(SCOPE).unwrap();
    let befores: Vec<&Presence> = effects
        .iter()
        .filter_map(|e| match &e.effect {
            StoredEffect::Install { before, .. } => Some(before),
            _ => None,
        })
        .collect();
    assert_eq!(befores.len(), 2);
    assert_eq!(*befores[0], Presence::Absent);
    assert!(matches!(befores[1], Presence::Present { .. }));
    assert!(j.unfinished_effects().unwrap().is_empty());
}

#[test]
fn a_run_whose_recovery_cannot_write_changes_nothing() {
    let (_d, path, j) = journal();
    intend_install(&j);
    drop(j);
    let j = common::open(&path).unwrap();
    let ledger = Faulty::new(&j, 0);
    let mut laptop = Laptop::ok();
    let plan = plan_of(vec![removable(vec![install("C.D")])]);
    let run = run_with(&plan, &mut laptop, &ledger);
    assert!(run.stopped.is_some());
    assert!(run.recovered.is_empty());
    assert!(!laptop.done.iter().any(|d| d.starts_with("install")));
    assert_eq!(run.results[0].1.status(), ItemStatus::Failed);
}

#[test]
fn a_change_that_cannot_be_seen_afterwards_is_not_counted_and_recovery_looks_again() {
    let (_d, _p, j) = journal();
    let plan = plan_of(vec![common::recipe(vec![set("k", SettingValue::Int(5))])]);

    /// Reads work before the change and fail after it.
    struct GoesBlind(Laptop, bool);
    impl pctwin_executor::Adapter for GoesBlind {
        fn platform(&self) -> pctwin_recipe::Platform {
            self.0.platform()
        }
        fn install(
            &mut self,
            g: pctwin_grant::InstallGrant,
            c: Catalog,
            p: &str,
        ) -> Result<(), Reason> {
            self.0.install(g, c, p)
        }
        fn copy_settings(
            &mut self,
            g: pctwin_grant::SettingGrant,
            f: &pctwin_recipe::SettingsPlace,
            t: &pctwin_recipe::SettingsPlace,
        ) -> Result<(), Reason> {
            self.0.copy_settings(g, f, t)
        }
        fn apply_setting(
            &mut self,
            g: pctwin_grant::SettingGrant,
            k: &str,
            v: &SettingValue,
        ) -> Result<(), Reason> {
            self.1 = true;
            self.0.apply_setting(g, k, v)
        }
        fn verify(&mut self, c: &pctwin_recipe::Check) -> bool {
            self.0.verify(c)
        }
        fn app_presence(&mut self, c: Catalog, p: &str) -> Result<Presence, String> {
            self.0.app_presence(c, p)
        }
        fn install_running(&mut self, c: Catalog, p: &str) -> Result<bool, String> {
            self.0.install_running(c, p)
        }
        fn restore_point(&mut self) -> pctwin_journal::RestorePoint {
            self.0.restore_point()
        }
        fn read_setting(&mut self, k: &str) -> Result<Option<SettingValue>, String> {
            if self.1 {
                return Err("cannot read it now".into());
            }
            self.0.read_setting(k)
        }
        fn read_settings_files(
            &mut self,
            p: &pctwin_recipe::SettingsPlace,
        ) -> Result<Vec<pctwin_journal::SettingsFile>, String> {
            self.0.read_settings_files(p)
        }
        fn uninstall(&mut self, c: Catalog, p: &str) -> Result<(), String> {
            self.0.uninstall(c, p)
        }
        fn write_setting_if(
            &mut self,
            k: &str,
            e: &SettingValue,
            v: Option<&SettingValue>,
        ) -> Result<pctwin_executor::SettingWrite, String> {
            self.0.write_setting_if(k, e, v)
        }
        fn replace_settings_file(
            &mut self,
            pl: &pctwin_recipe::SettingsPlace,
            p: &str,
            s: &pctwin_executor::files::Swap<'_>,
        ) -> Result<pctwin_executor::files::Swapped, String> {
            self.0.replace_settings_file(pl, p, s)
        }
    }
    let mut laptop = GoesBlind(Laptop::ok(), false);
    let run = plan.run(&mut laptop, &rebuild(&j), SCOPE, &common::actor());
    assert_eq!(run.results[0].1.reason(), Some(Reason::NotVerified));
    assert_eq!(
        j.unfinished_effects().unwrap().len(),
        1,
        "left for recovery"
    );
    let mut sighted = laptop.0;
    let got = recover(&rebuild(&j), &mut sighted).unwrap();
    assert_eq!(got[0].outcome, RecoveredAs::Receipt(Receipt::Done));
}

#[test]
fn the_faulty_journal_used_here_really_refuses() {
    let (_d, _p, j) = journal();
    let ledger = Faulty::new(&j, 0);
    assert!(ledger.unfinished_effects().unwrap().is_empty());
    assert!(
        ledger
            .intend_setting(
                &planned(0),
                &SettingsIntent::Apply {
                    key: "k".into(),
                    before: None,
                    value: SettingValue::Bool(true)
                }
            )
            .is_err()
    );
    assert!(j.effects().unwrap().is_empty());
}

#[test]
fn a_catalog_check_that_passes_is_not_enough_when_the_app_is_not_there() {
    let (_d, _p, j) = journal();
    let mut laptop = Laptop {
        lies: true,
        install_check_lies: true,
        ..Laptop::ok()
    };
    let run = run_with(
        &plan_of(vec![removable(vec![install("A.B")])]),
        &mut laptop,
        &rebuild(&j),
    );
    assert_eq!(run.results[0].1.reason(), Some(Reason::NotVerified));
}
