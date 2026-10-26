//! The effect journal: installs, settings and private-data imports (System Design S14, "Effect
//! journal and object identity"; Security Design part B, "Configuration preimages and supported
//! app compensations follow their own journal contracts"; Phase 2 plan 2.3, adopted 26 October
//! 2026).
//!
//! Every change to an app, a setting or imported data on the new laptop is one effect, recorded
//! in this order, each step its own durable transaction:
//!
//! 1. **Intent**: what is about to change, and what was there before it (the app absent or
//!    present with its version; the setting's value or "not set"; the files at the place, kept
//!    whole inside the journal with their content hashes). On disk before anything changes.
//! 2. The change itself (done by the system, outside the journal).
//! 3. **Observed**: what is actually there now, looked at after the change (or, after a crash,
//!    looked at by recovery: never assumed).
//! 4. **Receipt**: the step's honest result, worked out from what was observed. Only then is the
//!    effect finished.
//!
//! Steps cannot be skipped, repeated or taken back. A crash leaves the effect at the last step on
//! disk: an intent with no observation means the change may or may not have happened, and
//! recovery asks the system what is there now instead of running the change again. A late step
//! from a worker that was replaced (recovery already finished the effect) is refused.
//!
//! **Each effect belongs to one role** ([`Role`]: 2.1 installs apps, 2.4 restores settings, 2.5
//! imports private data), and every write goes through that role's own [`Recorder`]: an
//! installs recorder can only record what [`InstallIntent`] can say, a settings recorder only
//! what [`SettingsIntent`] can say, and a recorder refuses to observe, finish or undo an effect
//! of another role ([`JournalError::WrongRole`]). Each effect also has its class
//! ([`EffectClass`], the plan's section 3 table), which decides how far undo can go: a change made
//! with an online service or a maker's licence is never claimed undone here, and destroying data
//! on purpose is never something undo does.
//!
//! Undoing an effect needs the same undo right as undoing a file, so the undo gate covers effects
//! too, and closing undo refuses while an effect is part way put back
//! ([`crate::CloseError::EffectsPartWay`]). What was there before is kept for good, never removed
//! to save room or as a "temporary" file, also after undo has closed.

use std::marker::PhantomData;

use pctwin_grant::{
    CaptureKey, GrantError, InstallGrant, InstallKey, Now, ObjectRef, PlanSeal, SettingGrant,
    SettingKey,
};
use pctwin_recipe::{Catalog, RecipeRef, SettingValue, SettingsPlace, UndoDeclaration};
use pctwin_record::ItemId;
use pctwin_types::Reason;
use redb::{ReadableDatabase, ReadableTable, ReadableTableMetadata, TableDefinition};
use serde::{Deserialize, Serialize};

use crate::keystore::{KeyError, TagKey};
use crate::{
    Actor, GATE, Journal, JournalError, Scope, UNDO_GATE, UndoRight, damaged, storage, unreadable,
};

/// Every effect, by number.
pub(crate) const EFFECTS: TableDefinition<u64, &[u8]> = TableDefinition::new("effects");
/// Effects with no receipt yet, so a restart reads only those.
pub(crate) const EFFECTS_UNFINISHED: TableDefinition<u64, ()> =
    TableDefinition::new("effects-unfinished");
/// Files that were there before a change, whole, by their content hash.
pub(crate) const KEPT_FILES: TableDefinition<[u8; 32], &[u8]> = TableDefinition::new("kept-files");
/// Undo of each effect, by effect number: started, then how it ended.
pub(crate) const EFFECT_UNDO: TableDefinition<u64, &[u8]> = TableDefinition::new("effect-undo");
/// The Windows restore point asked for before a move's first change, by the move's scope.
pub(crate) const RESTORE_POINTS: TableDefinition<&str, &[u8]> =
    TableDefinition::new("restore-points");
/// The next effect number (in the `counters` table).
pub(crate) const NEXT_EFFECT: &str = "next-effect";

/// The largest file kept from before a change. A larger one stops the change before anything
/// changes ([`JournalError::TooLarge`]).
pub const MAX_KEPT_FILE: usize = 16 * 1024 * 1024;
/// The most files kept from before one change.
pub const MAX_KEPT_FILES: usize = 1000;
/// The most bytes kept from before one change, all files together.
pub const MAX_KEPT_TOTAL: usize = 64 * 1024 * 1024;
/// The longest identity of a change made with an online service or a maker.
pub const MAX_IDENTITY: usize = 200;

/// The content hash of a kept file (BLAKE3).
pub fn content_hash(bytes: &[u8]) -> [u8; 32] {
    blake3::hash(bytes).into()
}

/// The role an effect belongs to (Phase 2 plan, adopted 26 October 2026).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum Role {
    /// 2.1: installing apps.
    Install,
    /// 2.4: restoring settings.
    Settings,
    /// 2.5: importing private data (mail, browser data and the like).
    Import,
}

/// What kind of change an effect is, and so what undo can mean for it (Phase 2 plan 2.3,
/// section 3).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum EffectClass {
    /// Files PCTwin made where there were none: removed only while unchanged.
    OwnedStagedFile,
    /// Files PCTwin replaced: their earlier versions put back only while PCTwin's are unchanged.
    ExistingTargetReplacement,
    /// An app installed from its catalog: the real state is looked at; removal only as declared.
    NativePackageInstall,
    /// A setting: put back only while it still holds what PCTwin set.
    SettingsChange,
    /// Private data imported into an app's place: put back file by file, only while unchanged.
    PrivateDataImport,
    /// Done with an online service (an upload to a mailbox, say): only the service can undo it.
    RemoteServiceEffect,
    /// A sign-in, licence or activation: the app's maker governs it; no undo here.
    CredentialOrLicenceChange,
    /// Destroying data on purpose (the old laptop's wipe, under its own approval): never undone,
    /// and never recorded as an effect undo could act on.
    DestructiveSanitisation,
}

/// How far undo can go for a class of effect.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum UndoRoute {
    /// Undo here may put it back, under its own checks.
    Here,
    /// Only the service or maker can undo it: the person is told, nothing is claimed.
    Elsewhere,
    /// Never undone.
    Never,
}

impl EffectClass {
    /// How far undo can go for this class.
    pub fn undo_route(self) -> UndoRoute {
        match self {
            EffectClass::OwnedStagedFile
            | EffectClass::ExistingTargetReplacement
            | EffectClass::NativePackageInstall
            | EffectClass::SettingsChange
            | EffectClass::PrivateDataImport => UndoRoute::Here,
            EffectClass::RemoteServiceEffect | EffectClass::CredentialOrLicenceChange => {
                UndoRoute::Elsewhere
            }
            EffectClass::DestructiveSanitisation => UndoRoute::Never,
        }
    }
}

/// A change made away from this laptop's files, which only its service or maker can undo.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum ExternalClass {
    /// With an online service (a mailbox upload, a cloud sync).
    RemoteService,
    /// A sign-in, licence or activation with an app's maker.
    CredentialOrLicence,
}

/// Whether an app was on the laptop.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum Presence {
    Absent,
    /// There, with its version if the system says one.
    Present {
        version: Option<String>,
    },
}

