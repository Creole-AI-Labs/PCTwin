//! What may move: generic rules by KIND of value, never by app name (the user's decision, 27
//! October 2026: "different people have different things").
//!
//! Every setting found on the old laptop passes [`classify`] before it can be planned. It is
//! left behind, with the reason, when it is (in this order):
//! - in a store for security, privacy or organisation policy, or one that starts programs
//!   (start-up entries, file and protocol handlers, the environment): never copied;
//! - a credential or token: by its name (`password`, `token`, `apiKey`, ...) or by its form
//!   (Windows DPAPI blobs, private keys, JSON web tokens, well-known token prefixes, long random
//!   strings);
//! - a machine, device or install identity, or the old account's own identity (a Windows SID);
//! - a cache;
//! - a window's position or size (the new laptop's screens differ), unless the screens match;
//! - a hardware path (a device instance, `/dev/...`);
//! - anything that could make the new laptop run or fetch something ([`run_or_fetch`], the
//!   fourth review's finding F1): a value of a key whose name or store marks something to run
//!   or load (commands, handlers, hooks, proxies, editors, shells, screensavers; cursors, icons,
//!   sounds), a command line, a program's file, or a path or address in any spelling that is not
//!   one full local path (another computer, a web or app address, an environment variable, a
//!   relative place): kept for the person to look at, never written;
//! - one full local path to data: rewritten through the move's folder map when it points at
//!   something moved, otherwise left out (a recent-files entry whose file did not move is left
//!   out the same way);
//! - too large, or holding hidden characters.
//!
//! Everything else is copied as it is.

use pctwin_recipe::{SettingValue, text_ok};

use crate::apps::glob;
use crate::catalogue::ActionHint;
use crate::value::full_path;

/// Why a found setting was not moved. Shown to the person in plain words ([`Why::words`]);
/// the value itself is never shown or logged.
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum Why {
    Security,
    Policy,
    StartsProgram,
    Secret,
    MachineId,
    AccountBound,
    Cache,
    WindowPlace,
    HardwarePath,
    PathNotMoved,
    RecentMissing,
    TooLarge,
    HiddenText,
    /// It points at another computer or a web address (opening it would fetch something, and
    /// can hand the old laptop's sign-in to whoever runs that computer).
    Fetches,
    /// It points at a place PCTwin cannot check (written with an environment variable, or
    /// relative to somewhere).
    PlaceUnknown,
    /// Nested deeper than any parser is let read ([`pctwin_types::nesting`]).
    TooDeep,
    /// Past the most settings one discovery reads.
    TooMany,
    /// A registry key's unnamed (default) value.
    Unnamed,
    /// A kind of value PCTwin does not copy (dates, raw data in preferences).
    Kind,
    /// Moving between different systems needs a known translation (a catalogue row).
    OtherSystem,
    /// A store the system itself owns: moved only through known settings.
    SystemStore,
    /// The catalogue says this one must not move, and why.
    Exception(String),
    Unreadable(String),
}

impl Why {
    pub fn words(&self) -> String {
        match self {
            Why::Security => "a security or privacy setting: never copied; the new laptop keeps its own".into(),
            Why::Policy => "set by an organisation: never copied".into(),
            Why::StartsProgram => "it would start a program: kept for you to look at, not switched on".into(),
            Why::Secret => "it looks like a password or sign-in: never copied; sign in again on the new laptop".into(),
            Why::MachineId => "it identifies the old laptop itself".into(),
            Why::AccountBound => "it belongs to the old laptop's account".into(),
            Why::Cache => "a cache: the app makes it again".into(),
            Why::WindowPlace => "a window's place on the old laptop's screens".into(),
            Why::HardwarePath => "it points at a device on the old laptop".into(),
            Why::PathNotMoved => "it points at a file or folder that was not moved".into(),
            Why::RecentMissing => "a recent file that was not moved".into(),
            Why::TooLarge => "too large to move as a setting".into(),
            Why::HiddenText => "it holds hidden characters".into(),
            Why::Fetches => "it points at another computer or a web address: kept for you to look at, not written".into(),
            Why::PlaceUnknown => "it points at a place PCTwin cannot check: kept for you to look at, not written".into(),
            Why::TooDeep => pctwin_types::nesting::TOO_DEEP.into(),
            Why::TooMany => "found, not read: there were too many settings here to read them all".into(),
            Why::Unnamed => "found, not moved: a key's unnamed value, which PCTwin does not move yet".into(),
            Why::Kind => "a kind of value PCTwin does not copy".into(),
            Why::OtherSystem => "the new laptop's system keeps this differently, and PCTwin does not know how to translate it yet".into(),
            Why::SystemStore => "the system's own settings: only the ones PCTwin knows move".into(),
            Why::Exception(r) => r.clone(),
            Why::Unreadable(r) => r.clone(),
        }
    }
}

