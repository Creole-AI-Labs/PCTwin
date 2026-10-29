//! Finishing a move (Task List 2.7; plan 2.7 as adopted 26 October 2026; screens R01 to R17).
//!
//! The commands the finishing screens call, and the views they are sent. Every word is a key the
//! screens translate (a kebab-case value, or a `kind` tag): the engine never sends English. Names
//! the person knows (their files, their apps, places on their laptop) are sent as they are, and
//! only the mover's own; another person's things are only ever counted.
//!
//! A move is named by its My PCTwin history entry ID (`id`; none: the newest move). What a move
//! leaves on the new laptop for finishing it (its approved plan, the mover, its journal scope) is
//! kept by [`crate::finish_store`] when the move ends.
//!
//! Nothing here changes the old laptop. The steps that would (removing the mover's account,
//! erasing the old laptop) are the person's own, through the system's own tools: PCTwin checks
//! the gate (Security Design S14; plan 2.7 section 7), closes undo for good when the person
//! confirms an erase, opens the system's own page, and records what the person says happened,
//! never a certificate.

use serde::{Deserialize, Serialize};
use tauri::{AppHandle, Runtime};

/// A closed list of keys: kebab-case on the wire, with every value in `ALL` (made with the enum,
/// so the examples can hold each one).
macro_rules! keys {
    ($(#[$m:meta])* pub enum $t:ident { $($(#[$vm:meta])* $v:ident),+ $(,)? }) => {
        $(#[$m])*
        #[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
        #[serde(rename_all = "kebab-case")]
        pub enum $t { $($(#[$vm])* $v),+ }
        impl $t {
            /// Every value, in the order declared.
            #[allow(dead_code)]
            pub const ALL: &'static [$t] = &[$($t::$v),+];
        }
    };
}

/// The event a "check the old laptop again" or "bring me up to date" run sends when the old
/// laptop's fresh list has arrived and its changes are worked out.
pub const DELTA_EVENT: &str = "finish-delta";

pctwin_types::listed_enum! {
    /// Why a finishing command was refused, as a translation key.
    #[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
    #[serde(tag = "kind", rename_all = "kebab-case")]
    pub enum FinishProblem {
        /// No move with that ID is kept on this laptop (or none at all).
        NoMove,
        /// What the person answered is not what is shown now: look again.
        Changed,
        /// That cannot be done for this (an exception only PCTwin can check, say).
        NotAllowed,
        /// A move or another finishing step is going on.
        Busy,
        /// Something went wrong on this laptop; nothing was changed.
        Failed,
    }
}

/* ---------- The overview (R01, the after-move check) ---------- */

keys! {
    /// The one line that sums up the move, never a green tick over unfinished work (plan 2.7
    /// section 10: "ready for the selected work", "finished with tasks", "data preserved", "source
    /// handover unresolved").
    pub enum Headline {
        /// Everything picked is on the new laptop, and every check the person picked passed.
        ReadyForSelectedWork,
        /// Everything picked is on the new laptop, checked as it arrived; no work was checked.
        DataPreserved,
        /// Some things need the person, or failed (a key not checked counts).
        FinishedWithTasks,
        /// The new laptop is done; the before-wipe list still has open things.
        HandoverUnresolved,
        /// Nothing was picked to move.
        NothingMoved,
    }
}

keys! {
    /// Where one item stands, as one key.
    pub enum StandingKey {
        Copied,
        Reinstalled,
        Converted,
        NeedsAction,
        Failed,
        Waiting,
        NotReachable,
        NoResultYet,
        SkippedByUser,
        LeftOut,
    }
}

keys! {
    /// Whether a moved item can be opened, as far as keys go.
    pub enum AccessKey {
        Usable,
        KeyNotChecked,
        NotOnNewLaptop,
    }
}

/// Counts, every item in exactly one standing.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct TallyView {
    pub selected: u64,
    pub done: u64,
    pub needs_action: u64,
    pub failed: u64,
    pub waiting: u64,
    pub not_reachable: u64,
    pub no_result_yet: u64,
    pub skipped: u64,
    pub left_out: u64,
    /// Of those not done, how many the person chose to leave behind.
    pub accepted_loss: u64,
}

/// One of the mover's items that is not finished.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ItemView {
    /// The item's ID (hex), stable across screens and runs.
    pub id: String,
    /// Where it was, as the person knows it (`Letters/ada.txt`), or the app's or setting's name.
    pub label: String,
    pub kind: pctwin_record::ItemKind,
    pub standing: StandingKey,
    pub reason: Option<pctwin_types::Reason>,
    pub access: AccessKey,
}

/// One group in the after-move check: how many the scan counted, and how many are on the new
/// laptop now, checked against the journal on disk (never the screens' own count).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct AfterGroup {
    pub group: AfterKind,
    pub at_scan: u64,
    pub on_new_laptop: u64,
}

keys! {
    /// The groups the after-move check counts, the way the scan counts them.
    pub enum AfterKind {
        Photos,
        Videos,
        Music,
        Documents,
        OtherFiles,
        Apps,
        Settings,
    }
}

keys! {
    /// A card the overview shows beside the counts.
    pub enum FinishNote {
        /// Parental controls and family safety settings never move: set them up again.
        ParentalControls,
        /// Some places on the old laptop were not searched.
        NotEverywhere,
        /// Some results were refused because the new laptop's state did not show them.
        ResultsRefused,
        /// Undo is still open for this move.
        UndoOpen,
    }
}

/// Where PCTwin looked on the old laptop, and where it did not.
#[derive(Debug, Clone, PartialEq, Eq, Default, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct SearchView {
    pub searched: Vec<String>,
    pub not_searched: Vec<String>,
    /// Every place known was searched. Never more than that: it does not say there is nothing
    /// anywhere else.
    pub found_everything: bool,
}

/// The overview of one finished move (R01 and the after-move check).
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct FinishView {
    /// The history entry ID of this move.
    pub id: String,
    pub headline: Headline,
    pub tally: TallyView,
    /// Other people's items, counted only.
    pub others: u64,
    pub after_move: Vec<AfterGroup>,
    pub notes: Vec<FinishNote>,
    /// The mover's items not finished, at most [`MAX_LINES`]; `more` counts the rest.
    pub not_finished: Vec<ItemView>,
    pub more: u64,
    pub search: SearchView,
    /// When the old laptop was last checked (seconds since 1970; `None`: not since the move).
    pub old_laptop_checked_at: Option<u64>,
}

/// The most item lines a view carries; the rest are counted.
pub const MAX_LINES: usize = 200;

/* ---------- Before any wipe (R08 list, keep-until) ---------- */

keys! {
    /// What one thing on the before-wipe list is about.
    pub enum ExceptionKey {
        OnlyOnOldLaptop,
        KeyNotChecked,
        TiedToOldLaptop,
        DoFirstOnOldLaptop,
        StepsNotFinished,
        NotSearched,
        OldLaptopNotReachable,
        SpecialSignIn,
        SpecialRelink,
        SpecialExportFirst,
        SpecialLookAtIt,
    }
}

/// How one thing on the list stands.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(tag = "kind", rename_all = "kebab-case")]
pub enum ResolutionView {
    Open,
    /// The person says they did it (their word, not PCTwin's check).
    PersonDidIt {
        at: u64,
    },
    /// The person chose to leave it behind: a decision, never a recovery.
    AcceptedLoss {
        at: u64,
    },
}

/// One named thing an exception is about.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct NamedItem {
    pub id: String,
    pub label: String,
    pub kind: pctwin_record::ItemKind,
}

/// One thing on the before-wipe list.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ExceptionView {
    /// Names exactly this thing (hex).
    pub id: String,
    pub key: ExceptionKey,
    /// How many items, places or steps it is about.
    pub count: u64,
    pub items: Vec<NamedItem>,
    pub more: u64,
    /// Places on the old laptop, as the person knows them.
    pub places: Vec<String>,
    /// The apps or parts it names (special steps), as the person knows them.
    pub names: Vec<String>,
    pub resolution: ResolutionView,
    /// The person may choose to leave it behind (an old laptop that cannot be reached cannot be).
    pub can_accept: bool,
    /// Only the person can do it, so their word settles it.
    pub can_mark_done: bool,
    /// Binds an answer to exactly what is shown now (hex); sent back with the answer.
    pub shown: String,
}

/// The old laptop, as the list knows it.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(tag = "kind", rename_all = "kebab-case")]
pub enum OldLaptopView {
    /// Checked again since the move, at this time.
    Checked { at: u64 },
    /// Not checked since the move: connect it and check again.
    NotChecked,
}

/// The before-wipe list (R08): saved on the new laptop each time it is built.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct GoodbyeView {
    pub id: String,
    pub made_at: u64,
    pub suggested_keep_until: u64,
    pub keep_until: Option<u64>,
    pub undo_open: bool,
    pub old_laptop: OldLaptopView,
    /// Every thing resolved or accepted, and the old laptop checked: the wipe may be asked for.
    pub clear: bool,
    pub search: SearchView,
    pub exceptions: Vec<ExceptionView>,
    /// The list is saved on the new laptop (in PCTwin's data folder).
    pub saved: bool,
}

keys! {
    /// The person's answer to one thing on the list.
    pub enum GoodbyeAnswer {
        /// Leave it behind, after its exact warning.
        AcceptLoss,
        /// I did it (a step only the person can do).
        DidIt,
    }
}

impl ExceptionKey {
    /// A step from a special category (2.5), named by its app or part.
    pub fn special(self) -> bool {
        matches!(
            self,
            Self::SpecialSignIn
                | Self::SpecialRelink
                | Self::SpecialExportFirst
                | Self::SpecialLookAtIt
        )
    }
}

/* ---------- Bring me up to date (R13) ---------- */

keys! {
    /// Why the old laptop is being looked at again.
    pub enum CatchupPurpose {
        /// Only look: build the before-wipe list from the old laptop as it is now. Nothing moves.
        Check,
        /// Send only what changed since the move, matched by item ID, after the person reviews it.
        CatchUp,
    }
}

keys! {
    /// What changed for one item since the move.
    pub enum ChangeKey {
        New,
        Updated,
        BothChanged,
        GoneOnNewLaptop,
        GoneOnOldLaptop,
        CannotTell,
    }
}

impl ChangeKey {
    /// The person must choose: sending would land over, or beside, a copy that is not as it landed.
    pub fn needs_choice(self) -> bool {
        matches!(
            self,
            Self::BothChanged | Self::GoneOnNewLaptop | Self::CannotTell
        )
    }
}

/// One changed item.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct DeltaLineView {
    pub id: String,
    pub label: String,
    pub change: ChangeKey,
    /// The person must choose (both changed, gone on the new laptop, cannot tell).
    pub needs_choice: bool,
}

