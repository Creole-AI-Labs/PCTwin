//! The evidence ledger (plan 2.7 section 3; Engineering Plan section 11, "evidence is separate
//! from operation labels"; acceptance T27-001 to T27-004, T27-027).
//!
//! One stable [`Line`] per item of the approved plan, so nothing can disappear between screens.
//! Each line keeps three things apart:
//!
//! - where the item stands in the move ([`Selection`]: selected, skipped by the person, or left
//!   out by the rules; and for selected work a [`Hold`]: deferred or not reachable);
//! - its one result word ([`ItemResult`]: copied, reinstalled, converted, needs-action,
//!   skipped-by-user or failed), with every earlier result kept in its history;
//! - the evidence real observations gave, each of its own kind ([`Evidence`]): a copy checked
//!   against the original does not say the app can open it, a person's check of a sample does not
//!   say every file works.
//!
//! Results arrive as [`Receipt`]s from the part of PCTwin that did the work. A receipt is
//! refused when it names another laptop, an older session, another plan generation, an item not
//! in the move, or a result or evidence its part of PCTwin cannot give. A "done" or a piece of
//! evidence is also checked against the real state with a [`Probe`] (the journal on disk, the
//! 2.6 scheduler's own record): if the real state does not show it, or cannot tell, it is
//! refused and nothing is recorded. A failure needs no checking: erring toward "not done" is
//! always safe. Inside one program these are plain checks, not signatures (ADOPTION change 1).

use std::collections::{BTreeMap, BTreeSet};

use pctwin_journal::{
    Entry, Journal, JournalError, Scope as JournalScope, State, Undo, UndoOutcome,
};
use pctwin_orchestrate::jobs::{ChildJob, JobId, Scheduler, Scope, Worker};
use pctwin_record::{Inclusion, ItemId, ItemKind, LaptopId, LeftOutReason, Owner, Record};
use pctwin_types::{ItemResult, ItemStatus, Reason};
use serde::{Deserialize, Serialize};

/// Most refused receipts kept to show in "more details"; older ones are dropped first.
pub const MAX_REFUSED: usize = 1_000;
/// Most pieces of evidence one item keeps.
pub const MAX_EVIDENCE: usize = 64;
/// Most earlier results one item keeps.
pub const MAX_HISTORY: usize = 64;

/// Which move a ledger is for: the old laptop, the new laptop and the session between them.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct MoveId {
    pub source: LaptopId,
    pub target: LaptopId,
    pub session: u64,
}

/// The part of PCTwin a receipt comes from.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum Role {
    /// Moving files (the transfer and its journal).
    Transfer,
    /// Installing apps (2.1).
    Install,
    /// Restoring settings (2.4).
    Settings,
    /// Restoring app data, keys and exports (2.5).
    Data,
    /// The person, through a screen's checked intent.
    Person,
    /// This part's own read-only checks (2.7).
    Readiness,
    /// The permission broker (2.2).
    Broker,
}

impl From<Worker> for Role {
    fn from(w: Worker) -> Self {
        match w {
            Worker::Install => Role::Install,
            Worker::Settings => Role::Settings,
            Worker::Data => Role::Data,
        }
    }
}

/// How much of something a check covered.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case", tag = "coverage")]
pub enum Coverage {
    /// All of it.
    Whole,
    /// Only `checked` of `of`: a sample, never the whole.
    Sample { checked: u64, of: u64 },
}

/// A sample that cannot be.
#[derive(Debug, Clone, Copy, PartialEq, Eq, thiserror::Error)]
#[error("a sample must check at least one and at most all")]
pub struct BadSample;

impl Coverage {
    /// `checked` of `of`: the whole when every one was checked.
    pub fn sample(checked: u64, of: u64) -> Result<Self, BadSample> {
        if checked == 0 || checked > of {
            return Err(BadSample);
        }
        if checked == of {
            return Ok(Coverage::Whole);
        }
        Ok(Coverage::Sample { checked, of })
    }

    pub fn is_whole(&self) -> bool {
        matches!(self, Coverage::Whole)
    }

    fn valid(&self) -> bool {
        match *self {
            Coverage::Whole => true,
            Coverage::Sample { checked, of } => checked > 0 && checked < of,
        }
    }

