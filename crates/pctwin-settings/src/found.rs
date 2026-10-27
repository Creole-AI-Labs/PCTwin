//! Discovery first (the user's decision, 27 October 2026): every setting in the person's own
//! settings stores is found, whether or not the catalogue knows it, and moves between the same
//! kind of system through the generic mechanisms.
//!
//! - **Windows:** every value under the person's `HKEY_CURRENT_USER\Software\<vendor>\...` trees
//!   and `Control Panel\...` (personalisation). Microsoft's own tree is the system's: only the
//!   settings the catalogue knows move from it.
//! - **Mac:** every preferences domain `defaults domains` lists, and the global domain, read with
//!   `defaults export` (the preferences system, never the files).
//! - **GNOME:** every value the person set, from `dconf dump /`.
//!
//! Each found value has an [`Address`] (its exact place) and a neutral [`SettingValue`] that keeps
//! its native type in the address, so the same system gets back exactly what it had. Its executor
//! key is a digest of the address ([`Address::key`]): `found.<32 hex>`.

use std::collections::BTreeMap;

use pctwin_recipe::{SettingValue, text_ok};
use serde::{Deserialize, Serialize};

use crate::classify::{Why, store_kind};
use crate::gvariant;
use crate::mechanism::Mechanisms;
use crate::plist::{self, Plist};
use crate::registry::{Hive, RegValue, WriteError};
use crate::runner::Tool;

/// A registry value's type.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum RegKind {
    Dword,
    Qword,
    Sz,
    ExpandSz,
    /// Carried as a JSON array of strings.
    MultiSz,
    /// Carried as lowercase hex.
    Binary,
}

/// A Mac preference's type.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum PrefKind {
    Bool,
    Int,
    /// Carried as its decimal text.
    Real,
    String,
    /// An array or dictionary, carried as one line of XML property list.
    Xml,
}

/// The exact place of a found setting.
#[derive(Debug, Clone, PartialEq, Eq, Hash, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(tag = "store", rename_all = "kebab-case", deny_unknown_fields)]
pub enum Address {
    /// A value in the person's registry (`HKEY_CURRENT_USER`).
    Registry {
        key: String,
        name: String,
        kind: RegKind,
    },
    MacPref {
        domain: String,
        key: String,
        kind: PrefKind,
    },
    /// A dconf key, its value carried as GVariant text.
    Dconf { path: String, key: String },
}

/// The limits of one discovery, so a huge store cannot exhaust the laptop.
pub const MAX_FOUND: usize = 50_000;
const MAX_DEPTH: usize = 12;
const MAX_BINARY: usize = 1024;

impl Address {
    /// The executor key: `found.` and a digest of the address.
    pub fn key(&self) -> String {
        let bytes = serde_json::to_vec(self).unwrap_or_default();
        let d = blake3::derive_key("PCTwin found setting 2026-10-27 v1", &bytes);
        let hex: String = d[..16].iter().map(|b| format!("{b:02x}")).collect();
        format!("found.{hex}")
    }

    /// The store, as a path the classifier reads (`Software\Vendor\App`, a domain, a dconf path).
    pub fn store(&self) -> &str {
        match self {
            Address::Registry { key, .. } => key,
            Address::MacPref { domain, .. } => domain,
            Address::Dconf { path, .. } => path,
        }
    }

    pub fn name(&self) -> &str {
        match self {
            Address::Registry { name, .. } => name,
            Address::MacPref { key, .. } | Address::Dconf { key, .. } => key,
        }
    }

