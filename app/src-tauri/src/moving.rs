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
/// The new laptop: the other people in the move, to place (M06).
pub const PEOPLE_EVENT: &str = "move-people";
/// The old laptop: how much of the mover's own things the scan has found so far (M02). Only
/// ever grows while the scan runs.
pub const FOUND_EVENT: &str = "move-found";

/// What the scan has found of the mover's own things so far.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
pub struct Found {
    pub bytes: u64,
}

/// Both laptops, as soon as the move starts: what each laptop is (C06) and the connection
/// measured (C07). Also when their PCTwin versions differ, before the move ends.
pub const PEER_EVENT: &str = "move-peer";

/// The connection during the move, once the plan is approved: waiting for the other laptop after
/// a drop (C09), connected again, or ended for good (C10).
pub const LINK_EVENT: &str = "move-link";

pctwin_types::listed_enum! {
    /// The connection during the move, as C09 and C10 show it. Every number is measured.
    #[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
    #[serde(tag = "kind", rename_all = "kebab-case")]
    pub enum ConnectionView {
        /// Connected again: the move carries on.
        Connected,
        /// The connection dropped and the move waits to carry on over a new one, trying by
        /// itself (C09). `nothing_new`: the tries keep bringing nothing, so the screen says so,
        /// with Stop. `last_seen_seconds`: since the other laptop was last heard from. What is
        /// copied and checked so far, of the plan's size.
        #[serde(rename_all = "camelCase")]
        Waiting {
            nothing_new: bool,
            last_seen_seconds: u64,
            saved_bytes: u64,
            total_bytes: u64,
        },
        /// The move ended because the laptops could not find each other again without pairing
        /// again (C10). What moved is in the report; `moved_something`: there is anything to see.
        #[serde(rename_all = "camelCase")]
        Ended {
            moved_something: bool,
            saved_bytes: u64,
            total_bytes: u64,
        },
    }
}

/// What C09 and C10 say is kept: the bytes copied and checked so far, and the plan's size.
fn saved(meter: &pctwin_transfer::MoveMeter) -> (u64, u64) {
    let r = meter.reading();
    (
        r.bytes_checked.saturating_add(r.bytes_already_there),
        r.bytes_planned,
    )
}

/// This laptop and the other one, as each said when the move started (C06), and the link the new
/// laptop measured (C07).
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct Peer {
    pub mine: pctwin_move::greeting::About,
    pub theirs: pctwin_move::greeting::About,
    /// The other laptop's label as pairing showed it, when known.
    pub other: Option<LabelView>,
    pub versions: Versions,
    /// `None`: not measured (or the versions differ).
    pub link: Option<LinkView>,
}

/// Whether the two laptops' PCTwin versions can move together, and if not, which to update.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "kebab-case")]
pub enum Versions {
    Same,
    UpdateHere,
    UpdateThere,
}

/// This laptop, as it tells the other one: its power, and the free space in the home folder.
fn about_here() -> pctwin_move::greeting::About {
    use pctwin_move::greeting::Power;
    let now = pctwin_power::power_now();
    let power = match (now.on_battery, now.percent) {
        (false, _) => Power::Plugged,
        // Whole percents, 0 to 100.
        #[allow(clippy::cast_possible_truncation, clippy::cast_sign_loss)]
        (true, Some(p)) => Power::Battery {
            percent: p.round().clamp(0.0, 100.0) as u8,
        },
        (true, None) => Power::Unknown,
    };
    let free = std::env::home_dir()
        .and_then(|home| pctwin_gate::Destination::open(&home).ok())
        .and_then(|home| home.available_bytes());
    pctwin_move::greeting::About::here(power, free)
}

/// Both laptops, the moment the move starts and before anything is looked for: each says what
/// it is, they agree the protocol and the new laptop measures the link; both screens show it
/// (C06, C07). PCTwin versions that cannot move together are shown too, naming the one to update.
async fn greet<'l, R: Runtime>(
    app: &AppHandle<R>,
    link: &'l mut pctwin_link::Link,
    side: pctwin_move::greeting::Side,
    other: Option<LabelView>,
) -> Result<pctwin_move::greeting::Greeted<'l, pctwin_link::Link>, TransferError> {
    let mine = about_here();
    let peer = |theirs, versions, link| Peer {
        mine: mine.clone(),
        theirs,
        other: other.clone(),
        versions,
        link,
    };
    match pctwin_move::greeting::greet(link, &mine, side).await {
        Ok(greeted) => {
            let g = greeted.greeting();
            let measured = g.bytes_per_second.map(|b| LinkView {
                bytes_per_second: b,
                slow: b < SLOW_LINK,
            });
            let _ = app.emit(PEER_EVENT, peer(g.theirs.clone(), Versions::Same, measured));
            Ok(greeted)
        }
        Err(not) => {
            if let (Some(theirs), TransferError::VersionGap(gap)) = (not.theirs, &not.error) {
                let versions = match gap {
                    pctwin_transfer::VersionGap::ThisLaptopOlder => Versions::UpdateHere,
                    pctwin_transfer::VersionGap::OtherLaptopOlder => Versions::UpdateThere,
                };
                let _ = app.emit(PEER_EVENT, peer(theirs, versions, None));
            }
            Err(not.error)
        }
    }
}

/// The old laptop (a Mac): some of the mover's places were kept private by the Mac's privacy
/// settings; the Full Disk Access guide is shown.
pub const PRIVACY_EVENT: &str = "move-privacy";

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
    waiting: Option<(
        PlanKey,
        std::collections::BTreeSet<FolderRole>,
        oneshot::Sender<bool>,
    )>,
    /// The new laptop: the folders the person left out when they approved.
    left_out: std::collections::BTreeSet<FolderRole>,
    /// The new laptop: the folders the person sent to a place they chose (M05), each opened
    /// when they chose it, with the folder as the system showed it.
    chosen: BTreeMap<FolderRole, (pctwin_gate::Destination, String)>,
    /// The new laptop: the people on screen to place, this laptop's accounts, and where the
    /// person's choices go.
    placing: Option<Placing>,
    /// The old laptop: the person's answer to the Full Disk Access guide (true: look again).
    privacy: Option<oneshot::Sender<bool>>,
    report: Vec<ReportItem>,
    finished: bool,
}

/// The people the new laptop asked the mover to place.
struct Placing {
    asked: std::collections::BTreeSet<String>,
    accounts: Vec<crate::others::AccountHere>,
    answer: oneshot::Sender<Placed>,
}

