//! After the files arrived: the approved plan's apps and settings, each ending with one honest
//! result word.
//!
//! - **Apps**: an app PCTwin cannot install itself ends needs-action with its reason (the person
//!   installs it; their data is ready). The rest go to the [`Installer`]: on Windows the real
//!   chain (the maker's package downloaded as the person, the administrator helper's own maker
//!   check and install, then the laptop's own list looked at again); in tests a fake. Its result
//!   is taken as it comes: an installer never says "reinstalled" without the laptop showing it.
//! - **Settings**: through the executor, with the settings role's grants, every change journaled
//!   before it is made and read back after (so a value the system did not keep is a failure,
//!   never a success), one recipe per app or part of the system.
//! - **The restore point**: asked once, before the first change, by the installer (which starts
//!   the administrator helper); its honest answer is recorded in the journal.
//! - **Special categories**: until 2.5 joins, each ends needs-action with its reason.

use std::collections::BTreeMap;

use pctwin_executor::{Adapter, Plan, PlannedItem, SettingWrite, files};
use pctwin_grant::{InstallGrant, SettingGrant};
use pctwin_journal::{Actor, EffectLedger, Presence, RestorePoint, Scope, SettingsFile};
use pctwin_recipe::{Catalog, Check, Os, Platform, Recipe, SettingValue, SettingsPlace};
use pctwin_record::{
    FolderRole, Inclusion, Item, ItemKind, ItemName, ItemPath, LaptopId, ManagedBy, Owner, Place,
    Portability, Record, Storage,
};
use pctwin_settings::System;
use pctwin_settings::found::Address;
use pctwin_types::{ItemResult, ItemStatus, Reason};
use serde::{Deserialize, Serialize};

use crate::plan::{AppEntry, FullPlan, SPECIAL_NOT_YET, Title, needs_of, special_reason};

/// How one app ended, with what the person still has to do.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AppResult {
    pub result: ItemResult,
    /// What the person still has to do (sign in, a licence key, restart...).
    pub todo: Vec<pctwin_apps::after::Todo>,
}

impl AppResult {
    pub fn because(reason: Reason) -> Self {
        Self {
            result: ItemResult::because(reason),
            todo: Vec::new(),
        }
    }
}

/// Installs apps. The real one asks for the administrator once (with the restore point), the
/// test lab's is a fake.
pub trait Installer {
    /// The restore point's answer: asked once (later calls give the same answer). Installing asks
    /// for it before the first install; otherwise it is asked here, before the first setting.
    fn restore_point(&mut self) -> RestorePoint;
    /// The restore point's answer if it was asked already (by an install), without asking.
    fn asked(&self) -> Option<RestorePoint> {
        None
    }
    /// Installs `apps`, in order, calling `done` after each; one result per app, in order. The
    /// restore point is asked for before the first install.
    fn install(&mut self, apps: &[&AppEntry], done: &mut dyn FnMut()) -> Vec<AppResult>;
}

/// Where no installer works (not built for this system yet): every app needs the person, and no
/// restore point exists.
pub struct NoInstaller {
    pub why: &'static str,
}

impl Installer for NoInstaller {
    fn restore_point(&mut self) -> RestorePoint {
        RestorePoint::NotMade {
            why: self.why.into(),
        }
    }
    fn install(&mut self, apps: &[&AppEntry], done: &mut dyn FnMut()) -> Vec<AppResult> {
        apps.iter()
            .map(|_| {
                done();
                AppResult::because(Reason::ManualInstall)
            })
            .collect()
    }
}

/// Where the rebuilding is.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(tag = "step", rename_all = "kebab-case")]
pub enum Progress {
    /// Asking for the administrator and the restore point.
    Permission,
    Apps {
        done: u64,
        total: u64,
    },
    Settings {
        done: u64,
        total: u64,
    },
}

/// What undo needs to find and check this move's settings again, saved beside the journal.
#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct UndoBook {
    pub scope: [u8; 16],
    /// Each moved setting's place, by its executor key.
    pub book: BTreeMap<String, Address>,
    /// The recipes the settings were applied with (undo checks each effect against them).
    pub recipes: Vec<Recipe>,
    /// Each setting's group (the app or part of the system), by its executor key.
    pub labels: BTreeMap<String, String>,
    /// The apps PCTwin installed, by name (undo leaves them for the person to remove).
    pub apps: Vec<String>,
}

