//! Where each item lands on the new laptop (Task List 1.5): in the new laptop's own folder for
//! that role, wherever it is and whatever it is called; in a "from your old laptop" folder when the
//! new laptop has no such folder; left to the cloud service when both laptops keep that folder in
//! the same one; and never into another person's account without the administrator helper.

use std::collections::BTreeMap;
use std::path::Path;

use pctwin_gate::Destinations;
use pctwin_record::{
    CloudProvider, FolderRole, Inclusion, Item, ItemId, ItemKind, ItemName, ItemPath, LaptopId,
    ManagedBy, Owner, Place, Portability, Storage,
};
use pctwin_scan::{FolderLookup, FoundFolder};
use pctwin_transfer::{Landing, NewPlaces, approve_new_places, landing_for, role_label};

fn item(owner: Owner, role: FolderRole, storage: Storage, path: &[&str]) -> Item {
    let laptop = LaptopId::from_hex("00112233445566778899aabbccddeeff").unwrap();
    let place = Place { role, storage };
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

fn me() -> Owner {
    Owner::Person {
        account_id: "S-1-5-21-1001".into(),
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

fn onedrive() -> Storage {
    Storage::Cloud {
        provider: CloudProvider::OneDrive,
    }
}

fn places() -> NewPlaces {
    let mut roles = BTreeMap::new();
    roles.insert(FolderRole::Home, Storage::SystemDrive);
    roles.insert(FolderRole::Documents, Storage::SystemDrive);
    roles.insert(FolderRole::Pictures, onedrive());
    roles.insert(FolderRole::Public, Storage::SystemDrive);
    NewPlaces::new(roles)
}

fn fallback(role: FolderRole) -> String {
    format!("From your old laptop/{}", role_label(role))
}

#[test]
fn an_item_lands_in_the_new_laptops_own_folder_for_its_role() {
    let docs = item(
        me(),
        FolderRole::Documents,
        Storage::SystemDrive,
        &["Taxes", "2026.pdf"],
    );
    assert_eq!(
        landing_for(&docs, "S-1-5-21-1001", &places(), &fallback),
        Landing::Copy {
            destination: "documents".into(),
            path: "Taxes/2026.pdf".into()
        }
    );
    // Wherever the old laptop kept it: Documents on D: still lands in the new Documents.
    let on_d = item(
        me(),
        FolderRole::Documents,
        Storage::OtherDrive { drive: "D:".into() },
        &["cv.docx"],
    );
    assert_eq!(
        landing_for(&on_d, "S-1-5-21-1001", &places(), &fallback),
        Landing::Copy {
            destination: "documents".into(),
            path: "cv.docx".into()
        }
    );
}

#[test]
fn shared_items_land_in_the_shared_folder() {
    let shared = item(
        Owner::Shared,
        FolderRole::Public,
        Storage::SystemDrive,
        &["family.jpg"],
    );
    assert_eq!(
        landing_for(&shared, "S-1-5-21-1001", &places(), &fallback),
        Landing::Copy {
            destination: "public".into(),
            path: "family.jpg".into()
        }
    );
}

#[test]
fn a_shared_folder_of_any_kind_lands_in_the_shared_folder() {
    // A photo folder everyone on the old laptop shares still goes to the shared place.
    let shared_photo = item(
        Owner::Shared,
        FolderRole::Pictures,
        Storage::SystemDrive,
        &["trip.jpg"],
    );
    assert_eq!(
        landing_for(&shared_photo, "S-1-5-21-1001", &places(), &fallback),
        Landing::Copy {
            destination: "public".into(),
            path: "trip.jpg".into()
        }
    );
}
#[test]
fn a_role_the_new_laptop_lacks_lands_in_a_from_your_old_laptop_folder() {
    let song = item(
        me(),
        FolderRole::Music,
        Storage::SystemDrive,
        &["Album", "1.mp3"],
    );
    assert_eq!(
        landing_for(&song, "S-1-5-21-1001", &places(), &fallback),
        Landing::Copy {
            destination: "home".into(),
            path: "From your old laptop/music/Album/1.mp3".into()
        }
    );
    let nowhere = NewPlaces::new(BTreeMap::new());
    assert_eq!(
        landing_for(&song, "S-1-5-21-1001", &nowhere, &fallback),
        Landing::NoPlace
    );
}

#[test]
fn a_folder_both_laptops_keep_in_the_same_cloud_is_left_to_that_cloud() {
    let in_cloud = item(me(), FolderRole::Pictures, onedrive(), &["beach.jpg"]);
    assert_eq!(
        landing_for(&in_cloud, "S-1-5-21-1001", &places(), &fallback),
        Landing::LeaveToCloud {
            provider: CloudProvider::OneDrive
        }
    );
    // Only the same service: a Dropbox folder still copies into a OneDrive one.
    let dropbox = item(
        me(),
        FolderRole::Pictures,
        Storage::Cloud {
            provider: CloudProvider::Dropbox,
        },
        &["x.jpg"],
    );
    assert!(matches!(
        landing_for(&dropbox, "S-1-5-21-1001", &places(), &fallback),
        Landing::Copy { .. }
    ));
}

#[test]
fn another_persons_things_need_the_administrator_helper() {
    let theirs = item(
        Owner::Person {
            account_id: "S-1-5-21-1002".into(),
        },
        FolderRole::Documents,
        Storage::SystemDrive,
        &["a.txt"],
    );
    assert_eq!(
        landing_for(&theirs, "S-1-5-21-1001", &places(), &fallback),
        Landing::NeedsAdministrator
    );
    let system = item(
        Owner::System,
        FolderRole::Other,
        Storage::SystemDrive,
        &["x"],
    );
    assert_eq!(
        landing_for(&system, "S-1-5-21-1001", &places(), &fallback),
        Landing::NeedsAdministrator
    );
}

#[test]
fn the_new_laptops_folders_become_its_approved_destinations() {
    let base = tempfile::tempdir().unwrap();
    let docs = base.path().join("Documents");
    let public = base.path().join("Public");
    std::fs::create_dir_all(&docs).unwrap();
    std::fs::create_dir_all(&public).unwrap();
    let found_folders = vec![
        found(FolderRole::Home, base.path(), Storage::SystemDrive),
        found(FolderRole::Documents, &docs, Storage::SystemDrive),
        found(FolderRole::Public, &public, Storage::SystemDrive),
        FolderLookup::Missing {
            role: FolderRole::Music,
        },
    ];
    let mut table = Destinations::new();
    let places = approve_new_places(&mut table, &found_folders).unwrap();
    let mut ids: Vec<&str> = table.ids().collect();
    ids.sort_unstable();
    assert_eq!(ids, ["documents", "home", "public"]);
    assert_eq!(
        table.place("public").unwrap(),
        &pctwin_gate::Approved::SharedFolder
    );
    assert_eq!(
        table.place("documents").unwrap(),
        &pctwin_gate::Approved::MyFolders
    );
    assert!(places.has(FolderRole::Documents) && !places.has(FolderRole::Music));
}
