//! Shared by the move tests: a real approved plan for the files a test moves, built and approved
//! the way the app does, so a receiver only ever gets an allowance from an approval.

use pctwin_record::{
    FolderRole, Inclusion, Item, ItemId, ItemKind, ItemName, ItemPath, LaptopId, ManagedBy, Owner,
    PersonTarget, Place, Portability, Record, Storage,
};
use pctwin_transfer::Allowance;

/// The approved plan for these files (each with its size), as the new laptop holds it.
pub fn approved(files: &[(ItemId, u64)]) -> Allowance {
    let laptop = LaptopId::from_hex("00112233445566778899aabbccddeeff").unwrap();
    let mut record = Record::new(laptop);
    for (n, (id, size)) in files.iter().enumerate() {
        record.items.push(Item {
            id: *id,
            kind: ItemKind::File,
            owner: Owner::Person {
                account_id: "1000".into(),
            },
            place: Place {
                role: FolderRole::Documents,
                storage: Storage::SystemDrive,
            },
            path: ItemPath::new(vec![ItemName::from_text(&format!("file{n}"))]).unwrap(),
            size_bytes: *size,
            portability: Portability::Portable,
            managed_by: ManagedBy::Personal,
            download_bytes: None,
            inclusion: Inclusion::Included,
        });
    }
    record.mapping.people.insert(
        "1000".into(),
        PersonTarget::ExistingAccount {
            account_id: "1001".into(),
        },
    );
    let approval = record.approve().unwrap();
    Allowance::from_record(&record, &approval).unwrap()
}

/// A change journal for a test's receiver, kept until the test run ends.
#[allow(dead_code)]
pub fn journal() -> &'static pctwin_journal::Journal {
    let dir = Box::leak(Box::new(tempfile::tempdir().unwrap()));
    Box::leak(Box::new(
        pctwin_journal::Journal::open(&dir.path().join("journal.redb")).unwrap(),
    ))
}
