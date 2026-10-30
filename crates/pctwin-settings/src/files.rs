//! App settings files, found by their shape (Task List 2.4: same-OS app settings, cross-OS
//! translations, workspace and recent files).
//!
//! Every settings-shaped file (JSON, JSON with comments, TOML, INI, XML) under the person's app
//! settings folders is found, whether or not the catalogue knows the app. Each one is read
//! structurally (never by string replacement across an unknown format) and every field passes
//! the same generic rules as any setting ([`crate::classify`]): credentials, machine and account
//! identities, caches, window places, programs to start and paths to things not moved are left
//! out and reported; paths into moved folders are rewritten; recent-file entries whose file did
//! not move are dropped. Never a whole-folder copy.
//!
//! A catalogue app entry enriches: its files' writer (2.4, 2.5, 2.1 or nobody: T24-017), field
//! rules (drop, hold back, rewrite a path, switch a start-up hook off: T24-003), rules that apply
//! only between a Mac and another system (keyboard shortcuts: an equivalent the person confirms),
//! path placeholders such as JetBrains' `$USER_HOME$`, and the oldest app version the rules were
//! checked with (an older app on the new laptop is a guided step: T24-005). A running app's files
//! wait until it is closed (T24-018).
//!
//! Between different systems, only files a catalogue entry describes move.

use std::collections::{BTreeMap, BTreeSet};
use std::path::{Path, PathBuf};

use pctwin_recipe::{SettingValue, SettingsRoot};
use pctwin_types::nesting::{self, MAX_NESTING, Syntax};
use serde_json::Value;

use crate::apps::{parse_version, path_matches};
use crate::catalogue::{
    AppEntry, AppFile, Catalogue, FileFormat, MacroRoot, Rule, System, Table, Writer,
};
use crate::classify::{Ctx, Fate, InsideHome, Why, classify};

/// The largest settings file read ([`pctwin_types::limits`]).
pub use pctwin_types::limits::SETTINGS_FILE_BYTES as MAX_FILE;
use pctwin_types::limits::{SETTINGS_FILE_DEPTH as MAX_DEPTH, SETTINGS_FILES as MAX_FILES};

/// Folder names that hold caches, logs and browser-engine data, never settings.
const SKIP_DIRS: &[&str] = &[
    "cache",
    "caches",
    "cacheddata",
    "gpucache",
    "code cache",
    "dawncache",
    "shadercache",
    "crashpad",
    "crashes",
    "logs",
    "log",
    "tmp",
    "temp",
    "blob_storage",
    "indexeddb",
    "local storage",
    "session storage",
    "service worker",
    "webstorage",
    "node_modules",
    "extensions",
    "plugins",
    "backups",
];

/// One settings-shaped file found on the old laptop.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct FoundFile {
    pub root: SettingsRoot,
    /// Its path inside the root, `/`-separated.
    pub rel: String,
    /// The app or vendor it belongs to (its first folder).
    pub label: String,
    pub format: Option<FileFormat>,
}

/// The format a file's name says.
pub fn format_of(name: &str) -> Option<FileFormat> {
    let l = name.to_ascii_lowercase();
    let ext = l.rsplit_once('.').map(|(_, e)| e)?;
    Some(match ext {
        "json" | "jsonc" | "code-workspace" => FileFormat::Jsonc,
        "toml" => FileFormat::Toml,
        "ini" | "conf" | "cfg" | "properties" => FileFormat::Ini,
        "xml" | "icls" => FileFormat::Xml,
        _ => return None,
    })
}

/// Finds every settings-shaped file in the roots (the old laptop's app settings folders). Never
/// follows a link; skips cache-like folders; bounded in depth, count and size.
pub fn discover_files(roots: &[(SettingsRoot, PathBuf)]) -> Vec<FoundFile> {
    let mut out = Vec::new();
    for (root, dir) in roots {
        walk(*root, dir, "", 0, &mut out);
    }
    out
}

fn walk(root: SettingsRoot, dir: &Path, rel: &str, depth: usize, out: &mut Vec<FoundFile>) {
    if depth > MAX_DEPTH || out.len() >= MAX_FILES {
        return;
    }
    let Ok(entries) = std::fs::read_dir(dir) else {
        return;
    };
    let mut entries: Vec<_> = entries.flatten().collect();
    entries.sort_by_key(|e| e.file_name());
    for e in entries {
        let Ok(name) = e.file_name().into_string() else {
            continue;
        };
        let Ok(meta) = std::fs::symlink_metadata(e.path()) else {
            continue;
        };
        let child = if rel.is_empty() {
            name.clone()
        } else {
            format!("{rel}/{name}")
        };
        if meta.file_type().is_symlink() {
            continue;
        }
        if meta.is_dir() {
            if !SKIP_DIRS.contains(&name.to_lowercase().as_str()) {
                walk(root, &e.path(), &child, depth + 1, out);
            }
        } else if meta.is_file() && meta.len() <= MAX_FILE && depth > 0 {
            let format = format_of(&name);
            if format.is_some() {
                let label = child.split('/').next().unwrap_or_default().to_string();
                out.push(FoundFile {
                    root,
                    rel: child,
                    label,
                    format,
                });
            }
        }
    }
}

