//! Fixtures: a made-up old laptop with a few things on it (no real person's data).
#![allow(dead_code)]

use pctwin_readiness::ledger::{Ledger, MoveId, Observed, Probe, Receipt, Role};
use pctwin_record::{
    FolderRole, Inclusion, Item, ItemId, ItemKind, ItemName, ItemPath, LaptopId, LeftOutReason,
    ManagedBy, Owner, Place, Portability, Record, Storage,
};
use pctwin_types::ItemResult;

pub const MOVER: &str = "1000";
pub const T0: u64 = 1_800_000_000;

pub fn old_laptop() -> LaptopId {
    LaptopId::from_hex("00112233445566778899aabbccddeeff").unwrap()
}

pub fn new_laptop() -> LaptopId {
    LaptopId::from_hex("ffeeddccbbaa99887766554433221100").unwrap()
}

pub fn other_laptop() -> LaptopId {
    LaptopId::from_hex("0123456789abcdef0123456789abcdef").unwrap()
}

pub fn move_id() -> MoveId {
    MoveId {
        source: old_laptop(),
        target: new_laptop(),
        session: 7,
    }
}

pub fn item(owner: &str, name: &str, kind: ItemKind, inclusion: Inclusion) -> Item {
    let owner = Owner::Person {
        account_id: owner.into(),
    };
    let place = Place {
        role: FolderRole::Documents,
        storage: Storage::SystemDrive,
    };
    let path = ItemPath::new(vec![ItemName::from_text(name)]).unwrap();
    Item {
        id: ItemId::derive(&old_laptop(), &owner, &place, &path),
        kind,
        owner,
        place,
        path,
        size_bytes: 10,
        modified_ns: Some(1),
        portability: Portability::Portable,
        managed_by: ManagedBy::Personal,
        download_bytes: None,
        inclusion,
    }
}

pub fn file(name: &str) -> Item {
    item(MOVER, name, ItemKind::File, Inclusion::Included)
}

pub fn app(name: &str) -> Item {
    item(MOVER, name, ItemKind::App, Inclusion::Included)
}

pub fn left_out(name: &str, reason: LeftOutReason) -> Item {
    item(MOVER, name, ItemKind::File, Inclusion::LeftOut { reason })
}

pub fn record(items: Vec<Item>) -> Record {
    let mut r = Record::new(old_laptop());
    r.items = items;
    r
}

pub fn ledger(items: Vec<Item>) -> (Record, Ledger) {
    let r = record(items);
    let l = Ledger::from_record(&r, MOVER, move_id(), T0);
    (r, l)
}

/// A probe that sees exactly what it is told.
pub struct Sees(pub Observed);

impl Probe for Sees {
    fn observe(&self, _receipt: &Receipt) -> Observed {
        self.0
    }
}

pub fn receipt(l: &Ledger, role: Role, item: ItemId, result: Option<ItemResult>) -> Receipt {
    Receipt {
        role,
        source: old_laptop(),
        target: new_laptop(),
        session: 7,
        generation: l.generation(),
        item,
        result,
        evidence: None,
        observed_at: T0 + 10,
    }
}
