//! Shared helpers for the undo tests: committed writes ready to be undone, and undo stages.
#![allow(dead_code)]

use std::num::NonZeroU64;

use pctwin_journal::{Actor, FileId, Journal, Landed, Permission, PlannedWrite, Undo};
use pctwin_record::{ItemId, LaptopId};

/// How long a test waits on another thread (or process) before calling it a deadlock. It is
/// not a guess at the runner's speed: what is waited for takes well under a second, so a busy
/// runner never comes near it, while a wait that never ends still fails the test.
pub const HANG_AFTER: std::time::Duration = std::time::Duration::from_secs(300);

pub fn planned(n: u8) -> PlannedWrite {
    PlannedWrite {
        download_mark: None,
        item: ItemId::from_hex(&format!("{n:02x}{}", "0".repeat(30))).unwrap(),
        source_laptop: LaptopId::from_hex("00112233445566778899aabbccddeeff").unwrap(),
        scope: Default::default(),
        destination: "me".into(),
        path: format!("Docs/f{n}.txt"),
        size: 10,
        actor: Actor {
            acting_account: "1001".into(),
            for_account: "1001".into(),
            permission: Permission::OwnFolders,
        },
        block_size: 128 * 1024,
        source_modified_ns: None,
        place: None,
        source_file: None,
        partial_keep: Default::default(),
    }
}

/// The identity of committed write `n`'s copy.
pub fn file(n: u8) -> FileId {
    FileId {
        volume: 3,
        index: NonZeroU64::new(100 + u64::from(n)).unwrap(),
        born: None,
    }
}

/// The identity of the folder the copies are in.
pub fn dir() -> FileId {
    FileId {
        volume: 3,
        index: NonZeroU64::new(2).unwrap(),
        born: None,
    }
}

/// A committed write at `Docs/f{n}.txt` (in a folder it made), ready to be undone.
pub fn committed(j: &Journal, n: u8) -> u64 {
    committed_at(j, n, &format!("Docs/f{n}.txt"))
}

/// A committed write at the stored path `at`.
pub fn committed_at(j: &Journal, n: u8, at: &str) -> u64 {
    let id = j.plan(&planned(n)).unwrap();
    j.staged(id, "Docs/.pctwin-t.part", &[("Docs".into(), None)])
        .unwrap();
    j.verified(id, [n; 32], None).unwrap();
    j.applied(id, at).unwrap();
    j.committed(
        id,
        Landed {
            size: 10,
            modified_ns: None,
            file: Some(file(n)),
        },
    )
    .unwrap();
    id
}

pub fn removing(n: u8) -> Undo {
    Undo::Removing {
        file: file(n),
        dir_id: dir(),
        private: format!(".pctwin-undo-{n:032x}"),
    }
}

pub fn putting(n: u8, to: &str) -> Undo {
    Undo::Putting {
        file: file(n),
        dir_id: dir(),
        private: format!(".pctwin-undo-{n:032x}"),
        to: to.into(),
    }
}

pub fn salvaging(n: u8, to: &str) -> Undo {
    Undo::Salvaging {
        file: file(n),
        dir_id: dir(),
        temp: format!(".pctwin-salvage-{n:032x}"),
        to: to.into(),
        complete: false,
    }
}

pub fn journal() -> (tempfile::TempDir, std::path::PathBuf, Journal) {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("journal.redb");
    let j = open(&path).unwrap();
    (dir, path, j)
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
