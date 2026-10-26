//! Conflict-aware undo of installs and settings (System Design S14-P07 and S14-P10, S02-P06,
//! research problem F06): only what PCTwin did, and only while it is still as PCTwin left it, is
//! put back; anything the person changed since is kept and they are told; an app is removed only
//! when PCTwin installed it and its recipe says removal is safe, and never with its user data.

mod common;

use common::{Laptop, SCOPE, copy, install, journal, place, plan_of, recipe, removable, set};
use pctwin_executor::{Undone, undo_effects};
use pctwin_journal::{EffectUndo, EffectUndoOutcome, Journal};
use pctwin_recipe::{SettingValue, SideEffect, UndoDeclaration};

fn moved(j: &Journal, laptop: &mut Laptop, recipes: Vec<pctwin_recipe::Recipe>) {
    let plan = plan_of(recipes);
    let run = plan.run(laptop, j, SCOPE, &common::actor());
    assert_eq!(run.stopped, None);
    laptop.done.clear();
}

fn undo(j: &Journal, laptop: &mut Laptop) -> Vec<EffectUndoOutcome> {
    let permit = j.begin_undo().unwrap();
    undo_effects(j, &permit, laptop, SCOPE)
        .unwrap()
        .into_iter()
        .map(|Undone { outcome, .. }| outcome)
        .collect()
}

#[test]
fn an_app_pctwin_installed_is_removed_through_its_catalog_when_its_recipe_allows() {
    let (_d, _p, j) = journal();
    let mut laptop = Laptop::ok();
    moved(&j, &mut laptop, vec![removable(vec![install("A.B")])]);
    assert!(laptop.installed.contains_key("A.B"));
    assert_eq!(undo(&j, &mut laptop), [EffectUndoOutcome::Uninstalled]);
    assert!(!laptop.installed.contains_key("A.B"));
    assert_eq!(laptop.done, ["uninstall A.B"]);
    let id = j.effects().unwrap()[0].id;
    assert_eq!(
        j.effect_undo_of(id).unwrap(),
        Some(EffectUndo::Done {
            outcome: EffectUndoOutcome::Uninstalled
        })
    );
    // Undone once: asking again changes nothing and says the same.
    laptop.done.clear();
    assert_eq!(undo(&j, &mut laptop), [EffectUndoOutcome::Uninstalled]);
    assert!(laptop.done.is_empty());
}

#[test]
fn an_app_that_was_there_before_the_move_is_left() {
    let (_d, _p, j) = journal();
    let mut laptop = Laptop::ok();
    laptop.installed.insert("A.B".into(), "3.0".into());
    moved(&j, &mut laptop, vec![removable(vec![install("A.B")])]);
    assert_eq!(undo(&j, &mut laptop), [EffectUndoOutcome::AsBefore]);
    assert!(laptop.installed.contains_key("A.B"));
    assert!(laptop.done.is_empty());
}

#[test]
fn an_app_whose_recipe_does_not_allow_removal_is_kept_and_the_person_told_why() {
    for undo_decl in [
        UndoDeclaration::default(),
        UndoDeclaration {
            uninstall: true,
            side_effects: vec![SideEffect::RemovesUserData],
        },
        UndoDeclaration {
            uninstall: true,
            side_effects: vec![SideEffect::SharedParts],
        },
    ] {
        let (_d, _p, j) = journal();
        let mut laptop = Laptop::ok();
        let r = recipe(vec![install("A.B")])
            .with_undo(undo_decl.clone())
            .unwrap();
        moved(&j, &mut laptop, vec![r]);
        let got = undo(&j, &mut laptop);
        assert_eq!(
            got,
            [EffectUndoOutcome::Kept {
                why: undo_decl.why_kept().unwrap().into()
            }]
        );
        assert!(got[0].needs_action());
        assert!(laptop.installed.contains_key("A.B"));
        assert!(laptop.done.is_empty(), "no removal was even tried");
    }
}

