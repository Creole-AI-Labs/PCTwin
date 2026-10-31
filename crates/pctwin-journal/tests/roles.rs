//! Real walls between roles (Phase 2 adoption, change 1): every effect belongs to one role, each
//! role records only its own kind of change, and a role cannot observe, finish or undo another
//! role's effect. Each effect has its class (Phase 2 plan 2.3, section 3), which bounds what
//! undo may claim. And a record changed by hand is never a source of paths (T23-006), what
//! support sees carries nothing private (T23-030), and the journal is kept only on this laptop's
//! own drive (T23-009).

mod common;
use common::keys;

use pctwin_journal::place::{FsKind, Placement, classify, linux_kind, removable_in};
use pctwin_journal::{
    EffectClass, EffectUndo, EffectUndoOutcome, Found, ImportIntent, InstallIntent, Journal,
    JournalError, PlannedEffect, Presence, Receipt, Role, Scope, SettingsFile, SettingsIntent,
    StoredEffect, UndoRoute,
};
use pctwin_recipe::{
    Catalog, RecipeId, RecipeRef, RelPath, SettingValue, SettingsPlace, SettingsRoot,
};
use pctwin_record::ItemId;
use redb::ReadableTable;

fn planned() -> PlannedEffect {
    PlannedEffect {
        scope: Scope([7; 16]),
        item: ItemId::from_hex(&format!("aa{}", "0".repeat(30))).unwrap(),
        recipe: RecipeRef {
            id: RecipeId::new("org.example.mail").unwrap(),
            version: 1,
        },
        step: 0,
        actor: common::planned(1).actor,
    }
}

fn place() -> SettingsPlace {
    SettingsPlace {
        root: SettingsRoot::LocalData,
        path: RelPath::new("Mail").unwrap(),
    }
}

fn app() -> InstallIntent {
    InstallIntent::App {
        catalog: Catalog::Winget,
        package: "A.B".into(),
        before: Presence::Absent,
        undo: Default::default(),
    }
}

fn set() -> SettingsIntent {
    SettingsIntent::Apply {
        key: "k".into(),
        before: None,
        value: SettingValue::Int(1),
    }
}

fn import() -> ImportIntent {
    ImportIntent::Files {
        to: place(),
        before: vec![SettingsFile {
            path: "inbox.mbox".into(),
            bytes: b"before".to_vec(),
        }],
    }
}

/// Each role's effect, finished.
fn three(j: &Journal) -> (u64, u64, u64) {
    let i = j
        .installs(&keys().install)
        .intend(&planned(), &app())
        .unwrap();
    j.installs(&keys().install)
        .observed(i, &Found::App(Presence::Present { version: None }), false)
        .unwrap();
    j.installs(&keys().install)
        .receipt(i, &Receipt::Done)
        .unwrap();
    let s = j
        .settings(&keys().setting)
        .intend(&planned(), &set())
        .unwrap();
    j.settings(&keys().setting)
        .observed(s, &Found::Setting(Some(SettingValue::Int(1))), false)
        .unwrap();
    j.settings(&keys().setting)
        .receipt(s, &Receipt::Done)
        .unwrap();
    let m = j
        .imports(&keys().capture)
        .intend(&planned(), &import())
        .unwrap();
    j.imports(&keys().capture)
        .observed(m, &Found::Files(vec![]), false)
        .unwrap();
    j.imports(&keys().capture)
        .receipt(m, &Receipt::Done)
        .unwrap();
    (i, s, m)
}

#[test]
fn each_effect_is_stamped_with_the_role_that_recorded_it() {
    let (_d, _p, j) = common::journal();
    let (i, s, m) = three(&j);
    let role = |id| j.effect(id).unwrap().unwrap().role;
    assert_eq!(role(i), Role::Install);
    assert_eq!(role(s), Role::Settings);
    assert_eq!(role(m), Role::Import);
}

