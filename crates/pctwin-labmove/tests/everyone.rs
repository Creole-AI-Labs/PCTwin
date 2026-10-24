//! The lab's answer-key check for a move of everyone: it passes a run that did everything right,
//! and names each way a run can go wrong (so a broken move can never pass for a good one).
// Tests make files to test on.
#![allow(clippy::disallowed_methods)]

use std::path::Path;

use pctwin_labmove::everyone::{check, planted_bytes};
use serde_json::{Value, json};

struct Lab {
    old: tempfile::TempDir,
    new_home: tempfile::TempDir,
    bea_new: tempfile::TempDir,
}

fn plant(dir: &Path, rel: &str, size: u64) -> String {
    let path = dir.join(rel);
    std::fs::create_dir_all(path.parent().unwrap()).unwrap();
    let name = path.file_name().unwrap().to_string_lossy().into_owned();
    std::fs::write(&path, planted_bytes(&name, size)).unwrap();
    path.to_string_lossy().into_owned()
}

/// A run that did everything right: Ada's and the shared file in her new home, Bea's in her new
/// account, Carl's (not approved) nowhere, and the locked folder a blind spot.
fn good() -> (Lab, Value, Value) {
    let lab = Lab {
        old: tempfile::tempdir().unwrap(),
        new_home: tempfile::tempdir().unwrap(),
        bea_new: tempfile::tempdir().unwrap(),
    };
    let ada = plant(lab.old.path(), "ada/Documents/ada-notes.txt", 100);
    let shared = plant(lab.old.path(), "Public/shared-menu.txt", 40);
    let bea = plant(lab.old.path(), "bea/Documents/bea-secret-diary.txt", 70);
    let carl = plant(lab.old.path(), "carl/Documents/carl-private-tax.txt", 50);
    let locked = plant(lab.old.path(), "srv/locked/locked-thing.txt", 10);
    plant(lab.new_home.path(), "documents/ada-notes.txt", 100);
    plant(lab.new_home.path(), "public/shared-menu.txt", 40);
    plant(lab.bea_new.path(), "Documents/bea-secret-diary.txt", 70);
    let planted = |owner: &str, path: &str, size: u64, expect: &str| {
        json!({
            "owner": owner,
            "path": path,
            "name": Path::new(path).file_name().unwrap().to_string_lossy(),
            "size": size,
            "expect": expect,
        })
    };
    let key = json!({
        "mover": "1000",
        "new_home": lab.new_home.path(),
        "new_homes": { "2001": lab.bea_new.path() },
        "planted": [
            planted("1000", &ada, 100, "moves"),
            planted("shared", &shared, 40, "moves"),
            planted("1001", &bea, 70, "moves"),
            planted("1002", &carl, 50, "stays"),
            planted("1000", &locked, 10, "blind"),
        ],
    });
    let locked_dir = Path::new(&locked)
        .parent()
        .unwrap()
        .to_string_lossy()
        .into_owned();
    let evidence = json!({
        "made": [{ "old_account": "1001", "account": "2001", "secure_token": true }],
        "blind_spots": [{ "path": locked_dir, "reason": "no-permission" }],
        "top_line": { "line": "accounted-for-except", "places": 1 },
        "received": [{ "owner": "1000", "source": ada }],
    });
    (lab, key, evidence)
}

fn visible() -> Vec<(String, String)> {
    vec![(
        "the mover's report".into(),
        r#"{"own":[{"path":"Documents/ada-notes.txt"}],"people":[{"account_id":"1001","copied":1}]}"#
            .into(),
    )]
}

#[test]
fn a_run_that_did_everything_right_passes() {
    let (_lab, key, evidence) = good();
    assert_eq!(check(&key, &evidence, &visible()), Vec::<String>::new());
}

fn fails(key: &Value, evidence: &Value, visible: &[(String, String)], words: &str) {
    let problems = check(key, evidence, visible);
    assert!(
        problems.iter().any(|p| p.contains(words)),
        "expected a problem with {words:?}, got {problems:?}"
    );
}

#[test]
fn a_file_that_did_not_arrive_fails() {
    let (lab, key, evidence) = good();
    std::fs::remove_file(lab.bea_new.path().join("Documents/bea-secret-diary.txt")).unwrap();
    fails(&key, &evidence, &visible(), "did not arrive");
}

#[test]
fn a_file_that_arrived_damaged_fails() {
    let (lab, key, evidence) = good();
    std::fs::write(lab.new_home.path().join("documents/ada-notes.txt"), b"x").unwrap();
    fails(&key, &evidence, &visible(), "did not arrive");
}

#[test]
fn another_person_s_file_in_the_mover_s_place_fails() {
    let (lab, key, evidence) = good();
    plant(lab.new_home.path(), "other/bea-secret-diary.txt", 70);
    fails(&key, &evidence, &visible(), "also arrived");
}

#[test]
fn a_person_not_approved_whose_file_moved_fails() {
    let (lab, key, evidence) = good();
    plant(lab.new_home.path(), "documents/carl-private-tax.txt", 50);
    fails(&key, &evidence, &visible(), "not approved");
}

#[test]
fn another_person_s_name_where_the_mover_can_read_it_fails() {
    let (_lab, key, evidence) = good();
    let mut seen = visible();
    seen.push(("the mover's log".into(), "read bea-secret-diary.txt".into()));
    fails(&key, &evidence, &seen, "is named in the mover's log");
}

#[test]
fn a_changed_original_fails() {
    let (lab, key, evidence) = good();
    std::fs::write(
        lab.old.path().join("ada/Documents/ada-notes.txt"),
        b"edited",
    )
    .unwrap();
    fails(
        &key,
        &evidence,
        &visible(),
        "changed or gone on the old laptop",
    );
}

#[test]
fn a_blind_spot_not_named_or_everything_claimed_fails() {
    let (_lab, key, mut evidence) = good();
    evidence["blind_spots"] = json!([]);
    fails(&key, &evidence, &visible(), "not named as a blind spot");
    let (_lab, key, mut evidence) = good();
    evidence["top_line"] = json!({ "line": "everything" });
    fails(&key, &evidence, &visible(), "everything is accounted for");
}

#[test]
fn an_invented_item_fails() {
    let (_lab, key, mut evidence) = good();
    evidence["received"] = json!([{ "owner": "1000", "source": "/no/such/file.txt" }]);
    fails(&key, &evidence, &visible(), "no such file");
}

/// The lab, never the app, may leave out a drive (a CI runner's work drive, which holds the
/// runner's own build and nothing planted): asked for by name, ignoring how its mount is written.
#[test]
fn the_lab_leaves_out_only_the_drives_it_names() {
    let args: Vec<String> = [
        "--laptop",
        "00112233445566778899aabbccddeeff",
        "--new-home",
        "n",
        "--journal",
        "j",
        "--new-names",
        "x",
        "--leave-out-volume",
        r"D:\",
    ]
    .iter()
    .map(|s| s.to_string())
    .collect();
    let e = pctwin_labmove::everyone::parse(&args).unwrap();
    assert_eq!(e.leave_out, vec![std::path::PathBuf::from(r"D:\")]);
    let without: Vec<String> = args[..8].to_vec();
    assert!(
        pctwin_labmove::everyone::parse(&without)
            .unwrap()
            .leave_out
            .is_empty()
    );
    assert!(pctwin_labmove::everyone::leaves_out(
        &e.leave_out,
        std::path::Path::new(r"D:\")
    ));
    assert!(!pctwin_labmove::everyone::leaves_out(
        &e.leave_out,
        std::path::Path::new(r"C:\")
    ));
}