/// What happens to one found value.
#[derive(Debug, Clone, PartialEq)]
pub enum Fate {
    Copy(SettingValue),
    /// Copied with its paths pointed at the moved files.
    Rewritten(SettingValue),
    NotMoved(Why),
}

/// Where moved folders went: old folder to new folder.
pub trait Relocate {
    /// The new path for `old`, when it is inside something the move brought.
    fn new_path(&self, old: &str) -> Option<String>;
}

/// The move's folder map as pairs of full paths (old folder, new folder).
#[derive(Debug, Clone, Default)]
pub struct FolderMap(pub Vec<(String, String)>);

impl Relocate for FolderMap {
    fn new_path(&self, old: &str) -> Option<String> {
        if !full_path(old) {
            return None;
        }
        let windows = old.as_bytes().get(1) == Some(&b':');
        self.0.iter().find_map(|(from, to)| {
            let from = from.trim_end_matches(['\\', '/']);
            let to = to.trim_end_matches(['\\', '/']);
            let head = old.get(..from.len())?;
            let same = if windows {
                head.eq_ignore_ascii_case(from)
            } else {
                head == from
            };
            let rest = &old[from.len()..];
            if !same || !(rest.is_empty() || rest.starts_with(['\\', '/'])) {
                return None;
            }
            // The new laptop's separator.
            let sep = if to.as_bytes().get(1) == Some(&b':') {
                '\\'
            } else {
                '/'
            };
            let rest: String = rest
                .chars()
                .map(|c| if c == '\\' || c == '/' { sep } else { c })
                .collect();
            Some(format!("{to}{rest}"))
        })
    }
}

/// What the classifier knows about the move.
pub struct Ctx<'a> {
    pub paths: &'a dyn Relocate,
    /// The new laptop's screens are the old laptop's (window places may move).
    pub same_screens: bool,
    /// The catalogue's action-hint rows ([`crate::catalogue::Catalogue::action_hints`]): they
    /// only add caution to the general rule, which keeps back what it can see without them.
    pub hints: &'a [ActionHint],
}

/// The longest text a setting may carry ([`pctwin_types::limits`]).
pub use pctwin_types::limits::SETTING_TEXT_BYTES as MAX_VALUE;

/// Store paths (registry keys, preference domains, dconf paths; lowercase, `\` or `/` or `.`
/// separated) that hold security or privacy settings.
const SECURITY_STORES: &[&str] = &[
    "capabilityaccessmanager",
    "internet settings\\zones",
    "internet settings\\zonemap",
    "systemcertificates",
    "cryptography",
    "protected storage",
    "credentials",
    "com.apple.security",
    "com.apple.tcc",
    "com.apple.alf",
    "com.apple.systempolicy",
    "com.apple.keychainaccess",
    "/org/gnome/desktop/privacy",
    "/org/gnome/system/proxy",
    "/org/gnome/desktop/screensaver",
];

const POLICY_STORES: &[&str] = &[
    "\\policies",
    "software\\policies",
    "/org/gnome/desktop/lockdown",
    "com.apple.mdm",
    "managed preferences",
];

const PROGRAM_STORES: &[&str] = &[
    "\\run",
    "\\runonce",
    "software\\classes",
    "\\shell\\",
    "\\command",
    "environment",
    "winlogon",
    "image file execution options",
    "command processor",
    "explorer\\fileexts",
    "shell\\associations",
    "com.apple.loginwindow",
    "com.apple.loginitems",
    "com.apple.backgrounditems",
    "com.apple.launchservices",
    "/org/gnome/settings-daemon/plugins/media-keys",
];