    /// The plain words ("checked 12 of 10000").
    pub fn words(&self) -> String {
        match self {
            Coverage::Whole => "checked all".to_string(),
            Coverage::Sample { checked, of } => format!("checked {checked} of {of}"),
        }
    }
}

/// What one observation showed (plan 2.7 section 3). Each says what it means and no more.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case", tag = "kind")]
pub enum Evidence {
    /// The selected item was captured from the old laptop as this plan generation. It does not say
    /// every place on the old laptop was searched.
    Captured { generation: u64 },
    /// The copy on the new laptop matches what was read, by this fingerprint. It does not say an
    /// app can open it.
    VerifiedTransfer { fingerprint: [u8; 32] },
    /// This exact package and version is installed. It does not say the licence, the sign-in or
    /// the person's work moved.
    Installed { package: String, version: String },
    /// This adapter's checks passed. It does not say every document or project works.
    Imported { adapter: String, coverage: Coverage },
    /// The person checked this task themselves. It is not a machine check of all the data.
    PersonConfirmed { task: String, coverage: Coverage },
    /// This exact key, export or tool was checked to open the copy. It says nothing about what
    /// was never captured or is held only online.
    RecoveryChecked { what: String, coverage: Coverage },
    /// Seen only on the old laptop, within these searched places. It does not say there is no
    /// other copy anywhere.
    SourceOnly { places: Vec<String> },
    /// This clean-up step was observed as done. It is not proof a drive was physically erased.
    CleanupReceipt { operation: String },
}

/// The kinds of [`Evidence`], by name.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum EvidenceKind {
    Captured,
    VerifiedTransfer,
    Installed,
    Imported,
    PersonConfirmed,
    RecoveryChecked,
    SourceOnly,
    CleanupReceipt,
}

impl EvidenceKind {
    /// Every kind, in the plan's order.
    pub const ALL: [EvidenceKind; 8] = [
        EvidenceKind::Captured,
        EvidenceKind::VerifiedTransfer,
        EvidenceKind::Installed,
        EvidenceKind::Imported,
        EvidenceKind::PersonConfirmed,
        EvidenceKind::RecoveryChecked,
        EvidenceKind::SourceOnly,
        EvidenceKind::CleanupReceipt,
    ];
}

impl Evidence {
    pub fn kind(&self) -> EvidenceKind {
        match self {
            Evidence::Captured { .. } => EvidenceKind::Captured,
            Evidence::VerifiedTransfer { .. } => EvidenceKind::VerifiedTransfer,
            Evidence::Installed { .. } => EvidenceKind::Installed,
            Evidence::Imported { .. } => EvidenceKind::Imported,
            Evidence::PersonConfirmed { .. } => EvidenceKind::PersonConfirmed,
            Evidence::RecoveryChecked { .. } => EvidenceKind::RecoveryChecked,
            Evidence::SourceOnly { .. } => EvidenceKind::SourceOnly,
            Evidence::CleanupReceipt { .. } => EvidenceKind::CleanupReceipt,
        }
    }

    /// How much it covered, for the kinds that can be a sample.
    pub fn coverage(&self) -> Option<Coverage> {
        match self {
            Evidence::Imported { coverage, .. }
            | Evidence::PersonConfirmed { coverage, .. }
            | Evidence::RecoveryChecked { coverage, .. } => Some(*coverage),
            _ => None,
        }
    }
}

/// What a part of PCTwin hands over about one item.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Receipt {
    pub role: Role,
    pub source: LaptopId,
    pub target: LaptopId,
    pub session: u64,
    /// The plan generation it worked to.
    pub generation: u64,
    pub item: ItemId,
    /// The item's result word, if this receipt gives one.
    pub result: Option<ItemResult>,
    /// What was observed, if anything.
    pub evidence: Option<Evidence>,
    /// When it was observed (seconds since 1970).
    pub observed_at: u64,
}

/// What the real state shows of a receipt's claim.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Observed {
    Matches,
    Differs,
    CannotTell,
}

/// Looks at the real state behind a receipt.
pub trait Probe {
    fn observe(&self, receipt: &Receipt) -> Observed;
}

