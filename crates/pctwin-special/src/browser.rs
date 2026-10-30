//! Browsers, between different browsers (Task List 2.5; the 2.5 plan, section 7; acceptance rows
//! T25-021 to T25-026).
//!
//! The same browser on the new laptop takes its own profile as portable app data ([`crate::classify`]:
//! bookmarks and history move, cookies, sign-ins and saved passwords are held back). For a
//! **different** browser, there is one standard every browser imports: the bookmarks file
//! (`NETSCAPE-Bookmark-file-1`). So, for each browser profile found:
//!
//! - its bookmarks are read in the parser process ([`crate::browser_read`]: a JSON tree, a places
//!   database or a property list, by their formats) and written here, from the typed tree, as
//!   that standard file ([`netscape_html`]): every title and address escaped, only `http`,
//!   `https` and `ftp` addresses kept, the others (`javascript:`, `data:`, `file:` ...) counted
//!   and left out. It lands in PCTwin's own folder of things brought over and ends `converted`;
//!   the person imports it with the new browser's own "import bookmarks from an HTML file"
//!   ([`Note::BookmarksFile`]). A browser opened is never taken as proof it imported anything;
//! - history has no standard format between browsers: the person is told to use the new
//!   browser's own import while both are on one laptop, or the browser's sync
//!   ([`Note::HistoryOwnImport`]);
//! - its add-ons' names and its open pages (titles and addresses) are listed for the person in a
//!   plain page beside it ([`lists_html`]: no script, nothing loads by itself), so the browser's
//!   own sync, or the person, can bring them back. Old pages are never opened (T25-026). Session
//!   files in a format PCTwin does not list are counted and said ([`Note::OpenPagesBySync`]).
//!
//! These lists are the person's own: they are written only into the landed page, never into a
//! report, a log, a catalogue or anything sent to AI; only their counts are said.
//!
//! Which files are which is told by their bytes (formats), never by a browser's or a file's name;
//! which browser a folder is and its name come from what was found and the catalogue rows.

use std::collections::{BTreeMap, BTreeSet};
use std::path::{Path, PathBuf};

use pctwin_grant::{CaptureEffect, CaptureGrant, Now};

use crate::browser_read::{BrowserRead, Found, MOZ_LZ4, Node, Page, Tree};
use crate::capture::{Approved, Capture, CaptureError, CapturedFile, Method, Stop, generation};
use crate::category::{Category, Note};
use crate::classify::{FolderReport, Skip};
use crate::component::{Component, Custody, Evidence, Preservation};

/// The address kinds kept in a bookmarks file: pages on the web, nothing that runs or reads the
/// laptop.
pub const SAFE_SCHEMES: &[&str] = &["http", "https", "ftp"];

/// The most files looked at in one profile folder for its lists.
const MAX_LOOKED: usize = 2_000;
/// The most session files read in one profile folder.
const MAX_SESSIONS: usize = 8;
/// The largest file read for a bookmarks tree, a list or a session.
const MAX_READ: u64 = crate::content::MAX_JSON as u64;
/// What the landed files are called (in the profile's own folder there).
pub const BOOKMARKS_FILE: &str = "Bookmarks.html";
pub const LISTS_FILE: &str = "Open pages and add-ons.html";

/// The words of the files PCTwin writes for the person, in their language: the screens send
/// them ([`set_words`]); until then, and for any word that is not safe, the English ones.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Words {
    /// The bookmarks file's name (ends `.html`).
    pub bookmarks_file: String,
    /// The list page's name (ends `.html`).
    pub lists_file: String,
    /// The bookmarks file's title.
    pub bookmarks_title: String,
    /// The list page's headings.
    pub add_ons: String,
    pub open_pages: String,
}

impl Default for Words {
    fn default() -> Self {
        Self {
            bookmarks_file: BOOKMARKS_FILE.into(),
            lists_file: LISTS_FILE.into(),
            bookmarks_title: "Bookmarks".into(),
            add_ons: "Add-ons".into(),
            open_pages: "Open pages".into(),
        }
    }
}

static WORDS: std::sync::RwLock<Option<Words>> = std::sync::RwLock::new(None);

/// A file name the person's words may give: one plain name ending `.html`, nothing a system
/// treats specially (no folder parts, no reserved marks, no control characters, no leading or
/// trailing dot or space), at most 100 characters.
pub fn file_name_ok(n: &str) -> bool {
    let stem = n.strip_suffix(".html").unwrap_or("");
    !stem.trim().is_empty()
        && n.chars().count() <= 100
        && !n.starts_with(['.', ' '])
        && !stem.ends_with(['.', ' '])
        && !n.chars().any(|c| {
            c.is_control() || matches!(c, '/' | '\\' | ':' | '*' | '?' | '"' | '<' | '>' | '|')
        })
}

fn text_ok(t: &str) -> bool {
    !t.trim().is_empty() && t.chars().count() <= 100 && !t.chars().any(char::is_control)
}

