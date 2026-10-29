//! What the Phase 2 screens are shown (Task List 2.8 and 2.9): a cut-down, typed view of each
//! part's state, made from the engine's own types. Every value is a word key the screens
//! translate, a number, or a name to show exactly as it is (a file, app, drive, person or laptop
//! name: the screens never read it as markup or a link). No path the screens could act on, no
//! command, no secret.
//!
//! The parts it shows: undo (2.3: effect records, the undo preview and its outcomes), the
//! permission broker (2.2: one system prompt per move, the restore point's honest result, a
//! permission ended by sleep or another person's session), the more move types (2.6: "safe in
//! <provider>", keeping a local copy, the per-drive budget and its proposal, the offload drive,
//! the owner table across laptops, re-approving only what changed, the dead-laptop rescue and the
//! partial report with Try again) and the "before we change anything" preview (2.9) with the undo
//! explanation for every change.
//!
//! Lists are bounded ([`MAX_LINES`]): what is left out is counted in `more`, never dropped
//! silently.

use serde::Serialize;

use pctwin_helper::broker::restore::{NotMadeReason, RestorePointResult};
use pctwin_journal::{EffectClass, EffectUndoOutcome, UndoOutcome};
use pctwin_orchestrate::capacity::{Choice, NeedKind, OffloadDrive, VolumeBudget};
use pctwin_orchestrate::jobs::{Missing, Scope, Why};
use pctwin_orchestrate::local_copy::Stopped;
use pctwin_orchestrate::rescue::{Lock, ReadOnlyLevel, RescueError};
use pctwin_record::CloudProvider;

/// The most lines one list carries to the screens; the rest are counted.
pub const MAX_LINES: usize = 200;

/// The longest name sent, in characters; a longer one is cut with an ellipsis (the full name is
/// in the report on disk).
pub const MAX_NAME: usize = 260;

/// A name to show exactly as it is, bounded.
pub fn name(text: &str) -> String {
    if text.chars().count() <= MAX_NAME {
        return text.to_owned();
    }
    let mut cut: String = text.chars().take(MAX_NAME - 1).collect();
    cut.push('\u{2026}');
    cut
}

/// The first [`MAX_LINES`] of `lines`, and how many more there were.
pub fn bounded<T>(mut lines: Vec<T>) -> (Vec<T>, u64) {
    let more = lines.len().saturating_sub(MAX_LINES);
    lines.truncate(MAX_LINES);
    (lines, more as u64)
}

pctwin_types::listed_enum! {
    /// What a change does to the new laptop (the 2.9 preview).
    #[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
    #[serde(rename_all = "kebab-case")]
    pub enum ChangeAction {
        /// An app is installed.
        Install,
        /// Things are added next to what is already there (nothing already there is replaced).
        Merge,
        /// Something already there is replaced; its earlier version is kept for undo.
        Replace,
        /// Already on the new laptop, or not moved: left exactly as it is.
        LeaveAlone,
    }
}

pctwin_types::listed_enum! {
    /// What undo can do for one change, said before approval (2.9, "an undo explanation for
    /// every change"): what PCTwin can reverse, what it must keep, and what needs the person.
    #[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
    #[serde(rename_all = "kebab-case")]
    pub enum UndoPromise {
        /// PCTwin can put it back as it was, while nobody has changed it since.
        CanReverse,
        /// PCTwin can remove the app again; what the person makes with it is kept.
        CanRemoveApp,
        /// PCTwin keeps it: it was there before the move, so undo leaves it alone.
        MustKeep,
        /// Done with an online service: only that service can undo it.
        NeedsYouService,
        /// A sign-in, licence or activation: the app's maker governs it.
        NeedsYouMaker,
        /// An app PCTwin installed that PCTwin does not remove: the person removes it in the
        /// system's settings if they do not want it.
        RemoveYourself,
        /// An app PCTwin installs on Windows: undo removes it if it came as a Windows Installer
        /// package and is still that version; otherwise the person removes it.
        RemovesIfPackage,
        /// An older version here that PCTwin updates: undo cannot bring the older one back.
        KeepsNewer,
    }
}

