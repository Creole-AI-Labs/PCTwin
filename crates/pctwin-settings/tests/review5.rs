//! The fifth review's findings 1 to 4 and the mutants that survived its campaign, as tests of
//! the SAFE outcome: one judging pipeline ([`pctwin_settings::judge`]) for every value of every
//! kind, before planning and again at the moment of writing. The settings, paths and addresses
//! here are made up.

use pctwin_recipe::SettingValue;
use pctwin_settings::catalogue::{Catalogue, System};
use pctwin_settings::classify::{Ctx, Fate, FolderMap, Why, classify_as, classify_kind, danger};
use pctwin_settings::found::{Address, Discovery, FoundSetting, PrefKind, RegKind};
use pctwin_settings::generic::{may_write_with, plan_found};
use pctwin_settings::judge::{Facts, Kind, runs, texts};

const STORE: &str = r"Software\Acme\Viewer";

/// A system whose own facts say `.exe` runs and `calculator:` and `ms-settings:` open a
/// handler: the same everywhere the tests run.
fn facts() -> Facts {
    Facts::new(
        ["exe", "com", "scr"],
        Some(vec!["calculator", "ms-settings", "https"]),
    )
}

fn ctx<'a>(
    map: &'a FolderMap,
    hints: &'a [pctwin_settings::catalogue::ActionHint],
    f: &'a Facts,
) -> Ctx<'a> {
    Ctx {
        paths: map,
        same_screens: false,
        hints,
        facts: f,
    }
}

fn map() -> FolderMap {
    FolderMap(vec![(r"C:\Users\old".into(), r"C:\Users\new".into())])
}

fn reg(name: &str, kind: RegKind) -> Address {
    Address::Registry {
        key: STORE.into(),
        name: name.into(),
        kind,
    }
}

fn text(t: &str) -> SettingValue {
    SettingValue::Text(t.into())
}

fn utf16_hex(s: &str, odd: bool) -> String {
    let mut b: Vec<u8> = if odd { vec![0x07] } else { vec![] };
    b.extend(s.encode_utf16().flat_map(u16::to_le_bytes));
    b.extend([0, 0]);
    b.iter().map(|x| format!("{x:02x}")).collect()
}

/// Finding 1: a program's path in any of Windows' spellings of the same file is kept back, and
/// the write check refuses it, with no catalogue rows at all.
#[test]
fn f1_a_program_path_in_any_spelling_is_kept_back_with_zero_hint_rows() {
    let (m, f) = (map(), facts());
    let c = ctx(&m, &[], &f);
    for t in [
        r"C:\Users\old\Downloads\payload.exe",
        r#""C:\Users\old\Downloads\payload.exe""#,
        r"'C:\Users\old\Downloads\payload.exe'",
        r"C:\Users\old\Downloads\payload.exe.",
        r"C:\Users\old\Downloads\payload.exe . .",
        r"C:\Users\old\Downloads\payload.exe::$DATA",
        r"C:\Users\old\Downloads\notes.txt:payload.exe",
        r"\\?\C:\Users\old\Downloads\payload.exe",
        r"C:/Users/old/Downloads/payload.EXE",
        r#""C:\Users\old\Downloads\payload.exe" --tray"#,
        "payload.exe",
    ] {
        let fate = classify_kind(STORE, "Value", &text(t), false, &c);
        assert!(matches!(fate, Fate::NotMoved(_)), "{t}: {fate:?}");
        assert!(
            may_write_with(&f, &[], &reg("Value", RegKind::Sz), &text(t)).is_err(),
            "{t}"
        );
    }
    // A stream that names no program, and a short (8.3) name PCTwin cannot resolve, are places
    // it cannot check.
    for t in [
        r"C:\Users\old\Documents\notes.txt:hidden",
        r"C:\Users\old\Documents\PAYLOA~1\x.txt",
    ] {
        assert_eq!(
            danger(&f, &[], STORE, "Value", t),
            Some(Why::PlaceUnknown),
            "{t}"
        );
    }
    // Data is still data: pointed at the moved file.
    assert_eq!(
        classify_kind(
            STORE,
            "Value",
            &text(r"C:\Users\old\Documents\x.txt"),
            false,
            &c
        ),
        Fate::Rewritten(text(r"C:\Users\new\Documents\x.txt"))
    );
}

