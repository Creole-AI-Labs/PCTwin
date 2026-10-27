//! Focused repair and final changes (Task List 2.7 and 2.9; plan 2.7 section 5; acceptance
//! T27-008 to T27-010).

mod common;

use std::collections::{BTreeMap, BTreeSet};

use common::*;
use pctwin_orchestrate::jobs::{ChildJob, JobId, Scope, Worker};
use pctwin_readiness::ledger::{Observed, Role, Standing};
use pctwin_readiness::repair::{
    Change, Choice, DeltaError, Merge, Prerequisites, RepairError, Stamp, Target, approve,
    final_changes, focus, three_way,
};
use pctwin_record::{ItemId, LeftOutReason};
use pctwin_types::{ItemResult, ItemStatus, Reason};

const READY: Prerequisites = Prerequisites {
    source_connected: true,
    space_fits: Some(true),
};

fn give(l: &mut pctwin_readiness::ledger::Ledger, role: Role, item: ItemId, r: ItemResult) {
    l.accept(
        receipt(l, role, item, Some(r)),
        &Sees(Observed::Matches),
        T0 + 20,
    )
    .unwrap();
}

#[test]
fn fix_these_reruns_only_the_unfinished_and_what_they_need() {
    let (font, project, done_dep, other_failed, left_out, ed) = (
        file("font.ttf"),
        file("project.prproj"),
        file("clip.mov"),
        file("unrelated.txt"),
        left_out("plugin.dll", LeftOutReason::ChosenByUser),
        app("Editor"),
    );
    let (_, mut l) = ledger(vec![
        font.clone(),
        project.clone(),
        done_dep.clone(),
        other_failed.clone(),
        left_out.clone(),
        ed.clone(),
    ]);
    let failed = ItemResult::because(Reason::Unreadable);
    give(&mut l, Role::Transfer, project.id, failed);
    give(&mut l, Role::Transfer, font.id, failed);
    give(&mut l, Role::Transfer, other_failed.id, failed);
    give(
        &mut l,
        Role::Transfer,
        done_dep.id,
        ItemResult::done(ItemStatus::Copied).unwrap(),
    );
    give(
        &mut l,
        Role::Install,
        ed.id,
        ItemResult::because(Reason::InstallFailed),
    );
    let needs = BTreeMap::from([(
        project.id,
        BTreeSet::from([font.id, done_dep.id, left_out.id, ed.id]),
    )]);
    let r = focus(&l, &BTreeSet::from([project.id]), &needs).unwrap();
    assert_eq!(r.items, BTreeSet::from([project.id, font.id, ed.id]));
    assert_eq!(r.kept, BTreeSet::from([done_dep.id]));
    assert_eq!(r.needs_approval, BTreeSet::from([left_out.id]));
    // Picking what is done, not selected or unknown is refused; picking nothing too.
    assert_eq!(
        focus(&l, &BTreeSet::from([done_dep.id]), &needs),
        Err(RepairError::AlreadyDone(done_dep.id))
    );
    assert_eq!(
        focus(&l, &BTreeSet::from([left_out.id]), &needs),
        Err(RepairError::NotSelected(left_out.id))
    );
    let stranger = file("stranger").id;
    assert_eq!(
        focus(&l, &BTreeSet::from([stranger]), &needs),
        Err(RepairError::UnknownItem(stranger))
    );
    assert_eq!(
        focus(&l, &BTreeSet::new(), &needs),
        Err(RepairError::NothingPicked)
    );

    // The original jobs, narrowed to the repair for a new session.
    let jobs = vec![
        ChildJob {
            id: JobId("data".into()),
            worker: Worker::Data,
            source: old_laptop(),
            session: 7,
            scope: Scope::Items {
                laptop: old_laptop(),
                items: BTreeSet::from([project.id, done_dep.id, other_failed.id]),
            },
            after: vec![JobId("install".into()), JobId("settings".into())],
            essential: false,
        },
        ChildJob {
            id: JobId("install".into()),
            worker: Worker::Install,
            source: old_laptop(),
            session: 7,
            scope: Scope::App {
                name: "Editor".into(),
            },
            after: vec![],
            essential: true,
        },
        ChildJob {
            id: JobId("settings".into()),
            worker: Worker::Settings,
            source: old_laptop(),
            session: 7,
            scope: Scope::App {
                name: "Viewer".into(),
            },
            after: vec![],
            essential: false,
        },
    ];
    let job_items = BTreeMap::from([
        (
            JobId("data".into()),
            BTreeSet::from([project.id, done_dep.id, other_failed.id]),
        ),
        (JobId("install".into()), BTreeSet::from([ed.id])),
    ]);
    let rj = r.jobs(&l, &jobs, &job_items, 8, READY).unwrap();
    assert_eq!(rj.jobs.len(), 2);
    let data = rj.jobs.iter().find(|j| j.id.0 == "data").unwrap();
    assert_eq!(
        data.scope,
        Scope::Items {
            laptop: old_laptop(),
            items: BTreeSet::from([project.id])
        }
    );
    assert_eq!(data.session, 8);
    assert_eq!(data.after, vec![JobId("install".into())]);
    assert!(rj.jobs.iter().all(|j| j.session == 8));
    assert!(
        rj.jobs
            .iter()
            .find(|j| j.id.0 == "install")
            .unwrap()
            .essential
    );
    assert_eq!(
        rj.plan.order(),
        [JobId("install".into()), JobId("data".into())]
    );
    assert_eq!(rj.by_transfer, BTreeSet::from([font.id]));

    // Prerequisites are checked again before anything runs.
    for (pre, err) in [
        (
            Prerequisites {
                source_connected: false,
                space_fits: Some(true),
            },
            RepairError::SourceNotConnected,
        ),
        (
            Prerequisites {
                source_connected: true,
                space_fits: Some(false),
            },
            RepairError::NotEnoughSpace,
        ),
        (
            Prerequisites {
                source_connected: true,
                space_fits: None,
            },
            RepairError::SpaceUnknown,
        ),
    ] {
        assert_eq!(r.jobs(&l, &jobs, &job_items, 8, pre).unwrap_err(), err);
    }

    // A repaired item gets its new result; its failure stays in its history; finished work and
    // the item not picked are as they were.
    give(
        &mut l,
        Role::Transfer,
        project.id,
        ItemResult::done(ItemStatus::Copied).unwrap(),
    );
    assert_eq!(
        l.standing(project.id),
        Some(Standing::Done(ItemStatus::Copied))
    );
    assert_eq!(l.line(project.id).unwrap().history[0].result, failed);
    assert_eq!(l.standing(other_failed.id), Some(Standing::Failed));
    assert_eq!(l.line(done_dep.id).unwrap().history.len(), 0);
    // A repair made for an older plan cannot run on a newer one.
    let mut next = record(vec![project.clone()]);
    next.revision = 2;
    l.next_generation(&next, MOVER).unwrap();
    assert_eq!(
        r.jobs(&l, &jobs, &job_items, 9, READY).unwrap_err(),
        RepairError::StalePlan
    );
}

