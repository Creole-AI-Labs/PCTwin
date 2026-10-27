//! Planning the found settings: same system, copied through the generic mechanisms after the
//! generic rules ([`crate::classify`]); a different system, only through catalogue rows. The
//! catalogue enriches (friendly labels, exceptions for one app, known places it handles itself);
//! it never decides what is found.

use std::collections::BTreeMap;

use pctwin_recipe::{Arch, Os, Platform, Recipe, RecipeId, SettingValue, Step};

use crate::apps::glob;
use crate::catalogue::ActionHint;
use crate::catalogue::{Catalogue, Place, System};
use crate::classify::{Ctx, Fate, Why, action_kind, classify_as, may_write_text_with};
use crate::found::{Address, Discovery, FoundSetting, NotMoved, PrefKind, RegKind, dconf_string};
use crate::gvariant;
use crate::judge::{Facts, Kind};
use crate::map::PlanError;

/// The most steps one recipe holds (the recipe format's own limit).
pub const STEPS_PER_RECIPE: usize = 200;

/// One found setting that moves.
#[derive(Debug, Clone, PartialEq)]
pub struct Move {
    pub key: String,
    pub address: Address,
    pub label: String,
    pub value: SettingValue,
    /// Its paths were pointed at the moved files.
    pub rewritten: bool,
}

/// The plan for every found setting.
#[derive(Debug, Clone, Default, PartialEq)]
pub struct FoundPlan {
    pub moves: Vec<Move>,
    pub not_moved: Vec<NotMoved>,
    /// Found settings a catalogue row handles instead (with its translation or better route).
    pub by_catalogue: usize,
}

/// Plans every found setting.
pub fn plan_found(
    cat: &Catalogue,
    from: System,
    to: System,
    d: &Discovery,
    ctx: &Ctx<'_>,
) -> FoundPlan {
    let mut p = FoundPlan {
        not_moved: d.not_moved.clone(),
        ..FoundPlan::default()
    };
    for f in &d.found {
        if covered(cat, from, &f.address) {
            p.by_catalogue += 1;
            continue;
        }
        let label = label_of(cat, from, f);
        let store = f.address.store().to_string();
        let name = f.address.name().to_string();
        let not = |why| NotMoved {
            label: label.clone(),
            store: store.clone(),
            name: name.clone(),
            why,
        };
        if let Some(reason) = exception(cat, from, &f.address) {
            p.not_moved.push(not(Why::Exception(reason)));
            continue;
        }
        if from != to {
            p.not_moved.push(not(Why::OtherSystem));
            continue;
        }
        if !f.address.writable() {
            let why = match &f.address {
                Address::Registry { name, .. } if name.is_empty() => Why::Unnamed,
                _ => crate::classify::store_kind(&store).unwrap_or(Why::SystemStore),
            };
            p.not_moved.push(not(why));
            continue;
        }
        match fate(f, ctx) {
            Fate::Copy(v) => p.moves.push(Move {
                key: f.address.key(),
                address: f.address.clone(),
                label,
                value: v,
                rewritten: false,
            }),
            Fate::Rewritten(v) => p.moves.push(Move {
                key: f.address.key(),
                address: f.address.clone(),
                label,
                value: v,
                rewritten: true,
            }),
            Fate::NotMoved(why) => p.not_moved.push(not(why)),
        }
    }
    p.not_moved.sort();
    p
}

fn fate(f: &FoundSetting, ctx: &Ctx<'_>) -> Fate {
    let (store, name) = (f.address.store(), f.address.name());
    let kind = kind_of(&f.address);
    match (&f.address, kind) {
        // A dconf string is judged by its text; a rewritten one is quoted again.
        (Address::Dconf { .. }, _) => {
            let SettingValue::Text(t) = &f.value else {
                return Fate::NotMoved(Why::Kind);
            };
            match dconf_string(t) {
                Some(inner) => {
                    match classify_as(Kind::Text, store, name, &SettingValue::Text(inner), ctx) {
                        Fate::Copy(_) => Fate::Copy(f.value.clone()),
                        Fate::Rewritten(SettingValue::Text(n)) => {
                            Fate::Rewritten(SettingValue::Text(gvariant::quote(&n)))
                        }
                        Fate::Rewritten(_) => Fate::NotMoved(Why::Kind),
                        Fate::NotMoved(why) => Fate::NotMoved(why),
                    }
                }
                None => classify_as(Kind::GVariant, store, name, &f.value, ctx),
            }
        }
        (
            Address::MacPref {
                kind: PrefKind::Xml,
                ..
            },
            _,
        ) if !matches!(f.value, SettingValue::Text(_)) => Fate::NotMoved(Why::Kind),
        _ => classify_as(kind, store, name, &f.value, ctx),
    }
}

