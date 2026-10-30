//! Apple Mail from a Mac (Task List 2.5, email), end to end on one machine through the same
//! engine the app runs: a synthetic `~/Library/Mail/V10` (mailboxes On My Mac, a POP account, an
//! IMAP account, an account removed from Mail, and Mail's index with rows still in its log) is
//! planned and captured on the "old Mac" (only read: the tree is byte for byte the same after),
//! carried as the stream the link carries, staged on the "new laptop", and written out there:
//!
//! - only the mail kept on the Mac arrives, as one mbox file per mailbox, every message whole
//!   (read back by an mbox reader of the test's own);
//! - the IMAP account is a sign-in line, never "copied";
//! - a message whose attachment Mail kept apart is counted as missing it;
//! - the index read in the real parser process; a damaged index or a lying parser keeps mail,
//!   never drops it, and never names a path;
//! - the staging folder is gone afterwards.
#![allow(clippy::disallowed_methods)]

mod mail_fixture;

use mail_fixture::*;
use pctwin_fullmove::data::DataWhy;
use pctwin_fullmove::extras::SpecialStep;
use pctwin_special::category::Note;
use pctwin_types::ItemStatus;

struct Lab {
    _t: tempfile::TempDir,
    old: std::path::PathBuf,
    new: std::path::PathBuf,
    package: std::path::PathBuf,
    work: std::path::PathBuf,
}

fn lab() -> Lab {
    let t = tempfile::tempdir().unwrap();
    let p = t.path().to_path_buf();
    Lab {
        old: p.join("old"),
        new: p.join("new"),
        package: p.join("package"),
        work: p.join("work"),
        _t: t,
    }
}

#[test]
fn only_the_mail_kept_on_the_mac_arrives_as_mailboxes_and_the_server_account_is_a_sign_in() {
    let l = lab();
    let expect = old_mac(&l.old);
    let before = tree(&l.old.join("Library/Mail"));
    let line = send(&l.old, &l.package);
    // The plan's line: converted here, its import the person's step, about twice its size in
    // room, and each component's folder named.
    assert_eq!(line.step, SpecialStep::Convert);
    assert!(line.notes.contains(&Note::MailImport));
    assert!(line.bytes > 0);
    assert_eq!(line.parts.len(), line.ids.len());
    for part in [
        "Mailboxes",
        "POP-UUID",
        "IMAP-UUID",
        "GONE-UUID",
        "MailData",
    ] {
        assert!(line.parts.iter().any(|p| p == part), "{part}");
    }
    // The old Mac's mail is only read.
    assert_eq!(tree(&l.old.join("Library/Mail")), before);
    // Only messages, attachments kept apart, and the index with its log were carried.
    let carried: Vec<String> = std::fs::read_dir(l.package.join("stream"))
        .unwrap()
        .flatten()
        .filter(|e| e.path().extension().is_some_and(|x| x == "h"))
        .map(|e| String::from_utf8(std::fs::read(e.path()).unwrap()).unwrap())
        .collect();
    let all = carried.join("\n");
    assert!(all.contains("Envelope Index-wal"));
    assert!(all.contains("invoice.pdf"));
    for never in ["V9", "Envelope Index-shm", "Accounts.plist", "Info.plist"] {
        assert!(!all.contains(never), "{never} is not carried");
    }
    let r = receive(&l.package, &l.new, &l.work);
    print!("{}", check(&expect, &l.new, &r));
}

#[test]
fn a_damaged_index_keeps_every_account_s_mail() {
    let l = lab();
    let expect = old_mac(&l.old);
    let v = l.old.join("Library/Mail/V10/MailData");
    put(
        &v.join("Envelope Index"),
        b"SQLite format 3\0 but not really a database at all",
    );
    std::fs::remove_file(v.join("Envelope Index-wal")).unwrap();
    send(&l.old, &l.package);
    let r = receive(&l.package, &l.new, &l.work);
    // The IMAP account's mail is written too (every doubt keeps mail), and no one is told to
    // sign in for mail that was not checked to be on a server.
    let imap = r
        .done
        .lines
        .iter()
        .find(|x| x.title == "Mail account 2/INBOX")
        .expect("the IMAP account's mailbox, kept");
    assert_eq!(imap.result.status(), ItemStatus::Converted);
    assert!(
        r.done
            .lines
            .iter()
            .all(|x| !matches!(x.why, Some(DataWhy::MailSignIn { .. })))
    );
    for mailbox in expect.local.keys() {
        assert!(
            r.done.lines.iter().any(|x| &x.title == mailbox),
            "{mailbox}"
        );
    }
    assert!(!r.staging.exists());
}

