//! The safety gate's path checks (Security Design Part B): every path the old laptop sends is
//! relative to an approved folder, and anything that could climb out of it is refused.

use pctwin_gate::{IncomingPath, MAX_COMPONENT_BYTES, MAX_DEPTH, MAX_PATH_BYTES, PathError};
use proptest::prelude::*;

#[test]
fn ordinary_paths_are_accepted_component_by_component() {
    let p = IncomingPath::parse("Documents/Taxes 2026/return.pdf").unwrap();
    assert_eq!(p.components(), ["Documents", "Taxes 2026", "return.pdf"]);
    assert_eq!(
        IncomingPath::parse("notes.txt").unwrap().components(),
        ["notes.txt"]
    );
    // Names that are legal on Mac and Linux are kept as they are; conversion happens later.
    for name in [
        "Notes: May?",
        "a\\b",
        "CON",
        "Report.",
        "файл.txt",
        "写真.jpg",
    ] {
        assert_eq!(
            IncomingPath::parse(name).unwrap().components(),
            [name],
            "{name:?}"
        );
    }
}

#[test]
fn anything_that_could_leave_the_approved_folder_is_refused() {
    let cases = [
        ("", PathError::Empty),
        ("/etc/passwd", PathError::Absolute),
        ("/", PathError::Absolute),
        ("..", PathError::Traversal),
        ("../Windows/System32", PathError::Traversal),
        ("Documents/../../Windows", PathError::Traversal),
        ("a/..", PathError::Traversal),
        (".", PathError::DotComponent),
        ("a/./b", PathError::DotComponent),
        ("a//b", PathError::EmptyComponent),
        ("a/", PathError::EmptyComponent),
        ("a\0b", PathError::NulCharacter),
    ];
    for (input, expected) in cases {
        assert_eq!(
            IncomingPath::parse(input).unwrap_err(),
            expected,
            "{input:?}"
        );
    }
}

#[test]
fn paths_have_size_and_depth_limits() {
    let longest = "a".repeat(MAX_COMPONENT_BYTES);
    assert!(IncomingPath::parse(&longest).is_ok());
    assert_eq!(
        IncomingPath::parse(&format!("{longest}a")).unwrap_err(),
        PathError::ComponentTooLong
    );
    let deep = vec!["d"; MAX_DEPTH].join("/");
    assert!(IncomingPath::parse(&deep).is_ok());
    assert_eq!(
        IncomingPath::parse(&format!("{deep}/d")).unwrap_err(),
        PathError::TooDeep
    );
    let long_path = vec!["x".repeat(200); 25].join("/");
    assert!(long_path.len() > MAX_PATH_BYTES);
    assert_eq!(
        IncomingPath::parse(&long_path).unwrap_err(),
        PathError::TooLong
    );
}

#[test]
fn names_are_compared_in_one_unicode_form() {
    // macOS often stores accented names decomposed; the gate stores them composed.
    let decomposed = IncomingPath::parse("Cafe\u{301}/Re\u{301}sume\u{301}.pdf").unwrap();
    assert_eq!(
        decomposed.components(),
        ["Caf\u{e9}", "R\u{e9}sum\u{e9}.pdf"]
    );
}

proptest! {
    #[test]
    fn parsing_never_panics_and_accepted_paths_never_climb_out(s in "\\PC{0,64}") {
        if let Ok(p) = IncomingPath::parse(&s) {
            for c in p.components() {
                prop_assert!(!c.is_empty());
                prop_assert!(c != "." && c != "..");
                prop_assert!(!c.contains('/') && !c.contains('\0'));
                prop_assert!(c.len() <= MAX_COMPONENT_BYTES);
            }
            prop_assert!(p.components().len() <= MAX_DEPTH);
        }
    }

    #[test]
    fn hostile_mixes_of_separators_and_dots_never_get_through(
        parts in proptest::collection::vec(prop_oneof![
            Just("..".to_string()), Just(".".to_string()), Just("".to_string()),
            Just("ok".to_string()), Just("a b".to_string()),
        ], 1..8)
    ) {
        let s = parts.join("/");
        let bad = parts.iter().any(|p| p.is_empty() || p == "." || p == "..");
        prop_assert_eq!(IncomingPath::parse(&s).is_err(), bad, "{:?}", s);
    }
}