/// What the move knows when filtering files.
pub struct FileCtx<'a> {
    pub classify: Ctx<'a>,
    pub from: System,
    pub to: System,
    /// The person's home folder on each laptop (for `$USER_HOME$`-style placeholders).
    pub home_old: &'a str,
    pub home_new: &'a str,
    /// Apps running on the new laptop now (catalogue app names).
    pub running: &'a BTreeSet<String>,
    /// Each app's version on the old and new laptop, where known.
    pub versions: &'a BTreeMap<String, (String, String)>,
    /// A versioned settings folder's new name (`IntelliJIdea2025.2` to `IntelliJIdea2025.3`).
    pub folder_renames: &'a BTreeMap<String, String>,
}

/// One field left out of a file, and why.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Left {
    pub field: String,
    pub why: Why,
}

/// What happens to one found file.
#[derive(Debug, Clone, PartialEq)]
pub enum FileOutcome {
    /// Written on the new laptop at `rel` with these bytes (through the executor's settings
    /// copy, journaled with undo).
    Move {
        rel: String,
        bytes: Vec<u8>,
        rewritten: usize,
        left: Vec<Left>,
        /// Translated between systems in a way the person confirms (keyboard shortcuts).
        equivalent: bool,
    },
    /// Another part of PCTwin writes it (or nobody: sign-ins, machine state).
    OtherWriter(Writer),
    /// The app is running on the new laptop: waits until it is closed.
    WaitForClose,
    /// The person's step, and why.
    Guided(&'static str),
    NotMoved(Why),
}

pub const OLDER_APP: &str =
    "the new laptop has an older version of this app; open it and bring settings over there";

/// The entry and file rule that describe a file, if any.
pub fn entry_for<'a>(cat: &'a Catalogue, f: &FoundFile) -> Option<(&'a AppEntry, &'a AppFile)> {
    cat.apps.iter().find_map(|a| {
        a.files
            .iter()
            .find(|af| af.root == f.root && path_matches(&af.path, &f.rel))
            .map(|af| (a, af))
    })
}

/// Plans one found file from its bytes on the old laptop.
pub fn plan_file(cat: &Catalogue, f: &FoundFile, bytes: &[u8], ctx: &FileCtx<'_>) -> FileOutcome {
    // Its own size cap, whoever read the bytes (fourth review, small finding).
    if u64::try_from(bytes.len()).map_or(true, |n| n > MAX_FILE) {
        return FileOutcome::NotMoved(Why::TooLarge);
    }
    let entry = entry_for(cat, f);
    if let Some((_, af)) = entry
        && af.writer != Writer::Settings
    {
        return FileOutcome::OtherWriter(af.writer);
    }
    if entry.is_none() && ctx.from != ctx.to {
        return FileOutcome::NotMoved(Why::OtherSystem);
    }
    let mut rel = f.rel.clone();
    if let Some((app, _)) = entry {
        if ctx.running.contains(&app.app) {
            return FileOutcome::WaitForClose;
        }
        if let Some(why) = version_gate(app, f, ctx) {
            return FileOutcome::Guided(why);
        }
        if app.versioned_folder.is_some()
            && let Some(old) = rel.split('/').nth(1).map(str::to_string)
            && let Some(new) = ctx.folder_renames.get(&old)
        {
            rel = rel.replacen(&format!("/{old}/"), &format!("/{new}/"), 1);
        }
    }
    let format = entry.map(|(_, af)| af.format).or(f.format);
    let rules: Vec<(String, Rule)> = entry
        .map(|(_, af)| {
            // Between different systems (Windows, a Mac, Linux); two Linux desktops keep an
            // app's files the same way.
            let cross = ctx.from.column() != ctx.to.column();
            af.rules
                .iter()
                .chain(af.cross_os.iter().filter(|_| cross))
                .map(|r| (r.at.clone(), r.rule.clone()))
                .collect()
        })
        .unwrap_or_default();
    let default = entry
        .map(|(_, af)| af.default.clone())
        .unwrap_or(Rule::Copy);
    let macros = entry.map(|(_, af)| af.macros.clone()).unwrap_or_default();
    let mut w = Walker {
        tables: &cat.tables,
        ctx,
        store: f.rel.clone(),
        rules,
        default,
        macros,
        rewritten: 0,
        left: Vec::new(),
        equivalent: false,
    };
    let out = match format {
        Some(FileFormat::Jsonc | FileFormat::Json) => w.json(bytes),
        Some(FileFormat::Toml) => w.toml(bytes),
        Some(FileFormat::Ini) => w.ini(bytes),
        Some(FileFormat::Xml) => w.xml(bytes),
        Some(FileFormat::Opaque) | None => Err(Why::Kind),
    };
    match out {
        Ok(bytes) => FileOutcome::Move {
            rel,
            bytes,
            rewritten: w.rewritten,
            left: w.left,
            equivalent: w.equivalent,
        },
        Err(why) => FileOutcome::NotMoved(why),
    }
}

