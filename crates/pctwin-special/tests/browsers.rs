//! Browsers (Task List 2.5; the 2.5 plan, section 7; T25-021 and T25-026): the same browser
//! takes its bookmarks and history as data while its sign-ins stay behind; for another browser,
//! bookmarks become the standard bookmarks file (`converted`), history is said to have no shared
//! format, and add-ons and open pages are listed for the person only, never opened. Every byte
//! here is invented.

mod common;

use std::path::{Path, PathBuf};
use std::sync::OnceLock;
use std::time::Duration;

use common::*;
use pctwin_catalogue::Catalogue;
use pctwin_gate::Destination;
use pctwin_grant::{CaptureEffect, Now, PlanSeal, RoleKeys, RootRef};
use pctwin_journal::{Actor, Journal, Permission, PlannedEffect, Scope};
use pctwin_recipe::{RecipeId, RecipeRef};
use pctwin_record::{ItemId, Owner};
use pctwin_special::api::*;
use pctwin_special::browser::{BOOKMARKS_FILE, LISTS_FILE, is_conversion};
use pctwin_special::capture::{Holders, Holding, Stop};
use pctwin_special::catalogue::{NO_ROWS, SpecialRow};
use pctwin_special::category::{Category, Note};
use pctwin_special::component::{Custody, Evidence};
use pctwin_special::land::{Conflict, OwnerOnly};
use pctwin_special::roots::System;
use pctwin_types::ItemStatus;

static OWN_READERS: pctwin_special::content::InThisProcess =
    pctwin_special::content::InThisProcess::new(pctwin_parse_seal::in_child());

fn keys() -> &'static RoleKeys {
    static K: OnceLock<RoleKeys> = OnceLock::new();
    K.get_or_init(|| RoleKeys::mint().expect("minted once in this test program"))
}

const MIN: Duration = Duration::from_secs(60);

fn seal() -> PlanSeal {
    PlanSeal::new(1, [7; 32])
}

struct Free;
impl Holders for Free {
    fn holding(&self, _: &[PathBuf]) -> Holding {
        Holding::Free
    }
}

struct AppOwnerOnly;
impl OwnerOnly for AppOwnerOnly {
    fn private_folder(&self, path: &Path) -> std::io::Result<()> {
        #[cfg(windows)]
        return pctwin_helper_os::private_folder(path)
            .map_err(|e| std::io::Error::other(format!("{e:?}")));
        #[cfg(unix)]
        return pctwin_special::land::UnixOwnerOnly.private_folder(path);
    }
}

fn effect(step: u32) -> PlannedEffect {
    PlannedEffect {
        scope: Scope([3; 16]),
        item: ItemId::from_hex(&format!("dd{}", "0".repeat(30))).unwrap(),
        recipe: RecipeRef {
            id: RecipeId::new("special.move").unwrap(),
            version: 1,
        },
        step,
        actor: Actor {
            acting_account: "1001".into(),
            for_account: "1001".into(),
            permission: Permission::OwnFolders,
        },
    }
}

fn owner() -> Owner {
    Owner::Person {
        account_id: "1001".into(),
    }
}

fn rows(text: &str) -> Catalogue<SpecialRow> {
    Catalogue::shipped(&format!(
        r#"{{"format": 1, "catalogue": "special", "version": 2, "rows": [{text}]}}"#
    ))
}

fn plan_of(home: &Path, cat: &Catalogue<SpecialRow>) -> Plan {
    pctwin_special::content::install_reader(&OWN_READERS);
    plan(&owner(), home, &[], System::Windows, System::Windows, cat)
}

fn holding<'a>(plan: &'a Plan, member_end: &str) -> &'a Planned {
    plan.components
        .iter()
        .filter(|p| !is_conversion(&p.component))
        .find(|p| p.component.members.iter().any(|m| m.ends_with(member_end)))
        .unwrap_or_else(|| panic!("{member_end} is in no component"))
}

fn conversion<'a>(plan: &'a Plan, app: &str) -> &'a Planned {
    plan.components
        .iter()
        .find(|p| is_conversion(&p.component) && p.app == app)
        .unwrap_or_else(|| panic!("no conversion for {app}"))
}

/// A Mozilla LZ4 file of `json` (the session format).
fn moz_lz4(json: &[u8]) -> Vec<u8> {
    let mut out = b"mozLz40\0".to_vec();
    out.extend_from_slice(&(json.len() as u32).to_le_bytes());
    out.extend_from_slice(&lz4_flex::block::compress(json));
    out
}

