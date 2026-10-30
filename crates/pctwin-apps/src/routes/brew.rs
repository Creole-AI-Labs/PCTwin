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
//!
//! Only a **plain** cask is installed by PCTwin (30 October 2026, the 2.1 plan: "a cask existing is
//! not automatic PCTwin approval"): one `.app` in a disk image or zip, and nothing a cask would run
//! (`pkg`, `installer`, `preflight`, `postflight` and their `_steps` forms, checked against the
//! live list for Zoom and Docker Desktop on 30 October 2026). Command-line shims (`binary`,
//! `command_wrapper`, `manpage`, shell completions) are left out: the app itself is whole without
//! them. Any other part makes the cask not plain, so a part Homebrew adds later is never skipped
//! silently.
//!
//! **Intel and Apple silicon.** The list's own address is the Apple-silicon build on the newest
//! macOS; a cask that differs on Intel Macs says so under `variations`, keyed by macOS name
//! (`arm64_*` keys are Apple silicon on older systems, `*_linux` keys are not Macs). The Intel
//! build PCTwin takes is the newest version among those keys (by version, never by a list of macOS
//! names).

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
    /// One app and nothing the cask would run (see the module notes).
    pub plain: bool,
    /// The Intel build, where it differs from the list's own (Apple-silicon) one.
    pub intel: Option<CaskDownload>,
}

/// Where one build of a cask is, and the bytes it must be.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CaskDownload {
    pub version: String,
    pub url: String,
    pub sha256: Option<[u8; 32]>,
}

/// The artifact parts a plain cask may have (see the module notes): the app, where it goes, how
/// Homebrew would remove it, and command-line shims PCTwin leaves out.
const PLAIN_PARTS: &[&str] = &[
    "app",
    "target",
    "uninstall",
    "zap",
    "binary",
    "command_wrapper",
    "manpage",
    "bash_completion",
    "zsh_completion",
    "fish_completion",
];

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
    #[serde(default)]
    variations: serde_json::Map<String, Value>,
}

/// Whether every part of every artifact is one a plain cask may have, with exactly one app.
fn plain_parts(artifacts: &[Value], apps: usize) -> bool {
    apps == 1
        && artifacts.iter().all(|a| match a {
            Value::Object(o) => o.keys().all(|k| PLAIN_PARTS.contains(&k.as_str())),
            _ => false,
        })
}

/// The sha256 field: `None` for `"no_check"`, `Some(None)` when unreadable.
fn sha_field(v: &Value) -> Option<Option<[u8; 32]>> {
    match v {
        Value::String(s) if s == "no_check" => Some(None),
        Value::String(s) => hex32(s).map(Some),
        _ => None,
    }
}

