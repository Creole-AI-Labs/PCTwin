//! The journal's format number: a journal from a newer PCTwin is refused, a format-2 journal
//! (before the effect journal) is upgraded in place, a format-1 journal (before undo removed files
//! through a held handle) is upgraded only when it holds no undo record, and is otherwise refused
//! rather than misread.

use pctwin_journal::{Actor, FileId, JournalError, Landed, Permission, PlannedWrite, UndoGate};
use pctwin_record::{ItemId, LaptopId};
use redb::{ReadableDatabase, TableDefinition};
use std::path::Path;

const META: TableDefinition<&str, u32> = TableDefinition::new("meta");
const UNDO: TableDefinition<u64, &[u8]> = TableDefinition::new("undo");
const UNDO_FOLDERS: TableDefinition<(&str, &str), &[u8]> = TableDefinition::new("undo-folders");

fn planned() -> PlannedWrite {
    PlannedWrite {
        download_mark: None,
        item: ItemId::from_hex(&format!("01{}", "0".repeat(30))).unwrap(),
        source_laptop: LaptopId::from_hex("00112233445566778899aabbccddeeff").unwrap(),
        scope: Default::default(),
        destination: "me".into(),
        path: "Documents/f1.txt".into(),
        size: 1000,
        actor: Actor {
            acting_account: "1001".into(),
            for_account: "1001".into(),
            permission: Permission::OwnFolders,
        },
        block_size: 128 * 1024,
        source_modified_ns: None,
        source_file: None,
        partial_keep: Default::default(),
        place: None,
    }
}

fn format_on_disk(path: &Path) -> Option<u32> {
    let db = redb::Database::open(path).unwrap();
    let tx = db.begin_read().unwrap();
    let meta = tx.open_table(META).unwrap();
    meta.get("format").unwrap().map(|v| v.value())
}

fn set_format(path: &Path, format: u32) {
    let db = redb::Database::open(path).unwrap();
    let tx = db.begin_write().unwrap();
    tx.open_table(META)
        .unwrap()
        .insert("format", format)
        .unwrap();
    tx.commit().unwrap();
}

/// A journal as a format-1 PCTwin left it: one committed write, undo closed, no undo record.
fn format_1_journal(dir: &Path) -> (std::path::PathBuf, String) {
    let path = dir.join("journal.redb");
    let tag = {
        let j = open(&path).unwrap();
        let id = j.plan(&planned()).unwrap();
        j.staged(
            id,
            "Documents/.pctwin-x.part",
            &[("Documents".into(), None)],
        )
        .unwrap();
        j.verified(id, [1; 32], None).unwrap();
        j.applied(id, "Documents/f1.txt").unwrap();
        j.committed(
            id,
            Landed {
                size: 1000,
                modified_ns: None,
                file: Some(FileId {
                    volume: 1,
                    index: std::num::NonZeroU64::new(2).unwrap(),
                    born: None,
                }),
            },
        )
        .unwrap();
        let _closed = j
            .close_undo(&mut |_, _| Ok(pctwin_journal::Resolved::Removed))
            .unwrap();
        j.temp_tag(1)
    };
    set_format(&path, 1);
    (path, tag)
}

#[test]
fn a_new_journal_is_written_in_the_current_format_which_is_3() {
    assert_eq!(pctwin_journal::FORMAT, 3);
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("journal.redb");
    drop(open(&path).unwrap());
    assert_eq!(format_on_disk(&path), Some(3));
    // And it opens again as it is.
    drop(open(&path).unwrap());
    assert_eq!(format_on_disk(&path), Some(3));
}

#[test]
fn a_format_2_journal_is_upgraded_in_place_keeping_everything_and_its_gate() {
    let dir = tempfile::tempdir().unwrap();
    let (path, tag) = format_1_journal(dir.path());
    set_format(&path, 2);
    {
        let j = open(&path).unwrap();
        assert_eq!(j.temp_tag(1), tag);
        assert_eq!(j.entries().unwrap().len(), 1);
        assert_eq!(j.undo_gate().unwrap(), UndoGate::Closed);
        assert!(j.effects().unwrap().is_empty());
    }
    assert_eq!(format_on_disk(&path), Some(3));
}

#[test]
fn a_format_1_journal_with_no_undo_record_is_upgraded_in_place_keeping_everything() {
    let dir = tempfile::tempdir().unwrap();
    let (path, tag) = format_1_journal(dir.path());
    assert_eq!(format_on_disk(&path), Some(1));
    {
        let j = open(&path).unwrap();
        assert_eq!(j.temp_tag(1), tag, "the journal keeps its own number");
        assert_eq!(j.entries().unwrap().len(), 1);
        assert_eq!(
            j.undo_gate().unwrap(),
            UndoGate::Closed,
            "the gate stays shut"
        );
        assert!(matches!(j.begin_undo(), Err(JournalError::UndoClosed)));
    }
    assert_eq!(format_on_disk(&path), Some(3));
}