/// A file read at a place: its path inside the place (plain names joined by `/`; empty when the
/// place is itself one file) and its bytes.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SettingsFile {
    pub path: String,
    pub bytes: Vec<u8>,
}

/// A file kept in the journal from before a change: where it was, its content hash and its size.
/// Its bytes are read back with [`Journal::kept_bytes`].
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct KeptFile {
    pub path: String,
    pub hash: [u8; 32],
    pub size: u64,
}

/// A file found at a place: its path inside the place and its content hash.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct FileHash {
    pub path: String,
    pub hash: [u8; 32],
}

/// Which move, item, recipe step and person an effect is for.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct PlannedEffect {
    pub scope: Scope,
    pub item: ItemId,
    /// The exact recipe version the plan was approved with.
    pub recipe: RecipeRef,
    /// The step's position in the recipe (from 0).
    pub step: u32,
    pub actor: Actor,
}

/// What the installs role (2.1) can record: nothing else.
#[derive(Debug, Clone, PartialEq)]
pub enum InstallIntent {
    /// Install a package from its catalog. `undo` is the recipe's declaration, kept with it so
    /// undo follows what was approved, not a later recipe.
    App {
        catalog: Catalog,
        package: String,
        before: Presence,
        undo: UndoDeclaration,
    },
    /// A licence or activation with the app's maker, by an identity (never a key or password).
    Licence { identity: String },
}

/// What the settings role (2.4) can record: nothing else.
#[derive(Debug, Clone, PartialEq)]
pub enum SettingsIntent {
    /// Set one named setting. `before`: its value, or `None` when it was not set.
    Apply {
        key: String,
        before: Option<SettingValue>,
        value: SettingValue,
    },
    /// Copy settings into `to`. `before`: every file at `to` before the copy (none when there
    /// was nothing there), kept whole in the journal.
    Copy {
        from: SettingsPlace,
        to: SettingsPlace,
        before: Vec<SettingsFile>,
    },
}

/// What the imports role (2.5) can record: nothing else.
#[derive(Debug, Clone, PartialEq)]
pub enum ImportIntent {
    /// Import private data into an app's place. `before`: every file there before, kept whole.
    Files {
        to: SettingsPlace,
        before: Vec<SettingsFile>,
    },
    /// Data sent to an online service (a mailbox upload), by the service's identity for it
    /// (never a password or the data itself).
    Remote { identity: String },
}

/// An effect as the journal keeps it.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum StoredEffect {
    Install {
        catalog: Catalog,
        package: String,
        before: Presence,
        undo: UndoDeclaration,
    },
    ApplySetting {
        key: String,
        before: Option<SettingValue>,
        value: SettingValue,
    },
    CopySettings {
        from: SettingsPlace,
        to: SettingsPlace,
        before: Vec<KeptFile>,
    },
    Import {
        to: SettingsPlace,
        before: Vec<KeptFile>,
    },
    External {
        class: ExternalClass,
        identity: String,
    },
}

impl StoredEffect {
    fn kind(&self) -> &'static str {
        match self {
            StoredEffect::Install { .. } => "install",
            StoredEffect::ApplySetting { .. } => "apply-setting",
            StoredEffect::CopySettings { .. } | StoredEffect::Import { .. } => "files",
            StoredEffect::External { .. } => "external",
        }
    }

    /// Its class (Phase 2 plan 2.3, section 3).
    pub fn class(&self) -> EffectClass {
        match self {
            StoredEffect::Install { .. } => EffectClass::NativePackageInstall,
            StoredEffect::ApplySetting { .. } => EffectClass::SettingsChange,
            StoredEffect::CopySettings { before, .. } if before.is_empty() => {
                EffectClass::OwnedStagedFile
            }
            StoredEffect::CopySettings { .. } => EffectClass::ExistingTargetReplacement,
            StoredEffect::Import { .. } => EffectClass::PrivateDataImport,
            StoredEffect::External {
                class: ExternalClass::RemoteService,
                ..
            } => EffectClass::RemoteServiceEffect,
            StoredEffect::External {
                class: ExternalClass::CredentialOrLicence,
                ..
            } => EffectClass::CredentialOrLicenceChange,
        }
    }
}

/// What was found on the laptop after a change (or, after a crash, by recovery).
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum Found {
    /// After an install: whether the app is there.
    App(Presence),
    /// After a setting: its value, or `None` when it is not set.
    Setting(Option<SettingValue>),
    /// After a copy or an import: every file at the place, with its content hash.
    Files(Vec<FileHash>),
    /// After a change made with a service or maker: whether it says it happened.
    External { happened: bool },
}

impl Found {
    fn kind(&self) -> &'static str {
        match self {
            Found::App(_) => "install",
            Found::Setting(_) => "apply-setting",
            Found::Files(_) => "files",
            Found::External { .. } => "external",
        }
    }
}

/// A step's honest result.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum Receipt {
    /// It changed what it was to change, and its check passed.
    Done,
    /// It did not, and why.
    Failed { reason: Reason },
}

/// How far an effect got.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case", tag = "step")]
pub enum EffectState {
    /// Recorded before the change; nothing is known of the change yet.
    Intended,
    /// What was found after the change; `after_restart` when recovery found it after a crash.
    Observed { found: Found, after_restart: bool },
    /// Finished, with its result.
    Receipted {
        found: Found,
        after_restart: bool,
        receipt: Receipt,
    },
}

impl EffectState {
    /// The step's name, as plain as it gets.
    pub fn name(&self) -> &'static str {
        match self {
            EffectState::Intended => "intended",
            EffectState::Observed { .. } => "observed",
            EffectState::Receipted { .. } => "receipted",
        }
    }
}

/// One effect and how far it got.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct EffectEntry {
    pub id: u64,
    /// The role it belongs to.
    pub role: Role,
    pub planned: PlannedEffect,
    pub effect: StoredEffect,
    pub state: EffectState,
}

/// What support may see of an effect (Phase 2 plan 2.3, "minimal redacted support
/// projection"): its number, role, class, step, result and whether it was found after a restart;
/// never a setting's value, a key, a path, a package name or an identity.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct SupportView {
    pub id: u64,
    pub role: Role,
    pub class: EffectClass,
    pub step: &'static str,
    pub receipt: Option<Receipt>,
    pub after_restart: bool,
}

impl EffectEntry {
    /// What support may see of it.
    pub fn support_view(&self) -> SupportView {
        let (receipt, after_restart) = match &self.state {
            EffectState::Intended => (None, false),
            EffectState::Observed { after_restart, .. } => (None, *after_restart),
            EffectState::Receipted {
                receipt,
                after_restart,
                ..
            } => (Some(*receipt), *after_restart),
        };
        SupportView {
            id: self.id,
            role: self.role,
            class: self.effect.class(),
            step: self.state.name(),
            receipt,
            after_restart,
        }
    }
}