/// The newest Intel build among `variations`, when it is not the list's own build.
fn intel_build(
    variations: &serde_json::Map<String, Value>,
    own: &CaskDownload,
) -> Option<CaskDownload> {
    let mut best: Option<CaskDownload> = None;
    for (key, v) in variations {
        if key.starts_with("arm64") || key.contains("linux") {
            continue;
        }
        let Value::Object(o) = v else { continue };
        let version = match o.get("version") {
            Some(Value::String(s)) => s.clone(),
            None => own.version.clone(),
            Some(_) => continue,
        };
        let url = match o.get("url") {
            Some(Value::String(s)) if s.starts_with("https://") => s.clone(),
            None => own.url.clone(),
            Some(_) => continue,
        };
        let sha256 = match o.get("sha256") {
            Some(v) => match sha_field(v) {
                Some(h) => h,
                None => continue,
            },
            None => own.sha256,
        };
        let newer = best.as_ref().is_none_or(|b| {
            crate::version::compare(&version, &b.version) == crate::version::Order::Newer
        });
        if newer {
            best = Some(CaskDownload {
                version,
                url,
                sha256,
            });
        }
    }
    best.filter(|b| b != own)
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
            let Some(sha256) = sha_field(&c.sha256) else {
                index.left_out += 1;
                continue;
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
            let own = CaskDownload {
                version: version.clone(),
                url: url.clone(),
                sha256,
            };
            let intel = intel_build(&c.variations, &own);
            index.casks.push(Cask {
                token: c.token,
                version: version.clone(),
                sha256,
                url: url.clone(),
                auto_updates: c.auto_updates.unwrap_or(false),
                plain: plain_parts(&c.artifacts, apps.len()),
                apps,
                bundle_ids,
                intel,
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

impl Cask {
    /// The build for a Mac with this processor: the Intel build on an Intel Mac where the cask
    /// has one, otherwise the list's own.
    pub fn build_for(&self, arm64: bool) -> CaskDownload {
        match (&self.intel, arm64) {
            (Some(i), false) => i.clone(),
            _ => CaskDownload {
                version: self.version.clone(),
                url: self.url.clone(),
                sha256: self.sha256,
            },
        }
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

    /// Shaped like the live list on 30 October 2026 (Zoom: a package and postflight steps; VS
    /// Code: an app with command-line shims and an Intel build under `variations`).
    const SHAPES: &str = r#"[
 {"token":"zoom","version":"7.2.2","sha256":"no_check","url":"https://cdn.zoom.us/prod/7.2.2/arm64/zoomusInstallerFull.pkg",
  "artifacts":[{"uninstall":[{"quit":"us.zoom.xos"}]},{"pkg":["zoomusInstallerFull.pkg"]},{"app":["zoom.us.app"]},{"postflight_steps":[]},{"zap":[]}]},
 {"token":"visual-studio-code","version":"1.144.0","sha256":"1d7aa96b51fc9110185047de18228ac87238fba5e42bba53c9d696f88878cba6",
  "url":"https://update.code.visualstudio.com/1.144.0/darwin-arm64/stable",
  "artifacts":[{"uninstall":[{"quit":"com.microsoft.VSCode"}]},{"app":["Visual Studio Code.app"],"target":"x"},{"binary":["code"],"target":"code"},{"zap":[]}],
  "variations":{
   "tahoe":{"url":"https://update.code.visualstudio.com/1.144.0/darwin/stable","sha256":"a15a04aaa109641bfbe5d1d6204fa96e7ff14e9fd7c495d89fa2376c3edc580b"},
   "big_sur":{"version":"1.90.0","url":"https://update.code.visualstudio.com/1.90.0/darwin/stable","sha256":"no_check"},
   "arm64_big_sur":{"version":"1.90.0","url":"https://update.code.visualstudio.com/1.90.0/darwin-arm64/stable"},
   "x86_64_linux":{"url":"https://example.invalid/linux.tar.gz"}}},
 {"token":"two","version":"1","sha256":"no_check","url":"https://e.invalid/t.dmg","artifacts":[{"app":["A.app","B.app"]}]},
 {"token":"same","version":"1","sha256":"no_check","url":"https://e.invalid/s.dmg","artifacts":[{"app":["S.app"]}],
  "variations":{"sonoma":{"sha256":"no_check"},"ventura":{"url":"http://e.invalid/plain.dmg"}}},
 {"token":"newpart","version":"1","sha256":"no_check","url":"https://e.invalid/n.dmg","artifacts":[{"app":["N.app"]},{"something_new":["x"]}]},
 {"token":"armonly","version":"2","sha256":"no_check","url":"https://e.invalid/arm.dmg","artifacts":[{"app":["A.app"]}],
  "variations":{"arm64_sonoma":{"version":"3","url":"https://e.invalid/arm-sonoma.dmg"},"x86_64_linux":{"version":"4","url":"https://e.invalid/l.tgz"}}}
]"#;

    #[test]
    fn only_a_cask_with_one_app_and_nothing_it_runs_is_plain() {
        let ix = CaskIndex::parse(SHAPES.as_bytes()).unwrap();
        assert!(
            !ix.by_token("zoom").unwrap().plain,
            "a package and postflight steps"
        );
        assert!(
            ix.by_token("visual-studio-code").unwrap().plain,
            "shims are left out"
        );
        assert!(!ix.by_token("two").unwrap().plain, "two apps");
        assert!(ix.by_token("same").unwrap().plain);
        assert!(
            !ix.by_token("newpart").unwrap().plain,
            "a part PCTwin does not know"
        );
        assert!(ix.by_token("firefox").is_none());
        let ff = CaskIndex::parse(LIST.as_bytes()).unwrap();
        assert!(ff.by_token("firefox").unwrap().plain);
    }

    #[test]
    fn an_intel_mac_gets_the_newest_intel_build_and_apple_silicon_the_list_own() {
        let ix = CaskIndex::parse(SHAPES.as_bytes()).unwrap();
        let code = ix.by_token("visual-studio-code").unwrap();
        let intel = code.build_for(false);
        assert_eq!(
            intel.url,
            "https://update.code.visualstudio.com/1.144.0/darwin/stable"
        );
        assert_eq!(
            intel.version, "1.144.0",
            "the newest Intel build, not big_sur's"
        );
        assert_eq!(intel.sha256.unwrap()[0], 0xa1);
        let arm = code.build_for(true);
        assert_eq!(arm.url, code.url);
        assert_eq!(arm.sha256, code.sha256);
        // A variation that differs in nothing but its key, or names a plain-http address, adds
        // no Intel build.
        let same = ix.by_token("same").unwrap();
        assert_eq!(same.intel, None);
        assert_eq!(same.build_for(false).url, "https://e.invalid/s.dmg");
        // Apple-silicon and Linux keys are never an Intel Mac's build, however new.
        assert_eq!(ix.by_token("armonly").unwrap().intel, None);
    }
}