impl UndoPromise {
    /// What undo can promise for an effect of `class` doing `action`.
    pub fn of(class: EffectClass, action: ChangeAction) -> Self {
        if action == ChangeAction::LeaveAlone {
            return UndoPromise::MustKeep;
        }
        match class {
            EffectClass::NativePackageInstall => UndoPromise::CanRemoveApp,
            EffectClass::OwnedStagedFile
            | EffectClass::ExistingTargetReplacement
            | EffectClass::SettingsChange
            | EffectClass::PrivateDataImport => UndoPromise::CanReverse,
            EffectClass::RemoteServiceEffect => UndoPromise::NeedsYouService,
            EffectClass::CredentialOrLicenceChange => UndoPromise::NeedsYouMaker,
            // Never part of a move's changes; if it ever were, undo could not reverse it.
            EffectClass::DestructiveSanitisation => UndoPromise::MustKeep,
        }
    }
}

/// One change in the 2.9 preview.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ChangeLine {
    pub name: String,
    pub action: ChangeAction,
    pub undo: UndoPromise,
}

pctwin_types::listed_enum! {
    /// What takes room on a drive.
    #[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
    #[serde(rename_all = "kebab-case")]
    pub enum SpaceUse {
        Files,
        AppDownloads,
        Unpacking,
        UndoRecord,
        PartlyCopied,
    }
}

impl From<NeedKind> for SpaceUse {
    fn from(kind: NeedKind) -> Self {
        match kind {
            NeedKind::Files => SpaceUse::Files,
            NeedKind::AppDownloads => SpaceUse::AppDownloads,
            NeedKind::Unpacking => SpaceUse::Unpacking,
            NeedKind::Journal => SpaceUse::UndoRecord,
            NeedKind::PartlyCopied => SpaceUse::PartlyCopied,
        }
    }
}

/// Bytes of one use.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct SpacePart {
    pub used_for: SpaceUse,
    pub bytes: u64,
}

/// One drive's budget at the move's busiest moment (M31).
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct DriveBudget {
    pub drive: String,
    pub needed: u64,
    /// `None`: the drive does not say.
    pub free: Option<u64>,
    /// `None`: not known, because the free space is not known.
    pub fits: Option<bool>,
    pub short_by: u64,
    pub parts: Vec<SpacePart>,
}

impl From<&VolumeBudget> for DriveBudget {
    fn from(b: &VolumeBudget) -> Self {
        DriveBudget {
            drive: name(&b.volume),
            needed: b.total,
            free: b.free,
            fits: b.fits,
            short_by: b.short_by,
            parts: b
                .by_kind
                .iter()
                .map(|(k, bytes)| SpacePart {
                    used_for: (*k).into(),
                    bytes: *bytes,
                })
                .collect(),
        }
    }
}

pctwin_types::listed_enum! {
    /// Where something kept off a too-small new laptop goes (the proposal).
    #[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
    #[serde(rename_all = "kebab-case")]
    pub enum KeepOff {
        LeaveInCloud,
        OffloadDrive,
        LeaveOnOldLaptop,
    }
}

impl From<Choice> for KeepOff {
    fn from(c: Choice) -> Self {
        match c {
            Choice::LeaveInCloud => KeepOff::LeaveInCloud,
            Choice::Offload => KeepOff::OffloadDrive,
            Choice::LeaveOnOldLaptop => KeepOff::LeaveOnOldLaptop,
        }
    }
}

/// One line of the proposal: how many items, and their size, go where.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ProposalLine {
    pub to: KeepOff,
    pub items: u64,
    pub bytes: u64,
}

/// The offload drive, approved on its own.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct OffloadView {
    pub drive: String,
    pub free: Option<u64>,
    pub approved: bool,
    pub connected: bool,
}