/// The mover's answer: where each other person's things go, and the words of the window that
/// shows a new account's password.
type Placed = (
    BTreeMap<String, crate::others::PlaceChoice>,
    crate::others::PasswordWords,
);

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
            left_out: std::collections::BTreeSet::new(),
            chosen: BTreeMap::new(),
            placing: None,
            privacy: None,
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

    fn wait_for_privacy(&self) -> oneshot::Receiver<bool> {
        let (tx, rx) = oneshot::channel();
        if let Some(run) = lock(&self.run).as_mut() {
            run.privacy = Some(tx);
        }
        rx
    }

    /// The person's answer to the Full Disk Access guide: look again, or go on without them.
    fn privacy_answer(&self, again: bool) -> Result<(), MoveProblem> {
        self.with_run(|run| match run.privacy.take() {
            Some(tx) => {
                let _ = tx.send(again);
                Ok(())
            }
            None => Err(MoveProblem::NotMoving),
        })?
    }

    fn wait_for_answer(
        &self,
        key: PlanKey,
        groups: std::collections::BTreeSet<FolderRole>,
    ) -> oneshot::Receiver<bool> {
        let (tx, rx) = oneshot::channel();
        if let Some(run) = lock(&self.run).as_mut() {
            run.waiting = Some((key, groups, tx));
        }
        rx
    }

    /// The person's answer to the plan on screen: only for exactly that revision, and leaving
    /// out only folders that plan shows.
    fn answer(&self, key: &PlanKey, yes: bool, left_out: &[FolderRole]) -> Result<(), MoveProblem> {
        self.with_run(|run| match run.waiting.take() {
            Some((shown, groups, tx)) if shown == *key => {
                if left_out.iter().any(|r| !groups.contains(r)) {
                    run.waiting = Some((shown, groups, tx));
                    return Err(MoveProblem::PlanChanged);
                }
                run.left_out = left_out.iter().copied().collect();
                let _ = tx.send(yes);
                Ok(())
            }
            other => {
                run.waiting = other;
                Err(MoveProblem::PlanChanged)
            }
        })?
    }

    /// Whether a place may be chosen for the folders of `role` now: only while a plan showing
    /// them waits for the person's answer.
    fn may_choose(&self, role: FolderRole) -> Result<(), MoveProblem> {
        self.with_run(|run| match &run.waiting {
            Some((_, groups, _)) if groups.contains(&role) => Ok(()),
            _ => Err(MoveProblem::PlanChanged),
        })?
    }

    /// The person chose `place` (opened just now) for the folders of `role`, or the usual
    /// folder again (`None`). Only while the plan showing them waits for an answer.
    fn choose(
        &self,
        role: FolderRole,
        place: Option<(pctwin_gate::Destination, String)>,
    ) -> Result<(), MoveProblem> {
        self.with_run(|run| match &run.waiting {
            Some((_, groups, _)) if groups.contains(&role) => {
                match place {
                    Some(place) => run.chosen.insert(role, place),
                    None => run.chosen.remove(&role),
                };
                Ok(())
            }
            _ => Err(MoveProblem::PlanChanged),
        })?
    }

    /// The places chosen, once the plan is approved, for the folders not left out.
    fn take_chosen(&self) -> BTreeMap<FolderRole, (pctwin_gate::Destination, String)> {
        lock(&self.run)
            .as_mut()
            .map(|r| {
                let mut chosen = std::mem::take(&mut r.chosen);
                chosen.retain(|role, _| !r.left_out.contains(role));
                chosen
            })
            .unwrap_or_default()
    }

    fn left_out(&self) -> std::collections::BTreeSet<FolderRole> {
        lock(&self.run)
            .as_ref()
            .map(|r| r.left_out.clone())
            .unwrap_or_default()
    }

    fn wait_for_places(
        &self,
        asked: std::collections::BTreeSet<String>,
        accounts: Vec<crate::others::AccountHere>,
    ) -> oneshot::Receiver<Placed> {
        let (tx, rx) = oneshot::channel();
        if let Some(run) = lock(&self.run).as_mut() {
            run.placing = Some(Placing {
                asked,
                accounts,
                answer: tx,
            });
        }
        rx
    }

    /// The mover's places for the other people, checked against exactly who was asked.
    fn place(
        &self,
        choices: &[crate::others::PersonChoice],
        words: crate::others::PasswordWords,
    ) -> Result<(), MoveProblem> {
        if !words.fine() {
            return Err(MoveProblem::Failed);
        }
        self.with_run(|run| {
            let placing = run.placing.take().ok_or(MoveProblem::PlanChanged)?;
            match crate::others::checked_choices(&placing.asked, &placing.accounts, choices) {
                Ok(checked) => {
                    let _ = placing.answer.send((checked, words));
                    Ok(())
                }
                Err(p) => {
                    run.placing = Some(placing);
                    Err(p)
                }
            }
        })?
    }

    /// "Try now" while waiting for the other laptop (C09).
    fn retry(&self) -> Result<(), MoveProblem> {
        self.with_run(|run| run.live.connection.try_now())
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
            if let Some((_, _, tx)) = run.waiting.take() {
                let _ = tx.send(false);
            }
            // Dropping it ends the wait for the people's places: the move stops there.
            run.placing = None;
            run.privacy = None;
            run.live.control.stop();
        })
    }

    fn finish(&self, report: Vec<ReportItem>) {
        if let Some(run) = lock(&self.run).as_mut() {
            run.report = report;
            run.finished = true;
            run.waiting = None;
            run.placing = None;
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
        /// What was chosen holds more files than one send can carry (Quick Send): choose less.
        TooManyFiles,
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
        /// The old laptop is asking for an administrator's approval to move the other people
        /// ticked (M25): the system's own prompt is up.
        Administrator,
        /// The other people ticked are not moved this time, and why; the mover's things still
        /// move.
        OthersNotMoved { why: OthersWhy },
    }
}

/// Why the other people ticked were not moved.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "kebab-case")]
pub enum OthersWhy {
    /// The administrator said no at the system's prompt.
    NotApproved,
    /// The helper could not be started or reached.
    Failed,
    /// This system cannot move other people in this version.
    NotHere,
    /// A Mac: an administrator must first allow PCTwin in System Settings, Login Items (the
    /// page was opened); the next move can then take them.
    AllowInSettings,
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
        /// PCTwin on this laptop is older than on the other one: update it here (C06).
        UpdateHere,
        /// PCTwin on the other laptop is older: update it there (C06).
        UpdateThere,
    }
}

impl EndedEarly {
    fn of(e: &MoveError) -> Self {
        match e {
            MoveError::Stopped(_) => Self::NotApproved,
            MoveError::Transfer(TransferError::VersionGap(gap)) => match gap {
                pctwin_transfer::VersionGap::ThisLaptopOlder => Self::UpdateHere,
                pctwin_transfer::VersionGap::OtherLaptopOlder => Self::UpdateThere,
            },
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

/// The connection the new laptop measured as the laptops greeted (C07).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct LinkView {
    pub bytes_per_second: u64,
    /// Below [`SLOW_LINK`]: the person is told how to make it faster.
    pub slow: bool,
}

/// Below this, about 40 megabits a second, a move of a whole laptop takes hours: a cable, or
/// both laptops near the router, is worth suggesting.
pub const SLOW_LINK: u64 = 5_000_000;

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct PlanView {
    pub revision: u64,
    pub fingerprint: String,
    pub groups: Vec<PlanGroup>,
    pub left_out: Vec<LeftOut>,
    pub preview: PreviewView,
    /// The groups whose things move first ("we'll move these first"), in the order they start,
    /// from the very order the move sends in ([`pctwin_move::move_order`]).
    pub first: Vec<FolderRole>,
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
    // The mover's own groups that hold something moving first, in the order they start.
    let by_id: HashMap<ItemId, &Item> = record.items.iter().map(|i| (i.id, i)).collect();
    let mut first: Vec<FolderRole> = Vec::new();
    for (id, tier) in pctwin_move::move_order(record) {
        let Some(item) = by_id.get(&id) else { continue };
        if pctwin_move::moves_first(tier)
            && shown_to_mover(item, mover)
            && groups.contains_key(&item.place.role)
            && !first.contains(&item.place.role)
        {
            first.push(item.place.role);
        }
    }
    PlanView {
        first,
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

/// Another person in the move: how their things ended, counted, never named file by file.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct PersonOutcome {
    pub name: String,
    pub copied: u64,
    pub needs_action: u64,
    pub failed: u64,
    /// Their things are handed over when they next sign in on the new laptop (M32).
    pub at_sign_in: bool,
}

#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct MoveOutcome {
    /// The other people in the move, one line each.
    pub people: Vec<PersonOutcome>,
    pub by_status: ByStatus,
    pub not_copied: u64,
    pub bytes_checked: u64,
    pub stopped: bool,
    /// The move ended early because something went wrong between the laptops (not a person's
    /// stop): R01 says so, never "you stopped the move".
    pub broke_off: bool,
    pub checked_from_memory: u64,
    /// The after-move check: the mover's photos, videos, music and documents in the plan, and how
    /// many of each are copied and checked.
    pub after_move: AfterMove,
}

/// One kind of file in the after-move check: how many the approved plan held, and how many of
/// those are on the new laptop, copied and checked (or the same file already there).
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize)]
pub struct KindCount {
    pub planned: u64,
    pub copied: u64,
}

/// What matters to people, counted after the move the way the scan counts it (by each file's
/// kind, [`pctwin_scan::category`]). Other kinds of file are in the totals, not here.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize)]
pub struct AfterMove {
    pub photos: KindCount,
    pub videos: KindCount,
    pub music: KindCount,
    pub documents: KindCount,
}

impl AfterMove {
    /// Counts one planned file of the mover's, by its name, and whether it is copied and checked.
    fn add(&mut self, name: &str, copied: bool) {
        let kind = match pctwin_scan::category(name) {
            pctwin_scan::Category::Photo => &mut self.photos,
            pctwin_scan::Category::Video => &mut self.videos,
            pctwin_scan::Category::Music => &mut self.music,
            pctwin_scan::Category::Document => &mut self.documents,
            pctwin_scan::Category::Other => return,
        };
        kind.planned += 1;
        if copied {
            kind.copied += 1;
        }
    }
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
    /// What else the report says of this line, beside its result.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub note: Option<ReportNote>,
}

/// A word the report adds to a line ("Changed since the scan", Product Spec, 16 October 2026).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "kebab-case")]
pub enum ReportNote {
    /// It changed on the old laptop after the scan, so it was sent as it is now.
    ChangedSinceScan,
    /// It was gone from the old laptop by the time it was to be sent.
    NoLongerThere,
    /// The drive would not take the date it was last changed, so it shows the date it was copied.
    TimeNotKept,
}