const TAB: &str = "https://private-tab.example/inbox?id=42";
const ADD_ON: &str = "Invented Reader Mode";

/// A places-format profile with bookmarks (one a script), its session (open pages and a
/// session cookie), an add-ons list, cookies and saved-password keys.
fn places_profile(root: &Path) {
    nss_browser_profile(root);
    let p = root.join("Profiles/xyz.default");
    std::fs::remove_file(p.join("places.sqlite")).unwrap();
    sqlite(
        &p.join("places.sqlite"),
        "CREATE TABLE moz_places(id INTEGER PRIMARY KEY, url TEXT, title TEXT, visit_count INTEGER, last_visit_date INTEGER);
         CREATE TABLE moz_bookmarks(id INTEGER PRIMARY KEY, type INTEGER, fk INTEGER, parent INTEGER,
            position INTEGER, title TEXT, dateAdded INTEGER, guid TEXT);
         INSERT INTO moz_places VALUES (1,'https://news.example/','News',4,0),(2,'javascript:steal()','S',0,0);
         INSERT INTO moz_bookmarks VALUES
            (1,2,NULL,0,0,'',0,'root________'),
            (2,2,NULL,1,0,'toolbar',1600000000000000,'toolbar_____'),
            (3,1,1,2,0,'Daily <News> & more',1600000000000000,'aaaaaaaaaaaa'),
            (4,1,2,2,1,'Script',0,'bbbbbbbbbbbb');",
    );
    put(
        &p.join("sessionstore.jsonlz4"),
        &moz_lz4(
            format!(
                r#"{{"windows":[{{"tabs":[{{"entries":[{{"url":"{TAB}","title":"Inbox"}}],"index":1}}]}}],
                 "cookies":[{{"host":"private-tab.example","name":"sid","value":"live-session-cookie"}}]}}"#
            )
            .as_bytes(),
        ),
    );
    put(
        &p.join("extensions.json"),
        format!(
            r#"{{"schemaVersion":36,"addons":[{{"type":"extension","location":"app-profile","defaultLocale":{{"name":"{ADD_ON}"}}}}]}}"#
        )
        .as_bytes(),
    );
}

/// A Chromium-format profile whose bookmarks are the real file's shape, with an add-on whose
/// name is a message, and a session file in the format PCTwin does not list.
fn chromium_shaped(root: &Path) {
    chromium_profile(root);
    let d = root.join("Default");
    put(
        &d.join("Bookmarks"),
        br#"{"checksum":"0","version":1,"roots":{
            "bookmark_bar":{"type":"folder","name":"Bookmarks bar","date_added":"13300000000000000","children":[
              {"type":"url","name":"Recipes","url":"https://recipes.example/","date_added":"13300000000000000"},
              {"type":"url","name":"Disk","url":"file:///C:/Users/x/a.txt"}]},
            "other":{"type":"folder","name":"Other","children":[]}}}"#,
    );
    put(
        &d.join("Extensions/msg/2.0/manifest.json"),
        br#"{"manifest_version":3,"name":"__MSG_extName__","default_locale":"en","version":"2.0"}"#,
    );
    put(
        &d.join("Extensions/msg/2.0/_locales/en/messages.json"),
        br#"{"extName":{"message":"Invented Tab Saver"}}"#,
    );
    put(&d.join("Sessions/Session_1"), b"SNSS\x03\0\0\0 invented");
}

#[test]
fn the_same_browser_takes_bookmarks_and_history_and_leaves_sign_ins_behind() {
    // T25-021: portable bookmarks sit beside session tokens; the sign-ins never move as data.
    let t = temp();
    let home = home_of(&t);
    chromium_shaped(&home.join("AppData/Local/Chromey/User Data"));
    places_profile(&home.join("AppData/Roaming/Foxy"));
    let plan = plan_of(&home, &rows(""));

    for moves in [
        "Default/History",
        "Default/Bookmarks",
        "Profiles/xyz.default/places.sqlite",
    ] {
        let p = holding(&plan, moves);
        assert_eq!(p.component.custody, Custody::Portable, "{moves}");
        assert!(p.proposed && !p.component.secret, "{moves}");
        assert_eq!(p.category, Category::Browser);
    }
    for stays in [
        "Default/Cookies",
        "Default/Login Data",
        "Profiles/xyz.default/cookies.sqlite",
        "Profiles/xyz.default/logins.json",
        // A session file keeps the open pages' sign-ins and cookies: never moved as data.
        "Profiles/xyz.default/sessionstore.jsonlz4",
    ] {
        let p = holding(&plan, stays);
        assert!(!p.proposed, "{stays} is not proposed");
        assert!(
            p.component.custody != Custody::Portable || p.component.secret,
            "{stays}: {:?}",
            p.component.custody
        );
    }
    let session = holding(&plan, "Profiles/xyz.default/sessionstore.jsonlz4");
    assert!(session.component.evidence.contains(&Evidence::SessionStore));
}

