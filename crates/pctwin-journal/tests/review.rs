//! Regression tests for the fresh-context security review of 2.2 and 2.3 (26 October 2026).

mod common;
use common::keys;

use pctwin_journal::{
    ImportIntent, JournalError, PlannedEffect, Scope, SettingsFile, SettingsIntent,
};
use pctwin_recipe::{RecipeId, RecipeRef, RelPath, SettingsPlace, SettingsRoot};
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

fn place() -> SettingsPlace {
    SettingsPlace {
        root: SettingsRoot::Config,
        path: RelPath::new("vlc").unwrap(),
    }
}

/// Finding 5: an error never carries a path or an identity, nor a library's message quoting a
/// record; only fixed words.
#[test]
fn errors_carry_no_private_words() {
    let (_d, path, j) = common::journal();
    let secret_path = "../Users/someone/secret-folder/vlcrc";
    let e = j
        .settings(&keys().setting)
        .intend(
            &planned(),
            &SettingsIntent::Copy {
                from: place(),
                to: place(),
                before: vec![SettingsFile {
                    path: secret_path.into(),
                    bytes: b"x".to_vec(),
                }],
            },
        )
        .unwrap_err();
    assert!(matches!(e, JournalError::BadName), "{e:?}");
    assert!(!format!("{e} {e:?}").contains("secret"), "{e} {e:?}");
    let e = j
        .imports(&keys().capture)
        .intend(
            &planned(),
            &ImportIntent::Remote {
                identity: "someone@example.com\u{202e}".into(),
            },
        )
        .unwrap_err();
    assert!(!format!("{e} {e:?}").contains("someone"), "{e} {e:?}");
    // A record that does not read: the error does not quote it.
    drop(j);
    {
        let db = redb::Database::open(&path).unwrap();
        let tx = db.begin_write().unwrap();
        {
            let mut t = tx
                .open_table(redb::TableDefinition::<u64, &[u8]>::new("effects"))
                .unwrap();
            t.insert(1, b"{\"secret-folder\": 1".as_slice()).unwrap();
        }
        tx.commit().unwrap();
    }
    let j = common::open(&path).unwrap();
    let e = j.effects().unwrap_err();
    assert!(matches!(e, JournalError::Damaged(_)), "{e:?}");
    assert!(!format!("{e} {e:?}").contains("secret"), "{e} {e:?}");
    assert!(!format!("{e}").contains("line"), "no library message: {e}");
}

/// Seals `json` as record `id` of `table` with the test key, as PCTwin would: a forger who knows
/// the key (these tests do) still cannot get past the checks behind the seal.
fn forge(path: &std::path::Path, table: &str, kind: &str, id: u64, json: &[u8]) {
    let mut h = blake3::Hasher::new_keyed(&[0x5a; 32]);
    h.update(&(kind.len() as u64).to_be_bytes());
    h.update(kind.as_bytes());
    h.update(&id.to_be_bytes());
    h.update(json);
    let bytes = [h.finalize().as_bytes().as_slice(), json].concat();
    let db = redb::Database::open(path).unwrap();
    let tx = db.begin_write().unwrap();
    {
        let mut t = tx
            .open_table(redb::TableDefinition::<u64, &[u8]>::new(table))
            .unwrap();
        t.insert(id, bytes.as_slice()).unwrap();
    }
    tx.commit().unwrap();
}

fn read(path: &std::path::Path, table: &str, id: u64) -> Vec<u8> {
    let db = redb::Database::open(path).unwrap();
    let tx = redb::ReadableDatabase::begin_read(&db).unwrap();
    let t = tx
        .open_table(redb::TableDefinition::<u64, &[u8]>::new(table))
        .unwrap();
    redb::ReadableTable::get(&t, id)
        .unwrap()
        .unwrap()
        .value()
        .to_vec()
}

