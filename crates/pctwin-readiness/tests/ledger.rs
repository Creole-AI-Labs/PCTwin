//! The evidence ledger (Task List 2.7; plan 2.7 section 3; acceptance T27-001 to T27-005, T27-027).

mod common;

use std::collections::{BTreeMap, BTreeSet};

use common::*;
use pctwin_journal::{
    Actor, Entry, Landed, PartialKeep, Permission, PlannedWrite, Scope as JScope, State,
};
use pctwin_orchestrate::jobs::{ChildJob, JobId, Resources, Scheduler, Scope, Worker, seal};
use pctwin_readiness::ledger::{
    Coverage, DeferWhy, Evidence, EvidenceKind, InaccessibleWhy, JournalProbe, Observed, Refused,
    Role, SchedulerProbe, Standing,
};
use pctwin_record::{ItemKind, LeftOutReason};
use pctwin_types::{ItemResult, ItemStatus, Reason};

fn copied() -> Option<ItemResult> {
    Some(ItemResult::done(ItemStatus::Copied).unwrap())
}

#[test]
fn every_item_gets_one_line_and_the_seven_kinds_stay_apart() {
    let (a, b, c, d, e, f) = (
        file("a.txt"),
        file("b.txt"),
        file("c.txt"),
        file("d.txt"),
        left_out("skip.txt", LeftOutReason::ChosenByUser),
        left_out("work.txt", LeftOutReason::WorkManaged),
    );
    let g = file("g.txt");
    let (_, mut l) = ledger(vec![
        a.clone(),
        b.clone(),
        c.clone(),
        d.clone(),
        e.clone(),
        f.clone(),
        g.clone(),
    ]);
    let ok = Sees(Observed::Matches);
    l.accept(receipt(&l, Role::Transfer, a.id, copied()), &ok, T0 + 20)
        .unwrap();
    l.accept(
        receipt(
            &l,
            Role::Transfer,
            b.id,
            Some(ItemResult::because(Reason::Unreadable)),
        ),
        &ok,
        T0 + 20,
    )
    .unwrap();
    l.defer(c.id, DeferWhy::WaitingForAnotherStep).unwrap();
    l.inaccessible(d.id, InaccessibleWhy::DriveNotConnected)
        .unwrap();
    assert_eq!(l.standing(a.id), Some(Standing::Done(ItemStatus::Copied)));
    assert_eq!(l.standing(b.id), Some(Standing::Failed));
    assert_eq!(l.standing(c.id), Some(Standing::Deferred));
    assert_eq!(l.standing(d.id), Some(Standing::Inaccessible));
    assert_eq!(l.standing(e.id), Some(Standing::SkippedByUser));
    assert_eq!(l.standing(f.id), Some(Standing::Excluded));
    assert_eq!(l.standing(g.id), Some(Standing::NotTested));
    let t = l.tally();
    assert_eq!(t.selected, 5);
    assert_eq!(
        (
            t.done,
            t.failed,
            t.deferred,
            t.inaccessible,
            t.skipped,
            t.excluded,
            t.not_tested
        ),
        (1, 1, 1, 1, 1, 1, 1)
    );
    // Every selected item is in exactly one count: nothing disappears between screens.
    assert_eq!(
        t.done + t.needs_action + t.failed + t.deferred + t.inaccessible + t.not_tested,
        t.selected
    );
    // A hold is only for selected work, and an item not in the move cannot be held.
    assert!(l.defer(e.id, DeferWhy::PersonChoseLater).is_err());
}