/// Why a receipt was refused. Nothing of a refused receipt is recorded but the refusal.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, thiserror::Error)]
#[serde(rename_all = "kebab-case")]
pub enum Refused {
    #[error("it is about another laptop")]
    WrongDevice,
    #[error("it is from another session")]
    OldSession,
    #[error("it is for another version of the plan")]
    StalePlan,
    #[error("that item is not in this move")]
    UnknownItem,
    #[error("that item was not selected to move")]
    NotSelected,
    #[error("that part of PCTwin cannot say this")]
    WrongRole,
    #[error("the evidence is not well formed")]
    BadEvidence,
    #[error("it says nothing")]
    Empty,
    #[error("its time is before the move or after now")]
    BadTime,
    #[error("the new laptop does not show it")]
    NotWhatIsThere,
    #[error("the new laptop's state could not confirm it")]
    CouldNotConfirm,
    #[error("this item has too much evidence already")]
    TooMuch,
}

/// A refused receipt, kept for "more details".
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct RefusedReceipt {
    pub item: ItemId,
    pub by: Role,
    pub why: Refused,
}

/// Whether an item is part of the move.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case", tag = "selection")]
pub enum Selection {
    Selected,
    /// The person chose to leave it out.
    SkippedByUser,
    /// Left out by the rules (work-managed, tied to the old laptop, unreadable, ...).
    Excluded {
        why: LeftOutReason,
    },
}

/// Why selected work waits.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum DeferWhy {
    WaitingForAnotherStep,
    PersonChoseLater,
    WaitingForPerson,
}

/// Why selected work cannot be reached.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum InaccessibleWhy {
    OldLaptopNotConnected,
    DriveNotConnected,
    Locked,
    Unreadable,
}

/// Selected work that has not been tried yet, and why.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case", tag = "hold")]
pub enum Hold {
    Deferred { why: DeferWhy },
    Inaccessible { why: InaccessibleWhy },
}

/// One observation of evidence.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Observation {
    pub evidence: Evidence,
    pub by: Role,
    pub at: u64,
}

/// One result word given, kept when a later one replaces it.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Attempt {
    pub result: ItemResult,
    pub by: Role,
    pub at: u64,
}

/// The person accepted losing this item, after the exact warning with this digest.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct AcceptedLoss {
    pub at: u64,
    pub shown: [u8; 32],
}

/// One item's line.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Line {
    pub kind: ItemKind,
    /// Where it was, as the person knows it (`Letters/ada.txt`), or the app's or setting's name.
    pub label: String,
    /// The mover's own or shared: shown by name to the mover. Another person's item is only ever
    /// counted.
    pub mine: bool,
    pub selection: Selection,
    pub result: Option<Attempt>,
    pub hold: Option<Hold>,
    pub evidence: Vec<Observation>,
    /// Earlier results, oldest first.
    pub history: Vec<Attempt>,
    /// It cannot be opened without a key or recovery material.
    pub needs_key: bool,
    pub accepted_loss: Option<AcceptedLoss>,
}

/// Where an item stands, one of these each.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case", tag = "standing", content = "status")]
pub enum Standing {
    /// Copied, reinstalled or converted, and confirmed.
    Done(ItemStatus),
    NeedsAction,
    Failed,
    Deferred,
    Inaccessible,
    /// Selected, with no result yet.
    NotTested,
    SkippedByUser,
    Excluded,
}

/// Whether a moved item can be opened on the new laptop, as far as keys go.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum Access {
    /// On the new laptop, with no key known to be missing.
    Usable,
    /// On the new laptop, but the key or recovery material to open it is not checked there.
    PreservedKeyUnresolved,
    /// Not on the new laptop.
    NotOnNewLaptop,
}

/// Counts, every item in exactly one of the standing counts.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct Tally {
    pub selected: u64,
    pub done: u64,
    pub needs_action: u64,
    pub failed: u64,
    pub deferred: u64,
    pub inaccessible: u64,
    pub not_tested: u64,
    pub skipped: u64,
    pub excluded: u64,
    /// Of the items not done, how many the person accepted losing (also counted above).
    pub accepted_loss: u64,
}

/// Why a ledger change was refused.
#[derive(Debug, Clone, Copy, PartialEq, Eq, thiserror::Error)]
pub enum LedgerError {
    #[error("that item is not in this move")]
    UnknownItem,
    #[error("that item was not selected to move")]
    NotSelected,
    #[error("that item is already done")]
    AlreadyDone,
    #[error("the warning agreed to is not the one for this item now")]
    NotTheWarningShown,
    #[error("that plan is not the next one for this old laptop")]
    NotTheNextPlan,
}

