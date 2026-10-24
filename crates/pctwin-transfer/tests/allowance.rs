//! The new laptop holds the plan the person approved (security design notes, fresh-context review of
//! 8 October 2026): a file starts only if it is an included item of that exact approved revision,
//! exactly where the plan says it lands, no bigger than its approved size allows, and within the
//! plan's total. The old laptop's word for a place or a size is never enough to write to or
//! reserve the new laptop's disk.

use pctwin_record::{
    FolderRole, Inclusion, Item, ItemId, ItemKind, ItemName, ItemPath, LaptopId, LeftOutReason,
    ManagedBy, Owner, PersonTarget, Place, Portability, Record, Storage,
};
use pctwin_transfer::{Allowance, LandingPlace, Refusal, SizeRoom, Stamp, shown_path};

const MIB: u64 = 1024 * 1024;

/// Where a test plan's files land: the documents folder, at their own paths.
fn lands(i: &Item) -> Option<LandingPlace> {
    Some(LandingPlace::new("documents", shown_path(&i.path)))
}

fn item(n: u8, size: u64, kind: ItemKind, inclusion: Inclusion) -> Item {
    let laptop = LaptopId::from_hex("00112233445566778899aabbccddeeff").unwrap();
    let owner = Owner::Person {
        account_id: "1000".into(),
    };
    let place = Place {
        role: FolderRole::Documents,
        storage: Storage::SystemDrive,
    };
    let path = ItemPath::new(vec![ItemName::from_text(&format!("f{n}"))]).unwrap();
    Item {
        id: ItemId::derive(&laptop, &owner, &place, &path),
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

fn record(items: Vec<Item>) -> Record {
    let mut r = Record::new(LaptopId::from_hex("00112233445566778899aabbccddeeff").unwrap());
    r.items = items;
    r.mapping.people.insert(
        "1000".into(),
        PersonTarget::ExistingAccount {
            account_id: "1001".into(),
        },
    );
    r
}

#[test]
fn only_included_files_of_the_approved_revision_are_allowed() {
    let file = item(1, 100 * MIB, ItemKind::File, Inclusion::Included);
    let left_out = item(
        2,
        MIB,
        ItemKind::File,
        Inclusion::LeftOut {
            reason: LeftOutReason::ChosenByUser,
        },
    );
    let folder = item(3, 0, ItemKind::Folder, Inclusion::Included);
    let r = record(vec![file.clone(), left_out.clone(), folder.clone()]);
    let approval = r.approve().unwrap();
    let mut a = Allowance::from_record(&r, &approval, lands).unwrap();
    assert_eq!(
        a.admit(file.id, "documents", &shown_path(&file.path), 100 * MIB),
        Ok(())
    );
    assert_eq!(
        a.admit(left_out.id, "documents", &shown_path(&left_out.path), MIB),
        Err(Refusal::NotInPlan)
    );
    assert_eq!(
        a.admit(folder.id, "documents", &shown_path(&folder.path), 0),
        Err(Refusal::NotInPlan)
    );
    let stranger = item(9, MIB, ItemKind::File, Inclusion::Included);
    assert_eq!(
        a.admit(stranger.id, "documents", &shown_path(&stranger.path), MIB),
        Err(Refusal::NotInPlan)
    );
}

#[test]
fn an_approval_for_another_revision_gives_no_allowance() {
    let r = record(vec![item(1, MIB, ItemKind::File, Inclusion::Included)]);
    let approval = r.approve().unwrap();
    let mut changed = r.next_revision();
    changed.items[0].size_bytes = 50 * MIB;
    assert!(Allowance::from_record(&changed, &approval, lands).is_err());
}

#[test]
fn a_file_may_grow_a_little_since_the_plan_but_not_a_lot() {
    let big = item(1, 100 * MIB, ItemKind::File, Inclusion::Included);
    let small = item(2, 1000, ItemKind::File, Inclusion::Included);
    let r = record(vec![big.clone(), small.clone()]);
    let mut a = Allowance::from_record(&r, &r.approve().unwrap(), lands).unwrap();
    // 10% more is fine; any more is refused.
    assert_eq!(
        a.admit(big.id, "documents", &shown_path(&big.path), 110 * MIB),
        Ok(())
    );
    let mut b = Allowance::from_record(&r, &r.approve().unwrap(), lands).unwrap();
    assert_eq!(
        b.admit(big.id, "documents", &shown_path(&big.path), 110 * MIB + 1),
        Err(Refusal::GrewTooMuch {
            approved: 100 * MIB,
            announced: 110 * MIB + 1
        })
    );
    // A small file has at least 1 MiB of room (a document saved again).
    assert_eq!(
        a.admit(small.id, "documents", &shown_path(&small.path), 1000 + MIB),
        Ok(())
    );
    let mut c = Allowance::from_record(&r, &r.approve().unwrap(), lands).unwrap();
    assert!(matches!(
        c.admit(small.id, "documents", &shown_path(&small.path), 1001 + MIB),
        Err(Refusal::GrewTooMuch { .. })
    ));
    // Shrinking is always fine.
    let mut d = Allowance::from_record(&r, &r.approve().unwrap(), lands).unwrap();
    assert_eq!(
        d.admit(big.id, "documents", &shown_path(&big.path), 0),
        Ok(())
    );
}

#[test]
fn the_room_for_growth_cannot_be_used_over_and_over_to_fill_the_disk() {
    // 10,000 tiny files: each may grow by 1 MiB, but together not past the plan's total room.
    let items: Vec<Item> = (0..10_000u32)
        .map(|n| {
            let mut i = item(0, 10, ItemKind::File, Inclusion::Included);
            i.path = ItemPath::new(vec![ItemName::from_text(&format!("t{n}"))]).unwrap();
            i.id = ItemId::derive(
                &LaptopId::from_hex("00112233445566778899aabbccddeeff").unwrap(),
                &i.owner,
                &i.place,
                &i.path,
            );
            i
        })
        .collect();
    let total: u64 = items.iter().map(|i| i.size_bytes).sum();
    let r = record(items.clone());
    let mut a = Allowance::from_record(&r, &r.approve().unwrap(), lands).unwrap();
    let mut admitted = 0u64;
    let mut refused = false;
    for i in &items {
        match a.admit(i.id, "documents", &shown_path(&i.path), 10 + MIB) {
            Ok(()) => admitted += 10 + MIB,
            Err(Refusal::OverTotal) => {
                refused = true;
                break;
            }
            Err(e) => panic!("{e:?}"),
        }
    }
    assert!(refused);
    assert!(admitted <= total + total / 10 + 64 * MIB, "{admitted}");
}

#[test]
fn a_file_sent_again_counts_once_toward_the_total() {
    let f = item(1, 100 * MIB, ItemKind::File, Inclusion::Included);
    let r = record(vec![f.clone()]);
    let mut a = Allowance::from_record(&r, &r.approve().unwrap(), lands).unwrap();
    // It changed while being read and is sent again (each attempt ends before the next).
    assert_eq!(
        a.admit(f.id, "documents", &shown_path(&f.path), 100 * MIB),
        Ok(())
    );
    a.ended(f.id, false);
    assert_eq!(
        a.admit(f.id, "documents", &shown_path(&f.path), 100 * MIB),
        Ok(())
    );
    a.ended(f.id, false);
    assert_eq!(a.started_bytes(), 100 * MIB);
    // Sent again bigger (still within its room): the larger size counts, once.
    assert_eq!(
        a.admit(f.id, "documents", &shown_path(&f.path), 105 * MIB),
        Ok(())
    );
    a.ended(f.id, false);
    assert_eq!(a.started_bytes(), 105 * MIB);
    assert_eq!(
        a.admit(f.id, "documents", &shown_path(&f.path), 90 * MIB),
        Ok(())
    );
    assert_eq!(a.started_bytes(), 105 * MIB);
}

/// `n` plan files named `prefix0`, `prefix1`, ..., each `size` bytes.
fn files(prefix: &str, n: u32, size: u64) -> Vec<Item> {
    (0..n)
        .map(|k| {
            let mut i = item(0, size, ItemKind::File, Inclusion::Included);
            i.path = ItemPath::new(vec![ItemName::from_text(&format!("{prefix}{k}"))]).unwrap();
            i.id = ItemId::derive(
                &LaptopId::from_hex("00112233445566778899aabbccddeeff").unwrap(),
                &i.owner,
                &i.place,
                &i.path,
            );
            i
        })
        .collect()
}

#[test]
fn every_file_of_a_big_plan_may_use_its_own_room_to_grow() {
    // Ten 1 GiB files that each grew by 10%: 1 GiB more in all, which the plan's tenth covers.
    let items = files("v", 10, 1024 * MIB);
    let r = record(items.clone());
    let mut a = Allowance::from_record(&r, &r.approve().unwrap(), lands).unwrap();
    for i in &items {
        assert_eq!(
            a.admit(
                i.id,
                "documents",
                &shown_path(&i.path),
                1024 * MIB + 1024 * MIB / 10
            ),
            Ok(())
        );
    }
}

#[test]
fn the_whole_move_may_reach_its_limit_exactly_and_no_further() {
    // One 1,000 MiB file and 65 empty ones: the move's limit is 1,000 + 100 + 64 MiB.
    let big = files("big", 1, 1000 * MIB);
    let empty = files("e", 65, 0);
    let r = record(big.iter().chain(&empty).cloned().collect());
    let mut a = Allowance::from_record(&r, &r.approve().unwrap(), lands).unwrap();
    assert_eq!(
        a.admit(
            big[0].id,
            "documents",
            &shown_path(&big[0].path),
            1100 * MIB
        ),
        Ok(())
    );
    // Each empty file may grow to 1 MiB: 64 of them reach the limit exactly.
    for e in &empty[..64] {
        assert_eq!(
            a.admit(e.id, "documents", &shown_path(&e.path), MIB),
            Ok(())
        );
    }
    assert_eq!(a.started_bytes(), 1164 * MIB);
    assert_eq!(
        a.admit(empty[64].id, "documents", &shown_path(&empty[64].path), MIB),
        Err(Refusal::OverTotal)
    );
    // Nothing more was counted for the refused one.
    assert_eq!(a.started_bytes(), 1164 * MIB);
}

#[test]
fn a_file_that_landed_is_spent_and_cannot_be_started_again() {
    let f = item(1, MIB, ItemKind::File, Inclusion::Included);
    let r = record(vec![f.clone()]);
    let mut a = Allowance::from_record(&r, &r.approve().unwrap(), lands).unwrap();
    assert_eq!(
        a.admit(f.id, "documents", &shown_path(&f.path), MIB),
        Ok(())
    );
    a.ended(f.id, true);
    assert_eq!(
        a.admit(f.id, "documents", &shown_path(&f.path), MIB),
        Err(Refusal::AlreadyMoved)
    );
}

#[test]
fn one_attempt_at_a_file_runs_at_a_time() {
    let f = item(1, MIB, ItemKind::File, Inclusion::Included);
    let r = record(vec![f.clone()]);
    let mut a = Allowance::from_record(&r, &r.approve().unwrap(), lands).unwrap();
    assert_eq!(
        a.admit(f.id, "documents", &shown_path(&f.path), MIB),
        Ok(())
    );
    assert_eq!(
        a.admit(f.id, "documents", &shown_path(&f.path), MIB),
        Err(Refusal::AlreadyStarted)
    );
    // Once that attempt ends without landing, another may start.
    a.ended(f.id, false);
    assert_eq!(
        a.admit(f.id, "documents", &shown_path(&f.path), MIB),
        Ok(())
    );
}

#[test]
fn a_file_that_keeps_changing_gets_a_few_tries_then_is_reported() {
    let f = item(1, MIB, ItemKind::File, Inclusion::Included);
    let r = record(vec![f.clone()]);
    let mut a = Allowance::from_record(&r, &r.approve().unwrap(), lands).unwrap();
    for _ in 0..pctwin_transfer::MAX_ATTEMPTS {
        assert_eq!(
            a.admit(f.id, "documents", &shown_path(&f.path), MIB),
            Ok(())
        );
        a.ended(f.id, false);
    }
    assert_eq!(
        a.admit(f.id, "documents", &shown_path(&f.path), MIB),
        Err(Refusal::KeptChanging)
    );
}

#[test]
fn a_file_lands_only_where_the_plan_says() {
    let f = item(1, MIB, ItemKind::File, Inclusion::Included);
    let r = record(vec![f.clone()]);
    let mut a = Allowance::from_record(&r, &r.approve().unwrap(), lands).unwrap();
    assert_eq!(a.destination(f.id), Some("documents"));
    assert_eq!(a.check_place(f.id, "documents", "f1"), Ok(()));
    // Another path in the right place, or the right path in another place: refused, and never
    // counted as an attempt (however often it is tried).
    for _ in 0..=pctwin_transfer::MAX_ATTEMPTS {
        assert_eq!(
            a.admit(
                f.id,
                "documents",
                "AppData/Roaming/Microsoft/Windows/Start Menu/Programs/Startup/run.cmd",
                MIB
            ),
            Err(Refusal::NotWherePlanned)
        );
        assert_eq!(
            a.admit(f.id, "desktop", "f1", MIB),
            Err(Refusal::NotWherePlanned)
        );
        assert_eq!(
            a.check_place(f.id, "documents", "f1/../f1"),
            Err(Refusal::NotWherePlanned)
        );
    }
    assert_eq!(a.started_bytes(), 0);
    assert_eq!(a.check_place(f.id, "documents", "f1"), Ok(()));
    assert_eq!(a.admit(f.id, "documents", "f1", MIB), Ok(()));
}

#[test]
fn a_file_the_landing_gives_no_place_is_not_in_the_plan() {
    let kept = item(1, MIB, ItemKind::File, Inclusion::Included);
    let elsewhere = item(2, MIB, ItemKind::File, Inclusion::Included);
    let r = record(vec![kept.clone(), elsewhere.clone()]);
    let skip = elsewhere.id;
    let mut a = Allowance::from_record(&r, &r.approve().unwrap(), |i| {
        (i.id != skip).then(|| LandingPlace::new("documents", shown_path(&i.path)))
    })
    .unwrap();
    assert_eq!(a.destination(skip), None);
    assert_eq!(
        a.admit(skip, "documents", "f2", MIB),
        Err(Refusal::NotInPlan)
    );
    assert_eq!(a.admit(kept.id, "documents", "f1", MIB), Ok(()));
}

#[test]
fn with_no_growth_a_file_may_not_be_one_byte_bigger_than_approved() {
    let f = item(1, 1000, ItemKind::File, Inclusion::Included);
    let r = record(vec![f.clone()]);
    let approval = r.approve().unwrap();
    // A move gives it room to grow.
    let mut grows = Allowance::from_record(&r, &approval, lands).unwrap();
    assert_eq!(grows.admit(f.id, "documents", "f1", 1001), Ok(()));
    let mut exact = Allowance::from_record(&r, &approval, lands)
        .unwrap()
        .with_size_room(SizeRoom::NoGrowth);
    assert_eq!(
        exact.admit(f.id, "documents", "f1", 1001),
        Err(Refusal::Grew {
            approved: 1000,
            announced: 1001
        })
    );
    // Nothing was counted for it, and exactly the approved size (or less) is fine.
    assert_eq!(exact.started_bytes(), 0);
    assert_eq!(exact.admit(f.id, "documents", "f1", 1000), Ok(()));
    exact.ended(f.id, false);
    assert_eq!(exact.admit(f.id, "documents", "f1", 10), Ok(()));
    assert_eq!(exact.started_bytes(), 1000);
}

#[test]
fn the_marker_floor_is_the_least_marker_every_file_gets() {
    use pctwin_journal::DownloadMark::{Internet, Restricted};
    let r = record(vec![item(1, MIB, ItemKind::File, Inclusion::Included)]);
    let approval = r.approve().unwrap();
    // A move keeps the old laptop's markers as they are.
    let a = Allowance::from_record(&r, &approval, lands).unwrap();
    assert_eq!(a.mark_for(None), None);
    assert_eq!(a.mark_for(Some(Internet)), Some(Internet));
    // With a floor, the stricter of the two.
    let floored = a.with_mark_floor(Internet);
    assert_eq!(floored.mark_for(None), Some(Internet));
    assert_eq!(floored.mark_for(Some(Internet)), Some(Internet));
    assert_eq!(floored.mark_for(Some(Restricted)), Some(Restricted));
}

/// A file sent as it is now is "changed since the scan" when its size differs, or when both the
/// scan and the old laptop give its changed time and they differ; with no time on either side,
/// only the size can tell.
#[test]
fn a_file_is_changed_since_the_scan_by_its_size_or_its_changed_time() {
    let mut timed = item(1, 100, ItemKind::File, Inclusion::Included);
    timed.modified_ns = Some(1_700_000_000_000_000_000);
    let untimed = item(2, 100, ItemKind::File, Inclusion::Included);
    let r = record(vec![timed.clone(), untimed.clone()]);
    let a = Allowance::from_record(&r, &r.approve().unwrap(), lands).unwrap();
    let at = |size, modified_ns| Stamp { size, modified_ns };
    let t = timed.modified_ns;
    assert!(!a.changed_since_scan(timed.id, at(100, t)));
    assert!(a.changed_since_scan(timed.id, at(101, t)));
    assert!(a.changed_since_scan(timed.id, at(100, t.map(|t| t + 1))));
    // The old laptop could not say when: only the size counts.
    assert!(!a.changed_since_scan(timed.id, at(100, None)));
    // The scan had no time: only the size counts.
    assert!(!a.changed_since_scan(untimed.id, at(100, Some(5))));
    assert!(a.changed_since_scan(untimed.id, at(99, Some(5))));
    // Not in the plan: never "changed since the scan" (it is refused anyway).
    let stranger = item(9, 100, ItemKind::File, Inclusion::Included);
    assert!(!a.changed_since_scan(stranger.id, at(1, None)));
}

/// Fresh-context review of TRANSFER (24 October 2026): a start that ends before anything of it
/// arrived is not counted as an attempt (a lossy link), but an old laptop that starts a file over
/// and over without sending anything still meets a hard limit, so the journal and the temporary
/// files cannot be churned without end.
#[test]
fn a_file_started_over_and_over_with_nothing_sent_is_refused_in_the_end() {
    let f = item(1, MIB, ItemKind::File, Inclusion::Included);
    let r = record(vec![f.clone()]);
    let mut a = Allowance::from_record(&r, &r.approve().unwrap(), lands).unwrap();
    for _ in 0..pctwin_transfer::MAX_STARTS_PER_FILE {
        assert_eq!(
            a.admit(f.id, "documents", &shown_path(&f.path), MIB),
            Ok(())
        );
        a.ended_unstarted(f.id);
    }
    assert_eq!(
        a.admit(f.id, "documents", &shown_path(&f.path), MIB),
        Err(Refusal::StartedTooOften)
    );
}

/// The same limit for the whole move: however the starts are spread over the plan's files, no
/// more than the move's limit are ever agreed to.
#[test]
fn a_move_agrees_to_only_so_many_starts_in_all() {
    let files: Vec<Item> = (0..=u8::MAX)
        .map(|n| item(n, 10, ItemKind::File, Inclusion::Included))
        .collect();
    let r = record(files.clone());
    let mut a = Allowance::from_record(&r, &r.approve().unwrap(), lands).unwrap();
    let limit = pctwin_transfer::max_starts_per_move(files.len());
    assert!(
        limit < files.len() as u64 * u64::from(pctwin_transfer::MAX_STARTS_PER_FILE),
        "the test must reach the move's limit before every file's own"
    );
    let mut agreed = 0u64;
    for f in &files {
        for _ in 0..pctwin_transfer::MAX_STARTS_PER_FILE {
            match a.admit(f.id, "documents", &shown_path(&f.path), 10) {
                Ok(()) => {
                    agreed += 1;
                    a.ended_unstarted(f.id);
                }
                Err(e) => assert_eq!(e, Refusal::StartedTooOften),
            }
        }
    }
    assert_eq!(agreed, limit);
}