impl From<&OffloadDrive> for OffloadView {
    fn from(d: &OffloadDrive) -> Self {
        OffloadView {
            drive: name(&d.label),
            free: d.free,
            approved: d.approved,
            connected: d.connected,
        }
    }
}

pctwin_types::listed_enum! {
    /// Where keeping a local copy of online-only files is.
    #[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
    #[serde(rename_all = "kebab-case")]
    pub enum LocalCopyState {
        /// The person has not chosen yet.
        Choosing,
        /// Copies are being brought down and sent, in batches.
        Working,
        /// Paused: the connection may charge for data, and the person has not allowed that.
        PausedMetered,
        /// Paused: the old laptop has no room for the next batch.
        PausedNoSpace,
        /// Done.
        Done,
    }
}

impl From<Stopped> for LocalCopyState {
    fn from(s: Stopped) -> Self {
        match s {
            Stopped::MeteredNow => LocalCopyState::PausedMetered,
            Stopped::NoSpace => LocalCopyState::PausedNoSpace,
        }
    }
}

/// A cloud provider by its own name; `None` for one PCTwin does not know (the screens then say
/// "your cloud").
pub fn provider(p: CloudProvider) -> Option<String> {
    pctwin_orchestrate::cloud::provider_name(p).map(str::to_owned)
}

/// "Safe in <provider>": files kept in their cloud, never counted as copied.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct SafeInLine {
    pub provider: Option<String>,
    pub files: u64,
}

/// One row of the owner table across laptops (M07): a person on an old laptop, and where their
/// things land on the new one.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct OwnerRow {
    pub laptop: String,
    pub person: String,
    /// The new laptop's account; `None`: a new account is made for them.
    pub account: Option<String>,
    /// Their names clash with another laptop's: their things land in "From <laptop>" folders.
    pub own_folder: bool,
}

pctwin_types::listed_enum! {
    /// Why something changed after approval (X06, H10).
    #[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
    #[serde(rename_all = "kebab-case")]
    pub enum ChangedWhy {
        /// The file or folder changed on the old laptop.
        ChangedOnOldLaptop,
        /// An app updated itself.
        AppUpdated,
        /// The place it would go changed.
        DestinationChanged,
        /// A new person or laptop joined the move.
        NewScope,
        /// The permission given before has ended.
        PermissionEnded,
    }
}

/// One item that changed after approval: only these are asked about again.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ChangedLine {
    pub name: String,
    pub why: ChangedWhy,
}

pctwin_types::listed_enum! {
    /// Why the move needs the system's administrator prompt (2.2): one prompt for the whole move.
    #[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
    #[serde(rename_all = "kebab-case")]
    pub enum PermissionReason {
        /// Reading other people's files on the old laptop.
        ReadOtherPeople,
        /// Making accounts for them on the new laptop.
        MakeAccounts,
        /// Shared folders a standard account cannot write.
        SharedFolders,
        /// Installing apps for everyone on the laptop.
        InstallForEveryone,
        /// A Windows restore point before the changes.
        RestorePoint,
    }
}

pctwin_types::listed_enum! {
    /// Where the system prompt is.
    #[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
    #[serde(rename_all = "kebab-case")]
    pub enum PermissionStage {
        /// Why it is needed, before the system asks.
        Explain,
        /// The system's own prompt is open.
        WaitingForSystem,
        /// The person said no: only the actions that needed it are left out.
        Denied,
        /// The prompt was closed without an answer: not a yes.
        ClosedWithoutAnswer,
        /// Given, and in use.
        Given,
        /// It ended when the laptop went to sleep.
        EndedBySleep,
        /// It ended when another person signed in or switched user.
        EndedOtherSession,
    }
}

pctwin_types::listed_enum! {
    /// Why no restore point was made.
    #[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
    #[serde(rename_all = "kebab-case")]
    pub enum NotMadeWhy {
        NotWindows,
        ProtectionOff,
        NeedsAdmin,
        Unconfirmed,
        Failed,
    }
}