/// Places searched on the old laptop, and places known and not searched.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct SearchScope {
    pub searched: BTreeSet<String>,
    pub not_searched: BTreeSet<String>,
}

/// The evidence ledger of one move.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Ledger {
    id: MoveId,
    generation: u64,
    started_at: u64,
    lines: BTreeMap<ItemId, Line>,
    search: SearchScope,
    refused: Vec<RefusedReceipt>,
}

/// An item's label: its path as the person knows it.
fn label_of(item: &pctwin_record::Item) -> String {
    item.path
        .parts()
        .iter()
        .map(|p| p.display())
        .collect::<Vec<_>>()
        .join("/")
}

fn line_of(item: &pctwin_record::Item, mover: &str) -> Line {
    let mine = match &item.owner {
        Owner::Person { account_id } => account_id == mover,
        Owner::Shared | Owner::System => true,
    };
    let selection = match item.inclusion {
        Inclusion::Included => Selection::Selected,
        Inclusion::LeftOut {
            reason: LeftOutReason::ChosenByUser,
        } => Selection::SkippedByUser,
        Inclusion::LeftOut { reason } => Selection::Excluded { why: reason },
    };
    Line {
        kind: item.kind,
        label: label_of(item),
        mine,
        selection,
        result: None,
        hold: None,
        evidence: Vec::new(),
        history: Vec::new(),
        needs_key: false,
        accepted_loss: None,
    }
}

fn file_like(kind: ItemKind) -> bool {
    matches!(kind, ItemKind::File | ItemKind::Folder | ItemKind::Package)
}

/// Whether `role` may give `status` for an item of `kind`.
fn may_say(role: Role, kind: ItemKind, status: ItemStatus) -> bool {
    use ItemStatus as S;
    let common = matches!(status, S::NeedsAction | S::Failed);
    match role {
        Role::Transfer => file_like(kind) && (common || status == S::Copied),
        Role::Data => file_like(kind) && (common || matches!(status, S::Copied | S::Converted)),
        Role::Install => {
            kind == ItemKind::App && (common || matches!(status, S::Reinstalled | S::Converted))
        }
        Role::Settings => {
            kind == ItemKind::Setting && (common || matches!(status, S::Converted | S::Copied))
        }
        Role::Person | Role::Readiness | Role::Broker => false,
    }
}

/// Whether `role` may give evidence of `ev` about an item of `kind`.
fn may_give(role: Role, kind: ItemKind, ev: EvidenceKind) -> bool {
    use EvidenceKind as E;
    match ev {
        E::Captured => file_like(kind) && matches!(role, Role::Transfer | Role::Data),
        E::VerifiedTransfer => file_like(kind) && role == Role::Transfer,
        E::Installed => kind == ItemKind::App && role == Role::Install,
        E::Imported => matches!(role, Role::Settings | Role::Data),
        E::PersonConfirmed => role == Role::Person,
        E::RecoveryChecked => matches!(role, Role::Data | Role::Readiness),
        E::SourceOnly => role == Role::Readiness,
        E::CleanupReceipt => role == Role::Broker,
    }
}

fn evidence_well_formed(ev: &Evidence) -> bool {
    let named = |s: &str| !s.trim().is_empty();
    match ev {
        Evidence::Captured { .. } | Evidence::VerifiedTransfer { .. } => true,
        Evidence::Installed { package, version } => named(package) && named(version),
        Evidence::Imported { adapter, coverage } => named(adapter) && coverage.valid(),
        Evidence::PersonConfirmed { task, coverage } => named(task) && coverage.valid(),
        Evidence::RecoveryChecked { what, coverage } => named(what) && coverage.valid(),
        Evidence::SourceOnly { places } => !places.is_empty(),
        Evidence::CleanupReceipt { operation } => named(operation),
    }
}

impl Ledger {
    /// A line for every item of the approved `record`, for the person `mover` (their account ID on
    /// the old laptop). The plan generation is the record's revision.
    pub fn from_record(record: &Record, mover: &str, id: MoveId, started_at: u64) -> Self {
        let lines = record
            .items
            .iter()
            .map(|i| (i.id, line_of(i, mover)))
            .collect();
        Self {
            id,
            generation: record.revision,
            started_at,
            lines,
            search: SearchScope::default(),
            refused: Vec::new(),
        }
    }

