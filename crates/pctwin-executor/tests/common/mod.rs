//! A pretend new laptop and journal helpers shared by the executor tests.
#![allow(dead_code)]

use std::cell::{Cell, RefCell};
use std::collections::{BTreeMap, HashMap, HashSet};
use std::rc::Rc;

use pctwin_executor::{Adapter, Plan, PlannedItem, Run};
use pctwin_grant::{GrantError, InstallGrant, Now, ObjectRef, PlanSeal, SettingGrant};
use pctwin_journal::{
    Actor, EffectEntry, EffectLedger, Found, InstallIntent, Journal, JournalError, Permission,
    PlannedEffect, Presence, Rebuild, RebuildRole, Receipt, RestorePoint, Scope, SettingsFile,
    SettingsIntent,
};
use pctwin_recipe::{
    Arch, Catalog, Check, Os, Platform, Recipe, RecipeId, RelPath, SettingValue, SettingsPlace,
    SettingsRoot, Step, UndoDeclaration,
};
use pctwin_record::{
    Approval, FolderRole, Inclusion, Item, ItemId, ItemKind, ItemName, ItemPath, LaptopId,
    ManagedBy, Owner, Place, Portability, Record, Storage,
};
use pctwin_types::{ItemResult, Reason};

pub const WIN: Platform = Platform {
    os: Os::Windows,
    arch: Arch::X86_64,
};

pub const SCOPE: Scope = Scope([3; 16]);

pub fn actor() -> Actor {
    Actor {
        acting_account: "1001".into(),
        for_account: "1001".into(),
        permission: Permission::OwnFolders,
    }
}

pub fn laptop_id() -> LaptopId {
    LaptopId::from_hex("00112233445566778899aabbccddeeff").unwrap()
}

pub fn app(name: &str) -> Item {
    let owner = Owner::Shared;
    let place = Place {
        role: FolderRole::Documents,
        storage: Storage::SystemDrive,
    };
    let path = ItemPath::new(vec![ItemName::from_text(name)]).unwrap();
    Item {
        id: ItemId::derive(&laptop_id(), &owner, &place, &path),
        kind: ItemKind::App,
        owner,
        place,
        path,
        size_bytes: 0,
        modified_ns: None,
        portability: Portability::Rebuildable,
        managed_by: ManagedBy::Personal,
        download_bytes: None,
        inclusion: Inclusion::Included,
    }
}

thread_local! {
    /// Every recipe this test made, by ID: the checked recipes undo looks up again.
    static MADE: RefCell<BTreeMap<String, Recipe>> = RefCell::default();
}

/// Adds `r` to the checked recipes undo finds again.
pub fn made(r: Recipe) -> Recipe {
    MADE.with(|m| m.borrow_mut().insert(r.id.as_str().to_string(), r.clone()));
    r
}

/// A recipe with an ID of its own in this test ("x", "x2", "x3" ...).
pub fn recipe(steps: Vec<Step>) -> Recipe {
    let n = MADE.with(|m| m.borrow().len()) + 1;
    let id = if n == 1 {
        "x".to_string()
    } else {
        format!("x{n}")
    };
    made(Recipe::new(RecipeId::new(&id).unwrap(), 1, vec![WIN], steps).unwrap())
}

/// The checked recipes undo finds again: every one this test made.
pub fn book() -> Vec<Recipe> {
    MADE.with(|m| m.borrow().values().cloned().collect())
}

/// A recipe whose app undo may remove.
pub fn removable(steps: Vec<Step>) -> Recipe {
    made(
        recipe(steps)
            .with_undo(UndoDeclaration {
                uninstall: true,
                side_effects: vec![],
            })
            .unwrap(),
    )
}

pub fn install(package: &str) -> Step {
    Step::Install {
        catalog: Catalog::Winget,
        package: package.into(),
    }
}

pub fn set(key: &str, value: SettingValue) -> Step {
    Step::ApplySetting {
        key: key.into(),
        value,
    }
}

pub fn place(path: &str) -> SettingsPlace {
    SettingsPlace {
        root: SettingsRoot::Config,
        path: RelPath::new(path).unwrap(),
    }
}

pub fn copy(path: &str) -> Step {
    Step::CopySettings {
        from: place(path),
        to: place(path),
    }
}

pub fn install_and_check(package: &str) -> Recipe {
    recipe(vec![
        install(package),
        Step::Verify {
            check: Check::PackageInstalled {
                catalog: Catalog::Winget,
                package: package.into(),
            },
        },
    ])
}

pub fn approved(items: Vec<Item>) -> (Record, Approval) {
    let mut record = Record::new(laptop_id());
    record.items = items;
    let approval = record.approve().unwrap();
    (record, approval)
}