impl From<NotMadeReason> for NotMadeWhy {
    fn from(r: NotMadeReason) -> Self {
        match r {
            NotMadeReason::NotWindows => NotMadeWhy::NotWindows,
            NotMadeReason::ProtectionOff => NotMadeWhy::ProtectionOff,
            NotMadeReason::NeedsAdmin => NotMadeWhy::NeedsAdmin,
            NotMadeReason::Unconfirmed => NotMadeWhy::Unconfirmed,
            NotMadeReason::Failed => NotMadeWhy::Failed,
        }
    }
}

pctwin_types::listed_enum! {
    /// The restore point's honest result.
    #[derive(Debug, Clone, PartialEq, Eq, Serialize)]
    #[serde(tag = "kind", rename_all = "kebab-case")]
    pub enum RestorePointView {
        /// Made, at this time (as Windows wrote it).
        Made { at: String },
        /// Windows made none because one from this time exists.
        EarlierExists { at: String },
        /// Not made: the person chose not to.
        NotChosen,
        /// Not made, and why.
        NotMade { why: NotMadeWhy },
    }
}

impl From<&RestorePointResult> for RestorePointView {
    fn from(r: &RestorePointResult) -> Self {
        match r {
            RestorePointResult::Made { at, .. } => RestorePointView::Made { at: name(at) },
            RestorePointResult::EarlierExists { at, .. } => {
                RestorePointView::EarlierExists { at: name(at) }
            }
            RestorePointResult::NotMade { reason } => RestorePointView::NotMade {
                why: (*reason).into(),
            },
        }
    }
}

pctwin_types::listed_enum! {
    /// Why a rescue drive is locked (T03). The screens explain how to unlock it in the system;
    /// PCTwin never asks for the key.
    #[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
    #[serde(rename_all = "kebab-case")]
    pub enum LockView {
        BitLocker,
        FileVault,
        AppleChip,
    }
}

impl From<Lock> for LockView {
    fn from(l: Lock) -> Self {
        match l {
            Lock::BitLocker => LockView::BitLocker,
            Lock::FileVault => LockView::FileVault,
            Lock::AppleChip => LockView::AppleChip,
        }
    }
}

pctwin_types::listed_enum! {
    /// How sure PCTwin is that the rescued drive does not change while it is read.
    #[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
    #[serde(rename_all = "kebab-case")]
    pub enum ReadOnlyView {
        /// The system opened it read only: nothing can change on it.
        SystemReadOnly,
        /// PCTwin only reads it, but the system has it open for writing.
        PctwinReadsOnly,
    }
}

impl From<ReadOnlyLevel> for ReadOnlyView {
    fn from(l: ReadOnlyLevel) -> Self {
        match l {
            ReadOnlyLevel::MountedReadOnly => ReadOnlyView::SystemReadOnly,
            ReadOnlyLevel::ReadByPctwinOnly => ReadOnlyView::PctwinReadsOnly,
        }
    }
}

/// A drive the rescue picker shows (T02).
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct RescueDrive {
    pub drive: String,
    pub size: u64,
    pub file_system: String,
    /// Something of the move is put on this drive: it cannot also be the one rescued.
    pub is_target: bool,
}

pctwin_types::listed_enum! {
    /// Where the dead-laptop rescue is (T01 to T04).
    #[derive(Debug, Clone, PartialEq, Eq, Serialize)]
    #[serde(tag = "kind", rename_all = "kebab-case")]
    pub enum RescueStage {
        /// What a rescue can and cannot do, and that the drive is only read (T01).
        Intro,
        /// Choose the drive (T02). `refused`: the drive just chosen is also where things would
        /// be put, so it was refused.
        #[serde(rename_all = "camelCase")]
        PickDrive { drives: Vec<RescueDrive>, refused: Option<u32> },
        /// The drive is locked (T03).
        Locked { lock: LockView },
        /// The drive holds no laptop's system and home folders.
        NoSystem,
        /// Read (T04): how many items were read, how sure PCTwin is it did not change the
        /// drive, and what could not be read.
        #[serde(rename_all = "camelCase")]
        Salvage {
            readable: u64,
            read_only: ReadOnlyView,
            unreadable: Vec<String>,
            more: u64,
        },
    }
}