/// Sets the words of the files written from now on. False (and the words stay as they were) when
/// any is not safe.
pub fn set_words(w: Words) -> bool {
    let ok = file_name_ok(&w.bookmarks_file)
        && file_name_ok(&w.lists_file)
        && !w.bookmarks_file.eq_ignore_ascii_case(&w.lists_file)
        && [&w.bookmarks_title, &w.add_ons, &w.open_pages]
            .iter()
            .all(|t| text_ok(t));
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

/// Whether `url` is kept in a bookmarks file or a list: its scheme is one of [`SAFE_SCHEMES`].
pub fn safe_address(url: &str) -> bool {
    url.split_once(':').is_some_and(|(scheme, rest)| {
        !rest.is_empty() && SAFE_SCHEMES.iter().any(|s| scheme.eq_ignore_ascii_case(s))
    })
}

/// Text written into HTML: `&`, `<`, `>`, `"` and `'` escaped.
pub fn escape(s: &str) -> String {
    let mut out = String::with_capacity(s.len());
    for c in s.chars() {
        match c {
            '&' => out.push_str("&amp;"),
            '<' => out.push_str("&lt;"),
            '>' => out.push_str("&gt;"),
            '"' => out.push_str("&quot;"),
            '\'' => out.push_str("&#39;"),
            c if c.is_control() => {}
            c => out.push(c),
        }
    }
    out
}

/// A written file and what went into it.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Written {
    pub html: String,
    /// Bookmarks (or pages, or add-ons) written.
    pub kept: usize,
    /// Addresses left out because they are not pages on the web.
    pub dropped: usize,
}

/// The standard bookmarks file (`NETSCAPE-Bookmark-file-1`, which every major browser imports)
/// for `tree`, under the heading `name`.
pub fn netscape_html(tree: &Tree, name: &str) -> Written {
    netscape_html_in(tree, name, &words())
}

/// [`netscape_html`] with the words `w`.
fn netscape_html_in(tree: &Tree, name: &str, w: &Words) -> Written {
    fn nodes(n: &[Node], depth: usize, out: &mut String, kept: &mut usize, dropped: &mut usize) {
        let pad = "    ".repeat(depth + 1);
        for n in n {
            match n {
                Node::Folder {
                    title,
                    added,
                    children,
                } => {
                    out.push_str(&format!(
                        "{pad}<DT><H3{}>{}</H3>\n",
                        date(*added),
                        escape(title)
                    ));
                    out.push_str(&format!("{pad}<DL><p>\n"));
                    nodes(children, depth + 1, out, kept, dropped);
                    out.push_str(&format!("{pad}</DL><p>\n"));
                }
                Node::Link { title, url, added } => {
                    if !safe_address(url) {
                        *dropped += 1;
                        continue;
                    }
                    *kept += 1;
                    out.push_str(&format!(
                        "{pad}<DT><A HREF=\"{}\"{}>{}</A>\n",
                        escape(url),
                        date(*added),
                        escape(title)
                    ));
                }
            }
        }
    }
    fn date(d: Option<i64>) -> String {
        d.filter(|d| *d > 0)
            .map(|d| format!(" ADD_DATE=\"{d}\""))
            .unwrap_or_default()
    }
    let mut html = String::from(
        "<!DOCTYPE NETSCAPE-Bookmark-file-1>\n\
         <META HTTP-EQUIV=\"Content-Type\" CONTENT=\"text/html; charset=UTF-8\">\n\
         <TITLE>",
    );
    html.push_str(&escape(&w.bookmarks_title));
    html.push_str("</TITLE>\n");
    html.push_str(&format!("<H1>{}</H1>\n<DL><p>\n", escape(name)));
    let (mut kept, mut dropped) = (0, 0);
    nodes(&tree.roots, 0, &mut html, &mut kept, &mut dropped);
    html.push_str("</DL><p>\n");
    Written {
        html,
        kept,
        dropped,
    }
}

/// A plain page listing a browser's add-ons (names) and open pages (titles and addresses), for the
/// person: no script and nothing that loads by itself (the page forbids it); an address is a link
/// only when it is a page on the web, and nothing is ever opened for the person.
pub fn lists_html(name: &str, add_ons: &[String], pages: &[Page]) -> Written {
    lists_html_in(name, add_ons, pages, &words())
}

