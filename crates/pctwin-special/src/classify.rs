//! Each app data folder, split into components and classified by content (work package B25-01;
//! ADOPTION change 8: discovery first, the catalogue only enriches).
//!
//! [`classify_folder`] walks one folder found by [`crate::roots`] (never following a link,
//! bounded in depth and count), tells each file by its bytes ([`crate::sniff`]) and its inside
//! ([`crate::content`]), and builds components:
//!
//! - **files that only work together are one component**, found by how their formats relate,
//!   never by their names: a database and its write-ahead log or journal (SQLite's own naming
//!   rule); a LevelDB store's whole folder (its `CURRENT` file names its manifest); a key database
//!   and the files whose values are encrypted under it; model weights and the files that list
//!   them, by name or by hash;
//! - **custody by general rules**: key material bound to this laptop or a device and stores of
//!   sessions, tokens or machine-encrypted values are D (never copied); saved passwords are E
//!   (the app's own export); a server mailbox's offline copy is S; databases are P with a
//!   consistent capture; secrets the person may choose (private keys, password databases, key
//!   stores) are P and secret; anything whose evidence cannot be read is unresolved;
//! - **not moved, with the reason**: caches (by the system's cache places, a cache tag, a cache
//!   format or a cache folder), indexes a database rebuilds, programs and add-ons (2.1 gets code
//!   again), links;
//! - an app that keeps sign-ins gets one S component: its account, which comes back by signing
//!   in.
//!
//! A catalogue row ([`SpecialRow`]) adds a friendly name, an export route, stricter exceptions
//! and cross-system places; with no rows, everything is still classified. Nothing found is
//! dropped: every file ends in a component or in [`FolderReport::not_moved`] with its reason.

use std::collections::{BTreeMap, BTreeSet};
use std::io::{Read, Seek, SeekFrom};
use std::path::{Path, PathBuf};

use pctwin_catalogue::Catalogue;

use crate::catalogue::SpecialRow;
use crate::component::{Component, Custody, Evidence};
use crate::content::{self, DbKind, JsonFacts};
use crate::guard;
use crate::roots::{AppDataFolder, RootKind, System};
use crate::sniff::{self, Shape};

/// The deepest a folder is walked.
pub const MAX_DEPTH: usize = 24;
/// The most files read in one app data folder; past it, the rest is reported, not read.
pub const MAX_FILES: usize = 20_000;
/// The largest text file read for credential-shaped values (settings-sized; larger text is
/// logs and documents).
const MAX_TEXT: u64 = 256 * 1024;
/// The most time one app data folder's walk may take; the rest is "not fully checked".
pub const FOLDER_TIME: std::time::Duration = std::time::Duration::from_secs(60);
/// How many app folders are classified at once.
pub const PLAN_THREADS: usize = 4;
/// How much of a GGUF file's start is read for its split facts.
const GGUF_HEADER: u64 = 4 * 1024 * 1024;

/// The folders web engines (every Chromium-based browser and Electron app) keep site storage in:
/// local and session storage, IndexedDB, service workers. Sign-in tokens live there, and the
/// storage belongs to its origin: it is never copied (plan section 7).
const WEB_ENGINE_STORAGE: &[&str] = &[
    "local storage",
    "session storage",
    "indexeddb",
    "service worker",
    "blob_storage",
    "webstorage",
    "shared_proto_db",
];

/// Why a found file or folder does not move.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Skip {
    /// A cache the app rebuilds.
    Cache,
    /// An index a database rebuilds by itself (SQLite's shared-memory file, LMDB's lock file).
    Rebuilt,
    /// A program, library or script: 2.1 gets code again, never by copying it.
    Program,
    /// A link: never followed.
    Link,
    /// A browser add-on (a folder with a WebExtension manifest): code, installed again from its
    /// store through 2.1, never copied.
    AddOn,
    /// Kept online by a cloud app (or offline): opening it would download it, so it is never
    /// opened here.
    OnlineOnly,
    /// A pipe, socket, device or other entry that is not a file.
    NotAFile,
    /// Past the walk's depth or count limit: found, not read.
    NotRead,
}

impl Skip {
    pub fn words(&self) -> &'static str {
        match self {
            Skip::Cache => "a cache: the app makes it again",
            Skip::Rebuilt => "an index the database rebuilds by itself",
            Skip::Program => "a program or add-on: apps are installed again, never copied",
            Skip::Link => "a link to somewhere else: links are not followed",
            Skip::AddOn => "a browser add-on: it is installed again from its store, never copied",
            Skip::NotRead => {
                "found, not fully checked: too deep, too many files, or out of time here"
            }
            Skip::OnlineOnly => {
                "kept online by a cloud app: not downloaded or moved here; it stays in its cloud"
            }
            Skip::NotAFile => "not a file (a pipe, socket or device): nothing to move",
        }
    }
}

/// A file or folder that does not move, and why.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct NotMoved {
    pub rel: String,
    pub why: Skip,
}

/// One app data folder, split and classified.
#[derive(Debug, Clone)]
pub struct FolderReport {
    pub folder: AppDataFolder,
    /// The catalogue row that knows it, if any.
    pub row: Option<SpecialRow>,
    pub components: Vec<Component>,
    pub not_moved: Vec<NotMoved>,
}

impl FolderReport {
    /// The name to show: the row's, else the folder's own.
    pub fn name(&self) -> String {
        self.row
            .as_ref()
            .and_then(|r| r.name.clone())
            .unwrap_or_else(|| self.folder.rel.clone())
    }
}

/// A file found in the walk.
#[derive(Debug, Clone)]
struct Found {
    rel: String,
    path: PathBuf,
    len: u64,
    shape: Shape,
    /// For bytes of no known format: whether they look encrypted (near-random).
    random: bool,
}

/// Whether bytes look encrypted: near-random (Shannon entropy at least 7.5 bits per byte) over
/// at least 1 KiB. Compressed data looks the same; this is used only where a key bound to this
/// laptop sits beside them.
fn near_random(b: &[u8]) -> bool {
    if b.len() < 1024 {
        return false;
    }
    let mut counts = [0u32; 256];
    for x in b {
        counts[usize::from(*x)] += 1;
    }
    let n = b.len() as f64;
    let e: f64 = counts
        .iter()
        .filter(|c| **c > 0)
        .map(|c| {
            let p = f64::from(*c) / n;
            -p * p.log2()
        })
        .sum();
    e >= 7.5
}

fn read_ends(path: &Path, len: u64) -> std::io::Result<(Vec<u8>, Vec<u8>)> {
    let mut f = guard::open(path)?;
    let mut head = vec![0u8; (len as usize).min(sniff::HEAD)];
    f.read_exact(&mut head)?;
    let mut tail = Vec::new();
    if len >= sniff::TAIL as u64 {
        f.seek(SeekFrom::Start(len - sniff::TAIL as u64))?;
        tail = vec![0u8; sniff::TAIL];
        f.read_exact(&mut tail)?;
    }
    Ok((head, tail))
}

/// What a file holds at its ends, read within [`guard::READ_LIMIT`].
fn ends(path: &Path, len: u64) -> Result<(Vec<u8>, Vec<u8>), String> {
    let p = path.to_path_buf();
    guard::bounded(guard::READ_LIMIT, move || read_ends(&p, len))
        .ok_or_else(|| NO_ANSWER.to_string())?
        .map_err(|e| e.kind().to_string())
}

/// Reads a whole file of at most `max` bytes within [`guard::READ_LIMIT`].
fn whole(path: &Path, max: u64) -> Result<Vec<u8>, String> {
    let p = path.to_path_buf();
    guard::bounded(guard::READ_LIMIT, move || guard::read_all(&p, max))
        .ok_or_else(|| NO_ANSWER.to_string())?
        .map_err(|e| e.to_string())
}

/// Why a file was not read through.
const NO_ANSWER: &str = "it did not answer in time: not fully checked";
/// Why a file was not looked into.
const OUT_OF_TIME: &str = "not fully checked: the time for this folder ran out";

/// Runs `f` on `path` within [`guard::READ_LIMIT`].
fn within<T: Send + 'static>(
    path: &Path,
    f: impl FnOnce(&Path) -> T + Send + 'static,
) -> Option<T> {
    let p = path.to_path_buf();
    guard::bounded(guard::READ_LIMIT, move || f(&p))
}