/// A plan of one app item per recipe, in order.
pub fn plan_of(recipes: Vec<Recipe>) -> Plan {
    let items: Vec<Item> = (0..recipes.len())
        .map(|n| app(&format!("app{n}")))
        .collect();
    let (record, approval) = approved(items.clone());
    Plan::approved(
        &record,
        &approval,
        items
            .iter()
            .zip(recipes)
            .map(|(i, recipe)| PlannedItem { item: i.id, recipe })
            .collect(),
    )
    .unwrap()
}

/// This test program's role keys: made once (a program gets them only once) and lent to every
/// test.
pub fn keys() -> &'static pctwin_grant::RoleKeys {
    static KEYS: std::sync::OnceLock<pctwin_grant::RoleKeys> = std::sync::OnceLock::new();
    KEYS.get_or_init(|| pctwin_grant::RoleKeys::mint().expect("the keys are made once"))
}

/// The installs and settings recorders of `j`, as the program's starting code hands them to the
/// executor.
pub fn rebuild(j: &Journal) -> Rebuild<'_> {
    Rebuild::new(j, &keys().install, &keys().setting)
}

pub fn journal() -> (tempfile::TempDir, std::path::PathBuf, Journal) {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("journal.redb");
    let j = open(&path).unwrap();
    (dir, path, j)
}

/// Runs `plan` on `laptop` with a fresh journal; the items' results.
pub fn run(plan: &Plan, laptop: &mut Laptop) -> Vec<(ItemId, ItemResult)> {
    let (_d, _p, j) = journal();
    plan.run(laptop, &rebuild(&j), SCOPE, &actor()).results
}

pub fn run_with(plan: &Plan, laptop: &mut Laptop, ledger: &dyn EffectLedger) -> Run {
    plan.run(laptop, ledger, SCOPE, &actor())
}

fn key(place: &SettingsPlace) -> String {
    format!(
        "{:?}/{}",
        place.root,
        place.path.parts().collect::<Vec<_>>().join("/")
    )
}

/// A pretend new laptop: what is installed, each setting, each settings file, and the old
/// laptop's settings files a copy reads. `done` lists every change asked for, in order (looks
/// are not changes); `log`, when shared, gets the same entries beside the journal's.
#[derive(Default)]
pub struct Laptop {
    pub platform: Option<Platform>,
    pub done: Vec<String>,
    pub log: Option<Rc<RefCell<Vec<String>>>>,
    pub fail_install: Option<Reason>,
    /// An install reports success and does nothing.
    pub lies: bool,
    /// Package to version.
    pub installed: HashMap<String, String>,
    /// Whether "a setting is this value" checks pass.
    pub checks_pass: bool,
    pub settings: HashMap<String, SettingValue>,
    /// (place, path) to bytes, on the new laptop.
    pub files: BTreeMap<(String, String), Vec<u8>>,
    /// (place, path) to bytes, on the old laptop.
    pub source: BTreeMap<(String, String), Vec<u8>>,
    /// A settings copy reports success and copies nothing.
    pub copy_lies: bool,
    /// Looks that cannot tell now: "app", "setting", "files".
    pub blind: HashSet<&'static str>,
    pub fail_uninstall: bool,
    /// An uninstall reports success and leaves the app.
    pub uninstall_lies: bool,
    pub fail_restore: bool,
    /// Packages whose install may still be running.
    pub installing: HashSet<String>,
    /// Whether "is an install still running" cannot be told.
    pub running_unknown: bool,
    /// The restore point answer this laptop gives (default: not Windows).
    pub restore_answer: Option<RestorePoint>,
    /// How many times a restore point was asked for.
    pub restore_asked: usize,
    /// Settings files put back before putting one back fails.
    pub puts_before_failing: Option<usize>,
    /// The catalog's "is it installed" check says yes whatever is there.
    pub install_check_lies: bool,
    /// Putting a setting or a settings file back reports success and changes nothing.
    pub restore_lies: bool,
    /// Every permission an install or settings step was handed: (role, step, step digest).
    pub grants: Vec<(&'static str, u64, [u8; 32])>,
}

impl Laptop {
    pub fn ok() -> Self {
        Self {
            checks_pass: true,
            ..Self::default()
        }
    }

    fn note(&mut self, what: String) {
        if let Some(log) = &self.log {
            log.borrow_mut().push(what.clone());
        }
        self.done.push(what);
    }

    pub fn file(&self, place: &SettingsPlace, path: &str) -> Option<&Vec<u8>> {
        self.files.get(&(key(place), path.to_string()))
    }

