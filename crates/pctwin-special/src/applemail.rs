//! Apple Mail between laptops (Task List 2.5, email; design of 30 October 2026).
//!
//! One route for a Mac to a Mac, Windows or Linux: the old laptop only **sends** Apple Mail's raw
//! files; the **new** laptop's parser process does all the reading, and the new laptop writes the
//! mail kept only on the old Mac as standard mailbox (mbox) files, which Apple Mail's own "Import
//! Mailboxes" and other mail apps take. Apple Mail's folder is never copied whole: on a new Mac it
//! would overwrite the mail already there.
//!
//! - **The old laptop** ([`find`]): the newest `~/Library/Mail/V<n>` (the same rule as the scan's
//!   mail count), and in it every saved message (`.emlx`, `.partial.emlx`) and attachment kept
//!   apart (`Attachments/`) under `Mailboxes/` and each account folder, plus the mail index
//!   (`MailData/Envelope Index` and its log), as components carried raw
//!   ([`Evidence::CarriedRaw`]: each file copied steady, nothing in it opened there). Only the
//!   file system's own facts are read; never `~/Library/Accounts` or the keychain. A folder the
//!   system will not let PCTwin look into (Full Disk Access is off) is said
//!   ([`Found::NoAccess`]), never taken for "no mail".
//! - **The parser process on the new laptop** ([`mail_kinds`]): the received index is opened by
//!   SQLite from memory (no file), and only `(account folder, kind of address)` pairs from its
//!   mailboxes' addresses come back. The engine matches each pair's folder name to the folders it
//!   received by exact equality; it never builds a path from the parser's answer.
//! - **Which mail is kept only on the old Mac** ([`local_only`]): `Mailboxes/` (On My Mac), an
//!   account whose kind the `mail-account-kinds` catalogue says keeps mail on the laptop (POP),
//!   an account the index does not know (deleted), an account of a kind no row names, and every
//!   account when the index could not be read. Every doubt keeps mail. Only an account every one
//!   of whose addresses is a server kind (IMAP, Exchange) is left to its server: the person signs
//!   in on the new laptop.

use std::collections::BTreeSet;
use std::path::{Path, PathBuf};
use std::time::Instant;

use pctwin_catalogue::{Catalogue, Row};
use pctwin_parse_seal::InChild;
use pctwin_record::Owner;
use serde::{Deserialize, Serialize};

use crate::component::{Component, Custody, Evidence};
use crate::content::ContentError;
use crate::guard::{self, Kind};

/// Apple Mail's folder, inside the home folder.
pub const MAIL_FOLDER: [&str; 2] = ["Library", "Mail"];
/// The mail index, inside `V<n>`.
pub const INDEX: &str = "MailData/Envelope Index";
/// The index's write-ahead log (its shared-memory file is not needed: SQLite rebuilds it).
pub const INDEX_LOG: &str = "MailData/Envelope Index-wal";
/// The folder of mailboxes kept on the Mac ("On My Mac"), inside `V<n>`.
pub const ON_THIS_MAC: &str = "Mailboxes";
/// The folder holding the index, inside `V<n>`.
const MAIL_DATA: &str = "MailData";
/// The most files in one carried component (its record must stay small enough to cross).
pub const CHUNK: usize = 20_000;
/// The most files carried in all.
pub const MAX_FILES: usize = 1_000_000;
/// The deepest folder looked into.
const MAX_DEPTH: usize = 24;
/// The most account folders the parser may name.
pub const MAX_ACCOUNTS: usize = 4096;
/// The most mailbox addresses read from the index.
pub const MAX_MAILBOXES: usize = 200_000;
/// The longest account folder name or address kind the parser may name.
pub const MAX_NAME: usize = 255;

/* ---------- The catalogue: which kinds of address keep mail on a server ---------- */

/// Where an account's mail is kept.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum MailKind {
    /// On the mail server: it comes back when the person signs in.
    Server,
    /// Only on the laptop.
    Local,
}

/// One row of the `mail-account-kinds` catalogue: the kind of address a mailbox has in Apple
/// Mail's index (`imap`, `pop`...), and where its mail is kept.
#[derive(Debug, Clone, PartialEq, Eq, Deserialize)]
pub struct KindRow {
    pub scheme: String,
    pub kind: MailKind,
    pub name: String,
    pub checked_on: String,
}

