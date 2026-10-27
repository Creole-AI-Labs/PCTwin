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

pub mod greeting;
pub mod handover;
pub mod held_journal;
pub mod landing;
pub mod people;
pub mod reconnect;
pub mod report;
pub mod resume;
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
pub use pctwin_scan::ReadPlan;
use pctwin_transfer::{
    Allowance, Cause, Channel, Failure, Fit, FsOpener, LandingPlace, MOVE_PROTOCOL, MoveControl,
    MoveMeter, Opener, PlaceFit, ReceiveOutcome, ReceiverSession, SendJob, SendOutcome,
    SenderSession, Tier, TimeRange, TransferError, agree_version, answer_link_test, measure_link,
    plan_order_with, receive_inventory, send_inventory,
};

use reconnect::{LinkState, Reconnect};
use resume::{Comeback, Redial, Redialed};

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
    /// How carefully the old drive is read: [`ReadPlan::Normal`] unless set with
    /// [`read_with`](Self::read_with) (a weak drive, or a rescued one: each file once, the most
    /// important first, a clean stop after repeated read errors).
    pub read_plan: ReadPlan,
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
            read_plan: self.read_plan,
        }
    }

    /// Reads the old drive as `plan` says.
    pub fn read_with(mut self, plan: ReadPlan) -> SendPlan {
        self.read_plan = plan;
        self
    }
}

/// The order a move sends `record`'s items in, each with its tier: what the person asked for
/// first, then their essentials, then the rest. The send and the plan's "we'll move these first"
/// preview both read this one order, so the preview cannot say otherwise than the move does.
pub fn move_order(record: &Record) -> Vec<(ItemId, Tier)> {
    // The person's own essentials in the record's order: first first (the queue puts the larger
    // "when" first within a tier, so the first gets the largest).
    let personal: BTreeMap<ItemId, i64> = record
        .personal
        .iter()
        .enumerate()
        .map(|(rank, id)| (*id, -(rank as i64)))
        .collect();
    plan_order_with(&record.items, &[], &personal, &BTreeMap::new(), 0)
}

