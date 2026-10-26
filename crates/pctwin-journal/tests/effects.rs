//! The effect journal for installs and settings (System Design S14, "Effect journal and object
//! identity"): each install, setting and settings copy is recorded in the order intent durable,
//! effect, effect observed, receipt durable; what was there before is kept with it; steps cannot
//! be skipped or taken back; the undo gate and its closing cover effects too; and a damaged
//! record is reported, never trusted.

mod common;
use common::keys;

use pctwin_journal::{
    EffectEntry, EffectState, EffectUndo, EffectUndoOutcome, Found, InstallIntent, Journal,
    JournalError, KeptFile, PlannedEffect, Presence, Receipt, RestorePoint, Scope, SettingsFile,
    SettingsIntent, StoredEffect, content_hash,
};
use pctwin_recipe::{
    Catalog, RecipeId, RecipeRef, RelPath, SettingValue, SettingsPlace, SettingsRoot,
    UndoDeclaration,
};
use pctwin_record::ItemId;
use pctwin_types::Reason;

const SCOPE: Scope = Scope([7; 16]);

fn planned(step: u32) -> PlannedEffect {
    PlannedEffect {
        scope: SCOPE,
        item: ItemId::from_hex(&format!("aa{}", "0".repeat(30))).unwrap(),
        recipe: RecipeRef {
            id: RecipeId::new("org.videolan.vlc").unwrap(),
            version: 2,
        },
        step,
        actor: common::planned(1).actor,
    }
}

fn place() -> SettingsPlace {
    SettingsPlace {
        root: SettingsRoot::Config,
        path: RelPath::new("vlc").unwrap(),
    }
}

fn install(before: Presence) -> InstallIntent {
    InstallIntent::App {
        catalog: Catalog::Winget,
        package: "VideoLAN.VLC".into(),
        before,
        undo: UndoDeclaration {
            uninstall: true,
            side_effects: vec![],
        },
    }
}

fn setting(before: Option<SettingValue>) -> SettingsIntent {
    SettingsIntent::Apply {
        key: "dark-mode".into(),
        before,
        value: SettingValue::Bool(true),
    }
}

fn copy(files: &[(&str, &[u8])]) -> SettingsIntent {
    SettingsIntent::Copy {
        from: place(),
        to: place(),
        before: files
            .iter()
            .map(|(p, b)| SettingsFile {
                path: (*p).into(),
                bytes: b.to_vec(),
            })
            .collect(),
    }
}

fn present() -> Found {
    Found::App(Presence::Present {
        version: Some("3.0.21".into()),
    })
}

#[test]
fn an_install_goes_intent_observed_receipt_and_is_read_back_after_reopening() {
    let (_d, path, j) = common::journal();
    let id = j
        .installs(&keys().install)
        .intend(&planned(0), &install(Presence::Absent))
        .unwrap();
    let e = j.effect(id).unwrap().unwrap();
    assert_eq!(e.state, EffectState::Intended);
    assert_eq!(j.unfinished_effects().unwrap().len(), 1);
    j.installs(&keys().install)
        .observed(id, &present(), false)
        .unwrap();
    assert_eq!(j.unfinished_effects().unwrap().len(), 1, "no receipt yet");
    j.installs(&keys().install)
        .receipt(id, &Receipt::Done)
        .unwrap();
    drop(j);
    let j = common::open(&path).unwrap();
    assert!(j.unfinished_effects().unwrap().is_empty());
    let EffectEntry {
        planned: p,
        effect,
        state,
        ..
    } = j.effect(id).unwrap().unwrap();
    assert_eq!(p, planned(0));
    assert!(matches!(
        effect,
        StoredEffect::Install {
            before: Presence::Absent,
            ..
        }
    ));
    assert_eq!(
        state,
        EffectState::Receipted {
            found: present(),
            after_restart: false,
            receipt: Receipt::Done
        }
    );
}

