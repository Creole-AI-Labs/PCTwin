//! Data that links to files elsewhere (T25-003, T25-006, T25-042, T25-046): a catalog whose
//! originals are in other folders, a library whose attachments are outside the chosen roots, a
//! scene file whose media is gone. Found by the paths the data itself names, for any app.

mod common;

use std::path::Path;

use common::*;
use pctwin_catalogue::Catalogue;
use pctwin_record::Owner;
use pctwin_special::catalogue::{SHIPPED, SpecialRow};
use pctwin_special::classify::{FolderReport, classify_all};
use pctwin_special::component::{Component, Custody, Evidence};
use pctwin_special::content::local_path;
use pctwin_special::roots::{System, find_folders, roots_of};

fn found(home: &Path) -> Vec<FolderReport> {
    let owner = Owner::Person {
        account_id: "1001".into(),
    };
    let (folders, _) = find_folders(
        &owner,
        home,
        &roots_of(System::Windows, home, &|_| None),
        &[],
    );
    classify_all(
        folders,
        System::Windows,
        &Catalogue::<SpecialRow>::shipped(SHIPPED),
    )
}

fn holding<'a>(r: &'a [FolderReport], member: &str) -> &'a Component {
    r.iter()
        .flat_map(|f| f.components.iter())
        .find(|c| c.members.iter().any(|m| m == member))
        .unwrap()
}

fn sql_text(p: &Path) -> String {
    p.to_string_lossy().replace('\'', "''")
}

#[test]
fn a_catalog_whose_originals_are_elsewhere_or_gone_says_so() {
    pctwin_special::content::install_reader(&OWN_READERS);
    let t = temp();
    let home = home_of(&t);
    let app = home.join("AppData/Roaming/Photo Thing");
    let inside = app.join("previews/p1.jpg");
    put(&inside, b"preview");
    let original = home.join("Pictures/2026/beach.jpg");
    put(&original, b"original");
    let gone = home.join("Pictures/2025/lost.jpg");
    // A drive letter this laptop does not have now (an external drive not connected).
    let absent = (b'D'..=b'Z')
        .rev()
        .map(|c| format!("{}:\\", c as char))
        .find(|d| !Path::new(d).exists())
        .unwrap();
    let offline = format!(r"{absent}Photos\old.jpg");
    sqlite(
        &app.join("catalog.db"),
        &format!(
            "CREATE TABLE files(id INTEGER, path TEXT, note TEXT);
             INSERT INTO files VALUES(1, '{}', 'x'), (2, '{}', 'not/a/full/path'), (3, '{}', '/api/v1'), (4, '{}', NULL);",
            sql_text(&inside),
            sql_text(&original),
            sql_text(&gone),
            offline.replace('\'', "''")
        ),
    );
    let r = found(&home);
    let c = holding(&r, "catalog.db");
    assert_eq!(c.custody, Custody::Portable);
    // T25-003 / T25-042: an original is not on this laptop: a missing part, so never ready.
    assert!(
        c.evidence
            .iter()
            .any(|e| matches!(e, Evidence::MissingPart { what } if what.starts_with("1 file")))
    );
    // T25-011: on a drive that is not connected: said so, never "found everything".
    assert!(
        c.evidence
            .iter()
            .any(|e| matches!(e, Evidence::MissingPart { what } if what.contains("not connected")))
    );
    // T25-006 / T25-046: originals in another folder are offered, never added silently.
    let outside = c
        .evidence
        .iter()
        .find_map(|e| match e {
            Evidence::LinksOutside { folders } => Some(folders.clone()),
            _ => None,
        })
        .unwrap();
    assert_eq!(
        outside,
        vec![original.parent().unwrap().to_string_lossy().into_owned()]
    );
}

#[test]
fn a_scene_file_whose_media_is_gone_says_so() {
    pctwin_special::content::install_reader(&OWN_READERS);
    // T25-044: a profile or scene that moved without its media.
    let t = temp();
    let home = home_of(&t);
    let gone = home.join("Videos/intro.mp4");
    put(
        &home.join("AppData/Roaming/Streamer/scenes/main.json"),
        serde_json::json!({"sources": [{"name": "Intro", "settings": {"file": gone.to_string_lossy()}}]})
            .to_string()
            .as_bytes(),
    );
    put(
        &home.join("AppData/Roaming/Streamer/profile.ini"),
        b"[Output]\nMode=Simple\n",
    );
    let r = found(&home);
    let scene = holding(&r, "scenes/main.json");
    assert!(
        scene
            .evidence
            .iter()
            .any(|e| matches!(e, Evidence::MissingPart { .. }))
    );
    // A file that names nothing outside stays with the plain data.
    let profile = holding(&r, "profile.ini");
    assert_eq!(profile.evidence, vec![Evidence::NoSpecialShape]);
}

#[test]
fn only_full_local_paths_count() {
    pctwin_special::content::install_reader(&OWN_READERS);
    for yes in [
        r"C:\Users\a\b.jpg",
        "D:/x/y",
        "/home/a/b.jpg",
        "/Users/a/Pictures/x.png",
    ] {
        assert!(local_path(yes), "{yes}");
    }
    for no in [
        "relative/path.jpg",
        r"\\server\share\x",
        "//server/share",
        "https://example.org/a/b",
        "/api",
        "C:",
        "",
        "/a\n/b",
    ] {
        assert!(!local_path(no), "{no}");
    }
}

/// PCTwin's readers of files' contents, run in the test's own process (only a parser crate's
/// tests may).
static OWN_READERS: pctwin_special::content::InThisProcess =
    pctwin_special::content::InThisProcess::new(pctwin_parse_seal::in_child());
