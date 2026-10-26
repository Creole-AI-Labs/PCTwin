//! PCTwin executor (engineering notes, building block 4; Security Design B and C; System Design
//! S14; Phase 2 plans 2.1, 2.3 and 2.4).
//!
//! The executor rebuilds apps and settings on the new laptop from an approved plan, and nothing
//! else:
//!
//! - A [`Plan`] can only be made from a move record together with the person's
//!   [`Approval`] of that exact revision (re-checked against every approval rule, so a hand-made
//!   approval of a record that could not be approved fails), and only for apps and settings in
//!   that record that are part of the move. A changed record needs a new approval.
//! - Known gap until recipes join the record (Task List 2.1): the approval covers the record, not
//!   which recipe version rebuilds each item.
//! - Each planned item runs one checked [`Recipe`] through an OS [`Adapter`], step by typed step.
//!   The adapter is where each operating system does the work; it receives typed steps only,
//!   never a command line or an address.
//! - Every step that changes the laptop is journaled ([`EffectLedger`]) in the order S14 sets,
//!   installs through the installs role and settings through the settings role: what is there
//!   now is looked at first and recorded with the intent, durably, before anything changes; then
//!   the change; then what is there after it is looked at and recorded; then the step's honest
//!   receipt. An app already on the laptop is never installed again, and nothing is changed while
//!   an earlier change to the same app, setting or place is still unaccounted for. If the journal
//!   cannot be written, nothing more is changed and the run stops ([`Run::stopped`]).
//! - Before a move's first install or setting, a Windows restore point is asked for once, and its
//!   honest answer recorded ([`Adapter::restore_point`]).
//! - Before every run, changes a crash interrupted are reconciled ([`recover`]): the system is
//!   asked what is actually there now; an installer is never run again blindly, nor while it may
//!   still be running.
//! - Every item ends with one honest [`ItemResult`]: each changing step is checked right after it
//!   runs against what it changed; reinstalled only when something was installed and every check
//!   passed; failed or needs-action with its reason otherwise. A recipe that does not support the
//!   new laptop's system fails with [`Reason::NoEquivalent`] before anything runs.

mod recover;
mod undo;

pub use recover::{Recovered, RecoveredAs, STILL_INSTALLING, recover};
pub use undo::{Undone, undo_effects};

use pctwin_journal::{
    Actor, EffectEntry, EffectLedger, Found, InstallIntent, JournalError, PlannedEffect, Presence,
    Receipt, RestorePoint, Role, Scope, SettingsFile, SettingsIntent, StoredEffect,
};
use pctwin_recipe::{Catalog, Check, Platform, Recipe, SettingValue, SettingsPlace, Step};
use pctwin_record::{Approval, Inclusion, ItemId, ItemKind, Record, RecordError};
use pctwin_types::{ItemResult, ItemStatus, Reason};

/// Why a plan was refused.
#[derive(Debug, thiserror::Error)]
pub enum PlanError {
    #[error(transparent)]
    Record(#[from] RecordError),
    #[error("item {0} is not in the approved record")]
    UnknownItem(String),
    #[error("item {0} is not part of the move")]
    LeftOut(String),
    #[error("item {0} is planned twice")]
    Twice(String),
    #[error("item {0} is not an app or a setting, so no recipe rebuilds it")]
    NotRebuildable(String),
    #[error("item {0} is a setting, so its recipe may not install anything")]
    InstallForSetting(String),
    #[error("a recipe in the plan is not valid: {0}")]
    BadRecipe(String),
}

/// The work an OS adapter does for each typed step. Each changing method returns the reason when
/// the step did not succeed. The looking methods never change anything, and answer `Err` (why,
/// in plain words) whenever they cannot tell for sure: never a guess such as "absent" because a
/// catalog tool failed (catalog tools disagree on what their "not found" exit codes mean).
pub trait Adapter {
    /// The new laptop's system and processor.
    fn platform(&self) -> Platform;
    fn install(&mut self, catalog: Catalog, package: &str) -> Result<(), Reason>;
    fn copy_settings(&mut self, from: &SettingsPlace, to: &SettingsPlace) -> Result<(), Reason>;
    fn apply_setting(&mut self, key: &str, value: &SettingValue) -> Result<(), Reason>;
    /// Whether a check passes on the new laptop.
    fn verify(&mut self, check: &Check) -> bool;