/// T27-001: a result from the wrong laptop, an older session, an older plan, an unknown item, the
/// wrong part of PCTwin or the future is refused; a "done" the real state does not show is refused
/// too, and nothing is recorded.
#[test]
fn t27_001_forged_stale_and_wrong_device_results_are_refused() {
    let a = file("a.txt");
    let app_item = app("Editor");
    let (_, mut l) = ledger(vec![a.clone(), app_item.clone()]);
    let ok = Sees(Observed::Matches);
    let base = receipt(&l, Role::Transfer, a.id, copied());

    let mut r = base.clone();
    r.source = other_laptop();
    assert_eq!(l.accept(r, &ok, T0 + 20), Err(Refused::WrongDevice));
    let mut r = base.clone();
    r.target = other_laptop();
    assert_eq!(l.accept(r, &ok, T0 + 20), Err(Refused::WrongDevice));
    let mut r = base.clone();
    r.session = 6;
    assert_eq!(l.accept(r, &ok, T0 + 20), Err(Refused::OldSession));
    let mut r = base.clone();
    r.session = 8;
    assert_eq!(l.accept(r, &ok, T0 + 20), Err(Refused::OldSession));
    let mut r = base.clone();
    r.generation = l.generation() - 1;
    assert_eq!(l.accept(r, &ok, T0 + 20), Err(Refused::StalePlan));
    let mut r = base.clone();
    r.generation = l.generation() + 1;
    assert_eq!(l.accept(r, &ok, T0 + 20), Err(Refused::StalePlan));
    let mut r = base.clone();
    r.item = file("not-in-move.txt").id;
    assert_eq!(l.accept(r, &ok, T0 + 20), Err(Refused::UnknownItem));
    // An install result for a file, and a copy result for an app.
    let r = receipt(
        &l,
        Role::Install,
        a.id,
        Some(ItemResult::done(ItemStatus::Reinstalled).unwrap()),
    );
    assert_eq!(l.accept(r, &ok, T0 + 20), Err(Refused::WrongRole));
    let r = receipt(&l, Role::Transfer, app_item.id, copied());
    assert_eq!(l.accept(r, &ok, T0 + 20), Err(Refused::WrongRole));
    // "Skipped by the person" is the person's choice in the plan, never a worker's result.
    let r = receipt(
        &l,
        Role::Transfer,
        a.id,
        Some(ItemResult::skipped_by_user()),
    );
    assert_eq!(l.accept(r, &ok, T0 + 20), Err(Refused::WrongRole));
    // Seen after "now", or before the move began.
    let mut r = base.clone();
    r.observed_at = T0 + 1_000;
    assert_eq!(l.accept(r, &ok, T0 + 20), Err(Refused::BadTime));
    let mut r = base.clone();
    r.observed_at = T0 - 1;
    assert_eq!(l.accept(r, &ok, T0 + 20), Err(Refused::BadTime));
    // The real state does not show it, or cannot tell: a success is never taken on its word.
    assert_eq!(
        l.accept(base.clone(), &Sees(Observed::Differs), T0 + 20),
        Err(Refused::NotWhatIsThere)
    );
    assert_eq!(
        l.accept(base.clone(), &Sees(Observed::CannotTell), T0 + 20),
        Err(Refused::CouldNotConfirm)
    );
    assert_eq!(l.standing(a.id), Some(Standing::NotTested));
    assert!(l.line(a.id).unwrap().evidence.is_empty());
    assert_eq!(l.refused().len(), 14);
    // A failure needs no confirming: erring toward "not done" is always safe.
    let fail = receipt(
        &l,
        Role::Transfer,
        a.id,
        Some(ItemResult::because(Reason::NotVerified)),
    );
    l.accept(fail, &Sees(Observed::Differs), T0 + 20).unwrap();
    assert_eq!(l.standing(a.id), Some(Standing::Failed));
    // And the right one is taken.
    l.accept(base, &ok, T0 + 20).unwrap();
    assert_eq!(l.standing(a.id), Some(Standing::Done(ItemStatus::Copied)));
}

/// Each kind of evidence comes only from the part of PCTwin that can observe it.
#[test]
fn evidence_comes_only_from_its_own_role() {
    let a = file("a.txt");
    let (_, mut l) = ledger(vec![a.clone()]);
    let ok = Sees(Observed::Matches);
    let with = |l: &pctwin_readiness::ledger::Ledger, role, ev| {
        let mut r = receipt(l, role, a.id, None);
        r.evidence = Some(ev);
        r
    };
    let verified = Evidence::VerifiedTransfer {
        fingerprint: [1; 32],
    };
    let person = Evidence::PersonConfirmed {
        task: "Opened the budget".into(),
        coverage: Coverage::Whole,
    };
    assert_eq!(
        l.accept(with(&l, Role::Person, verified.clone()), &ok, T0 + 20),
        Err(Refused::WrongRole)
    );
    assert_eq!(
        l.accept(with(&l, Role::Transfer, person.clone()), &ok, T0 + 20),
        Err(Refused::WrongRole)
    );
    l.accept(with(&l, Role::Transfer, verified), &ok, T0 + 20)
        .unwrap();
    l.accept(with(&l, Role::Person, person), &ok, T0 + 20)
        .unwrap();
    let kinds: Vec<EvidenceKind> = l
        .line(a.id)
        .unwrap()
        .evidence
        .iter()
        .map(|o| o.evidence.kind())
        .collect();
    assert_eq!(
        kinds,
        [
            EvidenceKind::VerifiedTransfer,
            EvidenceKind::PersonConfirmed
        ]
    );
    // Evidence is not a result word: the item has no result yet.
    assert_eq!(l.standing(a.id), Some(Standing::NotTested));
    // The eight kinds, each with its own meaning (plan section 3).
    assert_eq!(EvidenceKind::ALL.len(), 8);
}

