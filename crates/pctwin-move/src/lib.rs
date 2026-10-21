//! One move from start to end, once the two laptops are connected and paired, for every program
//! that runs a move: the test lab (`pctwin-labmove`) and, later, the app. Pairing itself stays
//! with each of them; everything after it is here, once.
//!
//! - [`send_plan`] turns the old laptop's record into the files to send, in the order the app
//!   moves things, with every file not sent and why ([`NotSentWhy`]). Quick Send uses it too.
//! - [`worth_compressing`] is the one list of formats that are already compressed.
//! - [`send`] is the old laptop's side: it agrees the version, sends the record, sends the files,
//!   and gives one typed outcome per file.
//! - [`receive_plan`] and [`ApprovedPlan::receive`] (or [`receive`], both in one) are the new
//!   laptop's side: it agrees the version, receives the record, asks the person through [`Steps`]
//!   where each person goes and whether they approve the plan, then receives only what that plan
//!   allows, through the safety gate, with every write in the change journal.
//!
//! Only the new laptop approves a plan (decided 16 October 2026); the old laptop never does. A
//! step the person refuses ([`Stop`]) ends the move before anything is written.
//!
//! Nothing here writes a file itself: every write is the receiver's, through the gate.

// The type-resolved fence: clippy.toml bans every way to change a file or folder, as in
// pctwin-transfer. Tests make files to test on, so they are exempt.
#![cfg_attr(not(test), forbid(clippy::disallowed_methods))]

pub mod handover;
pub mod landing;
pub mod people;
pub mod reconnect;
pub mod report;
pub mod since;

use std::collections::{BTreeMap, BTreeSet};
use std::future::Future;
use std::path::{Path, PathBuf};
use std::sync::Arc;

use pctwin_gate::Destinations;
use pctwin_journal::Ledger;
use pctwin_record::{
    Inclusion, Item, ItemId, ItemKind, LeftOutReason, Owner, PersonTarget, Record, RecordError,
};
use pctwin_transfer::{
    Allowance, Cause, Channel, Failure, Fit, FsOpener, LandingPlace, MOVE_PROTOCOL, MoveControl,
    MoveMeter, Opener, ReceiveOutcome, ReceiverSession, SendJob, SendOutcome, SenderSession, Tier,
    TimeRange, TransferError, agree_version, answer_link_test, measure_link, plan_order,
    receive_inventory, send_inventory,
};

/// Files a move sends at once.
const IN_FLIGHT: usize = 4;

/// Formats that are already compressed, so compressing them again only costs time.
const COMPRESSED: [&str; 12] = [
    "jpg", "jpeg", "png", "heic", "mp3", "mp4", "mov", "zip", "docx", "xlsx", "gz", "7z",
];

/// Whether a file at `path` (`/` between folders) is worth trying to compress: false for formats
/// that are already compressed, judged by the last extension of its own name, in any case. A name
/// with no dot has no extension, so it is tried.
pub fn worth_compressing(path: &str) -> bool {
    let name = path.rsplit('/').next().unwrap_or(path);
    match name.rsplit_once('.') {
        Some((_, ext)) => !COMPRESSED.iter().any(|c| c.eq_ignore_ascii_case(ext)),
        None => true,
    }
}

/// Why a file in the record is not sent.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum NotSentWhy {
    /// The record leaves it out, for this reason.
    LeftOut { reason: LeftOutReason },
    /// The old laptop has no folder for its role, so there is nowhere to read it from.
    NoFolderForRole,
    /// The new laptop has no place for it (another person not approved, the system's own).
    NoPlaceOnNewLaptop,
}

/// A file in the record that is not sent.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct NotSent {
    pub item: ItemId,
    pub why: NotSentWhy,
}

/// What the old laptop sends: the jobs in the order planned, and the files it does not send.
#[derive(Debug, Clone)]
pub struct SendPlan {
    pub jobs: Vec<SendJob>,
    pub not_sent: Vec<NotSent>,
}

impl SendPlan {
    /// Only the files in `items` ("Try again" for files that were in use or kept changing):
    /// every other job is dropped, and nothing is listed as not sent, since the first move's
    /// report already says it.
    pub fn only(self, items: &BTreeSet<ItemId>) -> SendPlan {
        SendPlan {
            jobs: self
                .jobs
                .into_iter()
                .filter(|j| items.contains(&j.item))
                .collect(),
            not_sent: Vec::new(),
        }
    }
}

