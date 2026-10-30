//! The file formats game launchers and game save lists are written in, read only in PCTwin's
//! locked-down parser process (Phase 2 adoption, change 1): each reader takes an [`InChild`].
//! Code exists per format, never per launcher or game (adoption change 7):
//!
//! - [`record_tree`]: a launcher's install records, as one bounded tree of names and texts:
//!   Valve's text key-values ([`RecordFormat::Vdf`]: `libraryfolders.vdf`, `appmanifest_*.acf`,
//!   `remotecache.vdf`) and JSON ([`RecordFormat::Json`]: an Epic launcher `*.item`). Which
//!   fields mean what is said by the `game-launchers` catalogue rows, never here.
//! - [`save_manifest`]: the entries a save list in the Ludusavi manifest format holds for the
//!   games asked about ([`Wanted`]). Only the shape of that format is read here: block maps and
//!   sequences, plain and quoted scalars, `{}` and `[]`, as the manifest is written. Anything
//!   else (anchors, aliases, tags, flow collections, block scalars, tabs) makes that entry
//!   unreadable, counted, never guessed at. The person's own copy of the manifest is read this
//!   way; PCTwin never ships or downloads the manifest's data (see `games::fetch_manifest`).
//!
//! Every reader is bounded in bytes, depth, entries and text, and fails closed: an answer is
//! the facts read or why they could not be.

use std::collections::BTreeMap;
use std::io::{BufRead, Read};

use pctwin_parse_seal::InChild;
use serde::{Deserialize, Serialize};
use serde_json::Value;

use crate::content::ContentError;

/// The largest launcher record read.
pub const MAX_RECORD: u64 = 1024 * 1024;
/// The largest save list read (the whole Ludusavi manifest was about 18 MB on 30 October 2026).
pub const MAX_MANIFEST: u64 = 96 * 1024 * 1024;
/// The largest list of games asked about.
pub const MAX_WANTED: u64 = 256 * 1024;
/// The deepest a record's tree may go.
pub const MAX_DEPTH: usize = 16;
/// The most names and texts in one record.
const MAX_NODES: usize = 20_000;
/// The longest name or text kept.
pub const MAX_TEXT: usize = 4096;
/// The most bytes of names and texts in one record's answer.
const MAX_TREE_BYTES: usize = 512 * 1024;
/// The most entries handed back from a save list, and their bytes.
pub const MAX_ENTRIES: usize = 2_000;
const MAX_ENTRIES_BYTES: usize = 2 * 1024 * 1024;
/// The most entries one save list may hold.
const MAX_LIST_ENTRIES: u64 = 1_000_000;
/// The longest line, and the most lines, of one save list entry.
const MAX_LINE: usize = 8 * 1024;
const MAX_ENTRY_LINES: usize = 4_000;
/// The deepest one save list entry may go.
const MAX_ENTRY_DEPTH: usize = 12;

/// The format a launcher keeps its records in.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum RecordFormat {
    /// Valve's text key-values.
    Vdf,
    Json,
}

/// A record, as names and texts: a JSON array's items are named by their place (`0`, `1`, ..),
/// numbers and true/false are their text.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum Tree {
    Text(String),
    Map(Vec<(String, Tree)>),
}

impl Tree {
    /// The first child named `key`, ignoring ASCII case (Valve's key-values are read so).
    pub fn get(&self, key: &str) -> Option<&Tree> {
        match self {
            Tree::Map(m) => m
                .iter()
                .find(|(k, _)| k.eq_ignore_ascii_case(key))
                .map(|(_, v)| v),
            Tree::Text(_) => None,
        }
    }

    /// The tree at `path` (names, one level each).
    pub fn at(&self, path: &[String]) -> Option<&Tree> {
        path.iter().try_fold(self, |t, k| t.get(k))
    }

    pub fn text(&self) -> Option<&str> {
        match self {
            Tree::Text(t) => Some(t),
            Tree::Map(_) => None,
        }
    }

    /// Its children, in order (none for a text).
    pub fn children(&self) -> &[(String, Tree)] {
        match self {
            Tree::Map(m) => m,
            Tree::Text(_) => &[],
        }
    }
}

/// What a tree may still hold.
struct Budget {
    nodes: usize,
    bytes: usize,
}

impl Budget {
    fn new() -> Self {
        Self {
            nodes: MAX_NODES,
            bytes: MAX_TREE_BYTES,
        }
    }

