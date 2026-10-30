//! The forgotten-things finder: the places a catalogue names are found and counted by the file
//! system's facts alone, links are never followed, places that are not there are not said, and a
//! new kind of place is one catalogue row with no code change. Every byte here is invented.

mod common;

use std::path::{Path, PathBuf};
use std::time::{Duration, Instant};

use common::*;
use pctwin_catalogue::Catalogue;
use pctwin_special::forgotten::{
    MAX_PRINTERS, PlaceRow, PlaceStep, SHIPPED, Under, find_places, printers,
};
use pctwin_special::roots::{System, roots_of};

fn rows(text: &str) -> Catalogue<PlaceRow> {
    Catalogue::shipped(&format!(
        r#"{{"format": 1, "catalogue": "special-places", "version": 1, "rows": [{text}]}}"#
    ))
}

fn soon() -> Instant {
    Instant::now() + Duration::from_secs(30)
}

#[test]
fn the_shipped_places_read_whole() {
    let c = Catalogue::<PlaceRow>::shipped(SHIPPED);
    assert!(c.report().is_clean(), "{:?}", c.report());
    assert!(c.rows().len() >= 10);
    // Every system has places of its own.
    for s in [System::Windows, System::Mac, System::Linux] {
        assert!(c.rows().iter().any(|r| r.system.covers(s)), "{s:?}");
    }
}

#[test]
fn known_places_are_found_counted_and_said_with_their_step() {
    let t = temp();
    let home = home_of(&t);
    let docs = t.path().join("elsewhere/Documents");
    let fonts = home.join("AppData/Local/Microsoft/Windows/Fonts");
    put(&fonts.join("Synthetic Sans.ttf"), &[0, 1, 0, 0, 9, 9]);
    put(&fonts.join("Synthetic Serif.otf"), b"OTTO1234");
    put(&docs.join("My Games/Quest/save1.dat"), b"slot");
    put(
        &docs.join("Outlook Files/archive.pst"),
        b"!BDN\0\0\0\0SM\x17\0",
    );
    // Present but empty: nothing to say.
    std::fs::create_dir_all(home.join("AppData/Roaming/Microsoft/UProof")).unwrap();
    let programs = t.path().join("Programs x86");
    put(
        &programs.join("Steam/steamapps/common/Quest/quest.bin"),
        &[7; 100],
    );
    let roots = roots_of(System::Windows, &home, &|_| None);
    let under = |u: Under| match u {
        Under::Home => Some(home.clone()),
        Under::Documents => Some(docs.clone()),
        Under::ProgramFilesX86 => Some(programs.clone()),
        _ => None,
    };
    // The plan takes the documents, not the fonts' folder.
    let moving = |p: &Path| p.starts_with(&docs);
    let c = Catalogue::<PlaceRow>::shipped(SHIPPED);
    let f = find_places(System::Windows, &home, &roots, &under, &c, &moving, soon());
    let by = |kind: &str| {
        f.found
            .iter()
            .find(|x| x.kind == kind)
            .unwrap_or_else(|| panic!("{kind} not found: {:?}", f.found))
    };
    let fonts_found = by("fonts");
    assert_eq!(fonts_found.files, 2);
    assert_eq!(fonts_found.bytes, 14);
    assert!(fonts_found.complete);
    assert!(!fonts_found.moving);
    assert_eq!(fonts_found.step, PlaceStep::Install);
    assert_eq!(fonts_found.place, "AppData/Local/Microsoft/Windows/Fonts");
    let saves = by("game-saves");
    assert!(saves.moving);
    assert_eq!(saves.step, PlaceStep::Copy);
    // A mail archive copies with the documents, and the person opens it in the mail app.
    let archives = by("mail-archives");
    assert!(archives.moving);
    assert_eq!(archives.step, PlaceStep::OpenInMailApp);
    let library = by("game-library");
    assert_eq!(library.step, PlaceStep::LocateVerify);
    assert_eq!(library.bytes, 100);
    assert!(!library.moving);
    // An empty place and places that are not on this laptop are not said.
    assert!(f.found.iter().all(|x| x.kind != "dictionaries"));
    assert!(f.found.iter().all(|x| x.kind != "mail-signatures"));
    assert!(f.not_checked.is_empty(), "{:?}", f.not_checked);
    // Another system's places are never looked for here.
    let mac = find_places(System::Mac, &home, &roots, &under, &c, &moving, soon());
    assert!(mac.found.iter().all(|x| x.kind != "game-library"));
}