/// How the apps, settings and special categories of a move ended.
#[derive(Debug, Clone, PartialEq)]
pub struct Rebuilt {
    /// Every app in the plan PCTwin did not leave alone, in plan order.
    pub apps: Vec<(String, AppResult)>,
    /// Each group of settings: its label, how many settings, and its result.
    pub settings: Vec<(String, u64, ItemResult)>,
    pub special: Vec<(Title, ItemResult)>,
    pub restore_point: Option<RestorePoint>,
    /// Why the settings stopped part way, if they did (the journal could not be written).
    pub stopped: Option<String>,
    pub undo: UndoBook,
}

impl Rebuilt {
    /// Counts by result word, over every app, setting group and special line.
    pub fn counts(&self) -> Vec<(ItemStatus, u64)> {
        let mut by: Vec<(ItemStatus, u64)> = Vec::new();
        let all = self
            .apps
            .iter()
            .map(|(_, a)| a.result)
            .chain(self.settings.iter().map(|(_, _, r)| *r))
            .chain(self.special.iter().map(|(_, r)| *r));
        for r in all {
            match by.iter_mut().find(|(s, _)| *s == r.status()) {
                Some((_, n)) => *n += 1,
                None => by.push((r.status(), 1)),
            }
        }
        by
    }
}

/// An executor adapter for the move: settings through `inner`, the restore point already asked,
/// and `applied` told after each setting written.
struct MoveAdapter<'a> {
    inner: &'a mut dyn Adapter,
    point: RestorePoint,
    applied: &'a mut dyn FnMut(),
}

impl Adapter for MoveAdapter<'_> {
    fn platform(&self) -> Platform {
        self.inner.platform()
    }
    fn install(&mut self, _: InstallGrant, _: Catalog, _: &str) -> Result<(), Reason> {
        // Apps go through the installer and the administrator helper, never here.
        Err(Reason::RefusedBySafetyCheck)
    }
    fn copy_settings(
        &mut self,
        g: SettingGrant,
        from: &SettingsPlace,
        to: &SettingsPlace,
    ) -> Result<(), Reason> {
        self.inner.copy_settings(g, from, to)
    }
    fn apply_setting(
        &mut self,
        g: SettingGrant,
        key: &str,
        value: &SettingValue,
    ) -> Result<(), Reason> {
        let done = self.inner.apply_setting(g, key, value);
        (self.applied)();
        done
    }
    fn verify(&mut self, check: &Check) -> bool {
        self.inner.verify(check)
    }
    fn app_presence(&mut self, c: Catalog, p: &str) -> Result<Presence, String> {
        self.inner.app_presence(c, p)
    }
    fn install_running(&mut self, c: Catalog, p: &str) -> Result<bool, String> {
        self.inner.install_running(c, p)
    }
    fn read_setting(&mut self, key: &str) -> Result<Option<SettingValue>, String> {
        self.inner.read_setting(key)
    }
    fn read_settings_files(&mut self, p: &SettingsPlace) -> Result<Vec<SettingsFile>, String> {
        self.inner.read_settings_files(p)
    }
    fn restore_point(&mut self) -> RestorePoint {
        self.point.clone()
    }
    fn uninstall(&mut self, c: Catalog, p: &str) -> Result<(), String> {
        self.inner.uninstall(c, p)
    }
    fn write_setting_if(
        &mut self,
        key: &str,
        expect: &SettingValue,
        to: Option<&SettingValue>,
    ) -> Result<SettingWrite, String> {
        self.inner.write_setting_if(key, expect, to)
    }
    fn replace_settings_file(
        &mut self,
        p: &SettingsPlace,
        path: &str,
        swap: &files::Swap<'_>,
    ) -> Result<files::Swapped, String> {
        self.inner.replace_settings_file(p, path, swap)
    }
}

/// The executor's system name for `system`.
pub fn os_of(system: System) -> Os {
    match system {
        System::Windows => Os::Windows,
        System::Mac => Os::Macos,
        System::Gnome | System::Kde | System::OtherLinux => Os::Linux,
    }
}