/// The new laptop, for one test.
struct New {
    _j: Journal,
    home: Destination,
    path: PathBuf,
}

fn new_laptop(t: &tempfile::TempDir) -> New {
    let path = t.path().join("new");
    std::fs::create_dir_all(path.join("AppData/Roaming")).unwrap();
    std::fs::create_dir_all(path.join("AppData/Local")).unwrap();
    New {
        home: Destination::open(&path).unwrap(),
        _j: Journal::open_with_record_key(
            &t.path().join("j.redb"),
            pctwin_journal::keystore::TagKey::from_bytes([0x5b; 32]),
        )
        .unwrap(),
        path,
    }
}

/// Captures `p` on the old laptop and lands it on `new`, as the move does.
fn move_one(t: &tempfile::TempDir, new: &New, p: &Planned, step: u32) -> (Applied, PathBuf) {
    let k = &keys().capture;
    let stop = Stop::new();
    let approved = approve(p, seal(), RootRef::new(10), u64::from(step));
    let work = t.path().join(format!("work{step}"));
    std::fs::create_dir(&work).unwrap();
    let read = Grant::Ordinary(
        k.grant(
            seal(),
            vec![RootRef::new(10)],
            CaptureEffect::ReadPrivate,
            MIN,
            Now::real(),
        )
        .unwrap(),
    );
    let cap = capture_planned(p, read, &approved, &work, &Free, &stop, Now::real()).unwrap();
    // What travels beside the files says only counts: never a page, an address or an add-on.
    let record = serde_json::to_string(&cap).unwrap();
    for private in [TAB, ADD_ON, "Invented Tab Saver", "Recipes", "News"] {
        assert!(!record.contains(private), "{private} is in the record");
    }
    let rec = new._j.imports(&keys().capture);
    let made = MadeFolders::default();
    let n = NewLaptop {
        rec: &rec,
        home: &new.home,
        home_path: &new.path,
        root: RootRef::new(20),
        seal: seal(),
        system: System::Windows,
        owner_only: &AppOwnerOnly,
        stop: &stop,
        made: &made,
    };
    let import = Grant::Ordinary(
        k.grant(
            seal(),
            vec![RootRef::new(20)],
            CaptureEffect::Import,
            MIN,
            Now::real(),
        )
        .unwrap(),
    );
    let applied = apply(
        &n,
        import,
        &approved,
        &effect(step),
        p,
        &cap,
        &work,
        Conflict::KeepBoth,
        Now::real(),
    );
    let folder = applied
        .landed
        .as_ref()
        .map(|l| {
            l.folder
                .split('/')
                .fold(new.path.clone(), |p, part| p.join(part))
        })
        .unwrap_or_default();
    (applied, folder)
}

