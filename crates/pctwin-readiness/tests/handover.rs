//! Handover and clean-up gates (Task List 2.7; plan 2.7 sections 7 and 8; acceptance T27-015 to
//! T27-024, T27-026). Every refusal path is tried with everything else in order.

mod common;

use common::*;
use pctwin_journal::keystore::TagKey;
use pctwin_journal::{ClosedToken, Journal, Resolved};
use pctwin_readiness::goodbye::{Checklist, Inputs, OldLaptop};
use pctwin_readiness::handover::{
    Account, Action, ActionResult, DeviceIdentity, Facts, NativeOutcome, Ownership,
    QualifiedMethod, Refusal, SSD_NOTE, TempEntry, Used, WriteExclusion, confirm, preview, request,
};
use pctwin_readiness::ledger::{Observed, Role};
use pctwin_record::Record;
use pctwin_types::{ItemResult, ItemStatus};

fn journal(dir: &std::path::Path, name: &str, key: u8) -> Journal {
    Journal::open_with_record_key(&dir.join(name), TagKey::from_bytes([key; 32])).unwrap()
}

fn close(j: &Journal) -> ClosedToken {
    j.close_undo(&mut |_, _| Ok(Resolved::AlreadyGone)).unwrap()
}

fn drive() -> DeviceIdentity {
    DeviceIdentity {
        laptop: old_laptop(),
        drive_serial: "S3Z9NB0K123456".into(),
        drive_model: "Example SSD 512GB".into(),
        drive_bytes: 512_110_190_592,
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
            undo_open: false,
            special: &[],
        },
        None,
        T0 + 100,
    );
    assert!(checklist.is_clear());
    Setup { record, checklist }
}

fn accounts() -> Vec<Account> {
    vec![Account {
        id: MOVER.into(),
        has_own_files: true,
    }]
}

fn temp() -> Vec<TempEntry> {
    vec![
        TempEntry {
            name: "download-cache".into(),
            ownership: Ownership::RecordedByPctwin,
            item: None,
        },
        TempEntry {
            name: "model.gguf".into(),
            ownership: Ownership::RecordedByPctwin,
            item: Some(file("model.gguf").id),
        },
        TempEntry {
            name: "journal-looking-folder".into(),
            ownership: Ownership::LooksTemporary,
            item: None,
        },
    ]
}

macro_rules! facts {
    ($s:expr, $j:expr, $t:expr, $acc:expr, $tmp:expr, $q:expr, $dev:expr) => {
        Facts {
            checklist: &$s.checklist,
            old_laptop_now: Some(&$s.record),
            journal: $j,
            undo_closed: $t,
            expected: &drive(),
            device_now: $dev,
            write_exclusion: WriteExclusion::Held,
            accounts: $acc,
            mover: MOVER,
            temp: $tmp,
            qualified: $q,
            now: T0 + 200,
        }
    };
}