/// Whether an item of this tier moves before the rest (asked for, personal or essential).
pub fn moves_first(tier: Tier) -> bool {
    tier < Tier::Rest
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
    for (id, tier) in move_order(record) {
        if let Some(mut job) = by_item.remove(&id) {
            job.tier = tier;
            jobs.push(job);
        }
    }
    jobs.extend(by_item.into_values());
    SendPlan {
        jobs,
        not_sent,
        read_plan: ReadPlan::Normal,
    }
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

/// What the screens hold while the move runs: the meter they read, the person's stop and pause,
/// and the connection (waiting for the other laptop after a drop, with "Try now"). Cheap to clone;
/// the test lab passes a fresh one it never touches.
#[derive(Debug, Clone, Default)]
pub struct Live {
    pub meter: MoveMeter,
    pub control: MoveControl,
    pub connection: reconnect::Connection,
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
    /// After the plan was approved, the connection dropped and the laptops could not find each
    /// other again without pairing again (the time or the tries to reconnect ran out, or the old
    /// laptop could not listen for the new one). What moved is kept.
    LinkLost,
    /// After the plan was approved, the move could not go on: the other laptop sent something it
    /// should not have, or it stopped answering. What moved is kept, and every file it cut off
    /// failed with why ([`BROKE_OFF`]).
    Broke,
}

/// What a file cut off by a stop is reported as: failed, never copied.
pub const STOPPED: &str = "the move was stopped";

/// What a file cut off by a connection that could not be made again is reported as: failed
/// because the move ended, never copied.
pub const LINK_LOST: &str = "the connection between the laptops was lost before it finished";

/// What a file cut off by a move that could not go on ([`Ended::Broke`]) is reported as, before
/// what went wrong: failed because the move ended, never copied.
pub const BROKE_OFF: &str = "the move ended early because of a problem between the laptops";

/// What a file with no outcome of its own is reported as when the move ended as `ended` (`why`:
/// what went wrong, for [`Ended::Broke`]): nothing for a finished move (every file has one then).
fn cut_off(ended: Ended, why: Option<&str>) -> Option<Failure> {
    match ended {
        Ended::Finished => None,
        Ended::StoppedHere | Ended::StoppedThere => Some(Failure::new(Cause::Stopped, STOPPED)),
        Ended::LinkLost => Some(Failure::new(Cause::MoveEnded, LINK_LOST)),
        Ended::Broke => Some(Failure::new(
            Cause::MoveEnded,
            match why {
                Some(why) => format!("{BROKE_OFF} ({why})"),
                None => BROKE_OFF.to_string(),
            },
        )),
    }
}

/// How a run of the files after the approval came out, with every file's result kept: an error
/// that ends it (a message out of place, a laptop that stopped answering) ends the move as
/// [`Ended::Broke`], with what went wrong, rather than losing the report of what already landed.
/// Only this laptop's own record failing (nothing more can be recorded), or a dropped connection
/// where there is no way back (the test lab's way), still fails the move.
fn kept(ran: Result<Ended, MoveError>) -> Result<(Ended, Option<String>), MoveError> {
    match ran {
        Ok(ended) => Ok((ended, None)),
        Err(MoveError::Transfer(e))
            if !matches!(
                e,
                TransferError::Record(_) | TransferError::ConnectionDropped
            ) =>
        {
            Ok((Ended::Broke, Some(e.to_string())))
        }
        Err(e) => Err(e),
    }
}

/// Runs `work`, a wait on the other laptop before any file moves, unless the person here stops
/// first: then it ends at once with [`Stop::Cancelled`], whatever the other laptop is doing (a
/// silent laptop, a person taking their time, a link test fed a byte at a time). `work` is let go
/// where it waits, and a connection let go part way through a read is closed for good, so the
/// other laptop hears the move end when the caller lets the link go. Every wait before the files
/// move goes through here; the transfer has its own stop, which tells the other laptop, and the
/// waits for the other laptop's return after a drop watch the stop themselves.
pub async fn or_stop<T>(
    control: &MoveControl,
    work: impl Future<Output = Result<T, MoveError>>,
) -> Result<T, MoveError> {
    tokio::select! {
        biased;
        () = control.wait_stopped() => Err(Stop::Cancelled.into()),
        done = work => done,
    }
}

/// What a dropped main connection comes to.
enum OnDrop<T> {
    /// The move fails with the drop (before the plan is approved, and for the test lab).
    Fails,
    /// The move ends, with every file's result kept (approved, but no way back).
    Ends,
    /// The laptops find each other again and carry on.
    Resumes(T),
}

/// How much the move has done, to tell whether a connection moved anything before it ended.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
struct Done(u64, u64);

impl Done {
    fn of(meter: &MoveMeter) -> Self {
        let r = meter.reading();
        let f = &r.files;
        Self(
            r.bytes_written.saturating_add(r.bytes_confirmed),
            f.copied + f.already_there + f.needs_action + f.failed,
        )
    }
}

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
    let on_drop = OnDrop::<&mut resume::NoComeback>::Fails;
    run_sender(link, version, plan, live, opener, on_drop).await
}

