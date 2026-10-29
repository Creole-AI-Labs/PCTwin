//! Facts read from inside a file, by its structure (ADOPTION change 8): what a JSON file's values
//! are, and what shape an SQLite database's tables have. Never by the file's name or its app.
//!
//! - [`json_facts`]: values protected by this laptop (a key wrapped by Windows DPAPI or bound to
//!   the app, by the bytes inside the base64), values encrypted under a key in a key database
//!   beside it (the PKCS#11 secret-decoder ring format NSS-based apps use), values shaped like
//!   credentials, references to other files by their hash or name, and keys that start programs.
//! - [`sqlite_tables`] and [`db_kind`]: a database's tables and columns, and what kind of store
//!   that shape is (a key database, sessions and cookies, saved passwords, machine-encrypted
//!   values, or ordinary data).
//!
//! The values themselves are never logged, kept or returned: only the facts.
//!
//! **Read only in the parser process (Phase 2 adoption, change 1).** This crate copies a file's
//! bytes, bounded ([`MAX_JSON`], [`MAX_DB`], [`MAX_LOG`]), and hands them to PCTwin's
//! locked-down parser process through the [`ContentReader`] installed once by the program
//! ([`install_reader`]; `pctwin-parse`'s `Parser`). There, a database's write-ahead log is laid
//! over its pages in memory ([`apply_log`]: only frames whose checksums hold, up to the last
//! commit, as SQLite itself does) and SQLite reads the result from memory, read-only: SQLite
//! never opens the old laptop's file, and nothing is written beside it or in the temporary
//! folder. With no reader installed, nothing is read and every file is "not checked".

use std::path::Path;
use std::sync::OnceLock;
use std::time::{Duration, Instant};

use pctwin_parse_seal::InChild;
use serde::{Deserialize, Serialize};

use base64::Engine;
use base64::engine::general_purpose::STANDARD;
use serde_json::Value;

/// The largest JSON file read for its facts.
pub const MAX_JSON: usize = 4 * 1024 * 1024;
/// The most file names one JSON file's references are kept for.
const MAX_NAMES: usize = 10_000;

/// What a JSON file's values show.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
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
    /// Full local paths the values name (files the data links to).
    pub paths: Vec<String>,
    /// The file is a tree of saved web addresses (bookmarks), by its shape (2.5 browsers).
    #[serde(default)]
    pub bookmarks: bool,
}

/// Whether `v` is a tree of saved web addresses, by the two shapes browsers write: a `roots`
/// object whose nodes of `"type": "url"` carry a `url` (the Chromium bookmarks file), or nodes of
/// a `text/x-moz-place` type that carry a `uri` (a Mozilla bookmarks backup). Shapes of the
/// format, the same for every browser that writes it; never a file's name.
pub fn bookmark_tree(v: &Value) -> bool {
    fn any_node(v: &Value, f: &dyn Fn(&serde_json::Map<String, Value>) -> bool) -> bool {
        match v {
            Value::Object(o) => f(o) || o.values().any(|x| any_node(x, f)),
            Value::Array(a) => a.iter().any(|x| any_node(x, f)),
            _ => false,
        }
    }
    let text = |o: &serde_json::Map<String, Value>, k: &str| {
        o.get(k)
            .and_then(Value::as_str)
            .map(|s| s.trim().to_string())
    };
    let chromium = v.get("roots").is_some_and(|r| {
        r.is_object()
            && any_node(r, &|o| {
                text(o, "type").as_deref() == Some("url")
                    && text(o, "url").is_some_and(|u| !u.is_empty())
            })
    });
    chromium
        || any_node(v, &|o| {
            text(o, "type").is_some_and(|t| t.starts_with("text/x-moz-place"))
                && text(o, "uri").is_some_and(|u| !u.is_empty())
        })
}

/// Whether a database's tables are shaped like a browser's saved web addresses: a table of
/// addresses with how often each was visited (history), or a table named for bookmarks with
/// titles. Rules by the shape of the tables, the same for every app.
pub fn browsing(tables: &[Table]) -> bool {
    tables.iter().any(|t| {
        let has = |f: &dyn Fn(&str) -> bool| t.columns.iter().any(|c| f(c));
        let history = has(&|c| c == "url") && has(&|c| c.contains("visit"));
        let bookmarks = t.name.contains("bookmark") && has(&|c| c == "title");
        history || bookmarks
    })
}

