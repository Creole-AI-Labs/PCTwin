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
        /// Everything picked is on the new laptop and opens, as far as PCTwin checked.
        ReadyForSelected,
        /// Some things need the person, or failed.
        FinishedWithTasks,
        /// Everything is on the new laptop, but a key or recovery code to open some of it is not
        /// checked there.
        DataPreservedAccessUnresolved,
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

    /// Only the person can do it, so their word settles it.
    pub fn person_can_do(self) -> bool {
        matches!(
            self,
            Self::DoFirstOnOldLaptop
                | Self::TiedToOldLaptop
                | Self::SpecialSignIn
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
    pub system: SystemKey,
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

/* ---------- Commands (stubs: the contract; each is made real in its own commit) ---------- */

/// R01 and the after-move check, for the move `id` (none: the newest).
#[tauri::command]
pub fn finish_overview<R: Runtime>(
    app: AppHandle<R>,
    id: Option<String>,
) -> Result<FinishView, FinishProblem> {
    let _ = (app, id);
    Err(FinishProblem::NoMove)
}

/// The before-wipe list (R08), built again from what is known now and saved on this laptop.
#[tauri::command]
pub fn goodbye_list<R: Runtime>(
    app: AppHandle<R>,
    id: Option<String>,
) -> Result<GoodbyeView, FinishProblem> {
    let _ = (app, id);
    Err(FinishProblem::NoMove)
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
    let _ = (app, id, exception, shown, answer);
    Err(FinishProblem::NoMove)
}

/// The date until which the old laptop is kept untouched (seconds since 1970).
#[tauri::command]
pub fn goodbye_keep_until<R: Runtime>(
    app: AppHandle<R>,
    id: Option<String>,
    at: u64,
) -> Result<GoodbyeView, FinishProblem> {
    let _ = (app, id, at);
    Err(FinishProblem::NoMove)
}

/// Readies the next pairing with the old laptop to look at it again for this move: its fresh list
/// comes as a [`DeltaView`] on [`DELTA_EVENT`]. Then the screens pair and start as for a move.
#[tauri::command]
pub fn catchup_arm<R: Runtime>(
    app: AppHandle<R>,
    id: Option<String>,
    purpose: CatchupPurpose,
) -> Result<(), FinishProblem> {
    let _ = (app, id, purpose);
    Err(FinishProblem::NoMove)
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
    let _ = (app, id, digest, choices);
    Err(FinishProblem::NoMove)
}

/// What the person would confirm for one handover action, or why it is not offered.
#[tauri::command]
pub fn handover_preview<R: Runtime>(
    app: AppHandle<R>,
    id: Option<String>,
    action: HandoverAction,
) -> Result<HandoverView, FinishProblem> {
    let _ = (app, id, action);
    Err(FinishProblem::NoMove)
}

/// The person's fresh, separate confirmation of exactly the preview with `digest`: everything is
/// checked again, undo is closed for good for an erase, and the system's own page opens.
#[tauri::command]
pub fn handover_confirm<R: Runtime>(
    app: AppHandle<R>,
    id: Option<String>,
    action: HandoverAction,
    digest: String,
) -> Result<HandoverStarted, FinishProblem> {
    let _ = (app, id, action, digest);
    Err(FinishProblem::NoMove)
}

/// Records what the person says happened after the system's route (their word, never a
/// certificate).
#[tauri::command]
pub fn handover_result<R: Runtime>(
    app: AppHandle<R>,
    id: Option<String>,
    action: HandoverAction,
    says: PersonSays,
) -> Result<HandoverRecorded, FinishProblem> {
    let _ = (app, id, action, says);
    Err(FinishProblem::NoMove)
}

/// The full connection doctor on this laptop, with the phone-hotspot fallback.
#[tauri::command]
pub async fn doctor_run(role: DoctorRole) -> Result<DoctorView, FinishProblem> {
    let _ = role;
    Err(FinishProblem::Failed)
}

/// The finish-your-setup checklist (R03).
#[tauri::command]
pub fn setup_checklist<R: Runtime>(
    app: AppHandle<R>,
    id: Option<String>,
) -> Result<SetupView, FinishProblem> {
    let _ = (app, id);
    Err(FinishProblem::NoMove)
}

/// The person marks one task done (or not done again).
#[tauri::command]
pub fn setup_mark<R: Runtime>(
    app: AppHandle<R>,
    id: Option<String>,
    task: String,
    done: bool,
) -> Result<SetupView, FinishProblem> {
    let _ = (app, id, task, done);
    Err(FinishProblem::NoMove)
}

/// Saves the setup profile and app list (R04) in My PCTwin history.
#[tauri::command]
pub fn setup_save<R: Runtime>(
    app: AppHandle<R>,
    id: Option<String>,
) -> Result<SavedSetup, FinishProblem> {
    let _ = (app, id);
    Err(FinishProblem::NoMove)
}

/// What undo would do for the move (R05). Undo itself is `fullmove_undo` with `undo_id` (R06).
#[tauri::command]
pub fn undo_preview<R: Runtime>(
    app: AppHandle<R>,
    id: Option<String>,
) -> Result<UndoPreviewView, FinishProblem> {
    let _ = (app, id);
    Err(FinishProblem::NoMove)
}

/// What one clean-up question would remove on this laptop (R07).
#[tauri::command]
pub fn cleanup_preview<R: Runtime>(
    app: AppHandle<R>,
    what: CleanupWhat,
) -> Result<CleanupView, FinishProblem> {
    let _ = (app, what);
    Err(FinishProblem::Failed)
}

/// Removes exactly the list with `digest`, checked again now.
#[tauri::command]
pub fn cleanup_confirm<R: Runtime>(
    app: AppHandle<R>,
    what: CleanupWhat,
    digest: String,
) -> Result<CleanupDone, FinishProblem> {
    let _ = (app, what, digest);
    Err(FinishProblem::Changed)
}

/// The shareable report as it would leave the laptop, with what the person chose to include.
#[tauri::command]
pub fn report_preview<R: Runtime>(
    app: AppHandle<R>,
    id: Option<String>,
    disclosure: DisclosureIn,
) -> Result<ReportView, FinishProblem> {
    let _ = (app, id, disclosure);
    Err(FinishProblem::NoMove)
}

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
    let _ = (app, id, disclosure, digest, text);
    Err(FinishProblem::NoMove)
}