    fn take(&mut self, text: &str) -> Result<(), ContentError> {
        if text.len() > MAX_TEXT {
            return Err(ContentError::Damaged(
                "a text longer than PCTwin reads".into(),
            ));
        }
        self.nodes = self.nodes.checked_sub(1).ok_or(ContentError::TooLarge)?;
        self.bytes = self
            .bytes
            .checked_sub(text.len() + 8)
            .ok_or(ContentError::TooLarge)?;
        Ok(())
    }
}

/// A launcher record's tree, read in the parser process.
pub fn record_tree(_: InChild, format: RecordFormat, bytes: &[u8]) -> Result<Tree, ContentError> {
    if bytes.len() as u64 > MAX_RECORD {
        return Err(ContentError::TooLarge);
    }
    let bytes = bytes.strip_prefix(b"\xef\xbb\xbf").unwrap_or(bytes);
    let text = std::str::from_utf8(bytes)
        .map_err(|_| ContentError::Damaged("not text in UTF-8".into()))?;
    let mut budget = Budget::new();
    match format {
        RecordFormat::Vdf => vdf(text, &mut budget),
        RecordFormat::Json => {
            pctwin_types::nesting::check_bytes(bytes, pctwin_types::nesting::Syntax::Json)
                .map_err(|e| ContentError::Damaged(e.to_string()))?;
            let v: Value =
                serde_json::from_str(text).map_err(|e| ContentError::Damaged(e.to_string()))?;
            json_tree(&v, 0, &mut budget)
        }
    }
}

fn json_tree(v: &Value, depth: usize, budget: &mut Budget) -> Result<Tree, ContentError> {
    if depth > MAX_DEPTH {
        return Err(ContentError::Damaged(
            "nested deeper than PCTwin reads".into(),
        ));
    }
    let text = |t: String, budget: &mut Budget| -> Result<Tree, ContentError> {
        budget.take(&t)?;
        Ok(Tree::Text(t))
    };
    match v {
        Value::Null => text(String::new(), budget),
        Value::Bool(b) => text(b.to_string(), budget),
        Value::Number(n) => text(n.to_string(), budget),
        Value::String(s) => text(s.clone(), budget),
        Value::Array(a) => {
            let mut out = Vec::with_capacity(a.len().min(1024));
            for (i, x) in a.iter().enumerate() {
                let k = i.to_string();
                budget.take(&k)?;
                out.push((k, json_tree(x, depth + 1, budget)?));
            }
            Ok(Tree::Map(out))
        }
        Value::Object(o) => {
            let mut out = Vec::with_capacity(o.len().min(1024));
            for (k, x) in o {
                budget.take(k)?;
                out.push((k.clone(), json_tree(x, depth + 1, budget)?));
            }
            Ok(Tree::Map(out))
        }
    }
}

/// One token of Valve's text key-values.
#[derive(Debug, PartialEq, Eq)]
enum Tok {
    Open,
    Close,
    Str(String),
}

struct Lexer<'a> {
    rest: &'a str,
}

impl Lexer<'_> {
    fn next(&mut self) -> Result<Option<Tok>, ContentError> {
        loop {
            self.rest = self.rest.trim_start();
            if let Some(after) = self.rest.strip_prefix("//") {
                self.rest = after.find('\n').map_or("", |n| &after[n..]);
                continue;
            }
            break;
        }
        let mut chars = self.rest.char_indices();
        let Some((_, c)) = chars.next() else {
            return Ok(None);
        };
        match c {
            '{' => {
                self.rest = &self.rest[1..];
                Ok(Some(Tok::Open))
            }
            '}' => {
                self.rest = &self.rest[1..];
                Ok(Some(Tok::Close))
            }
            '"' => {
                let mut out = String::new();
                let mut escaped = false;
                for (i, c) in chars {
                    if escaped {
                        out.push(match c {
                            'n' => '\n',
                            't' => '\t',
                            '\\' => '\\',
                            '"' => '"',
                            _ => {
                                return Err(ContentError::Damaged(
                                    "an escape PCTwin does not read".into(),
                                ));
                            }
                        });
                        escaped = false;
                    } else if c == '\\' {
                        escaped = true;
                    } else if c == '"' {
                        self.rest = &self.rest[i + 1..];
                        return Ok(Some(Tok::Str(out)));
                    } else {
                        out.push(c);
                    }
                    if out.len() > MAX_TEXT {
                        return Err(ContentError::Damaged(
                            "a text longer than PCTwin reads".into(),
                        ));
                    }
                }
                Err(ContentError::Damaged("a text with no end".into()))
            }
            // Conditions (`[$WIN32]`) and other marks are not read.
            '[' | ']' => Err(ContentError::Damaged(
                "a condition PCTwin does not read".into(),
            )),
            _ => {
                let end = self
                    .rest
                    .find(|c: char| c.is_whitespace() || matches!(c, '{' | '}' | '"'))
                    .unwrap_or(self.rest.len());
                let word = &self.rest[..end];
                self.rest = &self.rest[end..];
                Ok(Some(Tok::Str(word.to_string())))
            }
        }
    }
}

