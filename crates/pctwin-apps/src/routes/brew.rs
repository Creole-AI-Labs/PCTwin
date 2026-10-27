//! Homebrew casks, read from Homebrew's public JSON without the `brew` program (the
//! "Installing (2.1)" research notes; field names checked against the live
//! `https://formulae.brew.sh/api/cask.json` on 27 October 2026: `token`, `version`, `sha256`
//! (or `"no_check"`), `url`, `auto_updates`, `deprecated`, `disabled`, `depends_on`, and
//! `artifacts`, whose `app` items name the bundles and whose `uninstall`/`zap` items name bundle
//! IDs in `quit`, `launchctl`, `signal` and `pkgutil`).
//!
//! A cask only says where a download is. PCTwin downloads the cask's address directly, checks
//! its SHA-256 when the cask names one, then the maker's Team ID and Apple's notarisation: a cask
//! that points somewhere else fails at the Team ID. Disabled and deprecated casks are treated as
//! absent (the research: 892 disabled and 247 deprecated of 7,780 on 16 October 2026).

use std::collections::BTreeMap;

use serde::Deserialize;
use serde_json::Value;

/// The address of the cask list.
pub const CASKS_URL: &str = "https://formulae.brew.sh/api/cask.json";
/// The largest cask list read (19 MB on 27 October 2026).
pub const MAX_CASKS_BYTES: usize = 64 * 1024 * 1024;

/// One cask, as PCTwin uses it.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Cask {
    pub token: String,
    pub version: String,
    /// `None` when the cask does not pin the bytes (`"no_check"`): the maker check alone decides.
    pub sha256: Option<[u8; 32]>,
    pub url: String,
    pub auto_updates: bool,
    /// The `.app` folders it installs.
    pub apps: Vec<String>,
    /// Bundle IDs it names (to quit or remove them).
    pub bundle_ids: Vec<String>,
}

#[derive(Deserialize)]
struct RawCask {
    token: String,
    version: Value,
    sha256: Value,
    url: Value,
    #[serde(default)]
    auto_updates: Option<bool>,
    #[serde(default)]
    deprecated: bool,
    #[serde(default)]
    disabled: bool,
    #[serde(default)]
    artifacts: Vec<Value>,
}

fn hex32(s: &str) -> Option<[u8; 32]> {
    if s.len() != 64 {
        return None;
    }
    let mut out = [0u8; 32];
    for (i, pair) in s.as_bytes().chunks(2).enumerate() {
        out[i] = u8::from_str_radix(std::str::from_utf8(pair).ok()?, 16).ok()?;
    }
    Some(out)
}

fn looks_like_bundle_id(s: &str) -> bool {
    s.len() <= 255
        && s.split('.').count() >= 2
        && !s.contains(['*', ' ', '/', '~', '$'])
        && s.bytes()
            .all(|b| b.is_ascii_alphanumeric() || matches!(b, b'.' | b'-' | b'_'))
}

fn strings(v: &Value, out: &mut Vec<String>) {
    match v {
        Value::String(s) => out.push(s.clone()),
        Value::Array(a) => a.iter().for_each(|v| strings(v, out)),
        _ => {}
    }
}

fn read_artifacts(artifacts: &[Value]) -> (Vec<String>, Vec<String>) {
    let mut apps = Vec::new();
    let mut ids = Vec::new();
    for a in artifacts {
        let Value::Object(o) = a else { continue };
        if let Some(v) = o.get("app") {
            let mut names = Vec::new();
            strings(v, &mut names);
            apps.extend(names.into_iter().filter(|n| n.ends_with(".app")));
        }
        for key in ["uninstall", "zap"] {
            let Some(Value::Array(items)) = o.get(key) else {
                continue;
            };
            for item in items {
                let Value::Object(inner) = item else { continue };
                for field in ["quit", "launchctl", "signal", "pkgutil"] {
                    if let Some(v) = inner.get(field) {
                        let mut found = Vec::new();
                        strings(v, &mut found);
                        ids.extend(found.into_iter().filter(|s| looks_like_bundle_id(s)));
                    }
                }
            }
        }
    }
    apps.sort();
    apps.dedup();
    ids.sort();
    ids.dedup();
    (apps, ids)
}

/// Every usable cask in Homebrew's list, indexed by the `.app` it installs and the bundle IDs it
/// names.
#[derive(Debug, Default)]
pub struct CaskIndex {
    casks: Vec<Cask>,
    by_app: BTreeMap<String, Vec<usize>>,
    by_bundle: BTreeMap<String, Vec<usize>>,
    /// Casks left out: disabled, deprecated, or not readable.
    pub left_out: usize,
}