/// A folder holding a WebExtension manifest (`manifest.json` with a `manifest_version`): the
/// standard format of browser add-ons, the same for every browser.
fn is_add_on(dir: &Path) -> bool {
    let m = dir.join("manifest.json");
    let Ok(meta) = std::fs::symlink_metadata(&m) else {
        return false;
    };
    if guard::kind_of(&meta) != guard::Kind::File || meta.len() > 1024 * 1024 {
        return false;
    }
    let Ok(bytes) = whole(&m, 1024 * 1024) else {
        return false;
    };
    let bytes = bytes.strip_prefix(b"\xef\xbb\xbf").unwrap_or(&bytes);
    if pctwin_types::nesting::check_bytes(bytes, pctwin_types::nesting::Syntax::Json).is_err() {
        return false;
    }
    serde_json::from_slice::<serde_json::Value>(bytes)
        .ok()
        .and_then(|v| v.get("manifest_version").map(serde_json::Value::is_u64))
        .unwrap_or(false)
}

fn is_cache_folder(name: &str, path: &Path) -> bool {
    if name.to_lowercase().contains("cache") {
        return true;
    }
    // A cache tag, by its signature line (the file's name is the standard's, its bytes decide).
    let tag = path.join("CACHEDIR.TAG");
    std::fs::symlink_metadata(&tag).is_ok_and(|m| {
        guard::kind_of(&m) == guard::Kind::File
            && ends(&tag, m.len())
                .is_ok_and(|(h, t)| sniff::shape(&h, &t, h.len() as u64) == Shape::CacheTag)
    })
}

struct Walk {
    files: Vec<Found>,
    web: Vec<String>,
    not_moved: Vec<NotMoved>,
    unreadable: Vec<(String, String)>,
    /// The deepest folder and most files read ([`MAX_DEPTH`], [`MAX_FILES`]; smaller in tests).
    max_depth: usize,
    max_files: usize,
    /// Past this moment, nothing more is read: the rest is reported "not fully checked".
    deadline: std::time::Instant,
}

fn walk(dir: &Path, rel: &str, depth: usize, w: &mut Walk) {
    let entries = match std::fs::read_dir(dir) {
        Ok(e) => e,
        Err(e) => {
            w.unreadable.push((rel.to_string(), e.kind().to_string()));
            return;
        }
    };
    let mut list: Vec<_> = entries.flatten().collect();
    list.sort_by_key(|e| e.file_name());
    for e in list {
        let Ok(name) = e.file_name().into_string() else {
            w.unreadable
                .push((rel.to_string(), "a name the system cannot spell".into()));
            continue;
        };
        let child = if rel.is_empty() {
            name.clone()
        } else {
            format!("{rel}/{name}")
        };
        let path = e.path();
        let Ok(meta) = std::fs::symlink_metadata(&path) else {
            w.unreadable.push((child, "could not be looked at".into()));
            continue;
        };
        // What it is from its attributes alone: nothing that is not a plain local file or
        // folder is ever opened or listed.
        let skip = match guard::kind_of(&meta) {
            guard::Kind::Link => Some(Skip::Link),
            guard::Kind::OnlineOnly => Some(Skip::OnlineOnly),
            guard::Kind::NotAFile => Some(Skip::NotAFile),
            guard::Kind::File | guard::Kind::Folder => None,
        };
        let skip =
            skip.or_else(|| (std::time::Instant::now() >= w.deadline).then_some(Skip::NotRead));
        if let Some(why) = skip {
            w.not_moved.push(NotMoved { rel: child, why });
            continue;
        }
        if meta.is_dir() {
            if is_add_on(&path) {
                w.not_moved.push(NotMoved {
                    rel: child,
                    why: Skip::AddOn,
                });
            } else if WEB_ENGINE_STORAGE.contains(&name.to_lowercase().as_str()) {
                w.web.push(child);
            } else if is_cache_folder(&name, &path) {
                w.not_moved.push(NotMoved {
                    rel: child,
                    why: Skip::Cache,
                });
            } else if depth + 1 > w.max_depth || w.files.len() >= w.max_files {
                w.not_moved.push(NotMoved {
                    rel: child,
                    why: Skip::NotRead,
                });
            } else {
                walk(&path, &child, depth + 1, w);
            }
            continue;
        }
        if !meta.is_file() {
            continue;
        }
        if w.files.len() >= w.max_files {
            w.not_moved.push(NotMoved {
                rel: child,
                why: Skip::NotRead,
            });
            continue;
        }
        let len = meta.len();
        match ends(&path, len) {
            Ok((head, tail)) => {
                let shape = sniff::shape(&head, &tail, len);
                let random = shape == Shape::Binary && near_random(&head);
                w.files.push(Found {
                    rel: child,
                    path,
                    len,
                    shape,
                    random,
                })
            }
            Err(why) => w.unreadable.push((child, why)),
        }
    }
}

/// The hash a file is named by, if it is named by one (`<64 hex>`, `sha256-<64 hex>`).
pub fn hash_name(name: &str) -> Option<String> {
    let h = name
        .strip_prefix("sha256-")
        .or_else(|| name.strip_prefix("sha256_"))
        .unwrap_or(name);
    (h.len() == 64 && h.bytes().all(|b| b.is_ascii_hexdigit())).then(|| h.to_ascii_lowercase())
}

fn file_name(rel: &str) -> &str {
    rel.rsplit('/').next().unwrap_or(rel)
}

fn parent(rel: &str) -> &str {
    rel.rsplit_once('/').map_or("", |(p, _)| p)
}

/// Whether a text file's values look like credentials (`key=value`, `key: value`, or a whole
/// line).
fn text_has_credentials(path: &Path, len: u64) -> bool {
    if len > MAX_TEXT {
        return false;
    }
    let Ok(bytes) = whole(path, MAX_TEXT) else {
        return false;
    };
    let Ok(text) = String::from_utf8(bytes) else {
        return false;
    };
    text.lines().any(|l| {
        let v = l
            .split_once('=')
            .or_else(|| l.split_once(':'))
            .map_or(l, |(_, v)| v)
            .trim()
            .trim_matches(['"', '\'']);
        pctwin_settings::classify::looks_secret(v).is_some()
    })
}

/// A GGUF file's split facts (`split.no`, `split.count`), read from its header.
pub fn gguf_split(path: &Path) -> Option<(u16, u16)> {
    within(path, |p| {
        let f = guard::open(p).ok()?;
        let mut buf = Vec::new();
        f.take(GGUF_HEADER).read_to_end(&mut buf).ok()?;
        gguf_split_in(&buf)
    })
    .flatten()
}

fn gguf_split_in(b: &[u8]) -> Option<(u16, u16)> {
    let rd = |at: usize, n: usize| b.get(at..at + n);
    let u32_at = |at: usize| Some(u32::from_le_bytes(rd(at, 4)?.try_into().ok()?));
    let u64_at = |at: usize| Some(u64::from_le_bytes(rd(at, 8)?.try_into().ok()?));
    if rd(0, 4)? != b"GGUF" || !(2..=3).contains(&u32_at(4)?) {
        return None;
    }
    let kv_count = u64_at(16)?;
    let mut at = 24usize;
    let (mut no, mut count) = (None, None);
    // Skips one value of type `t` at `at`, returning where the next starts.
    fn skip(b: &[u8], t: u32, at: usize, depth: u8) -> Option<usize> {
        let fixed = |n: usize| (at + n <= b.len()).then_some(at + n);
        match t {
            0 | 1 | 7 => fixed(1),
            2 | 3 => fixed(2),
            4..=6 => fixed(4),
            10..=12 => fixed(8),
            8 => {
                let n = usize::try_from(u64::from_le_bytes(b.get(at..at + 8)?.try_into().ok()?))
                    .ok()?;
                (at + 8 + n <= b.len()).then_some(at + 8 + n)
            }
            9 if depth < 2 => {
                let it = u32::from_le_bytes(b.get(at..at + 4)?.try_into().ok()?);
                let n = u64::from_le_bytes(b.get(at + 4..at + 12)?.try_into().ok()?);
                let mut p = at + 12;
                for _ in 0..n {
                    p = skip(b, it, p, depth + 1)?;
                }
                Some(p)
            }
            _ => None,
        }
    }
    for _ in 0..kv_count.min(100_000) {
        let klen = usize::try_from(u64_at(at)?).ok()?;
        let key = rd(at + 8, klen)?;
        at += 8 + klen;
        let t = u32_at(at)?;
        at += 4;
        if t == 2 && (key == b"split.no" || key == b"split.count") {
            let v = u16::from_le_bytes(rd(at, 2)?.try_into().ok()?);
            if key == b"split.no" {
                no = Some(v);
            } else {
                count = Some(v);
            }
            if let (Some(n), Some(c)) = (no, count) {
                return Some((n, c));
            }
        }
        at = skip(b, t, at, 0)?;
    }
    None
}