#[test]
fn a_removal_that_fails_or_does_nothing_keeps_the_app_and_needs_the_person() {
    for (fail, lie) in [(true, false), (false, true)] {
        let (_d, _p, j) = journal();
        let mut laptop = Laptop::ok();
        moved(&j, &mut laptop, vec![removable(vec![install("A.B")])]);
        laptop.fail_uninstall = fail;
        laptop.uninstall_lies = lie;
        let got = undo(&j, &mut laptop);
        assert!(matches!(&got[0], EffectUndoOutcome::Kept { .. }), "{got:?}");
        assert!(got[0].needs_action());
        assert!(laptop.installed.contains_key("A.B"));
    }
}

#[test]
fn an_app_the_person_already_removed_is_as_before() {
    let (_d, _p, j) = journal();
    let mut laptop = Laptop::ok();
    moved(&j, &mut laptop, vec![removable(vec![install("A.B")])]);
    laptop.installed.clear();
    assert_eq!(undo(&j, &mut laptop), [EffectUndoOutcome::AsBefore]);
    assert!(laptop.done.is_empty());
}

#[test]
fn an_install_that_never_happened_has_nothing_to_undo() {
    let (_d, _p, j) = journal();
    let mut laptop = Laptop {
        lies: true,
        ..Laptop::ok()
    };
    moved(&j, &mut laptop, vec![removable(vec![install("A.B")])]);
    assert_eq!(undo(&j, &mut laptop), [EffectUndoOutcome::AsBefore]);
    assert!(laptop.done.is_empty());
}

#[test]
fn a_setting_still_as_pctwin_set_it_goes_back_to_its_value_or_to_not_set() {
    let (_d, _p, j) = journal();
    let mut laptop = Laptop::ok();
    laptop
        .settings
        .insert("theme".into(), SettingValue::Text("light".into()));
    moved(
        &j,
        &mut laptop,
        vec![recipe(vec![
            set("theme", SettingValue::Text("dark".into())),
            set("font", SettingValue::Int(14)),
        ])],
    );
    assert_eq!(
        undo(&j, &mut laptop),
        [EffectUndoOutcome::Restored, EffectUndoOutcome::Restored]
    );
    assert_eq!(
        laptop.settings.get("theme"),
        Some(&SettingValue::Text("light".into()))
    );
    assert_eq!(laptop.settings.get("font"), None, "not set before: removed");
    assert_eq!(
        laptop.done,
        ["restore font", "restore theme"],
        "newest first"
    );
}

#[test]
fn a_setting_changed_since_the_move_keeps_the_persons_newer_value() {
    let (_d, _p, j) = journal();
    let mut laptop = Laptop::ok();
    laptop
        .settings
        .insert("theme".into(), SettingValue::Text("light".into()));
    moved(
        &j,
        &mut laptop,
        vec![recipe(vec![set(
            "theme",
            SettingValue::Text("dark".into()),
        )])],
    );
    laptop
        .settings
        .insert("theme".into(), SettingValue::Text("blue".into()));
    let got = undo(&j, &mut laptop);
    assert!(
        matches!(&got[0], EffectUndoOutcome::Kept { why } if why.contains("changed after the move"))
    );
    assert!(got[0].needs_action());
    assert_eq!(
        laptop.settings.get("theme"),
        Some(&SettingValue::Text("blue".into()))
    );
    assert!(laptop.done.is_empty());
}

#[test]
fn a_setting_removed_since_the_move_is_kept_removed() {
    let (_d, _p, j) = journal();
    let mut laptop = Laptop::ok();
    laptop.settings.insert("k".into(), SettingValue::Int(1));
    moved(
        &j,
        &mut laptop,
        vec![recipe(vec![set("k", SettingValue::Int(2))])],
    );
    laptop.settings.remove("k");
    assert!(matches!(
        undo(&j, &mut laptop)[0],
        EffectUndoOutcome::Kept { .. }
    ));
    assert_eq!(laptop.settings.get("k"), None);
}