#[test]
fn effect_numbers_are_never_reused_and_kept_apart_from_file_writes() {
    let (_d, path, j) = common::journal();
    let a = j
        .settings(&keys().setting)
        .intend(&planned(0), &setting(None))
        .unwrap();
    let w = common::committed(&j, 1);
    let b = j
        .settings(&keys().setting)
        .intend(&planned(1), &setting(None))
        .unwrap();
    assert!(b > a);
    drop(j);
    let j = common::open(&path).unwrap();
    let c = j
        .settings(&keys().setting)
        .intend(&planned(2), &setting(None))
        .unwrap();
    assert!(c > b);
    assert_eq!(j.effects().unwrap().len(), 3);
    assert_eq!(j.entries().unwrap().len(), 1);
    assert!(j.entry(w).unwrap().is_some());
}

#[test]
fn steps_cannot_be_skipped_repeated_or_taken_back() {
    let (_d, _p, j) = common::journal();
    let id = j
        .settings(&keys().setting)
        .intend(&planned(0), &setting(None))
        .unwrap();
    assert!(matches!(
        j.settings(&keys().setting).receipt(id, &Receipt::Done),
        Err(JournalError::OutOfOrder { .. })
    ));
    j.settings(&keys().setting)
        .observed(id, &Found::Setting(Some(SettingValue::Bool(true))), false)
        .unwrap();
    assert!(matches!(
        j.settings(&keys().setting)
            .observed(id, &Found::Setting(None), false),
        Err(JournalError::OutOfOrder { .. })
    ));
    j.settings(&keys().setting)
        .receipt(id, &Receipt::Done)
        .unwrap();
    assert!(matches!(
        j.settings(&keys().setting).receipt(
            id,
            &Receipt::Failed {
                reason: Reason::NotVerified
            }
        ),
        Err(JournalError::OutOfOrder { .. })
    ));
    assert!(matches!(
        j.settings(&keys().setting)
            .observed(id, &Found::Setting(None), true),
        Err(JournalError::OutOfOrder { .. })
    ));
    assert!(matches!(
        j.settings(&keys().setting)
            .observed(999, &Found::Setting(None), true),
        Err(JournalError::NoSuchEntry(999))
    ));
}

#[test]
fn an_observation_of_another_kind_is_refused() {
    let (_d, _p, j) = common::journal();
    let s = j
        .settings(&keys().setting)
        .intend(&planned(0), &setting(None))
        .unwrap();
    let i = j
        .installs(&keys().install)
        .intend(&planned(1), &install(Presence::Absent))
        .unwrap();
    let c = j
        .settings(&keys().setting)
        .intend(&planned(2), &copy(&[]))
        .unwrap();
    for (id, wrong) in [(s, present()), (c, present()), (s, Found::Files(vec![]))] {
        assert!(matches!(
            j.settings(&keys().setting).observed(id, &wrong, false),
            Err(JournalError::OutOfOrder { .. })
        ));
        assert_eq!(j.effect(id).unwrap().unwrap().state, EffectState::Intended);
    }
    assert!(matches!(
        j.installs(&keys().install)
            .observed(i, &Found::Setting(None), false),
        Err(JournalError::OutOfOrder { .. })
    ));
    assert_eq!(j.effect(i).unwrap().unwrap().state, EffectState::Intended);
}

#[test]
fn settings_files_there_before_are_kept_in_the_journal_with_their_hashes() {
    let (_d, path, j) = common::journal();
    let id = j
        .settings(&keys().setting)
        .intend(
            &planned(0),
            &copy(&[("vlcrc", b"old settings"), ("sub/x.ini", b"")]),
        )
        .unwrap();
    drop(j);
    let j = common::open(&path).unwrap();
    let StoredEffect::CopySettings { before, .. } = j.effect(id).unwrap().unwrap().effect else {
        panic!("not a copy");
    };
    assert_eq!(
        before,
        vec![
            KeptFile {
                path: "vlcrc".into(),
                hash: content_hash(b"old settings"),
                size: 12,
            },
            KeptFile {
                path: "sub/x.ini".into(),
                hash: content_hash(b""),
                size: 0,
            },
        ]
    );
    assert_eq!(
        j.kept_bytes(&before[0].hash).unwrap(),
        b"old settings".to_vec()
    );
    assert_eq!(j.kept_bytes(&before[1].hash).unwrap(), Vec::<u8>::new());
    assert!(matches!(
        j.kept_bytes(&content_hash(b"never kept")),
        Err(JournalError::Damaged(_))
    ));
}

