//! The special categories (Task List 2.5): mail, browsers, games, AI tools and secrets are told
//! from what was found, never from an app's name, and each brings its notes; a catalogue row can
//! only name the category of a folder the rules leave as other. Every byte here is invented.

mod common;

use common::*;
use pctwin_catalogue::Catalogue;
use pctwin_record::Owner;
use pctwin_special::catalogue::SpecialRow;
use pctwin_special::category::{self, Category, Note};
use pctwin_special::classify::{FolderReport, classify_all};
use pctwin_special::component::{Component, Custody, Evidence};
use pctwin_special::roots::{System, find_folders, roots_of};

static OWN_READERS: pctwin_special::content::InThisProcess =
    pctwin_special::content::InThisProcess::new(pctwin_parse_seal::in_child());

fn me() -> Owner {
    Owner::Person {
        account_id: "S-1-5-21-0-0-0-1001".into(),
    }
}

fn rows(text: &str) -> Catalogue<SpecialRow> {
    Catalogue::shipped(&format!(
        r#"{{"format": 1, "catalogue": "special", "version": 2, "rows": [{text}]}}"#
    ))
}

fn discover(home: &std::path::Path, cat: &Catalogue<SpecialRow>) -> Vec<FolderReport> {
    pctwin_special::content::install_reader(&OWN_READERS);
    let roots = roots_of(System::Windows, home, &|_| None);
    let (folders, _) = find_folders(&me(), home, &roots, &[]);
    classify_all(folders, System::Windows, cat)
}

fn folder<'a>(r: &'a [FolderReport], rel: &str) -> &'a FolderReport {
    r.iter()
        .find(|f| f.folder.rel == rel)
        .unwrap_or_else(|| panic!("folder {rel} not found"))
}

fn holding<'a>(f: &'a FolderReport, member: &str) -> &'a Component {
    f.components
        .iter()
        .find(|c| c.members.iter().any(|m| m == member))
        .unwrap_or_else(|| panic!("{member} is in no component of {}", f.folder.rel))
}

/// A browser profile shaped as real ones are: bookmarks whose nodes say `"type": "url"`, a
/// history of addresses with visit counts, cookies and saved passwords.
fn browser(root: &std::path::Path) {
    let d = root.join("Person 1");
    put(
        &d.join("Saved Places"),
        br#"{"checksum":"0","roots":{"bookmark_bar":{"children":[{"name":"Example","type":"url","url":"https://example.org/"}],"type":"folder"}},"version":1}"#,
    );
    sqlite(
        &d.join("Visited"),
        "CREATE TABLE urls(id INTEGER, url TEXT, title TEXT, visit_count INTEGER);",
    );
    sqlite(
        &d.join("Jar"),
        "CREATE TABLE cookies(host_key TEXT, name TEXT, path TEXT, expires_utc INTEGER);",
    );
    sqlite(
        &d.join("Keys"),
        "CREATE TABLE logins(origin_url TEXT, username_value TEXT, password_value BLOB);",
    );
}

