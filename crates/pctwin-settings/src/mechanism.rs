//! The generic mechanisms: read and write a catalogue place's fields on this laptop. One
//! implementation per kind of store (registry value, Mac preference, `gsettings` key, KDE config
//! key) and a few named special reads; no code per setting.

use std::collections::BTreeMap;
use std::path::PathBuf;

use crate::catalogue::{
    Field, Hive as Root, Kind, Place, Probe, RegPolicy, Special, Store, System,
};
use crate::codec::Native;
use crate::gvariant::{self, GValue};
use crate::plist::{self, Plist};
use crate::registry::{Hive, RegValue, WriteError};
use crate::runner::{Runner, Tool};

/// Why a place could not be read.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ReadError {
    /// This laptop has no such mechanism (a registry place on a Mac).
    NotHere,
    /// It could not be read, and why (plain words).
    Failed(String),
}

impl From<String> for ReadError {
    fn from(s: String) -> Self {
        ReadError::Failed(s)
    }
}

/// The mechanisms of one laptop.
pub struct Mechanisms {
    pub system: System,
    /// The person's registry (Windows).
    pub user: Option<Box<dyn Hive>>,
    /// The machine's registry (Windows), read only.
    pub machine: Option<Box<dyn Hive>>,
    /// The system's tools (Mac, Linux; Windows for Wi-Fi).
    pub runner: Option<Box<dyn Runner>>,
    /// Where the time zone link is (`/etc/localtime`).
    pub localtime: Option<PathBuf>,
}

impl Mechanisms {
    /// This laptop's real mechanisms, for `system`.
    pub fn real(system: System) -> Self {
        #[cfg(windows)]
        {
            Self {
                system,
                user: Some(Box::new(crate::registry::WinHive::current_user())),
                machine: Some(Box::new(crate::registry::WinHive::local_machine())),
                runner: Some(Box::new(crate::runner::SystemRunner)),
                localtime: None,
            }
        }
        #[cfg(not(windows))]
        {
            Self {
                system,
                user: None,
                machine: None,
                runner: Some(Box::new(crate::runner::SystemRunner)),
                localtime: Some(PathBuf::from("/etc/localtime")),
            }
        }
    }

    fn hive(&self, root: Root) -> Result<&dyn Hive, ReadError> {
        match root {
            Root::User => self.user.as_deref(),
            Root::Machine => self.machine.as_deref(),
        }
        .ok_or(ReadError::NotHere)
    }

    fn run(&mut self, tool: Tool, args: &[&str]) -> Result<crate::runner::Output, ReadError> {
        let r = self.runner.as_mut().ok_or(ReadError::NotHere)?;
        Ok(r.run(tool, args)?)
    }

    /// Each field's value now (`None` where not set).
    pub fn read(&mut self, place: &Place) -> Result<Vec<Option<Native>>, ReadError> {
        match place {
            Place::Registry {
                hive, key, fields, ..
            } => {
                let h = self.hive(*hive)?;
                fields.iter().map(|f| reg_read(h, key, f)).collect()
            }
            Place::MacPref { domain, fields, .. } => {
                let p = self.mac_export(domain)?;
                fields.iter().map(|f| Ok(mac_field(&p, f))).collect()
            }
            Place::Gsettings { schema, fields, .. } => {
                let mut out = Vec::new();
                for f in fields {
                    let o = self.run(Tool::Gsettings, &["get", schema, &f.name])?;
                    if !o.ok {
                        return Err(ReadError::Failed(
                            "this desktop does not have this setting".into(),
                        ));
                    }
                    out.push(g_field(&gvariant::parse(&o.stdout)?, f.kind));
                }
                Ok(out)
            }
            Place::KdeConfig {
                file,
                group,
                fields,
                ..
            } => {
                let mut out = Vec::new();
                for f in fields {
                    let args = ["--file", file.as_str(), "--group", group, "--key", &f.name];
                    let o = match self.run(Tool::Kreadconfig6, &args) {
                        Ok(o) if o.ok => o,
                        _ => self.run(Tool::Kreadconfig5, &args)?,
                    };
                    let t = o.stdout.trim().to_string();
                    out.push(if !o.ok || t.is_empty() {
                        None
                    } else {
                        match f.kind {
                            Kind::Bool => t.parse().ok().map(Native::Bool),
                            Kind::Int => t.parse().ok().map(Native::Int),
                            _ => Some(Native::Text(t)),
                        }
                    });
                }
                Ok(out)
            }
            Place::Special { special, .. } => self.special(special).map(|v| vec![v]),
        }
    }

