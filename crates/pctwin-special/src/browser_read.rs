//! What a browser's own files hold, read only in PCTwin's parser process (Phase 2 adoption,
//! change 1): every function here takes an [`InChild`]. The app hands over a bounded copy of the
//! bytes ([`crate::content::ContentReader::browser`]); nothing here opens a file.
//!
//! Readers per **format**, never per browser (ADOPTION change 7):
//! - a bookmarks tree in JSON: the Chromium bookmarks file (`roots`, nodes of `"type": "url"` or
//!   `"folder"`) or a Mozilla bookmarks backup (`text/x-moz-place` and `-container` nodes);
//! - a Mozilla places database (`moz_bookmarks` over `moz_places`), through the same read-only,
//!   in-memory SQLite reading as every other database ([`crate::content::db_snapshot`]);
//! - a property list of bookmarks (`WebBookmarkTypeList` and `WebBookmarkTypeLeaf` nodes);
//! - the names of add-ons: a Mozilla add-ons list (`addons`, user-installed extensions only) or
//!   a WebExtension manifest's `name` (with its `_locales` message when the name is one);
//! - the open pages of a Mozilla session file (`mozLz40\0`, a 4-byte size, an LZ4 block of JSON).
//!
//! What comes back is only what the person is shown: folder names, page titles and addresses,
//! dates and add-on names, bounded ([`MAX_NODES`], [`MAX_DEPTH`], [`MAX_TEXT`]); anything past a
//! bound is counted in `left_out`, never silently dropped. Cookies, passwords, form data and
//! anything else in these files never leave this process.

use std::collections::HashMap;

use pctwin_parse_seal::InChild;
use serde::{Deserialize, Serialize};
use serde_json::Value;

use crate::content::ContentError;

/// The most bookmarks, folders, pages or add-ons one reading returns.
pub const MAX_NODES: usize = 50_000;
/// The deepest folder kept.
pub const MAX_DEPTH: usize = 32;
/// The most text (titles, addresses, names) one reading returns, in bytes: the answer always
/// fits the parser's reply.
pub const MAX_TEXT: usize = 1_500_000;
/// The longest title or name kept (longer ones are cut, at a character).
pub const MAX_TITLE: usize = 1_000;
/// The longest address kept (a longer one is left out and counted, never cut).
pub const MAX_URL: usize = 4_096;
/// The largest session file, once unpacked.
pub const MAX_SESSION: usize = 64 * 1024 * 1024;
/// The largest property list read.
pub const MAX_PLIST: usize = 16 * 1024 * 1024;

/// What to read a file as.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum BrowserRead {
    /// JSON: a bookmarks tree, an add-ons list or a WebExtension manifest (with its messages as
    /// the second file, when it has one).
    Json,
    /// A Mozilla places database (with its write-ahead log as the second file).
    Places,
    /// A property list of bookmarks.
    PropertyList,
    /// A Mozilla session file.
    Session,
}

/// One bookmark or folder.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case", tag = "node", deny_unknown_fields)]
pub enum Node {
    Folder {
        title: String,
        /// Seconds since 1 January 1970, when known.
        added: Option<i64>,
        children: Vec<Node>,
    },
    Link {
        title: String,
        url: String,
        added: Option<i64>,
    },
}

/// A bookmarks tree.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Tree {
    pub roots: Vec<Node>,
    /// Bookmarks and folders past a bound (count, depth, text or an address too long).
    pub left_out: u32,
}

impl Tree {
    /// How many links it holds.
    pub fn links(&self) -> usize {
        fn count(n: &[Node]) -> usize {
            n.iter()
                .map(|n| match n {
                    Node::Folder { children, .. } => count(children),
                    Node::Link { .. } => 1,
                })
                .sum()
        }
        count(&self.roots)
    }
}

/// One open page: its title and address.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Page {
    pub title: String,
    pub url: String,
}

/// What a file held.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case", tag = "found", deny_unknown_fields)]
pub enum Found {
    Bookmarks {
        tree: Tree,
    },
    AddOns {
        names: Vec<String>,
        left_out: u32,
    },
    /// A WebExtension manifest: its name, or (when its name is a message) the locale whose
    /// messages say it, to be read next.
    AddOn {
        name: Option<String>,
        locale: Option<String>,
    },
    Pages {
        pages: Vec<Page>,
        left_out: u32,
        /// The file is a session (it has windows): it keeps sign-ins, cookies and form data too,
        /// so it never moves as data; only its pages are listed.
        session: bool,
    },
    /// None of the shapes above.
    Nothing,
}