impl ReportNote {
    /// The note for a file that did not arrive, by its cause: only one that was gone has one.
    fn of_failure(f: &pctwin_transfer::Failure) -> Option<Self> {
        (f.cause() == Cause::NoLongerThere).then_some(Self::NoLongerThere)
    }
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

/// Whether `f` is a file the move's end cut off (`ended`: how it ended), rather than one that
/// failed on its own: it was never copied, and is counted as not copied.
fn cut_off(f: &pctwin_transfer::Failure, ended: Ended) -> bool {
    match ended {
        Ended::Finished => false,
        Ended::StoppedHere | Ended::StoppedThere => f.cause() == Cause::Stopped,
        Ended::LinkLost => f.cause() == Cause::MoveEnded && f.why() == pctwin_move::LINK_LOST,
        Ended::Broke => {
            f.cause() == Cause::MoveEnded && f.why().starts_with(pctwin_move::BROKE_OFF)
        }
    }
}

/// The new laptop's outcome and report for `mover`: their own and the shared files line by line;
/// another person's files are never named or counted here.
fn received_outcome(received: &Received, mover: &str) -> (MoveOutcome, Vec<ReportItem>) {
    let items: HashMap<ItemId, &Item> = received.record.items.iter().map(|i| (i.id, i)).collect();
    let stopped = received.ended != Ended::Finished;
    let mut outcome = MoveOutcome {
        stopped,
        broke_off: received.ended == Ended::Broke,
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
        outcome.after_move.add(
            &name_of(&shown_path(&item.path)),
            matches!(
                file.outcome,
                Some(ReceiveOutcome::Finished(_) | ReceiveOutcome::AlreadyThere(_))
            ),
        );
        let changed = file
            .changed_since_scan
            .then_some(ReportNote::ChangedSinceScan);
        let (status, reason, evidence, destination, note) = match &file.outcome {
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
                    changed.or(f.time_not_kept.then_some(ReportNote::TimeNotKept)),
                )
            }
            Some(ReceiveOutcome::AlreadyThere(p)) => {
                outcome.bytes_checked = outcome.bytes_checked.saturating_add(item.size_bytes);
                (
                    ItemStatus::Copied,
                    None,
                    Some(Evidence::AlreadyThere),
                    Some(p.clone()),
                    changed,
                )
            }
            Some(ReceiveOutcome::NeedsAction { finished, .. }) => (
                ItemStatus::NeedsAction,
                Some(Reason::FinishByHand),
                None,
                Some(finished.final_path.clone()),
                changed,
            ),
            Some(ReceiveOutcome::Failed(f)) if cut_off(f, received.ended) => {
                outcome.not_copied += 1;
                continue;
            }
            Some(ReceiveOutcome::Failed(f)) => {
                let (status, reason) = status_of(f.cause());
                (status, Some(reason), None, None, ReportNote::of_failure(f))
            }
            None if stopped => {
                outcome.not_copied += 1;
                continue;
            }
            None => (
                ItemStatus::Failed,
                Some(Reason::NotVerified),
                None,
                None,
                None,
            ),
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
            note,
        });
    }
    let wholes = whole_lines(&received.record, mover, &report, &mut outcome);
    report.extend(wholes);
    (outcome, report)
}

/// A line for each folder that moves as one whole (a Photos Library, a project's history) shown
/// to `mover`: moved only when every file the plan had inside it was copied; otherwise it needs
/// action, and its missing files are on their own lines with why. Built from the files' lines in
/// `report`; one with none of its files reported (all of them skipped or cut off) has no line of
/// its own unless something inside it was copied.
fn whole_lines(
    record: &Record,
    mover: &str,
    report: &[ReportItem],
    outcome: &mut MoveOutcome,
) -> Vec<ReportItem> {
    let mut lines = Vec::new();
    for whole in record.items.iter().filter(|i| {
        i.kind == pctwin_record::ItemKind::Package
            && i.inclusion == Inclusion::Included
            && shown_to_mover(i, mover)
    }) {
        let depth = whole.path.parts().len();
        let inside = |i: &&Item| {
            i.kind == pctwin_record::ItemKind::File
                && i.inclusion == Inclusion::Included
                && i.owner == whole.owner
                && i.place == whole.place
                && i.path.parts().len() > depth
                && i.path.parts()[..depth] == *whole.path.parts()
        };
        let planned: Vec<String> = record
            .items
            .iter()
            .filter(inside)
            .map(|i| i.id.to_hex())
            .collect();
        if planned.is_empty() {
            continue;
        }
        let lines_inside: Vec<&ReportItem> =
            report.iter().filter(|r| planned.contains(&r.id)).collect();
        let copied = lines_inside
            .iter()
            .filter(|r| r.status == ItemStatus::Copied)
            .count();
        if copied == 0
            && lines_inside
                .iter()
                .all(|r| r.status == ItemStatus::SkippedByUser)
        {
            continue;
        }
        let source = source_of(whole);
        let all = copied == planned.len();
        // Where it landed: the folder its first copied file is in, at the whole's depth.
        let destination = lines_inside
            .iter()
            .find(|r| r.status == ItemStatus::Copied)
            .and_then(|r| {
                let rest = r.source.strip_prefix(&format!("{source}/"))?;
                r.destination
                    .as_deref()?
                    .strip_suffix(&format!("/{rest}"))
                    .map(str::to_string)
            });
        let (status, reason) = if all {
            (ItemStatus::Copied, None)
        } else {
            (ItemStatus::NeedsAction, Some(Reason::PartsMissing))
        };
        outcome.by_status.add(status);
        lines.push(ReportItem {
            id: whole.id.to_hex(),
            name: name_of(&source),
            source,
            destination,
            status,
            reason,
            evidence: None,
            note: None,
        });
    }
    lines
}

/// The old laptop's outcome: what it sent and how the new laptop answered. The new laptop's
/// report is the one that counts; this one never says more than was confirmed.
fn sent_outcome(sent: &Sent, record: &Record, mover: &str) -> (MoveOutcome, Vec<ReportItem>) {
    // Another person's files are neither named nor counted for the mover.
    let items: HashMap<ItemId, &Item> = record
        .items
        .iter()
        .filter(|i| shown_to_mover(i, mover))
        .map(|i| (i.id, i))
        .collect();
    let stopped = sent.ended != Ended::Finished;
    let mut outcome = MoveOutcome {
        stopped,
        broke_off: sent.ended == Ended::Broke,
        ..MoveOutcome::default()
    };
    let mut report = Vec::new();
    for file in &sent.files {
        let Some(item) = items.get(&file.item) else {
            continue;
        };
        outcome.after_move.add(
            &name_of(&shown_path(&item.path)),
            matches!(
                file.outcome,
                Some(SendOutcome::Arrived | SendOutcome::AlreadyThere)
            ),
        );
        let (status, reason, evidence, note) = match &file.outcome {
            Some(SendOutcome::Arrived) => {
                outcome.bytes_checked = outcome.bytes_checked.saturating_add(item.size_bytes);
                (ItemStatus::Copied, None, Some(Evidence::NewLaptop), None)
            }
            Some(SendOutcome::AlreadyThere) => {
                outcome.bytes_checked = outcome.bytes_checked.saturating_add(item.size_bytes);
                (ItemStatus::Copied, None, Some(Evidence::AlreadyThere), None)
            }
            Some(SendOutcome::Failed(f)) if cut_off(f, sent.ended) => {
                outcome.not_copied += 1;
                continue;
            }
            Some(SendOutcome::Failed(f)) => {
                let (s, r) = status_of(f.cause());
                (s, Some(r), None, ReportNote::of_failure(f))
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
            destination: None,
            status,
            reason,
            evidence,
            note,
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
            note: None,
        });
    }
    let wholes = whole_lines(record, mover, &report, &mut outcome);
    report.extend(wholes);
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
    /// The other people, as the old laptop named them.
    named: Vec<pctwin_move::landing::NamedPerson>,
    /// Where the mover put each of them (asked once, for all), and the password window's words.
    placed: Option<Placed>,
    /// The helper, once started for them, and who was placed.
    helper: Option<crate::others::NewHelper>,
    held_for: Vec<crate::others::Placed>,
    /// The person's stop, so starting the helper does not outlast it.
    live_control: pctwin_transfer::MoveControl,
    /// The places the person chose for some folders, approved in the table with the rules.
    chosen: BTreeMap<FolderRole, pctwin_gate::Destination>,
    /// Other people's holding places, so their writes are journaled there, never in the mover's
    /// journal.
    held_places: pctwin_move::held_journal::HeldPlaces,
}

impl<R: Runtime> AppSteps<R> {
    /// Asks the mover, once, where each other person's things go (M06 on the new laptop).
    async fn ask_places(&mut self, record: &Record) -> Result<(), Stop> {
        let accounts = crate::others::accounts_here();
        let people: Vec<PersonToPlace> = self
            .named
            .iter()
            .map(|p| {
                let (files, bytes) = record
                    .items
                    .iter()
                    .filter(|i| matches!(&i.owner, Owner::Person { account_id } if *account_id == p.account_id))
                    .fold((0u64, 0u64), |(f, b), i| (f + 1, b.saturating_add(i.size_bytes)));
                PersonToPlace {
                    id: p.account_id.clone(),
                    name: p.name.clone(),
                    files,
                    bytes,
                }
            })
            .collect();
        let asked = people.iter().map(|p| p.id.clone()).collect();
        let answer = self.moving.wait_for_places(asked, accounts.clone());
        let _ = self.app.emit(
            PEOPLE_EVENT,
            PeopleToPlace {
                people,
                accounts,
                can_place: crate::others::CAN_MOVE_OTHERS,
            },
        );
        self.placed = Some(answer.await.map_err(|_| Stop::Cancelled)?);
        Ok(())
    }