/// Finding 1, on this laptop's own facts (no catalogue rows): Windows' `PATHEXT` and file types,
/// or the file's own execute bit and header elsewhere.
#[test]
fn f1_this_laptops_own_facts_keep_a_program_back_with_zero_hint_rows() {
    let f = Facts::system();
    #[cfg(windows)]
    {
        assert!(f.ending_runs("exe") && f.ending_runs(".BAT") && f.ending_runs("cmd"));
        assert!(!f.ending_runs("txt"));
        let m = map();
        let c = ctx(&m, &[], f);
        let t = r"C:\Users\old\Downloads\payload.exe";
        assert_eq!(
            classify_kind(STORE, "Value", &text(t), false, &c),
            Fate::NotMoved(Why::StartsProgram)
        );
        assert_eq!(
            pctwin_settings::generic::may_write(
                &[],
                &reg("Value", RegKind::Sz),
                &text(r"C:\Users\new\Downloads\payload.exe")
            ),
            Err(Why::StartsProgram)
        );
    }
    // A program on disk is known by its header, whatever its name says.
    let dir = tempfile::tempdir().unwrap();
    let prog = dir.path().join("report");
    std::fs::write(&prog, b"MZ\x90\x00\x03").unwrap();
    let data = dir.path().join("notes");
    std::fs::write(&data, b"hello").unwrap();
    let p = prog.to_string_lossy().to_string();
    let d = data.to_string_lossy().to_string();
    assert_eq!(danger(f, &[], STORE, "Value", &p), Some(Why::StartsProgram));
    assert_eq!(danger(f, &[], STORE, "Value", &d), None);
    let script = dir.path().join("tool");
    std::fs::write(&script, b"#!/bin/sh\nid\n").unwrap();
    assert_eq!(
        danger(f, &[], STORE, "Value", &script.to_string_lossy()),
        Some(Why::StartsProgram)
    );
}

/// The catalogue's rows only ever add caution: whatever is kept back with no rows is kept back
/// with any one row and with all of them, and nothing moves with rows that did not without.
#[test]
fn hint_rows_never_make_a_value_less_cautious() {
    let cat = Catalogue::starter().unwrap();
    let all = cat.action_hints.clone();
    let (m, f) = (map(), facts());
    let corpus = [
        "ocean",
        "12",
        r"C:\Users\old\Documents\x.txt",
        r"C:\Users\old\Downloads\payload.exe",
        r"C:\Users\old\run.ps1",
        r"\\srv\share",
        "https:evil.example",
        "shell:startup",
        "cmd /c calc",
        "%APPDATA%\\x",
        "notepad",
        "x.cur",
        "/home/ada/run.sh",
        "Arial:12",
    ];
    let names = [
        "Value",
        "Editor",
        "Command",
        "Arrow",
        "SCRNSAVE.EXE",
        "Theme",
    ];
    let stores = [STORE, r"Control Panel\Cursors", r"Control Panel\Desktop"];
    let mut sets: Vec<Vec<_>> = all.iter().map(|h| vec![h.clone()]).collect();
    sets.push(all.clone());
    for store in stores {
        for name in names {
            for t in corpus {
                let none = classify_kind(store, name, &text(t), false, &ctx(&m, &[], &f));
                for set in &sets {
                    let with = classify_kind(store, name, &text(t), false, &ctx(&m, set, &f));
                    if matches!(none, Fate::NotMoved(_)) {
                        assert!(
                            matches!(with, Fate::NotMoved(_)),
                            "{store} {name} {t}: {with:?}"
                        );
                    }
                    if !matches!(with, Fate::NotMoved(_)) {
                        assert_eq!(with, none, "{store} {name} {t}");
                    }
                }
            }
        }
    }
}