/// What changed on the old laptop since the move (sent with [`DELTA_EVENT`]).
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct DeltaView {
    pub id: String,
    pub purpose: CatchupPurpose,
    /// Binds the approval to exactly this list (hex).
    pub digest: String,
    pub lines: Vec<DeltaLineView>,
    pub more: u64,
    /// Changed items of other people, counted only.
    pub others: u64,
}

keys! {
    /// The person's choice for one changed item.
    pub enum CatchupChoice {
        Send,
        KeepBoth,
        KeepNewLaptopVersion,
    }
}

/// One choice the screens send.
#[derive(Debug, Clone, PartialEq, Eq, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ChoiceIn {
    pub id: String,
    pub choice: CatchupChoice,
}

/* ---------- Handing on, safe goodbye (R08 to R10) ---------- */

keys! {
    /// One of the two handover requests. Each is asked separately; none follows from another.
    pub enum HandoverAction {
        /// Remove the mover's own account from the old laptop, with the system's own tools.
        RemoveMyAccount,
        /// Erase the old laptop, with the system's own reset.
        EraseOldLaptop,
    }
}

keys! {
    /// Why a handover request is not offered now. Nothing was done.
    pub enum RefusalKey {
        WrongDevice,
        UndoNotClosed,
        OldLaptopNotChecked,
        ListOutOfDate,
        ListNotClear,
        NoWriteExclusion,
        NoSuchAccount,
        OtherPeopleUseIt,
        NotQualified,
        NoFreshConfirmation,
        WrongConfirmation,
        ChangedSinceConfirmed,
        Expired,
        AlreadyUsed,
        NothingToRemove,
    }
}

keys! {
    /// Something the person must know before confirming.
    pub enum HandoverNote {
        /// Deleting files on an SSD is not a secure wipe.
        SsdNotSecureWipe,
        /// Removing an account does not cancel subscriptions or sign out elsewhere.
        DoesNotCancelSubscriptions,
        /// Everyone else's account stays as it is.
        OthersStay,
        /// This cannot be undone.
        CannotBeUndone,
        /// Confirming closes undo for this move for good.
        ClosesUndo,
        /// An account cannot remove itself: sign in with another administrator account first.
        SignInAsAnotherAdministrator,
        /// An account in a Microsoft family group is removed from the family first, online.
        FamilyGroupFirst,
        /// What PCTwin records is what was asked and what you said happened, not a certificate.
        NotACertificate,
        /// Turning on the drive's encryption before the reset makes leftovers far harder to read.
        EncryptFirstHelps,
    }
}

keys! {
    /// Which system's own tool the person uses.
    pub enum SystemKey {
        Windows,
        Macos,
        Linux,
    }
}

/// The system's own route, as steps the screens put in words, from the routes catalogue.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct RouteView {
    /// The old laptop's system (`None`: it could not be told; the route is the maker's help).
    pub system: Option<SystemKey>,
    /// The route's ID in the catalogue (a key: `windows-reset-clean-drive`, ...).
    pub route: String,
    /// Each step's key, in order.
    pub steps: Vec<String>,
    /// PCTwin can open the system's own page for the first step.
    pub can_open: bool,
}

/// What the person sees before confirming, or why it is not offered.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(tag = "kind", rename_all = "kebab-case")]
pub enum HandoverView {
    /// Every check passed: confirm exactly this (send `digest` back with the confirmation).
    #[serde(rename_all = "camelCase")]
    Ready {
        action: HandoverAction,
        digest: String,
        /// What would be removed, by name (the mover's account, the drive).
        removes: Vec<String>,
        /// What stays, by name (other people's accounts).
        keeps: Vec<String>,
        notes: Vec<HandoverNote>,
        route: RouteView,
    },
    /// Not offered now, and why (`open`: how many things on the list are still open).
    #[serde(rename_all = "camelCase")]
    Refused {
        action: HandoverAction,
        why: RefusalKey,
        open: u64,
    },
}

keys! {
    /// What the person says happened after following the system's route (recorded as their word).
    pub enum PersonSays {
        Done,
        Partly,
        CancelledPartWay,
        NotStarted,
        CannotTell,
    }
}

keys! {
    /// What was recorded for a handover action.
    pub enum OutcomeKey {
        /// The person says it finished. A record, not a certificate.
        PersonSaysDone,
        Partial,
        Interrupted,
        CancelledAfterSomeEffects,
        NotStarted,
        Inconclusive,
    }
}

/// The gate passed and the person confirmed: undo is closed (for an erase), and the system's
/// route is next.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct HandoverStarted {
    pub action: HandoverAction,
    /// Undo for this move is closed for good, on disk.
    pub undo_closed: bool,
    /// The system's own page was opened.
    pub opened: bool,
    pub route: RouteView,
}

/// A recorded handover outcome.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct HandoverRecorded {
    pub action: HandoverAction,
    pub outcome: OutcomeKey,
    pub at: u64,
}

/* ---------- Connection doctor ---------- */

keys! {
    /// Which laptop asks.
    pub enum DoctorRole {
        Old,
        New,
    }
}

/// A likely cause, as its message key and fix key.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct FindingView {
    pub cause: String,
    pub fix: String,
    /// "sure" or "likely".
    pub sure: bool,
    /// Network or VPN names the cause is about, as the system names them.
    pub names: Vec<String>,
}

/// The phone-hotspot fallback: always there as a last resort, suggested first when the findings
/// point at the network itself.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct HotspotView {
    pub suggested: bool,
}

/// What the doctor found on this laptop.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct DoctorView {
    pub findings: Vec<FindingView>,
    pub hotspot: HotspotView,
    /// What the probes could not look at (each a key): never blamed.
    pub not_checked: Vec<String>,
}

/* ---------- Finish your setup, saved setup (R03, R04) ---------- */

keys! {
    /// What one setup task asks.
    pub enum SetupWhat {
        SignIn,
        LicenceKey,
        SetUpAgain,
        DownloadFromMaker,
        Permission,
        Restart,
        FinishByHand,
        PartsMissing,
        ExportFirst,
        Relink,
        LookAtIt,
    }
}

keys! {
    /// How a task stands: PCTwin's check and the person's word are kept apart.
    pub enum SetupMark {
        Open,
        MachineChecked,
        YouDidIt,
    }
}

/// One setup task.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct SetupTask {
    /// Stable for this move (hex).
    pub id: String,
    pub what: SetupWhat,
    /// The app or item it is about, as the person knows it.
    pub name: String,
    pub mark: SetupMark,
}

/// A setup profile saved in My PCTwin history.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct SavedSetupView {
    pub history_id: String,
    pub when: String,
}

/// The finish-your-setup checklist (R03).
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct SetupView {
    pub id: String,
    pub tasks: Vec<SetupTask>,
    pub saved: Vec<SavedSetupView>,
}

/// What a saved setup holds (R04): the app list and the settings groups, never files.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct SavedSetup {
    pub history_id: String,
    pub apps: u64,
    pub settings: u64,
}

/* ---------- Undo preview (R05; R06 is `fullmove_undo`) ---------- */

/// What undo can and cannot do for one move, before the person starts it.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct UndoPreviewView {
    pub id: String,
    /// The move's undo book, for `fullmove_undo` (`None`: nothing of apps or settings to undo).
    pub undo_id: Option<String>,
    /// Apps undo removes itself (while each is still the version installed).
    pub apps_removed: Vec<String>,
    /// Apps undo leaves for the person to remove.
    pub apps_yours: Vec<String>,
    /// Settings put back as they were.
    pub settings: u64,
    /// Apps whose data landed, put back.
    pub app_data: Vec<String>,
    /// Files the move copied: undone only with the old laptop connected, and only copies nobody
    /// changed since.
    pub files: u64,
    /// Undo is still open (closed for good once an erase of the old laptop was confirmed).
    pub open: bool,
}

/* ---------- Clean-up (R07) ---------- */

keys! {
    /// One of the two clean-up questions, asked separately.
    pub enum CleanupWhat {
        /// PCTwin's own temporary files on this laptop.
        TemporaryFiles,
        /// PCTwin's model and caches on this laptop.
        ModelAndCache,
    }
}

/// One thing found for clean-up.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct CleanupEntry {
    pub name: String,
    pub bytes: u64,
}

/// What clean-up would remove, and what it keeps.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct CleanupView {
    pub what: CleanupWhat,
    /// Binds the confirmation to exactly this list (hex).
    pub digest: String,
    /// PCTwin recorded making each of these.
    pub removes: Vec<CleanupEntry>,
    /// These only look like PCTwin's, or are in use: kept.
    pub keeps: Vec<CleanupEntry>,
}

/// What clean-up did.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct CleanupDone {
    pub what: CleanupWhat,
    pub removed: u64,
    pub kept: u64,
    pub failed: u64,
}

/* ---------- Report and feedback (R11, R12) ---------- */

/// What the person chose to include. Nothing is included unless chosen.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct DisclosureIn {
    pub file_names: bool,
    pub app_names: bool,
    pub account_id: bool,
}

/// One unfinished line of the shareable report.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ReportLineView {
    /// The name, only when chosen, scrubbed of anything that looks like a secret.
    pub name: Option<String>,
    /// "file" or "app", for the words standing in for a name left out.
    pub is_app: bool,
    pub standing: StandingKey,
    pub reason: Option<pctwin_types::Reason>,
}

/// How many things were kept out of the report.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct WithheldView {
    pub names: u64,
    pub secrets: u64,
    pub account_ids: u64,
}

/// Exactly what the shareable report would hold (R11). Nothing is sent: saving is the person's
/// own act, and feedback goes through Help, with no account (R12).
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ReportView {
    pub id: String,
    pub headline: Headline,
    pub tally: TallyView,
    pub account_id: Option<String>,
    pub lines: Vec<ReportLineView>,
    pub others_not_finished: u64,
    /// The before-wipe list: how many things, how many still open (`None`: no list yet).
    pub goodbye: Option<(u64, u64)>,
    pub withheld: WithheldView,
    /// Binds a save to exactly this report (hex).
    pub digest: String,
}

/* ---------- What a move leaves (kept when it ends) ---------- */

use std::collections::{BTreeMap, BTreeSet};
use std::path::PathBuf;
use std::sync::{Mutex, OnceLock};

use pctwin_journal::{Journal, UndoGate};
use pctwin_readiness::goodbye::{
    Checklist, ChecklistError, ExceptionKind, Inputs, OldLaptop, Resolution, SpecialStep, StepWhat,
};
use pctwin_readiness::handover::{self as gate, Action, Refusal};
use pctwin_readiness::ledger::{
    Access, JournalProbe, Ledger, MoveId, Observed, Probe, Receipt, Role, Standing, Tally,
};
use pctwin_record::{ItemId, ItemKind, Record};
use pctwin_types::{ItemResult, ItemStatus, Reason};

use crate::finish_store::{self as store, Kept, KeptLine, KeptResult, LineKind};

/// What the move leaves, before its history entry is made.
pub struct Seed(Kept);

