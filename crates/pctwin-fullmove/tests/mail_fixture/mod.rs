//! A synthetic old Mac's Apple Mail (`~/Library/Mail/V10`), and both halves of carrying it: the
//! old laptop's capture written as a package of the stream it sends, and the new laptop reading
//! that package into its staging folder and writing the mail out. Shared by the in-process test
//! (`tests/applemail.rs`) and the two-laptop CI lane (`examples/mail_lab.rs`: a macOS runner
//! sends, Windows and Linux runners receive).
#![allow(dead_code, clippy::disallowed_methods)]

use std::collections::BTreeMap;
use std::io::Read;
use std::path::{Path, PathBuf};
use std::sync::Arc;
use std::time::{Duration, Instant};

use pctwin_fullmove::data::{Choice, Landing, Sink, Source};
use pctwin_fullmove::extras::{SpecialLine, SpecialStep};
use pctwin_fullmove::mailout::{MailDone, MailOut, convert_mail};
use pctwin_journal::{Actor, Journal, Permission, Scope};
use pctwin_move::extras::{DataIn, DataOut, NextData};
use pctwin_record::Owner;
use pctwin_special::capture::{Holders, Holding};
use pctwin_special::content::ContentReader;

pub const OLD_LAPTOP: &str = "Ada's Mac";
pub const PLIST: &[u8] =
    b"<?xml version=\"1.0\"?>\n<plist><dict><key>flags</key><integer>1</integer></dict></plist>\n";

pub fn owner() -> Owner {
    Owner::Person {
        account_id: "501".into(),
    }
}

pub fn keys() -> &'static pctwin_grant::RoleKeys {
    static KEYS: std::sync::OnceLock<pctwin_grant::RoleKeys> = std::sync::OnceLock::new();
    KEYS.get_or_init(|| pctwin_grant::RoleKeys::mint().expect("the keys are made once"))
}

struct Free;
impl Holders for Free {
    fn holding(&self, _: &[PathBuf]) -> Holding {
        Holding::Free
    }
}

pub fn put(p: &Path, bytes: &[u8]) {
    std::fs::create_dir_all(p.parent().unwrap()).unwrap();
    std::fs::write(p, bytes).unwrap();
}

/// A saved message in Apple Mail's format: its length, the message, Mail's own notes.
pub fn emlx(message: &[u8]) -> Vec<u8> {
    let mut b = format!("{}\n", message.len()).into_bytes();
    b.extend_from_slice(message);
    b.extend_from_slice(PLIST);
    b
}

/// What the synthetic Mac holds, mailbox by mailbox, as the new laptop should show them.
pub struct Expect {
    /// Mailboxes kept only on the Mac: their name, and each message whole.
    pub local: BTreeMap<String, Vec<Vec<u8>>>,
    /// Mailboxes holding a message whose attachments were kept apart.
    pub partial: Vec<String>,
    /// Messages of the server account (never written out).
    pub server_messages: Vec<Vec<u8>>,
}

/// The mail index Apple Mail keeps (`MailData/Envelope Index`), in write-ahead mode with some of
/// its rows still only in its log: the database's and the log's bytes, as they are on disk while
/// Mail has it open.
pub fn index(urls: &[&str]) -> (Vec<u8>, Vec<u8>) {
    let t = tempfile::tempdir().unwrap();
    let p = t.path().join("Envelope Index");
    let c = rusqlite::Connection::open(&p).unwrap();
    c.execute_batch(
        "PRAGMA journal_mode=WAL; PRAGMA wal_autocheckpoint=0;
         CREATE TABLE mailboxes(ROWID INTEGER PRIMARY KEY, url TEXT UNIQUE, total_count INTEGER);
         CREATE TABLE messages(ROWID INTEGER PRIMARY KEY, mailbox INTEGER);
         PRAGMA wal_checkpoint(TRUNCATE);",
    )
    .unwrap();
    // The first address is in the database itself, the rest only in the log.
    for (i, u) in urls.iter().enumerate() {
        c.execute(
            "INSERT INTO mailboxes(url, total_count) VALUES (?1, 0)",
            [u],
        )
        .unwrap();
        if i == 0 {
            c.execute_batch("PRAGMA wal_checkpoint(TRUNCATE);").unwrap();
        }
    }
    // A row that is not text is passed over.
    c.execute(
        "INSERT INTO mailboxes(url, total_count) VALUES (x'696d61703a2f2f42',0)",
        [],
    )
    .unwrap();
    let db = std::fs::read(&p).unwrap();
    let wal = std::fs::read(t.path().join("Envelope Index-wal")).unwrap();
    drop(c);
    (db, wal)
}

