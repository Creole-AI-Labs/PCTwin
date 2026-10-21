//! Files made on the old laptop after the plan was approved (Product Spec, "Changed since the
//! scan"): found when the move ends by scanning again with the same rules, counted, and offered
//! as a short follow-up move, which the person approves again like any plan. Nothing is added to
//! a plan already approved.

use std::collections::HashSet;

use pctwin_record::{Inclusion, Item, ItemId, ItemKind, Record};

/// What is new since the approved plan.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct MadeSince {
    pub files: u64,
    pub bytes: u64,
    pub items: Vec<Item>,
}

/// The files in `again` (a scan made as the move ends) that the `approved` plan does not have:
/// included files only, as the scan's rules left them.
pub fn made_since(approved: &Record, again: &Record) -> MadeSince {
    let known: HashSet<ItemId> = approved.items.iter().map(|i| i.id).collect();
    let items: Vec<Item> = again
        .items
        .iter()
        .filter(|i| {
            i.kind == ItemKind::File && i.inclusion == Inclusion::Included && !known.contains(&i.id)
        })
        .cloned()
        .collect();
    MadeSince {
        files: items.len() as u64,
        bytes: items.iter().map(|i| i.size_bytes).sum(),
        items,
    }
}

/// The follow-up plan for `made`: the next revision of `approved`, with the same people and
/// places, holding only the new files. The new laptop approves it as any plan.
pub fn follow_up(approved: &Record, made: &MadeSince) -> Record {
    let mut next = approved.next_revision();
    next.items.clone_from(&made.items);
    next
}

#[cfg(test)]
mod tests {
    use super::*;
    use pctwin_record::{
        FolderRole, ItemName, ItemPath, LaptopId, LeftOutReason, ManagedBy, Owner, Place,
        Portability, Storage,
    };

    fn laptop() -> LaptopId {
        LaptopId::from_hex("00112233445566778899aabbccddeeff").unwrap()
    }

    fn file(name: &str, size: u64, inclusion: Inclusion) -> Item {
        let owner = Owner::Person {
            account_id: "1000".into(),
        };
        let place = Place {
            role: FolderRole::Documents,
            storage: Storage::SystemDrive,
        };
        let path = ItemPath::new(vec![ItemName::from_text(name)]).unwrap();
        Item {
            id: ItemId::derive(&laptop(), &owner, &place, &path),
            kind: ItemKind::File,
            owner,
            place,
            path,
            size_bytes: size,
            modified_ns: None,
            portability: Portability::Portable,
            managed_by: ManagedBy::Personal,
            download_bytes: None,
            inclusion,
        }
    }

    #[test]
    fn new_files_are_counted_and_offered_as_a_plan_approved_again() {
        let mut approved = Record::new(laptop());
        approved.items = vec![file("old.txt", 5, Inclusion::Included)];
        approved.mapping.people.insert(
            "1000".into(),
            pctwin_record::PersonTarget::ExistingAccount {
                account_id: "1001".into(),
            },
        );
        approved.approve().unwrap();
        let mut again = approved.clone();
        again.items.push(file("new.txt", 7, Inclusion::Included));
        again.items.push(file(
            "new-but-left-out.tmp",
            9,
            Inclusion::LeftOut {
                reason: LeftOutReason::System,
            },
        ));
        let made = made_since(&approved, &again);
        assert_eq!((made.files, made.bytes), (1, 7));
        let next = follow_up(&approved, &made);
        assert_eq!(next.revision, approved.revision + 1);
        assert_eq!(next.previous, Some(approved.fingerprint()));
        assert_eq!(next.items, made.items);
        assert_eq!(next.mapping, approved.mapping);
        next.approve().unwrap();
        // Nothing new: nothing offered.
        assert_eq!(made_since(&approved, &approved), MadeSince::default());
    }
}