fn random_u64() -> Option<u64> {
    let mut b = [0u8; 8];
    getrandom::fill(&mut b).ok()?;
    Some(u64::from_le_bytes(b))
}

/// The new laptop, as a move ends: what finishing it needs. Only the mover's own and shared items
/// are kept; other people are counted.
pub(crate) fn seed<R: Runtime>(
    app: &AppHandle<R>,
    received: &pctwin_move::Received,
    mover: &str,
    named: &[pctwin_move::landing::NamedPerson],
    report: &[crate::moving::ReportItem],
    rebuilt: Option<&crate::fullmove::RebuiltView>,
    started_at: u64,
) -> Option<Seed> {
    use tauri::Manager;
    let dir = app.path().app_data_dir().ok();
    let target = crate::moving::this_laptops_id(dir.as_deref()).ok()?;
    let record = store::mover_only(&received.record, mover);
    let results = report
        .iter()
        .filter_map(|r| {
            Some(KeptResult {
                item: ItemId::from_hex(&r.id).ok()?,
                status: r.status,
                reason: r.reason,
            })
        })
        .collect();
    let searched: BTreeSet<String> = record
        .items
        .iter()
        .filter_map(|i| {
            serde_json::to_value(i.place.role)
                .ok()?
                .as_str()
                .map(String::from)
        })
        .collect();
    Some(Seed(Kept {
        format: store::FORMAT,
        history_id: String::new(),
        move_id: MoveId {
            source: received.record.source_laptop,
            target,
            session: random_u64()?,
        },
        mover: mover.to_string(),
        started_at,
        ended_at: store::now().max(started_at),
        others: store::others_of(&received.record, mover, named),
        record,
        results,
        lines: rebuilt.map(lines_of).unwrap_or_default(),
        undo_id: rebuilt.and_then(|r| r.undo_id.clone()),
        searched: searched.into_iter().collect(),
    }))
}

/// The rebuilt lines as kept: their kind, name, count, result and notes' keys.
fn lines_of(r: &crate::fullmove::RebuiltView) -> Vec<KeptLine> {
    let Ok(v) = serde_json::to_value(r) else {
        return Vec::new();
    };
    v["lines"]
        .as_array()
        .into_iter()
        .flatten()
        .filter_map(|l| {
            let kind = match l["kind"].as_str()? {
                "app" => LineKind::App,
                "settings" => LineKind::Settings,
                _ => LineKind::Special,
            };
            let name = match l["title"]["kind"].as_str()? {
                "name" => l["title"]["value"].as_str()?.to_string(),
                key => key.to_string(),
            };
            Some(KeptLine {
                kind,
                name,
                count: l["count"].as_u64().unwrap_or(0),
                status: serde_json::from_value(l["status"].clone()).ok()?,
                reason: serde_json::from_value(l["reason"].clone()).ok().flatten(),
                notes: l["notes"]
                    .as_array()
                    .into_iter()
                    .flatten()
                    .filter_map(|n| n["note"].as_str().map(String::from))
                    .collect(),
            })
        })
        .collect()
}

/// Keeps what the move left under its history entry `id`. Failing to keep it never fails a move.
pub(crate) fn keep<R: Runtime>(app: &AppHandle<R>, id: &str, seed: Seed) {
    let mut kept = seed.0;
    kept.history_id = id.to_string();
    if let Some(folder) = store::root(app).and_then(|r| store::folder(&r, id)) {
        let _ = store::save(&folder, "move.json", &kept);
    }
}

/* ---------- Reading a move back ---------- */

struct Move {
    folder: PathBuf,
    kept: Kept,
}

fn open<R: Runtime>(app: &AppHandle<R>, id: Option<String>) -> Result<Move, FinishProblem> {
    let root = store::root(app).ok_or(FinishProblem::Failed)?;
    let id = match id {
        Some(id) => id,
        None => store::newest(&root).ok_or(FinishProblem::NoMove)?,
    };
    let kept = store::kept(&root, &id).ok_or(FinishProblem::NoMove)?;
    let folder = store::folder(&root, &id).ok_or(FinishProblem::NoMove)?;
    Ok(Move { folder, kept })
}

fn journal<R: Runtime>(app: &AppHandle<R>) -> Option<Journal> {
    Journal::open(&crate::moving::journal_path(app)?).ok()
}

fn undo_open(j: Option<&Journal>) -> bool {
    !matches!(j.map(Journal::undo_gate), Some(Ok(UndoGate::Closed)))
}

fn hex(b: &[u8]) -> String {
    b.iter().map(|x| format!("{x:02x}")).collect()
}

fn unhex32(s: &str) -> Option<[u8; 32]> {
    if s.len() != 64 {
        return None;
    }
    let mut out = [0u8; 32];
    for (i, o) in out.iter_mut().enumerate() {
        *o = u8::from_str_radix(s.get(i * 2..i * 2 + 2)?, 16).ok()?;
    }
    Some(out)
}

/// The probe for a copy: the journal on disk, read again now.
struct JournalSees<'a>(&'a JournalProbe);

impl Probe for JournalSees<'_> {
    fn observe(&self, r: &Receipt) -> Observed {
        self.0.observe(r)
    }
}

/// The ledger of a move, built from its kept plan and results and checked against the journal on
/// disk now: a copy is "done" only when the journal shows it committed (or already there) and not
/// undone (T27-001). Returns it with how many results were refused.
fn ledger_of(k: &Kept, j: Option<&Journal>) -> (Ledger, u64) {
    let mut l = Ledger::from_record(&k.record, &k.mover, k.move_id, k.started_at);
    for place in &k.searched {
        l.searched(place);
    }
    let probe = j
        .and_then(|j| JournalProbe::read_moves(j, k.move_id.source).ok())
        .unwrap_or_default();
    let now = store::now().max(k.ended_at);
    let mut refused = 0;
    for r in &k.results {
        let Ok(result) = ItemResult::new(r.status, r.reason) else {
            continue;
        };
        let evidence = result
            .status()
            .is_done()
            .then(|| probe.fingerprint(r.item))
            .flatten()
            .map(
                |fingerprint| pctwin_readiness::ledger::Evidence::VerifiedTransfer { fingerprint },
            );
        let receipt = Receipt {
            role: Role::Transfer,
            source: k.move_id.source,
            target: k.move_id.target,
            session: k.move_id.session,
            generation: l.generation(),
            item: r.item,
            result: Some(result),
            evidence,
            observed_at: k.ended_at,
        };
        if l.accept(receipt, &JournalSees(&probe), now).is_err() {
            refused += 1;
        }
    }
    (l, refused)
}

fn standing_key(s: Standing) -> StandingKey {
    match s {
        Standing::Done(ItemStatus::Reinstalled) => StandingKey::Reinstalled,
        Standing::Done(ItemStatus::Converted) => StandingKey::Converted,
        Standing::Done(_) => StandingKey::Copied,
        Standing::NeedsAction => StandingKey::NeedsAction,
        Standing::Failed => StandingKey::Failed,
        Standing::Deferred => StandingKey::Waiting,
        Standing::Inaccessible => StandingKey::NotReachable,
        Standing::NotTested => StandingKey::NoResultYet,
        Standing::SkippedByUser => StandingKey::SkippedByUser,
        Standing::Excluded => StandingKey::LeftOut,
    }
}

fn access_key(a: Access) -> AccessKey {
    match a {
        Access::Usable => AccessKey::Usable,
        Access::PreservedKeyUnresolved => AccessKey::KeyNotChecked,
        Access::NotOnNewLaptop => AccessKey::NotOnNewLaptop,
    }
}

fn tally_view(t: Tally) -> TallyView {
    TallyView {
        selected: t.selected,
        done: t.done,
        needs_action: t.needs_action,
        failed: t.failed,
        waiting: t.deferred,
        not_reachable: t.inaccessible,
        no_result_yet: t.not_tested,
        skipped: t.skipped,
        left_out: t.excluded,
        accepted_loss: t.accepted_loss,
    }
}

fn line_done(s: ItemStatus) -> bool {
    s.is_done()
}

/// The one line for the move: never ready while anything picked is not done, a key is not
/// checked, an app or setting needs the person, or the before-wipe list is open.
fn headline(l: &Ledger, k: &Kept, list: Option<&Checklist>) -> Headline {
    let apps_left = k.lines.iter().any(|x| !line_done(x.status));
    if l.tally().selected == 0 && k.lines.is_empty() {
        return Headline::NothingMoved;
    }
    match pctwin_readiness::checks::summarize(l, &[]) {
        pctwin_readiness::checks::Summary::FinishedWithTasks => Headline::FinishedWithTasks,
        _ if apps_left => Headline::FinishedWithTasks,
        _ if list.is_some_and(|c| !c.is_clear()) => Headline::HandoverUnresolved,
        pctwin_readiness::checks::Summary::ReadyForSelectedWork => Headline::ReadyForSelectedWork,
        pctwin_readiness::checks::Summary::DataPreserved => Headline::DataPreserved,
    }
}

fn search_view(l: &Ledger) -> SearchView {
    SearchView {
        searched: l.search().searched.iter().cloned().collect(),
        not_searched: l.search().not_searched.iter().cloned().collect(),
        found_everything: l.found_everything(),
    }
}

/// The after-move check: the mover's files counted as the scan counts them, against what the
/// journal shows on the new laptop now; apps and settings from their own lines.
fn after_move(l: &Ledger, k: &Kept) -> Vec<AfterGroup> {
    let mut groups: BTreeMap<u8, AfterGroup> = BTreeMap::new();
    let mut add = |order: u8, group: AfterKind, n: u64, done: u64| {
        let g = groups.entry(order).or_insert(AfterGroup {
            group,
            at_scan: 0,
            on_new_laptop: 0,
        });
        g.at_scan += n;
        g.on_new_laptop += done;
    };
    for item in &k.record.items {
        if item.kind != ItemKind::File
            || item.inclusion != pctwin_record::Inclusion::Included
            || !l.line(item.id).is_some_and(|x| x.mine)
        {
            continue;
        }
        let name = item
            .path
            .parts()
            .last()
            .map(|p| p.display())
            .unwrap_or_default();
        let (order, group) = match pctwin_scan::category(name) {
            pctwin_scan::Category::Photo => (0, AfterKind::Photos),
            pctwin_scan::Category::Video => (1, AfterKind::Videos),
            pctwin_scan::Category::Music => (2, AfterKind::Music),
            pctwin_scan::Category::Document => (3, AfterKind::Documents),
            pctwin_scan::Category::Other => (4, AfterKind::OtherFiles),
        };
        let done = matches!(l.standing(item.id), Some(Standing::Done(_)));
        add(order, group, 1, u64::from(done));
    }
    for x in &k.lines {
        match x.kind {
            LineKind::App => add(5, AfterKind::Apps, 1, u64::from(line_done(x.status))),
            LineKind::Settings => add(
                6,
                AfterKind::Settings,
                x.count,
                if line_done(x.status) { x.count } else { 0 },
            ),
            LineKind::Special => {}
        }
    }
    groups.into_values().collect()
}