/// The most linked paths kept per file.
pub const MAX_PATHS: usize = 2000;
/// The most text values read from one database while looking for linked paths.
const MAX_VALUES: usize = 50_000;

/// Whether a text is one full local path (`C:\...`, `C:/...`, or `/a/b...`): not an address,
/// not another computer's share, not a relative place.
pub fn local_path(t: &str) -> bool {
    let b = t.as_bytes();
    if t.len() < 4 || t.len() > 1024 || t.contains(['\n', '\r', '\0']) {
        return false;
    }
    let windows = b[0].is_ascii_alphabetic() && b[1] == b':' && matches!(b[2], b'\\' | b'/');
    let unix = b[0] == b'/' && b[1] != b'/' && t[1..].contains('/') && !t.contains("://");
    windows || unix
}

/// Why a file's facts could not be read.
#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
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

/// Reads a JSON file's facts, in the parser process (it takes an [`InChild`]).
pub fn json_facts(_: InChild, bytes: &[u8]) -> Result<JsonFacts, ContentError> {
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
    f.bookmarks = bookmark_tree(&v);
    f.digests.sort();
    f.digests.dedup();
    f.file_names.sort();
    f.file_names.dedup();
    f.paths.sort();
    f.paths.dedup();
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
    // A key the system wrapped for this laptop and app, written in hex (Electron's safe storage
    // writes `v10`/`v11` and Chromium-style app-bound keys `v20`, as their first bytes).
    if t.len() >= 32 && t.len().is_multiple_of(2) && t.bytes().all(|b| b.is_ascii_hexdigit()) {
        let first: Vec<u8> = (0..6)
            .step_by(2)
            .filter_map(|i| u8::from_str_radix(&t[i..i + 2], 16).ok())
            .collect();
        if matches!(first.as_slice(), b"v10" | b"v11" | b"v20") {
            f.machine_bound_key = true;
            return;
        }
    }
    if t.len() >= 24
        && t.bytes()
            .all(|b| b.is_ascii_alphanumeric() || matches!(b, b'+' | b'/' | b'='))
        && let Ok(bin) = STANDARD.decode(t)
    {
        if bin.starts_with(b"DPAPI")
            || bin.starts_with(b"APPB")
            || bin.starts_with(b"v10")
            || bin.starts_with(b"v11")
            || bin.starts_with(b"v20")
        {
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
    if local_path(t) {
        if f.paths.len() < MAX_PATHS {
            f.paths.push(t.to_string());
        }
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
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Table {
    pub name: String,
    pub columns: Vec<String>,
}

/// The `file:` address that opens `path` read-only and immutable: SQLite then takes no lock and
/// writes nothing beside it (no journal, no shared-memory index). Used by the capture's online
/// backup.
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

/// The largest database read for its facts (its bytes are copied to the parser process).
pub const MAX_DB: u64 = 256 * 1024 * 1024;
/// The largest write-ahead log read with it.
pub const MAX_LOG: u64 = 64 * 1024 * 1024;

/// Why a file was not read when no parser process is there to read it.
pub const NO_READER: &str = "PCTwin's file checker is not available, so it was not checked";

/// What the parser process found in a database.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct DbFacts {
    /// Its tables and columns, in lower case.
    pub tables: Vec<Table>,
    pub kind: DbKind,
    /// For ordinary data, the full local paths its place-named columns hold.
    pub paths: Option<LinkedPaths>,
}

/// PCTwin's reading of a file's contents, in the parser process (`pctwin-parse`'s `Parser`).
/// The bytes are a bounded copy made here. Err: it could not be checked (why).
pub trait ContentReader: Sync {
    fn json(&self, bytes: &[u8]) -> Result<JsonFacts, ContentError>;
    /// A database and its write-ahead log (if any), read within `time`.
    fn sqlite(
        &self,
        db: &[u8],
        log: Option<&[u8]>,
        time: Duration,
    ) -> Result<DbFacts, ContentError>;
    /// One complete database from a database and its log (the capture's consistent copy), only
    /// if it passes SQLite's own `quick_check` ([`db_snapshot`]).
    fn snapshot(&self, db: &[u8], log: Option<&[u8]>) -> Result<Vec<u8>, ContentError>;
    /// Whether a `manifest.json` is a WebExtension's: JSON with a numeric `manifest_version`.
    fn add_on_manifest(&self, bytes: &[u8]) -> Result<bool, ContentError>;
}

static READER: OnceLock<&'static dyn ContentReader> = OnceLock::new();

/// Installs the reader this program reads files' contents through, once (the app at its start,
/// tests with theirs). False when one was installed already (that one stays).
pub fn install_reader(reader: &'static dyn ContentReader) -> bool {
    READER.set(reader).is_ok()
}

fn reader() -> Result<&'static dyn ContentReader, ContentError> {
    READER
        .get()
        .copied()
        .ok_or_else(|| ContentError::Damaged(NO_READER.into()))
}

/// A JSON file's facts from its bytes, read in the parser process.
pub fn json_facts_of(bytes: &[u8]) -> Result<JsonFacts, ContentError> {
    if bytes.len() > MAX_JSON {
        return Err(ContentError::TooLarge);
    }
    reader()?.json(bytes)
}

/// The readers, run in this process: only for the parser process and this crate's own tests
/// (it takes an [`InChild`]).
#[derive(Debug, Clone, Copy)]
pub struct InThisProcess(InChild);

impl InThisProcess {
    pub const fn new(seal: InChild) -> Self {
        Self(seal)
    }
}

/// The readers in this process, for this crate's own tests (the parser process's tests use the
/// real process).
#[cfg(test)]
pub(crate) static IN_TESTS: InThisProcess = InThisProcess::new(pctwin_parse_seal::in_child());

impl ContentReader for InThisProcess {
    fn json(&self, bytes: &[u8]) -> Result<JsonFacts, ContentError> {
        json_facts(self.0, bytes)
    }
    fn sqlite(
        &self,
        db: &[u8],
        log: Option<&[u8]>,
        time: Duration,
    ) -> Result<DbFacts, ContentError> {
        db_facts(self.0, db, log, Instant::now() + time)
    }
    fn snapshot(&self, db: &[u8], log: Option<&[u8]>) -> Result<Vec<u8>, ContentError> {
        db_snapshot(self.0, db, log)
    }
    fn add_on_manifest(&self, bytes: &[u8]) -> Result<bool, ContentError> {
        Ok(add_on_manifest(self.0, bytes))
    }
}

/// The capture's consistent copy of a database, made in the parser process from the bytes of
/// its steady snapshot and log ([`ContentReader::snapshot`]).
pub fn snapshot_of(db: &[u8], log: Option<&[u8]>) -> Result<Vec<u8>, ContentError> {
    reader()?.snapshot(db, log)
}

/// Whether a `manifest.json`'s bytes are a WebExtension's, read in the parser process.
pub fn is_add_on_manifest(bytes: &[u8]) -> Result<bool, ContentError> {
    reader()?.add_on_manifest(bytes)
}

/// Whether `bytes` are JSON with a numeric `manifest_version` (the WebExtension format), in the
/// parser process (it takes an [`InChild`]).
pub fn add_on_manifest(_: InChild, bytes: &[u8]) -> bool {
    let bytes = bytes.strip_prefix(b"\xef\xbb\xbf").unwrap_or(bytes);
    if bytes.len() > 1024 * 1024
        || pctwin_types::nesting::check_bytes(bytes, pctwin_types::nesting::Syntax::Json).is_err()
    {
        return false;
    }
    serde_json::from_slice::<serde_json::Value>(bytes)
        .ok()
        .and_then(|v| v.get("manifest_version").map(serde_json::Value::is_u64))
        .unwrap_or(false)
}

/// Copies a database and its non-empty log beside it, bounded, and has the parser read them.
fn facts_at(path: &Path, until: Instant) -> Result<DbFacts, ContentError> {
    let damaged = |e: &dyn std::fmt::Display| ContentError::Damaged(e.to_string());
    let too_large = |e: std::io::Error| {
        if e.to_string() == "too large" {
            ContentError::TooLarge
        } else {
            damaged(&e)
        }
    };
    let db = crate::guard::read_all(path, MAX_DB).map_err(too_large)?;
    let wal = wal_of(path);
    let log = if std::fs::symlink_metadata(&wal).is_ok_and(|m| m.is_file() && m.len() > 0) {
        Some(crate::guard::read_all(&wal, MAX_LOG).map_err(too_large)?)
    } else {
        None
    };
    reader()?.sqlite(
        &db,
        log.as_deref(),
        until.saturating_duration_since(Instant::now()),
    )
}

/// A database's tables and columns, read without writing anything on the old laptop: its bytes
/// and its log's are copied and read in the parser process.
pub fn sqlite_tables(path: &Path) -> Result<Vec<Table>, ContentError> {
    Ok(facts_at(path, Instant::now() + Duration::from_secs(8))?.tables)
}

/// The name every private database snapshot folder started with, in the system's temporary
/// folder (an earlier PCTwin copied a live database there to read it; this one reads it in
/// memory), so ones left by a PCTwin that was stopped mid-read can be found and removed.
pub const SNAPSHOT_PREFIX: &str = "pctwin-snapshot-";

/// Removes database snapshot folders an earlier PCTwin left in the temporary folder when it was
/// stopped mid-read (closed, crashed, laptop shut). One still in use cannot be removed on Windows
/// and is left alone. Returns how many were removed.
pub fn sweep_snapshots() -> usize {
    let Ok(dir) = std::fs::read_dir(std::env::temp_dir()) else {
        return 0;
    };
    let mut removed = 0;
    for entry in dir.flatten() {
        let named = entry
            .file_name()
            .to_string_lossy()
            .starts_with(SNAPSHOT_PREFIX);
        let folder = entry.file_type().is_ok_and(|t| t.is_dir());
        if named && folder && std::fs::remove_dir_all(entry.path()).is_ok() {
            removed += 1;
        }
    }
    removed
}

/// SQLite's write-ahead log: its magic numbers (the last bit says the checksums' byte order).
const WAL_MAGIC_LE: u32 = 0x377f_0682;
const WAL_MAGIC_BE: u32 = 0x377f_0683;
const WAL_HEADER: usize = 32;
/// The only log format version SQLite writes (3.7.0).
const WAL_VERSION: u32 = 3_007_000;
const FRAME_HEADER: usize = 24;

fn be32(b: &[u8], at: usize) -> u32 {
    u32::from_be_bytes([b[at], b[at + 1], b[at + 2], b[at + 3]])
}

/// SQLite's log checksum (sqlite.org/fileformat2.html, section 4.1), continued from `s`.
fn wal_checksum(big_endian: bool, s: (u32, u32), data: &[u8]) -> (u32, u32) {
    let (mut s0, mut s1) = s;
    for w in data.as_chunks::<8>().0 {
        let word = |b: &[u8]| {
            let a = [b[0], b[1], b[2], b[3]];
            if big_endian {
                u32::from_be_bytes(a)
            } else {
                u32::from_le_bytes(a)
            }
        };
        s0 = s0.wrapping_add(word(&w[..4])).wrapping_add(s1);
        s1 = s1.wrapping_add(word(&w[4..])).wrapping_add(s0);
    }
    (s0, s1)
}

/// Lays the committed frames of a write-ahead log over a database's pages, as SQLite reads them:
/// only frames whose salts match the log's header and whose running checksum holds, up to the
/// last frame that ends a transaction; anything after it (an unfinished write) is left out. A
/// log whose header is not sound is no log (SQLite ignores it too), and so is any log beside an
/// empty database (SQLite opens an empty database as empty and drops its log). A frame naming a
/// page past [`MAX_DB`] makes the database too large to read; nothing is grown for it.
pub fn apply_log(_: InChild, db: &mut Vec<u8>, log: &[u8]) -> Result<(), ContentError> {
    if log.len() < WAL_HEADER || db.is_empty() {
        return Ok(());
    }
    let big_endian = match be32(log, 0) {
        WAL_MAGIC_BE => true,
        WAL_MAGIC_LE => false,
        _ => return Ok(()),
    };
    if be32(log, 4) != WAL_VERSION {
        return Ok(());
    }
    let page = be32(log, 8) as usize;
    if !(512..=65_536).contains(&page) || !page.is_power_of_two() {
        return Ok(());
    }
    // The most pages a database PCTwin reads may have.
    let most_pages = (MAX_DB / page as u64) as usize;
    let salts = (be32(log, 16), be32(log, 20));
    let mut sum = wal_checksum(big_endian, (0, 0), &log[..24]);
    if sum != (be32(log, 24), be32(log, 28)) {
        return Ok(());
    }
    if db.len() >= 100 {
        let db_page = match u16::from_be_bytes([db[16], db[17]]) {
            1 => 65_536,
            p => usize::from(p),
        };
        if db_page != page {
            return Err(ContentError::Damaged(
                "its log is for pages of another size".into(),
            ));
        }
    }
    let mut pending: Vec<(usize, usize)> = Vec::new();
    let mut at = WAL_HEADER;
    while let Some(frame) = at
        .checked_add(FRAME_HEADER + page)
        .and_then(|end| log.get(at..end))
    {
        let (number, commit) = (be32(frame, 0) as usize, be32(frame, 4) as usize);
        if (be32(frame, 8), be32(frame, 12)) != salts || number == 0 {
            break;
        }
        if number > most_pages || commit > most_pages {
            return Err(ContentError::TooLarge);
        }
        sum = wal_checksum(big_endian, sum, &frame[..8]);
        sum = wal_checksum(big_endian, sum, &frame[FRAME_HEADER..]);
        if sum != (be32(frame, 16), be32(frame, 20)) {
            break;
        }
        pending.push((number, at + FRAME_HEADER));
        if commit != 0 {
            // Both at most `most_pages`, so neither product passes MAX_DB.
            let end = commit * page;
            for (number, from) in pending.drain(..) {
                let start = (number - 1) * page;
                if start + page > db.len() {
                    db.resize(start + page, 0);
                }
                db[start..start + page].copy_from_slice(&log[from..from + page]);
            }
            db.resize(end, 0);
        }
        at += FRAME_HEADER + page;
    }
    Ok(())
}

/// The database's pages with its log laid over, ready for SQLite to read from memory (legacy
/// mode: the log is in already), or why not.
fn image_of(seal: InChild, db: &[u8], log: Option<&[u8]>) -> Result<Vec<u8>, ContentError> {
    if db.len() as u64 > MAX_DB || log.is_some_and(|l| l.len() as u64 > MAX_LOG) {
        return Err(ContentError::TooLarge);
    }
    let mut image = db.to_vec();
    if let Some(log) = log {
        apply_log(seal, &mut image, log)?;
    }
    if image.len() >= 100 {
        if &image[..16] != b"SQLite format 3\0" {
            return Err(ContentError::Damaged("file is not a database".into()));
        }
        image[18] = 1;
        image[19] = 1;
    } else if !image.is_empty() {
        return Err(ContentError::Damaged("file is not a database".into()));
    }
    Ok(image)
}

/// SQLite reading `image` from memory, read-only.
fn memory_db(image: &[u8]) -> Result<rusqlite::Connection, ContentError> {
    let damaged = |e: &dyn std::fmt::Display| ContentError::Damaged(e.to_string());
    let mut c = rusqlite::Connection::open_in_memory().map_err(|e| damaged(&e))?;
    if !image.is_empty() {
        c.deserialize_read_exact(rusqlite::MAIN_DB, image, image.len(), true)
            .map_err(|e| damaged(&e))?;
    }
    Ok(c)
}

/// One complete database from a database's bytes and its log's: the log's committed changes
/// laid over its pages, kept only if SQLite's own `quick_check` says "ok". In the parser process
/// (it takes an [`InChild`]).
pub fn db_snapshot(seal: InChild, db: &[u8], log: Option<&[u8]>) -> Result<Vec<u8>, ContentError> {
    let image = image_of(seal, db, log)?;
    let c = memory_db(&image)?;
    let verdict: String = c
        .query_row("PRAGMA quick_check", [], |r| r.get(0))
        .map_err(|e| ContentError::Damaged(e.to_string()))?;
    if verdict != "ok" {
        return Err(ContentError::Damaged(verdict));
    }
    drop(c);
    Ok(image)
}

/// A database's facts from its bytes (and its log's), read by SQLite from memory, read-only, in
/// the parser process (it takes an [`InChild`]); a scan of linked paths stops at `until`.
pub fn db_facts(
    seal: InChild,
    db: &[u8],
    log: Option<&[u8]>,
    until: Instant,
) -> Result<DbFacts, ContentError> {
    let image = image_of(seal, db, log)?;
    let mut c = memory_db(&image)?;
    let tables: Vec<Table> = tables_as_named(&c)?
        .into_iter()
        .map(|t| Table {
            name: t.name.to_ascii_lowercase(),
            columns: t.columns.iter().map(|c| c.to_ascii_lowercase()).collect(),
        })
        .collect();
    let kind = db_kind(&tables);
    let paths = if kind == DbKind::Ordinary {
        Some(paths_in(&mut c, until)?)
    } else {
        None
    };
    Ok(DbFacts {
        tables,
        kind,
        paths,
    })
}

/// The tables and columns as the database names them.
fn tables_as_named(db: &rusqlite::Connection) -> Result<Vec<Table>, ContentError> {
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
            name,
            columns: cols,
        });
    }
    Ok(out)
}