/// The addresses of the synthetic index: an IMAP account (two mailboxes), a POP account, and a
/// mailbox On My Mac (no account folder). The account `GONE-UUID` was removed from Mail: it is
/// not in the index.
pub const URLS: &[&str] = &[
    "imap://IMAP-UUID/INBOX",
    "imap://IMAP-UUID/Sent%20Messages",
    "pop://POP-UUID/INBOX",
    "local:///Trips",
];

/// Builds `~/Library/Mail` in `home` as Apple Mail lays it out, and says what it holds.
pub fn old_mac(home: &Path) -> Expect {
    let mail = home.join("Library/Mail");
    let v = mail.join("V10");
    // An older layout beside it is not read.
    put(
        &mail.join("V9/Mailboxes/Old.mbox/Messages/1.emlx"),
        &emlx(b"Subject: old\n\nold\n"),
    );
    let mut local: BTreeMap<String, Vec<Vec<u8>>> = BTreeMap::new();
    let mut add = |mailbox: &str, rel: &str, msg: &[u8]| {
        put(&v.join(rel), &emlx(msg));
        local.entry(mailbox.into()).or_default().push(msg.to_vec());
    };
    // On My Mac: lines that would start a message inside bodies, names in other scripts, nested
    // mailboxes.
    add(
        "On My Mac/Trips",
        "Mailboxes/Trips.mbox/1F2E-AB/Data/Messages/1.emlx",
        b"From: ada@example.org\nSubject: Trip\n\nFrom the airport we drove.\n>From before\n>>From older\nbye\n",
    );
    add(
        "On My Mac/Trips",
        "Mailboxes/Trips.mbox/1F2E-AB/Data/Messages/2.emlx",
        "Subject: =?UTF-8?Q?=C3=89t=C3=A9?=\n\nCaf\u{e9} \u{65e5}\u{672c} \u{645}\u{631}\u{62d}\u{628}\u{627}\nFrom ".as_bytes(),
    );
    add(
        "On My Mac/\u{c9}t\u{e9} \u{65e5}\u{672c}",
        "Mailboxes/\u{c9}t\u{e9} \u{65e5}\u{672c}.mbox/Messages/5.emlx",
        b"Subject: summer\n\nsun\n",
    );
    add(
        "On My Mac/Work",
        "Mailboxes/Work.mbox/Messages/7.emlx",
        b"Subject: work\n\nplans\n",
    );
    add(
        "On My Mac/Work/Projects",
        "Mailboxes/Work.mbox/Projects.mbox/Messages/6.emlx",
        b"Subject: project\n\nFrom now on\n",
    );
    add(
        "On My Mac/Bills",
        "Mailboxes/Bills.mbox/Messages/4.emlx",
        b"Subject: bill\n\npaid\n",
    );
    // A message whose attachment Mail kept apart: carried, said as missing its attachment.
    let partial = b"Subject: invoice\nContent-Type: multipart/mixed; boundary=b\n\n--b\nContent-Type: application/pdf\nX-Apple-Content-Length: 12345\n\n--b--\n";
    add(
        "On My Mac/Bills",
        "Mailboxes/Bills.mbox/Messages/3.partial.emlx",
        partial,
    );
    put(
        &v.join("Mailboxes/Bills.mbox/Attachments/3/2/invoice.pdf"),
        b"%PDF-1.4 lab",
    );
    put(&v.join("Mailboxes/Bills.mbox/Info.plist"), PLIST);
    // The POP account (number 3 of the account folders, by name) keeps its mail on the Mac.
    add(
        "Mail account 3/INBOX",
        "POP-UUID/INBOX.mbox/9A9A/Data/0/Messages/10.emlx",
        b"Subject: pop\n\npop mail\n",
    );
    // The account removed from Mail (number 1): its mail is only on the Mac.
    add(
        "Mail account 1/Archive",
        "GONE-UUID/Archive.mbox/Messages/30.emlx",
        b"Subject: gone\n\nkept\n",
    );
    // The IMAP account (number 2): its mail is on its server.
    let server = b"Subject: imap\n\non the server\n".to_vec();
    put(
        &v.join("IMAP-UUID/INBOX.mbox/Messages/20.emlx"),
        &emlx(&server),
    );
    let (db, wal) = index(URLS);
    put(&v.join("MailData/Envelope Index"), &db);
    put(&v.join("MailData/Envelope Index-wal"), &wal);
    put(&v.join("MailData/Envelope Index-shm"), &[0u8; 64]);
    put(&v.join("MailData/Accounts.plist"), PLIST);
    Expect {
        local,
        partial: vec!["On My Mac/Bills".into()],
        server_messages: vec![server],
    }
}