fn item_view(l: &Ledger, id: ItemId) -> Option<ItemView> {
    let line = l.line(id)?;
    Some(ItemView {
        id: id.to_hex(),
        label: line.label.clone(),
        kind: line.kind,
        standing: standing_key(l.standing(id)?),
        reason: line.result.as_ref().and_then(|a| a.result.reason()),
        access: access_key(l.access(id)?),
    })
}

/* ---------- The before-wipe list ---------- */

/// The special-category steps from the app data lines that still need the person.
fn special_steps(k: &Kept) -> Vec<SpecialStep> {
    k.lines
        .iter()
        .filter(|x| x.kind == LineKind::Special && !line_done(x.status))
        .filter_map(|x| {
            let (kind, what) = match x.reason {
                Some(Reason::SignIn) => (ExceptionKind::TiedToOldLaptop, StepWhat::SignInAgain),
                Some(Reason::FinishByHand) if x.notes.iter().any(|n| n == "sites") => {
                    (ExceptionKind::TiedToOldLaptop, StepWhat::SignInAgain)
                }
                Some(Reason::FinishByHand) => {
                    (ExceptionKind::DoFirstOnOldLaptop, StepWhat::LookAtIt)
                }
                _ => return None,
            };
            Some(SpecialStep {
                kind,
                place: x.name.clone(),
                component: x.name.clone(),
                what,
            })
        })
        .collect()
}

/// The list, built again from what is known now, with the person's earlier answers, and saved on
/// the new laptop.
fn checklist(
    m: &Move,
    l: &Ledger,
    j: Option<&Journal>,
    old: Option<&store::OldNow>,
) -> (Checklist, bool) {
    let previous: Option<Checklist> = store::load(&m.folder, "goodbye.json");
    let special = special_steps(&m.kept);
    let c = Checklist::build(
        &Inputs {
            ledger: l,
            approved: &m.kept.record,
            old_laptop: match old {
                Some(o) => OldLaptop::Scanned(&o.record),
                None => OldLaptop::NotReachable,
            },
            left_on_old_laptop: &[],
            steps_not_finished: &[],
            undo_open: undo_open(j),
            special: &special,
        },
        previous.as_ref(),
        store::now(),
    );
    let saved = store::save(&m.folder, "goodbye.json", &c).is_ok();
    (c, saved)
}

fn exception_key(e: &pctwin_readiness::goodbye::Exception) -> ExceptionKey {
    match (e.what, e.kind) {
        (Some(StepWhat::SignInAgain), _) => ExceptionKey::SpecialSignIn,
        (Some(StepWhat::Relink), _) => ExceptionKey::SpecialRelink,
        (Some(StepWhat::ExportFirst), _) => ExceptionKey::SpecialExportFirst,
        (Some(StepWhat::LookAtIt), _) => ExceptionKey::SpecialLookAtIt,
        (None, ExceptionKind::OnlyOnOldLaptop) => ExceptionKey::OnlyOnOldLaptop,
        (None, ExceptionKind::KeyNotChecked) => ExceptionKey::KeyNotChecked,
        (None, ExceptionKind::TiedToOldLaptop) => ExceptionKey::TiedToOldLaptop,
        (None, ExceptionKind::DoFirstOnOldLaptop) => ExceptionKey::DoFirstOnOldLaptop,
        (None, ExceptionKind::StepsNotFinished) => ExceptionKey::StepsNotFinished,
        (None, ExceptionKind::NotSearched) => ExceptionKey::NotSearched,
        (None, ExceptionKind::OldLaptopNotReachable) => ExceptionKey::OldLaptopNotReachable,
    }
}

fn goodbye_view(
    m: &Move,
    l: &Ledger,
    c: &Checklist,
    j: Option<&Journal>,
    old: Option<&store::OldNow>,
    saved: bool,
) -> GoodbyeView {
    let exceptions = c
        .exceptions
        .iter()
        .map(|e| {
            let key = exception_key(e);
            let items: Vec<NamedItem> = e
                .items
                .iter()
                .filter_map(|id| {
                    let line = l.line(*id).filter(|x| x.mine)?;
                    Some(NamedItem {
                        id: id.to_hex(),
                        label: line.label.clone(),
                        kind: line.kind,
                    })
                })
                .collect();
            let count = (e.items.len().max(e.places.len()).max(e.steps.len())).max(1) as u64;
            let more = (items.len() as u64).saturating_sub(MAX_LINES as u64);
            let special = key.special();
            ExceptionView {
                id: hex(&e.id),
                key,
                count,
                items: items.into_iter().take(MAX_LINES).collect(),
                more,
                places: if special { vec![] } else { e.places.clone() },
                names: if special { e.places.clone() } else { vec![] },
                resolution: match e.resolution {
                    Resolution::Open => ResolutionView::Open,
                    Resolution::PersonDidIt { at } => ResolutionView::PersonDidIt { at },
                    Resolution::AcceptedLoss { at } => ResolutionView::AcceptedLoss { at },
                },
                can_accept: e.acceptable(),
                can_mark_done: e.person_can_do(),
                shown: c.shown(&e.id).map(|d| hex(&d)).unwrap_or_default(),
            }
        })
        .collect();
    GoodbyeView {
        id: m.kept.history_id.clone(),
        made_at: c.made_at,
        suggested_keep_until: c.suggested_keep_until,
        keep_until: c.keep_until,
        undo_open: undo_open(j),
        old_laptop: match old {
            Some(o) => OldLaptopView::Checked { at: o.at },
            None => OldLaptopView::NotChecked,
        },
        clear: c.is_clear(),
        search: search_view(l),
        exceptions,
        saved,
    }
}

fn goodbye_now<R: Runtime>(
    app: &AppHandle<R>,
    id: Option<String>,
    change: impl FnOnce(&mut Checklist) -> Result<(), FinishProblem>,
) -> Result<GoodbyeView, FinishProblem> {
    let m = open(app, id)?;
    let j = journal(app);
    let (l, _) = ledger_of(&m.kept, j.as_ref());
    let old: Option<store::OldNow> = store::load(&m.folder, "old-now.json");
    let (mut c, _) = checklist(&m, &l, j.as_ref(), old.as_ref());
    change(&mut c)?;
    let saved = store::save(&m.folder, "goodbye.json", &c).is_ok();
    Ok(goodbye_view(&m, &l, &c, j.as_ref(), old.as_ref(), saved))
}

fn checklist_problem(e: ChecklistError) -> FinishProblem {
    match e {
        ChecklistError::NoSuchException | ChecklistError::NotTheWarningShown => {
            FinishProblem::Changed
        }
        ChecklistError::CannotBeAccepted
        | ChecklistError::NotThePersonsStep
        | ChecklistError::DateInThePast => FinishProblem::NotAllowed,
        ChecklistError::NewerFormat => FinishProblem::Failed,
    }
}

/* ---------- Handover ---------- */

/// The system's own routes, as data (a catalogue, never code per system): which steps the person
/// follows for each action on each system. Checked on 29 October 2026 against the systems' own
/// help (Windows: Settings, Accounts, Other users, Remove; Recovery, Reset this PC, Remove
/// everything, Clean the drive. macOS 12 and later on Apple silicon or the T2 chip: General,
/// Transfer or Reset, Erase All Content and Settings; other Macs: macOS Recovery, Disk Utility).
const ROUTES: &str = include_str!("finish_routes.json");

#[derive(serde::Deserialize)]
struct RouteRow {
    id: String,
    system: Option<SystemKey>,
    action: HandoverAction,
    steps: Vec<String>,
}

#[derive(serde::Deserialize)]
struct Routes {
    routes: Vec<RouteRow>,
}

fn routes() -> &'static [RouteRow] {
    static R: OnceLock<Vec<RouteRow>> = OnceLock::new();
    R.get_or_init(|| {
        serde_json::from_str::<Routes>(ROUTES)
            .map(|r| r.routes)
            .unwrap_or_default()
    })
}

/// The old laptop's system, from the apps its scan found (`None`: it cannot be told).
fn old_system(r: &Record) -> Option<SystemKey> {
    system_of(r.apps.found.iter().map(|a| a.system))
}

fn system_of(systems: impl Iterator<Item = pctwin_record::AppSystem>) -> Option<SystemKey> {
    let mut n = [0usize; 3];
    for s in systems {
        n[match s {
            pctwin_record::AppSystem::Windows => 0,
            pctwin_record::AppSystem::Mac => 1,
            pctwin_record::AppSystem::Linux => 2,
        }] += 1;
    }
    let max = *n.iter().max()?;
    if max == 0 || n.iter().filter(|x| **x == max).count() > 1 {
        return None;
    }
    [SystemKey::Windows, SystemKey::Macos, SystemKey::Linux]
        .into_iter()
        .zip(n)
        .find(|(_, c)| *c == max)
        .map(|(s, _)| s)
}

/// The route for `action` on `system` (an unknown system has its own route: the maker's help).
fn route(action: HandoverAction, system: Option<SystemKey>) -> RouteView {
    let row = routes()
        .iter()
        .find(|r| r.action == action && r.system == system)
        .or_else(|| {
            routes()
                .iter()
                .find(|r| r.action == action && r.system.is_none())
        });
    RouteView {
        system,
        route: row.map(|r| r.id.clone()).unwrap_or_default(),
        steps: row.map(|r| r.steps.clone()).unwrap_or_default(),
        // The route is followed on the old laptop; PCTwin here is on the new one, and never opens
        // a reset or account page on this laptop.
        can_open: false,
    }
}

fn action_of(a: HandoverAction) -> Action {
    match a {
        HandoverAction::RemoveMyAccount => Action::RemoveMoversAccount,
        HandoverAction::EraseOldLaptop => Action::EraseOldLaptop,
    }
}

fn refusal_key(r: &Refusal) -> (RefusalKey, u64) {
    match r {
        Refusal::WrongDevice => (RefusalKey::WrongDevice, 0),
        Refusal::UndoNotClosed => (RefusalKey::UndoNotClosed, 0),
        Refusal::OldLaptopNotReachable => (RefusalKey::OldLaptopNotChecked, 0),
        Refusal::ChecklistStale => (RefusalKey::ListOutOfDate, 0),
        Refusal::ExceptionsOpen { open } => (RefusalKey::ListNotClear, *open as u64),
        Refusal::NoWriteExclusion => (RefusalKey::NoWriteExclusion, 0),
        Refusal::NoSuchAccount => (RefusalKey::NoSuchAccount, 0),
        Refusal::OtherPeopleUseIt => (RefusalKey::OtherPeopleUseIt, 0),
        Refusal::NoFreshConfirmation => (RefusalKey::NoFreshConfirmation, 0),
        Refusal::WrongConfirmation => (RefusalKey::WrongConfirmation, 0),
        Refusal::ChangedSinceConfirmed => (RefusalKey::ChangedSinceConfirmed, 0),
        Refusal::Expired => (RefusalKey::Expired, 0),
        Refusal::AlreadyUsed => (RefusalKey::AlreadyUsed, 0),
    }
}

