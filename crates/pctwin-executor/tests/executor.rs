//! The executor runs only an approved plan, through typed steps, and every item ends honestly.

use pctwin_executor::{Adapter, Plan, PlanError, PlannedItem};
use pctwin_recipe::{
    Arch, Catalog, Check, Os, Platform, Recipe, RecipeId, SettingValue, SettingsPlace, Step,
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

/// A pretend new laptop that records every step it is asked to do.
#[derive(Default)]
struct Fake {
    platform: Option<Platform>,
    done: Vec<String>,
    fail_install: Option<Reason>,
    checks_pass: bool,
}

impl Adapter for Fake {
    fn platform(&self) -> Platform {
        self.platform.unwrap_or(WIN)
    }
    fn install(&mut self, _: Catalog, package: &str) -> Result<(), Reason> {
        self.done.push(format!("install {package}"));
        self.fail_install.map_or(Ok(()), Err)
    }
    fn copy_settings(&mut self, _: &SettingsPlace, _: &SettingsPlace) -> Result<(), Reason> {
        self.done.push("copy".into());
        Ok(())
    }
    fn apply_setting(&mut self, key: &str, _: &SettingValue) -> Result<(), Reason> {
        self.done.push(format!("set {key}"));
        Ok(())
    }
    fn verify(&mut self, _: &Check) -> bool {
        self.done.push("check".into());
        self.checks_pass
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
    assert_eq!(good.done, ["install Mozilla.Firefox", "check"]);

    let mut unchecked = Fake::default();
    assert_eq!(
        plan.run(&mut unchecked)[0].1.reason(),
        Some(Reason::NotVerified)
    );
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
                what: "Download it from its maker".into(),
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
