//! The move for the screens (M23, X01, X02, X05, X07, X10, R01, R02): once both laptops are
//! paired, each starts its side here with the link pairing kept. The old laptop finds the signed-in
//! person's files and sends them by the new laptop's rules; the new laptop shows the plan, waits
//! for the person's approval, and receives. The screens get plain data only: measured counts,
//! translation keys, and the mover's own file names, never another person's and never engine text
//! outside "more details".
//!
//! The engine's move has no `Send` bound (it runs on one task), so it runs on a thread of its own,
//! on the app's runtime; the screens' commands reach it through [`Moving`].

use std::collections::{BTreeMap, HashMap};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex};
use std::time::Duration;

use pctwin_gate::{CheckedFrom, Destinations, WhyMemory};
use pctwin_move::landing::LandingRules;
use pctwin_move::{Ended, Live, MoveError, NotSentWhy, Preview, Received, Sent, Steps, Stop};
use pctwin_record::{
    FolderRole, Inclusion, Item, ItemId, LaptopId, LeftOutReason, Owner, PersonTarget, Record,
    Storage,
};
use pctwin_scan::FolderLookup;
use pctwin_transfer::{
    Cause, FsOpener, LandingPlace, ReceiveOutcome, SendOutcome, TimeLeft, TransferError,
    role_label, shown_path,
};
use pctwin_types::{ItemStatus, Reason};
use serde::{Deserialize, Serialize};
use tauri::{AppHandle, Emitter, Manager, Runtime, State};
use tokio::sync::oneshot;

use crate::pairing::{AppPairing, LabelView, PairedWith};

/// What the move is at (scanning, waiting for the other laptop, moving).
pub const STAGE_EVENT: &str = "move-stage";
/// The new laptop's plan for the person to approve (M23).
pub const PLAN_EVENT: &str = "move-plan";
/// The measured progress (X01, X02, X05), a few times a second at most.
pub const PROGRESS_EVENT: &str = "move-progress";
/// How the move ended (R01).
pub const FINISHED_EVENT: &str = "move-finished";
/// The move could not go on (C09, X07, the general error page).
pub const PROBLEM_EVENT: &str = "move-problem";

/// How often the screens are sent the progress.
const TICK: Duration = Duration::from_millis(400);
/// How often the battery is read during a move.
const POWER_EVERY: Duration = Duration::from_secs(15);
/// The longest name (in bytes) for the folder that stands for one the new laptop lacks.
const MAX_FALLBACK: usize = 255;

/// The move on this laptop: at most one at a time.
#[derive(Default)]
pub struct Moving {
    run: Mutex<Option<Run>>,
}

/// The app's move state, as the commands find it.
pub type AppMoving = Arc<Moving>;

/// A fresh move state for the app.
pub fn new_state() -> AppMoving {
    Arc::new(Moving::default())
}

struct Run {
    live: Live,
    /// The old laptop's scan: set to stop it.
    scan_stop: Arc<AtomicBool>,
    /// The new laptop: the plan on screen, and where the person's answer goes.
    waiting: Option<(PlanKey, oneshot::Sender<bool>)>,
    report: Vec<ReportItem>,
    finished: bool,
}

/// The plan revision the person saw, exactly.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct PlanKey {
    pub revision: u64,
    pub fingerprint: String,
}

fn lock<T>(m: &Mutex<T>) -> std::sync::MutexGuard<'_, T> {
    m.lock().unwrap_or_else(std::sync::PoisonError::into_inner)
}

impl Moving {
    /// Starts a run, unless one is going.
    fn begin(&self) -> Result<(Live, Arc<AtomicBool>), MoveProblem> {
        let mut run = lock(&self.run);
        if run.as_ref().is_some_and(|r| !r.finished) {
            return Err(MoveProblem::Busy);
        }
        let live = Live::default();
        let scan_stop = Arc::new(AtomicBool::new(false));
        *run = Some(Run {
            live: live.clone(),
            scan_stop: scan_stop.clone(),
            waiting: None,
            report: Vec::new(),
            finished: false,
        });
        Ok((live, scan_stop))
    }

    fn with_run<T>(&self, f: impl FnOnce(&mut Run) -> T) -> Result<T, MoveProblem> {
        lock(&self.run)
            .as_mut()
            .filter(|r| !r.finished)
            .map(f)
            .ok_or(MoveProblem::NotMoving)
    }

    fn wait_for_answer(&self, key: PlanKey) -> oneshot::Receiver<bool> {
        let (tx, rx) = oneshot::channel();
        if let Some(run) = lock(&self.run).as_mut() {
            run.waiting = Some((key, tx));
        }
        rx
    }

    /// The person's answer to the plan on screen: only for exactly that revision.
    fn answer(&self, key: &PlanKey, yes: bool) -> Result<(), MoveProblem> {
        self.with_run(|run| match run.waiting.take() {
            Some((shown, tx)) if shown == *key => {
                let _ = tx.send(yes);
                Ok(())
            }
            other => {
                run.waiting = other;
                Err(MoveProblem::PlanChanged)
            }
        })?
    }

    fn pause(&self, pause: bool) -> Result<(), MoveProblem> {
        self.with_run(|run| {
            if pause {
                run.live.control.pause();
            } else {
                run.live.control.carry_on();
            }
        })
    }

    /// Stops the move: before approval it is declined, during the scan the scan ends, and once
    /// moving no more is sent. What is already copied and checked stays.
    fn stop(&self) -> Result<(), MoveProblem> {
        self.with_run(|run| {
            run.scan_stop.store(true, Ordering::Release);
            if let Some((_, tx)) = run.waiting.take() {
                let _ = tx.send(false);
            }
            run.live.control.stop();
        })
    }

    fn finish(&self, report: Vec<ReportItem>) {
        if let Some(run) = lock(&self.run).as_mut() {
            run.report = report;
            run.finished = true;
            run.waiting = None;
        }
    }

    fn report(&self) -> Vec<ReportItem> {
        lock(&self.run)
            .as_ref()
            .map(|r| r.report.clone())
            .unwrap_or_default()
    }
}

pctwin_types::listed_enum! {
    /// Why a move command was refused, as a translation key.
    #[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
    #[serde(tag = "kind", rename_all = "kebab-case")]
    pub enum MoveProblem {
        /// The laptops are not paired (any more): pair again.
        NotPaired,
        /// A move is already going on this laptop.
        Busy,
        /// No move is going on.
        NotMoving,
        /// The plan changed since the person looked: look again.
        PlanChanged,
        /// The new laptop's folders could not be got ready.
        Failed,
    }
}