fn vdf(text: &str, budget: &mut Budget) -> Result<Tree, ContentError> {
    let mut lx = Lexer { rest: text };
    let tree = vdf_pairs(&mut lx, 0, budget)?;
    match lx.next()? {
        None => Ok(tree),
        Some(_) => Err(ContentError::Damaged(
            "a closing brace with no opening".into(),
        )),
    }
}

fn vdf_pairs(lx: &mut Lexer<'_>, depth: usize, budget: &mut Budget) -> Result<Tree, ContentError> {
    if depth > MAX_DEPTH {
        return Err(ContentError::Damaged(
            "nested deeper than PCTwin reads".into(),
        ));
    }
    let mut out = Vec::new();
    loop {
        let rest_before = lx.rest;
        let key = match lx.next()? {
            None if depth == 0 => return Ok(Tree::Map(out)),
            None => return Err(ContentError::Damaged("a block with no end".into())),
            Some(Tok::Close) if depth > 0 => return Ok(Tree::Map(out)),
            Some(Tok::Close) => {
                lx.rest = rest_before;
                return Ok(Tree::Map(out));
            }
            Some(Tok::Open) => return Err(ContentError::Damaged("a block with no name".into())),
            Some(Tok::Str(k)) => k,
        };
        budget.take(&key)?;
        let value = match lx.next()? {
            Some(Tok::Str(v)) => {
                budget.take(&v)?;
                Tree::Text(v)
            }
            Some(Tok::Open) => vdf_pairs(lx, depth + 1, budget)?,
            _ => return Err(ContentError::Damaged("a name with no value".into())),
        };
        out.push((key, value));
    }
}

/// The games a save list is read for: by name (ignoring case) and by store ID.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Wanted {
    pub names: Vec<String>,
    pub steam: Vec<u64>,
    pub gog: Vec<u64>,
}

impl Wanted {
    fn wants(&self, e: &SaveEntry) -> bool {
        self.names.iter().any(|n| n.eq_ignore_ascii_case(&e.name))
            || e.steam.as_ref().is_some_and(|s| self.steam.contains(&s.id))
            || e.id.steam_extra.iter().any(|i| self.steam.contains(i))
            || e.gog.as_ref().is_some_and(|s| self.gog.contains(&s.id))
            || e.id.gog_extra.iter().any(|i| self.gog.contains(i))
    }
}

/// When a path applies: on one system, from one store (both optional).
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct When {
    #[serde(default)]
    pub os: Option<String>,
    #[serde(default)]
    pub store: Option<String>,
}

/// A path's (or a registry key's) conditions and tags (`save`, `config`).
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct FileRule {
    #[serde(default)]
    pub when: Vec<When>,
    #[serde(default)]
    pub tags: Vec<String>,
}

/// A store's ID for a game.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct StoreId {
    pub id: u64,
}

/// Further store IDs for the same game.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct OtherIds {
    #[serde(default, rename = "steamExtra")]
    pub steam_extra: Vec<u64>,
    #[serde(default, rename = "gogExtra")]
    pub gog_extra: Vec<u64>,
}

/// An empty value (`{}`), as the format writes an install folder's name.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct Empty {}

/// One game's entry, in the Ludusavi manifest format: where its saves and settings are kept
/// (paths with placeholders, each with its conditions), its registry keys, its install folder's
/// names, which stores sync its saves online (`cloud`), and its store IDs. Fields the format
/// has and PCTwin does not use (launch options, notes) are not kept.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct SaveEntry {
    pub name: String,
    #[serde(default)]
    pub files: BTreeMap<String, FileRule>,
    #[serde(default)]
    pub registry: BTreeMap<String, FileRule>,
    #[serde(default, rename = "installDir")]
    pub install_dir: BTreeMap<String, Empty>,
    #[serde(default)]
    pub cloud: BTreeMap<String, bool>,
    #[serde(default)]
    pub steam: Option<StoreId>,
    #[serde(default)]
    pub gog: Option<StoreId>,
    #[serde(default)]
    pub id: OtherIds,
    /// Another entry's name: this one says nothing itself.
    #[serde(default)]
    pub alias: Option<String>,
}

