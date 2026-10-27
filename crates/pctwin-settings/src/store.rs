//! Reading on the old laptop and applying on the new one, row by row, through the generic
//! mechanisms (Phase 2.4 plan sections 5 to 7).
//!
//! - [`Store::capture`] reads every catalogue row this system has a place for, notes protected
//!   things, and lists settings found in known stores that have no row yet (T24-013, T24-014,
//!   T24-023). Nothing outside the catalogue's places and stores is read.
//! - [`Store::apply`] is what the executor's adapter calls for an `apply-setting` step: the
//!   setting must be a catalogue row with a writable native place on this system, the value must
//!   fit the row, the grant must name exactly this step of the approved plan and still be live,
//!   and the setting must not be managed. Then the place is written. The executor reads it back
//!   and records the receipt; a write the system accepted but did not keep fails (T24-025).
//! - [`Store::write_if`] is undo's compare-and-write: written only if the setting still holds
//!   what PCTwin set, so a later change by the person is kept (T24-019).
//! - [`SettingsAdapter`] is an executor adapter for a run of settings only.

use std::collections::{BTreeMap, BTreeSet};

use pctwin_executor::{Adapter, SettingWrite, files, object_of};
use pctwin_grant::{InstallGrant, Now, SettingGrant};
use pctwin_journal::{Presence, RestorePoint, SettingsFile};
use pctwin_recipe::{Catalog, Check, Platform, SettingValue, SettingsPlace, Step};
use pctwin_types::Reason;

use crate::catalogue::{Catalogue, Hive, Kind, Place, Route, Row, Store as StoreRef, System};
use crate::codec::{self, Native};
use crate::found::{Address, read_found, write_found};
use crate::map::{Capture, Found, Reading, Target};
use crate::mechanism::{Mechanisms, ReadError};
use crate::registry::WriteError;
use crate::value::ValueType;

/// One laptop's settings: catalogue rows, and found settings by their addresses.
pub struct Store {
    cat: Catalogue,
    mech: Mechanisms,
    /// Found settings' addresses by executor key (from the move's plan, kept with it).
    book: BTreeMap<String, Address>,
    /// Registry keys PCTwin made, in the order made.
    created: Vec<String>,
}

impl Store {
    pub fn new(cat: Catalogue, mech: Mechanisms) -> Self {
        Self {
            cat,
            mech,
            book: BTreeMap::new(),
            created: Vec::new(),
        }
    }

    /// Learns the found settings' addresses (the move's plan's book). An address whose key is
    /// not its own digest, or that may never be written, is refused.
    pub fn remember(&mut self, book: BTreeMap<String, Address>) -> Result<(), String> {
        for (k, a) in &book {
            if a.key() != *k || !a.writable() {
                return Err("a found setting's place is not allowed".into());
            }
        }
        self.book.extend(book);
        Ok(())
    }

    /// Registry keys PCTwin made (kept with the move, so undo can remove them once empty).
    pub fn created_keys(&self) -> &[String] {
        &self.created
    }

    /// After undo: removes the keys PCTwin made that are empty again, deepest first. Returns
    /// the ones removed. A key the person has put anything in stays.
    pub fn prune_created(&mut self) -> Vec<String> {
        let mut removed = Vec::new();
        let mut keys = self.created.clone();
        keys.sort_by_key(|k| std::cmp::Reverse(k.matches('\\').count()));
        if let Some(h) = self.mech.user.as_deref_mut() {
            for k in keys {
                if h.delete_key_if_empty(&k) == Ok(true) {
                    removed.push(k);
                }
            }
        }
        self.created.retain(|k| !removed.contains(k));
        removed
    }

    fn found(&self, key: &str) -> Option<Address> {
        self.book.get(key).filter(|a| a.key() == key).cloned()
    }

    pub fn catalogue(&self) -> &Catalogue {
        &self.cat
    }

    pub fn system(&self) -> System {
        self.mech.system
    }