impl Row for KindRow {
    const CATALOGUE: &'static str = "mail-account-kinds";
    const FIELDS: &'static [&'static str] = &["scheme", "kind", "name", "checked_on"];

    fn check(&self) -> Result<(), String> {
        if !scheme_ok(&self.scheme) || self.scheme.bytes().any(|b| b.is_ascii_uppercase()) {
            return Err("a scheme is 1 to 32 lower-case letters, digits, `+`, `-` or `.`".into());
        }
        if self.name.is_empty() || self.name.len() > 100 {
            return Err("a name is 1 to 100 characters".into());
        }
        if !crate::catalogue::date_ok(&self.checked_on) {
            return Err("checked_on must be a date (YYYY-MM-DD)".into());
        }
        Ok(())
    }

    fn identity(&self) -> Option<String> {
        Some(self.scheme.clone())
    }
}

/// The shipped `mail-account-kinds` catalogue (compiled in, covered by the app's own signature).
pub const SHIPPED_KINDS: &str = include_str!("../catalogue/mail-account-kinds.json");

/// The shipped kinds.
pub fn shipped_kinds() -> Catalogue<KindRow> {
    Catalogue::shipped(SHIPPED_KINDS)
}

fn scheme_ok(s: &str) -> bool {
    !s.is_empty()
        && s.len() <= 32
        && s.bytes()
            .all(|b| b.is_ascii_alphanumeric() || matches!(b, b'+' | b'-' | b'.'))
}

/// The kind a catalogue row gives `scheme` (compared without case); `None`: no row names it.
pub fn kind_of(kinds: &Catalogue<KindRow>, scheme: &str) -> Option<MailKind> {
    kinds
        .rows()
        .iter()
        .find(|r| r.scheme.eq_ignore_ascii_case(scheme))
        .map(|r| r.kind)
}

/* ---------- The old laptop: only the file system's facts ---------- */

/// What the old laptop found of Apple Mail.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Found {
    /// No Apple Mail folder (or no `V<n>` in it).
    NotHere,
    /// The folder is there, but the system did not let PCTwin look inside (Full Disk Access is
    /// off on a Mac): never said as "no mail".
    NoAccess,
    Mail(MailFound),
}

/// Apple Mail's files to carry.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct MailFound {
    /// `~/Library/Mail/V<n>`: every component's files are inside it.
    pub base: PathBuf,
    pub components: Vec<Component>,
    /// Every carried file's size, added up.
    pub bytes: u64,
    pub files: u64,
    /// Not every file was looked at (too many, too deep, out of time, a folder that could not be
    /// read, a name that cannot cross): said, never "everything".
    pub not_all: bool,
}

/// The newest `V<n>` folder in Apple Mail's folder `mail` (by its number), or `None`.
pub fn newest_version(mail: &Path) -> std::io::Result<Option<String>> {
    let mut best: Option<(u32, String)> = None;
    for e in std::fs::read_dir(mail)?.flatten() {
        let name = e.file_name().to_string_lossy().into_owned();
        let Some(n) = name
            .strip_prefix('V')
            .filter(|d| !d.is_empty() && d.bytes().all(|b| b.is_ascii_digit()))
            .and_then(|d| d.parse::<u32>().ok())
        else {
            continue;
        };
        // A real folder only, never a link.
        let real =
            std::fs::symlink_metadata(e.path()).is_ok_and(|m| guard::kind_of(&m) == Kind::Folder);
        if real && best.as_ref().is_none_or(|(b, _)| n > *b) {
            best = Some((n, name));
        }
    }
    Ok(best.map(|(_, name)| name))
}

/// A name that can cross as part of a path: no separator, colon, control character or dot name.
fn plain_name(n: &str) -> bool {
    !n.is_empty()
        && n != "."
        && n != ".."
        && n.len() <= MAX_NAME
        && !n
            .chars()
            .any(|c| matches!(c, '/' | '\\' | ':') || c.is_control())
}

/// Whether a file under an account or `Mailboxes/` is carried: a saved message, or a file kept
/// apart in an `Attachments` folder.
fn carried(rel: &[String]) -> bool {
    let Some(name) = rel.last() else {
        return false;
    };
    name.to_ascii_lowercase().ends_with(".emlx")
        || rel[..rel.len() - 1].iter().any(|p| p == "Attachments")
}