fn version_gate(app: &AppEntry, f: &FoundFile, ctx: &FileCtx<'_>) -> Option<&'static str> {
    let min = parse_version(&app.min_version)?;
    let (old, new) = match ctx.versions.get(&app.app) {
        Some((o, n)) => (parse_version(o), parse_version(n)),
        None if app.versioned_folder.is_some() => {
            // The version is in the folder name (`IntelliJIdea2025.2`).
            let folder = f.rel.split('/').nth(1)?;
            let digits = folder.trim_start_matches(|c: char| !c.is_ascii_digit());
            let new_folder = ctx
                .folder_renames
                .get(folder)
                .map(String::as_str)
                .unwrap_or(folder);
            let new_digits = new_folder.trim_start_matches(|c: char| !c.is_ascii_digit());
            (parse_version(digits), parse_version(new_digits))
        }
        None => return None,
    };
    match (old, new) {
        (Some(o), _) if o < min => Some(OLDER_APP),
        (Some(o), Some(n)) if n < o => Some(OLDER_APP),
        _ => None,
    }
}

struct Walker<'a, 'b> {
    tables: &'a BTreeMap<String, Table>,
    ctx: &'a FileCtx<'b>,
    store: String,
    rules: Vec<(String, Rule)>,
    default: Rule,
    macros: BTreeMap<String, MacroRoot>,
    rewritten: usize,
    left: Vec<Left>,
    equivalent: bool,
}

/// What a walker does to one leaf.
enum Leaf {
    Keep,
    Replace(Value),
    Remove(Why),
}

impl Walker<'_, '_> {
    fn rule_for(&self, path: &str) -> Option<Rule> {
        self.rules
            .iter()
            .find(|(at, _)| path_matches(at, path))
            .map(|(_, r)| r.clone())
    }

    fn leave(&mut self, field: &str, why: Why) {
        self.left.push(Left {
            field: field.to_string(),
            why,
        });
    }

    /// Expands placeholders (`$USER_HOME$/x` to the old home) before a path is judged, in the
    /// old system's own separators (JetBrains writes `/` on Windows too).
    fn expand(&self, s: &str) -> Option<String> {
        let home = self.ctx.home_old.trim_end_matches(['/', '\\']);
        let windows = home.as_bytes().get(1) == Some(&b':');
        self.macros.iter().find_map(|(m, root)| match root {
            MacroRoot::Home => s.strip_prefix(m.as_str()).map(|rest| {
                let rest = if windows {
                    rest.replace('/', "\\")
                } else {
                    rest.to_string()
                };
                format!("{home}{rest}")
            }),
        })
    }

    /// Puts the placeholder back in front of a new path inside the new home.
    fn collapse(&self, s: &str) -> String {
        for (m, root) in &self.macros {
            let MacroRoot::Home = root;
            let home = self.ctx.home_new.trim_end_matches(['/', '\\']);
            if let Some(rest) = s.strip_prefix(home)
                && (rest.is_empty() || rest.starts_with(['/', '\\']))
            {
                return format!("{m}{}", rest.replace('\\', "/"));
            }
        }
        s.to_string()
    }

    /// One leaf: the rule for its path first, then the generic rules.
    fn leaf(&mut self, path: &str, name: &str, v: &Value) -> Leaf {
        let rule = self.rule_for(path).unwrap_or_else(|| self.default.clone());
        match rule {
            Rule::Drop => return Leaf::Remove(Why::Exception("left out for this app".into())),
            Rule::HoldBack => return Leaf::Remove(Why::StartsProgram),
            Rule::Disable(off) => {
                self.leave(path, Why::StartsProgram);
                return Leaf::Replace(off);
            }
            Rule::Translate { table, keys } => {
                if let Value::String(s) = v
                    && let Some(t) = self.tables.get(&table)
                {
                    let new = translate(t, s, self.ctx.from, self.ctx.to, keys);
                    // A whole value the table names (a keymap such as `$default`) is one of
                    // the catalogue's own checked names, not a place to judge.
                    let named = !keys
                        && t.column(self.ctx.from.column())
                            .is_some_and(|c| t.rows.iter().any(|r| r[c].eq_ignore_ascii_case(s)));
                    if named {
                        self.equivalent |= new != *s;
                        return Leaf::Replace(Value::String(new));
                    }
                    if new != *s {
                        // The translated value is judged by the same rules as any other.
                        return match self.judge(name, &Value::String(new.clone())) {
                            Leaf::Keep => {
                                self.equivalent = true;
                                Leaf::Replace(Value::String(new))
                            }
                            other => other,
                        };
                    }
                }
            }
            Rule::Copy | Rule::RewritePath => {}
        }
        self.judge(name, v)
    }

    /// The generic rules for one leaf.
    fn judge(&mut self, name: &str, v: &Value) -> Leaf {
        let sv = match v {
            Value::String(s) => SettingValue::Text(s.clone()),
            Value::Bool(b) => SettingValue::Bool(*b),
            Value::Number(n) => match n.as_i64() {
                Some(i) => SettingValue::Int(i),
                None => SettingValue::Text(n.to_string()),
            },
            _ => return Leaf::Keep,
        };
        // A placeholder path is judged as the full path it stands for.
        let expanded = match &sv {
            SettingValue::Text(t) => self.expand(t),
            _ => None,
        };
        let judged = expanded.clone().map(SettingValue::Text).unwrap_or(sv);
        // A path is only ever pointed inside the person's home on the new laptop.
        let inside = InsideHome {
            inner: self.ctx.classify.paths,
            home: self.ctx.home_new,
        };
        let cctx = Ctx {
            paths: &inside,
            same_screens: self.ctx.classify.same_screens,
            hints: self.ctx.classify.hints,
            facts: self.ctx.classify.facts,
        };
        match classify(&self.store, name, &judged, &cctx) {
            Fate::Copy(_) => Leaf::Keep,
            Fate::Rewritten(SettingValue::Text(n)) => {
                self.rewritten += 1;
                let n = if expanded.is_some() {
                    self.collapse(&n)
                } else {
                    n
                };
                Leaf::Replace(Value::String(n))
            }
            Fate::Rewritten(_) => Leaf::Remove(Why::Kind),
            Fate::NotMoved(why) => Leaf::Remove(why),
        }
    }

    fn json(&mut self, bytes: &[u8]) -> Result<Vec<u8>, Why> {
        let text = std::str::from_utf8(bytes).map_err(|_| Why::Kind)?;
        nesting::check(text, Syntax::Jsonc).map_err(|_| Why::TooDeep)?;
        let clean = strip_jsonc(text.trim_start_matches('\u{feff}')).ok_or(Why::Kind)?;
        let v: Value = serde_json::from_str(&clean).map_err(json_why)?;
        let v = self.value(v, "", "", 0).ok_or(Why::Kind)?;
        let mut out = Vec::new();
        let fmt = serde_json::ser::PrettyFormatter::with_indent(b"    ");
        let mut ser = serde_json::Serializer::with_formatter(&mut out, fmt);
        serde::Serialize::serialize(&v, &mut ser).map_err(|_| Why::Kind)?;
        out.push(b'\n');
        Ok(out)
    }

    fn toml(&mut self, bytes: &[u8]) -> Result<Vec<u8>, Why> {
        let text = std::str::from_utf8(bytes).map_err(|_| Why::Kind)?;
        nesting::check(text, Syntax::Toml).map_err(|_| Why::TooDeep)?;
        let v: Value = toml::from_str(text).map_err(toml_why)?;
        let v = self.value(v, "", "", 0).ok_or(Why::Kind)?;
        toml::to_string(&v)
            .map(String::into_bytes)
            .map_err(|_| Why::Kind)
    }

    /// Walks a JSON value: `None` when it is removed. In a list, an entry that lost a path (a
    /// recent file that did not move) is removed whole.
    fn value(&mut self, v: Value, path: &str, name: &str, depth: usize) -> Option<Value> {
        if depth > MAX_NESTING {
            // Never reached past the gate; reported, never dropped silently.
            self.leave(path, Why::TooDeep);
            return None;
        }
        let join = |p: &str, k: &str| {
            if p.is_empty() {
                k.to_string()
            } else {
                format!("{p}/{k}")
            }
        };
        match v {
            Value::Object(map) => {
                if let Some(rule) = self.rule_for(path).filter(|_| !path.is_empty()) {
                    match rule {
                        Rule::Drop => {
                            self.leave(path, Why::Exception("left out for this app".into()));
                            return None;
                        }
                        Rule::HoldBack => {
                            self.leave(path, Why::StartsProgram);
                            return None;
                        }
                        _ => {}
                    }
                }
                let mut out = serde_json::Map::new();
                for (k, child) in map {
                    let p = join(path, &k);
                    if let Some(c) = self.value(child, &p, &k, depth + 1) {
                        out.insert(k, c);
                    }
                }
                Some(Value::Object(out))
            }
            Value::Array(items) => {
                if let Some(Rule::Drop | Rule::HoldBack) =
                    self.rule_for(path).filter(|_| !path.is_empty())
                {
                    self.leave(path, Why::StartsProgram);
                    return None;
                }
                let mut out = Vec::new();
                for (i, child) in items.into_iter().enumerate() {
                    let p = join(path, &i.to_string());
                    let before = self.left.len();
                    let kept = self.value(child, &p, name, depth + 1);
                    let lost_path = self.left[before..].iter().any(|l| {
                        matches!(
                            l.why,
                            Why::PathNotMoved
                                | Why::RecentMissing
                                | Why::Fetches
                                | Why::PlaceUnknown
                        )
                    });
                    match kept {
                        Some(c) if !lost_path => out.push(c),
                        _ => {}
                    }
                }
                Some(Value::Array(out))
            }
            leaf => match self.leaf(path, &context_name(path, name), &leaf) {
                Leaf::Keep => Some(leaf),
                Leaf::Replace(n) => Some(n),
                Leaf::Remove(why) => {
                    self.leave(path, why);
                    None
                }
            },
        }
    }

    /// INI: line by line, comments and layout kept; a left-out key's line is removed.
    fn ini(&mut self, bytes: &[u8]) -> Result<Vec<u8>, Why> {
        let text = std::str::from_utf8(bytes).map_err(|_| Why::Kind)?;
        let mut section = String::new();
        let mut out = String::with_capacity(text.len());
        for line in text.split_inclusive('\n') {
            let body = line.trim_end_matches(['\r', '\n']);
            let ending = &line[body.len()..];
            let t = body.trim();
            if t.starts_with('[') && t.ends_with(']') {
                section = t[1..t.len() - 1].trim().to_string();
                out.push_str(line);
                continue;
            }
            if t.is_empty() || t.starts_with([';', '#']) {
                out.push_str(line);
                continue;
            }
            let Some(parts) = IniLine::parse(body) else {
                out.push_str(line);
                continue;
            };
            let key = parts.key.trim();
            let path = if section.is_empty() {
                key.to_string()
            } else {
                format!("{section}/{key}")
            };
            match self.leaf(&path, key, &Value::String(parts.value.to_string())) {
                Leaf::Keep => out.push_str(line),
                Leaf::Replace(Value::String(n)) => {
                    out.push_str(&parts.with_value(&n));
                    out.push_str(ending);
                }
                Leaf::Replace(other) => {
                    out.push_str(&parts.with_value(&other.to_string()));
                    out.push_str(ending);
                }
                Leaf::Remove(why) => self.leave(&path, why),
            }
        }
        Ok(out.into_bytes())
    }

    /// XML: every attribute and text is judged; a rewritten value is replaced in place (exactly
    /// its bytes); an element holding a value that may not move is removed whole. A root that
    /// would be removed means the file does not move.
    fn xml(&mut self, bytes: &[u8]) -> Result<Vec<u8>, Why> {
        let text = std::str::from_utf8(bytes).map_err(|_| Why::Kind)?;
        nesting::check(text, Syntax::Xml).map_err(|_| Why::TooDeep)?;
        let opts = roxmltree::ParsingOptions {
            allow_dtd: false,
            nodes_limit: 200_000,
            ..Default::default()
        };
        let doc = roxmltree::Document::parse_with_options(text, opts).map_err(|_| Why::Kind)?;
        let root = doc.root_element();
        let mut edits: Vec<(std::ops::Range<usize>, String)> = Vec::new();
        let mut removed: Vec<std::ops::Range<usize>> = Vec::new();
        for node in doc.descendants().filter(|n| n.is_element()) {
            let path: Vec<&str> = node
                .ancestors()
                .filter(|a| a.is_element())
                .map(|a| a.tag_name().name())
                .collect::<Vec<_>>()
                .into_iter()
                .rev()
                .collect();
            let epath = path.join("/");
            let own_name = node.attribute("name").unwrap_or(node.tag_name().name());
            let mut drop_it: Option<Why> = None;
            for a in node.attributes() {
                let apath = format!("{epath}/@{}", a.name());
                // An element's name is what it is, not a value; only a rule makes it one
                // (`<active_keymap name="..."/>`).
                if a.name() == "name" && self.rule_for(&apath).is_none() {
                    continue;
                }
                // `<option name="x" value="y"/>`: the value is named by its element's name.
                let name = if a.name() == "value" {
                    own_name
                } else {
                    a.name()
                };
                match self.leaf(&apath, name, &Value::String(a.value().to_string())) {
                    Leaf::Keep => {}
                    Leaf::Replace(Value::String(n)) => {
                        edits.push((a.range_value(), xml_escape(&n)))
                    }
                    Leaf::Replace(_) => drop_it = Some(Why::Kind),
                    Leaf::Remove(why) => drop_it = Some(why),
                }
            }
            if drop_it.is_none()
                && let Some(t) = node.children().find(|c| c.is_text())
                && node.children().all(|c| c.is_text())
                && !t.text().unwrap_or("").trim().is_empty()
            {
                let raw = t.text().unwrap_or("");
                match self.leaf(&epath, own_name, &Value::String(raw.trim().to_string())) {
                    Leaf::Keep => {}
                    Leaf::Replace(Value::String(n)) => edits.push((t.range(), xml_escape(&n))),
                    Leaf::Replace(_) => drop_it = Some(Why::Kind),
                    Leaf::Remove(why) => drop_it = Some(why),
                }
            }
            if let Some(why) = drop_it {
                if node == root {
                    return Err(why);
                }
                self.leave(&epath, why);
                removed.push(node.range());
            }
        }
        // Edits inside a removed element are dropped with it.
        removed.sort_by_key(|r| (r.start, std::cmp::Reverse(r.end)));
        let mut outer: Vec<std::ops::Range<usize>> = Vec::new();
        for r in removed {
            if !outer.iter().any(|o| o.start <= r.start && r.end <= o.end) {
                outer.push(r);
            }
        }
        let mut all: Vec<(std::ops::Range<usize>, String)> =
            outer.iter().map(|r| (r.clone(), String::new())).collect();
        for (r, s) in edits {
            if !outer.iter().any(|o| o.start <= r.start && r.end <= o.end) {
                all.push((r, s));
            }
        }
        all.sort_by_key(|(r, _)| std::cmp::Reverse(r.start));
        let mut out = text.to_string();
        for (r, s) in all {
            out.replace_range(r, &s);
        }
        Ok(out.into_bytes())
    }
}

/// Why JSON could not be read: its parser's own nesting limit (128 levels, below the gate's) is
/// reported as nesting too, never as a damaged file.
fn json_why(e: serde_json::Error) -> Why {
    if e.to_string().starts_with("recursion limit exceeded") {
        Why::TooDeep
    } else {
        Why::Kind
    }
}

/// Why TOML could not be read: its parser's own nesting limit is reported as nesting too, never
/// as a damaged file (fifth review, finding 10; the same as [`json_why`]).
fn toml_why(e: toml::de::Error) -> Why {
    if e.to_string().contains("recursion limit") {
        Why::TooDeep
    } else {
        Why::Kind
    }
}

/// One `key = value` line of an INI file, in its parts, so a new value is written back between
/// the line's own text: its spacing and its quotes kept exactly. The value is everything after
/// `=` (a `;` later in the line included), as Windows and Python read it.
struct IniLine<'a> {
    /// Everything before the value (the key, the `=`, the spaces after it).
    before: &'a str,
    key: &'a str,
    /// The value, without its quotes.
    value: &'a str,
    quoted: bool,
    /// Everything after the value (a closing quote is not part of it; trailing spaces are).
    after: &'a str,
}

