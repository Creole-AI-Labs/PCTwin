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
    let c = Catalogue::<Food>::signed(&bytes, &sig, &keys, &crate::MemoryMarks::at(3)).unwrap();
    assert_eq!(c.version(), 4);
    assert_eq!(c.source(), Source::Signed);
    assert_eq!(c.rows().len(), 1);
    // The same version as the one in use is fine.
    assert!(Catalogue::<Food>::signed(&bytes, &sig, &keys, &crate::MemoryMarks::at(4)).is_ok());

    // Production trusts no key yet.
    assert_eq!(
        Catalogue::<Food>::signed(
            &bytes,
            &sig,
            &TrustedKeys::production(),
            &crate::MemoryMarks::at(0)
        )
        .unwrap_err(),
        CatalogueError::NotSigned
    );
    // A changed byte, another key, no signature at all.
    let mut changed = bytes.clone();
    let at = changed.len() - 3;
    changed[at] = b' ';
    assert_eq!(
        Catalogue::<Food>::signed(&changed, &sig, &keys, &crate::MemoryMarks::at(0)).unwrap_err(),
        CatalogueError::NotSigned
    );
    assert_eq!(
        Catalogue::<Food>::signed(
            &bytes,
            &sig,
            &test_key::privileged_keys(),
            &crate::MemoryMarks::at(0)
        )
        .unwrap_err(),
        CatalogueError::NotSigned
    );
    assert_eq!(
        Catalogue::<Food>::signed(&bytes, "", &keys, &crate::MemoryMarks::at(0)).unwrap_err(),
        CatalogueError::NotSigned
    );
    // Signed for another catalogue, or another version.
    for sig in [
        test_key::sign(&bytes, "drink", 4),
        test_key::sign(&bytes, "food", 5),
        test_key::sign_raw(&bytes, "anything"),
    ] {
        assert_eq!(
            Catalogue::<Food>::signed(&bytes, &sig, &keys, &crate::MemoryMarks::at(0)).unwrap_err(),
            CatalogueError::SignedForSomethingElse
        );
    }
    // Never rolled back.
    assert_eq!(
        Catalogue::<Food>::signed(&bytes, &sig, &keys, &crate::MemoryMarks::at(5)).unwrap_err(),
        CatalogueError::Older {
            found: 4,
            in_use: 5
        }
    );
    // A signed file still follows the format rule.
    let newer = br#"{"format":2,"catalogue":"food","version":1,"rows":[]}"#;
    let sig = test_key::sign(newer, "food", 1);
    assert!(matches!(
        Catalogue::<Food>::signed(newer, &sig, &keys, &crate::MemoryMarks::at(0)).unwrap_err(),
        CatalogueError::Format(FormatError::Newer { .. })
    ));
}

#[test]
fn privileged_rows_need_a_privileged_key() {
    let bytes = br#"{"format":1,"catalogue":"levers","version":1,"rows":[{"name":"a"}]}"#;
    let ordinary = test_key::sign(bytes, "levers", 1);
    assert_eq!(
        Catalogue::<Lever>::signed(
            bytes,
            &ordinary,
            &test_key::keys(),
            &crate::MemoryMarks::at(0)
        )
        .unwrap_err(),
        CatalogueError::NeedsPrivilegedKey
    );
    let privileged = test_key::sign_privileged(bytes, "levers", 1);
    assert_eq!(
        Catalogue::<Lever>::signed(
            bytes,
            &privileged,
            &test_key::privileged_keys(),
            &crate::MemoryMarks::at(0)
        )
        .unwrap()
        .rows()
        .len(),
        1
    );
    // A privileged key also signs ordinary catalogues.
    let food = file(1);
    let sig = test_key::sign_privileged(&food, "food", 1);
    assert!(
        Catalogue::<Food>::signed(
            &food,
            &sig,
            &test_key::privileged_keys(),
            &crate::MemoryMarks::at(0)
        )
        .is_ok()
    );
}

