//! The ledger's rules, cell by cell: which part of PCTwin may give which result word and which
//! evidence for which kind of item, what evidence is well formed, and the bounds.

mod common;

use common::*;
use pctwin_readiness::ledger::{
    Coverage, DeferWhy, Evidence, InaccessibleWhy, LedgerError, MAX_EVIDENCE, MAX_HISTORY,
    MAX_REFUSED, Observed, Refused, Role, Standing,
};
use pctwin_record::{Inclusion, ItemKind, LeftOutReason};
use pctwin_types::{ItemResult, ItemStatus, Reason};

const ROLES: [Role; 7] = [
    Role::Transfer,
    Role::Install,
    Role::Settings,
    Role::Data,
    Role::Person,
    Role::Readiness,
    Role::Broker,
];
const KINDS: [ItemKind; 5] = [
    ItemKind::File,
    ItemKind::Folder,
    ItemKind::Package,
    ItemKind::App,
    ItemKind::Setting,
];

fn file_like(k: ItemKind) -> bool {
    matches!(k, ItemKind::File | ItemKind::Folder | ItemKind::Package)
}

/// The policy, written out as a table.
fn may_say(role: Role, kind: ItemKind, s: ItemStatus) -> bool {
    use ItemStatus::*;
    let words: &[ItemStatus] = match (role, kind) {
        (Role::Transfer, k) if file_like(k) => &[Copied, NeedsAction, Failed],
        (Role::Data, k) if file_like(k) => &[Copied, Converted, NeedsAction, Failed],
        (Role::Install, ItemKind::App) => &[Reinstalled, Converted, NeedsAction, Failed],
        (Role::Settings, ItemKind::Setting) => &[Converted, Copied, NeedsAction, Failed],
        _ => &[],
    };
    words.contains(&s)
}

fn result_for(s: ItemStatus) -> ItemResult {
    match s {
        ItemStatus::NeedsAction => ItemResult::because(Reason::SignIn),
        ItemStatus::Failed => ItemResult::because(Reason::Unreadable),
        ItemStatus::SkippedByUser => ItemResult::skipped_by_user(),
        done => ItemResult::done(done).unwrap(),
    }
}

#[test]
fn who_may_give_which_result_word() {
    for role in ROLES {
        for kind in KINDS {
            for s in ItemStatus::ALL {
                let it = item(MOVER, "x", kind, Inclusion::Included);
                let (_, mut l) = ledger(vec![it.clone()]);
                let r = receipt(&l, role, it.id, Some(result_for(s)));
                let got = l.accept(r, &Sees(Observed::Matches), T0 + 20);
                if may_say(role, kind, s) {
                    assert_eq!(got, Ok(()), "{role:?} {kind:?} {s:?}");
                } else {
                    assert_eq!(got, Err(Refused::WrongRole), "{role:?} {kind:?} {s:?}");
                }
            }
        }
    }
}

fn samples() -> Vec<(Evidence, &'static [Role], bool)> {
    let w = Coverage::Whole;
    vec![
        (
            Evidence::Captured { generation: 1 },
            &[Role::Transfer, Role::Data],
            true,
        ),
        (
            Evidence::VerifiedTransfer {
                fingerprint: [1; 32],
            },
            &[Role::Transfer],
            true,
        ),
        (
            Evidence::Installed {
                package: "p".into(),
                version: "1".into(),
            },
            &[Role::Install],
            false,
        ),
        (
            Evidence::Imported {
                adapter: "a".into(),
                coverage: w,
            },
            &[Role::Settings, Role::Data],
            false,
        ),
        (
            Evidence::PersonConfirmed {
                task: "t".into(),
                coverage: w,
            },
            &[Role::Person],
            false,
        ),
        (
            Evidence::RecoveryChecked {
                what: "k".into(),
                coverage: w,
            },
            &[Role::Data, Role::Readiness],
            false,
        ),
        (
            Evidence::SourceOnly {
                places: vec!["Documents".into()],
            },
            &[Role::Readiness],
            false,
        ),
        (
            Evidence::CleanupReceipt {
                operation: "o".into(),
            },
            &[Role::Broker],
            false,
        ),
    ]
}