/// The running bounds of one reading.
struct Budget {
    nodes: usize,
    text: usize,
    left_out: u32,
}

impl Budget {
    fn new() -> Self {
        Self {
            nodes: 0,
            text: 0,
            left_out: 0,
        }
    }

    /// Takes one entry of `text` bytes, or counts it left out.
    fn take(&mut self, text: usize) -> bool {
        if self.nodes >= MAX_NODES || self.text.saturating_add(text) > MAX_TEXT {
            self.left_out = self.left_out.saturating_add(1);
            return false;
        }
        self.nodes += 1;
        self.text += text;
        true
    }

    fn skip(&mut self) {
        self.left_out = self.left_out.saturating_add(1);
    }
}

/// A title or name as shown: no control characters, trimmed, at most [`MAX_TITLE`] bytes.
pub fn clean(s: &str) -> String {
    let t: String = s.chars().filter(|c| !c.is_control()).collect();
    let t = t.trim();
    if t.len() <= MAX_TITLE {
        return t.to_string();
    }
    let mut end = MAX_TITLE;
    while !t.is_char_boundary(end) {
        end -= 1;
    }
    t[..end].to_string()
}

/// An address as kept: trimmed, with no control characters or spaces inside, at most
/// [`MAX_URL`] bytes; otherwise none.
fn address(s: &str) -> Option<String> {
    let t = s.trim();
    (!t.is_empty() && t.len() <= MAX_URL && !t.chars().any(|c| c.is_control() || c == ' '))
        .then(|| t.to_string())
}

/// Microseconds since 1 January 1601 (Chromium's clock) as seconds since 1970.
fn from_1601_micros(v: i64) -> Option<i64> {
    (v > 0)
        .then(|| v / 1_000_000 - 11_644_473_600)
        .filter(|s| *s > 0)
}

/// Microseconds since 1970 (Mozilla's clock) as seconds.
fn from_unix_micros(v: i64) -> Option<i64> {
    (v > 0).then_some(v / 1_000_000).filter(|s| *s > 0)
}

fn json_of(bytes: &[u8]) -> Result<Value, ContentError> {
    if bytes.len() > crate::content::MAX_JSON {
        return Err(ContentError::TooLarge);
    }
    let bytes = bytes.strip_prefix(b"\xef\xbb\xbf").unwrap_or(bytes);
    pctwin_types::nesting::check_bytes(bytes, pctwin_types::nesting::Syntax::Json)
        .map_err(|e| ContentError::Damaged(e.to_string()))?;
    serde_json::from_slice(bytes).map_err(|e| ContentError::Damaged(e.to_string()))
}

fn text<'a>(o: &'a serde_json::Map<String, Value>, k: &str) -> Option<&'a str> {
    o.get(k).and_then(Value::as_str)
}

/// A number written as a number or as a decimal string (Chromium writes its dates as strings).
fn number(o: &serde_json::Map<String, Value>, k: &str) -> Option<i64> {
    match o.get(k)? {
        Value::Number(n) => n.as_i64(),
        Value::String(s) => s.trim().parse().ok(),
        _ => None,
    }
}

/// A Chromium bookmarks file's node.
fn chromium_node(v: &Value, depth: usize, b: &mut Budget) -> Option<Node> {
    let o = v.as_object()?;
    let title = clean(text(o, "name").unwrap_or_default());
    let added = number(o, "date_added").and_then(from_1601_micros);
    match text(o, "type")? {
        "url" => {
            let Some(url) = text(o, "url").and_then(address) else {
                b.skip();
                return None;
            };
            b.take(title.len() + url.len())
                .then_some(Node::Link { title, url, added })
        }
        "folder" => {
            if depth >= MAX_DEPTH || !b.take(title.len()) {
                if depth >= MAX_DEPTH {
                    b.skip();
                }
                return None;
            }
            let children = o
                .get("children")
                .and_then(Value::as_array)
                .map(|a| {
                    a.iter()
                        .filter_map(|c| chromium_node(c, depth + 1, b))
                        .collect()
                })
                .unwrap_or_default();
            Some(Node::Folder {
                title,
                added,
                children,
            })
        }
        _ => None,
    }
}