    pub fn mechanisms(&mut self) -> &mut Mechanisms {
        &mut self.mech
    }

    fn place(&self, row: &Row) -> Option<Place> {
        row.places.get(&self.mech.system).cloned()
    }

    /// One row's reading on this laptop.
    pub fn reading(&mut self, row: &Row) -> Reading {
        if row.not_applicable.contains(&self.mech.system) {
            return Reading::NotApplicable;
        }
        let Some(place) = self.place(row) else {
            return Reading::NotBuilt;
        };
        match self.mech.managed(&place) {
            Ok(true) => return Reading::Managed,
            Ok(false) => {}
            Err(e) => return Reading::Unreadable(e),
        }
        match self.mech.read(&place) {
            Ok(natives) => match codec::decode(&self.cat, row, &place, &natives) {
                Ok(Some(v)) => Reading::Value(v),
                Ok(None) => Reading::NotSet,
                Err(e) => Reading::Unreadable(e),
            },
            Err(ReadError::NotHere) => Reading::NotBuilt,
            Err(ReadError::Failed(e)) => Reading::Unreadable(e),
        }
    }

    /// Reads every row, the protected things and the settings without a row yet.
    pub fn capture(&mut self) -> Capture {
        let system = self.mech.system;
        let mut c = Capture::new(system, self.cat.revision);
        let rows = self.cat.settings.clone();
        for row in &rows {
            let r = self.reading(row);
            c.readings.insert(row.id.clone(), r);
        }
        for p in self.cat.protected.clone() {
            if p.system == system && self.mech.probe(&p.probe) == Ok(true) {
                c.protected.insert(p.what);
            }
        }
        c.unsupported = self.discover();
        c
    }

    /// Settings in known stores (every store a row uses, and the catalogue's extra stores)
    /// that no row covers: their store and name, never their value.
    pub fn discover(&mut self) -> Vec<Found> {
        let system = self.mech.system;
        let mut stores: Vec<StoreRef> = Vec::new();
        let mut covered: BTreeSet<(String, String)> = BTreeSet::new();
        let mut whole: BTreeSet<String> = BTreeSet::new();
        for row in &self.cat.settings {
            let Some(place) = row.places.get(&system) else {
                continue;
            };
            let Some(store) = store_of(place) else {
                continue;
            };
            let label = label(&store);
            if place
                .fields()
                .iter()
                .any(|f| matches!(f.kind, Kind::ValueNames | Kind::NumberedValues))
            {
                // A list made of the store's own value names: every name is covered.
                whole.insert(label.clone());
            }
            for f in place.fields() {
                covered.insert((label.clone(), f.name.to_lowercase()));
            }
            if !stores.contains(&store) {
                stores.push(store);
            }
        }
        for d in &self.cat.discover {
            if d.system == system && !stores.contains(&d.store) {
                stores.push(d.store.clone());
            }
        }
        let mut found = Vec::new();
        for store in stores {
            let label = label(&store);
            if whole.contains(&label) {
                continue;
            }
            let Ok(names) = self.mech.store_names(&store) else {
                continue;
            };
            for name in names {
                if !covered.contains(&(label.clone(), name.to_lowercase())) {
                    found.push(Found {
                        store: label.clone(),
                        name,
                    });
                }
            }
        }
        found.sort();
        found.dedup();
        found
    }

    /// The new laptop's current values and the settings its organisation manages.
    pub fn probe_target(&mut self) -> Target {
        let mut t = Target::default();
        for row in self.cat.settings.clone() {
            match self.reading(&row) {
                Reading::Managed => {
                    t.managed.insert(row.id.clone());
                }
                r => {
                    t.current.insert(row.id.clone(), r);
                }
            }
        }
        t
    }