/// The record the executor's plan is approved from: one setting item per recipe, on this laptop,
/// standing for the signed-in person's settings. It is made from the plan the person approved
/// and nothing else.
fn settings_record(laptop: LaptopId, n: usize) -> Result<(Record, Vec<Item>), String> {
    // The settings are the signed-in person's own (the actor says whose); the items stand for
    // them only inside this plan.
    let owner = Owner::Shared;
    let place = Place {
        role: FolderRole::Documents,
        storage: Storage::SystemDrive,
    };
    let mut items = Vec::with_capacity(n);
    for k in 0..n {
        let path = ItemPath::new(vec![ItemName::from_text(&format!("settings-{k}"))])
            .map_err(|e| e.to_string())?;
        items.push(Item {
            id: pctwin_record::ItemId::derive(&laptop, &owner, &place, &path),
            kind: ItemKind::Setting,
            owner: owner.clone(),
            place: place.clone(),
            path,
            size_bytes: 0,
            modified_ns: None,
            portability: Portability::Rebuildable,
            managed_by: ManagedBy::Personal,
            download_bytes: None,
            inclusion: Inclusion::Included,
        });
    }
    let mut record = Record::new(laptop);
    record.items = items.clone();
    Ok((record, items))
}

/// What applying the settings came to.
struct SettingsRun {
    lines: Vec<(String, u64, ItemResult)>,
    recipes: Vec<Recipe>,
    point: Option<RestorePoint>,
    stopped: Option<String>,
}

/// Applies the plan's settings through the executor, recording each in `ledger` for move
/// `scope` as `actor`, with `settings` (an adapter that knows the plan's places) on this laptop
/// (`laptop`, the person `mover`).
#[allow(clippy::too_many_arguments)]
fn apply_settings(
    plan: &FullPlan,
    settings: &mut dyn Adapter,
    point: RestorePoint,
    ledger: &dyn EffectLedger,
    scope: Scope,
    actor: &Actor,
    laptop: LaptopId,
    applied: &mut dyn FnMut(),
) -> SettingsRun {
    let fail_all = |why: String| SettingsRun {
        lines: Vec::new(),
        recipes: Vec::new(),
        point: None,
        stopped: Some(why),
    };
    if plan.settings.moves.is_empty() {
        return SettingsRun {
            lines: Vec::new(),
            recipes: Vec::new(),
            point: None,
            stopped: None,
        };
    }
    let os = os_of(settings_system(settings.platform().os));
    let labelled = match plan.settings.recipes(os, 1) {
        Ok(r) => r,
        Err(e) => return fail_all(e.to_string()),
    };
    let (record, items) = match settings_record(laptop, labelled.len()) {
        Ok(r) => r,
        Err(e) => return fail_all(e),
    };
    let approval = match record.approve() {
        Ok(a) => a,
        Err(e) => return fail_all(e.to_string()),
    };
    let planned: Vec<PlannedItem> = items
        .iter()
        .zip(&labelled)
        .map(|(i, (_, r))| PlannedItem {
            item: i.id,
            recipe: r.clone(),
        })
        .collect();
    let exec = match Plan::approved(&record, &approval, planned) {
        Ok(p) => p,
        Err(e) => return fail_all(e.to_string()),
    };
    let mut adapter = MoveAdapter {
        inner: settings,
        point,
        applied,
    };
    let run = exec.run(&mut adapter, ledger, scope, actor);
    // One line per label (a label with more than one recipe is joined back).
    let mut lines: Vec<(String, u64, ItemResult)> = Vec::new();
    for ((label, recipe), (_, result)) in labelled.iter().zip(&run.results) {
        let steps = recipe.steps.len() as u64;
        match lines.iter_mut().find(|(l, _, _)| l == label) {
            Some(line) => {
                line.1 += steps;
                if line.2.status() == ItemStatus::Converted {
                    line.2 = *result;
                }
            }
            None => lines.push((label.clone(), steps, *result)),
        }
    }
    SettingsRun {
        lines,
        recipes: labelled.into_iter().map(|(_, r)| r).collect(),
        point: run.restore_point,
        stopped: run.stopped,
    }
}

fn settings_system(os: Os) -> System {
    match os {
        Os::Windows => System::Windows,
        Os::Macos => System::Mac,
        Os::Linux => System::Gnome,
    }
}