#[test]
fn every_gate_check_refuses_on_its_own() {
    let dir = tempfile::tempdir().unwrap();
    let j = journal(dir.path(), "j1", 1);
    let other = journal(dir.path(), "j2", 2);
    let other_token = close(&other);
    let s = setup();
    let acc = accounts();
    let tmp = temp();
    let dev = drive();
    let none: Vec<QualifiedMethod> = vec![];

    // Undo still open: no token can be had, so the gate refuses (T27-018).
    let f = facts!(s, &j, None, &acc, &tmp, &none, &dev);
    assert_eq!(
        preview(Action::RemoveTemporaryFiles, &f),
        Err(Refusal::UndoNotClosed)
    );
    // Another journal's close does not close this one.
    let f = facts!(s, &j, Some(&other_token), &acc, &tmp, &none, &dev);
    assert_eq!(
        preview(Action::RemoveTemporaryFiles, &f),
        Err(Refusal::UndoNotClosed)
    );
    let token = close(&j);
    // Closed on disk, but with no token, or with another journal's: still refused.
    let f = facts!(s, &j, None, &acc, &tmp, &none, &dev);
    assert_eq!(
        preview(Action::RemoveTemporaryFiles, &f),
        Err(Refusal::UndoNotClosed)
    );
    let f = facts!(s, &j, Some(&other_token), &acc, &tmp, &none, &dev);
    assert_eq!(
        preview(Action::RemoveTemporaryFiles, &f),
        Err(Refusal::UndoNotClosed)
    );
    let ok = facts!(s, &j, Some(&token), &acc, &tmp, &none, &dev);
    assert!(preview(Action::RemoveTemporaryFiles, &ok).is_ok());

    // T27-019: the same letter or label on another drive is another drive.
    for change in 0..4 {
        let mut d = drive();
        match change {
            0 => d.drive_serial = "S3Z9NB0K999999".into(),
            1 => d.drive_model = "Other".into(),
            2 => d.drive_bytes -= 1,
            _ => d.laptop = other_laptop(),
        }
        let f = facts!(s, &j, Some(&token), &acc, &tmp, &none, &d);
        assert_eq!(
            preview(Action::RemoveTemporaryFiles, &f),
            Err(Refusal::WrongDevice)
        );
    }
    // A drive with no serial cannot be told apart from another.
    let mut blank = drive();
    blank.drive_serial = " ".into();
    let mut f = facts!(s, &j, Some(&token), &acc, &tmp, &none, &blank);
    f.expected = &blank;
    assert_eq!(
        preview(Action::RemoveTemporaryFiles, &f),
        Err(Refusal::WrongDevice)
    );

    // T27-014: the old laptop cannot be reached.
    let mut f = facts!(s, &j, Some(&token), &acc, &tmp, &none, &dev);
    f.old_laptop_now = None;
    assert_eq!(
        preview(Action::RemoveTemporaryFiles, &f),
        Err(Refusal::OldLaptopNotReachable)
    );
    // T27-012: the old laptop changed since the list was made.
    let mut changed = s.record.clone();
    changed.items.push(file("new.txt"));
    let mut f = facts!(s, &j, Some(&token), &acc, &tmp, &none, &dev);
    f.old_laptop_now = Some(&changed);
    assert_eq!(
        preview(Action::RemoveTemporaryFiles, &f),
        Err(Refusal::ChecklistStale)
    );
    // An open exception.
    let mut open = s.checklist.clone();
    let (_, mut l) = ledger(vec![file("a.txt")]);
    l.searched("Documents");
    open.exceptions = Checklist::build(
        &Inputs {
            ledger: &l,
            approved: &s.record,
            old_laptop: OldLaptop::Scanned(&s.record),
            left_on_old_laptop: &[],
            steps_not_finished: &[],
            undo_open: false,
            special: &[],
        },
        None,
        T0,
    )
    .exceptions;
    let mut f = facts!(s, &j, Some(&token), &acc, &tmp, &none, &dev);
    f.checklist = &open;
    assert_eq!(
        preview(Action::RemoveTemporaryFiles, &f),
        Err(Refusal::ExceptionsOpen { open: 1 })
    );
    // T27-020: another program could write.
    let mut f = facts!(s, &j, Some(&token), &acc, &tmp, &none, &dev);
    f.write_exclusion = WriteExclusion::CannotExclude;
    for a in [
        Action::RemoveTemporaryFiles,
        Action::RemoveMoversAccount,
        Action::EraseOldLaptop,
    ] {
        assert_eq!(preview(a, &f), Err(Refusal::NoWriteExclusion));
    }
}