/// [`lists_html`] with the words `w`.
fn lists_html_in(name: &str, add_ons: &[String], pages: &[Page], w: &Words) -> Written {
    let mut html = format!(
        "<!DOCTYPE html>\n<html><head><meta charset=\"utf-8\">\n\
         <meta http-equiv=\"Content-Security-Policy\" content=\"default-src 'none'\">\n\
         <meta name=\"referrer\" content=\"no-referrer\">\n<title>{0}</title></head>\n<body>\n<h1>{0}</h1>\n",
        escape(name)
    );
    let (mut kept, mut dropped) = (0, 0);
    if !add_ons.is_empty() {
        html.push_str(&format!("<h2>{}</h2>\n<ul>\n", escape(&w.add_ons)));
        for a in add_ons {
            kept += 1;
            html.push_str(&format!("<li>{}</li>\n", escape(a)));
        }
        html.push_str("</ul>\n");
    }
    if !pages.is_empty() {
        html.push_str(&format!("<h2>{}</h2>\n<ul>\n", escape(&w.open_pages)));
        for p in pages {
            if !safe_address(&p.url) {
                dropped += 1;
                continue;
            }
            kept += 1;
            let title = if p.title.is_empty() { &p.url } else { &p.title };
            html.push_str(&format!(
                "<li><a href=\"{}\" rel=\"noopener noreferrer\">{}</a></li>\n",
                escape(&p.url),
                escape(title)
            ));
        }
        html.push_str("</ul>\n");
    }
    html.push_str("</body></html>\n");
    Written {
        html,
        kept,
        dropped,
    }
}

/// What a file found in a profile is, by its first bytes and shape (never its name).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Kind {
    /// A bookmarks source: a JSON tree, a places database or a property list.
    Bookmarks(BrowserRead),
    /// A list of add-ons (a JSON file shaped like one).
    AddOnList,
    /// A Mozilla session file.
    Session,
    /// A session file in a format PCTwin does not list (the Chromium session format).
    OtherSession,
}

fn joined(base: &Path, rel: &str) -> PathBuf {
    rel.split('/')
        .filter(|p| !p.is_empty())
        .fold(base.to_path_buf(), |p, part| p.join(part))
}

fn parent(rel: &str) -> &str {
    rel.rsplit_once('/').map_or("", |(p, _)| p)
}

fn modified(p: &Path) -> Option<std::time::SystemTime> {
    std::fs::symlink_metadata(p).ok()?.modified().ok()
}

fn contains(hay: &[u8], needle: &[u8]) -> bool {
    hay.windows(needle.len()).any(|w| w == needle)
}

fn head(p: &Path) -> Option<Vec<u8>> {
    use std::io::Read;
    let mut f = crate::guard::open(p).ok()?;
    let mut b = vec![0u8; 16];
    let n = f.read(&mut b).ok()?;
    b.truncate(n);
    Some(b)
}

/// The JSON-looking start of a file (after a byte-order mark and spaces).
fn json_start(h: &[u8]) -> bool {
    let h = h.strip_prefix(b"\xef\xbb\xbf").unwrap_or(h);
    h.iter()
        .find(|b| !b.is_ascii_whitespace())
        .is_some_and(|b| *b == b'{')
}

/// What `rel` (inside `base`) is, given whether its component holds saved web addresses.
fn kind_of(base: &Path, rel: &str, browsing: bool) -> Option<Kind> {
    let p = joined(base, rel);
    let meta = std::fs::symlink_metadata(&p).ok()?;
    if crate::guard::kind_of(&meta) != crate::guard::Kind::File {
        return None;
    }
    let h = head(&p)?;
    if h.starts_with(MOZ_LZ4) {
        return Some(Kind::Session);
    }
    if h.starts_with(b"SNSS") {
        return Some(Kind::OtherSession);
    }
    if h.starts_with(b"SQLite format 3\0") {
        if !browsing {
            return None;
        }
        let tables = crate::content::sqlite_tables(&p).ok()?;
        let has = |n: &str| tables.iter().any(|t| t.name == n);
        return (has("moz_bookmarks") && has("moz_places"))
            .then_some(Kind::Bookmarks(BrowserRead::Places));
    }
    if meta.len() > MAX_READ {
        return None;
    }
    if h.starts_with(b"bplist00") || h.starts_with(b"<?xml") {
        let bytes = crate::guard::read_all(&p, MAX_READ).ok()?;
        return contains(&bytes, b"WebBookmarkType")
            .then_some(Kind::Bookmarks(BrowserRead::PropertyList));
    }
    if json_start(&h) {
        let bytes = crate::guard::read_all(&p, MAX_READ).ok()?;
        // A bookmarks tree by its shape, read in the parser process; a file grouped with saved
        // addresses, or one that only shares a word with the format, is not one.
        if browsing && crate::content::json_facts_of(&bytes).is_ok_and(|f| f.bookmarks) {
            return Some(Kind::Bookmarks(BrowserRead::Json));
        }
        if contains(&bytes, b"\"addons\"") && contains(&bytes, b"\"defaultLocale\"") {
            return Some(Kind::AddOnList);
        }
    }
    None
}

/// What one profile folder holds for another browser.
#[derive(Debug, Default)]
struct Profile {
    bookmarks: Option<(String, Option<std::time::SystemTime>)>,
    lists: BTreeSet<String>,
    sessions: Vec<(String, Option<std::time::SystemTime>)>,
    other_sessions: usize,
}