#[test]
fn every_category_is_told_from_what_was_found_with_no_rows() {
    let t = temp();
    let home = home_of(&t);
    let app = home.join("AppData");
    mail_profile(&app.join("Roaming/Post Office"));
    put(
        &app.join("Local/Desk Mail/work.ost"),
        b"!BDN\0\0\0\0SO\x24\0 invented offline mailbox",
    );
    browser(&app.join("Local/Window Maker"));
    model_library(&home.join(".weights"), false);
    game_saves(&home.join("Saved Games/Synthetic Quest"));
    ssh_folder(&home.join(".ssh"));
    put(
        &app.join("Roaming/Tool Host/servers.json"),
        br#"{"servers":{"local":{"command":"node","args":["server.js"]}}}"#,
    );
    let r = discover(&home, &rows(""));

    // Mail: a mail store moves and its search is built again; a server mailbox comes back.
    let post = folder(&r, "Post Office");
    assert_eq!(category::of_folder(post), Category::Email);
    let inbox = holding(post, "Profiles/abc.default/Mail/Local Folders/Inbox");
    assert_eq!(inbox.custody, Custody::Portable);
    assert!(category::notes_for(post, inbox).contains(&Note::SearchRebuilds));
    let desk = folder(&r, "Desk Mail");
    let ost = holding(desk, "work.ost");
    assert_eq!(category::of(desk, ost), Category::Email);
    let notes = category::notes_for(desk, ost);
    assert!(notes.contains(&Note::ServerResync));
    // An offline copy of a server mailbox is never said to be mail that moves.
    assert!(!notes.contains(&Note::SearchRebuilds));
    assert_eq!(ost.custody, Custody::ServiceHeld);

    // A browser, by its bookmarks' and history's shapes: its own sync first; saved passwords
    // are a secret guided through the browser's own tools; passkeys come with the account.
    let web = folder(&r, "Window Maker");
    assert_eq!(category::of_folder(web), Category::Browser);
    let history = holding(web, "Person 1/Visited");
    assert!(history.evidence.contains(&Evidence::BrowsingData));
    assert!(
        holding(web, "Person 1/Saved Places")
            .evidence
            .contains(&Evidence::BrowsingData)
    );
    assert!(category::notes_for(web, history).contains(&Note::OwnSync { page: None }));
    let keys = holding(web, "Person 1/Keys");
    assert!(keys.secret);
    assert_eq!(category::of(web, keys), Category::Secret);
    let notes = category::notes_for(web, keys);
    assert!(notes.contains(&Note::PasswordsGuided), "{notes:?}");
    assert!(notes.contains(&Note::SecretTickEach));
    let account = web
        .components
        .iter()
        .find(|c| c.custody == Custody::ServiceHeld)
        .expect("a browser with sign-ins has its account");
    assert!(category::notes_for(web, account).contains(&Note::PasskeysGuided));

    // AI models: checked by fingerprint, never run.
    let models = folder(&r, ".weights");
    assert_eq!(category::of_folder(models), Category::AiTool);
    let lib = &models.components[0];
    assert!(category::notes_for(models, lib).contains(&Note::ModelsCheckedNotRun));

    // Games, by the system's own place for saves.
    let game = folder(&r, "Synthetic Quest");
    assert_eq!(category::of_folder(game), Category::Game);
    let save = holding(game, "Profile 1/slot1.sav");
    assert!(category::notes_for(game, save).contains(&Note::CloudSavesMayDiffer));

    // Secrets are secrets wherever they are.
    let ssh = folder(&r, ".ssh");
    let key = holding(ssh, "id_ed25519");
    assert_eq!(category::of(ssh, key), Category::Secret);

    // Settings that start programs are held for a look.
    let tool = folder(&r, "Tool Host");
    let servers = holding(tool, "servers.json");
    assert!(category::notes_for(tool, servers).contains(&Note::ToolSettingsReview));
    assert_eq!(category::of_folder(tool), Category::Other);

    // Mail outranks a browser's shapes in one folder (a mail program keeps history too).
    sqlite(
        &app.join("Roaming/Post Office/Profiles/abc.default/places.sqlite"),
        "CREATE TABLE moz_places(id INTEGER, url TEXT, title TEXT, visit_count INTEGER);",
    );
    let r = discover(&home, &rows(""));
    assert_eq!(
        category::of_folder(folder(&r, "Post Office")),
        Category::Email
    );
}

#[test]
fn a_new_category_comes_from_one_catalogue_row_and_never_makes_a_secret() {
    let t = temp();
    let home = home_of(&t);
    put(
        &home.join("AppData/Roaming/Synthetic Editor/User/settings.json"),
        br#"{"editor.fontSize": 14}"#,
    );
    ssh_folder(&home.join(".ssh"));
    let none = discover(&home, &rows(""));
    assert_eq!(
        category::of_folder(folder(&none, "Synthetic Editor")),
        Category::Other
    );
    // One row, no code change: the editor is an AI tool or editor.
    let one = rows(
        r#"{"system":"windows","root":"roaming","folder":"Synthetic Editor","name":"Synthetic Editor","category":"ai-tool","checked_on":"2026-10-29"},
           {"system":"any","root":"home-dot","folder":".ssh","category":"other","checked_on":"2026-10-29"}"#,
    );
    let r = discover(&home, &one);
    let ed = folder(&r, "Synthetic Editor");
    assert_eq!(category::of_folder(ed), Category::AiTool);
    assert_eq!(category::of(ed, &ed.components[0]), Category::AiTool);
    // A row naming a folder "other" cannot make its keys anything but secret.
    let ssh = folder(&r, ".ssh");
    assert_eq!(
        category::of(ssh, holding(ssh, "id_ed25519")),
        Category::Secret
    );
    // Nor can a row's category outrank what the files say they are.
    mail_profile(&home.join("AppData/Roaming/Synthetic Editor/Mailbox"));
    let r = discover(&home, &one);
    assert_eq!(
        category::of_folder(folder(&r, "Synthetic Editor")),
        Category::Email
    );
}

