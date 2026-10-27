//! The one judging pipeline (fifth review, findings 1 to 3): every value of every kind passes
//! through it before it is planned and again, on the new laptop, before it is written.
//!
//! 1. **Decode** ([`texts`]): every value yields the texts a program reading it would see,
//!    whatever its kind: a string; a list item by item; raw bytes (a registry binary value, a
//!    property list's data, a GVariant byte array) as every printable run of four or more
//!    characters read as UTF-8 and as UTF-16 in both byte orders and both alignments; a
//!    composite value (a dictionary, a list, a tuple) every text inside it. A value that cannot
//!    be decoded is "found, not moved" with its reason, never copied.
//! 2. **Normalise** ([`normalise`]): quotes around a text, Windows' path spellings that name the
//!    same file (trailing dots and spaces, `\\?\` and `\\.\` prefixes), letter case where the
//!    system ignores it.
//! 3. **Judge** ([`crate::classify::danger`]): an address is anything the WHATWG URL parser
//!    reads with a host, or any scheme-shaped word (`shell:startup`, `ms-settings:`): no list of schemes, because the
//!    shell opens names no registry lists (`shell:` is one); only a scheme followed by nothing
//!    or a number (`Arial:12`) is plain text, and only when the new laptop says no handler is
//!    registered for it. Whether a path can run is the new laptop's own facts ([`Facts`]), never
//!    a list in code and never only the catalogue: on Windows the endings `PATHEXT` names and
//!    every file type whose registered verbs run the file itself, take a command line, are
//!    shortcuts, or that Windows itself keeps out of recent documents; everywhere, the file
//!    itself when it is there (a program's header, a script's `#!`, an execute bit, an app
//!    bundle, a desktop entry). Catalogue action-hint rows can only add caution on top.

use std::collections::BTreeSet;
use std::sync::OnceLock;

use pctwin_recipe::SettingValue;

use crate::classify::Why;

/// What the new laptop itself says can run or open a handler.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct Facts {
    /// File endings (lowercase, without the dot) that run, install, load or act when opened.
    runs: BTreeSet<String>,
    /// Schemes with a handler registered (lowercase); `None` when the system's handlers could
    /// not be read, and every scheme is then treated as one that opens something.
    schemes: Option<BTreeSet<String>>,
    /// Whether a path's own file is looked at when it is there.
    look_at_files: bool,
}

impl Facts {
    /// Facts given directly (tests, and a system whose facts were read elsewhere). Files are
    /// looked at too.
    pub fn new<'a>(runs: impl IntoIterator<Item = &'a str>, schemes: Option<Vec<&'a str>>) -> Self {
        Self {
            runs: runs
                .into_iter()
                .map(|e| e.trim_start_matches('.').to_ascii_lowercase())
                .filter(|e| !e.is_empty())
                .collect(),
            schemes: schemes.map(|s| s.into_iter().map(str::to_ascii_lowercase).collect()),
            look_at_files: true,
        }
    }

    /// This laptop's own facts, read once.
    pub fn system() -> &'static Facts {
        static FACTS: OnceLock<Facts> = OnceLock::new();
        FACTS.get_or_init(discover)
    }

    /// Whether a file ending (with or without its dot) runs or acts when opened.
    pub fn ending_runs(&self, ending: &str) -> bool {
        let e = ending.trim_start_matches('.').to_ascii_lowercase();
        !e.is_empty() && self.runs.contains(&e)
    }

    /// Whether the system has a handler for `scheme`: `None` when it cannot say.
    pub fn handler(&self, scheme: &str) -> Option<bool> {
        self.schemes
            .as_ref()
            .map(|s| s.contains(&scheme.to_ascii_lowercase()))
    }

    /// How many endings the facts hold (a check that discovery found something).
    pub fn endings(&self) -> usize {
        self.runs.len()
    }
}

#[cfg(windows)]
fn discover() -> Facts {
    windows_facts::read()
}

#[cfg(all(unix, not(target_os = "macos")))]
fn discover() -> Facts {
    Facts {
        runs: BTreeSet::new(),
        schemes: unix_facts::desktop_schemes(),
        look_at_files: true,
    }
}

#[cfg(any(target_os = "macos", not(any(windows, unix))))]
fn discover() -> Facts {
    // The Mac's own handler and type knowledge (Launch Services) is not read without a system
    // call PCTwin does not make: every scheme is cautious, and a path is judged by its file.
    Facts {
        runs: BTreeSet::new(),
        schemes: None,
        look_at_files: true,
    }
}

