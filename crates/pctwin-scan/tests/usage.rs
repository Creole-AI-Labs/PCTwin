//! What this person actually uses (Task List 1.5, personal essentials): read from the records the
//! old laptop already keeps for the person, never from anyone's file contents. Used only to set
//! the order of the move; nothing here leaves the old laptop.

use std::collections::BTreeMap;
use std::path::{Path, PathBuf};

use pctwin_record::{
    FolderRole, Inclusion, Item, ItemId, ItemKind, ItemName, ItemPath, LaptopId, ManagedBy, Owner,
    Place, Portability, Storage,
};
use pctwin_scan::{
    Usage, bookmarks_from_gtk, parse_iso_utc_ns, personal_essentials,
    pinned_from_finder_favourites, pinned_from_quick_access, read_usage, recent_from_mdfind,
    recent_from_xbel,
};

#[test]
fn linux_recent_files_are_read_from_recently_used_xbel() {
    let xml = r#"<?xml version="1.0" encoding="UTF-8"?>
<xbel version="1.0" xmlns:bookmark="http://www.freedesktop.org/standards/desktop-bookmarks">
  <bookmark href="file:///home/ada/Documents/Taxes/return%202026.pdf" added="2026-09-01T10:00:00Z" modified="2026-10-01T09:30:00.123456Z" visited="2026-10-02T08:00:00Z">
    <info/>
  </bookmark>
  <bookmark href="file:///home/ada/Pictures/beach.jpg" added="2026-08-01T10:00:00Z" modified="2026-08-01T10:00:00Z" visited="2026-08-03T10:00:00Z"/>
  <bookmark href="https://example.com/page" added="2026-08-01T10:00:00Z" modified="2026-08-01T10:00:00Z" visited="2026-08-01T10:00:00Z"/>
</xbel>"#;
    let recent = recent_from_xbel(xml);
    assert_eq!(recent.len(), 2, "web addresses are not files");
    assert_eq!(
        recent[0].0,
        PathBuf::from("/home/ada/Documents/Taxes/return 2026.pdf")
    );
    // The latest of when it was modified or visited.
    assert_eq!(recent[0].1, parse_iso_utc_ns("2026-10-02T08:00:00Z"));
    assert!(recent_from_xbel("not xml").is_empty());
}

#[test]
fn iso_times_are_read_exactly() {
    assert_eq!(parse_iso_utc_ns("1970-01-01T00:00:00Z"), Some(0));
    assert_eq!(
        parse_iso_utc_ns("1970-01-02T00:00:01Z"),
        Some(86_401_000_000_000)
    );
    assert_eq!(
        parse_iso_utc_ns("2026-10-01T09:30:00.5Z"),
        Some(1_790_847_000_500_000_000)
    );
    assert_eq!(
        parse_iso_utc_ns("2024-02-29T00:00:00Z"),
        Some(1_709_164_800_000_000_000)
    );
    for bad in [
        "",
        "2026-13-01T00:00:00Z",
        "2026-10-01",
        "yesterday",
        "2026-10-01T25:00:00Z",
    ] {
        assert_eq!(parse_iso_utc_ns(bad), None, "{bad}");
    }
}

#[test]
fn linux_pinned_folders_are_read_from_file_manager_bookmarks() {
    let text = "file:///home/ada/Projects Projects\nfile:///home/ada/My%20Stuff\nsftp://server/share Remote\nsmb:///srv/share Network\n\n";
    assert_eq!(
        bookmarks_from_gtk(text),
        [
            PathBuf::from("/home/ada/Projects"),
            PathBuf::from("/home/ada/My Stuff")
        ]
    );
}

#[test]
fn mac_recent_files_are_read_from_spotlight_answers() {
    let out = "/Users/ada/Documents/cv.pages\n\n/Users/ada/Desktop/todo.txt\nnot a path\n";
    assert_eq!(
        recent_from_mdfind(out),
        [
            PathBuf::from("/Users/ada/Documents/cv.pages"),
            PathBuf::from("/Users/ada/Desktop/todo.txt")
        ]
    );
}

fn item(role: FolderRole, path: &[&str]) -> Item {
    let laptop = LaptopId::from_hex("00112233445566778899aabbccddeeff").unwrap();
    let owner = Owner::Person {
        account_id: "1000".into(),
    };
    let place = Place {
        role,
        storage: Storage::SystemDrive,
    };
    let path = ItemPath::new(path.iter().map(|p| ItemName::from_text(p)).collect()).unwrap();
    Item {
        id: ItemId::derive(&laptop, &owner, &place, &path),
        kind: ItemKind::File,
        owner,
        place,
        path,
        size_bytes: 1,
        portability: Portability::Portable,
        managed_by: ManagedBy::Personal,
        download_bytes: None,
        inclusion: Inclusion::Included,
    }
}