/// The files to send from `record`. `source` says where each file is read from (`None`: the old
/// laptop has no folder for its role) and `landing` where it lands, which must be the very
/// function the new laptop checks it with. Only files are sent; a file the record leaves out is
/// listed as not sent with the record's reason. Jobs come in the order the app moves things, each
/// with its tier.
pub fn send_plan(
    record: &Record,
    source: impl Fn(&Item) -> Option<PathBuf>,
    landing: impl Fn(&Item) -> LandingPlace,
) -> SendPlan {
    send_plan_where(record, source, |i| Some(landing(i)))
}

/// As [`send_plan`], where `landing` may give no place (`None`: the new laptop does not take it,
/// listed as not sent with [`NotSentWhy::NoPlaceOnNewLaptop`]).
pub fn send_plan_where(
    record: &Record,
    source: impl Fn(&Item) -> Option<PathBuf>,
    landing: impl Fn(&Item) -> Option<LandingPlace>,
) -> SendPlan {
    let mut not_sent = Vec::new();
    let mut by_item = BTreeMap::new();
    for item in &record.items {
        if item.kind != ItemKind::File {
            continue;
        }
        if let Inclusion::LeftOut { reason } = item.inclusion {
            not_sent.push(NotSent {
                item: item.id,
                why: NotSentWhy::LeftOut { reason },
            });
            continue;
        }
        let Some(source) = source(item) else {
            not_sent.push(NotSent {
                item: item.id,
                why: NotSentWhy::NoFolderForRole,
            });
            continue;
        };
        let Some(place) = landing(item) else {
            not_sent.push(NotSent {
                item: item.id,
                why: NotSentWhy::NoPlaceOnNewLaptop,
            });
            continue;
        };
        by_item.insert(
            item.id,
            SendJob {
                item: item.id,
                source,
                compressible: worth_compressing(&place.path),
                destination: place.destination,
                path: place.path,
                tier: Tier::Rest,
                size: item.size_bytes,
            },
        );
    }
    // The order the app moves things in; anything it does not place goes last.
    let mut jobs = Vec::new();
    for (id, tier) in plan_order(&record.items, &[], &BTreeMap::new(), 0) {
        if let Some(mut job) = by_item.remove(&id) {
            job.tier = tier;
            jobs.push(job);
        }
    }
    jobs.extend(by_item.into_values());
    SendPlan { jobs, not_sent }
}

/// Why a move did not go ahead or did not finish.
#[derive(Debug, thiserror::Error)]
pub enum MoveError {
    #[error(transparent)]
    Transfer(#[from] TransferError),
    #[error(transparent)]
    Record(#[from] RecordError),
    #[error(transparent)]
    Stopped(#[from] Stop),
}

/// What the screens hold while the move runs: the meter they read, and the person's stop and
/// pause. Cheap to clone; the test lab passes a fresh one it never touches.
#[derive(Debug, Clone, Default)]
pub struct Live {
    pub meter: MoveMeter,
    pub control: MoveControl,
}

/// How the move ended.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Ended {
    /// Every file was sent or received and answered for.
    Finished,
    /// The person on this laptop stopped it.
    StoppedHere,
    /// The person on the other laptop stopped it.
    StoppedThere,
}

/// What a file cut off by a stop is reported as: failed, never copied.
pub const STOPPED: &str = "the move was stopped";

/// How the run of a session ended: finished, stopped (by either person), or an error.
fn ended(ran: Result<(), TransferError>) -> Result<Ended, MoveError> {
    match ran {
        Ok(()) => Ok(Ended::Finished),
        Err(TransferError::Stopped) => Ok(Ended::StoppedHere),
        Err(TransferError::StoppedThere) => Ok(Ended::StoppedThere),
        Err(e) => Err(e.into()),
    }
}

/// How one sent file ended on the old laptop (`None`: it has no outcome).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SentFile {
    pub item: ItemId,
    pub outcome: Option<SendOutcome>,
}

/// What the old laptop's side of a move came to: one line per job, in the order sent, and the
/// files it did not send.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Sent {
    pub files: Vec<SentFile>,
    pub not_sent: Vec<NotSent>,
    pub ended: Ended,
}