    pub fn id(&self) -> MoveId {
        self.id
    }

    /// The plan generation receipts must name.
    pub fn generation(&self) -> u64 {
        self.generation
    }

    pub fn line(&self, item: ItemId) -> Option<&Line> {
        self.lines.get(&item)
    }

    /// Every line, in item order.
    pub fn lines(&self) -> impl Iterator<Item = (&ItemId, &Line)> {
        self.lines.iter()
    }

    /// Receipts refused, most recent last.
    pub fn refused(&self) -> &[RefusedReceipt] {
        &self.refused
    }

    pub fn search(&self) -> &SearchScope {
        &self.search
    }

    /// Takes a receipt, or refuses it (and keeps the refusal).
    pub fn accept(&mut self, r: Receipt, probe: &dyn Probe, now: u64) -> Result<(), Refused> {
        if let Err(why) = self.check(&r, probe, now) {
            if self.refused.len() >= MAX_REFUSED {
                self.refused.remove(0);
            }
            self.refused.push(RefusedReceipt {
                item: r.item,
                by: r.role,
                why,
            });
            return Err(why);
        }
        let line = self.lines.get_mut(&r.item).expect("checked above");
        if let Some(result) = r.result {
            let next = Attempt {
                result,
                by: r.role,
                at: r.observed_at,
            };
            if let Some(prev) = line.result.replace(next) {
                if line.history.len() >= MAX_HISTORY {
                    line.history.remove(0);
                }
                line.history.push(prev);
            }
            line.hold = None;
            if result.status().is_done() {
                line.accepted_loss = None;
            }
        }
        if let Some(evidence) = r.evidence {
            line.evidence.push(Observation {
                evidence,
                by: r.role,
                at: r.observed_at,
            });
        }
        Ok(())
    }

    fn check(&self, r: &Receipt, probe: &dyn Probe, now: u64) -> Result<(), Refused> {
        if r.source != self.id.source || r.target != self.id.target {
            return Err(Refused::WrongDevice);
        }
        if r.session != self.id.session {
            return Err(Refused::OldSession);
        }
        if r.generation != self.generation {
            return Err(Refused::StalePlan);
        }
        let line = self.lines.get(&r.item).ok_or(Refused::UnknownItem)?;
        if line.selection != Selection::Selected {
            return Err(Refused::NotSelected);
        }
        if r.result.is_none() && r.evidence.is_none() {
            return Err(Refused::Empty);
        }
        if let Some(result) = r.result
            && !may_say(r.role, line.kind, result.status())
        {
            return Err(Refused::WrongRole);
        }
        if let Some(ev) = &r.evidence {
            if !may_give(r.role, line.kind, ev.kind()) {
                return Err(Refused::WrongRole);
            }
            if !evidence_well_formed(ev) {
                return Err(Refused::BadEvidence);
            }
            if line.evidence.len() >= MAX_EVIDENCE {
                return Err(Refused::TooMuch);
            }
        }
        if r.observed_at > now || r.observed_at < self.started_at {
            return Err(Refused::BadTime);
        }
        let claims_success = r.result.is_some_and(|x| x.status().is_done());
        if claims_success || r.evidence.is_some() {
            match probe.observe(r) {
                Observed::Matches => {}
                Observed::Differs => return Err(Refused::NotWhatIsThere),
                Observed::CannotTell => return Err(Refused::CouldNotConfirm),
            }
        }
        Ok(())
    }

    fn selected_mut(&mut self, item: ItemId) -> Result<&mut Line, LedgerError> {
        let line = self.lines.get_mut(&item).ok_or(LedgerError::UnknownItem)?;
        if line.selection != Selection::Selected {
            return Err(LedgerError::NotSelected);
        }
        Ok(line)
    }

    fn hold(&mut self, item: ItemId, hold: Hold) -> Result<(), LedgerError> {
        let line = self.selected_mut(item)?;
        if line
            .result
            .as_ref()
            .is_some_and(|a| a.result.status().is_done())
        {
            return Err(LedgerError::AlreadyDone);
        }
        line.hold = Some(hold);
        Ok(())
    }

    /// Selected work waits (for another step, or for the person).
    pub fn defer(&mut self, item: ItemId, why: DeferWhy) -> Result<(), LedgerError> {
        self.hold(item, Hold::Deferred { why })
    }

