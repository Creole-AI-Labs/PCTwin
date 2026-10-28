//! Nothing on the old laptop can make the plan wait (28 October 2026: a real home's plan blocked
//! on one read). Entries that are not plain local files are judged by their attributes and never
//! opened: a file a cloud app keeps online (here a file marked offline, as a placeholder is), a
//! junction, a pipe. Each is reported "found, not moved" with its reason, and the plan finishes.

mod common;

use std::path::Path;
use std::time::{Duration, Instant};

use common::*;
use pctwin_catalogue::Catalogue;
use pctwin_record::Owner;
use pctwin_special::api::plan;
use pctwin_special::catalogue::{SHIPPED, SpecialRow};
use pctwin_special::roots::System;

fn run(home: &Path) -> pctwin_special::api::Plan {
    let owner = Owner::Person {
        account_id: "1001".into(),
    };
    plan(
        &owner,
        home,
        &[],
        System::Windows,
        System::Windows,
        &Catalogue::<SpecialRow>::shipped(SHIPPED),
    )
}

#[cfg(windows)]
#[test]
fn a_placeholder_and_a_junction_are_reported_never_opened() {
    let t = temp();
    let home = home_of(&t);
    let app = home.join("AppData/Roaming/Cloudy");
    put(&app.join("local.json"), br#"{"a":1}"#);
    let placeholder = app.join("online-only.bin");
    put(&placeholder, b"stand-in bytes of a file kept online");
    // The offline attribute, as cloud placeholders carry it: opening such a file asks its
    // provider to bring it back.
    let ok = std::process::Command::new("attrib")
        .arg("+O")
        .arg(&placeholder)
        .status()
        .unwrap();
    assert!(ok.success());
    // A junction to a folder outside, which must not be followed.
    let outside = t.path().join("outside");
    put(&outside.join("secret.txt"), b"not app data");
    let win = |p: &Path| p.to_string_lossy().replace('/', "\\");
    let ok = std::process::Command::new("cmd")
        .args(["/C", "mklink", "/J"])
        .arg(win(&app.join("jumped")))
        .arg(win(&outside))
        .status()
        .unwrap();
    assert!(ok.success());
    // A placeholder folder too: listing it would ask the provider for its contents.
    let cloud_dir = app.join("cloud-folder");
    std::fs::create_dir_all(&cloud_dir).unwrap();
    put(&cloud_dir.join("inside.txt"), b"x");
    let ok = std::process::Command::new("attrib")
        .arg("+O")
        .arg(&cloud_dir)
        .status()
        .unwrap();
    assert!(ok.success());

    let started = Instant::now();
    let p = run(&home);
    assert!(started.elapsed() < Duration::from_secs(60));
    // One line per reason in the app's folder, with how many.
    let count = |word: &str| {
        p.not_moved
            .iter()
            .find(|n| n.app == "Cloudy" && n.why.contains(word))
            .map(|n| n.count)
            .unwrap_or_else(|| panic!("{word} not reported: {:?}", p.not_moved))
    };
    assert_eq!(count("kept online"), 2, "the file and the folder");
    assert_eq!(count("link"), 1);
    // The detail per entry stays in the folder's own report.
    let detail: Vec<&str> = p.reports[0]
        .not_moved
        .iter()
        .map(|n| n.rel.as_str())
        .collect();
    for rel in ["online-only.bin", "cloud-folder", "jumped"] {
        assert!(detail.contains(&rel), "{rel}: {detail:?}");
    }
    let members: Vec<&String> = p
        .components
        .iter()
        .flat_map(|c| &c.component.members)
        .collect();
    assert!(members.contains(&&"local.json".to_string()));
    assert!(!members.iter().any(|m| m.contains("online-only")
        || m.starts_with("jumped")
        || m.starts_with("cloud-folder")));
    // Still offline: it was never opened (opening would not change the mark here, but nothing
    // that reads it was even attempted: it is in no component).
    let _ = std::process::Command::new("attrib")
        .arg("-O")
        .arg(&cloud_dir)
        .status();
}

#[cfg(unix)]
#[test]
fn a_pipe_is_reported_never_opened() {
    let t = temp();
    let home = home_of(&t);
    let app = home.join("AppData/Roaming/Piped");
    put(&app.join("local.json"), br#"{"a":1}"#);
    // Opening a pipe for reading waits until someone writes: it must never be opened.
    let ok = std::process::Command::new("mkfifo")
        .arg(app.join("fifo"))
        .status()
        .unwrap();
    assert!(ok.success());
    let started = Instant::now();
    let p = run(&home);
    assert!(started.elapsed() < Duration::from_secs(60));
    let n = p
        .not_moved
        .iter()
        .find(|n| n.why.contains("not a file"))
        .unwrap();
    assert_eq!((n.rel.as_str(), n.count), ("fifo", 1));
}
