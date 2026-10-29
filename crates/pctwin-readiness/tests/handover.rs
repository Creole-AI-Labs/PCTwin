//! Handover gates (Task List 2.7; plan 2.7 sections 7 and 8; acceptance T27-016 to T27-024,
//! T27-026). Every refusal path is tried with everything else in order.

mod common;

use common::*;
use pctwin_journal::keystore::TagKey;
use pctwin_journal::{Journal, UndoGate};
use pctwin_readiness::goodbye::{Checklist, Inputs, OldLaptop};
use pctwin_readiness::handover::{
    Account, Action, ActionResult, DeviceIdentity, DriveIdentity, Facts, Method, Note, PersonSays,
    QualifiedMethod, Refusal, Used, WriteExclusion, confirm, preview, request,
};
use pctwin_readiness::ledger::{Observed, Role};
use pctwin_record::{LeftOutReason, Record};
use pctwin_types::{ItemResult, ItemStatus};

fn journal(dir: &std::path::Path, name: &str, key: u8) -> Journal {
    Journal::open_with_record_key(&dir.join(name), TagKey::from_bytes([key; 32])).unwrap()
}

fn ssd() -> DriveIdentity {
    DriveIdentity {
        serial: "S3Z9NB0K123456".into(),
        model: "Example SSD 512GB".into(),
        bytes: 512_110_190_592,
    }
}

fn device(drive: Option<DriveIdentity>) -> DeviceIdentity {
    DeviceIdentity {
        laptop: old_laptop(),
        drive,
    }
}

struct Setup {
    record: Record,
    checklist: Checklist,
}

/// A finished move, its list clear.
fn setup() -> Setup {
    let (record, mut l) = ledger(vec![file("a.txt")]);
    let a = record.items[0].id;
    l.accept(
        receipt(
            &l,
            Role::Transfer,
            a,
            Some(ItemResult::done(ItemStatus::Copied).unwrap()),
        ),
        &Sees(Observed::Matches),
        T0 + 20,
    )
    .unwrap();
    l.searched("Documents");
    let checklist = Checklist::build(
        &Inputs {
            ledger: &l,
            approved: &record,
            old_laptop: OldLaptop::Scanned(&record),
            left_on_old_laptop: &[],
            steps_not_finished: &[],
            undo_open: true,
            special: &[],
        },
        None,
        T0 + 100,
    );
    assert!(checklist.is_clear());
    Setup { record, checklist }
}

fn accounts() -> Vec<Account> {
    vec![
        Account {
            id: MOVER.into(),
            has_own_files: true,
        },
        Account {
            id: "1001".into(),
            has_own_files: false,
        },
    ]
}

struct F<'a> {
    s: &'a Setup,
    j: &'a Journal,
    expected: DeviceIdentity,
    now_dev: DeviceIdentity,
    old_now: Option<&'a Record>,
    excl: WriteExclusion,
    acc: Vec<Account>,
    q: Vec<QualifiedMethod>,
    now: u64,
}

impl<'a> F<'a> {
    fn new(s: &'a Setup, j: &'a Journal) -> Self {
        Self {
            s,
            j,
            expected: device(None),
            now_dev: device(None),
            old_now: Some(&s.record),
            excl: WriteExclusion::Held,
            acc: accounts(),
            q: vec![],
            now: T0 + 200,
        }
    }

    fn facts(&self) -> Facts<'_> {
        Facts {
            checklist: &self.s.checklist,
            old_laptop_now: self.old_now,
            journal: self.j,
            expected: &self.expected,
            device_now: &self.now_dev,
            write_exclusion: self.excl,
            accounts: &self.acc,
            mover: MOVER,
            qualified: &self.q,
            now: self.now,
        }
    }
}

fn nonce(n: u8) -> [u8; 16] {
    [n; 16]
}