/// A Mozilla bookmarks backup's node.
fn moz_node(v: &Value, depth: usize, b: &mut Budget) -> Option<Node> {
    let o = v.as_object()?;
    let title = clean(text(o, "title").unwrap_or_default());
    let added = number(o, "dateAdded").and_then(from_unix_micros);
    match text(o, "type")? {
        "text/x-moz-place" => {
            let Some(url) = text(o, "uri").and_then(address) else {
                b.skip();
                return None;
            };
            b.take(title.len() + url.len())
                .then_some(Node::Link { title, url, added })
        }
        "text/x-moz-place-container" => {
            if depth >= MAX_DEPTH || !b.take(title.len()) {
                if depth >= MAX_DEPTH {
                    b.skip();
                }
                return None;
            }
            let children = o
                .get("children")
                .and_then(Value::as_array)
                .map(|a| a.iter().filter_map(|c| moz_node(c, depth + 1, b)).collect())
                .unwrap_or_default();
            Some(Node::Folder {
                title,
                added,
                children,
            })
        }
        _ => None,
    }
}

/// A Mozilla add-ons list's user-installed extensions, by name.
fn add_on_list(addons: &[Value]) -> Found {
    let mut b = Budget::new();
    let mut names = Vec::new();
    for a in addons {
        let Some(o) = a.as_object() else { continue };
        if text(o, "type") != Some("extension") || text(o, "location") != Some("app-profile") {
            continue;
        }
        let name = o
            .get("defaultLocale")
            .and_then(Value::as_object)
            .and_then(|l| text(l, "name"))
            .map(clean)
            .filter(|n| !n.is_empty());
        if let Some(n) = name
            && !names.contains(&n)
            && b.take(n.len())
        {
            names.push(n);
        }
    }
    names.sort();
    Found::AddOns {
        names,
        left_out: b.left_out,
    }
}

/// The key of a WebExtension message reference (`__MSG_key__`).
fn message_key(name: &str) -> Option<&str> {
    name.strip_prefix("__MSG_")?
        .strip_suffix("__")
        .filter(|k| !k.is_empty())
}

/// A locale name as a plain folder name.
fn plain_locale(l: &str) -> bool {
    (1..=16).contains(&l.len()) && l.bytes().all(|b| b.is_ascii_alphanumeric() || b == b'_')
}

fn manifest(o: &serde_json::Map<String, Value>, messages: Option<&[u8]>) -> Found {
    let Some(raw) = text(o, "name") else {
        return Found::AddOn {
            name: None,
            locale: None,
        };
    };
    let Some(key) = message_key(raw.trim()) else {
        let n = clean(raw);
        return Found::AddOn {
            name: (!n.is_empty()).then_some(n),
            locale: None,
        };
    };
    if let Some(m) = messages {
        // Message keys are compared ignoring case, as the format says.
        let said = json_of(m).ok().and_then(|v| {
            v.as_object()?
                .iter()
                .find(|(k, _)| k.eq_ignore_ascii_case(key))
                .and_then(|(_, x)| x.as_object())
                .and_then(|x| text(x, "message"))
                .map(clean)
        });
        return Found::AddOn {
            name: said.filter(|n| !n.is_empty()),
            locale: None,
        };
    }
    Found::AddOn {
        name: None,
        locale: text(o, "default_locale")
            .filter(|l| plain_locale(l))
            .map(str::to_string),
    }
}

/// A JSON file: a bookmarks tree, an add-ons list, a WebExtension manifest, or none of them.
fn json(bytes: &[u8], second: Option<&[u8]>) -> Result<Found, ContentError> {
    let v = json_of(bytes)?;
    let Some(o) = v.as_object() else {
        return Ok(Found::Nothing);
    };
    if let Some(roots) = o.get("roots").and_then(Value::as_object) {
        let mut b = Budget::new();
        let roots: Vec<Node> = roots
            .values()
            .filter(|r| r.get("type").and_then(Value::as_str) == Some("folder"))
            .filter_map(|r| chromium_node(r, 0, &mut b))
            .collect();
        return Ok(Found::Bookmarks {
            tree: Tree {
                roots,
                left_out: b.left_out,
            },
        });
    }
    if text(o, "type") == Some("text/x-moz-place-container") {
        let mut b = Budget::new();
        let roots = match moz_node(&v, 0, &mut b) {
            Some(Node::Folder { children, .. }) => children,
            _ => Vec::new(),
        };
        return Ok(Found::Bookmarks {
            tree: Tree {
                roots,
                left_out: b.left_out,
            },
        });
    }
    if let Some(addons) = o.get("addons").and_then(Value::as_array) {
        return Ok(add_on_list(addons));
    }
    if o.get("manifest_version").is_some_and(Value::is_u64) {
        return Ok(manifest(o, second));
    }
    Ok(Found::Nothing)
}