/// Undoing one effect, recorded beside it (the effect's own record never changes).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case", tag = "step")]
pub enum EffectUndo {
    /// About to put it back: recorded before anything is changed, so after a crash undo knows
    /// it may be part way, and closing undo waits for it.
    Reversing,
    /// About to put the settings file `path` (inside the effect's place) back, moving it aside
    /// first to the private name `private` (`.pctwin-undo-` and 32 hexadecimal digits) in its
    /// folder: recorded before it is moved, so after a crash undo finds it there and finishes
    /// (security review finding 2). Recorded again for each file.
    Swapping { path: String, private: String },
    /// Done; never written over, unless it could not be done this time.
    Done { outcome: EffectUndoOutcome },
}

/// How undoing one effect ended.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case", tag = "result")]
pub enum EffectUndoOutcome {
    /// The app PCTwin installed was removed through its catalog, keeping the person's own data.
    Uninstalled,
    /// What was there before the move is back (a setting's value, or not set; files).
    Restored,
    /// Nothing to put back: it is as it was before the move (the app was already there, or the
    /// change never happened or was already undone).
    AsBefore,
    /// Kept as it is now, and why in plain words (changed since the move, not safe to remove,
    /// or only its service can undo it): the person needs to look at it.
    Kept { why: String },
    /// It could not be done this time, and why; undo tries it again next time.
    NotDone { why: String },
}

impl EffectUndoOutcome {
    /// Whether undo is finished with it (only "not done this time" is tried again).
    pub fn is_final(&self) -> bool {
        !matches!(self, EffectUndoOutcome::NotDone { .. })
    }

    /// Whether the person has something to look at or do.
    pub fn needs_action(&self) -> bool {
        matches!(
            self,
            EffectUndoOutcome::Kept { .. } | EffectUndoOutcome::NotDone { .. }
        )
    }
}

/// The answer to asking Windows for a restore point before a move's first install or setting:
/// a second safety net beside this journal. Times are seconds since 1970. (Asking is the
/// administrator helper's work; this is only its honest result.)
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case", tag = "result")]
pub enum RestorePoint {
    /// A new restore point was made (its number in the system's list).
    Made { at: i64, number: u32 },
    /// Windows made no new one because one from `at` (within its limit, a day by default)
    /// already exists; that one is the safety net.
    EarlierExists { at: i64, number: u32 },
    /// None was made, and why in plain words (System Restore turned off, no administrator
    /// approval).
    NotMade { why: String },
    /// This laptop does not run Windows: restore points do not exist here.
    NotWindows,
}

mod sealed {
    pub trait Sealed {}
}

/// A role's own kind of intent, which only it can record. Sealed: no other kind exists.
pub trait RoleIntent: sealed::Sealed {
    /// The role this intent belongs to.
    const ROLE: Role;
    /// The effect as stored, and the files to keep whole with it.
    fn store(&self) -> Result<(StoredEffect, Vec<&SettingsFile>), JournalError>;
}

impl sealed::Sealed for InstallIntent {}
impl sealed::Sealed for SettingsIntent {}
impl sealed::Sealed for ImportIntent {}

impl RoleIntent for InstallIntent {
    const ROLE: Role = Role::Install;
    fn store(&self) -> Result<(StoredEffect, Vec<&SettingsFile>), JournalError> {
        Ok(match self {
            InstallIntent::App {
                catalog,
                package,
                before,
                undo,
            } => (
                StoredEffect::Install {
                    catalog: *catalog,
                    package: package.clone(),
                    before: before.clone(),
                    undo: undo.clone(),
                },
                Vec::new(),
            ),
            InstallIntent::Licence { identity } => {
                external(ExternalClass::CredentialOrLicence, identity)?
            }
        })
    }
}

impl RoleIntent for SettingsIntent {
    const ROLE: Role = Role::Settings;
    fn store(&self) -> Result<(StoredEffect, Vec<&SettingsFile>), JournalError> {
        Ok(match self {
            SettingsIntent::Apply { key, before, value } => (
                StoredEffect::ApplySetting {
                    key: key.clone(),
                    before: before.clone(),
                    value: value.clone(),
                },
                Vec::new(),
            ),
            SettingsIntent::Copy { from, to, before } => (
                StoredEffect::CopySettings {
                    from: from.clone(),
                    to: to.clone(),
                    before: kept(before)?,
                },
                before.iter().collect(),
            ),
        })
    }
}

impl RoleIntent for ImportIntent {
    const ROLE: Role = Role::Import;
    fn store(&self) -> Result<(StoredEffect, Vec<&SettingsFile>), JournalError> {
        Ok(match self {
            ImportIntent::Files { to, before } => (
                StoredEffect::Import {
                    to: to.clone(),
                    before: kept(before)?,
                },
                before.iter().collect(),
            ),
            ImportIntent::Remote { identity } => external(ExternalClass::RemoteService, identity)?,
        })
    }
}

/// An external effect, its identity checked: short, plain text.
fn external(
    class: ExternalClass,
    identity: &str,
) -> Result<(StoredEffect, Vec<&'static SettingsFile>), JournalError> {
    if identity.is_empty() || identity.len() > MAX_IDENTITY || !pctwin_recipe::text_ok(identity) {
        return Err(JournalError::BadName);
    }
    Ok((
        StoredEffect::External {
            class,
            identity: identity.to_string(),
        },
        Vec::new(),
    ))
}

/// The files from before a change as kept: checked paths, within the limits.
fn kept(before: &[SettingsFile]) -> Result<Vec<KeptFile>, JournalError> {
    distinct_paths(before.iter().map(|f| f.path.as_str()))?;
    if before.len() > MAX_KEPT_FILES
        || before.iter().any(|f| f.bytes.len() > MAX_KEPT_FILE)
        || before.iter().map(|f| f.bytes.len()).sum::<usize>() > MAX_KEPT_TOTAL
    {
        return Err(JournalError::TooLarge);
    }
    Ok(before
        .iter()
        .map(|f| KeptFile {
            path: f.path.clone(),
            hash: content_hash(&f.bytes),
            size: f.bytes.len() as u64,
        })
        .collect())
}

/// Marks the installs role (2.1).
#[derive(Debug, Clone, Copy)]
pub enum Installs {}
/// Marks the settings role (2.4).
#[derive(Debug, Clone, Copy)]
pub enum Settings {}
/// Marks the imports role (2.5).
#[derive(Debug, Clone, Copy)]
pub enum Imports {}

/// A role marker: which intent it records, and whose key makes its recorder. Sealed.
pub trait RoleMarker: sealed::Sealed {
    type Intent: RoleIntent;
    /// The role's own key ([`pctwin_grant`]): only code given it can make the recorder.
    type Key;
}
impl sealed::Sealed for Installs {}
impl sealed::Sealed for Settings {}
impl sealed::Sealed for Imports {}
impl RoleMarker for Installs {
    type Intent = InstallIntent;
    type Key = InstallKey;
}
impl RoleMarker for Settings {
    type Intent = SettingsIntent;
    type Key = SettingKey;
}
impl RoleMarker for Imports {
    type Intent = ImportIntent;
    type Key = CaptureKey;
}