#[test]
fn a_kept_file_damaged_on_disk_is_reported_never_handed_back() {
    let (_d, path, j) = common::journal();
    let id = j
        .settings(&keys().setting)
        .intend(&planned(0), &copy(&[("vlcrc", b"old")]))
        .unwrap();
    let StoredEffect::CopySettings { before, .. } = j.effect(id).unwrap().unwrap().effect else {
        panic!("not a copy");
    };
    drop(j);
    {
        let db = redb::Database::open(&path).unwrap();
        let tx = db.begin_write().unwrap();
        tx.open_table(redb::TableDefinition::<[u8; 32], &[u8]>::new("kept-files"))
            .unwrap()
            .insert(before[0].hash, b"new".as_slice())
            .unwrap();
        tx.commit().unwrap();
    }
    let j = common::open(&path).unwrap();
    assert!(matches!(
        j.kept_bytes(&before[0].hash),
        Err(JournalError::Damaged(_))
    ));
}

#[test]
fn a_damaged_effect_record_is_reported() {
    let (_d, path, j) = common::journal();
    let id = j
        .settings(&keys().setting)
        .intend(&planned(0), &setting(None))
        .unwrap();
    drop(j);
    {
        let db = redb::Database::open(&path).unwrap();
        let tx = db.begin_write().unwrap();
        tx.open_table(redb::TableDefinition::<u64, &[u8]>::new("effects"))
            .unwrap()
            .insert(id, b"{not json".as_slice())
            .unwrap();
        tx.commit().unwrap();
    }
    let j = common::open(&path).unwrap();
    assert!(matches!(j.effect(id), Err(JournalError::Damaged(_))));
    assert!(matches!(j.effects(), Err(JournalError::Damaged(_))));
    assert!(matches!(
        j.unfinished_effects(),
        Err(JournalError::Damaged(_))
    ));
    assert!(matches!(
        j.settings(&keys().setting)
            .observed(id, &Found::Setting(None), true),
        Err(JournalError::Damaged(_))
    ));
}

