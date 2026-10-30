//! Task List 2.5, mail and AI tools: a mail archive moves and the person opens it in the mail
//! app; a server mailbox's offline copy is never the mail, and folders kept only on the laptop
//! get the mail app's own export as a step; saved Apple Mail messages move whole and can be
//! written as mbox through the parser process; AI tools and local models are told from what was
//! found and the shipped rows, models checked and never run, tool settings held for a look.
//! Every byte here is invented.

mod common;

use common::*;
use pctwin_catalogue::Catalogue;
use pctwin_record::Owner;
use pctwin_special::catalogue::SpecialRow;
use pctwin_special::category::{self, Category, Note};
use pctwin_special::classify::{FolderReport, classify_all};
use pctwin_special::component::{Component, Custody, Evidence};
use pctwin_special::handoff::{self, Step};
use pctwin_special::roots::{System, find_folders, roots_of};

static OWN_READERS: pctwin_special::content::InThisProcess =
    pctwin_special::content::InThisProcess::new(pctwin_parse_seal::in_child());

fn me() -> Owner {
    Owner::Person {
        account_id: "S-1-5-21-0-0-0-1001".into(),
    }
}

fn discover(home: &std::path::Path, cat: &Catalogue<SpecialRow>) -> Vec<FolderReport> {
    pctwin_special::content::install_reader(&OWN_READERS);
    let roots = roots_of(System::Windows, home, &|_| None);
    let (folders, _) = find_folders(&me(), home, &roots, &[]);
    classify_all(folders, System::Windows, cat)
}

fn shipped() -> Catalogue<SpecialRow> {
    Catalogue::shipped(pctwin_special::catalogue::SHIPPED)
}