impl<'a> IniLine<'a> {
    fn parse(body: &'a str) -> Option<Self> {
        let eq = body.find('=')?;
        let key = &body[..eq];
        let rest = &body[eq + 1..];
        let start = eq + 1 + (rest.len() - rest.trim_start().len());
        let tail = &body[start..];
        // The whole text after `=`, as the systems' own readers return it (Windows'
        // `GetPrivateProfileString`, Python's `configparser`): a `;` or `#` later in the line is
        // part of the value, never a comment that hides what follows (fifth review, finding 4).
        // Quotes are taken off only when they hold the whole value.
        let whole = tail.trim_end();
        let after = &tail[whole.len()..];
        if whole.len() >= 2
            && whole.starts_with('"')
            && whole.ends_with('"')
            && !whole[1..whole.len() - 1].contains('"')
        {
            return Some(IniLine {
                before: &body[..start],
                key,
                value: &whole[1..whole.len() - 1],
                quoted: true,
                after,
            });
        }
        Some(IniLine {
            before: &body[..start],
            key,
            value: whole,
            quoted: false,
            after,
        })
    }

    fn with_value(&self, v: &str) -> String {
        let q = if self.quoted { "\"" } else { "" };
        format!("{}{q}{v}{q}{}", self.before, self.after)
    }
}

