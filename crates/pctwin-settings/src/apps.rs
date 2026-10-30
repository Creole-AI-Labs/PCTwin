//! App settings driven by catalogue entries (Task List 2.4: same-OS app settings and cross-OS
//! translations). An entry describes an app's settings files, their format and field rules; no
//! code is written per app.

use pctwin_recipe::text_ok;

use crate::catalogue::{AppEntry, FieldRule, FileFormat, ListForm, Rule, Writer};

/// Checks one app entry (part of reading a catalogue).
pub fn check_entry(a: &AppEntry) -> Result<(), String> {
    let id_ok = !a.app.is_empty()
        && a.app.len() <= 64
        && a.app
            .bytes()
            .all(|b| b.is_ascii_lowercase() || b.is_ascii_digit() || b == b'-');
    if !id_ok {
        return Err("an app name is lowercase letters, digits and -".into());
    }
    if parse_version(&a.min_version).is_none() {
        return Err("the oldest version is not a version".into());
    }
    if let Some(p) = &a.versioned_folder
        && !(pattern_ok(p) && !p.contains('/'))
    {
        return Err("the versioned folder is not a plain name".into());
    }
    if a.files.is_empty() || a.files.len() > 200 {
        return Err("an app needs between 1 and 200 files".into());
    }
    for f in &a.files {
        if !pattern_ok(&f.path) {
            return Err(format!("the file path {:?} is not allowed", f.path));
        }
        if f.format == FileFormat::Opaque && f.writer == Writer::Settings {
            return Err("a file 2.4 writes must have a format it reads".into());
        }
        let rules = f.rules.iter().chain(&f.cross_os);
        for FieldRule { at, rule } in rules {
            if !pattern_ok(at) {
                return Err(format!("the field pattern {at:?} is not allowed"));
            }
            if let Rule::Disable(v) = rule
                && !matches!(
                    v,
                    serde_json::Value::String(_)
                        | serde_json::Value::Bool(_)
                        | serde_json::Value::Number(_)
                        | serde_json::Value::Null
                )
            {
                return Err("a hook is switched off to a plain value".into());
            }
        }
        for m in f.macros.keys() {
            if !(m.starts_with('$') && m.ends_with('$') && m.len() > 2 && m.len() <= 32) {
                return Err("a path placeholder is written $LIKE_THIS$".into());
            }
        }
    }
    if a.recent.len() > 16
        || a.recent.iter().any(|r| {
            !pattern_ok(&r.path)
                || r.key.is_empty()
                || r.key.len() > 128
                || !text_ok(&r.key)
                // A database table: letters, digits and `_`, not starting with a digit.
                || r.table.as_ref().is_some_and(|t| {
                    t.is_empty()
                        || t.len() > 64
                        || t.starts_with(|c: char| c.is_ascii_digit())
                        || !t.chars().all(|c| c.is_ascii_alphanumeric() || c == '_')
                })
        })
    {
        return Err("a recent list is a plain file and key".into());
    }
    if a.documents.len() > 16
        || a.documents.iter().any(|d| {
            let e = d.ending.as_bytes();
            !(2..=32).contains(&e.len())
                || e[0] != b'.'
                || !e[1..]
                    .iter()
                    .all(|b| b.is_ascii_alphanumeric() || matches!(b, b'-' | b'_' | b'.'))
                || d.at.is_empty()
                || d.at.len() > 16
                || d.at.iter().any(|p| !pattern_ok(p))
        })
    {
        return Err("a document kind is a file ending and plain field patterns".into());
    }
    if let Some(x) = &a.extensions {
        if x.places.is_empty()
            || x.places.len() > 8
            || x.places.iter().any(|p| !pattern_ok(&p.path))
        {
            return Err("an add-on list needs between 1 and 8 plain places".into());
        }
        let plain =
            |s: &str| !s.is_empty() && s.len() <= 64 && text_ok(s) && !s.contains(['\\', '*']);
        if let ListForm::Json {
            id,
            source,
            store_sources,
        } = &x.form
            && !(plain(id)
                && source.as_deref().is_none_or(plain)
                && store_sources.len() <= 8
                && store_sources.iter().all(|s| plain(s)))
        {
            return Err("an add-on list's fields are plain names".into());
        }
        if let Some(store) = &x.store
            && !(plain(store)
                && store
                    .bytes()
                    .all(|b| b.is_ascii_lowercase() || b.is_ascii_digit() || b == b'-'))
        {
            return Err("an add-on store is named in lowercase letters, digits and -".into());
        }
    }
    Ok(())
}

/// A `/`-separated pattern: plain parts (each may hold `*`), or `**` as the last part (any
/// depth); never `.`, `..` or an empty part.
fn pattern_ok(p: &str) -> bool {
    let parts: Vec<&str> = p.split('/').collect();
    !p.is_empty()
        && p.len() <= 512
        && text_ok(p)
        && !p.contains('\\')
        && parts.iter().enumerate().all(|(i, part)| {
            !part.is_empty()
                && *part != "."
                && *part != ".."
                && (*part != "**" || i + 1 == parts.len())
        })
}

/// Whether `path` matches `pattern` (letter case ignored; `*` within a part matches any
/// characters of that part; a final `**` matches any rest). A path with `.` or `..` parts never
/// matches.
pub fn path_matches(pattern: &str, path: &str) -> bool {
    let pp: Vec<&str> = pattern.split('/').collect();
    let xp: Vec<&str> = path.split('/').collect();
    if xp.iter().any(|x| x.is_empty() || *x == "." || *x == "..") {
        return false;
    }
    let deep = pp.last() == Some(&"**");
    let fixed = if deep { &pp[..pp.len() - 1] } else { &pp[..] };
    if deep {
        if xp.len() <= fixed.len() {
            return false;
        }
    } else if xp.len() != fixed.len() {
        return false;
    }
    fixed.iter().zip(&xp).all(|(p, x)| glob(p, x))
}

/// `*` matches any run of characters; letter case ignored.
pub fn glob(pattern: &str, text: &str) -> bool {
    let p: Vec<char> = pattern.to_lowercase().chars().collect();
    let t: Vec<char> = text.to_lowercase().chars().collect();
    let (mut pi, mut ti) = (0, 0);
    let (mut star, mut mark) = (None, 0);
    while ti < t.len() {
        if pi < p.len() && p[pi] != '*' && p[pi] == t[ti] {
            pi += 1;
            ti += 1;
        } else if pi < p.len() && p[pi] == '*' {
            star = Some(pi);
            mark = ti;
            pi += 1;
        } else if let Some(s) = star {
            pi = s + 1;
            mark += 1;
            ti = mark;
        } else {
            return false;
        }
    }
    while pi < p.len() && p[pi] == '*' {
        pi += 1;
    }
    pi == p.len()
}

/// `major.minor[.patch]` as numbers.
pub fn parse_version(v: &str) -> Option<(u32, u32, u32)> {
    let mut parts = v.trim().split('.');
    let major = parts.next()?.parse().ok()?;
    let minor = parts.next()?.parse().ok()?;
    let patch = match parts.next() {
        Some(p) => p.parse().ok()?,
        None => 0,
    };
    parts.next().is_none().then_some((major, minor, patch))
}