impl RescueStage {
    /// The stage a rescue error leads to, given the drives shown and the one chosen.
    pub fn from_error(e: &RescueError, drives: Vec<RescueDrive>, chosen: u32) -> Self {
        match e {
            RescueError::SourceIsTarget => RescueStage::PickDrive {
                drives,
                refused: Some(chosen),
            },
            RescueError::Locked(lock) => RescueStage::Locked {
                lock: (*lock).into(),
            },
            // The drive in that place is not the one listed (it was swapped), or where things
            // would be put cannot be told apart from it: choose again, from a fresh list (third
            // review, findings 1 and 2). The screens have no words of their own for these yet.
            RescueError::NotTheDrive | RescueError::TargetUnknown => RescueStage::PickDrive {
                drives,
                refused: None,
            },
            RescueError::NoSystemFound => RescueStage::NoSystem,
            RescueError::Unreadable(what) => RescueStage::Salvage {
                readable: 0,
                read_only: ReadOnlyView::PctwinReadsOnly,
                unreadable: vec![name(what)],
                more: 0,
            },
        }
    }
}

pctwin_types::listed_enum! {
    /// What one report line is about.
    #[derive(Debug, Clone, PartialEq, Eq, Serialize)]
    #[serde(tag = "kind", rename_all = "kebab-case")]
    pub enum LineWhat {
        Items { count: u64 },
        App { name: String },
        Settings { count: u64 },
    }
}

pctwin_types::listed_enum! {
    /// Why a part of the move did not finish.
    #[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
    #[serde(rename_all = "kebab-case")]
    pub enum LineWhy {
        Failed,
        LaptopNotConnected,
        WaitsForAnother,
        NotStarted,
    }
}

/// A partial report line (X12), with its own Try again.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ReportLine {
    /// Its number, which Try again names.
    pub line: u32,
    pub what: LineWhat,
    pub laptop: String,
    pub why: LineWhy,
}

impl ReportLine {
    pub fn from_missing(line: u32, m: &Missing, laptop: &str) -> Self {
        let what = match &m.scope {
            Scope::Items { items, .. } => LineWhat::Items {
                count: items.len() as u64,
            },
            Scope::App { name: app } => LineWhat::App { name: name(app) },
            Scope::Settings { names, .. } => LineWhat::Settings {
                count: names.len() as u64,
            },
        };
        let why = match m.why {
            Why::Failed => LineWhy::Failed,
            Why::SourceUnavailable => LineWhy::LaptopNotConnected,
            Why::WaitingFor(_) => LineWhy::WaitsForAnother,
            Why::NotStarted => LineWhy::NotStarted,
        };
        ReportLine {
            line,
            what,
            laptop: name(laptop),
            why,
        }
    }
}

pctwin_types::listed_enum! {
    /// What undo will do with one change, looked at now (R05).
    #[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
    #[serde(rename_all = "kebab-case")]
    pub enum UndoPlan {
        /// Put back as it was.
        PutBack,
        /// PCTwin removes the app (through Windows Installer); it does not touch files the person
        /// made, and promises nothing about what the app's own uninstaller does.
        RemoveApp,
        /// Kept: it changed since the move.
        KeepChanged,
        /// Only its service or maker can undo it.
        Elsewhere,
        /// It was there before: left alone.
        LeaveAlone,
        /// An app PCTwin installed: the person removes it in the system's settings if they want.
        RemoveYourself,
        /// An app PCTwin installed that is another version now: kept.
        KeepOtherVersion,
        /// An app PCTwin installed that is no longer there.
        AlreadyGone,
    }
}