    /// Whether a catalog's package is on the laptop now, with its version if the catalog says.
    fn app_presence(&mut self, catalog: Catalog, package: &str) -> Result<Presence, String>;
    /// Whether an install of this package (or the system's installer, which runs one install at
    /// a time) may still be running now, after PCTwin stopped waiting for it. `Err` when it
    /// cannot be told: then nothing is installed.
    fn install_running(&mut self, catalog: Catalog, package: &str) -> Result<bool, String>;
    /// A named setting's value now; `None` when it is not set at all (kept apart from any
    /// value, so undo removes it rather than writing an empty one). On macOS through the
    /// preferences system (`CFPreferences`), never by reading or writing a preferences file,
    /// which the system caches.
    fn read_setting(&mut self, key: &str) -> Result<Option<SettingValue>, String>;
    /// Every settings file at a place now, whole (none when nothing is there; the empty path
    /// when the place is itself one file).
    fn read_settings_files(&mut self, place: &SettingsPlace) -> Result<Vec<SettingsFile>, String>;

    /// Asks for a Windows restore point (through the administrator helper, which owns asking)
    /// and says what really happened; elsewhere [`RestorePoint::NotWindows`].
    fn restore_point(&mut self) -> RestorePoint;

    /// Removes an app through its own catalog, keeping the person's data: never a purge, a zap
    /// or anything that removes settings or files the person made. Only undo calls it.
    fn uninstall(&mut self, catalog: Catalog, package: &str) -> Result<(), String>;
    /// Sets a named setting back to `value`, or removes it when `None` (it was not set before).
    /// Only undo calls it.
    fn write_setting(&mut self, key: &str, value: Option<&SettingValue>) -> Result<(), String>;
    /// Puts one settings file at a place back to `bytes`, or removes it when `None` (it was not
    /// there before). Only undo calls it.
    fn put_settings_file(
        &mut self,
        place: &SettingsPlace,
        path: &str,
        bytes: Option<&[u8]>,
    ) -> Result<(), String>;
}

/// One planned item: the record item it rebuilds, and the recipe that rebuilds it.
#[derive(Debug, Clone)]
pub struct PlannedItem {
    pub item: ItemId,
    pub recipe: Recipe,
}

/// An approved plan. It can only be made by [`Plan::approved`].
#[derive(Debug)]
pub struct Plan {
    items: Vec<PlannedItem>,
}

/// What a run did.
#[derive(Debug, Clone, PartialEq)]
pub struct Run {
    /// Changes an earlier run left part way, reconciled before this run changed anything.
    pub recovered: Vec<Recovered>,
    /// The restore point answer recorded for this move (asked before its first install or
    /// setting), if it has been asked.
    pub restore_point: Option<RestorePoint>,
    /// Each item's result, in plan order (every item, also those not reached).
    pub results: Vec<(ItemId, ItemResult)>,
    /// Why the run stopped before the end, in plain words, if it did: the journal could not be
    /// written, so nothing more was changed. Items not reached failed with
    /// [`Reason::NotVerified`].
    pub stopped: Option<String>,
}

/// The journal could not be written: nothing more may be changed.
#[derive(Debug)]
pub(crate) struct Stop;

impl From<JournalError> for Stop {
    fn from(_: JournalError) -> Self {
        Stop
    }
}

/// Why a stopped run stopped, in plain words.
pub const STOPPED: &str = "PCTwin could not write down its next change, so it stopped before \
     making it; nothing after that point was changed";

/// Who and what a run works for, and where it records.
struct Ctx<'a> {
    journal: &'a dyn EffectLedger,
    scope: Scope,
    actor: &'a Actor,
    /// The restore point answer, once asked or found recorded.
    restore_point: std::cell::RefCell<Option<RestorePoint>>,
}

impl Plan {
    /// Builds the plan for exactly the record revision the person approved.
    pub fn approved(
        record: &Record,
        approval: &Approval,
        items: Vec<PlannedItem>,
    ) -> Result<Self, PlanError> {
        record.check(approval)?;
        let mut seen = std::collections::HashSet::new();
        for p in &items {
            let hex = p.item.to_hex();
            let found = record
                .items
                .iter()
                .find(|i| i.id == p.item)
                .ok_or_else(|| PlanError::UnknownItem(hex.clone()))?;
            if found.inclusion != Inclusion::Included {
                return Err(PlanError::LeftOut(hex));
            }
            if !matches!(found.kind, ItemKind::App | ItemKind::Setting) {
                return Err(PlanError::NotRebuildable(hex));
            }
            if found.kind == ItemKind::Setting
                && p.recipe
                    .steps
                    .iter()
                    .any(|s| matches!(s, Step::Install { .. }))
            {
                return Err(PlanError::InstallForSetting(hex));
            }
            if !seen.insert(p.item) {
                return Err(PlanError::Twice(hex));
            }
            p.recipe
                .validate()
                .map_err(|e| PlanError::BadRecipe(e.to_string()))?;
        }
        Ok(Self { items })
    }