/// A path as compared: `/` between parts, and on Windows one case.
fn comparable(p: &str) -> String {
    let p = p.replace('\\', "/");
    if cfg!(windows) { p.to_lowercase() } else { p }
}

/// How long one question about a linked path may take (a disconnected network drive can take
/// far longer than that to say no).
const LINK_ASK: std::time::Duration = std::time::Duration::from_secs(2);
/// How long all the questions about one component's linked paths may take.
const LINK_TIME: std::time::Duration = std::time::Duration::from_secs(10);

/// Whether `p` is there, asked on the reader within `ask`; `None` when the system did not answer
/// in time (the question is left to finish on its own, never waited for).
fn ask_there(p: &Path, ask: std::time::Duration) -> Option<bool> {
    let p = p.to_path_buf();
    guard::bounded(ask, move || std::fs::symlink_metadata(&p).is_ok())
}

/// What the full local `paths` a component's data names say about what it needs: files inside
/// its own folder move with it; files elsewhere that exist are offered for the person to add
/// (never added silently); files that are not on this laptop are a missing part, and files on a
/// drive that is not connected now are said to be there. A text shaped like a path whose first
/// folder this laptop does not have (`/api/v1`) is not taken for a path. Each drive or top folder is asked about once; a
/// question with no answer in `ask` counts its drive as not connected from then on; past `total`
/// the rest are counted "not checked".
fn link_evidence_within(
    base: &Path,
    paths: &[String],
    ask: std::time::Duration,
    total: std::time::Duration,
) -> Vec<Evidence> {
    let inside = format!(
        "{}/",
        comparable(&base.to_string_lossy()).trim_end_matches('/')
    );
    let deadline = std::time::Instant::now() + total;
    let mut roots: BTreeMap<String, bool> = BTreeMap::new();
    let mut outside: BTreeSet<String> = BTreeSet::new();
    let (mut missing, mut offline, mut unchecked) = (0usize, 0usize, 0usize);
    for p in paths {
        if comparable(p).starts_with(&inside) {
            continue;
        }
        if std::time::Instant::now() >= deadline {
            unchecked += 1;
            continue;
        }
        let path = Path::new(p);
        let (root, is_drive) = if p.as_bytes().get(1) == Some(&b':') {
            (p[..3].to_ascii_uppercase(), true)
        } else {
            let first = p[1..].split('/').next().unwrap_or("");
            if first.is_empty() {
                continue;
            }
            (format!("/{first}"), false)
        };
        let root_here = *roots
            .entry(root.clone())
            .or_insert_with(|| ask_there(Path::new(&root), ask).unwrap_or(false));
        if !root_here {
            // A drive that is not here, or does not answer: on a drive not connected. A text whose
            // first folder this laptop lacks (`/api/v1`) is not a path at all.
            if is_drive {
                offline += 1;
            }
            continue;
        }
        match ask_there(path, ask) {
            Some(true) => {
                let folder = path
                    .parent()
                    .map_or(p.clone(), |d| d.to_string_lossy().into_owned());
                outside.insert(folder);
            }
            Some(false) => missing += 1,
            None => {
                roots.insert(root, false);
                offline += 1;
            }
        }
    }
    let mut ev = Vec::new();
    if missing > 0 {
        ev.push(Evidence::MissingPart {
            what: format!("{missing} file(s) it links to are not on this laptop"),
        });
    }
    if offline > 0 {
        ev.push(Evidence::MissingPart {
            what: format!("{offline} file(s) it links to are on a drive that is not connected"),
        });
    }
    if unchecked > 0 {
        ev.push(Evidence::MissingPart {
            what: format!("{unchecked} file(s) it links to could not be checked in time"),
        });
    }
    if !outside.is_empty() {
        ev.push(Evidence::LinksOutside {
            folders: outside.into_iter().take(20).collect(),
        });
    }
    ev
}

/// Builds the components of one folder.
struct Build<'a> {
    folder: &'a AppDataFolder,
    place: String,
    row: Option<&'a SpecialRow>,
    components: Vec<Component>,
}

impl Build<'_> {
    fn push(
        &mut self,
        members: Vec<String>,
        custody: Custody,
        secret: bool,
        mut ev: Vec<Evidence>,
    ) {
        let mut custody = custody;
        if let Some(row) = self.row {
            for m in &members {
                let (c, why) = row.tighten(m, custody);
                if let Some(why) = why {
                    custody = c;
                    ev.push(Evidence::Catalogue {
                        row: why.to_string(),
                    });
                }
            }
        }
        self.components.push(Component::new(
            self.folder.owner.clone(),
            &self.place,
            members,
            custody,
            secret,
            ev,
        ));
    }
}

/// Splits and classifies the app data folder `folder`, found on `system`, with the catalogue
/// row `row` that knows it (if any).
pub fn classify_folder(folder: &AppDataFolder, row: Option<&SpecialRow>) -> FolderReport {
    classify_within(
        folder,
        row,
        MAX_DEPTH,
        MAX_FILES,
        std::time::Instant::now() + FOLDER_TIME,
    )
}