    fn special(&mut self, s: &Special) -> Result<Option<Native>, ReadError> {
        match s {
            Special::MacUrlHandler(scheme) => {
                let p =
                    self.mac_export("com.apple.LaunchServices/com.apple.launchservices.secure")?;
                let Some(Plist::Array(handlers)) = p.get("LSHandlers") else {
                    return Ok(None);
                };
                Ok(handlers
                    .iter()
                    .find(|h| h.get("LSHandlerURLScheme") == Some(&Plist::String(scheme.clone())))
                    .and_then(|h| match h.get("LSHandlerRoleAll") {
                        Some(Plist::String(id)) => Some(Native::Text(id.clone())),
                        _ => None,
                    }))
            }
            Special::XdgDefaultBrowser => {
                let o = self.run(Tool::XdgSettings, &["get", "default-web-browser"])?;
                let t = o.stdout.trim();
                Ok((o.ok && !t.is_empty()).then(|| Native::Text(t.to_string())))
            }
            Special::LocaltimeLink => {
                let path = self.localtime.clone().ok_or(ReadError::NotHere)?;
                let link = std::fs::read_link(&path)
                    .map_err(|_| ReadError::Failed("the time zone could not be read".into()))?;
                let s = link.to_string_lossy();
                Ok(s.split_once("zoneinfo/")
                    .map(|(_, z)| Native::Text(z.to_string())))
            }
        }
    }

    fn mac_export(&mut self, domain: &str) -> Result<Plist, ReadError> {
        let o = self.run(Tool::Defaults, &["export", domain, "-"])?;
        if !o.ok {
            return Err(ReadError::Failed(
                "the preferences could not be read".into(),
            ));
        }
        Ok(plist::parse(&o.stdout)?)
    }

    /// Writes each field (`None` removes it).
    pub fn write(&mut self, place: &Place, values: &[Option<Native>]) -> Result<(), WriteError> {
        if place.read_only() {
            return Err(WriteError::Failed("this place is only ever read".into()));
        }
        let fields = place.fields();
        if values.len() != fields.len() {
            return Err(WriteError::Failed("the wrong number of values".into()));
        }
        let not_here = || WriteError::Failed("this laptop has no such settings store".into());
        match place {
            Place::Registry { hive, key, .. } => {
                if *hive != Root::User {
                    return Err(WriteError::Denied);
                }
                let h = self.user.as_deref_mut().ok_or_else(not_here)?;
                for (f, v) in fields.iter().zip(values) {
                    match v {
                        None => h.delete(key, &f.name)?,
                        Some(n) => h.set(key, &f.name, &reg_value(f.kind, n)?)?,
                    }
                }
                Ok(())
            }
            Place::MacPref { domain, .. } => {
                let r = self.runner.as_mut().ok_or_else(not_here)?;
                for (f, v) in fields.iter().zip(values) {
                    let ok = match v {
                        None => {
                            // Removing a key that is not there fails harmlessly; the read-back
                            // decides.
                            r.run(Tool::Defaults, &["delete", domain, &f.name])
                                .map_err(WriteError::Failed)?;
                            true
                        }
                        Some(n) => {
                            let (flag, text) = mac_arg(f.kind, n)?;
                            r.run(Tool::Defaults, &["write", domain, &f.name, flag, &text])
                                .map_err(WriteError::Failed)?
                                .ok
                        }
                    };
                    if !ok {
                        return Err(WriteError::Failed(
                            "the preferences were not changed".into(),
                        ));
                    }
                }
                Ok(())
            }
            Place::Gsettings { schema, .. } => {
                for (f, v) in fields.iter().zip(values) {
                    let ok = match v {
                        None => {
                            let r = self.runner.as_mut().ok_or_else(not_here)?;
                            r.run(Tool::Gsettings, &["reset", schema, &f.name])
                                .map_err(WriteError::Failed)?
                                .ok
                        }
                        Some(n) => {
                            let text = match (f.kind, n) {
                                (Kind::SourceList, Native::List(xkb)) => {
                                    self.merged_sources(schema, &f.name, xkb)?
                                }
                                _ => gvariant::format(&g_value(f.kind, n)?),
                            };
                            let r = self.runner.as_mut().ok_or_else(not_here)?;
                            r.run(Tool::Gsettings, &["set", schema, &f.name, &text])
                                .map_err(WriteError::Failed)?
                                .ok
                        }
                    };
                    if !ok {
                        return Err(WriteError::Failed(
                            "the desktop setting was not changed".into(),
                        ));
                    }
                }
                Ok(())
            }
            Place::KdeConfig { .. } | Place::Special { .. } => {
                Err(WriteError::Failed("this place is only ever read".into()))
            }
        }
    }

