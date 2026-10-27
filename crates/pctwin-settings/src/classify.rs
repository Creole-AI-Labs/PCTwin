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
//! - a program to start (a path to a `.exe`, `.sh`, ...): kept for the person, not switched on;
//! - a full path: rewritten through the move's folder map when it points at something moved,
//!   otherwise left out (a recent-files entry whose file did not move is left out the same way);
//! - too large, or holding hidden characters.
//!
//! Everything else is copied as it is.

use pctwin_recipe::{SettingValue, text_ok};

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
}

/// The longest text a setting may carry.
pub const MAX_VALUE: usize = 4000;

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
const PROGRAM_ENDS: &[&str] = &[
    ".exe", ".bat", ".cmd", ".com", ".ps1", ".vbs", ".js", ".msi", ".lnk", ".sh", ".command",
    ".app", ".py", ".jar", ".scr",
];

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
            } else {
                Fate::Copy(value.clone())
            }
        }
        SettingValue::Text(t) => text_fate(t, value, recent, ctx),
        _ => Fate::Copy(value.clone()),
    }
}

fn text_fate(t: &str, value: &SettingValue, recent: bool, ctx: &Ctx<'_>) -> Fate {
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
            match text_fate(item, &SettingValue::Text(item.clone()), recent, ctx) {
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
    if let Some(hit) = path_in(t) {
        let pl = hit.path.to_lowercase();
        if PROGRAM_ENDS.iter().any(|e| pl.ends_with(e)) || (hit.args && pl.contains(".exe")) {
            return Fate::NotMoved(Why::StartsProgram);
        }
        return match ctx.paths.new_path(&hit.path) {
            Some(n) => Fate::Rewritten(SettingValue::Text((hit.rebuild)(&n))),
            None if recent => Fate::NotMoved(Why::RecentMissing),
            None => Fate::NotMoved(Why::PathNotMoved),
        };
    }
    Fate::Copy(value.clone())
}

/// A full path found in a text, and how to put a new one in its place.
struct PathHit<'a> {
    path: String,
    /// Text follows the path (a command line's arguments).
    args: bool,
    rebuild: Box<dyn Fn(&str) -> String + 'a>,
}

/// The full path a text holds: the whole text, a `file://` address, or a quoted path at the
/// start (followed by arguments).
fn path_in(t: &str) -> Option<PathHit<'_>> {
    if full_path(t) {
        return Some(PathHit {
            path: t.to_string(),
            args: false,
            rebuild: Box::new(|n| n.to_string()),
        });
    }
    if t.starts_with("file://") {
        let decoded = crate::value::path_of_file_uri(t)?;
        // `file:///c:/Users/...` names a Windows path.
        let b = decoded.as_bytes();
        if b.len() > 3 && b[0] == b'/' && b[1].is_ascii_alphabetic() && b[2] == b':' {
            let win = decoded[1..].replace('/', "\\");
            return full_path(&win).then(|| PathHit {
                path: win,
                args: false,
                rebuild: Box::new(|n: &str| {
                    let unix = format!("/{}", n.replace('\\', "/"));
                    crate::value::file_uri(&unix).unwrap_or_default()
                }),
            });
        }
        return full_path(&decoded).then(|| PathHit {
            path: decoded,
            args: false,
            rebuild: Box::new(|n: &str| crate::value::file_uri(n).unwrap_or_default()),
        });
    }
    let inner = t.strip_prefix('"')?;
    let end = inner.find('"')?;
    let path = &inner[..end];
    let rest = &inner[end + 1..];
    full_path(path).then(|| PathHit {
        path: path.to_string(),
        args: !rest.trim().is_empty(),
        rebuild: Box::new(move |n: &str| format!("\"{n}\"{rest}")),
    })
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
    // A long random-looking string.
    if s.len() >= 32 && !s.contains([' ', '\\', '/']) || s.len() >= 32 && b64(s) && !s.contains(' ')
    {
        let hex = s.bytes().all(|b| b.is_ascii_hexdigit());
        let e = entropy(s);
        if (hex && s.len() >= 40 && e >= 3.0) || (!hex && b64(s) && e >= 4.2) {
            return Some(Why::Secret);
        }
    }
    None
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