#[test]
fn a_role_cannot_observe_finish_or_undo_another_roles_effect() {
    let (_d, _p, j) = common::journal();
    let i = j
        .installs(&keys().install)
        .intend(&planned(), &app())
        .unwrap();
    let wrong = |r: Result<(), JournalError>, owner: Role| {
        assert!(
            matches!(r, Err(JournalError::WrongRole { belongs_to, .. }) if belongs_to == owner),
            "{r:?}"
        );
    };
    let present = Found::App(Presence::Present { version: None });
    wrong(
        j.settings(&keys().setting).observed(i, &present, false),
        Role::Install,
    );
    wrong(
        j.imports(&keys().capture).observed(i, &present, false),
        Role::Install,
    );
    j.installs(&keys().install)
        .observed(i, &present, false)
        .unwrap();
    wrong(
        j.settings(&keys().setting).receipt(i, &Receipt::Done),
        Role::Install,
    );
    j.installs(&keys().install)
        .receipt(i, &Receipt::Done)
        .unwrap();
    let permit = j.begin_undo().unwrap();
    wrong(
        j.settings(&keys().setting)
            .record_undo(&permit, i, &EffectUndo::Reversing),
        Role::Install,
    );
    wrong(
        j.imports(&keys().capture)
            .record_undo(&permit, i, &EffectUndo::Reversing),
        Role::Install,
    );
    assert_eq!(j.effect_undo_of(i).unwrap(), None, "nothing was written");
    j.installs(&keys().install)
        .record_undo(&permit, i, &EffectUndo::Reversing)
        .unwrap();
}

#[test]
fn every_effect_has_its_class_and_the_class_bounds_undo() {
    let (_d, _p, j) = common::journal();
    let (i, s, m) = three(&j);
    let class = |id| j.effect(id).unwrap().unwrap().effect.class();
    assert_eq!(class(i), EffectClass::NativePackageInstall);
    assert_eq!(class(s), EffectClass::SettingsChange);
    assert_eq!(class(m), EffectClass::PrivateDataImport);
    let empty = j
        .settings(&keys().setting)
        .intend(
            &planned(),
            &SettingsIntent::Copy {
                from: place(),
                to: place(),
                before: vec![],
            },
        )
        .unwrap();
    assert_eq!(class(empty), EffectClass::OwnedStagedFile);
    let over = j
        .settings(&keys().setting)
        .intend(
            &planned(),
            &SettingsIntent::Copy {
                from: place(),
                to: place(),
                before: vec![SettingsFile {
                    path: "x".into(),
                    bytes: vec![1],
                }],
            },
        )
        .unwrap();
    assert_eq!(class(over), EffectClass::ExistingTargetReplacement);
    let remote = j
        .imports(&keys().capture)
        .intend(
            &planned(),
            &ImportIntent::Remote {
                identity: "upload 7f3a".into(),
            },
        )
        .unwrap();
    assert_eq!(class(remote), EffectClass::RemoteServiceEffect);
    let licence = j
        .installs(&keys().install)
        .intend(
            &planned(),
            &InstallIntent::Licence {
                identity: "activation 2291".into(),
            },
        )
        .unwrap();
    assert_eq!(class(licence), EffectClass::CredentialOrLicenceChange);
    for c in [
        EffectClass::OwnedStagedFile,
        EffectClass::ExistingTargetReplacement,
        EffectClass::NativePackageInstall,
        EffectClass::SettingsChange,
        EffectClass::PrivateDataImport,
    ] {
        assert_eq!(c.undo_route(), UndoRoute::Here, "{c:?}");
    }
    assert_eq!(
        EffectClass::RemoteServiceEffect.undo_route(),
        UndoRoute::Elsewhere
    );
    assert_eq!(
        EffectClass::CredentialOrLicenceChange.undo_route(),
        UndoRoute::Elsewhere
    );
    assert_eq!(
        EffectClass::DestructiveSanitisation.undo_route(),
        UndoRoute::Never
    );
}

