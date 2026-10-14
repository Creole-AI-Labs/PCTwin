//! The executor runs only an approved plan, through typed steps, and every item ends honestly.

use pctwin_executor::{Adapter, Plan, PlanError, PlannedItem};
use pctwin_recipe::{
    Arch, Catalog, Check, ChecklistKey, Os, Platform, Recipe, RecipeId, SettingValue,
    SettingsPlace, Step,
};
use pctwin_record::{
    Approval, FolderRole, Inclusion, Item, ItemId, ItemKind, ItemName, ItemPath, LaptopId,
    LeftOutReason, ManagedBy, Owner, Place, Portability, Record, Storage,
};
use pctwin_types::{ItemStatus, Reason};

fn laptop() -> LaptopId {
    LaptopId::from_hex("00112233445566778899aabbccddeeff").unwrap()
}

fn app(name: &str) -> Item {
    let owner = Owner::Shared;
    let place = Place {
        role: FolderRole::Documents,
        storage: Storage::SystemDrive,
    };
    let path = ItemPath::new(vec![ItemName::from_text(name)]).unwrap();
    Item {
        id: ItemId::derive(&laptop(), &owner, &place, &path),
        kind: ItemKind::App,
        owner,
        place,
        path,
        size_bytes: 0,
        portability: Portability::Rebuildable,
        managed_by: ManagedBy::Personal,
        download_bytes: None,
        inclusion: Inclusion::Included,
    }
}

const WIN: Platform = Platform {
    os: Os::Windows,
    arch: Arch::X86_64,
};

fn recipe(steps: Vec<Step>) -> Recipe {
    Recipe::new(RecipeId::new("x").unwrap(), 1, vec![WIN], steps).unwrap()
}

fn install_and_check(package: &str) -> Recipe {
    recipe(vec![
        Step::Install {
            catalog: Catalog::Winget,
            package: package.into(),
        },
        Step::Verify {
            check: Check::PackageInstalled {
                catalog: Catalog::Winget,
                package: package.into(),
            },
        },
    ])
}

/// A pretend new laptop that records every step it is asked to do. It knows what it really
/// installed; `lies` makes an install report success and do nothing.
#[derive(Default)]
struct Fake {
    platform: Option<Platform>,
    done: Vec<String>,
    fail_install: Option<Reason>,
    lies: bool,
    installed: std::collections::HashSet<String>,
    /// Whether checks other than "is it installed" pass.
    checks_pass: bool,
    /// Settings places whose contents really match their source.
    copied: std::collections::HashSet<String>,
    /// Makes a settings copy report success and copy nothing.
    copy_lies: bool,
}

impl Adapter for Fake {
    fn platform(&self) -> Platform {
        self.platform.unwrap_or(WIN)
    }
    fn install(&mut self, _: Catalog, package: &str) -> Result<(), Reason> {
        self.done.push(format!("install {package}"));
        if let Some(reason) = self.fail_install {
            return Err(reason);
        }
        if !self.lies {
            self.installed.insert(package.to_string());
        }
        Ok(())
    }
    fn copy_settings(&mut self, _: &SettingsPlace, to: &SettingsPlace) -> Result<(), Reason> {
        self.done.push("copy".into());
        if !self.copy_lies {
            self.copied.insert(format!("{to:?}"));
        }
        Ok(())
    }
    fn apply_setting(&mut self, key: &str, _: &SettingValue) -> Result<(), Reason> {
        self.done.push(format!("set {key}"));
        Ok(())
    }
    fn verify(&mut self, check: &Check) -> bool {
        self.done.push("check".into());
        match check {
            Check::PackageInstalled { package, .. } => self.installed.contains(package),
            Check::SettingsMatch { to, .. } => self.copied.contains(&format!("{to:?}")),
            _ => self.checks_pass,
        }
    }
}

fn approved(items: Vec<Item>) -> (Record, Approval) {
    let mut record = Record::new(laptop());
    record.items = items;
    let approval = record.approve().unwrap();
    (record, approval)
}

fn one_app_plan(a: &Item) -> Plan {
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
    let mut good = Fake {
        checks_pass: true,
        ..Fake::default()
    };
    assert_eq!(plan.run(&mut good)[0].1.status(), ItemStatus::Reinstalled);
    // The install is checked right after it runs, then the recipe's own check runs.
    assert_eq!(good.done, ["install Mozilla.Firefox", "check", "check"]);

    let mut liar = Fake {
        lies: true,
        checks_pass: true,
        ..Fake::default()
    };
    assert_eq!(plan.run(&mut liar)[0].1.reason(), Some(Reason::NotVerified));
}

#[test]
fn a_failed_step_stops_the_recipe_and_says_why() {
    let plan = one_app_plan(&app("Firefox"));
    let mut fake = Fake {
        fail_install: Some(Reason::InstallFailed),
        checks_pass: true,
        ..Fake::default()
    };
    let results = plan.run(&mut fake);
    assert_eq!(results[0].1.status(), ItemStatus::Failed);
    assert_eq!(results[0].1.reason(), Some(Reason::InstallFailed));
    assert_eq!(fake.done, ["install Mozilla.Firefox"]);
}

#[test]
fn nothing_runs_for_a_system_the_recipe_does_not_support() {
    let plan = one_app_plan(&app("Firefox"));
    let mut mac = Fake {
        platform: Some(Platform {
            os: Os::Macos,
            arch: Arch::Aarch64,
        }),
        checks_pass: true,
        ..Fake::default()
    };
    assert_eq!(plan.run(&mut mac)[0].1.reason(), Some(Reason::NoEquivalent));
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
    let results = plan.run(&mut Fake::default());
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
    let mut liar = Fake {
        lies: true,
        checks_pass: true,
        ..Fake::default()
    };
    liar.installed.insert("Already.There".into());
    let result = plan.run(&mut liar)[0].1;
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
    let mut ok = Fake {
        checks_pass: true,
        ..Fake::default()
    };
    assert_eq!(plan.run(&mut ok)[0].1.status(), ItemStatus::Converted);
    assert_eq!(ok.done, ["set editor.font", "check"]);
    let mut wrong = Fake::default();
    assert_eq!(
        plan.run(&mut wrong)[0].1.reason(),
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
    let mut record = Record::new(laptop());
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
    let place = SettingsPlace {
        root: pctwin_recipe::SettingsRoot::Config,
        path: pctwin_recipe::RelPath::new("Editor/settings.json").unwrap(),
    };
    let recipe = recipe(vec![Step::CopySettings {
        from: place.clone(),
        to: place,
    }]);
    let plan =
        Plan::approved(&record, &approval, vec![PlannedItem { item: a.id, recipe }]).unwrap();
    // The defaults are there, so "is a file there" would pass; the copy itself did nothing.
    let mut liar = Fake {
        copy_lies: true,
        checks_pass: true,
        ..Fake::default()
    };
    assert_eq!(plan.run(&mut liar)[0].1.reason(), Some(Reason::NotVerified));
    let mut honest = Fake::default();
    assert_eq!(plan.run(&mut honest)[0].1.status(), ItemStatus::Converted);
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