/// What a save list holds for the games asked about.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ManifestRead {
    pub entries: Vec<SaveEntry>,
    /// Entries PCTwin could not read (outside the shape it reads): counted, never guessed at.
    pub unreadable: u64,
    /// More entries were wanted than are handed back at once.
    pub more: bool,
}

/// An entry being gathered: its name, then its lines (indent and text); `None` once it is too
/// long to read.
type Gathering = (String, Option<Vec<(usize, String)>>);

/// The entries for `wanted` in a save list of `size` bytes in the Ludusavi manifest format,
/// read line by line from `r`, in the parser process.
pub fn save_manifest(
    _: InChild,
    r: &mut impl BufRead,
    size: u64,
    wanted: &Wanted,
) -> Result<ManifestRead, ContentError> {
    if size > MAX_MANIFEST {
        return Err(ContentError::TooLarge);
    }
    let mut r = r.take(size);
    let mut out = ManifestRead::default();
    let mut out_bytes = 0usize;
    let mut seen = 0u64;
    let mut first = true;
    // The entry being gathered: its name line, then its lines; `None` once it is too long.
    let mut current: Option<Gathering> = None;
    let mut line = Vec::new();
    loop {
        line.clear();
        let n = (&mut r)
            .take(MAX_LINE as u64 + 1)
            .read_until(b'\n', &mut line)
            .map_err(|e| ContentError::Damaged(e.to_string()))?;
        let end = n == 0;
        if n > MAX_LINE {
            return Err(ContentError::Damaged(
                "a line longer than PCTwin reads".into(),
            ));
        }
        let text = if end {
            ""
        } else {
            let t = std::str::from_utf8(&line)
                .map_err(|_| ContentError::Damaged("not text in UTF-8".into()))?;
            let t = t.strip_suffix('\n').unwrap_or(t);
            let t = t.strip_suffix('\r').unwrap_or(t);
            if first {
                first = false;
                t.strip_prefix('\u{feff}').unwrap_or(t)
            } else {
                t
            }
        };
        let body = text.trim_start_matches(' ');
        let indent = text.len() - body.len();
        let blank = body.is_empty() || body.starts_with('#');
        if !end && blank {
            continue;
        }
        if !end && indent == 0 && body == "---" && current.is_none() && seen == 0 {
            continue;
        }
        if !end && body.starts_with('\t') {
            return Err(ContentError::Damaged("indented with tabs".into()));
        }
        if !end && indent > 0 {
            match &mut current {
                Some((_, Some(lines))) if lines.len() < MAX_ENTRY_LINES => {
                    lines.push((indent, body.to_string()));
                }
                Some((_, lines)) => *lines = None,
                None => return Err(ContentError::Damaged("indented before any entry".into())),
            }
            continue;
        }
        // A line at the left edge, or the end: the entry gathered so far is complete.
        if let Some((name, lines)) = current.take() {
            match lines.ok_or(()).and_then(|l| entry(name, l).map_err(|_| ())) {
                Ok(e) if wanted.wants(&e) => {
                    let len = serde_json::to_vec(&e)
                        .map(|v| v.len())
                        .unwrap_or(usize::MAX);
                    if out.entries.len() < MAX_ENTRIES
                        && out_bytes.saturating_add(len) <= MAX_ENTRIES_BYTES
                    {
                        out_bytes += len;
                        out.entries.push(e);
                    } else {
                        out.more = true;
                    }
                }
                Ok(_) => {}
                Err(()) => out.unreadable += 1,
            }
        }
        if end {
            return Ok(out);
        }
        seen += 1;
        if seen > MAX_LIST_ENTRIES {
            return Err(ContentError::TooLarge);
        }
        let Some((name, rest)) = key_of(body)? else {
            return Err(ContentError::Damaged(
                "an entry that is not a game's name".into(),
            ));
        };
        if !rest.is_empty() {
            return Err(ContentError::Damaged("an entry with no fields".into()));
        }
        current = Some((name, Some(Vec::new())));
    }
}