/// A Mozilla places database's bookmarks.
fn places(seal: InChild, db: &[u8], log: Option<&[u8]>) -> Result<Found, ContentError> {
    let bad = |e: rusqlite::Error| ContentError::Damaged(e.to_string());
    let image = crate::content::db_snapshot(seal, db, log)?;
    let mut c = rusqlite::Connection::open_in_memory().map_err(bad)?;
    if image.is_empty() {
        return Ok(Found::Nothing);
    }
    c.deserialize_read_exact(rusqlite::MAIN_DB, image.as_slice(), image.len(), true)
        .map_err(bad)?;
    let shaped: i64 = c
        .query_row(
            "SELECT count(*) FROM sqlite_schema WHERE type = 'table' AND name IN ('moz_bookmarks', 'moz_places')",
            [],
            |r| r.get(0),
        )
        .map_err(bad)?;
    if shaped != 2 {
        return Ok(Found::Nothing);
    }
    struct Row {
        id: i64,
        kind: i64,
        title: String,
        added: Option<i64>,
        url: Option<String>,
        guid: String,
    }
    let mut by_parent: HashMap<i64, Vec<Row>> = HashMap::new();
    let mut root = None;
    {
        let mut st = c
            .prepare(&format!(
                "SELECT b.id, b.type, b.parent, coalesce(b.title, ''), b.dateAdded, p.url, coalesce(b.guid, '') \
                 FROM moz_bookmarks b LEFT JOIN moz_places p ON b.fk = p.id \
                 ORDER BY b.parent, b.position LIMIT {}",
                MAX_NODES * 2
            ))
            .map_err(bad)?;
        let mut rows = st.query([]).map_err(bad)?;
        while let Some(r) = rows.next().map_err(bad)? {
            let parent: i64 = r.get::<_, Option<i64>>(2).map_err(bad)?.unwrap_or(0);
            let row = Row {
                id: r.get(0).map_err(bad)?,
                kind: r.get::<_, Option<i64>>(1).map_err(bad)?.unwrap_or(0),
                title: r
                    .get::<_, Option<String>>(3)
                    .map_err(bad)?
                    .unwrap_or_default(),
                added: r
                    .get::<_, Option<i64>>(4)
                    .map_err(bad)?
                    .and_then(from_unix_micros),
                url: r.get::<_, Option<String>>(5).ok().flatten(),
                guid: r
                    .get::<_, Option<String>>(6)
                    .map_err(bad)?
                    .unwrap_or_default(),
            };
            if parent == 0 && root.is_none() {
                root = Some(row.id);
            }
            by_parent.entry(parent).or_default().push(row);
        }
    }
    fn build(parent: i64, depth: usize, by: &HashMap<i64, Vec<Row>>, b: &mut Budget) -> Vec<Node> {
        let mut out = Vec::new();
        for r in by.get(&parent).into_iter().flatten() {
            // The places format keeps tags as a folder of their own: they are not bookmarks.
            if r.guid == "tags________" || r.id == parent {
                continue;
            }
            let title = clean(&r.title);
            match r.kind {
                1 => {
                    let Some(url) = r.url.as_deref().and_then(address) else {
                        b.skip();
                        continue;
                    };
                    if b.take(title.len() + url.len()) {
                        out.push(Node::Link {
                            title,
                            url,
                            added: r.added,
                        });
                    }
                }
                2 => {
                    if depth >= MAX_DEPTH {
                        b.skip();
                        continue;
                    }
                    if b.take(title.len()) {
                        let children = build(r.id, depth + 1, by, b);
                        out.push(Node::Folder {
                            title,
                            added: r.added,
                            children,
                        });
                    }
                }
                _ => {}
            }
        }
        out
    }
    let mut b = Budget::new();
    let roots = root.map_or_else(Vec::new, |r| build(r, 0, &by_parent, &mut b));
    Ok(Found::Bookmarks {
        tree: Tree {
            roots,
            left_out: b.left_out,
        },
    })
}

