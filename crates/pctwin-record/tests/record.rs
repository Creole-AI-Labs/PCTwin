//! The shared move record (Task List 0.6, decided 7 October 2026): one record for every item found,
//! one mapping table for where things go, kept as numbered revisions, with a plan approval bound to
//! exactly one revision.

use pctwin_record::{
    Approval, CloudProvider, FORMAT, FolderRole, Inclusion, Item, ItemId, ItemKind, ItemName,
    ItemPath, LaptopId, LeftOutReason, ManagedBy, Owner, PersonTarget, Place, Portability, RawName,
    Record, RecordError, SharedTarget, Storage,
};

fn laptop() -> LaptopId {
    LaptopId::from_hex("00112233445566778899aabbccddeeff").unwrap()
}

fn person(id: &str) -> Owner {
    Owner::Person {
        account_id: id.into(),
    }
}

fn docs() -> Place {
    Place {
        role: FolderRole::Documents,
        storage: Storage::SystemDrive,
    }
}

fn path(parts: &[&str]) -> ItemPath {
    ItemPath::new(parts.iter().map(|p| ItemName::from_text(p)).collect()).unwrap()
}

fn file(owner: Owner, parts: &[&str]) -> Item {
    let place = docs();
    let path = path(parts);
    Item {
        id: ItemId::derive(&laptop(), &owner, &place, &path),
        kind: ItemKind::File,
        owner,
        place,
        path,
        size_bytes: 10,
        portability: Portability::Portable,
        managed_by: ManagedBy::Personal,
        download_bytes: None,
        inclusion: Inclusion::Included,
    }
}

fn record_with(items: Vec<Item>) -> Record {
    let mut record = Record::new(laptop());
    record.items = items;
    record.mapping.people.insert(
        "S-1-5-21-1".into(),
        PersonTarget::ExistingAccount {
            account_id: "ada".into(),
        },
    );
    record
}

fn ada_file(name: &str) -> Item {
    file(person("S-1-5-21-1"), &["Taxes", name])
}

// ---------- permanent IDs ----------

#[test]
fn an_item_keeps_its_id_when_the_laptop_is_scanned_again() {
    let a = ItemId::derive(&laptop(), &person("S-1"), &docs(), &path(&["a.txt"]));
    let b = ItemId::derive(&laptop(), &person("S-1"), &docs(), &path(&["a.txt"]));
    assert_eq!(a, b);
}

#[test]
fn ids_differ_by_laptop_owner_place_and_path() {
    let base = ItemId::derive(&laptop(), &person("S-1"), &docs(), &path(&["a.txt"]));
    let other_laptop = LaptopId::from_hex("ffeeddccbbaa99887766554433221100").unwrap();
    let cloud = Place {
        role: FolderRole::Documents,
        storage: Storage::Cloud {
            provider: CloudProvider::OneDrive,
        },
    };
    for other in [
        ItemId::derive(&other_laptop, &person("S-1"), &docs(), &path(&["a.txt"])),
        ItemId::derive(&laptop(), &person("S-2"), &docs(), &path(&["a.txt"])),
        ItemId::derive(&laptop(), &Owner::Shared, &docs(), &path(&["a.txt"])),
        ItemId::derive(&laptop(), &person("S-1"), &cloud, &path(&["a.txt"])),
        ItemId::derive(&laptop(), &person("S-1"), &docs(), &path(&["b.txt"])),
        ItemId::derive(&laptop(), &person("S-1"), &docs(), &path(&["a", "txt"])),
    ] {
        assert_ne!(base, other);
    }
}

#[test]
fn new_laptop_ids_are_random() {
    let a = LaptopId::generate().unwrap();
    let b = LaptopId::generate().unwrap();
    assert_ne!(a, b);
    assert_eq!(LaptopId::from_hex(&a.to_hex()).unwrap(), a);
    assert!(LaptopId::from_hex("xyz").is_err());
    assert!(LaptopId::from_hex("0011").is_err());
}

// ---------- names stored exactly ----------

