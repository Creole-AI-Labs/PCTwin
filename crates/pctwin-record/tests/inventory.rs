//! The scan inventory, version 2 (docs: Engineering Plan 9.11; design 20 October 2026): a
//! manifest and pages, read as from a stranger, with a fingerprint over the bytes that crossed.

use pctwin_record::{
    CloudProvider, EncodedInventory, FolderRole, Inclusion, InventoryReader, Item, ItemId,
    ItemKind, ItemName, ItemPath, LaptopId, LeftOutReason, MAX_PAGE_BYTES, ManagedBy, Owner,
    PAGE_ITEMS, Place, Portability, RecordError, Storage, decode_inventory, encode_inventory,
};

fn laptop() -> LaptopId {
    LaptopId::from_hex("00112233445566778899aabbccddeeff").unwrap()
}

fn person(id: &str) -> Owner {
    Owner::Person {
        account_id: id.into(),
    }
}

fn place(role: FolderRole, storage: Storage) -> Place {
    Place { role, storage }
}

fn item(owner: Owner, place: Place, parts: Vec<ItemName>, n: u64) -> Item {
    let path = ItemPath::new(parts).unwrap();
    Item {
        id: ItemId::derive(&laptop(), &owner, &place, &path),
        kind: if n.is_multiple_of(7) {
            ItemKind::Folder
        } else {
            ItemKind::File
        },
        owner,
        place,
        path,
        size_bytes: n * 1000 + 7,
        portability: if n.is_multiple_of(11) {
            Portability::Rebuildable
        } else {
            Portability::Portable
        },
        managed_by: if n.is_multiple_of(13) {
            ManagedBy::Organisation
        } else {
            ManagedBy::Personal
        },
        download_bytes: (n.is_multiple_of(5)).then_some(n),
        inclusion: if n.is_multiple_of(17) {
            Inclusion::LeftOut {
                reason: LeftOutReason::CloudOnly,
            }
        } else {
            Inclusion::Included
        },
    }
}

fn text(parts: &[&str]) -> Vec<ItemName> {
    parts.iter().map(|p| ItemName::from_text(p)).collect()
}

/// Every awkward kind of name and owner and place, and a deep tree.
fn awkward() -> Vec<Item> {
    let docs = place(FolderRole::Documents, Storage::SystemDrive);
    let cloud = place(
        FolderRole::Pictures,
        Storage::Cloud {
            provider: CloudProvider::OneDrive,
        },
    );
    let drive = place(
        FolderRole::Other,
        Storage::OtherDrive { drive: "D:".into() },
    );
    let mut items = vec![
        item(person("S-1-5-21-1001"), docs.clone(), text(&["top.txt"]), 1),
        item(
            person("S-1-5-21-1001"),
            docs.clone(),
            text(&["Taxes", "2025", "return.pdf"]),
            2,
        ),
        item(
            person("S-1-5-21-1001"),
            docs.clone(),
            text(&["Taxes", "2026", "return.pdf"]),
            3,
        ),
        item(
            person("S-1-5-21-1001"),
            docs.clone(),
            text(&["Taxes", "2025", "notes.txt"]),
            4,
        ),
        item(
            Owner::Shared,
            cloud.clone(),
            text(&["Été", "写真", "مرحبا.jpg"]),
            5,
        ),
        item(Owner::System, drive.clone(), text(&["Windows", "x"]), 6),
        item(
            person("501"),
            docs.clone(),
            text(&["Taxes", "2025", "return.pdf"]),
            7,
        ),
    ];
    // Names that are not valid text, kept exactly.
    let bad_unix = ItemName::from_unix_bytes(b"caf\xe9.txt");
    let bad_win = ItemName::from_windows_wide(&[0x0061, 0xD800, 0x0062]);
    items.push(item(
        person("S-1-5-21-1001"),
        docs.clone(),
        vec![ItemName::from_text("Odd"), bad_unix],
        8,
    ));
    items.push(item(
        person("S-1-5-21-1001"),
        docs.clone(),
        vec![ItemName::from_windows_wide(&[0x004F, 0xDC00]), bad_win],
        9,
    ));
    // A deep path.
    let deep: Vec<String> = (0..60).map(|i| format!("level{i}")).collect();
    let deep: Vec<&str> = deep.iter().map(String::as_str).collect();
    items.push(item(person("S-1-5-21-1001"), docs, text(&deep), 10));
    items
}