/// T27-002: a sample is labelled a sample, never the whole.
#[test]
fn t27_002_a_sample_is_labelled_as_a_sample() {
    assert_eq!(Coverage::sample(10_000, 10_000), Ok(Coverage::Whole));
    let s = Coverage::sample(12, 10_000).unwrap();
    assert!(!s.is_whole());
    assert_eq!(s.words(), "checked 12 of 10000");
    assert_eq!(Coverage::Whole.words(), "checked all");
    assert!(Coverage::sample(0, 5).is_err());
    assert!(Coverage::sample(6, 5).is_err());
    // A sample stays a sample through the ledger.
    let a = file("photos");
    let (_, mut l) = ledger(vec![a.clone()]);
    let mut r = receipt(&l, Role::Person, a.id, None);
    r.evidence = Some(Evidence::PersonConfirmed {
        task: "Looked at some photos".into(),
        coverage: s,
    });
    l.accept(r, &Sees(Observed::Matches), T0 + 20).unwrap();
    assert_eq!(
        l.line(a.id).unwrap().evidence[0].evidence.coverage(),
        Some(s)
    );
}

/// T27-003: a place PCTwin did not search is never covered by "found everything".
#[test]
fn t27_003_an_unsearched_place_is_never_claimed() {
    let a = left_out("draft.txt", LeftOutReason::ChosenByUser);
    let (_, mut l) = ledger(vec![a]);
    l.searched("Documents");
    assert!(l.found_everything());
    l.not_searched("USB drive \"Backup\"");
    assert!(!l.found_everything());
    let words = l.source_only_words();
    assert!(
        words.contains("1 item is only on the old laptop"),
        "{words}"
    );
    assert!(words.contains("in the places PCTwin looked: Documents"));
    assert!(words.contains("PCTwin did not look in: USB drive \"Backup\""));
    assert!(!words.to_lowercase().contains("everything"));
    // Searched later: no longer listed as unsearched.
    l.searched("USB drive \"Backup\"");
    assert!(l.found_everything());
}

/// T27-004: bytes that arrived but cannot be opened without a key are preserved, not usable.
#[test]
fn t27_004_ciphertext_without_its_key_is_preserved_with_access_unresolved() {
    let vault = file("vault.kdbx");
    let (_, mut l) = ledger(vec![vault.clone()]);
    l.accept(
        receipt(&l, Role::Transfer, vault.id, copied()),
        &Sees(Observed::Matches),
        T0 + 20,
    )
    .unwrap();
    l.needs_key(vault.id).unwrap();
    assert_eq!(
        l.access(vault.id),
        Some(pctwin_readiness::ledger::Access::PreservedKeyUnresolved)
    );
    // A recovery check of the exact key makes it usable.
    let mut r = receipt(&l, Role::Data, vault.id, None);
    r.evidence = Some(Evidence::RecoveryChecked {
        what: "key file opened the copy".into(),
        coverage: Coverage::Whole,
    });
    l.accept(r, &Sees(Observed::Matches), T0 + 30).unwrap();
    assert_eq!(
        l.access(vault.id),
        Some(pctwin_readiness::ledger::Access::Usable)
    );
}

/// T27-027: a loss the person accepts is recorded as accepted loss, never as success.
#[test]
fn t27_027_accepted_loss_is_never_success() {
    let a = file("old-draft.txt");
    let done = file("done.txt");
    let (_, mut l) = ledger(vec![a.clone(), done.clone()]);
    l.accept(
        receipt(
            &l,
            Role::Transfer,
            a.id,
            Some(ItemResult::because(Reason::Unreadable)),
        ),
        &Sees(Observed::Matches),
        T0 + 20,
    )
    .unwrap();
    l.accept(
        receipt(&l, Role::Transfer, done.id, copied()),
        &Sees(Observed::Matches),
        T0 + 20,
    )
    .unwrap();
    let (words, digest) = l.loss_warning(a.id).unwrap();
    assert!(words.contains("not on the new laptop"), "{words}");
    // Agreeing to some other warning, or to one for a finished item, is refused.
    assert!(l.accept_loss(a.id, [0; 32], T0 + 30).is_err());
    assert!(l.loss_warning(done.id).is_none());
    assert!(l.accept_loss(done.id, digest, T0 + 30).is_err());
    l.accept_loss(a.id, digest, T0 + 30).unwrap();
    assert_eq!(l.standing(a.id), Some(Standing::Failed));
    let t = l.tally();
    assert_eq!((t.done, t.failed, t.accepted_loss), (1, 1, 1));
    assert!(l.line(a.id).unwrap().accepted_loss.is_some());
    assert!(l.settled(a.id));
    assert!(l.settled(done.id));
}