    /// Whether PCTwin may write here at all: inside the person's own stores that discovery
    /// reads, not a security, policy or program store, plain names.
    pub fn writable(&self) -> bool {
        let plain = |s: &str| !s.is_empty() && s.len() <= 255 && text_ok(s);
        match self {
            Address::Registry { key, name, .. } => {
                let k = key.to_lowercase();
                let scoped = (k.starts_with("software\\") && !k.starts_with("software\\microsoft"))
                    || k == "control panel"
                    || k.starts_with("control panel\\");
                scoped
                    && plain(key)
                    && plain(name)
                    && !key.split('\\').any(|p| p.is_empty() || p == "..")
                    && store_kind(key).is_none()
            }
            Address::MacPref { domain, key, .. } => {
                plain(key)
                    && !domain.is_empty()
                    && domain.len() <= 128
                    && !domain.contains("..")
                    && domain
                        .bytes()
                        .all(|b| b.is_ascii_alphanumeric() || matches!(b, b'.' | b'-' | b'_'))
                    && store_kind(domain).is_none()
            }
            Address::Dconf { path, key } => {
                path.starts_with('/')
                    && path.ends_with('/')
                    && path.len() <= 255
                    && !path.contains("//")
                    && path.bytes().all(|b| {
                        b.is_ascii_alphanumeric() || matches!(b, b'/' | b'-' | b'_' | b'.')
                    })
                    && !key.is_empty()
                    && key.len() <= 128
                    && key
                        .bytes()
                        .all(|b| b.is_ascii_alphanumeric() || matches!(b, b'-' | b'_'))
                    && store_kind(path).is_none()
            }
        }
    }
}

/// One found setting.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct FoundSetting {
    pub address: Address,
    /// Who it belongs to, in plain words from the store (the vendor and app, the domain).
    pub label: String,
    pub value: SettingValue,
}

/// A found setting that does not move, and why. Never its value.
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct NotMoved {
    pub label: String,
    pub store: String,
    pub name: String,
    pub why: Why,
}

/// What discovery found.
#[derive(Debug, Clone, Default, PartialEq)]
pub struct Discovery {
    pub found: Vec<FoundSetting>,
    pub not_moved: Vec<NotMoved>,
}

/// Registry vendors discovery does not walk as the person's apps.
const SYSTEM_VENDORS: &[&str] = &[
    "microsoft",
    "classes",
    "policies",
    "wow6432node",
    "registeredapplications",
    "clients",
    "appdatalow",
];

/// A registry value as a neutral value (its type kept in the kind).
pub fn reg_to_setting(v: &RegValue) -> Result<(RegKind, SettingValue), Why> {
    Ok(match v {
        RegValue::Dword(d) => (RegKind::Dword, SettingValue::Int(i64::from(*d))),
        RegValue::Qword(q) => (
            RegKind::Qword,
            SettingValue::Int(i64::try_from(*q).map_err(|_| Why::Kind)?),
        ),
        RegValue::Sz(s) => (RegKind::Sz, SettingValue::Text(s.clone())),
        RegValue::ExpandSz(s) => (RegKind::ExpandSz, SettingValue::Text(s.clone())),
        RegValue::MultiSz(p) => (
            RegKind::MultiSz,
            SettingValue::Text(serde_json::to_string(p).map_err(|_| Why::Kind)?),
        ),
        RegValue::Binary(b) if b.len() <= MAX_BINARY => (
            RegKind::Binary,
            SettingValue::Text(b.iter().map(|x| format!("{x:02x}")).collect()),
        ),
        RegValue::Binary(_) => return Err(Why::TooLarge),
        RegValue::Other => return Err(Why::Kind),
    })
}

/// The registry value a neutral value of `kind` is.
pub fn setting_to_reg(kind: RegKind, v: &SettingValue) -> Option<RegValue> {
    Some(match (kind, v) {
        (RegKind::Dword, SettingValue::Int(i)) => RegValue::Dword(u32::try_from(*i).ok()?),
        (RegKind::Qword, SettingValue::Int(i)) => RegValue::Qword(u64::try_from(*i).ok()?),
        (RegKind::Sz, SettingValue::Text(t)) => RegValue::Sz(t.clone()),
        (RegKind::ExpandSz, SettingValue::Text(t)) => RegValue::ExpandSz(t.clone()),
        (RegKind::MultiSz, SettingValue::Text(t)) => {
            RegValue::MultiSz(serde_json::from_str(t).ok()?)
        }
        (RegKind::Binary, SettingValue::Text(t)) => {
            if t.len() % 2 != 0 || t.len() > MAX_BINARY * 2 {
                return None;
            }
            RegValue::Binary(
                (0..t.len())
                    .step_by(2)
                    .map(|i| u8::from_str_radix(t.get(i..i + 2)?, 16).ok())
                    .collect::<Option<Vec<u8>>>()?,
            )
        }
        _ => return None,
    })
}

