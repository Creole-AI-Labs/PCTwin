//! `ScanPerson`: the sweep run as the approved person keeps exactly their own files, where they
//! are on the drive, and nothing of the mover's.
// Tests make files to test on.
#![allow(clippy::disallowed_methods)]

use std::path::Path;
use std::sync::atomic::AtomicBool;

use pctwin_helper::person::{census_as, entries_of};
use pctwin_record::LaptopId;
use pctwin_scan::{
    AccessFact, AccessReader, Census, DiskKind, FoundBy, PersonKind, Resident, Rules, SweepFacts,
    SweepLimits, SystemKind, UnixMode, Volume, VolumeKind, access_from_sddl, sweep,
};

const ADA: &str = "S-1-5-21-111-222-333-1001";
const BEA: &str = "S-1-5-21-111-222-333-1002";

struct Everyone;

impl AccessReader for Everyone {
    fn read(&mut self, _path: &Path, _unix: Option<UnixMode>) -> AccessFact {
        AccessFact::Known(access_from_sddl("O:BAD:(A;;FR;;;WD)").unwrap())
    }
}

fn write(path: &Path, bytes: usize) {
    std::fs::create_dir_all(path.parent().unwrap()).unwrap();
    std::fs::write(path, vec![b'x'; bytes]).unwrap();
}

fn resident(id: &str, kind: PersonKind, home: &Path) -> Resident {
    Resident {
        account_id: Some(id.into()),
        kind,
        suggested_name: id.into(),
        home: Some(home.to_path_buf()),
        found_by: vec![FoundBy::AccountList],
    }
}

#[test]
fn the_approved_person_s_own_files_are_found_where_they_are() {
    let dir = tempfile::tempdir().unwrap();
    let c = dir.path().join("C");
    let ada = c.join("Users").join("ada");
    let bea = c.join("Users").join("bea");
    write(&ada.join("Documents/ada.txt"), 3);
    write(&bea.join("Documents/letter.txt"), 5);
    write(&bea.join("Pictures/beach.jpg"), 7);
    let census = Census {
        me: ADA.into(),
        profiles_root: Some(c.join("Users")),
        residents: vec![
            resident(ADA, PersonKind::Me, &ada),
            resident(BEA, PersonKind::Other, &bea),
        ],
        account_list_read: true,
        system: SystemKind::this(),
        uid_range: (1000, 60000),
    };
    // Only a real other person with a home can be scanned as themselves.
    assert!(census_as(census.clone(), "S-1-5-21-9").is_none());
    assert!(census_as(census.clone(), ADA).is_none());
    let as_bea = census_as(census, BEA).unwrap();
    assert_eq!(as_bea.me, BEA);
    let facts = SweepFacts {
        laptop: LaptopId::from_hex("00112233445566778899aabbccddeeff").unwrap(),
        census: as_bea,
        volumes: vec![Volume {
            mount: c.clone(),
            name: String::new(),
            kind: VolumeKind::Internal,
            file_system: "NTFS".into(),
            total_bytes: 1 << 40,
            free_bytes: 1 << 39,
            disk: DiskKind::Ssd,
            is_system: true,
            skip: Vec::new(),
            also: Vec::new(),
        }],
        known_folders: Vec::new(),
        remembered: Vec::new(),
        rules: Rules::shipped(),
    };
    let swept = sweep(
        facts,
        &mut Everyone,
        &AtomicBool::new(false),
        &mut |_| {},
        &SweepLimits::default(),
        None,
    );
    let entries = entries_of(&swept, BEA);
    // Each one is the very file: read whole through the helper's own read.
    for (item, e) in &entries {
        let bytes = pctwin_helper::read::read_range(e, &mut None, 0, e.size as u32).unwrap();
        assert_eq!(bytes.len() as u64, e.size);
        assert_eq!(item.size_bytes, e.size);
        assert_eq!(
            item.owner,
            pctwin_record::Owner::Person {
                account_id: BEA.into()
            }
        );
    }
    let mut found: Vec<(String, u64)> = entries
        .into_iter()
        .map(|(_, e)| {
            (
                e.path
                    .strip_prefix(&bea)
                    .unwrap()
                    .to_string_lossy()
                    .replace('\\', "/"),
                e.size,
            )
        })
        .collect();
    found.sort();
    assert_eq!(
        found,
        [
            ("Documents/letter.txt".to_string(), 5),
            ("Pictures/beach.jpg".to_string(), 7)
        ]
    );
}

/// The helper sweeps the drives once per session, for every approved person together, on the
/// first ScanPerson, and answers every ScanPerson from that one sweep, each person only their own
/// (a family laptop with four people is never swept four times).
#[test]
fn every_approved_person_is_answered_from_one_sweep() {
    use std::collections::{BTreeMap, BTreeSet};
    let sweeps = std::cell::Cell::new(0);
    let approved = BTreeSet::from(["bea".to_string(), "cal".to_string(), "dan".to_string()]);
    let mut once = pctwin_helper::person::ScanOnce::new(approved.clone(), |people| {
        sweeps.set(sweeps.get() + 1);
        assert_eq!(people, &approved, "one sweep for everyone approved");
        people
            .iter()
            .map(|p| {
                let scan = pctwin_helper::person::PersonScan {
                    privacy_blocked: p.len() as u64,
                    ..Default::default()
                };
                (p.clone(), Ok(scan))
            })
            .collect::<BTreeMap<_, _>>()
    });
    assert_eq!(sweeps.get(), 0, "nothing is swept before it is asked for");
    for p in ["cal", "bea", "dan", "bea"] {
        assert_eq!(once.scan(p).unwrap().privacy_blocked, p.len() as u64);
    }
    assert_eq!(sweeps.get(), 1);
    assert!(once.scan("eve").is_err(), "nobody else is answered");
}