#[test]
fn a_log_from_another_database_never_loses_mail() {
    let l = lab();
    let expect = old_mac(&l.old);
    let (_, other_log) = index(&["imap://POP-UUID/INBOX", "imap://GONE-UUID/x"]);
    put(
        &l.old.join("Library/Mail/V10/MailData/Envelope Index-wal"),
        &other_log,
    );
    send(&l.old, &l.package);
    let r = receive(&l.package, &l.new, &l.work);
    // Whatever SQLite makes of the mismatched log, the POP account and the removed one are
    // never taken for server accounts.
    for mailbox in expect.local.keys() {
        let line = r.done.lines.iter().find(|x| &x.title == mailbox);
        assert!(line.is_some(), "{mailbox}: {:?}", r.done.lines);
    }
}

#[test]
fn a_parser_answer_naming_paths_or_control_characters_does_nothing() {
    let l = lab();
    let expect = old_mac(&l.old);
    send(&l.old, &l.package);
    let liar = |_: &[u8], _: Option<&[u8]>| {
        Ok(vec![
            pctwin_special::applemail::AccountScheme {
                folder: "../x".into(),
                scheme: "imap".into(),
            },
            pctwin_special::applemail::AccountScheme {
                folder: "..".into(),
                scheme: "imap".into(),
            },
            pctwin_special::applemail::AccountScheme {
                folder: "POP-UUID\u{7}".into(),
                scheme: "imap".into(),
            },
            pctwin_special::applemail::AccountScheme {
                folder: "IMAP-UUID".into(),
                scheme: "imap".into(),
            },
        ])
    };
    let r = receive_with(&l.package, &l.new, &l.work, Some(&liar));
    // Exactly as with the true answer: the names it gave matched no folder by equality.
    print!("{}", check(&expect, &l.new, &r));
    // Nothing was written anywhere but the new folder in Documents.
    let written: Vec<String> = tree(&l.new).into_keys().collect();
    assert!(
        written
            .iter()
            .all(|p| p.starts_with(&format!("Documents/Mail from {OLD_LAPTOP}/"))),
        "{written:?}"
    );
}

#[test]
fn the_index_is_read_in_the_parser_process_rows_in_its_log_included() {
    use pctwin_special::content::ContentReader;
    let (db, wal) = index(mail_fixture::URLS);
    let parser = pctwin_parse::testing::parser();
    let got = parser.mail_kinds(&db, Some(&wal)).unwrap();
    let pairs: Vec<(String, String)> = got.into_iter().map(|a| (a.folder, a.scheme)).collect();
    assert_eq!(
        pairs,
        vec![
            ("IMAP-UUID".into(), "imap".into()),
            ("POP-UUID".into(), "pop".into()),
        ]
    );
    // Without its log, only what reached the database itself.
    let only_db = parser.mail_kinds(&db, None).unwrap();
    assert_eq!(only_db.len(), 1);
    // Bytes that are not a database: refused, never a guess.
    assert!(parser.mail_kinds(b"not a database at all", None).is_err());
}

/// A Mac whose mail PCTwin may not look into (Full Disk Access off): the line says so and ends
/// needs-action, never "no mail".
#[cfg(unix)]
#[test]
fn mail_that_could_not_be_looked_into_is_said_never_zero() {
    use std::os::unix::fs::PermissionsExt;
    let l = lab();
    old_mac(&l.old);
    let mail = l.old.join("Library/Mail");
    std::fs::set_permissions(&mail, std::fs::Permissions::from_mode(0o000)).unwrap();
    // Running as the system's administrator reads it anyway: nothing to check then.
    let refused = std::fs::read_dir(&mail).is_err();
    let found = pctwin_fullmove::extras::special_of(
        pctwin_special::roots::System::Mac,
        &owner(),
        &l.old,
        &[],
        std::time::Instant::now() + std::time::Duration::from_secs(60),
    );
    std::fs::set_permissions(&mail, std::fs::Permissions::from_mode(0o755)).unwrap();
    if !refused {
        return;
    }
    let line = found
        .lines
        .iter()
        .find(|x| x.step == SpecialStep::Convert)
        .expect("the Apple Mail line, though nothing could be read");
    assert_eq!(line.notes, vec![Note::MailNoAccess]);
    assert!(line.ids.is_empty());
}