    /// Selected work cannot be reached now.
    pub fn inaccessible(&mut self, item: ItemId, why: InaccessibleWhy) -> Result<(), LedgerError> {
        self.hold(item, Hold::Inaccessible { why })
    }

    /// The item cannot be opened without a key or recovery material.
    pub fn needs_key(&mut self, item: ItemId) -> Result<(), LedgerError> {
        self.lines
            .get_mut(&item)
            .ok_or(LedgerError::UnknownItem)?
            .needs_key = true;
        Ok(())
    }

    /// Where an item stands.
    pub fn standing(&self, item: ItemId) -> Option<Standing> {
        self.lines.get(&item).map(standing_of)
    }

    /// Whether a moved item can be opened, as far as keys go (T27-004).
    pub fn access(&self, item: ItemId) -> Option<Access> {
        let line = self.lines.get(&item)?;
        Some(match standing_of(line) {
            Standing::Done(_) if line.needs_key && !has(line, EvidenceKind::RecoveryChecked) => {
                Access::PreservedKeyUnresolved
            }
            Standing::Done(_) => Access::Usable,
            _ => Access::NotOnNewLaptop,
        })
    }

    pub fn tally(&self) -> Tally {
        let mut t = Tally::default();
        for line in self.lines.values() {
            if line.selection == Selection::Selected {
                t.selected += 1;
            }
            match standing_of(line) {
                Standing::Done(_) => t.done += 1,
                Standing::NeedsAction => t.needs_action += 1,
                Standing::Failed => t.failed += 1,
                Standing::Deferred => t.deferred += 1,
                Standing::Inaccessible => t.inaccessible += 1,
                Standing::NotTested => t.not_tested += 1,
                Standing::SkippedByUser => t.skipped += 1,
                Standing::Excluded => t.excluded += 1,
            }
            if line.accepted_loss.is_some() {
                t.accepted_loss += 1;
            }
        }
        t
    }

    /// The warning for losing an item that is not on the new laptop, and the digest that binds an
    /// agreement to exactly these words for this item and plan.
    pub fn loss_warning(&self, item: ItemId) -> Option<(String, [u8; 32])> {
        let line = self.lines.get(&item)?;
        if matches!(standing_of(line), Standing::Done(_)) {
            return None;
        }
        let words = format!(
            "\u{201c}{}\u{201d} is not on the new laptop. If you wipe or give away the old \
             laptop, it will be gone. PCTwin will record that you chose to leave it behind, not \
             that it moved.",
            line.label
        );
        let mut h = blake3::Hasher::new();
        h.update(b"pctwin/v1/loss-warning\0");
        h.update(item.to_hex().as_bytes());
        h.update(&self.generation.to_le_bytes());
        h.update(words.as_bytes());
        Some((words, *h.finalize().as_bytes()))
    }

    /// The person accepts losing an item, after the exact warning `shown` (T27-027). The item's
    /// result stays what it was: an accepted loss is a decision, never a success.
    pub fn accept_loss(
        &mut self,
        item: ItemId,
        shown: [u8; 32],
        at: u64,
    ) -> Result<(), LedgerError> {
        let (_, digest) = match self.loss_warning(item) {
            Some(w) => w,
            None if self.lines.contains_key(&item) => return Err(LedgerError::AlreadyDone),
            None => return Err(LedgerError::UnknownItem),
        };
        if digest != shown {
            return Err(LedgerError::NotTheWarningShown);
        }
        self.lines
            .get_mut(&item)
            .expect("warned above")
            .accepted_loss = Some(AcceptedLoss { at, shown });
        Ok(())
    }

    /// Done, or its loss accepted: nothing more is owed for it before leaving the old laptop.
    pub fn settled(&self, item: ItemId) -> bool {
        self.lines.get(&item).is_some_and(|l| {
            matches!(standing_of(l), Standing::Done(_)) || l.accepted_loss.is_some()
        })
    }

    /// A place on the old laptop was searched.
    pub fn searched(&mut self, place: &str) {
        self.search.not_searched.remove(place);
        self.search.searched.insert(place.to_string());
    }

    /// A place on the old laptop is known and was not searched (a drive not connected, say).
    pub fn not_searched(&mut self, place: &str) {
        self.search.searched.remove(place);
        self.search.not_searched.insert(place.to_string());
    }

