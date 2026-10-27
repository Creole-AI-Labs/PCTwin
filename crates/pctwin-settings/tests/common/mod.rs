//! A journal, role keys and approved plans of setting items, shared by the settings tests.
#![allow(dead_code)]

use pctwin_executor::{Plan, PlannedItem, RecipeBook, Undone, undo_effects};
use pctwin_journal::{Actor, EffectUndoOutcome, Journal, Permission, Rebuild, Scope};
use pctwin_recipe::Recipe;
use pctwin_record::{
    FolderRole, Inclusion, Item, ItemId, ItemKind, ItemName, ItemPath, LaptopId, ManagedBy, Owner,
    Place, Portability, Record, Storage,
};

pub const SCOPE: Scope = Scope([7; 16]);

pub fn actor() -> Actor {
    Actor {
        acting_account: "1001".into(),
        for_account: "1001".into(),
        permission: Permission::OwnFolders,
    }
}

pub fn keys() -> &'static pctwin_grant::RoleKeys {
    static KEYS: std::sync::OnceLock<pctwin_grant::RoleKeys> = std::sync::OnceLock::new();
    KEYS.get_or_init(|| pctwin_grant::RoleKeys::mint().expect("the keys are made once"))
}

pub fn rebuild(j: &Journal) -> Rebuild<'_> {
    Rebuild::new(j, &keys().install, &keys().setting)
}

pub fn journal() -> (tempfile::TempDir, Journal) {
    let dir = tempfile::tempdir().unwrap();
    let j = Journal::open_with_record_key(
        &dir.path().join("journal.redb"),
        pctwin_journal::keystore::TagKey::from_bytes([0x5a; 32]),
    )
    .unwrap();
    (dir, j)
}

fn laptop_id() -> LaptopId {
    LaptopId::from_hex("00112233445566778899aabbccddeeff").unwrap()
}

pub fn setting_item(name: &str) -> Item {
    let owner = Owner::Shared;
    let place = Place {
        role: FolderRole::Documents,
        storage: Storage::SystemDrive,
    };
    let path = ItemPath::new(vec![ItemName::from_text(name)]).unwrap();
    Item {
        id: ItemId::derive(&laptop_id(), &owner, &place, &path),
        kind: ItemKind::Setting,
        owner,
        place,
        path,
        size_bytes: 0,
        modified_ns: None,
        portability: Portability::Rebuildable,
        managed_by: ManagedBy::Personal,
        download_bytes: None,
        inclusion: Inclusion::Included,
    }
}

/// An approved plan of one setting item per recipe.
pub fn plan_of(recipes: &[Recipe]) -> Plan {
    let items: Vec<Item> = (0..recipes.len())
        .map(|n| setting_item(&format!("setting{n}")))
        .collect();
    let mut record = Record::new(laptop_id());
    record.items = items.clone();
    let approval = record.approve().unwrap();
    Plan::approved(
        &record,
        &approval,
        items
            .iter()
            .zip(recipes)
            .map(|(i, r)| PlannedItem {
                item: i.id,
                recipe: r.clone(),
            })
            .collect(),
    )
    .unwrap()
}

pub fn undo(
    j: &Journal,
    adapter: &mut dyn pctwin_executor::Adapter,
    book: &dyn RecipeBook,
    scope: Scope,
) -> Vec<EffectUndoOutcome> {
    let permit = j.begin_undo().unwrap();
    undo_effects(&rebuild(j), &permit, adapter, scope, book)
        .unwrap()
        .into_iter()
        .map(|Undone { outcome, .. }| outcome)
        .collect()
}

/// A recipe book of these recipes.
pub fn book(recipes: &[Recipe]) -> Vec<Recipe> {
    recipes.to_vec()
}