/// [`classify_folder`] with its walk's limits given.
fn classify_within(
    folder: &AppDataFolder,
    row: Option<&SpecialRow>,
    max_depth: usize,
    max_files: usize,
    deadline: std::time::Instant,
) -> FolderReport {
    let mut w = Walk {
        files: Vec::new(),
        web: Vec::new(),
        not_moved: Vec::new(),
        unreadable: Vec::new(),
        max_depth,
        max_files,
        deadline,
    };
    let base = folder.path();
    match std::fs::symlink_metadata(&base) {
        Ok(m) if m.is_file() => {
            // A folder that is one file (a Mac preferences list).
            if guard::kind_of(&m) != guard::Kind::File {
                w.not_moved.push(NotMoved {
                    rel: String::new(),
                    why: Skip::OnlineOnly,
                });
            } else if let Ok((h, t)) = ends(&base, m.len()) {
                w.files.push(Found {
                    rel: String::new(),
                    path: base.clone(),
                    len: m.len(),
                    shape: sniff::shape(&h, &t, m.len()),
                    random: false,
                });
            } else {
                w.unreadable
                    .push((String::new(), "could not be read".into()));
            }
        }
        Ok(_) => walk(&base, "", 0, &mut w),
        Err(e) => w.unreadable.push((String::new(), e.kind().to_string())),
    }
    let mut b = Build {
        folder,
        place: folder.place(),
        row,
        components: Vec::new(),
    };
    let mut not_moved = std::mem::take(&mut w.not_moved);
    let mut sessions = false;
    // A key bound to this laptop sits here (a protected blob, or a value wrapped by the system):
    // near-random files of no known format beside it are taken to be encrypted under it.
    // (The JSON half is decided once the JSON files are read, below: each is read once.)
    let protected_blob_here = w.files.iter().any(|f| f.shape == Shape::DpapiBlob);

    for (rel, why) in std::mem::take(&mut w.unreadable) {
        b.push(
            vec![rel],
            Custody::Unresolved,
            false,
            vec![Evidence::Unreadable { why }],
        );
    }
    for rel in std::mem::take(&mut w.web) {
        sessions = true;
        b.push(
            vec![rel],
            Custody::MachineBound,
            false,
            vec![Evidence::SessionStore],
        );
    }

    let by_rel: BTreeMap<String, Found> = w.files.into_iter().map(|f| (f.rel.clone(), f)).collect();
    let mut used: BTreeSet<String> = BTreeSet::new();

    // Not moved by their format.
    for f in by_rel.values() {
        let why = match f.shape {
            Shape::Executable => Some(Skip::Program),
            Shape::CacheTag | Shape::CacheEntry => Some(Skip::Cache),
            Shape::SqliteShm | Shape::LmdbLock => Some(Skip::Rebuilt),
            _ => None,
        };
        if let Some(why) = why {
            used.insert(f.rel.clone());
            not_moved.push(NotMoved {
                rel: f.rel.clone(),
                why,
            });
        }
    }

    // LevelDB stores: the whole folder of a `CURRENT` file that names a manifest present there.
    let level_dirs: BTreeSet<String> = by_rel
        .values()
        .filter_map(|f| match &f.shape {
            Shape::LevelDbCurrent { manifest } => {
                let dir = parent(&f.rel);
                let m = if dir.is_empty() {
                    manifest.clone()
                } else {
                    format!("{dir}/{manifest}")
                };
                by_rel.contains_key(&m).then(|| dir.to_string())
            }
            _ => None,
        })
        .collect();
    for dir in &level_dirs {
        let members: Vec<String> = by_rel
            .keys()
            .filter(|r| parent(r) == dir && !used.contains(*r))
            .cloned()
            .collect();
        used.extend(members.iter().cloned());
        b.push(members, Custody::Portable, false, vec![Evidence::LiveStore]);
    }

    // Past the folder's time, nothing more is read: what is left is "not fully checked".
    let deadline = w.deadline;
    let late = || std::time::Instant::now() >= deadline;

    // JSON facts, read once.
    let json: BTreeMap<String, Result<JsonFacts, String>> = by_rel
        .values()
        .filter(|f| f.shape == Shape::Json && !used.contains(&f.rel))
        .map(|f| {
            let facts = if f.len as usize > content::MAX_JSON {
                Err("too large to read".to_string())
            } else if late() {
                Err(OUT_OF_TIME.to_string())
            } else {
                whole(&f.path, content::MAX_JSON as u64)
                    .and_then(|bytes| content::json_facts(&bytes).map_err(|e| e.to_string()))
            };
            (f.rel.clone(), facts)
        })
        .collect();

    let bound_key_here = protected_blob_here
        || json
            .values()
            .any(|j| j.as_ref().is_ok_and(|j| j.machine_bound_key));
    // The linked paths of one component, checked within what is left of the folder's time.
    let links = |paths: &[String]| {
        let left = deadline.saturating_duration_since(std::time::Instant::now());
        link_evidence_within(&base, paths, LINK_ASK, LINK_TIME.min(left))
    };

    // Files not looked into before the folder's time ran out: one unresolved component, never
    // copied, never one line each.
    let mut unchecked: Vec<String> = Vec::new();

    // SQLite databases with their log or journal, and what kind of store each is.
    let mut key_dbs: BTreeMap<String, Vec<String>> = BTreeMap::new();
    for f in by_rel.values() {
        if f.shape != Shape::Sqlite || used.contains(&f.rel) {
            continue;
        }
        let mut members = vec![f.rel.clone()];
        for (suffix, want) in [
            ("-wal", Shape::SqliteWal),
            ("-journal", Shape::SqliteJournal),
        ] {
            let comp = format!("{}{suffix}", f.rel);
            if by_rel.get(&comp).is_some_and(|c| c.shape == want) {
                members.push(comp);
            }
        }
        used.extend(members.iter().cloned());
        let mut ev = vec![Evidence::SqliteDatabase];
        if members.len() > 1 {
            ev.push(Evidence::SqliteCompanion);
        }
        // One opening: its kind, and for ordinary data the paths it links to. SQLite stops its own
        // scan before the reader's limit or the folder's time.
        let until = deadline
            .min(std::time::Instant::now() + guard::READ_LIMIT - std::time::Duration::from_secs(2));
        let facts = if late() {
            Err(content::ContentError::Damaged(OUT_OF_TIME.into()))
        } else {
            within(&f.path, move |p| content::sqlite_facts(p, until))
                .unwrap_or_else(|| Err(content::ContentError::Damaged(NO_ANSWER.into())))
        };
        let (kind, db_paths) = match facts {
            Ok((k, paths)) => (Ok(k), paths),
            Err(e) => (Err(e), None),
        };
        match kind {
            Ok(DbKind::Ordinary) => {
                match db_paths {
                    Some((paths, complete)) => {
                        ev.extend(links(&paths));
                        if !complete {
                            ev.push(Evidence::MissingPart {
                                what: "the files it links to were not all checked in time".into(),
                            });
                        }
                    }
                    None => ev.push(Evidence::MissingPart {
                        what: "the files it links to could not be checked".into(),
                    }),
                }
                b.push(members, Custody::Portable, false, ev)
            }
            Ok(DbKind::KeyDatabase) => {
                key_dbs.insert(f.rel.clone(), members);
            }
            Ok(DbKind::Sessions) => {
                sessions = true;
                ev.push(Evidence::SessionStore);
                b.push(members, Custody::MachineBound, false, ev);
            }
            Ok(DbKind::Credentials) => {
                ev.push(Evidence::CredentialStore);
                b.push(members, Custody::NeedsExport, true, ev);
            }
            Ok(DbKind::MachineEncrypted) => {
                ev.push(Evidence::MachineBoundProtection);
                b.push(members, Custody::MachineBound, false, ev);
            }
            Err(content::ContentError::Damaged(m)) if m == OUT_OF_TIME => {
                unchecked.extend(members);
            }
            Err(e) => {
                ev.push(Evidence::Unreadable { why: e.to_string() });
                b.push(members, Custody::Unresolved, false, ev);
            }
        }
    }
    // A log or journal whose database is not here.
    for f in by_rel.values() {
        if matches!(f.shape, Shape::SqliteWal | Shape::SqliteJournal) && !used.contains(&f.rel) {
            used.insert(f.rel.clone());
            b.push(
                vec![f.rel.clone()],
                Custody::Unresolved,
                false,
                vec![
                    Evidence::SqliteCompanion,
                    Evidence::MissingPart {
                        what: "the database it belongs to".into(),
                    },
                ],
            );
        }
    }

    // Key databases with the files encrypted under them, in the same folder: one secret group.
    let mut encrypted: Vec<String> = json
        .iter()
        .filter(|(r, f)| !used.contains(*r) && f.as_ref().is_ok_and(|f| f.key_db_encrypted > 0))
        .map(|(r, _)| r.clone())
        .collect();
    for (db, mut members) in key_dbs {
        let dir = parent(&db).to_string();
        let mut ev = vec![Evidence::KeyDatabase];
        let (with, rest): (Vec<String>, Vec<String>) =
            encrypted.into_iter().partition(|r| parent(r) == dir);
        encrypted = rest;
        if !with.is_empty() {
            ev.push(Evidence::EncryptedWithKeyDatabase);
        }
        members.extend(with);
        used.extend(members.iter().cloned());
        b.push(members, Custody::Portable, true, ev);
    }
    for r in encrypted {
        used.insert(r.clone());
        b.push(
            vec![r],
            Custody::Unresolved,
            true,
            vec![
                Evidence::EncryptedWithKeyDatabase,
                Evidence::MissingPart {
                    what: "the key store that opens it".into(),
                },
            ],
        );
    }

    // Content-addressed files and the manifests that list them by hash: one model library.
    let hashed: BTreeMap<String, String> = by_rel
        .values()
        .filter(|f| !used.contains(&f.rel))
        .filter_map(|f| hash_name(file_name(&f.rel)).map(|h| (h, f.rel.clone())))
        .collect();
    let manifests: Vec<(String, Vec<String>)> = json
        .iter()
        .filter(|(r, _)| !used.contains(*r))
        .filter_map(|(r, f)| {
            f.as_ref()
                .ok()
                .filter(|f| !f.digests.is_empty())
                .map(|f| (r.clone(), f.digests.clone()))
        })
        .collect();
    if !hashed.is_empty() || !manifests.is_empty() {
        let mut members: Vec<String> = hashed.values().cloned().collect();
        let mut ev = vec![
            Evidence::ModelData,
            Evidence::ContentAddressed { verified: false },
        ];
        let wanted: BTreeSet<&String> = manifests.iter().flat_map(|(_, d)| d).collect();
        let missing = wanted.iter().filter(|d| !hashed.contains_key(**d)).count();
        if missing > 0 {
            ev.push(Evidence::MissingPart {
                what: format!("{missing} file(s) its lists name by fingerprint"),
            });
        }
        members.extend(manifests.into_iter().map(|(r, _)| r));
        used.extend(members.iter().cloned());
        b.push(members, Custody::Portable, false, ev);
    }

    // Model weights: each folder that holds them moves whole, with what lists them.
    let weight_dirs: BTreeSet<String> = by_rel
        .values()
        .filter(|f| !used.contains(&f.rel) && matches!(f.shape, Shape::Gguf | Shape::Safetensors))
        .map(|f| parent(&f.rel).to_string())
        .collect();
    for dir in weight_dirs {
        let members: Vec<String> = by_rel
            .values()
            .filter(|f| parent(&f.rel) == dir && !used.contains(&f.rel))
            .map(|f| f.rel.clone())
            .collect();
        let present: BTreeSet<&str> = members.iter().map(|m| file_name(m)).collect();
        let mut missing: BTreeSet<String> = BTreeSet::new();
        for m in &members {
            if let Some(Ok(f)) = json.get(m) {
                for n in &f.file_names {
                    let l = n.to_ascii_lowercase();
                    if (l.ends_with(".safetensors") || l.ends_with(".gguf"))
                        && !present.contains(n.as_str())
                    {
                        missing.insert(n.clone());
                    }
                }
            }
        }
        // Split GGUF files say their number and how many there are.
        let mut splits: BTreeMap<u16, BTreeSet<u16>> = BTreeMap::new();
        for m in &members {
            if by_rel.get(m).is_some_and(|f| f.shape == Shape::Gguf)
                && !late()
                && let Some((no, count)) = gguf_split(&by_rel[m].path)
            {
                splits.entry(count).or_default().insert(no);
            }
        }
        for (count, have) in splits {
            for n in 0..count {
                if !have.contains(&n) {
                    missing.insert(format!("part {} of {count}", n + 1));
                }
            }
        }
        let mut ev = vec![Evidence::ModelData];
        if !missing.is_empty() {
            ev.push(Evidence::MissingPart {
                what: missing.into_iter().collect::<Vec<_>>().join(", "),
            });
        }
        used.extend(members.iter().cloned());
        b.push(members, Custody::Portable, false, ev);
    }

    // Everything else, one file at a time; plain data gathers into one component.
    let mut plain: Vec<String> = Vec::new();
    for f in by_rel.values() {
        if used.contains(&f.rel) {
            continue;
        }
        let one = vec![f.rel.clone()];
        match &f.shape {
            Shape::DpapiBlob => b.push(
                one,
                Custody::MachineBound,
                false,
                vec![Evidence::MachineBoundProtection],
            ),
            Shape::PasswordDatabase => b.push(
                one,
                Custody::Portable,
                true,
                vec![Evidence::PasswordDatabase],
            ),
            Shape::PasswordKeyFile => b.push(
                one,
                Custody::Portable,
                true,
                vec![Evidence::PasswordKeyFile],
            ),
            Shape::PrivateKey { hardware: true, .. }
            | Shape::GpgPrivateKey { shadowed: true, .. } => b.push(
                one,
                Custody::MachineBound,
                false,
                vec![Evidence::HardwareKeyStub],
            ),
            Shape::PrivateKey {
                encrypted: Some(true),
                ..
            }
            | Shape::GpgPrivateKey {
                protected: true, ..
            }
            | Shape::PgpPrivate => b.push(
                one,
                Custody::Portable,
                true,
                vec![Evidence::EncryptedPrivateKey],
            ),
            Shape::PrivateKey {
                encrypted: Some(false),
                ..
            }
            | Shape::GpgPrivateKey { .. } => b.push(
                one,
                Custody::Portable,
                true,
                vec![Evidence::UnprotectedPrivateKey],
            ),
            Shape::PrivateKey {
                encrypted: None, ..
            } => b.push(
                one,
                Custody::Unresolved,
                true,
                vec![Evidence::Unreadable {
                    why: "its key format could not be read".into(),
                }],
            ),
            Shape::OutlookData { server_cache: true } => {
                let mut c = Component::new(
                    folder.owner.clone(),
                    &b.place,
                    one,
                    Custody::ServiceHeld,
                    false,
                    vec![Evidence::ServerMailCache],
                );
                // Anything never sent to the server may exist only here: a labelled copy is kept.
                c.preserve = true;
                b.components.push(c);
            }
            Shape::OutlookData {
                server_cache: false,
            }
            | Shape::Mbox => b.push(one, Custody::Portable, false, vec![Evidence::MailStore]),
            Shape::LmdbData => b.push(one, Custody::Portable, false, vec![Evidence::LiveStore]),
            Shape::Json => match json.get(&f.rel) {
                Some(Ok(j)) if j.machine_bound_key => b.push(
                    one,
                    Custody::MachineBound,
                    false,
                    vec![Evidence::MachineBoundProtection],
                ),
                Some(Ok(j)) if j.credential_values => {
                    sessions = true;
                    b.push(
                        one,
                        Custody::MachineBound,
                        false,
                        vec![Evidence::CredentialValues],
                    )
                }
                Some(Ok(j)) if j.starts_programs => b.push(
                    one,
                    Custody::Portable,
                    false,
                    vec![Evidence::StructuredData, Evidence::EnablesCommands],
                ),
                Some(Ok(j)) => {
                    let links = links(&j.paths);
                    if links.is_empty() {
                        plain.push(f.rel.clone())
                    } else {
                        let mut ev = vec![Evidence::StructuredData];
                        ev.extend(links);
                        b.push(one, Custody::Portable, false, ev)
                    }
                }
                Some(Err(why)) if why == OUT_OF_TIME => unchecked.push(f.rel.clone()),
                Some(Err(why)) => b.push(
                    one,
                    Custody::Unresolved,
                    false,
                    vec![Evidence::Unreadable { why: why.clone() }],
                ),
                None => plain.push(f.rel.clone()),
            },
            // Not looked into in time: it may hold sign-ins, so it is never simply copied.
            Shape::Ini | Shape::Text if late() => unchecked.push(f.rel.clone()),
            Shape::Ini | Shape::Text if text_has_credentials(&f.path, f.len) => {
                sessions = true;
                b.push(
                    one,
                    Custody::MachineBound,
                    false,
                    vec![Evidence::CredentialValues],
                )
            }
            Shape::SessionData => {
                sessions = true;
                b.push(
                    one,
                    Custody::MachineBound,
                    false,
                    vec![Evidence::SessionStore],
                )
            }
            Shape::Binary if f.random && bound_key_here => b.push(
                one,
                Custody::MachineBound,
                false,
                vec![Evidence::MachineBoundProtection],
            ),
            _ => plain.push(f.rel.clone()),
        }
    }
    if !plain.is_empty() {
        b.push(
            plain,
            Custody::Portable,
            false,
            vec![Evidence::NoSpecialShape],
        );
    }
    if !unchecked.is_empty() {
        b.push(
            unchecked,
            Custody::Unresolved,
            false,
            vec![Evidence::Unreadable {
                why: OUT_OF_TIME.into(),
            }],
        );
    }
    if sessions {
        b.push(
            Vec::new(),
            Custody::ServiceHeld,
            false,
            vec![Evidence::ServiceAccount],
        );
    }
    b.components = merge(std::mem::take(&mut b.components));
    if let Some(row) = row {
        let what = row.name.clone().unwrap_or_else(|| row.folder.clone());
        for c in &mut b.components {
            c.evidence.push(Evidence::Catalogue {
                row: format!("{what} (checked {})", row.checked_on),
            });
        }
    }
    let components = b.components;
    FolderReport {
        folder: folder.clone(),
        row: row.cloned(),
        components,
        not_moved,
    }
}