/// One role's way to write effects: it records only its own kind of intent, and refuses to
/// observe, finish or undo any other role's effect ([`JournalError::WrongRole`]). Made with
/// [`Journal::installs`], [`Journal::settings`] or [`Journal::imports`], each of which needs that
/// role's own key: code that was not given the key cannot make the recorder, and there is no
/// other way to write an effect (Phase 2 adoption, change 1; security review, 26 October 2026).
///
/// # The walls, proven by the compiler
///
/// With its own key, a role writes its own effects:
///
/// ```
/// use pctwin_grant::RoleKeys;
/// use pctwin_journal::{Journal, Rebuild};
/// let keys = RoleKeys::mint().unwrap();
/// let dir = tempfile::tempdir().unwrap();
/// let j = Journal::open(&dir.path().join("j.redb")).unwrap();
/// let _installs = j.installs(&keys.install);
/// let _imports = j.imports(&keys.capture);
/// let _executor = Rebuild::new(&j, &keys.install, &keys.setting);
/// ```
///
/// Holding the journal is not enough: there is no recorder without a key (review proof P3),
///
/// ```compile_fail,E0061
/// # let dir = tempfile::tempdir().unwrap();
/// let j = pctwin_journal::Journal::open(&dir.path().join("j.redb")).unwrap();
/// let _installs = j.installs();
/// ```
///
/// another role's key does not fit,
///
/// ```compile_fail,E0308
/// let keys = pctwin_grant::RoleKeys::mint().unwrap();
/// # let dir = tempfile::tempdir().unwrap();
/// let j = pctwin_journal::Journal::open(&dir.path().join("j.redb")).unwrap();
/// let _installs = j.installs(&keys.setting);
/// ```
///
/// nor does it make the executor's pair,
///
/// ```compile_fail,E0308
/// let keys = pctwin_grant::RoleKeys::mint().unwrap();
/// # let dir = tempfile::tempdir().unwrap();
/// let j = pctwin_journal::Journal::open(&dir.path().join("j.redb")).unwrap();
/// let _r = pctwin_journal::Rebuild::new(&j, &keys.capture, &keys.setting);
/// ```
///
/// the journal itself is not a ledger an effect can be finished through,
///
/// ```compile_fail,E0277
/// use pctwin_journal::{EffectLedger, Found, Presence, RebuildRole};
/// # let dir = tempfile::tempdir().unwrap();
/// let j = pctwin_journal::Journal::open(&dir.path().join("j.redb")).unwrap();
/// EffectLedger::observed(&j, RebuildRole::Install, 1, &Found::App(Presence::Absent), true).unwrap();
/// ```
///
/// its own steps are its own,
///
/// ```compile_fail,E0624
/// use pctwin_journal::{Receipt, Role};
/// # let dir = tempfile::tempdir().unwrap();
/// let j = pctwin_journal::Journal::open(&dir.path().join("j.redb")).unwrap();
/// j.receipt(Role::Install, 1, &Receipt::Done).unwrap();
/// ```
///
/// ```compile_fail,E0624
/// use pctwin_journal::{RestorePoint, Scope};
/// # let dir = tempfile::tempdir().unwrap();
/// let j = pctwin_journal::Journal::open(&dir.path().join("j.redb")).unwrap();
/// j.record_restore_point(Scope([1; 16]), &RestorePoint::NotWindows).unwrap();
/// ```
///
/// and the executor's ledger has no way to name an import:
///
/// ```compile_fail,E0599
/// let _ = pctwin_journal::RebuildRole::Import;
/// ```
pub struct Recorder<'j, R: RoleMarker> {
    journal: &'j Journal,
    key: &'j R::Key,
    _role: PhantomData<R>,
}

impl<R: RoleMarker> std::fmt::Debug for Recorder<'_, R> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "Recorder({:?})", R::Intent::ROLE)
    }
}

impl<'j, R: RoleMarker> Recorder<'j, R> {
    /// Records a change about to be made, with what was there before it; returns its effect
    /// number (never reused). Files from before are kept whole in the same transaction. Nothing
    /// is recorded if a path is not inside its place or is listed twice, or an identity is not
    /// plain short text ([`JournalError::BadName`]), or the files are too large to keep
    /// ([`JournalError::TooLarge`]): the change must then not be made.
    pub fn intend(&self, planned: &PlannedEffect, intent: &R::Intent) -> Result<u64, JournalError> {
        self.journal.intend(R::Intent::ROLE, planned, intent)
    }

    /// What was found after effect `id`'s change (`after_restart`: by recovery after a crash).
    /// Only from Intended, only of the effect's own kind, only for this role's effect.
    pub fn observed(
        &self,
        id: u64,
        found: &Found,
        after_restart: bool,
    ) -> Result<(), JournalError> {
        self.journal
            .observed(R::Intent::ROLE, id, found, after_restart)
    }

    /// Effect `id`'s honest result, worked out from what was observed: it is then finished.
    pub fn receipt(&self, id: u64, receipt: &Receipt) -> Result<(), JournalError> {
        self.journal.receipt(R::Intent::ROLE, id, receipt)
    }

    /// Records a step of undoing this role's effect `id` (one with a receipt), with leave from
    /// the journal's undo gate.
    pub fn record_undo(
        &self,
        right: &impl UndoRight,
        id: u64,
        undo: &EffectUndo,
    ) -> Result<(), JournalError> {
        self.journal
            .record_effect_undo(R::Intent::ROLE, right, id, undo)
    }

    /// The journal, for reading (reading is open to every part of PCTwin; writing is not).
    pub fn journal(&self) -> &'j Journal {
        self.journal
    }
}

impl Recorder<'_, Installs> {
    /// A permission for the installs role's adapter to install one exact package of the approved
    /// plan (see [`pctwin_grant::InstallKey::grant`]): only this recorder's key makes one.
    pub fn grant(
        &self,
        seal: PlanSeal,
        artifact: ObjectRef,
        lifetime: std::time::Duration,
        now: Now,
    ) -> Result<InstallGrant, GrantError> {
        self.key.grant(seal, artifact, lifetime, now)
    }

    /// Records the restore point answer for move `scope` (asked before its first install or
    /// setting): once, never written over.
    pub fn record_restore_point(
        &self,
        scope: Scope,
        point: &RestorePoint,
    ) -> Result<(), JournalError> {
        self.journal.record_restore_point(scope, point)
    }
}

impl Recorder<'_, Settings> {
    /// A permission for the settings role's adapter to change one named setting or settings
    /// place of the approved plan (see [`pctwin_grant::SettingKey::grant`]).
    pub fn grant(
        &self,
        seal: PlanSeal,
        setting: ObjectRef,
        lifetime: std::time::Duration,
        now: Now,
    ) -> Result<SettingGrant, GrantError> {
        self.key.grant(seal, setting, lifetime, now)
    }

    /// Records the restore point answer for move `scope`: once, never written over.
    pub fn record_restore_point(
        &self,
        scope: Scope,
        point: &RestorePoint,
    ) -> Result<(), JournalError> {
        self.journal.record_restore_point(scope, point)
    }
}