/// A property list of bookmarks.
fn property_list(bytes: &[u8]) -> Result<Found, ContentError> {
    if bytes.len() > MAX_PLIST {
        return Err(ContentError::TooLarge);
    }
    if !bytes.starts_with(b"bplist00") {
        pctwin_types::nesting::check_bytes(bytes, pctwin_types::nesting::Syntax::Xml)
            .map_err(|e| ContentError::Damaged(e.to_string()))?;
    }
    let v = plist::Value::from_reader(std::io::Cursor::new(bytes))
        .map_err(|e| ContentError::Damaged(e.to_string()))?;
    fn node(v: &plist::Value, depth: usize, b: &mut Budget) -> Option<Node> {
        let d = v.as_dictionary()?;
        let s = |k: &str| d.get(k).and_then(plist::Value::as_string);
        match s("WebBookmarkType")? {
            "WebBookmarkTypeLeaf" => {
                let title = clean(
                    d.get("URIDictionary")
                        .and_then(plist::Value::as_dictionary)
                        .and_then(|u| u.get("title"))
                        .and_then(plist::Value::as_string)
                        .unwrap_or_default(),
                );
                let Some(url) = s("URLString").and_then(address) else {
                    b.skip();
                    return None;
                };
                b.take(title.len() + url.len()).then_some(Node::Link {
                    title,
                    url,
                    added: None,
                })
            }
            "WebBookmarkTypeList" => {
                if depth >= MAX_DEPTH {
                    b.skip();
                    return None;
                }
                let title = clean(s("Title").unwrap_or_default());
                if !b.take(title.len()) {
                    return None;
                }
                let children = d
                    .get("Children")
                    .and_then(plist::Value::as_array)
                    .map(|a| a.iter().filter_map(|c| node(c, depth + 1, b)).collect())
                    .unwrap_or_default();
                Some(Node::Folder {
                    title,
                    added: None,
                    children,
                })
            }
            _ => None,
        }
    }
    let mut b = Budget::new();
    let roots = match node(&v, 0, &mut b) {
        Some(Node::Folder { children, .. }) => children,
        _ => return Ok(Found::Nothing),
    };
    Ok(Found::Bookmarks {
        tree: Tree {
            roots,
            left_out: b.left_out,
        },
    })
}

/// The magic a Mozilla LZ4 file starts with.
pub const MOZ_LZ4: &[u8; 8] = b"mozLz40\0";

/// The open pages of a Mozilla session file: in each open window, each tab's current page.
fn session(bytes: &[u8]) -> Result<Found, ContentError> {
    let rest = bytes
        .strip_prefix(MOZ_LZ4)
        .ok_or_else(|| ContentError::Damaged("not a session file".into()))?;
    let (size, block) = rest
        .split_first_chunk::<4>()
        .ok_or_else(|| ContentError::Damaged("its size is missing".into()))?;
    let size = u32::from_le_bytes(*size) as usize;
    if size > MAX_SESSION {
        return Err(ContentError::TooLarge);
    }
    let plain = lz4_flex::block::decompress(block, size)
        .map_err(|e| ContentError::Damaged(e.to_string()))?;
    if plain.len() > crate::content::MAX_JSON * 16 {
        return Err(ContentError::TooLarge);
    }
    pctwin_types::nesting::check_bytes(&plain, pctwin_types::nesting::Syntax::Json)
        .map_err(|e| ContentError::Damaged(e.to_string()))?;
    let v: Value =
        serde_json::from_slice(&plain).map_err(|e| ContentError::Damaged(e.to_string()))?;
    let mut b = Budget::new();
    let mut pages = Vec::new();
    let windows = v.get("windows").and_then(Value::as_array);
    let is_session = windows.is_some();
    for w in windows.into_iter().flatten() {
        let tabs = w.get("tabs").and_then(Value::as_array);
        for t in tabs.into_iter().flatten() {
            let Some(entries) = t.get("entries").and_then(Value::as_array) else {
                continue;
            };
            // `index` counts from 1; without one, the last page is the current one.
            let at = t
                .get("index")
                .and_then(Value::as_u64)
                .and_then(|i| usize::try_from(i).ok())
                .and_then(|i| i.checked_sub(1))
                .unwrap_or(entries.len().saturating_sub(1));
            let Some(e) = entries.get(at).and_then(Value::as_object) else {
                continue;
            };
            let Some(url) = text(e, "url").and_then(address) else {
                b.skip();
                continue;
            };
            let title = clean(text(e, "title").unwrap_or_default());
            if b.take(title.len() + url.len()) {
                pages.push(Page { title, url });
            }
        }
    }
    Ok(Found::Pages {
        pages,
        left_out: b.left_out,
        session: is_session,
    })
}