#[test]
fn who_may_give_which_evidence() {
    for (ev, roles, files_only) in samples() {
        for role in ROLES {
            for kind in KINDS {
                let installed = matches!(ev, Evidence::Installed { .. });
                let fits = if installed {
                    kind == ItemKind::App
                } else if files_only {
                    file_like(kind)
                } else {
                    true
                };
                let want = roles.contains(&role) && fits;
                let it = item(MOVER, "x", kind, Inclusion::Included);
                let (_, mut l) = ledger(vec![it.clone()]);
                let mut r = receipt(&l, role, it.id, None);
                r.evidence = Some(ev.clone());
                let got = l.accept(r, &Sees(Observed::Matches), T0 + 20);
                assert_eq!(
                    got,
                    if want {
                        Ok(())
                    } else {
                        Err(Refused::WrongRole)
                    },
                    "{ev:?} {role:?} {kind:?}"
                );
            }
        }
    }
}

#[test]
fn badly_formed_evidence_and_empty_receipts_are_refused() {
    let a = file("a.txt");
    let ap = app("Editor");
    let bad = [
        (
            Role::Install,
            ap.id,
            Evidence::Installed {
                package: " ".into(),
                version: "1".into(),
            },
        ),
        (
            Role::Install,
            ap.id,
            Evidence::Installed {
                package: "p".into(),
                version: "".into(),
            },
        ),
        (
            Role::Data,
            a.id,
            Evidence::Imported {
                adapter: "".into(),
                coverage: Coverage::Whole,
            },
        ),
        (
            Role::Data,
            a.id,
            Evidence::Imported {
                adapter: "a".into(),
                coverage: Coverage::Sample { checked: 0, of: 5 },
            },
        ),
        (
            Role::Person,
            a.id,
            Evidence::PersonConfirmed {
                task: "t".into(),
                coverage: Coverage::Sample { checked: 5, of: 5 },
            },
        ),
        (
            Role::Person,
            a.id,
            Evidence::PersonConfirmed {
                task: "".into(),
                coverage: Coverage::Whole,
            },
        ),
        (
            Role::Data,
            a.id,
            Evidence::RecoveryChecked {
                what: "".into(),
                coverage: Coverage::Whole,
            },
        ),
        (
            Role::Data,
            a.id,
            Evidence::RecoveryChecked {
                what: "k".into(),
                coverage: Coverage::Sample { checked: 6, of: 5 },
            },
        ),
        (
            Role::Readiness,
            a.id,
            Evidence::SourceOnly { places: vec![] },
        ),
        (
            Role::Broker,
            a.id,
            Evidence::CleanupReceipt {
                operation: "".into(),
            },
        ),
    ];
    for (role, id, ev) in bad {
        let (_, mut l) = ledger(vec![a.clone(), ap.clone()]);
        let mut r = receipt(&l, role, id, None);
        r.evidence = Some(ev.clone());
        assert_eq!(
            l.accept(r, &Sees(Observed::Matches), T0 + 20),
            Err(Refused::BadEvidence),
            "{ev:?}"
        );
    }
    // A sample that is a real part passes.
    let (_, mut l) = ledger(vec![a.clone()]);
    let mut r = receipt(&l, Role::Person, a.id, None);
    r.evidence = Some(Evidence::PersonConfirmed {
        task: "t".into(),
        coverage: Coverage::Sample { checked: 4, of: 5 },
    });
    l.accept(r, &Sees(Observed::Matches), T0 + 20).unwrap();
    // A receipt saying nothing.
    let r = receipt(&l, Role::Transfer, a.id, None);
    assert_eq!(
        l.accept(r, &Sees(Observed::Matches), T0 + 20),
        Err(Refused::Empty)
    );
    // About an item not selected.
    let s = left_out("s.txt", LeftOutReason::ChosenByUser);
    let (_, mut l) = ledger(vec![s.clone()]);
    let r = receipt(
        &l,
        Role::Transfer,
        s.id,
        Some(ItemResult::because(Reason::Unreadable)),
    );
    assert_eq!(
        l.accept(r, &Sees(Observed::Matches), T0 + 20),
        Err(Refused::NotSelected)
    );
    // At the edges of the allowed time.
    let (_, mut l) = ledger(vec![a.clone()]);
    let mut r = receipt(
        &l,
        Role::Transfer,
        a.id,
        Some(ItemResult::because(Reason::Unreadable)),
    );
    r.observed_at = T0;
    l.accept(r.clone(), &Sees(Observed::Matches), T0 + 20)
        .unwrap();
    r.observed_at = T0 + 20;
    l.accept(r, &Sees(Observed::Matches), T0 + 20).unwrap();
}

