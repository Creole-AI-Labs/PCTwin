//! The new laptop holds the plan the person approved (Security Design A, fresh-context review of
//! 8 October 2026): a file starts only if it is an included item of that exact approved revision,
//! no bigger than its approved size allows, and within the plan's total. The old laptop's word
//! for a size is never enough to reserve the new laptop's disk.

use pctwin_record::{
    FolderRole, Inclusion, Item, ItemId, ItemKind, ItemName, ItemPath, LaptopId, LeftOutReason,
    ManagedBy, Owner, PersonTarget, Place, Portability, Record, Storage,
};
use pctwin_transfer::{Allowance, Refusal};

const MIB: u64 = 1024 * 1024;

fn item(n: u8, size: u64, kind: ItemKind, inclusion: Inclusion) -> Item {
    let laptop = LaptopId::from_hex("00112233445566778899aabbccddeeff").unwrap();
    let owner = Owner::Person {
        account_id: "1000".into(),
    };
    let place = Place {
        role: FolderRole::Documents,
        storage: Storage::SystemDrive,
    };
    let path = ItemPath::new(vec![ItemName::from_text(&format!("f{n}"))]).unwrap();
    Item {
        id: ItemId::derive(&laptop, &owner, &place, &path),
        kind,
        owner,
        place,
        path,
        size_bytes: size,
        portability: Portability::Portable,
        managed_by: ManagedBy::Personal,
        download_bytes: None,
        inclusion,
    }
}

fn record(items: Vec<Item>) -> Record {
    let mut r = Record::new(LaptopId::from_hex("00112233445566778899aabbccddeeff").unwrap());
    r.items = items;
    r.mapping.people.insert(
        "1000".into(),
        PersonTarget::ExistingAccount {
            account_id: "1001".into(),
        },
    );
    r
}

#[test]
fn only_included_files_of_the_approved_revision_are_allowed() {
    let file = item(1, 100 * MIB, ItemKind::File, Inclusion::Included);
    let left_out = item(
        2,
        MIB,
        ItemKind::File,
        Inclusion::LeftOut {
            reason: LeftOutReason::ChosenByUser,
        },
    );
    let folder = item(3, 0, ItemKind::Folder, Inclusion::Included);
    let r = record(vec![file.clone(), left_out.clone(), folder.clone()]);
    let approval = r.approve().unwrap();
    let mut a = Allowance::from_record(&r, &approval).unwrap();
    assert_eq!(a.admit(file.id, 100 * MIB), Ok(()));
    assert_eq!(a.admit(left_out.id, MIB), Err(Refusal::NotInPlan));
    assert_eq!(a.admit(folder.id, 0), Err(Refusal::NotInPlan));
    let stranger = item(9, MIB, ItemKind::File, Inclusion::Included);
    assert_eq!(a.admit(stranger.id, MIB), Err(Refusal::NotInPlan));
}

#[test]
fn an_approval_for_another_revision_gives_no_allowance() {
    let r = record(vec![item(1, MIB, ItemKind::File, Inclusion::Included)]);
    let approval = r.approve().unwrap();
    let mut changed = r.next_revision();
    changed.items[0].size_bytes = 50 * MIB;
    assert!(Allowance::from_record(&changed, &approval).is_err());
}

#[test]
fn a_file_may_grow_a_little_since_the_plan_but_not_a_lot() {
    let big = item(1, 100 * MIB, ItemKind::File, Inclusion::Included);
    let small = item(2, 1000, ItemKind::File, Inclusion::Included);
    let r = record(vec![big.clone(), small.clone()]);
    let mut a = Allowance::from_record(&r, &r.approve().unwrap()).unwrap();
    // 10% more is fine; any more is refused.
    assert_eq!(a.admit(big.id, 110 * MIB), Ok(()));
    let mut b = Allowance::from_record(&r, &r.approve().unwrap()).unwrap();
    assert_eq!(
        b.admit(big.id, 110 * MIB + 1),
        Err(Refusal::GrewTooMuch {
            approved: 100 * MIB,
            announced: 110 * MIB + 1
        })
    );
    // A small file has at least 1 MiB of room (a document saved again).
    assert_eq!(a.admit(small.id, 1000 + MIB), Ok(()));
    let mut c = Allowance::from_record(&r, &r.approve().unwrap()).unwrap();
    assert!(matches!(
        c.admit(small.id, 1001 + MIB),
        Err(Refusal::GrewTooMuch { .. })
    ));
    // Shrinking is always fine.
    let mut d = Allowance::from_record(&r, &r.approve().unwrap()).unwrap();
    assert_eq!(d.admit(big.id, 0), Ok(()));
}