/// A name written as an SQL identifier.
fn ident(name: &str) -> String {
    format!("\"{}\"", name.replace('"', "\"\""))
}

/// The full local paths a database names, and whether every value was read.
pub type LinkedPaths = (Vec<String>, bool);

/// A database's tables, kind and linked paths (see [`sqlite_facts`]), from one reading of it in
/// the parser process.
pub fn sqlite_db_facts(path: &Path, until: Instant) -> Result<DbFacts, ContentError> {
    facts_at(path, until)
}

/// A database's kind and, for ordinary data, the full local paths its place-named columns hold,
/// from one reading of it in the parser process. The scan of linked paths stops at `until`: the
/// paths found by then come with `false` (not every value was read).
pub fn sqlite_facts(
    path: &Path,
    until: Instant,
) -> Result<(DbKind, Option<LinkedPaths>), ContentError> {
    let facts = facts_at(path, until)?;
    Ok((facts.kind, facts.paths))
}

fn paths_in(
    db: &mut rusqlite::Connection,
    until: Instant,
) -> Result<(Vec<String>, bool), ContentError> {
    db.progress_handler(10_000, Some(move || Instant::now() >= until))
        .map_err(|e| ContentError::Damaged(e.to_string()))?;
    let interrupted = |e: &rusqlite::Error| {
        matches!(e, rusqlite::Error::SqliteFailure(f, _)
            if f.code == rusqlite::ErrorCode::OperationInterrupted)
    };
    let bad = |e: rusqlite::Error| ContentError::Damaged(e.to_string());
    let mut out = Vec::new();
    let mut read = 0usize;
    'tables: for t in tables_as_named(db)? {
        // Only columns whose names say they hold places (paths, files, folders, locations,
        // addresses): the same words for every app; other columns are not read.
        for c in t.columns.iter().filter(|c| place_column(c)) {
            let sql = format!(
                "SELECT {col} FROM {table} WHERE typeof({col}) = 'text' LIMIT {limit}",
                col = ident(c),
                table = ident(&t.name),
                limit = MAX_VALUES.saturating_sub(read)
            );
            let mut st = db.prepare(&sql).map_err(bad)?;
            let mut rows = st.query([]).map_err(bad)?;
            loop {
                let r = match rows.next() {
                    Ok(Some(r)) => r,
                    Ok(None) => break,
                    Err(e) if interrupted(&e) => {
                        out.sort();
                        out.dedup();
                        return Ok((out, false));
                    }
                    Err(e) => return Err(bad(e)),
                };
                read += 1;
                let v: String = r.get(0).map_err(bad)?;
                if local_path(&v) && out.len() < MAX_PATHS {
                    out.push(v);
                }
                if read >= MAX_VALUES {
                    break 'tables;
                }
            }
        }
    }
    out.sort();
    out.dedup();
    Ok((out, true))
}