/// The kind of store a place is, by its path: `None` for an ordinary one.
pub fn store_kind(store: &str) -> Option<Why> {
    let s = store.to_lowercase();
    let has = |list: &[&str]| {
        list.iter().any(|p| {
            s == p.trim_start_matches('\\')
                || s.starts_with(&format!("{}\\", p.trim_start_matches('\\')))
                || s.starts_with(&format!("{}/", p.trim_end_matches('/')))
                || s.starts_with(&format!("{}.", p))
                || (p.starts_with('\\') && (s.contains(&format!("{p}\\")) || s.ends_with(p)))
                || (p.starts_with('/') && s.starts_with(p))
                || (!p.starts_with(['\\', '/']) && s.contains(&format!("\\{p}")))
        })
    };
    if has(POLICY_STORES) {
        Some(Why::Policy)
    } else if has(SECURITY_STORES) {
        Some(Why::Security)
    } else if has(PROGRAM_STORES) {
        Some(Why::StartsProgram)
    } else if s
        .split(['\\', '/'])
        .any(|p| matches!(p, "cache" | "caches" | "temp" | "tmp"))
    {
        Some(Why::Cache)
    } else {
        None
    }
}

/// The words of a name: split at anything not a letter or digit and at lower-to-upper case
/// changes (`MachineGuid` is `machine`, `guid`).
pub fn words(name: &str) -> Vec<String> {
    let mut out = Vec::new();
    let mut cur = String::new();
    let mut prev_lower = false;
    for c in name.chars() {
        if !c.is_alphanumeric() {
            if !cur.is_empty() {
                out.push(std::mem::take(&mut cur));
            }
            prev_lower = false;
            continue;
        }
        if c.is_uppercase() && prev_lower && !cur.is_empty() {
            out.push(std::mem::take(&mut cur));
        }
        prev_lower = c.is_lowercase() || c.is_ascii_digit();
        cur.extend(c.to_lowercase());
    }
    if !cur.is_empty() {
        out.push(cur);
    }
    out
}

const SECRET_WORDS: &[&str] = &[
    "password",
    "passwd",
    "pwd",
    "passphrase",
    "secret",
    "secrets",
    "token",
    "tokens",
    "apikey",
    "credential",
    "credentials",
    "cookie",
    "cookies",
    "auth",
    "authorization",
    "bearer",
    "otp",
];
const SECRET_JOINED: &[&str] = &[
    "apikey",
    "accesskey",
    "privatekey",
    "secretkey",
    "clientsecret",
    "sessionkey",
    "refreshtoken",
    "accesstoken",
    "authtoken",
    "signingkey",
    "encryptionkey",
];
const ID_WORDS: &[&str] = &["id", "guid", "uuid", "serial", "fingerprint", "hwid"];
const MACHINE_WORDS: &[&str] = &[
    "machine",
    "device",
    "install",
    "installation",
    "hardware",
    "client",
    "instance",
    "computer",
    "host",
    "anonymous",
    "tracking",
    "telemetry",
    "system",
];
const ACCOUNT_WORDS: &[&str] = &["user", "account", "sid", "owner"];
const CACHE_WORDS: &[&str] = &[
    "cache",
    "caches",
    "cached",
    "thumbnail",
    "thumbnails",
    "tmp",
    "temp",
];
const PLACE_LAST: &[&str] = &[
    "placement",
    "position",
    "pos",
    "rect",
    "bounds",
    "geometry",
    "left",
    "top",
    "width",
    "height",
    "maximized",
    "maximised",
    "x",
    "y",
    "monitor",
    "screen",
    "dpi",
    "size",
    "state",
];
const PLACE_OWNER: &[&str] = &[
    "window", "wnd", "win", "frame", "dialog", "dlg", "form", "main", "panel", "pane", "sidebar",
    "toolbar", "splitter",
];
const RECENT_WORDS: &[&str] = &[
    "mru",
    "recent",
    "history",
    "lastopened",
    "lastused",
    "lastfile",
];
/// What a key's name or store says its value is for.
pub use crate::catalogue::HintKind as Action;