/// Reads `bytes` (and `second`: a database's log, or a manifest's messages) as `what`, in the
/// parser process (it takes an [`InChild`]).
pub fn read(
    seal: InChild,
    what: BrowserRead,
    bytes: &[u8],
    second: Option<&[u8]>,
) -> Result<Found, ContentError> {
    match what {
        BrowserRead::Json => json(bytes, second),
        BrowserRead::Places => places(seal, bytes, second),
        BrowserRead::PropertyList => property_list(bytes),
        BrowserRead::Session => session(bytes),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn seal() -> InChild {
        pctwin_parse_seal::in_child()
    }

    fn tree(f: Found) -> Tree {
        match f {
            Found::Bookmarks { tree } => tree,
            other => panic!("not bookmarks: {other:?}"),
        }
    }

    #[test]
    fn a_chromium_tree_keeps_folders_titles_addresses_and_dates() {
        let t = tree(
            read(
                seal(),
                BrowserRead::Json,
                br#"{"checksum":"0","version":1,"sync_transaction_version":"3","roots":{
                "bookmark_bar":{"type":"folder","name":"Bar","date_added":"13300000000000000","children":[
                    {"type":"url","name":"Ex\u0007ample","url":"https://example.org/","date_added":"13300000000000000"},
                    {"type":"folder","name":"Inner","children":[{"type":"url","name":"Two","url":"http://two.example/"}]},
                    {"type":"url","name":"No address"}]},
                "other":{"type":"folder","name":"Other","children":[]}}}"#,
                None,
            )
            .unwrap(),
        );
        assert_eq!(t.links(), 2);
        assert_eq!(t.left_out, 1, "a link with no address is counted");
        let Node::Folder {
            title,
            children,
            added,
        } = &t.roots[0]
        else {
            panic!()
        };
        assert_eq!(title, "Bar");
        assert_eq!(*added, Some(13_300_000_000 - 11_644_473_600));
        assert_eq!(
            children[0],
            Node::Link {
                title: "Example".into(),
                url: "https://example.org/".into(),
                added: Some(1_655_526_400),
            }
        );
    }

    #[test]
    fn a_mozilla_backup_tree_is_read_by_its_node_types() {
        let t = tree(
            read(
                seal(),
                BrowserRead::Json,
                br#"{"type":"text/x-moz-place-container","title":"","children":[
                {"type":"text/x-moz-place-container","title":"menu","dateAdded":1600000000000000,"children":[
                    {"type":"text/x-moz-place","title":"A","uri":"https://a.example/","dateAdded":1600000000000000},
                    {"type":"text/x-moz-place-separator"}]}]}"#,
                None,
            )
            .unwrap(),
        );
        assert_eq!(t.links(), 1);
        assert!(
            matches!(&t.roots[0], Node::Folder { title, added: Some(1_600_000_000), .. } if title == "menu")
        );
    }

    #[test]
    fn add_on_names_come_from_the_list_or_the_manifest() {
        let f = read(
            seal(),
            BrowserRead::Json,
            br#"{"schemaVersion":36,"addons":[
                {"type":"extension","location":"app-profile","defaultLocale":{"name":"Blocker"}},
                {"type":"extension","location":"app-builtin","defaultLocale":{"name":"Built in"}},
                {"type":"theme","location":"app-profile","defaultLocale":{"name":"Dark"}},
                {"type":"extension","location":"app-profile","defaultLocale":{"name":"Blocker"}},
                {"type":"extension","location":"app-profile"}]}"#,
            None,
        )
        .unwrap();
        assert_eq!(
            f,
            Found::AddOns {
                names: vec!["Blocker".into()],
                left_out: 0
            }
        );
        let plain = br#"{"manifest_version":3,"name":"Notes Helper","version":"1"}"#;
        assert_eq!(
            read(seal(), BrowserRead::Json, plain, None).unwrap(),
            Found::AddOn {
                name: Some("Notes Helper".into()),
                locale: None
            }
        );
        let msg = br#"{"manifest_version":2,"name":"__MSG_appName__","default_locale":"en_GB"}"#;
        assert_eq!(
            read(seal(), BrowserRead::Json, msg, None).unwrap(),
            Found::AddOn {
                name: None,
                locale: Some("en_GB".into())
            }
        );
        assert_eq!(
            read(
                seal(),
                BrowserRead::Json,
                msg,
                Some(br#"{"APPNAME":{"message":"Reader View"}}"#)
            )
            .unwrap(),
            Found::AddOn {
                name: Some("Reader View".into()),
                locale: None
            }
        );
        // A locale that is not a plain folder name is never asked for.
        let bad = br#"{"manifest_version":2,"name":"__MSG_a__","default_locale":"../x"}"#;
        assert_eq!(
            read(seal(), BrowserRead::Json, bad, None).unwrap(),
            Found::AddOn {
                name: None,
                locale: None
            }
        );
        assert_eq!(
            read(seal(), BrowserRead::Json, b"[1,2]", None).unwrap(),
            Found::Nothing
        );
    }

    fn moz_lz4(json: &[u8]) -> Vec<u8> {
        let mut out = MOZ_LZ4.to_vec();
        out.extend_from_slice(&(json.len() as u32).to_le_bytes());
        out.extend_from_slice(&lz4_flex::block::compress(json));
        out
    }

    #[test]
    fn a_sessions_open_pages_are_each_tabs_current_page_and_nothing_else() {
        let s = moz_lz4(
            br#"{"windows":[{"tabs":[
                {"entries":[{"url":"https://old.example/","title":"Old"},{"url":"https://now.example/","title":"Now"}],"index":2},
                {"entries":[{"url":"https://first.example/","title":"First"},{"url":"https://later.example/"}],"index":1},
                {"entries":[{"url":"https://last.example/","title":"Last"}]}]}],
              "_closedWindows":[{"tabs":[{"entries":[{"url":"https://closed.example/"}],"index":1}]}],
              "cookies":[{"host":"example.org","name":"sid","value":"secret-cookie-value"}]}"#,
        );
        let f = read(seal(), BrowserRead::Session, &s, None).unwrap();
        let Found::Pages {
            pages,
            left_out,
            session,
        } = f
        else {
            panic!()
        };
        assert_eq!(left_out, 0);
        assert!(session);
        // A compressed file of another kind (a bookmarks backup) is no session.
        assert!(matches!(
            read(
                seal(),
                BrowserRead::Session,
                &moz_lz4(br#"{"children":[]}"#),
                None
            )
            .unwrap(),
            Found::Pages { session: false, .. }
        ));
        let urls: Vec<&str> = pages.iter().map(|p| p.url.as_str()).collect();
        assert_eq!(
            urls,
            [
                "https://now.example/",
                "https://first.example/",
                "https://last.example/"
            ]
        );
        assert!(!format!("{pages:?}").contains("secret-cookie-value"));
        // Not a session file, a size past the bound, or a damaged block: refused.
        assert!(read(seal(), BrowserRead::Session, b"{}", None).is_err());
        let mut big = MOZ_LZ4.to_vec();
        big.extend_from_slice(&u32::MAX.to_le_bytes());
        assert_eq!(
            read(seal(), BrowserRead::Session, &big, None).unwrap_err(),
            ContentError::TooLarge
        );
        let mut cut = s.clone();
        cut.truncate(s.len() - 5);
        assert!(read(seal(), BrowserRead::Session, &cut, None).is_err());
    }

    #[test]
    fn a_property_list_of_bookmarks_is_read_by_its_node_types() {
        use plist::{Dictionary, Value as P};
        let leaf = |t: &str, u: &str| {
            let mut uri = Dictionary::new();
            uri.insert("title".into(), P::String(t.into()));
            let mut d = Dictionary::new();
            d.insert(
                "WebBookmarkType".into(),
                P::String("WebBookmarkTypeLeaf".into()),
            );
            d.insert("URLString".into(), P::String(u.into()));
            d.insert("URIDictionary".into(), P::Dictionary(uri));
            P::Dictionary(d)
        };
        let list = |t: &str, c: Vec<P>| {
            let mut d = Dictionary::new();
            d.insert(
                "WebBookmarkType".into(),
                P::String("WebBookmarkTypeList".into()),
            );
            d.insert("Title".into(), P::String(t.into()));
            d.insert("Children".into(), P::Array(c));
            P::Dictionary(d)
        };
        let mut proxy = Dictionary::new();
        proxy.insert(
            "WebBookmarkType".into(),
            P::String("WebBookmarkTypeProxy".into()),
        );
        let root = list(
            "",
            vec![
                list("BookmarksBar", vec![leaf("Ex", "https://example.org/")]),
                P::Dictionary(proxy),
            ],
        );
        let mut bin = Vec::new();
        root.to_writer_binary(&mut bin).unwrap();
        let t = tree(read(seal(), BrowserRead::PropertyList, &bin, None).unwrap());
        assert_eq!(t.links(), 1);
        assert_eq!(t.roots.len(), 1);
        let mut xml = Vec::new();
        root.to_writer_xml(&mut xml).unwrap();
        assert_eq!(
            tree(read(seal(), BrowserRead::PropertyList, &xml, None).unwrap()).links(),
            1
        );
        assert!(read(seal(), BrowserRead::PropertyList, b"bplist00junk", None).is_err());
    }

    #[test]
    fn a_places_database_gives_its_bookmark_tree_without_tags() {
        let dir = tempfile::tempdir().unwrap();
        let p = dir.path().join("p.sqlite");
        let c = rusqlite::Connection::open(&p).unwrap();
        c.execute_batch(
            "CREATE TABLE moz_places(id INTEGER PRIMARY KEY, url TEXT, title TEXT, visit_count INTEGER);
             CREATE TABLE moz_bookmarks(id INTEGER PRIMARY KEY, type INTEGER, fk INTEGER, parent INTEGER,
                position INTEGER, title TEXT, dateAdded INTEGER, guid TEXT);
             INSERT INTO moz_places VALUES (1,'https://a.example/','A',3),(2,'javascript:alert(1)','J',0),(3,'https://tagged.example/','T',0);
             INSERT INTO moz_bookmarks VALUES
               (1,2,NULL,0,0,'',0,'root________'),
               (2,2,NULL,1,0,'menu',1600000000000000,'menu________'),
               (4,2,NULL,1,1,'tags',0,'tags________'),
               (5,1,1,2,0,'A link',1600000000000000,'aaaaaaaaaaaa'),
               (6,1,2,2,1,'Script',0,'bbbbbbbbbbbb'),
               (7,3,NULL,2,2,NULL,0,'cccccccccccc'),
               (8,1,3,4,0,NULL,0,'dddddddddddd');",
        )
        .unwrap();
        drop(c);
        let bytes = std::fs::read(&p).unwrap();
        let t = tree(read(seal(), BrowserRead::Places, &bytes, None).unwrap());
        assert_eq!(t.roots.len(), 1, "tags are not a folder of bookmarks");
        let Node::Folder {
            title, children, ..
        } = &t.roots[0]
        else {
            panic!()
        };
        assert_eq!(title, "menu");
        // The script address is kept here: the writer decides what is safe, and counts it.
        assert_eq!(children.len(), 2);
        // A database of another shape holds no bookmarks.
        let q = dir.path().join("q.sqlite");
        rusqlite::Connection::open(&q)
            .unwrap()
            .execute_batch("CREATE TABLE urls(url TEXT, visit_count INTEGER);")
            .unwrap();
        assert_eq!(
            read(
                seal(),
                BrowserRead::Places,
                &std::fs::read(&q).unwrap(),
                None
            )
            .unwrap(),
            Found::Nothing
        );
    }

    #[test]
    fn every_reading_is_bounded() {
        // Deep folders past the bound are counted, never followed.
        let mut j = String::from(r#"{"roots":{"b":"#);
        for _ in 0..(MAX_DEPTH + 5) {
            j.push_str(r#"{"type":"folder","name":"f","children":["#);
        }
        j.push_str(r#"{"type":"url","name":"x","url":"https://x.example/"}"#);
        for _ in 0..(MAX_DEPTH + 5) {
            j.push_str("]}");
        }
        j.push_str("}}");
        let t = tree(read(seal(), BrowserRead::Json, j.as_bytes(), None).unwrap());
        assert_eq!(t.links(), 0);
        assert!(t.left_out >= 1);
        // Too many bookmarks: the rest are counted.
        let many: Vec<String> = (0..MAX_NODES + 10)
            .map(|i| format!(r#"{{"type":"url","name":"n","url":"https://e.example/{i}"}}"#))
            .collect();
        let j = format!(
            r#"{{"roots":{{"b":{{"type":"folder","name":"b","children":[{}]}}}}}}"#,
            many.join(",")
        );
        let t = tree(read(seal(), BrowserRead::Json, j.as_bytes(), None).unwrap());
        assert_eq!(t.links(), MAX_NODES - 1);
        assert_eq!(t.left_out, 11);
        // A long address is left out whole, never cut.
        let long = format!(
            r#"{{"roots":{{"b":{{"type":"folder","name":"b","children":[{{"type":"url","name":"l","url":"https://e.example/{}"}}]}}}}}}"#,
            "a".repeat(MAX_URL)
        );
        let t = tree(read(seal(), BrowserRead::Json, long.as_bytes(), None).unwrap());
        assert_eq!((t.links(), t.left_out), (0, 1));
        assert_eq!(clean(&"é".repeat(MAX_TITLE)).len(), MAX_TITLE);
    }
}
