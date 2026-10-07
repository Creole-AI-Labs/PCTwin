//! The whole scan of the old laptop (Task List 1.4): the people, the signed-in person's special
//! folders and the shared ones, the drives, all in one move record, with honest sizes for
//! everyone and the system's own clutter recorded as left out.

use std::path::{Path, PathBuf};

use pctwin_record::{
    CloudProvider, FolderRole, Inclusion, Item, LaptopId, LeftOutReason, Owner, Storage,
};
use pctwin_scan::{
    FolderLookup, FoundFolder, Person, ScanError, Size, build_scan, scan_this_laptop,
};

fn laptop() -> LaptopId {
    LaptopId::from_hex("00112233445566778899aabbccddeeff").unwrap()
}

fn person(id: &str, home: &Path, is_me: bool) -> Person {
    Person {
        account_id: id.into(),
        suggested_name: id.into(),
        home: home.to_path_buf(),
        is_me,
    }
}

fn found(role: FolderRole, path: &Path, storage: Storage) -> FolderLookup {
    FolderLookup::Found(FoundFolder {
        role,
        path: path.to_path_buf(),
        storage,
        moved: false,
    })
}

fn write(path: PathBuf, bytes: usize) {
    std::fs::create_dir_all(path.parent().unwrap()).unwrap();
    std::fs::write(path, vec![b'x'; bytes]).unwrap();
}

fn names(item: &Item) -> String {
    item.path
        .parts()
        .iter()
        .map(|n| n.display())
        .collect::<Vec<_>>()
        .join("/")
}

#[test]
fn the_signed_in_persons_folders_and_the_shared_ones_make_one_record() {
    let base = tempfile::tempdir().unwrap();
    let home = base.path().join("ada");
    let docs = home.join("Documents");
    let pics = base.path().join("OneDrive/Pictures");
    let public = base.path().join("Public");
    write(docs.join("cv.docx"), 10);
    write(docs.join("tax/return.pdf"), 20);
    write(pics.join("beach.jpg"), 30);
    write(public.join("family.jpg"), 40);
    let tunde_home = base.path().join("tunde");
    write(tunde_home.join("notes.txt"), 7);

    let people = vec![
        person("S-1-5-21-1001", &home, true),
        person("S-1-5-21-1002", &tunde_home, false),
    ];
    let folders = vec![
        found(FolderRole::Home, &home, Storage::SystemDrive),
        found(FolderRole::Documents, &docs, Storage::SystemDrive),
        found(
            FolderRole::Pictures,
            &pics,
            Storage::Cloud {
                provider: CloudProvider::OneDrive,
            },
        ),
        found(FolderRole::Public, &public, Storage::SystemDrive),
        FolderLookup::Missing {
            role: FolderRole::Desktop,
        },
    ];
    let scan = build_scan(laptop(), people, folders, Vec::new()).unwrap();
    scan.record.validate().unwrap();

    let ada = Owner::Person {
        account_id: "S-1-5-21-1001".into(),
    };
    let by_name = |n: &str| scan.record.items.iter().find(|i| names(i) == n).unwrap();
    assert_eq!(by_name("cv.docx").owner, ada);
    assert_eq!(by_name("cv.docx").place.role, FolderRole::Documents);
    assert_eq!(by_name("beach.jpg").place.role, FolderRole::Pictures);
    assert_eq!(
        by_name("beach.jpg").place.storage,
        Storage::Cloud {
            provider: CloudProvider::OneDrive
        }
    );
    assert_eq!(by_name("family.jpg").owner, Owner::Shared);
    // The home folder is the container of the others, not scanned on its own.
    assert!(scan.record.items.iter().all(|i| names(i) != "Documents"));
    assert_eq!(scan.counts.photos, 2);
    assert_eq!(scan.counts.documents, 2);

    // Everyone gets an honest size: Ada's is what of hers was scanned (not the shared folder),
    // Tunde's what can be read.
    assert_eq!(scan.people.len(), 2);
    assert_eq!(
        scan.people[0].size,
        Size::Known {
            bytes: 60,
            files: 3
        }
    );
    assert_eq!(scan.people[1].size, Size::Known { bytes: 7, files: 1 });
}