/// One app's data as the person sees it: its folder's components merged into one per class and
/// step (copied; copied but needs a look; model files; a sign-in; set up again; the app's own
/// export; not known), the files being each one's inventory. Each secret stays its own component,
/// because each is chosen on its own.
fn merge(components: Vec<Component>) -> Vec<Component> {
    let needs_a_look = |c: &Component| {
        c.evidence.iter().any(|e| {
            matches!(
                e,
                Evidence::MissingPart { .. }
                    | Evidence::LinksOutside { .. }
                    | Evidence::EnablesCommands
            )
        })
    };
    let model = |c: &Component| c.evidence.contains(&Evidence::ModelData);
    let mut out: Vec<Component> = Vec::new();
    let mut at: BTreeMap<(u8, bool, bool, bool), usize> = BTreeMap::new();
    for c in components {
        if c.secret {
            out.push(c);
            continue;
        }
        let key = (
            crate::catalogue::strictness(c.custody),
            needs_a_look(&c),
            model(&c),
            c.preserve,
        );
        match at.get(&key) {
            Some(&i) => {
                let g = &mut out[i];
                g.members.extend(c.members);
                for e in c.evidence {
                    if !g.evidence.contains(&e) {
                        g.evidence.push(e);
                    }
                }
            }
            None => {
                at.insert(key, out.len());
                out.push(c);
            }
        }
    }
    for c in &mut out {
        c.members.sort();
        c.members.dedup();
        c.id = crate::component::ComponentId::of(&c.owner, &c.place, &c.members);
    }
    out
}

