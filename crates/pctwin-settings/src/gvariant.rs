//! The few GVariant text forms `gsettings get` prints and `gsettings set` takes: strings, numbers,
//! booleans and the `a(ss)` input-source list. Parsed structurally; written with every quote and
//! backslash escaped, and passed as one argument (never through a shell).

/// A parsed GVariant value.
#[derive(Debug, Clone, PartialEq)]
pub enum GValue {
    Str(String),
    Int(i64),
    Float(f64),
    Bool(bool),
    /// `[('xkb', 'us'), ('ibus', 'mozc-jp')]`
    Pairs(Vec<(String, String)>),
}

/// Parses `gsettings get` output.
pub fn parse(text: &str) -> Result<GValue, String> {
    let bad = || "a value PCTwin does not know".to_string();
    let t = text.trim();
    // A type prefix (`uint32 2`, `@a(ss) []`).
    let t = match t.split_once(' ') {
        Some(("uint32" | "int32" | "int64" | "uint64" | "double" | "@a(ss)", rest)) => rest.trim(),
        _ => t,
    };
    if t == "true" || t == "false" {
        return Ok(GValue::Bool(t == "true"));
    }
    if t.starts_with('\'') || t.starts_with('"') {
        let (s, rest) = string(t).ok_or_else(bad)?;
        return if rest.trim().is_empty() {
            Ok(GValue::Str(s))
        } else {
            Err(bad())
        };
    }
    if let Some(inner) = t.strip_prefix('[').and_then(|r| r.strip_suffix(']')) {
        let mut pairs = Vec::new();
        let mut rest = inner.trim();
        while !rest.is_empty() {
            let r = rest.strip_prefix('(').ok_or_else(bad)?;
            let (a, r) = string(r.trim_start()).ok_or_else(bad)?;
            let r = r.trim_start().strip_prefix(',').ok_or_else(bad)?;
            let (b, r) = string(r.trim_start()).ok_or_else(bad)?;
            let r = r.trim_start().strip_prefix(')').ok_or_else(bad)?;
            pairs.push((a, b));
            if pairs.len() > 256 {
                return Err(bad());
            }
            rest = r.trim_start();
            rest = rest.strip_prefix(',').map(str::trim_start).unwrap_or(rest);
        }
        return Ok(GValue::Pairs(pairs));
    }
    if let Ok(i) = t.parse::<i64>() {
        return Ok(GValue::Int(i));
    }
    match t.parse::<f64>() {
        Ok(f) if f.is_finite() => Ok(GValue::Float(f)),
        _ => Err(bad()),
    }
}

/// One quoted string at the start of `t`, and what follows it.
fn string(t: &str) -> Option<(String, &str)> {
    let mut chars = t.char_indices();
    let (_, q) = chars.next()?;
    if q != '\'' && q != '"' {
        return None;
    }
    let mut out = String::new();
    while let Some((i, c)) = chars.next() {
        match c {
            '\\' => {
                let (_, e) = chars.next()?;
                out.push(match e {
                    'n' => '\n',
                    't' => '\t',
                    other => other,
                });
            }
            c if c == q => return Some((out, &t[i + c.len_utf8()..])),
            c => out.push(c),
        }
    }
    None
}

/// A string as GVariant text: single quotes, with `\` and `'` escaped.
pub fn quote(s: &str) -> String {
    let mut out = String::with_capacity(s.len() + 2);
    out.push('\'');
    for c in s.chars() {
        if c == '\\' || c == '\'' {
            out.push('\\');
        }
        out.push(c);
    }
    out.push('\'');
    out
}

/// Writes a value as GVariant text.
pub fn format(v: &GValue) -> String {
    match v {
        GValue::Str(s) => quote(s),
        GValue::Int(i) => i.to_string(),
        GValue::Float(f) => {
            let s = format!("{f}");
            if s.contains(['.', 'e']) {
                s
            } else {
                format!("{s}.0")
            }
        }
        GValue::Bool(b) => b.to_string(),
        GValue::Pairs(p) if p.is_empty() => "@a(ss) []".into(),
        GValue::Pairs(p) => format!(
            "[{}]",
            p.iter()
                .map(|(a, b)| format!("({}, {})", quote(a), quote(b)))
                .collect::<Vec<_>>()
                .join(", ")
        ),
    }
}

/// Every string inside a GVariant text (a list, a tuple, a dictionary, a variant), in order;
/// `None` when the text holds anything other than strings, numbers, booleans, type words and
/// the brackets around them. Read once, left to right, without recursion (the depth gate has
/// already bounded it).
pub fn strings(text: &str) -> Option<Vec<String>> {
    let mut out = Vec::new();
    let mut rest = text.trim();
    while let Some(c) = rest.chars().next() {
        if c == '\'' || c == '"' {
            let (s, r) = string(rest)?;
            out.push(s);
            rest = r;
            continue;
        }
        let plain = c.is_ascii_alphanumeric()
            || c.is_whitespace()
            || matches!(
                c,
                '[' | ']' | '(' | ')' | '{' | '}' | '<' | '>' | ',' | ':' | '.' | '-' | '+' | '@'
            );
        if !plain {
            return None;
        }
        rest = &rest[c.len_utf8()..];
    }
    Some(out)
}