    /// The input sources with exactly `xkb` as their keyboard layouts: every other source (an
    /// input method) stays where it is, layouts already there keep their order, new ones go at
    /// the end, so the source the person types with first never changes.
    fn merged_sources(
        &mut self,
        schema: &str,
        key: &str,
        xkb: &[String],
    ) -> Result<String, WriteError> {
        let r = self
            .runner
            .as_mut()
            .ok_or_else(|| WriteError::Failed("no desktop tools".into()))?;
        let o = r
            .run(Tool::Gsettings, &["get", schema, key])
            .map_err(WriteError::Failed)?;
        let now = match gvariant::parse(&o.stdout) {
            Ok(GValue::Pairs(p)) if o.ok => p,
            _ => {
                return Err(WriteError::Failed(
                    "the input sources could not be read".into(),
                ));
            }
        };
        let mut out: Vec<(String, String)> = now
            .into_iter()
            .filter(|(kind, id)| kind != "xkb" || xkb.contains(id))
            .collect();
        for id in xkb {
            if !out.iter().any(|(k, i)| k == "xkb" && i == id) {
                out.push(("xkb".into(), id.clone()));
            }
        }
        Ok(gvariant::format(&GValue::Pairs(out)))
    }

    /// Whether the organisation manages this place on this laptop.
    pub fn managed(&mut self, place: &Place) -> Result<bool, String> {
        match place {
            Place::Registry { policy, .. } => {
                for RegPolicy { hive, key, value } in policy {
                    let h = match self.hive(*hive) {
                        Ok(h) => h,
                        Err(_) => continue,
                    };
                    if h.get(key, value)?.is_some() {
                        return Ok(true);
                    }
                }
                Ok(false)
            }
            Place::Gsettings { schema, fields, .. } => {
                for f in fields {
                    let o = self
                        .run(Tool::Gsettings, &["writable", schema, &f.name])
                        .map_err(|e| format!("{e:?}"))?;
                    if o.ok && o.stdout.trim() == "false" {
                        return Ok(true);
                    }
                }
                Ok(false)
            }
            _ => Ok(false),
        }
    }

    /// The names of every setting in a store (never their values), to find the ones the
    /// catalogue has no row for yet.
    pub fn store_names(&mut self, store: &Store) -> Result<Vec<String>, ReadError> {
        match store {
            Store::Registry { hive, key } => {
                let h = self.hive(*hive)?;
                Ok(h.names(key)?
                    .into_iter()
                    .filter(|n| !n.is_empty())
                    .collect())
            }
            Store::MacPref { domain } => Ok(self.mac_export(domain)?.keys()),
            Store::Gsettings { schema } => {
                let o = self.run(Tool::Gsettings, &["list-keys", schema])?;
                if !o.ok {
                    return Err(ReadError::Failed(
                        "this desktop does not have this group".into(),
                    ));
                }
                Ok(o.stdout
                    .lines()
                    .map(str::trim)
                    .filter(|l| !l.is_empty())
                    .map(str::to_string)
                    .collect())
            }
        }
    }

    /// Whether a protected thing is there (an approval given, a protection switched off).
    pub fn probe(&mut self, probe: &Probe) -> Result<bool, ReadError> {
        match probe {
            Probe::RegistryConsent { key } => {
                let h = self.hive(Root::User)?;
                let allowed = |k: &str| -> Result<bool, String> {
                    for sub in h.subkeys(k)? {
                        let path = format!("{k}\\{sub}");
                        if h.get(&path, "Value")? == Some(RegValue::Sz("Allow".into())) {
                            return Ok(true);
                        }
                    }
                    Ok(false)
                };
                Ok(allowed(key)? || allowed(&format!("{key}\\NonPackaged"))?)
            }
            Probe::RegistryEquals {
                hive,
                key,
                value,
                equals,
            } => {
                let h = self.hive(*hive)?;
                Ok(match (h.get(key, value)?, equals) {
                    (Some(RegValue::Dword(d)), serde_json::Value::Number(n)) => {
                        n.as_u64() == Some(u64::from(d))
                    }
                    (Some(RegValue::Sz(s)), serde_json::Value::String(e)) => {
                        s.eq_ignore_ascii_case(e)
                    }
                    _ => false,
                })
            }
            Probe::GatekeeperOff => {
                let o = self.run(Tool::Spctl, &["--status"])?;
                Ok(o.stdout.contains("assessments disabled"))
            }
        }
    }
}