#[test]
fn bounds_keep_the_ledger_small() {
    let a = file("a.txt");
    let (_, mut l) = ledger(vec![a.clone()]);
    for n in 0..MAX_EVIDENCE {
        let mut r = receipt(&l, Role::Person, a.id, None);
        r.evidence = Some(Evidence::PersonConfirmed {
            task: format!("t{n}"),
            coverage: Coverage::Whole,
        });
        l.accept(r, &Sees(Observed::Matches), T0 + 20).unwrap();
    }
    let mut r = receipt(&l, Role::Person, a.id, None);
    r.evidence = Some(Evidence::PersonConfirmed {
        task: "one more".into(),
        coverage: Coverage::Whole,
    });
    assert_eq!(
        l.accept(r, &Sees(Observed::Matches), T0 + 20),
        Err(Refused::TooMuch)
    );
    // A result word is still taken when the evidence list is full.
    let r = receipt(
        &l,
        Role::Transfer,
        a.id,
        Some(ItemResult::because(Reason::Unreadable)),
    );
    l.accept(r, &Sees(Observed::Matches), T0 + 20).unwrap();

    for n in 0..MAX_HISTORY + 3 {
        let mut r = receipt(
            &l,
            Role::Transfer,
            a.id,
            Some(ItemResult::because(Reason::Unreadable)),
        );
        r.observed_at = T0 + 1 + (n as u64 % 10);
        l.accept(r, &Sees(Observed::Matches), T0 + 20).unwrap();
    }
    assert_eq!(l.line(a.id).unwrap().history.len(), MAX_HISTORY);

    let stranger = file("stranger").id;
    for n in 0..MAX_REFUSED + 1 {
        let mut r = receipt(&l, Role::Transfer, stranger, None);
        r.session = n as u64 + 100;
        assert!(l.accept(r, &Sees(Observed::Matches), T0 + 20).is_err());
    }
    assert_eq!(l.refused().len(), MAX_REFUSED);
    assert!(l.refused().iter().all(|r| r.why == Refused::OldSession));
}

