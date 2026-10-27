//! Facts read from inside a file, by its structure (ADOPTION change 8): what a JSON file's values
//! are, and what shape an SQLite database's tables have. Never by the file's name or its app.
//!
//! - [`json_facts`]: values protected by this laptop (a key wrapped by Windows DPAPI or bound to
//!   the app, by the bytes inside the base64), values encrypted under a key in a key database
//!   beside it (the PKCS#11 secret-decoder ring format NSS-based apps use), values shaped like
//!   credentials, references to other files by their hash or name, and keys that start programs.
//! - [`sqlite_tables`] and [`db_kind`]: a database's tables and columns, read through SQLite
//!   opened read-only and `immutable`, so the old laptop's file is never written (not even a
//!   lock or a shared-memory index); and what kind of store that shape is (a key database,
//!   sessions and cookies, saved passwords, machine-encrypted values, or ordinary data).
//!
//! The values themselves are never logged, kept or returned: only the facts.

use std::path::Path;

use base64::Engine;
use base64::engine::general_purpose::STANDARD;
use serde_json::Value;

/// The largest JSON file read for its facts.
pub const MAX_JSON: usize = 16 * 1024 * 1024;
/// The most file names one JSON file's references are kept for.
const MAX_NAMES: usize = 10_000;

/// What a JSON file's values show.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct JsonFacts {
    /// A value is a key protected by this laptop's account (DPAPI) or bound to the app.
    pub machine_bound_key: bool,
    /// How many values are encrypted under a key held in a key database beside the file.
    pub key_db_encrypted: u32,
    /// A value looks like a password, key or sign-in token.
    pub credential_values: bool,
    /// A key that names a program to start holds a value.
    pub starts_programs: bool,
    /// Hashes other files are named by (`sha256:<hex>` or `sha256-<hex>`), lower case hex.
    pub digests: Vec<String>,
    /// Plain file names the values mention (no folder parts).
    pub file_names: Vec<String>,
}

/// Why a file's facts could not be read.
#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
pub enum ContentError {
    #[error("too large to read for its contents")]
    TooLarge,
    #[error("not readable as its format: {0}")]
    Damaged(String),
}

/// Key names that name a program to run (generic words, as the settings rules use).
const RUN_WORDS: &[&str] = &[
    "command",
    "cmd",
    "exec",
    "executable",
    "args",
    "arguments",
    "entrypoint",
    "shell",
    "program",
    "hook",
    "hooks",
];

fn words_of(key: &str) -> Vec<String> {
    let mut out = Vec::new();
    let mut cur = String::new();
    let mut prev_lower = false;
    for c in key.chars() {
        if c.is_ascii_uppercase() && prev_lower {
            out.push(std::mem::take(&mut cur));
        }
        if c.is_ascii_alphanumeric() {
            cur.push(c.to_ascii_lowercase());
            prev_lower = c.is_ascii_lowercase() || c.is_ascii_digit();
        } else {
            if !cur.is_empty() {
                out.push(std::mem::take(&mut cur));
            }
            prev_lower = false;
        }
    }
    if !cur.is_empty() {
        out.push(cur);
    }
    out
}

/// Reads a JSON file's facts.
pub fn json_facts(bytes: &[u8]) -> Result<JsonFacts, ContentError> {
    if bytes.len() > MAX_JSON {
        return Err(ContentError::TooLarge);
    }
    let bytes = bytes.strip_prefix(b"\xef\xbb\xbf").unwrap_or(bytes);
    pctwin_types::nesting::check_bytes(bytes, pctwin_types::nesting::Syntax::Json)
        .map_err(|e| ContentError::Damaged(e.to_string()))?;
    let v: Value =
        serde_json::from_slice(bytes).map_err(|e| ContentError::Damaged(e.to_string()))?;
    let mut f = JsonFacts::default();
    walk(&v, None, &mut f);
    f.digests.sort();
    f.digests.dedup();
    f.file_names.sort();
    f.file_names.dedup();
    Ok(f)
}