fn many(n: u64) -> Vec<Item> {
    let docs = place(FolderRole::Documents, Storage::SystemDrive);
    (0..n)
        .map(|i| {
            let a = format!("Project {}", i / 1000);
            let b = format!("Folder {}", i / 50);
            let f = format!("file-{i}.dat");
            item(
                person("S-1-5-21-1001"),
                docs.clone(),
                text(&[&a, &b, &f]),
                i,
            )
        })
        .collect()
}

#[test]
fn every_awkward_item_comes_back_exactly() {
    let items = awkward();
    let enc = encode_inventory(&laptop(), &items).unwrap();
    let (from, back, fp) = decode_inventory(&enc).unwrap();
    assert_eq!(from, laptop());
    assert_eq!(back, items);
    assert_eq!(fp, enc.fingerprint());
}

#[test]
fn encoding_what_was_read_gives_the_same_bytes() {
    let items = many(25_000);
    let enc = encode_inventory(&laptop(), &items).unwrap();
    let (_, back, _) = decode_inventory(&enc).unwrap();
    assert_eq!(encode_inventory(&laptop(), &back).unwrap(), enc);
}

#[test]
fn pages_hold_at_most_their_share_and_the_whole_is_much_smaller_than_json() {
    let items = many(300_000);
    let enc = encode_inventory(&laptop(), &items).unwrap();
    assert_eq!(enc.pages.len(), 300_000_usize.div_ceil(PAGE_ITEMS));
    assert!(enc.pages.iter().all(|p| p.len() <= MAX_PAGE_BYTES));
    let binary: usize = enc.manifest.len() + enc.pages.iter().map(Vec::len).sum::<usize>();
    let json = serde_json::to_vec(&items).unwrap().len();
    assert!(
        binary * 3 < json,
        "binary {binary} bytes against JSON {json}"
    );
    assert_eq!(decode_inventory(&enc).unwrap().1.len(), 300_000);
}

#[test]
fn a_page_ends_early_rather_than_grow_past_its_size() {
    // Long names: 10,000 of these would pass 8 MB.
    let docs = place(FolderRole::Documents, Storage::SystemDrive);
    let long = "n".repeat(1500);
    let items: Vec<Item> = (0..12_000)
        .map(|i| {
            item(
                person("S-1"),
                docs.clone(),
                text(&[&format!("{long}{i}")]),
                i,
            )
        })
        .collect();
    let enc = encode_inventory(&laptop(), &items).unwrap();
    assert!(enc.pages.len() > 2);
    assert!(enc.pages.iter().all(|p| p.len() <= MAX_PAGE_BYTES));
    assert_eq!(decode_inventory(&enc).unwrap().1, items);
}

#[test]
fn no_items_is_an_empty_inventory() {
    let enc = encode_inventory(&laptop(), &[]).unwrap();
    assert!(enc.pages.is_empty());
    let (_, items, _) = decode_inventory(&enc).unwrap();
    assert!(items.is_empty());
}

// ---------- the fingerprint ----------

#[test]
fn the_fingerprint_is_of_the_bytes_and_any_change_moves_it() {
    let enc = encode_inventory(&laptop(), &many(20_000)).unwrap();
    assert_eq!(enc.fingerprint(), enc.clone().fingerprint());
    let mut changed = enc.clone();
    let last = changed.pages[1].len() - 1;
    changed.pages[1][last] ^= 1;
    assert_ne!(changed.fingerprint(), enc.fingerprint());
    let mut moved = enc.clone();
    moved.manifest[20] ^= 1;
    assert_ne!(moved.fingerprint(), enc.fingerprint());
}

// ---------- read as from a stranger ----------

fn refused(enc: &EncodedInventory) -> RecordError {
    decode_inventory(enc).expect_err("a broken inventory is refused")
}

