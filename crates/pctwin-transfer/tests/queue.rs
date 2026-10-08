//! The order things move in (Product Spec "Transfer, most important first"; Task List 1.5): what
//! the person asked for first (in Smart mode or by picking), then essentials (documents, the
//! desktop, recent files), then everything else, newest first; several files take turns so one
//! huge file never holds up the rest; a new "move this first" during the move jumps the queue.

use std::collections::BTreeMap;

use pctwin_record::{
    FolderRole, Inclusion, Item, ItemId, ItemKind, ItemName, ItemPath, LaptopId, LeftOutReason,
    ManagedBy, Owner, Place, Portability, Storage,
};
use pctwin_transfer::{Scheduler, Tier, plan_order};

const DAY_NS: i64 = 86_400_000_000_000;
const NOW: i64 = 1_800_000_000 * 1_000_000_000;

fn item(role: FolderRole, path: &[&str], kind: ItemKind, size: u64) -> Item {
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
        kind,
        owner,
        place,
        path,
        size_bytes: size,
        portability: Portability::Portable,
        managed_by: ManagedBy::Personal,
        download_bytes: None,
        inclusion: Inclusion::Included,
    }
}

fn file(role: FolderRole, path: &[&str]) -> Item {
    item(role, path, ItemKind::File, 10)
}

fn name(items: &[Item], id: ItemId) -> String {
    let i = items.iter().find(|i| i.id == id).unwrap();
    i.path
        .parts()
        .iter()
        .map(|n| n.display())
        .collect::<Vec<_>>()
        .join("/")
}

fn ordered(items: &[Item], asked: &[ItemId], modified: &BTreeMap<ItemId, i64>) -> Vec<String> {
    plan_order(items, asked, modified, NOW)
        .into_iter()
        .map(|(id, _)| name(items, id))
        .collect()
}

#[test]
fn asked_first_then_essentials_then_the_rest_newest_first() {
    let items = vec![
        file(FolderRole::Music, &["old song.mp3"]),
        file(FolderRole::Music, &["new song.mp3"]),
        file(FolderRole::Documents, &["cv.docx"]),
        file(FolderRole::Desktop, &["todo.txt"]),
        file(FolderRole::Videos, &["holiday.mp4"]),
        file(FolderRole::Pictures, &["fresh.jpg"]),
    ];
    let mut modified = BTreeMap::new();
    for (i, days) in [
        (0usize, 400i64),
        (1, 60),
        (2, 300),
        (3, 200),
        (4, 90),
        (5, 2),
    ] {
        modified.insert(items[i].id, NOW - days * DAY_NS);
    }
    let asked = vec![items[4].id];
    assert_eq!(
        ordered(&items, &asked, &modified),
        [
            // Asked for first.
            "holiday.mp4",
            // Essentials: recent files, documents and the desktop, newest first.
            "fresh.jpg",
            "todo.txt",
            "cv.docx",
            // Everything else, newest first.
            "new song.mp3",
            "old song.mp3",
        ]
    );
    let tiers: Vec<Tier> = plan_order(&items, &asked, &modified, NOW)
        .into_iter()
        .map(|(_, t)| t)
        .collect();
    assert_eq!(tiers[0], Tier::AskedFirst { rank: 0 });
    assert_eq!(tiers[1], Tier::Essential);
    assert_eq!(tiers[5], Tier::Rest);
}

#[test]
fn several_requests_keep_the_order_they_were_asked_in() {
    let items = vec![
        file(FolderRole::Music, &["a.mp3"]),
        file(FolderRole::Music, &["b.mp3"]),
        file(FolderRole::Music, &["c.mp3"]),
    ];
    let asked = vec![items[2].id, items[0].id];
    assert_eq!(
        ordered(&items, &asked, &BTreeMap::new()),
        ["c.mp3", "a.mp3", "b.mp3"]
    );
}

#[test]
fn asking_for_a_folder_covers_everything_inside_it() {
    let items = vec![
        file(FolderRole::Music, &["loose.mp3"]),
        item(FolderRole::Documents, &["Tax"], ItemKind::Folder, 0),
        file(FolderRole::Documents, &["Tax", "2026", "return.pdf"]),
        file(FolderRole::Documents, &["Tax", "receipt.pdf"]),
        file(FolderRole::Documents, &["Taxi.pdf"]),
        file(FolderRole::Pictures, &["Tax", "photo.jpg"]),
    ];
    let asked = vec![items[1].id];
    let order = ordered(&items, &asked, &BTreeMap::new());
    // The folder's own files, not a lookalike name and not another place's folder of that name.
    let mut first_two = order[..2].to_vec();
    first_two.sort();
    assert_eq!(first_two, ["Tax/2026/return.pdf", "Tax/receipt.pdf"]);
    assert!(
        !order.contains(&"Tax".to_string()),
        "folder entries are not sent"
    );
    assert_eq!(order.len(), 5);
}