#[test]
fn recent_and_pinned_things_are_matched_to_the_scanned_items() {
    let home = if cfg!(windows) {
        Path::new(r"C:\Users\Ada")
    } else {
        Path::new("/home/ada")
    };
    let docs_root = home.join("Documents");
    let pics_root = home.join("Pictures");
    let items = vec![
        item(FolderRole::Documents, &["Taxes", "2026.pdf"]),
        item(FolderRole::Documents, &["Projects", "plan.docx"]),
        item(FolderRole::Documents, &["Projects", "Notes", "a.txt"]),
        item(FolderRole::Documents, &["old.txt"]),
        item(FolderRole::Pictures, &["beach.jpg"]),
    ];
    let mut roots = BTreeMap::new();
    roots.insert(FolderRole::Documents, docs_root.clone());
    roots.insert(FolderRole::Pictures, pics_root.clone());
    let usage = Usage {
        recent: vec![
            (docs_root.join("Taxes").join("2026.pdf"), Some(500)),
            (pics_root.join("beach.jpg"), Some(900)),
            // Opened recently but not part of this move: ignored.
            (home.join("Downloads").join("setup.exe"), Some(950)),
        ],
        pinned: vec![docs_root.join("Projects")],
    };
    let mine = personal_essentials(&items, &roots, &usage);
    let ids: Vec<ItemId> = items.iter().map(|i| i.id).collect();
    assert_eq!(mine.get(&ids[0]), Some(&500));
    assert_eq!(mine.get(&ids[4]), Some(&900));
    // Everything inside a pinned folder counts too.
    assert!(mine.contains_key(&ids[1]) && mine.contains_key(&ids[2]));
    assert!(!mine.contains_key(&ids[3]));
    assert_eq!(mine.len(), 4);
}

#[test]
fn this_laptops_usage_records_are_read_without_failing() {
    let home = std::env::var_os(if cfg!(windows) { "USERPROFILE" } else { "HOME" })
        .map(PathBuf::from)
        .unwrap();
    let usage = read_usage(&home);
    // Only counts are shown: the paths themselves are personal.
    eprintln!(
        "recent files: {}, pinned folders: {}",
        usage.recent.len(),
        usage.pinned.len()
    );
    assert!(usage.recent.len() <= 5000);
}

// ---------- pinned folders: Windows Quick Access and macOS Finder favourites ----------

/// One DestList entry (format version 3 or 4): `pin` is -1 for a recent item, or its place among
/// the pinned ones.
fn dest_entry(path: &str, pin: i32, version: u32) -> Vec<u8> {
    let mut e = vec![0u8; 108];
    e.extend_from_slice(&pin.to_le_bytes());
    if version >= 2 {
        e.extend_from_slice(&[0u8; 16]);
    }
    let wide: Vec<u16> = path.encode_utf16().collect();
    e.extend_from_slice(&u16::try_from(wide.len()).unwrap().to_le_bytes());
    for c in wide {
        e.extend_from_slice(&c.to_le_bytes());
    }
    if version >= 2 {
        e.extend_from_slice(&[0u8; 4]);
    }
    e
}

/// A Quick Access jump list: a compound file whose DestList stream lists `entries`.
fn quick_access(entries: &[(&str, i32)], version: u32) -> Vec<u8> {
    let mut dest = version.to_le_bytes().to_vec();
    dest.extend_from_slice(&u32::try_from(entries.len()).unwrap().to_le_bytes());
    dest.extend_from_slice(&[0u8; 24]);
    for (path, pin) in entries {
        dest.extend_from_slice(&dest_entry(path, *pin, version));
    }
    let mut file = cfb::CompoundFile::create(std::io::Cursor::new(Vec::new())).unwrap();
    std::io::Write::write_all(&mut file.create_stream("DestList").unwrap(), &dest).unwrap();
    file.flush().unwrap();
    file.into_inner().into_inner()
}

#[test]
fn windows_quick_access_pins_are_read_in_their_order() {
    for version in [1, 3, 4] {
        let bytes = quick_access(
            &[
                (r"C:\Users\Ada\Documents\notes.txt", -1),
                (r"D:\Photos", 1),
                (r"C:\Users\Ada\Projects", 0),
            ],
            version,
        );
        assert_eq!(
            pinned_from_quick_access(&bytes),
            [
                PathBuf::from(r"C:\Users\Ada\Projects"),
                PathBuf::from(r"D:\Photos")
            ],
            "version {version}"
        );
    }
}

#[test]
fn a_damaged_quick_access_file_gives_nothing() {
    assert!(pinned_from_quick_access(b"not a compound file").is_empty());
    let mut bytes = quick_access(&[(r"C:\Users\Ada\Projects", 0)], 4);
    let n = bytes.len();
    bytes.truncate(n / 2);
    assert!(pinned_from_quick_access(&bytes).is_empty());
}

/// A bookmark record (mac_alias's documented layout) for `components`.
fn bookmark(components: &[&str]) -> Vec<u8> {
    bookmark_with(components, 0xffff_fffe, 0x0601)
}

