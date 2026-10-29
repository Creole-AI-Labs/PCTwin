//! Before leaving the old laptop (Task List 2.7; plan 2.7 section 6; acceptance T27-011 to
//! T27-014).

// Fixtures are written with std::fs; only the crate itself is fenced.
#![allow(clippy::disallowed_methods)]

mod common;

use std::collections::BTreeSet;

use common::*;
use pctwin_orchestrate::jobs::{JobId, Missing, Scope, Why};
use pctwin_readiness::goodbye::{
    Checklist, ChecklistError, ExceptionKind, Inputs, OldLaptop, Resolution, SUGGESTED_KEEP,
};
use pctwin_readiness::ledger::{Coverage, Evidence, Observed, Role};
use pctwin_record::{ItemId, LeftOutReason, Portability};
use pctwin_types::{ItemResult, ItemStatus, Reason};

fn kinds(c: &Checklist) -> Vec<ExceptionKind> {
    c.exceptions.iter().map(|e| e.kind).collect()
}

fn of(c: &Checklist, k: ExceptionKind) -> &pctwin_readiness::goodbye::Exception {
    c.exceptions.iter().find(|e| e.kind == k).unwrap()
}

fn give(l: &mut pctwin_readiness::ledger::Ledger, role: Role, item: ItemId, r: ItemResult) {
    l.accept(
        receipt(l, role, item, Some(r)),
        &Sees(Observed::Matches),
        T0 + 20,
    )
    .unwrap();
}

struct World {
    approved: pctwin_record::Record,
    l: pctwin_readiness::ledger::Ledger,
    moved: ItemId,
    failed: ItemId,
    skipped: ItemId,
    system: ItemId,
    vault: ItemId,
    passkey: ItemId,
    licensed: ItemId,
    sign_in: ItemId,
    online: ItemId,
    bea: ItemId,
}

fn world() -> World {
    let moved = file("moved.txt");
    let failed = file("draft.txt");
    let skipped = left_out("old-games", LeftOutReason::ChosenByUser);
    let system = left_out("pagefile", LeftOutReason::System);
    let vault = file("vault.kdbx");
    let passkey = left_out("passkey", LeftOutReason::DeviceBound);
    let mut licensed = app("Photo Studio");
    licensed.portability = Portability::LimitedActivations;
    let sign_in = app("Chat");
    let online = file("cloud.docx");
    let bea = item(
        "1001",
        "bea.txt",
        pctwin_record::ItemKind::File,
        pctwin_record::Inclusion::Included,
    );
    let (approved, mut l) = ledger(vec![
        moved.clone(),
        failed.clone(),
        skipped.clone(),
        system.clone(),
        vault.clone(),
        passkey.clone(),
        licensed.clone(),
        sign_in.clone(),
        online.clone(),
        bea.clone(),
    ]);
    let copied = ItemResult::done(ItemStatus::Copied).unwrap();
    give(&mut l, Role::Transfer, moved.id, copied);
    give(
        &mut l,
        Role::Transfer,
        failed.id,
        ItemResult::because(Reason::Unreadable),
    );
    give(&mut l, Role::Transfer, vault.id, copied);
    give(
        &mut l,
        Role::Transfer,
        online.id,
        ItemResult::because(Reason::OnlineOnly),
    );
    give(
        &mut l,
        Role::Install,
        licensed.id,
        ItemResult::done(ItemStatus::Reinstalled).unwrap(),
    );
    give(
        &mut l,
        Role::Install,
        sign_in.id,
        ItemResult::because(Reason::SignIn),
    );
    l.needs_key(vault.id).unwrap();
    l.searched("Documents");
    World {
        approved,
        l,
        moved: moved.id,
        failed: failed.id,
        skipped: skipped.id,
        system: system.id,
        vault: vault.id,
        passkey: passkey.id,
        licensed: licensed.id,
        sign_in: sign_in.id,
        online: online.id,
        bea: bea.id,
    }
}

fn build(
    w: &World,
    scan: OldLaptop<'_>,
    left: &[ItemId],
    steps: &[Missing],
    prev: Option<&Checklist>,
) -> Checklist {
    Checklist::build(
        &Inputs {
            ledger: &w.l,
            approved: &w.approved,
            old_laptop: scan,
            left_on_old_laptop: left,
            steps_not_finished: steps,
            undo_open: true,
            special: &[],
        },
        prev,
        T0 + 100,
    )
}

