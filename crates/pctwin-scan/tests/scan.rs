//! Scanning a folder into the shared move record (Task List 1.4): every file and folder becomes an
//! item with a permanent ID, its owner and where it really lives; scanning again gives the same IDs;
//! macOS packages are one item; what can't be read is recorded with the reason; photos, videos,
//! music and documents are counted for the after-move check.

use std::path::Path;

use pctwin_record::{
    FolderRole, Inclusion, Item, ItemKind, LaptopId, Owner, Place, Record, Storage,
};
use pctwin_scan::{Counts, scan_folder};

fn laptop() -> LaptopId {
    LaptopId::from_hex("00112233445566778899aabbccddeeff").unwrap()
}

fn ada() -> Owner {
    Owner::Person {
        account_id: "S-1-5-21-1".into(),
    }
}

fn docs() -> Place {
    Place {
        role: FolderRole::Documents,
        storage: Storage::SystemDrive,
    }
}

fn names(item: &Item) -> Vec<&str> {
    item.path.parts().iter().map(|n| n.display()).collect()
}

fn find<'a>(items: &'a [Item], path: &[&str]) -> &'a Item {
    items
        .iter()
        .find(|i| names(i) == path)
        .unwrap_or_else(|| panic!("{path:?} not found"))
}

fn write(root: &Path, rel: &str, bytes: usize) {
    let p = root.join(rel);
    std::fs::create_dir_all(p.parent().unwrap()).unwrap();
    std::fs::write(p, vec![b'x'; bytes]).unwrap();
}

#[test]
fn every_file_and_folder_becomes_an_item_with_its_owner_and_place() {
    let root = tempfile::tempdir().unwrap();
    write(root.path(), "Taxes/2026/return.pdf", 300);
    write(root.path(), "notes.txt", 5);
    std::fs::create_dir(root.path().join("Empty")).unwrap();
    let scan = scan_folder(&laptop(), &ada(), &docs(), root.path());

    let mut paths: Vec<Vec<&str>> = scan.items.iter().map(names).collect();
    paths.sort();
    assert_eq!(
        paths,
        [
            vec!["Empty"],
            vec!["Taxes"],
            vec!["Taxes", "2026"],
            vec!["Taxes", "2026", "return.pdf"],
            vec!["notes.txt"],
        ]
    );
    let ret = find(&scan.items, &["Taxes", "2026", "return.pdf"]);
    assert_eq!(ret.kind, ItemKind::File);
    assert_eq!(ret.size_bytes, 300);
    assert_eq!(ret.owner, ada());
    assert_eq!(ret.place, docs());
    assert_eq!(ret.inclusion, Inclusion::Included);
    assert_eq!(find(&scan.items, &["Empty"]).kind, ItemKind::Folder);
    assert!(scan.unreadable.is_empty());
}

#[test]
fn scanning_again_gives_the_same_ids() {
    let root = tempfile::tempdir().unwrap();
    write(root.path(), "a/b.txt", 1);
    write(root.path(), "c.jpg", 2);
    let ids = |scan: &pctwin_scan::Scan| {
        let mut v: Vec<_> = scan
            .items
            .iter()
            .map(|i| (names(i).join("/"), i.id))
            .collect();
        v.sort();
        v
    };
    let first = scan_folder(&laptop(), &ada(), &docs(), root.path());
    let second = scan_folder(&laptop(), &ada(), &docs(), root.path());
    assert_eq!(ids(&first), ids(&second));
}

#[test]
fn the_items_make_a_valid_record() {
    let root = tempfile::tempdir().unwrap();
    write(root.path(), "a/b.txt", 1);
    write(root.path(), "a/c.txt", 1);
    let scan = scan_folder(&laptop(), &ada(), &docs(), root.path());
    let mut record = Record::new(laptop());
    record.items = scan.items;
    record.validate().unwrap();
}

#[test]
fn photos_videos_music_and_documents_are_counted() {
    let root = tempfile::tempdir().unwrap();
    for f in [
        "IMG_0001.JPG",
        "IMG_0002.heic",
        "raw/DSC001.CR3",
        "raw/DSC002.nef",
        "raw/DSC003.ARW",
        "raw/DSC004.dng",
        "web.webp",
        "clip.MOV",
        "film.mp4",
        "song.mp3",
        "song.FLAC",
        "cv.docx",
        "sheet.xlsx",
        "scan.pdf",
        "setup.exe",
        "data.bin",
    ] {
        write(root.path(), f, 1);
    }
    let scan = scan_folder(&laptop(), &ada(), &docs(), root.path());
    assert_eq!(
        scan.counts,
        Counts {
            photos: 7,
            videos: 2,
            music: 2,
            documents: 3,
            other: 2,
        }
    );
}

