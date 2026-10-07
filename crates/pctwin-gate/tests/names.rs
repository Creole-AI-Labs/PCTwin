//! Cross-system name rules (Engineering Plan 9.6): a name that a system cannot store is changed to
//! a safe lookalike, and every change is reported so nothing is renamed silently.

use pctwin_gate::{MAX_COMPONENT_BYTES, NameChange, Platform, convert_name};
use proptest::prelude::*;

fn win(name: &str) -> (String, Vec<NameChange>) {
    let c = convert_name(name, Platform::Windows);
    (c.name, c.changes)
}

#[test]
fn ordinary_names_are_left_alone_everywhere() {
    for platform in [Platform::Windows, Platform::MacOs, Platform::Linux] {
        let c = convert_name("Holiday photos 2026.jpg", platform);
        assert_eq!(c.name, "Holiday photos 2026.jpg");
        assert!(c.changes.is_empty());
    }
}

#[test]
fn characters_windows_forbids_become_lookalikes() {
    let (name, changes) = win("Notes: May? <draft> \"final\" a|b*c\\d");
    assert_eq!(
        name,
        "Notes\u{FF1A} May\u{FF1F} \u{FF1C}draft\u{FF1E} \u{FF02}final\u{FF02} a\u{FF5C}b\u{FF0A}c\u{FF3C}d"
    );
    assert_eq!(changes, [NameChange::ForbiddenCharacters]);
    // Control characters are replaced too.
    assert_eq!(win("a\u{1}b\u{1f}c").0, "a_b_c");
}

#[test]
fn windows_reserved_names_are_renamed_whatever_their_case_or_extension() {
    for (input, expected) in [
        ("CON", "CON_"),
        ("con", "con_"),
        ("aux.txt", "aux_.txt"),
        ("NUL.tar.gz", "NUL_.tar.gz"),
        ("com1.log", "com1_.log"),
        ("LPT9", "LPT9_"),
        ("COM\u{b9}", "COM\u{b9}_"),
        ("CONIN$", "CONIN$_"),
        ("prn ", "prn_"), // trailing space trimmed first, then reserved
    ] {
        let (name, changes) = win(input);
        assert_eq!(name, expected, "{input:?}");
        assert!(changes.contains(&NameChange::ReservedName), "{input:?}");
    }
    // Close but not reserved.
    for ok in [
        "CONSOLE",
        "com10",
        "LPT0x",
        "auxiliary.txt",
        "COM0",
        "LPT0",
        "COMX",
        "LPTA",
    ] {
        assert_eq!(win(ok).0, ok);
    }
}

#[test]
fn trailing_dots_and_spaces_are_trimmed_on_windows() {
    assert_eq!(win("Report.").0, "Report");
    assert_eq!(win("Draft ").0, "Draft");
    assert_eq!(win("x. . ").0, "x");
    assert_eq!(win("Report.").1, [NameChange::TrailingDotsOrSpaces]);
    // A name that is nothing but dots and spaces still gets a usable name.
    assert_eq!(win("...").0, "_");
    // Mac and Linux keep them.
    assert_eq!(convert_name("Report.", Platform::MacOs).name, "Report.");
}

#[test]
fn a_name_made_longer_by_lookalikes_is_shortened_keeping_its_extension() {
    let name = format!("{}.docx", "?".repeat(120)); // each ? becomes a 3-byte lookalike
    let (out, changes) = win(&name);
    assert!(out.len() <= MAX_COMPONENT_BYTES, "{}", out.len());
    assert!(out.ends_with(".docx"));
    assert!(changes.contains(&NameChange::Shortened));
}

#[test]
fn mac_and_linux_store_names_composed() {
    let c = convert_name("Cafe\u{301}.txt", Platform::Linux);
    assert_eq!(c.name, "Caf\u{e9}.txt");
}

proptest! {
    #[test]
    fn every_converted_windows_name_is_storable(s in "\\PC{1,80}") {
        let c = convert_name(&s, Platform::Windows);
        let n = &c.name;
        prop_assert!(!n.is_empty());
        prop_assert!(n.len() <= MAX_COMPONENT_BYTES);
        prop_assert!(!n.chars().any(|ch| "<>:\"/\\|?*".contains(ch) || (ch as u32) < 32));
        prop_assert!(!n.ends_with('.') && !n.ends_with(' '));
        let stem = n.split('.').next().unwrap_or("").trim_end().to_ascii_uppercase();
        let reserved = ["CON", "PRN", "AUX", "NUL", "CONIN$", "CONOUT$"];
        prop_assert!(!reserved.contains(&stem.as_str()));
        prop_assert!(!(stem.len() == 4 && (stem.starts_with("COM") || stem.starts_with("LPT"))
            && stem.as_bytes()[3].is_ascii_digit() && stem.as_bytes()[3] != b'0'));
        // Any change a person would notice is reported (storing accents composed is not one).
        use unicode_normalization::UnicodeNormalization;
        let composed: String = s.nfc().collect();
        prop_assert_eq!(c.changes.is_empty(), *n == composed);
    }
}