#[cfg(windows)]
mod windows_facts {
    use std::collections::BTreeSet;

    use winreg::RegKey;
    use winreg::enums::{HKEY_CLASSES_ROOT, HKEY_CURRENT_USER, HKEY_LOCAL_MACHINE, KEY_READ};

    use super::Facts;

    /// `FTA_AlwaysUnsafe` and `FTA_NoRecentDocs`: Windows' own marks for file types that act
    /// rather than show (registry imports, installers, consoles, connections).
    const ACTS: u32 = 0x0002_0000 | 0x0010_0000;

    pub(super) fn read() -> Facts {
        let mut runs = BTreeSet::new();
        if let Ok(p) = std::env::var("PATHEXT") {
            runs.extend(super::endings_of(&p));
        }
        for hive in [HKEY_LOCAL_MACHINE, HKEY_CURRENT_USER] {
            if let Ok(k) = RegKey::predef(hive).open_subkey_with_flags(
                r"Software\Microsoft\Windows\CurrentVersion\Policies\Associations",
                KEY_READ,
            ) {
                for name in ["HighRiskFileTypes", "ModRiskFileTypes"] {
                    if let Ok(v) = k.get_value::<String, _>(name) {
                        runs.extend(super::endings_of(&v));
                    }
                }
            }
        }
        let root = RegKey::predef(HKEY_CLASSES_ROOT);
        let mut schemes = BTreeSet::new();
        for name in root.enum_keys().flatten() {
            let Ok(key) = root.open_subkey_with_flags(&name, KEY_READ) else {
                continue;
            };
            if let Some(ending) = name.strip_prefix('.') {
                if ending.is_empty() {
                    continue;
                }
                let class = key
                    .get_value::<String, _>("")
                    .ok()
                    .and_then(|c| root.open_subkey_with_flags(c.trim(), KEY_READ).ok());
                if acts(&key) || class.as_ref().is_some_and(acts) {
                    runs.insert(ending.to_ascii_lowercase());
                }
            } else if key.get_raw_value("URL Protocol").is_ok() {
                schemes.insert(name.to_ascii_lowercase());
            }
        }
        if let Ok(h) = root.open_subkey_with_flags(r"PROTOCOLS\Handler", KEY_READ) {
            schemes.extend(h.enum_keys().flatten().map(|n| n.to_ascii_lowercase()));
        }
        Facts {
            runs,
            schemes: Some(schemes),
            look_at_files: true,
        }
    }

    /// Whether a file type's key says opening it runs or acts: a shortcut, Windows' own marks,
    /// perceived as an application or a system file, or a verb that runs the file itself or
    /// passes it a command line.
    fn acts(key: &RegKey) -> bool {
        if key.get_raw_value("IsShortcut").is_ok() {
            return true;
        }
        if let Ok(v) = key.get_raw_value("EditFlags") {
            let b = &v.bytes;
            if b.len() >= 4 && u32::from_le_bytes([b[0], b[1], b[2], b[3]]) & ACTS != 0 {
                return true;
            }
        }
        if let Ok(p) = key.get_value::<String, _>("PerceivedType")
            && matches!(p.to_ascii_lowercase().as_str(), "application" | "system")
        {
            return true;
        }
        let Ok(shell) = key.open_subkey_with_flags("shell", KEY_READ) else {
            return false;
        };
        shell.enum_keys().flatten().take(64).any(|verb| {
            shell
                .open_subkey_with_flags(format!(r"{verb}\command"), KEY_READ)
                .and_then(|c| c.get_value::<String, _>(""))
                .is_ok_and(|cmd| super::command_runs_file(&cmd))
        })
    }
}

/// Whether a registered verb's command runs the file itself (`"%1" %*`) or hands it a command
/// line (`%*`): what a program or script takes, never a document.
pub fn command_runs_file(cmd: &str) -> bool {
    let c = cmd.trim();
    let first = c.trim_start_matches('"');
    first.starts_with("%1")
        || first.starts_with("%L")
        || first.starts_with("%l")
        || c.contains("%*")
}