/// The old laptop, over `link` (already paired): agrees the version, sends `record`, then sends
/// the files `plan` holds, counted in `live`'s meter and held or stopped by its control. A stop,
/// on either laptop, ends it with the report so far ([`Ended`]).
pub async fn send<C: Channel>(
    link: &mut C,
    record: &Record,
    plan: SendPlan,
    live: &Live,
) -> Result<Sent, MoveError> {
    send_with(link, record, plan, live, Arc::new(FsOpener)).await
}

/// As [`send`], reading each file through `opener` (a move of several people reads theirs
/// through the administrator helper: see [`people::HelperOpener`]).
pub async fn send_with<C: Channel>(
    link: &mut C,
    record: &Record,
    plan: SendPlan,
    live: &Live,
    opener: Arc<dyn Opener>,
) -> Result<Sent, MoveError> {
    let version = agree_version(link, MOVE_PROTOCOL).await?;
    send_inventory(link, record, version).await?;
    answer_link_test(link, version).await?;
    run_sender(link, version, plan, live, opener).await
}

/// The old laptop's side of a move with the app: as [`send_with`], except the files to send are
/// worked out only once the new laptop's person has approved the plan and the new laptop has
/// sent its [`landing::LandingRules`], so every file is announced exactly where that laptop
/// checks it. `source` says where each file is read from.
pub async fn send_by_rules<C: Channel>(
    link: &mut C,
    record: &Record,
    mover: &str,
    source: impl Fn(&Item) -> Option<PathBuf>,
    live: &Live,
    opener: Arc<dyn Opener>,
) -> Result<(Sent, landing::LandingRules), MoveError> {
    let version = agree_version(link, MOVE_PROTOCOL).await?;
    send_inventory(link, record, version).await?;
    answer_link_test(link, version).await?;
    landing::send_mover(link, mover).await?;
    let rules = landing::receive_rules(link).await?;
    let plan = send_plan_where(record, source, |i| rules.landing(i));
    let sent = run_sender(link, version, plan, live, opener).await?;
    Ok((sent, rules))
}

/// Runs the old laptop's sending of `plan` over `link` (the version agreed): one outcome per
/// file, a stop said as one.
async fn run_sender<C: Channel>(
    link: &mut C,
    version: u16,
    plan: SendPlan,
    live: &Live,
    opener: Arc<dyn Opener>,
) -> Result<Sent, MoveError> {
    let items: Vec<ItemId> = plan.jobs.iter().map(|j| j.item).collect();
    let mut sender = SenderSession::new(plan.jobs, IN_FLIGHT)
        .with_opener(opener)
        .with_protocol(version)
        .with_meter(live.meter.clone())
        .with_control(live.control.clone());
    let ended = ended(sender.run(link).await)?;
    let stopped = ended != Ended::Finished;
    Ok(Sent {
        files: items
            .into_iter()
            .map(|item| SentFile {
                item,
                // A file the stop cut off failed; it is never reported copied.
                outcome: sender.outcome(item).cloned().or_else(|| {
                    stopped.then(|| SendOutcome::Failed(Failure::new(Cause::Stopped, STOPPED)))
                }),
            })
            .collect(),
        not_sent: plan.not_sent,
        ended,
    })
}

/// Why the person stopped a move on the new laptop.
#[derive(Debug, Clone, Copy, PartialEq, Eq, thiserror::Error)]
pub enum Stop {
    #[error("the plan was not approved")]
    NotApproved,
    #[error("the move was cancelled")]
    Cancelled,
}

/// The choices a person makes on the new laptop during a move. The test lab answers them by
/// itself; the app answers them from its screens. Each answer may be [`Stop`], which ends the
/// move before anything is written.
///
/// Written as `async fn` in an implementation. There is no `Send` bound: a move runs on one task.
pub trait Steps {
    /// Where the things of the person `account_id` on the old laptop go on this one. Asked once
    /// for each person who owns something in `record`.
    fn person(
        &mut self,
        record: &Record,
        account_id: &str,
    ) -> impl Future<Output = Result<PersonTarget, Stop>>;

    /// The person approves `record`, the plan as it will run, with every person placed, seeing
    /// `preview`: whether it fits and about how long it takes. Only the new laptop ever approves.
    fn approve(
        &mut self,
        record: &Record,
        preview: &Preview,
    ) -> impl Future<Output = Result<(), Stop>>;