fn an_install(j: &pctwin_journal::Journal) -> u64 {
    j.installs(&keys().install)
        .intend(
            &planned(),
            &pctwin_journal::InstallIntent::App {
                catalog: pctwin_recipe::Catalog::Winget,
                package: "VideoLAN.VLC".into(),
                before: pctwin_journal::Presence::Absent,
                undo: Default::default(),
            },
        )
        .unwrap()
}

/// Finding 3: behind a right seal, a record must still be filed under its own number and name
/// only what a checked recipe could (a package, a setting).
#[test]
fn a_sealed_record_must_still_be_its_own_and_plain() {
    let (_d, path, j) = common::journal();
    let id = an_install(&j);
    drop(j);
    let json = read(&path, "effects", id)[32..].to_vec();
    let text = String::from_utf8(json.clone()).unwrap();
    let cases = [
        // Filed under another number.
        (id + 1, json.clone()),
        // A package no recipe could hold.
        (id, text.replace("VideoLAN.VLC", "../evil app").into_bytes()),
    ];
    for (at, bytes) in cases {
        forge(&path, "effects", "effects", at, &bytes);
        let j = common::open(&path).unwrap();
        assert!(
            matches!(j.effects(), Err(JournalError::Damaged(_))),
            "{}",
            String::from_utf8_lossy(&bytes)
        );
        drop(j);
        forge(&path, "effects", "effects", id, &json);
        if at != id {
            let db = redb::Database::open(&path).unwrap();
            let tx = db.begin_write().unwrap();
            tx.open_table(redb::TableDefinition::<u64, &[u8]>::new("effects"))
                .unwrap()
                .remove(at)
                .unwrap();
            tx.commit().unwrap();
        }
    }
    // A setting's name no recipe could hold.
    let (_d2, path, j) = common::journal();
    let id = j
        .settings(&keys().setting)
        .intend(
            &planned(),
            &SettingsIntent::Apply {
                key: "dark-mode".into(),
                before: None,
                value: pctwin_recipe::SettingValue::Bool(true),
            },
        )
        .unwrap();
    drop(j);
    let text = String::from_utf8(read(&path, "effects", id)[32..].to_vec()).unwrap();
    forge(
        &path,
        "effects",
        "effects",
        id,
        text.replace("dark-mode", "-dark mode").as_bytes(),
    );
    let j = common::open(&path).unwrap();
    assert!(matches!(j.effects(), Err(JournalError::Damaged(_))));
}

/// Finding 3: a record of undoing an effect whose seal is wrong counts as part way, so closing
/// undo (and the old laptop's wipe) waits.
#[test]
fn an_undo_record_changed_by_hand_holds_undo_open() {
    let (_d, path, j) = common::journal();
    let id = an_install(&j);
    j.installs(&keys().install)
        .observed(
            id,
            &pctwin_journal::Found::App(pctwin_journal::Presence::Absent),
            false,
        )
        .unwrap();
    j.installs(&keys().install)
        .receipt(id, &pctwin_journal::Receipt::Done)
        .unwrap();
    let permit = j.begin_undo().unwrap();
    j.installs(&keys().install)
        .record_undo(
            &permit,
            id,
            &pctwin_journal::EffectUndo::Done {
                outcome: pctwin_journal::EffectUndoOutcome::AsBefore,
            },
        )
        .unwrap();
    drop(permit);
    assert!(j.effects_part_way().unwrap().is_empty());
    drop(j);
    let mut bytes = read(&path, "effect-undo", id);
    bytes[0] ^= 1;
    let db = redb::Database::open(&path).unwrap();
    let tx = db.begin_write().unwrap();
    tx.open_table(redb::TableDefinition::<u64, &[u8]>::new("effect-undo"))
        .unwrap()
        .insert(id, bytes.as_slice())
        .unwrap();
    tx.commit().unwrap();
    drop(db);
    let j = common::open(&path).unwrap();
    assert_eq!(j.effects_part_way().unwrap(), vec![id]);
    assert!(matches!(
        j.effect_undo_of(id),
        Err(JournalError::Damaged(_))
    ));
}
