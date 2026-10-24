//! What the old laptop sends: one list of formats not worth compressing, and one way to turn a
//! record into send jobs, in the order the app moves things, with every file not sent and why.

use std::collections::BTreeMap;
use std::path::PathBuf;

use pctwin_move::{NotSent, NotSentWhy, send_plan, worth_compressing};
use pctwin_record::{
    FolderRole, Inclusion, Item, ItemId, ItemKind, ItemName, ItemPath, LaptopId, LeftOutReason,
    ManagedBy, Owner, Place, Portability, Record, Storage,
};
use pctwin_transfer::{LandingPlace, Tier, role_label, shown_path};

/// Formats that are already compressed are never tried, whatever case their name is in.
#[test]
fn compressed_formats_are_not_worth_compressing_in_any_case() {
    for ext in [
        "jpg", "jpeg", "png", "heic", "mp3", "mp4", "mov", "zip", "docx", "xlsx", "gz", "7z",
    ] {
        let upper = ext.to_ascii_uppercase();
        let mixed: String = ext
            .chars()
            .enumerate()
            .map(|(i, c)| {
                if i % 2 == 0 {
                    c.to_ascii_uppercase()
                } else {
                    c
                }
            })
            .collect();
        for name in [
            format!("a.{ext}"),
            format!("Holiday/day 2/boat.{upper}"),
            format!("old.notes.{mixed}"),
        ] {
            assert!(!worth_compressing(&name), "{name}");
        }
    }
}

/// Anything else is tried: a text file, a name with no dot (even one spelt like a format), and a
/// file inside a folder whose own name has a dot.
#[test]
fn everything_else_is_worth_compressing() {
    for name in [
        "notes.txt",
        "Letters/notes.TXT",
        "README",
        "zip",
        "Photos.zip/readme",
        "Holiday.jpg/list",
        "archive.tar",
    ] {
        assert!(worth_compressing(name), "{name}");
    }
}

fn laptop() -> LaptopId {
    LaptopId::from_hex("00112233445566778899aabbccddeeff").unwrap()
}

