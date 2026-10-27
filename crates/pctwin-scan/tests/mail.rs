//! Mail stores are found first; the catalogue only names them (Phase 2 adoption, changes 7
//! and 8).
// Tests make files to test on.
#![allow(clippy::disallowed_methods)]

use std::path::PathBuf;

use pctwin_catalogue::{Catalogue, test_key};
use pctwin_scan::{
    MailAccounts, MailClientRow, MailStore, StoreKind, Walk, mac_mail_accounts_in,
    mapi_accounts_in, name_stores, names_mail_servers, office_version_keys,
    profile_mail_accounts_in, profile_paths_in, shape_of, shipped_mail_clients, worth_a_look,
};

#[test]
fn mapi_accounts_are_counted_from_the_profile_keys_of_any_version() {
    for version in ["16.0", "15.0", "17.0"] {
        let k = format!(
            "HKEY_CURRENT_USER\\Software\\Microsoft\\Office\\{version}\\Outlook\\Profiles\\Outlook"
        );
        let out = format!(
            "\r\n{k}\r\n{k}\\9375CFF0413111d3B88A00104B2A6676\r\n    {{ED475418}}    REG_BINARY    02000000\r\n\
{k}\\9375CFF0413111d3B88A00104B2A6676\\00000002\r\n    Account Name    REG_BINARY    61006400\r\n\
{k}\\9375CFF0413111d3B88A00104B2A6676\\00000003\r\n\
{k}\\9375CFF0413111d3B88A00104B2A6676\\00000003\\Extra\r\n\
{k}\\9375CFF0413111d3B88A00104B2A6676\\0000000G\r\n\
{k}\\9375CFF0413111d3B88A00104B2A6676\\00000003\\0000000A\r\n\
{k}\\9375CFF0413111d3B88A00104B2A6676\\\r\n"
        );
        assert_eq!(mapi_accounts_in(&out), 2, "{version}");
    }
    assert_eq!(mapi_accounts_in(""), 0);
}

#[test]
fn every_office_version_key_is_found() {
    let out = "\r\nHKEY_CURRENT_USER\\Software\\Microsoft\\Office\\16.0\r\n\
HKEY_CURRENT_USER\\Software\\Microsoft\\Office\\15.0\r\n\
HKEY_CURRENT_USER\\Software\\Microsoft\\Office\\17.0\r\n\
HKEY_CURRENT_USER\\Software\\Microsoft\\Office\\Common\r\n\
HKEY_CURRENT_USER\\Software\\Microsoft\\Office\\16.\r\n\
HKEY_CURRENT_USER\\Software\\Microsoft\\Office\\.0\r\n\
HKEY_CURRENT_USER\\Software\\Microsoft\\Office\\1a.0\r\n\
HKEY_CURRENT_USER\\Software\\Microsoft\\Office\\16.x\r\n\
    16.0    REG_SZ    x\r\n\
    Path    REG_SZ    C:\\Office\\16.0\r\n";
    let keys = office_version_keys(out);
    let last: Vec<&str> = keys
        .iter()
        .map(|k| k.rsplit('\\').next().unwrap())
        .collect();
    assert_eq!(last, ["16.0", "15.0", "17.0"]);
}

#[test]
fn profile_stores_count_mail_servers_by_the_general_rule_or_a_row() {
    let prefs = r#"
user_pref("mail.server.server1.type", "none");
user_pref("mail.server.server2.type", "imap");
user_pref("mail.server.server3.type", "pop3");
user_pref("mail.server.server4.type", "rss");
user_pref("mail.server.server5.type", "nntp");
user_pref("mail.server.server6.type", "jmap");
user_pref("mail.server.server7.type", "movemail");
user_pref("mail.server.server8.type", "");
user_pref("mail.server.server2.hostname", "imap.example.com");
"#;
    // No row: every kind but local folders, news and feeds (a new kind, jmap, counts).
    assert_eq!(profile_mail_accounts_in(prefs, &[]), 3);
    // A row's kinds, compared ignoring case.
    assert_eq!(
        profile_mail_accounts_in(prefs, &["IMAP".into(), "pop3".into()]),
        2
    );
    assert!(names_mail_servers(prefs));
    assert!(!names_mail_servers(
        r#"user_pref("browser.startup.page", 3);"#
    ));
    let ini = "[Profile0]\nPath=Profiles/abc.default\n[Profile1]\nPath=../escape\nPath=/abs\nPath=C:/x\nPath=\\\\server\nPath=\nPath=  ok  \n";
    assert_eq!(
        profile_paths_in(ini),
        [PathBuf::from("Profiles/abc.default"), PathBuf::from("ok")]
    );
}

