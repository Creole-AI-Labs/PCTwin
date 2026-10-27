//! Neutral values: the shapes a catalogue row may give a setting, and the checks every value
//! passes before it is planned and again before it is written (T24-002, T24-010).

use pctwin_recipe::{SettingValue, text_ok};
use serde::{Deserialize, Serialize};

/// The longest path a file setting may hold.
pub const MAX_PATH: usize = 1024;
/// The longest text value.
pub const MAX_TEXT: usize = 1024;

/// What kind of name a [`ValueType::Name`] holds.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum NameKind {
    /// No spaces: app IDs, time zones, layout codes ([`plain_name`]).
    #[default]
    Plain,
    /// A device as the system shows it (a printer): spaces allowed, no path separators.
    Device,
}

/// A setting's value shape, as its catalogue row declares it.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case", deny_unknown_fields)]
pub enum ValueType {
    /// One of a fixed list of words.
    Choice {
        options: Vec<String>,
    },
    Bool,
    Int {
        min: i64,
        max: i64,
    },
    /// A file the move brings (a desktop picture): applied only once it has arrived inside the
    /// person's folders on the new laptop.
    FilePath,
    /// A comma-separated list of names (plain names, or names with spaces such as app names).
    List {
        max: usize,
    },
    /// One name.
    Name {
        #[serde(default)]
        kind: NameKind,
    },
}

#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
pub enum ValueError {
    #[error("the value for {0} has the wrong type")]
    WrongType(String),
    #[error("the value for {0} is outside what the setting allows")]
    OutOfRange(String),
    #[error("the value for {0} holds hidden or control characters")]
    HiddenText(String),
}

impl ValueType {
    /// Checks `value` against this shape; `key` names the setting in the error.
    pub fn check(&self, key: &str, value: &SettingValue) -> Result<(), ValueError> {
        let out = || ValueError::OutOfRange(key.to_string());
        let hidden = || ValueError::HiddenText(key.to_string());
        match (self, value) {
            (ValueType::Choice { options }, SettingValue::Text(t)) => {
                options.iter().any(|o| o == t).then_some(()).ok_or_else(out)
            }
            (ValueType::Bool, SettingValue::Bool(_)) => Ok(()),
            (ValueType::Int { min, max }, SettingValue::Int(n)) => {
                (min..=max).contains(&n).then_some(()).ok_or_else(out)
            }
            (ValueType::FilePath, SettingValue::Text(t)) => {
                if !text_ok(t) {
                    Err(hidden())
                } else if t.len() > MAX_PATH || !full_path(t) {
                    Err(out())
                } else {
                    Ok(())
                }
            }
            (ValueType::List { max }, SettingValue::Text(t)) => {
                if !text_ok(t) {
                    return Err(hidden());
                }
                if t.is_empty() {
                    return Ok(());
                }
                let items: Vec<&str> = t.split(',').collect();
                let item_ok = |i: &&str| plain_name(i) || device_name(i);
                (t.len() <= MAX_TEXT && items.len() <= *max && items.iter().all(item_ok))
                    .then_some(())
                    .ok_or_else(out)
            }
            (ValueType::Name { kind }, SettingValue::Text(t)) => {
                if !text_ok(t) {
                    return Err(hidden());
                }
                let ok = match kind {
                    NameKind::Plain => plain_name(t),
                    NameKind::Device => device_name(t),
                };
                ok.then_some(()).ok_or_else(out)
            }
            _ => Err(ValueError::WrongType(key.to_string())),
        }
    }

    /// Whether the shape itself is sound (a catalogue check).
    pub fn sound(&self) -> bool {
        match self {
            ValueType::Choice { options } => {
                !options.is_empty()
                    && options.len() <= 64
                    && options.iter().all(|o| plain_name(o))
                    && options
                        .iter()
                        .enumerate()
                        .all(|(i, o)| !options[..i].contains(o))
            }
            ValueType::Int { min, max } => min <= max,
            ValueType::List { max } => (1..=256).contains(max),
            ValueType::Bool | ValueType::FilePath | ValueType::Name { .. } => true,
        }
    }
}