/// Every folder classified, with the catalogue read through its only matching path: the folders a
/// row knows, and those found with no row ("found, not yet supported"), both classified.
pub fn classify_all(
    folders: Vec<AppDataFolder>,
    system: System,
    catalogue: &Catalogue<SpecialRow>,
) -> Vec<FolderReport> {
    classify_all_within(folders, system, catalogue, ROOT_TIME)
}

/// Every folder classified within one `deadline` for all of them, the most recently written
/// first (by the folder's own last-write time), each also within [`FOLDER_TIME`]. A folder
/// reached after `deadline` is not read: it is reported "not fully checked" as a whole.
pub fn classify_newest_first(
    folders: Vec<AppDataFolder>,
    system: System,
    catalogue: &Catalogue<SpecialRow>,
    deadline: std::time::Instant,
) -> Vec<FolderReport> {
    let reading = catalogue.read(folders, |f, row| row.matches(system, f));
    let mut all: Vec<(&AppDataFolder, Option<&SpecialRow>)> = reading
        .known
        .iter()
        .map(|(f, r)| (f, Some(*r)))
        .chain(reading.found_not_supported.iter().map(|f| (f, None)))
        .collect();
    let written = |f: &AppDataFolder| {
        std::fs::symlink_metadata(f.path())
            .and_then(|m| m.modified())
            .unwrap_or(std::time::UNIX_EPOCH)
    };
    let mut keyed: Vec<(std::time::SystemTime, usize)> = all
        .iter()
        .enumerate()
        .map(|(i, (f, _))| (written(f), i))
        .collect();
    keyed.sort_by(|a, b| b.0.cmp(&a.0).then(a.1.cmp(&b.1)));
    let order: Vec<usize> = keyed.into_iter().map(|(_, i)| i).collect();
    // Several folders at once (each its own reader), results kept in newest-first order.
    let workers = std::thread::available_parallelism()
        .map_or(1, |n| n.get())
        .clamp(1, PLAN_THREADS);
    let next = std::sync::atomic::AtomicUsize::new(0);
    let slots: Vec<std::sync::Mutex<Option<FolderReport>>> =
        order.iter().map(|_| std::sync::Mutex::new(None)).collect();
    std::thread::scope(|s| {
        for _ in 0..workers {
            s.spawn(|| {
                loop {
                    let k = next.fetch_add(1, std::sync::atomic::Ordering::SeqCst);
                    let Some(&i) = order.get(k) else {
                        break;
                    };
                    let (f, row) = all[i];
                    let now = std::time::Instant::now();
                    let report = if now >= deadline {
                        FolderReport {
                            folder: f.clone(),
                            row: row.cloned(),
                            components: Vec::new(),
                            not_moved: vec![NotMoved {
                                rel: String::new(),
                                why: Skip::NotRead,
                            }],
                        }
                    } else {
                        classify_within(
                            f,
                            row,
                            MAX_DEPTH,
                            MAX_FILES,
                            deadline.min(now + FOLDER_TIME),
                        )
                    };
                    *slots[k]
                        .lock()
                        .unwrap_or_else(std::sync::PoisonError::into_inner) = Some(report);
                }
            });
        }
    });
    let out: Vec<FolderReport> = slots
        .into_iter()
        .filter_map(|m| {
            m.into_inner()
                .unwrap_or_else(std::sync::PoisonError::into_inner)
        })
        .collect();
    all.clear();
    out
}

/// The most time all the folders of one root kind may take together; folders past it are
/// reported "not fully checked" without being read.
pub const ROOT_TIME: std::time::Duration = std::time::Duration::from_secs(600);

/// [`classify_all`] with each root kind's time given.
pub fn classify_all_within(
    folders: Vec<AppDataFolder>,
    system: System,
    catalogue: &Catalogue<SpecialRow>,
    root_time: std::time::Duration,
) -> Vec<FolderReport> {
    let reading = catalogue.read(folders, |f, row| row.matches(system, f));
    let mut ends: BTreeMap<RootKind, std::time::Instant> = BTreeMap::new();
    let mut one = |f: &AppDataFolder, row: Option<&SpecialRow>| {
        let now = std::time::Instant::now();
        let root_end = *ends.entry(f.root).or_insert(now + root_time);
        classify_within(
            f,
            row,
            MAX_DEPTH,
            MAX_FILES,
            root_end.min(now + FOLDER_TIME),
        )
    };
    let mut out: Vec<FolderReport> = Vec::new();
    for (f, row) in &reading.known {
        out.push(one(f, Some(row)));
    }
    for f in &reading.found_not_supported {
        out.push(one(f, None));
    }
    out
}

/// Where a folder's data goes on the new laptop.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Placement {
    /// The same root kind and folder (the same kind of system, or a root every system has in the
    /// same place, such as the home's hidden folders).
    Same { root: RootKind, folder: String },
    /// Another system's place for the same app's data, from a catalogue row.
    Translated { root: RootKind, folder: String },
    /// Kept on the new laptop, not handed to the app: the app there may keep its data elsewhere
    /// or in another format, and no row says where.
    PreservedOnly,
}

/// Where `report`'s data goes when moving from `from` to `to`.
pub fn placement(report: &FolderReport, from: System, to: System) -> Placement {
    let f = &report.folder;
    if from == to || f.root == RootKind::HomeDot {
        return Placement::Same {
            root: f.root,
            folder: f.rel.clone(),
        };
    }
    report
        .row
        .as_ref()
        .and_then(|r| r.elsewhere.iter().find(|p| p.system == to))
        .map_or(Placement::PreservedOnly, |p| Placement::Translated {
            root: p.root,
            folder: p.folder.clone(),
        })
}

#[cfg(test)]
mod tests {
    use super::*;

    fn kv(key: &str, t: u32, val: &[u8]) -> Vec<u8> {
        let mut b = (key.len() as u64).to_le_bytes().to_vec();
        b.extend_from_slice(key.as_bytes());
        b.extend_from_slice(&t.to_le_bytes());
        b.extend_from_slice(val);
        b
    }

    pub(crate) fn gguf(parts: &[Vec<u8>]) -> Vec<u8> {
        let mut b = b"GGUF".to_vec();
        b.extend_from_slice(&3u32.to_le_bytes());
        b.extend_from_slice(&0u64.to_le_bytes());
        b.extend_from_slice(&(parts.len() as u64).to_le_bytes());
        for p in parts {
            b.extend_from_slice(p);
        }
        b
    }

    pub(crate) fn split(no: u16, count: u16) -> Vec<u8> {
        let mut arr = 8u32.to_le_bytes().to_vec();
        arr.extend_from_slice(&2u64.to_le_bytes());
        for s in ["a", "bb"] {
            arr.extend_from_slice(&(s.len() as u64).to_le_bytes());
            arr.extend_from_slice(s.as_bytes());
        }
        gguf(&[
            kv("general.name", 8, &[1, 0, 0, 0, 0, 0, 0, 0, b'x']),
            kv("tokenizer.tokens", 9, &arr),
            kv("general.flag", 7, &[1]),
            kv("split.no", 2, &no.to_le_bytes()),
            kv("split.count", 2, &count.to_le_bytes()),
        ])
    }

    #[test]
    fn a_gguf_header_says_which_part_it_is() {
        assert_eq!(gguf_split_in(&split(1, 3)), Some((1, 3)));
        assert_eq!(gguf_split_in(&gguf(&[kv("general.flag", 7, &[1])])), None);
        let mut cut = split(1, 3);
        cut.truncate(cut.len() - 3);
        assert_eq!(gguf_split_in(&cut), None);
        assert_eq!(gguf_split_in(b"GGUF\x01\0\0\0"), None);
        assert_eq!(gguf_split_in(&gguf(&[kv("x", 99, &[0])])), None);
    }