#[test]
fn an_external_identity_is_short_plain_text_or_nothing_is_recorded() {
    let (_d, _p, j) = common::journal();
    for bad in [
        String::new(),
        "x".repeat(pctwin_journal::MAX_IDENTITY + 1),
        "id\u{202E}gpj".into(),
        "line\nbreak".into(),
    ] {
        assert!(matches!(
            j.imports(&keys().capture)
                .intend(&planned(), &ImportIntent::Remote { identity: bad }),
            Err(JournalError::BadName)
        ));
    }
    assert!(j.effects().unwrap().is_empty());
    j.imports(&keys().capture)
        .intend(
            &planned(),
            &ImportIntent::Remote {
                identity: "x".repeat(pctwin_journal::MAX_IDENTITY),
            },
        )
        .unwrap();
}

#[test]
fn an_external_effect_is_observed_by_whether_it_happened() {
    let (_d, _p, j) = common::journal();
    let r = j
        .imports(&keys().capture)
        .intend(
            &planned(),
            &ImportIntent::Remote {
                identity: "upload 1".into(),
            },
        )
        .unwrap();
    assert!(matches!(
        j.imports(&keys().capture)
            .observed(r, &Found::Files(vec![]), false),
        Err(JournalError::OutOfOrder { .. })
    ));
    j.imports(&keys().capture)
        .observed(r, &Found::External { happened: true }, false)
        .unwrap();
}

#[test]
fn imported_files_keep_what_was_there_before_whole() {
    let (_d, _p, j) = common::journal();
    let m = j
        .imports(&keys().capture)
        .intend(&planned(), &import())
        .unwrap();
    let StoredEffect::Import { before, .. } = j.effect(m).unwrap().unwrap().effect else {
        panic!("not an import");
    };
    assert_eq!(j.kept_bytes(&before[0].hash).unwrap(), b"before");
}

#[test]
fn what_support_sees_carries_no_value_key_path_package_or_identity() {
    let (_d, _p, j) = common::journal();
    let secret_value = "hunter2-wifi-passphrase";
    let s = j
        .settings(&keys().setting)
        .intend(
            &planned(),
            &SettingsIntent::Apply {
                key: "private.key.name".into(),
                before: Some(SettingValue::Text(secret_value.into())),
                value: SettingValue::Text("other-secret".into()),
            },
        )
        .unwrap();
    let r = j
        .imports(&keys().capture)
        .intend(
            &planned(),
            &ImportIntent::Remote {
                identity: "mailbox ada@example.org".into(),
            },
        )
        .unwrap();
    let m = j
        .imports(&keys().capture)
        .intend(&planned(), &import())
        .unwrap();
    for id in [s, r, m] {
        let view = j.effect(id).unwrap().unwrap().support_view();
        let shown = format!("{view:?}");
        for private in [
            secret_value,
            "other-secret",
            "private.key.name",
            "ada@example.org",
            "inbox.mbox",
            "Mail",
            "A.B",
        ] {
            assert!(!shown.contains(private), "{shown}");
        }
        assert_eq!(view.id, id);
        assert_eq!(view.step, "intended");
        assert_eq!(view.receipt, None);
    }
    let (_d2, _p2, k) = common::journal();
    let (i, _, _) = three(&k);
    let view = k.effect(i).unwrap().unwrap().support_view();
    assert_eq!(view.receipt, Some(Receipt::Done));
    assert_eq!(view.class, EffectClass::NativePackageInstall);
    assert_eq!(view.role, Role::Install);
    assert!(!view.after_restart);
}

/// Writes raw bytes as effect `id`'s record (someone changing the journal by hand).
fn tamper(path: &std::path::Path, id: u64, edit: impl Fn(String) -> String) {
    let db = redb::Database::open(path).unwrap();
    let tx = db.begin_write().unwrap();
    {
        let mut t = tx
            .open_table(redb::TableDefinition::<u64, &[u8]>::new("effects"))
            .unwrap();
        // A record is its seal (32 bytes), then its words; the words are changed, the seal kept.
        let stored = t.get(id).unwrap().unwrap().value().to_vec();
        let (seal, json) = stored.split_at(32);
        let new = edit(String::from_utf8(json.to_vec()).unwrap());
        t.insert(id, [seal, new.as_bytes()].concat().as_slice())
            .unwrap();
    }
    tx.commit().unwrap();
}

