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
    Usage, bookmarks_from_gtk, parse_iso_utc_ns, personal_essentials, read_usage,
    recent_from_mdfind, recent_from_xbel,
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