/// Rebuilds the approved `plan` on this laptop (`laptop`), after the files: the restore point
/// once, the apps through `installer`, the settings through `settings` and the executor (each
/// change in `ledger`, for move `scope`, as `actor`), and the special categories. `progress` is
/// told where it is.
#[allow(clippy::too_many_arguments)]
pub fn rebuild(
    plan: &FullPlan,
    installer: &mut dyn Installer,
    settings: &mut dyn Adapter,
    ledger: &dyn EffectLedger,
    scope: Scope,
    actor: &Actor,
    laptop: LaptopId,
    progress: &mut dyn FnMut(Progress),
) -> Rebuilt {
    if plan.changes_anything() {
        progress(Progress::Permission);
    }
    // Apps: PCTwin installs only what it can check; the rest need the person. The installer asks
    // for the administrator once, with the restore point first, before its first install.
    let to_install = plan.to_install();
    let total = to_install.len() as u64;
    let mut done = 0u64;
    progress(Progress::Apps { done, total });
    let installed = if to_install.is_empty() {
        Vec::new()
    } else {
        installer.install(&to_install, &mut || {
            done += 1;
            progress(Progress::Apps { done, total });
        })
    };
    // The restore point's answer: asked now, before the first setting, if no install asked for it
    // already; with no setting to change and nothing installed, nobody is asked for anything.
    let point = if plan.settings.moves.is_empty() {
        installer.asked()
    } else {
        Some(installer.restore_point())
    };
    let mut by_name: Vec<(&str, AppResult)> = to_install
        .iter()
        .map(|e| e.app.name.as_str())
        .zip(installed)
        .collect();
    let mut apps = Vec::new();
    for e in &plan.apps {
        if let Some(reason) = needs_of(&e.plan) {
            apps.push((e.app.name.clone(), AppResult::because(reason)));
        } else if let Some(at) = by_name.iter().position(|(n, _)| *n == e.app.name) {
            let (_, r) = by_name.remove(at);
            apps.push((e.app.name.clone(), r));
        }
        // An app already here (or part of the system) is left alone: not an item of the move.
    }
    // An install the installer gave no answer for is never counted as done.
    for e in &to_install {
        if !apps.iter().any(|(n, _)| *n == e.app.name) {
            apps.push((e.app.name.clone(), AppResult::because(Reason::NotVerified)));
        }
    }
    // Settings.
    let total = plan.settings.moves.len() as u64;
    let mut done = 0u64;
    let settings_point = point.clone().unwrap_or(RestorePoint::NotMade {
        why: "nothing needed one".into(),
    });
    let ran = apply_settings(
        plan,
        settings,
        settings_point,
        ledger,
        scope,
        actor,
        laptop,
        &mut || {
            done += 1;
            progress(Progress::Settings { done, total });
        },
    );
    // A run that could not start says so for every group; one that stopped part way already
    // said "not verified" for each group it did not reach.
    let settings_lines = if ran.lines.is_empty() && !plan.groups.is_empty() {
        plan.groups
            .iter()
            .map(|g| {
                (
                    g.label.clone(),
                    g.count,
                    ItemResult::because(Reason::NotVerified),
                )
            })
            .collect()
    } else {
        ran.lines
    };
    // Special categories.
    let mut special: Vec<(Title, ItemResult)> = plan
        .special
        .iter()
        .map(|s| {
            // Copying a component's files is not joined to the move yet: never claimed.
            let r = ItemResult::because(special_reason(s.step).unwrap_or(Reason::FinishByHand));
            (Title::Name(s.name.clone()), r)
        })
        .collect();
    if plan.special_not_checked > 0 {
        special.push((
            Title::Key(crate::plan::SPECIAL_NOT_CHECKED),
            ItemResult::because(Reason::FinishByHand),
        ));
    }
    if plan.from.is_some() && !plan.special_searched {
        special.push((
            Title::Key(SPECIAL_NOT_YET),
            ItemResult::because(Reason::FinishByHand),
        ));
    }
    let labels = plan
        .settings
        .moves
        .iter()
        .map(|m| (m.key.clone(), m.label.clone()))
        .collect();
    let undo = UndoBook {
        scope: scope.0,
        book: plan.settings.book(),
        recipes: ran.recipes,
        labels,
        apps: apps
            .iter()
            .filter(|(_, a)| a.result.status() == ItemStatus::Reinstalled)
            .map(|(n, _)| n.clone())
            .collect(),
    };
    Rebuilt {
        apps,
        settings: settings_lines,
        special,
        restore_point: ran.point.or(point),
        stopped: ran.stopped,
        undo,
    }
}