    fn noise(n: usize) -> Vec<u8> {
        let mut x: u64 = 0x2545_f491_4f6c_dd1d;
        (0..n)
            .map(|_| {
                x ^= x << 13;
                x ^= x >> 7;
                x ^= x << 17;
                (x >> 24) as u8
            })
            .collect()
    }

    #[test]
    fn near_random_needs_enough_bytes_and_spread() {
        assert!(!near_random(&noise(1023)));
        assert!(near_random(&noise(1024)));
        assert!(!near_random(&vec![0u8; 4096]));
        // Text-like spread is far below the threshold.
        assert!(!near_random(&b"the quick brown fox ".repeat(100)));
        // Half the byte values, evenly used: 7 bits a byte, below 7.5.
        let half: Vec<u8> = (0..4096u32).map(|i| (i % 128) as u8).collect();
        assert!(!near_random(&half));
    }

    #[test]
    fn every_skip_says_something_of_its_own() {
        let all = [
            Skip::Cache,
            Skip::Rebuilt,
            Skip::Program,
            Skip::Link,
            Skip::AddOn,
            Skip::NotRead,
            Skip::OnlineOnly,
            Skip::NotAFile,
        ];
        let words: std::collections::BTreeSet<&str> = all.iter().map(Skip::words).collect();
        assert_eq!(words.len(), all.len());
        assert!(words.iter().all(|w| w.len() > 10));
    }