/// A new result keeps the earlier one in the item's history (focused repair keeps the failure).
#[test]
fn a_later_result_keeps_the_earlier_one_in_history() {
    let a = file("a.txt");
    let (_, mut l) = ledger(vec![a.clone()]);
    let ok = Sees(Observed::Matches);
    let mut first = receipt(
        &l,
        Role::Transfer,
        a.id,
        Some(ItemResult::because(Reason::ChangedDuringMove)),
    );
    first.observed_at = T0 + 15;
    l.accept(first, &ok, T0 + 20).unwrap();
    l.accept(receipt(&l, Role::Transfer, a.id, copied()), &ok, T0 + 40)
        .unwrap();
    let line = l.line(a.id).unwrap();
    assert_eq!(line.history.len(), 1);
    assert_eq!(
        line.history[0].result.reason(),
        Some(Reason::ChangedDuringMove)
    );
    assert_eq!(line.history[0].at, T0 + 15);
    assert_eq!(l.standing(a.id), Some(Standing::Done(ItemStatus::Copied)));
}

fn entry(
    id: u64,
    item: pctwin_record::ItemId,
    source: pctwin_record::LaptopId,
    state: State,
) -> Entry {
    Entry {
        id,
        write: PlannedWrite {
            item,
            source_laptop: source,
            scope: JScope([9; 16]),
            destination: "Documents".into(),
            path: "a.txt".into(),
            size: 10,
            actor: Actor {
                acting_account: "2000".into(),
                for_account: "2000".into(),
                permission: Permission::OwnFolders,
            },
            block_size: 1 << 20,
            source_modified_ns: Some(1),
            place: None,
            source_file: None,
            partial_keep: PartialKeep::default(),
            download_mark: None,
        },
        state,
    }
}

fn committed(fp: [u8; 32]) -> State {
    State::Committed {
        final_path: "a.txt".into(),
        fingerprint: fp,
        landed: Landed {
            size: 10,
            modified_ns: Some(1),
            file: None,
        },
    }
}

/// The journal on disk is the real state of a copy: its probe confirms only what it shows, for
/// this move and this old laptop, and not what undo took back.
#[test]
fn the_journal_probe_confirms_only_what_the_journal_shows() {
    let (a, b, c, d, e) = (
        file("a.txt"),
        file("b.txt"),
        file("c.txt"),
        file("d.txt"),
        file("e.txt"),
    );
    let (_, mut l) = ledger(vec![a.clone(), b.clone(), c.clone(), d.clone(), e.clone()]);
    let entries = vec![
        entry(1, a.id, old_laptop(), committed([1; 32])),
        entry(2, b.id, other_laptop(), committed([2; 32])),
        entry(3, c.id, old_laptop(), committed([3; 32])),
        entry(
            4,
            d.id,
            old_laptop(),
            State::Existing {
                stored_path: "d.txt".into(),
            },
        ),
        entry(
            5,
            e.id,
            old_laptop(),
            State::Failed {
                why: "x".into(),
                reached: Box::new(committed([5; 32])),
            },
        ),
    ];
    let undone = BTreeSet::from([3]);
    let probe = JournalProbe::from_entries(&entries, &undone, old_laptop(), JScope([9; 16]));
    let ok = |l: &mut pctwin_readiness::ledger::Ledger, item, fp: Option<[u8; 32]>| {
        let mut r = receipt(l, Role::Transfer, item, copied());
        r.evidence = fp.map(|fingerprint| Evidence::VerifiedTransfer { fingerprint });
        l.accept(r, &probe, T0 + 20)
    };
    assert_eq!(ok(&mut l, a.id, Some([1; 32])), Ok(()));
    assert_eq!(
        ok(&mut l, a.id, Some([7; 32])),
        Err(Refused::NotWhatIsThere)
    );
    // Another old laptop's write, a copy undone, a failed write: not this item's copy.
    assert_eq!(ok(&mut l, b.id, None), Err(Refused::NotWhatIsThere));
    assert_eq!(ok(&mut l, c.id, None), Err(Refused::NotWhatIsThere));
    assert_eq!(ok(&mut l, e.id, None), Err(Refused::NotWhatIsThere));
    // An identical file already there: copied, but there is no fingerprint of PCTwin's to match.
    assert_eq!(ok(&mut l, d.id, None), Ok(()));
    assert_eq!(
        ok(&mut l, d.id, Some([4; 32])),
        Err(Refused::CouldNotConfirm)
    );
    // Another move's entries (another journal scope) are not this move's.
    let other_scope = JournalProbe::from_entries(&entries, &undone, old_laptop(), JScope([8; 16]));
    let r = receipt(&l, Role::Transfer, a.id, copied());
    assert_eq!(
        l.accept(r, &other_scope, T0 + 20),
        Err(Refused::NotWhatIsThere)
    );
}