/// A leaf's name with its parent's (`window.width`), skipping list positions.
fn context_name(path: &str, name: &str) -> String {
    let parts: Vec<&str> = path
        .split('/')
        .filter(|p| !p.bytes().all(|b| b.is_ascii_digit()))
        .collect();
    match parts.len() {
        0 | 1 => name.to_string(),
        n => format!("{}.{}", parts[n - 2], parts[n - 1]),
    }
}

fn xml_escape(s: &str) -> String {
    let mut out = String::with_capacity(s.len());
    for c in s.chars() {
        match c {
            '&' => out.push_str("&amp;"),
            '<' => out.push_str("&lt;"),
            '>' => out.push_str("&gt;"),
            '"' => out.push_str("&quot;"),
            '\'' => out.push_str("&apos;"),
            c => out.push(c),
        }
    }
    out
}

/// A value between systems through a catalogue table ([`Rule::Translate`]): from the old
/// system's column to the new one's. With `keys`, each key name of a shortcut on its own (split
/// at spaces and `+`, which are kept), so the table's rows swap modifiers both ways
/// (`ctrl` and `cmd`, `win` and `ctrl`) and no two old shortcuts become one. A name the table
/// does not hold stays as it is.
pub fn translate(t: &Table, value: &str, from: System, to: System, keys: bool) -> String {
    let (f, n) = (from.column(), to.column());
    if f == n {
        return value.to_string();
    }
    let one = |k: &str| t.lookup(f, k, n, false).unwrap_or(k).to_string();
    if !keys {
        return one(value);
    }
    let mut out = String::with_capacity(value.len());
    let mut word = String::new();
    for c in value.chars() {
        if c == ' ' || c == '+' {
            out.push_str(&one(&word));
            word.clear();
            out.push(c);
        } else {
            word.push(c);
        }
    }
    out.push_str(&one(&word));
    out
}