/// Finds every value in the person's registry trees.
pub fn discover_registry(hive: &dyn Hive) -> Discovery {
    let mut d = Discovery::default();
    let vendors = hive.subkeys("Software").unwrap_or_default();
    for v in vendors {
        let key = format!("Software\\{v}");
        if SYSTEM_VENDORS.contains(&v.to_lowercase().as_str()) {
            let why = store_kind(&key).unwrap_or(Why::SystemStore);
            d.not_moved.push(NotMoved {
                label: v.clone(),
                store: format!("HKCU\\{key}"),
                name: "*".into(),
                why,
            });
            continue;
        }
        walk(hive, &key, &v, 0, &mut d);
    }
    if hive.key_exists("Control Panel").unwrap_or(false) {
        walk(hive, "Control Panel", "Control Panel", 0, &mut d);
    }
    d
}

fn walk(hive: &dyn Hive, key: &str, label: &str, depth: usize, d: &mut Discovery) {
    if depth > MAX_DEPTH || d.found.len() >= MAX_FOUND {
        return;
    }
    if let Some(why) = store_kind(key) {
        d.not_moved.push(NotMoved {
            label: label.into(),
            store: format!("HKCU\\{key}"),
            name: "*".into(),
            why,
        });
        return;
    }
    for name in hive.names(key).unwrap_or_default() {
        if name.is_empty() {
            continue;
        }
        let not = |why| NotMoved {
            label: label.into(),
            store: format!("HKCU\\{key}"),
            name: name.clone(),
            why,
        };
        match hive.get(key, &name) {
            Ok(Some(v)) => match reg_to_setting(&v) {
                Ok((kind, value)) => d.found.push(FoundSetting {
                    address: Address::Registry {
                        key: key.to_string(),
                        name: name.clone(),
                        kind,
                    },
                    label: label.into(),
                    value,
                }),
                Err(why) => d.not_moved.push(not(why)),
            },
            Ok(None) => {}
            Err(e) => d.not_moved.push(not(Why::Unreadable(e))),
        }
    }
    for sub in hive.subkeys(key).unwrap_or_default() {
        let child = format!("{key}\\{sub}");
        let child_label = if depth == 0 && !label.starts_with("Control Panel") {
            format!("{label} {sub}")
        } else if depth == 0 {
            format!("Control Panel {sub}")
        } else {
            label.to_string()
        };
        walk(hive, &child, &child_label, depth + 1, d);
    }
}

/// A preference value as a neutral value.
pub fn pref_to_setting(v: &Plist) -> Result<(PrefKind, SettingValue), Why> {
    Ok(match v {
        Plist::Bool(b) => (PrefKind::Bool, SettingValue::Bool(*b)),
        Plist::Integer(i) => (PrefKind::Int, SettingValue::Int(*i)),
        Plist::Real(r) => (PrefKind::Real, SettingValue::Text(r.to_string())),
        Plist::String(s) => (PrefKind::String, SettingValue::Text(s.clone())),
        Plist::Array(_) | Plist::Dict(_) => (
            PrefKind::Xml,
            SettingValue::Text(plist::to_xml(v).ok_or(Why::Kind)?),
        ),
        Plist::Other => return Err(Why::Kind),
    })
}

/// Finds every preference in every domain the person has.
pub fn discover_mac(m: &mut Mechanisms) -> Discovery {
    let mut d = Discovery::default();
    let mut domains = vec!["NSGlobalDomain".to_string()];
    if let Some(r) = m.runner.as_mut()
        && let Ok(o) = r.run(Tool::Defaults, &["domains"])
        && o.ok
    {
        domains.extend(
            o.stdout
                .split(", ")
                .map(|s| s.trim().to_string())
                .filter(|s| !s.is_empty())
                .take(2000),
        );
    }
    for domain in domains {
        if let Some(why) = store_kind(&domain) {
            d.not_moved.push(NotMoved {
                label: domain.clone(),
                store: format!("defaults:{domain}"),
                name: "*".into(),
                why,
            });
            continue;
        }
        let Some(r) = m.runner.as_mut() else { break };
        let exported = r
            .run(Tool::Defaults, &["export", &domain, "-"])
            .ok()
            .filter(|o| o.ok)
            .and_then(|o| plist::parse(&o.stdout).ok());
        let Some(Plist::Dict(items)) = exported else {
            d.not_moved.push(NotMoved {
                label: domain.clone(),
                store: format!("defaults:{domain}"),
                name: "*".into(),
                why: Why::Unreadable("these preferences could not be read".into()),
            });
            continue;
        };
        for (key, v) in items {
            if d.found.len() >= MAX_FOUND {
                return d;
            }
            match pref_to_setting(&v) {
                Ok((kind, value)) => d.found.push(FoundSetting {
                    address: Address::MacPref {
                        domain: domain.clone(),
                        key,
                        kind,
                    },
                    label: domain.clone(),
                    value,
                }),
                Err(why) => d.not_moved.push(NotMoved {
                    label: domain.clone(),
                    store: format!("defaults:{domain}"),
                    name: key,
                    why,
                }),
            }
        }
    }
    d
}