/// A scalar as written: plain (its meaning read from the text) or quoted (always text).
#[derive(Debug, Clone, PartialEq, Eq)]
enum Y {
    Map(Vec<(String, Y)>),
    Seq(Vec<Y>),
    Plain(String),
    Quoted(String),
}

fn entry(name: String, mut lines: Vec<(usize, String)>) -> Result<SaveEntry, ContentError> {
    let fields = if lines.is_empty() {
        Y::Map(Vec::new())
    } else {
        let mut i = 0;
        let indent = lines[0].0;
        let y = node(&mut lines, &mut i, indent, 0)?;
        if i != lines.len() {
            return Err(ContentError::Damaged("lines out of place".into()));
        }
        y
    };
    let Y::Map(_) = fields else {
        return Err(ContentError::Damaged("an entry that is not fields".into()));
    };
    let mut v = to_value(fields)?;
    if let Value::Object(o) = &mut v {
        o.insert("name".into(), Value::String(name));
    }
    serde_json::from_value(v).map_err(|e| ContentError::Damaged(e.to_string()))
}

fn to_value(y: Y) -> Result<Value, ContentError> {
    Ok(match y {
        Y::Map(m) => {
            let mut o = serde_json::Map::new();
            for (k, v) in m {
                let v = to_value(v)?;
                // A field with no value is as if it were not there.
                if v.is_null() {
                    continue;
                }
                if o.insert(k, v).is_some() {
                    return Err(ContentError::Damaged("a name given twice".into()));
                }
            }
            Value::Object(o)
        }
        Y::Seq(s) => Value::Array(s.into_iter().map(to_value).collect::<Result<_, _>>()?),
        Y::Quoted(t) => Value::String(t),
        Y::Plain(t) => match t.as_str() {
            "" | "~" | "null" => Value::Null,
            "true" => Value::Bool(true),
            "false" => Value::Bool(false),
            _ if !t.is_empty() && t.len() <= 19 && t.bytes().all(|b| b.is_ascii_digit()) => {
                t.parse::<u64>().map_or(Value::String(t), Value::from)
            }
            _ => Value::String(t),
        },
    })
}

fn is_item(text: &str) -> bool {
    text == "-" || text.starts_with("- ")
}

fn node(
    lines: &mut [(usize, String)],
    i: &mut usize,
    indent: usize,
    depth: usize,
) -> Result<Y, ContentError> {
    if depth > MAX_ENTRY_DEPTH {
        return Err(ContentError::Damaged(
            "nested deeper than PCTwin reads".into(),
        ));
    }
    if is_item(&lines[*i].1) {
        seq(lines, i, indent, depth)
    } else {
        map(lines, i, indent, depth)
    }
}

/// The value after a name with nothing on its own line: the block below, if any.
fn below(
    lines: &mut [(usize, String)],
    i: &mut usize,
    indent: usize,
    depth: usize,
) -> Result<Y, ContentError> {
    match lines.get(*i) {
        Some((n, t)) if *n > indent || (*n == indent && is_item(t)) => {
            let n = *n;
            node(lines, i, n, depth + 1)
        }
        _ => Ok(Y::Plain(String::new())),
    }
}

fn map(
    lines: &mut [(usize, String)],
    i: &mut usize,
    indent: usize,
    depth: usize,
) -> Result<Y, ContentError> {
    let mut out = Vec::new();
    while let Some((n, text)) = lines.get(*i) {
        if *n < indent || (*n == indent && is_item(text)) {
            break;
        }
        if *n > indent {
            return Err(ContentError::Damaged("lines out of place".into()));
        }
        let Some((key, rest)) = key_of(text)? else {
            return Err(ContentError::Damaged("a line that is not a field".into()));
        };
        *i += 1;
        let value = if rest.is_empty() {
            // A sequence may sit at its name's own indent.
            below(lines, i, indent, depth)?
        } else {
            inline(&rest)?
        };
        out.push((key, value));
    }
    Ok(Y::Map(out))
}

fn seq(
    lines: &mut [(usize, String)],
    i: &mut usize,
    indent: usize,
    depth: usize,
) -> Result<Y, ContentError> {
    let mut out = Vec::new();
    while let Some((n, text)) = lines.get(*i) {
        if *n != indent || !is_item(text) {
            if *n > indent {
                return Err(ContentError::Damaged("lines out of place".into()));
            }
            break;
        }
        let after = text[1..].trim_start_matches(' ');
        let at = indent + (text.len() - after.len());
        if after.is_empty() {
            *i += 1;
            out.push(below(lines, i, indent, depth)?);
        } else if key_of(after)?.is_some() {
            // `- name: value`: a map whose further names sit under the first.
            let after = after.to_string();
            lines[*i] = (at, after);
            out.push(map(lines, i, at, depth + 1)?);
        } else {
            let v = inline(after)?;
            *i += 1;
            out.push(v);
        }
    }
    Ok(Y::Seq(out))
}

