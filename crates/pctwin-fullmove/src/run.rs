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
    /// PCTwin installed it here (the laptop's own list shows it), whatever is left to do: undo
    /// lists it for the person to remove if they do not want it.
    pub installed: bool,
    /// A Windows Installer product this install added, as the helper recorded it and Windows'
    /// list confirmed: undo may remove it through the helper.
    pub removable: Option<Box<Removable>>,
}

/// What undo needs to remove an app PCTwin installed: the product the helper recorded (its
/// code and version) and the key of that install in the helper's own journal (from its
/// receipt), which the helper checks before removing anything.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Removable {
    pub product: pctwin_helper::broker::actions::InstalledProduct,
    pub installed_by: pctwin_helper::broker::actions::Hex32,
}

/// An app undo may remove, by its name.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct RemovableApp {
    pub name: String,
    pub removable: Removable,
    /// How an earlier undo of this move left it, once that undo ended for it (removed, found
    /// already gone, or kept because it changed): a later undo never asks for it again (review
    /// finding F1). `None` while it is still to be removed.
    #[serde(default)]
    pub settled: Option<Settled>,
}

/// How an undo ended for an app it may remove.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum Settled {
    Removed,
    AlreadyGone,
    KeptOtherVersion,
}

impl AppResult {
    pub fn because(reason: Reason) -> Self {
        Self {
            result: ItemResult::because(reason),
            todo: Vec::new(),
            installed: false,
            removable: None,
        }
    }