/// Apple Mail's files of the person whose home folder is `home` (old laptop, read only: nothing
/// is opened, only listed), as components owned by `owner`, by `deadline`.
pub fn find(owner: &Owner, home: &Path, deadline: Instant) -> Found {
    let mail = MAIL_FOLDER
        .iter()
        .fold(home.to_path_buf(), |p, n| p.join(n));
    match std::fs::symlink_metadata(&mail) {
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => return Found::NotHere,
        Err(_) => return Found::NoAccess,
        Ok(m) if guard::kind_of(&m) != Kind::Folder => return Found::NotHere,
        Ok(_) => {}
    }
    let version = match newest_version(&mail) {
        Ok(Some(v)) => v,
        Ok(None) => return Found::NotHere,
        Err(_) => return Found::NoAccess,
    };
    let base = mail.join(&version);
    let Ok(tops) = std::fs::read_dir(&base) else {
        return Found::NoAccess;
    };
    let mut tops: Vec<String> = tops
        .flatten()
        .filter(|e| {
            std::fs::symlink_metadata(e.path()).is_ok_and(|m| guard::kind_of(&m) == Kind::Folder)
        })
        .map(|e| e.file_name().to_string_lossy().into_owned())
        .collect();
    tops.sort();
    let mut found = MailFound {
        base: base.clone(),
        components: Vec::new(),
        bytes: 0,
        files: 0,
        not_all: false,
    };
    let place = |top: &str| format!("Mail/{version}/{top}");
    let mut evidence = vec![Evidence::MailStore, Evidence::CarriedRaw];
    evidence.dedup();
    for top in &tops {
        if !plain_name(top) {
            found.not_all = true;
            continue;
        }
        if top == MAIL_DATA {
            let members: Vec<String> = [INDEX, INDEX_LOG]
                .into_iter()
                .filter(|rel| {
                    std::fs::symlink_metadata(base.join(rel))
                        .is_ok_and(|m| guard::kind_of(&m) == Kind::File)
                })
                .map(str::to_string)
                .collect();
            if members.first().map(String::as_str) == Some(INDEX) {
                for m in &members {
                    found.bytes += std::fs::symlink_metadata(base.join(m)).map_or(0, |m| m.len());
                }
                found.files += members.len() as u64;
                found.components.push(Component::new(
                    owner.clone(),
                    &place(top),
                    members,
                    Custody::NeedsExport,
                    false,
                    evidence.clone(),
                ));
            }
            continue;
        }
        let mut members = Vec::new();
        let mut stack = vec![(vec![top.clone()], 0usize)];
        while let Some((rel, depth)) = stack.pop() {
            if Instant::now() >= deadline {
                found.not_all = true;
                break;
            }
            let dir = rel.iter().fold(base.clone(), |p, n| p.join(n));
            let Ok(entries) = std::fs::read_dir(&dir) else {
                found.not_all = true;
                continue;
            };
            for e in entries.flatten() {
                let name = e.file_name().to_string_lossy().into_owned();
                let Ok(m) = std::fs::symlink_metadata(e.path()) else {
                    found.not_all = true;
                    continue;
                };
                let kind = guard::kind_of(&m);
                if !matches!(kind, Kind::Folder | Kind::File) {
                    continue;
                }
                if !plain_name(&name) {
                    found.not_all = true;
                    continue;
                }
                let mut path = rel.clone();
                path.push(name);
                match kind {
                    Kind::Folder if depth < MAX_DEPTH => stack.push((path, depth + 1)),
                    Kind::Folder => found.not_all = true,
                    _ if carried(&path) => {
                        if found.files as usize >= MAX_FILES {
                            found.not_all = true;
                            continue;
                        }
                        found.files += 1;
                        found.bytes += m.len();
                        members.push(path.join("/"));
                    }
                    _ => {}
                }
            }
        }
        members.sort();
        for chunk in members.chunks(CHUNK) {
            found.components.push(Component::new(
                owner.clone(),
                &place(top),
                chunk.to_vec(),
                Custody::NeedsExport,
                false,
                evidence.clone(),
            ));
        }
    }
    Found::Mail(found)
}

/* ---------- The parser process on the new laptop ---------- */

/// One account folder the index names, and the kind of address of one of its mailboxes.
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct AccountScheme {
    pub folder: String,
    pub scheme: String,
}