#[test]
fn ordinary_names_are_kept_as_text() {
    let name = ItemName::from_text("Résumé.pdf");
    assert_eq!(name.display(), "Résumé.pdf");
    assert!(name.exact().is_none());
    // Not normalised: the name is kept as the disk had it.
    let decomposed = ItemName::from_text("Re\u{301}sume\u{301}.pdf");
    assert_eq!(decomposed.display(), "Re\u{301}sume\u{301}.pdf");
    assert_eq!(ItemName::from_unix_bytes("a.txt".as_bytes()).exact(), None);
    let wide: Vec<u16> = "a.txt".encode_utf16().collect();
    assert_eq!(ItemName::from_windows_wide(&wide).exact(), None);
}

#[test]
fn names_that_are_not_valid_text_keep_their_exact_bytes() {
    let unix = ItemName::from_unix_bytes(b"caf\xe9.txt");
    assert_eq!(unix.display(), "caf\u{FFFD}.txt");
    assert_eq!(unix.exact(), Some(&RawName::Unix(b"caf\xe9.txt".to_vec())));

    let wide = vec![0x61, 0xD800, 0x62];
    let windows = ItemName::from_windows_wide(&wide);
    assert_eq!(windows.display(), "a\u{FFFD}b");
    assert_eq!(windows.exact(), Some(&RawName::Windows(wide.clone())));

    // Both survive saving and loading.
    let record = record_with(vec![file(person("S-1-5-21-1"), &["x"])]);
    let mut record = record;
    let p = ItemPath::new(vec![unix.clone(), windows.clone()]).unwrap();
    record.items[0].path = p.clone();
    let back = Record::from_json(&record.to_json()).unwrap();
    assert_eq!(back.items[0].path, p);
}

#[test]
fn two_names_that_look_the_same_but_differ_in_bytes_are_different() {
    let a = ItemName::from_unix_bytes(b"caf\xe9");
    let b = ItemName::from_unix_bytes(b"caf\xea");
    assert_eq!(a.display(), b.display());
    assert_ne!(a, b);
    let ia = ItemId::derive(
        &laptop(),
        &Owner::System,
        &docs(),
        &ItemPath::new(vec![a]).unwrap(),
    );
    let ib = ItemId::derive(
        &laptop(),
        &Owner::System,
        &docs(),
        &ItemPath::new(vec![b]).unwrap(),
    );
    assert_ne!(ia, ib);
}

#[test]
fn path_parts_that_could_climb_out_are_refused() {
    for bad in ["", ".", "..", "a/b", "a\0b"] {
        assert!(
            matches!(
                ItemPath::new(vec![ItemName::from_text(bad)]),
                Err(RecordError::BadName)
            ),
            "{bad:?}"
        );
    }
    assert!(matches!(ItemPath::new(vec![]), Err(RecordError::BadName)));
    // A backslash is an ordinary character in Mac and Linux names.
    ItemPath::new(vec![ItemName::from_text("a\\b")]).unwrap();
}

// ---------- saving, loading and format versions ----------

#[test]
fn a_record_survives_saving_and_loading() {
    let mut record = record_with(vec![ada_file("a.pdf"), ada_file("b.pdf")]);
    record.items[1].inclusion = Inclusion::LeftOut {
        reason: LeftOutReason::ChosenByUser,
    };
    let back = Record::from_json(&record.to_json()).unwrap();
    assert_eq!(back, record);
    assert_eq!(back.fingerprint(), record.fingerprint());
}