/// T27-015: only what PCTwin recorded making is removed; what looks temporary, or is the
/// person's chosen work, is kept and named.
#[test]
fn t27_015_only_pctwin_s_own_temporary_files() {
    let dir = tempfile::tempdir().unwrap();
    let j = journal(dir.path(), "j", 1);
    let token = close(&j);
    let s = setup();
    let (acc, tmp, dev) = (accounts(), temp(), drive());
    let none = vec![];
    let f = facts!(s, &j, Some(&token), &acc, &tmp, &none, &dev);
    let p = preview(Action::RemoveTemporaryFiles, &f).unwrap();
    assert_eq!(p.removes, ["download-cache"]);
    assert_eq!(p.keeps, ["model.gguf", "journal-looking-folder"]);
    let only_looks = vec![tmp[2].clone(), tmp[1].clone()];
    let f = facts!(s, &j, Some(&token), &acc, &only_looks, &none, &dev);
    assert_eq!(
        preview(Action::RemoveTemporaryFiles, &f),
        Err(Refusal::NothingToRemove)
    );
}

/// T27-016: handing on to family removes only the mover's account; erasing is refused while
/// other people keep their files there.
#[test]
fn t27_016_other_people_s_accounts_are_never_removed() {
    let dir = tempfile::tempdir().unwrap();
    let j = journal(dir.path(), "j", 1);
    let token = close(&j);
    let s = setup();
    let family = vec![
        Account {
            id: MOVER.into(),
            has_own_files: true,
        },
        Account {
            id: "1001".into(),
            has_own_files: true,
        },
        Account {
            id: "1002".into(),
            has_own_files: false,
        },
    ];
    let (tmp, dev) = (temp(), drive());
    let q = vec![QualifiedMethod {
        drive: drive(),
        method: "the drive's own sanitize command".into(),
    }];
    let f = facts!(s, &j, Some(&token), &family, &tmp, &q, &dev);
    let p = preview(Action::RemoveMoversAccount, &f).unwrap();
    assert_eq!(p.removes, [MOVER]);
    assert_eq!(p.keeps, ["1001", "1002"]);
    assert!(p.words.contains(SSD_NOTE));
    assert!(p.words.contains("does not cancel subscriptions"));
    assert_eq!(
        preview(Action::EraseOldLaptop, &f),
        Err(Refusal::OtherPeopleUseIt)
    );
    // An empty account of someone else does not hold the erase back; the mover not being on the
    // laptop refuses the account removal.
    let others_empty = vec![family[0].clone(), family[2].clone()];
    let f = facts!(s, &j, Some(&token), &others_empty, &tmp, &q, &dev);
    assert!(preview(Action::EraseOldLaptop, &f).is_ok());
    let not_here = vec![family[1].clone()];
    let f = facts!(s, &j, Some(&token), &not_here, &tmp, &q, &dev);
    assert_eq!(
        preview(Action::RemoveMoversAccount, &f),
        Err(Refusal::NoSuchAccount)
    );
}