#[test]
fn a_record_changed_by_hand_never_supplies_a_path_or_a_role() {
    let (_d, path, j) = common::journal();
    let m = j
        .imports(&keys().capture)
        .intend(&planned(), &import())
        .unwrap();
    let s = j
        .settings(&keys().setting)
        .intend(&planned(), &set())
        .unwrap();
    drop(j);
    // A path that climbs out of its folder.
    tamper(&path, m, |r| {
        r.replace("inbox.mbox", "../../Windows/System32/x.dll")
    });
    // An install that claims to be a setting.
    tamper(&path, s, |r| {
        r.replace(r#""role":"settings""#, r#""role":"install""#)
    });
    let j = common::open(&path).unwrap();
    assert!(matches!(j.effect(m), Err(JournalError::Damaged(_))));
    assert!(matches!(j.effect(s), Err(JournalError::Damaged(_))));
    assert!(matches!(j.effects(), Err(JournalError::Damaged(_))));
    assert!(matches!(
        j.installs(&keys().install)
            .observed(s, &Found::Setting(None), true),
        Err(JournalError::Damaged(_))
    ));
}

#[test]
fn a_truncated_journal_is_reported_and_left_exactly_as_it_was() {
    let (_d, path, j) = common::journal();
    three(&j);
    drop(j);
    let bytes = std::fs::read(&path).unwrap();
    std::fs::write(&path, &bytes[..bytes.len() / 3]).unwrap();
    let cut = std::fs::read(&path).unwrap();
    assert!(matches!(
        common::open(&path),
        Err(JournalError::Damaged(_) | JournalError::Storage(_))
    ));
    assert_eq!(
        std::fs::read(&path).unwrap(),
        cut,
        "never repaired by erasing"
    );
}

#[test]
fn a_journal_is_kept_only_on_this_laptops_own_drive() {
    assert_eq!(
        classify(r"C:\Users\ada", FsKind::Local, Some(false)),
        Placement::Local
    );
    assert_eq!(classify("/home/ada", FsKind::Local, None), Placement::Local);
    // A mapped network drive resolves to its share.
    assert_eq!(
        classify(r"\\?\UNC\nas\share\pctwin", FsKind::Local, None),
        Placement::Network
    );
    assert_eq!(
        classify(r"\\nas\share\pctwin", FsKind::Local, Some(false)),
        Placement::Network
    );
    assert_eq!(
        classify("/mnt/nas", FsKind::Network, Some(false)),
        Placement::Network
    );
    assert_eq!(
        classify(r"\\?\C:\x", FsKind::Local, Some(false)),
        Placement::Local
    );
    assert_eq!(
        classify(r"\\.\pipe\x", FsKind::Local, Some(false)),
        Placement::Local
    );
    assert_eq!(
        classify("/media/usb", FsKind::Local, Some(true)),
        Placement::Removable
    );
    assert_eq!(
        classify("/fuse", FsKind::Unknown, Some(false)),
        Placement::Unknown
    );
    assert_eq!(classify("/fuse", FsKind::Unknown, None), Placement::Unknown);
    for p in [Placement::Network, Placement::Removable, Placement::Unknown] {
        let why = p.why_not().unwrap();
        assert!(why.contains("drive"), "{why}");
    }
    assert_eq!(Placement::Local.why_not(), None);
}

#[test]
fn linux_file_systems_are_known_local_network_or_in_doubt() {
    for local in [0xEF53, 0x5846_5342, 0x9123_683E, 0x0102_1994, 0x794C_7630] {
        assert_eq!(linux_kind(local), FsKind::Local, "{local:x}");
    }
    for network in [0x6969, 0x517B, 0xFF53_4D42, 0xFE53_4D42] {
        assert_eq!(linux_kind(network), FsKind::Network, "{network:x}");
    }
    // FUSE (exfat-fuse, ntfs-3g, sshfs): in doubt.
    assert_eq!(linux_kind(0x6573_5546), FsKind::Unknown);
}

#[test]
fn the_volume_holding_a_folder_is_the_longest_mount_it_lies_inside() {
    let sep = if cfg!(windows) { r"\" } else { "/" };
    let root = if cfg!(windows) { r"C:\" } else { "/" };
    let media = format!("{root}media");
    let usb = format!("{media}{sep}usb");
    let volumes = vec![(root.to_string(), false), (usb.clone(), true)];
    assert_eq!(removable_in(&format!("{usb}{sep}x"), &volumes), Some(true));
    assert_eq!(removable_in(&usb, &volumes), Some(true));
    // A name that only starts like the mount is not inside it.
    assert_eq!(
        removable_in(&format!("{usb}2{sep}x"), &volumes),
        Some(false)
    );
    assert_eq!(
        removable_in(&format!("{media}{sep}x"), &volumes),
        Some(false)
    );
    if cfg!(windows) {
        assert_eq!(removable_in(r"\\?\c:\users", &volumes), Some(false));
        assert_eq!(removable_in(r"D:\x", &volumes), None);
    }
}

#[test]
fn a_journal_opens_in_a_temporary_folder_on_this_laptop() {
    let dir = tempfile::tempdir().unwrap();
    assert_eq!(pctwin_journal::place::judge(dir.path()), Placement::Local);
    drop(common::open(&dir.path().join("journal.redb")).unwrap());
    // A folder that does not exist cannot be judged: refused, nothing made.
    let missing = dir.path().join("nowhere").join("journal.redb");
    assert!(matches!(
        common::open(&missing),
        Err(JournalError::UnsafePlace(Placement::Unknown))
    ));
    assert!(!missing.exists());
}

#[test]
fn an_effect_undone_elsewhere_is_never_claimed_undone_here() {
    // The journal accepts only what the outcome honestly is; the executor decides it. Here: a
    // remote effect's undo is recorded as kept with words, and stays final.
    let (_d, _p, j) = common::journal();
    let r = j
        .imports(&keys().capture)
        .intend(
            &planned(),
            &ImportIntent::Remote {
                identity: "upload 1".into(),
            },
        )
        .unwrap();
    j.imports(&keys().capture)
        .observed(r, &Found::External { happened: true }, false)
        .unwrap();
    j.imports(&keys().capture)
        .receipt(r, &Receipt::Done)
        .unwrap();
    let permit = j.begin_undo().unwrap();
    let kept = EffectUndoOutcome::Kept {
        why: "only the service can undo this".into(),
    };
    j.imports(&keys().capture)
        .record_undo(
            &permit,
            r,
            &EffectUndo::Done {
                outcome: kept.clone(),
            },
        )
        .unwrap();
    assert!(kept.needs_action() && kept.is_final());
}

#[test]
fn a_recorded_observation_changed_by_hand_never_supplies_a_path() {
    let (_d, path, j) = common::journal();
    let m = j
        .imports(&keys().capture)
        .intend(&planned(), &import())
        .unwrap();
    j.imports(&keys().capture)
        .observed(
            m,
            &Found::Files(vec![pctwin_journal::FileHash {
                path: "inbox.mbox".into(),
                hash: [1; 32],
                id: None,
            }]),
            false,
        )
        .unwrap();
    drop(j);
    tamper(&path, m, |r| {
        let at = r.rfind("inbox.mbox").unwrap();
        format!(
            "{}{}{}",
            &r[..at],
            "../escape",
            &r[at + "inbox.mbox".len()..]
        )
    });
    let j = common::open(&path).unwrap();
    assert!(matches!(j.effect(m), Err(JournalError::Damaged(_))));
}