#[test]
fn bookmarks_become_the_standard_file_for_another_browser_and_end_converted() {
    let t = temp();
    let home = home_of(&t);
    chromium_shaped(&home.join("AppData/Local/Chromey/User Data"));
    places_profile(&home.join("AppData/Roaming/Foxy"));
    let plan = plan_of(&home, &rows(""));
    let new = new_laptop(&t);

    // A places database: its bookmarks, the script left out; the open page and add-on listed.
    let fox = conversion(&plan, "Foxy");
    assert!(fox.proposed && !fox.component.secret);
    assert_eq!(fox.category, Category::Browser);
    for n in [
        Note::BookmarksFile,
        Note::HistoryOwnImport,
        Note::PagesAndAddOnsListed,
    ] {
        assert!(fox.notes.contains(&n), "{n:?}");
    }
    assert!(!fox.notes.contains(&Note::OpenPagesBySync));
    // Nothing private is in what the plan says.
    let said = format!("{} {:?} {:?}", fox.why, fox.notes, fox.component.evidence);
    assert!(!said.contains(TAB) && !said.contains(ADD_ON));
    let (applied, folder) = move_one(&t, &new, fox, 1);
    assert_eq!(
        applied.result.status(),
        ItemStatus::Converted,
        "{}",
        applied.words
    );
    assert!(
        folder.starts_with(new.path.join("PCTwin brought over")),
        "{folder:?}"
    );
    let html = std::fs::read_to_string(folder.join("Profiles/xyz.default").join(BOOKMARKS_FILE))
        .unwrap_or_else(|e| panic!("{e}: {:?} {}", walk(&folder), applied.words));
    assert!(html.starts_with("<!DOCTYPE NETSCAPE-Bookmark-file-1>"));
    assert!(html.contains("<H1>Foxy</H1>"));
    assert!(html.contains("<DT><H3 ADD_DATE=\"1600000000\">toolbar</H3>"));
    assert!(html.contains(
        "<DT><A HREF=\"https://news.example/\" ADD_DATE=\"1600000000\">Daily &lt;News&gt; &amp; more</A>"
    ));
    assert!(!html.contains("javascript"));
    let lists =
        std::fs::read_to_string(folder.join("Profiles/xyz.default").join(LISTS_FILE)).unwrap();
    assert!(lists.contains(ADD_ON));
    assert!(lists.contains("https://private-tab.example/inbox?id=42"));
    // The session's cookie never left the parser.
    assert!(!lists.contains("live-session-cookie"));
    assert!(!lists.contains("<script"));
    // No session, cookie or saved password landed for another browser.
    let landed: Vec<String> = walk(&folder);
    assert_eq!(
        landed,
        vec![
            format!("Profiles/xyz.default/{BOOKMARKS_FILE}"),
            format!("Profiles/xyz.default/{LISTS_FILE}"),
        ]
    );

    // A Chromium-format profile: its bookmarks file, the file address left out; the add-on's
    // name from its messages; its session file said, not listed.
    let chromey = conversion(&plan, "Chromey");
    assert!(chromey.notes.contains(&Note::OpenPagesBySync));
    assert!(chromey.notes.contains(&Note::BookmarksFile));
    let (applied, folder) = move_one(&t, &new, chromey, 2);
    assert_eq!(
        applied.result.status(),
        ItemStatus::Converted,
        "{}",
        applied.words
    );
    let html =
        std::fs::read_to_string(folder.join("User Data/Default").join(BOOKMARKS_FILE)).unwrap();
    assert!(html.contains(">Recipes</A>"));
    assert!(html.contains("<DT><H3 ADD_DATE=\"1655526400\">Bookmarks bar</H3>"));
    assert!(!html.contains("file:"));
    let lists = std::fs::read_to_string(folder.join("User Data/Default").join(LISTS_FILE)).unwrap();
    assert!(lists.contains("Invented Tab Saver"));
    assert!(lists.contains("Synthetic add-on"));
}

fn walk(dir: &Path) -> Vec<String> {
    let mut out = Vec::new();
    let mut stack = vec![(dir.to_path_buf(), String::new())];
    while let Some((d, rel)) = stack.pop() {
        for e in std::fs::read_dir(&d).unwrap().flatten() {
            let name = e.file_name().to_string_lossy().into_owned();
            let r = if rel.is_empty() {
                name
            } else {
                format!("{rel}/{name}")
            };
            if e.file_type().unwrap().is_dir() {
                stack.push((e.path(), r));
            } else {
                out.push(r);
            }
        }
    }
    out.sort();
    out
}

#[test]
fn a_new_browser_works_from_one_catalogue_row_without_code() {
    // ADOPTION change 7: a browser PCTwin has no code for, added only as a row, is named and
    // converted end to end; with no rows, it is still found and converted by its formats alone.
    let t = temp();
    let home = home_of(&t);
    chromium_shaped(&home.join("AppData/Roaming/Brand New Thing"));
    let none = plan_of(&home, &Catalogue::<SpecialRow>::shipped(NO_ROWS));
    assert!(
        none.components
            .iter()
            .any(|p| is_conversion(&p.component) && p.app == "Brand New Thing")
    );
    let plan = plan_of(
        &home,
        &rows(
            r#"{"system": "windows", "root": "roaming", "folder": "Brand New Thing", "name": "Neo Browser",
                "category": "browser", "checked_on": "2026-10-30", "relink": "https://neo.example/sync"}"#,
        ),
    );
    let p = conversion(&plan, "Neo Browser");
    let new = new_laptop(&t);
    let (applied, folder) = move_one(&t, &new, p, 3);
    assert_eq!(applied.result.status(), ItemStatus::Converted);
    assert!(folder.ends_with("Neo Browser"), "{folder:?}");
    let html = std::fs::read_to_string(folder.join("Default").join(BOOKMARKS_FILE)).unwrap();
    assert!(html.contains("<H1>Neo Browser</H1>"));
    // Its own sync page comes from the same row.
    let history = holding(&plan, "Default/History");
    assert!(history.notes.contains(&Note::OwnSync {
        page: Some("https://neo.example/sync".into())
    }));
}