/// JSON with comments and trailing commas, as plain JSON. `None` for an unclosed string or
/// comment.
pub fn strip_jsonc(s: &str) -> Option<String> {
    let no_comments = strip_comments(s)?;
    // A comma before `}` or `]` (after blanks) is dropped; strings are left alone.
    let b: Vec<char> = no_comments.chars().collect();
    let mut out = String::with_capacity(no_comments.len());
    let mut i = 0;
    while i < b.len() {
        let c = b[i];
        if c == '"' {
            i = copy_string(&b, i, &mut out)?;
            continue;
        }
        if c == ',' {
            let mut j = i + 1;
            while j < b.len() && b[j].is_whitespace() {
                j += 1;
            }
            if matches!(b.get(j), Some('}' | ']')) {
                i += 1;
                continue;
            }
        }
        out.push(c);
        i += 1;
    }
    Some(out)
}

/// Copies the string starting at `i` (its opening quote) into `out`; the index after it.
fn copy_string(b: &[char], mut i: usize, out: &mut String) -> Option<usize> {
    out.push(b[i]);
    i += 1;
    loop {
        let d = *b.get(i)?;
        out.push(d);
        i += 1;
        if d == '\\' {
            out.push(*b.get(i)?);
            i += 1;
        } else if d == '"' {
            return Some(i);
        }
    }
}