fn stamp(size: u64, t: i64) -> Stamp {
    Stamp {
        size,
        modified_ns: Some(t),
        digest: None,
    }
}

fn changed(mut i: pctwin_record::Item, size: u64, t: i64) -> pctwin_record::Item {
    i.size_bytes = size;
    i.modified_ns = Some(t);
    i
}

/// T27-008, T27-009: changes on the old laptop after the move become a reviewed delta; a copy the
/// person edited on the new laptop is never written over.
#[test]
fn t27_008_009_final_changes_never_overwrite_a_target_edit() {
    let (same, newer, edited, gone_t, unclear, removed) = (
        file("same.txt"),
        file("newer.txt"),
        file("edited.txt"),
        file("gone.txt"),
        file("unclear.txt"),
        file("removed.txt"),
    );
    let approved = record(vec![
        same.clone(),
        newer.clone(),
        edited.clone(),
        gone_t.clone(),
        unclear.clone(),
        removed.clone(),
    ]);
    let mut no_time = unclear.clone();
    no_time.modified_ns = None;
    let brand_new = file("brand-new.txt");
    let mut again = record(vec![
        same.clone(),
        changed(newer.clone(), 11, 2),
        changed(edited.clone(), 12, 2),
        changed(gone_t.clone(), 13, 2),
        no_time,
        brand_new.clone(),
        left_out("new-but-left-out.tmp", LeftOutReason::System),
    ]);
    again.revision = 2;
    let landed = stamp(10, 1);
    let target = BTreeMap::from([
        (
            newer.id,
            Target::Landed {
                at_move: landed,
                now: Some(landed),
            },
        ),
        (
            edited.id,
            Target::Landed {
                at_move: landed,
                now: Some(stamp(10, 99)),
            },
        ),
        (
            gone_t.id,
            Target::Landed {
                at_move: landed,
                now: None,
            },
        ),
    ]);
    let delta = final_changes(&approved, &again, &target).unwrap();
    let got: BTreeMap<ItemId, Change> = delta.lines.iter().map(|l| (l.item, l.change)).collect();
    assert_eq!(
        got,
        BTreeMap::from([
            (newer.id, Change::Updated),
            (edited.id, Change::BothChanged),
            (gone_t.id, Change::TargetGone),
            (unclear.id, Change::CannotTell),
            (brand_new.id, Change::New),
            (removed.id, Change::GoneAtSource),
        ])
    );
    let seen = delta.digest();
    // Unreviewed conflicts, and sending over an edited copy, are refused.
    assert_eq!(
        approve(&delta, seen, &approved, &again, &BTreeMap::new()),
        Err(DeltaError::NeedsChoice(edited.id))
    );
    let mut choices = BTreeMap::from([
        (edited.id, Choice::Send),
        (gone_t.id, Choice::KeepNewLaptopVersion),
        (unclear.id, Choice::KeepBoth),
    ]);
    assert_eq!(
        approve(&delta, seen, &approved, &again, &choices),
        Err(DeltaError::WouldOverwrite(edited.id))
    );
    choices.insert(edited.id, Choice::KeepBoth);
    // A review of another delta, or of a changed scan, is stale.
    assert_eq!(
        approve(&delta, [0; 32], &approved, &again, &choices),
        Err(DeltaError::Stale)
    );
    let mut again2 = again.clone();
    again2.items.pop();
    assert_eq!(
        approve(&delta, seen, &approved, &again2, &choices),
        Err(DeltaError::Stale)
    );
    let mut stray = choices.clone();
    stray.insert(same.id, Choice::Send);
    assert_eq!(
        approve(&delta, seen, &approved, &again, &stray),
        Err(DeltaError::NotInDelta(same.id))
    );
    let f = approve(&delta, seen, &approved, &again, &choices).unwrap();
    let sent: BTreeSet<ItemId> = f.record.items.iter().map(|i| i.id).collect();
    assert_eq!(
        sent,
        BTreeSet::from([newer.id, edited.id, unclear.id, brand_new.id])
    );
    assert_eq!(f.newer, BTreeSet::from([newer.id]));
    assert_eq!(f.beside, BTreeSet::from([edited.id, unclear.id]));
    assert!(f.beside.is_disjoint(&f.newer));
    // The follow-up is the next revision, approved again like any plan; nothing is removed.
    assert_eq!(f.record.revision, approved.revision + 1);
    assert_eq!(f.record.previous, Some(approved.fingerprint()));
    assert!(!sent.contains(&removed.id));
    // The person can keep the new laptop's version of a plain update too.
    let keep = BTreeMap::from([
        (newer.id, Choice::KeepNewLaptopVersion),
        (edited.id, Choice::KeepNewLaptopVersion),
        (gone_t.id, Choice::KeepNewLaptopVersion),
        (unclear.id, Choice::KeepNewLaptopVersion),
    ]);
    let f = approve(&delta, seen, &approved, &again, &keep).unwrap();
    assert_eq!(f.record.items.len(), 1);
    assert!(f.newer.is_empty());
    // Another laptop's scan is refused.
    let mut other = again.clone();
    other.source_laptop = other_laptop();
    assert_eq!(
        final_changes(&approved, &other, &target),
        Err(DeltaError::OtherLaptop)
    );
    // A changed item that never landed is sent as new.
    let d = final_changes(&approved, &again, &BTreeMap::new()).unwrap();
    assert!(
        d.lines
            .iter()
            .any(|l| l.item == edited.id && l.change == Change::New)
    );
}