/// Every file under `dir` with its bytes' hash: the tree, byte for byte.
pub fn tree(dir: &Path) -> BTreeMap<String, [u8; 32]> {
    let mut out = BTreeMap::new();
    let mut stack = vec![dir.to_path_buf()];
    while let Some(d) = stack.pop() {
        for e in std::fs::read_dir(&d).unwrap().flatten() {
            let p = e.path();
            if e.file_type().unwrap().is_dir() {
                stack.push(p);
            } else {
                let rel = p
                    .strip_prefix(dir)
                    .unwrap()
                    .to_string_lossy()
                    .replace('\\', "/");
                out.insert(rel, *blake3::hash(&std::fs::read(&p).unwrap()).as_bytes());
            }
        }
    }
    out
}

/// The old laptop: plans the person's app data on a Mac (only reading), takes the Apple Mail
/// line, captures what it chose and writes the stream it would send into `package`.
pub fn send(home: &Path, package: &Path) -> SpecialLine {
    let found = pctwin_fullmove::extras::special_of(
        pctwin_special::roots::System::Mac,
        &owner(),
        home,
        &[],
        Instant::now() + Duration::from_secs(120),
    );
    let line = found
        .lines
        .iter()
        .find(|l| l.step == SpecialStep::Convert)
        .cloned()
        .expect("the Apple Mail line");
    let choice = Choice {
        copy: line.ids.clone(),
        secrets: Vec::new(),
    };
    std::fs::create_dir_all(package.join("stream")).unwrap();
    std::fs::write(
        package.join("line.json"),
        serde_json::to_vec(&line).unwrap(),
    )
    .unwrap();
    let work = package.join("work");
    let mut source = Source::new(found.planned.clone(), &keys().capture, Arc::new(Free), work);
    source.chosen(choice.encode());
    let mut n = 0u32;
    loop {
        match source.next_within(Duration::from_secs(30)) {
            NextData::File(h, mut r) => {
                n += 1;
                let mut body = Vec::new();
                r.read_to_end(&mut body).unwrap();
                std::fs::write(package.join(format!("stream/{n:06}.h")), &h).unwrap();
                std::fs::write(package.join(format!("stream/{n:06}.b")), &body).unwrap();
            }
            NextData::Wait(_) => {}
            NextData::Done => break,
        }
    }
    drop(source);
    assert!(
        !package.join("work").exists(),
        "the old laptop's work folder is removed"
    );
    line
}

/// What the new laptop did with a package.
pub struct Received {
    pub line: SpecialLine,
    pub done: MailDone,
    pub staging: PathBuf,
}

/// The new laptop: reads the package into a sink in `staging` (inside `work`), and writes the
/// mail out into `home` through the real parser process.
pub fn receive(package: &Path, home: &Path, work: &Path) -> Received {
    receive_with(package, home, work, None)
}