/// The installs and settings roles' recorders together: what the program's starting code hands
/// the executor (which runs the recipes of 2.1 and 2.4), and nothing more. It cannot reach the
/// imports role's effects: that needs the imports role's own key.
#[derive(Debug)]
pub struct Rebuild<'j> {
    pub installs: Recorder<'j, Installs>,
    pub settings: Recorder<'j, Settings>,
}

impl<'j> Rebuild<'j> {
    /// Both recorders of `journal`, made from the two roles' own keys.
    pub fn new(journal: &'j Journal, install: &'j InstallKey, setting: &'j SettingKey) -> Self {
        Self {
            installs: journal.installs(install),
            settings: journal.settings(setting),
        }
    }

    /// The journal, for reading.
    pub fn journal(&self) -> &'j Journal {
        self.installs.journal
    }
}

/// Which of the two rebuilding roles an effect belongs to: the only roles the executor can
/// write for.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RebuildRole {
    Install,
    Settings,
}

impl RebuildRole {
    /// The rebuilding role of an effect, or `None` for an import (the imports role's own).
    pub fn of(role: Role) -> Option<Self> {
        match role {
            Role::Install => Some(RebuildRole::Install),
            Role::Settings => Some(RebuildRole::Settings),
            Role::Import => None,
        }
    }
}

/// The steps an effect is recorded through, as the executor (which runs the installs and
/// settings roles' recipes) uses them. [`Rebuild`] is the real one, and only it can be made from
/// a journal (with the two roles' keys); a test can wrap it to make the record fail at any step.
/// An effect is observed or finished only through one of the two recorders ([`RebuildRole`] has
/// no imports member), so the imports role's effects are out of reach.
pub trait EffectLedger {
    /// See [`Recorder::intend`] (installs).
    fn intend_install(
        &self,
        planned: &PlannedEffect,
        intent: &InstallIntent,
    ) -> Result<u64, JournalError>;
    /// See [`Recorder::intend`] (settings).
    fn intend_setting(
        &self,
        planned: &PlannedEffect,
        intent: &SettingsIntent,
    ) -> Result<u64, JournalError>;
    /// See [`Recorder::observed`], through the recorder of `role`.
    fn observed(
        &self,
        role: RebuildRole,
        id: u64,
        found: &Found,
        after_restart: bool,
    ) -> Result<(), JournalError>;
    /// See [`Recorder::receipt`], through the recorder of `role`.
    fn receipt(&self, role: RebuildRole, id: u64, receipt: &Receipt) -> Result<(), JournalError>;
    /// See [`Journal::unfinished_effects`].
    fn unfinished_effects(&self) -> Result<Vec<EffectEntry>, JournalError>;
    /// See [`Journal::restore_point`].
    fn restore_point(&self, scope: Scope) -> Result<Option<RestorePoint>, JournalError>;
    /// Records the move's restore point answer, through the installs recorder.
    fn record_restore_point(&self, scope: Scope, point: &RestorePoint) -> Result<(), JournalError>;
    /// A permission for one install step, from the installs recorder's key.
    fn install_grant(
        &self,
        seal: PlanSeal,
        artifact: ObjectRef,
    ) -> Result<InstallGrant, GrantError>;
    /// A permission for one settings step, from the settings recorder's key.
    fn setting_grant(&self, seal: PlanSeal, setting: ObjectRef)
    -> Result<SettingGrant, GrantError>;
}

/// How long a permission handed to an adapter for one step lasts: the longest allowed, since an
/// installer may take a while; the adapter checks it before it starts.
pub const STEP_GRANT: std::time::Duration = pctwin_grant::MAX_LIFETIME;

impl EffectLedger for Rebuild<'_> {
    fn intend_install(
        &self,
        planned: &PlannedEffect,
        intent: &InstallIntent,
    ) -> Result<u64, JournalError> {
        self.installs.intend(planned, intent)
    }
    fn intend_setting(
        &self,
        planned: &PlannedEffect,
        intent: &SettingsIntent,
    ) -> Result<u64, JournalError> {
        self.settings.intend(planned, intent)
    }
    fn observed(
        &self,
        role: RebuildRole,
        id: u64,
        found: &Found,
        after_restart: bool,
    ) -> Result<(), JournalError> {
        match role {
            RebuildRole::Install => self.installs.observed(id, found, after_restart),
            RebuildRole::Settings => self.settings.observed(id, found, after_restart),
        }
    }
    fn receipt(&self, role: RebuildRole, id: u64, receipt: &Receipt) -> Result<(), JournalError> {
        match role {
            RebuildRole::Install => self.installs.receipt(id, receipt),
            RebuildRole::Settings => self.settings.receipt(id, receipt),
        }
    }
    fn unfinished_effects(&self) -> Result<Vec<EffectEntry>, JournalError> {
        self.journal().unfinished_effects()
    }
    fn restore_point(&self, scope: Scope) -> Result<Option<RestorePoint>, JournalError> {
        self.journal().restore_point(scope)
    }
    fn record_restore_point(&self, scope: Scope, point: &RestorePoint) -> Result<(), JournalError> {
        self.installs.record_restore_point(scope, point)
    }
    fn install_grant(
        &self,
        seal: PlanSeal,
        artifact: ObjectRef,
    ) -> Result<InstallGrant, GrantError> {
        self.installs.grant(seal, artifact, STEP_GRANT, Now::real())
    }
    fn setting_grant(
        &self,
        seal: PlanSeal,
        setting: ObjectRef,
    ) -> Result<SettingGrant, GrantError> {
        self.settings.grant(seal, setting, STEP_GRANT, Now::real())
    }
}

/// An error unless `path` is a file's path inside its place: empty (the place itself), or plain
/// names joined by single `/`.
fn settings_path(path: &str) -> Result<(), JournalError> {
    if path.is_empty() {
        return Ok(());
    }
    for part in path.split('/') {
        crate::plain_name(part).map_err(|_| JournalError::BadName)?;
    }
    Ok(())
}

/// An error unless no path is listed twice and every path is a file path inside its place.
fn distinct_paths<'a>(paths: impl Iterator<Item = &'a str>) -> Result<(), JournalError> {
    let mut seen = std::collections::BTreeSet::new();
    for p in paths {
        settings_path(p)?;
        if !seen.insert(p) {
            return Err(JournalError::BadName);
        }
    }
    Ok(())
}

/// The names records are sealed under, so one kind can never pass for another.
const EFFECT_SEAL: &str = "effects";
const UNDO_SEAL: &str = "effect-undo";

/// A record as stored: its keyed tag ([`TagKey`]), then its JSON.
fn seal(key: &TagKey, kind: &str, id: u64, json: &[u8]) -> Vec<u8> {
    let mut out = Vec::with_capacity(32 + json.len());
    out.extend_from_slice(&key.tag(kind, id, json));
    out.extend_from_slice(json);
    out
}

/// A stored record's JSON, only if its tag is right: one changed by hand, or sealed under
/// another number or kind, is damaged.
fn unseal<'b>(
    key: &TagKey,
    kind: &str,
    id: u64,
    bytes: &'b [u8],
) -> Result<&'b [u8], JournalError> {
    let (tag, json) = bytes
        .split_first_chunk::<32>()
        .ok_or(damaged("a record is too short to be one of PCTwin's"))?;
    if key.verify(kind, id, json, tag) {
        Ok(json)
    } else {
        Err(damaged("a record was changed outside PCTwin"))
    }
}