#[test]
fn a_record_from_a_newer_app_is_refused_safely() {
    let json = String::from_utf8(record_with(vec![]).to_json()).unwrap();
    let newer = json.replacen(&format!("\"format\":{FORMAT}"), "\"format\":99", 1);
    assert_ne!(newer, json);
    assert!(matches!(
        Record::from_json(newer.as_bytes()),
        Err(RecordError::NewerFormat { found: 99 })
    ));
    // Even when the rest has changed shape.
    assert!(matches!(
        Record::from_json(br#"{"format":2,"everything":"different"}"#),
        Err(RecordError::NewerFormat { found: 2 })
    ));
    for broken in [&b"{}"[..], b"[]", b"not json", br#"{"format":0}"#] {
        assert!(matches!(
            Record::from_json(broken),
            Err(RecordError::BadFormat(_))
        ));
    }
}

#[test]
fn unknown_kinds_and_extra_fields_are_refused() {
    let json = String::from_utf8(record_with(vec![ada_file("a.pdf")]).to_json()).unwrap();
    let unknown_kind = json.replacen("\"file\"", "\"teleporter\"", 1);
    assert!(matches!(
        Record::from_json(unknown_kind.as_bytes()),
        Err(RecordError::BadFormat(_))
    ));
    let extra = json.replacen("{", "{\"surprise\":1,", 1);
    assert!(matches!(
        Record::from_json(extra.as_bytes()),
        Err(RecordError::BadFormat(_))
    ));
}

#[test]
fn a_loaded_record_is_checked_like_a_new_one() {
    let mut record = record_with(vec![ada_file("a.pdf")]);
    record.items.push(record.items[0].clone());
    let json = record.to_json();
    assert!(matches!(
        Record::from_json(&json),
        Err(RecordError::DuplicateId)
    ));
}

#[test]
fn a_name_whose_exact_bytes_and_text_disagree_is_refused() {
    let mut record = record_with(vec![ada_file("a.pdf")]);
    record.items[0].path = ItemPath::new(vec![ItemName::from_unix_bytes(b"caf\xe9")]).unwrap();
    let json = String::from_utf8(record.to_json()).unwrap();
    let forged = json.replacen("caf\u{FFFD}", "invoice", 1);
    assert_ne!(forged, json);
    assert!(Record::from_json(forged.as_bytes()).is_err());
}

// ---------- rules every record keeps ----------

#[test]
fn every_item_id_appears_once() {
    let mut record = record_with(vec![ada_file("a.pdf")]);
    record.items.push(record.items[0].clone());
    assert!(matches!(record.validate(), Err(RecordError::DuplicateId)));
}

#[test]
fn things_tied_to_the_old_laptop_are_never_included() {
    let mut record = record_with(vec![ada_file("authenticator")]);
    record.items[0].portability = Portability::DeviceBound;
    assert!(matches!(
        record.validate(),
        Err(RecordError::DeviceBoundIncluded)
    ));
    record.items[0].inclusion = Inclusion::LeftOut {
        reason: LeftOutReason::DeviceBound,
    };
    record.validate().unwrap();
}

#[test]
fn two_people_never_land_in_one_account() {
    let mut record = record_with(vec![ada_file("a.pdf"), file(person("S-2"), &["b.pdf"])]);
    record.mapping.people.insert(
        "S-2".into(),
        PersonTarget::ExistingAccount {
            account_id: "ada".into(),
        },
    );
    assert!(matches!(
        record.validate(),
        Err(RecordError::AccountUsedTwice)
    ));
    record.mapping.people.insert(
        "S-2".into(),
        PersonTarget::NewAccount {
            name: "tunde".into(),
        },
    );
    record.validate().unwrap();
    // Nor two new accounts with one name.
    record.mapping.people.insert(
        "S-1-5-21-1".into(),
        PersonTarget::NewAccount {
            name: "Tunde".into(),
        },
    );
    assert!(matches!(
        record.validate(),
        Err(RecordError::AccountUsedTwice)
    ));
}

#[test]
fn shared_items_may_go_to_a_mapped_persons_account() {
    let mut record = record_with(vec![
        ada_file("a.pdf"),
        file(Owner::Shared, &["Public", "p.jpg"]),
    ]);
    record.mapping.shared = SharedTarget::Account {
        account_id: "ada".into(),
    };
    record.validate().unwrap();
}

#[test]
fn the_mapping_only_names_people_in_the_record() {
    let mut record = record_with(vec![ada_file("a.pdf")]);
    record
        .mapping
        .people
        .insert("S-9".into(), PersonTarget::Skip);
    assert!(matches!(record.validate(), Err(RecordError::UnknownPerson)));
}

// ---------- revisions and approval ----------

#[test]
fn a_plan_cannot_be_approved_until_everyone_is_mapped() {
    let mut record = record_with(vec![ada_file("a.pdf"), file(person("S-2"), &["b.pdf"])]);
    assert!(matches!(
        record.approve(),
        Err(RecordError::Unmapped { account_id }) if account_id == "S-2"
    ));
    record
        .mapping
        .people
        .insert("S-2".into(), PersonTarget::Skip);
    record.approve().unwrap();
}

#[test]
fn someone_with_nothing_included_need_not_be_mapped() {
    let mut item = file(person("S-2"), &["b.pdf"]);
    item.inclusion = Inclusion::LeftOut {
        reason: LeftOutReason::WorkManaged,
    };
    let record = record_with(vec![ada_file("a.pdf"), item]);
    record.approve().unwrap();
}

#[test]
fn an_approval_holds_only_for_the_revision_it_was_given_for() {
    let record = record_with(vec![ada_file("a.pdf")]);
    let approval = record.approve().unwrap();
    record.check(&approval).unwrap();

    // A new revision needs a new approval.
    let mut next = record.next_revision();
    assert_eq!(next.revision, record.revision + 1);
    assert_eq!(next.previous, Some(record.fingerprint()));
    assert!(matches!(
        next.check(&approval),
        Err(RecordError::StaleApproval)
    ));
    next.items[0].size_bytes = 99;
    let second = next.approve().unwrap();
    next.check(&second).unwrap();
    assert!(matches!(
        record.check(&second),
        Err(RecordError::StaleApproval)
    ));
}

#[test]
fn changing_a_record_without_a_new_revision_breaks_its_approval() {
    let mut record = record_with(vec![ada_file("a.pdf")]);
    let approval = record.approve().unwrap();
    record.items[0].inclusion = Inclusion::LeftOut {
        reason: LeftOutReason::ChosenByUser,
    };
    assert!(matches!(
        record.check(&approval),
        Err(RecordError::StaleApproval)
    ));
    record.items[0].inclusion = Inclusion::Included;
    record.mapping.shared = SharedTarget::Skip;
    assert!(matches!(
        record.check(&approval),
        Err(RecordError::StaleApproval)
    ));
}

#[test]
fn an_approval_survives_saving_and_loading() {
    let record = record_with(vec![ada_file("a.pdf")]);
    let approval = record.approve().unwrap();
    let json = serde_json::to_vec(&approval).unwrap();
    let back: Approval = serde_json::from_slice(&json).unwrap();
    assert_eq!(back, approval);
    Record::from_json(&record.to_json())
        .unwrap()
        .check(&back)
        .unwrap();
}

#[test]
fn the_fingerprint_depends_on_every_part_of_the_record() {
    let base = record_with(vec![ada_file("a.pdf")]);
    let f = base.fingerprint();
    let mut changed: Vec<Record> = Vec::new();
    let mut r = base.clone();
    r.revision += 1;
    changed.push(r);
    let mut r = base.clone();
    r.source_laptop = LaptopId::from_hex("ffeeddccbbaa99887766554433221100").unwrap();
    changed.push(r);
    let mut r = base.clone();
    r.items[0].download_bytes = Some(1);
    changed.push(r);
    let mut r = base.clone();
    r.items[0].managed_by = ManagedBy::Organisation;
    changed.push(r);
    let mut r = base.clone();
    r.items[0].kind = ItemKind::Folder;
    changed.push(r);
    let mut r = base.clone();
    r.mapping
        .people
        .insert("S-1-5-21-1".into(), PersonTarget::Skip);
    changed.push(r);
    for (i, r) in changed.iter().enumerate() {
        assert_ne!(r.fingerprint(), f, "change {i} was not noticed");
    }
}

// ---------- stable across app versions ----------

/// IDs and fingerprints must never change between app versions, or "bring me up to date" and
/// resume would lose track of items. A change here needs a new FORMAT.
#[test]
fn ids_and_fingerprints_never_change_between_versions() {
    let item = ItemId::derive(&laptop(), &person("S-1"), &docs(), &path(&["a.txt"]));
    let record = record_with(vec![ada_file("a.pdf")]);
    assert_eq!(item.to_hex(), "0b59c47693e9a1719a3fecba9ae8b57a");
    assert_eq!(
        record.fingerprint().to_hex(),
        "b9e1dd2c2fa8b0fed192a539ae8667b2c87526cb9cad393a966766eae96f6f70"
    );
}