    pub fn items(&self) -> &[PlannedItem] {
        &self.items
    }

    /// Runs every planned item for move `scope`, as `actor`, recording each change in
    /// `journal`; first reconciles any change a crash left part way. Returns each item's
    /// result, in plan order.
    pub fn run(
        &self,
        adapter: &mut dyn Adapter,
        journal: &dyn EffectLedger,
        scope: Scope,
        actor: &Actor,
    ) -> Run {
        let not_reached = |items: &[PlannedItem]| -> Vec<(ItemId, ItemResult)> {
            items
                .iter()
                .map(|p| (p.item, ItemResult::because(Reason::NotVerified)))
                .collect()
        };
        let stopped = |recovered, restore_point, results| Run {
            recovered,
            restore_point,
            results,
            stopped: Some(STOPPED.into()),
        };
        let (Ok(recovered), Ok(point)) = (recover(journal, adapter), journal.restore_point(scope))
        else {
            return stopped(Vec::new(), None, not_reached(&self.items));
        };
        let ctx = Ctx {
            journal,
            scope,
            actor,
            restore_point: std::cell::RefCell::new(point),
        };
        let mut results = Vec::with_capacity(self.items.len());
        for (n, p) in self.items.iter().enumerate() {
            match run_recipe(p, adapter, &ctx) {
                Ok(result) => results.push((p.item, result)),
                Err(Stop) => {
                    results.extend(not_reached(&self.items[n..]));
                    return stopped(recovered, ctx.restore_point.into_inner(), results);
                }
            }
        }
        Run {
            recovered,
            restore_point: ctx.restore_point.into_inner(),
            results,
            stopped: None,
        }
    }
}

/// Runs one recipe. Each step that changes something is journaled and checked right after it
/// runs, against what it changed; it stops at the first step that does not succeed. The item is
/// reinstalled only when the recipe installed something and every check passed; a recipe that
/// only changed settings converted them.
fn run_recipe(
    p: &PlannedItem,
    adapter: &mut dyn Adapter,
    ctx: &Ctx<'_>,
) -> Result<ItemResult, Stop> {
    let recipe = &p.recipe;
    if !recipe.supports(adapter.platform()) {
        return Ok(ItemResult::because(Reason::NoEquivalent));
    }
    let mut by_hand = false;
    let mut installed = false;
    for (n, step) in recipe.steps.iter().enumerate() {
        let done = match step {
            Step::Install { .. } | Step::CopySettings { .. } | Step::ApplySetting { .. } => {
                installed |= matches!(step, Step::Install { .. });
                let step_no = u32::try_from(n).unwrap_or(u32::MAX);
                effect(p, step_no, step, adapter, ctx)?
            }
            Step::Verify { check } => checked(adapter, check),
            Step::Checklist { .. } => {
                by_hand = true;
                Ok(())
            }
        };
        if let Err(reason) = done {
            return Ok(ItemResult::because(reason));
        }
    }
    let status = if installed {
        ItemStatus::Reinstalled
    } else {
        ItemStatus::Converted
    };
    let changed = recipe.steps.iter().any(|s| s.check().is_some());
    Ok(match (by_hand, changed) {
        (true, _) => ItemResult::because(Reason::FinishByHand),
        (false, true) => {
            ItemResult::done(status).unwrap_or(ItemResult::because(Reason::NotVerified))
        }
        // Validation refuses recipes that change nothing; never report one as done.
        (false, false) => ItemResult::because(Reason::NotVerified),
    })
}

/// What a step changes, to tell whether an earlier change to it is still unaccounted for.
#[derive(PartialEq)]
enum Target<'a> {
    App(Catalog, &'a str),
    Setting(&'a str),
    Place(&'a SettingsPlace),
}

fn target_of_step(step: &Step) -> Option<Target<'_>> {
    match step {
        Step::Install { catalog, package } => Some(Target::App(*catalog, package)),
        Step::ApplySetting { key, .. } => Some(Target::Setting(key)),
        Step::CopySettings { to, .. } => Some(Target::Place(to)),
        Step::Verify { .. } | Step::Checklist { .. } => None,
    }
}

fn target_of_effect(e: &StoredEffect) -> Option<Target<'_>> {
    match e {
        StoredEffect::Install {
            catalog, package, ..
        } => Some(Target::App(*catalog, package)),
        StoredEffect::ApplySetting { key, .. } => Some(Target::Setting(key)),
        StoredEffect::CopySettings { to, .. } | StoredEffect::Import { to, .. } => {
            Some(Target::Place(to))
        }
        StoredEffect::External { .. } => None,
    }
}