/// Reads one effect. A record whose tag is wrong, that does not read, whose names are not ones a
/// recipe could hold, or whose paths are not plain paths inside their place (a record changed
/// by hand), is damaged: never a source of paths, packages or undo rules.
fn read_effect(
    table: &impl ReadableTable<u64, &'static [u8]>,
    id: u64,
    key: &TagKey,
) -> Result<EffectEntry, JournalError> {
    let found = table
        .get(id)
        .map_err(storage)?
        .ok_or(JournalError::NoSuchEntry(id))?;
    parse_effect(id, found.value(), key)
}

fn parse_effect(id: u64, bytes: &[u8], key: &TagKey) -> Result<EffectEntry, JournalError> {
    let entry: EffectEntry =
        serde_json::from_slice(unseal(key, EFFECT_SEAL, id, bytes)?).map_err(unreadable)?;
    if entry.id != id {
        return Err(damaged("an effect record is filed under another number"));
    }
    // The names undo would hand to the system: only ones a checked recipe could hold.
    let names_ok = match &entry.effect {
        StoredEffect::Install { package, .. } => pctwin_recipe::package_ok(package),
        StoredEffect::ApplySetting { key, .. } => pctwin_recipe::key_ok(key),
        StoredEffect::External { identity, .. } => {
            !identity.is_empty()
                && identity.len() <= MAX_IDENTITY
                && pctwin_recipe::text_ok(identity)
        }
        StoredEffect::CopySettings { .. } | StoredEffect::Import { .. } => true,
    };
    if !names_ok {
        return Err(damaged("an effect record names something no recipe could"));
    }
    let bad = |_| damaged("an effect record names a place outside its folder");
    match &entry.effect {
        StoredEffect::CopySettings { before, .. } | StoredEffect::Import { before, .. } => {
            distinct_paths(before.iter().map(|f| f.path.as_str())).map_err(bad)?;
        }
        _ => {}
    }
    if let EffectState::Observed {
        found: Found::Files(files),
        ..
    }
    | EffectState::Receipted {
        found: Found::Files(files),
        ..
    } = &entry.state
    {
        distinct_paths(files.iter().map(|f| f.path.as_str())).map_err(bad)?;
    }
    let role_fits = matches!(
        (entry.role, &entry.effect),
        (
            Role::Install,
            StoredEffect::Install { .. }
                | StoredEffect::External {
                    class: ExternalClass::CredentialOrLicence,
                    ..
                }
        ) | (
            Role::Settings,
            StoredEffect::ApplySetting { .. } | StoredEffect::CopySettings { .. }
        ) | (
            Role::Import,
            StoredEffect::Import { .. }
                | StoredEffect::External {
                    class: ExternalClass::RemoteService,
                    ..
                }
        )
    );
    if !role_fits {
        return Err(damaged(
            "an effect record belongs to a role that cannot make it",
        ));
    }
    Ok(entry)
}

fn write_effect(
    table: &mut redb::Table<'_, u64, &'static [u8]>,
    entry: &EffectEntry,
    key: &TagKey,
) -> Result<(), JournalError> {
    let json = serde_json::to_vec(entry).map_err(storage)?;
    let bytes = seal(key, EFFECT_SEAL, entry.id, &json);
    table.insert(entry.id, bytes.as_slice()).map_err(storage)?;
    Ok(())
}

/// Reads one record of undoing effect `id`, only if its tag is right and any name in it is a
/// path inside its place and one of undo's private names.
fn read_undo(id: u64, bytes: &[u8], key: &TagKey) -> Result<EffectUndo, JournalError> {
    let undo: EffectUndo =
        serde_json::from_slice(unseal(key, UNDO_SEAL, id, bytes)?).map_err(unreadable)?;
    if let EffectUndo::Swapping { path, private } = &undo
        && (settings_path(path).is_err() || !is_private_name(private))
    {
        return Err(damaged("an undo record names a place outside its folder"));
    }
    Ok(undo)
}

/// Whether `name` is one of undo's private names: `.pctwin-undo-` and 32 lowercase hexadecimal
/// digits.
pub fn is_private_name(name: &str) -> bool {
    name.strip_prefix(".pctwin-undo-").is_some_and(|t| {
        t.len() == 32
            && t.bytes()
                .all(|b| b.is_ascii_digit() || (b'a'..=b'f').contains(&b))
    })
}

fn write_undo(
    table: &mut redb::Table<'_, u64, &'static [u8]>,
    id: u64,
    undo: &EffectUndo,
    key: &TagKey,
) -> Result<(), JournalError> {
    let json = serde_json::to_vec(undo).map_err(storage)?;
    table
        .insert(id, seal(key, UNDO_SEAL, id, &json).as_slice())
        .map_err(storage)?;
    Ok(())
}

/// The effects in one read, each checked on its own: those whose record is right, and the
/// numbers of those that are damaged (which never drive anything).
#[derive(Debug, Clone, PartialEq, Default)]
pub struct CheckedEffects {
    pub effects: Vec<EffectEntry>,
    pub damaged: Vec<u64>,
}

fn scope_key(scope: Scope) -> String {
    scope.0.iter().map(|b| format!("{b:02x}")).collect()
}

/// Whether `next` may be recorded after `before` (none: not started).
fn may_follow(before: Option<&EffectUndo>, next: &EffectUndo) -> bool {
    match before {
        None => true,
        Some(EffectUndo::Done { outcome }) => !outcome.is_final(),
        Some(EffectUndo::Reversing | EffectUndo::Swapping { .. }) => {
            !matches!(next, EffectUndo::Reversing)
        }
    }
}

/// The effects whose undo is part way (Reversing) in this table, in order. A record that cannot
/// be checked (its key not found, or its tag wrong) counts as part way: closing undo waits.
pub(crate) fn part_way_in(
    table: &impl ReadableTable<u64, &'static [u8]>,
    key: Option<&TagKey>,
) -> Result<Vec<u64>, JournalError> {
    let mut out = Vec::new();
    for row in table.iter().map_err(storage)? {
        let (k, v) = row.map_err(storage)?;
        let id = k.value();
        let stage = key.map(|key| read_undo(id, v.value(), key));
        if !matches!(stage, Some(Ok(EffectUndo::Done { .. }))) {
            out.push(id);
        }
    }
    Ok(out)
}

/// An error unless `entry` belongs to `role`.
fn own(role: Role, entry: &EffectEntry) -> Result<(), JournalError> {
    if entry.role == role {
        Ok(())
    } else {
        Err(JournalError::WrongRole {
            effect: entry.id,
            belongs_to: entry.role,
        })
    }
}

impl Journal {
    /// The installs role's recorder (2.1), made only with its own key.
    pub fn installs<'j>(&'j self, key: &'j InstallKey) -> Recorder<'j, Installs> {
        Recorder {
            journal: self,
            key,
            _role: PhantomData,
        }
    }