    /// A setting's value now, by its key (for the executor and undo).
    pub fn read(&mut self, key: &str) -> Result<Option<SettingValue>, String> {
        if key.starts_with("found.") {
            let a = self
                .found(key)
                .ok_or("PCTwin does not know this setting's place")?;
            return read_found(&mut self.mech, &a);
        }
        let row = self
            .cat
            .row(key)
            .cloned()
            .ok_or_else(|| "PCTwin does not move this setting".to_string())?;
        match self.reading(&row) {
            Reading::Value(v) => Ok(Some(v)),
            Reading::NotSet => Ok(None),
            Reading::Unreadable(e) => Err(e),
            Reading::Managed => Err("your organisation manages this setting".into()),
            Reading::NotApplicable | Reading::NotBuilt => {
                Err("this laptop has no place for this setting".into())
            }
        }
    }

    /// The writable native place of a setting on this laptop, with its row.
    fn writable(&self, key: &str) -> Result<(Row, Place), Reason> {
        let row = self
            .cat
            .row(key)
            .cloned()
            .ok_or(Reason::RefusedBySafetyCheck)?;
        if row.route != Route::Native {
            return Err(Reason::FinishByHand);
        }
        let place = self
            .place(&row)
            .filter(|p| !p.read_only())
            .ok_or(Reason::NoEquivalent)?;
        Ok((row, place))
    }

    /// Writes `value` (or removes the setting), checked against its row; a file setting only
    /// when its file is there.
    fn put(
        &mut self,
        row: &Row,
        place: &Place,
        value: Option<&SettingValue>,
    ) -> Result<(), WriteError> {
        let natives: Vec<Option<Native>> = match value {
            Some(v) => {
                if let (ValueType::FilePath, SettingValue::Text(p)) = (&row.value, v)
                    && !std::path::Path::new(p).is_file()
                {
                    return Err(WriteError::Failed("the file is not on this laptop".into()));
                }
                codec::encode(&self.cat, row, place, v).map_err(WriteError::Failed)?
            }
            None => vec![None; place.fields().len()],
        };
        self.mech.write(place, &natives)
    }

    /// Applies one `apply-setting` step for the executor's adapter.
    pub fn apply(
        &mut self,
        grant: SettingGrant,
        key: &str,
        value: &SettingValue,
    ) -> Result<(), Reason> {
        if grant.expiry().is_over(Now::real()) {
            return Err(Reason::NotVerified);
        }
        let step = Step::ApplySetting {
            key: key.to_string(),
            value: value.clone(),
        };
        let named = grant.setting();
        let step_no = u32::try_from(named.handle()).map_err(|_| Reason::RefusedBySafetyCheck)?;
        if object_of(step_no, &step) != named {
            return Err(Reason::RefusedBySafetyCheck);
        }
        if key.starts_with("found.") {
            let a = self.found(key).ok_or(Reason::RefusedBySafetyCheck)?;
            return match write_found(&mut self.mech, &a, Some(value), &mut self.created) {
                Ok(()) => Ok(()),
                Err(WriteError::Denied) => Err(Reason::Permission),
                Err(WriteError::Failed(_)) => Err(Reason::NotVerified),
            };
        }
        let (row, place) = self.writable(key)?;
        if row.value.check(key, value).is_err() {
            return Err(Reason::RefusedBySafetyCheck);
        }
        if self.mech.managed(&place) != Ok(false) {
            return Err(Reason::Permission);
        }
        match self.put(&row, &place, Some(value)) {
            Ok(()) => Ok(()),
            Err(WriteError::Denied) => Err(Reason::Permission),
            Err(WriteError::Failed(_)) => Err(Reason::NotVerified),
        }
    }