#[test]
fn a_folder_with_no_browser_data_gets_no_bookmarks_file_and_nothing_read_is_claimed() {
    let t = temp();
    let home = home_of(&t);
    sqlite(
        &home.join("AppData/Roaming/Journal App/journal.db"),
        "CREATE TABLE notes(body TEXT);",
    );
    // A browser with history only: history is said, no bookmarks file is promised.
    sqlite(
        &home.join("AppData/Local/History Only/Default/History"),
        "CREATE TABLE urls(id INTEGER, url TEXT, title TEXT, visit_count INTEGER);",
    );
    let plan = plan_of(&home, &rows(""));
    assert!(
        !plan
            .components
            .iter()
            .any(|p| is_conversion(&p.component) && p.app == "Journal App")
    );
    assert!(
        !plan
            .components
            .iter()
            .any(|p| is_conversion(&p.component) && p.app == "History Only"),
        "nothing to write for another browser, so nothing is planned"
    );
    let history = holding(&plan, "Default/History");
    assert_eq!(history.category, Category::Browser);
}

#[test]
fn a_bookmarks_file_that_changed_since_approval_is_not_written() {
    let t = temp();
    let home = home_of(&t);
    chromium_shaped(&home.join("AppData/Local/Chromey/User Data"));
    let plan = plan_of(&home, &rows(""));
    let p = conversion(&plan, "Chromey");
    let approved = approve(p, seal(), RootRef::new(10), 1);
    std::thread::sleep(Duration::from_millis(20));
    put(
        &home.join("AppData/Local/Chromey/User Data/Default/Bookmarks"),
        br#"{"roots":{"b":{"type":"folder","name":"b","children":[]}},"changed":true}"#,
    );
    let work = t.path().join("w");
    std::fs::create_dir(&work).unwrap();
    let k = &keys().capture;
    let read = Grant::Ordinary(
        k.grant(
            seal(),
            vec![RootRef::new(10)],
            CaptureEffect::ReadPrivate,
            MIN,
            Now::real(),
        )
        .unwrap(),
    );
    let got = capture_planned(p, read, &approved, &work, &Free, &Stop::new(), Now::real());
    assert!(
        matches!(
            got,
            Err(pctwin_special::capture::CaptureError::ChangedSinceApproval)
        ),
        "{got:?}"
    );
    // A grant for another root is refused too.
    let other = Grant::Ordinary(
        k.grant(
            seal(),
            vec![RootRef::new(11)],
            CaptureEffect::ReadPrivate,
            MIN,
            Now::real(),
        )
        .unwrap(),
    );
    let approved = approve(p, seal(), RootRef::new(10), 2);
    assert!(matches!(
        capture_planned(p, other, &approved, &work, &Free, &Stop::new(), Now::real()),
        Err(pctwin_special::capture::CaptureError::Denied(_))
    ));
    // While the browser holds its files open for writing, nothing is read: it waits for it to
    // close, as the rest of its data does.
    struct Held;
    impl Holders for Held {
        fn holding(&self, _: &[PathBuf]) -> Holding {
            Holding::Held {
                by: vec!["Chromey".into()],
            }
        }
    }
    let plan = plan_of(&home, &rows(""));
    let p = conversion(&plan, "Chromey");
    let approved = approve(p, seal(), RootRef::new(10), 3);
    let read = Grant::Ordinary(
        k.grant(
            seal(),
            vec![RootRef::new(10)],
            CaptureEffect::ReadPrivate,
            MIN,
            Now::real(),
        )
        .unwrap(),
    );
    assert!(matches!(
        capture_planned(p, read, &approved, &work, &Held, &Stop::new(), Now::real()),
        Err(pctwin_special::capture::CaptureError::AwaitingClose { .. })
    ));
    assert_eq!(std::fs::read_dir(&work).unwrap().count(), 0);
}
