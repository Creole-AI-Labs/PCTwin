//! Small builders the tests share: items as a scan would make them.
#![allow(dead_code)]

use pctwin_record::{
    CloudProvider, FolderRole, Inclusion, Item, ItemId, ItemKind, ItemName, ItemPath, LaptopId,
    LeftOutReason, ManagedBy, Owner, Place, Portability, Storage,
};

pub fn laptop(n: u8) -> LaptopId {
    LaptopId::from_hex(&format!("{n:02x}").repeat(16)).unwrap()
}

pub fn person(id: &str) -> Owner {
    Owner::Person {
        account_id: id.into(),
    }
}

pub fn path(parts: &[&str]) -> ItemPath {
    ItemPath::new(parts.iter().map(|p| ItemName::from_text(p)).collect()).unwrap()
}

pub fn item_in(
    laptop: &LaptopId,
    owner: Owner,
    role: FolderRole,
    storage: Storage,
    parts: &[&str],
    size: u64,
) -> Item {
    let place = Place { role, storage };
    let path = path(parts);
    Item {
        id: ItemId::derive(laptop, &owner, &place, &path),
        kind: ItemKind::File,
        owner,
        place,
        path,
        size_bytes: size,
        modified_ns: Some(1_700_000_000_000_000_000),
        portability: Portability::Portable,
        managed_by: ManagedBy::Personal,
        download_bytes: None,
        inclusion: Inclusion::Included,
    }
}

pub fn file(owner: Owner, parts: &[&str], size: u64) -> Item {
    item_in(
        &laptop(1),
        owner,
        FolderRole::Documents,
        Storage::SystemDrive,
        parts,
        size,
    )
}

pub fn cloud_file(owner: Owner, provider: CloudProvider, parts: &[&str], size: u64) -> Item {
    item_in(
        &laptop(1),
        owner,
        FolderRole::Documents,
        Storage::Cloud { provider },
        parts,
        size,
    )
}

pub fn online_only(owner: Owner, provider: CloudProvider, parts: &[&str], size: u64) -> Item {
    let mut i = cloud_file(owner, provider, parts, size);
    i.inclusion = Inclusion::LeftOut {
        reason: LeftOutReason::CloudOnly,
    };
    i
}