pctwin_types::listed_enum! {
    /// What the move is at, for the screens between pairing and progress.
    #[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
    #[serde(tag = "kind", rename_all = "kebab-case")]
    pub enum Stage {
        /// The old laptop is finding the person's files (M02).
        Scanning,
        /// Waiting for the other laptop: its list, or its person's approval.
        Waiting,
        /// Files are moving.
        Moving,
    }
}

pctwin_types::listed_enum! {
    /// Why a move ended before it finished, as a translation key; `detail` is the engine's words,
    /// for "more details" only.
    #[derive(Debug, Clone, PartialEq, Eq, Serialize)]
    #[serde(tag = "kind", rename_all = "kebab-case")]
    pub enum EndedEarly {
        /// The connection between the laptops was lost (C09).
        LinkLost { detail: String },
        /// The person on the new laptop did not approve the plan.
        NotApproved,
        /// Something else went wrong (the general error page).
        Failed { detail: String },
    }
}

impl EndedEarly {
    fn of(e: &MoveError) -> Self {
        match e {
            MoveError::Stopped(_) => Self::NotApproved,
            MoveError::Transfer(TransferError::ConnectionDropped) => Self::LinkLost {
                detail: e.to_string(),
            },
            _ => Self::Failed {
                detail: e.to_string(),
            },
        }
    }
}

/* ---------- The plan (M23) ---------- */

/// A folder group of the plan, as M23 lists it.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct PlanGroup {
    pub id: String,
    pub role: FolderRole,
    pub path: String,
    pub size: Size,
    pub to_drive: String,
    pub to_folder: String,
}