    /// Where `item` lands on this laptop (`None`: it is not received). It must be the very
    /// function the old laptop sends with, so the old laptop never chooses a place.
    fn landing(&self, item: &Item) -> Option<LandingPlace>;

    /// Which person on the old laptop is moving (their account ID there), as the old laptop said
    /// (the app's way of moving, [`receive_by_rules`]), before anyone is placed.
    fn mover(&mut self, _account_id: &str) {}
}

/// What the person sees before approving: whether the plan fits on this laptop, and about how
/// long it takes, both from what was measured (see [`Fit`] and [`TimeRange`]).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Preview {
    pub fit: Fit,
    /// What the new laptop's drives cannot take as it is, found before anything moves.
    pub problems: Problems,
    /// Every file the plan receives, at its size from the scan.
    pub bytes: u64,
    /// The link speed the test after pairing measured (`None`: the other laptop's version has
    /// no link test).
    pub bytes_per_second: Option<u64>,
    pub time: TimeRange,
}

/// How many examples of each problem a preview keeps (the count is always whole).
pub const EXAMPLES: usize = 20;

/// Space lost to a drive's big blocks is shown from this much on (less is not worth a line).
pub const LOST_TO_BLOCKS_SHOWN: u64 = 64 * 1024 * 1024;

/// Files of one problem: how many, and the first few.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct Counted {
    pub count: u64,
    /// At most [`EXAMPLES`], in the record's order.
    pub examples: Vec<ItemId>,
}

impl Counted {
    fn add(&mut self, item: ItemId) {
        self.count += 1;
        if self.examples.len() < EXAMPLES {
            self.examples.push(item);
        }
    }
}

/// Problems found before the move (Product Spec: found at the scan, shown before anything
/// moves, with what PCTwin will do about each), each predicted by the very rules the write uses
/// ([`pctwin_gate::Destination::predict`]).
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct Problems {
    /// Names (of the file or a folder above it) the drive cannot hold: they land under a name
    /// changed to fit, and the report says so.
    pub renamed: Counted,
    /// Files bigger than the drive holds (FAT32: 4 GB): not copied there; another place, or a
    /// drive that holds them, is needed.
    pub too_big: Counted,
    /// Paths the new laptop refuses (too long, or not allowed): not copied.
    pub refused: Counted,
    /// Per place, the space its drive's big blocks lose (the rest of each file's last block),
    /// when it is at least [`LOST_TO_BLOCKS_SHOWN`].
    pub lost_to_blocks: BTreeMap<String, u64>,
    /// Files that would land where the new laptop starts programs by itself when someone signs
    /// in (a Startup folder, autostart, launch agents): shown before the plan is approved, so
    /// nothing starts running on the new laptop that the person did not see.
    pub runs_at_sign_in: Counted,
}

/// Folders (as the end of a path, `/` between names, compared ignoring case) whose contents the
/// system starts by itself when someone signs in.
const SIGN_IN_PLACES: [&str; 6] = [
    "/start menu/programs/startup/",
    "/.config/autostart/",
    "/.config/systemd/user/",
    "/library/launchagents/",
    "/library/launchdaemons/",
    "/library/startupitems/",
];

/// Whether a file landing at `path` inside the approved folder at `root` would be started by
/// the system when someone signs in.
pub fn runs_at_sign_in(root: Option<&Path>, path: &str) -> bool {
    let whole = format!(
        "/{}/{}",
        root.map(|r| r.to_string_lossy().replace('\\', "/"))
            .unwrap_or_default(),
        path
    )
    .to_lowercase();
    SIGN_IN_PLACES.iter().any(|p| whole.contains(p))
}