#[test]
fn every_gate_check_refuses_on_its_own() {
    let dir = tempfile::tempdir().unwrap();
    let j = journal(dir.path(), "j1", 1);
    let s = setup();
    for action in [Action::RemoveMoversAccount, Action::EraseOldLaptop] {
        // Everything in order: offered, by the system's own route.
        let f = F::new(&s, &j);
        let p = preview(action, &f.facts()).unwrap();
        assert_eq!(p.method, Method::SystemRoute);

        // T27-019: another laptop answering now.
        let mut f = F::new(&s, &j);
        f.now_dev.laptop = other_laptop();
        assert_eq!(preview(action, &f.facts()), Err(Refusal::WrongDevice));

        // T27-019: the drive named at the move is not the drive there now (same model, another
        // serial), or no drive is named now.
        let mut f = F::new(&s, &j);
        f.expected = device(Some(ssd()));
        let mut other = ssd();
        other.serial = "S3Z9NB0K999999".into();
        f.now_dev = device(Some(other));
        assert_eq!(preview(action, &f.facts()), Err(Refusal::WrongDevice));
        f.now_dev = device(None);
        assert_eq!(preview(action, &f.facts()), Err(Refusal::WrongDevice));
        // A blank serial recorded is never an identity.
        let mut blank = ssd();
        blank.serial = " ".into();
        f.expected = device(Some(blank.clone()));
        f.now_dev = device(Some(blank));
        assert_eq!(preview(action, &f.facts()), Err(Refusal::WrongDevice));

        // T27-014: the old laptop was not checked again.
        let mut f = F::new(&s, &j);
        f.old_now = None;
        assert_eq!(
            preview(action, &f.facts()),
            Err(Refusal::OldLaptopNotReachable)
        );

        // A scan of another laptop is not this one.
        let stranger = Record::new(other_laptop());
        let mut f = F::new(&s, &j);
        f.old_now = Some(&stranger);
        assert_eq!(preview(action, &f.facts()), Err(Refusal::WrongDevice));

        // T27-012: the old laptop changed since the list was made.
        let mut changed = s.record.clone();
        changed.items.push(file("new.txt"));
        let mut f = F::new(&s, &j);
        f.old_now = Some(&changed);
        assert_eq!(preview(action, &f.facts()), Err(Refusal::ChecklistStale));
    }
}

/// An open list refuses with how many are open.
#[test]
fn an_open_list_refuses_both_actions() {
    let dir = tempfile::tempdir().unwrap();
    let j = journal(dir.path(), "j", 1);
    let (record, l) = ledger(vec![file("a.txt"), file("b.txt")]);
    let checklist = Checklist::build(
        &Inputs {
            ledger: &l,
            approved: &record,
            old_laptop: OldLaptop::Scanned(&record),
            left_on_old_laptop: &[],
            steps_not_finished: &[],
            undo_open: true,
            special: &[],
        },
        None,
        T0 + 100,
    );
    let s = Setup { record, checklist };
    let f = F::new(&s, &j);
    for action in [Action::RemoveMoversAccount, Action::EraseOldLaptop] {
        assert_eq!(
            preview(action, &f.facts()),
            Err(Refusal::ExceptionsOpen { open: 1 })
        );
    }
}

/// T27-016: handing on to family removes only the mover's account; erasing is refused while other
/// people keep their files there; the preview says plainly an SSD delete is not a secure wipe.
#[test]
fn t27_016_other_people_s_accounts_are_never_removed() {
    let dir = tempfile::tempdir().unwrap();
    let j = journal(dir.path(), "j", 1);
    let s = setup();
    let f = F::new(&s, &j);
    let p = preview(Action::RemoveMoversAccount, &f.facts()).unwrap();
    assert_eq!(p.removes, [MOVER]);
    assert_eq!(p.keeps, ["1001"]);
    for n in [
        Note::SignInAsAnotherAdministrator,
        Note::OthersStay,
        Note::DoesNotCancelSubscriptions,
        Note::SsdNotSecureWipe,
        Note::ClosesUndo,
        Note::NotACertificate,
    ] {
        assert!(p.notes.contains(&n), "{n:?}");
    }
    // The mover's account is not on the laptop: nothing to remove.
    let mut f = F::new(&s, &j);
    f.acc.retain(|a| a.id != MOVER);
    assert_eq!(
        preview(Action::RemoveMoversAccount, &f.facts()),
        Err(Refusal::NoSuchAccount)
    );
    // Another person keeps files there: no erase.
    let mut f = F::new(&s, &j);
    f.acc[1].has_own_files = true;
    assert_eq!(
        preview(Action::EraseOldLaptop, &f.facts()),
        Err(Refusal::OtherPeopleUseIt)
    );
    // ... but their account still is not removed with the mover's.
    let p = preview(Action::RemoveMoversAccount, &f.facts()).unwrap();
    assert_eq!(p.removes, [MOVER]);
    let p = preview(Action::EraseOldLaptop, &F::new(&s, &j).facts()).unwrap();
    for n in [
        Note::CannotBeUndone,
        Note::SsdNotSecureWipe,
        Note::EncryptFirstHelps,
    ] {
        assert!(p.notes.contains(&n), "{n:?}");
    }
}