/// File endings in a `;`-separated list (`.COM;.EXE`).
pub fn endings_of(list: &str) -> impl Iterator<Item = String> + '_ {
    list.split(';')
        .map(|e| e.trim().trim_start_matches('.').to_ascii_lowercase())
        .filter(|e| !e.is_empty() && e.bytes().all(|b| b.is_ascii_alphanumeric() || b == b'-'))
}

#[cfg(all(unix, not(target_os = "macos")))]
mod unix_facts {
    use std::collections::BTreeSet;

    /// The schemes desktop entries say they handle (`MimeType=x-scheme-handler/<scheme>;`), in
    /// the system's and the person's application folders. `None` when none could be read.
    pub(super) fn desktop_schemes() -> Option<BTreeSet<String>> {
        let mut dirs: Vec<std::path::PathBuf> = std::env::var("XDG_DATA_DIRS")
            .unwrap_or_else(|_| "/usr/local/share:/usr/share".into())
            .split(':')
            .filter(|d| d.starts_with('/'))
            .map(|d| std::path::Path::new(d).join("applications"))
            .collect();
        if let Ok(h) = std::env::var("HOME") {
            dirs.push(std::path::Path::new(&h).join(".local/share/applications"));
        }
        let mut out = BTreeSet::new();
        let mut read_any = false;
        for d in dirs {
            let Ok(list) = std::fs::read_dir(&d) else {
                continue;
            };
            read_any = true;
            for e in list.flatten().take(5000) {
                let p = e.path();
                if p.extension().is_none_or(|x| x != "desktop") {
                    continue;
                }
                let Ok(meta) = std::fs::metadata(&p) else {
                    continue;
                };
                if !meta.is_file() || meta.len() > 256 * 1024 {
                    continue;
                }
                let Ok(text) = std::fs::read_to_string(&p) else {
                    continue;
                };
                out.extend(super::desktop_entry_schemes(&text));
            }
        }
        read_any.then_some(out)
    }
}

/// The schemes one desktop entry handles.
pub fn desktop_entry_schemes(text: &str) -> Vec<String> {
    text.lines()
        .filter_map(|l| l.trim().strip_prefix("MimeType="))
        .flat_map(|m| m.split(';'))
        .filter_map(|m| m.trim().strip_prefix("x-scheme-handler/"))
        .filter(|s| !s.is_empty())
        .map(str::to_ascii_lowercase)
        .collect()
}

/// The kind of a value, for decoding.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Kind {
    /// One text.
    Text,
    /// A list of texts carried as a JSON array (a registry multi-string).
    List,
    /// Raw bytes carried as hex (a registry binary value).
    Bytes,
    /// A property list value carried as one line of XML.
    Plist,
    /// GVariant text (a dconf value).
    GVariant,
}

/// The fewest characters a printable run in raw bytes must have to be judged.
pub const RUN: usize = 4;

/// Every text inside a value of `kind`, as a program reading it would see it. A value that cannot
/// be decoded is refused with its reason (never copied unjudged).
pub fn texts(kind: Kind, v: &SettingValue) -> Result<Vec<String>, Why> {
    let SettingValue::Text(t) = v else {
        return Ok(Vec::new());
    };
    Ok(match kind {
        Kind::Text => vec![t.clone()],
        Kind::List => serde_json::from_str::<Vec<String>>(t).map_err(|_| Why::Kind)?,
        Kind::Bytes => runs(&hex(t).ok_or(Why::Kind)?),
        Kind::Plist => {
            let p = crate::plist::from_xml(t).map_err(|_| Why::Kind)?;
            let mut out = crate::plist::strings(&p);
            for d in crate::plist::data(&p) {
                out.extend(runs(&d));
            }
            out
        }
        Kind::GVariant => match crate::found::dconf_string(t) {
            Some(inner) => vec![inner],
            None => {
                pctwin_types::nesting::check(t, pctwin_types::nesting::Syntax::GVariant)
                    .map_err(|_| Why::TooDeep)?;
                let mut out = crate::gvariant::strings(t).ok_or(Why::Kind)?;
                out.extend(runs(&crate::gvariant::bytes(t)));
                out
            }
        },
    })
}

/// Lowercase or uppercase hex as bytes.
pub fn hex(t: &str) -> Option<Vec<u8>> {
    let t = t.trim();
    if !t.len().is_multiple_of(2) {
        return None;
    }
    (0..t.len())
        .step_by(2)
        .map(|i| u8::from_str_radix(t.get(i..i + 2)?, 16).ok())
        .collect()
}

