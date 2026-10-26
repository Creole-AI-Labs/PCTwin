//! The executor runs only an approved plan, through typed steps, and every item ends honestly.

mod common;

use common::{Laptop, app, approved, install_and_check, laptop_id, recipe, run};
use pctwin_executor::{Plan, PlanError, PlannedItem};
use pctwin_recipe::{Arch, Catalog, Check, ChecklistKey, Os, Platform, SettingValue, Step};
use pctwin_record::{Approval, Inclusion, ItemId, ItemKind, LeftOutReason, Portability, Record};
use pctwin_types::{ItemStatus, Reason};

fn one_app_plan(a: &pctwin_record::Item) -> Plan {
    let (record, approval) = approved(vec![a.clone()]);
    Plan::approved(
        &record,
        &approval,
        vec![PlannedItem {
            item: a.id,
            recipe: install_and_check("Mozilla.Firefox"),
        }],
    )
    .unwrap()
}

#[test]
fn an_approved_app_is_reinstalled_only_when_its_check_passes() {
    let plan = one_app_plan(&app("Firefox"));
    let mut good = Laptop::ok();
    assert_eq!(run(&plan, &mut good)[0].1.status(), ItemStatus::Reinstalled);
    // The install is checked right after it runs, then the recipe's own check runs.
    assert_eq!(
        good.done,
        ["restore point", "install Mozilla.Firefox", "check", "check"]
    );

    let mut liar = Laptop {
        lies: true,
        ..Laptop::ok()
    };
    assert_eq!(
        run(&plan, &mut liar)[0].1.reason(),
        Some(Reason::NotVerified)
    );
}

#[test]
fn a_failed_step_stops_the_recipe_and_says_why() {
    let plan = one_app_plan(&app("Firefox"));
    let mut fake = Laptop {
        fail_install: Some(Reason::InstallFailed),
        ..Laptop::ok()
    };
    let results = run(&plan, &mut fake);
    assert_eq!(results[0].1.status(), ItemStatus::Failed);
    assert_eq!(results[0].1.reason(), Some(Reason::InstallFailed));
    assert_eq!(fake.done, ["restore point", "install Mozilla.Firefox"]);
}

#[test]
fn nothing_runs_for_a_system_the_recipe_does_not_support() {
    let plan = one_app_plan(&app("Firefox"));
    let mut mac = Laptop {
        platform: Some(Platform {
            os: Os::Macos,
            arch: Arch::Aarch64,
        }),
        ..Laptop::ok()
    };
    assert_eq!(
        run(&plan, &mut mac)[0].1.reason(),
        Some(Reason::NoEquivalent)
    );
    assert!(mac.done.is_empty());
}

#[test]
fn a_checklist_step_leaves_the_item_for_the_person() {
    let a = app("Tax app");
    let (record, approval) = approved(vec![a.clone()]);
    let plan = Plan::approved(
        &record,
        &approval,
        vec![PlannedItem {
            item: a.id,
            recipe: recipe(vec![Step::Checklist {
                what: ChecklistKey::new("download-from-maker").unwrap(),
            }]),
        }],
    )
    .unwrap();
    let results = run(&plan, &mut Laptop::default());
    assert_eq!(results[0].1.status(), ItemStatus::NeedsAction);
    assert_eq!(results[0].1.reason(), Some(Reason::FinishByHand));
}

#[test]
fn a_plan_needs_an_approval_of_this_exact_revision() {
    let a = app("Firefox");
    let (mut record, approval) = approved(vec![a.clone()]);
    record.items.push(app("Other"));
    let planned = vec![PlannedItem {
        item: a.id,
        recipe: install_and_check("Mozilla.Firefox"),
    }];
    assert!(matches!(
        Plan::approved(&record, &approval, planned),
        Err(PlanError::Record(_))
    ));
}

#[test]
fn a_plan_cannot_add_items_outside_the_move() {
    let a = app("Firefox");
    let mut left = app("Old game");
    left.inclusion = Inclusion::LeftOut {
        reason: LeftOutReason::ChosenByUser,
    };
    let stranger = app("Never scanned");
    let (record, approval) = approved(vec![a.clone(), left.clone()]);
    let plan = |item: ItemId| {
        Plan::approved(
            &record,
            &approval,
            vec![PlannedItem {
                item,
                recipe: install_and_check("X"),
            }],
        )
    };
    assert!(matches!(plan(stranger.id), Err(PlanError::UnknownItem(_))));
    assert!(matches!(plan(left.id), Err(PlanError::LeftOut(_))));
    let twice = Plan::approved(
        &record,
        &approval,
        vec![
            PlannedItem {
                item: a.id,
                recipe: install_and_check("X"),
            },
            PlannedItem {
                item: a.id,
                recipe: install_and_check("Y"),
            },
        ],
    );
    assert!(matches!(twice, Err(PlanError::Twice(_))));
}