    #[test]
    fn an_add_on_manifest_is_read_only_up_to_its_limit() {
        let dir = tempfile::tempdir().unwrap();
        let m = dir.path().join("manifest.json");
        let body = br#"{"manifest_version": 3}"#;
        let pad = |n: usize| {
            let mut b = body.to_vec();
            b.resize(n, b' ');
            b
        };
        std::fs::write(&m, pad(1024 * 1024)).unwrap();
        assert!(is_add_on(dir.path()));
        std::fs::write(&m, pad(1024 * 1024 + 1)).unwrap();
        assert!(!is_add_on(dir.path()));
        std::fs::write(&m, br#"{"manifest_version": "3"}"#).unwrap();
        assert!(!is_add_on(dir.path()), "a version must be a number");
        std::fs::write(&m, br#"{"name": "x"}"#).unwrap();
        assert!(!is_add_on(dir.path()));
        std::fs::remove_file(&m).unwrap();
        std::fs::create_dir(&m).unwrap();
        assert!(
            !is_add_on(dir.path()),
            "a folder named so is not a manifest"
        );
    }

    #[test]
    fn credentials_are_looked_for_through_the_whole_text_up_to_its_limit() {
        let dir = tempfile::tempdir().unwrap();
        let p = dir.path().join("settings.ini");
        let mut text = "a=1\n".repeat(2000);
        text.push_str("token=ghp_0123456789abcdefABCDEF0123456789abcd\n");
        std::fs::write(&p, &text).unwrap();
        assert!(text_has_credentials(&p, text.len() as u64));
        assert!(!text_has_credentials(&p, MAX_TEXT + 1));
        assert!(text_has_credentials(&p, MAX_TEXT));
        std::fs::write(&p, "a=1\nb: plain words\n").unwrap();
        assert!(!text_has_credentials(&p, 20));
    }

    #[test]
    fn a_gguf_split_is_found_past_a_large_vocabulary() {
        // A vocabulary of 200 000 short strings (about 2.4 MB) before the split facts.
        let mut arr = 8u32.to_le_bytes().to_vec();
        arr.extend_from_slice(&200_000u64.to_le_bytes());
        for _ in 0..200_000 {
            arr.extend_from_slice(&4u64.to_le_bytes());
            arr.extend_from_slice(b"word");
        }
        let b = gguf(&[
            kv("tokenizer.tokens", 9, &arr),
            kv("split.no", 2, &1u16.to_le_bytes()),
            kv("split.count", 2, &2u16.to_le_bytes()),
        ]);
        assert!(b.len() as u64 > 2 * 1024 * 1024 && (b.len() as u64) < GGUF_HEADER);
        let dir = tempfile::tempdir().unwrap();
        let p = dir.path().join("m.gguf");
        std::fs::write(&p, &b).unwrap();
        assert_eq!(gguf_split(&p), Some((1, 2)));
    }

    fn folder_at(path: &std::path::Path) -> AppDataFolder {
        AppDataFolder {
            root: RootKind::Roaming,
            root_path: path.parent().unwrap().to_path_buf(),
            rel: path.file_name().unwrap().to_string_lossy().into_owned(),
            owner: pctwin_record::Owner::Shared,
            app: None,
            tie: None,
        }
    }

    #[test]
    fn the_walk_stops_at_its_limits_and_says_so() {
        let dir = tempfile::tempdir().unwrap();
        let app = dir.path().join("App");
        let deep = app.join("a/b/c");
        std::fs::create_dir_all(&deep).unwrap();
        std::fs::write(app.join("a/b/in.txt"), b"x").unwrap();
        std::fs::write(deep.join("too-deep.txt"), b"x").unwrap();
        // Two folders down are read; the third is reported, not read.
        let r = classify_within(&folder_at(&app), None, 2, 100, far());
        let members: Vec<&String> = r.components.iter().flat_map(|c| &c.members).collect();
        assert!(members.contains(&&"a/b/in.txt".to_string()));
        assert!(!members.iter().any(|m| m.ends_with("too-deep.txt")));
        assert!(r.not_moved.contains(&NotMoved {
            rel: "a/b/c".into(),
            why: Skip::NotRead
        }));
        let r = classify_within(&folder_at(&app), None, 3, 100, far());
        assert!(r.not_moved.is_empty());

        // Past the count, files and folders are reported, not read.
        let many = dir.path().join("Many");
        std::fs::create_dir_all(many.join("z-folder")).unwrap();
        for n in ["1.txt", "2.txt", "3.txt", "4.txt"] {
            std::fs::write(many.join(n), b"x").unwrap();
        }
        let r = classify_within(&folder_at(&many), None, 10, 3, far());
        let not_read: Vec<&str> = r
            .not_moved
            .iter()
            .filter(|n| n.why == Skip::NotRead)
            .map(|n| n.rel.as_str())
            .collect();
        assert_eq!(not_read, vec!["4.txt", "z-folder"]);
        let read: usize = r.components.iter().map(|c| c.members.len()).sum();
        assert_eq!(read, 3);
    }

    #[test]
    fn a_folder_with_a_cache_tag_is_a_cache_whatever_its_name() {
        let dir = tempfile::tempdir().unwrap();
        let app = dir.path().join("App");
        std::fs::create_dir_all(app.join("store")).unwrap();
        std::fs::write(
            app.join("store/CACHEDIR.TAG"),
            b"Signature: 8a477f597d28d172789f06886806bc55
",
        )
        .unwrap();
        std::fs::write(app.join("store/blob"), b"x").unwrap();
        std::fs::create_dir_all(app.join("kept")).unwrap();
        std::fs::write(
            app.join("kept/CACHEDIR.TAG"),
            b"not the signature
",
        )
        .unwrap();
        let r = classify_folder(&folder_at(&app), None);
        assert!(r.not_moved.contains(&NotMoved {
            rel: "store".into(),
            why: Skip::Cache
        }));
        let members: Vec<&String> = r.components.iter().flat_map(|c| &c.members).collect();
        assert!(members.contains(&&"kept/CACHEDIR.TAG".to_string()));
        assert!(!members.iter().any(|m| m.starts_with("store")));
    }

    #[test]
    fn a_gguf_header_is_read_through_every_value_type() {
        let s = |t: &str| {
            let mut b = (t.len() as u64).to_le_bytes().to_vec();
            b.extend_from_slice(t.as_bytes());
            b
        };
        let split = [
            kv("split.no", 2, &4u16.to_le_bytes()),
            kv("split.count", 2, &9u16.to_le_bytes()),
        ];
        let mut arr_u32 = 4u32.to_le_bytes().to_vec();
        arr_u32.extend_from_slice(&3u64.to_le_bytes());
        for v in [1u32, 2, 3] {
            arr_u32.extend_from_slice(&v.to_le_bytes());
        }
        let mut nested = 9u32.to_le_bytes().to_vec();
        nested.extend_from_slice(&1u64.to_le_bytes());
        nested.extend_from_slice(&arr_u32);
        let mut kvs = vec![
            kv("u8", 0, &[7]),
            kv("i8", 1, &[7]),
            kv("u16", 2, &7u16.to_le_bytes()),
            kv("i16", 3, &7i16.to_le_bytes()),
            kv("u32", 4, &7u32.to_le_bytes()),
            kv("i32", 5, &7i32.to_le_bytes()),
            kv("f32", 6, &7f32.to_le_bytes()),
            kv("bool", 7, &[1]),
            kv("name", 8, &s("a much longer name than eight bytes")),
            kv("arr", 9, &arr_u32),
            kv("nested", 9, &nested),
            kv("u64", 10, &7u64.to_le_bytes()),
            kv("i64", 11, &7i64.to_le_bytes()),
            kv("f64", 12, &7f64.to_le_bytes()),
        ];
        kvs.extend(split.clone());
        assert_eq!(gguf_split_in(&gguf(&kvs)), Some((4, 9)));
        // Each value cut short: nothing is read past the end.
        let whole = gguf(&kvs);
        for cut in [30, 60, 120, whole.len() - 1] {
            assert_eq!(gguf_split_in(&whole[..cut]), None, "{cut}");
        }
        // A string longer than what is there.
        let mut lie = 1000u64.to_le_bytes().to_vec();
        lie.extend_from_slice(b"short");
        assert_eq!(gguf_split_in(&gguf(&[kv("name", 8, &lie)])), None);
        // Arrays nested three deep are refused.
        let mut deep = 9u32.to_le_bytes().to_vec();
        deep.extend_from_slice(&1u64.to_le_bytes());
        deep.extend_from_slice(&nested);
        let mut d = vec![kv("deep", 9, &deep)];
        d.extend(split.clone());
        assert_eq!(gguf_split_in(&gguf(&d)), None);
        // Only versions 2 and 3.
        for v in [1u32, 4] {
            let mut b = gguf(&split);
            b[4..8].copy_from_slice(&v.to_le_bytes());
            assert_eq!(gguf_split_in(&b), None, "version {v}");
        }
        let mut b = gguf(&split);
        b[4..8].copy_from_slice(&2u32.to_le_bytes());
        assert_eq!(gguf_split_in(&b), Some((4, 9)));
        let mut b = gguf(&split);
        b[0] = b'X';
        assert_eq!(gguf_split_in(&b), None);
        // The split facts in either order; one alone is not enough.
        assert_eq!(
            gguf_split_in(&gguf(&[split[1].clone(), split[0].clone()])),
            Some((4, 9))
        );
        assert_eq!(gguf_split_in(&gguf(&[split[0].clone()])), None);
        // Split facts of another type are not read as the split.
        assert_eq!(
            gguf_split_in(&gguf(&[
                kv("split.no", 4, &4u32.to_le_bytes()),
                split[1].clone()
            ])),
            None
        );
    }

    #[test]
    fn linked_paths_are_checked_within_a_budget() {
        let dir = tempfile::tempdir().unwrap();
        let there = dir.path().join("a.jpg");
        std::fs::write(&there, b"x").unwrap();
        let paths: Vec<String> = (0..5)
            .map(|_| there.to_string_lossy().into_owned())
            .collect();
        let base = dir.path().join("app");
        // No time at all: every one is "not checked", none is claimed there or missing.
        let ev = link_evidence_within(&base, &paths, LINK_ASK, std::time::Duration::ZERO);
        assert_eq!(
            ev,
            vec![Evidence::MissingPart {
                what: "5 file(s) it links to could not be checked in time".into()
            }]
        );
        let ev = link_evidence_within(&base, &paths, LINK_ASK, LINK_TIME);
        assert!(matches!(&ev[..], [Evidence::LinksOutside { .. }]), "{ev:?}");
    }

    fn far() -> std::time::Instant {
        std::time::Instant::now() + std::time::Duration::from_secs(600)
    }

    #[test]
    fn past_its_time_a_folder_is_reported_not_fully_checked_never_read() {
        let dir = tempfile::tempdir().unwrap();
        let app = dir.path().join("App");
        std::fs::create_dir_all(app.join("sub")).unwrap();
        std::fs::write(app.join("a.txt"), b"x").unwrap();
        std::fs::write(app.join("sub/b.txt"), b"x").unwrap();
        let r = classify_within(&folder_at(&app), None, 10, 100, std::time::Instant::now());
        assert!(
            r.components
                .iter()
                .all(|c| c.members.is_empty() || c.custody == Custody::Unresolved)
        );
        let mut not_read: Vec<&str> = r
            .not_moved
            .iter()
            .filter(|n| n.why == Skip::NotRead)
            .map(|n| n.rel.as_str())
            .collect();
        not_read.sort();
        assert_eq!(not_read, vec!["a.txt", "sub"]);
        // Files walked in time but not looked into before the time ran out gather into one
        // unresolved component.
        let dir2 = tempfile::tempdir().unwrap();
        let many = dir2.path().join("Many");
        std::fs::create_dir_all(&many).unwrap();
        for n in 0..20 {
            std::fs::write(many.join(format!("{n}.json")), br#"{"a":1}"#).unwrap();
            std::fs::write(many.join(format!("{n}.ini")), b"a=1\n").unwrap();
        }
        let mut w = Walk {
            files: Vec::new(),
            web: Vec::new(),
            not_moved: Vec::new(),
            unreadable: Vec::new(),
            max_depth: 10,
            max_files: 100,
            deadline: far(),
        };
        walk(&many, "", 0, &mut w);
        assert_eq!(w.files.len(), 40);
        let r = classify_within(
            &folder_at(&many),
            None,
            10,
            100,
            std::time::Instant::now() + std::time::Duration::from_millis(1),
        );
        let unresolved: Vec<&Component> = r
            .components
            .iter()
            .filter(|c| c.custody == Custody::Unresolved)
            .collect();
        assert!(unresolved.len() <= 1, "{}", unresolved.len());
        // A root's time spent: its later folders are not read either.
        let other = dir.path().join("Other");
        std::fs::create_dir_all(&other).unwrap();
        std::fs::write(other.join("c.txt"), b"x").unwrap();
        let reports = classify_all_within(
            vec![folder_at(&app), folder_at(&other)],
            System::Windows,
            &Catalogue::shipped(crate::catalogue::SHIPPED),
            std::time::Duration::ZERO,
        );
        for r in reports {
            assert!(
                r.not_moved.iter().all(|n| n.why == Skip::NotRead),
                "{:?}",
                r.not_moved
            );
            assert!(r.components.iter().all(|c| c.members.is_empty()));
        }
    }

    #[test]
    fn folders_are_taken_newest_first_and_none_past_the_deadline_is_read() {
        let dir = tempfile::tempdir().unwrap();
        let old = dir.path().join("Old");
        std::fs::create_dir_all(&old).unwrap();
        std::fs::write(old.join("a.txt"), b"x").unwrap();
        std::thread::sleep(std::time::Duration::from_millis(100));
        let new = dir.path().join("New");
        std::fs::create_dir_all(&new).unwrap();
        std::fs::write(new.join("b.txt"), b"x").unwrap();
        let cat = Catalogue::shipped(crate::catalogue::SHIPPED);
        let r = classify_newest_first(
            vec![folder_at(&old), folder_at(&new)],
            System::Windows,
            &cat,
            far(),
        );
        assert_eq!(r[0].folder.rel, "New");
        assert_eq!(r[1].folder.rel, "Old");
        assert!(r.iter().all(|f| !f.components.is_empty()));
        let late = classify_newest_first(
            vec![folder_at(&old), folder_at(&new)],
            System::Windows,
            &cat,
            std::time::Instant::now(),
        );
        for f in late {
            assert!(f.components.is_empty());
            assert_eq!(
                f.not_moved,
                vec![NotMoved {
                    rel: String::new(),
                    why: Skip::NotRead
                }]
            );
        }
    }

    #[test]
    fn hash_names_are_recognised() {
        let h = "A".repeat(64);
        assert_eq!(hash_name(&h), Some("a".repeat(64)));
        assert_eq!(hash_name(&format!("sha256-{h}")), Some("a".repeat(64)));
        assert_eq!(hash_name("sha256-abc"), None);
        assert_eq!(hash_name(&"g".repeat(64)), None);
    }
}
