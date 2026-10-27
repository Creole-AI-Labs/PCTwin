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

/// Finding 2: a record of putting a file back that names a path outside its place, or a private
/// name that is not undo's, is damaged (even sealed right).
#[test]
fn a_swapping_record_names_only_undos_own_names() {
    let (_d, path, j) = common::journal();
    let id = an_install(&j);
    drop(j);
    for (step, p, private) in [
        (
            "swapping",
            "../vlcrc",
            ".pctwin-undo-0123456789abcdef0123456789abcdef",
        ),
        ("swapping", "vlcrc", "vlcrc.bak"),
        (
            "put-back",
            "../vlcrc",
            ".pctwin-undo-0123456789abcdef0123456789abcdef",
        ),
        ("put-back", "vlcrc", "vlcrc.bak"),
    ] {
        let json = format!(r#"{{"step":"{step}","path":"{p}","private":"{private}"}}"#);
        forge(&path, "effect-undo", "effect-undo", id, json.as_bytes());
        let j = common::open(&path).unwrap();
        assert!(
            matches!(j.effect_undo_of(id), Err(JournalError::Damaged(_))),
            "{json}"
        );
    }
    let good = r#"{"step":"swapping","path":"vlcrc","private":".pctwin-undo-0123456789abcdef0123456789abcdef"}"#;
    forge(&path, "effect-undo", "effect-undo", id, good.as_bytes());
    let j = common::open(&path).unwrap();
    assert!(matches!(
        j.effect_undo_of(id),
        Ok(Some(pctwin_journal::EffectUndo::Swapping { .. }))
    ));
}

/// Finding 4: on Windows only a disk positively on a bus inside the laptop, and listed as not
/// removable, may hold the journal; a USB hard disk that reports itself fixed, a volume missing
/// from the list, a disk file mounted as a drive, or an unknown bus may not.
#[test]
fn windows_placement_needs_a_disk_positively_inside_the_laptop() {
    use pctwin_journal::Placement::*;
    use pctwin_journal::place::windows_placement as judge;
    let c = r"\\?\C:\Users\someone\AppData\Local\PCTwin";
    for (bus, listed, want) in [
        (Some(17), Some(false), Local),     // NVMe
        (Some(11), Some(false), Local),     // SATA
        (Some(14), Some(false), Local),     // a virtual machine's own disk
        (Some(7), Some(false), Removable),  // a USB hard disk calling itself fixed
        (Some(12), Some(false), Removable), // an SD card
        (Some(17), Some(true), Removable),
        (Some(9), Some(false), Network),  // iSCSI
        (Some(17), None, Unknown),        // not in the volume list
        (None, Some(false), Unknown),     // the bus could not be found
        (Some(15), Some(false), Unknown), // a disk file mounted as a drive
        (Some(0), Some(false), Unknown),
    ] {
        assert_eq!(
            judge(c, bus, listed, Some(false)),
            want,
            "{bus:?} {listed:?}"
        );
    }
    assert_eq!(
        judge(
            r"\\?\UNC\server\share\x",
            Some(17),
            Some(false),
            Some(false)
        ),
        Network
    );
    // Second review, low finding: an NVMe disk in a Thunderbolt or USB4 case reports the NVMe
    // bus but can be unplugged; a disk that will not say is not trusted to stay.
    assert_eq!(judge(c, Some(17), Some(false), Some(true)), Removable);
    assert_eq!(judge(c, Some(11), Some(false), Some(true)), Removable);
    assert_eq!(judge(c, Some(17), Some(false), None), Unknown);
    assert_eq!(judge(c, Some(9), Some(false), Some(true)), Network);
}

/// Second review, low finding, on Linux's device tree: a removable bus device anywhere above the
/// disk makes it removable; the block device's own flag alone is not the whole answer.
#[test]
fn linux_a_disk_behind_a_removable_bus_device_is_removable() {
    use pctwin_journal::place::linux_removable_chain as chain;
    use std::collections::HashMap;
    use std::path::Path;
    let tb = "/sys/devices/pci0000:00/0000:00:07.0/0000:05:00.0/nvme/nvme1/nvme1n1";
    let files = |entries: &[(&str, &str)]| -> HashMap<String, String> {
        entries
            .iter()
            .map(|(k, v)| (k.to_string(), v.to_string()))
            .collect()
    };
    let read_from = |m: HashMap<String, String>| {
        move |p: &Path| m.get(&p.to_string_lossy().replace('\\', "/")).cloned()
    };
    // A Thunderbolt case: the PCI device behind the external port says "removable".
    let r = read_from(files(&[
        (&format!("{tb}/removable"), "0\n"),
        (
            "/sys/devices/pci0000:00/0000:00:07.0/0000:05:00.0/removable",
            "removable\n",
        ),
    ]));
    assert!(chain(Path::new(tb), &r));
    // The laptop's own NVMe disk: "fixed" all the way up.
    let r = read_from(files(&[
        (&format!("{tb}/removable"), "0\n"),
        (
            "/sys/devices/pci0000:00/0000:00:07.0/0000:05:00.0/removable",
            "fixed\n",
        ),
        (
            "/sys/devices/pci0000:00/0000:00:07.0/removable",
            "unknown\n",
        ),
    ]));
    assert!(!chain(Path::new(tb), &r));
    // A card reader's own block flag.
    let r = read_from(files(&[(&format!("{tb}/removable"), "1")]));
    assert!(chain(Path::new(tb), &r));
    // Nothing outside the device tree is read.
    let r = read_from(files(&[("/sys/removable", "1"), ("/removable", "1")]));
    assert!(!chain(Path::new(tb), &r));
    assert!(!chain(Path::new("/tmp/x"), &|_| Some("1".into())));
}

/// Finding 4, on this very laptop: its own temporary folder is judged local through the real
/// bus lookup (Windows), as the journal tests rely on.
#[test]
fn this_laptops_own_folder_is_local() {
    let dir = tempfile::tempdir().unwrap();
    assert_eq!(
        pctwin_journal::place::judge(dir.path()),
        pctwin_journal::Placement::Local
    );
}

/// Second review, finding S1: a record saying a file is settled names only undo's own names, as
/// a record of moving it aside does: refused when written, damaged when forged.
#[test]
fn a_settled_record_names_only_undos_own_names() {
    let (_d, _p, j) = common::journal();
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
    for (p, private) in [
        ("../vlcrc", ".pctwin-undo-0123456789abcdef0123456789abcdef"),
        ("vlcrc", "vlcrc.bak"),
    ] {
        assert!(matches!(
            j.installs(&keys().install).record_undo(
                &permit,
                id,
                &pctwin_journal::EffectUndo::PutBack {
                    path: p.into(),
                    private: private.into()
                }
            ),
            Err(JournalError::BadName)
        ));
    }
}