fn assert_refused_and_untouched(path: &Path) {
    for _ in 0..2 {
        match open(path) {
            Err(JournalError::OlderFormatWithUndo { found: 1 }) => {}
            other => panic!("expected the format-1 journal refused, got {other:?}"),
        }
        assert_eq!(
            format_on_disk(path),
            Some(1),
            "a refused journal is not changed"
        );
    }
}

#[test]
fn a_format_1_journal_holding_an_undo_record_is_refused_not_misread() {
    let dir = tempfile::tempdir().unwrap();
    let (path, _) = format_1_journal(dir.path());
    {
        // A record only format 1 wrote: the copy set aside.
        let db = redb::Database::open(&path).unwrap();
        let tx = db.begin_write().unwrap();
        tx.open_table(UNDO)
            .unwrap()
            .insert(1, br#"{"step":"aside","to":"x"}"#.as_slice())
            .unwrap();
        tx.commit().unwrap();
    }
    assert_refused_and_untouched(&path);
}

#[test]
fn a_format_1_journal_holding_a_folder_undo_record_is_refused_not_misread() {
    let dir = tempfile::tempdir().unwrap();
    let (path, _) = format_1_journal(dir.path());
    {
        let db = redb::Database::open(&path).unwrap();
        let tx = db.begin_write().unwrap();
        tx.open_table(UNDO_FOLDERS)
            .unwrap()
            .insert(("me", "Documents"), br#"{"result":"removed"}"#.as_slice())
            .unwrap();
        tx.commit().unwrap();
    }
    assert_refused_and_untouched(&path);
}

#[test]
fn a_journal_in_any_newer_format_is_refused_and_left_as_it_is() {
    for newer in [4, 99, u32::MAX] {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("journal.redb");
        drop(open(&path).unwrap());
        set_format(&path, newer);
        match open(&path) {
            Err(JournalError::NewerFormat { found }) => assert_eq!(found, newer),
            other => panic!("expected format {newer} refused, got {other:?}"),
        }
        assert_eq!(format_on_disk(&path), Some(newer));
    }
}

#[test]
fn a_journal_claiming_format_0_is_reported_damaged() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("journal.redb");
    drop(open(&path).unwrap());
    set_format(&path, 0);
    assert!(matches!(open(&path), Err(JournalError::Damaged(_))));
    assert_eq!(format_on_disk(&path), Some(0));
}

/// The saved example of format 2 (one committed write), read by every later build (engineering
/// notes 9.11). Set `PCTWIN_WRITE_FORMAT_FIXTURES=1` to write it, only when a new format number is
/// introduced: the old file must then stay.
#[test]
fn the_saved_format_2_example_still_opens() {
    let saved = Path::new(env!("CARGO_MANIFEST_DIR")).join("../../fixtures/formats/journal-2.redb");
    if std::env::var_os("PCTWIN_WRITE_FORMAT_FIXTURES").is_some() && pctwin_journal::FORMAT == 2 {
        let _ = std::fs::remove_file(&saved);
        let j = open(&saved).unwrap();
        let id = j.plan(&planned()).unwrap();
        j.staged(
            id,
            "Documents/.pctwin-x.part",
            &[("Documents".into(), None)],
        )
        .unwrap();
        j.verified(id, [1; 32], None).unwrap();
        j.applied(id, "Documents/f1.txt").unwrap();
        j.committed(
            id,
            Landed {
                size: 1000,
                modified_ns: None,
                file: None,
            },
        )
        .unwrap();
    }
    // Opened from a copy: opening may write, and the saved example must never change.
    let dir = tempfile::tempdir().unwrap();
    let copy = dir.path().join("journal.redb");
    std::fs::copy(&saved, &copy).unwrap();
    let j = open(&copy).unwrap();
    let entries = j.entries().unwrap();
    assert_eq!(entries.len(), 1);
    assert_eq!(entries[0].write, planned());
    assert!(j.unfinished().unwrap().is_empty());
    drop(j);
    assert_eq!(format_on_disk(&copy), Some(3), "opened, it is upgraded");
}

fn effect() -> (
    pctwin_journal::PlannedEffect,
    pctwin_journal::SettingsIntent,
) {
    use pctwin_recipe::{RecipeId, RecipeRef, RelPath, SettingsPlace, SettingsRoot};
    let place = SettingsPlace {
        root: SettingsRoot::Config,
        path: RelPath::new("vlc").unwrap(),
    };
    (
        pctwin_journal::PlannedEffect {
            scope: pctwin_journal::Scope([7; 16]),
            item: ItemId::from_hex(&format!("aa{}", "0".repeat(30))).unwrap(),
            recipe: RecipeRef {
                id: RecipeId::new("org.videolan.vlc").unwrap(),
                version: 1,
            },
            step: 1,
            actor: planned().actor,
        },
        pctwin_journal::SettingsIntent::Copy {
            from: place.clone(),
            to: place,
            before: vec![pctwin_journal::SettingsFile {
                path: "vlcrc".into(),
                bytes: b"old".to_vec(),
            }],
        },
    )
}

/// The saved example of format 3 (one committed write, one copy of settings with its receipt
/// and the file that was there before, a restore point answer), read by every later build. Set
/// `PCTWIN_WRITE_FORMAT_FIXTURES=1` to write it, only when a new format number is introduced:
/// the old file must then stay.
#[test]
fn the_saved_format_3_example_still_opens() {
    use pctwin_journal::{FileHash, Found, Receipt, RestorePoint, Scope, content_hash};
    let saved = Path::new(env!("CARGO_MANIFEST_DIR")).join("../../fixtures/formats/journal-3.redb");
    let point = RestorePoint::EarlierExists {
        at: 1_790_000_000,
        number: 12,
    };
    let found = Found::Files(vec![FileHash {
        path: "vlcrc".into(),
        hash: content_hash(b"new"),
    }]);
    if std::env::var_os("PCTWIN_WRITE_FORMAT_FIXTURES").is_some() && pctwin_journal::FORMAT == 3 {
        let _ = std::fs::remove_file(&saved);
        let j = open(&saved).unwrap();
        let id = j.plan(&planned()).unwrap();
        j.staged(id, "Documents/.pctwin-x.part", &[]).unwrap();
        j.verified(id, [1; 32], None).unwrap();
        j.applied(id, "Documents/f1.txt").unwrap();
        j.committed(
            id,
            Landed {
                size: 1000,
                modified_ns: None,
                file: None,
            },
        )
        .unwrap();
        j.installs(&keys().install)
            .record_restore_point(Scope([7; 16]), &point)
            .unwrap();
        let (p, intent) = effect();
        let e = j.settings(&keys().setting).intend(&p, &intent).unwrap();
        j.settings(&keys().setting)
            .observed(e, &found, false)
            .unwrap();
        j.settings(&keys().setting)
            .receipt(e, &Receipt::Done)
            .unwrap();
    }
    let dir = tempfile::tempdir().unwrap();
    let copy = dir.path().join("journal.redb");
    std::fs::copy(&saved, &copy).unwrap();
    let j = open(&copy).unwrap();
    assert_eq!(j.entries().unwrap()[0].write, planned());
    assert_eq!(j.restore_point(Scope([7; 16])).unwrap(), Some(point));
    let effects = j.effects().unwrap();
    assert_eq!(effects.len(), 1);
    assert_eq!(effects[0].planned, effect().0);
    assert!(j.unfinished_effects().unwrap().is_empty());
    assert_eq!(
        effects[0].state,
        pctwin_journal::EffectState::Receipted {
            found,
            after_restart: false,
            receipt: Receipt::Done
        }
    );
    assert_eq!(
        j.kept_bytes(&content_hash(b"old")).unwrap(),
        b"old".to_vec()
    );
    drop(j);
    assert_eq!(format_on_disk(&copy), Some(3));
}

/// This test program's role keys: made once (a program gets them only once) and lent to every
/// test.
pub fn keys() -> &'static pctwin_grant::RoleKeys {
    static KEYS: std::sync::OnceLock<pctwin_grant::RoleKeys> = std::sync::OnceLock::new();
    KEYS.get_or_init(|| pctwin_grant::RoleKeys::mint().expect("the keys are made once"))
}

/// Opens a journal with this test program's record key (never the system's key store, so tests
/// leave nothing behind in it).
pub fn open(
    path: &std::path::Path,
) -> Result<pctwin_journal::Journal, pctwin_journal::JournalError> {
    pctwin_journal::Journal::open_with_record_key(path, test_key())
}

/// The record key every test journal is sealed with.
pub fn test_key() -> pctwin_journal::keystore::TagKey {
    pctwin_journal::keystore::TagKey::from_bytes([0x5a; 32])
}