    /// Whether every place known was searched (and some place was). Never more than that: it does
    /// not say there is nothing anywhere else (T27-003).
    pub fn found_everything(&self) -> bool {
        self.search.not_searched.is_empty() && !self.search.searched.is_empty()
    }

    /// The mover's items that are still only on the old laptop: not done on the new laptop and
    /// not safe elsewhere (an online-only file is in its cloud; a system item and a rebuildable
    /// folder are not the person's work; something tied to the old laptop is a key question).
    pub fn only_on_old_laptop(&self) -> Vec<ItemId> {
        self.lines
            .iter()
            .filter(|(_, l)| l.mine && only_old(l))
            .map(|(id, _)| *id)
            .collect()
    }

    /// The plain words for what is only on the old laptop, with where PCTwin looked and where it
    /// did not (T27-003).
    pub fn source_only_words(&self) -> String {
        let n = self.only_on_old_laptop().len();
        let what = if n == 1 {
            "1 item is".to_string()
        } else {
            format!("{n} items are")
        };
        let list = |s: &BTreeSet<String>| s.iter().cloned().collect::<Vec<_>>().join(", ");
        let mut words = if self.search.searched.is_empty() {
            format!("{what} only on the old laptop. PCTwin has not looked through it yet.")
        } else {
            format!(
                "{what} only on the old laptop, in the places PCTwin looked: {}.",
                list(&self.search.searched)
            )
        };
        if !self.search.not_searched.is_empty() {
            words.push_str(&format!(
                " PCTwin did not look in: {}. Check there yourself before you wipe or give away \
                 the old laptop.",
                list(&self.search.not_searched)
            ));
        }
        words
    }

    /// Moves the ledger to the next plan `record` of the same old laptop (final changes): new
    /// items get a line; every existing line, its results and history are kept.
    pub fn next_generation(&mut self, record: &Record, mover: &str) -> Result<(), LedgerError> {
        if record.source_laptop != self.id.source || record.revision <= self.generation {
            return Err(LedgerError::NotTheNextPlan);
        }
        for item in &record.items {
            self.lines
                .entry(item.id)
                .or_insert_with(|| line_of(item, mover));
        }
        self.generation = record.revision;
        Ok(())
    }
}

fn has(line: &Line, kind: EvidenceKind) -> bool {
    line.evidence.iter().any(|o| o.evidence.kind() == kind)
}

fn standing_of(line: &Line) -> Standing {
    match line.selection {
        Selection::SkippedByUser => return Standing::SkippedByUser,
        Selection::Excluded { .. } => return Standing::Excluded,
        Selection::Selected => {}
    }
    match (line.hold, line.result.as_ref().map(|a| a.result.status())) {
        (Some(Hold::Deferred { .. }), _) => Standing::Deferred,
        (Some(Hold::Inaccessible { .. }), _) => Standing::Inaccessible,
        (None, None) => Standing::NotTested,
        (None, Some(ItemStatus::NeedsAction)) => Standing::NeedsAction,
        (None, Some(ItemStatus::Failed | ItemStatus::SkippedByUser)) => Standing::Failed,
        (None, Some(done)) => Standing::Done(done),
    }
}

fn only_old(line: &Line) -> bool {
    match line.selection {
        Selection::Excluded {
            why:
                LeftOutReason::System
                | LeftOutReason::Rebuildable
                | LeftOutReason::CloudOnly
                | LeftOutReason::DeviceBound,
        } => false,
        Selection::Excluded { .. } | Selection::SkippedByUser => true,
        Selection::Selected => match standing_of(line) {
            Standing::Done(_) => false,
            // In place on the new laptop but unfinished (a sign-in, a restart, a permission, an
            // online-only file in its cloud, an app to download from its maker) is not only on
            // the old laptop; missing parts, or a step PCTwin cannot judge, are.
            Standing::NeedsAction => matches!(
                line.result.as_ref().and_then(|a| a.result.reason()),
                None | Some(Reason::PartsMissing | Reason::FinishByHand)
            ),
            _ => true,
        },
    }
}

/// What the journal on disk shows for one item.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum OnDisk {
    /// Written and checked, with its fingerprint.
    Written([u8; 32]),
    /// An identical file was already there; nothing was written.
    AlreadyThere,
}