/// Finding 2: addresses the URL parser reads with a host, in any spelling, and scheme-shaped
/// names a handler opens, are kept back; no list of schemes decides it.
#[test]
fn f2_every_address_spelling_is_kept_back() {
    let (m, f) = (FolderMap::default(), facts());
    let c = ctx(&m, &[], &f);
    for t in [
        "https:evil.example",
        "HTTPS:evil.example",
        "https:1.2.3.4",
        "http:evil.example",
        r"https:\\evil.example",
        "http:8080",
        "see https:evil.example now",
        "shell:startup",
        "shell:::{20D04FE0-3AEA-1069-A2D8-08002B30309D}",
        "ms-settings:privacy-webcam",
        "calculator:",
        "ms-settings:",
        "zz-unknown:thing",
    ] {
        let fate = classify_kind(STORE, "StartPage", &text(t), false, &c);
        assert!(matches!(fate, Fate::NotMoved(_)), "{t}: {fate:?}");
        assert!(
            may_write_with(&f, &[], &reg("StartPage", RegKind::Sz), &text(t)).is_err(),
            "{t}"
        );
    }
    // Where the system's handlers cannot be read, a bare scheme is cautious too.
    let unknown = Facts::new([], None);
    assert!(danger(&unknown, &[], STORE, "V", "calculator:").is_some());
    // A number pair or a label is text when nothing handles that name.
    for t in ["Arial:12", "12:30", "Note:", "abc:1"] {
        assert_eq!(danger(&f, &[], STORE, "V", t), None, "{t}");
    }
}

/// Finding 3: raw bytes are judged by every printable run in them, as UTF-8 and UTF-16 in both
/// byte orders and alignments.
#[test]
fn f3_a_binary_value_holding_an_address_is_kept_back_and_never_written() {
    let (m, f) = (FolderMap::default(), facts());
    let c = ctx(&m, &[], &f);
    let unc = r"\\attacker.example\share\x.dat";
    let utf8: String = unc.bytes().map(|b| format!("{b:02x}")).collect();
    let be: String = unc
        .encode_utf16()
        .flat_map(u16::to_be_bytes)
        .map(|b| format!("{b:02x}"))
        .collect();
    for hex in [utf16_hex(unc, false), utf16_hex(unc, true), utf8, be] {
        let v = text(&hex);
        let fate = classify_kind(STORE, "LastFolder", &v, true, &c);
        assert!(matches!(fate, Fate::NotMoved(_)), "{hex}: {fate:?}");
        assert!(may_write_with(&f, &[], &reg("LastFolder", RegKind::Binary), &v).is_err());
    }
    // Plain bytes with a font's name in them move.
    let v = text(&utf16_hex("Segoe UI", false));
    assert_eq!(
        classify_kind(STORE, "Face", &v, true, &c),
        Fate::Copy(v.clone())
    );
    assert_eq!(
        may_write_with(&f, &[], &reg("Face", RegKind::Binary), &v),
        Ok(())
    );
    // Bytes that are not bytes are never copied unjudged.
    let bad = text("zz");
    assert_eq!(
        classify_kind(STORE, "Face", &bad, true, &c),
        Fate::NotMoved(Why::Kind)
    );
    assert_eq!(
        may_write_with(&f, &[], &reg("Face", RegKind::Binary), &bad),
        Err(Why::Kind)
    );
    assert!(runs(&[0x41, 0x42, 0x43]).is_empty());
}