/// One line of the undo preview.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct UndoPlanLine {
    pub name: String,
    pub plan: UndoPlan,
}

pctwin_types::listed_enum! {
    /// How undoing one thing ended (R06).
    #[derive(Debug, Clone, PartialEq, Eq, Serialize)]
    #[serde(tag = "kind", rename_all = "kebab-case")]
    pub enum UndoResult {
        /// What was there before is back.
        Restored,
        /// PCTwin's copy or app is gone (the original is still on the old laptop).
        Removed,
        /// Nothing to put back: it was already as before.
        AsBefore,
        /// Kept because the person changed it after the move.
        KeptChanged,
        /// Kept for another reason (in use, not empty, not safe to remove).
        KeptOther,
        /// Kept, under another name beside it.
        KeptAt { at: String },
        /// Only its service or maker can undo it.
        DoneElsewhere,
        /// Not done this time; undo tries again next time.
        NotDoneYet,
        /// An app PCTwin installed, left for the person to remove in the system's settings.
        RemoveYourself,
        /// An app PCTwin installed that is another version now: kept.
        KeptOtherVersion,
        /// An app PCTwin installed, removed; a restart finishes it.
        RemovedRestart,
        /// An app PCTwin installed that could not be removed this time: the person can remove it
        /// in the system's settings.
        NotRemoved,
    }
}

/// Which "kept" an engine reason means. The engine's words are kept for More details.
fn kept(why: &str) -> UndoResult {
    if why.contains("changed") || why.contains("edited") {
        UndoResult::KeptChanged
    } else {
        UndoResult::KeptOther
    }
}

/// One line of the undo report: the result, and the engine's own words for More details.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct UndoLine {
    pub name: String,
    pub result: UndoResult,
    pub detail: Option<String>,
}

impl UndoLine {
    pub fn from_file(item: &str, o: &UndoOutcome) -> Self {
        let (result, detail) = match o {
            UndoOutcome::Deleted | UndoOutcome::Removed => (UndoResult::Removed, None),
            UndoOutcome::Trashed { .. } => (UndoResult::Removed, None),
            UndoOutcome::AlreadyGone => (UndoResult::AsBefore, None),
            UndoOutcome::Kept { why } => (kept(why), Some(name(why))),
            UndoOutcome::KeptAt { at, why } => {
                (UndoResult::KeptAt { at: name(at) }, Some(name(why)))
            }
            UndoOutcome::NotDone { why } => (UndoResult::NotDoneYet, Some(name(why))),
        };
        UndoLine {
            name: name(item),
            result,
            detail,
        }
    }

    pub fn from_effect(item: &str, class: EffectClass, o: &EffectUndoOutcome) -> Self {
        let elsewhere = matches!(
            class.undo_route(),
            pctwin_journal::UndoRoute::Elsewhere | pctwin_journal::UndoRoute::Never
        );
        let (result, detail) = match o {
            _ if elsewhere => (UndoResult::DoneElsewhere, None),
            EffectUndoOutcome::Uninstalled => (UndoResult::Removed, None),
            EffectUndoOutcome::Restored => (UndoResult::Restored, None),
            EffectUndoOutcome::AsBefore => (UndoResult::AsBefore, None),
            EffectUndoOutcome::Kept { why } => (kept(why), Some(name(why))),
            EffectUndoOutcome::NotDone { why } => (UndoResult::NotDoneYet, Some(name(why))),
        };
        UndoLine {
            name: name(item),
            result,
            detail,
        }
    }
}