fn note_key(n: gate::Note) -> HandoverNote {
    match n {
        gate::Note::SsdNotSecureWipe => HandoverNote::SsdNotSecureWipe,
        gate::Note::DoesNotCancelSubscriptions => HandoverNote::DoesNotCancelSubscriptions,
        gate::Note::OthersStay => HandoverNote::OthersStay,
        gate::Note::CannotBeUndone => HandoverNote::CannotBeUndone,
        gate::Note::ClosesUndo => HandoverNote::ClosesUndo,
        gate::Note::SignInAsAnotherAdministrator => HandoverNote::SignInAsAnotherAdministrator,
        gate::Note::NotACertificate => HandoverNote::NotACertificate,
        gate::Note::EncryptFirstHelps => HandoverNote::EncryptFirstHelps,
    }
}

/// Everything the gate reads, gathered again for each preview and each confirmation.
struct GateNow {
    m: Move,
    checklist: Checklist,
    old: Option<store::OldNow>,
    journal: Journal,
    expected: gate::DeviceIdentity,
    device_now: gate::DeviceIdentity,
    accounts: Vec<gate::Account>,
}

fn gate_now<R: Runtime>(app: &AppHandle<R>, id: Option<String>) -> Result<GateNow, FinishProblem> {
    let m = open(app, id)?;
    // No journal, no gate: undo cannot be closed, so nothing is offered.
    let journal = journal(app).ok_or(FinishProblem::Failed)?;
    let (l, _) = ledger_of(&m.kept, Some(&journal));
    let old: Option<store::OldNow> = store::load(&m.folder, "old-now.json");
    let (checklist, _) = checklist(&m, &l, Some(&journal), old.as_ref());
    let expected = gate::DeviceIdentity {
        laptop: m.kept.move_id.source,
        drive: None,
    };
    let device_now = gate::DeviceIdentity {
        laptop: old
            .as_ref()
            .map(|o| o.record.source_laptop)
            .unwrap_or(m.kept.move_id.source),
        drive: None,
    };
    // The accounts known: the mover, and every other person the old laptop's latest scan found
    // with things of their own.
    let others = old.as_ref().map(|o| &o.others).unwrap_or(&m.kept.others);
    let mut accounts = vec![gate::Account {
        id: m.kept.mover.clone(),
        has_own_files: true,
    }];
    accounts.extend(others.iter().map(|p| gate::Account {
        id: p.account_id.clone(),
        has_own_files: p.items > 0,
    }));
    Ok(GateNow {
        m,
        checklist,
        old,
        journal,
        expected,
        device_now,
        accounts,
    })
}

impl GateNow {
    fn facts(&self) -> gate::Facts<'_> {
        gate::Facts {
            checklist: &self.checklist,
            old_laptop_now: self.old.as_ref().map(|o| &o.record),
            journal: &self.journal,
            expected: &self.expected,
            device_now: &self.device_now,
            // PCTwin holds nothing on the old laptop: only a method it ran itself would need it,
            // and none is qualified.
            write_exclusion: gate::WriteExclusion::CannotExclude,
            accounts: &self.accounts,
            mover: &self.m.kept.mover,
            qualified: &[],
            now: store::now(),
        }
    }

    /// Other people's account IDs shown by the names the mover already saw, when known.
    fn shown_name(&self, account: &str) -> String {
        let others = self
            .old
            .as_ref()
            .map(|o| &o.others)
            .unwrap_or(&self.m.kept.others);
        others
            .iter()
            .find(|p| p.account_id == account)
            .and_then(|p| p.name.clone())
            .unwrap_or_else(|| account.to_string())
    }

    fn system(&self) -> Option<SystemKey> {
        old_system(
            self.old
                .as_ref()
                .map(|o| &o.record)
                .unwrap_or(&self.m.kept.record),
        )
    }
}

fn used() -> &'static Mutex<gate::Used> {
    static U: OnceLock<Mutex<gate::Used>> = OnceLock::new();
    U.get_or_init(|| Mutex::new(gate::Used::default()))
}

/* ---------- Finish your setup ---------- */

fn task_id(what: SetupWhat, name: &str) -> String {
    let mut h = blake3::Hasher::new();
    h.update(b"pctwin/v1/setup-task\0");
    h.update(&serde_json::to_vec(&(what, name)).unwrap_or_default());
    hex(h.finalize().as_bytes())
}

/// What a line asks of the person, by its reason and notes (`None`: nothing).
fn setup_whats(status: ItemStatus, reason: Option<Reason>, notes: &[String]) -> Vec<SetupWhat> {
    let mut out = Vec::new();
    for n in notes {
        match n.as_str() {
            "sign-in" | "sites" => out.push(SetupWhat::SignIn),
            "licence-key" | "registration" => out.push(SetupWhat::LicenceKey),
            _ => {}
        }
    }
    if !status.is_done() {
        match reason {
            Some(Reason::SignIn) => out.push(SetupWhat::SignIn),
            Some(Reason::Activation) => out.push(SetupWhat::LicenceKey),
            Some(Reason::Permission) => out.push(SetupWhat::Permission),
            Some(Reason::Restart) => out.push(SetupWhat::Restart),
            Some(Reason::ManualInstall) => out.push(SetupWhat::DownloadFromMaker),
            Some(Reason::PartsMissing) => out.push(SetupWhat::PartsMissing),
            Some(Reason::FinishByHand) => out.push(SetupWhat::FinishByHand),
            _ => {}
        }
    }
    out.sort_by_key(|w| *w as u8);
    out.dedup();
    out
}

fn setup_view(m: &Move) -> SetupView {
    let marks: store::Marks = store::load(&m.folder, "setup.json").unwrap_or_default();
    let mut tasks = Vec::new();
    let mut seen = BTreeSet::new();
    let mut push = |what: SetupWhat, name: String, machine: bool| {
        let id = task_id(what, &name);
        if !seen.insert(id.clone()) {
            return;
        }
        let mark = if machine {
            SetupMark::MachineChecked
        } else if marks.done.contains_key(&id) {
            SetupMark::YouDidIt
        } else {
            SetupMark::Open
        };
        tasks.push(SetupTask {
            id,
            what,
            name,
            mark,
        });
    };
    for x in &m.kept.lines {
        for what in setup_whats(x.status, x.reason, &x.notes) {
            push(what, x.name.clone(), false);
        }
    }
    // Things tied to the old laptop (passkeys, authenticator apps): set up again.
    for i in &m.kept.record.items {
        if i.inclusion
            == (pctwin_record::Inclusion::LeftOut {
                reason: pctwin_record::LeftOutReason::DeviceBound,
            })
            && store::mover_only_owner(&i.owner, &m.kept.mover)
        {
            let name = i
                .path
                .parts()
                .iter()
                .map(|p| p.display())
                .collect::<Vec<_>>()
                .join("/");
            push(SetupWhat::SetUpAgain, name, false);
        }
    }
    let saved = store::load::<Vec<(String, String)>>(&m.folder, "setups.json")
        .unwrap_or_default()
        .into_iter()
        .map(|(history_id, when)| SavedSetupView { history_id, when })
        .collect();
    SetupView {
        id: m.kept.history_id.clone(),
        tasks,
        saved,
    }
}

/* ---------- Clean-up ---------- */

use pctwin_readiness::cleanup::{self, Entry as CleanEntry, Ownership as CleanOwner};

/// PCTwin's own temporary folders on this laptop: the ones it records making (`made-temp.json`
/// in its data folder), each still a real folder (never a link) with the name recorded. Anything
/// in the system's temporary folder that only looks like PCTwin's is listed and kept.
fn found_cleanup<R: Runtime>(app: &AppHandle<R>) -> Vec<CleanEntry> {
    use tauri::Manager;
    let recorded: BTreeSet<PathBuf> = app
        .path()
        .app_data_dir()
        .ok()
        .and_then(|d| store::load::<Vec<PathBuf>>(&d, "made-temp.json"))
        .unwrap_or_default()
        .into_iter()
        .collect();
    let busy = crate::finish_cleanup::busy(app);
    let mut out = Vec::new();
    let temp = std::env::temp_dir();
    for e in std::fs::read_dir(&temp).into_iter().flatten().flatten() {
        let name = e.file_name().to_string_lossy().into_owned();
        if !crate::finish_cleanup::looks_ours(&name) {
            continue;
        }
        let path = e.path();
        let Ok(meta) = std::fs::symlink_metadata(&path) else {
            continue;
        };
        let real = meta.is_dir() && !meta.file_type().is_symlink();
        out.push(CleanEntry {
            bytes: crate::finish_cleanup::size_of(&path),
            what: cleanup::What::TemporaryFiles,
            ownership: if real && recorded.contains(&path) {
                CleanOwner::RecordedByPctwin
            } else {
                CleanOwner::LooksTemporary
            },
            item: None,
            in_use: busy,
            name,
        });
    }
    out.sort_by(|a, b| a.name.cmp(&b.name));
    out
}

fn clean_what(w: CleanupWhat) -> cleanup::What {
    match w {
        CleanupWhat::TemporaryFiles => cleanup::What::TemporaryFiles,
        CleanupWhat::ModelAndCache => cleanup::What::ModelAndCache,
    }
}

fn entry_view(e: &CleanEntry) -> CleanupEntry {
    CleanupEntry {
        name: e.name.clone(),
        bytes: e.bytes,
    }
}

/* ---------- Report ---------- */

fn disclosure(d: DisclosureIn) -> pctwin_readiness::report::Disclosure {
    pctwin_readiness::report::Disclosure {
        file_names: d.file_names,
        app_names: d.app_names,
        account_id: d.account_id,
    }
}

fn share<R: Runtime>(
    app: &AppHandle<R>,
    id: Option<String>,
    d: DisclosureIn,
) -> Result<(Move, pctwin_readiness::report::ShareReport, Headline), FinishProblem> {
    let m = open(app, id)?;
    let j = journal(app);
    let (l, _) = ledger_of(&m.kept, j.as_ref());
    let list: Option<Checklist> = store::load(&m.folder, "goodbye.json");
    let head = headline(&l, &m.kept, list.as_ref());
    let summary = pctwin_readiness::checks::summarize(&l, &[]);
    let r = pctwin_readiness::report::share_preview(
        &l,
        summary,
        &[],
        list.as_ref(),
        &m.kept.mover,
        disclosure(d),
    );
    Ok((m, r, head))
}

/* ---------- Commands ---------- */