/// Every plain file in `dir` and the folders right inside it (no links followed), at most
/// [`MAX_LOOKED`].
fn files_near(base: &Path, dir: &str) -> Vec<String> {
    let mut out = Vec::new();
    let mut folders = vec![(dir.to_string(), 0)];
    while let Some((d, depth)) = folders.pop() {
        let Ok(entries) = std::fs::read_dir(joined(base, &d)) else {
            continue;
        };
        let mut list: Vec<_> = entries.flatten().collect();
        list.sort_by_key(|e| e.file_name());
        for e in list {
            if out.len() >= MAX_LOOKED {
                return out;
            }
            let Ok(name) = e.file_name().into_string() else {
                continue;
            };
            let rel = if d.is_empty() {
                name
            } else {
                format!("{d}/{name}")
            };
            let Ok(meta) = std::fs::symlink_metadata(e.path()) else {
                continue;
            };
            match crate::guard::kind_of(&meta) {
                crate::guard::Kind::File => out.push(rel),
                crate::guard::Kind::Folder if depth == 0 => folders.push((rel, 1)),
                _ => {}
            }
        }
    }
    out
}

/// The component that writes a browser folder's bookmarks for another browser and lists its
/// add-ons and open pages, with what the person is told; `None` when the folder is not a
/// browser's or holds none of them.
pub fn planned(report: &FolderReport) -> Option<(Component, Vec<Note>)> {
    if crate::category::of_folder(report) != Category::Browser {
        return None;
    }
    let base = report.folder.path();
    let first = report.components.first()?;
    // Profiles: the folders holding saved web addresses.
    let browsing: BTreeSet<&str> = report
        .components
        .iter()
        .filter(|c| !c.secret && c.evidence.contains(&Evidence::BrowsingData))
        .flat_map(|c| c.members.iter().map(String::as_str))
        .collect();
    let secret: BTreeSet<&str> = report
        .components
        .iter()
        .filter(|c| c.secret)
        .flat_map(|c| c.members.iter().map(String::as_str))
        .collect();
    let dirs: BTreeSet<&str> = browsing.iter().map(|m| parent(m)).collect();
    let mut profiles: BTreeMap<String, Profile> = BTreeMap::new();
    for dir in dirs {
        let mut p = Profile::default();
        for rel in files_near(&base, dir) {
            // A secret's bytes are never read for a list; a session file (which also keeps
            // sign-ins) is read only in the parser process, for its pages' titles and addresses.
            if secret.contains(rel.as_str())
                && !head(&joined(&base, &rel)).is_some_and(|h| h.starts_with(MOZ_LZ4))
            {
                continue;
            }
            let near = parent(&rel) == dir;
            let in_browsing = near && browsing.contains(rel.as_str());
            match kind_of(&base, &rel, in_browsing) {
                Some(Kind::Bookmarks(_)) if near => {
                    // The newest bookmarks source of the profile (a browser keeps an older copy
                    // beside it).
                    let at = modified(&joined(&base, &rel));
                    if p.bookmarks.as_ref().is_none_or(|(_, t)| at > *t) {
                        p.bookmarks = Some((rel, at));
                    }
                }
                Some(Kind::AddOnList) if near => {
                    p.lists.insert(rel);
                }
                Some(Kind::Session) => {
                    let at = modified(&joined(&base, &rel));
                    p.sessions.push((rel, at));
                }
                Some(Kind::OtherSession) => p.other_sessions += 1,
                _ => {}
            }
        }
        // Add-ons found in the profile (folders with a WebExtension manifest): their manifests.
        for n in &report.not_moved {
            let inside = dir.is_empty() || n.rel.starts_with(&format!("{dir}/"));
            if n.why == Skip::AddOn && inside {
                p.lists.insert(format!("{}/manifest.json", n.rel));
            }
        }
        p.sessions.sort_by_key(|a| std::cmp::Reverse(a.1));
        p.sessions.truncate(MAX_SESSIONS);
        profiles.insert(dir.to_string(), p);
    }
    let mut members: Vec<String> = Vec::new();
    let mut notes = Vec::new();
    let (mut any_bookmarks, mut any_lists, mut other_sessions) = (false, false, 0);
    let mut kept_profiles = Vec::new();
    for (dir, p) in &profiles {
        let before = members.len();
        if let Some((rel, _)) = &p.bookmarks {
            any_bookmarks = true;
            members.push(rel.clone());
        }
        if !p.lists.is_empty() || !p.sessions.is_empty() {
            any_lists = true;
        }
        members.extend(p.lists.iter().cloned());
        members.extend(p.sessions.iter().map(|(r, _)| r.clone()));
        other_sessions += p.other_sessions;
        if members.len() > before {
            kept_profiles.push(dir.clone());
        }
    }
    if members.is_empty() {
        return None;
    }
    members.sort();
    members.dedup();
    if any_bookmarks {
        notes.push(Note::BookmarksFile);
    }
    notes.push(Note::HistoryOwnImport);
    if any_lists {
        notes.push(Note::PagesAndAddOnsListed);
    }
    if other_sessions > 0 {
        notes.push(Note::OpenPagesBySync);
    }
    let c = Component::new(
        first.owner.clone(),
        &format!("{}#for-another-browser", first.place),
        members,
        Custody::Portable,
        false,
        vec![Evidence::ForAnotherBrowser {
            profiles: kept_profiles,
        }],
    );
    Some((c, notes))
}