/// Whether a column's name says it holds a place.
fn place_column(name: &str) -> bool {
    let n = name.to_ascii_lowercase();
    ["path", "file", "folder", "dir", "location", "uri", "url"]
        .iter()
        .any(|w| n.contains(w))
}

/// What kind of store a database is, by its tables' shape.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
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

    #[test]
    fn bookmarks_are_told_by_their_trees_shape_never_by_a_name() {
        let tree = |t: &str| bookmark_tree(&serde_json::from_str(t).unwrap());
        assert!(tree(
            r#"{"roots":{"bar":{"children":[{"type":"url","url":"https://example.org/"}]}}}"#
        ));
        assert!(tree(
            r#"{"children":[{"type":"text/x-moz-place","uri":"https://example.org/"}]}"#
        ));
        // A url node outside a `roots` tree, an empty address, or another type is not.
        assert!(!tree(
            r#"{"items":[{"type":"url","url":"https://example.org/"}]}"#
        ));
        assert!(!tree(
            r#"{"roots":{"bar":{"children":[{"type":"url","url":"  "}]}}}"#
        ));
        assert!(!tree(
            r#"{"roots":{"bar":{"children":[{"type":"folder","url":"x"}]}}}"#
        ));
        assert!(!tree(r#"{"roots":[{"type":"url","url":"x"}]}"#));
        assert!(!tree(r#"{"roots":"x"}"#));
        assert!(!tree(
            r#"{"children":[{"type":"text/x-moz-place","uri":""}]}"#
        ));
        assert!(!tree(r#"{"children":[{"type":"text/plain","uri":"x"}]}"#));
        let facts = json_facts(
            pctwin_parse_seal::in_child(),
            br#"{"roots":{"b":{"children":[{"type":"url","url":"https://example.org/"}]}}}"#,
        )
        .unwrap();
        assert!(facts.bookmarks);
        assert!(
            !json_facts(pctwin_parse_seal::in_child(), b"{}")
                .unwrap()
                .bookmarks
        );
    }

    #[test]
    fn browsing_data_is_told_by_the_tables_shape() {
        let t = |name: &str, cols: &[&str]| Table {
            name: name.into(),
            columns: cols.iter().map(|c| c.to_string()).collect(),
        };
        assert!(browsing(&[t("urls", &["id", "url", "visit_count"])]));
        assert!(browsing(&[t("places", &["url", "last_visit_date"])]));
        assert!(browsing(&[t("moz_bookmarks", &["fk", "title"])]));
        assert!(!browsing(&[t("urls", &["id", "url", "title"])]));
        assert!(!browsing(&[t("links", &["href", "visit_count"])]));
        assert!(!browsing(&[t("bookmarks", &["fk", "name"])]));
        assert!(!browsing(&[t("notes", &["title", "body"])]));
        assert!(!browsing(&[]));
    }

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
        let f = json_facts(pctwin_parse_seal::in_child(), doc.as_bytes()).unwrap();
        assert!(f.machine_bound_key);
        assert_eq!(f.key_db_encrypted, 2);
        assert!(!f.credential_values && !f.starts_programs);
        let f = json_facts(
            pctwin_parse_seal::in_child(),
            format!(r#"["{appb}"]"#).as_bytes(),
        )
        .unwrap();
        assert!(f.machine_bound_key);
        // Base64 of anything else is not either.
        let plain = STANDARD.encode(b"just some ordinary bytes here");
        let f = json_facts(
            pctwin_parse_seal::in_child(),
            format!(r#"["{plain}"]"#).as_bytes(),
        )
        .unwrap();
        assert!(!f.machine_bound_key && f.key_db_encrypted == 0);
    }

    #[test]
    fn credentials_commands_and_references_are_found() {
        let f = json_facts(pctwin_parse_seal::in_child(), br#"{"token":"ghp_0123456789abcdefABCDEF0123456789abcd",
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
        let f = json_facts(pctwin_parse_seal::in_child(), br#"{"preCommandHook":"x"}"#).unwrap();
        assert!(f.starts_programs, "camelCase words are split");
        let f = json_facts(
            pctwin_parse_seal::in_child(),
            br#"{"commander":"x","title":"echo | grep"}"#,
        )
        .unwrap();
        assert!(
            !f.starts_programs,
            "a word containing a run word is not one"
        );
    }

    #[test]
    fn json_reading_is_bounded() {
        assert_eq!(
            json_facts(pctwin_parse_seal::in_child(), &vec![b' '; MAX_JSON + 1]).unwrap_err(),
            ContentError::TooLarge
        );
        let deep = "[".repeat(10_000);
        assert!(matches!(
            json_facts(pctwin_parse_seal::in_child(), deep.as_bytes()),
            Err(ContentError::Damaged(_))
        ));
        assert!(json_facts(pctwin_parse_seal::in_child(), b"\xef\xbb\xbf{}").is_ok());
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
        install_reader(&IN_TESTS);
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