#[test]
fn unsafe_settings_file_paths_and_oversized_copies_are_refused_and_nothing_is_recorded() {
    let (_d, _p, j) = common::journal();
    for bad in [
        "../x", "/etc/x", "a//b", "a/./b", "a/../b", "C:\\x", "a\\b", "x\0", ".", "a/",
    ] {
        assert!(
            matches!(
                j.settings(&keys().setting)
                    .intend(&planned(0), &copy(&[(bad, b"x")])),
                Err(JournalError::BadName)
            ),
            "{bad:?}"
        );
    }
    let big = vec![0u8; pctwin_journal::MAX_KEPT_FILE + 1];
    assert!(matches!(
        j.settings(&keys().setting)
            .intend(&planned(0), &copy(&[("big", &big)])),
        Err(JournalError::TooLarge)
    ));
    let many: Vec<(String, Vec<u8>)> = (0..=pctwin_journal::MAX_KEPT_FILES)
        .map(|n| (format!("f{n}"), vec![]))
        .collect();
    let many: Vec<(&str, &[u8])> = many
        .iter()
        .map(|(p, b)| (p.as_str(), b.as_slice()))
        .collect();
    assert!(matches!(
        j.settings(&keys().setting)
            .intend(&planned(0), &copy(&many)),
        Err(JournalError::TooLarge)
    ));
    let chunk = vec![1u8; pctwin_journal::MAX_KEPT_FILE];
    let names: Vec<String> = (0..=pctwin_journal::MAX_KEPT_TOTAL / pctwin_journal::MAX_KEPT_FILE)
        .map(|n| format!("c{n}"))
        .collect();
    let total: Vec<(&str, &[u8])> = names
        .iter()
        .map(|n| (n.as_str(), chunk.as_slice()))
        .collect();
    assert!(matches!(
        j.settings(&keys().setting)
            .intend(&planned(0), &copy(&total)),
        Err(JournalError::TooLarge)
    ));
    // The same path twice is not one place's files.
    assert!(matches!(
        j.settings(&keys().setting)
            .intend(&planned(0), &copy(&[("a", b"1"), ("a", b"2")])),
        Err(JournalError::BadName)
    ));
    assert!(j.effects().unwrap().is_empty());
    // An exactly-full file and an empty place are fine.
    let full = vec![2u8; pctwin_journal::MAX_KEPT_FILE];
    j.settings(&keys().setting)
        .intend(&planned(0), &copy(&[("full", &full)]))
        .unwrap();
    j.settings(&keys().setting)
        .intend(&planned(1), &copy(&[]))
        .unwrap();
    // A place that is one file is kept under the empty path.
    j.settings(&keys().setting)
        .intend(&planned(2), &copy(&[("", b"one file")]))
        .unwrap();
    let exactly: Vec<(String, Vec<u8>)> = (0..pctwin_journal::MAX_KEPT_FILES)
        .map(|n| (format!("f{n}"), vec![]))
        .collect();
    let exactly: Vec<(&str, &[u8])> = exactly
        .iter()
        .map(|(p, b)| (p.as_str(), b.as_slice()))
        .collect();
    j.settings(&keys().setting)
        .intend(&planned(3), &copy(&exactly))
        .unwrap();
    let names: Vec<String> = (0..pctwin_journal::MAX_KEPT_TOTAL / pctwin_journal::MAX_KEPT_FILE)
        .map(|n| format!("c{n}"))
        .collect();
    let at_total: Vec<(&str, &[u8])> = names
        .iter()
        .map(|n| (n.as_str(), full.as_slice()))
        .collect();
    j.settings(&keys().setting)
        .intend(&planned(4), &copy(&at_total))
        .unwrap();
}

#[test]
fn undo_of_an_effect_needs_a_receipt_a_right_and_an_open_gate() {
    let (_d, _p, j) = common::journal();
    let id = j
        .settings(&keys().setting)
        .intend(&planned(0), &setting(None))
        .unwrap();
    let permit = j.begin_undo().unwrap();
    assert!(matches!(
        j.settings(&keys().setting)
            .record_undo(&permit, id, &EffectUndo::Reversing),
        Err(JournalError::OutOfOrder { .. })
    ));
    j.settings(&keys().setting)
        .observed(id, &Found::Setting(Some(SettingValue::Bool(true))), false)
        .unwrap();
    assert!(matches!(
        j.settings(&keys().setting)
            .record_undo(&permit, id, &EffectUndo::Reversing),
        Err(JournalError::OutOfOrder { .. })
    ));
    j.settings(&keys().setting)
        .receipt(id, &Receipt::Done)
        .unwrap();
    // A right from another journal is refused.
    let (_d2, _p2, other) = common::journal();
    let foreign = other.begin_undo().unwrap();
    assert!(matches!(
        j.settings(&keys().setting)
            .record_undo(&foreign, id, &EffectUndo::Reversing),
        Err(JournalError::OtherJournal)
    ));
    let (lending, lent) = pctwin_journal::UndoRight::lend(&permit);
    assert!(matches!(
        j.settings(&keys().setting)
            .record_undo(&lent, id, &EffectUndo::Reversing),
        Err(JournalError::OtherJournal)
    ));
    drop(lending);
    j.settings(&keys().setting)
        .record_undo(&permit, id, &EffectUndo::Reversing)
        .unwrap();
    assert_eq!(j.effect_undo_of(id).unwrap(), Some(EffectUndo::Reversing));
}