/// The account folder and kind of address in one mailbox address (`imap://<folder>/INBOX`), or
/// `None` for an address that names no account folder.
fn account_of(url: &str) -> Option<AccountScheme> {
    let (scheme, rest) = url.split_once("://")?;
    if !scheme_ok(scheme) {
        return None;
    }
    let folder = rest.split('/').next().unwrap_or_default();
    (!folder.is_empty() && folder.len() <= MAX_NAME && !folder.chars().any(char::is_control)).then(
        || AccountScheme {
            folder: folder.to_string(),
            scheme: scheme.to_ascii_lowercase(),
        },
    )
}

/// Which kind of address each account folder's mailboxes have, from the received mail index
/// `db` and its log, read by SQLite from memory, in the parser process (it takes an
/// [`InChild`]). Only the pairs come back, each once, in order.
pub fn mail_kinds(
    seal: InChild,
    db: &[u8],
    log: Option<&[u8]>,
) -> Result<Vec<AccountScheme>, ContentError> {
    let damaged = |e: &dyn std::fmt::Display| ContentError::Damaged(e.to_string());
    let image = crate::content::image_of(seal, db, log)?;
    let c = crate::content::memory_db(&image)?;
    let mut stmt = c
        .prepare("SELECT url FROM mailboxes")
        .map_err(|e| damaged(&e))?;
    let mut rows = stmt.query([]).map_err(|e| damaged(&e))?;
    let mut out = BTreeSet::new();
    let mut seen = 0usize;
    while let Some(row) = rows.next().map_err(|e| damaged(&e))? {
        seen += 1;
        if seen > MAX_MAILBOXES {
            return Err(ContentError::Damaged("too many mailboxes".into()));
        }
        let Ok(rusqlite::types::ValueRef::Text(t)) = row.get_ref(0) else {
            continue;
        };
        let Some(a) = std::str::from_utf8(t).ok().and_then(account_of) else {
            continue;
        };
        out.insert(a);
        if out.len() > MAX_ACCOUNTS {
            return Err(ContentError::Damaged("too many accounts".into()));
        }
    }
    Ok(out.into_iter().collect())
}

/* ---------- The new laptop: which mail is kept only on the old Mac ---------- */

/// Whether the account folder `top` (a folder name inside `V<n>`, as received) holds mail kept
/// only on the old Mac, by the parser's `answer` (`None`: the index could not be read).
/// Folders are matched by exact equality; nothing in the answer is ever used as a path.
pub fn local_only(top: &str, answer: Option<&[AccountScheme]>, kinds: &Catalogue<KindRow>) -> bool {
    if top == ON_THIS_MAC {
        return true;
    }
    let Some(answer) = answer else {
        return true;
    };
    let mut schemes = answer.iter().filter(|a| a.folder == top).peekable();
    if schemes.peek().is_none() {
        // The index does not know it: an account removed from Mail, whose mail is only here.
        return true;
    }
    !schemes.all(|a| kind_of(kinds, &a.scheme) == Some(MailKind::Server))
}

/// A parser answer as the engine accepts it: within its bounds, each name plain; otherwise none
/// (and every account folder is then kept as mail only on the old Mac).
pub fn checked_answer(answer: Vec<AccountScheme>) -> Option<Vec<AccountScheme>> {
    (answer.len() <= MAX_ACCOUNTS
        && answer
            .iter()
            .all(|a| a.folder.len() <= MAX_NAME && scheme_ok(&a.scheme)))
    .then_some(answer)
}

/* ---------- The words of what is written for the person ---------- */

/// The names PCTwin gives what it writes for the person, in their language: the screens send
/// them ([`set_words`]); until then, and for any word that is not safe, the English ones.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Words {
    /// The folder the mailbox files land in, with `{laptop}` where the old laptop's name goes.
    pub folder: String,
    /// The name shown for the mailboxes kept on the Mac ("On My Mac").
    pub on_this_mac: String,
    /// The name shown for an account's mailboxes, with `{n}` where its number goes.
    pub account: String,
}

impl Default for Words {
    fn default() -> Self {
        Self {
            folder: "Mail from {laptop}".into(),
            on_this_mac: "On My Mac".into(),
            account: "Mail account {n}".into(),
        }
    }
}

static WORDS: std::sync::RwLock<Option<Words>> = std::sync::RwLock::new(None);