fn walk(v: &Value, key: Option<&str>, f: &mut JsonFacts) {
    let run_key = key.is_some_and(|k| words_of(k).iter().any(|w| RUN_WORDS.contains(&w.as_str())));
    match v {
        Value::String(s) => {
            if run_key && !s.trim().is_empty() {
                f.starts_programs = true;
            }
            string_facts(s, f);
        }
        Value::Array(a) => {
            for x in a {
                walk(x, key, f);
            }
        }
        Value::Object(o) => {
            for (k, x) in o {
                walk(x, Some(k), f);
            }
        }
        _ => {}
    }
}

fn string_facts(s: &str, f: &mut JsonFacts) {
    let t = s.trim();
    for prefix in ["sha256:", "sha256-"] {
        if let Some(hex) = t.strip_prefix(prefix)
            && hex.len() == 64
            && hex.bytes().all(|b| b.is_ascii_hexdigit())
        {
            f.digests.push(hex.to_ascii_lowercase());
            return;
        }
    }
    if t.len() >= 24
        && t.bytes()
            .all(|b| b.is_ascii_alphanumeric() || matches!(b, b'+' | b'/' | b'='))
        && let Ok(bin) = STANDARD.decode(t)
    {
        if bin.starts_with(b"DPAPI") || bin.starts_with(b"APPB") {
            f.machine_bound_key = true;
            return;
        }
        if secret_decoder_ring(&bin) {
            f.key_db_encrypted = f.key_db_encrypted.saturating_add(1);
            return;
        }
    }
    if pctwin_settings::classify::looks_secret(t).is_some() {
        f.credential_values = true;
        return;
    }
    if f.file_names.len() < MAX_NAMES
        && (1..=255).contains(&t.len())
        && !t.contains(['/', '\\', ':', '\n'])
        && t.rsplit_once('.')
            .is_some_and(|(stem, ext)| !stem.is_empty() && (1..=16).contains(&ext.len()))
    {
        f.file_names.push(t.to_string());
    }
}

/// A DER length at `b[at]`: (length, bytes it took).
fn der_len(b: &[u8], at: usize) -> Option<(usize, usize)> {
    let first = *b.get(at)?;
    if first < 0x80 {
        return Some((usize::from(first), 1));
    }
    let n = usize::from(first & 0x7f);
    if n == 0 || n > 3 {
        return None;
    }
    let mut len = 0usize;
    for i in 0..n {
        len = (len << 8) | usize::from(*b.get(at + 1 + i)?);
    }
    Some((len, 1 + n))
}

/// The PKCS#11 secret-decoder-ring form: a DER sequence holding a 16-byte key ID (the key is in
/// the key database), then the cipher's parameters (a sequence starting with its OID), then the
/// encrypted bytes.
fn secret_decoder_ring(b: &[u8]) -> bool {
    if b.first() != Some(&0x30) {
        return false;
    }
    let Some((len, took)) = der_len(b, 1) else {
        return false;
    };
    if 1 + took + len != b.len() {
        return false;
    }
    let at = 1 + took;
    if b.get(at..at + 2) != Some(&[0x04, 0x10]) {
        return false;
    }
    let at = at + 2 + 16;
    if b.get(at) != Some(&0x30) {
        return false;
    }
    let Some((_, took)) = der_len(b, at + 1) else {
        return false;
    };
    b.get(at + 1 + took) == Some(&0x06)
}

/// One table and its columns, in lower case.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Table {
    pub name: String,
    pub columns: Vec<String>,
}

/// The `file:` address that opens `path` read-only and immutable: SQLite then takes no lock and
/// writes nothing beside it (no journal, no shared-memory index).
pub fn immutable_uri(path: &Path) -> String {
    let p = path.to_string_lossy().replace('\\', "/");
    let mut out = String::from("file:");
    if p.as_bytes().get(1) == Some(&b':') {
        out.push_str("///");
    }
    for c in p.chars() {
        match c {
            '%' | '?' | '#' | ' ' => out.push_str(&format!("%{:02X}", c as u32)),
            c => out.push(c),
        }
    }
    out.push_str("?mode=ro&immutable=1");
    out
}