#[test]
fn stamps_compare_by_fingerprint_first() {
    let a = Stamp {
        size: 1,
        modified_ns: Some(1),
        digest: Some([1; 32]),
    };
    let b = Stamp {
        size: 2,
        modified_ns: Some(2),
        digest: Some([1; 32]),
    };
    let c = Stamp {
        size: 1,
        modified_ns: Some(1),
        digest: Some([2; 32]),
    };
    use pctwin_readiness::repair::Same;
    assert_eq!(a.same(&b), Same::Yes);
    assert_eq!(a.same(&c), Same::No);
    assert_eq!(stamp(1, 1).same(&stamp(2, 1)), Same::No);
    assert_eq!(stamp(1, 1).same(&stamp(1, 2)), Same::No);
    assert_eq!(stamp(1, 1).same(&stamp(1, 1)), Same::Yes);
    let none = Stamp {
        size: 1,
        modified_ns: None,
        digest: None,
    };
    assert_eq!(none.same(&stamp(1, 1)), Same::CannotTell);
    assert_eq!(stamp(1, 1).same(&none), Same::CannotTell);
    assert_eq!(none.same(&stamp(2, 1)), Same::No);
}

/// T27-010: a returned laptop is a three-way comparison, never a blind copy back.
#[test]
fn t27_010_a_return_is_a_three_way_comparison() {
    let base = Some(stamp(1, 1));
    let old_changed = Some(stamp(2, 2));
    let new_changed = Some(stamp(3, 3));
    let same_change = Some(stamp(4, 4));
    let unknown = Some(Stamp {
        size: 1,
        modified_ns: None,
        digest: None,
    });
    let cases = [
        (base, base, base, Merge::Unchanged),
        (base, base, new_changed, Merge::BringBack),
        (base, old_changed, base, Merge::KeepOld),
        (base, old_changed, new_changed, Merge::Conflict),
        (base, same_change, same_change, Merge::SameOnBoth),
        (base, base, None, Merge::RemovedOnNewLaptop),
        (base, None, base, Merge::RemovedOnOldLaptop),
        (base, old_changed, None, Merge::Conflict),
        (base, None, new_changed, Merge::Conflict),
        (base, None, None, Merge::Unchanged),
        (base, unknown, base, Merge::CannotTell),
        (base, base, unknown, Merge::CannotTell),
        (base, unknown, None, Merge::CannotTell),
        (base, None, unknown, Merge::CannotTell),
        // Made after the move.
        (None, None, new_changed, Merge::BringBack),
        (None, old_changed, None, Merge::KeepOld),
        (None, old_changed, new_changed, Merge::Conflict),
        (None, same_change, same_change, Merge::SameOnBoth),
        (None, None, None, Merge::Unchanged),
        (None, unknown, unknown, Merge::CannotTell),
    ];
    for (b, o, n, want) in cases {
        assert_eq!(three_way(b, o, n), want, "{b:?} {o:?} {n:?}");
    }
}
