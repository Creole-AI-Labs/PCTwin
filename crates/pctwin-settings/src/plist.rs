//! Reading the XML property lists `defaults export <domain> -` prints: structurally, with
//! bounded depth and size, never by string search. Mac preferences are only ever written through
//! `defaults` (the preferences system), never by editing a property list file.

/// A property list value.
#[derive(Debug, Clone, PartialEq)]
pub enum Plist {
    Dict(Vec<(String, Plist)>),
    Array(Vec<Plist>),
    String(String),
    Integer(i64),
    Real(f64),
    Bool(bool),
    /// Data and dates: not used by settings.
    Other,
}

const MAX_DEPTH: usize = 32;
const MAX_BYTES: usize = 8 * 1024 * 1024;

impl Plist {
    /// The value under `key` in a dictionary.
    pub fn get(&self, key: &str) -> Option<&Plist> {
        match self {
            Plist::Dict(items) => items.iter().find(|(k, _)| k == key).map(|(_, v)| v),
            _ => None,
        }
    }

    /// The keys of a dictionary.
    pub fn keys(&self) -> Vec<String> {
        match self {
            Plist::Dict(items) => items.iter().map(|(k, _)| k.clone()).collect(),
            _ => Vec::new(),
        }
    }
}

/// Parses an XML property list (its top value).
pub fn parse(text: &str) -> Result<Plist, String> {
    if text.len() > MAX_BYTES {
        return Err("the preferences are too large".into());
    }
    let opts = roxmltree::ParsingOptions {
        // `defaults` prints the standard property list DOCTYPE; no external entity is ever
        // fetched (no resolver), and roxmltree refuses entity expansion attacks.
        allow_dtd: true,
        nodes_limit: 1_000_000,
        ..Default::default()
    };
    let doc = roxmltree::Document::parse_with_options(text, opts)
        .map_err(|_| "the preferences could not be read".to_string())?;
    let root = doc.root_element();
    if root.tag_name().name() != "plist" {
        return Err("not a property list".into());
    }
    let top = root
        .children()
        .find(|n| n.is_element())
        .ok_or("an empty property list")?;
    value(top, 0)
}

fn value(n: roxmltree::Node<'_, '_>, depth: usize) -> Result<Plist, String> {
    if depth > MAX_DEPTH {
        return Err("the preferences are nested too deeply".into());
    }
    let text = || n.text().unwrap_or("").to_string();
    Ok(match n.tag_name().name() {
        "dict" => {
            let mut items = Vec::new();
            let mut key: Option<String> = None;
            for c in n.children().filter(|c| c.is_element()) {
                if c.tag_name().name() == "key" {
                    key = Some(c.text().unwrap_or("").to_string());
                } else {
                    let k = key.take().ok_or("a value without a key")?;
                    items.push((k, value(c, depth + 1)?));
                }
            }
            Plist::Dict(items)
        }
        "array" => Plist::Array(
            n.children()
                .filter(|c| c.is_element())
                .map(|c| value(c, depth + 1))
                .collect::<Result<_, _>>()?,
        ),
        "string" => Plist::String(text()),
        "integer" => Plist::Integer(text().trim().parse().map_err(|_| "a bad number")?),
        "real" => {
            let f: f64 = text().trim().parse().map_err(|_| "a bad number")?;
            if !f.is_finite() {
                return Err("a bad number".into());
            }
            Plist::Real(f)
        }
        "true" => Plist::Bool(true),
        "false" => Plist::Bool(false),
        _ => Plist::Other,
    })
}

/// A value as one line of XML property list (no line breaks: control characters are written as
/// character references), as `defaults write <domain> <key> '<xml>'` takes it. `None` for data
/// and dates, which settings do not carry.
pub fn to_xml(v: &Plist) -> Option<String> {
    Some(match v {
        Plist::Dict(items) => {
            let mut s = String::from("<dict>");
            for (k, v) in items {
                s.push_str(&format!("<key>{}</key>{}", escape(k), to_xml(v)?));
            }
            s.push_str("</dict>");
            s
        }
        Plist::Array(items) => {
            let mut s = String::from("<array>");
            for v in items {
                s.push_str(&to_xml(v)?);
            }
            s.push_str("</array>");
            s
        }
        Plist::String(t) => format!("<string>{}</string>", escape(t)),
        Plist::Integer(i) => format!("<integer>{i}</integer>"),
        Plist::Real(r) => format!("<real>{r}</real>"),
        Plist::Bool(true) => "<true/>".into(),
        Plist::Bool(false) => "<false/>".into(),
        Plist::Other => return None,
    })
}

fn escape(t: &str) -> String {
    let mut out = String::with_capacity(t.len());
    for c in t.chars() {
        match c {
            '&' => out.push_str("&amp;"),
            '<' => out.push_str("&lt;"),
            '>' => out.push_str("&gt;"),
            c if c.is_control() => out.push_str(&format!("&#{};", u32::from(c))),
            c => out.push(c),
        }
    }
    out
}

/// Parses one value written by [`to_xml`] (a fragment, without the `<plist>` wrapper).
pub fn from_xml(fragment: &str) -> Result<Plist, String> {
    parse(&format!("<plist version=\"1.0\">{fragment}</plist>"))
}