/// The 2.6 scheduler is the real state of a child job: a child's "finished" counts only for a
/// job the scheduler itself took as done, and only for items inside that job's scope.
#[test]
fn the_scheduler_probe_confirms_only_finished_jobs_in_scope() {
    let a = file("a.txt");
    let b = file("b.txt");
    let ed = app("Editor");
    let (_, mut l) = ledger(vec![a.clone(), b.clone(), ed.clone()]);
    let jobs = vec![
        ChildJob {
            id: JobId("data".into()),
            worker: Worker::Data,
            source: old_laptop(),
            session: 7,
            scope: Scope::Items {
                laptop: old_laptop(),
                items: BTreeSet::from([a.id]),
            },
            after: vec![],
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
            essential: false,
        },
    ];
    let mut s = Scheduler::new(seal(jobs.clone()).unwrap());
    let all = Resources {
        on_battery: false,
        battery_percent: None,
        memory_free_bytes: Some(u64::MAX),
    };
    let grants = s.next(&all);
    assert_eq!(grants.len(), 2);
    let job_items = BTreeMap::from([
        (JobId("data".into()), BTreeSet::from([a.id, b.id])),
        (JobId("install".into()), BTreeSet::from([ed.id])),
    ]);
    let converted = Some(ItemResult::done(ItemStatus::Converted).unwrap());
    let reinstalled = Some(ItemResult::done(ItemStatus::Reinstalled).unwrap());
    // Not finished yet: cannot be confirmed.
    {
        let probe = SchedulerProbe::new(&s, &jobs, &job_items);
        let r = receipt(&l, Role::Data, a.id, converted);
        assert_eq!(l.accept(r, &probe, T0 + 20), Err(Refused::NotWhatIsThere));
    }
    s.report(&s.result_for(&JobId("data".into()), true))
        .unwrap();
    s.report(&s.result_for(&JobId("install".into()), false))
        .unwrap();
    let probe = SchedulerProbe::new(&s, &jobs, &job_items);
    l.accept(receipt(&l, Role::Data, a.id, converted), &probe, T0 + 20)
        .unwrap();
    // b is named for the job here, but the job's own scope does not hold it.
    assert_eq!(
        l.accept(receipt(&l, Role::Data, b.id, converted), &probe, T0 + 20),
        Err(Refused::NotWhatIsThere)
    );
    // The install job failed: its "reinstalled" is refused; the wrong worker's claim too.
    assert_eq!(
        l.accept(
            receipt(&l, Role::Install, ed.id, reinstalled),
            &probe,
            T0 + 20
        ),
        Err(Refused::NotWhatIsThere)
    );
    assert_eq!(
        l.accept(receipt(&l, Role::Transfer, a.id, copied()), &probe, T0 + 20),
        Err(Refused::CouldNotConfirm)
    );
    let _ = ItemKind::App;
}

/// The ledger is saved and read back whole.
#[test]
fn the_ledger_round_trips() {
    let a = file("a.txt");
    let (_, mut l) = ledger(vec![a.clone()]);
    l.accept(
        receipt(&l, Role::Transfer, a.id, copied()),
        &Sees(Observed::Matches),
        T0 + 20,
    )
    .unwrap();
    let json = serde_json::to_string(&l).unwrap();
    let back: pctwin_readiness::ledger::Ledger = serde_json::from_str(&json).unwrap();
    assert_eq!(back, l);
}