#[test]
fn mail_is_told_by_its_shape() {
    let maildir: Vec<String> = ["cur", "new", "tmp", "dovecot.index"]
        .iter()
        .map(|s| s.to_string())
        .collect();
    assert_eq!(shape_of("", Some(&maildir), &[]), Some(StoreKind::Maildir));
    assert_eq!(shape_of("", Some(&maildir[..2]), &[]), None);
    let saved = vec!["Invoice.EML".to_string()];
    assert_eq!(
        shape_of("", Some(&saved), &[]),
        Some(StoreKind::SavedMessages)
    );
    assert_eq!(
        shape_of("Archive.PST", None, b"!BDN\x01"),
        Some(StoreKind::OutlookDataFile)
    );
    assert_eq!(
        shape_of("ada@example.com.ost", None, b"!BDN"),
        Some(StoreKind::OutlookOfflineCache)
    );
    // Only named so, or the right start under another name: not an Outlook file.
    assert_eq!(shape_of("fake.pst", None, b"hello"), None);
    assert_eq!(shape_of("x.dat", None, b"!BDN"), None);
    assert_eq!(
        shape_of("old.olm", None, b"PK\x03\x04"),
        Some(StoreKind::OutlookMacArchive)
    );
    assert_eq!(shape_of("old.olm", None, b"PK"), None);
    assert_eq!(
        shape_of("Inbox.mbox", None, b"From MAILER-DAEMON Thu\nX"),
        Some(StoreKind::Mbox)
    );
    assert_eq!(shape_of("Inbox.mbox", None, b"From nobody"), None);
    assert!(worth_a_look("a.PST") && worth_a_look("b.mbox") && worth_a_look("c.olm"));
    assert!(!worth_a_look("Inbox") && !worth_a_look("notes.txt"));
    assert_eq!(
        mac_mail_accounts_in(&[
            "MailData".into(),
            ".DS_Store".into(),
            "IMAP-ada@imap.example.com".into(),
            "4F2C-UUID".into()
        ]),
        2
    );
}

fn store(kind: StoreKind, found_name: &str, accounts: Option<u32>) -> MailStore {
    MailStore {
        kind,
        found_name: found_name.into(),
        client: None,
        accounts,
        at: PathBuf::from(found_name),
    }
}

const PREFS: &str = r#"user_pref("mail.server.server1.type", "imap");
user_pref("mail.server.server2.type", "jmap");"#;

#[test]
fn every_store_found_is_reported_and_named_when_a_row_knows_it() {
    let found = vec![
        (store(StoreKind::MapiProfiles, "Outlook", Some(2)), None),
        (store(StoreKind::MapiProfiles, "Outlook", Some(1)), None),
        (
            store(StoreKind::ProfileStore, "Thunderbird", None),
            Some(vec![PREFS.to_string()]),
        ),
        (
            store(StoreKind::ProfileStore, "Kestrel Mail", None),
            Some(vec![PREFS.to_string(), PREFS.to_string()]),
        ),
        (store(StoreKind::Maildir, "offlineimap", None), None),
    ];
    let mail = name_stores(shipped_mail_clients(), found);
    let got: Vec<(Option<&str>, Option<u32>)> = mail
        .stores
        .iter()
        .map(|s| (s.client.as_deref(), s.accounts))
        .collect();
    assert_eq!(
        got,
        [
            (Some("Outlook"), Some(2)),
            (Some("Outlook"), Some(1)),
            // Thunderbird's row counts only its own kinds.
            (Some("Thunderbird"), Some(1)),
            // No row: found, not yet named, counted by the general rule.
            (None, Some(4)),
            (None, None),
        ]
    );
    // MAPI copies from an Office upgrade count once (the larger); the rest add up.
    assert_eq!(mail.total(), 2 + 1 + 4);
    let unnamed: Vec<&str> = mail
        .not_yet_named()
        .map(|s| s.found_name.as_str())
        .collect();
    assert_eq!(unnamed, ["Kestrel Mail", "offlineimap"]);
    assert_eq!(MailAccounts::default().total(), 0);
}

/// The new-row proof: a mail program added only as a signed row is named, with its own account
/// kinds, with no code change.
#[test]
fn a_new_signed_row_names_a_found_store() {
    assert!(shipped_mail_clients().report().is_clean());
    let bytes = br#"{"format":1,"catalogue":"mail-clients","version":2,"rows":[
        {"name":"Kestrel","found-as":["Kestrel Mail"],"account-kinds":["jmap"]},
        {"name":"","found-as":["x"]},
        {"name":"No names","found-as":[]},
        {"name":"Bad kind","found-as":["y"],"account-kinds":["im ap"]},
        {"name":"Empty kind","found-as":["z"],"account-kinds":[""]},
        {"name":"Dashes","found-as":["--"]}
    ]}"#;
    let sig = test_key::sign(bytes, "mail-clients", 2);
    let rows = Catalogue::<MailClientRow>::signed(
        bytes,
        &sig,
        &test_key::keys(),
        &pctwin_catalogue::MemoryMarks::at(1),
    )
    .unwrap();
    assert_eq!(rows.rows().len(), 1);
    assert_eq!(rows.report().skipped.len(), 5);
    let mail = name_stores(
        &rows,
        vec![(
            store(StoreKind::ProfileStore, "kestrel-mail", None),
            Some(vec![PREFS.to_string()]),
        )],
    );
    assert_eq!(mail.stores[0].client.as_deref(), Some("Kestrel"));
    assert_eq!(mail.stores[0].accounts, Some(1));
    assert_eq!(mail.not_yet_named().count(), 0);
}

