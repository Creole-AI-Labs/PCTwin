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
    assert!(matches!(e, JournalError::BadName { .. }), "{e:?}");
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
    let j = pctwin_journal::Journal::open(&path).unwrap();
    let e = j.effects().unwrap_err();
    assert!(matches!(e, JournalError::Damaged { .. }), "{e:?}");
    assert!(!format!("{e} {e:?}").contains("secret"), "{e} {e:?}");
    assert!(!format!("{e}").contains("line"), "no library message: {e}");
}