/// T27-017: a finished move is never the confirmation; the confirmation is fresh, for exactly the
/// preview shown and this action, and used once.
#[test]
fn t27_017_only_a_fresh_separate_confirmation() {
    let dir = tempfile::tempdir().unwrap();
    let j = journal(dir.path(), "j", 1);
    let token = close(&j);
    let s = setup();
    let (acc, tmp, dev) = (accounts(), temp(), drive());
    let none = vec![];
    let f = facts!(s, &j, Some(&token), &acc, &tmp, &none, &dev);
    let mut used = Used::default();
    // Everything else is in order (the move finished, the list is clear), and still: no.
    assert_eq!(
        request(Action::RemoveTemporaryFiles, &f, None, &mut used),
        Err(Refusal::NoFreshConfirmation)
    );
    let p = preview(Action::RemoveTemporaryFiles, &f).unwrap();
    let other = preview(Action::RemoveMoversAccount, &f).unwrap();
    let wrong = confirm(&other, T0 + 190, [1; 16]);
    assert_eq!(
        request(Action::RemoveTemporaryFiles, &f, Some(&wrong), &mut used),
        Err(Refusal::WrongConfirmation)
    );
    let old = confirm(&p, T0 + 200 - 601, [2; 16]);
    assert_eq!(
        request(Action::RemoveTemporaryFiles, &f, Some(&old), &mut used),
        Err(Refusal::Expired)
    );
    let future = confirm(&p, T0 + 201, [3; 16]);
    assert_eq!(
        request(Action::RemoveTemporaryFiles, &f, Some(&future), &mut used),
        Err(Refusal::Expired)
    );
    // Confirmed, then something changed (another file to remove appeared): not what was shown.
    let c = confirm(&p, T0 + 190, [4; 16]);
    let mut more = tmp.clone();
    more.push(TempEntry {
        name: "x".into(),
        ownership: Ownership::RecordedByPctwin,
        item: None,
    });
    let f2 = facts!(s, &j, Some(&token), &acc, &more, &none, &dev);
    assert_eq!(
        request(Action::RemoveTemporaryFiles, &f2, Some(&c), &mut used),
        Err(Refusal::ChangedSinceConfirmed)
    );
    // Confirmed this very second is fresh.
    let now = confirm(&p, T0 + 200, [6; 16]);
    request(Action::RemoveTemporaryFiles, &f, Some(&now), &mut used).unwrap();
    let edge = confirm(&p, T0 + 200 - 600, [5; 16]);
    let r = request(Action::RemoveTemporaryFiles, &f, Some(&edge), &mut used).unwrap();
    assert_eq!(r.removes, ["download-cache"]);
    assert_eq!(r.device, drive());
    assert_eq!(r.generation, s.checklist.generation);
    assert_eq!(
        request(Action::RemoveTemporaryFiles, &f, Some(&edge), &mut used),
        Err(Refusal::AlreadyUsed)
    );
    // The destructive step itself is not built yet: nothing is done.
    assert_eq!(r.run(), NativeOutcome::NotBuiltYet);
}

/// T27-021: no erase method is qualified for this drive: the guided, unsupported outcome.
#[test]
fn t27_021_an_unqualified_drive_is_not_supported() {
    let dir = tempfile::tempdir().unwrap();
    let j = journal(dir.path(), "j", 1);
    let token = close(&j);
    let s = setup();
    let (acc, tmp, dev) = (accounts(), temp(), drive());
    let none = vec![];
    let f = facts!(s, &j, Some(&token), &acc, &tmp, &none, &dev);
    assert_eq!(
        preview(Action::EraseOldLaptop, &f),
        Err(Refusal::NotQualified)
    );
    // A method qualified for another drive does not count for this one.
    let mut d2 = drive();
    d2.drive_serial = "OTHER".into();
    let elsewhere = vec![QualifiedMethod {
        drive: d2,
        method: "m".into(),
    }];
    let f = facts!(s, &j, Some(&token), &acc, &tmp, &elsewhere, &dev);
    assert_eq!(
        preview(Action::EraseOldLaptop, &f),
        Err(Refusal::NotQualified)
    );
    assert!(ActionResult::NotSupported.words().contains("Reset this PC"));
}

/// T27-022 to T27-024, T27-026: results say what was observed and what may be gone; none is a
/// certificate or claims the old laptop is as it was.
#[test]
fn t27_022_to_026_results_never_overclaim() {
    for r in [
        ActionResult::Finished,
        ActionResult::Partial,
        ActionResult::Inconclusive,
        ActionResult::Interrupted,
        ActionResult::CancelledAfterSomeEffects,
        ActionResult::NotSupported,
    ] {
        let w = r.words().to_lowercase();
        for claim in [
            "certified",
            "securely erased",
            "sanitised",
            "restored",
            "is safe to",
            "guarantee",
        ] {
            assert!(!w.contains(claim), "{r:?}: {claim}");
        }
    }
    assert!(ActionResult::Finished.words().contains("not a certificate"));
    assert!(
        ActionResult::Interrupted
            .words()
            .contains("may already be gone")
    );
    assert!(
        ActionResult::CancelledAfterSomeEffects
            .words()
            .contains("cannot be brought back")
    );
    assert!(SSD_NOTE.contains("not a secure wipe"));
}