#[test]
fn holds_keys_and_generations() {
    let (a, b) = (file("a.txt"), file("b.txt"));
    let s = left_out("s.txt", LeftOutReason::ChosenByUser);
    let (rec, mut l) = ledger(vec![a.clone(), b.clone(), s.clone()]);
    let stranger = file("stranger").id;
    l.accept(
        receipt(
            &l,
            Role::Transfer,
            a.id,
            Some(ItemResult::done(ItemStatus::Copied).unwrap()),
        ),
        &Sees(Observed::Matches),
        T0 + 20,
    )
    .unwrap();
    assert_eq!(
        l.defer(a.id, DeferWhy::WaitingForPerson),
        Err(LedgerError::AlreadyDone)
    );
    assert_eq!(
        l.inaccessible(a.id, InaccessibleWhy::Locked),
        Err(LedgerError::AlreadyDone)
    );
    assert_eq!(
        l.defer(stranger, DeferWhy::WaitingForPerson),
        Err(LedgerError::UnknownItem)
    );
    assert_eq!(
        l.inaccessible(s.id, InaccessibleWhy::Locked),
        Err(LedgerError::NotSelected)
    );
    assert_eq!(l.needs_key(stranger), Err(LedgerError::UnknownItem));
    // A failed item can be held, and a new result ends the hold.
    l.accept(
        receipt(
            &l,
            Role::Transfer,
            b.id,
            Some(ItemResult::because(Reason::Unreadable)),
        ),
        &Sees(Observed::Matches),
        T0 + 20,
    )
    .unwrap();
    l.inaccessible(b.id, InaccessibleWhy::DriveNotConnected)
        .unwrap();
    assert_eq!(l.standing(b.id), Some(Standing::Inaccessible));
    l.accept(
        receipt(
            &l,
            Role::Transfer,
            b.id,
            Some(ItemResult::because(Reason::SignIn)),
        ),
        &Sees(Observed::Matches),
        T0 + 20,
    )
    .unwrap();
    assert_eq!(l.standing(b.id), Some(Standing::NeedsAction));
    assert_eq!(l.standing(stranger), None);
    assert_eq!(l.access(stranger), None);
    assert_eq!(
        l.access(b.id),
        Some(pctwin_readiness::ledger::Access::NotOnNewLaptop)
    );
    assert_eq!(
        l.access(a.id),
        Some(pctwin_readiness::ledger::Access::Usable)
    );
    assert!(l.loss_warning(stranger).is_none());
    assert_eq!(
        l.accept_loss(stranger, [0; 32], T0),
        Err(LedgerError::UnknownItem)
    );
    assert!(!l.settled(stranger));
    // A loss accepted, then the item moves after all: no longer a loss.
    let d = l.loss_warning(b.id).unwrap();
    l.accept_loss(b.id, d, T0 + 21).unwrap();
    assert_eq!(l.tally().accepted_loss, 1);
    l.accept(
        receipt(
            &l,
            Role::Transfer,
            b.id,
            Some(ItemResult::done(ItemStatus::Copied).unwrap()),
        ),
        &Sees(Observed::Matches),
        T0 + 20,
    )
    .unwrap();
    assert_eq!(l.tally().accepted_loss, 0);
    // The warning names the item and its plan: another plan's warning differs.
    let d1 = l.loss_warning(s.id).unwrap();
    // Moving to the next plan: same laptop, a later revision only.
    let mut next = rec.next_revision();
    next.items.push(file("c.txt"));
    let mut other = next.clone();
    other.source_laptop = other_laptop();
    assert_eq!(
        l.next_generation(&other, MOVER),
        Err(LedgerError::NotTheNextPlan)
    );
    assert_eq!(
        l.next_generation(&rec, MOVER),
        Err(LedgerError::NotTheNextPlan)
    );
    l.next_generation(&next, MOVER).unwrap();
    assert_eq!(l.generation(), 2);
    assert_eq!(l.standing(file("c.txt").id), Some(Standing::NotTested));
    assert_eq!(l.standing(a.id), Some(Standing::Done(ItemStatus::Copied)));
    let d2 = l.loss_warning(s.id).unwrap();
    assert_ne!(d1, d2);
    assert_eq!(
        l.accept_loss(s.id, d1, T0 + 30),
        Err(LedgerError::NotTheWarningShown)
    );
}

#[test]
fn what_is_only_on_the_old_laptop_and_the_words_for_it() {
    let items = vec![
        file("failed.txt"),
        file("cloud.docx"),
        file("signin.txt"),
        file("parts.pkg"),
        left_out("skip.txt", LeftOutReason::ChosenByUser),
        left_out("work.txt", LeftOutReason::WorkManaged),
        left_out("unreadable.txt", LeftOutReason::Unreadable),
        left_out("big", LeftOutReason::TooManyFiles),
        left_out("sys", LeftOutReason::System),
        left_out("node_modules", LeftOutReason::Rebuildable),
        left_out("online", LeftOutReason::CloudOnly),
        left_out("passkey", LeftOutReason::DeviceBound),
        item("1001", "bea.txt", ItemKind::File, Inclusion::Included),
        item(
            "1001",
            "bea-skip.txt",
            ItemKind::File,
            Inclusion::LeftOut {
                reason: LeftOutReason::ChosenByUser,
            },
        ),
    ];
    let ids: Vec<_> = items.iter().map(|i| i.id).collect();
    let (_, mut l) = ledger(items);
    assert_eq!(l.only_on_old_laptop().len(), 8);
    assert!(l.search().searched.is_empty());
    assert!(!l.found_everything());
    let give = |l: &mut pctwin_readiness::ledger::Ledger, id, r| {
        l.accept(
            receipt(l, Role::Transfer, id, Some(r)),
            &Sees(Observed::Matches),
            T0 + 20,
        )
        .unwrap();
    };
    give(&mut l, ids[0], ItemResult::because(Reason::Unreadable));
    give(&mut l, ids[1], ItemResult::because(Reason::OnlineOnly));
    give(&mut l, ids[2], ItemResult::because(Reason::SignIn));
    give(&mut l, ids[3], ItemResult::because(Reason::PartsMissing));
    let only = l.only_on_old_laptop();
    assert_eq!(
        only.iter()
            .copied()
            .collect::<std::collections::BTreeSet<_>>(),
        [ids[0], ids[3], ids[4], ids[5], ids[6], ids[7]]
            .into_iter()
            .collect()
    );
    l.searched("Documents");
    l.searched("Desktop");
    assert!(l.found_everything());
    l.not_searched("Desktop");
    assert_eq!(
        l.search().searched.iter().collect::<Vec<_>>(),
        ["Documents"]
    );
    assert_eq!(
        l.search().not_searched.iter().collect::<Vec<_>>(),
        ["Desktop"]
    );
    assert!(!l.found_everything());
}