/// The problems `record` would meet on this laptop: each included file this laptop places,
/// predicted in its place in `table`.
pub fn problems(
    record: &Record,
    landing: impl Fn(&Item) -> Option<LandingPlace>,
    table: &Destinations,
) -> Problems {
    let mut found = Problems::default();
    let mut lost: BTreeMap<String, u64> = BTreeMap::new();
    for i in &record.items {
        if i.kind != ItemKind::File || i.inclusion != Inclusion::Included {
            continue;
        }
        let Some(place) = landing(i) else {
            continue;
        };
        let Ok(dest) = table.get(&place.destination) else {
            continue;
        };
        if runs_at_sign_in(dest.shown_path(), &place.path) {
            found.runs_at_sign_in.add(i.id);
        }
        match dest.predict(&place.path, i.size_bytes) {
            Err(_) => found.refused.add(i.id),
            Ok(p) => {
                if p.too_big.is_some() {
                    found.too_big.add(i.id);
                    continue;
                }
                if !p.changes.is_empty() {
                    found.renamed.add(i.id);
                }
                if let Some(block) = dest.block_bytes() {
                    let rest = i.size_bytes % block;
                    let lost_here = if rest == 0 { 0 } else { block - rest };
                    let n = lost.entry(place.destination).or_default();
                    *n = n.saturating_add(lost_here);
                }
            }
        }
    }
    found.lost_to_blocks = lost
        .into_iter()
        .filter(|(_, n)| *n >= LOST_TO_BLOCKS_SHOWN)
        .collect();
    found
}

/// The preview of `record` on this laptop: what each place receives (only included files this
/// laptop places), against `table`'s free space, at the measured `bytes_per_second`.
pub fn preview(
    record: &Record,
    landing: impl Fn(&Item) -> Option<LandingPlace>,
    table: &Destinations,
    bytes_per_second: Option<u64>,
) -> Preview {
    let mut needed: BTreeMap<String, u64> = BTreeMap::new();
    for i in &record.items {
        if i.kind != ItemKind::File || i.inclusion != Inclusion::Included {
            continue;
        }
        if let Some(place) = landing(i) {
            let n = needed.entry(place.destination).or_default();
            *n = n.saturating_add(i.size_bytes);
        }
    }
    let bytes = needed.values().fold(0u64, |t, n| t.saturating_add(*n));
    Preview {
        fit: Fit::check(&needed, table),
        problems: problems(record, &landing, table),
        bytes,
        bytes_per_second,
        time: TimeRange::of(bytes, bytes_per_second),
    }
}

/// A plan the person approved on the new laptop, ready to receive.
pub struct ApprovedPlan {
    record: Record,
    allowance: Allowance,
    /// The move protocol version both laptops agreed.
    version: u16,
}

/// How one received file ended on the new laptop (`None`: it has no outcome).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ReceivedFile {
    pub item: ItemId,
    pub outcome: Option<ReceiveOutcome>,
    /// It was sent as it was when it was sent, not as it was at the scan (its size or its changed
    /// time differed): the report labels it "changed since the scan".
    pub changed_since_scan: bool,
}

/// What the new laptop's side of a move came to: the plan as approved, and one line per included
/// file, in the record's order.
#[derive(Debug, Clone)]
pub struct Received {
    pub record: Record,
    pub files: Vec<ReceivedFile>,
    pub ended: Ended,
}

/// The new laptop, over `link` (already paired): agrees the version, receives the old laptop's
/// record, times the link, asks `steps` where each person goes and for the person's approval
/// (showing whether it fits in `table`'s places and about how long it takes), and holds the
/// approved plan. Nothing is written.
pub async fn receive_plan<C: Channel, S: Steps>(
    link: &mut C,
    steps: &mut S,
    table: &Destinations,
) -> Result<ApprovedPlan, MoveError> {
    receive_plan_told(link, steps, table, false).await
}

/// [`receive_plan`]; with `told`, the old laptop says who is moving after the link test, and
/// `steps` hears it before anyone is placed.
async fn receive_plan_told<C: Channel, S: Steps>(
    link: &mut C,
    steps: &mut S,
    table: &Destinations,
    told: bool,
) -> Result<ApprovedPlan, MoveError> {
    let version = agree_version(link, MOVE_PROTOCOL).await?;
    let mut record = receive_inventory(link, version).await?;
    let speed = measure_link(link, version).await?;
    if told {
        let mover = landing::receive_mover(link).await?;
        steps.mover(&mover);
    }
    let mut people = BTreeSet::new();
    let owners: Vec<String> = record
        .items
        .iter()
        .filter_map(|i| match &i.owner {
            Owner::Person { account_id } if people.insert(account_id.as_str()) => {
                Some(account_id.clone())
            }
            _ => None,
        })
        .collect();
    for account_id in owners {
        let target = steps.person(&record, &account_id).await?;
        record.mapping.people.insert(account_id, target);
    }
    let shown = preview(&record, |i| steps.landing(i), table, speed);
    steps.approve(&record, &shown).await?;
    let approval = record.approve()?;
    let allowance = Allowance::from_record(&record, &approval, |i| steps.landing(i))?;
    Ok(ApprovedPlan {
        record,
        allowance,
        version,
    })
}