/// An intent for the role that records it.
enum Intent {
    Install(InstallIntent),
    Setting(SettingsIntent),
}

/// Asks for the move's restore point once, before its first install or setting, and records the
/// honest answer. Already recorded (an earlier run of this move): not asked again.
fn ensure_restore_point(adapter: &mut dyn Adapter, ctx: &Ctx<'_>) -> Result<(), Stop> {
    if ctx.restore_point.borrow().is_some() {
        return Ok(());
    }
    let point = adapter.restore_point();
    ctx.journal.record_restore_point(ctx.scope, &point)?;
    *ctx.restore_point.borrow_mut() = Some(point);
    Ok(())
}

/// One changing step, journaled: what is there now (recorded with the intent, durably, before
/// anything changes), the change, what is there after it (recorded), then the step's receipt.
/// `Err(Stop)` when the journal could not be written; `Ok(Err(reason))` when the step did not
/// succeed.
fn effect(
    p: &PlannedItem,
    step_no: u32,
    step: &Step,
    adapter: &mut dyn Adapter,
    ctx: &Ctx<'_>,
) -> Result<Result<(), Reason>, Stop> {
    // 0. Nothing is changed while an earlier change to the same thing is unaccounted for (a
    //    crash's change recovery could not look at yet, an installer that may still run).
    let target = target_of_step(step);
    let unfinished = ctx.journal.unfinished_effects()?;
    if unfinished
        .iter()
        .any(|e| target.is_some() && target_of_effect(&e.effect) == target)
    {
        return Ok(Err(Reason::NotVerified));
    }
    let planned = PlannedEffect {
        scope: ctx.scope,
        item: p.item,
        recipe: p.recipe.reference(),
        step: step_no,
        actor: ctx.actor.clone(),
    };
    // 1. What is there now. Without knowing it, undo could not be honest: nothing is changed.
    let intended = match step {
        Step::Install { catalog, package } => {
            let Ok(before) = adapter.app_presence(*catalog, package) else {
                return Ok(Err(Reason::NotVerified));
            };
            let stored = StoredEffect::Install {
                catalog: *catalog,
                package: package.clone(),
                before: before.clone(),
                undo: p.recipe.undo.clone(),
            };
            let intent = InstallIntent::App {
                catalog: *catalog,
                package: package.clone(),
                before,
                undo: p.recipe.undo.clone(),
            };
            (Intent::Install(intent), stored)
        }
        Step::ApplySetting { key, value } => {
            let Ok(before) = adapter.read_setting(key) else {
                return Ok(Err(Reason::NotVerified));
            };
            let stored = StoredEffect::ApplySetting {
                key: key.clone(),
                before: before.clone(),
                value: value.clone(),
            };
            let intent = SettingsIntent::Apply {
                key: key.clone(),
                before,
                value: value.clone(),
            };
            (Intent::Setting(intent), stored)
        }
        Step::CopySettings { from, to } => {
            let Ok(before) = adapter.read_settings_files(to) else {
                return Ok(Err(Reason::NotVerified));
            };
            let stored = StoredEffect::CopySettings {
                from: from.clone(),
                to: to.clone(),
                before: Vec::new(),
            };
            let intent = SettingsIntent::Copy {
                from: from.clone(),
                to: to.clone(),
                before,
            };
            (Intent::Setting(intent), stored)
        }
        Step::Verify { .. } | Step::Checklist { .. } => return Ok(Ok(())),
    };
    let (intent, stored) = intended;
    // 2. The restore point, once, before the move's first install or setting.
    ensure_restore_point(adapter, ctx)?;
    // 3. The intent with what was there, on disk before anything changes. What cannot be kept
    //    safely (too large, a strange name) means the change is not made.
    let (written, role) = match &intent {
        Intent::Install(i) => (ctx.journal.intend_install(&planned, i), Role::Install),
        Intent::Setting(i) => (ctx.journal.intend_setting(&planned, i), Role::Settings),
    };
    let id = match written {
        Ok(id) => id,
        Err(JournalError::TooLarge | JournalError::BadName(_)) => {
            return Ok(Err(Reason::NotVerified));
        }
        Err(_) => return Err(Stop),
    };
    // 4. The change. An app already there is never installed again.
    let acted = match (step, &stored) {
        (
            Step::Install { catalog, package },
            StoredEffect::Install {
                before: Presence::Absent,
                ..
            },
        ) => adapter.install(*catalog, package),
        (Step::ApplySetting { key, value }, _) => adapter.apply_setting(key, value),
        (Step::CopySettings { from, to }, _) => adapter.copy_settings(from, to),
        _ => Ok(()),
    };
    // 5. What is there now. If it cannot be told, the intent stays as it is and the next run's
    //    recovery looks again; this step is not counted done.
    let Ok(found) = look(adapter, &stored) else {
        return Ok(Err(Reason::NotVerified));
    };
    ctx.journal.observed(role, id, &found, false)?;
    // 6. The receipt, from what was found and the step's own check (not run when the change
    //    itself failed).
    let passed = acted.is_ok() && check_of(&stored).is_some_and(|c| adapter.verify(&c));
    let receipt = judge(&stored, &found, acted.err(), passed);
    ctx.journal.receipt(role, id, &receipt)?;
    Ok(match receipt {
        Receipt::Done => Ok(()),
        Receipt::Failed { reason } => Err(reason),
    })
}