/// A name that can be a folder's or a file's on every system (no separator, reserved character
/// or control character; not starting or ending with a dot or a space), at most 100 characters.
pub fn name_ok(n: &str) -> bool {
    !n.trim().is_empty()
        && n.chars().count() <= 100
        && !n.starts_with(['.', ' '])
        && !n.ends_with(['.', ' '])
        && !n.chars().any(|c| {
            c.is_control() || matches!(c, '/' | '\\' | ':' | '*' | '?' | '"' | '<' | '>' | '|')
        })
}

/// The marks a translation may put around a placeholder (Unicode isolates), removed.
fn without_isolates(s: &str) -> String {
    s.chars()
        .filter(|c| !matches!(c, '\u{2066}'..='\u{2069}'))
        .collect()
}

/// Sets the words used from now on. False (and the words stay as they were) when any is not
/// safe: each holds its placeholder exactly once and is a safe name with it filled in.
pub fn set_words(w: Words) -> bool {
    let w = Words {
        folder: without_isolates(&w.folder),
        on_this_mac: without_isolates(&w.on_this_mac),
        account: without_isolates(&w.account),
    };
    let ok = w.folder.matches("{laptop}").count() == 1
        && name_ok(&w.folder.replace("{laptop}", "Laptop"))
        && name_ok(&w.on_this_mac)
        && w.account.matches("{n}").count() == 1
        && name_ok(&w.account.replace("{n}", "1"));
    if ok && let Ok(mut g) = WORDS.write() {
        *g = Some(w);
        return true;
    }
    false
}

/// The words in use.
pub fn words() -> Words {
    WORDS
        .read()
        .ok()
        .and_then(|g| g.clone())
        .unwrap_or_default()
}

impl Words {
    /// The folder's name for the old laptop named `laptop` (its letters kept, anything a name
    /// cannot hold left out; never empty).
    pub fn folder_for(&self, laptop: &str) -> String {
        let clean: String = laptop
            .chars()
            .filter(|c| c.is_alphanumeric() || matches!(c, ' ' | '-' | '_' | '\''))
            .take(60)
            .collect();
        let clean = clean.trim();
        let named = self
            .folder
            .replace("{laptop}", if clean.is_empty() { "PCTwin" } else { clean });
        if name_ok(&named) {
            named
        } else {
            Words::default().folder.replace("{laptop}", "PCTwin")
        }
    }