/// R01 and the after-move check, for the move `id` (none: the newest).
#[tauri::command]
pub fn finish_overview<R: Runtime>(
    app: AppHandle<R>,
    id: Option<String>,
) -> Result<FinishView, FinishProblem> {
    let m = open(&app, id)?;
    let j = journal(&app);
    let (l, refused) = ledger_of(&m.kept, j.as_ref());
    let list: Option<Checklist> = store::load(&m.folder, "goodbye.json");
    let old: Option<store::OldNow> = store::load(&m.folder, "old-now.json");
    let mut notes = vec![FinishNote::ParentalControls];
    if !l.found_everything() {
        notes.push(FinishNote::NotEverywhere);
    }
    if refused > 0 {
        notes.push(FinishNote::ResultsRefused);
    }
    if undo_open(j.as_ref()) {
        notes.push(FinishNote::UndoOpen);
    }
    let open_items: Vec<ItemView> = l
        .lines()
        .filter(|(id, line)| line.mine && !matches!(l.standing(**id), Some(Standing::Done(_))))
        .filter_map(|(id, _)| item_view(&l, *id))
        .collect();
    let more = (open_items.len() as u64).saturating_sub(MAX_LINES as u64);
    Ok(FinishView {
        id: m.kept.history_id.clone(),
        headline: headline(&l, &m.kept, list.as_ref()),
        tally: tally_view(l.tally()),
        others: m.kept.others.iter().map(|p| p.items).sum(),
        after_move: after_move(&l, &m.kept),
        notes,
        not_finished: open_items.into_iter().take(MAX_LINES).collect(),
        more,
        search: search_view(&l),
        old_laptop_checked_at: old.map(|o| o.at),
    })
}

/// The before-wipe list (R08), built again from what is known now and saved on this laptop.
#[tauri::command]
pub fn goodbye_list<R: Runtime>(
    app: AppHandle<R>,
    id: Option<String>,
) -> Result<GoodbyeView, FinishProblem> {
    goodbye_now(&app, id, |_| Ok(()))
}

/// The person's answer to one thing on the list, bound to what was shown (`shown`).
#[tauri::command]
pub fn goodbye_answer<R: Runtime>(
    app: AppHandle<R>,
    id: Option<String>,
    exception: String,
    shown: String,
    answer: GoodbyeAnswer,
) -> Result<GoodbyeView, FinishProblem> {
    let (Some(e), Some(s)) = (unhex32(&exception), unhex32(&shown)) else {
        return Err(FinishProblem::Changed);
    };
    goodbye_now(&app, id, |c| {
        let at = store::now();
        match answer {
            GoodbyeAnswer::AcceptLoss => c.accept(&e, s, at),
            GoodbyeAnswer::DidIt => c.person_did_it(&e, s, at),
        }
        .map_err(checklist_problem)
    })
}

/// The date until which the old laptop is kept untouched (seconds since 1970).
#[tauri::command]
pub fn goodbye_keep_until<R: Runtime>(
    app: AppHandle<R>,
    id: Option<String>,
    at: u64,
) -> Result<GoodbyeView, FinishProblem> {
    goodbye_now(&app, id, |c| {
        c.choose_keep_until(at, store::now())
            .map_err(checklist_problem)
    })
}

/// Readies the next pairing with the old laptop to look at it again for this move: its fresh list
/// comes as a [`DeltaView`] on [`DELTA_EVENT`]. Then the screens pair and start as for a move.
#[tauri::command]
pub fn catchup_arm<R: Runtime>(
    app: AppHandle<R>,
    id: Option<String>,
    purpose: CatchupPurpose,
) -> Result<(), FinishProblem> {
    let m = open(&app, id)?;
    crate::finish_catchup::arm(&m.kept, m.folder.clone(), purpose)
}

/// Approves the reviewed changes (`digest` of the [`DeltaView`] shown), with the person's choices:
/// only those items are sent.
#[tauri::command]
pub fn catchup_approve<R: Runtime>(
    app: AppHandle<R>,
    id: Option<String>,
    digest: String,
    choices: Vec<ChoiceIn>,
) -> Result<(), FinishProblem> {
    let m = open(&app, id)?;
    let seen = unhex32(&digest).ok_or(FinishProblem::Changed)?;
    crate::finish_catchup::approve(&app, &m.kept, seen, &choices)
}

/// What the person would confirm for one handover action, or why it is not offered.
#[tauri::command]
pub fn handover_preview<R: Runtime>(
    app: AppHandle<R>,
    id: Option<String>,
    action: HandoverAction,
) -> Result<HandoverView, FinishProblem> {
    let g = gate_now(&app, id)?;
    Ok(match gate::preview(action_of(action), &g.facts()) {
        Ok(p) => HandoverView::Ready {
            action,
            digest: hex(&p.digest()),
            removes: p.removes.iter().map(|a| g.shown_name(a)).collect(),
            keeps: p.keeps.iter().map(|a| g.shown_name(a)).collect(),
            notes: p.notes.iter().map(|n| note_key(*n)).collect(),
            route: route(action, g.system()),
        },
        Err(r) => {
            let (why, open) = refusal_key(&r);
            HandoverView::Refused { action, why, open }
        }
    })
}

/// The person's fresh, separate confirmation of exactly the preview with `digest`: everything is
/// checked again, undo is closed for good, and the system's own route is next (on the old laptop).
#[tauri::command]
pub fn handover_confirm<R: Runtime>(
    app: AppHandle<R>,
    id: Option<String>,
    action: HandoverAction,
    digest: String,
) -> Result<HandoverStarted, FinishProblem> {
    let seen = unhex32(&digest).ok_or(FinishProblem::Changed)?;
    let g = gate_now(&app, id)?;
    let facts = g.facts();
    let mut nonce = [0u8; 16];
    getrandom::fill(&mut nonce).map_err(|_| FinishProblem::Failed)?;
    let c = gate::confirm(action_of(action), seen, facts.now, nonce);
    let mut used = used().lock().map_err(|_| FinishProblem::Failed)?;
    let request =
        gate::request(action_of(action), &facts, Some(&c), &mut used).map_err(|r| match r {
            Refusal::ChangedSinceConfirmed | Refusal::WrongConfirmation | Refusal::Expired => {
                FinishProblem::Changed
            }
            _ => FinishProblem::NotAllowed,
        })?;
    let mut book: store::Handovers = store::load(&g.m.folder, "handover.json").unwrap_or_default();
    book.entries.push(store::HandoverEntry {
        action: request.action,
        confirmed_at: request.confirmed_at,
        result: None,
    });
    // Undo is closed whether or not this is written: the journal is the record of that.
    let _ = store::save(&g.m.folder, "handover.json", &book);
    let route = route(action, g.system());
    Ok(HandoverStarted {
        action,
        undo_closed: true,
        opened: false,
        route,
    })
}

/// Records what the person says happened after the system's route (their word, never a
/// certificate). Only for an action they confirmed.
#[tauri::command]
pub fn handover_result<R: Runtime>(
    app: AppHandle<R>,
    id: Option<String>,
    action: HandoverAction,
    says: PersonSays,
) -> Result<HandoverRecorded, FinishProblem> {
    let m = open(&app, id)?;
    let mut book: store::Handovers = store::load(&m.folder, "handover.json").unwrap_or_default();
    let entry = book
        .entries
        .iter_mut()
        .rev()
        .find(|e| e.action == action_of(action))
        .ok_or(FinishProblem::NotAllowed)?;
    let result = gate::ActionResult::from_person(match says {
        PersonSays::Done => gate::PersonSays::Done,
        PersonSays::Partly => gate::PersonSays::Partly,
        PersonSays::CancelledPartWay => gate::PersonSays::CancelledPartWay,
        PersonSays::NotStarted => gate::PersonSays::NotStarted,
        PersonSays::CannotTell => gate::PersonSays::CannotTell,
    });
    let at = store::now();
    entry.result = Some((result, at));
    store::save(&m.folder, "handover.json", &book).map_err(|_| FinishProblem::Failed)?;
    Ok(HandoverRecorded {
        action,
        outcome: match result {
            gate::ActionResult::PersonSaidDone => OutcomeKey::PersonSaysDone,
            gate::ActionResult::Partial => OutcomeKey::Partial,
            gate::ActionResult::Interrupted => OutcomeKey::Interrupted,
            gate::ActionResult::CancelledAfterSomeEffects => OutcomeKey::CancelledAfterSomeEffects,
            gate::ActionResult::NotStarted => OutcomeKey::NotStarted,
            gate::ActionResult::Inconclusive => OutcomeKey::Inconclusive,
        },
        at,
    })
}

/// The full connection doctor on this laptop, with the phone-hotspot fallback.
#[tauri::command]
pub async fn doctor_run(role: DoctorRole) -> Result<DoctorView, FinishProblem> {
    tauri::async_runtime::spawn_blocking(move || crate::finish_doctor::run(role))
        .await
        .map_err(|_| FinishProblem::Failed)
}

/// The finish-your-setup checklist (R03).
#[tauri::command]
pub fn setup_checklist<R: Runtime>(
    app: AppHandle<R>,
    id: Option<String>,
) -> Result<SetupView, FinishProblem> {
    let m = open(&app, id)?;
    Ok(setup_view(&m))
}

/// The person marks one task done (or not done again). PCTwin's own checks are never theirs to
/// change.
#[tauri::command]
pub fn setup_mark<R: Runtime>(
    app: AppHandle<R>,
    id: Option<String>,
    task: String,
    done: bool,
) -> Result<SetupView, FinishProblem> {
    let m = open(&app, id)?;
    let view = setup_view(&m);
    let t = view
        .tasks
        .iter()
        .find(|t| t.id == task)
        .ok_or(FinishProblem::Changed)?;
    if t.mark == SetupMark::MachineChecked {
        return Err(FinishProblem::NotAllowed);
    }
    let mut marks: store::Marks = store::load(&m.folder, "setup.json").unwrap_or_default();
    if done {
        marks.done.insert(task, store::now());
    } else {
        marks.done.remove(&task);
    }
    store::save(&m.folder, "setup.json", &marks).map_err(|_| FinishProblem::Failed)?;
    Ok(setup_view(&m))
}

/// Saves the setup profile and app list (R04) in My PCTwin history: which apps and which settings
/// groups, with their results. Never files, and never anything secret.
#[tauri::command]
pub fn setup_save<R: Runtime>(
    app: AppHandle<R>,
    id: Option<String>,
) -> Result<SavedSetup, FinishProblem> {
    let m = open(&app, id)?;
    let apps: Vec<&KeptLine> = m
        .kept
        .lines
        .iter()
        .filter(|x| x.kind == LineKind::App)
        .collect();
    let settings: Vec<&KeptLine> = m
        .kept
        .lines
        .iter()
        .filter(|x| x.kind == LineKind::Settings)
        .collect();
    let profile = crate::history::Kept::Setup {
        of: m.kept.history_id.clone(),
        apps: apps
            .iter()
            .map(|x| crate::history::SetupLine::of(x))
            .collect(),
        settings: settings
            .iter()
            .map(|x| crate::history::SetupLine::of(x))
            .collect(),
    };
    let history_id =
        crate::history::add(&app, "setup", (None, None), None, "finished", 0, &profile)
            .ok_or(FinishProblem::Failed)?;
    let mut saved: Vec<(String, String)> =
        store::load(&m.folder, "setups.json").unwrap_or_default();
    saved.push((history_id.clone(), crate::history::rfc3339(store::now())));
    store::save(&m.folder, "setups.json", &saved).map_err(|_| FinishProblem::Failed)?;
    Ok(SavedSetup {
        history_id,
        apps: apps.len() as u64,
        settings: settings.iter().map(|x| x.count).sum(),
    })
}