/// Every printable run of [`RUN`] or more characters in `b`, read as UTF-8 and as UTF-16 (little-
/// and big-endian, from either byte), each once.
pub fn runs(b: &[u8]) -> Vec<String> {
    let mut out: Vec<String> = Vec::new();
    let mut push = |s: &mut String| {
        if s.chars().count() >= RUN && !out.contains(s) {
            out.push(std::mem::take(s));
        } else {
            s.clear();
        }
    };
    let printable = |c: char| !c.is_control() && c != '\u{fffd}' && c != '\u{feff}';
    let mut cur = String::new();
    for c in String::from_utf8_lossy(b).chars() {
        if printable(c) {
            cur.push(c);
        } else {
            push(&mut cur);
        }
    }
    push(&mut cur);
    for start in [0usize, 1] {
        for big in [false, true] {
            let units = b
                .get(start..)
                .unwrap_or_default()
                .as_chunks::<2>()
                .0
                .iter()
                .map(|p| {
                    if big {
                        u16::from_be_bytes(*p)
                    } else {
                        u16::from_le_bytes(*p)
                    }
                });
            for c in char::decode_utf16(units) {
                match c {
                    Ok(c) if printable(c) => cur.push(c),
                    _ => push(&mut cur),
                }
            }
            push(&mut cur);
        }
    }
    out
}

/// A text as it names a place: trimmed, without the quotes around it, a Windows path without
/// the `\\?\` or `\\.\` prefix (`\\?\UNC\server` is `\\server`).
pub fn normalise(t: &str) -> String {
    let mut s = t.trim();
    loop {
        let inner = s
            .strip_prefix('"')
            .and_then(|r| r.strip_suffix('"'))
            .or_else(|| s.strip_prefix('\'').and_then(|r| r.strip_suffix('\'')));
        match inner {
            Some(i) if s.len() >= 2 => s = i.trim(),
            _ => break,
        }
    }
    let lower = s.get(..8).map(str::to_ascii_lowercase).unwrap_or_default();
    if lower.starts_with(r"\\?\unc\") {
        return format!(r"\\{}", &s[8..]);
    }
    for p in [r"\\?\", r"\\.\", "//?/", "//./"] {
        if let Some(rest) = s.strip_prefix(p)
            && is_drive(rest.as_bytes())
        {
            return rest.to_string();
        }
    }
    s.to_string()
}

pub(crate) fn is_drive(b: &[u8]) -> bool {
    b.len() >= 2 && b[0].is_ascii_alphabetic() && b[1] == b':'
}

/// The last part of a path, as Windows names the file (trailing dots and spaces dropped).
fn last_part(path: &str) -> &str {
    let name = path.rsplit(['\\', '/']).next().unwrap_or(path);
    if is_drive(path.as_bytes()) {
        name.trim_end_matches(['.', ' '])
    } else {
        name
    }
}

/// The ending of a file name (`exe` in `x.exe`), when it has one.
pub fn ending(name: &str) -> Option<&str> {
    let (stem, e) = name.rsplit_once('.')?;
    (!stem.is_empty() && !e.is_empty() && e.len() <= 32 && !e.contains([' ', ':'])).then_some(e)
}

/// Whether one full local path could run, install or load something on the new laptop (or names
/// a place PCTwin cannot check), by the new laptop's own facts. `None`: a path to data.
pub fn path_danger(facts: &Facts, path: &str) -> Option<Why> {
    let windows = is_drive(path.as_bytes());
    let mut path = path.to_string();
    if windows {
        // `x.exe::$DATA`, `notes.txt:hidden.exe`: a file's streams are never a setting's data.
        let after = &path[2..];
        if let Some(i) = after.find(':') {
            let name = last_part(&path[..2 + i]);
            let streams = &after[i + 1..];
            let runs = std::iter::once(name)
                .chain(streams.split(':'))
                .filter_map(ending)
                .any(|e| facts.ending_runs(e));
            return Some(if runs {
                Why::StartsProgram
            } else {
                Why::PlaceUnknown
            });
        }
        // A short (8.3) name hides the real ending: the long name is used when the system can
        // say it, otherwise PCTwin cannot check the place.
        if path.split(['\\', '/']).any(|p| {
            p.split_once('~')
                .is_some_and(|(_, r)| r.starts_with(|c: char| c.is_ascii_digit()))
        }) {
            match std::fs::canonicalize(&path) {
                Ok(long) => {
                    let l = long.to_string_lossy().to_string();
                    path = normalise(&l);
                }
                Err(_) => return Some(Why::PlaceUnknown),
            }
        }
    }
    if ending(last_part(&path)).is_some_and(|e| facts.ending_runs(e)) {
        return Some(Why::StartsProgram);
    }
    if facts.look_at_files && file_runs(&path) {
        return Some(Why::StartsProgram);
    }
    None
}