/// A measured size: the whole truth here, since the plan is the old laptop's record.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(tag = "kind", rename_all = "kebab-case")]
pub enum Size {
    Known { bytes: u64, files: u64 },
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
pub struct LeftOut {
    pub reason: LeftOutReason,
    pub count: u64,
}

/// A range of whole minutes, as measured (never a promise).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
pub struct Range {
    pub low: u64,
    pub high: u64,
}

/// What the person sees before approving: whether it fits, about how long, and what will not go
/// as it is.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct PreviewView {
    /// `None`: the free space could not be read.
    pub fits: Option<bool>,
    pub short_bytes: u64,
    pub minutes: Option<Range>,
    pub renamed: u64,
    pub too_big: u64,
    pub refused: u64,
    pub runs_at_sign_in: u64,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct PlanView {
    pub revision: u64,
    pub fingerprint: String,
    pub groups: Vec<PlanGroup>,
    pub left_out: Vec<LeftOut>,
    pub preview: PreviewView,
}

/// Whether `item` is the mover's to see by name: their own and the shared things, never another
/// person's (Security Design part C).
fn shown_to_mover(item: &Item, mover: &str) -> bool {
    match &item.owner {
        Owner::Person { account_id } => account_id == mover,
        Owner::Shared => true,
        Owner::System => false,
    }
}

/// Where a file lands, as the person reads it: the real folder on this laptop (from `shown`, by the
/// destination's label), then any folder PCTwin adds in front of the file's own path (the one that
/// stands for a folder this laptop lacks).
fn shown_place(item: &Item, place: &LandingPlace, shown: &BTreeMap<String, String>) -> String {
    let base = shown
        .get(&place.destination)
        .cloned()
        .unwrap_or_else(|| place.destination.clone());
    let own = shown_path(&item.path);
    let added = place
        .path
        .strip_suffix(own.as_str())
        .unwrap_or("")
        .trim_end_matches('/');
    if added.is_empty() {
        base
    } else {
        format!("{base}/{added}")
    }
}

/// The plan as M23 shows it. `shown` gives each destination label's real folder on this laptop.
fn plan_view(
    record: &Record,
    preview: &Preview,
    rules: &LandingRules,
    shown: &BTreeMap<String, String>,
) -> PlanView {
    let mover = rules.mover.as_str();
    let mut groups: BTreeMap<FolderRole, (u64, u64, String)> = BTreeMap::new();
    let mut left: BTreeMap<String, (LeftOutReason, u64)> = BTreeMap::new();
    for item in &record.items {
        match item.inclusion {
            Inclusion::Included if shown_to_mover(item, mover) => {
                let g = groups.entry(item.place.role).or_insert_with(|| {
                    let to = rules
                        .landing(item)
                        .map(|p| shown_place(item, &p, shown))
                        .unwrap_or_default();
                    (0, 0, to)
                });
                g.0 = g.0.saturating_add(item.size_bytes);
                g.1 += 1;
            }
            Inclusion::LeftOut { reason } if shown_to_mover(item, mover) => {
                left.entry(format!("{reason:?}")).or_insert((reason, 0)).1 += 1;
            }
            // Another person's things are neither listed nor counted for the mover.
            _ => {}
        }
    }
    let fit = &preview.fit;
    let fits = fit
        .places
        .iter()
        .map(|p| p.fits())
        .try_fold(true, |all, f| f.map(|f| all && f));
    let minutes = match preview.time {
        pctwin_transfer::TimeRange::NotKnown => None,
        pctwin_transfer::TimeRange::Between { fastest, slowest } => Some(Range {
            low: fastest.as_secs().div_ceil(60),
            high: slowest.as_secs().div_ceil(60),
        }),
    };
    PlanView {
        revision: record.revision,
        fingerprint: record.fingerprint().to_hex(),
        groups: groups
            .into_iter()
            .map(|(role, (bytes, files, to))| PlanGroup {
                id: role_label(role).to_string(),
                role,
                path: String::new(),
                size: Size::Known { bytes, files },
                to_drive: String::new(),
                to_folder: to,
            })
            .collect(),
        left_out: left
            .into_values()
            .map(|(reason, count)| LeftOut { reason, count })
            .collect(),
        preview: PreviewView {
            fits,
            short_bytes: fit.places.iter().map(|p| p.short_by()).sum(),
            minutes,
            renamed: preview.problems.renamed.count,
            too_big: preview.problems.too_big.count,
            refused: preview.problems.refused.count,
            runs_at_sign_in: preview.problems.runs_at_sign_in.count,
        },
    }
}

/* ---------- Progress (X01, X02, X05) ---------- */

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Counts {
    pub total: u64,
    pub waiting: u64,
    pub copying: u64,
    pub checking: u64,
    pub done: u64,
    pub needs_action: u64,
    pub failed: u64,
    pub skipped_by_user: u64,
}

impl From<&pctwin_transfer::StepCounts> for Counts {
    fn from(c: &pctwin_transfer::StepCounts) -> Self {
        Self {
            total: c.total,
            waiting: c.waiting,
            copying: c.copying,
            checking: c.checking,
            done: c.copied + c.already_there,
            needs_action: c.needs_action,
            failed: c.failed,
            skipped_by_user: 0,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(tag = "kind", rename_all = "kebab-case")]
pub enum GroupName {
    Role { role: FolderRole },
    Folder { name: String },
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct MoveGroup {
    pub id: String,
    pub name: GroupName,
    pub counts: Counts,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "kebab-case")]
pub enum RunState {
    Running,
    Pausing,
    Paused,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct Laptops {
    pub old: Option<LabelView>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
pub struct Bytes {
    pub checked: u64,
    pub total: u64,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct Current {
    pub name: String,
    pub step: &'static str,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(tag = "kind", rename_all = "kebab-case")]
pub enum Notice {
    /// The drive of this place is not keeping files correctly: the move is held (X07).
    DriveProblem { place: GroupName },
    /// This laptop is low on battery; at the safe limit the move was paused for it (the person
    /// carries on once it is plugged in).
    LowBattery { paused: bool },
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct MoveProgress {
    pub state: RunState,
    pub laptops: Laptops,
    pub groups: Vec<MoveGroup>,
    pub counts: Counts,
    pub bytes: Bytes,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub current: Option<Current>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub speed: Option<Range>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub minutes_left: Option<Range>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub notice: Option<Notice>,
}

/// A group's name on screen, from its destination label: a folder by its role, or the place's
/// own name.
fn group_name(label: &str) -> GroupName {
    const ROLES: [FolderRole; 10] = [
        FolderRole::Home,
        FolderRole::Desktop,
        FolderRole::Documents,
        FolderRole::Downloads,
        FolderRole::Pictures,
        FolderRole::Music,
        FolderRole::Videos,
        FolderRole::Public,
        FolderRole::AppData,
        FolderRole::Other,
    ];
    match ROLES.into_iter().find(|r| role_label(*r) == label) {
        Some(role) => GroupName::Role { role },
        None => GroupName::Folder {
            name: label.to_string(),
        },
    }
}

/// What this laptop's battery means for the move: nothing, a warning, or a safe pause.
fn battery_notice(advice: pctwin_power::BatteryAdvice) -> Option<Notice> {
    match advice {
        pctwin_power::BatteryAdvice::PlugIn => Some(Notice::LowBattery { paused: false }),
        pctwin_power::BatteryAdvice::PauseNow => Some(Notice::LowBattery { paused: true }),
        pctwin_power::BatteryAdvice::Fine | pctwin_power::BatteryAdvice::OnBattery => None,
    }
}

fn progress_view(
    snap: &pctwin_transfer::MeterSnapshot,
    paused_asked: bool,
    braked: Option<String>,
    battery: Option<Notice>,
    laptops: &Laptops,
    names: &HashMap<ItemId, String>,
) -> MoveProgress {
    let r = &snap.reading;
    let state = if r.paused_here || r.paused_there {
        RunState::Paused
    } else if paused_asked {
        RunState::Pausing
    } else {
        RunState::Running
    };
    let one = |n: u64| Range { low: n, high: n };
    MoveProgress {
        state,
        laptops: laptops.clone(),
        groups: r
            .groups
            .iter()
            .map(|(label, c)| MoveGroup {
                id: label.clone(),
                name: group_name(label),
                counts: c.into(),
            })
            .collect(),
        counts: (&r.files).into(),
        bytes: Bytes {
            checked: r.bytes_checked.saturating_add(r.bytes_already_there),
            total: r.bytes_planned,
        },
        current: r
            .checking
            .and_then(|id| names.get(&id))
            .map(|name| Current {
                name: name.clone(),
                step: "checking",
            }),
        speed: snap.bytes_per_second.map(one),
        minutes_left: match snap.time_left {
            TimeLeft::About { seconds } => Some(one(seconds.div_ceil(60))),
            _ => None,
        },
        // A drive that is not keeping files is the more urgent of the two.
        notice: braked
            .map(|place| Notice::DriveProblem {
                place: group_name(&place),
            })
            .or(battery),
    }
}

/* ---------- The outcome (R01) and the report (R02) ---------- */

/// Items by how they ended: one count for each of the six statuses.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize)]
#[serde(rename_all = "kebab-case")]
pub struct ByStatus {
    pub copied: u64,
    pub reinstalled: u64,
    pub converted: u64,
    pub needs_action: u64,
    pub skipped_by_user: u64,
    pub failed: u64,
}

impl ByStatus {
    fn add(&mut self, status: ItemStatus) {
        *match status {
            ItemStatus::Copied => &mut self.copied,
            ItemStatus::Reinstalled => &mut self.reinstalled,
            ItemStatus::Converted => &mut self.converted,
            ItemStatus::NeedsAction => &mut self.needs_action,
            ItemStatus::SkippedByUser => &mut self.skipped_by_user,
            ItemStatus::Failed => &mut self.failed,
        } += 1;
    }
}

#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct MoveOutcome {
    pub by_status: ByStatus,
    pub not_copied: u64,
    pub bytes_checked: u64,
    pub stopped: bool,
    pub checked_from_memory: u64,
}

/// How a copied file is known to be a true copy: every copied line says which, and they are never
/// shown as the same line.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(tag = "from", rename_all = "kebab-case")]
pub enum Evidence {
    /// Read back from the drive.
    Drive,
    /// Read back, but through this laptop's memory, and why.
    Memory { why: WhyMemory },
    /// An identical file was already on the new laptop.
    AlreadyThere,
    /// Checked before the laptop restarted, and proven again after.
    Recovered,
    /// The old laptop's view: the new laptop said it checked it.
    NewLaptop,
}

impl Evidence {
    fn of(checked: Option<CheckedFrom>) -> Self {
        match checked {
            Some(CheckedFrom::Drive) => Self::Drive,
            Some(CheckedFrom::Memory(why)) => Self::Memory { why },
            None => Self::Recovered,
        }
    }
}

/// One line of R02: the mover's own and shared files only.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct ReportItem {
    pub id: String,
    pub name: String,
    pub source: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub destination: Option<String>,
    pub status: ItemStatus,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub reason: Option<Reason>,
    /// Exactly when it was copied.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub evidence: Option<Evidence>,
}

/// The status and reason the report shows for a file that did not arrive, by its cause. A file
/// stored online only is the person's to download; everything else failed.
fn status_of(cause: Cause) -> (ItemStatus, Reason) {
    let failed = |r| (ItemStatus::Failed, r);
    match cause {
        Cause::NoSpace | Cause::TooBigForDrive => failed(Reason::NotEnoughSpace),
        Cause::ChangedDuringMove => failed(Reason::ChangedDuringMove),
        Cause::Unreadable | Cause::NoLongerThere | Cause::InUse => failed(Reason::Unreadable),
        Cause::OnlineOnly => (ItemStatus::NeedsAction, Reason::OnlineOnly),
        Cause::RefusedBySafetyCheck => failed(Reason::RefusedBySafetyCheck),
        Cause::NameNotAllowed => failed(Reason::NameNotAllowed),
        Cause::NotVerified
        | Cause::CouldNotWrite
        | Cause::RemovedAfterCopy
        | Cause::Stopped
        | Cause::NotAccepted
        | Cause::MoveEnded => failed(Reason::NotVerified),
    }
}

fn name_of(path: &str) -> String {
    path.rsplit('/').next().unwrap_or(path).to_string()
}

fn source_of(item: &Item) -> String {
    format!("{}/{}", role_label(item.place.role), shown_path(&item.path))
}

/// The new laptop's outcome and report for `mover`: their own and the shared files line by line;
/// another person's files are never named or counted here.
fn received_outcome(received: &Received, mover: &str) -> (MoveOutcome, Vec<ReportItem>) {
    let items: HashMap<ItemId, &Item> = received.record.items.iter().map(|i| (i.id, i)).collect();
    let stopped = received.ended != Ended::Finished;
    let mut outcome = MoveOutcome {
        stopped,
        ..MoveOutcome::default()
    };
    let mut report = Vec::new();
    for file in &received.files {
        let Some(item) = items.get(&file.item) else {
            continue;
        };
        if !shown_to_mover(item, mover) {
            continue;
        }
        let (status, reason, evidence, destination) = match &file.outcome {
            Some(ReceiveOutcome::Finished(f)) => {
                outcome.bytes_checked = outcome.bytes_checked.saturating_add(item.size_bytes);
                if matches!(f.checked_from, Some(CheckedFrom::Memory(_))) {
                    outcome.checked_from_memory += 1;
                }
                (
                    ItemStatus::Copied,
                    None,
                    Some(Evidence::of(f.checked_from)),
                    Some(f.final_path.clone()),
                )
            }
            Some(ReceiveOutcome::AlreadyThere(p)) => {
                outcome.bytes_checked = outcome.bytes_checked.saturating_add(item.size_bytes);
                (
                    ItemStatus::Copied,
                    None,
                    Some(Evidence::AlreadyThere),
                    Some(p.clone()),
                )
            }
            Some(ReceiveOutcome::NeedsAction { finished, .. }) => (
                ItemStatus::NeedsAction,
                Some(Reason::FinishByHand),
                None,
                Some(finished.final_path.clone()),
            ),
            Some(ReceiveOutcome::Failed(f)) if stopped && f.cause() == Cause::Stopped => {
                outcome.not_copied += 1;
                continue;
            }
            Some(ReceiveOutcome::Failed(f)) => {
                let (status, reason) = status_of(f.cause());
                (status, Some(reason), None, None)
            }
            None if stopped => {
                outcome.not_copied += 1;
                continue;
            }
            None => (ItemStatus::Failed, Some(Reason::NotVerified), None, None),
        };
        outcome.by_status.add(status);
        let source = source_of(item);
        report.push(ReportItem {
            id: item.id.to_hex(),
            name: name_of(&source),
            source,
            destination,
            status,
            reason,
            evidence,
        });
    }
    (outcome, report)
}

/// The old laptop's outcome: what it sent and how the new laptop answered. The new laptop's
/// report is the one that counts; this one never says more than was confirmed.
fn sent_outcome(sent: &Sent, record: &Record) -> (MoveOutcome, Vec<ReportItem>) {
    let items: HashMap<ItemId, &Item> = record.items.iter().map(|i| (i.id, i)).collect();
    let stopped = sent.ended != Ended::Finished;
    let mut outcome = MoveOutcome {
        stopped,
        ..MoveOutcome::default()
    };
    let mut report = Vec::new();
    for file in &sent.files {
        let Some(item) = items.get(&file.item) else {
            continue;
        };
        let (status, reason, evidence) = match &file.outcome {
            Some(SendOutcome::Arrived) => {
                outcome.bytes_checked = outcome.bytes_checked.saturating_add(item.size_bytes);
                (ItemStatus::Copied, None, Some(Evidence::NewLaptop))
            }
            Some(SendOutcome::AlreadyThere) => {
                outcome.bytes_checked = outcome.bytes_checked.saturating_add(item.size_bytes);
                (ItemStatus::Copied, None, Some(Evidence::AlreadyThere))
            }
            Some(SendOutcome::Failed(f)) if stopped && f.cause() == Cause::Stopped => {
                outcome.not_copied += 1;
                continue;
            }
            Some(SendOutcome::Failed(f)) => {
                let (s, r) = status_of(f.cause());
                (s, Some(r), None)
            }
            None if stopped => {
                outcome.not_copied += 1;
                continue;
            }
            None => (ItemStatus::Failed, Some(Reason::NotVerified), None),
        };
        outcome.by_status.add(status);
        let source = source_of(item);
        report.push(ReportItem {
            id: item.id.to_hex(),
            name: name_of(&source),
            source,
            destination: None,
            status,
            reason,
            evidence,
        });
    }
    for not in &sent.not_sent {
        let Some(item) = items.get(&not.item) else {
            continue;
        };
        if !matches!(
            not.why,
            NotSentWhy::LeftOut {
                reason: LeftOutReason::ChosenByUser
            }
        ) {
            continue;
        }
        outcome.by_status.add(ItemStatus::SkippedByUser);
        let source = source_of(item);
        report.push(ReportItem {
            id: item.id.to_hex(),
            name: name_of(&source),
            source,
            destination: None,
            status: ItemStatus::SkippedByUser,
            reason: None,
            evidence: None,
        });
    }
    (outcome, report)
}

/* ---------- Running it ---------- */

/// The new laptop's answers: the mover's things go to the signed-in person here, other people
/// are not moved by this version of the app's flow, and the person approves on M23.
struct AppSteps<R: Runtime> {
    app: AppHandle<R>,
    moving: AppMoving,
    me: String,
    rules: LandingRules,
    names: Arc<Mutex<HashMap<ItemId, String>>>,
    /// Each destination label's real folder on this laptop, as the person reads it.
    shown: BTreeMap<String, String>,
}

impl<R: Runtime> Steps for AppSteps<R> {
    async fn person(&mut self, _: &Record, account_id: &str) -> Result<PersonTarget, Stop> {
        Ok(if account_id == self.rules.mover {
            PersonTarget::ExistingAccount {
                account_id: self.me.clone(),
            }
        } else {
            PersonTarget::Skip
        })
    }

    async fn approve(&mut self, record: &Record, preview: &Preview) -> Result<(), Stop> {
        let view = plan_view(record, preview, &self.rules, &self.shown);
        {
            let mut names = lock(&self.names);
            for item in &record.items {
                if shown_to_mover(item, &self.rules.mover) {
                    names.insert(item.id, name_of(&shown_path(&item.path)));
                }
            }
        }
        let answer = self.moving.wait_for_answer(PlanKey {
            revision: view.revision,
            fingerprint: view.fingerprint.clone(),
        });
        let _ = self.app.emit(PLAN_EVENT, view);
        match answer.await {
            Ok(true) => {
                let _ = self.app.emit(STAGE_EVENT, Stage::Moving);
                Ok(())
            }
            _ => Err(Stop::NotApproved),
        }
    }

    fn landing(&self, item: &Item) -> Option<LandingPlace> {
        self.rules.landing(item)
    }

    fn mover(&mut self, account_id: &str) {
        self.rules.mover = account_id.to_string();
    }
}

/// Sends the progress until `done`.
fn tick<R: Runtime>(
    app: AppHandle<R>,
    live: Live,
    laptops: Laptops,
    names: Arc<Mutex<HashMap<ItemId, String>>>,
    done: Arc<AtomicBool>,
) {
    tauri::async_runtime::spawn(async move {
        let mut last = None;
        let mut battery = None;
        let mut power_read: Option<std::time::Instant> = None;
        let mut paused_for_power = false;
        while !done.load(Ordering::Acquire) {
            if power_read.is_none_or(|t| t.elapsed() >= POWER_EVERY) {
                power_read = Some(std::time::Instant::now());
                if let Ok(power) =
                    tauri::async_runtime::spawn_blocking(pctwin_power::power_now).await
                {
                    let advice = pctwin_power::advise(&power);
                    // At the safe limit the move pauses once; carrying on is the person's choice.
                    if advice == pctwin_power::BatteryAdvice::PauseNow && !paused_for_power {
                        live.control.pause();
                        paused_for_power = true;
                    }
                    battery = battery_notice(advice);
                }
            }
            let view = progress_view(
                &live.meter.snapshot(),
                live.control.paused(),
                live.control.braked(),
                battery.clone(),
                &laptops,
                &lock(&names),
            );
            if last.as_ref() != Some(&view) {
                let _ = app.emit(PROGRESS_EVENT, &view);
                last = Some(view);
            }
            tokio::time::sleep(TICK).await;
        }
    });
}

/// The old laptop's side: find the person's files, then send them by the new laptop's rules.
async fn run_old<R: Runtime>(
    app: &AppHandle<R>,
    mut link: pctwin_link::Link,
    live: &Live,
    scan_stop: &AtomicBool,
) -> Result<(MoveOutcome, Vec<ReportItem>), MoveError> {
    let _ = app.emit(STAGE_EVENT, Stage::Scanning);
    let laptop = LaptopId::generate().map_err(|e| TransferError::Protocol(e.to_string()))?;
    let sweep = pctwin_scan::sweep_this_laptop(laptop, scan_stop, &mut |_| {}, None);
    if scan_stop.load(Ordering::Acquire) {
        return Err(Stop::Cancelled.into());
    }
    let roots: HashMap<ItemId, std::path::PathBuf> = sweep
        .placed_by
        .iter()
        .filter_map(|(id, place)| sweep.item_roots.get(place).map(|r| (*id, r.clone())))
        .collect();
    let _ = app.emit(STAGE_EVENT, Stage::Waiting);
    let (sent, _rules) = pctwin_move::send_by_rules(
        &mut link,
        &sweep.record,
        &sweep.census.me,
        |item| {
            let mut path = roots.get(&item.id)?.clone();
            for part in item.path.parts() {
                path.push(part.on_disk());
            }
            Some(path)
        },
        live,
        Arc::new(FsOpener),
    )
    .await?;
    Ok(sent_outcome(&sent, &sweep.record))
}

/// What the new laptop needs before it can take a move: its places approved, the rules, the
/// signed-in person, and each place's real folder to show.
struct NewSide {
    table: Destinations,
    rules: LandingRules,
    me: String,
    shown: BTreeMap<String, String>,
}

/// The new laptop's places, as found here: every special folder approved by its role.
fn new_places(fallback: BTreeMap<FolderRole, String>) -> Result<NewSide, MoveProblem> {
    let me = pctwin_scan::list_people()
        .into_iter()
        .find(|p| p.is_me)
        .map(|p| p.account_id)
        .ok_or(MoveProblem::Failed)?;
    let (table, found) = this_laptops_places()?;
    let roles: BTreeMap<FolderRole, Storage> = found
        .iter()
        .filter_map(|f| match f {
            FolderLookup::Found(f) => Some((f.role, f.storage.clone())),
            FolderLookup::Missing { .. } => None,
        })
        .collect();
    let rules = LandingRules {
        mover: String::new(),
        roles,
        fallback,
        held: BTreeMap::new(),
    };
    let shown = found
        .iter()
        .filter_map(|f| match f {
            FolderLookup::Found(f) => {
                Some((role_label(f.role).to_string(), f.path.display().to_string()))
            }
            FolderLookup::Missing { .. } => None,
        })
        .collect();
    Ok(NewSide {
        table,
        rules,
        me,
        shown,
    })
}

/// This laptop's special folders, each approved as a destination by its role: the only places
/// a move (or tidying up after one) may write.
pub(crate) fn this_laptops_places() -> Result<(Destinations, Vec<FolderLookup>), MoveProblem> {
    let found = pctwin_scan::find_special_folders();
    let mut table = Destinations::new();
    pctwin_transfer::approve_new_places(&mut table, &found).map_err(|_| MoveProblem::Failed)?;
    Ok((table, found))
}

/// Where this laptop's journal of writes is kept (`None`: the app has no data folder).
pub(crate) fn journal_path<R: Runtime>(app: &AppHandle<R>) -> Option<std::path::PathBuf> {
    app.path()
        .app_data_dir()
        .ok()
        .map(|dir| dir.join("journal"))
}

/// The new laptop's side: show the plan, wait for the approval, receive.
async fn run_new<R: Runtime>(
    app: &AppHandle<R>,
    moving: AppMoving,
    mut link: pctwin_link::Link,
    live: &Live,
    setup: NewSide,
    names: Arc<Mutex<HashMap<ItemId, String>>>,
) -> Result<(MoveOutcome, Vec<ReportItem>), MoveError> {
    let _ = app.emit(STAGE_EVENT, Stage::Waiting);
    let NewSide {
        table,
        rules,
        me,
        shown,
    } = setup;
    let path = journal_path(app)
        .ok_or_else(|| TransferError::Protocol("this laptop has no data folder".into()))?;
    if let Some(dir) = path.parent() {
        std::fs::create_dir_all(dir).map_err(|e| TransferError::Protocol(e.to_string()))?;
    }
    let journal =
        pctwin_journal::Journal::open(&path).map_err(|e| TransferError::Protocol(e.to_string()))?;
    // Anything a move left part way is tidied up first (as the engine requires, before anything
    // is received): a finished copy is put in place, a part-copied one kept to carry on.
    pctwin_transfer::recover(&journal, &table)
        .map_err(|e| TransferError::Protocol(e.to_string()))?;
    let mut steps = AppSteps {
        app: app.clone(),
        moving,
        me: me.clone(),
        rules,
        names,
        shown,
    };
    let received = pctwin_move::receive_by_rules(
        &mut link,
        &mut steps,
        |s| s.rules.clone(),
        &table,
        &journal,
        &me,
        live,
    )
    .await?;
    Ok(received_outcome(&received, &steps.rules.mover))
}

/// Runs this laptop's side of the move on a thread of its own, then tells the screens how it
/// ended.
fn spawn_run<R: Runtime>(
    app: AppHandle<R>,
    moving: AppMoving,
    link: pctwin_link::Link,
    paired: PairedWith,
    setup: Option<NewSide>,
    live: Live,
    scan_stop: Arc<AtomicBool>,
) -> Result<(), MoveProblem> {
    let runtime = tokio::runtime::Handle::current();
    let laptops = Laptops {
        old: paired.label.as_ref().map(LabelView::from),
    };
    let names = Arc::new(Mutex::new(HashMap::new()));
    let done = Arc::new(AtomicBool::new(false));
    tick(
        app.clone(),
        live.clone(),
        laptops,
        names.clone(),
        done.clone(),
    );
    std::thread::Builder::new()
        .name("pctwin-move".into())
        .spawn(move || {
            // Held for the whole move: the laptop does not sleep while idle (the person's own
            // sleep, closing the lid, is never blocked). On battery the screen stays on too, or
            // some laptops stop honouring the request.
            let _awake = pctwin_power::stay_awake(
                "Moving files with PCTwin",
                pctwin_power::power_now().on_battery,
            );
            let ran = runtime.block_on(async {
                match setup {
                    Some(setup) => run_new(&app, moving.clone(), link, &live, setup, names).await,
                    None => run_old(&app, link, &live, &scan_stop).await,
                }
            });
            done.store(true, Ordering::Release);
            match ran {
                Ok((outcome, report)) => {
                    moving.finish(report);
                    let _ = app.emit(FINISHED_EVENT, outcome);
                }
                Err(e) => {
                    moving.finish(Vec::new());
                    let _ = app.emit(PROBLEM_EVENT, EndedEarly::of(&e));
                }
            }
        })
        .map(|_| ())
        .map_err(|_| MoveProblem::Failed)
}

fn check_fallback(fallback: &BTreeMap<FolderRole, String>) -> Result<(), MoveProblem> {
    let fine = fallback.values().all(|name| {
        !name.is_empty() && name.len() <= MAX_FALLBACK && !name.chars().any(char::is_control)
    });
    if fine {
        Ok(())
    } else {
        Err(MoveProblem::Failed)
    }
}

/* ---------- Commands ---------- */

/// Starts this laptop's side of the move with the paired link. `fallback`: for each folder the new
/// laptop may lack, the folder in the home that stands for it, in the person's language.
#[tauri::command]
pub async fn move_begin<R: Runtime>(
    app: AppHandle<R>,
    pairing: State<'_, AppPairing>,
    moving: State<'_, AppMoving>,
    fallback: BTreeMap<FolderRole, String>,
) -> Result<(), MoveProblem> {
    check_fallback(&fallback)?;
    let (live, scan_stop) = moving.begin()?;
    let Some((link, paired)) = pairing.take_link().await else {
        moving.finish(Vec::new());
        return Err(MoveProblem::NotPaired);
    };
    let setup = if paired.old {
        None
    } else {
        match new_places(fallback) {
            Ok(setup) => Some(setup),
            Err(p) => {
                moving.finish(Vec::new());
                return Err(p);
            }
        }
    };
    let moving = Arc::clone(&moving);
    spawn_run(app, moving, link, paired, setup, live, scan_stop)
}

/// The person's answer to the plan on screen (M23), for exactly that revision.
#[tauri::command]
pub fn move_approve(
    moving: State<'_, AppMoving>,
    plan: PlanKey,
    approve: bool,
) -> Result<(), MoveProblem> {
    moving.answer(&plan, approve)
}

/// Pauses at the next safe point (X05).
#[tauri::command]
pub fn move_pause(moving: State<'_, AppMoving>) -> Result<(), MoveProblem> {
    moving.pause(true)
}

/// Carries on a paused move, or withdraws a pause not yet reached.
#[tauri::command]
pub fn move_resume(moving: State<'_, AppMoving>) -> Result<(), MoveProblem> {
    moving.pause(false)
}

/// Stops the rest of the move (X10). Not undo: what is copied and checked stays.
#[tauri::command]
pub fn move_stop(moving: State<'_, AppMoving>) -> Result<(), MoveProblem> {
    moving.stop()
}

/// The finished move's report (R02): the mover's own and the shared files.
#[tauri::command]
pub fn move_report(moving: State<'_, AppMoving>) -> Vec<ReportItem> {
    moving.report()
}

#[cfg(test)]
mod tests {
    use super::*;
    use pctwin_gate::Finished;
    use pctwin_move::ReceivedFile;
    use pctwin_record::{ItemKind, ItemName, ItemPath, ManagedBy, Place, Portability};
    use pctwin_transfer::Failure;

    fn laptop() -> LaptopId {
        LaptopId::from_hex("00112233445566778899aabbccddeeff").unwrap()
    }

    fn item(owner: Owner, role: FolderRole, path: &[&str], size: u64) -> Item {
        let place = Place {
            role,
            storage: Storage::SystemDrive,
        };
        let path = ItemPath::new(path.iter().map(|p| ItemName::from_text(p)).collect()).unwrap();
        Item {
            id: ItemId::derive(&laptop(), &owner, &place, &path),
            kind: ItemKind::File,
            owner,
            place,
            path,
            size_bytes: size,
            modified_ns: None,
            portability: Portability::Portable,
            managed_by: ManagedBy::Personal,
            download_bytes: None,
            inclusion: Inclusion::Included,
        }
    }

    fn person(id: &str) -> Owner {
        Owner::Person {
            account_id: id.into(),
        }
    }

    fn finished(path: &str, from: CheckedFrom) -> ReceiveOutcome {
        ReceiveOutcome::Finished(Finished {
            final_path: path.into(),
            sent_path: path.into(),
            changes: Vec::new(),
            mark_not_kept: None,
            checked_from: Some(from),
        })
    }

    fn received(files: Vec<(Item, Option<ReceiveOutcome>)>, ended: Ended) -> Received {
        let mut record = Record::new(laptop());
        record.items = files.iter().map(|(i, _)| i.clone()).collect();
        Received {
            record,
            files: files
                .into_iter()
                .map(|(i, outcome)| ReceivedFile {
                    item: i.id,
                    outcome,
                    changed_since_scan: false,
                })
                .collect(),
            ended,
        }
    }

    #[test]
    fn another_persons_files_are_never_named_or_counted_for_the_mover() {
        let mine = item(person("1000"), FolderRole::Documents, &["a.txt"], 10);
        let shared = item(Owner::Shared, FolderRole::Public, &["menu.txt"], 5);
        let bea = item(person("1001"), FolderRole::Pictures, &["secret.jpg"], 7);
        let r = received(
            vec![
                (mine, Some(finished("a.txt", CheckedFrom::Drive))),
                (
                    shared,
                    Some(ReceiveOutcome::AlreadyThere("menu.txt".into())),
                ),
                (bea, Some(finished("secret.jpg", CheckedFrom::Drive))),
            ],
            Ended::Finished,
        );
        let (outcome, report) = received_outcome(&r, "1000");
        assert_eq!(outcome.by_status.copied, 2);
        assert_eq!(outcome.bytes_checked, 15);
        assert_eq!(report.len(), 2);
        let text = serde_json::to_string(&report).unwrap();
        assert!(!text.contains("secret"), "{text}");
        assert_eq!(report[0].evidence, Some(Evidence::Drive));
        assert_eq!(report[1].evidence, Some(Evidence::AlreadyThere));
        assert_eq!(report[0].source, "documents/a.txt");
        assert_eq!(report[0].name, "a.txt");
    }

    #[test]
    fn a_stopped_move_counts_what_never_finished_as_not_copied() {
        let a = item(person("1000"), FolderRole::Documents, &["a.txt"], 10);
        let b = item(person("1000"), FolderRole::Documents, &["b.txt"], 10);
        let c = item(person("1000"), FolderRole::Documents, &["c.txt"], 10);
        let stopped = ReceiveOutcome::Failed(Failure::new(Cause::Stopped, "stopped"));
        let r = received(
            vec![
                (a, Some(finished("a.txt", CheckedFrom::Drive))),
                (b, Some(stopped)),
                (c, None),
            ],
            Ended::StoppedHere,
        );
        let (outcome, report) = received_outcome(&r, "1000");
        assert!(outcome.stopped);
        assert_eq!(outcome.by_status.copied, 1);
        assert_eq!(outcome.not_copied, 2);
        assert_eq!(report.len(), 1);
    }

    #[test]
    fn a_file_with_no_outcome_in_a_finished_move_failed() {
        let a = item(person("1000"), FolderRole::Documents, &["a.txt"], 10);
        let r = received(vec![(a, None)], Ended::Finished);
        let (outcome, report) = received_outcome(&r, "1000");
        assert_eq!(outcome.by_status.failed, 1);
        assert_eq!(report[0].status, ItemStatus::Failed);
        assert_eq!(report[0].reason, Some(Reason::NotVerified));
    }

    #[test]
    fn every_cause_gets_a_reason_that_fits_its_status() {
        for cause in [
            Cause::NoSpace,
            Cause::TooBigForDrive,
            Cause::ChangedDuringMove,
            Cause::Unreadable,
            Cause::NoLongerThere,
            Cause::InUse,
            Cause::OnlineOnly,
            Cause::NotVerified,
            Cause::RefusedBySafetyCheck,
            Cause::NameNotAllowed,
            Cause::CouldNotWrite,
            Cause::RemovedAfterCopy,
            Cause::Stopped,
            Cause::NotAccepted,
            Cause::MoveEnded,
        ] {
            let (status, reason) = status_of(cause);
            assert_eq!(reason.status(), status, "{cause:?}");
        }
        assert_eq!(
            status_of(Cause::OnlineOnly),
            (ItemStatus::NeedsAction, Reason::OnlineOnly)
        );
    }

    #[test]
    fn an_answer_counts_only_for_the_plan_on_screen() {
        let moving = Moving::default();
        moving.begin().unwrap();
        let key = PlanKey {
            revision: 2,
            fingerprint: "ab".into(),
        };
        let mut rx = moving.wait_for_answer(key.clone());
        let stale = PlanKey {
            revision: 1,
            fingerprint: "ab".into(),
        };
        assert_eq!(moving.answer(&stale, true), Err(MoveProblem::PlanChanged));
        assert!(rx.try_recv().is_err(), "no answer yet");
        moving.answer(&key, true).unwrap();
        assert_eq!(rx.try_recv(), Ok(true));
        // Answered once only.
        assert_eq!(moving.answer(&key, true), Err(MoveProblem::PlanChanged));
    }

    #[test]
    fn stopping_before_the_approval_declines_the_plan() {
        let moving = Moving::default();
        let (live, scan_stop) = moving.begin().unwrap();
        let mut rx = moving.wait_for_answer(PlanKey {
            revision: 1,
            fingerprint: "ab".into(),
        });
        moving.stop().unwrap();
        assert_eq!(rx.try_recv(), Ok(false));
        assert!(live.control.stopped());
        assert!(scan_stop.load(Ordering::Acquire));
    }

    #[test]
    fn one_move_at_a_time_and_none_after_it_ends() {
        let moving = Moving::default();
        assert_eq!(moving.pause(true), Err(MoveProblem::NotMoving));
        moving.begin().unwrap();
        assert!(matches!(moving.begin(), Err(MoveProblem::Busy)));
        moving.finish(Vec::new());
        assert_eq!(moving.stop(), Err(MoveProblem::NotMoving));
        assert!(
            moving.begin().is_ok(),
            "a new move may start once one ended"
        );
    }

    #[test]
    fn a_group_is_named_by_its_role_or_by_its_own_name() {
        assert_eq!(
            group_name("documents"),
            GroupName::Role {
                role: FolderRole::Documents
            }
        );
        assert_eq!(
            group_name("for-1"),
            GroupName::Folder {
                name: "for-1".into()
            }
        );
    }

    #[test]
    fn a_pause_asked_for_reads_pausing_until_it_holds() {
        let laptops = Laptops { old: None };
        let names = HashMap::new();
        let mut snap = pctwin_transfer::MoveMeter::default().snapshot();
        let state = |s: &pctwin_transfer::MeterSnapshot, asked| {
            progress_view(s, asked, None, None, &laptops, &names).state
        };
        assert_eq!(state(&snap, false), RunState::Running);
        assert_eq!(state(&snap, true), RunState::Pausing);
        snap.reading.paused_here = true;
        assert_eq!(state(&snap, true), RunState::Paused);
        snap.reading.paused_here = false;
        snap.reading.paused_there = true;
        assert_eq!(state(&snap, false), RunState::Paused);
        let braked = progress_view(
            &snap,
            false,
            Some("pictures".into()),
            Some(Notice::LowBattery { paused: false }),
            &laptops,
            &names,
        );
        assert_eq!(
            braked.notice,
            Some(Notice::DriveProblem {
                place: GroupName::Role {
                    role: FolderRole::Pictures
                }
            })
        );
    }

    fn preview() -> Preview {
        Preview {
            fit: pctwin_transfer::Fit { places: Vec::new() },
            problems: pctwin_move::Problems::default(),
            bytes: 0,
            bytes_per_second: None,
            time: pctwin_transfer::TimeRange::NotKnown,
        }
    }

    fn rules() -> LandingRules {
        LandingRules {
            mover: "1000".into(),
            roles: BTreeMap::from([
                (FolderRole::Documents, Storage::SystemDrive),
                (FolderRole::Home, Storage::SystemDrive),
            ]),
            fallback: BTreeMap::from([(FolderRole::Music, "From your old laptop/Music".into())]),
            held: BTreeMap::new(),
        }
    }

    #[test]
    fn each_group_shows_the_real_folder_it_lands_in() {
        let shown = BTreeMap::from([
            (
                "documents".to_string(),
                "C:/Users/Ada/Documents".to_string(),
            ),
            ("home".to_string(), "C:/Users/Ada".to_string()),
        ]);
        let mut record = Record::new(laptop());
        record.items = vec![
            item(
                person("1000"),
                FolderRole::Documents,
                &["Letters", "a.txt"],
                10,
            ),
            item(person("1000"), FolderRole::Documents, &["b.txt"], 5),
            item(person("1000"), FolderRole::Music, &["song.mp3"], 7),
            item(person("1001"), FolderRole::Pictures, &["secret.jpg"], 9),
        ];
        let mut bea_left = item(person("1001"), FolderRole::Videos, &["v.mp4"], 1);
        bea_left.inclusion = Inclusion::LeftOut {
            reason: LeftOutReason::CloudOnly,
        };
        let mut mine_left = item(person("1000"), FolderRole::Videos, &["w.mp4"], 1);
        mine_left.inclusion = Inclusion::LeftOut {
            reason: LeftOutReason::CloudOnly,
        };
        record.items.extend([bea_left, mine_left]);
        let view = plan_view(&record, &preview(), &rules(), &shown);
        let to: Vec<(&str, &str)> = view
            .groups
            .iter()
            .map(|g| (g.id.as_str(), g.to_folder.as_str()))
            .collect();
        assert_eq!(
            to,
            [
                ("documents", "C:/Users/Ada/Documents"),
                ("music", "C:/Users/Ada/From your old laptop/Music"),
            ]
        );
        assert_eq!(
            view.groups[0].size,
            Size::Known {
                bytes: 15,
                files: 2
            }
        );
        // Bea's picture and her left-out video are neither listed nor counted.
        assert_eq!(
            view.left_out,
            [LeftOut {
                reason: LeftOutReason::CloudOnly,
                count: 1
            }]
        );
        assert!(!serde_json::to_string(&view).unwrap().contains("secret"));
    }

    #[test]
    fn a_low_battery_warns_then_pauses_and_a_drive_problem_comes_first() {
        use pctwin_power::BatteryAdvice;
        assert_eq!(battery_notice(BatteryAdvice::Fine), None);
        assert_eq!(battery_notice(BatteryAdvice::OnBattery), None);
        assert_eq!(
            battery_notice(BatteryAdvice::PlugIn),
            Some(Notice::LowBattery { paused: false })
        );
        assert_eq!(
            battery_notice(BatteryAdvice::PauseNow),
            Some(Notice::LowBattery { paused: true })
        );
        let snap = pctwin_transfer::MoveMeter::default().snapshot();
        let laptops = Laptops { old: None };
        let low = Some(Notice::LowBattery { paused: false });
        let view = progress_view(&snap, false, None, low.clone(), &laptops, &HashMap::new());
        assert_eq!(view.notice, low);
    }

    #[test]
    fn a_fallback_name_must_be_a_plain_short_name() {
        let ok = BTreeMap::from([(FolderRole::Music, "From your old laptop/Music".to_string())]);
        assert!(check_fallback(&ok).is_ok());
        let long = "x".repeat(MAX_FALLBACK + 1);
        for bad in ["", "a\nb", long.as_str()] {
            let map = BTreeMap::from([(FolderRole::Music, bad.to_string())]);
            assert!(check_fallback(&map).is_err(), "{bad:?}");
        }
    }
}