/// Finding 3: a Mac preference's data (a bookmark-shaped blob) and a GVariant byte array are
/// decoded and judged too.
#[test]
fn f3_preference_data_and_byte_arrays_are_judged() {
    let cat = Catalogue::starter().unwrap();
    let (m, f) = (FolderMap::default(), facts());
    let c = ctx(&m, &cat.action_hints, &f);
    // base64 of "smb://attacker.example/share".
    let xml =
        "<dict><key>LastPlace</key><data>c21iOi8vYXR0YWNrZXIuZXhhbXBsZS9zaGFyZQ==</data></dict>";
    let a = Address::MacPref {
        domain: "com.acme.viewer".into(),
        key: "Recents".into(),
        kind: PrefKind::Xml,
    };
    let d = Discovery {
        found: vec![FoundSetting {
            address: a.clone(),
            label: "Acme".into(),
            value: text(xml),
        }],
        not_moved: vec![],
    };
    let p = plan_found(&cat, System::Mac, System::Mac, &d, &c);
    assert!(p.moves.is_empty());
    assert_eq!(p.not_moved.len(), 1);
    assert!(may_write_with(&f, &cat.action_hints, &a, &text(xml)).is_err());
    // Data that cannot be decoded is refused, never copied.
    let broken = "<dict><key>k</key><data>@@@</data></dict>";
    assert_eq!(
        classify_as(Kind::Plist, "com.acme.viewer", "Recents", &text(broken), &c),
        Fate::NotMoved(Why::Kind)
    );
    // Harmless data moves.
    let ok = "<dict><key>k</key><data>AAECAw==</data></dict>";
    assert_eq!(
        classify_as(Kind::Plist, "com.acme.viewer", "Recents", &text(ok), &c),
        Fate::Copy(text(ok))
    );

    // A dconf byte array spelling "smb://attacker.example/share".
    let bytes: Vec<String> = "smb://attacker.example/share"
        .bytes()
        .map(|b| format!("0x{b:02x}"))
        .collect();
    for g in [
        format!("[byte {}]", bytes.join(", ")),
        format!(
            "@ay [{}]",
            "smb://attacker.example/share"
                .bytes()
                .map(|b| b.to_string())
                .collect::<Vec<_>>()
                .join(", ")
        ),
    ] {
        let a = Address::Dconf {
            path: "/org/acme/viewer/".into(),
            key: "last-place".into(),
        };
        assert!(
            matches!(
                classify_as(
                    Kind::GVariant,
                    "/org/acme/viewer/",
                    "last-place",
                    &text(&g),
                    &c
                ),
                Fate::NotMoved(_)
            ),
            "{g}"
        );
        assert!(
            may_write_with(&f, &cat.action_hints, &a, &text(&g)).is_err(),
            "{g}"
        );
        assert!(
            texts(Kind::GVariant, &text(&g))
                .unwrap()
                .iter()
                .any(|t| t.contains("smb://"))
        );
    }
}

/// Surviving mutants C11 and G1: raw bytes of a key that runs or loads something are kept back
/// at planning and refused at writing, whatever they hold.
#[test]
fn m_binary_value_of_a_hinted_key_is_kept_back_and_refused() {
    let cat = Catalogue::starter().unwrap();
    let h = &cat.action_hints;
    let (m, f) = (FolderMap::default(), facts());
    let c = ctx(&m, h, &f);
    let v = text("00010203");
    for (store, name) in [(STORE, "Command"), (r"Control Panel\Cursors", "Arrow")] {
        assert_eq!(
            classify_kind(store, name, &v, true, &c),
            Fate::NotMoved(Why::StartsProgram),
            "{name}"
        );
        let a = Address::Registry {
            key: store.into(),
            name: name.into(),
            kind: RegKind::Binary,
        };
        assert_eq!(
            may_write_with(&f, h, &a, &v),
            Err(Why::StartsProgram),
            "{name}"
        );
    }
    // A key named like a file (`x.webp`, a four-letter ending) loads that file.
    let a = Address::Registry {
        key: STORE.into(),
        name: "Logo.webp".into(),
        kind: RegKind::Binary,
    };
    assert_eq!(may_write_with(&f, &[], &a, &v), Err(Why::StartsProgram));
}