#[test]
fn a_setting_already_back_as_before_is_as_before() {
    let (_d, _p, j) = journal();
    let mut laptop = Laptop::ok();
    laptop.settings.insert("k".into(), SettingValue::Int(1));
    moved(
        &j,
        &mut laptop,
        vec![recipe(vec![set("k", SettingValue::Int(2))])],
    );
    laptop.settings.insert("k".into(), SettingValue::Int(1));
    assert_eq!(undo(&j, &mut laptop), [EffectUndoOutcome::AsBefore]);
    assert!(laptop.done.is_empty());
}

#[test]
fn a_setting_that_cannot_be_put_back_is_not_done_and_tried_again_next_time() {
    let (_d, _p, j) = journal();
    let mut laptop = Laptop::ok();
    moved(
        &j,
        &mut laptop,
        vec![recipe(vec![set("k", SettingValue::Int(2))])],
    );
    laptop.fail_restore = true;
    let got = undo(&j, &mut laptop);
    assert!(matches!(got[0], EffectUndoOutcome::NotDone { .. }));
    laptop.fail_restore = false;
    assert_eq!(undo(&j, &mut laptop), [EffectUndoOutcome::Restored]);
    assert_eq!(laptop.settings.get("k"), None);
}

#[test]
fn the_same_setting_changed_twice_in_a_move_goes_back_to_its_first_value() {
    let (_d, _p, j) = journal();
    let mut laptop = Laptop::ok();
    laptop.settings.insert("k".into(), SettingValue::Int(1));
    moved(
        &j,
        &mut laptop,
        vec![
            recipe(vec![set("k", SettingValue::Int(2))]),
            recipe(vec![set("k", SettingValue::Int(3))]),
        ],
    );
    assert_eq!(
        undo(&j, &mut laptop),
        [EffectUndoOutcome::Restored, EffectUndoOutcome::Restored]
    );
    assert_eq!(laptop.settings.get("k"), Some(&SettingValue::Int(1)));
}

#[test]
fn copied_settings_files_go_back_only_where_unchanged_and_new_files_are_left() {
    let (_d, _p, j) = journal();
    let mut laptop = Laptop::ok();
    let p = place("vlc");
    laptop.put_file(&p, "vlcrc", b"defaults");
    laptop.put_file(&p, "mine.ini", b"untouched by the move");
    laptop.put_source(&p, "vlcrc", b"old laptop vlcrc");
    laptop.put_source(&p, "extra.ini", b"old laptop extra");
    laptop.put_source(&p, "edited.ini", b"old laptop edited");
    moved(&j, &mut laptop, vec![recipe(vec![copy("vlc")])]);
    // Since the move: the person edited one copied file and made a new one.
    laptop.put_file(&p, "edited.ini", b"my edit");
    laptop.put_file(&p, "new.ini", b"made after the move");
    let got = undo(&j, &mut laptop);
    assert!(
        matches!(&got[0], EffectUndoOutcome::Kept { why } if why.starts_with("1 of")),
        "{got:?}"
    );
    assert_eq!(
        laptop.file(&p, "vlcrc").unwrap(),
        b"defaults",
        "earlier version back"
    );
    assert_eq!(
        laptop.file(&p, "extra.ini"),
        None,
        "not there before: removed"
    );
    assert_eq!(
        laptop.file(&p, "edited.ini").unwrap(),
        b"my edit",
        "edit kept"
    );
    assert_eq!(laptop.file(&p, "new.ini").unwrap(), b"made after the move");
    assert_eq!(
        laptop.file(&p, "mine.ini").unwrap(),
        b"untouched by the move"
    );
}

#[test]
fn copied_settings_files_all_unchanged_are_all_restored() {
    let (_d, _p, j) = journal();
    let mut laptop = Laptop::ok();
    let p = place("vlc");
    laptop.put_file(&p, "", b"one file before");
    laptop.put_source(&p, "", b"one file from the old laptop");
    moved(&j, &mut laptop, vec![recipe(vec![copy("vlc")])]);
    assert_eq!(undo(&j, &mut laptop), [EffectUndoOutcome::Restored]);
    assert_eq!(laptop.file(&p, "").unwrap(), b"one file before");
    // Asking again: already undone.
    assert_eq!(undo(&j, &mut laptop), [EffectUndoOutcome::Restored]);
}

