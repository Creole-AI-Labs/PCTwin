use serde::Deserialize;

use super::*;

#[derive(Debug, Clone, Deserialize, PartialEq)]
#[serde(rename_all = "kebab-case")]
enum Kind {
    Fruit,
    Nut,
}

#[derive(Debug, Clone, Deserialize, PartialEq)]
struct Food {
    name: String,
    kind: Kind,
}

impl Row for Food {
    const CATALOGUE: &'static str = "food";
    const FIELDS: &'static [&'static str] = &["name", "kind"];
    fn check(&self) -> Result<(), String> {
        if self.name.is_empty() {
            Err("an empty name".into())
        } else {
            Ok(())
        }
    }
}

#[derive(Debug, Deserialize)]
struct Lever {
    #[allow(dead_code)]
    name: String,
}

impl Row for Lever {
    const CATALOGUE: &'static str = "levers";
    const FIELDS: &'static [&'static str] = &["name"];
    const TRUST: Trust = Trust::Privileged;
}

const FOOD: &str = r#"{"format":1,"catalogue":"food","version":3,"rows":[
    {"name":"apple","kind":"fruit"},
    {"name":"pecan","kind":"nut","colour":"brown"},
    {"name":"","kind":"fruit"},
    {"name":"kale","kind":"leaf"},
    "not a row",
    {"kind":"nut"}
]}"#;

#[test]
fn rows_load_one_by_one_and_every_problem_is_reported() {
    let c = Catalogue::<Food>::shipped(FOOD);
    assert_eq!(c.version(), 3);
    assert_eq!(c.source(), Source::Shipped);
    let names: Vec<&str> = c.rows().iter().map(|r| r.name.as_str()).collect();
    assert_eq!(names, ["apple", "pecan"]);
    let skipped: Vec<usize> = c.report().skipped.iter().map(|s| s.row).collect();
    assert_eq!(skipped, [2, 3, 4, 5]);
    assert_eq!(c.report().skipped[0].why, "an empty name");
    assert!(c.report().skipped[1].why.contains("leaf"));
    assert_eq!(c.report().unknown_fields, [(Some(1), "colour".to_string())]);
    assert!(!c.report().is_clean());
    assert!(c.report().refused.is_none());
}

#[test]
fn a_clean_catalogue_reports_clean() {
    let c = Catalogue::<Food>::shipped(
        r#"{"format":1,"catalogue":"food","version":1,"rows":[{"name":"fig","kind":"fruit"}]}"#,
    );
    assert!(c.report().is_clean());
    assert_eq!(c.rows().len(), 1);
    let top = Catalogue::<Food>::shipped(
        r#"{"format":1,"catalogue":"food","version":1,"rows":[],"extra":1}"#,
    );
    assert_eq!(top.report().unknown_fields, [(None, "extra".to_string())]);
}