/// A bookmark with a chosen table-of-contents marker and kind for the path record.
fn bookmark_with(components: &[&str], toc_magic: u32, path_kind: u32) -> Vec<u8> {
    let mut body = Vec::new();
    let record = |kind: u32, data: &[u8], body: &mut Vec<u8>| -> u32 {
        let at = u32::try_from(body.len()).unwrap();
        body.extend_from_slice(&u32::try_from(data.len()).unwrap().to_le_bytes());
        body.extend_from_slice(&kind.to_le_bytes());
        body.extend_from_slice(data);
        while !body.len().is_multiple_of(4) {
            body.push(0);
        }
        at
    };
    // Offsets are from the end of the 48-byte header; the first 4 bytes there point at the TOC.
    body.extend_from_slice(&[0u8; 4]);
    let strings: Vec<u32> = components
        .iter()
        .map(|c| record(0x0101, c.as_bytes(), &mut body))
        .collect();
    let array: Vec<u8> = strings.iter().flat_map(|o| o.to_le_bytes()).collect();
    let path = record(path_kind, &array, &mut body);
    let toc = u32::try_from(body.len()).unwrap();
    body[..4].copy_from_slice(&toc.to_le_bytes());
    // TOC: size (less 8), magic, identifier, next TOC, entry count; then one entry.
    for v in [24u32, toc_magic, 1, 0, 1, 0x1004, path, 0] {
        body.extend_from_slice(&v.to_le_bytes());
    }
    let mut out = b"book".to_vec();
    out.extend_from_slice(&u32::try_from(48 + body.len()).unwrap().to_le_bytes());
    out.extend_from_slice(&0x1004_0000u32.to_le_bytes());
    out.extend_from_slice(&48u32.to_le_bytes());
    out.extend_from_slice(&[0u8; 32]);
    out.extend_from_slice(&body);
    out
}

/// A Finder favourites list (a keyed archive) holding these bookmarks.
fn favourites(bookmarks: Vec<Vec<u8>>) -> Vec<u8> {
    let mut objects = vec![plist::Value::String("$null".into())];
    for b in bookmarks {
        let mut item = plist::Dictionary::new();
        item.insert("Bookmark".into(), plist::Value::Data(b));
        objects.push(plist::Value::Dictionary(item));
    }
    let mut root = plist::Dictionary::new();
    root.insert("$archiver".into(), "NSKeyedArchiver".into());
    root.insert("$objects".into(), plist::Value::Array(objects));
    let mut out = Vec::new();
    plist::Value::Dictionary(root)
        .to_writer_binary(&mut out)
        .unwrap();
    out
}

#[test]
fn macos_finder_favourites_are_read_from_their_bookmarks() {
    let bytes = favourites(vec![
        bookmark(&["Users", "ada", "Projects"]),
        bookmark(&["Users", "ada", "Pictures", "Holiday 2026"]),
    ]);
    assert_eq!(
        pinned_from_finder_favourites(&bytes),
        [
            PathBuf::from("/Users/ada/Projects"),
            PathBuf::from("/Users/ada/Pictures/Holiday 2026")
        ]
    );
}

#[test]
fn damaged_finder_favourites_give_nothing_and_never_crash() {
    assert!(pinned_from_finder_favourites(b"not a property list").is_empty());
    let mut b = bookmark(&["Users", "ada", "Projects"]);
    // A table of contents pointing far past the end, and a path array pointing nowhere.
    let toc_at = 48;
    b[toc_at..toc_at + 4].copy_from_slice(&u32::MAX.to_le_bytes());
    assert!(pinned_from_finder_favourites(&favourites(vec![b])).is_empty());
    // A table of contents without its marker, and a path that is not a list.
    assert!(
        pinned_from_finder_favourites(&favourites(vec![bookmark_with(
            &["Users", "ada"],
            0x1234,
            0x0601
        )]))
        .is_empty()
    );
    assert!(
        pinned_from_finder_favourites(&favourites(vec![bookmark_with(
            &["Users", "ada"],
            0xffff_fffe,
            0x0101
        )]))
        .is_empty()
    );
    // A path that would climb out, or a component holding a separator, is not trusted.
    for bad in [
        &["Users", "..", "etc"][..],
        &["Users", "ada/../../etc"],
        &["Users", ""],
    ] {
        assert!(
            pinned_from_finder_favourites(&favourites(vec![bookmark(bad)])).is_empty(),
            "{bad:?}"
        );
    }
    let mut c = bookmark(&["Users"]);
    let n = c.len();
    c.truncate(n - 10);
    assert!(pinned_from_finder_favourites(&favourites(vec![c])).is_empty());
}

proptest::proptest! {
    #[test]
    fn any_bytes_never_crash_either_reader(bytes in proptest::collection::vec(proptest::prelude::any::<u8>(), 0..600)) {
        let _ = pinned_from_quick_access(&bytes);
        let _ = pinned_from_finder_favourites(&favourites(vec![{
            let mut b = b"book".to_vec();
            b.extend_from_slice(&bytes);
            b
        }]));
    }
}