/// A value on the same line as its name.
fn inline(rest: &str) -> Result<Y, ContentError> {
    match rest {
        "{}" => return Ok(Y::Map(Vec::new())),
        "[]" => return Ok(Y::Seq(Vec::new())),
        _ => {}
    }
    let (y, after) = scalar(rest)?;
    let after = after.trim_start_matches(' ');
    if after.is_empty() || after.starts_with('#') {
        Ok(y)
    } else {
        Err(ContentError::Damaged("text after a value".into()))
    }
}

/// Characters that begin something other than a plain scalar.
fn not_plain(c: char) -> bool {
    matches!(
        c,
        '[' | ']'
            | '{'
            | '}'
            | ','
            | '&'
            | '*'
            | '!'
            | '|'
            | '>'
            | '%'
            | '@'
            | '`'
            | '?'
            | '-'
            | ':'
            | '#'
    )
}

/// A scalar at the start of `s`, and what follows it.
fn scalar(s: &str) -> Result<(Y, &str), ContentError> {
    let too_long = || ContentError::Damaged("a text longer than PCTwin reads".into());
    if let Some(body) = s.strip_prefix('"') {
        let mut out = String::new();
        let mut chars = body.char_indices();
        while let Some((i, c)) = chars.next() {
            match c {
                '"' => return Ok((Y::Quoted(out), &body[i + 1..])),
                '\\' => {
                    let (_, e) = chars
                        .next()
                        .ok_or_else(|| ContentError::Damaged("a text with no end".into()))?;
                    let hex = |n: usize, chars: &mut std::str::CharIndices<'_>| {
                        let digits: String = chars.by_ref().take(n).map(|(_, c)| c).collect();
                        u32::from_str_radix(&digits, 16)
                            .ok()
                            .filter(|_| digits.len() == n)
                            .and_then(char::from_u32)
                            .ok_or_else(|| {
                                ContentError::Damaged("an escape PCTwin does not read".into())
                            })
                    };
                    out.push(match e {
                        '"' => '"',
                        '\\' => '\\',
                        '/' => '/',
                        'n' => '\n',
                        't' => '\t',
                        'r' => '\r',
                        '0' => '\0',
                        ' ' => ' ',
                        'x' => hex(2, &mut chars)?,
                        'u' => hex(4, &mut chars)?,
                        'U' => hex(8, &mut chars)?,
                        _ => {
                            return Err(ContentError::Damaged(
                                "an escape PCTwin does not read".into(),
                            ));
                        }
                    });
                }
                c => out.push(c),
            }
            if out.len() > MAX_TEXT {
                return Err(too_long());
            }
        }
        return Err(ContentError::Damaged("a text with no end".into()));
    }
    if let Some(body) = s.strip_prefix('\'') {
        let mut out = String::new();
        let mut rest = body;
        loop {
            let q = rest
                .find('\'')
                .ok_or_else(|| ContentError::Damaged("a text with no end".into()))?;
            out.push_str(&rest[..q]);
            if out.len() > MAX_TEXT {
                return Err(too_long());
            }
            rest = &rest[q + 1..];
            match rest.strip_prefix('\'') {
                Some(r) => {
                    out.push('\'');
                    rest = r;
                }
                None => return Ok((Y::Quoted(out), rest)),
            }
        }
    }
    match s.chars().next() {
        None => return Ok((Y::Plain(String::new()), s)),
        Some(c) if not_plain(c) => {
            // `-5` and `:x` style plain scalars are not in the format: refused, not guessed.
            return Err(ContentError::Damaged("a value PCTwin does not read".into()));
        }
        _ => {}
    }
    let end = s.find(" #").unwrap_or(s.len());
    let t = s[..end].trim_end_matches(' ');
    if t.contains(": ") || t.ends_with(':') {
        return Err(ContentError::Damaged("a value PCTwin does not read".into()));
    }
    if t.len() > MAX_TEXT {
        return Err(too_long());
    }
    Ok((Y::Plain(t.to_string()), &s[end..]))
}