/// Whether the file at `path`, when it is here, is something to run: a program's or library's
/// header, a script's `#!`, a desktop entry, an execute bit, an app bundle. A file kept online
/// only is never opened (that would fetch it).
pub fn file_runs(path: &str) -> bool {
    let Ok(meta) = std::fs::metadata(path) else {
        return false;
    };
    #[cfg(windows)]
    {
        use std::os::windows::fs::MetadataExt;
        // Offline, recall-on-open, recall-on-data-access.
        if meta.file_attributes() & (0x1000 | 0x4_0000 | 0x40_0000) != 0 {
            return false;
        }
    }
    if meta.is_dir() {
        return std::path::Path::new(path)
            .join("Contents")
            .join("Info.plist")
            .is_file();
    }
    if !meta.is_file() {
        return false;
    }
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        if meta.permissions().mode() & 0o111 != 0 {
            return true;
        }
    }
    let mut head = [0u8; 64];
    let n = match std::fs::File::open(path) {
        Ok(mut f) => std::io::Read::read(&mut f, &mut head).unwrap_or(0),
        Err(_) => return false,
    };
    runnable_header(&head[..n])
}

/// The first bytes of a program, a library or a script: what the systems' loaders run.
pub fn runnable_header(h: &[u8]) -> bool {
    let h = h.strip_prefix(&[0xef, 0xbb, 0xbf]).unwrap_or(h);
    h.starts_with(b"MZ")
        || h.starts_with(b"\x7fELF")
        || h.starts_with(b"#!")
        || [
            [0xfe, 0xed, 0xfa, 0xce],
            [0xfe, 0xed, 0xfa, 0xcf],
            [0xce, 0xfa, 0xed, 0xfe],
            [0xcf, 0xfa, 0xed, 0xfe],
            [0xca, 0xfe, 0xba, 0xbe],
        ]
        .iter()
        .any(|m| h.starts_with(m))
        || String::from_utf8_lossy(h)
            .trim_start()
            .starts_with("[Desktop Entry]")
}

/// Whether a word ends the way a program does on the new laptop.
pub fn word_runs(facts: &Facts, word: &str) -> bool {
    ending(last_part(word.trim_end_matches(['\\', '/']))).is_some_and(|e| facts.ending_runs(e))
}

/// The scheme of what may be an address (`https` in `https:x`): a letter, then letters, digits,
/// `+`, `.` or `-`, at least two characters (one letter is a drive), then a colon.
pub fn scheme(t: &str) -> Option<(&str, &str)> {
    let (s, rest) = t.split_once(':')?;
    let ok = s.len() >= 2
        && s.len() <= 32
        && s.as_bytes()[0].is_ascii_alphabetic()
        && s.bytes()
            .all(|b| b.is_ascii_alphanumeric() || matches!(b, b'+' | b'.' | b'-'));
    ok.then_some((s, rest))
}

/// Whether a word is an address: one the URL parser reads with a host, or a scheme followed by
/// a name (`shell:startup`); a scheme followed by nothing or a number only when the new laptop
/// has, or may have, a handler for it.
pub fn address(facts: &Facts, word: &str) -> Option<Why> {
    if word.starts_with(r"\\") || word.starts_with("//") {
        return Some(Why::Fetches);
    }
    let (s, rest) = scheme(word)?;
    if s.eq_ignore_ascii_case("file") {
        return None;
    }
    if let Ok(u) = url::Url::parse(word)
        && u.host_str().is_some_and(|h| !h.is_empty())
    {
        return Some(Why::Fetches);
    }
    let plain =
        rest.is_empty() || rest.starts_with(|c: char| c.is_ascii_digit() || c.is_whitespace());
    match (plain, facts.handler(s)) {
        (true, Some(false)) => None,
        (_, Some(true)) => Some(Why::Fetches),
        _ => Some(Why::PlaceUnknown),
    }
}
