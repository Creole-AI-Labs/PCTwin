//! The example files the screens are tested against (app/src/contract/*.json): one value of every
//! shape that crosses the bridge, as the engine really serialises it. The screens' own test
//! (contract.test.ts) parses each one with the same functions bridge.ts uses, so a change on
//! either side that the other does not follow fails a test.
//!
//! Each list below is made by walking an exhaustive `match` (no catch-all arm): a new variant does
//! not compile until it is given a place in the walk, and then it is in the files.
//!
//! The files must match: `cargo test -p pctwin-app contract` fails when they differ, and
//! `PCTWIN_WRITE_CONTRACT=1 cargo test -p pctwin-app contract` writes them.

use std::path::PathBuf;

use pctwin_discovery::Label;
use pctwin_gate::CheckedFrom;
use pctwin_journey::Problem;
use serde_json::Value;

use crate::pairing::{LabelView, NearbyView, NewView, OldView, ProblemView};

/// Follows `after` from `first` until it ends: every value the walk reaches, in order.
fn walk<T>(first: T, after: impl Fn(&T) -> Option<T>) -> Vec<T> {
    let mut all = vec![first];
    while let Some(next) = after(all.last().unwrap()) {
        all.push(next);
    }
    all
}

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
    walk(picked(), |v| match v {
        LabelView::Picked { .. } => Some(named()),
        LabelView::Named { .. } => None,
    })
}

fn choose_network() -> ProblemView {
    Problem::ChooseNetwork(vec!["Ethernet".into(), "Wi-Fi".into()]).into()
}

fn old_views() -> Vec<OldView> {
    let mut all = walk(OldView::Label { label: picked() }, |v| match v {
        OldView::Label { .. } => Some(OldView::Code {
            code: "123456".into(),
            seconds_left: 42,
        }),
        OldView::Code { .. } => Some(OldView::Wait { seconds_left: 30 }),
        OldView::Wait { .. } => Some(OldView::Locked),
        OldView::Locked => Some(OldView::Choose {
            round: 3,
            numbers: [11, 42, 87],
        }),
        OldView::Choose { .. } => Some(OldView::Paired),
        OldView::Paired => Some(OldView::TimedOut),
        OldView::TimedOut => Some(OldView::Left),
        OldView::Left => Some(OldView::Lost),
        OldView::Lost => None,
    });
    all.push(OldView::Label { label: named() });
    all
}

fn new_views() -> Vec<NewView> {
    walk(NewView::Paired, |v| match v {
        NewView::Paired => Some(NewView::Problem {
            problem: Problem::WrongCode.into(),
        }),
        NewView::Problem { .. } => Some(NewView::Lost),
        NewView::Lost => None,
    })
    .into_iter()
    .chain([NewView::Problem {
        problem: choose_network(),
    }])
    .collect()
}

/// Every problem the journey has (its own list, which its tests keep whole), as a rejected command
/// carries it; choosing a network comes with networks to choose from.
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

#[test]
fn every_view_variant_is_in_the_examples() {
    let kinds = |v: Value| -> Vec<String> {
        v.as_array()
            .unwrap()
            .iter()
            .map(|e| e["kind"].as_str().unwrap().to_owned())
            .collect()
    };
    let old = kinds(json(old_views()));
    for k in [
        "label",
        "code",
        "wait",
        "locked",
        "choose",
        "paired",
        "timed-out",
        "left",
        "lost",
    ] {
        assert!(old.contains(&k.to_owned()), "old-view has no {k}");
    }
    let new = kinds(json(new_views()));
    for k in ["paired", "problem", "lost"] {
        assert!(new.contains(&k.to_owned()), "new-view has no {k}");
    }
    assert_eq!(problems().len(), Problem::all().len());
    assert_eq!(CheckedFrom::ALL.len(), 6);
}
