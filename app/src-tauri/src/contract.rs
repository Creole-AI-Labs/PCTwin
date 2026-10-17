//! The example files the screens are tested against (app/src/contract/*.json): one value of every
//! shape that crosses the bridge, as the engine really serialises it. The screens' own test
//! (contract.test.ts) parses each one with the same functions bridge.ts uses, so a change on
//! either side that the other does not follow fails a test.
//!
//! Each enum that crosses the bridge is declared with the names of all its variants
//! (`pctwin_types::listed_enum!`), and a test below requires every one of them in the examples, each
//! with its own tag on the wire. A new variant is in that list the moment it exists, so it cannot be
//! left out of the files.
//!
//! The files must match: `cargo test -p pctwin-app contract` fails when they differ, and
//! `PCTWIN_WRITE_CONTRACT=1 cargo test -p pctwin-app contract` writes them.

use std::collections::BTreeSet;
use std::path::PathBuf;

use pctwin_discovery::Label;
use pctwin_gate::{CheckedFrom, WhyMemory};
use pctwin_journey::Problem;
use serde_json::Value;

use crate::pairing::{LabelView, NearbyView, NewView, OldView, ProblemView};

fn picked() -> LabelView {
    // Colour 4 and animal 0 are "blue" and "ant".
    LabelView::from(&Label::Picked {
        colour: 4,
        animal: 0,
    })
}

fn named() -> LabelView {
    LabelView::from(&Label::named("Kitchen laptop").unwrap())
}

fn labels() -> Vec<LabelView> {
    vec![picked(), named()]
}

fn choose_network() -> ProblemView {
    Problem::ChooseNetwork(vec!["Ethernet".into(), "Wi-Fi".into()]).into()
}

fn old_views() -> Vec<OldView> {
    vec![
        OldView::Label { label: picked() },
        OldView::Code {
            code: "123456".into(),
            seconds_left: 42,
        },
        OldView::Wait { seconds_left: 30 },
        OldView::Locked,
        OldView::Choose {
            round: 3,
            numbers: [11, 42, 87],
        },
        OldView::Paired,
        OldView::TimedOut,
        OldView::Left,
        OldView::Lost,
        OldView::Label { label: named() },
    ]
}

fn new_views() -> Vec<NewView> {
    vec![
        NewView::Paired,
        NewView::Problem {
            problem: Problem::WrongCode.into(),
        },
        NewView::Lost,
        NewView::Problem {
            problem: choose_network(),
        },
    ]
}

/// Every problem the journey has (`Problem::all()`, which its test checks against the variants
/// made with the enum), as a rejected command carries it; choosing a network comes with networks
/// to choose from.
fn problems() -> Vec<ProblemView> {
    Problem::all()
        .into_iter()
        .map(|p| match p {
            Problem::ChooseNetwork(_) => choose_network(),
            p => p.into(),
        })
        .collect()
}

fn nearby() -> Vec<NearbyView> {
    vec![
        NearbyView {
            index: 0,
            label: picked(),
            same_label_nearby: false,
        },
        NearbyView {
            index: 1,
            label: named(),
            same_label_nearby: true,
        },
    ]
}

fn json(examples: impl serde::Serialize) -> Value {
    serde_json::to_value(examples).expect("examples serialise")
}

/// Each file's name (without .json) and its examples. bridge.ts has one parse function per name.
fn families() -> Vec<(&'static str, Value)> {
    vec![
        ("label-view", json(labels())),
        ("old-view", json(old_views())),
        ("new-view", json(new_views())),
        ("problem-view", json(problems())),
        ("nearby-view", json(nearby())),
        ("evidence", json(CheckedFrom::ALL)),
    ]
}

fn dir() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../src/contract")
}

/// Pretty JSON with a final line break, the same on every system.
fn text(value: &Value) -> String {
    let mut s = serde_json::to_string_pretty(value).unwrap();
    s.push('\n');
    s
}

