//! A smaller new laptop (Task List 2.6; acceptance T26-010 to T26-012): every real volume is
//! budgeted, what does not fit is proposed (left in the cloud, an approved offload drive, or left
//! on the old laptop) and never dropped silently, and an offload drive that is not connected is
//! never counted as restored.

mod common;

use std::collections::BTreeMap;

use common::*;
use pctwin_orchestrate::capacity::{
    Accounting, Candidate, Choice, Decisions, KEEP_FREE, Need, NeedKind, OffloadDrive, Outcome,
    Volume, account, budget, propose,
};
use pctwin_record::{CloudProvider, ItemId};

const GB: u64 = 1024 * 1024 * 1024;

fn vol(id: &str, free: Option<u64>) -> Volume {
    Volume {
        id: id.into(),
        free,
    }
}

fn need(volume: &str, kind: NeedKind, bytes: u64) -> Need {
    Need {
        volume: volume.into(),
        kind,
        bytes,
    }
}

fn id(n: &str) -> ItemId {
    file(person("me"), &[n], 0).id
}

/// T26-011: each real volume is budgeted with everything that lands on it: the files, app
/// downloads and unpacking, the change journal and partly copied files. A plan whose files fit
/// one drive can still be short on another.
#[test]
fn every_volume_is_budgeted_with_everything_it_holds() {
    let volumes = [vol("C:", Some(20 * GB)), vol("D:", Some(500 * GB))];
    let needs = [
        need("D:", NeedKind::Files, 300 * GB),
        need("C:", NeedKind::Files, 10 * GB),
        need("C:", NeedKind::AppDownloads, 6 * GB),
        need("C:", NeedKind::Unpacking, 3 * GB),
        need("C:", NeedKind::Journal, GB / 2),
        need("C:", NeedKind::PartlyCopied, GB / 2),
    ];
    let b = budget(&volumes, &needs);
    assert_eq!(b.len(), 2);
    let c = &b[0];
    assert_eq!(c.volume, "C:");
    assert_eq!(c.total, 20 * GB);
    assert_eq!(c.by_kind[&NeedKind::AppDownloads], 6 * GB);
    // 20 GB needed, 20 GB free, 1 GB always kept free: short by 1 GB.
    assert_eq!(c.fits, Some(false));
    assert_eq!(c.short_by, GB);
    let d = &b[1];
    assert_eq!(d.fits, Some(true));
    assert_eq!(d.short_by, 0);
    // Exactly enough fits.
    let b = budget(
        &[vol("C:", Some(11 * GB))],
        &[need("C:", NeedKind::Files, 10 * GB)],
    );
    assert_eq!((b[0].fits, b[0].short_by), (Some(true), 0));
    let b = budget(
        &[vol("C:", Some(11 * GB))],
        &[need("C:", NeedKind::Files, 10 * GB + 1)],
    );
    assert_eq!((b[0].fits, b[0].short_by), (Some(false), 1));
}

/// A volume whose free space is not known, or that is not there at all, is never judged as
/// fitting.
#[test]
fn an_unknown_or_missing_volume_never_fits() {
    let b = budget(
        &[vol("C:", None)],
        &[
            need("C:", NeedKind::Files, 1),
            need("E:", NeedKind::Files, 1),
        ],
    );
    assert_eq!(b.len(), 2);
    assert_eq!(b[0].volume, "C:");
    assert_eq!(b[0].fits, None);
    assert_eq!(b[1].volume, "E:");
    assert_eq!(b[1].fits, None);
    assert_eq!(b[1].free, None);
}

fn cand(n: &str, bytes: u64, essential: bool, safe_in_cloud: bool) -> Candidate {
    Candidate {
        item: id(n),
        volume: "C:".into(),
        bytes,
        essential,
        safe_in_cloud,
        last_changed_ns: None,
    }
}