/// The write-ahead log SQLite keeps beside a database: its own naming rule (the database's path
/// and `-wal`), the same for every app.
pub fn wal_of(path: &Path) -> std::path::PathBuf {
    let mut p = path.as_os_str().to_owned();
    p.push("-wal");
    std::path::PathBuf::from(p)
}

/// A database's tables and columns, read without writing anything on the old laptop. With no
/// log beside it, SQLite reads the file itself, immutable. With a log (recent changes, perhaps
/// its tables too, that an immutable read would not see), the file and its log are first copied
/// to a private folder, and the copy is read.
pub fn sqlite_tables(path: &Path) -> Result<Vec<Table>, ContentError> {
    use rusqlite::OpenFlags;
    let wal = wal_of(path);
    let has_log = std::fs::symlink_metadata(&wal).is_ok_and(|m| m.is_file() && m.len() > 0);
    let _snapshot;
    let db = if has_log {
        let dir = tempfile::tempdir().map_err(|e| ContentError::Damaged(e.to_string()))?;
        let copy = dir.path().join("db");
        std::fs::copy(path, &copy).map_err(|e| ContentError::Damaged(e.to_string()))?;
        std::fs::copy(&wal, wal_of(&copy)).map_err(|e| ContentError::Damaged(e.to_string()))?;
        let db = rusqlite::Connection::open_with_flags(
            &copy,
            OpenFlags::SQLITE_OPEN_READ_WRITE | OpenFlags::SQLITE_OPEN_NO_MUTEX,
        );
        _snapshot = dir;
        db
    } else {
        rusqlite::Connection::open_with_flags(
            immutable_uri(path),
            OpenFlags::SQLITE_OPEN_READ_ONLY
                | OpenFlags::SQLITE_OPEN_URI
                | OpenFlags::SQLITE_OPEN_NO_MUTEX,
        )
    }
    .map_err(|e| ContentError::Damaged(e.to_string()))?;
    let bad = |e: rusqlite::Error| ContentError::Damaged(e.to_string());
    let mut names = Vec::new();
    {
        let mut st = db
            .prepare("SELECT name FROM sqlite_schema WHERE type = 'table' ORDER BY name")
            .map_err(bad)?;
        let rows = st.query_map([], |r| r.get::<_, String>(0)).map_err(bad)?;
        for r in rows {
            names.push(r.map_err(bad)?);
        }
    }
    let mut out = Vec::with_capacity(names.len());
    for name in names.into_iter().take(1000) {
        let mut st = db
            .prepare("SELECT name FROM pragma_table_info(?1)")
            .map_err(bad)?;
        let cols = st
            .query_map([&name], |r| r.get::<_, String>(0))
            .map_err(bad)?
            .collect::<Result<Vec<_>, _>>()
            .map_err(bad)?;
        out.push(Table {
            name: name.to_ascii_lowercase(),
            columns: cols.into_iter().map(|c| c.to_ascii_lowercase()).collect(),
        });
    }
    Ok(out)
}

/// What kind of store a database is, by its tables' shape.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DbKind {
    /// A key database that decrypts values stored beside it.
    KeyDatabase,
    /// Cookies or sign-in tokens.
    Sessions,
    /// Saved passwords.
    Credentials,
    /// Values encrypted by this laptop or the app (machine-bound).
    MachineEncrypted,
    /// Ordinary data.
    Ordinary,
}