fn strip_comments(s: &str) -> Option<String> {
    let b: Vec<char> = s.chars().collect();
    let mut out = String::with_capacity(s.len());
    let mut i = 0;
    while i < b.len() {
        let c = b[i];
        if c == '"' {
            i = copy_string(&b, i, &mut out)?;
            continue;
        }
        if c == '/' && b.get(i + 1) == Some(&'/') {
            while i < b.len() && b[i] != '\n' {
                i += 1;
            }
            continue;
        }
        if c == '/' && b.get(i + 1) == Some(&'*') {
            i += 2;
            loop {
                if i + 1 >= b.len() {
                    return None;
                }
                if b[i] == '*' && b[i + 1] == '/' {
                    i += 2;
                    break;
                }
                i += 1;
            }
            out.push(' ');
            continue;
        }
        out.push(c);
        i += 1;
    }
    Some(out)
}

/// VS Code's recent list (`history.recentlyOpenedPathsList`, kept in `state.vscdb`, which the
/// private-data role writes): the same list with entries pointed at the moved files, and those
/// whose files did not move left out. 2.4 gives 2.5 this value; it never opens the database.
pub fn rewrite_recent_list(json: &str, ctx: &FileCtx<'_>) -> Result<(String, Vec<Left>), Why> {
    let no_tables = BTreeMap::new();
    let mut w = Walker {
        tables: &no_tables,
        ctx,
        store: "recent".into(),
        rules: Vec::new(),
        default: Rule::Copy,
        macros: BTreeMap::new(),
        rewritten: 0,
        left: Vec::new(),
        equivalent: false,
    };
    nesting::check(json, Syntax::Json).map_err(|_| Why::TooDeep)?;
    let v: Value = serde_json::from_str(json).map_err(json_why)?;
    let v = w.value(v, "", "recent", 0).ok_or(Why::Kind)?;
    Ok((serde_json::to_string(&v).map_err(|_| Why::Kind)?, w.left))
}

/// Whether the value stored under `key` in the file at `rel` (inside `root`) is a list of recent
/// files or workspaces the catalogue names: the private-data role, which reads such stores, asks
/// this before handing the value to [`rewrite_recent_list`].
pub fn recent_list_for(cat: &Catalogue, root: SettingsRoot, rel: &str, key: &str) -> bool {
    cat.apps
        .iter()
        .flat_map(|a| &a.recent)
        .any(|r| r.root == root && r.key == key && path_matches(&r.path, rel))
}