#[test]
fn a_browsers_sync_page_comes_from_its_row() {
    let t = temp();
    let home = home_of(&t);
    browser(&home.join("AppData/Local/Window Maker"));
    let r = discover(
        &home,
        &rows(
            r#"{"system":"windows","root":"local","folder":"Window Maker","checked_on":"2026-10-29","relink":"https://example.org/sync"}"#,
        ),
    );
    let web = folder(&r, "Window Maker");
    let history = holding(web, "Person 1/Visited");
    assert!(category::notes_for(web, history).contains(&Note::OwnSync {
        page: Some("https://example.org/sync".into())
    }));
}

#[test]
fn the_shipped_rows_name_the_examples_categories() {
    let t = temp();
    let home = home_of(&t);
    put(
        &home.join(".ollama/models/manifests/x/latest"),
        br#"{"schemaVersion":2,"layers":[]}"#,
    );
    put(
        &home.join("AppData/Roaming/Code/User/settings.json"),
        br#"{"editor.tabSize": 4}"#,
    );
    let r = discover(
        &home,
        &Catalogue::shipped(pctwin_special::catalogue::SHIPPED),
    );
    let code = folder(&r, "Code");
    assert_eq!(code.name(), "Visual Studio Code");
    assert_eq!(category::of_folder(code), Category::AiTool);
    assert_eq!(category::of_folder(folder(&r, ".ollama")), Category::AiTool);
}

#[test]
fn a_browser_is_known_by_either_its_history_or_its_bookmarks_alone() {
    let t = temp();
    let home = home_of(&t);
    sqlite(
        &home.join("AppData/Local/Only History/Visited"),
        "CREATE TABLE urls(id INTEGER, url TEXT, title TEXT, visit_count INTEGER);",
    );
    put(
        &home.join("AppData/Local/Only Marks/Saved Places"),
        br#"{"roots":{"bar":{"children":[{"type":"url","url":"https://example.org/"}]}}}"#,
    );
    put(
        &home.join("AppData/Local/Neither/data.json"),
        br#"{"items":[{"type":"url","url":"https://example.org/"}]}"#,
    );
    let r = discover(&home, &rows(""));
    for (rel, file) in [("Only History", "Visited"), ("Only Marks", "Saved Places")] {
        let f = folder(&r, rel);
        assert_eq!(category::of_folder(f), Category::Browser, "{rel}");
        assert!(
            holding(f, file).evidence.contains(&Evidence::BrowsingData),
            "{rel}"
        );
    }
    assert_eq!(category::of_folder(folder(&r, "Neither")), Category::Other);
}

#[test]
fn notes_follow_how_each_part_ends_not_only_its_category() {
    let t = temp();
    let home = home_of(&t);
    let app = home.join("AppData");
    mail_profile(&app.join("Roaming/Post Office"));
    // A game folder that also keeps a sign-in token: that part is never a save.
    game_saves(&home.join("Saved Games/Synthetic Quest"));
    put(
        &home.join("Saved Games/Synthetic Quest/account.json"),
        br#"{"refresh_token":"invented-token-value-0123456789abcdef"}"#,
    );
    // A row that holds the mail program's local folders back as tied to this laptop.
    let strict = rows(
        r#"{"system":"windows","root":"roaming","folder":"Post Office","checked_on":"2026-10-29","exceptions":[{"members":"profiles/abc.default/mail/*","custody":"D","why":"invented: tied to this laptop"}]}"#,
    );
    let r = discover(&home, &strict);
    let post = folder(&r, "Post Office");
    let inbox = holding(post, "Profiles/abc.default/Mail/Local Folders/Inbox");
    assert_eq!(inbox.custody, Custody::MachineBound);
    assert!(!category::notes_for(post, inbox).contains(&Note::SearchRebuilds));
    let game = folder(&r, "Synthetic Quest");
    let token = holding(game, "account.json");
    assert_ne!(token.custody, Custody::Portable);
    assert!(!category::notes_for(game, token).contains(&Note::CloudSavesMayDiffer));
    let save = holding(game, "Profile 1/slot1.sav");
    assert!(category::notes_for(game, save).contains(&Note::CloudSavesMayDiffer));
}