/// Whether the file at `path` is a session file in the Mozilla LZ4 format (by its bytes, read in
/// the parser process): `Some(true)` a session, which keeps sign-ins and so never moves as data;
/// `Some(false)` another file in that format; `None` when it could not be read.
pub fn session_file(path: &Path) -> Option<bool> {
    let bytes = crate::guard::read_all(path, MAX_READ).ok()?;
    match crate::content::browser_of(BrowserRead::Session, &bytes, None) {
        Ok(Found::Pages { session, .. }) => Some(session),
        _ => None,
    }
}

/// For a file in the Mozilla LZ4 format (by its first bytes): whether it is a session
/// ([`session_file`]); `None` for any other file.
pub fn moz_session(path: &Path) -> Option<Option<bool>> {
    head(path)
        .is_some_and(|h| h.starts_with(MOZ_LZ4))
        .then(|| session_file(path))
}

/// Whether `c` is the component that writes bookmarks for another browser.
pub fn is_conversion(c: &Component) -> bool {
    c.evidence
        .iter()
        .any(|e| matches!(e, Evidence::ForAnotherBrowser { .. }))
}

fn profiles_of(c: &Component) -> Vec<String> {
    c.evidence
        .iter()
        .find_map(|e| match e {
            Evidence::ForAnotherBrowser { profiles } => Some(profiles.clone()),
            _ => None,
        })
        .unwrap_or_default()
}

/// The profile `rel` belongs to: the longest one it is inside.
fn profile_of<'a>(profiles: &'a [String], rel: &str) -> Option<&'a str> {
    profiles
        .iter()
        .filter(|p| p.is_empty() || rel.starts_with(&format!("{p}/")))
        .max_by_key(|p| p.len())
        .map(String::as_str)
}

/// What was read for one profile.
#[derive(Default)]
struct Read {
    tree: Option<Tree>,
    add_ons: BTreeSet<String>,
    pages: Vec<Page>,
    left_out: u32,
}

fn read_one(path: &Path, what: BrowserRead, second: Option<&[u8]>) -> Result<Found, String> {
    let bytes = crate::guard::read_all(
        path,
        if what == BrowserRead::Places {
            crate::content::MAX_DB
        } else {
            MAX_READ
        },
    )
    .map_err(|e| e.to_string())?;
    let log = if what == BrowserRead::Places {
        let wal = crate::content::wal_of(path);
        if std::fs::symlink_metadata(&wal).is_ok_and(|m| m.is_file() && m.len() > 0) {
            Some(crate::guard::read_all(&wal, crate::content::MAX_LOG).map_err(|e| e.to_string())?)
        } else {
            None
        }
    } else {
        None
    };
    crate::content::browser_of(what, &bytes, log.as_deref().or(second)).map_err(|e| e.to_string())
}