#[test]
fn a_damaged_earlier_version_is_never_written_back() {
    let (_d, path, j) = journal();
    let mut laptop = Laptop::ok();
    let p = place("vlc");
    laptop.put_file(&p, "vlcrc", b"defaults");
    laptop.put_source(&p, "vlcrc", b"old laptop");
    moved(&j, &mut laptop, vec![recipe(vec![copy("vlc")])]);
    drop(j);
    {
        let db = redb::Database::open(&path).unwrap();
        let tx = db.begin_write().unwrap();
        tx.open_table(redb::TableDefinition::<[u8; 32], &[u8]>::new("kept-files"))
            .unwrap()
            .insert(
                pctwin_journal::content_hash(b"defaults"),
                b"garbage".as_slice(),
            )
            .unwrap();
        tx.commit().unwrap();
    }
    let j = Journal::open(&path).unwrap();
    let got = undo(&j, &mut laptop);
    assert!(matches!(&got[0], EffectUndoOutcome::Kept { why } if why.contains("damaged")));
    assert_eq!(laptop.file(&p, "vlcrc").unwrap(), b"old laptop");
    assert!(laptop.done.is_empty());
}

#[test]
fn after_a_crash_part_way_a_setting_is_looked_at_again_and_finished() {
    let (_d, path, j) = journal();
    let mut laptop = Laptop::ok();
    laptop.settings.insert("k".into(), SettingValue::Int(1));
    moved(
        &j,
        &mut laptop,
        vec![recipe(vec![set("k", SettingValue::Int(2))])],
    );
    let id = j.effects().unwrap()[0].id;
    {
        let permit = j.begin_undo().unwrap();
        j.settings()
            .record_undo(&permit, id, &EffectUndo::Reversing)
            .unwrap();
    }
    drop(j); // the power went out before the setting was written back
    let j = Journal::open(&path).unwrap();
    assert_eq!(undo(&j, &mut laptop), [EffectUndoOutcome::Restored]);
    assert_eq!(laptop.settings.get("k"), Some(&SettingValue::Int(1)));
    // And if it had already been written back before the power went out:
    let (_d2, path2, j2) = journal();
    let mut laptop2 = Laptop::ok();
    moved(
        &j2,
        &mut laptop2,
        vec![recipe(vec![set("k", SettingValue::Int(2))])],
    );
    let id2 = j2.effects().unwrap()[0].id;
    {
        let permit = j2.begin_undo().unwrap();
        j2.settings()
            .record_undo(&permit, id2, &EffectUndo::Reversing)
            .unwrap();
    }
    laptop2.settings.remove("k");
    drop(j2);
    let j2 = Journal::open(&path2).unwrap();
    assert_eq!(undo(&j2, &mut laptop2), [EffectUndoOutcome::Restored]);
    assert!(laptop2.done.is_empty());
}