    pub fn put_file(&mut self, place: &SettingsPlace, path: &str, bytes: &[u8]) {
        self.files
            .insert((key(place), path.to_string()), bytes.to_vec());
    }

    pub fn put_source(&mut self, place: &SettingsPlace, path: &str, bytes: &[u8]) {
        self.source
            .insert((key(place), path.to_string()), bytes.to_vec());
    }

    fn files_at(
        &self,
        map: &BTreeMap<(String, String), Vec<u8>>,
        place: &SettingsPlace,
    ) -> Vec<(String, Vec<u8>)> {
        let k = key(place);
        map.iter()
            .filter(|((p, _), _)| *p == k)
            .map(|((_, path), b)| (path.clone(), b.clone()))
            .collect()
    }
}

impl Adapter for Laptop {
    fn platform(&self) -> Platform {
        self.platform.unwrap_or(WIN)
    }
    fn install(&mut self, grant: InstallGrant, _: Catalog, package: &str) -> Result<(), Reason> {
        assert!(!grant.expiry().is_over(Now::real()), "a live permission");
        let a = grant.artifact();
        self.grants.push(("install", a.handle(), a.digest()));
        self.note(format!("install {package}"));
        if let Some(reason) = self.fail_install {
            return Err(reason);
        }
        if !self.lies {
            self.installed.insert(package.to_string(), "1.0".into());
        }
        Ok(())
    }
    fn copy_settings(
        &mut self,
        grant: SettingGrant,
        from: &SettingsPlace,
        to: &SettingsPlace,
    ) -> Result<(), Reason> {
        assert!(!grant.expiry().is_over(Now::real()), "a live permission");
        let s = grant.setting();
        self.grants.push(("copy", s.handle(), s.digest()));
        self.note("copy".into());
        if !self.copy_lies {
            for (path, bytes) in self.files_at(&self.source.clone(), from) {
                self.put_file(to, &path, &bytes);
            }
        }
        Ok(())
    }
    fn apply_setting(
        &mut self,
        grant: SettingGrant,
        key: &str,
        value: &SettingValue,
    ) -> Result<(), Reason> {
        assert!(!grant.expiry().is_over(Now::real()), "a live permission");
        let s = grant.setting();
        self.grants.push(("set", s.handle(), s.digest()));
        self.note(format!("set {key}"));
        self.settings.insert(key.into(), value.clone());
        Ok(())
    }
    fn verify(&mut self, check: &Check) -> bool {
        self.note("check".into());
        match check {
            Check::PackageInstalled { package, .. } => {
                self.install_check_lies || self.installed.contains_key(package)
            }
            Check::SettingsMatch { from, to } => {
                self.files_at(&self.source, from) == self.files_at(&self.files, to)
            }
            _ => self.checks_pass,
        }
    }
    fn app_presence(&mut self, _: Catalog, package: &str) -> Result<Presence, String> {
        if self.blind.contains("app") {
            return Err("the app list could not be read".into());
        }
        Ok(match self.installed.get(package) {
            Some(v) => Presence::Present {
                version: Some(v.clone()),
            },
            None => Presence::Absent,
        })
    }
    fn install_running(&mut self, _: Catalog, package: &str) -> Result<bool, String> {
        if self.running_unknown {
            return Err("the system's installer could not be asked".into());
        }
        Ok(self.installing.contains(package))
    }
    fn restore_point(&mut self) -> RestorePoint {
        self.note("restore point".into());
        self.restore_asked += 1;
        self.restore_answer
            .clone()
            .unwrap_or(RestorePoint::NotWindows)
    }
    fn read_setting(&mut self, key: &str) -> Result<Option<SettingValue>, String> {
        if self.blind.contains("setting") {
            return Err("the setting could not be read".into());
        }
        Ok(self.settings.get(key).cloned())
    }
    fn read_settings_files(&mut self, place: &SettingsPlace) -> Result<Vec<SettingsFile>, String> {
        if self.blind.contains("files") {
            return Err("the settings folder could not be read".into());
        }
        Ok(self
            .files_at(&self.files, place)
            .into_iter()
            .map(|(path, bytes)| SettingsFile { path, bytes })
            .collect())
    }
    fn uninstall(&mut self, _: Catalog, package: &str) -> Result<(), String> {
        self.note(format!("uninstall {package}"));
        if self.fail_uninstall {
            return Err("the system would not remove it".into());
        }
        if !self.uninstall_lies {
            self.installed.remove(package);
        }
        Ok(())
    }
    fn write_setting(&mut self, key: &str, value: Option<&SettingValue>) -> Result<(), String> {
        self.note(format!("restore {key}"));
        if self.fail_restore {
            return Err("the setting could not be written".into());
        }
        if self.restore_lies {
            return Ok(());
        }
        match value {
            Some(v) => self.settings.insert(key.into(), v.clone()),
            None => self.settings.remove(key),
        };
        Ok(())
    }
    fn put_settings_file(
        &mut self,
        place: &SettingsPlace,
        path: &str,
        bytes: Option<&[u8]>,
    ) -> Result<(), String> {
        self.note(format!("restore file {path}"));
        if let Some(left) = self.puts_before_failing {
            if left == 0 {
                return Err("the file could not be written".into());
            }
            self.puts_before_failing = Some(left - 1);
        }
        if self.fail_restore {
            return Err("the file could not be written".into());
        }
        if self.restore_lies {
            return Ok(());
        }
        match bytes {
            Some(b) => self.put_file(place, path, b),
            None => {
                self.files.remove(&(key(place), path.to_string()));
            }
        }
        Ok(())
    }
}

/// A journal that records its calls in a shared log, and whose writes fail once `writes`
/// successful writes have been made (a full disk, or a journal that cannot be written).
pub struct Faulty<'j> {
    pub inner: Rebuild<'j>,
    pub writes: Cell<usize>,
    pub log: Rc<RefCell<Vec<String>>>,
    /// Refuses only this kind of write (every other one is written).
    pub only: Option<&'static str>,
}

