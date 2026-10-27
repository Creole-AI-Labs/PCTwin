//! Kinds of file for "most important first" are catalogue rows (Phase 2 adoption, changes 7 and
//! 8): a new extension needs only a row, and a file no row names is still counted, as Other.

use pctwin_catalogue::{Catalogue, test_key};
use pctwin_scan::{Categories, Category, CategoryRow, category, shipped_categories};

#[test]
fn the_shipped_rows_read_whole_and_name_the_usual_kinds() {
    assert!(shipped_categories().report().is_clean());
    for (name, want) in [
        ("IMG_0001.HEIC", Category::Photo),
        ("raw.cr3", Category::Photo),
        ("clip.MOV", Category::Video),
        ("song.flac", Category::Music),
        ("thesis.docx", Category::Document),
        ("notes.md", Category::Document),
        ("model.blend", Category::Other),
        ("README", Category::Other),
        (".jpg", Category::Other),
        ("trailing.", Category::Other),
    ] {
        assert_eq!(category(name), want, "{name}");
    }
}

/// The new-row proof, and the general rule for what no row names.
#[test]
fn a_new_signed_row_names_a_kind_with_no_code_change() {
    let bytes = br#"{"format":1,"catalogue":"file-categories","version":2,"rows":[
        {"category":"photo","extensions":["jxr","jpg"]},
        {"category":"document","extensions":["jpg","blend"]},
        {"category":"other","extensions":["tmp"]},
        {"category":"photo","extensions":["J PG"]},
        {"category":"photo","extensions":[""]},
        {"category":"photo","extensions":["abcdefghijklmnopq"]},
        {"category":"hologram","extensions":["holo"]}
    ]}"#;
    let sig = test_key::sign(bytes, "file-categories", 2);
    let rows = Catalogue::<CategoryRow>::signed(bytes, &sig, &test_key::keys(), 1).unwrap();
    assert_eq!(rows.rows().len(), 2);
    assert_eq!(rows.report().skipped.len(), 5);
    let c = Categories::from_catalogue(&rows);
    assert_eq!(c.of("wedding.JXR"), Category::Photo);
    // The first row naming an extension wins.
    assert_eq!(c.of("a.jpg"), Category::Photo);
    assert_eq!(c.of("scene.blend"), Category::Document);
    assert_eq!(c.of("x.holo"), Category::Other);
    // With no rows at all, every file is still a file: Other.
    let none = Categories::default();
    assert_eq!(none.of("a.jpg"), Category::Other);
}