/// T27-011: the list names what is only on the old laptop, each kind apart, and each is resolved
/// by evidence or accepted as a specific loss.
#[test]
fn t27_011_the_list_names_each_exception_and_resolves_or_accepts_it() {
    let mut w = world();
    let left = file("big-video.mov").id;
    let steps = vec![Missing {
        job: JobId("settings".into()),
        scope: Scope::App {
            name: "Chat".into(),
        },
        why: Why::Failed,
    }];
    let mut c = build(&w, OldLaptop::Scanned(&w.approved), &[left], &steps, None);
    assert_eq!(
        kinds(&c),
        [
            ExceptionKind::OnlyOnOldLaptop,
            ExceptionKind::KeyNotChecked,
            ExceptionKind::TiedToOldLaptop,
            ExceptionKind::DoFirstOnOldLaptop,
            ExceptionKind::StepsNotFinished,
        ]
    );
    let only = of(&c, ExceptionKind::OnlyOnOldLaptop);
    let only_items: BTreeSet<ItemId> = only.items.iter().copied().collect();
    // The failed file, the one the person skipped and the one left to fit; not what moved, not a
    // system file, not the online-only file (it is in its cloud), never another person's.
    assert_eq!(only_items, BTreeSet::from([w.failed, w.skipped, left]));
    for not in [w.moved, w.system, w.online, w.bea] {
        assert!(!only_items.contains(&not));
    }
    assert_eq!(of(&c, ExceptionKind::KeyNotChecked).items, [w.vault]);
    assert_eq!(of(&c, ExceptionKind::TiedToOldLaptop).items, [w.passkey]);
    let first: BTreeSet<ItemId> = of(&c, ExceptionKind::DoFirstOnOldLaptop)
        .items
        .iter()
        .copied()
        .collect();
    assert_eq!(first, BTreeSet::from([w.licensed, w.sign_in]));
    assert_eq!(of(&c, ExceptionKind::StepsNotFinished).steps, ["settings"]);
    assert!(!c.is_clear());
    assert!(c.undo_open);
    assert_eq!(c.suggested_keep_until, T0 + 100 + SUGGESTED_KEEP);

    // Accepting needs the exact warning; there is no "all moved" answer.
    let id = of(&c, ExceptionKind::OnlyOnOldLaptop).id;
    assert_eq!(
        c.accept(&id, [0; 32], T0 + 101),
        Err(ChecklistError::NotTheWarningShown)
    );
    assert_eq!(
        c.accept(&[9; 32], [0; 32], T0 + 101),
        Err(ChecklistError::NoSuchException)
    );
    let shown = c.shown(&id).unwrap();
    c.accept(&id, shown, T0 + 101).unwrap();
    assert_eq!(
        of(&c, ExceptionKind::OnlyOnOldLaptop).resolution,
        Resolution::AcceptedLoss { at: T0 + 101 }
    );
    // A key is not the person's step to tick off: it is checked, or accepted as a loss.
    let key = of(&c, ExceptionKind::KeyNotChecked).id;
    let key_shown = c.shown(&key).unwrap();
    assert_eq!(
        c.person_did_it(&key, key_shown, T0 + 101),
        Err(ChecklistError::NotThePersonsStep)
    );

    // Evidence resolves: the key checked on the new laptop, the steps finished. Built again, they
    // are gone; the decision on the unchanged exception carries over.
    let mut r = receipt(&w.l, Role::Data, w.vault, None);
    r.evidence = Some(Evidence::RecoveryChecked {
        what: "the key file opened the copy".into(),
        coverage: Coverage::Whole,
    });
    w.l.accept(r, &Sees(Observed::Matches), T0 + 50).unwrap();
    let c2 = build(&w, OldLaptop::Scanned(&w.approved), &[left], &[], Some(&c));
    assert_eq!(
        kinds(&c2),
        [
            ExceptionKind::OnlyOnOldLaptop,
            ExceptionKind::TiedToOldLaptop,
            ExceptionKind::DoFirstOnOldLaptop,
        ]
    );
    assert_eq!(
        of(&c2, ExceptionKind::OnlyOnOldLaptop).resolution,
        Resolution::AcceptedLoss { at: T0 + 101 }
    );
    // If what it names changes (one more item), the earlier acceptance does not cover it.
    let c3 = build(&w, OldLaptop::Scanned(&w.approved), &[], &[], Some(&c));
    assert!(of(&c3, ExceptionKind::OnlyOnOldLaptop).is_open());
}