#[test]
fn a_shipped_file_that_cannot_be_read_gives_no_rows_and_never_panics() {
    for (bad, why) in [
        ("not json", "damaged"),
        (
            r#"{"format":2,"catalogue":"food","version":1,"rows":[]}"#,
            "newer",
        ),
        (
            r#"{"format":0,"catalogue":"food","version":1,"rows":[]}"#,
            "format",
        ),
        (
            r#"{"format":1,"catalogue":"drink","version":1,"rows":[]}"#,
            "other",
        ),
        (
            r#"{"format":1,"catalogue":"food","version":0,"rows":[]}"#,
            "version",
        ),
        (r#"{"format":1,"catalogue":"food","rows":[]}"#, "version"),
        (r#"{"format":1,"catalogue":"food","version":1}"#, "rows"),
        (r#"[1]"#, "shape"),
    ] {
        let c = Catalogue::<Food>::shipped(bad);
        assert!(c.rows().is_empty(), "{why}");
        assert!(c.report().refused.is_some(), "{why}");
        assert!(!c.report().is_clean(), "{why}");
        assert_eq!(c.version(), 0, "{why}");
    }
    let newer = Catalogue::<Food>::shipped(r#"{"format":2,"catalogue":"food","version":1}"#);
    assert!(matches!(
        newer.report().refused,
        Some(CatalogueError::Format(FormatError::Newer { found: 2, .. }))
    ));
    let other = Catalogue::<Food>::shipped(r#"{"format":1,"catalogue":"drink","version":1}"#);
    assert_eq!(
        other.report().refused,
        Some(CatalogueError::WrongCatalogue {
            want: "food",
            found: "drink".into()
        })
    );
}

#[test]
fn reading_returns_every_found_thing_once_known_or_not() {
    let c = Catalogue::<Food>::shipped(FOOD);
    let r = c.read(["pecan", "durian", "apple", "durian"], |found, row| {
        row.name == *found
    });
    let known: Vec<(&str, &str)> = r
        .known
        .iter()
        .map(|(f, row)| (*f, row.name.as_str()))
        .collect();
    assert_eq!(known, [("pecan", "pecan"), ("apple", "apple")]);
    assert_eq!(r.found_not_supported, ["durian", "durian"]);
    // The first row that knows a thing is the one used.
    let r = c.read(["x"], |_, _| true);
    assert_eq!(r.known[0].1.name, "apple");
    // An empty catalogue knows nothing, and drops nothing.
    let empty = Catalogue::<Food>::shipped("broken");
    let r = empty.read([1, 2, 3], |_, _| true);
    assert!(r.known.is_empty());
    assert_eq!(r.found_not_supported, [1, 2, 3]);
}

fn file(version: u32) -> Vec<u8> {
    format!(
        r#"{{"format":1,"catalogue":"food","version":{version},"rows":[{{"name":"fig","kind":"fruit"}}]}}"#
    )
    .into_bytes()
}

#[test]
fn a_delivered_catalogue_needs_a_trusted_signature_for_its_name_and_version() {
    let keys = test_key::keys();
    let bytes = file(4);
    let sig = test_key::sign(&bytes, "food", 4);
    let c = Catalogue::<Food>::signed(&bytes, &sig, &keys, 3).unwrap();
    assert_eq!(c.version(), 4);
    assert_eq!(c.source(), Source::Signed);
    assert_eq!(c.rows().len(), 1);
    // The same version as the one in use is fine.
    assert!(Catalogue::<Food>::signed(&bytes, &sig, &keys, 4).is_ok());

    // Production trusts no key yet.
    assert_eq!(
        Catalogue::<Food>::signed(&bytes, &sig, &TrustedKeys::production(), 0).unwrap_err(),
        CatalogueError::NotSigned
    );
    // A changed byte, another key, no signature at all.
    let mut changed = bytes.clone();
    let at = changed.len() - 3;
    changed[at] = b' ';
    assert_eq!(
        Catalogue::<Food>::signed(&changed, &sig, &keys, 0).unwrap_err(),
        CatalogueError::NotSigned
    );
    assert_eq!(
        Catalogue::<Food>::signed(&bytes, &sig, &test_key::privileged_keys(), 0).unwrap_err(),
        CatalogueError::NotSigned
    );
    assert_eq!(
        Catalogue::<Food>::signed(&bytes, "", &keys, 0).unwrap_err(),
        CatalogueError::NotSigned
    );
    // Signed for another catalogue, or another version.
    for sig in [
        test_key::sign(&bytes, "drink", 4),
        test_key::sign(&bytes, "food", 5),
        test_key::sign_raw(&bytes, "anything"),
    ] {
        assert_eq!(
            Catalogue::<Food>::signed(&bytes, &sig, &keys, 0).unwrap_err(),
            CatalogueError::SignedForSomethingElse
        );
    }
    // Never rolled back.
    assert_eq!(
        Catalogue::<Food>::signed(&bytes, &sig, &keys, 5).unwrap_err(),
        CatalogueError::Older {
            found: 4,
            in_use: 5
        }
    );
    // A signed file still follows the format rule.
    let newer = br#"{"format":2,"catalogue":"food","version":1,"rows":[]}"#;
    let sig = test_key::sign(newer, "food", 1);
    assert!(matches!(
        Catalogue::<Food>::signed(newer, &sig, &keys, 0).unwrap_err(),
        CatalogueError::Format(FormatError::Newer { .. })
    ));
}

#[test]
fn privileged_rows_need_a_privileged_key() {
    let bytes = br#"{"format":1,"catalogue":"levers","version":1,"rows":[{"name":"a"}]}"#;
    let ordinary = test_key::sign(bytes, "levers", 1);
    assert_eq!(
        Catalogue::<Lever>::signed(bytes, &ordinary, &test_key::keys(), 0).unwrap_err(),
        CatalogueError::NeedsPrivilegedKey
    );
    let privileged = test_key::sign_privileged(bytes, "levers", 1);
    assert_eq!(
        Catalogue::<Lever>::signed(bytes, &privileged, &test_key::privileged_keys(), 0)
            .unwrap()
            .rows()
            .len(),
        1
    );
    // A privileged key also signs ordinary catalogues.
    let food = file(1);
    let sig = test_key::sign_privileged(&food, "food", 1);
    assert!(Catalogue::<Food>::signed(&food, &sig, &test_key::privileged_keys(), 0).is_ok());
}

#[test]
fn sizes_are_bounded() {
    let mut big = file(1);
    big.resize(MAX_BYTES + 1, b' ');
    let sig = test_key::sign(&big, "food", 1);
    assert_eq!(
        Catalogue::<Food>::signed(&big, &sig, &test_key::keys(), 0).unwrap_err(),
        CatalogueError::TooLarge
    );
    let text = String::from_utf8(big).unwrap();
    assert_eq!(
        Catalogue::<Food>::shipped(&text).report().refused,
        Some(CatalogueError::TooLarge)
    );
    let rows = vec!["{}"; MAX_ROWS + 1].join(",");
    let many = format!(r#"{{"format":1,"catalogue":"food","version":1,"rows":[{rows}]}}"#);
    assert_eq!(
        Catalogue::<Food>::shipped(&many).report().refused,
        Some(CatalogueError::TooLarge)
    );
    let rows = vec!["{}"; MAX_ROWS].join(",");
    let most = format!(r#"{{"format":1,"catalogue":"food","version":1,"rows":[{rows}]}}"#);
    let c = Catalogue::<Food>::shipped(&most);
    assert!(c.report().refused.is_none());
    // Problems past the first thousand are counted, not listed.
    assert_eq!(c.report().skipped.len(), MAX_REPORTED);
    assert_eq!(c.report().more, MAX_ROWS - MAX_REPORTED);
}

#[test]
fn the_newer_catalogue_wins() {
    let keys = test_key::keys();
    let a = Catalogue::<Food>::shipped(std::str::from_utf8(&file(2)).unwrap());
    let b_bytes = file(3);
    let b = Catalogue::<Food>::signed(&b_bytes, &test_key::sign(&b_bytes, "food", 3), &keys, 0)
        .unwrap();
    assert_eq!(a.clone().newer(b.clone()).version(), 3);
    assert_eq!(b.clone().newer(a.clone()).version(), 3);
    let two = file(2);
    let same = Catalogue::<Food>::signed(&two, &test_key::sign(&two, "food", 2), &keys, 0).unwrap();
    assert_eq!(a.newer(same).source(), Source::Shipped);
}

#[test]
fn an_old_form_signature_is_refused() {
    let bytes = file(1);
    let sig = test_key::sign_legacy(&bytes, "food", 1);
    assert_eq!(
        Catalogue::<Food>::signed(&bytes, &sig, &test_key::keys(), 0).unwrap_err(),
        CatalogueError::NotSigned
    );
}

#[test]
fn an_oversized_file_is_refused_before_its_signature_is_checked() {
    let mut big = file(1);
    big.resize(MAX_BYTES + 1, b' ');
    assert_eq!(
        Catalogue::<Food>::signed(&big, "", &test_key::keys(), 0).unwrap_err(),
        CatalogueError::TooLarge
    );
}

#[test]
fn any_problem_makes_a_report_unclean() {
    assert!(Report::default().is_clean());
    let counted = Report {
        more: 1,
        ..Report::default()
    };
    assert!(!counted.is_clean());
    let unknown = Report {
        unknown_fields: vec![(None, "x".into())],
        ..Report::default()
    };
    assert!(!unknown.is_clean());
    let skipped = Report {
        skipped: vec![Skipped {
            row: 0,
            why: "x".into(),
        }],
        ..Report::default()
    };
    assert!(!skipped.is_clean());
}