/// Whether a key's name has the shape of a file's name (`SCRNSAVE.EXE`, `x.cur`): a stem with a
/// letter in it, a dot, and a short ending of letters and digits with a letter in it. Such a key
/// names the file the system uses, so a path in it is kept back. Pure shape: no list of names.
pub fn file_shaped_name(name: &str) -> bool {
    let Some((stem, ending)) = name.rsplit_once('.') else {
        return false;
    };
    let letter = |s: &str| s.bytes().any(|b| b.is_ascii_alphabetic());
    !stem.is_empty()
        && !stem.contains(['\\', '/', ' '])
        && letter(stem)
        && (1..=4).contains(&ending.len())
        && ending.bytes().all(|b| b.is_ascii_alphanumeric())
        && letter(ending)
}

/// Whether the key `name` in `store` holds something to run or load. The general rule is the
/// name's shape ([`file_shaped_name`]); the catalogue's action-hint rows only add caution
/// (screensavers, cursors, commands, handlers, proxies ...), never take it away.
pub fn action_kind(hints: &[ActionHint], store: &str, name: &str) -> Option<Action> {
    let mut kind = file_shaped_name(name).then_some(Action::File);
    for h in hints.iter().filter(|h| h.ending.is_none()) {
        if glob(&h.store, store) && glob(&h.name, name) {
            if h.kind == Action::Run {
                return Some(Action::Run);
            }
            kind = Some(Action::File);
        }
    }
    kind
}

/// Whether a word ends the way the catalogue's hints say programs end (`.exe`, `.sh` ...).
fn program_end(hints: &[ActionHint], token: &str) -> bool {
    let l = token.trim_end_matches(['\\', '/']).to_ascii_lowercase();
    hints.iter().filter_map(|h| h.ending.as_deref()).any(|e| {
        let e = e.to_ascii_lowercase();
        l.len() > e.len() && l.ends_with(&e)
    })
}

/// A plain number: it can name nothing to run or fetch.
fn plain_number(t: &str) -> bool {
    let t = t.strip_prefix('-').unwrap_or(t);
    !t.is_empty()
        && !t.starts_with('.')
        && t.bytes().all(|b| b.is_ascii_digit() || b == b'.')
        && t.bytes().filter(|b| *b == b'.').count() <= 1
}

/// A path or an address found in a text, in any spelling.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Spot {
    /// Another computer (`\\server\share`, `//server/share`, `file://server/...`) or a web or
    /// app address (`https:`, `ftp:`, `mailto:`, `shell:`, `ms-settings:` ...).
    Remote,
    /// A place on this computer PCTwin cannot check: written with an environment variable
    /// (`%SystemRoot%`, `$HOME`), relative (`..\x`, `bin\x`, `~/x`), or `C:x`.
    Unchecked,
    /// A full local path (`C:\...`, `C:/...`, `/...`) or a local `file:///` address.
    Local,
}

fn is_drive(b: &[u8]) -> bool {
    b.len() >= 2 && b[0].is_ascii_alphabetic() && b[1] == b':'
}

/// The scheme of what may be an address (`https` in `https://x`), when `t` starts with one. One
/// letter is a drive; a digit after the colon is a time or a number pair (`12:30`, `Arial:12`).
fn scheme(t: &str) -> Option<&str> {
    let (s, rest) = t.split_once(':')?;
    let ok = s.len() >= 2
        && s.len() <= 32
        && s.as_bytes()[0].is_ascii_alphabetic()
        && s.bytes()
            .all(|b| b.is_ascii_alphanumeric() || matches!(b, b'+' | b'.' | b'-'));
    let next = rest.as_bytes().first();
    (ok && next.is_some_and(|c| !c.is_ascii_digit() && !c.is_ascii_whitespace())).then_some(s)
}