/// T27-027 through the list: an accepted item stays a loss in the ledger, and leaves the list.
#[test]
fn an_item_accepted_in_the_ledger_leaves_the_list() {
    let mut w = world();
    let d = w.l.loss_warning(w.failed).unwrap();
    w.l.accept_loss(w.failed, d, T0 + 30).unwrap();
    let c = build(&w, OldLaptop::Scanned(&w.approved), &[], &[], None);
    assert_eq!(of(&c, ExceptionKind::OnlyOnOldLaptop).items, [w.skipped]);
}

/// T27-012: a list built from an older scan of the old laptop is not current.
#[test]
fn t27_012_a_list_from_an_older_scan_is_stale() {
    let w = world();
    let c = build(&w, OldLaptop::Scanned(&w.approved), &[], &[], None);
    assert!(c.is_current(&w.approved));
    let mut later = w.approved.clone();
    later.items.push(file("written-after.txt"));
    assert!(!c.is_current(&later));
}

/// T27-013: a licence or sign-in step on the old laptop is the person's to do, and their word is
/// recorded as theirs; PCTwin offers no action that does it.
#[test]
fn t27_013_licence_steps_are_the_person_s_own() {
    let w = world();
    let mut c = build(&w, OldLaptop::Scanned(&w.approved), &[], &[], None);
    let first = of(&c, ExceptionKind::DoFirstOnOldLaptop);
    assert!(first.person_can_do());
    let id = first.id;
    let shown = c.shown(&id).unwrap();
    c.person_did_it(&id, shown, T0 + 120).unwrap();
    assert_eq!(
        of(&c, ExceptionKind::DoFirstOnOldLaptop).resolution,
        Resolution::PersonDidIt { at: T0 + 120 }
    );
    // Things tied to the old laptop are also set up again by the person.
    let tied = of(&c, ExceptionKind::TiedToOldLaptop).id;
    let s = c.shown(&tied).unwrap();
    c.person_did_it(&tied, s, T0 + 121).unwrap();
}

/// T27-014: an old laptop that cannot be reached keeps the list uncertain: that cannot be
/// accepted away, and the list is never clear.
#[test]
fn t27_014_an_unreachable_old_laptop_keeps_the_list_unclear() {
    let (approved, mut l) = ledger(vec![file("a.txt")]);
    let a = approved.items[0].id;
    give(
        &mut l,
        Role::Transfer,
        a,
        ItemResult::done(ItemStatus::Copied).unwrap(),
    );
    let inputs = |scan| Inputs {
        ledger: &l,
        approved: &approved,
        old_laptop: scan,
        left_on_old_laptop: &[],
        steps_not_finished: &[],
        undo_open: false,
        special: &[],
    };
    let clear = Checklist::build(&inputs(OldLaptop::Scanned(&approved)), None, T0 + 100);
    assert!(clear.exceptions.is_empty());
    assert!(clear.is_clear());
    let mut c = Checklist::build(&inputs(OldLaptop::NotReachable), None, T0 + 100);
    assert_eq!(kinds(&c), [ExceptionKind::OldLaptopNotReachable]);
    let id = c.exceptions[0].id;
    let shown = c.shown(&id).unwrap();
    assert_eq!(
        c.accept(&id, shown, T0 + 101),
        Err(ChecklistError::CannotBeAccepted)
    );
    assert_eq!(
        c.person_did_it(&id, shown, T0 + 101),
        Err(ChecklistError::NotThePersonsStep)
    );
    assert!(!c.is_clear());
    assert!(!c.is_current(&approved));
}

/// A place not searched is on the list until searched, or accepted.
#[test]
fn an_unsearched_place_is_on_the_list() {
    let mut w = world();
    w.l.not_searched("USB drive \"Backup\"");
    let mut c = build(&w, OldLaptop::Scanned(&w.approved), &[], &[], None);
    let e = of(&c, ExceptionKind::NotSearched);
    assert_eq!(e.places, ["USB drive \"Backup\""]);
    assert!(w.l.search().not_searched.contains("USB drive \"Backup\""));
    let id = e.id;
    let s = c.shown(&id).unwrap();
    c.accept(&id, s, T0 + 101).unwrap();
}