/// [`receive`], with the parser's answer about the index replaced by `kinds` (a liar's).
pub fn receive_with(
    package: &Path,
    home: &Path,
    work: &Path,
    kinds: Option<pctwin_fullmove::mailout::KindsReader<'_>>,
) -> Received {
    let line: SpecialLine =
        serde_json::from_slice(&std::fs::read(package.join("line.json")).unwrap()).unwrap();
    let staging = work.join("fullmove-data-lab");
    let mut sink = Sink::new(
        Choice {
            copy: line.ids.clone(),
            secrets: Vec::new(),
        },
        staging.clone(),
    );
    let mut heads: Vec<PathBuf> = std::fs::read_dir(package.join("stream"))
        .unwrap()
        .flatten()
        .map(|e| e.path())
        .filter(|p| p.extension().is_some_and(|x| x == "h"))
        .collect();
    heads.sort();
    for h in heads {
        sink.begin(&std::fs::read(&h).unwrap()).unwrap();
        let body = std::fs::read(h.with_extension("b")).unwrap();
        for piece in body.chunks(64 * 1024) {
            sink.chunk(piece).unwrap();
        }
        sink.end().unwrap();
    }
    std::fs::create_dir_all(home.join("Documents")).unwrap();
    let journal = Journal::open_with_record_key(
        &work.join("journal.redb"),
        pctwin_journal::keystore::TagKey::from_bytes([0x5a; 32]),
    )
    .unwrap();
    let dest = pctwin_gate::Destination::open(home).unwrap();
    let parser = pctwin_parse::testing::parser();
    let done = convert_mail(
        &sink,
        &Landing {
            journal: &journal,
            key: &keys().capture,
            home: &dest,
            home_path: home,
            owner_only: &NoSecrets,
            scope: Scope::for_a_move().unwrap(),
            actor: Actor {
                acting_account: "1001".into(),
                for_account: "1001".into(),
                permission: Permission::OwnFolders,
            },
            system: pctwin_special::roots::System::here(),
        },
        &MailOut {
            line: &line.name,
            ids: &line.ids,
            parts: &line.parts,
            documents: "Documents",
            old_laptop: OLD_LAPTOP,
            words: pctwin_special::applemail::Words::default(),
            emlx: &|b| parser.emlx(b),
            kinds: kinds.unwrap_or(&|db, log| parser.mail_kinds(db, log)),
            deadline: Instant::now() + Duration::from_secs(120),
        },
    );
    sink.remove_staging();
    Received {
        line,
        done,
        staging,
    }
}

/// No secret is ever landed by the mail route.
struct NoSecrets;
impl pctwin_special::land::OwnerOnly for NoSecrets {
    fn private_folder(&self, _: &Path) -> std::io::Result<()> {
        Err(std::io::Error::other("mail is never a secret"))
    }
}

/// An mbox reader written apart from the writer: messages split at lines that start `From `,
/// the envelope line dropped, one `>` taken off each line that is `>`s then `From ` (mboxrd), and
/// the blank line the writer ends each message with removed.
pub fn read_mbox(bytes: &[u8]) -> Vec<Vec<u8>> {
    let mut out: Vec<Vec<u8>> = Vec::new();
    let mut lines: Vec<&[u8]> = bytes.split_inclusive(|b| *b == b'\n').collect();
    if lines.last().is_some_and(|l| l.is_empty()) {
        lines.pop();
    }
    for line in lines {
        if line.starts_with(b"From ") {
            out.push(Vec::new());
            continue;
        }
        let Some(cur) = out.last_mut() else {
            panic!("an mbox starts with an envelope line");
        };
        let gt = line.iter().take_while(|b| **b == b'>').count();
        if gt > 0 && line[gt..].starts_with(b"From ") {
            cur.extend_from_slice(&line[1..]);
        } else {
            cur.extend_from_slice(line);
        }
    }
    for m in &mut out {
        // The blank line after each message.
        assert!(
            m.ends_with(b"\n\n") || m.ends_with(b"\n"),
            "a message ends its line"
        );
        m.pop();
    }
    out
}