impl ApprovedPlan {
    /// The plan as the person approved it.
    pub fn record(&self) -> &Record {
        &self.record
    }

    /// Receives over `link` into the places in `table`, only what the plan allows, recording every
    /// write in `journal`; `account` is the signed-in person's account on this laptop. Counted in
    /// `live`'s meter and held or stopped by its control; a stop, on either laptop, ends it with
    /// the report so far ([`Ended`]).
    pub async fn receive<C: Channel>(
        self,
        link: &mut C,
        table: &Destinations,
        journal: &dyn Ledger,
        account: &str,
        live: &Live,
    ) -> Result<Received, MoveError> {
        let mut receiver = ReceiverSession::new(table, self.allowance, journal, account)
            .with_protocol(self.version)
            .with_meter(live.meter.clone())
            .with_control(live.control.clone());
        let ended = ended(receiver.run(link).await)?;
        let stopped = ended != Ended::Finished;
        let files = self
            .record
            .items
            .iter()
            .filter(|i| i.kind == ItemKind::File && i.inclusion == Inclusion::Included)
            .map(|i| ReceivedFile {
                item: i.id,
                changed_since_scan: receiver.changed_since_scan(i.id),
                // A file the stop cut off failed; it is never reported copied.
                outcome: receiver.outcome(i.id).or_else(|| {
                    stopped.then(|| ReceiveOutcome::Failed(Failure::new(Cause::Stopped, STOPPED)))
                }),
            })
            .collect();
        Ok(Received {
            record: self.record,
            files,
            ended,
        })
    }
}

/// The new laptop's side with the app: [`receive_plan`] with `steps`, then `rules` (which must
/// give exactly the places `steps.landing` gives) sent to the old laptop so it announces every
/// file where this laptop checks it, then [`ApprovedPlan::receive`]. `rules` is asked for after
/// the approval, since the people's places are decided during it.
pub async fn receive_by_rules<C: Channel, S: Steps>(
    link: &mut C,
    steps: &mut S,
    rules: impl FnOnce(&S) -> landing::LandingRules,
    table: &Destinations,
    journal: &dyn Ledger,
    account: &str,
    live: &Live,
) -> Result<Received, MoveError> {
    let approved = receive_plan_told(link, steps, table, true).await?;
    landing::send_rules(link, &rules(steps)).await?;
    approved.receive(link, table, journal, account, live).await
}

/// The new laptop's whole side: [`receive_plan`], then [`ApprovedPlan::receive`].
pub async fn receive<C: Channel, S: Steps>(
    link: &mut C,
    steps: &mut S,
    table: &Destinations,
    journal: &dyn Ledger,
    account: &str,
    live: &Live,
) -> Result<Received, MoveError> {
    receive_plan(link, steps, table)
        .await?
        .receive(link, table, journal, account, live)
        .await
}

#[cfg(test)]
mod sign_in {
    use super::runs_at_sign_in;
    use std::path::Path;

    #[test]
    fn places_the_system_starts_at_sign_in_are_named() {
        let roaming = Path::new(r"C:\Users\ada\AppData\Roaming");
        assert!(runs_at_sign_in(
            Some(roaming),
            "Microsoft/Windows/Start Menu/Programs/Startup/updater.exe"
        ));
        assert!(runs_at_sign_in(
            Some(Path::new("/home/ada")),
            ".config/autostart/sync.desktop"
        ));
        assert!(runs_at_sign_in(
            Some(Path::new("/Users/ada/Library")),
            "LaunchAgents/com.example.helper.plist"
        ));
        // Ordinary places, and a folder only named like one, are not.
        assert!(!runs_at_sign_in(
            Some(roaming),
            "Microsoft/Windows/Themes/a.jpg"
        ));
        assert!(!runs_at_sign_in(
            Some(Path::new("/home/ada/Documents")),
            "notes about startup/plan.txt"
        ));
        assert!(!runs_at_sign_in(None, "Documents/letter.txt"));
    }
}
