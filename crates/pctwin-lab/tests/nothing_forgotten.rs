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
        // On Windows a person who is not an administrator may not even read who can open
        // another person's private folder: the system refuses (UnauthorizedAccessException).
        if cfg!(windows) && p.by != self.spec.mover() && p.opens != Opens::Everyone {
            return AccessFact::Unknown {
                why: "UnauthorizedAccessException".into(),
            };
        }
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

/// One lab laptop, swept: the spec, its roots, the sweep's JSON and the folder holding it all
/// (kept alive while the answer is checked). `None` when this account reads everything (root).
fn lab_run(
    seed: u64,
    people: usize,
) -> Option<(LaptopSpec, Roots, serde_json::Value, tempfile::TempDir)> {
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
        ids: ids.clone(),
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
            return None;
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
        ids: ids.clone(),
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
    // With other people present, their closed homes are blind spots, so never "everything".
    if spec.several() {
        assert_ne!(result.top_line, pctwin_scan::TopLine::Everything);
    }
    Some((spec, roots, json, base))
}

fn exists(p: &str) -> bool {
    PathBuf::from(p).exists() || Path::new(&p.replace('/', "\\")).exists()
}

fn run_one(seed: u64, people: usize) {
    let Some((spec, roots, json, _base)) = lab_run(seed, people) else {
        return;
    };
    let problems = check(&spec, &roots, &json, &Observed::default(), &exists);
    assert!(
        problems.is_empty(),
        "seed {seed}, {people} people:\n{}\nblind spots: {:#}\nunnamed places: {:#}",
        problems.join("\n"),
        json["blind_spots"],
        serde_json::Value::Array(
            json["places"]
                .as_array()
                .unwrap()
                .iter()
                .filter(|p| p["path"].is_null())
                .cloned()
                .collect()
        )
    );
}

/// The checker must catch every way a sweep can fall short: each damaged copy of a good sweep's
/// answer is reported.
#[test]
fn the_answer_key_catches_every_kind_of_wrong_sweep() {
    use pctwin_lab::plants::Expect;
    let Some((spec, roots, good, _base)) = lab_run(23, 4) else {
        return;
    };
    let check_of = |v: &serde_json::Value| check(&spec, &roots, v, &Observed::default(), &exists);
    assert!(check_of(&good).is_empty());
    let token = |want: &dyn Fn(&Expect) -> bool| {
        spec.plants
            .iter()
            .find(|p| want(&p.expect))
            .map(|p| p.token.clone())
            .unwrap()
    };
    let items = |v: &mut serde_json::Value| -> Vec<serde_json::Value> {
        v["record"]["items"].as_array().unwrap().clone()
    };
    let names_token = |item: &serde_json::Value, t: &str| item["path"].to_string().contains(t);

    // 1. A thing of the mover's missing from the record.
    let mover = token(&|e| *e == Expect::Mover);
    let mut bad = good.clone();
    let kept: Vec<_> = items(&mut bad)
        .into_iter()
        .filter(|i| !names_token(i, &mover))
        .collect();
    bad["record"]["items"] = kept.into();
    assert!(
        check_of(&bad).iter().any(|l| l.contains(&mover)),
        "missing item"
    );

    // 2. A shared thing given to the mover.
    let shared = token(&|e| *e == Expect::Shared);
    let mut bad = good.clone();
    let mut list = items(&mut bad);
    for i in &mut list {
        if names_token(i, &shared) {
            i["owner"] = serde_json::json!({"person": {"account_id": good["census"]["me"]}});
        }
    }
    bad["record"]["items"] = list.into();
    assert!(
        check_of(&bad).iter().any(|l| l.contains(&shared)),
        "wrong owner"
    );

    // 3. Another person's thing named anywhere.
    let theirs = token(&|e| matches!(e, Expect::NotNamed { .. }));
    let mut bad = good.clone();
    bad["places"][0]["path"] = serde_json::Value::String(format!("D:/{theirs}"));
    assert!(check_of(&bad).iter().any(|l| l.contains(&theirs)), "named");

    // 4. Another person's thing neither counted nor a blind spot.
    let mut bad = good.clone();
    let places: Vec<_> = bad["places"]
        .as_array()
        .unwrap()
        .iter()
        .filter(|p| !matches!(p["whose"].as_str(), Some("person" | "from-old-account")))
        .cloned()
        .collect();
    bad["places"] = places.into();
    bad["blind_spots"] = serde_json::json!([]);
    bad["top_line"] = serde_json::json!({"line": "everything"});
    assert!(
        check_of(&bad).iter().any(|l| l.contains("neither counted")),
        "forgotten silently"
    );

    // 5. "Everything" while a blind spot remains.
    let mut bad = good.clone();
    bad["top_line"] = serde_json::json!({"line": "everything"});
    assert!(
        check_of(&bad).iter().any(|l| l.contains("top line")),
        "top line"
    );

    // 6. An invented item inside the lab's places.
    let mut bad = good.clone();
    let mut list = items(&mut bad);
    let mut fake = list
        .iter()
        .filter(|i| names_token(i, &mover))
        .max_by_key(|i| i["path"].as_array().map_or(0, Vec::len))
        .cloned()
        .unwrap();
    // Inside the planted folder, where nothing changes between the sweep and the check.
    let mut parts = fake["path"].as_array().unwrap().clone();
    parts.pop();
    parts.push(serde_json::json!({"text": "invented.txt"}));
    fake["path"] = parts.into();
    fake["id"] = serde_json::json!("0123456789abcdef0123456789abcdef");
    list.push(fake);
    bad["record"]["items"] = list.into();
    assert!(
        check_of(&bad).iter().any(|l| l.starts_with("invented")),
        "invented"
    );

    // 7. A refused folder that is not a blind spot.
    let locked = token(&|e| matches!(e, Expect::Unreadable { .. }));
    let mut bad = good.clone();
    let spots: Vec<_> = bad["blind_spots"]
        .as_array()
        .unwrap()
        .iter()
        .filter(|b| b["reason"] != "no-permission")
        .cloned()
        .collect();
    bad["blind_spots"] = spots.into();
    assert!(
        check_of(&bad).iter().any(|l| l.contains(&locked)),
        "refused folder"
    );
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

#[test]
fn the_spec_and_roots_read_back_exactly_as_written() {
    // The CI runners pass the spec between programs as JSON.
    for people in 1..=6 {
        let spec = plan(41, people);
        let json = serde_json::to_vec(&spec).unwrap();
        let back: LaptopSpec = serde_json::from_slice(&json).unwrap();
        assert_eq!(back, spec);
    }
    let roots = Roots {
        system_root: PathBuf::from("C:/"),
        other_drive: PathBuf::from("E:/"),
        homes: BTreeMap::from([("ada".to_string(), PathBuf::from("C:/Users/ada"))]),
        ids: BTreeMap::from([("ada".to_string(), "S-1-5-21-9-9-9-1000".to_string())]),
    };
    let back: Roots = serde_json::from_slice(&serde_json::to_vec(&roots).unwrap()).unwrap();
    assert_eq!(back, roots);
}
