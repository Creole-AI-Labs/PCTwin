//! The fixture factory makes the same synthetic laptop every time, covers every case the file
//! system allows, says why it skipped the rest, and its snapshot sees any change.

use std::collections::BTreeSet;

use pctwin_lab::{Case, Spec, build, compare, snapshot};

fn small() -> Spec {
    Spec {
        seed: 7,
        ordinary_files: 40,
        big_file: 512 * 1024,
    }
}

#[test]
fn every_case_is_made_or_skipped_with_a_reason() {
    let dir = tempfile::tempdir().unwrap();
    let mut made = build(dir.path(), &small()).unwrap();
    let mut seen: BTreeSet<Case> = made.cases();
    seen.extend(made.skipped.iter().map(|s| s.case));
    for case in [
        Case::Ordinary,
        Case::Empty,
        Case::BlockEdge,
        Case::Big,
        Case::UnicodeForms,
        Case::CaseCollision,
        Case::Reserved,
        Case::TrailingDotOrSpace,
        Case::ScriptMix,
        Case::LongPath,
        Case::Link,
        Case::ReadOnly,
        Case::Locked,
        Case::Dotfile,
        Case::NotText,
    ] {
        assert!(seen.contains(&case), "{case:?} neither made nor skipped");
    }
    for s in &made.skipped {
        assert!(!s.why.is_empty(), "{s:?}");
    }
    // These exist on every system PCTwin supports.
    for case in [
        Case::Ordinary,
        Case::Empty,
        Case::BlockEdge,
        Case::Big,
        Case::ScriptMix,
        Case::LongPath,
        Case::ReadOnly,
    ] {
        assert!(
            made.cases().contains(&case),
            "{case:?} was skipped: {:?}",
            made.skipped
        );
    }
    made.release().unwrap();
}

#[test]
fn the_same_seed_makes_the_same_laptop() {
    let (a, b) = (tempfile::tempdir().unwrap(), tempfile::tempdir().unwrap());
    let mut ma = build(a.path(), &small()).unwrap();
    let mut mb = build(b.path(), &small()).unwrap();
    assert_eq!(ma.made, mb.made);
    assert_eq!(snapshot(a.path()).unwrap(), snapshot(b.path()).unwrap());
    let c = tempfile::tempdir().unwrap();
    let mut mc = build(c.path(), &Spec { seed: 8, ..small() }).unwrap();
    assert_ne!(snapshot(a.path()).unwrap(), snapshot(c.path()).unwrap());
    for m in [&mut ma, &mut mb, &mut mc] {
        m.release().unwrap();
    }
}

#[test]
fn a_snapshot_sees_any_change() {
    let dir = tempfile::tempdir().unwrap();
    let mut made = build(dir.path(), &small()).unwrap();
    let before = snapshot(dir.path()).unwrap();
    assert!(compare(&before, &snapshot(dir.path()).unwrap()).is_empty());

    std::fs::write(dir.path().join("Documents/empty.txt"), b"x").unwrap();
    std::fs::write(dir.path().join("Documents/new.txt"), b"x").unwrap();
    std::fs::remove_file(dir.path().join("Documents/edge-exact.bin")).unwrap();
    let diff = compare(&before, &snapshot(dir.path()).unwrap());
    assert_eq!(
        diff,
        [
            "gone: Documents/edge-exact.bin",
            "changed: Documents/empty.txt",
            "added: Documents/new.txt"
        ]
    );
    made.release().unwrap();
}

#[test]
fn links_are_recorded_not_followed() {
    let dir = tempfile::tempdir().unwrap();
    let mut made = build(dir.path(), &small()).unwrap();
    let snap = snapshot(dir.path()).unwrap();
    if made.cases().contains(&Case::Link) {
        assert!(matches!(
            snap.get("Documents/Links/to-outside"),
            Some(pctwin_lab::Entry::Link { .. })
        ));
        // Nothing from outside the profile appears in the snapshot.
        assert!(
            !snap
                .keys()
                .any(|k| k.contains("passwd") || k.contains("kernel32"))
        );
    }
    made.release().unwrap();
}