/// T27-017, T27-018: a finished move is never the confirmation; only a fresh, separate
/// confirmation of exactly this preview and action, used once; and only then is undo closed, for
/// good, on disk.
#[test]
fn t27_017_018_only_a_fresh_separate_confirmation_closes_undo() {
    let dir = tempfile::tempdir().unwrap();
    let j = journal(dir.path(), "j", 1);
    let s = setup();
    {
        let f = F::new(&s, &j);
        let facts = f.facts();
        let p = preview(Action::EraseOldLaptop, &facts).unwrap();
        let mut used = Used::default();
        let open = |j: &Journal| j.undo_gate().unwrap() == UndoGate::Open;

        assert_eq!(
            request(Action::EraseOldLaptop, &facts, None, &mut used).unwrap_err(),
            Refusal::NoFreshConfirmation
        );
        let other = confirm(Action::RemoveMoversAccount, p.digest(), T0 + 150, nonce(1));
        assert_eq!(
            request(Action::EraseOldLaptop, &facts, Some(&other), &mut used).unwrap_err(),
            Refusal::WrongConfirmation
        );
        let stale = confirm(Action::EraseOldLaptop, [9; 32], T0 + 150, nonce(2));
        assert_eq!(
            request(Action::EraseOldLaptop, &facts, Some(&stale), &mut used).unwrap_err(),
            Refusal::ChangedSinceConfirmed
        );
        let old = confirm(Action::EraseOldLaptop, p.digest(), T0 - 1000, nonce(3));
        assert_eq!(
            request(Action::EraseOldLaptop, &facts, Some(&old), &mut used).unwrap_err(),
            Refusal::Expired
        );
        let future = confirm(Action::EraseOldLaptop, p.digest(), T0 + 201, nonce(4));
        assert_eq!(
            request(Action::EraseOldLaptop, &facts, Some(&future), &mut used).unwrap_err(),
            Refusal::Expired
        );
        assert!(open(&j), "no refusal closed undo");
        // Exactly at the edge of fresh is still fresh.
        let edge = confirm(
            Action::EraseOldLaptop,
            p.digest(),
            T0 + 200 - pctwin_readiness::handover::FRESH,
            nonce(5),
        );
        let r = request(Action::EraseOldLaptop, &facts, Some(&edge), &mut used).unwrap();
        assert!(r.undo_closed.is_for(&j));
        assert!(!open(&j), "undo is closed for good, on disk");
        assert_eq!(r.method, Method::SystemRoute);
        // The same confirmation again: undo is closed now, so the preview changed.
        assert_eq!(
            request(Action::EraseOldLaptop, &facts, Some(&edge), &mut used).unwrap_err(),
            Refusal::ChangedSinceConfirmed
        );
        // Its number is used, whatever it is put to.
        let p2 = preview(Action::RemoveMoversAccount, &facts).unwrap();
        let reused = confirm(Action::RemoveMoversAccount, p2.digest(), T0 + 150, nonce(5));
        assert_eq!(
            request(
                Action::RemoveMoversAccount,
                &facts,
                Some(&reused),
                &mut used
            )
            .unwrap_err(),
            Refusal::AlreadyUsed
        );
        let fresh = confirm(Action::RemoveMoversAccount, p2.digest(), T0 + 150, nonce(6));
        assert!(request(Action::RemoveMoversAccount, &facts, Some(&fresh), &mut used).is_ok());
        // With undo now closed, the preview no longer says confirming closes it.
        let again = preview(Action::EraseOldLaptop, &facts).unwrap();
        assert!(!again.notes.contains(&Note::ClosesUndo));
    }
    // The journal stays closed when it is opened again.
    drop(j);
    let j = journal(dir.path(), "j", 1);
    assert_eq!(j.undo_gate().unwrap(), UndoGate::Closed);
}

/// T27-018: a refused request never closes undo, whatever check refused it.
#[test]
fn t27_018_a_refused_request_leaves_undo_open() {
    let dir = tempfile::tempdir().unwrap();
    let j = journal(dir.path(), "j", 1);
    let s = setup();
    let mut f = F::new(&s, &j);
    let p = preview(Action::RemoveMoversAccount, &f.facts()).unwrap();
    let c = confirm(Action::RemoveMoversAccount, p.digest(), T0 + 150, nonce(1));
    f.old_now = None;
    let mut used = Used::default();
    assert_eq!(
        request(Action::RemoveMoversAccount, &f.facts(), Some(&c), &mut used).unwrap_err(),
        Refusal::OldLaptopNotReachable
    );
    assert_eq!(j.undo_gate().unwrap(), UndoGate::Open);
}