/// What one word of a text (or the whole text) points at.
fn spot(token: &str) -> Option<Spot> {
    let t = token.trim_matches(['"', '\'']);
    let b = t.as_bytes();
    if t.is_empty() {
        return None;
    }
    if t.starts_with("\\\\") || t.starts_with("//") {
        return Some(Spot::Remote);
    }
    if let Some(s) = scheme(t) {
        if s.eq_ignore_ascii_case("file") {
            return Some(if t[5..].starts_with("///") {
                Spot::Local
            } else {
                Spot::Remote
            });
        }
        // An address that names a place to reach: another computer (`//host`), a mailbox
        // (`@`), a path or a query. A word before a colon alone (`HH:mm:ss`, a time format) is
        // not one.
        let rest = &t[s.len() + 1..];
        if rest.starts_with("//") || rest.contains(['@', '/', '\\', '?', '=', '(']) {
            return Some(Spot::Remote);
        }
    }
    if is_drive(b) {
        return Some(if matches!(b.get(2), Some(b'\\' | b'/')) {
            Spot::Local
        } else {
            Spot::Unchecked
        });
    }
    if t.starts_with('/') && t.len() > 1 {
        return Some(Spot::Local);
    }
    let env = t.matches('%').count() >= 2 || t.contains('$');
    if env || t.starts_with('~') || t.starts_with("./") || t.starts_with("../") || t.contains('\\')
    {
        return Some(Spot::Unchecked);
    }
    None
}

/// Where a command line or a list would split a text.
fn splits(c: char) -> bool {
    c.is_whitespace()
        || matches!(
            c,
            '"' | '\''
                | ','
                | ';'
                | '='
                | '('
                | ')'
                | '['
                | ']'
                | '{'
                | '}'
                | '<'
                | '>'
                | '|'
                | '*'
                | '`'
                | '&'
        )
}

/// The words of a text, split where a command line or a list would split them.
fn tokens(t: &str) -> impl Iterator<Item = &str> {
    t.split(splits).filter(|s| !s.is_empty())
}

/// A word that is a switch of a command line (`/c`, `-c`, `--run`, `-ExecutionPolicy`).
fn switch(token: &str) -> bool {
    match token.as_bytes() {
        [b'/', c, rest @ ..] => {
            (c.is_ascii_alphabetic() || *c == b'?')
                && rest.len() <= 2
                && rest.iter().all(|x| x.is_ascii_alphanumeric() || *x == b':')
        }
        b @ [b'-', ..] => {
            let body = b.strip_prefix(b"--").unwrap_or(&b[1..]);
            body.first().is_some_and(u8::is_ascii_alphabetic)
                && body
                    .iter()
                    .all(|x| x.is_ascii_alphanumeric() || matches!(x, b'-' | b'_' | b'.'))
        }
        _ => false,
    }
}

/// Whether a text has the shape of a command line: shell operators, command substitution, or a
/// word followed by switches.
fn command_shaped(t: &str) -> bool {
    ["&&", "||", "|", ">", "<", "`", "$(", "${"]
        .iter()
        .any(|op| t.contains(op))
        || tokens(t).skip(1).any(switch)
}

/// Whether a word looks like a file name (it has an ending such as `.cur`).
fn file_like(token: &str) -> bool {
    token.rsplit_once('.').is_some_and(|(stem, ext)| {
        !stem.is_empty()
            && (1..=8).contains(&ext.len())
            && ext.bytes().all(|b| b.is_ascii_alphanumeric())
    })
}

/// Whether a text could make the new laptop run or fetch something, and why (the general rule
/// of the fourth review, finding F1: by the key's kind and the value, never by app name).
///
/// Anything that is, or holds, a path or an address in any spelling, a command line, or a value
/// of a key whose name or store marks something to run or load, is kept back. `None` means plain
/// text, a plain number, or a single full local path (which the caller points at the moved file
/// when it is plain data, or leaves out when it is not moved).
pub fn run_or_fetch(hints: &[ActionHint], store: &str, name: &str, t: &str) -> Option<Why> {
    let action = action_kind(hints, store, name);
    let t = t.trim();
    if t.is_empty() || plain_number(t) {
        return None;
    }
    if action == Some(Action::Run) {
        return Some(Why::StartsProgram);
    }
    // One full local path (spaces allowed): data, unless it is a program or a file the system
    // loads.
    if path_in(unquoted(t)).is_some() {
        return (program_end(hints, t) || action == Some(Action::File))
            .then_some(Why::StartsProgram);
    }
    if command_shaped(t) {
        return Some(Why::StartsProgram);
    }
    let mut spots = Vec::new();
    for tok in tokens(t) {
        if program_end(hints, tok) || (action == Some(Action::File) && file_like(tok)) {
            return Some(Why::StartsProgram);
        }
        spots.extend(spot(tok));
    }
    // The whole text too: a path with spaces in it is one place.
    spots.extend(spot(t));
    if program_end(hints, t) {
        return Some(Why::StartsProgram);
    }
    if spots.contains(&Spot::Remote) {
        return Some(Why::Fetches);
    }
    if action == Some(Action::File) && !spots.is_empty() {
        return Some(Why::StartsProgram);
    }
    if spots.contains(&Spot::Unchecked) {
        return Some(Why::PlaceUnknown);
    }
    None
}

