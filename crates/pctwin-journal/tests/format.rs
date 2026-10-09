//! The journal's format number: a journal from a newer PCTwin is refused, a format-1 journal
//! (before undo removed files through a held handle) is upgraded only when it holds no undo
//! record, and is otherwise refused rather than misread.

use pctwin_journal::{
    Actor, FileId, Journal, JournalError, Landed, Permission, PlannedWrite, UndoGate,
};
use pctwin_record::{ItemId, LaptopId};
use redb::{ReadableDatabase, TableDefinition};
use std::path::Path;

const META: TableDefinition<&str, u32> = TableDefinition::new("meta");
const UNDO: TableDefinition<u64, &[u8]> = TableDefinition::new("undo");
const UNDO_FOLDERS: TableDefinition<(&str, &str), &[u8]> = TableDefinition::new("undo-folders");

fn planned() -> PlannedWrite {
    PlannedWrite {
        item: ItemId::from_hex(&format!("01{}", "0".repeat(30))).unwrap(),
        source_laptop: LaptopId::from_hex("00112233445566778899aabbccddeeff").unwrap(),
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
        let j = Journal::open(&path).unwrap();
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
        let _closed = j.close_undo().unwrap();
        j.temp_tag(1)
    };
    set_format(&path, 1);
    (path, tag)
}

#[test]
fn a_new_journal_is_written_in_the_current_format_which_is_2() {
    assert_eq!(pctwin_journal::FORMAT, 2);
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("journal.redb");
    drop(Journal::open(&path).unwrap());
    assert_eq!(format_on_disk(&path), Some(2));
    // And it opens again as it is.
    drop(Journal::open(&path).unwrap());
    assert_eq!(format_on_disk(&path), Some(2));
}

#[test]
fn a_format_1_journal_with_no_undo_record_is_upgraded_in_place_keeping_everything() {
    let dir = tempfile::tempdir().unwrap();
    let (path, tag) = format_1_journal(dir.path());
    assert_eq!(format_on_disk(&path), Some(1));
    {
        let j = Journal::open(&path).unwrap();
        assert_eq!(j.temp_tag(1), tag, "the journal keeps its own number");
        assert_eq!(j.entries().unwrap().len(), 1);
        assert_eq!(
            j.undo_gate().unwrap(),
            UndoGate::Closed,
            "the gate stays shut"
        );
        assert!(matches!(j.begin_undo(), Err(JournalError::UndoClosed)));
    }
    assert_eq!(format_on_disk(&path), Some(2));
}

fn assert_refused_and_untouched(path: &Path) {
    for _ in 0..2 {
        match Journal::open(path) {
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
    for newer in [3, 99, u32::MAX] {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("journal.redb");
        drop(Journal::open(&path).unwrap());
        set_format(&path, newer);
        match Journal::open(&path) {
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
    drop(Journal::open(&path).unwrap());
    set_format(&path, 0);
    assert!(matches!(
        Journal::open(&path),
        Err(JournalError::Damaged(_))
    ));
    assert_eq!(format_on_disk(&path), Some(0));
}