/// What undo would do for the move (R05). Undo itself is `fullmove_undo` with `undo_id` (R06).
#[tauri::command]
pub fn undo_preview<R: Runtime>(
    app: AppHandle<R>,
    id: Option<String>,
) -> Result<UndoPreviewView, FinishProblem> {
    let m = open(&app, id)?;
    let j = journal(&app);
    let (l, _) = ledger_of(&m.kept, j.as_ref());
    let book = m
        .kept
        .undo_id
        .as_deref()
        .and_then(|u| crate::fullmove::book_of(&app, u));
    let files = l
        .lines()
        .filter(|(id, line)| line.mine && matches!(l.standing(**id), Some(Standing::Done(_))))
        .count() as u64;
    Ok(UndoPreviewView {
        id: m.kept.history_id.clone(),
        undo_id: book.as_ref().map(|b| b.id()),
        apps_removed: book
            .as_ref()
            .map(|b| b.removable.iter().map(|a| a.name.clone()).collect())
            .unwrap_or_default(),
        apps_yours: book.as_ref().map(|b| b.apps.clone()).unwrap_or_default(),
        settings: book.as_ref().map(|b| b.book.len() as u64).unwrap_or(0),
        app_data: book
            .as_ref()
            .map(|b| b.landed.iter().map(|(n, _)| n.clone()).collect())
            .unwrap_or_default(),
        files,
        open: undo_open(j.as_ref()),
    })
}

/// What one clean-up question would remove on this laptop (R07).
#[tauri::command]
pub fn cleanup_preview<R: Runtime>(
    app: AppHandle<R>,
    what: CleanupWhat,
) -> Result<CleanupView, FinishProblem> {
    let plan = cleanup::Plan::of(clean_what(what), &found_cleanup(&app));
    Ok(CleanupView {
        what,
        digest: hex(&plan.digest()),
        removes: plan.removes.iter().map(entry_view).collect(),
        keeps: plan.keeps.iter().map(entry_view).collect(),
    })
}

/// Removes exactly the list with `digest`, checked again now: each folder only while it is still
/// the real folder PCTwin recorded making, never through a link.
#[tauri::command]
pub fn cleanup_confirm<R: Runtime>(
    app: AppHandle<R>,
    what: CleanupWhat,
    digest: String,
) -> Result<CleanupDone, FinishProblem> {
    let seen = unhex32(&digest).ok_or(FinishProblem::Changed)?;
    let found = found_cleanup(&app);
    let removes = cleanup::approve(clean_what(what), &found, seen).map_err(|e| match e {
        cleanup::CleanupError::Changed => FinishProblem::Changed,
        cleanup::CleanupError::Nothing => FinishProblem::NotAllowed,
    })?;
    let (removed, failed) = crate::finish_cleanup::remove(&app, &removes);
    let kept = (found.iter().filter(|e| e.what == clean_what(what)).count() as u64)
        .saturating_sub(removed + failed);
    Ok(CleanupDone {
        what,
        removed,
        kept,
        failed,
    })
}

fn report_view(id: &str, r: &pctwin_readiness::report::ShareReport, head: Headline) -> ReportView {
    ReportView {
        id: id.to_string(),
        headline: head,
        tally: tally_view(r.tally),
        account_id: r.account_id.clone(),
        lines: r
            .lines
            .iter()
            .map(|x| ReportLineView {
                name: x.name.clone(),
                is_app: x.is_app,
                standing: standing_key(x.standing),
                reason: x.reason,
            })
            .collect(),
        others_not_finished: r.others_not_finished,
        goodbye: r.goodbye,
        withheld: WithheldView {
            names: r.withheld.names,
            secrets: r.withheld.secrets,
            account_ids: r.withheld.account_ids,
        },
        digest: hex(&r.digest()),
    }
}

/// The shareable report as it would leave the laptop, with what the person chose to include.
#[tauri::command]
pub fn report_preview<R: Runtime>(
    app: AppHandle<R>,
    id: Option<String>,
    disclosure: DisclosureIn,
) -> Result<ReportView, FinishProblem> {
    let (m, r, head) = share(&app, id, disclosure)?;
    Ok(report_view(&m.kept.history_id, &r, head))
}

/// The most text a saved report may hold.
const MAX_REPORT_TEXT: usize = 1024 * 1024;