    /// The settings role's recorder (2.4), made only with its own key.
    pub fn settings<'j>(&'j self, key: &'j SettingKey) -> Recorder<'j, Settings> {
        Recorder {
            journal: self,
            key,
            _role: PhantomData,
        }
    }

    /// The imports role's recorder (2.5), made only with its own key.
    pub fn imports<'j>(&'j self, key: &'j CaptureKey) -> Recorder<'j, Imports> {
        Recorder {
            journal: self,
            key,
            _role: PhantomData,
        }
    }

    /// The key that seals this journal's effect records: given when it was opened, or kept by
    /// the system ([`crate::keystore`]), found once and held.
    pub(crate) fn record_key(&self) -> Result<TagKey, JournalError> {
        let mut held = self
            .record_key
            .held
            .lock()
            .map_err(|_| damaged("the record key's lock was broken"))?;
        if let Some(key) = held.as_ref() {
            return Ok(key.clone());
        }
        let key = match &self.record_key.given {
            Some(key) => key.clone(),
            None => {
                let may_make = self.holds_no_effect()?;
                crate::keystore::key_for(self.record_key.store.as_ref(), self.journal_id, may_make)
                    .map_err(|e| match e {
                        KeyError::Missing => JournalError::RecordKeyMissing,
                        KeyError::Unavailable => JournalError::RecordKeyUnavailable,
                    })?
            }
        };
        *held = Some(key.clone());
        Ok(key)
    }

    /// Whether no effect, and no undo of one, was ever recorded here (only then may a new key be
    /// made).
    fn holds_no_effect(&self) -> Result<bool, JournalError> {
        let tx = self.db.begin_read().map_err(storage)?;
        let effects = tx.open_table(EFFECTS).map_err(storage)?;
        let undo = tx.open_table(EFFECT_UNDO).map_err(storage)?;
        let broker = tx.open_table(crate::broker::BROKER).map_err(storage)?;
        Ok(effects.is_empty().map_err(storage)?
            && undo.is_empty().map_err(storage)?
            && broker.is_empty().map_err(storage)?)
    }

    fn intend<I: RoleIntent>(
        &self,
        role: Role,
        planned: &PlannedEffect,
        intent: &I,
    ) -> Result<u64, JournalError> {
        let (effect, keep) = intent.store()?;
        let key = self.record_key()?;
        let tx = self.db.begin_write().map_err(storage)?;
        let id = {
            let mut counters = tx.open_table(crate::COUNTERS).map_err(storage)?;
            let id = counters
                .get(NEXT_EFFECT)
                .map_err(storage)?
                .map_or(1, |v| v.value());
            counters.insert(NEXT_EFFECT, id + 1).map_err(storage)?;
            let mut files = tx.open_table(KEPT_FILES).map_err(storage)?;
            for f in keep {
                files
                    .insert(content_hash(&f.bytes), f.bytes.as_slice())
                    .map_err(storage)?;
            }
            let entry = EffectEntry {
                id,
                role,
                planned: planned.clone(),
                effect,
                state: EffectState::Intended,
            };
            let mut effects = tx.open_table(EFFECTS).map_err(storage)?;
            write_effect(&mut effects, &entry, &key)?;
            let mut unfinished = tx.open_table(EFFECTS_UNFINISHED).map_err(storage)?;
            unfinished.insert(id, ()).map_err(storage)?;
            id
        };
        tx.commit().map_err(storage)?;
        Ok(id)
    }

    fn observed(
        &self,
        role: Role,
        id: u64,
        found: &Found,
        after_restart: bool,
    ) -> Result<(), JournalError> {
        if let Found::Files(files) = found {
            distinct_paths(files.iter().map(|f| f.path.as_str()))?;
        }
        self.effect_step(role, id, |entry| {
            if entry.effect.kind() != found.kind() {
                return Err(JournalError::OutOfOrder {
                    from: entry.effect.kind(),
                    to: found.kind(),
                });
            }
            match entry.state {
                EffectState::Intended => Ok(EffectState::Observed {
                    found: found.clone(),
                    after_restart,
                }),
                _ => Err(JournalError::OutOfOrder {
                    from: entry.state.name(),
                    to: "observed",
                }),
            }
        })
    }

    fn receipt(&self, role: Role, id: u64, receipt: &Receipt) -> Result<(), JournalError> {
        self.effect_step(role, id, |entry| match &entry.state {
            EffectState::Observed {
                found,
                after_restart,
            } => Ok(EffectState::Receipted {
                found: found.clone(),
                after_restart: *after_restart,
                receipt: *receipt,
            }),
            other => Err(JournalError::OutOfOrder {
                from: other.name(),
                to: "receipted",
            }),
        })
    }

    /// Takes one step of `role`'s effect as its own committed transaction.
    fn effect_step(
        &self,
        role: Role,
        id: u64,
        next: impl FnOnce(&EffectEntry) -> Result<EffectState, JournalError>,
    ) -> Result<(), JournalError> {
        let key = self.record_key()?;
        let tx = self.db.begin_write().map_err(storage)?;
        {
            let mut effects = tx.open_table(EFFECTS).map_err(storage)?;
            let mut entry = read_effect(&effects, id, &key)?;
            own(role, &entry)?;
            entry.state = next(&entry)?;
            write_effect(&mut effects, &entry, &key)?;
            if matches!(entry.state, EffectState::Receipted { .. }) {
                let mut unfinished = tx.open_table(EFFECTS_UNFINISHED).map_err(storage)?;
                unfinished.remove(id).map_err(storage)?;
            }
        }
        tx.commit().map_err(storage)
    }

    /// One effect, if there is one with this number.
    pub fn effect(&self, id: u64) -> Result<Option<EffectEntry>, JournalError> {
        let tx = self.db.begin_read().map_err(storage)?;
        let effects = tx.open_table(EFFECTS).map_err(storage)?;
        if effects.get(id).map_err(storage)?.is_none() {
            return Ok(None);
        }
        let key = self.record_key()?;
        match read_effect(&effects, id, &key) {
            Ok(e) => Ok(Some(e)),
            Err(JournalError::NoSuchEntry(_)) => Ok(None),
            Err(e) => Err(e),
        }
    }

    /// Every effect, in the order intended.
    pub fn effects(&self) -> Result<Vec<EffectEntry>, JournalError> {
        let tx = self.db.begin_read().map_err(storage)?;
        let effects = tx.open_table(EFFECTS).map_err(storage)?;
        if effects.is_empty().map_err(storage)? {
            return Ok(Vec::new());
        }
        let key = self.record_key()?;
        let mut out = Vec::new();
        for row in effects.iter().map_err(storage)? {
            let (k, v) = row.map_err(storage)?;
            out.push(parse_effect(k.value(), v.value(), &key)?);
        }
        Ok(out)
    }