#[test]
fn cut_short_anywhere_it_is_refused_without_a_panic() {
    let enc = encode_inventory(&laptop(), &awkward()).unwrap();
    for n in 0..enc.manifest.len() {
        let mut cut = enc.clone();
        cut.manifest.truncate(n);
        refused(&cut);
    }
    for n in 0..enc.pages[0].len() {
        let mut cut = enc.clone();
        cut.pages[0].truncate(n);
        refused(&cut);
    }
}

#[test]
fn every_single_byte_changed_is_refused_or_read_without_a_panic() {
    let enc = encode_inventory(&laptop(), &awkward()).unwrap();
    for i in 0..enc.pages[0].len() {
        for flip in [0x01, 0x80, 0xff] {
            let mut m = enc.clone();
            m.pages[0][i] ^= flip;
            let _ = decode_inventory(&m);
        }
    }
    for i in 0..enc.manifest.len() {
        let mut m = enc.clone();
        m.manifest[i] ^= 0xff;
        let _ = decode_inventory(&m);
    }
}

#[test]
fn bytes_after_the_end_are_refused() {
    let enc = encode_inventory(&laptop(), &awkward()).unwrap();
    let mut longer = enc.clone();
    longer.manifest.push(0);
    refused(&longer);
    let mut longer = enc;
    longer.pages[0].push(0);
    refused(&longer);
}

#[test]
fn pages_missing_extra_or_out_of_order_are_refused() {
    let enc = encode_inventory(&laptop(), &many(25_000)).unwrap();
    assert_eq!(enc.pages.len(), 3);
    let mut missing = enc.clone();
    missing.pages.pop();
    refused(&missing);
    let mut swapped = enc.clone();
    swapped.pages.swap(0, 1);
    refused(&swapped);
    let mut extra = enc.clone();
    extra.pages.push(enc.pages[2].clone());
    refused(&extra);
}

#[test]
fn the_same_item_twice_across_pages_is_refused() {
    let mut items = many(10_000);
    let again = items[0].clone();
    items.push(again);
    let enc = encode_inventory(&laptop(), &items).unwrap();
    assert_eq!(enc.pages.len(), 2);
    assert!(matches!(refused(&enc), RecordError::DuplicateId));
}

#[test]
fn a_newer_format_says_so() {
    let mut enc = encode_inventory(&laptop(), &awkward()).unwrap();
    enc.manifest[8..12].copy_from_slice(&3u32.to_be_bytes());
    assert!(matches!(
        refused(&enc),
        RecordError::NewerFormat { found: 3 }
    ));
}

#[test]
fn a_page_larger_than_allowed_is_refused_before_it_is_read() {
    let enc = encode_inventory(&laptop(), &awkward()).unwrap();
    let mut reader = InventoryReader::start(&enc.manifest).unwrap();
    let mut big = enc.pages[0].clone();
    big.resize(MAX_PAGE_BYTES + 1, 0);
    assert!(matches!(
        reader.page(&big),
        Err(RecordError::BadInventory(_))
    ));
}

/// A hand-built page: header, folders, then items, each field in order.
fn page(number: u32, first_folder: u32, folders: &[(u32, &str)], items: &[[u32; 3]]) -> Vec<u8> {
    let mut p = Vec::new();
    p.extend_from_slice(&number.to_be_bytes());
    p.extend_from_slice(&first_folder.to_be_bytes());
    p.extend_from_slice(&(folders.len() as u32).to_be_bytes());
    p.extend_from_slice(&(items.len() as u32).to_be_bytes());
    let name = |p: &mut Vec<u8>, s: &str| {
        p.push(0);
        p.extend_from_slice(&(s.len() as u32).to_be_bytes());
        p.extend_from_slice(s.as_bytes());
    };
    for (parent, n) in folders {
        p.extend_from_slice(&parent.to_be_bytes());
        name(&mut p, n);
    }
    for (k, [owner, place, folder]) in items.iter().enumerate() {
        p.extend_from_slice(&[k as u8 + 1; 16]);
        p.push(0); // a file
        p.extend_from_slice(&owner.to_be_bytes());
        p.extend_from_slice(&place.to_be_bytes());
        p.extend_from_slice(&folder.to_be_bytes());
        name(&mut p, "f.txt");
        p.extend_from_slice(&5u64.to_be_bytes());
        p.extend_from_slice(&[0, 0, 0, 0]); // portable, personal, no download, included
    }
    p
}