/// Saves the report the person read (`text`, in their language, made from the [`ReportView`]
/// with `digest`) where they choose. Refused if it holds anything the report kept out.
#[tauri::command]
pub async fn report_save<R: Runtime>(
    app: AppHandle<R>,
    id: Option<String>,
    disclosure: DisclosureIn,
    digest: String,
    text: String,
) -> Result<Option<String>, FinishProblem> {
    let seen = unhex32(&digest).ok_or(FinishProblem::Changed)?;
    if text.len() > MAX_REPORT_TEXT || text.chars().any(|c| c.is_control() && c != '\n') {
        return Err(FinishProblem::NotAllowed);
    }
    let (_, r, _) = share(&app, id, disclosure)?;
    if r.digest() != seen {
        return Err(FinishProblem::Changed);
    }
    pctwin_readiness::report::check_text(&r, &text).map_err(|_| FinishProblem::NotAllowed)?;
    let folder = tauri::async_runtime::spawn_blocking(|| rfd::FileDialog::new().pick_folder())
        .await
        .map_err(|_| FinishProblem::Failed)?;
    let Some(folder) = folder else {
        return Ok(None);
    };
    let path =
        crate::history::save_new_named(&folder, "PCTwin move report", "txt", text.as_bytes())
            .map_err(|_| FinishProblem::Failed)?;
    Ok(Some(path.display().to_string()))
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Every route in the catalogue is well formed, and every action has a route for every system
    /// and one for a system that cannot be told.
    #[test]
    fn the_routes_catalogue_covers_every_system_and_action() {
        assert!(!routes().is_empty(), "the catalogue reads");
        for action in HandoverAction::ALL {
            for system in [
                Some(SystemKey::Windows),
                Some(SystemKey::Macos),
                Some(SystemKey::Linux),
                None,
            ] {
                let r = route(*action, system);
                assert!(
                    !r.route.is_empty() && !r.steps.is_empty(),
                    "{action:?} {system:?}"
                );
                assert!(!r.can_open, "never opened on the new laptop");
                assert!(r.steps.iter().all(|s| is_key(s)), "{:?}", r.steps);
            }
        }
    }

    /// Every key the engine sends for the screens to put in words (each route step, each doctor
    /// finding, fix and probe) has its words in all seven languages.
    #[test]
    fn every_engine_word_key_has_words_in_every_language() {
        let mut keys: BTreeSet<String> = routes()
            .iter()
            .flat_map(|r| r.steps.iter().cloned())
            .collect();
        for c in [
            pctwin_doctor::Cause::NotConnected,
            pctwin_doctor::Cause::MacLocalNetworkDenied,
            pctwin_doctor::Cause::DiscoveryBlockedOnThisLaptop,
            pctwin_doctor::Cause::WindowsPublicNetwork(String::new()),
            pctwin_doctor::Cause::VpnOn(vec![]),
            pctwin_doctor::Cause::OldLaptopNotPairing,
            pctwin_doctor::Cause::DifferentNetworks,
            pctwin_doctor::Cause::GuestNetworkHidesDevices,
        ] {
            keys.insert(c.message_key().into());
            keys.insert(c.fix_key().into());
        }
        for k in [
            "doctor-probe-interfaces",
            "doctor-probe-hears-itself",
            "doctor-probe-local-network",
            "doctor-probe-network-profiles",
            "doctor-hotspot",
        ] {
            keys.insert(k.into());
        }
        let dir = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../src/locales");
        for lang in ["en", "es", "fr", "pt", "hi", "zh-CN", "ar"] {
            let text = std::fs::read_to_string(dir.join(lang).join("main.ftl")).unwrap();
            for k in &keys {
                assert!(
                    text.lines().any(|l| l.starts_with(&format!("{k} = "))),
                    "{lang} has no words for {k}"
                );
            }
        }
    }

    fn is_key(s: &str) -> bool {
        !s.is_empty()
            && s.split('-').all(|p| {
                !p.is_empty()
                    && p.chars()
                        .all(|c| c.is_ascii_lowercase() || c.is_ascii_digit())
            })
    }

    #[test]
    fn hex_round_trips_and_refuses_what_is_not_32_bytes() {
        let d = [7u8; 32];
        assert_eq!(unhex32(&hex(&d)), Some(d));
        assert_eq!(unhex32("00"), None);
        assert_eq!(unhex32(&"zz".repeat(32)), None);
        assert_eq!(unhex32(&format!("{}é", "0".repeat(62))), None);
    }

    /// The setup list asks for each step once, by what the line says.
    #[test]
    fn setup_steps_come_from_the_line_s_reason_and_notes() {
        let notes = vec!["sign-in".to_string(), "licence-key".to_string()];
        assert_eq!(
            setup_whats(ItemStatus::NeedsAction, Some(Reason::SignIn), &notes),
            [SetupWhat::SignIn, SetupWhat::LicenceKey]
        );
        assert_eq!(
            setup_whats(ItemStatus::Reinstalled, None, &[]),
            Vec::<SetupWhat>::new()
        );
        assert_eq!(
            setup_whats(ItemStatus::NeedsAction, Some(Reason::ManualInstall), &[]),
            [SetupWhat::DownloadFromMaker]
        );
    }

    /// The old laptop's system is told from its apps, never guessed when it is unclear.
    #[test]
    fn the_old_system_is_told_from_its_apps_only() {
        use pctwin_record::AppSystem as S;
        assert_eq!(system_of([].into_iter()), None);
        assert_eq!(system_of([S::Mac].into_iter()), Some(SystemKey::Macos));
        assert_eq!(system_of([S::Mac, S::Windows].into_iter()), None);
        assert_eq!(
            system_of([S::Windows, S::Windows, S::Mac].into_iter()),
            Some(SystemKey::Windows)
        );
        assert_eq!(system_of([S::Linux].into_iter()), Some(SystemKey::Linux));
    }

    /* ---------- The finishing views from a kept move (acceptance T27-001, T27-003, T27-011, T27-014) ---------- */

    use pctwin_record::{
        FolderRole, Inclusion, Item, ItemName, ItemPath, LaptopId, LeftOutReason, ManagedBy, Owner,
        Place, Portability, Storage,
    };

    const MOVER: &str = "1000";

    fn old() -> LaptopId {
        LaptopId::from_hex("00112233445566778899aabbccddeeff").unwrap()
    }

    fn item(name: &str, inclusion: Inclusion) -> Item {
        let owner = Owner::Person {
            account_id: MOVER.into(),
        };
        let place = Place {
            role: FolderRole::Documents,
            storage: Storage::SystemDrive,
        };
        let path = ItemPath::new(vec![ItemName::from_text(name)]).unwrap();
        Item {
            id: ItemId::derive(&old(), &owner, &place, &path),
            kind: ItemKind::File,
            owner,
            place,
            path,
            size_bytes: 10,
            modified_ns: Some(1),
            portability: Portability::Portable,
            managed_by: ManagedBy::Personal,
            download_bytes: None,
            inclusion,
        }
    }

    fn kept() -> Kept {
        let mut record = Record::new(old());
        record.items = vec![
            item("photo.jpg", Inclusion::Included),
            item("letter.docx", Inclusion::Included),
            item(
                "passkey",
                Inclusion::LeftOut {
                    reason: LeftOutReason::DeviceBound,
                },
            ),
        ];
        let ids: Vec<ItemId> = record.items.iter().map(|i| i.id).collect();
        Kept {
            format: store::FORMAT,
            history_id: "100-0".into(),
            move_id: MoveId {
                source: old(),
                target: LaptopId::from_hex("ffeeddccbbaa99887766554433221100").unwrap(),
                session: 1,
            },
            mover: MOVER.into(),
            started_at: 1_000,
            ended_at: 2_000,
            record,
            others: vec![store::OtherPerson {
                account_id: "1001".into(),
                name: Some("Bea".into()),
                items: 4,
            }],
            results: vec![
                KeptResult {
                    item: ids[0],
                    status: ItemStatus::Copied,
                    reason: None,
                },
                KeptResult {
                    item: ids[1],
                    status: ItemStatus::Failed,
                    reason: Some(Reason::Unreadable),
                },
            ],
            lines: vec![
                KeptLine {
                    kind: LineKind::App,
                    name: "Writer".into(),
                    count: 0,
                    status: ItemStatus::NeedsAction,
                    reason: Some(Reason::SignIn),
                    notes: vec!["sign-in".into()],
                },
                KeptLine {
                    kind: LineKind::Settings,
                    name: "Writer settings".into(),
                    count: 12,
                    status: ItemStatus::Converted,
                    reason: None,
                    notes: vec![],
                },
                KeptLine {
                    kind: LineKind::Special,
                    name: "Mail".into(),
                    count: 0,
                    status: ItemStatus::NeedsAction,
                    reason: Some(Reason::FinishByHand),
                    notes: vec![],
                },
            ],
            undo_id: None,
            searched: vec!["documents".into()],
        }
    }

    /// T27-001: with no journal to show it, a copy the move called "copied" is not done: the
    /// result is refused, never trusted on its word; a failure needs no proof.
    #[test]
    fn t27_001_a_copy_is_done_only_when_the_journal_shows_it() {
        let k = kept();
        let (l, refused) = ledger_of(&k, None);
        assert_eq!(refused, 1);
        let ids: Vec<ItemId> = k.record.items.iter().map(|i| i.id).collect();
        assert_eq!(l.standing(ids[0]), Some(Standing::NotTested));
        assert_eq!(l.standing(ids[1]), Some(Standing::Failed));
        assert_eq!(headline(&l, &k, None), Headline::FinishedWithTasks);
        // Nothing of the photo is counted on the new laptop.
        let groups = after_move(&l, &k);
        let photos = groups
            .iter()
            .find(|g| g.group == AfterKind::Photos)
            .unwrap();
        assert_eq!((photos.at_scan, photos.on_new_laptop), (1, 0));
        let settings = groups
            .iter()
            .find(|g| g.group == AfterKind::Settings)
            .unwrap();
        assert_eq!((settings.at_scan, settings.on_new_laptop), (12, 12));
        let apps = groups.iter().find(|g| g.group == AfterKind::Apps).unwrap();
        assert_eq!((apps.at_scan, apps.on_new_laptop), (1, 0));
    }

    /// The journal shows the photo copied: done, with the journal's own fingerprint as evidence.
    #[test]
    fn a_copy_the_journal_shows_is_done_with_its_fingerprint() {
        let mut k = kept();
        k.results.truncate(1);
        let photo = k.record.items[0].id;
        let probe =
            JournalProbe::from_moves(&[journal_entry(photo, [5; 32])], &BTreeSet::new(), old());
        let mut l = Ledger::from_record(&k.record, &k.mover, k.move_id, k.started_at);
        let r = Receipt {
            role: Role::Transfer,
            source: k.move_id.source,
            target: k.move_id.target,
            session: k.move_id.session,
            generation: l.generation(),
            item: photo,
            result: Some(ItemResult::done(ItemStatus::Copied).unwrap()),
            evidence: probe.fingerprint(photo).map(|fingerprint| {
                pctwin_readiness::ledger::Evidence::VerifiedTransfer { fingerprint }
            }),
            observed_at: k.ended_at,
        };
        l.accept(r, &JournalSees(&probe), 3_000).unwrap();
        assert_eq!(l.standing(photo), Some(Standing::Done(ItemStatus::Copied)));
    }

    fn journal_entry(item: ItemId, fp: [u8; 32]) -> pctwin_journal::Entry {
        use pctwin_journal::{Actor, Entry, Landed, Permission, PlannedWrite, Scope, State};
        Entry {
            id: 1,
            write: PlannedWrite {
                item,
                source_laptop: old(),
                scope: Scope([3; 16]),
                destination: "Documents".into(),
                path: "photo.jpg".into(),
                size: 10,
                actor: Actor {
                    acting_account: "2000".into(),
                    for_account: "2000".into(),
                    permission: Permission::OwnFolders,
                },
                block_size: 1 << 20,
                source_modified_ns: Some(1),
                place: None,
                source_file: None,
                partial_keep: Default::default(),
                download_mark: None,
            },
            state: State::Committed {
                final_path: "photo.jpg".into(),
                fingerprint: fp,
                landed: Landed {
                    size: 10,
                    modified_ns: Some(1),
                    file: None,
                },
            },
        }
    }

    /// T27-014, T27-011: an old laptop not checked again keeps the list unclear and cannot be
    /// accepted away; what is only on the old laptop is named, the mover's only; the special step
    /// and the passkey are the person's own to do.
    #[test]
    fn t27_011_014_the_list_names_what_is_only_on_the_old_laptop() {
        let dir = tempfile::tempdir().unwrap();
        let m = Move {
            folder: dir.path().join("100-0"),
            kept: kept(),
        };
        let (l, _) = ledger_of(&m.kept, None);
        let (c, saved) = checklist(&m, &l, None, None);
        assert!(saved, "saved on the new laptop");
        let v = goodbye_view(&m, &l, &c, None, None, saved);
        assert!(!v.clear);
        assert_eq!(v.old_laptop, OldLaptopView::NotChecked);
        let by = |k: ExceptionKey| v.exceptions.iter().find(|e| e.key == k);
        let only = by(ExceptionKey::OnlyOnOldLaptop).expect("listed");
        // The failed letter and the unconfirmed photo: both still only on the old laptop.
        assert_eq!(only.count, 2);
        assert!(only.items.iter().all(|i| !i.label.contains("bea")));
        assert!(only.can_accept && !only.can_mark_done);
        let gone = by(ExceptionKey::OldLaptopNotReachable).expect("listed");
        assert!(!gone.can_accept);
        let passkey = by(ExceptionKey::TiedToOldLaptop).expect("listed");
        assert!(passkey.can_mark_done);
        let mail = by(ExceptionKey::SpecialLookAtIt).expect("listed");
        assert_eq!(mail.names, ["Mail"]);
        assert!(mail.can_mark_done);
        // Every answer is bound to what is shown.
        assert!(v.exceptions.iter().all(|e| unhex32(&e.shown).is_some()));
        // Saved, and the person's answer carries over to the list built again.
        let id = c
            .exceptions
            .iter()
            .find(|e| e.kind == ExceptionKind::OnlyOnOldLaptop)
            .unwrap()
            .id;
        let mut c2 = c.clone();
        c2.accept(&id, c2.shown(&id).unwrap(), 2_500).unwrap();
        store::save(&m.folder, "goodbye.json", &c2).unwrap();
        let (c3, _) = checklist(&m, &l, None, None);
        let again = c3.exceptions.iter().find(|e| e.id == id).unwrap();
        assert_eq!(again.resolution, Resolution::AcceptedLoss { at: 2_500 });
    }

    /// The finish-your-setup list: each step once, the person's mark kept apart.
    #[test]
    fn the_setup_list_comes_from_the_lines_and_the_person_s_marks() {
        let dir = tempfile::tempdir().unwrap();
        let m = Move {
            folder: dir.path().join("100-0"),
            kept: kept(),
        };
        let v = setup_view(&m);
        let whats: Vec<(SetupWhat, &str)> =
            v.tasks.iter().map(|t| (t.what, t.name.as_str())).collect();
        assert!(whats.contains(&(SetupWhat::SignIn, "Writer")));
        assert!(whats.contains(&(SetupWhat::FinishByHand, "Mail")));
        assert!(whats.contains(&(SetupWhat::SetUpAgain, "passkey")));
        assert_eq!(
            v.tasks
                .iter()
                .filter(|t| t.what == SetupWhat::SignIn)
                .count(),
            1,
            "once, though its note and its reason both say so"
        );
        assert!(v.tasks.iter().all(|t| t.mark == SetupMark::Open));
        let id = v.tasks[0].id.clone();
        let marks = store::Marks {
            done: [(id.clone(), 9)].into(),
        };
        store::save(&m.folder, "setup.json", &marks).unwrap();
        let v = setup_view(&m);
        assert_eq!(
            v.tasks.iter().find(|t| t.id == id).unwrap().mark,
            SetupMark::YouDidIt
        );
    }

    /// The special steps of app data that still need the person, by what they ask.
    #[test]
    fn special_steps_ask_by_their_reason() {
        let k = kept();
        let s = special_steps(&k);
        assert_eq!(s.len(), 1);
        assert_eq!(
            (s[0].kind, s[0].what),
            (ExceptionKind::DoFirstOnOldLaptop, StepWhat::LookAtIt)
        );
        let mut k = kept();
        k.lines[2].reason = Some(Reason::SignIn);
        assert_eq!(special_steps(&k)[0].what, StepWhat::SignInAgain);
        k.lines[2].status = ItemStatus::Copied;
        assert!(special_steps(&k).is_empty());
    }
}