/// The list is saved on the new laptop and read back whole; the keep-until date is chosen.
#[test]
fn the_list_is_saved_and_read_back() {
    let w = world();
    let mut c = build(&w, OldLaptop::Scanned(&w.approved), &[], &[], None);
    assert_eq!(
        c.choose_keep_until(T0, T0 + 100),
        Err(ChecklistError::DateInThePast)
    );
    c.choose_keep_until(T0 + 1_000_000, T0 + 100).unwrap();
    let dir = tempfile::tempdir().unwrap();
    let p = dir.path().join("before-wipe.json");
    c.save(&p).unwrap();
    c.save(&p).unwrap();
    assert_eq!(Checklist::load(&p).unwrap(), c);
    // The keep-until date carries over to a list built again.
    let again = build(&w, OldLaptop::Scanned(&w.approved), &[], &[], Some(&c));
    assert_eq!(again.keep_until, Some(T0 + 1_000_000));
    // Only the saved file is there: no temporary file left behind.
    assert_eq!(std::fs::read_dir(dir.path()).unwrap().count(), 1);
    // A list from a newer PCTwin is refused.
    let text = std::fs::read_to_string(&p)
        .unwrap()
        .replacen("\"format\": 2", "\"format\": 3", 1);
    std::fs::write(&p, text).unwrap();
    assert!(Checklist::load(&p).is_err());
}

/// One thing of a kind is counted as one, and the list carries no words of its own (the screens hold them).
#[test]
fn one_thing_is_counted_as_one() {
    let failed = file("draft.txt");
    let vault = file("vault.kdbx");
    let passkey = left_out("passkey", LeftOutReason::DeviceBound);
    let (approved, mut l) = ledger(vec![failed.clone(), vault.clone(), passkey]);
    give(
        &mut l,
        Role::Transfer,
        failed.id,
        ItemResult::because(Reason::Unreadable),
    );
    give(
        &mut l,
        Role::Transfer,
        vault.id,
        ItemResult::done(ItemStatus::Copied).unwrap(),
    );
    l.needs_key(vault.id).unwrap();
    let c = Checklist::build(
        &Inputs {
            ledger: &l,
            approved: &approved,
            old_laptop: OldLaptop::Scanned(&approved),
            left_on_old_laptop: &[],
            steps_not_finished: &[],
            undo_open: true,
            special: &[],
        },
        None,
        T0 + 100,
    );
    // The words are the screens' (seven languages): the list gives the key and the count.
    assert_eq!(of(&c, ExceptionKind::OnlyOnOldLaptop).items.len(), 1);
    assert_eq!(of(&c, ExceptionKind::KeyNotChecked).items.len(), 1);
    assert_eq!(of(&c, ExceptionKind::TiedToOldLaptop).items.len(), 1);
    assert!(c.exceptions.iter().all(|e| e.what.is_none()));
    let w = world();
    let many = build(&w, OldLaptop::Scanned(&w.approved), &[], &[], None);
    assert_eq!(of(&many, ExceptionKind::OnlyOnOldLaptop).items.len(), 2);
}

/// D21 (2.5): special-category steps appear on the list, each its own exception, resolved like any
/// other (a relink the person does is their word; a step on the old laptop first too).
#[test]
fn special_category_steps_are_on_the_list_each_on_its_own() {
    use pctwin_readiness::goodbye::{SpecialStep, StepWhat};
    let w = world();
    let special = vec![
        SpecialStep {
            kind: ExceptionKind::TiedToOldLaptop,
            place: "Chat app".into(),
            component: "aa".into(),
            what: StepWhat::Relink,
        },
        SpecialStep {
            kind: ExceptionKind::TiedToOldLaptop,
            place: "Chat app".into(),
            component: "bb".into(),
            what: StepWhat::Relink,
        },
        SpecialStep {
            kind: ExceptionKind::DoFirstOnOldLaptop,
            place: "Browser".into(),
            component: "cc".into(),
            what: StepWhat::ExportFirst,
        },
    ];
    let c = Checklist::build(
        &Inputs {
            ledger: &w.l,
            approved: &w.approved,
            old_laptop: OldLaptop::Scanned(&w.approved),
            left_on_old_laptop: &[],
            steps_not_finished: &[],
            undo_open: false,
            special: &special,
        },
        None,
        T0 + 100,
    );
    for s in &special {
        let e = c
            .exceptions
            .iter()
            .find(|e| e.what == Some(s.what) && e.steps == [s.component.clone()])
            .expect("each step is on the list");
        assert_eq!((e.kind, e.places.clone()), (s.kind, vec![s.place.clone()]));
        assert!(e.is_open());
    }
    let ids: BTreeSet<[u8; 32]> = c.exceptions.iter().map(|e| e.id).collect();
    assert_eq!(ids.len(), c.exceptions.len(), "no two steps merge");
    assert!(!c.is_clear());
}