    /// Once the plan is approved: the helper makes the new accounts and a holding place for
    /// each person placed, approved in `table`; the rules then land their things there. If the
    /// administrator says no, or the helper cannot start, they are not moved, and the screens are
    /// told why.
    fn place_others(&mut self, table: &mut Destinations) -> LandingRules {
        // A chosen place that cannot be approved now leaves its files with no place: each is
        // said to have failed, never put somewhere else.
        for (role, place) in std::mem::take(&mut self.chosen) {
            let _ = table.approve_chosen(&pctwin_move::landing::chosen_label(role), place);
        }
        let mut rules = self.rules.clone();
        let Some((choices, words)) = self.placed.clone() else {
            return rules;
        };
        let going: Vec<(
            &pctwin_move::landing::NamedPerson,
            &crate::others::PlaceChoice,
        )> = self
            .named
            .iter()
            .filter_map(|p| Some((p, choices.get(&p.account_id)?)))
            .filter(|(_, c)| **c != crate::others::PlaceChoice::LeaveOut)
            .collect();
        if going.is_empty() {
            return rules;
        }
        let not_moved = |why| {
            let _ = self.app.emit(STAGE_EVENT, Stage::OthersNotMoved { why });
        };
        if !crate::others::CAN_MOVE_OTHERS {
            not_moved(OthersWhy::NotHere);
            return rules;
        }
        // The sign-in names of the new accounts, none already on this laptop.
        let mut taken: std::collections::BTreeSet<String> = crate::others::accounts_here()
            .into_iter()
            .map(|a| a.name.to_lowercase())
            .collect();
        let mut names = BTreeMap::new();
        let mut existing = Vec::new();
        for (p, c) in &going {
            match c {
                crate::others::PlaceChoice::New => {
                    let name = crate::others::login_name(&p.name, &taken);
                    taken.insert(name.clone());
                    names.insert(p.account_id.clone(), name);
                }
                crate::others::PlaceChoice::Existing { account_id } => {
                    existing.push(account_id.clone());
                }
                crate::others::PlaceChoice::LeaveOut => {}
            }
        }
        let _ = self.app.emit(STAGE_EVENT, Stage::Administrator);
        let new_names: Vec<String> = names.values().cloned().collect();
        let control = self.live_control.clone();
        let stopped = move || control.stopped();
        let who: Vec<String> = going.iter().map(|(p, _)| p.name.clone()).collect();
        let prompt = crate::others::approve_prompt(&words.approve, &who).unwrap_or_default();
        let mut helper = match crate::others::NewHelper::start(
            &self.me, &new_names, &existing, &prompt, &stopped,
        ) {
            Ok(crate::others::Started::Helper(h)) => h,
            Ok(crate::others::Started::Refused) => {
                not_moved(OthersWhy::NotApproved);
                return rules;
            }
            Ok(crate::others::Started::AllowInSettings) => {
                not_moved(OthersWhy::AllowInSettings);
                return rules;
            }
            Err(_) => {
                not_moved(OthersWhy::Failed);
                return rules;
            }
        };
        for (n, (p, c)) in going.iter().enumerate() {
            let account = match c {
                crate::others::PlaceChoice::Existing { account_id } => Ok(account_id.clone()),
                crate::others::PlaceChoice::New => match names.get(&p.account_id) {
                    Some(name) => {
                        helper.make_account(name, &crate::others::full_name(&p.name), &words)
                    }
                    None => Err(MoveProblem::Failed),
                },
                crate::others::PlaceChoice::LeaveOut => continue,
            };
            let label = format!("for-{}", n + 1);
            let held = account.and_then(|account| {
                let place = helper.hold_for(&account)?;
                table
                    .approve_holding(&label, &account, &place)
                    .map_err(|_| MoveProblem::Failed)?;
                // Remembered before anything lands there, so a restart can pick it up; a place
                // that cannot be remembered is not used.
                crate::recovery::remember_held(
                    &self.app,
                    pctwin_move::held_journal::HeldPlace {
                        label: label.clone(),
                        account: account.clone(),
                        folder: place.clone(),
                    },
                )?;
                self.held_places.hold(&label, &place);
                Ok(account)
            });
            // A person whose place could not be made is not received: their things are
            // reported as not sent, never as copied.
            if let Ok(account) = held {
                rules.held.insert(p.account_id.clone(), label);
                self.held_for.push(crate::others::Placed {
                    old_id: p.account_id.clone(),
                    account,
                });
            }
        }
        self.helper = Some(helper);
        // This laptop places files by the very rules it sends.
        self.rules = rules.clone();
        let _ = self.app.emit(STAGE_EVENT, Stage::Moving);
        rules
    }

    /// After the move: each placed person's things are handed over at their next sign-in, and
    /// the helper ends. Returns who it could not be set up for.
    fn finish_others(&mut self) -> Vec<String> {
        match self.helper.take() {
            Some(helper) => helper.finish(&self.held_for),
            None => Vec::new(),
        }
    }
}

/// Another person to place, as M06 on the new laptop shows them: their name and how much is
/// theirs, counted (nothing of their files is named).
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct PersonToPlace {
    pub id: String,
    pub name: String,
    pub files: u64,
    pub bytes: u64,
}

/// What M06 on the new laptop shows.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct PeopleToPlace {
    pub people: Vec<PersonToPlace>,
    pub accounts: Vec<crate::others::AccountHere>,
    /// False where other people cannot be moved yet (a Mac): they are listed, left out.
    pub can_place: bool,
}