#[test]
fn a_new_kind_of_place_is_one_row_and_links_are_never_followed() {
    let t = temp();
    let home = home_of(&t);
    let presets = home.join("Studio Presets");
    put(&presets.join("warm.preset"), b"x");
    let roots = roots_of(System::Windows, &home, &|_| None);
    let under = |u: Under| (u == Under::Home).then(|| home.clone());
    let one = rows(
        r#"{"system":"any","under":"home","path":"Studio Presets","kind":"sound-presets","name":"Sound presets","step":"copy","checked_on":"2026-10-29"}"#,
    );
    let f = find_places(
        System::Linux,
        &home,
        &roots,
        &under,
        &one,
        &|_| false,
        soon(),
    );
    assert_eq!(f.found.len(), 1);
    assert_eq!(f.found[0].kind, "sound-presets");
    assert_eq!(f.found[0].name, "Sound presets");
    assert_eq!(f.found[0].files, 1);

    // A link inside is not followed, and the count says it is not whole.
    let outside = t.path().join("outside");
    put(&outside.join("big.bin"), &[1; 1000]);
    if make_link(&outside, &presets.join("more")) {
        let f = find_places(
            System::Linux,
            &home,
            &roots,
            &under,
            &one,
            &|_| false,
            soon(),
        );
        assert_eq!(f.found[0].bytes, 1);
        assert!(!f.found[0].complete);
        // A place that is itself a link is said, never followed.
        let linked = rows(
            r#"{"system":"any","under":"home","path":"Studio Presets/more","kind":"sound-presets","name":"Sound presets","step":"copy","checked_on":"2026-10-29"}"#,
        );
        let f = find_places(
            System::Linux,
            &home,
            &roots,
            &under,
            &linked,
            &|_| false,
            soon(),
        );
        assert!(f.found.is_empty());
        assert_eq!(f.not_checked.len(), 1);
    }

    // Out of time: counted as far as it got, and said not whole (a place with no link in it).
    put(&home.join("Plain Presets/a.preset"), b"x");
    let plain = rows(
        r#"{"system":"any","under":"home","path":"Plain Presets","kind":"sound-presets","name":"Sound presets","step":"copy","checked_on":"2026-10-29"}"#,
    );
    let whole = find_places(
        System::Linux,
        &home,
        &roots,
        &under,
        &plain,
        &|_| false,
        soon(),
    );
    assert!(whole.found[0].complete);
    let f = find_places(
        System::Linux,
        &home,
        &roots,
        &under,
        &plain,
        &|_| false,
        Instant::now(),
    );
    assert_eq!(f.found.len(), 1);
    assert!(!f.found[0].complete);
    assert_eq!(f.found[0].files, 0);
}

/// A folder link (a junction needs no rights on Windows; a symlink elsewhere).
fn make_link(target: &Path, link: &PathBuf) -> bool {
    #[cfg(windows)]
    {
        std::process::Command::new("cmd")
            .args(["/C", "mklink", "/J"])
            .arg(link)
            .arg(target)
            .output()
            .is_ok_and(|o| o.status.success())
    }
    #[cfg(unix)]
    {
        std::os::unix::fs::symlink(target, link).is_ok()
    }
}