    /// Every effect of move `scope` whose record is right, in the order intended, and the
    /// numbers of every damaged record (of any move: a damaged record's move cannot be trusted).
    /// Undo works from this, so one damaged record never stops the rest, and never drives
    /// anything. Fails only when the records cannot be read at all, or their key is not found
    /// ([`JournalError::RecordKeyMissing`]).
    pub fn effects_checked(&self, scope: Scope) -> Result<CheckedEffects, JournalError> {
        let tx = self.db.begin_read().map_err(storage)?;
        let effects = tx.open_table(EFFECTS).map_err(storage)?;
        let mut out = CheckedEffects::default();
        if effects.is_empty().map_err(storage)? {
            return Ok(out);
        }
        let key = self.record_key()?;
        for row in effects.iter().map_err(storage)? {
            let (k, v) = row.map_err(storage)?;
            match parse_effect(k.value(), v.value(), &key) {
                Ok(e) if e.planned.scope == scope => out.effects.push(e),
                Ok(_) => {}
                Err(JournalError::Damaged(_)) => out.damaged.push(k.value()),
                Err(e) => return Err(e),
            }
        }
        Ok(out)
    }

    /// Every effect of one move, in the order intended.
    pub fn effects_of(&self, scope: Scope) -> Result<Vec<EffectEntry>, JournalError> {
        Ok(self
            .effects()?
            .into_iter()
            .filter(|e| e.planned.scope == scope)
            .collect())
    }

    /// Effects with no receipt yet, in the order intended: what recovery looks at.
    pub fn unfinished_effects(&self) -> Result<Vec<EffectEntry>, JournalError> {
        let tx = self.db.begin_read().map_err(storage)?;
        let unfinished = tx.open_table(EFFECTS_UNFINISHED).map_err(storage)?;
        let effects = tx.open_table(EFFECTS).map_err(storage)?;
        let mut out = Vec::new();
        if unfinished.is_empty().map_err(storage)? {
            return Ok(out);
        }
        let key = self.record_key()?;
        for row in unfinished.iter().map_err(storage)? {
            let (k, _) = row.map_err(storage)?;
            out.push(read_effect(&effects, k.value(), &key)?);
        }
        Ok(out)
    }

    /// The bytes of a file kept from before a change, by its content hash: checked against that
    /// hash, so a damaged or missing copy is reported, never handed back.
    pub fn kept_bytes(&self, hash: &[u8; 32]) -> Result<Vec<u8>, JournalError> {
        let tx = self.db.begin_read().map_err(storage)?;
        let files = tx.open_table(KEPT_FILES).map_err(storage)?;
        let Some(v) = files.get(*hash).map_err(storage)? else {
            return Err(damaged("a file kept from before the move is missing"));
        };
        let bytes = v.value().to_vec();
        if content_hash(&bytes) != *hash {
            return Err(damaged("a file kept from before the move is damaged"));
        }
        Ok(bytes)
    }

    fn record_effect_undo(
        &self,
        role: Role,
        right: &impl UndoRight,
        id: u64,
        undo: &EffectUndo,
    ) -> Result<(), JournalError> {
        self.check_right(right)?;
        if let EffectUndo::Swapping { path, private } = undo
            && (settings_path(path).is_err() || !is_private_name(private))
        {
            return Err(JournalError::BadName);
        }
        let key = self.record_key()?;
        let tx = self.db.begin_write().map_err(storage)?;
        {
            let gate = tx.open_table(GATE).map_err(storage)?;
            if gate.get(UNDO_GATE).map_err(storage)?.is_some() {
                return Err(JournalError::UndoClosed);
            }
            let effects = tx.open_table(EFFECTS).map_err(storage)?;
            let entry = read_effect(&effects, id, &key)?;
            own(role, &entry)?;
            if !matches!(entry.state, EffectState::Receipted { .. }) {
                return Err(JournalError::OutOfOrder {
                    from: entry.state.name(),
                    to: "undo",
                });
            }
            let mut table = tx.open_table(EFFECT_UNDO).map_err(storage)?;
            let before: Option<EffectUndo> = table
                .get(id)
                .map_err(storage)?
                .map(|v| read_undo(id, v.value(), &key))
                .transpose()?;
            if !may_follow(before.as_ref(), undo) {
                return Err(JournalError::OutOfOrder {
                    from: match before {
                        Some(EffectUndo::Reversing) => "reversing",
                        Some(EffectUndo::Swapping { .. }) => "swapping",
                        _ => "done",
                    },
                    to: match undo {
                        EffectUndo::Reversing => "reversing",
                        EffectUndo::Swapping { .. } => "swapping",
                        EffectUndo::Done { .. } => "done",
                    },
                });
            }
            write_undo(&mut table, id, undo, &key)?;
        }
        tx.commit().map_err(storage)
    }

    /// How far undoing effect `id` got, if it was started.
    pub fn effect_undo_of(&self, id: u64) -> Result<Option<EffectUndo>, JournalError> {
        let tx = self.db.begin_read().map_err(storage)?;
        let table = tx.open_table(EFFECT_UNDO).map_err(storage)?;
        let Some(v) = table.get(id).map_err(storage)? else {
            return Ok(None);
        };
        read_undo(id, v.value(), &self.record_key()?).map(Some)
    }

    /// Effects whose undo is part way, in order (closing undo refuses while there are any). One
    /// whose record cannot be checked counts as part way.
    pub fn effects_part_way(&self) -> Result<Vec<u64>, JournalError> {
        let tx = self.db.begin_read().map_err(storage)?;
        let table = tx.open_table(EFFECT_UNDO).map_err(storage)?;
        let key = self.key_if_any(&table);
        part_way_in(&table, key.as_ref())
    }

    /// The record key, if `table` holds any record to check with it (and it can be had).
    pub(crate) fn key_if_any(
        &self,
        table: &impl ReadableTable<u64, &'static [u8]>,
    ) -> Option<TagKey> {
        match table.is_empty() {
            Ok(false) => self.record_key().ok(),
            _ => None,
        }
    }

    /// The restore point answer recorded for move `scope`, if it was asked for.
    pub fn restore_point(&self, scope: Scope) -> Result<Option<RestorePoint>, JournalError> {
        let tx = self.db.begin_read().map_err(storage)?;
        let table = tx.open_table(RESTORE_POINTS).map_err(storage)?;
        table
            .get(scope_key(scope).as_str())
            .map_err(storage)?
            .map(|v| serde_json::from_slice(v.value()).map_err(unreadable))
            .transpose()
    }

    /// Records the restore point answer for move `scope`: once, never written over. Only through
    /// the installs or settings recorder.
    fn record_restore_point(&self, scope: Scope, point: &RestorePoint) -> Result<(), JournalError> {
        let tx = self.db.begin_write().map_err(storage)?;
        {
            let mut table = tx.open_table(RESTORE_POINTS).map_err(storage)?;
            let key = scope_key(scope);
            if table.get(key.as_str()).map_err(storage)?.is_some() {
                return Err(JournalError::OutOfOrder {
                    from: "restore-point",
                    to: "restore-point",
                });
            }
            let bytes = serde_json::to_vec(point).map_err(storage)?;
            table
                .insert(key.as_str(), bytes.as_slice())
                .map_err(storage)?;
        }
        tx.commit().map_err(storage)
    }
}