fn reg_read(h: &dyn Hive, key: &str, f: &Field) -> Result<Option<Native>, ReadError> {
    let unknown = || ReadError::Failed("a value PCTwin does not know".into());
    match f.kind {
        Kind::ValueNames => {
            let names: Vec<String> = h
                .names(key)?
                .into_iter()
                .filter(|n| !n.is_empty())
                .collect();
            return Ok((!names.is_empty()).then_some(Native::List(names)));
        }
        Kind::NumberedValues => {
            let mut numbered: BTreeMap<u32, String> = BTreeMap::new();
            for n in h.names(key)? {
                if let Ok(i) = n.parse::<u32>()
                    && let Some(RegValue::Sz(s)) = h.get(key, &n)?
                {
                    numbered.insert(i, s);
                }
            }
            return Ok(
                (!numbered.is_empty()).then(|| Native::List(numbered.into_values().collect()))
            );
        }
        _ => {}
    }
    let Some(v) = h.get(key, &f.name)? else {
        return Ok(None);
    };
    Ok(Some(match (f.kind, v) {
        (Kind::Dword, RegValue::Dword(d)) => Native::Int(i64::from(d)),
        (Kind::Sz, RegValue::Sz(s)) => Native::Text(s),
        (Kind::SzInt, RegValue::Sz(s)) => Native::Int(s.trim().parse().map_err(|_| unknown())?),
        (Kind::SzInt, RegValue::Dword(d)) => Native::Int(i64::from(d)),
        _ => return Err(unknown()),
    }))
}

fn reg_value(kind: Kind, n: &Native) -> Result<RegValue, WriteError> {
    let wrong = || WriteError::Failed("a value of the wrong kind".into());
    Ok(match (kind, n) {
        (Kind::Dword, Native::Int(i)) => RegValue::Dword(u32::try_from(*i).map_err(|_| wrong())?),
        (Kind::Dword, Native::Bool(b)) => RegValue::Dword(u32::from(*b)),
        (Kind::Sz, Native::Text(t)) => RegValue::Sz(t.clone()),
        (Kind::SzInt, Native::Int(i)) => RegValue::Sz(i.to_string()),
        _ => return Err(wrong()),
    })
}

fn mac_field(p: &Plist, f: &Field) -> Option<Native> {
    let v = p.get(&f.name)?;
    match (f.kind, v) {
        (Kind::Bool, Plist::Bool(b)) => Some(Native::Bool(*b)),
        (Kind::Bool, Plist::Integer(i @ (0 | 1))) => Some(Native::Bool(*i == 1)),
        (Kind::Int, Plist::Integer(i)) => Some(Native::Int(*i)),
        (Kind::Float, Plist::Real(r)) => Some(Native::Float(*r)),
        (Kind::Float, Plist::Integer(i)) => Some(Native::Float(*i as f64)),
        (Kind::String, Plist::String(s)) => Some(Native::Text(s.clone())),
        (Kind::DictList, Plist::Array(items)) => {
            let (item, wk, wv) = (
                f.item.as_deref()?,
                f.where_key.as_deref()?,
                f.where_value.as_deref()?,
            );
            let list: Vec<String> = items
                .iter()
                .filter(|d| d.get(wk) == Some(&Plist::String(wv.to_string())))
                .filter_map(|d| match d.get(item) {
                    Some(Plist::String(s)) => Some(s.clone()),
                    _ => None,
                })
                .collect();
            Some(Native::List(list))
        }
        _ => None,
    }
}

fn mac_arg(kind: Kind, n: &Native) -> Result<(&'static str, String), WriteError> {
    Ok(match (kind, n) {
        (Kind::Bool, Native::Bool(b)) => ("-bool", b.to_string()),
        (Kind::Int, Native::Int(i)) => ("-int", i.to_string()),
        (Kind::Float, Native::Float(f)) if f.is_finite() => ("-float", f.to_string()),
        (Kind::Float, Native::Int(i)) => ("-float", i.to_string()),
        (Kind::String, Native::Text(t)) => ("-string", t.clone()),
        _ => return Err(WriteError::Failed("a value of the wrong kind".into())),
    })
}

fn g_field(v: &GValue, kind: Kind) -> Option<Native> {
    match (kind, v) {
        (Kind::Bool, GValue::Bool(b)) => Some(Native::Bool(*b)),
        (Kind::Int, GValue::Int(i)) => Some(Native::Int(*i)),
        (Kind::Float, GValue::Float(f)) => Some(Native::Float(*f)),
        (Kind::Float, GValue::Int(i)) => Some(Native::Float(*i as f64)),
        (Kind::String, GValue::Str(s)) => Some(Native::Text(s.clone())),
        (Kind::SourceList, GValue::Pairs(p)) => Some(Native::List(
            p.iter()
                .filter(|(k, _)| k == "xkb")
                .map(|(_, id)| id.clone())
                .collect(),
        )),
        _ => None,
    }
}

fn g_value(kind: Kind, n: &Native) -> Result<GValue, WriteError> {
    Ok(match (kind, n) {
        (Kind::Bool, Native::Bool(b)) => GValue::Bool(*b),
        (Kind::Int, Native::Int(i)) => GValue::Int(*i),
        (Kind::Float, Native::Float(f)) if f.is_finite() => GValue::Float(*f),
        (Kind::Float, Native::Int(i)) => GValue::Float(*i as f64),
        (Kind::String, Native::Text(t)) => GValue::Str(t.clone()),
        _ => return Err(WriteError::Failed("a value of the wrong kind".into())),
    })
}