pctwin_types::listed_enum! {
    /// One screen's content. Which screen shows is chosen by this, never by where the screens
    /// navigated to.
    #[derive(Debug, Clone, PartialEq, Eq, Serialize)]
    #[serde(tag = "kind", rename_all = "kebab-case")]
    pub enum ScreenView {
        /// After a restart: the engine is checking where things got to before any action is
        /// offered (X09).
        Reconciling,
        /// "Before we change anything" (2.9; M21, M24, M31): every change with its undo
        /// explanation, the download, the busiest-moment space per drive, and the permissions.
        #[serde(rename_all = "camelCase")]
        Changes {
            changes: Vec<ChangeLine>,
            more: u64,
            /// `None`: not known yet.
            download_bytes: Option<u64>,
            drives: Vec<DriveBudget>,
            permissions: Vec<PermissionReason>,
            /// Whether a Windows restore point will be asked for first.
            restore_point: bool,
        },
        /// Only the items that changed after approval, to approve again (X06, H10).
        #[serde(rename_all = "camelCase")]
        Changed { lines: Vec<ChangedLine>, more: u64 },
        /// A new laptop that is too small: each drive's budget, and the proposal (M31).
        #[serde(rename_all = "camelCase")]
        Space {
            drives: Vec<DriveBudget>,
            proposal: Vec<ProposalLine>,
            offload: Option<OffloadView>,
            safe_in: Vec<SafeInLine>,
        },
        /// Keeping a local copy of online-only files (M09).
        #[serde(rename_all = "camelCase")]
        LocalCopy {
            provider: Option<String>,
            files: u64,
            download_bytes: u64,
            /// The old laptop's free space; `None`: it does not say.
            old_free_bytes: Option<u64>,
            state: LocalCopyState,
        },
        /// Who goes where, across laptops (M07).
        #[serde(rename_all = "camelCase")]
        Owners { rows: Vec<OwnerRow>, more: u64 },
        /// The one system prompt for the move (M25, H10).
        #[serde(rename_all = "camelCase")]
        Permission {
            stage: PermissionStage,
            reasons: Vec<PermissionReason>,
            restore_point: Option<RestorePointView>,
        },
        /// The dead-laptop rescue (T01 to T04).
        Rescue { stage: RescueStage },
        /// What did not finish, each with Try again (X12), and what is safe in a cloud.
        #[serde(rename_all = "camelCase")]
        Report {
            lines: Vec<ReportLine>,
            more: u64,
            safe_in: Vec<SafeInLine>,
        },
        /// What undo would do, looked at now (R05).
        #[serde(rename_all = "camelCase")]
        UndoPreview {
            lines: Vec<UndoPlanLine>,
            more: u64,
            /// When the move it undoes finished, in seconds since 1970 (`None`: a move kept by
            /// PCTwin 0.1.x, whose time was not kept).
            when: Option<u64>,
        },
        /// What undo did (R06).
        #[serde(rename_all = "camelCase")]
        UndoDone { lines: Vec<UndoLine>, more: u64 },
    }
}

impl ScreenView {
    /// How many lines of Try again it offers (for checking a Try again's line number).
    pub fn report_lines(&self) -> Vec<u32> {
        match self {
            ScreenView::Report { lines, .. } => lines.iter().map(|l| l.line).collect(),
            _ => Vec::new(),
        }
    }

    /// How many drives the rescue picker shows.
    pub fn rescue_drives(&self) -> usize {
        match self {
            ScreenView::Rescue {
                stage: RescueStage::PickDrive { drives, .. },
            } => drives.len(),
            _ => 0,
        }
    }