/// The store's kind from its tables. Rules by the shape of columns, the same for every app.
pub fn db_kind(tables: &[Table]) -> DbKind {
    let has_table = |n: &str| tables.iter().any(|t| t.name == n);
    if has_table("metadata") && has_table("nssprivate") {
        return DbKind::KeyDatabase;
    }
    let any = |t: &Table, f: &dyn Fn(&str) -> bool| t.columns.iter().any(|c| f(c));
    for t in tables {
        let password = any(t, &|c| c.contains("password") || c.contains("passwd"));
        if password {
            return DbKind::Credentials;
        }
    }
    for t in tables {
        let cookie = any(t, &|c| c.contains("host"))
            && any(t, &|c| c == "name")
            && any(t, &|c| c == "path")
            && any(t, &|c| c.contains("expir"));
        let token = any(t, &|c| c == "token" || c.ends_with("_token"))
            && any(t, &|c| {
                c.contains("expir")
                    || c.contains("host")
                    || c.contains("url")
                    || c.contains("origin")
                    || c.contains("account")
            });
        if cookie || token {
            return DbKind::Sessions;
        }
    }
    if tables.iter().any(|t| any(t, &|c| c.contains("encrypted"))) {
        return DbKind::MachineEncrypted;
    }
    DbKind::Ordinary
}

#[cfg(test)]
mod tests {
    use super::*;

    fn sdr() -> String {
        let mut inner = vec![0x04, 0x10];
        inner.extend_from_slice(&[0xf8; 16]);
        inner.extend_from_slice(&[0x30, 0x05, 0x06, 0x03, 0x2a, 0x86, 0x48]);
        inner.extend_from_slice(&[0x04, 0x08, 1, 2, 3, 4, 5, 6, 7, 8]);
        let mut b = vec![0x30, inner.len() as u8];
        b.extend_from_slice(&inner);
        STANDARD.encode(b)
    }