/// Parses `dconf dump /`: `[path]` sections of `key=value` lines.
pub fn parse_dconf_dump(text: &str) -> Discovery {
    let mut d = Discovery::default();
    let mut path: Option<String> = None;
    for line in text.lines() {
        let line = line.trim_end();
        if let Some(p) = line.strip_prefix('[').and_then(|l| l.strip_suffix(']')) {
            path = Some(if p == "/" {
                "/".into()
            } else {
                format!("/{p}/")
            });
            continue;
        }
        let (Some(p), Some((key, value))) = (&path, line.split_once('=')) else {
            continue;
        };
        if d.found.len() >= MAX_FOUND {
            break;
        }
        let label = p.trim_matches('/').replace('/', " ");
        d.found.push(FoundSetting {
            address: Address::Dconf {
                path: p.clone(),
                key: key.to_string(),
            },
            label,
            value: SettingValue::Text(value.to_string()),
        });
    }
    d
}

pub fn discover_dconf(m: &mut Mechanisms) -> Discovery {
    match m
        .runner
        .as_mut()
        .map(|r| r.run(Tool::Dconf, &["dump", "/"]))
    {
        Some(Ok(o)) if o.ok => parse_dconf_dump(&o.stdout),
        _ => Discovery {
            found: Vec::new(),
            not_moved: vec![NotMoved {
                label: "desktop".into(),
                store: "dconf".into(),
                name: "*".into(),
                why: Why::Unreadable("the desktop's settings could not be read".into()),
            }],
        },
    }
}

/// The value at a found setting's address now (`None` when not set). A value whose type changed
/// reads as a different value, never as the old one.
pub fn read_found(m: &mut Mechanisms, a: &Address) -> Result<Option<SettingValue>, String> {
    match a {
        Address::Registry { key, name, kind } => {
            let h = m.user.as_deref().ok_or("this laptop has no registry")?;
            Ok(h.get(key, name)?.map(|v| match reg_to_setting(&v) {
                Ok((k, s)) if k == *kind => s,
                _ => SettingValue::Text(format!("(a different kind of value: {})", kind_word(&v))),
            }))
        }
        Address::MacPref { domain, key, kind } => {
            let r = m.runner.as_mut().ok_or("no preferences tool")?;
            let o = r.run(Tool::Defaults, &["export", domain, "-"])?;
            if !o.ok {
                return Err("the preferences could not be read".into());
            }
            let p = plist::parse(&o.stdout)?;
            Ok(p.get(key).map(|v| match pref_to_setting(v) {
                Ok((k, s)) if k == *kind => s,
                _ => SettingValue::Text("(a different kind of value)".into()),
            }))
        }
        Address::Dconf { path, key } => {
            let r = m.runner.as_mut().ok_or("no desktop tools")?;
            let full = format!("{path}{key}");
            let o = r.run(Tool::Dconf, &["read", &full])?;
            if !o.ok {
                return Err("the desktop setting could not be read".into());
            }
            let t = o.stdout.trim();
            Ok((!t.is_empty()).then(|| SettingValue::Text(t.to_string())))
        }
    }
}

fn kind_word(v: &RegValue) -> &'static str {
    match v {
        RegValue::Dword(_) => "number",
        RegValue::Qword(_) => "large number",
        RegValue::Sz(_) | RegValue::ExpandSz(_) => "text",
        RegValue::MultiSz(_) => "list",
        RegValue::Binary(_) => "data",
        RegValue::Other => "other",
    }
}