#[test]
fn effect_undo_stages_go_one_way() {
    let (_d, _p, j) = common::journal();
    let id = receipted_setting(&j, 0);
    let permit = j.begin_undo().unwrap();
    let done = |o: EffectUndoOutcome| EffectUndo::Done { outcome: o };
    // Not started: Reversing, or straight to Done (nothing to put back).
    j.settings(&keys().setting)
        .record_undo(&permit, id, &EffectUndo::Reversing)
        .unwrap();
    assert!(matches!(
        j.settings(&keys().setting)
            .record_undo(&permit, id, &EffectUndo::Reversing),
        Err(JournalError::OutOfOrder { .. })
    ));
    j.settings(&keys().setting)
        .record_undo(
            &permit,
            id,
            &done(EffectUndoOutcome::NotDone {
                why: "the drive is gone".into(),
            }),
        )
        .unwrap();
    // "Not done this time" may start again.
    j.settings(&keys().setting)
        .record_undo(&permit, id, &EffectUndo::Reversing)
        .unwrap();
    j.settings(&keys().setting)
        .record_undo(&permit, id, &done(EffectUndoOutcome::Restored))
        .unwrap();
    // A final Done is never written over.
    for next in [
        EffectUndo::Reversing,
        done(EffectUndoOutcome::AsBefore),
        done(EffectUndoOutcome::NotDone { why: "x".into() }),
    ] {
        assert!(matches!(
            j.settings(&keys().setting).record_undo(&permit, id, &next),
            Err(JournalError::OutOfOrder { .. })
        ));
    }
    assert_eq!(
        j.effect_undo_of(id).unwrap(),
        Some(done(EffectUndoOutcome::Restored))
    );
    let other = receipted_setting(&j, 1);
    j.settings(&keys().setting)
        .record_undo(&permit, other, &done(EffectUndoOutcome::AsBefore))
        .unwrap();
}

fn receipted_setting(j: &Journal, step: u32) -> u64 {
    let id = j
        .settings(&keys().setting)
        .intend(&planned(step), &setting(None))
        .unwrap();
    j.settings(&keys().setting)
        .observed(id, &Found::Setting(Some(SettingValue::Bool(true))), false)
        .unwrap();
    j.settings(&keys().setting)
        .receipt(id, &Receipt::Done)
        .unwrap();
    id
}

#[test]
fn outcomes_say_which_need_the_person() {
    assert!(EffectUndoOutcome::Kept { why: "x".into() }.needs_action());
    assert!(EffectUndoOutcome::NotDone { why: "x".into() }.needs_action());
    for done in [
        EffectUndoOutcome::Uninstalled,
        EffectUndoOutcome::Restored,
        EffectUndoOutcome::AsBefore,
    ] {
        assert!(!done.needs_action());
        assert!(done.is_final());
    }
    assert!(!EffectUndoOutcome::NotDone { why: "x".into() }.is_final());
    assert!(EffectUndoOutcome::Kept { why: "x".into() }.is_final());
}

#[test]
fn closing_undo_refuses_while_an_app_or_setting_is_being_put_back_and_then_shuts_effects_too() {
    let (_d, path, j) = common::journal();
    let id = receipted_setting(&j, 0);
    {
        let permit = j.begin_undo().unwrap();
        j.settings(&keys().setting)
            .record_undo(&permit, id, &EffectUndo::Reversing)
            .unwrap();
    }
    match j.close_undo(&mut |_, _| unreachable!("no file undo is part-way")) {
        Err(pctwin_journal::CloseError::EffectsPartWay { ids }) => assert_eq!(ids, vec![id]),
        other => panic!("expected refused, got {other:?}"),
    }
    assert_eq!(j.undo_gate().unwrap(), pctwin_journal::UndoGate::Open);
    {
        let permit = j.begin_undo().unwrap();
        j.settings(&keys().setting)
            .record_undo(
                &permit,
                id,
                &EffectUndo::Done {
                    outcome: EffectUndoOutcome::Restored,
                },
            )
            .unwrap();
    }
    let other = receipted_setting(&j, 1);
    j.close_undo(&mut |_, _| unreachable!()).unwrap();
    assert!(matches!(j.begin_undo(), Err(JournalError::UndoClosed)));
    drop(j);
    let j = common::open(&path).unwrap();
    assert!(matches!(j.begin_undo(), Err(JournalError::UndoClosed)));
    // What was there before stays kept for good, even with undo closed.
    assert!(j.effect(other).unwrap().is_some());
}