#[test]
fn left_out_items_are_not_queued() {
    let mut cloud = file(FolderRole::Documents, &["cloud.docx"]);
    cloud.inclusion = Inclusion::LeftOut {
        reason: LeftOutReason::CloudOnly,
    };
    let items = vec![cloud, file(FolderRole::Documents, &["here.docx"])];
    assert_eq!(ordered(&items, &[], &BTreeMap::new()), ["here.docx"]);
}

#[test]
fn whole_packages_and_apps_are_queued_like_files() {
    let items = vec![
        item(
            FolderRole::Documents,
            &["Report.pages"],
            ItemKind::Folder,
            300,
        ),
        item(FolderRole::Documents, &["Empty"], ItemKind::Folder, 0),
    ];
    assert_eq!(ordered(&items, &[], &BTreeMap::new()), ["Report.pages"]);
}

#[test]
fn the_order_is_the_same_every_time() {
    let items: Vec<Item> = (0..50)
        .map(|i| file(FolderRole::Music, &[&format!("{i}.mp3")]))
        .collect();
    let a = ordered(&items, &[], &BTreeMap::new());
    let b = ordered(&items, &[], &BTreeMap::new());
    assert_eq!(a, b);
}

// ---------- taking turns ----------

fn ids(n: u8) -> Vec<ItemId> {
    (0..n)
        .map(|i| ItemId::from_hex(&format!("{i:02x}{}", "0".repeat(30))).unwrap())
        .collect()
}

#[test]
fn several_files_take_turns_so_a_big_one_never_holds_up_the_rest() {
    let id = ids(6);
    let mut s = Scheduler::new(3);
    for &i in &id {
        s.push(i, Tier::Rest);
    }
    // Three at a time, taking turns.
    let turns: Vec<ItemId> = (0..6).map(|_| s.next_turn().unwrap()).collect();
    assert_eq!(turns, [id[0], id[1], id[2], id[0], id[1], id[2]]);
    // When one finishes, the next waiting file takes its place.
    s.finished(id[1]);
    let turns: Vec<ItemId> = (0..3).map(|_| s.next_turn().unwrap()).collect();
    assert!(
        turns.contains(&id[3]) && !turns.contains(&id[1]),
        "{turns:?}"
    );
    for i in [id[0], id[2], id[3], id[4], id[5]] {
        s.finished(i);
    }
    assert_eq!(s.next_turn(), None);
}

#[test]
fn more_important_files_get_every_turn() {
    let id = ids(3);
    let mut s = Scheduler::new(3);
    s.push(id[0], Tier::Rest);
    s.push(id[1], Tier::Essential);
    s.push(id[2], Tier::Rest);
    let turns: Vec<ItemId> = (0..4).map(|_| s.next_turn().unwrap()).collect();
    assert_eq!(turns, [id[1]; 4]);
    s.finished(id[1]);
    assert!(s.next_turn().is_some());
}

#[test]
fn a_request_during_the_move_jumps_the_queue() {
    let id = ids(6);
    let mut s = Scheduler::new(2);
    for &i in &id[..5] {
        s.push(i, Tier::Rest);
    }
    assert_eq!(s.next_turn(), Some(id[0]));
    // The person asks for file 4 (already waiting) and file 5 (new) first.
    s.ask_first(&[id[4], id[5]]);
    // The request starts at once and gets every turn; the files already in flight just pause
    // where they are (nothing is lost or sent again) and carry on afterwards.
    let next: Vec<ItemId> = (0..3).map(|_| s.next_turn().unwrap()).collect();
    assert_eq!(next, [id[4]; 3]);
    s.finished(id[4]);
    assert_eq!(s.next_turn(), Some(id[5]));
    s.finished(id[5]);
    let back: Vec<ItemId> = (0..2).map(|_| s.next_turn().unwrap()).collect();
    assert!(back.contains(&id[0]) && back.contains(&id[1]), "{back:?}");
}

#[test]
fn asking_for_a_file_already_on_its_way_hurries_it() {
    let id = ids(3);
    let mut s = Scheduler::new(3);
    for &i in &id {
        s.push(i, Tier::Rest);
    }
    assert!(s.next_turn().is_some());
    s.ask_first(&[id[2]]);
    let turns: Vec<ItemId> = (0..3).map(|_| s.next_turn().unwrap()).collect();
    assert_eq!(turns, [id[2]; 3]);
}