/// What landed in the new laptop's Documents: each mbox file's messages, by file name.
pub fn landed(home: &Path) -> (PathBuf, BTreeMap<String, Vec<Vec<u8>>>) {
    let folder = home
        .join("Documents")
        .join(format!("Mail from {OLD_LAPTOP}"));
    let mut out = BTreeMap::new();
    for e in std::fs::read_dir(&folder).unwrap().flatten() {
        let name = e.file_name().to_string_lossy().into_owned();
        out.insert(name, read_mbox(&std::fs::read(e.path()).unwrap()));
    }
    (folder, out)
}

/// The message as the writer ends it: with a line end.
pub fn ended(m: &[u8]) -> Vec<u8> {
    let mut m = m.to_vec();
    if !m.ends_with(b"\n") {
        m.push(b'\n');
    }
    m
}

/// Checks everything the new laptop should show and hold, given what the old Mac held. Returns
/// a plain report of what was checked.
pub fn check(expect: &Expect, home: &Path, r: &Received) -> String {
    use pctwin_fullmove::data::DataWhy;
    use pctwin_types::ItemStatus;
    let mut report = String::new();
    let lines: BTreeMap<&str, &pctwin_fullmove::mailout::MailLine> =
        r.done.lines.iter().map(|l| (l.title.as_str(), l)).collect();
    // Every mailbox kept only on the Mac has its line; nothing else does but the sign-in line.
    for (mailbox, msgs) in &expect.local {
        let l = lines
            .get(mailbox.as_str())
            .unwrap_or_else(|| panic!("a line for {mailbox}: {:?}", r.done.lines));
        let n = msgs.len() as u64;
        if expect.partial.contains(mailbox) {
            assert_eq!(l.result.status(), ItemStatus::NeedsAction, "{mailbox}");
            assert!(
                matches!(&l.why, Some(DataWhy::MailPartsMissing { messages, missing: 1, failed: 0, .. }) if *messages == n),
                "{mailbox}: {:?}",
                l.why
            );
        } else {
            assert_eq!(l.result.status(), ItemStatus::Converted, "{mailbox}");
            assert!(
                matches!(&l.why, Some(DataWhy::MailReady { messages, folder }) if *messages == n && folder.ends_with(&format!("Mail from {OLD_LAPTOP}"))),
                "{mailbox}: {:?}",
                l.why
            );
        }
        report.push_str(&format!(
            "{mailbox}: {:?}, {n} messages\n",
            l.result.status()
        ));
    }
    let sign_in = lines.get(r.line.name.as_str()).expect("the sign-in line");
    assert_eq!(sign_in.result.status(), ItemStatus::NeedsAction);
    assert_eq!(sign_in.why, Some(DataWhy::MailSignIn { accounts: 1 }));
    assert_eq!(
        r.done.lines.len(),
        expect.local.len() + 1,
        "{:?}",
        r.done.lines
    );
    assert!(
        r.done.effect.is_some(),
        "the landing is in the journal, for undo"
    );
    report.push_str("server account: sign in (needs-action), never copied\n");
    // What landed holds exactly the messages, read back by a reader of its own.
    let (folder, files) = landed(home);
    let mut all: Vec<Vec<u8>> = files.values().flatten().cloned().collect();
    all.sort();
    let mut want: Vec<Vec<u8>> = expect.local.values().flatten().map(|m| ended(m)).collect();
    want.sort();
    assert_eq!(all, want, "every message kept only on the Mac, whole");
    for m in &expect.server_messages {
        assert!(
            !all.contains(&ended(m)),
            "the server account's mail is not written"
        );
    }
    assert_eq!(files.len(), expect.local.len(), "one file per mailbox");
    report.push_str(&format!(
        "{} mbox files in {}, {} messages, read back whole\n",
        files.len(),
        folder.display(),
        all.len()
    ));
    // Nothing staged is left.
    assert!(!r.staging.exists(), "the staging folder is removed");
    report.push_str("staging removed\n");
    report
}