/// A plain name: ASCII letters, digits and `. _ - + / : =`, or letters of other scripts with
/// their combining marks; 1 to 128 characters; no spaces; no `..`. Covers app IDs
/// (`org.mozilla.firefox`), layouts (`us`, `windows:00000409`), time zones
/// (`America/Argentina/Buenos_Aires`).
pub fn plain_name(s: &str) -> bool {
    let n = s.chars().count();
    (1..=128).contains(&n)
        && !s.starts_with(['-', '.', '/'])
        && !s.contains("..")
        && text_ok(s)
        && s.chars().all(|c| {
            c.is_ascii_alphanumeric()
                || matches!(c, '.' | '_' | '-' | '+' | '/' | ':' | '=')
                || (!c.is_ascii() && !c.is_whitespace() && !c.is_control())
        })
}

/// A device's name as the system shows it: may hold spaces, never a path separator or a comma.
pub fn device_name(s: &str) -> bool {
    let n = s.chars().count();
    (1..=256).contains(&n) && !s.contains(['\\', '/', ',']) && s.trim() == s && text_ok(s)
}

/// A full path on any of the systems: `C:\...` or `/...`, never with a `.` or `..` part, never
/// a network share or a device path.
pub fn full_path(p: &str) -> bool {
    let windows = {
        let b = p.as_bytes();
        b.len() > 3 && b[0].is_ascii_alphabetic() && b[1] == b':' && b[2] == b'\\'
    };
    let unix = p.starts_with('/') && !p.starts_with("//");
    (windows || unix)
        && !p.contains('\0')
        && p.split(['/', '\\']).all(|part| part != ".." && part != ".")
}

/// Whether `path` is inside one of `roots` (whole folder names only, either separator, letters'
/// case ignored on Windows paths).
pub fn inside(path: &str, roots: &[String]) -> bool {
    if !full_path(path) {
        return false;
    }
    let norm = |s: &str| {
        let s = s.replace('\\', "/");
        if s.as_bytes().get(1) == Some(&b':') {
            s.to_lowercase()
        } else {
            s
        }
    };
    let p = norm(path);
    roots.iter().any(|r| {
        let r = norm(r.trim_end_matches(['/', '\\']));
        !r.is_empty() && p.len() > r.len() && p.starts_with(&r) && p.as_bytes()[r.len()] == b'/'
    })
}

/// A Unix path as a `file://` address, every byte outside the unreserved set and `/` encoded.
pub fn file_uri(path: &str) -> Option<String> {
    if !path.starts_with('/') {
        return None;
    }
    let mut out = String::from("file://");
    for b in path.bytes() {
        if b.is_ascii_alphanumeric() || matches!(b, b'/' | b'-' | b'.' | b'_' | b'~') {
            out.push(char::from(b));
        } else {
            out.push_str(&format!("%{b:02X}"));
        }
    }
    Some(out)
}

/// The path a local `file://` address names; `None` for any other address or broken encoding.
pub fn path_of_file_uri(uri: &str) -> Option<String> {
    let rest = uri.strip_prefix("file://")?;
    if !rest.starts_with('/') {
        return None;
    }
    let bytes = rest.as_bytes();
    let mut out = Vec::with_capacity(bytes.len());
    let mut i = 0;
    while i < bytes.len() {
        if bytes[i] == b'%' {
            let hex = std::str::from_utf8(bytes.get(i + 1..i + 3)?).ok()?;
            out.push(u8::from_str_radix(hex, 16).ok()?);
            i += 3;
        } else {
            out.push(bytes[i]);
            i += 1;
        }
    }
    let s = String::from_utf8(out).ok()?;
    (!s.contains('\0')).then_some(s)
}