#[test]
fn the_walk_finds_mail_by_shape_and_never_inside_a_store_already_found() {
    let dir = tempfile::tempdir().unwrap();
    let root = dir.path();
    let md = root.join("offlineimap").join("INBOX");
    for d in ["cur", "new", "tmp"] {
        std::fs::create_dir_all(md.join(d)).unwrap();
    }
    std::fs::create_dir_all(root.join("Saved")).unwrap();
    std::fs::write(root.join("Saved").join("a.eml"), b"x").unwrap();
    std::fs::create_dir_all(root.join("Office")).unwrap();
    std::fs::write(root.join("Office").join("Archive.pst"), b"!BDN....").unwrap();
    std::fs::write(root.join("Office").join("Fake.pst"), b"nope").unwrap();
    // A note that happens to start like mail is never opened: its name says it is not mail.
    std::fs::write(
        root.join("Office").join("letter.txt"),
        b"From the desk\nof Ada",
    )
    .unwrap();
    // Inside a maildir only the maildir counts, not what is in its folders.
    std::fs::write(md.join("cur").join("1.eml"), b"x").unwrap();
    let store = root.join("Bird");
    std::fs::create_dir_all(&store).unwrap();
    std::fs::write(store.join("Inbox.mbox"), b"From x\nhi").unwrap();
    let mut walk = Walk::new();
    walk.below(root, 0, &[store]);
    let mut got: Vec<(StoreKind, String)> = walk
        .found
        .iter()
        .map(|s| (s.kind, s.found_name.clone()))
        .collect();
    got.sort_by_key(|(_, n)| n.clone());
    assert_eq!(
        got,
        [
            (StoreKind::OutlookDataFile, "Office".to_string()),
            (StoreKind::SavedMessages, "Saved".to_string()),
            (StoreKind::Maildir, "offlineimap".to_string()),
        ]
    );
}

/// Reads this laptop's mail stores and prints counts only (never names or paths). Run by hand:
/// `cargo test -p pctwin-scan --test mail -- --ignored --nocapture`.
#[test]
#[ignore = "reads the real laptop"]
fn count_this_laptops_mail() {
    let home = dirs_home();
    let mail = pctwin_scan::mail_accounts(&home);
    println!(
        "mail stores {}, named {}, not yet named {}, accounts {}",
        mail.stores.len(),
        mail.stores.len() - mail.not_yet_named().count(),
        mail.not_yet_named().count(),
        mail.total()
    );
}

fn dirs_home() -> PathBuf {
    std::env::var_os("USERPROFILE")
        .or_else(|| std::env::var_os("HOME"))
        .map(PathBuf::from)
        .unwrap()
}

/// Fourth review, finding F5: mail past the walk's cap was dropped without a word. The folder
/// the walk stopped in, and every one after it, is now reported.
#[test]
fn f5_mail_past_the_walk_cap_is_reported_not_dropped() {
    let d = tempfile::tempdir().unwrap();
    let base = d.path().join("AppData");
    std::fs::create_dir_all(base.join("aaa")).unwrap();
    for i in 0..=pctwin_scan::MAX_LOOKED_AT {
        std::fs::write(base.join("aaa").join(format!("f{i:06}")), b"").unwrap();
    }
    std::fs::create_dir_all(base.join("zzz")).unwrap();
    std::fs::write(base.join("zzz").join("archive.pst"), b"!BDN....").unwrap();
    let mut w = Walk::new();
    w.below(&base, 0, &[]);
    w.below(&d.path().join("Later"), 0, &[]);
    assert!(w.found.is_empty());
    assert!(
        w.not_looked_at.contains(&base.join("aaa")),
        "{:?}",
        w.not_looked_at
    );
    assert!(
        w.not_looked_at.contains(&d.path().join("Later")),
        "{:?}",
        w.not_looked_at
    );
    // On its own, the store is found.
    let mut w = Walk::new();
    w.below(&base.join("zzz"), 0, &[]);
    assert_eq!(w.found.len(), 1);
    assert!(w.not_looked_at.is_empty());
}