impl<'j> Faulty<'j> {
    pub fn new(inner: &'j Journal, writes: usize) -> Self {
        Self {
            inner: rebuild(inner),
            writes: Cell::new(writes),
            log: Rc::default(),
            only: None,
        }
    }

    fn write(&self, what: &str) -> Result<(), JournalError> {
        if self.only.is_some_and(|only| only != what) {
            self.log.borrow_mut().push(what.into());
            return Ok(());
        }
        if self.writes.get() == 0 {
            self.log.borrow_mut().push(format!("{what} refused"));
            return Err(JournalError::Storage("the disk is full".into()));
        }
        self.writes.set(self.writes.get() - 1);
        self.log.borrow_mut().push(what.into());
        Ok(())
    }
}

impl EffectLedger for Faulty<'_> {
    fn intend_install(
        &self,
        planned: &PlannedEffect,
        intent: &InstallIntent,
    ) -> Result<u64, JournalError> {
        self.write("intend")?;
        self.inner.installs.intend(planned, intent)
    }
    fn intend_setting(
        &self,
        planned: &PlannedEffect,
        intent: &SettingsIntent,
    ) -> Result<u64, JournalError> {
        self.write("intend")?;
        self.inner.settings.intend(planned, intent)
    }
    fn observed(
        &self,
        role: RebuildRole,
        id: u64,
        found: &Found,
        after_restart: bool,
    ) -> Result<(), JournalError> {
        self.write("observed")?;
        EffectLedger::observed(&self.inner, role, id, found, after_restart)
    }
    fn receipt(&self, role: RebuildRole, id: u64, receipt: &Receipt) -> Result<(), JournalError> {
        self.write("receipt")?;
        EffectLedger::receipt(&self.inner, role, id, receipt)
    }
    fn unfinished_effects(&self) -> Result<Vec<EffectEntry>, JournalError> {
        self.inner.journal().unfinished_effects()
    }
    fn restore_point(&self, scope: Scope) -> Result<Option<RestorePoint>, JournalError> {
        self.inner.journal().restore_point(scope)
    }
    fn install_grant(&self, seal: PlanSeal, a: ObjectRef) -> Result<InstallGrant, GrantError> {
        self.inner.install_grant(seal, a)
    }
    fn setting_grant(&self, seal: PlanSeal, s: ObjectRef) -> Result<SettingGrant, GrantError> {
        self.inner.setting_grant(seal, s)
    }
    fn record_restore_point(&self, scope: Scope, point: &RestorePoint) -> Result<(), JournalError> {
        self.write("restore-point")?;
        EffectLedger::record_restore_point(&self.inner, scope, point)
    }
}

/// Opens a journal with this test program's record key (never the system's key store, so tests
/// leave nothing behind in it).
pub fn open(
    path: &std::path::Path,
) -> Result<pctwin_journal::Journal, pctwin_journal::JournalError> {
    pctwin_journal::Journal::open_with_record_key(path, test_key())
}

/// The record key every test journal is sealed with.
pub fn test_key() -> pctwin_journal::keystore::TagKey {
    pctwin_journal::keystore::TagKey::from_bytes([0x5a; 32])
}