#[test]
fn closing_checks_effects_again_inside_its_own_transaction() {
    // A file undo the resolver finishes, and an effect undo begun with the close ticket while
    // closing ran: the close must not be written beside it.
    let (_d, _p, j) = common::journal();
    let w = common::committed(&j, 1);
    let id = receipted_setting(&j, 0);
    {
        let permit = j.begin_undo().unwrap();
        j.record_removals(&permit, &[(w, common::removing(1))])
            .unwrap();
    }
    let r = j.close_undo(&mut |ticket, _| {
        j.settings(&keys().setting)
            .record_undo(ticket, id, &EffectUndo::Reversing)
            .unwrap();
        Ok(pctwin_journal::Resolved::Removed)
    });
    match r {
        Err(pctwin_journal::CloseError::EffectsPartWay { ids }) => assert_eq!(ids, vec![id]),
        other => panic!("expected refused, got {other:?}"),
    }
    assert_eq!(j.undo_gate().unwrap(), pctwin_journal::UndoGate::Open);
    assert!(
        j.undo_of(w).unwrap().is_some_and(|u| !u.is_done()),
        "nothing of the refused close was written"
    );
}

#[test]
fn closing_asks_nothing_of_the_files_while_an_effect_is_part_way() {
    // Closing cannot finish while a setting is half put back, so it must not start removing
    // files either.
    let (_d, _p, j) = common::journal();
    let w = common::committed(&j, 1);
    let id = receipted_setting(&j, 0);
    {
        let permit = j.begin_undo().unwrap();
        j.record_removals(&permit, &[(w, common::removing(1))])
            .unwrap();
        j.settings(&keys().setting)
            .record_undo(&permit, id, &EffectUndo::Reversing)
            .unwrap();
    }
    let r = j.close_undo(&mut |_, _| panic!("no file may be touched"));
    assert!(matches!(
        r,
        Err(pctwin_journal::CloseError::EffectsPartWay { .. })
    ));
}

#[test]
fn the_restore_point_answer_is_kept_once_per_move() {
    let (_d, path, j) = common::journal();
    assert_eq!(j.restore_point(SCOPE).unwrap(), None);
    let made = RestorePoint::Made {
        at: 1_790_000_000,
        number: 41,
    };
    j.installs(&keys().install)
        .record_restore_point(SCOPE, &made)
        .unwrap();
    assert!(matches!(
        j.installs(&keys().install)
            .record_restore_point(SCOPE, &RestorePoint::NotWindows),
        Err(JournalError::OutOfOrder { .. })
    ));
    let other = Scope([8; 16]);
    j.installs(&keys().install)
        .record_restore_point(
            other,
            &RestorePoint::NotMade {
                why: "System Restore is turned off on this laptop".into(),
            },
        )
        .unwrap();
    drop(j);
    let j = common::open(&path).unwrap();
    assert_eq!(j.restore_point(SCOPE).unwrap(), Some(made));
    assert!(matches!(
        j.restore_point(other).unwrap(),
        Some(RestorePoint::NotMade { .. })
    ));
}

#[test]
fn effects_of_one_move_are_listed_in_order_without_another_moves() {
    let (_d, _p, j) = common::journal();
    let a = j
        .settings(&keys().setting)
        .intend(&planned(0), &setting(None))
        .unwrap();
    let mut elsewhere = planned(1);
    elsewhere.scope = Scope([9; 16]);
    j.settings(&keys().setting)
        .intend(&elsewhere, &setting(None))
        .unwrap();
    let b = j
        .installs(&keys().install)
        .intend(&planned(2), &install(Presence::Absent))
        .unwrap();
    let ids: Vec<u64> = j
        .effects_of(SCOPE)
        .unwrap()
        .into_iter()
        .map(|e| e.id)
        .collect();
    assert_eq!(ids, vec![a, b]);
}