#[test]
fn sizes_are_bounded() {
    let mut big = file(1);
    big.resize(MAX_BYTES + 1, b' ');
    let sig = test_key::sign(&big, "food", 1);
    assert_eq!(
        Catalogue::<Food>::signed(&big, &sig, &test_key::keys(), &crate::MemoryMarks::at(0))
            .unwrap_err(),
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
    let b = Catalogue::<Food>::signed(
        &b_bytes,
        &test_key::sign(&b_bytes, "food", 3),
        &keys,
        &crate::MemoryMarks::at(0),
    )
    .unwrap();
    assert_eq!(a.clone().newer(b.clone()).version(), 3);
    assert_eq!(b.clone().newer(a.clone()).version(), 3);
    let two = file(2);
    let same = Catalogue::<Food>::signed(
        &two,
        &test_key::sign(&two, "food", 2),
        &keys,
        &crate::MemoryMarks::at(0),
    )
    .unwrap();
    assert_eq!(a.newer(same).source(), Source::Shipped);
}

#[test]
fn an_old_form_signature_is_refused() {
    let bytes = file(1);
    let sig = test_key::sign_legacy(&bytes, "food", 1);
    assert_eq!(
        Catalogue::<Food>::signed(&bytes, &sig, &test_key::keys(), &crate::MemoryMarks::at(0))
            .unwrap_err(),
        CatalogueError::NotSigned
    );
}

#[test]
fn an_oversized_file_is_refused_before_its_signature_is_checked() {
    let mut big = file(1);
    big.resize(MAX_BYTES + 1, b' ');
    assert_eq!(
        Catalogue::<Food>::signed(&big, "", &test_key::keys(), &crate::MemoryMarks::at(0))
            .unwrap_err(),
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

/// Fourth review, catalogue delivery gates: the rollback mark is kept by PCTwin and raised by
/// the load itself, so replaying an older signed catalogue fails even when the caller would
/// have said "nothing is in use" (the review's replay passed with `in_use` 0).
#[test]
fn review4_the_rollback_mark_is_kept_and_raised_by_the_load_itself() {
    let keys = test_key::keys();
    let marks = MemoryMarks::new();
    let v3 = file(3);
    let v2 = file(2);
    let sig3 = test_key::sign(&v3, "food", 3);
    let sig2 = test_key::sign(&v2, "food", 2);
    assert!(Catalogue::<Food>::signed(&v3, &sig3, &keys, &marks).is_ok());
    assert_eq!(marks.highest("food"), Ok(Some(3)));
    assert_eq!(
        Catalogue::<Food>::signed(&v2, &sig2, &keys, &marks).unwrap_err(),
        CatalogueError::Older {
            found: 2,
            in_use: 3
        }
    );
    // Another catalogue's mark is its own.
    assert_eq!(marks.highest("drinks"), Ok(None));
    // A refused catalogue never raises the mark.
    let v9 = file(9);
    assert!(Catalogue::<Food>::signed(&v9, &sig3, &keys, &marks).is_err());
    assert_eq!(marks.highest("food"), Ok(Some(3)));

    struct Broken;
    impl RollbackMarks for Broken {
        fn highest(&self, _: &str) -> Result<Option<u32>, MarkUnavailable> {
            Err(MarkUnavailable)
        }
        fn raise(&self, _: &str, _: u32) -> Result<(), MarkUnavailable> {
            Err(MarkUnavailable)
        }
    }
    assert_eq!(
        Catalogue::<Food>::signed(&v3, &sig3, &keys, &Broken).unwrap_err(),
        CatalogueError::MarkUnavailable
    );
    struct ReadOnly;
    impl RollbackMarks for ReadOnly {
        fn highest(&self, _: &str) -> Result<Option<u32>, MarkUnavailable> {
            Ok(None)
        }
        fn raise(&self, _: &str, _: u32) -> Result<(), MarkUnavailable> {
            Err(MarkUnavailable)
        }
    }
    assert_eq!(
        Catalogue::<Food>::signed(&v3, &sig3, &keys, &ReadOnly).unwrap_err(),
        CatalogueError::MarkUnavailable
    );
}

/// Fourth review: a signed file giving `catalogue` and `version` twice (which one wins?) was
/// accepted. A key given twice anywhere refuses the whole file.
#[test]
fn review4_a_key_given_twice_refuses_the_file() {
    let dup = br#"{"format":1,"catalogue":"drinks","catalogue":"food","version":1,"version":7,"rows":[]}"#;
    let sig = test_key::sign(dup, "food", 7);
    let e =
        Catalogue::<Food>::signed(dup, &sig, &test_key::keys(), &MemoryMarks::new()).unwrap_err();
    assert!(
        matches!(&e, CatalogueError::Damaged(w) if w.contains("given twice")),
        "{e:?}"
    );
    let in_row = r#"{"format":1,"catalogue":"food","version":1,"rows":[
        {"name":"apple","kind":"fruit","kind":"nut"}]}"#;
    let c = Catalogue::<Food>::shipped(in_row);
    assert!(c.rows().is_empty());
    assert!(matches!(
        c.report().refused,
        Some(CatalogueError::Damaged(_))
    ));
}

#[derive(Debug, Deserialize)]
struct Named {
    id: String,
    #[allow(dead_code)]
    to: String,
}

impl Row for Named {
    const CATALOGUE: &'static str = "named";
    const FIELDS: &'static [&'static str] = &["id", "to"];
    fn identity(&self) -> Option<String> {
        Some(self.id.clone())
    }
}

/// Fourth review: two rows for one ID (the first silently winning) and a row given twice were
/// accepted as clean. Both are skipped and reported.
#[test]
fn review4_rows_given_twice_or_naming_the_same_thing_are_reported() {
    let c = Catalogue::<Named>::shipped(
        r#"{"format":1,"catalogue":"named","version":1,"rows":[
        {"id":"a","to":"x"},{"id":"a","to":"y"},{"id":"b","to":"z"},{"id":"b","to":"z"},{"id":"c","to":"w"}]}"#,
    );
    let ids: Vec<&str> = c.rows().iter().map(|r| r.id.as_str()).collect();
    assert_eq!(ids, ["b", "c"]);
    assert!(!c.report().is_clean());
    let why: Vec<(usize, &str)> = c
        .report()
        .skipped
        .iter()
        .map(|s| (s.row, s.why.as_str()))
        .collect();
    assert_eq!(
        why,
        [
            (3, "this row repeats one before it"),
            (0, "another row names the same thing"),
            (1, "another row names the same thing")
        ]
    );
    // Rows with no identity: only exact repeats are refused.
    let f = Catalogue::<Food>::shipped(
        r#"{"format":1,"catalogue":"food","version":1,"rows":[
        {"name":"apple","kind":"fruit"},{"name":"apple","kind":"fruit"},{"name":"apple","kind":"nut"}]}"#,
    );
    assert_eq!(f.rows().len(), 2);
    assert_eq!(f.report().skipped.len(), 1);
}

#[test]
fn review4_deep_catalogue_text_is_refused_before_it_is_parsed() {
    let deep = format!(
        r#"{{"format":1,"catalogue":"food","version":1,"rows":[{}1{}]}}"#,
        "[".repeat(5000),
        "]".repeat(5000)
    );
    let c = Catalogue::<Food>::shipped(&deep);
    assert!(
        matches!(&c.report().refused, Some(CatalogueError::Damaged(w)) if w.contains("too deeply nested")),
        "{:?}",
        c.report().refused
    );
}