/// A text without one pair of double quotes around it.
fn unquoted(t: &str) -> &str {
    t.strip_prefix('"')
        .and_then(|r| r.strip_suffix('"'))
        .filter(|r| !r.contains('"'))
        .unwrap_or(t)
}

/// The one full local path a text is (the whole text, `C:/...` with forward slashes, or a local
/// `file:///` address), and how to write a new one back in the same spelling.
struct PathHit {
    path: String,
    rebuild: Box<dyn Fn(&str) -> String>,
}

fn path_in(t: &str) -> Option<PathHit> {
    if full_path(t) {
        return Some(PathHit {
            path: t.to_string(),
            rebuild: Box::new(|n| n.to_string()),
        });
    }
    let b = t.as_bytes();
    if is_drive(b) && b.get(2) == Some(&b'/') && !t.contains('\\') {
        let win = t.replace('/', "\\");
        return full_path(&win).then(|| PathHit {
            path: win,
            rebuild: Box::new(|n: &str| n.replace('\\', "/")),
        });
    }
    if t.len() > 8 && t.is_char_boundary(7) && t[..7].eq_ignore_ascii_case("file://") {
        let decoded = crate::value::path_of_file_uri(&format!("file://{}", &t[7..]))?;
        // `file:///c:/Users/...` names a Windows path.
        let d = decoded.as_bytes();
        if d.len() > 3 && d[0] == b'/' && d[1].is_ascii_alphabetic() && d[2] == b':' {
            let win = decoded[1..].replace('/', "\\");
            return full_path(&win).then(|| PathHit {
                path: win,
                rebuild: Box::new(|n: &str| {
                    let unix = format!("/{}", n.replace('\\', "/"));
                    crate::value::file_uri(&unix).unwrap_or_default()
                }),
            });
        }
        return full_path(&decoded).then(|| PathHit {
            path: decoded,
            rebuild: Box::new(|n: &str| crate::value::file_uri(n).unwrap_or_default()),
        });
    }
    None
}

/// Whether PCTwin may write the text `t` to the key `name` in `store`, checked again on the new
/// laptop at the moment of writing with the same rule as planning ([`run_or_fetch`]). A single
/// full local path passes: it was pointed at the moved file when it was planned.
pub fn may_write_text(hints: &[ActionHint], store: &str, name: &str, t: &str) -> Result<(), Why> {
    if !text_ok(t) {
        return Err(Why::HiddenText);
    }
    if t.len() > MAX_VALUE {
        return Err(Why::TooLarge);
    }
    match run_or_fetch(hints, store, name, t) {
        None => Ok(()),
        Some(why) => Err(why),
    }
}

/// What happens to the value `value` named `name` in the store `store`.
pub fn classify(store: &str, name: &str, value: &SettingValue, ctx: &Ctx<'_>) -> Fate {
    classify_kind(store, name, value, false, ctx)
}