fn item(role: FolderRole, path: &[&str], kind: ItemKind, size: u64, inclusion: Inclusion) -> Item {
    let owner = Owner::Person {
        account_id: "1000".into(),
    };
    let place = Place {
        role,
        storage: Storage::SystemDrive,
    };
    let path = ItemPath::new(path.iter().map(|p| ItemName::from_text(p)).collect()).unwrap();
    Item {
        id: ItemId::derive(&laptop(), &owner, &place, &path),
        kind,
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

fn file(role: FolderRole, path: &[&str], size: u64) -> Item {
    item(role, path, ItemKind::File, size, Inclusion::Included)
}

fn record(items: Vec<Item>) -> Record {
    let mut record = Record::new(laptop());
    record.items = items;
    record
}

fn landing(item: &Item) -> LandingPlace {
    LandingPlace::new(role_label(item.place.role), shown_path(&item.path))
}

/// The old laptop's folders: Documents and Pictures only.
fn source(item: &Item) -> Option<PathBuf> {
    let root = match item.place.role {
        FolderRole::Documents => PathBuf::from("old/Documents"),
        FolderRole::Pictures => PathBuf::from("old/Pictures"),
        _ => return None,
    };
    let mut source = root;
    for part in item.path.parts() {
        source.push(part.on_disk());
    }
    Some(source)
}

/// Jobs come in the order the app moves things (essentials first, then the rest; smaller first
/// within a tier), each with its tier, its source, where it lands, and whether to compress it.
#[test]
fn jobs_are_in_plan_order_with_their_tiers() {
    let photo = file(FolderRole::Pictures, &["beach.jpg"], 10);
    let big = file(FolderRole::Documents, &["Reports", "year.txt"], 500);
    let small = file(FolderRole::Documents, &["letter.txt"], 20);
    let record = record(vec![photo.clone(), big.clone(), small.clone()]);
    let plan = send_plan(&record, source, landing);
    let got: Vec<(ItemId, Tier)> = plan.jobs.iter().map(|j| (j.item, j.tier)).collect();
    assert_eq!(
        got,
        [
            (small.id, Tier::Essential),
            (big.id, Tier::Essential),
            (photo.id, Tier::Rest),
        ]
    );
    // Exactly the order and tiers the transfer crate plans.
    let planned = pctwin_transfer::plan_order(&record.items, &[], &BTreeMap::new(), 0);
    assert_eq!(got, planned);
    let year = &plan.jobs[1];
    assert_eq!(year.source, PathBuf::from("old/Documents/Reports/year.txt"));
    assert_eq!(year.destination, role_label(FolderRole::Documents));
    assert_eq!(year.path, "Reports/year.txt");
    assert!(year.compressible);
    assert!(!plan.jobs[2].compressible, "a photo is already compressed");
    assert!(plan.not_sent.is_empty());
}

/// A file the record leaves out is not sent, with the record's own reason; a file whose role has
/// no folder on the old laptop is not sent either, and says so.
#[test]
fn files_not_sent_say_why() {
    let kept = file(FolderRole::Documents, &["kept.txt"], 5);
    let locked = item(
        FolderRole::Documents,
        &["locked.txt"],
        ItemKind::File,
        5,
        Inclusion::LeftOut {
            reason: LeftOutReason::Unreadable,
        },
    );
    let song = file(FolderRole::Music, &["song.mp3"], 5);
    let record = record(vec![kept.clone(), locked.clone(), song.clone()]);
    let plan = send_plan(&record, source, landing);
    let sent: Vec<ItemId> = plan.jobs.iter().map(|j| j.item).collect();
    assert_eq!(sent, [kept.id]);
    assert_eq!(
        plan.not_sent,
        [
            NotSent {
                item: locked.id,
                why: NotSentWhy::LeftOut {
                    reason: LeftOutReason::Unreadable
                },
            },
            NotSent {
                item: song.id,
                why: NotSentWhy::NoFolderForRole,
            },
        ]
    );
}

/// Only files are sent: folders, apps and settings are neither sent nor listed as not sent, even
/// when the record leaves them out.
#[test]
fn only_files_are_sent() {
    let letter = file(FolderRole::Documents, &["letter.txt"], 5);
    let record = record(vec![
        item(
            FolderRole::Documents,
            &["Reports"],
            ItemKind::Folder,
            0,
            Inclusion::Included,
        ),
        item(
            FolderRole::Documents,
            &["Old"],
            ItemKind::Folder,
            40,
            Inclusion::LeftOut {
                reason: LeftOutReason::ChosenByUser,
            },
        ),
        item(
            FolderRole::Other,
            &["Editor"],
            ItemKind::App,
            9,
            Inclusion::Included,
        ),
        item(
            FolderRole::Other,
            &["Wallpaper"],
            ItemKind::Setting,
            1,
            Inclusion::Included,
        ),
        letter.clone(),
    ]);
    let plan = send_plan(&record, source, landing);
    let sent: Vec<ItemId> = plan.jobs.iter().map(|j| j.item).collect();
    assert_eq!(sent, [letter.id]);
    assert!(plan.not_sent.is_empty(), "{:?}", plan.not_sent);
}

/// The person's own essentials (the record's personal order) move first, in that order, after
/// only what they asked for and before the general essentials; the send plan is in the same
/// order, so what the plan shows first is what goes first.
#[test]
fn the_personal_order_moves_first_in_its_own_order() {
    let inc = Inclusion::Included;
    let mut record = Record::new(laptop());
    record.items = vec![
        item(FolderRole::Documents, &["cv.docx"], ItemKind::File, 5, inc),
        item(FolderRole::Music, &["song.mp3"], ItemKind::File, 7, inc),
        item(FolderRole::Pictures, &["beach.jpg"], ItemKind::File, 9, inc),
        item(FolderRole::Videos, &["clip.mp4"], ItemKind::File, 3, inc),
    ];
    let (song, beach) = (record.items[1].id, record.items[2].id);
    record.personal = vec![beach, song];
    let order = pctwin_move::move_order(&record);
    assert_eq!(order[0], (beach, Tier::Personal));
    assert_eq!(order[1], (song, Tier::Personal));
    // The general essentials (Documents) come next.
    assert_eq!(order[2].0, record.items[0].id);
    assert!(order[2..].iter().all(|(_, t)| *t != Tier::Personal));
    let plan = send_plan(&record, |_| Some(PathBuf::from("x")), landing);
    let jobs: Vec<ItemId> = plan.jobs.iter().map(|j| j.item).collect();
    assert_eq!(&jobs[..2], &[beach, song]);
}