/// A document of the person's with its folder paths pointed at their new places.
#[derive(Debug, Clone, PartialEq)]
pub struct Document {
    pub bytes: Vec<u8>,
    pub rewritten: usize,
    /// Paths left as the person wrote them, and why.
    pub left: Vec<Left>,
}

/// Why a path in a document was not changed though it moved: it is written in more than one
/// spelling, so changing it by its text alone could change something else.
pub const SPELLED_TWICE: &str =
    "this place is written in more than one way in the file: change it there yourself";

/// The person's own document (a VS Code `.code-workspace`, by the endings the catalogue names)
/// with the folder paths at the catalogue's fields pointed at where the move put them, inside
/// the person's home on the new laptop. It is the person's file, so nothing in it is removed: a
/// relative path moves with the file, and a path that did not move, or points at another
/// computer, stays as written and is reported. Only the path's own text is replaced, so comments
/// and layout stay. `None` when the catalogue names no such document.
pub fn rewrite_document(
    cat: &Catalogue,
    name: &str,
    bytes: &[u8],
    ctx: &FileCtx<'_>,
) -> Option<Result<Document, Why>> {
    let lower = name.to_ascii_lowercase();
    let doc = cat.apps.iter().flat_map(|a| &a.documents).find(|d| {
        lower.len() > d.ending.len() && lower.ends_with(&d.ending.to_ascii_lowercase())
    })?;
    Some(rewrite_doc(&doc.at, name, bytes, ctx))
}

fn rewrite_doc(
    at: &[String],
    name: &str,
    bytes: &[u8],
    ctx: &FileCtx<'_>,
) -> Result<Document, Why> {
    if u64::try_from(bytes.len()).map_or(true, |n| n > MAX_FILE) {
        return Err(Why::TooLarge);
    }
    let text = std::str::from_utf8(bytes).map_err(|_| Why::Kind)?;
    nesting::check(text, Syntax::Jsonc).map_err(|_| Why::TooDeep)?;
    let clean = strip_jsonc(text.trim_start_matches('\u{feff}')).ok_or(Why::Kind)?;
    let v: Value = serde_json::from_str(&clean).map_err(json_why)?;
    // Every text at one of the document's path fields.
    let mut found: Vec<(String, String, String)> = Vec::new();
    fn walk(
        v: &Value,
        path: &str,
        key: &str,
        at: &[String],
        out: &mut Vec<(String, String, String)>,
    ) {
        let join = |k: &str| {
            if path.is_empty() {
                k.to_string()
            } else {
                format!("{path}/{k}")
            }
        };
        match v {
            Value::Object(m) => m.iter().for_each(|(k, c)| walk(c, &join(k), k, at, out)),
            Value::Array(a) => a
                .iter()
                .enumerate()
                .for_each(|(i, c)| walk(c, &join(&i.to_string()), key, at, out)),
            Value::String(s) if at.iter().any(|p| path_matches(p, path)) => {
                out.push((path.to_string(), key.to_string(), s.clone()))
            }
            _ => {}
        }
    }
    walk(&v, "", "", at, &mut found);
    let no_tables = BTreeMap::new();
    let mut w = Walker {
        tables: &no_tables,
        ctx,
        store: name.to_string(),
        rules: Vec::new(),
        default: Rule::Copy,
        macros: BTreeMap::new(),
        rewritten: 0,
        left: Vec::new(),
        equivalent: false,
    };
    let mut changes: Vec<(String, String, String)> = Vec::new();
    for (path, key, old) in &found {
        match w.judge(key, &Value::String(old.clone())) {
            Leaf::Keep => {}
            Leaf::Replace(Value::String(new)) => changes.push((path.clone(), old.clone(), new)),
            Leaf::Replace(_) => w.leave(path, Why::Kind),
            Leaf::Remove(why) => w.leave(path, why),
        }
    }
    let mut out = text.to_string();
    let mut rewritten = 0;
    for (path, old, new) in &changes {
        let lit = serde_json::to_string(old).map_err(|_| Why::Kind)?;
        let same_value = found.iter().filter(|(_, _, s)| s == old).count();
        // Every place this text is written must be one of the path fields being changed.
        if text.matches(&lit).count() != same_value
            || changes.iter().filter(|(_, o, _)| o == old).count() != same_value
        {
            w.leave(path, Why::Unreadable(SPELLED_TWICE.into()));
            continue;
        }
        if out.contains(&lit) {
            out = out.replace(&lit, &serde_json::to_string(new).map_err(|_| Why::Kind)?);
        }
        rewritten += 1;
    }
    Ok(Document {
        bytes: out.into_bytes(),
        rewritten,
        left: w.left,
    })
}