#[test]
fn a_folder_inside_another_is_scanned_once() {
    let base = tempfile::tempdir().unwrap();
    let docs = base.path().join("Documents");
    let pics = docs.join("My Pictures");
    write(pics.join("a.jpg"), 1);
    let people = vec![person("1000", base.path(), true)];
    let folders = vec![
        found(FolderRole::Pictures, &pics, Storage::SystemDrive),
        found(FolderRole::Documents, &docs, Storage::SystemDrive),
    ];
    let scan = build_scan(laptop(), people, folders, Vec::new()).unwrap();
    scan.record.validate().unwrap();
    let photos: Vec<_> = scan
        .record
        .items
        .iter()
        .filter(|i| names(i).ends_with("a.jpg"))
        .collect();
    assert_eq!(photos.len(), 1);
    assert_eq!(photos[0].place.role, FolderRole::Documents);
    assert_eq!(scan.counts.photos, 1);
}

#[test]
fn without_a_signed_in_person_there_is_no_scan() {
    let base = tempfile::tempdir().unwrap();
    let people = vec![person("1000", base.path(), false)];
    assert!(matches!(
        build_scan(laptop(), people, Vec::new(), Vec::new()),
        Err(ScanError::NoSignedInPerson)
    ));
}

#[test]
fn another_persons_private_folder_is_never_shown_as_empty() {
    let base = tempfile::tempdir().unwrap();
    let people = vec![
        person("1000", base.path(), true),
        person("1001", &base.path().join("cannot-see"), false),
    ];
    let scan = build_scan(laptop(), people, Vec::new(), Vec::new()).unwrap();
    assert_eq!(scan.people[1].size, Size::Unreadable);
}

#[test]
fn the_systems_own_clutter_is_recorded_as_left_out() {
    let base = tempfile::tempdir().unwrap();
    let docs = base.path().join("Documents");
    for f in [
        "Thumbs.db",
        "desktop.ini",
        ".DS_Store",
        "~$report.docx",
        "$RECYCLE.BIN/S-1-5-21/old.txt",
        ".Trash/old.txt",
        ".Spotlight-V100/index",
        "report.docx",
    ] {
        write(docs.join(f), 1);
    }
    let people = vec![person("1000", base.path(), true)];
    let folders = vec![found(FolderRole::Documents, &docs, Storage::SystemDrive)];
    let scan = build_scan(laptop(), people, folders, Vec::new()).unwrap();
    let clutter = Inclusion::LeftOut {
        reason: LeftOutReason::System,
    };
    for name in [
        "Thumbs.db",
        "desktop.ini",
        ".DS_Store",
        "~$report.docx",
        "$RECYCLE.BIN",
        ".Trash",
        ".Spotlight-V100",
    ] {
        let item = scan
            .record
            .items
            .iter()
            .find(|i| names(i) == name)
            .unwrap_or_else(|| panic!("{name} not recorded"));
        assert_eq!(item.inclusion, clutter, "{name}");
    }
    // Inside clutter folders nothing is scanned.
    assert!(scan.record.items.iter().all(|i| !names(i).contains('/')));
    let report = scan
        .record
        .items
        .iter()
        .find(|i| names(i) == "report.docx")
        .unwrap();
    assert_eq!(report.inclusion, Inclusion::Included);
    // Clutter is not counted as your documents.
    assert_eq!(scan.counts.documents, 1);
}

#[test]
fn this_laptop_scans_into_a_valid_record() {
    // This reads the details (never the contents) of every file in the signed-in person's folders,
    // so it runs only on GitHub's throwaway test machines, not on a person's laptop.
    if std::env::var("GITHUB_ACTIONS").as_deref() != Ok("true") {
        eprintln!("skipped: scans the whole signed-in profile; runs only on CI");
        return;
    }
    let scan = scan_this_laptop(laptop()).unwrap();
    scan.record.validate().unwrap();
    assert_eq!(scan.people.iter().filter(|p| p.person.is_me).count(), 1);
    assert!(scan.drives.iter().any(|d| d.is_system));
}