/// Surviving mutant C18: a key whose name has a four-character file ending names a file the
/// system loads, so any path in it is kept back.
#[test]
fn m_a_four_letter_file_ending_in_a_key_name_marks_a_loaded_file() {
    let (m, f) = (map(), facts());
    let c = ctx(&m, &[], &f);
    for n in ["Logo.webp", "Icon.icns"] {
        assert!(pctwin_settings::classify::file_shaped_name(n), "{n}");
        assert_eq!(
            classify_kind(STORE, n, &text(r"C:\Users\old\Pictures\a.png"), false, &c),
            Fate::NotMoved(Why::StartsProgram),
            "{n}"
        );
    }
    assert!(!pctwin_settings::classify::file_shaped_name("a.abcde"));
}

/// Surviving mutant G2: a place outside discovery's scope is never written, whatever the value.
#[test]
fn m_writing_outside_discoverys_scope_is_refused() {
    let f = facts();
    for a in [
        Address::Registry {
            key: r"Software\Microsoft\Windows\CurrentVersion\Run".into(),
            name: "x".into(),
            kind: RegKind::Sz,
        },
        Address::Registry {
            key: r"System\CurrentControlSet\Services".into(),
            name: "x".into(),
            kind: RegKind::Sz,
        },
        Address::Dconf {
            path: "relative/".into(),
            key: "k".into(),
        },
    ] {
        assert_eq!(
            may_write_with(&f, &[], &a, &text("ocean")),
            Err(Why::SystemStore),
            "{a:?}"
        );
    }
}

/// Surviving mutant G3: every item of a multi-string value is checked at the moment of writing,
/// and a list that is not one is refused.
#[test]
fn m_multi_string_items_are_checked_when_written() {
    let f = facts();
    let a = reg("Folders", RegKind::MultiSz);
    assert_eq!(
        may_write_with(&f, &[], &a, &text(r#"["ocean","sky"]"#)),
        Ok(())
    );
    assert_eq!(
        may_write_with(&f, &[], &a, &text(r#"["ocean","\\\\attacker.example\\s"]"#)),
        Err(Why::Fetches)
    );
    assert_eq!(
        may_write_with(&f, &[], &a, &text("not a list")),
        Err(Why::Kind)
    );
}

/// Surviving mutant G4: a path inside a composite value is never rewritten in place: the value
/// is kept back.
#[test]
fn m_a_moved_path_inside_a_composite_value_keeps_it_back() {
    let cat = Catalogue::starter().unwrap();
    let (m, f) = (map(), facts());
    let c = ctx(&m, &cat.action_hints, &f);
    let xml = r"<dict><key>Last</key><string>C:\Users\old\Documents\x.txt</string></dict>";
    assert_eq!(
        classify_as(Kind::Plist, "com.acme.viewer", "Recents", &text(xml), &c),
        Fate::NotMoved(Why::PathNotMoved)
    );
    let g = r"['C:\\Users\\old\\Documents\\x.txt', 'b']";
    assert_eq!(
        classify_as(Kind::GVariant, "/org/acme/viewer/", "files", &text(g), &c),
        Fate::NotMoved(Why::PathNotMoved)
    );
}

/// Surviving mutant G7: a dconf value nested past the depth gate is refused at the moment of
/// writing too.
#[test]
fn m_a_too_deep_dconf_value_is_refused_when_written() {
    let f = facts();
    let a = Address::Dconf {
        path: "/org/acme/viewer/".into(),
        key: "k".into(),
    };
    let deep = format!("{}'x'{}", "[".repeat(300), "]".repeat(300));
    assert_eq!(may_write_with(&f, &[], &a, &text(&deep)), Err(Why::TooDeep));
    assert_eq!(may_write_with(&f, &[], &a, &text("[['x']]")), Ok(()));
}
