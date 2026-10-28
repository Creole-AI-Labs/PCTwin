//! Gap 5 of the complications audit (28 October 2026): a setting that points at a moved file is
//! written only to point at where that file really landed, and only while it is there.

use pctwin_recipe::SettingValue;
use pctwin_settings::System;
use pctwin_settings::catalogue::Catalogue;
use pctwin_settings::classify::{Ctx, FolderMap, Landed, Relocate, Why};
use pctwin_settings::found::{Address, Discovery, FoundSetting, RegKind};
use pctwin_settings::generic::{FoundPlan, plan_found};

const OLD_DOCS: &str = "C:\\Users\\ada\\Documents";
const NEW_DOCS: &str = "D:\\Users\\ada2\\Documents";

fn ctx<'a>(paths: &'a dyn Relocate) -> Ctx<'a> {
    Ctx {
        paths,
        same_screens: false,
        hints: Catalogue::starter_hints(),
        facts: pctwin_settings::judge::Facts::system(),
    }
}

fn found(name: &str, path: &str) -> FoundSetting {
    FoundSetting {
        address: Address::Registry {
            key: "Software\\Vendor\\Editor".into(),
            name: name.into(),
            kind: RegKind::Sz,
        },
        label: "Editor".into(),
        value: SettingValue::Text(path.into()),
    }
}

/// The plan as made before the files move: pointed at the planned place by folder.
fn planned() -> FoundPlan {
    let map = FolderMap(vec![(OLD_DOCS.into(), NEW_DOCS.into())]);
    let d = Discovery {
        found: vec![
            found("LastFile", &format!("{OLD_DOCS}\\Report.docx")),
            found("Project", &format!("{OLD_DOCS}\\Thesis")),
            found("Template", &format!("{OLD_DOCS}\\Letter.dotx")),
            found("Gone", &format!("{OLD_DOCS}\\Old.txt")),
            found("Title", "My editor"),
        ],
        not_moved: Vec::new(),
    };
    let p = plan_found(
        &Catalogue::starter().unwrap(),
        System::Windows,
        System::Windows,
        &d,
        &ctx(&map),
    );
    // As planned: every path setting pointed at the planned name, by prefix.
    let value = |n: &str| {
        p.moves
            .iter()
            .find(|m| m.address.name() == n)
            .map(|m| m.value.clone())
    };
    assert_eq!(
        value("LastFile"),
        Some(SettingValue::Text(format!("{NEW_DOCS}\\Report.docx")))
    );
    assert_eq!(p.moves.len(), 5, "{:?}", p.not_moved);
    p
}

fn value(p: &FoundPlan, n: &str) -> Option<String> {
    p.moves
        .iter()
        .find(|m| m.address.name() == n)
        .map(|m| match &m.value {
            SettingValue::Text(t) => t.clone(),
            other => panic!("{other:?}"),
        })
}

#[test]
fn a_setting_points_only_at_where_its_file_really_landed() {
    let mut p = planned();
    // What really landed: Report.docx under "(2)" (a different one was there), the Thesis folder
    // as a whole beside one already there ("Thesis (2)"), the template under its own name but
    // removed since, and Old.txt never arrived.
    let files = vec![
        (
            format!("{OLD_DOCS}\\Report.docx"),
            format!("{NEW_DOCS}\\Report (2).docx"),
        ),
        (
            format!("{OLD_DOCS}\\Thesis\\chapter1.tex"),
            format!("{NEW_DOCS}\\Thesis (2)\\chapter1.tex"),
        ),
        (
            format!("{OLD_DOCS}\\Letter.dotx"),
            format!("{NEW_DOCS}\\Letter.dotx"),
        ),
    ];
    let here = [
        format!("{NEW_DOCS}\\Report (2).docx"),
        format!("{NEW_DOCS}\\Thesis (2)"),
        format!("{NEW_DOCS}\\Thesis (2)\\chapter1.tex"),
    ];
    let there = |p: &str| here.iter().any(|h| h == p);
    let landed = Landed {
        files: &files,
        there: &there,
    };
    let left = p.point_at_landed(&ctx(&landed));
    assert_eq!(
        value(&p, "LastFile").as_deref(),
        Some(&*format!("{NEW_DOCS}\\Report (2).docx"))
    );
    assert_eq!(
        value(&p, "Project").as_deref(),
        Some(&*format!("{NEW_DOCS}\\Thesis (2)"))
    );
    // Plain text is untouched.
    assert_eq!(value(&p, "Title").as_deref(), Some("My editor"));
    // Not there: never written, each said with why.
    assert_eq!(value(&p, "Template"), None);
    assert_eq!(value(&p, "Gone"), None);
    let mut names: Vec<&str> = left.iter().map(|n| n.name.as_str()).collect();
    names.sort();
    assert_eq!(names, ["Gone", "Template"]);
    assert!(left.iter().all(|n| n.why == Why::NotLanded));
    assert!(Why::NotLanded.words().contains("choose the file again"));
}

/// With nothing known to have landed (an empty record), no path setting is written at all.
#[test]
fn with_no_record_of_what_landed_no_path_setting_is_written() {
    let mut p = planned();
    let landed = Landed {
        files: &[],
        there: &|_| true,
    };
    let left = p.point_at_landed(&ctx(&landed));
    assert_eq!(left.len(), 4);
    assert_eq!(p.moves.len(), 1);
}

/// A folder whose files landed in different places (Documents on another drive, Pictures in
/// the new home) has no one new place: it is never pointed at a bare drive or at whichever file
/// came first. Fresh review of the gap fixes, finding F5 (28 October 2026).
#[test]
fn a_folder_whose_files_landed_apart_is_never_pointed_at() {
    let files = [
        (
            "C:\\Users\\ada\\Documents\\a.txt".to_string(),
            "D:\\Documents\\a.txt".to_string(),
        ),
        (
            "C:\\Users\\ada\\Pictures\\b.jpg".to_string(),
            "C:\\Users\\ada2\\Pictures\\b.jpg".to_string(),
        ),
    ];
    let mut reversed = files.clone();
    reversed.reverse();
    for files in [&files[..], &reversed[..]] {
        let landed = Landed {
            files,
            there: &|_| true,
        };
        assert_eq!(landed.new_path("C:\\Users\\ada"), None);
        // Each folder that did land whole is still found as itself.
        assert_eq!(
            landed.new_path("C:\\Users\\ada\\Pictures").as_deref(),
            Some("C:\\Users\\ada2\\Pictures")
        );
        assert_eq!(
            landed.new_path("C:\\Users\\ada\\Documents").as_deref(),
            Some("D:\\Documents")
        );
    }
    // A folder that would be a bare drive here is never pointed at.
    let root = [("C:\\Users\\ada\\a.txt".to_string(), "D:\\a.txt".to_string())];
    let landed = Landed {
        files: &root,
        there: &|_| true,
    };
    assert_eq!(landed.new_path("C:\\Users\\ada"), None);
}