/// The old laptop's side of a move with the app: as [`send_with`], except the files to send are
/// worked out only once the new laptop's person has approved the plan and the new laptop has
/// sent its [`landing::LandingRules`], so every file is announced exactly where that laptop
/// checks it. `source` says where each file is read from. The laptops greeted first
/// ([`greeting::greet`], before the files were found); its list follows [`greeting::list_coming`].
///
/// Once the plan is approved the old laptop listens for the new laptop's return through `back`
/// ([`resume`]): a dropped connection is then picked up again, without pairing again, and the move
/// carries on where it stopped; if the laptops cannot find each other again the move ends with
/// every file's result kept ([`Ended::LinkLost`]). A drop before then fails the move.
pub async fn send_by_rules<C: Channel, B: Comeback>(
    link: &mut greeting::Greeted<'_, C>,
    record: &Record,
    who: &Who<'_>,
    source: impl Fn(&Item) -> Option<PathBuf>,
    live: &Live,
    opener: Arc<dyn Opener>,
    back: &mut B,
) -> Result<(Sent, landing::LandingRules), MoveError> {
    // The link was tested when the laptops greeted.
    let version = link.greeting().version;
    // Everything before the files move waits on the other laptop (its person may take as long as
    // they like to approve): the person here can stop it at any point.
    let (rules, port) = or_stop(&live.control, async {
        greeting::list_coming(link).await?;
        send_inventory(link, record, version).await?;
        landing::send_mover(link, who.mover).await?;
        landing::send_people(link, who.people).await?;
        let rules = landing::receive_rules(link).await?;
        // The plan is approved: the way back opens, for exactly this plan.
        let port = back.open(&resume::plan_print(&record.items, &rules)).await;
        resume::say_port(link, port).await?;
        Ok((rules, port))
    })
    .await?;
    // An older new laptop was sent no personal order, so none is used here either: the order
    // its person saw is the order sent.
    let mut plain = None;
    if version < pctwin_transfer::PERSONAL_SINCE && !record.personal.is_empty() {
        let mut r = record.clone();
        r.personal.clear();
        plain = Some(r);
    }
    let record = plain.as_ref().unwrap_or(record);
    let plan = send_plan_where(record, source, |i| rules.landing(i));
    let on_drop = match port {
        Some(_) => OnDrop::Resumes(back),
        None => OnDrop::Ends,
    };
    let sent = run_sender(link, version, plan, live, opener, on_drop).await?;
    Ok((sent, rules))
}

/// Who is in the move, as the old laptop tells the new one: the person moving (their account ID
/// here) and the other people, by name.
#[derive(Debug, Clone, Copy)]
pub struct Who<'a> {
    pub mover: &'a str,
    pub people: &'a [landing::NamedPerson],
}

/// Runs the old laptop's sending of `plan` over `link` (the version agreed): one outcome per
/// file, a stop said as one.
async fn run_sender<C: Channel, B: Comeback>(
    link: &mut C,
    version: u16,
    plan: SendPlan,
    live: &Live,
    opener: Arc<dyn Opener>,
    on_drop: OnDrop<&mut B>,
) -> Result<Sent, MoveError> {
    let items: Vec<ItemId> = plan.jobs.iter().map(|j| j.item).collect();
    let mut sender = SenderSession::new(plan.jobs, IN_FLIGHT)
        .with_read_plan(plan.read_plan)
        .with_opener(opener)
        .with_protocol(version)
        .with_meter(live.meter.clone())
        .with_control(live.control.clone());
    let (ended, why) = kept(keep_sending(&mut sender, link, on_drop, live).await)?;
    let cut = cut_off(ended, why.as_deref());
    Ok(Sent {
        files: items
            .into_iter()
            .map(|item| SentFile {
                item,
                // A file a stop or a lost connection cut off failed; it is never reported copied.
                outcome: sender
                    .outcome(item)
                    .cloned()
                    .or_else(|| cut.clone().map(SendOutcome::Failed)),
            })
            .collect(),
        not_sent: plan.not_sent,
        ended,
    })
}