/// Writes the files for another browser of the planned component `c` (its folder on the old
/// laptop is `base`; the browser's name `name`) into the private, empty PCTwin folder `work`,
/// with the ordinary 2.5 grant. The old laptop's files are only read; their insides only in the
/// parser process.
#[allow(clippy::too_many_arguments)]
pub fn capture(
    grant: CaptureGrant,
    approved: &Approved,
    c: &Component,
    base: &Path,
    name: &str,
    work: &Path,
    holders: &dyn crate::capture::Holders,
    stop: &Stop,
    now: Now,
) -> Result<Capture, CaptureError> {
    if grant.effect() != CaptureEffect::ReadPrivate {
        return Err(CaptureError::Denied("it allows another kind of work"));
    }
    if grant.seal() != approved.seal {
        return Err(CaptureError::Denied("it belongs to another plan"));
    }
    if !grant.roots().contains(&approved.root) {
        return Err(CaptureError::Denied("it does not cover this folder"));
    }
    if grant.expiry().is_over(now) {
        return Err(CaptureError::Denied("it has ended"));
    }
    if c.secret || !is_conversion(c) || approved.component != c.id {
        return Err(CaptureError::Denied("it is not this step"));
    }
    let gen_now = generation(base, &c.members);
    if gen_now.digest != approved.generation.digest {
        return Err(CaptureError::ChangedSinceApproval);
    }
    // While the browser writes these files, they are read once it is closed, as every other
    // part of its data is.
    let paths: Vec<PathBuf> = c.members.iter().map(|m| joined(base, m)).collect();
    if let crate::capture::Holding::Held { by } = holders.holding(&paths) {
        return Err(CaptureError::AwaitingClose { by });
    }
    let profiles = profiles_of(c);
    let mut read: BTreeMap<String, Read> = BTreeMap::new();
    let mut failed: Vec<String> = Vec::new();
    // Newest first: of a profile's session files, the newest with open pages says them.
    let mut members: Vec<(&str, Option<std::time::SystemTime>)> = c
        .members
        .iter()
        .map(|m| (m.as_str(), modified(&joined(base, m))))
        .collect();
    members.sort_by_key(|a| std::cmp::Reverse(a.1));
    for (rel, _) in members {
        if stop.stopped() {
            return Err(CaptureError::Stopped);
        }
        let Some(profile) = profile_of(&profiles, rel) else {
            continue;
        };
        let path = joined(base, rel);
        let Some(h) = head(&path) else {
            failed.push(rel.to_string());
            continue;
        };
        let r = read.entry(profile.to_string()).or_default();
        let what = if h.starts_with(MOZ_LZ4) {
            if !r.pages.is_empty() {
                continue;
            }
            BrowserRead::Session
        } else if h.starts_with(b"SQLite format 3\0") {
            BrowserRead::Places
        } else if h.starts_with(b"bplist00") || h.starts_with(b"<?xml") {
            BrowserRead::PropertyList
        } else {
            BrowserRead::Json
        };
        match read_one(&path, what, None) {
            Ok(Found::Bookmarks { tree }) => {
                if r.tree.is_none() {
                    r.tree = Some(tree);
                }
            }
            Ok(Found::AddOns { names, left_out }) => {
                r.add_ons.extend(names);
                r.left_out = r.left_out.saturating_add(left_out);
            }
            Ok(Found::AddOn { name: Some(n), .. }) => {
                r.add_ons.insert(n);
            }
            Ok(Found::AddOn {
                name: None,
                locale: Some(l),
            }) => {
                // The name is a message: read it from the add-on's own messages for its locale.
                let messages = joined(base, parent(rel))
                    .join("_locales")
                    .join(&l)
                    .join("messages.json");
                let second = crate::guard::read_all(&messages, MAX_READ).ok();
                if let (Some(m), Ok(bytes)) = (second, crate::guard::read_all(&path, MAX_READ))
                    && let Ok(Found::AddOn { name: Some(n), .. }) =
                        crate::content::browser_of(BrowserRead::Json, &bytes, Some(&m))
                {
                    r.add_ons.insert(n);
                }
            }
            Ok(Found::Pages {
                pages, left_out, ..
            }) => {
                r.pages = pages;
                r.left_out = r.left_out.saturating_add(left_out);
            }
            Ok(_) => {}
            Err(_) => failed.push(rel.to_string()),
        }
    }
    // Nothing read may have changed under the reading.
    if generation(base, &c.members).digest != gen_now.digest {
        return Err(CaptureError::ChangedSinceApproval);
    }
    let mut files = Vec::new();
    let (mut bookmarks, mut add_ons, mut pages, mut left_out) = (0u32, 0u32, 0u32, 0u32);
    let mut put = |rel: String, html: &str| -> Result<(), CaptureError> {
        let to = joined(work, &rel);
        if let Some(dir) = to.parent() {
            std::fs::create_dir_all(dir).map_err(|e| CaptureError::Unreadable(e.to_string()))?;
        }
        std::fs::write(&to, html.as_bytes())
            .map_err(|e| CaptureError::Unreadable(e.to_string()))?;
        files.push(CapturedFile {
            rel,
            len: html.len() as u64,
            hash: *blake3::hash(html.as_bytes()).as_bytes(),
        });
        Ok(())
    };
    let at = |dir: &str, file: &str| {
        if dir.is_empty() {
            file.to_string()
        } else {
            format!("{dir}/{file}")
        }
    };
    let words = words();
    for (dir, r) in &read {
        left_out = left_out.saturating_add(r.left_out);
        if let Some(t) = &r.tree {
            left_out = left_out.saturating_add(t.left_out);
            let w = netscape_html(t, name);
            if w.kept > 0 {
                bookmarks = bookmarks.saturating_add(w.kept as u32);
                left_out = left_out.saturating_add(w.dropped as u32);
                put(at(dir, &words.bookmarks_file), &w.html)?;
            }
        }
        let names: Vec<String> = r.add_ons.iter().cloned().collect();
        if !names.is_empty() || !r.pages.is_empty() {
            let w = lists_html(name, &names, &r.pages);
            if w.kept > 0 {
                add_ons = add_ons.saturating_add(names.len() as u32);
                pages = pages.saturating_add((w.kept - names.len()) as u32);
                left_out = left_out.saturating_add(w.dropped as u32);
                put(at(dir, &words.lists_file), &w.html)?;
            }
        }
    }
    if files.is_empty() {
        return Err(CaptureError::Unreadable(
            "no bookmarks, add-ons or open pages could be read; use the browser's own export or sync"
                .into(),
        ));
    }
    let mut evidence = vec![Evidence::WrittenForAnotherBrowser {
        bookmarks,
        add_ons,
        pages,
        left_out,
    }];
    if !failed.is_empty() {
        evidence.push(Evidence::Unreadable {
            why: format!("{} of its files could not be read", failed.len()),
        });
    }
    Ok(Capture {
        component: c.id,
        method: Method::SteadyCopy,
        preservation: Preservation::Captured,
        generation: gen_now,
        files,
        folded: Vec::new(),
        evidence,
        // Nothing was copied from the browser's live files: they were read and written anew.
        closed_unconfirmed: true,
    })
}