    #[test]
    fn values_bound_to_the_laptop_or_a_key_database_are_found_by_their_bytes() {
        let dpapi = STANDARD.encode(b"DPAPI\x01\x00\x00\x00 synthetic");
        let appb = STANDARD.encode(b"APPB\x01 synthetic bound key");
        let doc = format!(
            r#"{{"a":{{"k":"{dpapi}"}},"list":[{{"u":"{s}","p":"{s}"}}],"other":"plain words"}}"#,
            s = sdr()
        );
        let f = json_facts(doc.as_bytes()).unwrap();
        assert!(f.machine_bound_key);
        assert_eq!(f.key_db_encrypted, 2);
        assert!(!f.credential_values && !f.starts_programs);
        let f = json_facts(format!(r#"["{appb}"]"#).as_bytes()).unwrap();
        assert!(f.machine_bound_key);
        // Base64 of anything else is not either.
        let plain = STANDARD.encode(b"just some ordinary bytes here");
        let f = json_facts(format!(r#"["{plain}"]"#).as_bytes()).unwrap();
        assert!(!f.machine_bound_key && f.key_db_encrypted == 0);
    }

    #[test]
    fn credentials_commands_and_references_are_found() {
        let f = json_facts(
            br#"{"token":"ghp_0123456789abcdefABCDEF0123456789abcd",
                "servers":{"x":{"command":"npx","args":["-y","tool"]}},
                "layers":[{"digest":"sha256:AAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAA"}],
                "weight_map":{"w":"model-00001-of-00002.safetensors"},
                "note":"see folder/inside.txt"}"#,
        )
        .unwrap();
        assert!(f.credential_values);
        assert!(f.starts_programs);
        assert_eq!(f.digests, vec!["a".repeat(64)]);
        assert_eq!(f.file_names, vec!["model-00001-of-00002.safetensors"]);
        let f = json_facts(br#"{"preCommandHook":"x"}"#).unwrap();
        assert!(f.starts_programs, "camelCase words are split");
        let f = json_facts(br#"{"commander":"x","title":"echo | grep"}"#).unwrap();
        assert!(
            !f.starts_programs,
            "a word containing a run word is not one"
        );
    }

    #[test]
    fn json_reading_is_bounded() {
        assert_eq!(
            json_facts(&vec![b' '; MAX_JSON + 1]).unwrap_err(),
            ContentError::TooLarge
        );
        let deep = "[".repeat(10_000);
        assert!(matches!(
            json_facts(deep.as_bytes()),
            Err(ContentError::Damaged(_))
        ));
        assert!(json_facts(b"\xef\xbb\xbf{}").is_ok());
    }

    #[test]
    fn the_address_opens_any_path_without_writing() {
        assert_eq!(
            immutable_uri(Path::new(r"C:\Users\a b\x%?#.db")),
            "file:///C:/Users/a%20b/x%25%3F%23.db?mode=ro&immutable=1"
        );
        assert_eq!(
            immutable_uri(Path::new("/home/a/x.db")),
            "file:/home/a/x.db?mode=ro&immutable=1"
        );
    }

    fn table(name: &str, cols: &[&str]) -> Table {
        Table {
            name: name.into(),
            columns: cols.iter().map(|c| c.to_string()).collect(),
        }
    }

    #[test]
    fn a_stores_kind_comes_from_its_columns() {
        assert_eq!(
            db_kind(&[table("metadata", &["id"]), table("nssprivate", &["a11"])]),
            DbKind::KeyDatabase
        );
        assert_eq!(
            db_kind(&[table(
                "cookies",
                &["host_key", "name", "encrypted_value", "path", "expires_utc"]
            )]),
            DbKind::Sessions
        );
        assert_eq!(
            db_kind(&[table(
                "moz_cookies",
                &["host", "name", "value", "path", "expiry"]
            )]),
            DbKind::Sessions
        );
        assert_eq!(
            db_kind(&[table("t", &["refresh_token", "account_id"])]),
            DbKind::Sessions
        );
        assert_eq!(
            db_kind(&[table(
                "logins",
                &["origin_url", "username_value", "password_value"]
            )]),
            DbKind::Credentials
        );
        assert_eq!(
            db_kind(&[table("cards", &["name_on_card", "card_number_encrypted"])]),
            DbKind::MachineEncrypted
        );
        assert_eq!(
            db_kind(&[table("urls", &["url", "title", "visit_count"])]),
            DbKind::Ordinary
        );
        // A token count with no place or expiry is not a sign-in.
        assert_eq!(
            db_kind(&[table("t", &["token", "count"])]),
            DbKind::Ordinary
        );
    }

    #[test]
    fn a_databases_tables_are_read_without_touching_it() {
        let dir = tempfile::tempdir().unwrap();
        let p = dir.path().join("synthetic db.sqlite");
        {
            let c = rusqlite::Connection::open(&p).unwrap();
            c.execute_batch(
                "PRAGMA journal_mode=WAL; CREATE TABLE Urls(URL TEXT, title TEXT); INSERT INTO Urls VALUES('x','y');",
            )
            .unwrap();
            // Keep it open, as a running app would: a log and an index are beside it.
            let before: Vec<_> = std::fs::read_dir(dir.path())
                .unwrap()
                .map(|e| {
                    let e = e.unwrap();
                    (e.file_name(), e.metadata().unwrap().len())
                })
                .collect();
            let bytes_before: Vec<Vec<u8>> = std::fs::read_dir(dir.path())
                .unwrap()
                .map(|e| std::fs::read(e.unwrap().path()).unwrap())
                .collect();
            let t = sqlite_tables(&p).unwrap();
            assert_eq!(t, vec![table("urls", &["url", "title"])]);
            let after: Vec<_> = std::fs::read_dir(dir.path())
                .unwrap()
                .map(|e| {
                    let e = e.unwrap();
                    (e.file_name(), e.metadata().unwrap().len())
                })
                .collect();
            let bytes_after: Vec<Vec<u8>> = std::fs::read_dir(dir.path())
                .unwrap()
                .map(|e| std::fs::read(e.unwrap().path()).unwrap())
                .collect();
            assert_eq!(before, after);
            assert_eq!(bytes_before, bytes_after, "nothing beside it was written");
            drop(c);
        }
        // With no log beside it, the file itself is read, immutable.
        assert_eq!(
            sqlite_tables(&p).unwrap(),
            vec![table("urls", &["url", "title"])]
        );
        assert!(!wal_of(&p).exists());
        assert!(matches!(
            sqlite_tables(&dir.path().join("missing.db")),
            Err(ContentError::Damaged(_))
        ));
        std::fs::write(dir.path().join("junk.db"), b"not a database at all, really").unwrap();
        assert!(sqlite_tables(&dir.path().join("junk.db")).is_err());
    }
}