/// The journal as the real state of copies (T27-001): a copy is confirmed only when this move's
/// journal (its scope, its old laptop) shows it committed or already there, and undo has not
/// taken it back.
#[derive(Debug, Clone, Default)]
pub struct JournalProbe {
    copies: BTreeMap<ItemId, OnDisk>,
}

fn taken_back(outcome: &UndoOutcome) -> bool {
    matches!(
        outcome,
        UndoOutcome::Deleted | UndoOutcome::Trashed { .. } | UndoOutcome::AlreadyGone
    )
}

impl JournalProbe {
    /// From journal `entries`, `undone` naming the entries undo took back.
    pub fn from_entries(
        entries: &[Entry],
        undone: &BTreeSet<u64>,
        source: LaptopId,
        scope: JournalScope,
    ) -> Self {
        let mut copies = BTreeMap::new();
        for e in entries {
            if e.write.source_laptop != source || e.write.scope != scope || undone.contains(&e.id) {
                continue;
            }
            match &e.state {
                State::Committed { fingerprint, .. } => {
                    copies.insert(e.write.item, OnDisk::Written(*fingerprint));
                }
                State::Existing { .. } => {
                    copies.entry(e.write.item).or_insert(OnDisk::AlreadyThere);
                }
                _ => {}
            }
        }
        Self { copies }
    }

    /// Reads `journal` (read only): its entries, and which undo took back.
    pub fn read(
        journal: &Journal,
        source: LaptopId,
        scope: JournalScope,
    ) -> Result<Self, JournalError> {
        let entries = journal.entries()?;
        let mut undone = BTreeSet::new();
        for e in &entries {
            if let Some(Undo::Done { outcome }) = journal.undo_of(e.id)?
                && taken_back(&outcome)
            {
                undone.insert(e.id);
            }
        }
        Ok(Self::from_entries(&entries, &undone, source, scope))
    }
}

impl Probe for JournalProbe {
    fn observe(&self, r: &Receipt) -> Observed {
        if r.role != Role::Transfer {
            return Observed::CannotTell;
        }
        let claimed = match &r.evidence {
            Some(Evidence::VerifiedTransfer { fingerprint }) => Some(*fingerprint),
            _ => None,
        };
        match (self.copies.get(&r.item), claimed) {
            (None, _) => Observed::Differs,
            (Some(OnDisk::Written(fp)), Some(c)) if *fp != c => Observed::Differs,
            (Some(OnDisk::AlreadyThere), Some(_)) => Observed::CannotTell,
            (Some(_), _) => Observed::Matches,
        }
    }
}

/// The 2.6 scheduler as the real state of child jobs (T27-001): a worker's result for an item is
/// confirmed only when the scheduler itself took a job of that worker as done, the job given here
/// is the very job it ran (same digest), it was for this old laptop and session, and the item is
/// inside the job (its scope's items; for an app or settings job, the items named for it).
pub struct SchedulerProbe<'a> {
    scheduler: &'a Scheduler,
    jobs: &'a [ChildJob],
    job_items: &'a BTreeMap<JobId, BTreeSet<ItemId>>,
}

impl<'a> SchedulerProbe<'a> {
    pub fn new(
        scheduler: &'a Scheduler,
        jobs: &'a [ChildJob],
        job_items: &'a BTreeMap<JobId, BTreeSet<ItemId>>,
    ) -> Self {
        Self {
            scheduler,
            jobs,
            job_items,
        }
    }
}

impl Probe for SchedulerProbe<'_> {
    fn observe(&self, r: &Receipt) -> Observed {
        if !matches!(r.role, Role::Install | Role::Settings | Role::Data) {
            return Observed::CannotTell;
        }
        let confirmed = self.jobs.iter().any(|job| {
            Role::from(job.worker) == r.role
                && job.source == r.source
                && job.session == r.session
                && self
                    .job_items
                    .get(&job.id)
                    .is_some_and(|items| items.contains(&r.item))
                && match &job.scope {
                    Scope::Items { laptop, items } => {
                        *laptop == r.source && items.contains(&r.item)
                    }
                    Scope::App { .. } | Scope::Settings { .. } => true,
                }
                && self.scheduler.is_done(&job.id)
                && self.scheduler.result_for(&job.id, true).digest == job.digest()
        });
        if confirmed {
            Observed::Matches
        } else {
            Observed::Differs
        }
    }
}