#[test]
fn bad_place_rows_are_skipped_each_for_its_own_fault() {
    let bad = [
        r#"{"system":"any","path":"x","kind":"k","name":"n","step":"copy","checked_on":"2026-10-29"}"#,
        r#"{"system":"any","under":"home","root":"roaming","path":"x","kind":"k","name":"n","step":"copy","checked_on":"2026-10-29"}"#,
        r#"{"system":"any","root":"system-cache","path":"x","kind":"k","name":"n","step":"copy","checked_on":"2026-10-29"}"#,
        r#"{"system":"any","under":"home","path":"../x","kind":"k","name":"n","step":"copy","checked_on":"2026-10-29"}"#,
        r#"{"system":"any","under":"home","path":"a/b/c/d/e/f/g","kind":"k","name":"n","step":"copy","checked_on":"2026-10-29"}"#,
        r#"{"system":"any","under":"home","path":"x","kind":"Bad Kind","name":"n","step":"copy","checked_on":"2026-10-29"}"#,
        r#"{"system":"any","under":"home","path":"x","kind":"k","name":"","step":"copy","checked_on":"2026-10-29"}"#,
        r#"{"system":"any","under":"home","path":"x","kind":"k","name":"n","step":"add-again","checked_on":"2026-10-29"}"#,
        r#"{"system":"any","under":"home","path":"x","kind":"k","name":"n","step":"copy","page":"http://x","checked_on":"2026-10-29"}"#,
        r#"{"system":"any","under":"home","path":"x","kind":"k","name":"n","step":"copy","checked_on":"29 October"}"#,
    ];
    for (i, b) in bad.iter().enumerate() {
        let c = rows(b);
        assert!(c.rows().is_empty(), "row {i} was taken");
        assert_eq!(c.report().skipped.len(), 1, "row {i}");
    }
    // At their edges.
    let row = |kind: &str, name: &str| {
        rows(&format!(
            r#"{{"system":"any","under":"home","path":"x","kind":"{kind}","name":"{name}","step":"copy","checked_on":"2026-10-29"}}"#
        ))
        .rows()
        .len()
    };
    assert_eq!(row(&"k".repeat(40), "n"), 1);
    assert_eq!(row(&"k".repeat(41), "n"), 0);
    assert_eq!(row("k", &"n".repeat(100)), 1);
    assert_eq!(row("k", &"n".repeat(101)), 0);
    let ok = rows(
        r#"{"system":"any","under":"home","path":"a/b/c/d/e/f","kind":"k-1","name":"n","step":"copy","page":"https://example.org/x","checked_on":"2026-10-29"}"#,
    );
    assert_eq!(ok.rows().len(), 1, "{:?}", ok.report());
}

#[test]
fn printers_are_names_only_and_bounded() {
    // Whatever this laptop has: names only, each once, within the bound.
    if let Ok(list) = printers() {
        assert!(list.len() <= MAX_PRINTERS);
        let mut names: Vec<String> = list.iter().map(|p| p.name.to_lowercase()).collect();
        let n = names.len();
        names.dedup();
        assert_eq!(names.len(), n);
        assert!(list.iter().all(|p| !p.name.trim().is_empty()));
    }
}

/// A file kept online only is counted apart and never read (Windows marks it offline).
#[cfg(windows)]
#[test]
fn files_kept_online_only_are_counted_apart_never_read() {
    let t = temp();
    let home = home_of(&t);
    let presets = home.join("Studio Presets");
    put(&presets.join("here.preset"), b"x");
    let away = presets.join("away.preset");
    put(&away, b"yy");
    let marked = std::process::Command::new("attrib")
        .arg("+O")
        .arg(&away)
        .output()
        .is_ok_and(|o| o.status.success());
    assert!(marked, "attrib could not mark the file");
    let roots = roots_of(System::Windows, &home, &|_| None);
    let under = |u: Under| (u == Under::Home).then(|| home.clone());
    let one = rows(
        r#"{"system":"any","under":"home","path":"Studio Presets","kind":"sound-presets","name":"Sound presets","step":"copy","checked_on":"2026-10-29"}"#,
    );
    let f = find_places(
        System::Windows,
        &home,
        &roots,
        &under,
        &one,
        &|_| false,
        soon(),
    );
    assert_eq!(f.found[0].files, 1);
    assert_eq!(f.found[0].bytes, 1);
    assert_eq!(f.found[0].online_only, 1);
    let _ = std::process::Command::new("attrib")
        .arg("-O")
        .arg(&away)
        .output();
}