/// The manifest of an inventory with `items` items in one page, owned by one person, in one place.
fn manifest(items: u64) -> Vec<u8> {
    let docs = place(FolderRole::Documents, Storage::SystemDrive);
    let one = item(person("S-1"), docs, text(&["a"]), 1);
    let mut m = encode_inventory(&laptop(), &[one]).unwrap().manifest;
    m[28..36].copy_from_slice(&items.to_be_bytes());
    m
}

#[test]
fn indexes_out_of_range_and_folders_that_loop_are_refused() {
    const ROOT: u32 = u32::MAX;
    let ok = page(0, 0, &[(ROOT, "A"), (0, "B")], &[[0, 0, 1]]);
    let good = EncodedInventory {
        manifest: manifest(1),
        pages: vec![ok],
    };
    let (_, items, _) = decode_inventory(&good).unwrap();
    assert_eq!(
        items[0].path,
        ItemPath::new(text(&["A", "B", "f.txt"])).unwrap()
    );
    for bad in [
        page(0, 0, &[], &[[1, 0, ROOT]]),      // no second owner
        page(0, 0, &[], &[[0, 1, ROOT]]),      // no second place
        page(0, 0, &[], &[[0, 0, 0]]),         // no folder 0
        page(0, 0, &[(0, "A")], &[[0, 0, 0]]), // a folder its own parent
        page(0, 0, &[(1, "A"), (ROOT, "B")], &[[0, 0, 0]]), // a parent after its folder
        page(0, 1, &[(ROOT, "A")], &[[0, 0, 0]]), // folders that do not follow
        page(1, 0, &[], &[[0, 0, ROOT]]),      // the wrong page number
        page(0, 0, &[(ROOT, "..")], &[[0, 0, 0]]), // a name that breaks the rules
    ] {
        let enc = EncodedInventory {
            manifest: manifest(1),
            pages: vec![bad],
        };
        assert!(
            matches!(refused(&enc), RecordError::BadInventory(_)),
            "{enc:?}"
        );
    }
}

#[test]
fn more_or_fewer_items_than_announced_are_refused() {
    let p = page(0, 0, &[], &[[0, 0, u32::MAX], [0, 0, u32::MAX]]);
    for announced in [1, 3] {
        let enc = EncodedInventory {
            manifest: manifest(announced),
            pages: vec![p.clone()],
        };
        refused(&enc);
    }
}

/// A name is written one way only: its exact bytes (or UTF-16 units) are carried only when they
/// are not valid text. Exact bytes that are valid text are a second spelling of a plain name, so
/// they are refused, for Mac and Linux bytes and for Windows units alike.
#[test]
fn an_exact_name_that_is_valid_text_is_refused() {
    let one = page(0, 0, &[], &[[0, 0, u32::MAX]]);
    let plain_at = one.len() - (1 + 4 + 5) - 8 - 4;
    // The plain name "f.txt" written as exact Mac and Linux bytes (kind 1).
    let mut unix = one[..plain_at].to_vec();
    unix.push(1);
    unix.extend_from_slice(&5u32.to_be_bytes());
    unix.extend_from_slice(b"f.txt");
    unix.extend_from_slice(&one[plain_at + 10..]);
    // And as exact Windows units (kind 2).
    let mut win = one[..plain_at].to_vec();
    win.push(2);
    win.extend_from_slice(&5u32.to_be_bytes());
    for u in "f.txt".encode_utf16() {
        win.extend_from_slice(&u.to_be_bytes());
    }
    win.extend_from_slice(&one[plain_at + 10..]);
    for spelled in [unix, win] {
        let enc = EncodedInventory {
            manifest: manifest(1),
            pages: vec![spelled],
        };
        assert!(matches!(refused(&enc), RecordError::BadInventory(_)));
    }
    // The page as written is read.
    let enc = EncodedInventory {
        manifest: manifest(1),
        pages: vec![one],
    };
    decode_inventory(&enc).unwrap();
}