/// The old laptop sends over `first`, then (when it resumes) over each connection the new laptop
/// comes back on, until the move ends. While waiting for the new laptop the screen says so; a stop
/// then ends the move at once.
async fn keep_sending<C: Channel, B: Comeback>(
    sender: &mut SenderSession,
    first: &mut C,
    on_drop: OnDrop<&mut B>,
    live: &Live,
) -> Result<Ended, MoveError> {
    let back = match on_drop {
        OnDrop::Resumes(back) => back,
        OnDrop::Fails => return ended(sender.run(first).await),
        OnDrop::Ends => return lost_for_good(sender.run(first).await, live),
    };
    let mut policy = Reconnect::new();
    let mut now_on: Option<B::Link> = None;
    loop {
        let before = Done::of(&live.meter);
        let ran = {
            let run = async {
                match now_on.as_mut() {
                    Some(link) => sender.run(link).await,
                    None => sender.run(first).await,
                }
            };
            // The new laptop may come back while this connection still looks alive here (it gave
            // up on it first): the move carries on over the new one, and this one is let go. (A run
            // is only ever left at a wait, so the session stays as a drop leaves it.)
            let came_back = async {
                match back.returned().await {
                    Some(link) => link,
                    // It can no longer come back: this connection carries on while it lasts.
                    None => std::future::pending().await,
                }
            };
            tokio::select! {
                ran = run => Err(ran),
                link = came_back => Ok(link),
            }
        };
        let ran = match ran {
            Ok(link) => {
                now_on = Some(link);
                live.connection.set(LinkState::Connected);
                continue;
            }
            Err(ran) => ran,
        };
        if !matches!(ran, Err(TransferError::ConnectionDropped)) {
            return ended(ran);
        }
        // The connection dropped: it is let go, and this laptop waits for the new one's return as
        // long as its keys last. The waits are the new laptop's own, so this screen says "nothing
        // new" when that one would.
        drop(now_on.take());
        let since = tokio::time::Instant::now();
        let mut next = policy.ended(Done::of(&live.meter) != before);
        let came_back = back.returned();
        tokio::pin!(came_back);
        let wait = tokio::time::sleep(next.wait);
        tokio::pin!(wait);
        let link = loop {
            live.connection.set(LinkState::Waiting {
                since,
                nothing_new: next.nothing_new,
            });
            tokio::select! {
                biased;
                () = live.control.wait_stopped() => return Ok(Ended::StoppedHere),
                link = &mut came_back => match link {
                    Some(link) => break link,
                    None => {
                        live.connection.set(LinkState::Over);
                        return Ok(Ended::LinkLost);
                    }
                },
                () = &mut wait => {
                    next = policy.ended(false);
                    wait.as_mut().reset(tokio::time::Instant::now() + next.wait);
                }
            }
        };
        now_on = Some(link);
        live.connection.set(LinkState::Connected);
    }
}

/// A run that ends, with no way back: a dropped connection ends the move with every result kept.
fn lost_for_good(ran: Result<(), TransferError>, live: &Live) -> Result<Ended, MoveError> {
    match ran {
        Err(TransferError::ConnectionDropped) => {
            live.connection.set(LinkState::Over);
            Ok(Ended::LinkLost)
        }
        ran => ended(ran),
    }
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
    /// With the app's way of moving ([`receive_by_rules`]) it is asked again, before anything is
    /// written, if once every person's place is made a place is shorter of space than it showed.
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

    /// The other people whose things are in the move, by name, as the old laptop said (the app's
    /// way of moving), before anyone is placed.
    fn people(&mut self, _people: &[landing::NamedPerson]) {}

    /// Files the person chose, at their approval, to leave out (whole folders of theirs, say):
    /// they are left out of the plan as their own choice, and `landing` gives them no place.
    fn left_out(&self, _record: &Record) -> Vec<ItemId> {
        Vec::new()
    }
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
    /// The person's approval of exactly `record`.
    approval: pctwin_record::Approval,
    allowance: Allowance,
    /// The move protocol version both laptops agreed.
    version: u16,
    /// The old laptop's list as it arrived, for the plan's fingerprint ([`resume::plan_print`]).
    listed: blake3::Hasher,
    /// How far short of space the preview the person approved was ([`most_short`]).
    short: u64,
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
    let version = agree_version(link, MOVE_PROTOCOL).await?;
    receive_plan_told(link, version, None, steps, table).await
}