/// `name: rest` (`rest` empty when the value is on the lines below), or none if `text` is not a
/// field.
fn key_of(text: &str) -> Result<Option<(String, String)>, ContentError> {
    if text.starts_with(['"', '\'']) {
        let (y, after) = scalar(text)?;
        let Y::Quoted(k) = y else {
            return Ok(None);
        };
        let Some(after) = after.strip_prefix(':') else {
            return Ok(None);
        };
        if !(after.is_empty() || after.starts_with(' ')) {
            return Ok(None);
        }
        return Ok(Some((k, after.trim_start_matches(' ').to_string())));
    }
    if text.chars().next().is_none_or(not_plain) {
        return Ok(None);
    }
    let (k, rest) = if let Some(at) = text.find(": ") {
        (&text[..at], text[at + 2..].trim_start_matches(' '))
    } else if let Some(k) = text.strip_suffix(':') {
        (k, "")
    } else {
        return Ok(None);
    };
    if k.len() > MAX_TEXT || k.contains(" #") {
        return Err(ContentError::Damaged("a name PCTwin does not read".into()));
    }
    Ok(Some((
        k.trim_end_matches(' ').to_string(),
        rest.to_string(),
    )))
}

#[cfg(test)]
mod tests {
    use super::*;

    const SEAL: InChild = pctwin_parse_seal::in_child();

    fn vdf_of(s: &str) -> Result<Tree, ContentError> {
        record_tree(SEAL, RecordFormat::Vdf, s.as_bytes())
    }

    #[test]
    fn steam_library_folders_read_as_a_tree() {
        let t = vdf_of(
            "\"libraryfolders\"\n{\n\t\"0\"\n\t{\n\t\t\"path\"\t\t\"C:\\\\Program Files (x86)\\\\Steam\"\n\t\t\"apps\"\n\t\t{\n\t\t\t\"228980\"\t\t\"123\"\n\t\t}\n\t}\n\t// a comment\n\t\"1\" { \"path\" \"D:\\\\Games\" }\n}\n",
        )
        .unwrap();
        let lf = t.get("LibraryFolders").unwrap();
        assert_eq!(lf.children().len(), 2);
        assert_eq!(
            lf.get("0").and_then(|x| x.get("path")).and_then(Tree::text),
            Some(r"C:\Program Files (x86)\Steam")
        );
        assert_eq!(
            lf.at(&["1".into(), "PATH".into()]).and_then(Tree::text),
            Some(r"D:\Games")
        );
    }

    #[test]
    fn broken_vdf_is_refused_never_half_read() {
        for bad in [
            "\"a\" {",
            "\"a\" { \"b\" }",
            "\"a\" \"b\" }",
            "{ \"a\" \"b\" }",
            "\"a",
            "\"a\" \"b\" [$WIN32]",
            "\"a\" \"\\q\"",
            "\"a\"",
        ] {
            assert!(vdf_of(bad).is_err(), "{bad}");
        }
        let deep = format!(
            "{}{}",
            "\"a\" {".repeat(MAX_DEPTH + 2),
            "}".repeat(MAX_DEPTH + 2)
        );
        assert!(vdf_of(&deep).is_err());
        assert_eq!(vdf_of("").unwrap(), Tree::Map(vec![]));
        let wide: String = (0..MAX_NODES)
            .map(|i| format!("\"k{i}\" \"v\"\n"))
            .collect();
        assert_eq!(vdf_of(&wide), Err(ContentError::TooLarge));
        let long = format!("\"a\" \"{}\"", "x".repeat(MAX_TEXT + 1));
        assert!(vdf_of(&long).is_err());
        assert_eq!(
            record_tree(
                SEAL,
                RecordFormat::Vdf,
                &vec![b' '; MAX_RECORD as usize + 1]
            ),
            Err(ContentError::TooLarge)
        );
        assert!(record_tree(SEAL, RecordFormat::Vdf, b"\"a\" \"\xff\"").is_err());
    }

    #[test]
    fn json_records_read_as_the_same_tree() {
        let t = record_tree(
            SEAL,
            RecordFormat::Json,
            br#"{"DisplayName":"A Game","InstallLocation":"D:\\Epic\\AGame","InstallSize":12,"bIsIncompleteInstall":false,"List":["x"]}"#,
        )
        .unwrap();
        assert_eq!(t.get("displayname").and_then(Tree::text), Some("A Game"));
        assert_eq!(t.get("InstallSize").and_then(Tree::text), Some("12"));
        assert_eq!(
            t.get("bIsIncompleteInstall").and_then(Tree::text),
            Some("false")
        );
        assert_eq!(
            t.at(&["List".into(), "0".into()]).and_then(Tree::text),
            Some("x")
        );
        assert!(record_tree(SEAL, RecordFormat::Json, b"{").is_err());
        let deep = format!("{}{}", "[".repeat(40), "]".repeat(40));
        assert!(record_tree(SEAL, RecordFormat::Json, deep.as_bytes()).is_err());
    }