/// How a value at `a` is decoded ([`crate::judge::texts`]).
pub fn kind_of(a: &Address) -> Kind {
    match a {
        Address::Registry {
            kind: RegKind::MultiSz,
            ..
        } => Kind::List,
        Address::Registry {
            kind: RegKind::Binary,
            ..
        } => Kind::Bytes,
        Address::Registry { .. } => Kind::Text,
        Address::MacPref {
            kind: PrefKind::Xml,
            ..
        } => Kind::Plist,
        Address::MacPref { .. } => Kind::Text,
        Address::Dconf { .. } => Kind::GVariant,
    }
}

/// The check at the moment of writing on the new laptop (fourth review, finding F1; fifth
/// review, findings 1 to 3), by this laptop's own facts: [`may_write_with`].
pub fn may_write(hints: &[ActionHint], a: &Address, v: &SettingValue) -> Result<(), Why> {
    may_write_with(Facts::system(), hints, a, v)
}

/// The place must be in discovery's scope, and every text the value holds, decoded as planning
/// decoded it ([`crate::judge::texts`]), must pass the same rule
/// ([`crate::classify::may_write_text_with`]).
pub fn may_write_with(
    facts: &Facts,
    hints: &[ActionHint],
    a: &Address,
    v: &SettingValue,
) -> Result<(), Why> {
    if !a.writable() {
        return Err(Why::SystemStore);
    }
    let (store, name) = (a.store(), a.name());
    let kind = kind_of(a);
    if kind == Kind::Bytes && action_kind(hints, store, name).is_some() {
        return Err(Why::StartsProgram);
    }
    if let SettingValue::Text(t) = v
        && t.len() > crate::classify::MAX_VALUE
    {
        return Err(Why::TooLarge);
    }
    for t in crate::judge::texts(kind, v)? {
        may_write_text_with(facts, hints, store, name, &t)?;
    }
    Ok(())
}

/// Whether a catalogue row reads this address on `system` (it then moves through the row).
pub fn covered(cat: &Catalogue, system: System, a: &Address) -> bool {
    cat.settings
        .iter()
        .filter_map(|r| r.places.get(&system))
        .any(|place| match (place, a) {
            (Place::Registry { key, fields, .. }, Address::Registry { key: k, name, .. }) => {
                key.eq_ignore_ascii_case(k)
                    && fields
                        .iter()
                        .any(|f| f.name.eq_ignore_ascii_case(name) || f.kind.is_list())
            }
            (Place::MacPref { domain, fields, .. }, Address::MacPref { domain: d, key, .. }) => {
                domain == d && fields.iter().any(|f| &f.name == key)
            }
            (Place::Gsettings { schema, fields, .. }, Address::Dconf { path, key }) => {
                *path == format!("/{}/", schema.replace('.', "/"))
                    && fields.iter().any(|f| &f.name == key)
            }
            _ => false,
        })
}

fn exception(cat: &Catalogue, system: System, a: &Address) -> Option<String> {
    cat.exceptions
        .iter()
        .find(|e| e.system == system && glob(&e.store, a.store()) && glob(&e.name, a.name()))
        .map(|e| e.reason.clone())
}

fn label_of(cat: &Catalogue, system: System, f: &FoundSetting) -> String {
    cat.labels
        .iter()
        .find(|l| l.system == system && glob(&l.store, f.address.store()))
        .map(|l| l.label.clone())
        .unwrap_or_else(|| f.label.clone())
}

impl FoundPlan {
    /// Every move's address by its executor key: what the new laptop's store must be given
    /// ([`crate::store::Store::remember`]) to apply, read back and undo them.
    pub fn book(&self) -> BTreeMap<String, Address> {
        self.moves
            .iter()
            .map(|m| (m.key.clone(), m.address.clone()))
            .collect()
    }

    /// The executor recipes: one group of `apply-setting` steps per label, at most
    /// [`STEPS_PER_RECIPE`] each.
    pub fn recipes(&self, os: Os, revision: u64) -> Result<Vec<(String, Recipe)>, PlanError> {
        let version = u32::try_from(revision).map_err(|_| PlanError::Recipe("revision".into()))?;
        let mut by_label: BTreeMap<&str, Vec<&Move>> = BTreeMap::new();
        for m in &self.moves {
            by_label.entry(m.label.as_str()).or_default().push(m);
        }
        let platforms: Vec<Platform> = [Arch::X86_64, Arch::Aarch64]
            .into_iter()
            .map(|arch| Platform { os, arch })
            .collect();
        let mut out = Vec::new();
        for (label, moves) in by_label {
            let d = blake3::derive_key("PCTwin found label 2026-10-27 v1", label.as_bytes());
            let tag: String = d[..8].iter().map(|b| format!("{b:02x}")).collect();
            for (n, chunk) in moves.chunks(STEPS_PER_RECIPE).enumerate() {
                let id = RecipeId::new(&format!("pctwin.settings.found.{tag}.{n}"))?;
                let steps = chunk
                    .iter()
                    .map(|m| Step::ApplySetting {
                        key: m.key.clone(),
                        value: m.value.clone(),
                    })
                    .collect();
                out.push((
                    label.to_string(),
                    Recipe::new(id, version, platforms.clone(), steps)?,
                ));
            }
        }
        Ok(out)
    }
}
