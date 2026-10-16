//! "Nothing forgotten" (Engineering Plan 5.1), on lab laptops built in a temporary folder: every
//! planted thing is found with the right owner and place or listed as a blind spot with the right
//! reason, nothing is invented, the old laptop is unchanged, another person's things are never
//! named, and the top line is never "everything" while a blind spot remains.
//!
//! Here one account plays everyone, so who may open what is simulated: other people's homes are
//! locked against this account, and the answers to "who could open this folder" come from the
//! spec. The same spec and checker run on CI runners with real accounts and real extra drives
//! (fixtures/nothing-forgotten).

use std::collections::BTreeMap;
use std::path::{Path, PathBuf};
use std::sync::atomic::AtomicBool;

use pctwin_lab::plants::{LaptopSpec, Observed, Opens, Role, Roots, check, plan, plant, unlock};
use pctwin_record::LaptopId;
use pctwin_scan::{
    AccessFact, AccessReader, Census, DiskKind, FoundBy, PersonKind, Resident, Rules, SweepFacts,
    SweepLimits, SystemKind, UnixMode, Volume, VolumeKind, access_from_sddl, sweep,
};

fn sid(i: usize) -> String {
    format!("S-1-5-21-9-9-9-{}", 1000 + i)
}

/// "Who could open this folder", answered from the spec.
struct FromSpec {
    spec: LaptopSpec,
    ids: BTreeMap<String, String>,
}

impl AccessReader for FromSpec {
    fn read(&mut self, path: &Path, _unix: Option<UnixMode>) -> AccessFact {
        let name = path.file_name().unwrap().to_string_lossy().into_owned();
        let everyone = || AccessFact::Known(access_from_sddl("O:BAD:(A;;FR;;;BU)").unwrap());
        let Some(p) = self.spec.plants.iter().find(|p| p.top() == name) else {
            return everyone();
        };
        let owner = &self.ids[&p.by];
        let sddl = match p.opens {
            Opens::Everyone => format!("O:{owner}D:(A;;FA;;;{owner})(A;;0x1200a9;;;BU)"),
            Opens::OwnerOnly | Opens::Nobody => {
                format!("O:{owner}D:P(A;;FA;;;BA)(A;;FA;;;SY)(A;;FA;;;{owner})")
            }
        };
        AccessFact::Known(access_from_sddl(&sddl).unwrap())
    }
}

fn du(path: &Path) -> u64 {
    let mut total = 0;
    let mut pending = vec![path.to_path_buf()];
    while let Some(dir) = pending.pop() {
        let Ok(rd) = std::fs::read_dir(&dir) else {
            continue;
        };
        for e in rd.flatten() {
            let Ok(t) = e.file_type() else { continue };
            if t.is_dir() {
                pending.push(e.path());
            } else if t.is_file() {
                total += e.metadata().map(|m| m.len()).unwrap_or(0);
            }
        }
    }
    total
}

/// Refuses this account a folder (another person's home, as the system normally does).
fn refuse(dir: &Path) -> bool {
    pctwin_lab::plants::set_access(dir, Opens::Nobody).is_ok() && std::fs::read_dir(dir).is_err()
}