/// [`classify`], for a value that is raw bytes written as hex (`binary`): only a known credential
/// form is looked for in it, since random-looking bytes are normal there.
pub fn classify_kind(
    store: &str,
    name: &str,
    value: &SettingValue,
    binary: bool,
    ctx: &Ctx<'_>,
) -> Fate {
    if let Some(why) = store_kind(store) {
        return Fate::NotMoved(why);
    }
    let w = words(name);
    let joined: String = w.concat();
    let any = |list: &[&str]| w.iter().any(|x| list.contains(&x.as_str()));
    if any(SECRET_WORDS) || SECRET_JOINED.iter().any(|j| joined.contains(j)) {
        return Fate::NotMoved(Why::Secret);
    }
    let names_id = any(ID_WORDS) || joined.ends_with("guid") || joined.ends_with("uuid");
    if names_id && any(MACHINE_WORDS)
        || ["machineid", "deviceid", "installid", "hwid", "uuid", "guid"].contains(&joined.as_str())
    {
        return Fate::NotMoved(Why::MachineId);
    }
    if (names_id && any(ACCOUNT_WORDS)) || w.iter().any(|x| x == "sid") {
        return Fate::NotMoved(Why::AccountBound);
    }
    if any(CACHE_WORDS) {
        return Fate::NotMoved(Why::Cache);
    }
    let last = w.last().map(String::as_str).unwrap_or("");
    let place = PLACE_LAST.contains(&last)
        && (any(PLACE_OWNER) || matches!(last, "placement" | "geometry" | "rect" | "bounds"))
        // macOS keeps window frames as `NSWindow Frame <name>`.
        || joined.contains("windowframe");
    if place && !ctx.same_screens {
        return Fate::NotMoved(Why::WindowPlace);
    }
    let recent = any(RECENT_WORDS) || RECENT_WORDS.iter().any(|r| joined.contains(r));
    match value {
        SettingValue::Text(t) if binary => {
            if t.len() > MAX_VALUE {
                Fate::NotMoved(Why::TooLarge)
            } else if t
                .to_ascii_lowercase()
                .starts_with("01000000d08c9ddf0115d1118c7a00c04fc297eb")
            {
                Fate::NotMoved(Why::Secret)
            } else if action_kind(ctx.hints, store, name).is_some() {
                // Raw bytes of a key that runs or loads something can hold a path too.
                Fate::NotMoved(Why::StartsProgram)
            } else {
                Fate::Copy(value.clone())
            }
        }
        SettingValue::Text(t) => text_fate(store, name, t, value, recent, ctx),
        _ => Fate::Copy(value.clone()),
    }
}

fn text_fate(
    store: &str,
    name: &str,
    t: &str,
    value: &SettingValue,
    recent: bool,
    ctx: &Ctx<'_>,
) -> Fate {
    if !text_ok(t) {
        return Fate::NotMoved(Why::HiddenText);
    }
    if t.len() > MAX_VALUE {
        return Fate::NotMoved(Why::TooLarge);
    }
    if let Some(why) = looks_secret(t) {
        return Fate::NotMoved(why);
    }
    if is_sid(t) {
        return Fate::NotMoved(Why::AccountBound);
    }
    if hardware_path(t) {
        return Fate::NotMoved(Why::HardwarePath);
    }
    // A list (a multi-string value, written as a JSON array): each item on its own.
    if t.starts_with('[')
        && let Ok(items) = serde_json::from_str::<Vec<String>>(t)
    {
        let mut out = Vec::with_capacity(items.len());
        let mut changed = false;
        for item in &items {
            match text_fate(
                store,
                name,
                item,
                &SettingValue::Text(item.clone()),
                recent,
                ctx,
            ) {
                Fate::Copy(_) => out.push(item.clone()),
                Fate::Rewritten(SettingValue::Text(n)) => {
                    changed = true;
                    out.push(n);
                }
                // A recent list keeps the entries that moved.
                Fate::NotMoved(Why::PathNotMoved | Why::RecentMissing) if recent => changed = true,
                Fate::NotMoved(why) => return Fate::NotMoved(why),
                Fate::Rewritten(_) => return Fate::NotMoved(Why::Kind),
            }
        }
        let json = serde_json::to_string(&out).unwrap_or_default();
        return if changed {
            Fate::Rewritten(SettingValue::Text(json))
        } else {
            Fate::Copy(value.clone())
        };
    }
    // Anything that could make the new laptop run or fetch something stays behind.
    if let Some(why) = run_or_fetch(ctx.hints, store, name, t) {
        return Fate::NotMoved(why);
    }
    // What is left is plain text or one full local path to data: pointed at the moved file.
    let inner = unquoted(t);
    let Some(hit) = path_in(inner) else {
        return Fate::Copy(value.clone());
    };
    let quoted = inner.len() != t.len();
    match ctx.paths.new_path(&hit.path) {
        Some(n) => {
            let n = (hit.rebuild)(&n);
            Fate::Rewritten(SettingValue::Text(if quoted {
                format!("\"{n}\"")
            } else {
                n
            }))
        }
        None if recent => Fate::NotMoved(Why::RecentMissing),
        None => Fate::NotMoved(Why::PathNotMoved),
    }
}