fn none() -> Catalogue<SpecialRow> {
    Catalogue::shipped(r#"{"format": 1, "catalogue": "special", "version": 2, "rows": []}"#)
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

fn emlx(subject: &str) -> Vec<u8> {
    let msg = format!("From: a@example.org\nSubject: {subject}\n\nhello\n");
    let mut b = format!("{}\n", msg.len()).into_bytes();
    b.extend_from_slice(msg.as_bytes());
    b.extend_from_slice(b"<?xml version=\"1.0\"?>\n<plist version=\"1.0\"><dict/></plist>\n");
    b
}

#[test]
fn a_mail_archive_moves_and_the_person_opens_it_in_the_mail_app() {
    let t = temp();
    let home = home_of(&t);
    put(
        &home.join("AppData/Local/Desk Mail/archive.pst"),
        b"!BDN\0\0\0\0SM\x17\0 invented archive",
    );
    put(
        &home.join("AppData/Local/Desk Mail/work.ost"),
        b"!BDN\0\0\0\0SO\x24\0 invented offline mailbox",
    );
    let r = discover(&home, &none());
    let desk = folder(&r, "Desk Mail");
    assert_eq!(category::of_folder(desk), Category::Email);

    let pst = holding(desk, "archive.pst");
    assert_eq!(pst.custody, Custody::Portable);
    assert!(pst.evidence.contains(&Evidence::MailArchive));
    assert!(pst.evidence.contains(&Evidence::MailStore));
    let notes = category::notes_for(desk, pst);
    assert!(notes.contains(&Note::OpenArchiveInMailApp), "{notes:?}");
    assert!(notes.contains(&Note::SearchRebuilds));
    assert!(!notes.contains(&Note::ServerResync));
    assert!(!notes.contains(&Note::LocalOnlyFoldersExport));

    // The offline copy is never the mail: it comes back from the server, and what was kept
    // only on this laptop is exported with the mail app's own export first.
    let ost = holding(desk, "work.ost");
    assert_eq!(ost.custody, Custody::ServiceHeld);
    assert!(!ost.evidence.contains(&Evidence::MailArchive));
    let notes = category::notes_for(desk, ost);
    assert!(notes.contains(&Note::ServerResync));
    assert!(notes.contains(&Note::LocalOnlyFoldersExport));
    assert!(!notes.contains(&Note::OpenArchiveInMailApp));
    assert!(matches!(handoff::step_for(desk, ost), Step::SignIn { .. }));
    assert_eq!(handoff::step_for(desk, pst), Step::None);
}

#[test]
fn saved_apple_mail_messages_move_whole_as_one_part() {
    let t = temp();
    let home = home_of(&t);
    let mb = home.join("AppData/Roaming/Post Box/Mailboxes/Trips.mbox/Messages");
    for i in 1..=3 {
        put(&mb.join(format!("{i}.emlx")), &emlx(&format!("trip {i}")));
    }
    let r = discover(&home, &none());
    let post = folder(&r, "Post Box");
    assert_eq!(category::of_folder(post), Category::Email);
    let one = holding(post, "Mailboxes/Trips.mbox/Messages/1.emlx");
    assert_eq!(one.members.len(), 3, "{:?}", one.members);
    assert_eq!(one.custody, Custody::Portable);
    assert!(category::notes_for(post, one).contains(&Note::SearchRebuilds));
}

#[test]
fn saved_messages_are_written_as_mbox_through_the_installed_reader() {
    pctwin_special::content::install_reader(&OWN_READERS);
    let t = temp();
    let src = t.path().join("Mail");
    let out = t.path().join("out");
    std::fs::create_dir_all(&out).unwrap();
    put(
        &src.join("Mailboxes/Trips.mbox/Messages/1.emlx"),
        &emlx("one"),
    );
    put(
        &src.join("Mailboxes/Trips.mbox/Messages/2.emlx"),
        &emlx("two"),
    );
    let r = pctwin_special::mail::export_mailboxes(
        &src,
        &out,
        &pctwin_special::content::emlx_facts_of,
        std::time::Instant::now() + std::time::Duration::from_secs(60),
    )
    .unwrap();
    assert_eq!(r.mailboxes.len(), 1);
    assert_eq!(r.mailboxes[0].written, 2);
    assert_eq!(
        r.mailboxes[0].result().status(),
        pctwin_types::ItemStatus::Converted
    );
    let text = std::fs::read_to_string(r.mailboxes[0].file.as_ref().unwrap()).unwrap();
    assert!(text.starts_with("From - "));
    assert!(text.contains("Subject: one") && text.contains("Subject: two"));
}

#[test]
fn ai_tools_from_the_shipped_rows_models_checked_and_tool_settings_held() {
    let t = temp();
    let home = home_of(&t);
    // A local model app's own folder: a model file kept by publisher and name.
    put(
        &home.join(".lmstudio/models/example-pub/tiny-model/tiny.gguf"),
        b"GGUF\x03\x00\x00\x00 invented weights",
    );
    // An AI editor: plain settings move; its tool servers start programs and are held.
    put(
        &home.join("AppData/Roaming/Cursor/User/settings.json"),
        br#"{"editor.fontSize": 14}"#,
    );
    put(
        &home.join(".cursor/mcp.json"),
        br#"{"mcpServers":{"local":{"command":"node","args":["server.js"]}}}"#,
    );
    // A new AI tool known only by a shipped row.
    put(
        &home.join(".continue/config.json"),
        br#"{"tabAutocompleteOptions": {"debounceDelay": 300}}"#,
    );
    let r = discover(&home, &shipped());

    let lm = folder(&r, ".lmstudio");
    assert_eq!(lm.name(), "LM Studio");
    assert_eq!(category::of_folder(lm), Category::AiTool);
    let model = holding(lm, "models/example-pub/tiny-model/tiny.gguf");
    assert!(model.evidence.contains(&Evidence::ModelData));
    assert_eq!(model.custody, Custody::Portable);
    assert!(category::notes_for(lm, model).contains(&Note::ModelsCheckedNotRun));

    let cursor = folder(&r, "Cursor");
    assert_eq!(category::of_folder(cursor), Category::AiTool);
    let dot = folder(&r, ".cursor");
    assert_eq!(category::of_folder(dot), Category::AiTool);
    let mcp = holding(dot, "mcp.json");
    assert!(mcp.evidence.contains(&Evidence::EnablesCommands));
    assert!(category::notes_for(dot, mcp).contains(&Note::ToolSettingsReview));

    let cont = folder(&r, ".continue");
    assert_eq!(category::of_folder(cont), Category::AiTool);
    assert!(!cont.name().is_empty());
}

#[test]
fn an_ai_tools_key_in_its_settings_never_moves_as_a_plain_setting() {
    let t = temp();
    let home = home_of(&t);
    put(
        &home.join(".continue/config.json"),
        br#"{"models":[{"provider":"example","apiKey":"sk-0123456789abcdefghijklmnopqrstuv"}]}"#,
    );
    let r = discover(&home, &shipped());
    let cont = folder(&r, ".continue");
    let cfg = holding(cont, "config.json");
    assert_ne!(cfg.custody, Custody::Portable, "{:?}", cfg.evidence);
    assert!(cfg.evidence.contains(&Evidence::CredentialValues));
}