fn run_one(seed: u64, people: usize) {
    let spec = plan(seed, people);
    let base = tempfile::tempdir().unwrap();
    let c = base.path().join("C");
    let d = base.path().join("D");
    let users = c.join("Users");
    let mut homes = BTreeMap::new();
    let mut ids = BTreeMap::new();
    let mut residents = Vec::new();
    for (i, p) in spec.people.iter().enumerate() {
        let home = users.join(&p.label);
        std::fs::create_dir_all(home.join("Documents")).unwrap();
        homes.insert(p.label.clone(), home.clone());
        ids.insert(p.label.clone(), sid(i));
        residents.push(Resident {
            account_id: Some(sid(i)),
            kind: match p.role {
                Role::Mover => PersonKind::Me,
                Role::Leftover => PersonKind::Leftover,
                Role::Other | Role::Locked => PersonKind::Other,
            },
            suggested_name: p.label.clone(),
            home: Some(home),
            found_by: vec![FoundBy::AccountList],
        });
    }
    std::fs::create_dir_all(&d).unwrap();
    let roots = Roots {
        system_root: c.clone(),
        other_drive: d.clone(),
        homes: homes.clone(),
    };
    plant(&spec, &roots, None).unwrap();
    // The snapshot reads everything, so the folders the mover may not open are opened for it,
    // then closed again.
    let refused: Vec<PathBuf> = spec
        .plants
        .iter()
        .filter(|p| p.opens == Opens::Nobody)
        .map(|p| roots.top_of(p).unwrap())
        .collect();
    for top in &refused {
        unlock(top).unwrap();
    }
    let before = pctwin_lab::snapshot(base.path()).unwrap();
    for top in &refused {
        pctwin_lab::plants::set_access(top, Opens::Nobody).unwrap();
    }

    // Other people's homes are closed to the mover, as the system keeps them.
    let mut closed = Vec::new();
    for p in spec
        .people
        .iter()
        .filter(|p| matches!(p.role, Role::Other | Role::Locked))
    {
        let home = &homes[&p.label];
        if !refuse(home) {
            eprintln!("not reachable here: this account reads every folder (root)");
            return;
        }
        closed.push(home.clone());
    }
    let mover_label = spec.mover().to_string();
    let census = Census {
        me: ids[&mover_label].clone(),
        profiles_root: Some(users.clone()),
        residents,
        account_list_read: true,
        system: SystemKind::this(),
        uid_range: (1000, 60000),
    };
    let mut volumes = Vec::new();
    for (mount, system) in [(&c, true), (&d, false)] {
        let used = du(mount);
        volumes.push(Volume {
            mount: mount.clone(),
            name: String::new(),
            kind: VolumeKind::Internal,
            file_system: "NTFS".into(),
            total_bytes: 1 << 40,
            free_bytes: (1 << 40) - used,
            disk: DiskKind::Ssd,
            is_system: system,
            skip: Vec::new(),
            also: Vec::new(),
        });
    }
    let facts = SweepFacts {
        laptop: LaptopId::from_hex("00112233445566778899aabbccddeeff").unwrap(),
        census,
        volumes,
        known_folders: Vec::new(),
        remembered: Vec::new(),
        rules: Rules::shipped(),
    };
    let mut reader = FromSpec {
        spec: spec.clone(),
        ids,
    };
    let stop = AtomicBool::new(false);
    let result = sweep(
        facts,
        &mut reader,
        &stop,
        &mut |_| {},
        &SweepLimits::default(),
        None,
    );
    let json = serde_json::to_value(&result).unwrap();

    for home in &closed {
        unlock(home).unwrap();
    }
    for p in spec.plants.iter().filter(|p| p.opens == Opens::Nobody) {
        unlock(&roots.top_of(p).unwrap()).unwrap();
    }
    let after = pctwin_lab::snapshot(base.path()).unwrap();
    assert!(
        pctwin_lab::compare(&before, &after).is_empty(),
        "the old laptop changed"
    );
    let exists = |p: &str| PathBuf::from(p).exists() || Path::new(&p.replace('/', "\\")).exists();
    let problems = check(&spec, &roots, &json, &Observed::default(), &exists);
    assert!(
        problems.is_empty(),
        "seed {seed}, {people} people:\n{}\nblind spots: {:#?}",
        problems.join("\n"),
        result.blind_spots
    );
    // With other people present, their closed homes are blind spots, so never "everything".
    if spec.several() {
        assert_ne!(result.top_line, pctwin_scan::TopLine::Everything);
    }
}

#[test]
fn nothing_forgotten_on_a_laptop_with_one_person() {
    run_one(7, 1);
}

#[test]
fn nothing_forgotten_on_a_laptop_with_two_people() {
    run_one(11, 2);
}

#[test]
fn nothing_forgotten_on_a_laptop_with_four_people_and_a_leftover() {
    run_one(23, 4);
}

#[test]
fn nothing_forgotten_on_a_laptop_with_six_people() {
    run_one(31, 6);
}

#[test]
fn the_spec_is_the_same_for_the_same_seed_and_names_are_unique() {
    assert_eq!(plan(5, 3), plan(5, 3));
    let spec = plan(5, 6);
    let mut tokens: Vec<_> = spec.plants.iter().map(|p| p.token.clone()).collect();
    tokens.sort();
    tokens.dedup();
    assert_eq!(tokens.len(), spec.plants.len());
    assert!(spec.plants.len() >= 25, "{}", spec.plants.len());
    assert_eq!(
        spec.people.iter().filter(|p| p.role == Role::Mover).count(),
        1
    );
    assert!(spec.people.iter().any(|p| p.role == Role::Leftover));
    assert!(spec.people.iter().any(|p| p.role == Role::Locked));
}