    /// An app installed and checked, with what is left to do: an app that still needs a licence
    /// or a sign-in is never a bare "reinstalled", it needs the person (the licence first, as it
    /// is the step that keeps the app working).
    pub fn installed(result: ItemResult, todo: Vec<pctwin_apps::after::Todo>) -> Self {
        use pctwin_apps::after::Todo;
        let result = if result.status() != ItemStatus::Reinstalled {
            result
        } else if todo.iter().any(|t| matches!(t, Todo::LicenceKey)) {
            ItemResult::because(Reason::Activation)
        } else if todo.iter().any(|t| matches!(t, Todo::SignIn)) {
            ItemResult::because(Reason::SignIn)
        } else {
            result
        };
        Self {
            result,
            todo,
            installed: true,
            removable: None,
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
    /// Waiting for free Wi-Fi before the apps download (the person chose it): at most this many
    /// minutes more, then the apps that need a download end needs-action.
    #[serde(rename_all = "camelCase")]
    WaitingFreeWifi {
        minutes_left: u64,
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
    /// The apps PCTwin installed that undo leaves for the person to remove, by name (installed by
    /// an installer program, or with nothing recorded to remove them by).
    pub apps: Vec<String>,
    /// The Windows Installer apps PCTwin installed that undo removes itself, while each is still
    /// the version installed.
    #[serde(default)]
    pub removable: Vec<RemovableApp>,
    /// App data landed (2.5): its app's name and its landing in the journal.
    #[serde(default)]
    pub landed: Vec<(String, u64)>,
    /// The person's home folder on this laptop, where app data landed.
    #[serde(default)]
    pub home: Option<String>,
    /// When the move finished, in seconds since 1970 (so undo can say which move it undoes);
    /// `None` in a book kept by PCTwin 0.1.x.
    #[serde(default)]
    pub when: Option<u64>,
}

impl UndoBook {
    /// Whether undo has anything to do for this move: settings, apps to remove or list, or app
    /// data that landed.
    pub fn has_anything(&self) -> bool {
        !self.recipes.is_empty()
            || !self.apps.is_empty()
            || !self.removable.is_empty()
            || !self.landed.is_empty()
    }

    /// The book's ID: its move's scope, as hex (one book per move).
    pub fn id(&self) -> String {
        self.scope.iter().map(|b| format!("{b:02x}")).collect()
    }
}

/// The apps PCTwin installed, as undo keeps them: those it can remove itself (a Windows
/// Installer product the helper recorded), and the rest, which undo lists for the person, by
/// name.
pub fn installed_apps(apps: &[(String, AppResult)]) -> (Vec<String>, Vec<RemovableApp>) {
    let mut listed = Vec::new();
    let mut removable = Vec::new();
    for (name, a) in apps {
        match a.removable.as_deref() {
            Some(r) => removable.push(RemovableApp {
                name: name.clone(),
                removable: r.clone(),
                settled: None,
            }),
            None if a.installed || a.result.status() == ItemStatus::Reinstalled => {
                listed.push(name.clone());
            }
            None => {}
        }
    }
    (listed, removable)
}

/// How one line of app data (an app's components with one step) ended.
#[derive(Debug, Clone, PartialEq)]
pub struct SpecialDone {
    pub title: Title,
    pub result: ItemResult,
    /// Why it needs the person, when the landing knows (its first component not done).
    pub why: Option<crate::data::DataWhy>,
    /// It holds saved sign-ins to websites or accounts: the person signs in again here.
    pub sites: bool,
    /// What the person is told because of its special category (2.5), as the plan said it: the
    /// report keeps saying it after the move.
    pub notes: Vec<pctwin_special::category::Note>,
}

impl SpecialDone {
    fn of(title: Title, result: ItemResult) -> Self {
        Self {
            title,
            result,
            why: None,
            sites: false,
            notes: Vec::new(),
        }
    }
}

/// How the apps, settings and special categories of a move ended.
#[derive(Debug, Clone, PartialEq)]
pub struct Rebuilt {
    /// Every app in the plan PCTwin did not leave alone, in plan order.
    pub apps: Vec<(String, AppResult)>,
    /// The apps already on this laptop, left as they are, in plan order: each with the version
    /// here when it is older than the old laptop's. Not an item of the move, so outside the six
    /// result words and their counts, but every one is said.
    pub here: Vec<(String, Option<String>)>,
    /// Each group of settings: its label, how many settings, and its result.
    pub settings: Vec<(String, u64, ItemResult)>,
    pub special: Vec<SpecialDone>,
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
            .chain(self.special.iter().map(|s| s.result));
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
/// The app data that came with the move: the secrets the person ticked (lines of the plan's
/// special list) and each landed component's result, by its ID.
#[derive(Debug, Clone, Default)]
pub struct DataDone {
    pub picked: Vec<u32>,
    pub landed: BTreeMap<String, ItemResult>,
    /// Why each component that needs the person does, by its ID.
    pub why: BTreeMap<String, crate::data::DataWhy>,
    /// Each landing in the journal, by its app's name (for undo).
    pub effects: Vec<(String, u64)>,
    pub home: Option<String>,
    /// Every file of the move that is on this laptop now (copied and checked, or found already
    /// there), as (its full path on the old laptop, its full path where it is stored here): the
    /// move's own record of what landed where, by which a setting that points at a moved file is
    /// pointed at it ([`pctwin_settings::generic::FoundPlan::point_at_landed`]).
    pub files: Vec<(String, String)>,
}

/// The move's own record of what landed where, for [`DataDone::files`]: each file on this laptop
/// now (`here`: the item, and the full path where it is stored here, as the move's results say),
/// with its full path on the old laptop worked out from its folder there (`plan.old_folders`) and
/// its path inside it. A file whose folder the old laptop did not name, or whose name is not
/// plain text, is left out (no setting can name it).
pub fn files_landed<'a>(
    plan: &FullPlan,
    here: impl IntoIterator<Item = (&'a Item, std::path::PathBuf)>,
) -> Vec<(String, String)> {
    let mut out = Vec::new();
    for (item, stored) in here {
        let Some((_, folder)) = plan.old_folders.iter().find(|(r, _)| *r == item.place.role) else {
            continue;
        };
        if item.path.parts().iter().any(|p| p.exact().is_some()) {
            continue;
        }
        let sep = if folder.as_bytes().get(1) == Some(&b':') {
            '\\'
        } else {
            '/'
        };
        let mut old = folder.trim_end_matches(['\\', '/']).to_string();
        for part in item.path.parts() {
            old.push(sep);
            old.push_str(part.display());
        }
        // As this laptop writes its paths (a Windows path never with a `/` inside).
        let mut stored = stored.display().to_string();
        if stored.as_bytes().get(1) == Some(&b':') {
            stored = stored.replace('/', "\\");
        }
        out.push((old, stored));
    }
    out
}

/// Points each setting whose paths were pointed at the moved files at where they really landed
/// (`files`, checked on this laptop's drives now); returns the plan to apply and the settings
/// taken out of it because their file is not there, counted by group.
fn settings_at_landed(
    plan: &FullPlan,
    files: &[(String, String)],
) -> (FullPlan, Vec<(String, u64)>) {
    let mut now = plan.clone();
    if !now.settings.moves.iter().any(|m| m.rewritten) {
        return (now, Vec::new());
    }
    let there = |p: &str| std::fs::symlink_metadata(p).is_ok();
    let landed = pctwin_settings::classify::Landed {
        files,
        there: &there,
    };
    let ctx = pctwin_settings::classify::Ctx {
        paths: &landed,
        same_screens: false,
        hints: pctwin_settings::catalogue::Catalogue::starter_hints(),
        facts: pctwin_settings::judge::Facts::system(),
    };
    let mut gone: Vec<(String, u64)> = Vec::new();
    for n in now.settings.point_at_landed(&ctx) {
        match gone.iter_mut().find(|(l, _)| *l == n.label) {
            Some((_, c)) => *c += 1,
            None => gone.push((n.label, 1)),
        }
    }
    (now, gone)
}

/// Each app's line: its name and result.
pub type AppLines = Vec<(String, AppResult)>;
/// Each app already here: its name, and the version here when it is older.
pub type HereLines = Vec<(String, Option<String>)>;

/// One line per app of `plan`: the installer's results for `to_install` (in order), the
/// person's step for each app PCTwin cannot install, a held app needs-action (Task List 2.9), and
/// the apps already here, said apart.
pub fn app_lines(
    plan: &FullPlan,
    to_install: &[&AppEntry],
    installed: Vec<AppResult>,
) -> (AppLines, HereLines) {
    let mut by_name: Vec<(&str, AppResult)> = to_install
        .iter()
        .map(|e| e.app.name.as_str())
        .zip(installed)
        .collect();
    let mut apps = Vec::new();
    let mut here = Vec::new();
    for (at, e) in plan.apps.iter().enumerate() {
        if plan.held.contains(&(at as u32)) {
            // Changed here after approval: held until the person approves it again, never
            // installed over what is there now nor said to be here (Task List 2.9).
            apps.push((e.app.name.clone(), AppResult::because(Reason::FinishByHand)));
        } else if let Some(reason) = needs_of(&e.plan) {
            // The person installs it; what the catalogue and the old laptop's list know about
            // after (a sign-in, a licence key, its registration) goes with it.
            let card = &e.plan.card;
            let mut r = AppResult::because(reason);
            r.todo = pctwin_apps::after::checklist(
                &e.app,
                card.sign_in,
                card.licence_key,
                pctwin_apps::after::Installed::Failed,
                &[],
            );
            apps.push((e.app.name.clone(), r));
        } else if let Some(at) = by_name.iter().position(|(n, _)| *n == e.app.name) {
            let (_, r) = by_name.remove(at);
            apps.push((e.app.name.clone(), r));
        } else if e.plan.disposition == pctwin_apps::routes::Disposition::AlreadyHere {
            // Already here: left as it is, and said (with the older version here, if it is).
            here.push((e.app.name.clone(), crate::plan::older_here(&e.plan)));
        }
    }
    // An install the installer gave no answer for is never counted as done.
    for e in to_install {
        if !apps.iter().any(|(n, _)| *n == e.app.name) {
            apps.push((e.app.name.clone(), AppResult::because(Reason::NotVerified)));
        }
    }
    (apps, here)
}

#[allow(clippy::too_many_arguments)]
pub fn rebuild(
    plan: &FullPlan,
    data: &DataDone,
    installer: &mut dyn Installer,
    settings: &mut dyn Adapter,
    ledger: &dyn EffectLedger,
    scope: Scope,
    actor: &Actor,
    laptop: LaptopId,
    progress: &mut dyn FnMut(Progress),
) -> Rebuilt {
    // A setting pointed at a moved file is written only once that file is known to be where it
    // points: the plan as it applies, with the move's own record of what landed where.
    let (plan, not_landed) = settings_at_landed(plan, &data.files);
    let plan = &plan;
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
    let (apps, here) = app_lines(plan, &to_install, installed);
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
    let mut settings_lines =
        if ran.lines.is_empty() && !plan.groups.is_empty() && !plan.settings.moves.is_empty() {
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
    // Each setting whose file is not where it would point: left as it was, said so.
    settings_lines.extend(
        not_landed
            .into_iter()
            .map(|(label, n)| (label, n, ItemResult::because(Reason::MovedFileMissing))),
    );
    // Special categories.
    let not_arrived = || {
        (
            ItemResult::because(Reason::FinishByHand),
            Some(crate::data::DataWhy::DidNotArrive),
        )
    };
    let mut special: Vec<SpecialDone> = plan
        .special
        .iter()
        .enumerate()
        .map(|(at, s)| {
            let (result, why) = match special_reason(s.step) {
                Some(reason) => (ItemResult::because(reason), None),
                // A secret not ticked stays on the old laptop, by the person's choice.
                None if s.secret && !data.picked.contains(&(at as u32)) => {
                    (ItemResult::skipped_by_user(), None)
                }
                // Copied only when every component of the line landed copied, by 2.5's own word;
                // one never landed (or not proven) ends needs-action, with why.
                None => {
                    let results: Vec<(ItemResult, Option<crate::data::DataWhy>)> = s
                        .ids
                        .iter()
                        .map(|id| match data.landed.get(id) {
                            Some(r) => (*r, data.why.get(id).cloned()),
                            None => not_arrived(),
                        })
                        .collect();
                    results
                        .iter()
                        .find(|(r, _)| r.status() != ItemStatus::Copied)
                        .or(results.first())
                        .cloned()
                        .unwrap_or_else(not_arrived)
                }
            };
            SpecialDone {
                title: Title::Name(s.name.clone()),
                result,
                why,
                sites: s.sites,
                notes: s.notes.clone(),
            }
        })
        .collect();
    if plan.extras_refused {
        special.push(SpecialDone::of(
            Title::Key(crate::plan::EXTRAS_REFUSED),
            ItemResult::because(Reason::FinishByHand),
        ));
    }
    if plan.special_not_checked > 0 {
        special.push(SpecialDone::of(
            Title::Key(crate::plan::SPECIAL_NOT_CHECKED),
            ItemResult::because(Reason::FinishByHand),
        ));
    }
    if plan.left_out_by_person {
        special.push(SpecialDone::of(
            Title::Key(crate::plan::LEFT_BY_PERSON),
            ItemResult::skipped_by_user(),
        ));
    } else if plan.from.is_some() && !plan.special_searched {
        special.push(SpecialDone::of(
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
    let (listed, removable) = installed_apps(&apps);
    let undo = UndoBook {
        scope: scope.0,
        book: plan.settings.book(),
        recipes: ran.recipes,
        labels,
        apps: listed,
        removable,
        landed: data.effects.clone(),
        home: data.home.clone(),
        when: std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .ok()
            .map(|d| d.as_secs()),
    };
    Rebuilt {
        apps,
        here,
        settings: settings_lines,
        special,
        restore_point: ran.point.or(point),
        stopped: ran.stopped,
        undo,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use pctwin_helper::broker::actions::{
        Hex32, InstallDate, InstalledProduct, ProductCode, ProductVersion,
    };

    fn removable() -> Box<Removable> {
        Box::new(Removable {
            product: InstalledProduct {
                code: ProductCode::parse("{3B0346D3-155D-43E5-9561-B9BE51CF2EB6}").unwrap(),
                version: ProductVersion::parse("2.1.0").unwrap(),
                installed_on: InstallDate::parse("20261028").unwrap(),
            },
            installed_by: Hex32([1; 32]),
        })
    }

    /// Undo removes what the helper recorded, lists the rest PCTwin installed for the person,
    /// and leaves out apps PCTwin did not install.
    #[test]
    fn the_undo_book_keeps_each_installed_app_once_in_its_place() {
        let reinstalled = ItemResult::done(ItemStatus::Reinstalled).unwrap();
        let mut msi = AppResult::installed(reinstalled, vec![]);
        msi.removable = Some(removable());
        let exe = AppResult::installed(reinstalled, vec![]);
        let mut signs_in = AppResult::installed(ItemResult::because(Reason::SignIn), vec![]);
        signs_in.installed = true;
        let manual = AppResult::because(Reason::ManualInstall);
        let apps = vec![
            ("Writer".to_string(), msi),
            ("Tool".to_string(), exe),
            ("Chat".to_string(), signs_in),
            ("Game".to_string(), manual),
        ];
        let (listed, kept) = installed_apps(&apps);
        assert_eq!(listed, ["Tool", "Chat"]);
        assert_eq!(kept.len(), 1);
        assert_eq!(kept[0].name, "Writer");
        assert_eq!(kept[0].removable, *removable());
        // An old book without the field still reads.
        let old: UndoBook = serde_json::from_str(
            r#"{"scope":[0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0],"book":{},"recipes":[],"labels":{},"apps":["Tool"]}"#,
        )
        .unwrap();
        assert!(old.removable.is_empty());
        assert!(old.has_anything());
        let mut empty = old.clone();
        empty.apps.clear();
        assert!(!empty.has_anything());
        // A move whose only change was an app undo removes still has an undo.
        let mut only_removable = empty.clone();
        only_removable.removable = kept.clone();
        assert!(only_removable.has_anything());
        let mut only_landed = empty.clone();
        only_landed.landed = vec![("Mail".into(), 1)];
        assert!(only_landed.has_anything());
        let mut only_settings = empty.clone();
        only_settings.recipes = vec![
            serde_json::from_str(
                r#"{"format":1,"id":"org.example.app","version":1,"platforms":[],"steps":[]}"#,
            )
            .unwrap(),
        ];
        assert!(only_settings.has_anything());
        let mut book = old.clone();
        book.removable = kept;
        let json = serde_json::to_string(&book).unwrap();
        assert_eq!(serde_json::from_str::<UndoBook>(&json).unwrap(), book);
    }
}