/// T27-020, T27-021: an erase PCTwin would run itself needs a method qualified for this exact
/// drive and no other writer; with none qualified, it is the system's own route, never refused
/// for want of one and never a success certificate.
#[test]
fn t27_020_021_qualified_methods_only_for_the_exact_drive() {
    let dir = tempfile::tempdir().unwrap();
    let j = journal(dir.path(), "j", 1);
    let s = setup();
    let mut f = F::new(&s, &j);
    f.expected = device(Some(ssd()));
    f.now_dev = device(Some(ssd()));
    // None qualified: the system's route.
    assert_eq!(
        preview(Action::EraseOldLaptop, &f.facts()).unwrap().method,
        Method::SystemRoute
    );
    // Qualified for another drive only: still the system's route.
    let mut elsewhere = ssd();
    elsewhere.serial = "OTHER".into();
    f.q = vec![QualifiedMethod {
        drive: elsewhere,
        method: "nvme-sanitize".into(),
    }];
    assert_eq!(
        preview(Action::EraseOldLaptop, &f.facts()).unwrap().method,
        Method::SystemRoute
    );
    // Qualified for this exact drive: it would be PCTwin's, and needs every writer kept out.
    f.q = vec![QualifiedMethod {
        drive: ssd(),
        method: "nvme-sanitize".into(),
    }];
    let p = preview(Action::EraseOldLaptop, &f.facts()).unwrap();
    assert_eq!(
        p.method,
        Method::Qualified {
            name: "nvme-sanitize".into()
        }
    );
    assert_eq!(p.removes, ["Example SSD 512GB"]);
    f.excl = WriteExclusion::CannotExclude;
    assert_eq!(
        preview(Action::EraseOldLaptop, &f.facts()),
        Err(Refusal::NoWriteExclusion)
    );
    // The account is the system's own route always: no writer exclusion asked of it.
    assert_eq!(
        preview(Action::RemoveMoversAccount, &f.facts())
            .unwrap()
            .method,
        Method::SystemRoute
    );
}

/// T27-022 to T27-024, T27-026: what is recorded is what the person said happened, never a
/// certificate; part-way and cancelled say so.
#[test]
fn t27_022_to_026_results_are_what_the_person_says() {
    use ActionResult as R;
    assert_eq!(R::from_person(PersonSays::Done), R::PersonSaidDone);
    assert_eq!(R::from_person(PersonSays::Partly), R::Partial);
    assert_eq!(
        R::from_person(PersonSays::CancelledPartWay),
        R::CancelledAfterSomeEffects
    );
    assert_eq!(R::from_person(PersonSays::NotStarted), R::NotStarted);
    assert_eq!(R::from_person(PersonSays::CannotTell), R::Inconclusive);
}

/// A device-bound thing on the list keeps every handover refused until the person says they set
/// it up again.
#[test]
fn a_device_bound_item_blocks_until_the_person_did_it() {
    let dir = tempfile::tempdir().unwrap();
    let j = journal(dir.path(), "j", 1);
    let (record, mut l) = ledger(vec![left_out("passkey", LeftOutReason::DeviceBound)]);
    l.searched("Documents");
    let inputs = Inputs {
        ledger: &l,
        approved: &record,
        old_laptop: OldLaptop::Scanned(&record),
        left_on_old_laptop: &[],
        steps_not_finished: &[],
        undo_open: true,
        special: &[],
    };
    let mut checklist = Checklist::build(&inputs, None, T0 + 100);
    let s = Setup {
        record: record.clone(),
        checklist: checklist.clone(),
    };
    assert_eq!(
        preview(Action::EraseOldLaptop, &F::new(&s, &j).facts()),
        Err(Refusal::ExceptionsOpen { open: 1 })
    );
    let id = checklist.exceptions[0].id;
    let shown = checklist.shown(&id).unwrap();
    checklist.person_did_it(&id, shown, T0 + 120).unwrap();
    let s = Setup { record, checklist };
    assert!(preview(Action::EraseOldLaptop, &F::new(&s, &j).facts()).is_ok());
}