#[test]
fn after_a_crash_part_way_an_app_removal_is_never_run_again_by_itself() {
    let (_d, path, j) = journal();
    let mut laptop = Laptop::ok();
    moved(&j, &mut laptop, vec![removable(vec![install("A.B")])]);
    let id = j.effects().unwrap()[0].id;
    {
        let permit = j.begin_undo().unwrap();
        j.installs()
            .record_undo(&permit, id, &EffectUndo::Reversing)
            .unwrap();
    }
    drop(j);
    let j = Journal::open(&path).unwrap();
    let got = undo(&j, &mut laptop);
    assert!(matches!(&got[0], EffectUndoOutcome::NotDone { why } if why.contains("again")));
    assert!(
        laptop.done.is_empty(),
        "the removal was not run again by itself"
    );
    // Asking for undo again starts it afresh.
    assert_eq!(undo(&j, &mut laptop), [EffectUndoOutcome::Uninstalled]);
    assert_eq!(laptop.done, ["uninstall A.B"]);
    // And a removal that had finished before the crash is found finished.
    let (_d2, path2, j2) = journal();
    let mut laptop2 = Laptop::ok();
    moved(&j2, &mut laptop2, vec![removable(vec![install("A.B")])]);
    let id2 = j2.effects().unwrap()[0].id;
    {
        let permit = j2.begin_undo().unwrap();
        j2.installs()
            .record_undo(&permit, id2, &EffectUndo::Reversing)
            .unwrap();
    }
    laptop2.installed.clear();
    drop(j2);
    let j2 = Journal::open(&path2).unwrap();
    assert_eq!(undo(&j2, &mut laptop2), [EffectUndoOutcome::Uninstalled]);
}

#[test]
fn a_change_not_yet_checked_is_reported_and_not_undone() {
    let (_d, _p, j) = journal();
    let mut laptop = Laptop::ok();
    let ledger = common::Faulty::new(&j, 2);
    let plan = plan_of(vec![recipe(vec![set("k", SettingValue::Int(2))])]);
    let run = plan.run(&mut laptop, &ledger, SCOPE, &common::actor());
    assert!(run.stopped.is_some());
    laptop.done.clear();
    let got = undo(&j, &mut laptop);
    assert!(matches!(&got[0], EffectUndoOutcome::NotDone { .. }));
    assert!(laptop.done.is_empty());
    let id = j.effects().unwrap()[0].id;
    assert_eq!(j.effect_undo_of(id).unwrap(), None);
}

#[test]
fn what_cannot_be_looked_at_now_is_not_done_this_time() {
    for blind in ["app", "setting", "files"] {
        let (_d, _p, j) = journal();
        let mut laptop = Laptop::ok();
        let p = place("vlc");
        laptop.put_source(&p, "vlcrc", b"x");
        let r = match blind {
            "app" => removable(vec![install("A.B")]),
            "setting" => recipe(vec![set("k", SettingValue::Int(2))]),
            _ => recipe(vec![copy("vlc")]),
        };
        moved(&j, &mut laptop, vec![r]);
        laptop.blind.insert(blind);
        assert!(
            matches!(undo(&j, &mut laptop)[0], EffectUndoOutcome::NotDone { .. }),
            "{blind}"
        );
        assert!(laptop.done.is_empty());
    }
}

#[test]
fn undo_reasons_are_plain_words() {
    let (_d, _p, j) = journal();
    let mut laptop = Laptop::ok();
    laptop.settings.insert("k".into(), SettingValue::Int(1));
    moved(
        &j,
        &mut laptop,
        vec![
            recipe(vec![install("A.B")]),
            recipe(vec![set("k", SettingValue::Int(2))]),
        ],
    );
    laptop.settings.insert("k".into(), SettingValue::Int(9));
    for outcome in undo(&j, &mut laptop) {
        if let EffectUndoOutcome::Kept { why } | EffectUndoOutcome::NotDone { why } = outcome {
            for jargon in [
                "preimage", "purge", "zap", "catalog", "journal", "effect", "hash",
            ] {
                assert!(!why.to_lowercase().contains(jargon), "{why}");
            }
        }
    }
}

#[test]
fn undo_follows_the_gate_closing_undo_shuts_it_for_effects_too() {
    let (_d, _p, j) = journal();
    let mut laptop = Laptop::ok();
    moved(
        &j,
        &mut laptop,
        vec![recipe(vec![set("k", SettingValue::Int(2))])],
    );
    j.close_undo(&mut |_, _| unreachable!()).unwrap();
    assert!(matches!(
        j.begin_undo(),
        Err(pctwin_journal::JournalError::UndoClosed)
    ));
    assert_eq!(laptop.settings.get("k"), Some(&SettingValue::Int(2)));
}