/// What was used longest ago is proposed first, before anything larger but recent; a file whose
/// time is not known counts as recent (research notes 2.6, 29 October 2026).
#[test]
fn what_was_used_longest_ago_is_proposed_first() {
    let day: i64 = 86_400 * 1_000_000_000;
    let mut old = cand("old", 10, false, false);
    old.last_changed_ns = Some(100 * day);
    let mut recent_big = cand("recent", 50, false, false);
    recent_big.last_changed_ns = Some(900 * day);
    let unknown = cand("unknown", 80, false, false);
    let b = budget(
        &[vol("C:", Some(KEEP_FREE + 100))],
        &[need("C:", NeedKind::Files, 110)],
    );
    // 10 short: the old file alone covers it, though both others are bigger.
    let p = propose(
        &b,
        &[recent_big.clone(), unknown.clone(), old.clone()],
        None,
    );
    assert_eq!(p.choices.keys().copied().collect::<Vec<_>>(), [id("old")]);
    // 55 short: the old one, then the recent one, never the one whose time is unknown.
    let b = budget(
        &[vol("C:", Some(KEEP_FREE + 55))],
        &[need("C:", NeedKind::Files, 110)],
    );
    let p = propose(&b, &[unknown, recent_big, old], None);
    let mut chosen: Vec<_> = p.choices.keys().copied().collect();
    chosen.sort();
    let mut want = vec![id("old"), id("recent")];
    want.sort();
    assert_eq!(chosen, want);
}

/// T26-010: when the new laptop is too small, PCTwin proposes what to keep elsewhere: first what
/// is already safe in a cloud, then the approved offload drive, then leaving things on the old
/// laptop; largest first, never the person's essentials; and only as much as the shortfall.
#[test]
fn a_shortfall_is_met_by_a_proposal_never_by_dropping_things() {
    let b = budget(
        &[vol("C:", Some(12 * GB))],
        &[need("C:", NeedKind::Files, 18 * GB)],
    );
    assert_eq!(b[0].short_by, 7 * GB);
    let candidates = [
        cand("essay", 2 * GB, true, false),
        cand("films", 4 * GB, false, false),
        cand("cloud-trip", GB, false, true),
        cand("games", 3 * GB, false, false),
        cand("small", GB / 4, false, false),
    ];
    let offload = OffloadDrive {
        label: "Passport".into(),
        free: Some(4 * GB + GB),
        approved: true,
        connected: true,
    };
    let p = propose(&b, &candidates, Some(&offload));
    // The cloud copy first (1 GB), then the largest to the offload drive (4 GB fills its room
    // with 1 GB kept free), then the next largest stays on the old laptop (3 GB). The essay never.
    assert_eq!(
        p.choices,
        BTreeMap::from([
            (id("cloud-trip"), Choice::LeaveInCloud),
            (id("films"), Choice::Offload),
            (id("games"), Choice::LeaveOnOldLaptop),
        ])
    );
    assert_eq!(p.still_short, BTreeMap::new());
    // The offload drive keeps room free too: 4 GB free does not take a 4 GB file.
    let tight = OffloadDrive {
        free: Some(4 * GB),
        ..offload.clone()
    };
    let p = propose(&b, &candidates, Some(&tight));
    assert_eq!(p.choices[&id("films")], Choice::LeaveOnOldLaptop);
    // Without an approved, connected offload drive, nothing goes to one.
    for drive in [
        OffloadDrive {
            approved: false,
            ..offload.clone()
        },
        OffloadDrive {
            connected: false,
            ..offload.clone()
        },
        OffloadDrive {
            free: None,
            ..offload.clone()
        },
    ] {
        let p = propose(&b, &candidates, Some(&drive));
        assert!(
            !p.choices.values().any(|c| *c == Choice::Offload),
            "{drive:?}"
        );
        assert_eq!(p.choices[&id("films")], Choice::LeaveOnOldLaptop);
        assert_eq!(p.choices[&id("games")], Choice::LeaveOnOldLaptop);
    }
    // When even everything but the essentials is not enough, the rest is said, never forced.
    let b = budget(
        &[vol("C:", Some(2 * GB))],
        &[need("C:", NeedKind::Files, 18 * GB)],
    );
    let p = propose(&b, &candidates, None);
    assert_eq!(p.choices.len(), 4);
    assert!(!p.choices.contains_key(&id("essay")));
    assert_eq!(
        p.still_short,
        BTreeMap::from([("C:".to_string(), 17 * GB - (4 * GB + GB + 3 * GB + GB / 4))])
    );
    // A volume that fits needs nothing proposed; candidates on other volumes are not used.
    let b = budget(
        &[vol("C:", Some(100 * GB)), vol("D:", Some(GB))],
        &[
            need("C:", NeedKind::Files, GB),
            need("D:", NeedKind::Files, GB),
        ],
    );
    let p = propose(&b, &candidates, None);
    assert!(p.choices.is_empty());
    assert_eq!(p.still_short, BTreeMap::from([("D:".to_string(), GB)]));
}