/// Writes a found setting's value (`None` removes it). Registry keys PCTwin makes are added to
/// `created`, so undo can remove them again once they are empty.
pub fn write_found(
    m: &mut Mechanisms,
    a: &Address,
    v: Option<&SettingValue>,
    created: &mut Vec<String>,
) -> Result<(), WriteError> {
    if !a.writable() {
        return Err(WriteError::Failed("PCTwin never writes this place".into()));
    }
    let wrong = || WriteError::Failed("a value of the wrong kind".into());
    match a {
        Address::Registry { key, name, kind } => {
            let h = m
                .user
                .as_deref_mut()
                .ok_or_else(|| WriteError::Failed("this laptop has no registry".into()))?;
            match v {
                None => h.delete(key, name),
                Some(v) => {
                    let raw = setting_to_reg(*kind, v).ok_or_else(wrong)?;
                    // Each missing key on the way, deepest last.
                    let mut path = String::new();
                    for part in key.split('\\') {
                        if !path.is_empty() {
                            path.push('\\');
                        }
                        path.push_str(part);
                        if !h.key_exists(&path).map_err(WriteError::Failed)?
                            && !created.contains(&path)
                        {
                            created.push(path.clone());
                        }
                    }
                    h.set(key, name, &raw)
                }
            }
        }
        Address::MacPref { domain, key, kind } => {
            let r = m
                .runner
                .as_mut()
                .ok_or_else(|| WriteError::Failed("no preferences tool".into()))?;
            let ok = match v {
                None => {
                    r.run(Tool::Defaults, &["delete", domain, key])
                        .map_err(WriteError::Failed)?;
                    true
                }
                Some(v) => {
                    let (flag, text) = match (kind, v) {
                        (PrefKind::Bool, SettingValue::Bool(b)) => (Some("-bool"), b.to_string()),
                        (PrefKind::Int, SettingValue::Int(i)) => (Some("-int"), i.to_string()),
                        (PrefKind::Real, SettingValue::Text(t)) => {
                            t.parse::<f64>().map_err(|_| wrong())?;
                            (Some("-float"), t.clone())
                        }
                        (PrefKind::String, SettingValue::Text(t)) => (Some("-string"), t.clone()),
                        (PrefKind::Xml, SettingValue::Text(t)) => {
                            plist::from_xml(t).map_err(|_| wrong())?;
                            (None, t.clone())
                        }
                        _ => return Err(wrong()),
                    };
                    let mut args = vec!["write", domain.as_str(), key.as_str()];
                    if let Some(f) = flag {
                        args.push(f);
                    }
                    args.push(&text);
                    r.run(Tool::Defaults, &args).map_err(WriteError::Failed)?.ok
                }
            };
            if ok {
                Ok(())
            } else {
                Err(WriteError::Failed(
                    "the preferences were not changed".into(),
                ))
            }
        }
        Address::Dconf { path, key } => {
            let r = m
                .runner
                .as_mut()
                .ok_or_else(|| WriteError::Failed("no desktop tools".into()))?;
            let full = format!("{path}{key}");
            let o = match v {
                None => r.run(Tool::Dconf, &["reset", &full]),
                Some(SettingValue::Text(t)) => {
                    // Only a value that is GVariant text dconf printed: never passed to a shell.
                    if !text_ok(t) || t.is_empty() {
                        return Err(wrong());
                    }
                    r.run(Tool::Dconf, &["write", &full, t])
                }
                Some(_) => return Err(wrong()),
            }
            .map_err(WriteError::Failed)?;
            if o.ok {
                Ok(())
            } else {
                Err(WriteError::Failed(
                    "the desktop setting was not changed".into(),
                ))
            }
        }
    }
}

/// The text a dconf value holds when it is one string (for the classifier), and how to put a
/// new string back.
pub fn dconf_string(v: &str) -> Option<String> {
    match gvariant::parse(v) {
        Ok(gvariant::GValue::Str(s)) => Some(s),
        _ => None,
    }
}

/// Found settings grouped by label, for one recipe group each.
pub fn by_label(found: &[FoundSetting]) -> BTreeMap<String, Vec<&FoundSetting>> {
    let mut m: BTreeMap<String, Vec<&FoundSetting>> = BTreeMap::new();
    for f in found {
        m.entry(f.label.clone()).or_default().push(f);
    }
    m
}