#[test]
fn the_example_files_match_the_engine() {
    let write = std::env::var_os("PCTWIN_WRITE_CONTRACT").is_some_and(|v| v == "1");
    let dir = dir();
    let families = families();
    let names: Vec<String> = families.iter().map(|(n, _)| format!("{n}.json")).collect();
    let mut stale = Vec::new();
    for (name, value) in &families {
        let path = dir.join(format!("{name}.json"));
        let want = text(value);
        if write {
            std::fs::write(&path, &want).unwrap();
            continue;
        }
        // A checkout that turned line breaks into CRLF is not a difference.
        let have = std::fs::read_to_string(&path).map(|s| s.replace("\r\n", "\n"));
        if have.ok().as_deref() != Some(want.as_str()) {
            stale.push(format!("{name}.json"));
        }
    }
    // A file for a shape that no longer crosses the bridge is stale too.
    for entry in std::fs::read_dir(&dir).unwrap() {
        let file = entry.unwrap().file_name().to_string_lossy().into_owned();
        if file.ends_with(".json") && !names.contains(&file) {
            if write {
                std::fs::remove_file(dir.join(&file)).unwrap();
            } else {
                stale.push(format!("{file} (no longer made)"));
            }
        }
    }
    assert!(
        stale.is_empty(),
        "the screens' example files in app/src/contract differ from what the engine sends: {stale:?}. \
         If the change is intended, run with PCTWIN_WRITE_CONTRACT=1 to update them, then update \
         bridge.ts so contract.test.ts passes."
    );
}

/// The `kind` tags in a family's examples.
fn tags(examples: Value) -> BTreeSet<String> {
    examples
        .as_array()
        .unwrap()
        .iter()
        .map(|e| e["kind"].as_str().expect("a kind tag").to_owned())
        .collect()
}

/// Each enum's examples hold every one of its variants (named by the enum's own list, made with
/// it), and the wire tags are as many as the variants, so no two variants look alike to the screens.
#[test]
fn every_variant_is_in_the_examples_with_its_own_tag() {
    fn check<T: serde::Serialize>(
        family: &str,
        examples: &[T],
        variant: fn(&T) -> &'static str,
        all: &[&str],
    ) {
        let have: BTreeSet<&str> = examples.iter().map(variant).collect();
        let want: BTreeSet<&str> = all.iter().copied().collect();
        let missing: Vec<_> = want.difference(&have).collect();
        assert!(
            missing.is_empty(),
            "{family}.json has no example of {missing:?}"
        );
        assert_eq!(
            tags(json(examples)).len(),
            all.len(),
            "{family}.json: two variants share a tag"
        );
    }
    check(
        "label-view",
        &labels(),
        LabelView::variant,
        LabelView::VARIANTS,
    );
    check(
        "old-view",
        &old_views(),
        OldView::variant,
        OldView::VARIANTS,
    );
    check(
        "new-view",
        &new_views(),
        NewView::variant,
        NewView::VARIANTS,
    );

    // Problems: one example per problem, each with its own key.
    let keys: BTreeSet<&str> = problems().iter().map(|p| p.key).collect();
    assert_eq!(problems().len(), Problem::VARIANTS.len());
    assert_eq!(
        keys.len(),
        Problem::VARIANTS.len(),
        "two problems share a key"
    );

    // Evidence: every value (the gate's test checks `ALL` against its variants), each distinct.
    let evidence: BTreeSet<String> = CheckedFrom::ALL
        .iter()
        .map(|e| json(e).to_string())
        .collect();
    assert_eq!(evidence.len(), CheckedFrom::ALL.len());
    let reasons: BTreeSet<&str> = CheckedFrom::ALL
        .iter()
        .filter_map(|e| match e {
            CheckedFrom::Memory(why) => Some(why.variant()),
            CheckedFrom::Drive => None,
        })
        .collect();
    assert_eq!(reasons.len(), WhyMemory::VARIANTS.len());
}