/// T26-012 and nothing dropped silently: every selected item ends in exactly one place; an
/// offload drive that is not connected is "on the offload drive, not connected", never on the
/// new laptop; an item with no result is missing, never left out of the count.
#[test]
fn every_selected_item_is_accounted_for_and_an_absent_offload_drive_is_not_restored() {
    let ids: Vec<ItemId> = ["a", "b", "c", "d", "e", "f", "g"]
        .iter()
        .map(|n| id(n))
        .collect();
    let mut decisions = Decisions::default();
    decisions.choices.insert(ids[1], Choice::Offload);
    decisions.choices.insert(ids[2], Choice::LeaveInCloud);
    decisions.choices.insert(ids[3], Choice::LeaveOnOldLaptop);
    decisions.choices.insert(ids[5], Choice::Offload);
    // What the move reported: a copied, b on the offload drive, f failed on the offload drive,
    // e and g never reported.
    let copied = BTreeMap::from([(ids[0], true), (ids[1], true), (ids[5], false)]);
    for connected in [true, false] {
        let acc: Accounting = account(&ids, &decisions, &copied, connected);
        let mut seen: Vec<ItemId> = acc.by_outcome.values().flatten().copied().collect();
        seen.sort();
        let mut want = ids.clone();
        want.sort();
        assert_eq!(seen, want, "each selected item exactly once");
        assert_eq!(acc.by_outcome[&Outcome::OnNewLaptop], [ids[0]]);
        assert_eq!(acc.by_outcome[&Outcome::LeftInCloud], [ids[2]]);
        assert_eq!(acc.by_outcome[&Outcome::OnlyOnOldLaptop], [ids[3]]);
        let mut missing = acc.by_outcome[&Outcome::Missing].clone();
        missing.sort();
        let mut want_missing = vec![ids[4], ids[5], ids[6]];
        want_missing.sort();
        assert_eq!(missing, want_missing);
        if connected {
            assert_eq!(acc.by_outcome[&Outcome::OnOffloadDrive], [ids[1]]);
            assert!(
                !acc.by_outcome
                    .contains_key(&Outcome::OffloadDriveNotConnected)
            );
        } else {
            assert_eq!(acc.by_outcome[&Outcome::OffloadDriveNotConnected], [ids[1]]);
            assert!(!acc.by_outcome.contains_key(&Outcome::OnOffloadDrive));
        }
        // What is only on the old laptop joins that list, so nobody wipes it believing it moved.
        assert_eq!(acc.still_only_on_old_laptop, [ids[3]]);
        assert!(!acc.by_outcome[&Outcome::OnNewLaptop].contains(&ids[1]));
    }
    assert_eq!(
        Outcome::OffloadDriveNotConnected.words(),
        "On the offload drive, which is not connected now. Connect it to open these."
    );
    assert_eq!(
        Outcome::OnlyOnOldLaptop.words(),
        "Still only on the old laptop: keep it until these are moved."
    );
    assert_eq!(
        Outcome::Missing.words(),
        "Not on the new laptop: PCTwin has no result for these. Try the move again for them."
    );
}

/// A proposal is only a proposal: the person's selection does not change until they approve
/// the decisions, and the plan is not ready while any volume is short.
#[test]
fn a_proposal_changes_nothing_until_approved() {
    let b = budget(
        &[vol("C:", Some(5 * GB))],
        &[need("C:", NeedKind::Files, 8 * GB)],
    );
    let candidates = [cand("films", 4 * GB, false, false)];
    let p = propose(&b, &candidates, None);
    let none = Decisions::default();
    assert!(!none.covers(&b, &candidates));
    let approved = Decisions {
        choices: p.choices.clone(),
    };
    assert!(approved.covers(&b, &candidates));
    // Choosing less than the shortfall does not cover it.
    let small = [cand("films", GB, false, false)];
    let d = Decisions {
        choices: BTreeMap::from([(id("films"), Choice::LeaveOnOldLaptop)]),
    };
    assert!(!d.covers(&b, &small));
    // A volume of unknown free space is never covered.
    let unknown = budget(&[vol("C:", None)], &[need("C:", NeedKind::Files, 1)]);
    assert!(!approved.covers(&unknown, &candidates));
    // Leaving in the cloud only counts for what is safe in a cloud.
    let cloudy = Decisions {
        choices: BTreeMap::from([(id("films"), Choice::LeaveInCloud)]),
    };
    assert!(!cloudy.covers(&b, &candidates));
    let _ = CloudProvider::OneDrive;
}