/// Whether recovery and undo here act on this effect: the installs and settings roles' own.
/// Imports belong to the imports role (2.5), which reconciles and undoes its own.
pub(crate) fn ours(e: &EffectEntry) -> bool {
    matches!(e.role, Role::Install | Role::Settings)
}

/// What is on the laptop now for an effect: the app, the setting, or every file at the place
/// with its content hash. A change made with a service or maker cannot be looked at here.
pub(crate) fn look(adapter: &mut dyn Adapter, effect: &StoredEffect) -> Result<Found, String> {
    Ok(match effect {
        StoredEffect::Install {
            catalog, package, ..
        } => Found::App(adapter.app_presence(*catalog, package)?),
        StoredEffect::ApplySetting { key, .. } => Found::Setting(adapter.read_setting(key)?),
        StoredEffect::CopySettings { to, .. } | StoredEffect::Import { to, .. } => {
            Found::Files(hashes(adapter.read_settings_files(to)?))
        }
        StoredEffect::External { .. } => {
            return Err("only the service or the app's maker can say whether this happened".into());
        }
    })
}

/// Each file's path and content hash.
pub(crate) fn hashes(files: Vec<SettingsFile>) -> Vec<pctwin_journal::FileHash> {
    files
        .into_iter()
        .map(|f| pctwin_journal::FileHash {
            hash: pctwin_journal::content_hash(&f.bytes),
            path: f.path,
        })
        .collect()
}

/// The check of what an effect changed (the step's own check); none for a change made away
/// from this laptop's files.
pub(crate) fn check_of(effect: &StoredEffect) -> Option<Check> {
    match effect {
        StoredEffect::Install {
            catalog, package, ..
        } => Some(Check::PackageInstalled {
            catalog: *catalog,
            package: package.clone(),
        }),
        StoredEffect::ApplySetting { key, value, .. } => Some(Check::SettingIs {
            key: key.clone(),
            value: value.clone(),
        }),
        StoredEffect::CopySettings { from, to, .. } => Some(Check::SettingsMatch {
            from: from.clone(),
            to: to.clone(),
        }),
        StoredEffect::Import { .. } | StoredEffect::External { .. } => None,
    }
}

/// A step's honest receipt: done only when what was found is what the step was to make, the
/// change itself reported no failure (`acted`; `None` after a restart, when it is not known),
/// and the step's own check passed. Otherwise failed, with the change's own reason if it gave
/// one.
pub(crate) fn judge(
    effect: &StoredEffect,
    found: &Found,
    acted: Option<Reason>,
    check_passed: bool,
) -> Receipt {
    let as_wanted = match (effect, found) {
        (StoredEffect::Install { .. }, Found::App(Presence::Present { .. })) => true,
        (StoredEffect::ApplySetting { value, .. }, Found::Setting(Some(now))) => now == value,
        // The settings check compares the place with the old laptop's, file by file.
        (StoredEffect::CopySettings { .. }, Found::Files(_)) => true,
        _ => false,
    };
    match acted {
        Some(reason) => Receipt::Failed { reason },
        None if as_wanted && check_passed => Receipt::Done,
        None => Receipt::Failed {
            reason: Reason::NotVerified,
        },
    }
}

fn checked(adapter: &mut dyn Adapter, check: &Check) -> Result<(), Reason> {
    if adapter.verify(check) {
        Ok(())
    } else {
        Err(Reason::NotVerified)
    }
}