    /// The name shown for account number `n`.
    pub fn account_for(&self, n: usize) -> String {
        self.account.replace("{n}", &n.to_string())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn words_that_are_not_safe_names_are_refused_and_the_folder_name_is_always_safe() {
        let w = Words::default();
        assert_eq!(w.folder_for("Ada's Mac"), "Mail from Ada's Mac");
        assert_eq!(w.folder_for("../../x"), "Mail from x");
        assert_eq!(w.folder_for("C:\\Windows"), "Mail from CWindows");
        assert_eq!(w.folder_for(""), "Mail from PCTwin");
        assert_eq!(w.account_for(2), "Mail account 2");
        let bad = |f: &str, o: &str, a: &str| {
            !set_words(Words {
                folder: f.into(),
                on_this_mac: o.into(),
                account: a.into(),
            })
        };
        assert!(bad("Mail", "On My Mac", "Account {n}"), "no placeholder");
        assert!(bad("{laptop}/x", "On My Mac", "Account {n}"));
        assert!(bad("Mail from {laptop}", "..", "Account {n}"));
        assert!(bad("Mail from {laptop}", "On My Mac", "Account"));
        assert!(bad("Mail from {laptop}", "a\nb", "Account {n}"));
        // Isolation marks a translation puts around a placeholder are taken out.
        assert!(set_words(Words {
            folder: "Courrier de \u{2068}{laptop}\u{2069}".into(),
            on_this_mac: "Sur mon Mac".into(),
            account: "Compte \u{2068}{n}\u{2069}".into(),
        }));
        assert_eq!(words().folder_for("Ada"), "Courrier de Ada");
        assert!(set_words(Words::default()));
    }

    fn a(folder: &str, scheme: &str) -> AccountScheme {
        AccountScheme {
            folder: folder.into(),
            scheme: scheme.into(),
        }
    }

    fn seal() -> InChild {
        pctwin_parse_seal::in_child()
    }

    /// An index as Apple Mail keeps it, rows in its log included.
    fn index(urls: &[&str]) -> (Vec<u8>, Vec<u8>) {
        let t = tempfile::tempdir().unwrap();
        let p = t.path().join("Envelope Index");
        let c = rusqlite::Connection::open(&p).unwrap();
        c.execute_batch(
            "PRAGMA journal_mode=WAL; PRAGMA wal_autocheckpoint=0;
             CREATE TABLE mailboxes(ROWID INTEGER PRIMARY KEY, url TEXT);
             PRAGMA wal_checkpoint(TRUNCATE);",
        )
        .unwrap();
        for u in urls {
            c.execute("INSERT INTO mailboxes(url) VALUES (?1)", [u])
                .unwrap();
        }
        c.execute("INSERT INTO mailboxes(url) VALUES (NULL)", [])
            .unwrap();
        c.execute("INSERT INTO mailboxes(url) VALUES (x'00ff')", [])
            .unwrap();
        let db = std::fs::read(&p).unwrap();
        let wal = std::fs::read(t.path().join("Envelope Index-wal")).unwrap();
        drop(c);
        (db, wal)
    }

    #[test]
    fn the_index_gives_only_account_folders_and_their_kinds() {
        let (db, wal) = index(&[
            "imap://IMAP-1/INBOX",
            "imap://IMAP-1/Sent",
            "pop://POP-1/INBOX",
            "local:///Trips",
            "ews://EWS-1/Inbox",
        ]);
        let got = mail_kinds(seal(), &db, Some(&wal)).unwrap();
        assert_eq!(
            got,
            vec![a("EWS-1", "ews"), a("IMAP-1", "imap"), a("POP-1", "pop")]
        );
        // Every row was in the log: the database alone names none.
        assert_eq!(mail_kinds(seal(), &db, None).unwrap(), vec![]);
        // Not a database, or a database without the table: refused.
        assert!(mail_kinds(seal(), b"nothing", None).is_err());
        let t = tempfile::tempdir().unwrap();
        let other = t.path().join("x.db");
        rusqlite::Connection::open(&other)
            .unwrap()
            .execute_batch("CREATE TABLE t(a);")
            .unwrap();
        assert!(mail_kinds(seal(), &std::fs::read(&other).unwrap(), None).is_err());
    }

    #[test]
    fn the_old_laptop_lists_only_and_says_what_it_could_not_read() {
        let t = tempfile::tempdir().unwrap();
        let home = t.path();
        let owner = Owner::Person {
            account_id: "501".into(),
        };
        let later = Instant::now() + std::time::Duration::from_secs(30);
        assert_eq!(find(&owner, home, later), Found::NotHere);
        let v = home.join("Library/Mail/V10");
        let put = |rel: &str, b: &[u8]| {
            let p = v.join(rel);
            std::fs::create_dir_all(p.parent().unwrap()).unwrap();
            std::fs::write(p, b).unwrap();
        };
        put(
            "Mailboxes/A.mbox/Messages/1.emlx",
            b"3
abc",
        );
        put(
            "Mailboxes/A.mbox/Messages/2.partial.emlx",
            b"3
abc",
        );
        put("Mailboxes/A.mbox/Attachments/2/1/a.pdf", b"pdf");
        put("Mailboxes/A.mbox/Info.plist", b"x");
        // A name another system cannot hold (a Mac allows `:`): left, and said.
        if cfg!(unix) {
            put(
                "Mailboxes/Re: x.mbox/Messages/3.emlx",
                b"3
abc",
            );
        }
        put(
            "ACC/INBOX.mbox/Messages/4.emlx",
            b"3
abc",
        );
        put("MailData/Envelope Index", b"db");
        put("MailData/Envelope Index-wal", b"log");
        put("MailData/Envelope Index-shm", b"shm");
        std::fs::create_dir_all(home.join("Library/Mail/V2")).unwrap();
        let Found::Mail(m) = find(&owner, home, later) else {
            panic!("found");
        };
        assert_eq!(m.base, v);
        assert_eq!(m.not_all, cfg!(unix), "a name that cannot cross is said");
        let mut members: Vec<&str> = m
            .components
            .iter()
            .flat_map(|c| c.members.iter().map(String::as_str))
            .collect();
        members.sort();
        assert_eq!(
            members,
            vec![
                "ACC/INBOX.mbox/Messages/4.emlx",
                "MailData/Envelope Index",
                "MailData/Envelope Index-wal",
                "Mailboxes/A.mbox/Attachments/2/1/a.pdf",
                "Mailboxes/A.mbox/Messages/1.emlx",
                "Mailboxes/A.mbox/Messages/2.partial.emlx",
            ]
        );
        assert_eq!(m.files, 6);
        assert_eq!(m.bytes, 5 + 5 + 3 + 5 + 2 + 3);
        for c in &m.components {
            assert_eq!(c.custody, Custody::NeedsExport);
            assert!(c.evidence.contains(&Evidence::CarriedRaw));
            assert!(!c.secret);
            // Each component holds one folder's files.
            let top = c.members[0].split('/').next().unwrap();
            assert!(c.members.iter().all(|x| x.split('/').next() == Some(top)));
        }
        // An older layout is never looked at, the newest one is.
        assert_eq!(
            newest_version(&home.join("Library/Mail")).unwrap(),
            Some("V10".into())
        );
    }

    #[test]
    fn the_shipped_kinds_are_clean_and_name_server_and_local_kinds() {
        let k = shipped_kinds();
        assert!(k.report().is_clean(), "{:?}", k.report());
        assert_eq!(kind_of(&k, "imap"), Some(MailKind::Server));
        assert_eq!(kind_of(&k, "EWS"), Some(MailKind::Server));
        assert_eq!(kind_of(&k, "pop"), Some(MailKind::Local));
        assert_eq!(kind_of(&k, "local"), Some(MailKind::Local));
        assert_eq!(kind_of(&k, "gopher"), None);
    }

    #[test]
    fn every_doubt_keeps_mail() {
        let k = shipped_kinds();
        let answer = vec![
            a("IMAP-UUID", "imap"),
            a("POP-UUID", "pop"),
            a("ODD-UUID", "gopher"),
            a("MIXED-UUID", "imap"),
            a("MIXED-UUID", "local"),
        ];
        let only = |top: &str| local_only(top, Some(&answer), &k);
        assert!(!only("IMAP-UUID"), "a server account is left to its server");
        assert!(only("POP-UUID"));
        assert!(only("ODD-UUID"), "an unknown kind");
        assert!(only("MIXED-UUID"), "one address not on a server");
        assert!(only("GONE-UUID"), "an account the index does not know");
        assert!(only(ON_THIS_MAC));
        assert!(local_only("IMAP-UUID", None, &k), "the index unreadable");
        // Exact equality only: no case folding, no trimming.
        assert!(only("imap-uuid"));
        assert!(only("IMAP-UUID "));
    }

    #[test]
    fn mailbox_addresses_name_their_account_folder_or_nothing() {
        assert_eq!(account_of("imap://ABC/INBOX"), Some(a("ABC", "imap")));
        assert_eq!(account_of("EWS://ABC"), Some(a("ABC", "ews")));
        assert_eq!(account_of("local:///Trips"), None);
        assert_eq!(account_of("no address"), None);
        assert_eq!(account_of("im ap://ABC/x"), None);
        assert_eq!(account_of("imap://A\u{7}B/x"), None);
        assert_eq!(account_of(&format!("imap://{}/x", "x".repeat(300))), None);
    }

    #[test]
    fn an_answer_past_its_bounds_is_not_used() {
        assert!(checked_answer(vec![a("A", "imap")]).is_some());
        assert!(checked_answer(vec![a("A", "im ap")]).is_none());
        assert!(checked_answer(vec![a(&"x".repeat(300), "imap")]).is_none());
        assert!(checked_answer(vec![a("A", "imap"); MAX_ACCOUNTS + 1]).is_none());
    }

    #[test]
    fn names_that_cannot_cross_are_left_and_said() {
        for bad in ["", ".", "..", "a/b", "a\\b", "Re: x", "a\nb"] {
            assert!(!plain_name(bad), "{bad:?}");
        }
        for ok in ["INBOX.mbox", "Été.mbox", "1.emlx"] {
            assert!(plain_name(ok), "{ok:?}");
        }
        let p = |s: &[&str]| s.iter().map(|x| x.to_string()).collect::<Vec<_>>();
        assert!(carried(&p(&["A", "INBOX.mbox", "Messages", "1.emlx"])));
        assert!(carried(&p(&[
            "A",
            "INBOX.mbox",
            "Messages",
            "2.partial.emlx"
        ])));
        assert!(carried(&p(&[
            "A",
            "x",
            "Attachments",
            "2",
            "1.1",
            "doc.pdf"
        ])));
        assert!(!carried(&p(&["A", "INBOX.mbox", "Info.plist"])));
    }
}