#[test]
fn the_room_for_growth_cannot_be_used_over_and_over_to_fill_the_disk() {
    // 10,000 tiny files: each may grow by 1 MiB, but together not past the plan's total room.
    let items: Vec<Item> = (0..10_000u32)
        .map(|n| {
            let mut i = item(0, 10, ItemKind::File, Inclusion::Included);
            i.path = ItemPath::new(vec![ItemName::from_text(&format!("t{n}"))]).unwrap();
            i.id = ItemId::derive(
                &LaptopId::from_hex("00112233445566778899aabbccddeeff").unwrap(),
                &i.owner,
                &i.place,
                &i.path,
            );
            i
        })
        .collect();
    let total: u64 = items.iter().map(|i| i.size_bytes).sum();
    let r = record(items.clone());
    let mut a = Allowance::from_record(&r, &r.approve().unwrap()).unwrap();
    let mut admitted = 0u64;
    let mut refused = false;
    for i in &items {
        match a.admit(i.id, 10 + MIB) {
            Ok(()) => admitted += 10 + MIB,
            Err(Refusal::OverTotal) => {
                refused = true;
                break;
            }
            Err(e) => panic!("{e:?}"),
        }
    }
    assert!(refused);
    assert!(admitted <= total + total / 10 + 64 * MIB, "{admitted}");
}

#[test]
fn a_file_sent_again_counts_once_toward_the_total() {
    let f = item(1, 100 * MIB, ItemKind::File, Inclusion::Included);
    let r = record(vec![f.clone()]);
    let mut a = Allowance::from_record(&r, &r.approve().unwrap()).unwrap();
    // It changed while being read and is sent again, many times.
    for _ in 0..50 {
        assert_eq!(a.admit(f.id, 100 * MIB), Ok(()));
    }
    assert_eq!(a.started_bytes(), 100 * MIB);
    // Sent again bigger (still within its room): the larger size counts, once.
    assert_eq!(a.admit(f.id, 105 * MIB), Ok(()));
    assert_eq!(a.started_bytes(), 105 * MIB);
    assert_eq!(a.admit(f.id, 90 * MIB), Ok(()));
    assert_eq!(a.started_bytes(), 105 * MIB);
}

/// `n` plan files named `prefix0`, `prefix1`, ..., each `size` bytes.
fn files(prefix: &str, n: u32, size: u64) -> Vec<Item> {
    (0..n)
        .map(|k| {
            let mut i = item(0, size, ItemKind::File, Inclusion::Included);
            i.path = ItemPath::new(vec![ItemName::from_text(&format!("{prefix}{k}"))]).unwrap();
            i.id = ItemId::derive(
                &LaptopId::from_hex("00112233445566778899aabbccddeeff").unwrap(),
                &i.owner,
                &i.place,
                &i.path,
            );
            i
        })
        .collect()
}

#[test]
fn every_file_of_a_big_plan_may_use_its_own_room_to_grow() {
    // Ten 1 GiB files that each grew by 10%: 1 GiB more in all, which the plan's tenth covers.
    let items = files("v", 10, 1024 * MIB);
    let r = record(items.clone());
    let mut a = Allowance::from_record(&r, &r.approve().unwrap()).unwrap();
    for i in &items {
        assert_eq!(a.admit(i.id, 1024 * MIB + 1024 * MIB / 10), Ok(()));
    }
}

#[test]
fn the_whole_move_may_reach_its_limit_exactly_and_no_further() {
    // One 1,000 MiB file and 65 empty ones: the move's limit is 1,000 + 100 + 64 MiB.
    let big = files("big", 1, 1000 * MIB);
    let empty = files("e", 65, 0);
    let r = record(big.iter().chain(&empty).cloned().collect());
    let mut a = Allowance::from_record(&r, &r.approve().unwrap()).unwrap();
    assert_eq!(a.admit(big[0].id, 1100 * MIB), Ok(()));
    // Each empty file may grow to 1 MiB: 64 of them reach the limit exactly.
    for e in &empty[..64] {
        assert_eq!(a.admit(e.id, MIB), Ok(()));
    }
    assert_eq!(a.started_bytes(), 1164 * MIB);
    assert_eq!(a.admit(empty[64].id, MIB), Err(Refusal::OverTotal));
    // Nothing more was counted for the refused one.
    assert_eq!(a.started_bytes(), 1164 * MIB);
}