    /// Undo's compare-and-write: `to` (or removed) only if the setting holds `expect` now.
    pub fn write_if(
        &mut self,
        key: &str,
        expect: &SettingValue,
        to: Option<&SettingValue>,
    ) -> Result<SettingWrite, String> {
        if key.starts_with("found.") {
            let a = self
                .found(key)
                .ok_or("PCTwin does not know this setting's place")?;
            if read_found(&mut self.mech, &a)?.as_ref() != Some(expect) {
                return Ok(SettingWrite::Changed);
            }
            write_found(&mut self.mech, &a, to, &mut self.created).map_err(|e| e.to_string())?;
            return Ok(SettingWrite::Written);
        }
        let (row, place) = self
            .writable(key)
            .map_err(|_| "this setting cannot be put back here".to_string())?;
        if let Some(v) = to {
            row.value.check(key, v).map_err(|e| e.to_string())?;
        }
        if self.read(key)?.as_ref() != Some(expect) {
            return Ok(SettingWrite::Changed);
        }
        self.put(&row, &place, to).map_err(|e| e.to_string())?;
        Ok(SettingWrite::Written)
    }
}

fn store_of(place: &Place) -> Option<StoreRef> {
    match place {
        Place::Registry { hive, key, .. } => Some(StoreRef::Registry {
            hive: *hive,
            key: key.clone(),
        }),
        Place::MacPref { domain, .. } => Some(StoreRef::MacPref {
            domain: domain.clone(),
        }),
        Place::Gsettings { schema, .. } => Some(StoreRef::Gsettings {
            schema: schema.clone(),
        }),
        _ => None,
    }
}

fn label(store: &StoreRef) -> String {
    match store {
        StoreRef::Registry { hive, key } => format!(
            "{}\\{key}",
            match hive {
                Hive::User => "HKCU",
                Hive::Machine => "HKLM",
            }
        ),
        StoreRef::MacPref { domain } => format!("defaults:{domain}"),
        StoreRef::Gsettings { schema } => format!("gsettings:{schema}"),
    }
}

/// An executor adapter for a run made only of settings: every other step is refused.
pub struct SettingsAdapter {
    pub store: Store,
    pub platform: Platform,
}

impl Adapter for SettingsAdapter {
    fn platform(&self) -> Platform {
        self.platform
    }
    fn install(&mut self, _: InstallGrant, _: Catalog, _: &str) -> Result<(), Reason> {
        Err(Reason::RefusedBySafetyCheck)
    }
    fn copy_settings(
        &mut self,
        _: SettingGrant,
        _: &SettingsPlace,
        _: &SettingsPlace,
    ) -> Result<(), Reason> {
        Err(Reason::RefusedBySafetyCheck)
    }
    fn apply_setting(
        &mut self,
        grant: SettingGrant,
        key: &str,
        value: &SettingValue,
    ) -> Result<(), Reason> {
        self.store.apply(grant, key, value)
    }
    fn verify(&mut self, check: &Check) -> bool {
        match check {
            Check::SettingIs { key, value } => {
                self.store.read(key).ok().flatten().as_ref() == Some(value)
            }
            _ => false,
        }
    }
    fn app_presence(&mut self, _: Catalog, _: &str) -> Result<Presence, String> {
        Err("this run changes settings only".into())
    }
    fn install_running(&mut self, _: Catalog, _: &str) -> Result<bool, String> {
        Err("this run changes settings only".into())
    }
    fn read_setting(&mut self, key: &str) -> Result<Option<SettingValue>, String> {
        self.store.read(key)
    }
    fn read_settings_files(&mut self, _: &SettingsPlace) -> Result<Vec<SettingsFile>, String> {
        Err("this run changes settings only".into())
    }
    fn restore_point(&mut self) -> RestorePoint {
        RestorePoint::NotMade {
            why: "settings are put back by PCTwin's own undo".into(),
        }
    }
    fn uninstall(&mut self, _: Catalog, _: &str) -> Result<(), String> {
        Err("this run changes settings only".into())
    }
    fn write_setting_if(
        &mut self,
        key: &str,
        expect: &SettingValue,
        to: Option<&SettingValue>,
    ) -> Result<SettingWrite, String> {
        self.store.write_if(key, expect, to)
    }
    fn replace_settings_file(
        &mut self,
        _: &SettingsPlace,
        _: &str,
        _: &files::Swap<'_>,
    ) -> Result<files::Swapped, String> {
        Err("this run changes settings only".into())
    }
}