impl<R: Runtime> Steps for AppSteps<R> {
    async fn person(&mut self, record: &Record, account_id: &str) -> Result<PersonTarget, Stop> {
        if account_id == self.rules.mover {
            return Ok(PersonTarget::ExistingAccount {
                account_id: self.me.clone(),
            });
        }
        if self.placed.is_none() && !self.named.is_empty() {
            self.ask_places(record).await?;
        }
        let choice = self
            .placed
            .as_ref()
            .and_then(|(c, _)| c.get(account_id).cloned());
        Ok(match choice {
            Some(crate::others::PlaceChoice::Existing { account_id }) => {
                PersonTarget::ExistingAccount { account_id }
            }
            Some(crate::others::PlaceChoice::New) => PersonTarget::NewAccount {
                name: self
                    .named
                    .iter()
                    .find(|p| p.account_id == account_id)
                    .map(|p| crate::others::full_name(&p.name))
                    .unwrap_or_default(),
            },
            Some(crate::others::PlaceChoice::LeaveOut) | None => PersonTarget::Skip,
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
        let groups = view.groups.iter().map(|g| g.role).collect();
        let answer = self.moving.wait_for_answer(
            PlanKey {
                revision: view.revision,
                fingerprint: view.fingerprint.clone(),
            },
            groups,
        );
        let _ = self.app.emit(PLAN_EVENT, view);
        match answer.await {
            Ok(true) => {
                // The folders the person unticked: none of their files is sent or received.
                self.rules.left_out = self.moving.left_out();
                // The places the person chose: those folders land there, and nowhere else.
                for (role, (place, shown)) in self.moving.take_chosen() {
                    let label = pctwin_move::landing::chosen_label(role);
                    self.rules.chosen.insert(role);
                    self.shown.insert(label, shown);
                    self.chosen.insert(role, place);
                }
                let _ = self.app.emit(STAGE_EVENT, Stage::Moving);
                Ok(())
            }
            _ => Err(Stop::NotApproved),
        }
    }

    fn landing(&self, item: &Item) -> Option<LandingPlace> {
        self.rules.landing(item)
    }

    fn left_out(&self, record: &Record) -> Vec<ItemId> {
        record
            .items
            .iter()
            .filter(|i| {
                shown_to_mover(i, &self.rules.mover) && self.rules.left_out.contains(&i.place.role)
            })
            .map(|i| i.id)
            .collect()
    }

    fn mover(&mut self, account_id: &str) {
        self.rules.mover = account_id.to_string();
    }

    fn people(&mut self, people: &[pctwin_move::landing::NamedPerson]) {
        self.named = people.to_vec();
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
        let mut last_link: Option<ConnectionView> = None;
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
            // While waiting for the other laptop, C09 is shown instead of the progress.
            match live.connection.state() {
                pctwin_move::reconnect::LinkState::Waiting { since, nothing_new } => {
                    let (saved_bytes, total_bytes) = saved(&live.meter);
                    let view = ConnectionView::Waiting {
                        nothing_new,
                        last_seen_seconds: since.elapsed().as_secs(),
                        saved_bytes,
                        total_bytes,
                    };
                    if last_link != Some(view) {
                        let _ = app.emit(LINK_EVENT, view);
                        last_link = Some(view);
                    }
                    tokio::time::sleep(TICK).await;
                    continue;
                }
                pctwin_move::reconnect::LinkState::Connected if last_link.is_some() => {
                    let _ = app.emit(LINK_EVENT, ConnectionView::Connected);
                    last_link = None;
                    // The progress is sent again in full.
                    last = None;
                }
                _ => {}
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
/// The mover's places a Mac's privacy settings kept PCTwin out of.
fn privacy_blocked(sweep: &pctwin_scan::Sweep) -> u64 {
    sweep
        .blind_spots
        .iter()
        .filter(|b| matches!(b.reason, pctwin_scan::BlindReason::MacPrivacy))
        .map(|b| b.count)
        .sum()
}

/// What the Full Disk Access guide shows: how many of the mover's places were kept private.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
pub struct Privacy {
    pub places: u64,
}

async fn run_old<R: Runtime>(
    app: &AppHandle<R>,
    moving: &AppMoving,
    mut link: pctwin_link::Link,
    live: &Live,
    scan_stop: &AtomicBool,
    others: Others,
    other: Option<LabelView>,
) -> Result<Ran, MoveError> {
    // The pairing's keys, for the way back after a drop once the plan is approved.
    let mut back =
        pctwin_move::resume::NetComeback::new(&mut link, pctwin_link::LinkConfig::default());
    // Greeted first, so the new laptop knows at once, and waits for the list as long as the
    // finding (and this laptop's person) takes. A stop ends the greeting at once.
    let mut link = pctwin_move::or_stop(&live.control, async {
        Ok(greet(app, &mut link, pctwin_move::greeting::Side::Old, other).await?)
    })
    .await?;
    let _ = app.emit(STAGE_EVENT, Stage::Scanning);
    let laptop = LaptopId::generate().map_err(|e| TransferError::Protocol(e.to_string()))?;
    let mut found = |v: &pctwin_scan::FrontierView<'_>| {
        let _ = app.emit(
            FOUND_EVENT,
            Found {
                bytes: v.found_so_far().mover_bytes,
            },
        );
    };
    let mut sweep = pctwin_scan::sweep_this_laptop(laptop, scan_stop, &mut found, None);
    // A Mac kept some of the mover's places private: the guide says how to let PCTwin in, and
    // the person looks again or goes on without them (they are then not in the move at all).
    loop {
        let places = privacy_blocked(&sweep);
        if places == 0 || scan_stop.load(Ordering::Acquire) {
            break;
        }
        let answer = moving.wait_for_privacy();
        let _ = app.emit(PRIVACY_EVENT, Privacy { places });
        match answer.await {
            Ok(true) => {
                let _ = app.emit(STAGE_EVENT, Stage::Scanning);
                sweep = pctwin_scan::sweep_this_laptop(laptop, scan_stop, &mut found, None);
            }
            Ok(false) => break,
            Err(_) => return Err(Stop::Cancelled.into()),
        }
    }
    let approve_words = others.approve;
    let others = crate::others::checked_others(&sweep.census, &others.ids)
        .map_err(|_| TransferError::Protocol("someone ticked is not on this laptop".into()))?;
    if scan_stop.load(Ordering::Acquire) {
        return Err(Stop::Cancelled.into());
    }
    let roots: HashMap<ItemId, std::path::PathBuf> = sweep
        .placed_by
        .iter()
        .filter_map(|(id, place)| sweep.item_roots.get(place).map(|r| (*id, r.clone())))
        .collect();
    let mover = sweep.census.me.clone();
    let mut record = sweep.record.clone();
    let mut handles = HashMap::new();
    let mut named = Vec::new();
    let mut helper = None;
    let mut opener: Arc<dyn pctwin_transfer::Opener> = Arc::new(FsOpener);
    if !others.is_empty() {
        let not_moved = |why| {
            let _ = app.emit(STAGE_EVENT, Stage::OthersNotMoved { why });
        };
        if !crate::others::CAN_MOVE_OTHERS {
            not_moved(OthersWhy::NotHere);
        } else if pctwin_helper::harden_engine().is_err() {
            // Other people's things are carried only by a hardened engine.
            not_moved(OthersWhy::Failed);
        } else {
            let _ = app.emit(STAGE_EVENT, Stage::Administrator);
            let names: Vec<String> = others
                .iter()
                .filter_map(|id| Some(sweep.census.by_account(id)?.suggested_name.clone()))
                .collect();
            let prompt = crate::others::approve_prompt(&approve_words, &names).unwrap_or_default();
            match crate::others::ask_for(&mover, &others, laptop, &prompt, &|| {
                scan_stop.load(Ordering::Acquire)
            }) {
                Ok(crate::others::Asked::Approved(client, scanned)) => {
                    let people: Vec<pctwin_helper::Scanned> =
                        scanned.iter().map(|(_, s)| s.clone()).collect();
                    handles = pctwin_move::people::add_people(&mut record, &people);
                    named = scanned
                        .iter()
                        .filter_map(|(id, _)| {
                            let r = sweep.census.by_account(id)?;
                            Some(pctwin_move::landing::NamedPerson {
                                account_id: id.clone(),
                                name: r.suggested_name.clone(),
                            })
                        })
                        .collect();
                    opener = Arc::new(pctwin_move::people::HelperOpener::new(
                        Arc::clone(&client),
                        &people,
                    ));
                    helper = Some(client);
                }
                Ok(crate::others::Asked::NotApproved) => not_moved(OthersWhy::NotApproved),
                Ok(crate::others::Asked::AllowInSettings) => not_moved(OthersWhy::AllowInSettings),
                Err(_) => not_moved(OthersWhy::Failed),
            }
        }
    }
    let _ = app.emit(STAGE_EVENT, Stage::Waiting);
    let who = pctwin_move::Who {
        mover: &mover,
        people: &named,
    };
    let sent = pctwin_move::send_by_rules(
        &mut link,
        &record,
        &who,
        |item| match handles.get(&item.id) {
            Some(h) => Some(pctwin_move::people::helper_source(*h)),
            None => {
                let mut path = roots.get(&item.id)?.clone();
                for part in item.path.parts() {
                    path.push(part.on_disk());
                }
                Some(path)
            }
        },
        live,
        opener,
        &mut back,
    )
    .await;
    // The helper's session ends with the move, whatever became of it.
    if let Some(client) = helper.and_then(|h| Arc::try_unwrap(h).ok())
        && let Ok(client) = client.into_inner()
    {
        let _ = client.bye();
    }
    let (sent, _rules) = sent?;
    let (outcome, report) = sent_outcome(&sent, &record, &mover);
    Ok(Ran {
        outcome,
        report,
        link_lost: sent.ended == Ended::LinkLost,
    })
}

/// How this laptop's side of the move came out: the outcome (R01), the report (R02), and whether
/// it ended because the laptops could not find each other again (C10).
struct Ran {
    outcome: MoveOutcome,
    report: Vec<ReportItem>,
    link_lost: bool,
}

/// Which laptop this is in the move, with what that side starts from.
enum ThisSide {
    /// The old laptop, with the other people the mover ticked (M06).
    Old {
        others: Others,
    },
    New(Box<NewSide>),
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
        left_out: Default::default(),
        chosen: Default::default(),
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
    other: Option<LabelView>,
) -> Result<Ran, MoveError> {
    let _ = app.emit(STAGE_EVENT, Stage::Waiting);
    let NewSide {
        mut table,
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
    // Other people's writes go to the journal in their holding folder, never into this one.
    let held_places = pctwin_move::held_journal::HeldPlaces::default();
    let ledger = pctwin_move::held_journal::SplitLedger::new(&journal, held_places.clone());
    let mut steps = AppSteps {
        app: app.clone(),
        moving,
        me: me.clone(),
        rules,
        names,
        shown,
        named: Vec::new(),
        placed: None,
        helper: None,
        held_for: Vec::new(),
        live_control: live.control.clone(),
        chosen: BTreeMap::new(),
        held_places: held_places.clone(),
    };
    // The pairing's keys, for the way back after a drop once the plan is approved.
    let mut redial =
        pctwin_move::resume::NetRedial::new(&mut link, pctwin_link::LinkConfig::default());
    // A stop ends the greeting at once.
    let mut link = pctwin_move::or_stop(&live.control, async {
        Ok(greet(app, &mut link, pctwin_move::greeting::Side::New, other).await?)
    })
    .await?;
    let here = pctwin_move::Here {
        journal: &ledger,
        account: &me,
    };
    let received = pctwin_move::receive_by_rules(
        &mut link,
        &mut steps,
        |s, table| s.place_others(table),
        &mut table,
        here,
        live,
        &mut redial,
    )
    .await;
    // Whatever became of the move, the helper's session ends; those placed get their things at
    // their next sign-in.
    let not_handed = steps.finish_others();
    let received = received?;
    let (mut outcome, report) = received_outcome(&received, &steps.rules.mover);
    outcome.people = people_outcome(&received, &steps.rules.mover, &steps.named, &not_handed);
    Ok(Ran {
        outcome,
        report,
        link_lost: received.ended == Ended::LinkLost,
    })
}

/// One line per other person in the move: how their things ended, counted, and whether they are
/// handed over at their next sign-in.
fn people_outcome(
    received: &Received,
    mover: &str,
    named: &[pctwin_move::landing::NamedPerson],
    not_handed: &[String],
) -> Vec<PersonOutcome> {
    let report = pctwin_move::report::for_the_mover(received, mover);
    named
        .iter()
        .filter_map(|p| {
            let line = report.people.get(&p.account_id)?;
            Some(PersonOutcome {
                name: p.name.clone(),
                copied: line.copied,
                needs_action: line.needs_action,
                failed: line.failed,
                at_sign_in: line.copied > 0 && !not_handed.contains(&p.account_id),
            })
        })
        .collect()
}

/// Runs this laptop's side of the move on a thread of its own, then tells the screens how it
/// ended.
fn spawn_run<R: Runtime>(
    app: AppHandle<R>,
    moving: AppMoving,
    link: pctwin_link::Link,
    paired: PairedWith,
    side: ThisSide,
    live: Live,
    scan_stop: Arc<AtomicBool>,
) -> Result<(), MoveProblem> {
    let runtime = tokio::runtime::Handle::current();
    let laptops = Laptops {
        old: paired.label.as_ref().map(LabelView::from),
    };
    let label = laptops.old.clone();
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
                match side {
                    ThisSide::New(setup) => {
                        let other = label.clone();
                        run_new(&app, moving.clone(), link, &live, *setup, names, other).await
                    }
                    ThisSide::Old { others } => {
                        let other = label.clone();
                        run_old(&app, &moving, link, &live, &scan_stop, others, other).await
                    }
                }
            });
            done.store(true, Ordering::Release);
            match ran {
                Ok(Ran {
                    outcome,
                    report,
                    link_lost,
                }) => {
                    let (state, left) = crate::history::move_state(&outcome);
                    crate::history::add(
                        &app,
                        "move",
                        label.as_ref(),
                        state,
                        left,
                        &crate::history::report_kept(&report),
                    );
                    moving.finish(report);
                    if !link_lost {
                        crate::restart::nothing_left(&app);
                    }
                    if link_lost {
                        // The laptops could not find each other again: C10, with what moved kept
                        // in the report and the history.
                        let (saved_bytes, total_bytes) = saved(&live.meter);
                        let by = &outcome.by_status;
                        let _ = app.emit(
                            LINK_EVENT,
                            ConnectionView::Ended {
                                moved_something: by.copied + by.needs_action > 0,
                                saved_bytes,
                                total_bytes,
                            },
                        );
                    } else {
                        let _ = app.emit(FINISHED_EVENT, outcome);
                    }
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

/// The other people the mover ticked on the old laptop (M06), by account ID, with the words of a
/// Mac's administrator window (`{names}` filled in with theirs).
struct Others {
    ids: Vec<String>,
    approve: String,
}

/// Starts this laptop's side of the move with the paired link. `fallback`: for each folder the new
/// laptop may lack, the folder in the home that stands for it, in the person's language.
/// `approve`: the words of a Mac's administrator window, with `{names}`.
#[tauri::command]
pub async fn move_begin<R: Runtime>(
    app: AppHandle<R>,
    pairing: State<'_, AppPairing>,
    moving: State<'_, AppMoving>,
    fallback: BTreeMap<FolderRole, String>,
    others: Vec<String>,
    approve: String,
) -> Result<(), MoveProblem> {
    check_fallback(&fallback)?;
    if others.len() > pctwin_move::landing::MAX_PEOPLE
        || crate::others::approve_prompt(&approve, &[]).is_none()
    {
        return Err(MoveProblem::Failed);
    }
    let others = Others {
        ids: others,
        approve,
    };
    let (live, scan_stop) = moving.begin()?;
    // Until the move is over, a restart opens PCTwin again at sign-in (src/restart.rs).
    crate::restart::while_moving(&app);
    let Some((link, paired)) = pairing.take_link().await else {
        moving.finish(Vec::new());
        return Err(MoveProblem::NotPaired);
    };
    // The new laptop has no one to tick: its people come from the old laptop.
    let side = if paired.old {
        ThisSide::Old { others }
    } else {
        match new_places(fallback) {
            Ok(setup) => ThisSide::New(Box::new(setup)),
            Err(p) => {
                moving.finish(Vec::new());
                return Err(p);
            }
        }
    };
    let moving = Arc::clone(&moving);
    spawn_run(app, moving, link, paired, side, live, scan_stop)
}

/// The person's answer to the plan on screen (M23), for exactly that revision, with the folders
/// they unticked (`left_out`): none of those is moved.
#[tauri::command]
pub fn move_approve(
    moving: State<'_, AppMoving>,
    plan: PlanKey,
    approve: bool,
    left_out: Vec<FolderRole>,
) -> Result<(), MoveProblem> {
    moving.answer(&plan, approve, &left_out)
}

/// What came of choosing a place for a group of folders (M05).
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(tag = "kind", rename_all = "kebab-case")]
pub enum PlaceAnswer {
    /// The folder, as the system shows it, and its free space when the drive says.
    #[serde(rename_all = "camelCase")]
    Chosen {
        path: String,
        free_bytes: Option<u64>,
    },
    /// The person closed the picker.
    NotChosen,
    /// The drive has no safe way to name files: choose another.
    NoSafeName,
    /// The folder could not be opened.
    CannotOpen,
}

/// The new laptop, while the plan waits for an answer: the person chooses, in the system's
/// picker, another folder for the folders of `role` (M05). It is opened and its drive tested
/// now; it is approved only with the plan.
#[tauri::command]
pub async fn move_choose_place(
    moving: State<'_, AppMoving>,
    role: FolderRole,
) -> Result<PlaceAnswer, MoveProblem> {
    moving.may_choose(role)?;
    let picked = tauri::async_runtime::spawn_blocking(|| rfd::FileDialog::new().pick_folder())
        .await
        .map_err(|_| MoveProblem::Failed)?;
    let Some(folder) = picked else {
        return Ok(PlaceAnswer::NotChosen);
    };
    let Ok(place) = pctwin_gate::Destination::open(&folder) else {
        return Ok(PlaceAnswer::CannotOpen);
    };
    if place
        .check_drive()
        .is_ok_and(|check| check.naming == pctwin_gate::Naming::NoSafeMethod)
    {
        return Ok(PlaceAnswer::NoSafeName);
    }
    let path = folder.display().to_string();
    let free_bytes = place.available_bytes();
    moving.choose(role, Some((place, path.clone())))?;
    Ok(PlaceAnswer::Chosen { path, free_bytes })
}

/// The new laptop: the folders of `role` go to their usual place again.
#[tauri::command]
pub fn move_usual_place(moving: State<'_, AppMoving>, role: FolderRole) -> Result<(), MoveProblem> {
    moving.choose(role, None)
}

/// The new laptop: where each other person's things go (M06), and the words of the system
/// window that shows a new account's temporary password.
#[tauri::command]
pub fn move_place_people(
    moving: State<'_, AppMoving>,
    choices: Vec<crate::others::PersonChoice>,
    words: crate::others::PasswordWords,
) -> Result<(), MoveProblem> {
    moving.place(&choices, words)
}

/// The old laptop: the person's answer to the Full Disk Access guide (`again`: look again).
#[tauri::command]
pub fn move_privacy_answer(moving: State<'_, AppMoving>, again: bool) -> Result<(), MoveProblem> {
    moving.privacy_answer(again)
}

/// Opens the Mac's Full Disk Access settings, where PCTwin is switched on (nothing elsewhere).
#[tauri::command]
pub fn open_privacy_settings() {
    if cfg!(target_os = "macos") {
        let _ = std::process::Command::new("/usr/bin/open")
            .arg("x-apple.systempreferences:com.apple.preference.security?Privacy_AllFiles")
            .spawn();
    }
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

/// "Try now" while waiting for the other laptop after a drop (C09): the next try starts at once.
#[tauri::command]
pub fn move_retry(moving: State<'_, AppMoving>) -> Result<(), MoveProblem> {
    moving.retry()
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
            time_not_kept: false,
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

    /// A folder that moves as one whole (here a Photos Library) has its own line: moved only when
    /// every file in it was, else needs action, with the file that did not arrive on its own line.
    #[test]
    fn a_whole_folder_says_moved_only_when_every_file_in_it_did() {
        let lib = "Album.photoslibrary";
        let mut whole = item(person("1000"), FolderRole::Pictures, &[lib], 0);
        whole.kind = ItemKind::Package;
        let a = item(
            person("1000"),
            FolderRole::Pictures,
            &[lib, "originals", "a.jpg"],
            4,
        );
        let b = item(
            person("1000"),
            FolderRole::Pictures,
            &[lib, "database", "p.db"],
            3,
        );
        let line = |report: &[ReportItem]| {
            report
                .iter()
                .find(|r| r.id == whole.id.to_hex())
                .cloned()
                .expect("the whole has a line")
        };
        let with = |b_outcome| {
            let mut r = received(
                vec![
                    (
                        a.clone(),
                        Some(finished(
                            &format!("{lib}/originals/a.jpg"),
                            CheckedFrom::Drive,
                        )),
                    ),
                    (b.clone(), b_outcome),
                ],
                Ended::Finished,
            );
            r.record.items.insert(0, whole.clone());
            received_outcome(&r, "1000")
        };
        let (outcome, report) = with(Some(finished(
            &format!("{lib}/database/p.db"),
            CheckedFrom::Drive,
        )));
        let l = line(&report);
        assert_eq!(l.status, ItemStatus::Copied);
        assert_eq!(l.reason, None);
        assert_eq!(l.destination.as_deref(), Some(lib));
        assert_eq!(l.source, format!("pictures/{lib}"));
        assert_eq!(outcome.by_status.copied, 3);

        let failed =
            ReceiveOutcome::Failed(pctwin_transfer::Failure::new(Cause::NotVerified, "no"));
        let (outcome, report) = with(Some(failed));
        let l = line(&report);
        assert_eq!(l.status, ItemStatus::NeedsAction);
        assert_eq!(l.reason, Some(Reason::PartsMissing));
        assert_eq!(outcome.by_status.needs_action, 1);
        // The file that did not arrive is named on its own line, with why.
        let missing = report.iter().find(|r| r.id == b.id.to_hex()).unwrap();
        assert_eq!(missing.status, ItemStatus::Failed);
        assert!(missing.reason.is_some());
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
    fn the_old_laptops_report_names_only_the_movers_and_shared_files() {
        let mine = item(person("1000"), FolderRole::Documents, &["a.txt"], 10);
        let bea = item(person("1001"), FolderRole::Pictures, &["secret.jpg"], 7);
        let mut record = Record::new(laptop());
        record.items = vec![mine.clone(), bea.clone()];
        let sent = Sent {
            files: vec![
                pctwin_move::SentFile {
                    item: mine.id,
                    outcome: Some(SendOutcome::Arrived),
                },
                pctwin_move::SentFile {
                    item: bea.id,
                    outcome: Some(SendOutcome::Arrived),
                },
            ],
            not_sent: Vec::new(),
            ended: Ended::Finished,
        };
        let (outcome, report) = sent_outcome(&sent, &record, "1000");
        assert_eq!(outcome.by_status.copied, 1);
        assert_eq!(outcome.bytes_checked, 10);
        assert_eq!(report.len(), 1);
        assert!(!serde_json::to_string(&report).unwrap().contains("secret"));
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

    /// A move that ended because the laptops could not find each other again: what landed is
    /// copied, what the end cut off is not copied (never copied, never a failure of its own), and
    /// a file that failed on its own still says so; the history keeps it as stopped part way.
    #[test]
    fn a_move_ended_by_a_lost_connection_keeps_every_result_honest() {
        let a = item(person("1000"), FolderRole::Documents, &["a.txt"], 10);
        let b = item(person("1000"), FolderRole::Documents, &["b.txt"], 10);
        let c = item(person("1000"), FolderRole::Documents, &["c.txt"], 10);
        let cut = ReceiveOutcome::Failed(Failure::new(Cause::MoveEnded, pctwin_move::LINK_LOST));
        let own = ReceiveOutcome::Failed(Failure::new(Cause::NoSpace, "no space"));
        let r = received(
            vec![
                (a, Some(finished("a.txt", CheckedFrom::Drive))),
                (b, Some(cut)),
                (c, Some(own)),
            ],
            Ended::LinkLost,
        );
        let (outcome, report) = received_outcome(&r, "1000");
        assert!(outcome.stopped);
        assert_eq!(outcome.by_status.copied, 1);
        assert_eq!(outcome.by_status.failed, 1);
        assert_eq!(outcome.not_copied, 1);
        assert_eq!(report.len(), 2);
        assert_eq!(report[1].reason, Some(Reason::NotEnoughSpace));
        assert_eq!(crate::history::move_state(&outcome), ("stopped", 2));
    }

    /// Fresh-context review of TRANSFER (24 October 2026): a move that could not go on (a message
    /// out of place) keeps its report as a lost connection does, and R01 is told it ended early
    /// on its own, not that the person stopped it.
    #[test]
    fn a_move_that_broke_off_keeps_every_result_and_says_it_ended_early() {
        let a = item(person("1000"), FolderRole::Documents, &["a.txt"], 10);
        let b = item(person("1000"), FolderRole::Documents, &["b.txt"], 10);
        let cut = ReceiveOutcome::Failed(Failure::new(
            Cause::MoveEnded,
            format!("{} (something unexpected)", pctwin_move::BROKE_OFF),
        ));
        let r = received(
            vec![
                (a, Some(finished("a.txt", CheckedFrom::Drive))),
                (b, Some(cut)),
            ],
            Ended::Broke,
        );
        let (outcome, report) = received_outcome(&r, "1000");
        assert!(outcome.stopped && outcome.broke_off);
        assert_eq!(outcome.by_status.copied, 1);
        assert_eq!(outcome.not_copied, 1);
        assert_eq!(report.len(), 1);
        let finished_move = received_outcome(&received(Vec::new(), Ended::Finished), "1000").0;
        assert!(!finished_move.broke_off);
    }

    #[test]
    fn the_after_move_check_counts_the_movers_photos_and_documents_planned_and_copied() {
        let photo = item(
            person("1000"),
            FolderRole::Pictures,
            &["Holiday", "IMG_1.JPG"],
            10,
        );
        let lost = item(person("1000"), FolderRole::Pictures, &["IMG_2.heic"], 10);
        let there = item(person("1000"), FolderRole::Pictures, &["IMG_3.png"], 10);
        let doc = item(person("1000"), FolderRole::Documents, &["tax.pdf"], 10);
        let other = item(person("1000"), FolderRole::Documents, &["notes.bin"], 10);
        let beas = item(person("1001"), FolderRole::Pictures, &["secret.jpg"], 7);
        let r = received(
            vec![
                (photo, Some(finished("IMG_1.JPG", CheckedFrom::Drive))),
                (
                    lost,
                    Some(ReceiveOutcome::Failed(Failure::new(Cause::Unreadable, "x"))),
                ),
                (
                    there,
                    Some(ReceiveOutcome::AlreadyThere("IMG_3.png".into())),
                ),
                (doc, Some(finished("tax.pdf", CheckedFrom::Drive))),
                (other, Some(finished("notes.bin", CheckedFrom::Drive))),
                (beas, Some(finished("secret.jpg", CheckedFrom::Drive))),
            ],
            Ended::Finished,
        );
        let (outcome, _) = received_outcome(&r, "1000");
        // Another person's photo is not counted for the mover; a failed one is planned, not copied.
        assert_eq!(
            outcome.after_move.photos,
            KindCount {
                planned: 3,
                copied: 2
            }
        );
        assert_eq!(
            outcome.after_move.documents,
            KindCount {
                planned: 1,
                copied: 1
            }
        );
        assert_eq!(outcome.after_move.videos, KindCount::default());
        let text = serde_json::to_value(&outcome).unwrap();
        assert_eq!(text["afterMove"]["photos"]["planned"], 3);
    }

    #[test]
    fn the_old_laptop_counts_as_copied_only_what_the_new_laptop_confirmed() {
        let a = item(person("1000"), FolderRole::Pictures, &["a.jpg"], 10);
        let b = item(person("1000"), FolderRole::Pictures, &["b.jpg"], 10);
        let mut record = Record::new(laptop());
        record.items = vec![a.clone(), b.clone()];
        let sent = Sent {
            files: vec![
                pctwin_move::SentFile {
                    item: a.id,
                    outcome: Some(SendOutcome::Arrived),
                },
                pctwin_move::SentFile {
                    item: b.id,
                    outcome: None,
                },
            ],
            not_sent: Vec::new(),
            ended: Ended::Finished,
        };
        let (outcome, _) = sent_outcome(&sent, &record, "1000");
        assert_eq!(
            outcome.after_move.photos,
            KindCount {
                planned: 2,
                copied: 1
            }
        );
    }

    #[test]
    fn a_file_sent_as_it_is_now_or_gone_before_it_was_sent_says_so_beside_its_result() {
        let a = item(person("1000"), FolderRole::Documents, &["a.txt"], 10);
        let b = item(person("1000"), FolderRole::Documents, &["b.txt"], 10);
        let c = item(person("1000"), FolderRole::Documents, &["c.txt"], 10);
        let d = item(person("1000"), FolderRole::Documents, &["d.txt"], 10);
        let gone = ReceiveOutcome::Failed(Failure::new(Cause::NoLongerThere, "not found"));
        let unreadable = ReceiveOutcome::Failed(Failure::new(Cause::Unreadable, "denied"));
        let mut r = received(
            vec![
                (a, Some(finished("a.txt", CheckedFrom::Drive))),
                (b, Some(finished("b.txt", CheckedFrom::Drive))),
                (c, Some(gone)),
                (d, Some(unreadable)),
            ],
            Ended::Finished,
        );
        r.files[0].changed_since_scan = true;
        let (_, report) = received_outcome(&r, "1000");
        let notes: Vec<_> = report.iter().map(|l| l.note).collect();
        assert_eq!(
            notes,
            vec![
                Some(ReportNote::ChangedSinceScan),
                None,
                Some(ReportNote::NoLongerThere),
                None
            ]
        );
        // A changed file is still copied and checked: the note never changes the result.
        assert_eq!(report[0].status, ItemStatus::Copied);
        assert_eq!(report[2].status, ItemStatus::Failed);
        let text = serde_json::to_value(&report).unwrap();
        assert_eq!(text[0]["note"], "changed-since-scan");
        assert_eq!(text[2]["note"], "no-longer-there");
        assert!(text[1].get("note").is_none());
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
    fn a_place_is_chosen_only_for_a_folder_on_the_plan_waiting_and_only_kept_if_it_moves() {
        // Opened only: nothing is written there.
        let dir = std::env::temp_dir();
        let open = || {
            Some((
                pctwin_gate::Destination::open(&dir).unwrap(),
                "E:/Photos".to_string(),
            ))
        };
        let moving = Moving::default();
        moving.begin().unwrap();
        // No plan on screen yet.
        assert_eq!(
            moving.choose(FolderRole::Pictures, open()),
            Err(MoveProblem::PlanChanged)
        );
        let key = PlanKey {
            revision: 1,
            fingerprint: "ab".into(),
        };
        let _rx = moving.wait_for_answer(
            key.clone(),
            [FolderRole::Pictures, FolderRole::Music, FolderRole::Videos].into(),
        );
        // A folder the plan does not show.
        assert_eq!(
            moving.choose(FolderRole::Documents, open()),
            Err(MoveProblem::PlanChanged)
        );
        assert!(moving.may_choose(FolderRole::Pictures).is_ok());
        moving.choose(FolderRole::Pictures, open()).unwrap();
        moving.choose(FolderRole::Music, open()).unwrap();
        moving.choose(FolderRole::Videos, open()).unwrap();
        // Back to the usual folder.
        moving.choose(FolderRole::Videos, None).unwrap();
        moving.answer(&key, true, &[FolderRole::Music]).unwrap();
        // Once approved, nothing more is chosen.
        assert_eq!(
            moving.choose(FolderRole::Videos, open()),
            Err(MoveProblem::PlanChanged)
        );
        let chosen = moving.take_chosen();
        assert_eq!(
            chosen.keys().copied().collect::<Vec<_>>(),
            [FolderRole::Pictures],
            "a folder left out goes nowhere, and one put back goes to its usual place"
        );
        assert!(moving.take_chosen().is_empty(), "taken once");
    }

    #[test]
    fn an_answer_counts_only_for_the_plan_on_screen() {
        let moving = Moving::default();
        moving.begin().unwrap();
        let key = PlanKey {
            revision: 2,
            fingerprint: "ab".into(),
        };
        let mut rx = moving.wait_for_answer(key.clone(), [FolderRole::Documents].into());
        let stale = PlanKey {
            revision: 1,
            fingerprint: "ab".into(),
        };
        assert_eq!(
            moving.answer(&stale, true, &[]),
            Err(MoveProblem::PlanChanged)
        );
        assert!(rx.try_recv().is_err(), "no answer yet");
        // Leaving out a folder the plan does not show is refused, and the plan still waits.
        assert_eq!(
            moving.answer(&key, true, &[FolderRole::Music]),
            Err(MoveProblem::PlanChanged)
        );
        moving.answer(&key, true, &[FolderRole::Documents]).unwrap();
        assert_eq!(moving.left_out(), [FolderRole::Documents].into());
        assert_eq!(rx.try_recv(), Ok(true));
        // Answered once only.
        assert_eq!(
            moving.answer(&key, true, &[]),
            Err(MoveProblem::PlanChanged)
        );
    }

    #[test]
    fn stopping_before_the_approval_declines_the_plan() {
        let moving = Moving::default();
        let (live, scan_stop) = moving.begin().unwrap();
        let mut rx = moving.wait_for_answer(
            PlanKey {
                revision: 1,
                fingerprint: "ab".into(),
            },
            Default::default(),
        );
        moving.stop().unwrap();
        assert_eq!(rx.try_recv(), Ok(false));
        assert!(live.control.stopped());
        assert!(scan_stop.load(Ordering::Acquire));
    }

    fn words() -> crate::others::PasswordWords {
        crate::others::PasswordWords {
            title: "New account".into(),
            body: "{name}: {account} {password}".into(),
            approve: "Approve reading the files of {names}.".into(),
        }
    }

    #[test]
    fn places_are_taken_only_for_the_people_asked_and_a_wrong_answer_can_be_given_again() {
        use crate::others::{PersonChoice, PlaceChoice};
        let moving = Moving::default();
        moving.begin().unwrap();
        let new = |id: &str| PersonChoice {
            id: id.into(),
            to: PlaceChoice::New,
        };
        // Nothing asked yet.
        assert_eq!(
            moving.place(&[new("S-2")], words()),
            Err(MoveProblem::PlanChanged)
        );
        let mut rx = moving.wait_for_places(["S-2".to_string()].into(), Vec::new());
        assert_eq!(
            moving.place(&[new("S-9")], words()),
            Err(MoveProblem::Failed)
        );
        let bad = crate::others::PasswordWords {
            title: "x".into(),
            body: "no places".into(),
            approve: "Approve {names}.".into(),
        };
        assert_eq!(moving.place(&[new("S-2")], bad), Err(MoveProblem::Failed));
        assert!(rx.try_recv().is_err(), "still waiting");
        moving.place(&[new("S-2")], words()).unwrap();
        let (choices, _) = rx.try_recv().unwrap();
        assert_eq!(choices.get("S-2"), Some(&PlaceChoice::New));
    }

    #[test]
    fn the_full_disk_access_guide_is_answered_once_and_a_stop_ends_it() {
        let moving = Moving::default();
        moving.begin().unwrap();
        assert_eq!(moving.privacy_answer(true), Err(MoveProblem::NotMoving));
        let mut rx = moving.wait_for_privacy();
        moving.privacy_answer(true).unwrap();
        assert_eq!(rx.try_recv(), Ok(true));
        assert_eq!(moving.privacy_answer(false), Err(MoveProblem::NotMoving));
        let mut rx = moving.wait_for_privacy();
        moving.stop().unwrap();
        assert!(rx.try_recv().is_err());
    }

    #[test]
    fn stopping_while_people_are_placed_ends_the_wait() {
        let moving = Moving::default();
        moving.begin().unwrap();
        let mut rx = moving.wait_for_places(["S-2".to_string()].into(), Vec::new());
        moving.stop().unwrap();
        assert!(matches!(
            rx.try_recv(),
            Err(tokio::sync::oneshot::error::TryRecvError::Closed)
        ));
    }

    #[test]
    fn each_other_person_is_one_line_of_counts() {
        let bea = item(person("1001"), FolderRole::Pictures, &["a.jpg"], 7);
        let bea2 = item(person("1001"), FolderRole::Pictures, &["b.jpg"], 7);
        let carl = item(person("1002"), FolderRole::Pictures, &["c.jpg"], 7);
        let failed = ReceiveOutcome::Failed(Failure::new(Cause::Unreadable, "x"));
        let r = received(
            vec![
                (bea, Some(finished("a.jpg", CheckedFrom::Drive))),
                (bea2, Some(failed)),
                (carl, Some(finished("c.jpg", CheckedFrom::Drive))),
            ],
            Ended::Finished,
        );
        let named = |id: &str, name: &str| pctwin_move::landing::NamedPerson {
            account_id: id.into(),
            name: name.into(),
        };
        let lines = people_outcome(
            &r,
            "1000",
            &[named("1001", "Bea"), named("1002", "Carl")],
            &["1002".to_string()],
        );
        assert_eq!(
            lines,
            [
                PersonOutcome {
                    name: "Bea".into(),
                    copied: 1,
                    needs_action: 0,
                    failed: 1,
                    at_sign_in: true,
                },
                PersonOutcome {
                    name: "Carl".into(),
                    copied: 1,
                    needs_action: 0,
                    failed: 0,
                    at_sign_in: false,
                },
            ]
        );
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
            left_out: Default::default(),
            chosen: Default::default(),
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
        // The documents move first (essentials), the music after: as the move's own order says.
        assert_eq!(view.first, [FolderRole::Documents]);
    }

    #[test]
    fn moves_first_lists_only_the_movers_groups_in_the_order_the_move_sends_them() {
        let shown = BTreeMap::new();
        let mut record = Record::new(laptop());
        record.items = vec![
            item(person("1000"), FolderRole::Music, &["song.mp3"], 7),
            item(person("1000"), FolderRole::Documents, &["b.txt"], 5),
            item(person("1000"), FolderRole::Desktop, &["note.txt"], 50),
            // Bea's desktop is an essential of hers, never named for the mover.
            item(person("1001"), FolderRole::Desktop, &["secret.txt"], 1),
        ];
        let view = plan_view(&record, &preview(), &rules(), &shown);
        let order: Vec<FolderRole> = pctwin_move::move_order(&record)
            .into_iter()
            .filter(|(_, t)| pctwin_move::moves_first(*t))
            .filter_map(|(id, _)| {
                record
                    .items
                    .iter()
                    .find(|i| i.id == id && i.owner == person("1000"))
            })
            .map(|i| i.place.role)
            .collect();
        assert_eq!(view.first, order);
        assert_eq!(view.first.len(), 2);
        assert!(view.first.contains(&FolderRole::Desktop));
        assert!(view.first.contains(&FolderRole::Documents));
        assert!(!view.first.contains(&FolderRole::Music));

        // The mover's own essentials (the old laptop's personal order) go first, and the plan
        // says so: their song's group leads.
        record.personal = vec![record.items[0].id];
        let view = plan_view(&record, &preview(), &rules(), &shown);
        assert_eq!(view.first.first(), Some(&FolderRole::Music));
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