/// A Windows account ID (`S-1-5-21-...`).
fn is_sid(t: &str) -> bool {
    let t = t.trim();
    let Some(rest) = t.strip_prefix("S-1-5-21-") else {
        return false;
    };
    rest.split('-').count() >= 3
        && rest
            .split('-')
            .all(|p| !p.is_empty() && p.bytes().all(|b| b.is_ascii_digit()))
}

/// A device's own path.
fn hardware_path(t: &str) -> bool {
    let u = t.to_ascii_uppercase();
    [
        "\\\\?\\",
        "\\\\.\\",
        "USB\\",
        "HID\\",
        "PCI\\",
        "SWD\\",
        "BTHENUM\\",
        "ACPI\\",
        "\\DEVICE\\",
        "/DEV/",
        "/SYS/",
        "/PROC/",
    ]
    .iter()
    .any(|p| u.starts_with(p))
        || (u.contains("VID_") && u.contains("PID_"))
}

/// Whether text has the form of a credential.
pub fn looks_secret(t: &str) -> Option<Why> {
    let s = t.trim();
    // Windows DPAPI blobs, as base64 or hex text.
    if s.starts_with("AQAAANCMnd8BFdERjHoAwE/Cl+s")
        || s.to_ascii_lowercase()
            .starts_with("01000000d08c9ddf0115d1118c7a00c04fc297eb")
    {
        return Some(Why::Secret);
    }
    if s.contains("-----BEGIN") || s.starts_with("Bearer ") || s.starts_with("Basic ") {
        return Some(Why::Secret);
    }
    let prefixes = [
        "ghp_",
        "gho_",
        "ghu_",
        "ghs_",
        "ghr_",
        "github_pat_",
        "glpat-",
        "xoxb-",
        "xoxp-",
        "xoxa-",
        "xoxr-",
        "sk-",
        "AKIA",
        "ASIA",
        "AIza",
        "ya29.",
        "npm_",
        "pypi-",
    ];
    if prefixes.iter().any(|p| s.starts_with(p)) && s.len() >= 16 && !s.contains(' ') {
        return Some(Why::Secret);
    }
    // A JSON web token: three base64url parts, the first a JSON header.
    let parts: Vec<&str> = s.split('.').collect();
    if parts.len() == 3 && s.starts_with("eyJ") && parts.iter().all(|p| p.len() >= 4 && b64url(p)) {
        return Some(Why::Secret);
    }
    // A long random-looking string (base64 alphabets include `/`, so only spaces rule it out).
    if s.len() >= 32 && !s.contains(' ') {
        let hex = s.bytes().all(|b| b.is_ascii_hexdigit());
        let e = entropy(s);
        if (hex && s.len() >= 40 && e >= 3.0) || (!hex && b64(s) && e >= 4.2) {
            return Some(Why::Secret);
        }
    }
    // A credential inside a longer text (a list, a quoted value): each word on its own.
    let token =
        |c: char| c.is_ascii_alphanumeric() || matches!(c, '+' | '/' | '=' | '_' | '-' | '.');
    s.split(|c: char| !token(c))
        .filter(|t| t.len() >= 16 && t.len() < s.len())
        .find_map(looks_secret)
}

fn b64(s: &str) -> bool {
    s.bytes()
        .all(|b| b.is_ascii_alphanumeric() || matches!(b, b'+' | b'/' | b'=' | b'-' | b'_'))
}

fn b64url(s: &str) -> bool {
    s.bytes()
        .all(|b| b.is_ascii_alphanumeric() || matches!(b, b'-' | b'_' | b'='))
}

/// Shannon entropy in bits per character.
fn entropy(s: &str) -> f64 {
    let mut counts = [0u32; 256];
    for b in s.bytes() {
        counts[usize::from(b)] += 1;
    }
    let n = s.len() as f64;
    counts
        .iter()
        .filter(|&&c| c > 0)
        .map(|&c| {
            let p = f64::from(c) / n;
            -p * p.log2()
        })
        .sum()
}