/// The result of the files written for another browser, from how they ended: `converted` only
/// when they were written from what was read, landed, and every byte read back as written. The
/// browser's own import is the person's step and is never taken as done.
pub fn result_of(o: &crate::component::Outcome) -> pctwin_types::ItemResult {
    use crate::component::Transport;
    use pctwin_types::{ItemResult, ItemStatus, Reason};
    if o.preservation == Preservation::NotSelected {
        return ItemResult::skipped_by_user();
    }
    match (o.preservation, o.transport) {
        (Preservation::Captured, Transport::Verified) => ItemResult::done(ItemStatus::Converted)
            .unwrap_or_else(|_| ItemResult::because(Reason::FinishByHand)),
        (Preservation::Unavailable, _) => ItemResult::because(Reason::Unreadable),
        (_, Transport::Interrupted) => ItemResult::because(Reason::NotVerified),
        _ => ItemResult::because(Reason::FinishByHand),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn link(t: &str, u: &str) -> Node {
        Node::Link {
            title: t.into(),
            url: u.into(),
            added: Some(1_600_000_000),
        }
    }

    #[test]
    fn only_pages_on_the_web_are_kept_and_the_rest_are_counted() {
        for ok in [
            "https://a.example/",
            "HTTP://a.example/",
            "ftp://f.example/x",
        ] {
            assert!(safe_address(ok), "{ok}");
        }
        for bad in [
            "javascript:alert(1)",
            "JavaScript:alert(1)",
            "data:text/html,<b>x</b>",
            "file:///C:/Users/x/secret.txt",
            "vbscript:x",
            "chrome://settings",
            "about:blank",
            "https:",
            "no scheme",
            "",
        ] {
            assert!(!safe_address(bad), "{bad}");
        }
        let tree = Tree {
            roots: vec![Node::Folder {
                title: "Bar <&>".into(),
                added: None,
                children: vec![
                    link(
                        "A \"quoted\" <b>title</b> & 'more'",
                        "https://a.example/?q=1&r=\"2\"",
                    ),
                    link("Run", "javascript:alert(document.cookie)"),
                    link("Data", "data:text/html,<script>x</script>"),
                    link("Disk", "file:///etc/passwd"),
                ],
            }],
            left_out: 0,
        };
        let w = netscape_html(&tree, "Some <Browser>");
        assert_eq!((w.kept, w.dropped), (1, 3));
        assert!(w.html.starts_with("<!DOCTYPE NETSCAPE-Bookmark-file-1>"));
        assert!(w.html.contains("<H1>Some &lt;Browser&gt;</H1>"));
        assert!(w.html.contains("<DT><H3>Bar &lt;&amp;&gt;</H3>"));
        assert!(w.html.contains(
            "<DT><A HREF=\"https://a.example/?q=1&amp;r=&quot;2&quot;\" ADD_DATE=\"1600000000\">A &quot;quoted&quot; &lt;b&gt;title&lt;/b&gt; &amp; &#39;more&#39;</A>"
        ));
        for gone in ["javascript", "data:", "file:", "<script", "cookie"] {
            assert!(!w.html.contains(gone), "{gone}");
        }
        // Every opened list is closed.
        assert_eq!(
            w.html.matches("<DL>").count(),
            w.html.matches("</DL>").count()
        );
    }

    #[test]
    fn converted_only_when_written_landed_and_read_back() {
        use crate::component::{Access, Import, Outcome, Readiness, Reversal, Transport};
        use pctwin_types::{ItemStatus, Reason};
        let o = |preservation, transport| Outcome {
            preservation,
            transport,
            access: Access::NotNeeded,
            import: Import::PreservedOnly,
            readiness: Readiness::Untested,
            reversal: Reversal::LocallyReversible,
        };
        let r = result_of(&o(Preservation::Captured, Transport::Verified));
        assert_eq!(r.status(), ItemStatus::Converted);
        for (p, t, why) in [
            (
                Preservation::Captured,
                Transport::Interrupted,
                Reason::NotVerified,
            ),
            (
                Preservation::Captured,
                Transport::NotYet,
                Reason::FinishByHand,
            ),
            (
                Preservation::Pending,
                Transport::NotYet,
                Reason::FinishByHand,
            ),
            (
                Preservation::Unavailable,
                Transport::NotYet,
                Reason::Unreadable,
            ),
            (
                Preservation::Partial,
                Transport::Verified,
                Reason::FinishByHand,
            ),
        ] {
            let r = result_of(&o(p, t));
            assert_ne!(r.status(), ItemStatus::Converted, "{p:?} {t:?}");
            assert_eq!(r.reason(), Some(why), "{p:?} {t:?}");
        }
        assert_eq!(
            result_of(&o(Preservation::NotSelected, Transport::NotYet)).status(),
            ItemStatus::SkippedByUser
        );
    }

    #[test]
    fn the_lists_page_runs_nothing_and_links_only_web_pages() {
        let w = lists_html(
            "B",
            &["Ad <blocker>".into()],
            &[
                Page {
                    title: "Mail \"inbox\"".into(),
                    url: "https://mail.example/#inbox".into(),
                },
                Page {
                    title: String::new(),
                    url: "https://untitled.example/".into(),
                },
                Page {
                    title: "Evil".into(),
                    url: "javascript:fetch('x')".into(),
                },
            ],
        );
        assert_eq!((w.kept, w.dropped), (3, 1));
        assert!(w.html.contains("default-src 'none'"));
        assert!(w.html.contains("<li>Ad &lt;blocker&gt;</li>"));
        assert!(w.html.contains(
            "<a href=\"https://mail.example/#inbox\" rel=\"noopener noreferrer\">Mail &quot;inbox&quot;</a>"
        ));
        assert!(w.html.contains(">https://untitled.example/</a>"));
        assert!(!w.html.contains("javascript") && !w.html.contains("<script"));
        assert!(!w.html.to_lowercase().contains("onload"));
    }
}

#[cfg(test)]
mod words_tests {
    use super::*;

    #[test]
    fn only_safe_words_are_taken_and_english_stays_until_then() {
        let fr = Words {
            bookmarks_file: "Favoris.html".into(),
            lists_file: "Pages ouvertes et extensions.html".into(),
            bookmarks_title: "Favoris".into(),
            add_ons: "Extensions".into(),
            open_pages: "Pages ouvertes".into(),
        };
        for bad in [
            Words {
                bookmarks_file: "../x.html".into(),
                ..fr.clone()
            },
            Words {
                bookmarks_file: "a\\b.html".into(),
                ..fr.clone()
            },
            Words {
                bookmarks_file: "Favoris.txt".into(),
                ..fr.clone()
            },
            Words {
                bookmarks_file: ".html".into(),
                ..fr.clone()
            },
            Words {
                bookmarks_file: " .html".into(),
                ..fr.clone()
            },
            Words {
                bookmarks_file: ".x.html".into(),
                ..fr.clone()
            },
            Words {
                bookmarks_file: "x..html".into(),
                ..fr.clone()
            },
            Words {
                bookmarks_file: "a:b.html".into(),
                ..fr.clone()
            },
            Words {
                bookmarks_file: "a\u{7}b.html".into(),
                ..fr.clone()
            },
            Words {
                bookmarks_file: format!("{}.html", "x".repeat(96)),
                ..fr.clone()
            },
            Words {
                lists_file: "favoris.HTML".into(),
                bookmarks_file: "Favoris.HTML".into(),
                ..fr.clone()
            },
            Words {
                lists_file: "FAVORIS.html".into(),
                ..fr.clone()
            },
            Words {
                add_ons: "".into(),
                ..fr.clone()
            },
            Words {
                open_pages: "a\nb".into(),
                ..fr.clone()
            },
            Words {
                bookmarks_title: "x".repeat(101),
                ..fr.clone()
            },
        ] {
            assert!(!set_words(bad.clone()), "{bad:?}");
        }
        assert!(file_name_ok(&format!("{}.html", "x".repeat(95))));
        assert!(file_name_ok("书签.html"));
        // Taken whole when every word is safe (set here: no other test in this program sets them).
        assert!(set_words(fr.clone()));
        assert_eq!(words(), fr);
        assert!(set_words(Words::default()));
        let tree = Tree {
            roots: vec![],
            left_out: 0,
        };
        assert!(
            netscape_html_in(&tree, "B", &fr)
                .html
                .contains("<TITLE>Favoris</TITLE>")
        );
        let page = lists_html_in(
            "B",
            &["A".into()],
            &[Page {
                title: "T".into(),
                url: "https://example.org/".into(),
            }],
            &fr,
        );
        assert!(
            page.html.contains("<h2>Extensions</h2>")
                && page.html.contains("<h2>Pages ouvertes</h2>")
        );
    }
}