/// Regression (fresh-context review of PR 20): a check of something else, placed before the
/// install, let an install that did nothing count as reinstalled.
#[test]
fn an_install_that_did_nothing_is_never_reinstalled() {
    let a = app("Firefox");
    let (record, approval) = approved(vec![a.clone()]);
    let recipe = recipe(vec![
        Step::Verify {
            check: Check::PackageInstalled {
                catalog: Catalog::Winget,
                package: "Already.There".into(),
            },
        },
        Step::Install {
            catalog: Catalog::Winget,
            package: "Mozilla.Firefox".into(),
        },
    ]);
    let plan =
        Plan::approved(&record, &approval, vec![PlannedItem { item: a.id, recipe }]).unwrap();
    let mut liar = Laptop {
        lies: true,
        ..Laptop::ok()
    };
    liar.installed.insert("Already.There".into(), "2.0".into());
    let result = run(&plan, &mut liar)[0].1;
    assert_eq!(result.status(), ItemStatus::Failed);
    assert_eq!(result.reason(), Some(Reason::NotVerified));
}

#[test]
fn settings_alone_are_converted_and_each_change_is_checked() {
    let a = app("Editor");
    let (record, approval) = approved(vec![a.clone()]);
    let recipe = recipe(vec![Step::ApplySetting {
        key: "editor.font".into(),
        value: SettingValue::Int(14),
    }]);
    let plan =
        Plan::approved(&record, &approval, vec![PlannedItem { item: a.id, recipe }]).unwrap();
    let mut ok = Laptop::ok();
    assert_eq!(run(&plan, &mut ok)[0].1.status(), ItemStatus::Converted);
    assert_eq!(ok.done, ["restore point", "set editor.font", "check"]);
    let mut wrong = Laptop::default();
    assert_eq!(
        run(&plan, &mut wrong)[0].1.reason(),
        Some(Reason::NotVerified)
    );
}

/// Regression (fresh-context review of PR 20): a hand-made approval of a record that could not be
/// approved, and a recipe planned for a file, are refused.
#[test]
fn forged_approvals_and_recipes_for_files_are_refused() {
    let a = app("Firefox");
    let mut bound = app("Authenticator");
    bound.portability = Portability::DeviceBound;
    let mut record = Record::new(laptop_id());
    record.items = vec![a.clone(), bound];
    assert!(record.approve().is_err());
    let forged = Approval {
        revision: record.revision,
        fingerprint: record.fingerprint(),
    };
    assert!(matches!(
        Plan::approved(
            &record,
            &forged,
            vec![PlannedItem {
                item: a.id,
                recipe: install_and_check("Mozilla.Firefox"),
            }],
        ),
        Err(PlanError::Record(_))
    ));

    let mut file = app("notes.txt");
    file.kind = ItemKind::File;
    let (record, approval) = approved(vec![file.clone()]);
    assert!(matches!(
        Plan::approved(
            &record,
            &approval,
            vec![PlannedItem {
                item: file.id,
                recipe: install_and_check("X"),
            }],
        ),
        Err(PlanError::NotRebuildable(_))
    ));
}

/// Regression (second fresh-context review of PR 20): a settings file that already existed (an
/// app's defaults) must not make a copy that did nothing count as done.
#[test]
fn a_copy_onto_existing_defaults_is_checked_by_contents() {
    let a = app("Editor");
    let (record, approval) = approved(vec![a.clone()]);
    let place = common::place("Editor/settings.json");
    let recipe = recipe(vec![Step::CopySettings {
        from: place.clone(),
        to: place.clone(),
    }]);
    let plan =
        Plan::approved(&record, &approval, vec![PlannedItem { item: a.id, recipe }]).unwrap();
    // The defaults are there, so "is a file there" would pass; the copy itself did nothing.
    let mut liar = Laptop {
        copy_lies: true,
        ..Laptop::ok()
    };
    liar.put_file(&place, "", b"{\"defaults\":true}");
    liar.put_source(&place, "", b"{\"font\":14}");
    assert_eq!(
        run(&plan, &mut liar)[0].1.reason(),
        Some(Reason::NotVerified)
    );
    let mut honest = Laptop::default();
    honest.put_file(&place, "", b"{\"defaults\":true}");
    honest.put_source(&place, "", b"{\"font\":14}");
    assert_eq!(run(&plan, &mut honest)[0].1.status(), ItemStatus::Converted);
}

/// Regression (second fresh-context review of PR 20): a setting is never "reinstalled".
#[test]
fn a_recipe_for_a_setting_may_not_install() {
    let mut setting = app("Wallpaper");
    setting.kind = ItemKind::Setting;
    let (record, approval) = approved(vec![setting.clone()]);
    assert!(matches!(
        Plan::approved(
            &record,
            &approval,
            vec![PlannedItem {
                item: setting.id,
                recipe: install_and_check("X"),
            }],
        ),
        Err(PlanError::InstallForSetting(_))
    ));
}