impl CaskIndex {
    /// Reads Homebrew's cask list (the whole JSON array).
    pub fn parse(bytes: &[u8]) -> Result<CaskIndex, String> {
        if bytes.len() > MAX_CASKS_BYTES {
            return Err("the cask list is too large".into());
        }
        let raw: Vec<Value> = serde_json::from_slice(bytes).map_err(|e| e.to_string())?;
        let mut index = CaskIndex::default();
        for v in raw {
            let Ok(c) = serde_json::from_value::<RawCask>(v) else {
                index.left_out += 1;
                continue;
            };
            let (Value::String(version), Value::String(url)) = (&c.version, &c.url) else {
                index.left_out += 1;
                continue;
            };
            if c.deprecated || c.disabled || !url.starts_with("https://") {
                index.left_out += 1;
                continue;
            }
            let sha256 = match &c.sha256 {
                Value::String(s) if s == "no_check" => None,
                Value::String(s) => match hex32(s) {
                    Some(h) => Some(h),
                    None => {
                        index.left_out += 1;
                        continue;
                    }
                },
                _ => {
                    index.left_out += 1;
                    continue;
                }
            };
            let (apps, bundle_ids) = read_artifacts(&c.artifacts);
            if apps.is_empty() {
                // Not an app (a font, a driver package, a command-line tool).
                index.left_out += 1;
                continue;
            }
            let n = index.casks.len();
            for a in &apps {
                index.by_app.entry(a.to_lowercase()).or_default().push(n);
            }
            for b in &bundle_ids {
                index.by_bundle.entry(b.to_lowercase()).or_default().push(n);
            }
            index.casks.push(Cask {
                token: c.token,
                version: version.clone(),
                sha256,
                url: url.clone(),
                auto_updates: c.auto_updates.unwrap_or(false),
                apps,
                bundle_ids,
            });
        }
        Ok(index)
    }

    pub fn len(&self) -> usize {
        self.casks.len()
    }

    pub fn is_empty(&self) -> bool {
        self.casks.is_empty()
    }

    /// The casks for a bundle: those naming its bundle ID first, then those installing a folder
    /// of its name. At most three; the Team ID check decides among them.
    pub fn for_bundle(&self, bundle_id: &str, folder: &str) -> Vec<&Cask> {
        let mut out: Vec<usize> = self
            .by_bundle
            .get(&bundle_id.to_lowercase())
            .cloned()
            .unwrap_or_default();
        for n in self
            .by_app
            .get(&folder.to_lowercase())
            .into_iter()
            .flatten()
        {
            if !out.contains(n) {
                out.push(*n);
            }
        }
        out.into_iter().take(3).map(|n| &self.casks[n]).collect()
    }

    /// One cask by its token (a catalogue row may name it).
    pub fn by_token(&self, token: &str) -> Option<&Cask> {
        self.casks.iter().find(|c| c.token == token)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Shaped like the live list (the Firefox cask's fields, trimmed).
    const LIST: &str = r#"[
 {"token":"firefox","version":"158.0.1","sha256":"1d7aa96b51fc9110185047de18228ac87238fba5e42bba53c9d696f88878cba6",
  "url":"https://download-installer.cdn.mozilla.net/pub/firefox/releases/158.0.1/mac/en-US/Firefox%20158.0.1.dmg",
  "auto_updates":true,"deprecated":false,"disabled":false,"depends_on":{"macos":{}},
  "artifacts":[{"uninstall":[{"quit":"org.mozilla.firefox"}]},{"app":["Firefox.app"],"target":"/Applications/Firefox.app"},
   {"zap":[{"trash":["~/Library/Caches/Firefox"]}]}]},
 {"token":"old-thing","version":"1.0","sha256":"no_check","url":"https://example.invalid/old.dmg","deprecated":true,"artifacts":[{"app":["Old.app"]}]},
 {"token":"nocheck","version":"latest","sha256":"no_check","url":"https://example.invalid/n.dmg","artifacts":[{"app":["Notes.app"]}]},
 {"token":"font-x","version":"1","sha256":"no_check","url":"https://example.invalid/f.zip","artifacts":[{"font":["X.ttf"]}]},
 {"token":"plain","version":"1","sha256":"no_check","url":"http://example.invalid/p.dmg","artifacts":[{"app":["Plain.app"]}]},
 {"token":"weird","version":{"a":1},"sha256":"x","url":"https://x.invalid","artifacts":[]}
]"#;

    #[test]
    fn the_cask_list_is_indexed_by_app_and_bundle_id() {
        let ix = CaskIndex::parse(LIST.as_bytes()).unwrap();
        assert_eq!(ix.len(), 2);
        assert_eq!(ix.left_out, 4);
        let ff = ix.for_bundle("org.mozilla.firefox", "Firefox.app");
        assert_eq!(ff.len(), 1);
        assert_eq!(ff[0].token, "firefox");
        assert!(ff[0].auto_updates);
        assert_eq!(ff[0].sha256.unwrap()[0], 0x1d);
        // Found by the folder alone.
        assert_eq!(ix.for_bundle("com.other", "firefox.APP").len(), 1);
        // A cask that does not pin its bytes is kept; the maker check decides.
        assert_eq!(ix.by_token("nocheck").unwrap().sha256, None);
        // Deprecated and plain-http casks are absent.
        assert!(ix.by_token("old-thing").is_none());
        assert!(ix.by_token("plain").is_none());
        assert!(CaskIndex::parse(b"{}").is_err());
        // Many casks for one app: three at most are tried.
        let many: Vec<String> = (0..5)
            .map(|n| format!(r#"{{"token":"p{n}","version":"1","sha256":"no_check","url":"https://e.invalid/{n}.dmg","artifacts":[{{"app":["P.app"]}}]}}"#))
            .collect();
        let ix = CaskIndex::parse(format!("[{}]", many.join(",")).as_bytes()).unwrap();
        assert_eq!(ix.for_bundle("x.y", "P.app").len(), 3);
    }
}