/// A shared item is the mover's to see; the needs-action count is its own.
#[test]
fn shared_items_count_as_the_mover_s_and_every_count_is_its_own() {
    let mut shared = file("shared.txt");
    shared.owner = pctwin_record::Owner::Shared;
    let mut system = file("system.txt");
    system.owner = pctwin_record::Owner::System;
    let n = file("n.txt");
    let (_, mut l) = ledger(vec![shared.clone(), system.clone(), n.clone()]);
    assert!(l.line(shared.id).unwrap().mine);
    assert!(l.line(system.id).unwrap().mine);
    assert!(!l.line(n.id).unwrap().needs_key);
    assert_eq!(l.only_on_old_laptop().len(), 3);
    l.accept(
        receipt(
            &l,
            Role::Transfer,
            n.id,
            Some(ItemResult::because(Reason::PartsMissing)),
        ),
        &Sees(Observed::Matches),
        T0 + 20,
    )
    .unwrap();
    let t = l.tally();
    assert_eq!((t.needs_action, t.not_tested, t.accepted_loss), (1, 2, 0));
    // Parts missing: still only on the old laptop.
    assert!(l.only_on_old_laptop().contains(&n.id));
}

/// A finished app job confirms its install; the scheduler's own record decides.
#[test]
fn a_finished_app_job_confirms_its_install() {
    use pctwin_orchestrate::jobs::{ChildJob, JobId, Resources, Scheduler, Scope, Worker, seal};
    use pctwin_readiness::ledger::SchedulerProbe;
    use std::collections::{BTreeMap, BTreeSet};
    let ed = app("Editor");
    let (_, mut l) = ledger(vec![ed.clone()]);
    let jobs = vec![ChildJob {
        id: JobId("install".into()),
        worker: Worker::Install,
        source: old_laptop(),
        session: 7,
        scope: Scope::App {
            name: "Editor".into(),
        },
        after: vec![],
        essential: false,
    }];
    let mut s = Scheduler::new(seal(jobs.clone()).unwrap());
    s.next(&Resources {
        on_battery: false,
        battery_percent: None,
        memory_free_bytes: Some(u64::MAX),
    });
    s.report(&s.result_for(&JobId("install".into()), true))
        .unwrap();
    let job_items = BTreeMap::from([(JobId("install".into()), BTreeSet::from([ed.id]))]);
    let probe = SchedulerProbe::new(&s, &jobs, &job_items);
    let r = receipt(
        &l,
        Role::Install,
        ed.id,
        Some(ItemResult::done(ItemStatus::Reinstalled).unwrap()),
    );
    l.accept(r, &probe, T0 + 20).unwrap();
    // A job changed since it ran (another session in the list given) is not the job that ran.
    let mut changed = jobs.clone();
    changed[0].session = 8;
    let probe = SchedulerProbe::new(&s, &changed, &job_items);
    let mut r = receipt(
        &l,
        Role::Install,
        ed.id,
        Some(ItemResult::done(ItemStatus::Reinstalled).unwrap()),
    );
    r.session = 7;
    assert_eq!(l.accept(r, &probe, T0 + 20), Err(Refused::NotWhatIsThere));
}

/// Done on the new laptop: never on the "only on the old laptop" list.
#[test]
fn a_done_item_is_not_only_on_the_old_laptop() {
    let a = file("a.txt");
    let (_, mut l) = ledger(vec![a.clone()]);
    assert_eq!(l.only_on_old_laptop(), [a.id]);
    l.accept(
        receipt(
            &l,
            Role::Transfer,
            a.id,
            Some(ItemResult::done(ItemStatus::Copied).unwrap()),
        ),
        &Sees(Observed::Matches),
        T0 + 20,
    )
    .unwrap();
    assert!(l.only_on_old_laptop().is_empty());
}