    fn read(text: &str, wanted: &Wanted) -> Result<ManifestRead, ContentError> {
        save_manifest(
            SEAL,
            &mut std::io::Cursor::new(text.as_bytes()),
            text.len() as u64,
            wanted,
        )
    }

    const LIST: &str = "---\n\"A Game: Remade\":\n  cloud:\n    steam: true\n  files:\n    \"<winAppData>/A Game/Saves\":\n      tags:\n        - save\n      when:\n        - os: windows\n          store: steam\n    <home>/.agame/*.sav:\n      when:\n      - os: linux\n  installDir:\n    AGame: {}\n  steam:\n    id: 1234\nOther Game:\n  steam:\n    id: 99\n  id:\n    steamExtra:\n      - 77\n'Quoted ''One''':\n  gog:\n    id: 5\n";

    #[test]
    fn a_save_list_gives_the_entries_asked_for() {
        let all = Wanted {
            names: vec!["a game: remade".into()],
            steam: vec![77],
            gog: vec![5],
        };
        let r = read(LIST, &all).unwrap();
        assert_eq!(r.unreadable, 0);
        assert!(!r.more);
        let names: Vec<&str> = r.entries.iter().map(|e| e.name.as_str()).collect();
        assert_eq!(names, ["A Game: Remade", "Other Game", "Quoted 'One'"]);
        let a = &r.entries[0];
        assert_eq!(a.cloud.get("steam"), Some(&true));
        assert_eq!(a.steam, Some(StoreId { id: 1234 }));
        assert!(a.install_dir.contains_key("AGame"));
        let rule = &a.files["<winAppData>/A Game/Saves"];
        assert_eq!(rule.tags, ["save"]);
        assert_eq!(rule.when[0].os.as_deref(), Some("windows"));
        assert_eq!(rule.when[0].store.as_deref(), Some("steam"));
        assert_eq!(
            a.files["<home>/.agame/*.sav"].when[0].os.as_deref(),
            Some("linux")
        );
        // Asked for nothing: nothing handed back.
        assert!(read(LIST, &Wanted::default()).unwrap().entries.is_empty());
    }

    #[test]
    fn entries_outside_the_format_are_counted_not_guessed() {
        let wanted = Wanted {
            names: vec!["x".into(), "y".into(), "z".into(), "w".into()],
            ..Wanted::default()
        };
        let text = "x:\n  files:\n    &anchor a: b\ny:\n  alias: *anchor\nz:\n  notes: |\n    text\nw:\n  steam:\n    id: 3\n";
        let r = read(text, &wanted).unwrap();
        assert_eq!(r.unreadable, 3);
        assert_eq!(r.entries.len(), 1);
        assert_eq!(r.entries[0].name, "w");
        // The whole file out of shape: refused.
        assert!(read("  indented: first\n", &wanted).is_err());
        assert!(read("x:\n\tfiles: {}\n", &wanted).is_err());
        assert!(read("[a, b]\n", &wanted).is_err());
        assert!(read("x: 1\n", &wanted).is_err());
        assert_eq!(
            save_manifest(
                SEAL,
                &mut std::io::Cursor::new(b"".as_slice()),
                MAX_MANIFEST + 1,
                &wanted
            ),
            Err(ContentError::TooLarge)
        );
        let long = format!("{}:\n", "x".repeat(MAX_LINE + 1));
        assert!(read(&long, &wanted).is_err());
    }

    #[test]
    fn handed_back_entries_are_bounded() {
        let mut text = String::new();
        for i in 0..(MAX_ENTRIES + 5) {
            text.push_str(&format!("g{i}:\n  steam:\n    id: {i}\n"));
        }
        let wanted = Wanted {
            steam: (0..(MAX_ENTRIES as u64 + 5)).collect(),
            ..Wanted::default()
        };
        let r = read(&text, &wanted).unwrap();
        assert_eq!(r.entries.len(), MAX_ENTRIES);
        assert!(r.more);
    }
}