    /// Whether the rescue drive at `index` is also a target (and so may not be chosen).
    pub fn rescue_drive_is_target(&self, index: usize) -> bool {
        match self {
            ScreenView::Rescue {
                stage: RescueStage::PickDrive { drives, .. },
            } => drives.get(index).is_some_and(|d| d.is_target),
            _ => false,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn every_effect_class_has_an_honest_undo_promise() {
        use EffectClass::*;
        let install = ChangeAction::Install;
        assert_eq!(
            UndoPromise::of(NativePackageInstall, install),
            UndoPromise::CanRemoveApp
        );
        assert_eq!(
            UndoPromise::of(SettingsChange, ChangeAction::Replace),
            UndoPromise::CanReverse
        );
        assert_eq!(
            UndoPromise::of(RemoteServiceEffect, ChangeAction::Merge),
            UndoPromise::NeedsYouService
        );
        assert_eq!(
            UndoPromise::of(CredentialOrLicenceChange, install),
            UndoPromise::NeedsYouMaker
        );
        assert_eq!(
            UndoPromise::of(DestructiveSanitisation, install),
            UndoPromise::MustKeep
        );
        // Something already there is never promised away.
        assert_eq!(
            UndoPromise::of(OwnedStagedFile, ChangeAction::LeaveAlone),
            UndoPromise::MustKeep
        );
    }

    #[test]
    fn undo_outcomes_are_never_shown_as_more_than_they_are() {
        let changed = UndoLine::from_file(
            "a.txt",
            &UndoOutcome::Kept {
                why: "it changed since the move".into(),
            },
        );
        assert_eq!(changed.result, UndoResult::KeptChanged);
        assert_eq!(changed.detail.as_deref(), Some("it changed since the move"));
        let in_use = UndoLine::from_file(
            "b.txt",
            &UndoOutcome::Kept {
                why: "in use".into(),
            },
        );
        assert_eq!(in_use.result, UndoResult::KeptOther);
        let later = UndoLine::from_file(
            "c.txt",
            &UndoOutcome::NotDone {
                why: "locked".into(),
            },
        );
        assert_eq!(later.result, UndoResult::NotDoneYet);
        // A service's change is "done elsewhere", whatever the local outcome says.
        let mail = UndoLine::from_effect(
            "Mail upload",
            EffectClass::RemoteServiceEffect,
            &EffectUndoOutcome::Restored,
        );
        assert_eq!(mail.result, UndoResult::DoneElsewhere);
        let setting = UndoLine::from_effect(
            "Dark mode",
            EffectClass::SettingsChange,
            &EffectUndoOutcome::Restored,
        );
        assert_eq!(setting.result, UndoResult::Restored);
    }

    #[test]
    fn restore_point_results_keep_their_meaning() {
        let made = RestorePointResult::Made {
            at: "2026-10-27 10:00".into(),
            number: 3,
        };
        assert_eq!(
            RestorePointView::from(&made),
            RestorePointView::Made {
                at: "2026-10-27 10:00".into()
            }
        );
        let earlier = RestorePointResult::EarlierExists {
            at: "2026-10-27 08:00".into(),
            number: 2,
        };
        assert!(matches!(
            RestorePointView::from(&earlier),
            RestorePointView::EarlierExists { .. }
        ));
        let off = RestorePointResult::NotMade {
            reason: NotMadeReason::ProtectionOff,
        };
        assert_eq!(
            RestorePointView::from(&off),
            RestorePointView::NotMade {
                why: NotMadeWhy::ProtectionOff
            }
        );
    }

    #[test]
    fn a_drive_chosen_as_both_source_and_target_is_refused_on_the_picker() {
        let drives = vec![RescueDrive {
            drive: "E:".into(),
            size: 1,
            file_system: "NTFS".into(),
            is_target: true,
        }];
        let stage = RescueStage::from_error(&RescueError::SourceIsTarget, drives, 0);
        assert!(matches!(
            stage,
            RescueStage::PickDrive {
                refused: Some(0),
                ..
            }
        ));
        let locked = RescueStage::from_error(&RescueError::Locked(Lock::BitLocker), vec![], 0);
        assert_eq!(
            locked,
            RescueStage::Locked {
                lock: LockView::BitLocker
            }
        );
    }

    #[test]
    fn names_and_lists_are_bounded_and_the_rest_counted() {
        let long = "x".repeat(MAX_NAME + 50);
        assert_eq!(name(&long).chars().count(), MAX_NAME);
        let (kept, more) = bounded((0..MAX_LINES + 7).collect::<Vec<_>>());
        assert_eq!(kept.len(), MAX_LINES);
        assert_eq!(more, 7);
    }
}