#[test]
fn a_mac_package_is_one_item_and_is_not_split() {
    let root = tempfile::tempdir().unwrap();
    write(root.path(), "Report.pages/Index.zip", 100);
    write(root.path(), "Report.pages/Data/image.png", 50);
    write(root.path(), "Tool.app/Contents/MacOS/Tool", 400);
    write(root.path(), "Tool.app/Contents/Info.plist", 20);
    let scan = scan_folder(&laptop(), &ada(), &docs(), root.path());
    assert_eq!(
        scan.items.len(),
        2,
        "{:?}",
        scan.items.iter().map(names).collect::<Vec<_>>()
    );
    let pages = find(&scan.items, &["Report.pages"]);
    assert_eq!(pages.kind, ItemKind::Folder);
    assert_eq!(pages.size_bytes, 150);
    let app = find(&scan.items, &["Tool.app"]);
    assert_eq!(app.kind, ItemKind::App);
    assert_eq!(app.size_bytes, 420);
    // The picture inside a document is not a photo of yours.
    assert_eq!(scan.counts.photos, 0);
    assert_eq!(scan.counts.documents, 1);
}

#[test]
fn photos_in_a_photos_library_are_counted_from_its_originals_only() {
    let root = tempfile::tempdir().unwrap();
    let lib = "Photos Library.photoslibrary";
    write(root.path(), &format!("{lib}/originals/0/0A1B.heic"), 10);
    write(root.path(), &format!("{lib}/originals/F/F00D.jpeg"), 10);
    write(root.path(), &format!("{lib}/originals/F/F00E.mov"), 10);
    write(
        root.path(),
        &format!("{lib}/resources/derivatives/0/0A1B_thumb.jpeg"),
        1,
    );
    write(root.path(), &format!("{lib}/database/Photos.sqlite"), 1);
    let scan = scan_folder(&laptop(), &ada(), &docs(), root.path());
    assert_eq!(scan.items.len(), 1);
    assert_eq!(scan.counts.photos, 2);
    assert_eq!(scan.counts.videos, 1);
    assert_eq!(find(&scan.items, &[lib]).size_bytes, 32);
}

#[test]
fn an_older_photos_library_counts_its_masters() {
    let root = tempfile::tempdir().unwrap();
    let lib = "Photos Library.photoslibrary";
    write(
        root.path(),
        &format!("{lib}/Masters/2019/01/02/IMG_1.JPG"),
        1,
    );
    write(root.path(), &format!("{lib}/Thumbnails/IMG_1.jpg"), 1);
    let scan = scan_folder(&laptop(), &ada(), &docs(), root.path());
    assert_eq!(scan.counts.photos, 1);
}

#[cfg(unix)]
#[test]
fn links_are_not_items_and_are_counted() {
    let root = tempfile::tempdir().unwrap();
    write(root.path(), "real.txt", 1);
    std::os::unix::fs::symlink(root.path().join("real.txt"), root.path().join("alias.txt"))
        .unwrap();
    let scan = scan_folder(&laptop(), &ada(), &docs(), root.path());
    assert_eq!(scan.items.len(), 1);
    assert_eq!(scan.links, 1);
}

#[cfg(unix)]
#[test]
fn a_folder_that_cannot_be_read_is_recorded_with_the_reason() {
    use std::os::unix::fs::PermissionsExt;
    let root = tempfile::tempdir().unwrap();
    write(root.path(), "locked/secret.txt", 1);
    write(root.path(), "open.txt", 1);
    let locked = root.path().join("locked");
    std::fs::set_permissions(&locked, std::fs::Permissions::from_mode(0o000)).unwrap();
    let scan = scan_folder(&laptop(), &ada(), &docs(), root.path());
    std::fs::set_permissions(&locked, std::fs::Permissions::from_mode(0o755)).unwrap();
    if scan.unreadable.is_empty() {
        return; // running as root: everything is readable
    }
    let item = find(&scan.items, &["locked"]);
    assert_eq!(
        item.inclusion,
        Inclusion::LeftOut {
            reason: pctwin_record::LeftOutReason::Unreadable
        }
    );
    assert_eq!(scan.unreadable, [locked]);
    find(&scan.items, &["open.txt"]);
}

#[cfg(all(unix, not(target_os = "macos")))]
#[test]
fn a_name_that_is_not_valid_text_keeps_its_exact_bytes() {
    use std::os::unix::ffi::OsStrExt;
    let root = tempfile::tempdir().unwrap();
    let name = std::ffi::OsStr::from_bytes(b"caf\xe9.txt");
    std::fs::write(root.path().join(name), b"x").unwrap();
    let scan = scan_folder(&laptop(), &ada(), &docs(), root.path());
    let item = &scan.items[0];
    assert_eq!(
        item.path.parts()[0].exact(),
        Some(&pctwin_record::RawName::Unix(b"caf\xe9.txt".to_vec()))
    );
}

#[test]
fn a_missing_folder_gives_no_items_and_is_listed() {
    let root = tempfile::tempdir().unwrap();
    let gone = root.path().join("gone");
    let scan = scan_folder(&laptop(), &ada(), &docs(), &gone);
    assert!(scan.items.is_empty());
    assert_eq!(scan.unreadable, [gone]);
}
