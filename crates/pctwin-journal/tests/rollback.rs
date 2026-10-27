//! Second fresh-context security review, finding S2 (27 October 2026): a journal put back from an
//! older copy of its file is refused, whatever was written after the copy was taken (a step of a
//! change, a broker action, the close of undo).

mod common;
use common::keys;

use pctwin_journal::{
    BrokerAction, Found, InstallIntent, Journal, JournalError, PlannedEffect, Presence, Receipt,
    Scope,
};
use pctwin_recipe::{RecipeId, RecipeRef};
use pctwin_record::ItemId;

fn planned() -> PlannedEffect {
    PlannedEffect {
        scope: Scope([7; 16]),
        item: ItemId::from_hex(&format!("aa{}", "0".repeat(30))).unwrap(),
        recipe: RecipeRef {
            id: RecipeId::new("org.videolan.vlc").unwrap(),
            version: 2,
        },
        step: 0,
        actor: common::planned(1).actor,
    }
}

fn an_install(j: &Journal) -> u64 {
    j.installs(&keys().install)
        .intend(
            &planned(),
            &InstallIntent::App {
                catalog: pctwin_recipe::Catalog::Winget,
                package: "VideoLAN.VLC".into(),
                before: Presence::Absent,
                undo: Default::default(),
            },
        )
        .unwrap()
}

/// Runs `then` on the journal at `path`, after copying the file aside; puts the copy back and
/// returns the journal reopened on it.
fn rolled_back(
    dir: &std::path::Path,
    path: &std::path::Path,
    j: Journal,
    then: impl FnOnce(&Journal),
) -> Journal {
    drop(j);
    let copy = dir.join("copy.redb");
    std::fs::copy(path, &copy).unwrap();
    let j = common::open(path).unwrap();
    then(&j);
    drop(j);
    std::fs::copy(&copy, path).unwrap();
    common::open(path).unwrap()
}

#[test]
fn a_copy_from_before_a_step_of_a_change_is_refused() {
    let (d, path, j) = common::journal();
    let id = an_install(&j);
    let j = rolled_back(d.path(), &path, j, |j| {
        let installs = j.installs(&keys().install);
        installs
            .observed(id, &Found::App(Presence::Absent), false)
            .unwrap();
        installs.receipt(id, &Receipt::Done).unwrap();
    });
    assert!(matches!(j.effects(), Err(JournalError::RolledBack)));
    assert!(matches!(j.effect(id), Err(JournalError::RolledBack)));
    assert!(matches!(
        j.effects_checked(Scope([7; 16])),
        Err(JournalError::RolledBack)
    ));
}

fn action(n: u8) -> BrokerAction {
    BrokerAction {
        key: [n; 32],
        plan_generation: 7,
        plan_digest: [9; 32],
        action_id: format!("install.app{n}"),
        action: r#"{"create-restore-point":{}}"#.into(),
    }
}

/// A finished administrator action must never read as unfinished again (it would be run again).
#[test]
fn a_copy_from_before_a_broker_outcome_is_refused() {
    let (d, path, j) = common::journal();
    let id = j.broker(&keys().broker).intend(&action(1)).unwrap();
    let j = rolled_back(d.path(), &path, j, |j| {
        j.broker(&keys().broker).outcome(id, "done").unwrap();
    });
    assert!(matches!(
        j.broker(&keys().broker).lookup(&[1; 32]),
        Err(JournalError::RolledBack)
    ));
}

#[test]
fn a_copy_from_before_a_broker_intent_is_refused() {
    let (d, path, j) = common::journal();
    j.broker(&keys().broker).intend(&action(1)).unwrap();
    let j = rolled_back(d.path(), &path, j, |j| {
        j.broker(&keys().broker).intend(&action(2)).unwrap();
    });
    assert!(matches!(
        j.broker(&keys().broker).lookup(&[1; 32]),
        Err(JournalError::RolledBack)
    ));
}

/// Closing undo is counted too: a copy from before the close does not reopen undo of changes.
#[test]
fn a_copy_from_before_undo_closed_is_refused() {
    let (d, path, j) = common::journal();
    let id = an_install(&j);
    let installs = j.installs(&keys().install);
    installs
        .observed(id, &Found::App(Presence::Absent), false)
        .unwrap();
    installs.receipt(id, &Receipt::Done).unwrap();
    let j = rolled_back(d.path(), &path, j, |j| {
        j.close_undo(&mut |_, _| unreachable!()).unwrap();
    });
    assert!(matches!(j.effects(), Err(JournalError::RolledBack)));
}

/// The journal as PCTwin left it is trusted, however many times it is opened.
#[test]
fn the_newest_journal_is_trusted() {
    let (_d, path, j) = common::journal();
    let id = an_install(&j);
    drop(j);
    for _ in 0..3 {
        let j = common::open(&path).unwrap();
        assert_eq!(j.effect(id).unwrap().unwrap().id, id);
    }
}

/// A committed Phase 1 file write (no sealed record of a change at all).
fn a_file_write(j: &Journal) -> u64 {
    let id = j.plan(&common::planned(1)).unwrap();
    let path = common::planned(1).path;
    j.staged(id, ".pctwin-t.part", &[]).unwrap();
    j.verified(id, [1; 32], None).unwrap();
    j.applied(id, &path).unwrap();
    j.committed(
        id,
        pctwin_journal::Landed {
            size: common::planned(1).size,
            modified_ns: None,
            file: None,
        },
    )
    .unwrap();
    id
}

/// Third review, finding 9 (the Chief's decision): a journal holding only Phase 1 file records is
/// counted too. A copy from before undo closed, put back after the old laptop was wiped, never
/// reopens undo (which would remove the only copies left).
#[test]
fn a_copy_of_a_file_only_journal_from_before_undo_closed_is_refused() {
    let (d, path, j) = common::journal();
    a_file_write(&j);
    let j = rolled_back(d.path(), &path, j, |j| {
        j.close_undo(&mut |_, _| unreachable!()).unwrap();
    });
    assert_eq!(j.undo_gate().unwrap(), pctwin_journal::UndoGate::Open);
    assert!(matches!(j.begin_undo(), Err(JournalError::RolledBack)));
}

/// A file-only journal as PCTwin left it still undoes, before and after it is reopened, until
/// it is closed.
#[test]
fn a_file_only_journal_undoes_until_closed() {
    let (_d, path, j) = common::journal();
    a_file_write(&j);
    drop(j.begin_undo().unwrap());
    drop(j);
    let j = common::open(&path).unwrap();
    drop(j.begin_undo().unwrap());
    j.close_undo(&mut |_, _| unreachable!()).unwrap();
    drop(j);
    let j = common::open(&path).unwrap();
    assert!(matches!(j.begin_undo(), Err(JournalError::UndoClosed)));
}

/// A journal that counts, put back from before a later sealed write, does not begin undo.
#[test]
fn a_counted_journal_put_back_does_not_begin_undo() {
    let (d, path, j) = common::journal();
    let id = an_install(&j);
    let j = rolled_back(d.path(), &path, j, |j| {
        j.installs(&keys().install)
            .observed(id, &Found::App(Presence::Absent), false)
            .unwrap();
    });
    assert!(matches!(j.begin_undo(), Err(JournalError::RolledBack)));
}