/// [`receive_plan`]. With `greeted` (the app's way, [`receive_by_rules`]), the link speed is the
/// one measured when the laptops greeted, and the old laptop says who is moving after its list,
/// which `steps` hears before anyone is placed; without, the link is tested after the list.
async fn receive_plan_told<C: Channel, S: Steps>(
    link: &mut C,
    version: u16,
    greeted: Option<Option<u64>>,
    steps: &mut S,
    table: &Destinations,
) -> Result<ApprovedPlan, MoveError> {
    let mut record = receive_inventory(link, version).await?;
    let listed = resume::print_items(&record.items);
    let told = greeted.is_some();
    let speed = match greeted {
        Some(measured) => measured,
        None => measure_link(link, version).await?,
    };
    if told {
        let mover = landing::receive_mover(link).await?;
        steps.mover(&mover);
        let people = landing::receive_people(link).await?;
        steps.people(&people);
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
    leave_out(steps, &mut record);
    let approval = record.approve()?;
    let allowance = Allowance::from_record(&record, &approval, |i| steps.landing(i))?;
    Ok(ApprovedPlan {
        record,
        approval,
        allowance,
        version,
        listed,
        short: most_short(&shown),
    })
}

/// Marks the files the person chose, at their approval, to leave out (see [`Steps::left_out`]).
/// Whether any was.
fn leave_out<S: Steps>(steps: &S, record: &mut Record) -> bool {
    let out: std::collections::HashSet<ItemId> = steps.left_out(record).into_iter().collect();
    let mut changed = false;
    for item in &mut record.items {
        if out.contains(&item.id) && item.inclusion == Inclusion::Included {
            item.inclusion = Inclusion::LeftOut {
                reason: LeftOutReason::ChosenByUser,
            };
            changed = true;
        }
    }
    changed
}

/// How far short of space the worst place in `shown` is (0: every place fits, or is not known).
fn most_short(shown: &Preview) -> u64 {
    shown
        .fit
        .places
        .iter()
        .map(PlaceFit::short_by)
        .max()
        .unwrap_or(0)
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
        let here = Here { journal, account };
        self.receive_on(
            link,
            table,
            here,
            live,
            OnDrop::<&mut resume::NoRedial>::Fails,
        )
        .await
    }

    /// [`receive`](Self::receive), with what a dropped connection comes to.
    async fn receive_on<C: Channel, R: Redial>(
        self,
        link: &mut C,
        table: &Destinations,
        here: Here<'_>,
        live: &Live,
        on_drop: OnDrop<&mut R>,
    ) -> Result<Received, MoveError> {
        let mut receiver = ReceiverSession::new(table, self.allowance, here.journal, here.account)
            .with_protocol(self.version)
            .with_meter(live.meter.clone())
            .with_control(live.control.clone());
        let (ended, why) = kept(keep_receiving(&mut receiver, link, on_drop, live).await)?;
        let cut = cut_off(ended, why.as_deref());
        let files = self
            .record
            .items
            .iter()
            .filter(|i| i.kind == ItemKind::File && i.inclusion == Inclusion::Included)
            .map(|i| ReceivedFile {
                item: i.id,
                changed_since_scan: receiver.changed_since_scan(i.id),
                // A file a stop or a lost connection cut off failed; it is never reported copied.
                outcome: receiver
                    .outcome(i.id)
                    .or_else(|| cut.clone().map(ReceiveOutcome::Failed)),
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
/// the approval, since the people's places are decided during it; it may add places to `table`
/// then (each other person's holding place, made only once the plan is approved). The laptops
/// greeted first ([`greeting::greet`]); the old laptop's list is waited for however long it takes.
///
/// Once the plan is approved the new laptop can come back to the old one through `redial`
/// ([`resume`]) after a drop: it waits a little longer after each try that brings nothing, tries
/// at once when the person asks ([`reconnect::Connection::try_now`]), and the move carries on where
/// it stopped. If the laptops cannot find each other again without pairing again the move ends
/// with every file's result kept ([`Ended::LinkLost`]). A drop before then fails the move.
pub async fn receive_by_rules<C: Channel, S: Steps, R: Redial>(
    link: &mut greeting::Greeted<'_, C>,
    steps: &mut S,
    rules: impl FnOnce(&mut S, &mut Destinations) -> landing::LandingRules,
    table: &mut Destinations,
    here: Here<'_>,
    live: &Live,
    redial: &mut R,
) -> Result<Received, MoveError> {
    // Everything before the files move waits on the other laptop (its person may take as long as
    // they like to find the files): the person here can stop it at any point.
    let (approved, port, plan) = or_stop(&live.control, async {
        // Greeted already: the list comes once the old laptop has found the files, however long
        // its person takes.
        greeting::wait_for_list(link).await?;
        let (version, measured) = (link.greeting().version, link.greeting().bytes_per_second);
        let mut approved = receive_plan_told(link, version, Some(measured), steps, table).await?;
        let rules = rules(steps, table);
        // The places made after the approval (another person's holding place, a folder the person
        // chose) hold files the preview could not count where they now land. The files are the
        // ones approved (another person's with their place chosen, their files and size shown),
        // but if where they land now leaves a place shorter of space than the person saw, they
        // see the plan as it will run and approve it again (or stop, with nothing written).
        let placed = preview(&approved.record, |i| rules.landing(i), table, measured);
        if most_short(&placed) > approved.short {
            steps.approve(&approved.record, &placed).await?;
            approved.short = most_short(&placed);
            if leave_out(steps, &mut approved.record) {
                approved.approval = approved.record.approve()?;
            }
        }
        // A folder that moves as one whole never lands merged into one already here, nor into
        // another of this move: where each is stored is decided here, once every place is known.
        let wholes = landing::place_wholes(&approved.record.items, &rules, |destination, path| {
            let dest = table.get(destination).ok()?;
            let met = dest.meet(&pctwin_gate::IncomingPath::parse(path).ok()?);
            Some((met.key, met.taken))
        });
        // What this laptop accepts is made from the very rules the old laptop is sent, now that
        // every place is known (another person's holding place is made only after the
        // approval): both laptops place every file by one function, by construction.
        approved.allowance =
            Allowance::from_record(&approved.record, &approved.approval, |i| rules.landing(i))?
                .with_wholes(wholes);
        landing::send_rules(link, &rules).await?;
        // The plan is approved and the rules said: both laptops make their resume keys for
        // exactly this plan, and the old laptop says where it waits for this one's return.
        let plan = resume::finish_print(approved.listed.clone(), &rules);
        let port = resume::hear_port(link).await?;
        Ok((approved, port, plan))
    })
    .await?;
    let on_drop = match port {
        Some(port) if redial.prepare(&plan, port) => OnDrop::Resumes(redial),
        _ => OnDrop::Ends,
    };
    approved.receive_on(link, table, here, live, on_drop).await
}

/// Where the new laptop records what it writes: the change journal, and the signed-in person's
/// account here (who acts).
#[derive(Clone, Copy)]
pub struct Here<'a> {
    pub journal: &'a dyn Ledger,
    pub account: &'a str,
}

/// The new laptop receives over `first`, then (when it resumes) over each connection it comes back
/// on, until the move ends. While waiting the screen says so; a stop then ends the move at once.
async fn keep_receiving<C: Channel, R: Redial>(
    receiver: &mut ReceiverSession<'_>,
    first: &mut C,
    on_drop: OnDrop<&mut R>,
    live: &Live,
) -> Result<Ended, MoveError> {
    let redial = match on_drop {
        OnDrop::Resumes(redial) => redial,
        OnDrop::Fails => return ended(receiver.run(first).await),
        OnDrop::Ends => return lost_for_good(receiver.run(first).await, live),
    };
    let mut policy = Reconnect::new();
    let mut before = Done::of(&live.meter);
    let mut ran = receiver.run(first).await;
    loop {
        if !matches!(ran, Err(TransferError::ConnectionDropped)) {
            return ended(ran);
        }
        let since = tokio::time::Instant::now();
        let mut next = policy.ended(Done::of(&live.meter) != before);
        let mut link = loop {
            live.connection.set(LinkState::Waiting {
                since,
                nothing_new: next.nothing_new,
            });
            tokio::select! {
                biased;
                () = live.control.wait_stopped() => return Ok(Ended::StoppedHere),
                () = live.connection.tried_now() => {}
                () = tokio::time::sleep(next.wait) => {}
            }
            let tried = tokio::select! {
                biased;
                () = live.control.wait_stopped() => return Ok(Ended::StoppedHere),
                tried = redial.redial() => tried,
            };
            match tried {
                Ok(link) => break link,
                Err(Redialed::NotThere) => next = policy.ended(false),
                Err(Redialed::Over) => {
                    live.connection.set(LinkState::Over);
                    return Ok(Ended::LinkLost);
                }
            }
        };
        live.connection.set(LinkState::Connected);
        before = Done::of(&live.meter);
        ran = receiver.run(&mut link).await;
    }
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
