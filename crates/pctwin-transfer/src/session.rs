use std::cell::{Cell, RefCell};
use std::collections::{BTreeMap, HashMap, HashSet, VecDeque};
use std::future::Future;
use std::io::Read;
use std::path::{Path, PathBuf};
use std::sync::Arc;
use std::sync::atomic::{AtomicU64, Ordering};
use std::time::{Duration, Instant};

use pctwin_gate::{Approved, Claim, Destination, Destinations, Finished, GateError, IncomingPath};
use pctwin_journal::{
    Actor, FileId, Journal, JournalError, Landed, Ledger, Permission, PlannedWrite, State,
};
use pctwin_record::ItemId;

use crate::cause::{Cause, Failure, Side};
use crate::message::{
    BLOCK_WIRE_OVERHEAD, GiveUpWhy, Message, NONCE_LEN, Nonce, split_into_pieces,
};
use crate::meter::{Group, MoveControl, MoveMeter, Step};
use crate::queue::{Scheduler, Tier};
use crate::reading::{ReadBudget, is_drive_error};
use crate::sections::FileSections;
use crate::send_states::States;
use crate::version::{COPY_SINCE, HOLD_SINCE, KEYED_SINCE, STOP_SINCE};
use crate::{
    Allowance, Assembly, Block, Confirmed, FileSender, FsOpener, MAX_ORIGINALS_PER_REQUEST, Opener,
    OriginalNow, OriginalsFault, ResumeTicket, StoredCheck, Trailer, TransferError,
    answer_originals,
};
use futures_util::StreamExt;
use futures_util::stream::FuturesUnordered;
use pctwin_scan::ReadPlan;
use tokio::sync::Notify;
use tokio::sync::mpsc::UnboundedReceiver;

/// Why the files not yet read were left: reading stopped to protect a failing drive.
const STOPPED: &str =
    "not read: the old drive kept failing, so PCTwin stopped reading it to protect it";

/// How much may be sent before the receiver confirms it (as Syncthing does): enough to keep the
/// connection busy, small enough that memory stays bounded.
const IN_FLIGHT_BYTES: u64 = 32 * 1024 * 1024;

/// While a move is held by a pause, the old laptop says so this often, and the new laptop
/// answers: neither takes the quiet for a dropped link, and a link that does drop during a pause
/// is found within about this long (or the link's own keep-alive, if that is later) and is
/// reported as a drop.
pub const HOLD_TICK: Duration = Duration::from_secs(1);

/// Most files the new laptop keeps open (started and not ended) at once. The old laptop keeps far
/// fewer in flight; the limit stops one that starts files and never finishes them from holding
/// space for all of them. (rsync and others time out the connection, not each file, so a paused
/// move or a slow old drive never loses a file to a timer.)
pub const MAX_OPEN_FILES: usize = 64;

/// How long an extra lane may stay silent while receipts are owed on it before it is let go and
/// its blocks are sent on the others (the idle limit QUIC uses). The main connection is never
/// timed out this way: losing it ends the move, so a dead one is left to the keep-alive.
pub const LANE_SILENCE: std::time::Duration = std::time::Duration::from_secs(30);

/// Files not in the plan whose refusals are remembered (for the report); beyond this they are
/// still refused, just not remembered.
const MAX_REMEMBERED_REFUSALS: usize = 1024;

/// One connection's partial block on the new laptop: which file it belongs to, and its pieces so
/// far.
type Slot = Option<(u32, Vec<u8>)>;

/// The connection dropped.
#[derive(Debug, Clone, Copy, PartialEq, Eq, thiserror::Error)]
#[error("the connection dropped")]
pub struct ChannelError;

/// A connection that carries whole messages in order, one at a time each way.
pub trait Channel {
    fn send(&mut self, data: &[u8]) -> impl Future<Output = Result<(), ChannelError>>;
    fn recv(&mut self) -> impl Future<Output = Result<Vec<u8>, ChannelError>>;
}

impl Channel for pctwin_link::Link {
    async fn send(&mut self, data: &[u8]) -> Result<(), ChannelError> {
        pctwin_link::Link::send(self, data)
            .await
            .map_err(|_| ChannelError)
    }

    async fn recv(&mut self) -> Result<Vec<u8>, ChannelError> {
        pctwin_link::Link::recv(self)
            .await
            .map(|m| m.to_vec())
            .map_err(|_| ChannelError)
    }
}

/// How long a laptop that stops the move waits to tell the other one so.
const STOP_NOTICE: Duration = Duration::from_secs(2);

/// Tells the other laptop the person stopped the move, then keeps the connection open until the
/// other laptop lets it go, at most [`STOP_NOTICE`] in all: a laptop that writes before it reads
/// (a paused old laptop saying it is still held) would otherwise find the connection gone before
/// it read the notice, and report a lost connection. What arrives meanwhile is let go unread. Best
/// effort: if the stop cut a message short, the link refuses to send more, and the other laptop
/// sees the connection end as it would have without the notice.
async fn tell_stopped(main: &mut impl Channel) {
    let _ = tokio::time::timeout(STOP_NOTICE, async {
        if send(main, &Message::Stopping).await.is_ok() {
            while main.recv().await.is_ok() {}
        }
    })
    .await;
}

async fn send(ch: &mut impl Channel, m: &Message) -> Result<(), TransferError> {
    ch.send(&m.encode())
        .await
        .map_err(|_| TransferError::ConnectionDropped)
}

async fn recv(ch: &mut impl Channel) -> Result<Message, TransferError> {
    let bytes = ch
        .recv()
        .await
        .map_err(|_| TransferError::ConnectionDropped)?;
    Message::decode(&bytes)
}

fn protocol(why: &str) -> TransferError {
    TransferError::Protocol(why.to_string())
}

/// How long the new laptop waits for the old laptop's answer to one request. Longer than the old
/// laptop's own limit for a request ([`ORIGINALS_REQUEST_TIME`]), so a slow old laptop answers
/// "cannot look" itself before the new laptop gives up.
pub const ORIGINALS_REPLY_WAIT: Duration = Duration::from_secs(90);

/// New laptop, for undo: asks the old laptop, over the paired connection, what it sees now for
/// each of `items`, and returns the answers as a [`Confirmed`] tied to `journal` and to this
/// moment. Every item asked gets an answer in it, and only an exact, single, well-formed answer
/// counts: an item the old laptop skipped, answered twice, or answered with something else is
/// `CannotLook` (never "confirmed"), and an answer for an item that was not asked is thrown away.
/// A damaged or oversized message, or no answer in time, is an error (confirm nothing: use
/// [`Confirmed::none`]). Long lists go in several requests of at most
/// `MAX_ORIGINALS_PER_REQUEST`, so the old laptop must call [`serve_originals`] once for each.
pub async fn check_originals(
    ch: &mut impl Channel,
    journal: &Journal,
    items: &[ItemId],
) -> Result<Confirmed, TransferError> {
    check_originals_within(ch, journal, items, ORIGINALS_REPLY_WAIT).await
}

pub(crate) async fn check_originals_within(
    ch: &mut impl Channel,
    journal: &Journal,
    items: &[ItemId],
    wait: Duration,
) -> Result<Confirmed, TransferError> {
    // Taken before asking: the answers can only be as old as this or newer.
    let taken = Instant::now();
    let mut seen = HashSet::new();
    let unique: Vec<ItemId> = items.iter().copied().filter(|i| seen.insert(*i)).collect();
    let mut result = HashMap::new();
    let mut faults = Vec::new();
    for chunk in unique.chunks(MAX_ORIGINALS_PER_REQUEST) {
        // Fresh for every request, so a reply can only ever answer the one question just asked.
        let nonce = fresh_nonce()?;
        send(
            ch,
            &Message::CheckOriginals {
                nonce,
                items: chunk.to_vec(),
            },
        )
        .await?;
        let reply = tokio::time::timeout(wait, recv(ch))
            .await
            .map_err(|_| protocol("the old laptop did not answer about the originals in time"))??;
        let Message::Originals {
            nonce: echoed,
            answers,
        } = reply
        else {
            return Err(protocol("expected the answer about the originals"));
        };
        if echoed == nonce {
            result.extend(match_answers(chunk, answers));
        } else {
            // A stale or replayed reply (or one for another question): none of it is believed.
            // The request's reply is spent either way, so the next request starts clean.
            faults.push(OriginalsFault::WrongNonce { items: chunk.len() });
            result.extend(chunk.iter().map(|i| (*i, OriginalNow::CannotLook)));
        }
    }
    Ok(Confirmed::from_answers(
        journal.instance(),
        taken,
        result,
        faults,
    ))
}

/// 128 random bits from the operating system. Without them nothing is asked: an old laptop's
/// answer cannot be tied to a question, so none is trusted.
fn fresh_nonce() -> Result<Nonce, TransferError> {
    let mut nonce = [0u8; NONCE_LEN];
    getrandom::fill(&mut nonce).map_err(|_| protocol("no random numbers to ask with"))?;
    Ok(nonce)
}

/// Pairs answers with the items that were asked, strictly. Anything not exactly one answer to a
/// question that was asked becomes `CannotLook`.
fn match_answers(
    asked: &[ItemId],
    answers: Vec<(ItemId, OriginalNow)>,
) -> HashMap<ItemId, OriginalNow> {
    let asked_set: HashSet<ItemId> = asked.iter().copied().collect();
    let mut got: HashMap<ItemId, Option<OriginalNow>> = HashMap::new();
    for (item, now) in answers {
        if !asked_set.contains(&item) {
            continue;
        }
        // A second answer for the same item spoils it: neither one is trusted.
        match got.get_mut(&item) {
            Some(slot) => *slot = None,
            None => {
                got.insert(item, Some(now));
            }
        }
    }
    asked
        .iter()
        .map(|item| {
            let now = got
                .get(item)
                .copied()
                .flatten()
                .unwrap_or(OriginalNow::CannotLook);
            (*item, now)
        })
        .collect()
}

/// Most items one session may ask the old laptop about, in all. A move holds at most a few
/// hundred thousand files in ordinary use; a million is well beyond that, yet the old laptop's
/// disk is looked at a bounded number of times however the new laptop behaves.
pub const MAX_ORIGINALS_PER_SESSION: usize = 1_000_000;

/// Most requests one session may make, in all. A full million items is about a thousand full
/// requests; four times that leaves room, while one-item requests cannot be sent without end.
pub const MAX_ORIGINAL_REQUESTS_PER_SESSION: usize = 4096;

/// How long the old laptop spends looking for one request. A request is at most 1024 looks; a
/// sleeping disk or a slow share can take a few seconds each at worst, so a minute covers real
/// delays. When it runs out the answer is "cannot look", which is the safe way to fail.
pub const ORIGINALS_REQUEST_TIME: Duration = Duration::from_secs(60);

/// What the old laptop has been asked in one session so far. Make one per session and pass it to
/// every [`serve_originals`] call of that session.
#[derive(Debug)]
pub struct OriginalsBudget {
    items_left: usize,
    requests_left: usize,
    time: Duration,
    /// True while a look that ran out of time is still going on a blocked thread. No further look
    /// is started until it ends, so a hung drive or share holds one thread, never a growing pile.
    look_running: Arc<std::sync::atomic::AtomicBool>,
}

/// Clears the flag when the look ends, however it ends.
struct LookEnded(Arc<std::sync::atomic::AtomicBool>);

impl Drop for LookEnded {
    fn drop(&mut self) {
        self.0.store(false, Ordering::SeqCst);
    }
}

impl OriginalsBudget {
    pub fn new() -> Self {
        Self::with_limits(
            MAX_ORIGINALS_PER_SESSION,
            MAX_ORIGINAL_REQUESTS_PER_SESSION,
            ORIGINALS_REQUEST_TIME,
        )
    }

    pub(crate) fn with_limits(items: usize, requests: usize, time: Duration) -> Self {
        Self {
            items_left: items,
            requests_left: requests,
            time,
            look_running: Arc::new(std::sync::atomic::AtomicBool::new(false)),
        }
    }
}

impl Default for OriginalsBudget {
    fn default() -> Self {
        Self::new()
    }
}

/// Old laptop, for undo: answers one `CheckOriginals` request from what it sent (`sent`: item to
/// source path). It looks only at files in `sent`, read-only, and changes nothing. Call it once
/// per request, when no file is being sent on this connection.
///
/// `budget` limits the whole session. A request over the session's totals, or one whose looking
/// takes longer than the time limit, is still answered (so the new laptop is not left waiting),
/// but every item in it is `CannotLook`: never confirmed.
pub async fn serve_originals(
    ch: &mut impl Channel,
    sent: &HashMap<ItemId, PathBuf>,
    budget: &mut OriginalsBudget,
) -> Result<(), TransferError> {
    serve_with(ch, sent, budget, answer_originals).await
}

pub(crate) async fn serve_with<F>(
    ch: &mut impl Channel,
    sent: &HashMap<ItemId, PathBuf>,
    budget: &mut OriginalsBudget,
    answer: F,
) -> Result<(), TransferError>
where
    F: FnOnce(&HashMap<ItemId, PathBuf>, &[ItemId]) -> Vec<(ItemId, OriginalNow)> + Send + 'static,
{
    let Message::CheckOriginals { nonce, items } = recv(ch).await? else {
        return Err(protocol("expected a request about the originals"));
    };
    let refuse = |items: &[ItemId]| -> Vec<(ItemId, OriginalNow)> {
        items
            .iter()
            .map(|i| (*i, OriginalNow::CannotLook))
            .collect()
    };
    // A look that timed out and has not ended yet is still holding its thread: refuse, do not add.
    let busy = budget.look_running.load(Ordering::SeqCst);
    let within = budget.requests_left > 0 && items.len() <= budget.items_left && !busy;
    budget.requests_left = budget.requests_left.saturating_sub(1);
    let answers = if within {
        budget.items_left -= items.len();
        // Only the asked items' paths go to the worker; the new laptop never names a path.
        let known: HashMap<ItemId, PathBuf> = items
            .iter()
            .filter_map(|i| sent.get(i).map(|p| (*i, p.clone())))
            .collect();
        let asked = items.clone();
        budget.look_running.store(true, Ordering::SeqCst);
        let ended = LookEnded(Arc::clone(&budget.look_running));
        let work = tokio::task::spawn_blocking(move || {
            let _ended = ended;
            answer(&known, &asked)
        });
        match tokio::time::timeout(budget.time, work).await {
            Ok(Ok(answers)) => answers,
            Ok(Err(_)) => return Err(protocol("the look at the originals failed")),
            // The look runs on in the background until it ends by itself; its result is dropped.
            Err(_) => refuse(&items),
        }
    } else {
        // Over the session's totals: nothing more is looked at this session, so what was left is
        // spent and it stays refused. (A look still hung only refuses this request, not later ones.)
        if !busy {
            budget.items_left = 0;
        }
        refuse(&items)
    };
    // The request's nonce goes back with the answers, refused ones included.
    send(ch, &Message::Originals { nonce, answers }).await
}

/// One file the old laptop sends.
#[derive(Debug, Clone)]
pub struct SendJob {
    pub item: ItemId,
    /// Where the file is on the old laptop.
    pub source: PathBuf,
    /// The approved destination's label on the new laptop.
    pub destination: String,
    /// The path inside that destination.
    pub path: String,
    /// False for formats that are already compressed.
    pub compressible: bool,
    pub tier: Tier,
    /// Its size at the scan (0: not known). The old laptop's time left counts down from it.
    pub size: u64,
}

/// How a sent file ended.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum SendOutcome {
    /// The new laptop confirmed it is complete under its real name.
    Arrived,
    /// The new laptop already had an identical file there, so it was not sent.
    AlreadyThere,
    Failed(Failure),
}

#[expect(
    clippy::large_enum_variant,
    reason = "one per file in flight (a bounded few); boxing the reader buys nothing"
)]
pub(crate) enum SendState {
    Waiting,
    Open {
        reader: FileSender,
        /// Which lane sends which blocks; done blocks (from a resume) are already marked.
        sections: FileSections,
        /// The start was sent on this connection.
        announced: bool,
        /// How many blocks the new laptop's resume ticket said it had (0 for a fresh start).
        resumed_done: u64,
        /// The new laptop answered the start (nothing identical there), so blocks may go.
        cleared: bool,
    },
    /// The end was sent; waiting for the new laptop's answer.
    Ended {
        /// Send the whole file again if the answer is no (it changed while being read).
        retry: bool,
        /// Why it failed on this side, if it did.
        failure: Option<Failure>,
        /// It could not be read; try it once more after everything else.
        later: bool,
    },
    /// Found identical on the new laptop and told to skip it; it is already there only once the
    /// new laptop answers so.
    Skipping,
    Done(SendOutcome),
}

/// What a lane does next.
enum Work {
    /// Send this block (its wire form) of this file.
    Send { stream: u32, wire: Vec<u8> },
    /// Nothing to send now.
    Wait,
}

/// The old laptop's side of a move. Keeps its place across dropped connections: call
/// [`run`](Self::run) or [`run_lanes`](Self::run_lanes) again with a new connection to continue.
///
/// Each attempt at a file has its own stream number: a file started again (it changed while
/// being read) gets a new one, so a block of the old attempt still travelling on another lane can
/// never land in the new one. A file continued after a drop keeps its stream.
pub struct SenderSession {
    jobs: Vec<SendJob>,
    /// Per job, each change counted in the meter.
    states: States,
    /// Per job: the stream of its current attempt.
    streams: Vec<u32>,
    /// Stream to job, for current attempts only.
    job_of: HashMap<u32, usize>,
    next_stream: u32,
    by_item: HashMap<ItemId, usize>,
    scheduler: Scheduler,
    capacity: usize,
    in_flight_limit: u64,
    /// Jobs found identical on the new laptop, to tell it to skip.
    skips: Vec<usize>,
    /// Ends to send on the main connection (a file finished, or failed partway).
    ends: Vec<Message>,
    /// Planned files failed here before the new laptop heard of them, with why (one per job at
    /// most), and how many of them were said on this connection.
    not_sent: Vec<(ItemId, Cause)>,
    not_sent_said: usize,
    opener: Arc<dyn Opener>,
    budget: ReadBudget,
    /// Jobs that could not be read, waiting for one more try after everything else.
    later: Vec<usize>,
    /// Jobs already given their second try.
    retried: HashSet<usize>,
    /// Files whose skip the new laptop did not accept: sent after all, never skipped again.
    no_skip: HashSet<usize>,
    /// Per lane (0 is the main connection): blocks sent and not yet confirmed, oldest first, as
    /// (stream, block, bytes). Receipts come back in order on the lane that carried the block.
    lanes: Vec<VecDeque<(u32, u32, u64)>>,
    /// Bytes sent and not yet confirmed, over every lane.
    in_flight: u64,
    /// Something changed that other lanes may be waiting for.
    dirty: bool,
    /// Same-size files on the new laptop whose fingerprints are still to be compared.
    checks: Vec<(usize, [u8; 32])>,
    /// Jobs opened at least once: opening one again starts a new attempt on a new stream.
    attempted: HashSet<usize>,
    /// Per job: times the new laptop asked for some of its blocks again in the current attempt,
    /// kept across connections (a new attempt starts again from 0, as on the new laptop), so a
    /// new laptop that keeps asking cannot make this one read and send it without end.
    asked_again: Vec<u8>,
    /// Per job: how important it was when it last ended (raised if the person asked for it
    /// first), so a file asked for again keeps its place.
    ended_tier: Vec<Tier>,
    /// Per job: the file as this laptop read it for the current attempt. Whether it changed is
    /// judged against this, never against what the new laptop says it is.
    attempt_stamp: Vec<Option<crate::Stamp>>,
    /// Each block on its way and not yet confirmed, as sent (by stream and block), so a drop
    /// keeps it rather than reading it again.
    on_the_way: HashMap<(u32, u32), Vec<u8>>,
    /// Blocks a drop cut off, kept to send again without reading the drive again (a weak drive
    /// is read once): by job and block, with the version of the file they were read from.
    spool: HashMap<(usize, u32), (crate::Stamp, Vec<u8>)>,
    spooled: u64,
    /// The most that may be in flight now, in raw block bytes: the whole limit until a loss, then
    /// the floor, growing by each block receipted written (TCP's slow start with byte counting,
    /// RFC 5681 and RFC 3465). Every block a loss makes this laptop read again was earned by
    /// progress it recorded itself, so a poor connection slows the move but never fails a file.
    window: u64,
    /// The drive is read carefully: the floor is one block.
    careful: bool,
    /// Whether a run started already, and whether the last one ended well (one that did not was
    /// a loss).
    ran: bool,
    ended_well: bool,
    /// Per job: raw bytes of its blocks the new laptop receipted (written or not) for the version
    /// in `attempt_stamp`, over every attempt, connection and round at that version; past
    /// [`answered_cap`] it stops. Drops never count: a lost block is never receipted.
    answered: Vec<u64>,
    /// Per job: versions of it (stamps unlike the one before) this laptop opened. Only the first
    /// [`MAX_ATTEMPTS`](crate::MAX_ATTEMPTS) are ever read.
    versions: Vec<u32>,
    /// Per job: its fingerprint, worked out once for the version and the new laptop's key it
    /// was worked out with (`None` when it could not be: then it is "not identical").
    fingerprint: Vec<Option<Printed>>,
    /// Per job being skipped (move protocol 4): its fingerprint under the new laptop's key, sent
    /// with `Print` for the new laptop to judge.
    print_of: HashMap<usize, [u8; 32]>,
    /// Per job: the end (or stop) last said for it, said again on each new connection while it
    /// is unanswered.
    last_end: Vec<Option<Message>>,
    /// Per job: what the new laptop said it has, on this connection; the file is continued from
    /// it only when it is its turn to be in flight, so a reconnect opens no more files than may be
    /// in flight, however many tickets come.
    resume_from: Vec<Option<ResumeTicket>>,
    /// What the move has counted, readable while it runs.
    meter: MoveMeter,
    /// The person's stop and pause.
    control: MoveControl,
    /// The move protocol version both laptops agreed.
    version: u16,
    /// On this connection: whether the new laptop was last told this side is paused.
    told_hold: bool,
    /// On this connection: whether the new laptop's person has paused the move.
    their_hold: bool,
    /// `Hold`s said and not yet answered.
    holds_owed: u32,
}

impl SenderSession {
    /// `jobs` in the order planned; `capacity` files are in flight at once (give at least one per
    /// lane, so every lane has something to send).
    pub fn new(jobs: Vec<SendJob>, capacity: usize) -> Self {
        let mut scheduler = Scheduler::new(capacity);
        let mut by_item = HashMap::new();
        for (i, job) in jobs.iter().enumerate() {
            scheduler.push(job.item, job.tier);
            by_item.insert(job.item, i);
        }
        let meter = MoveMeter::new();
        let states = States::new(&jobs, meter.clone());
        let jobs_len = jobs.len();
        let tiers = jobs.iter().map(|j| j.tier).collect();
        let streams: Vec<u32> = (0..jobs.len())
            .map(|i| u32::try_from(i).unwrap_or(u32::MAX))
            .collect();
        let job_of = streams.iter().enumerate().map(|(i, s)| (*s, i)).collect();
        let next_stream = u32::try_from(jobs.len()).unwrap_or(u32::MAX);
        Self {
            jobs,
            states,
            streams,
            job_of,
            next_stream,
            by_item,
            scheduler,
            capacity,
            in_flight_limit: IN_FLIGHT_BYTES,
            skips: Vec::new(),
            ends: Vec::new(),
            not_sent: Vec::new(),
            not_sent_said: 0,
            opener: Arc::new(FsOpener),
            budget: ReadBudget::for_plan(ReadPlan::Normal),
            later: Vec::new(),
            retried: HashSet::new(),
            no_skip: HashSet::new(),
            lanes: Vec::new(),
            in_flight: 0,
            dirty: false,
            checks: Vec::new(),
            attempted: HashSet::new(),
            asked_again: vec![0; jobs_len],
            ended_tier: tiers,
            attempt_stamp: vec![None; jobs_len],
            on_the_way: HashMap::new(),
            spool: HashMap::new(),
            spooled: 0,
            window: IN_FLIGHT_BYTES,
            careful: false,
            ran: false,
            ended_well: false,
            answered: vec![0; jobs_len],
            versions: vec![0; jobs_len],
            fingerprint: vec![None; jobs_len],
            print_of: HashMap::new(),
            last_end: vec![None; jobs_len],
            resume_from: vec![None; jobs_len],
            meter,
            control: MoveControl::new(),
            version: crate::MOVE_PROTOCOL.newest(),
            told_hold: false,
            their_hold: false,
            holds_owed: 0,
        }
    }

    /// Speaks move protocol `version`, the one both laptops agreed
    /// ([`agree_version`](crate::agree_version)); the newest this release speaks unless set.
    /// Below [`HOLD_SINCE`](crate::HOLD_SINCE) a pause holds this laptop only and is not told.
    pub fn with_protocol(mut self, version: u16) -> Self {
        self.version = version;
        self
    }

    /// Counts in `meter` from now on (the move's counts start from this session's files, all
    /// waiting). Give it before the first run.
    pub fn with_meter(mut self, meter: MoveMeter) -> Self {
        self.states = States::new(&self.jobs, meter.clone());
        self.meter = meter;
        self
    }

    /// Stops and pauses when `control` says so (see [`MoveControl`]).
    pub fn with_control(mut self, control: MoveControl) -> Self {
        self.control = control;
        self
    }

    /// What this move has counted so far (the same meter the caller may hold).
    pub fn move_meter(&self) -> MoveMeter {
        self.meter.clone()
    }

    /// Sends at most `bytes` of blocks (over every lane together) before the new laptop confirms
    /// them (32 MiB unless set).
    pub fn with_in_flight_limit(mut self, bytes: u64) -> Self {
        self.in_flight_limit = bytes.max(1);
        self.window = self.in_flight_limit;
        self
    }

    /// Reads files through `opener` (for example, a snapshot of files in use).
    pub fn with_opener(mut self, opener: Arc<dyn Opener>) -> Self {
        self.opener = opener;
        self
    }

    /// How carefully to read the old drive (from its health check).
    pub fn with_read_plan(mut self, plan: ReadPlan) -> Self {
        self.budget = ReadBudget::for_plan(plan);
        self.careful = matches!(plan, ReadPlan::Careful { .. });
        self
    }

    /// Whether reading stopped because the old drive kept failing.
    pub fn stopped_reading(&self) -> bool {
        self.budget.stopped()
    }

    /// Blocks sent so far, over every connection and lane.
    pub fn blocks_sent(&self) -> u64 {
        self.meter.blocks_sent()
    }

    /// Blocks the new laptop has confirmed so far (a receipt for each), over every connection and
    /// lane. Those sent and not yet confirmed are the most a drop can make send again.
    pub fn blocks_confirmed(&self) -> u64 {
        self.meter.blocks_confirmed()
    }

    /// What this laptop sent, item to source path: the list [`serve_originals`] answers from.
    pub fn sent_sources(&self) -> HashMap<ItemId, PathBuf> {
        self.jobs
            .iter()
            .map(|j| (j.item, j.source.clone()))
            .collect()
    }

    pub fn outcome(&self, item: ItemId) -> Option<&SendOutcome> {
        match &self.states[*self.by_item.get(&item)?] {
            SendState::Done(o) => Some(o),
            _ => None,
        }
    }

    /// The person asked for these to move first, during the move; every lane follows at once.
    pub fn ask_first(&mut self, items: &[ItemId]) {
        self.scheduler.ask_first(items);
    }

    /// Sends everything not yet done over `ch`. Returns when every file is answered for, or with
    /// [`TransferError::ConnectionDropped`] (call again with a new connection to continue).
    pub async fn run<C: Channel>(&mut self, ch: &mut C) -> Result<(), TransferError> {
        self.run_lanes(ch, Vec::<C>::new()).await
    }

    /// Sends everything not yet done over the main connection `main` and the extra `lanes` to
    /// the same new laptop. See [`run_joining`](Self::run_joining).
    pub async fn run_lanes<C: Channel, L: Channel>(
        &mut self,
        main: &mut C,
        lanes: Vec<L>,
    ) -> Result<(), TransferError> {
        self.run_joining(main, &mut inbox_of(lanes)).await
    }

    /// Sends everything not yet done over the main connection `main`, with extra lanes to the same
    /// new laptop joining through `joining` at any time during the move. Starts, ends and skips go
    /// on the main connection; blocks go on every lane, a big file's sections over several at
    /// once, most important files first. A lane that drops gives its unconfirmed blocks back to be
    /// sent on the others; if the main connection drops, call again with a new one to continue.
    /// Lanes end with the move.
    pub async fn run_joining<C: Channel, L: Channel>(
        &mut self,
        main: &mut C,
        joining: &mut UnboundedReceiver<L>,
    ) -> Result<(), TransferError> {
        // A run that did not end well lost what it had in flight: the next starts again from the
        // floor. (Judged at the start of each run, so a run given up by its caller counts too.)
        let after_loss = self.ran && !self.ended_well;
        self.ran = true;
        self.ended_well = false;
        let control = self.control.clone();
        // A stop ends the run at once (before it starts, if it came first), wherever it waits.
        // The run is only ever left at a wait, never partway through a change, so the session
        // stays as a dropped connection leaves it.
        let ran = tokio::select! {
            biased;
            () = control.wait_stopped() => Err(TransferError::Stopped),
            ran = self.run_once(main, joining, after_loss) => ran,
        };
        if matches!(ran, Err(TransferError::Stopped)) && self.version >= STOP_SINCE {
            tell_stopped(main).await;
        }
        self.states.check();
        self.ended_well = ran.is_ok();
        ran
    }

    async fn run_once<C: Channel, L: Channel>(
        &mut self,
        main: &mut C,
        joining: &mut UnboundedReceiver<L>,
        after_loss: bool,
    ) -> Result<(), TransferError> {
        // What the last connection left unconfirmed is kept to send again.
        for lane in 0..self.lanes.len() {
            self.spool_lane(lane);
        }
        self.resume(main).await?;
        self.states.check();
        if after_loss {
            self.window = self.floor();
        }
        self.lanes = vec![VecDeque::new()];
        self.in_flight = 0;
        // What either laptop said about a pause was said on the last connection: on this one
        // this side says its own again, and the new laptop says its own.
        self.told_hold = false;
        self.their_hold = false;
        self.holds_owed = 0;
        self.meter.paused_there(false);
        let control = self.control.clone();
        let state = RefCell::new(self);
        let changed = Notify::new();
        // Extra lanes still running.
        let alive = Cell::new(0usize);
        let main_loop = async {
            loop {
                // Registered before looking, so a change made meanwhile is not missed (by a lane,
                // or a pause, carrying on or stop, even both of a pause and a carrying on).
                let wake = changed.notified();
                let seen = control.changes();
                // The state is never borrowed across a wait, so every lane can take its turn.
                let hold = state.borrow_mut().hold_to_say();
                if let Some(m) = hold {
                    // This side paused or carried on: the new laptop hears it first.
                    send(main, &m).await?;
                    changed.notify_waiters();
                    continue;
                }
                let said = state.borrow_mut().control();
                if !said.is_empty() {
                    for m in &said {
                        send(main, m).await?;
                    }
                    changed.notify_waiters();
                    continue;
                }
                if state.borrow().awaiting_answer_to_start() {
                    // A started file waits for the new laptop's answer before any lane may send
                    // it, so that answer is read first rather than after this lane runs out of
                    // blocks. It is owed, so waiting for it cannot hang.
                    let m = recv(main).await?;
                    state.borrow_mut().answer(0, m)?;
                    state.borrow().states.check();
                    compare_fingerprints(&state).await;
                    changed.notify_waiters();
                    continue;
                }
                let work = state.borrow_mut().work(0);
                if state.borrow_mut().take_dirty() {
                    changed.notify_waiters();
                }
                if let Work::Send { stream, wire } = work {
                    for piece in split_into_pieces(stream, &wire) {
                        send(main, &piece).await?;
                    }
                    continue;
                }
                if state.borrow().has_control() {
                    // An end or skip was queued (a block could not be read): send it first.
                    continue;
                }
                if state.borrow().awaiting_main() {
                    // A reply is owed on this connection, so waiting for it cannot hang.
                    let m = recv(main).await?;
                    state.borrow_mut().answer(0, m)?;
                    state.borrow().states.check();
                    compare_fingerprints(&state).await;
                    changed.notify_waiters();
                } else if state.borrow().all_done() {
                    break;
                } else if state.borrow().held() {
                    // Paused (here or there), and nothing owed on this connection: hold. Waiting
                    // here touches no connection, so this side's carrying on (or stop) is seen at
                    // once; between, the new laptop is told it is still held and answers, so a
                    // link that dropped is found and reported as a drop.
                    let tick = tokio::select! {
                        () = control.wait_change(seen) => false,
                        () = tokio::time::sleep(HOLD_TICK) => true,
                    };
                    let still = if tick {
                        state.borrow_mut().hold_tick()
                    } else {
                        None
                    };
                    if let Some(m) = still {
                        send(main, &m).await?;
                    }
                } else if alive.get() > 0 {
                    // Other lanes are still at work: wait until one of them changes something,
                    // or this side pauses, carries on or stops.
                    tokio::select! {
                        () = wake => {}
                        () = control.wait_change(seen) => {}
                    }
                } else if state.borrow().control.stopped() {
                    // Nothing to send because the move was stopped: that is a stop, never a
                    // move that stopped making progress.
                    return Err(TransferError::Stopped);
                } else {
                    // Nothing is owed and no lane is left to change anything: waiting would
                    // never end, so say so rather than hang.
                    return Err(protocol("the move stopped making progress"));
                }
            }
            // Ending the main loop ends the extra lanes with it.
            send(main, &Message::AllSent).await?;
            Ok(())
        };
        let extra = async {
            let mut running = FuturesUnordered::new();
            let mut open = true;
            loop {
                tokio::select! {
                    joined = joining.recv(), if open => match joined {
                        Some(ch) => {
                            let lane = state.borrow_mut().add_lane();
                            alive.set(alive.get() + 1);
                            running.push(send_lane(&state, &changed, &control, &alive, lane, ch));
                            changed.notify_waiters();
                        }
                        None => open = false,
                    },
                    Some(()) = running.next(), if !running.is_empty() => {}
                    else => break,
                }
            }
        };
        tokio::pin!(main_loop);
        tokio::pin!(extra);
        tokio::select! {
            done = &mut main_loop => done,
            () = &mut extra => main_loop.await,
        }
    }

    /// Reads what the new laptop already has and picks each file up from there.
    async fn resume(&mut self, ch: &mut impl Channel) -> Result<(), TransferError> {
        // What was said on the last connection is said again on this one.
        self.not_sent_said = 0;
        let mut tickets: BTreeMap<usize, ResumeTicket> = BTreeMap::new();
        loop {
            match recv(ch).await? {
                // By the file, never by stream number: either app may have restarted since, and
                // stream numbers start again with each.
                Message::ResumeFrom { item, ticket, .. } => {
                    if let Some(&job) = self.by_item.get(&item) {
                        tickets.insert(job, ticket);
                    }
                }
                Message::FileDone { stream, ok } => {
                    if let Some(&job) = self.job_of.get(&stream) {
                        self.done(job, ok);
                    }
                }
                Message::ItemDone { item, ok } if self.version >= KEYED_SINCE => {
                    if let Some(&job) = self.by_item.get(&item) {
                        self.done(job, ok);
                    }
                }
                Message::Ready => break,
                _ => return Err(protocol("expected the new laptop's resume list")),
            }
        }
        self.scheduler = Scheduler::new(self.capacity);
        self.ends.clear();
        self.skips.clear();
        // Tickets are this connection's word only.
        self.resume_from.iter_mut().for_each(|t| *t = None);
        for job in 0..self.jobs.len() {
            if matches!(self.states[job], SendState::Done(_)) || self.later.contains(&job) {
                // Done, or still waiting for its second try at the end.
                continue;
            }
            match &self.states[job] {
                SendState::Ended {
                    failure: Some(why), ..
                } => {
                    // An end stays an end: a file that failed here, or was given up on, is never
                    // opened again; what was said is said again until the new laptop answers.
                    match self.last_end[job].clone() {
                        Some(end) => self.ends.push(end),
                        None => {
                            *self.states.slot(job) =
                                SendState::Done(SendOutcome::Failed(why.clone()))
                        }
                    }
                    continue;
                }
                SendState::Skipping => {
                    // Its skip was never answered: started again, whatever the ticket says (the
                    // fingerprint is kept, so comparing again reads nothing).
                    *self.states.slot(job) = SendState::Waiting;
                    self.scheduler
                        .push(self.jobs[job].item, self.jobs[job].tier);
                    continue;
                }
                _ => {}
            }
            // Opened (from its ticket, if it has one) only when it is its turn: see `open`.
            self.resume_from[job] = tickets.remove(&job);
            *self.states.slot(job) = SendState::Waiting;
            self.scheduler
                .push(self.jobs[job].item, self.jobs[job].tier);
        }
        Ok(())
    }

    /// Messages for the main connection: skips, ends, and starts of the files now in flight.
    fn control(&mut self) -> Vec<Message> {
        let mut out = Vec::new();
        if self.version >= STOP_SINCE {
            for (item, cause) in &self.not_sent[self.not_sent_said..] {
                if let Some(cause) = cause.said() {
                    out.push(Message::NotSent { item: *item, cause });
                }
            }
        }
        self.not_sent_said = self.not_sent.len();
        for job in std::mem::take(&mut self.skips) {
            let stream = self.streams[job];
            out.push(match self.print_of.get(&job) {
                Some(print) if self.version >= KEYED_SINCE => Message::Print {
                    stream,
                    print: *print,
                },
                _ => Message::Skip { stream },
            });
            self.scheduler.finished(self.jobs[job].item);
            // Already there only once the new laptop says so (see `done`).
            *self.states.slot(job) = SendState::Skipping;
        }
        out.append(&mut self.ends);
        // A file ends once every block is confirmed, so the new laptop has them all first.
        for job in 0..self.states.len() {
            let complete = matches!(
                &self.states[job],
                SendState::Open { sections, cleared: true, .. } if sections.is_done()
            );
            if complete && let SendState::Open { reader, .. } = self.states.take(job) {
                let end = self.end(job, reader);
                self.last_end[job] = Some(end.clone());
                out.push(end);
            }
        }
        // Everything readable is done: one more try for what could not be read.
        let idle = !self.states.iter().any(|s| {
            matches!(
                s,
                SendState::Open { .. } | SendState::Ended { .. } | SendState::Skipping
            )
        }) && self.in_flight == 0;
        if idle && !self.later.is_empty() && !self.budget.stopped() {
            for job in std::mem::take(&mut self.later) {
                self.retried.insert(job);
                self.scheduler
                    .push(self.jobs[job].item, self.jobs[job].tier);
            }
        }
        if self.held() {
            // Held: nothing new is opened or started until the move carries on.
            return out;
        }
        // Files that fail to open free their place at once, so keep filling until those in
        // flight are open.
        let mut in_flight = self.scheduler.in_flight();
        loop {
            let mut freed = false;
            for item in &in_flight {
                let job = self.by_item[item];
                if matches!(self.states[job], SendState::Waiting) {
                    self.open(job);
                    freed |= !matches!(self.states[job], SendState::Open { .. });
                }
            }
            if !freed {
                break;
            }
            in_flight = self.scheduler.in_flight();
        }
        for item in in_flight {
            let job = self.by_item[&item];
            if let SendState::Open {
                reader,
                announced,
                resumed_done,
                ..
            } = &mut *self.states.slot(job)
                && !*announced
            {
                *announced = true;
                let j = &self.jobs[job];
                out.push(Message::StartFile {
                    stream: self.streams[job],
                    item: j.item,
                    destination: j.destination.clone(),
                    path: j.path.clone(),
                    header: reader.header().clone(),
                    resumed_done: *resumed_done,
                });
            }
        }
        out
    }

    /// Opens a waiting file now that it is in flight: continued from the new laptop's ticket on
    /// its own stream if it has one that fits the file as it is, else for a new attempt, with a
    /// new stream.
    fn open(&mut self, job: usize) {
        if let Some(ticket) = self.resume_from[job].take() {
            let j = &self.jobs[job];
            let picked_up =
                FileSender::open_with(&*self.opener, &j.source, Some(&ticket), j.compressible)
                    .ok()
                    .and_then(|reader| {
                        let sections = sections_for(reader.header(), &ticket.done.to_bools())?;
                        Some((reader, sections))
                    });
            if let Some((reader, sections)) = picked_up {
                // Continued on its stream; any later fresh start is a new attempt on a new one,
                // so nothing of this attempt still on its way can land in it.
                self.attempted.insert(job);
                self.read_version(job, reader.header().stamp);
                *self.states.slot(job) = SendState::Open {
                    reader,
                    sections,
                    announced: false,
                    resumed_done: ticket.done.done_count(),
                    cleared: false,
                };
                return;
            }
            // The file changed since, or the ticket does not fit: it starts again from the
            // beginning.
        }
        let j = &self.jobs[job];
        let opened = FileSender::open_with(&*self.opener, &j.source, None, j.compressible)
            .and_then(|reader| {
                let sections = sections_for(reader.header(), &[])
                    .ok_or_else(|| TransferError::Damaged("the file is too big to send".into()))?;
                Ok((reader, sections))
            });
        match opened {
            Ok((reader, sections)) => {
                self.new_stream(job);
                // Past the versions any file is read in, it is still started (so the new laptop
                // can say it kept changing) but never read again: see `answer`.
                self.read_version(job, reader.header().stamp);
                // A new attempt: the new laptop counts its rounds from 0 again, and so does this.
                self.asked_again[job] = 0;
                *self.states.slot(job) = SendState::Open {
                    reader,
                    sections,
                    announced: false,
                    resumed_done: 0,
                    cleared: false,
                };
            }
            Err(e) => {
                let drive = matches!(&e, TransferError::Io(io) if is_drive_error(io));
                self.scheduler.finished(j.item);
                if drive && self.read_failed(job) {
                    // One more try after everything else.
                    self.later.push(job);
                    *self.states.slot(job) = SendState::Waiting;
                } else {
                    self.fail_unannounced(job, Failure::of(&e, Side::Old));
                }
            }
        }
    }

    /// Gives `job` a new stream when it is opened again, so nothing of an earlier attempt can
    /// land in this one.
    fn new_stream(&mut self, job: usize) {
        if !self.attempted.insert(job) {
            self.job_of.remove(&self.streams[job]);
            let stream = self.next_stream;
            self.next_stream = self.next_stream.saturating_add(1);
            self.streams[job] = stream;
            self.job_of.insert(stream, job);
        }
    }

    /// `job` is now read at `stamp`. A stamp unlike the one before is a new version: what the new
    /// laptop was answered starts again from 0, and the version is counted.
    fn read_version(&mut self, job: usize, stamp: crate::Stamp) {
        if self.attempt_stamp[job] != Some(stamp) {
            self.attempt_stamp[job] = Some(stamp);
            self.answered[job] = 0;
            self.versions[job] = self.versions[job].saturating_add(1);
        }
    }

    /// Whether `job` is at a version past those any file is read in.
    fn too_many_versions(&self, job: usize) -> bool {
        self.versions[job] > crate::MAX_ATTEMPTS
    }

    /// The window after a loss: 1 MiB, or the block of the largest file in flight if that is
    /// bigger (as this laptop last read it); one block when the drive is read carefully. (One
    /// block always goes: see `work`.)
    fn floor(&mut self) -> u64 {
        let in_flight = self.scheduler.in_flight();
        let block = in_flight
            .iter()
            .filter_map(|item| self.attempt_stamp[self.by_item[item]])
            .map(|s| crate::block_size_for(s.size))
            .max()
            .unwrap_or(crate::MIN_BLOCK);
        if self.careful {
            block
        } else {
            block.max(FLOOR_BYTES)
        }
    }

    /// Queues `end` (an end or a stop of `job`) for the main connection, and keeps it to say again
    /// on a new connection until it is answered.
    fn push_end(&mut self, job: usize, end: Message) {
        self.last_end[job] = Some(end.clone());
        self.ends.push(end);
    }

    /// Every block of `job` is confirmed: its end, after checking it did not change while read.
    fn end(&mut self, job: usize, reader: FileSender) -> Message {
        let stream = self.streams[job];
        let stamp = reader.header().stamp;
        let item = self.jobs[job].item;
        self.ended_tier[job] = self.scheduler.tier_of(item).unwrap_or(self.jobs[job].tier);
        self.scheduler.finished(item);
        match reader.finish() {
            Ok(trailer) => {
                // On a weak drive each file is read only once: a file that changed is not read again.
                let changed = trailer.changed_while_read();
                let again = changed && self.budget.try_again_later();
                *self.states.slot(job) = SendState::Ended {
                    retry: again,
                    failure: (changed && !again).then(|| {
                        Failure::new(Cause::ChangedDuringMove, "it changed while being read")
                    }),
                    later: false,
                };
                Message::EndFile {
                    stream,
                    stamp_after: trailer.stamp_after,
                    changed,
                }
            }
            Err(e) => {
                *self.states.slot(job) = SendState::Ended {
                    retry: false,
                    failure: Some(Failure::of(&e, Side::Old)),
                    later: false,
                };
                Message::EndFile {
                    stream,
                    stamp_after: stamp,
                    changed: true,
                }
            }
        }
    }

    /// The next block for `lane`: from the most important file in flight that has one for it
    /// (a section this lane holds, a part no lane holds, or the back half of a big section).
    /// While less than the window is in flight (raw block bytes, so compression cannot hide what
    /// is read), one more block goes.
    fn work(&mut self, lane: usize) -> Work {
        if self.control.stopped()
            || self.held()
            || self.budget.stopped()
            || self.in_flight >= self.in_flight_limit.min(self.window)
        {
            return Work::Wait;
        }
        let lane_no = u32::try_from(lane).unwrap_or(u32::MAX);
        for item in self.scheduler.in_flight() {
            let job = self.by_item[&item];
            let stream = self.streams[job];
            let (b, read) = {
                let mut open = self.states.slot(job);
                let SendState::Open {
                    reader,
                    sections,
                    cleared: true,
                    ..
                } = &mut *open
                else {
                    continue;
                };
                let next = sections.next_block(lane_no).or_else(|| {
                    sections.claim(lane_no)?;
                    sections.next_block(lane_no)
                });
                let Some(b) = next else {
                    continue;
                };
                // Cut off by a drop and kept, from this very version of the file: sent as it
                // was, without reading the drive again.
                let kept = self
                    .spool
                    .remove(&(job, b))
                    .filter(|(at, _)| Some(*at) == self.attempt_stamp[job]);
                match kept {
                    Some((_, wire)) => {
                        self.spooled = self.spooled.saturating_sub(wire.len() as u64);
                        (b, Err(wire))
                    }
                    None => (b, Ok(reader.block_at(u64::from(b)))),
                }
            };
            let read = match read {
                Ok(read) => read.map(|block| (block.data.len() as u64, block.encode(), true)),
                Err(wire) => Ok((u64::from(Block::length_in(&wire).unwrap_or(0)), wire, false)),
            };
            match read {
                Ok((raw, wire, from_drive)) => {
                    if from_drive {
                        self.budget.read_ok();
                    }
                    self.on_the_way.insert((stream, b), wire.clone());
                    self.lanes[lane].push_back((stream, b, raw));
                    self.in_flight += raw;
                    self.meter.block_sent(raw);
                    return Work::Send { stream, wire };
                }
                Err(e) => self.read_error(job, e),
            }
        }
        Work::Wait
    }

    /// `job` failed here before the new laptop heard of it: done, and the new laptop is told why
    /// (protocol 4 and later), so its report gives the true reason.
    fn fail_unannounced(&mut self, job: usize, failure: Failure) {
        self.not_sent.push((self.jobs[job].item, failure.cause()));
        self.dirty = true;
        *self.states.slot(job) = SendState::Done(SendOutcome::Failed(failure));
    }

    /// `job` stops here, for `why` (`failure` is what this laptop says): the new laptop is told if
    /// it holds the file, and nothing more of it is read.
    fn give_up(&mut self, job: usize, why: GiveUpWhy, failure: Failure) {
        self.dirty = true;
        self.scheduler.finished(self.jobs[job].item);
        self.checks.retain(|(j, _)| *j != job);
        self.skips.retain(|j| *j != job);
        let told = matches!(
            self.states[job],
            SendState::Open {
                announced: true,
                ..
            } | SendState::Ended { .. }
        );
        if told {
            self.push_end(
                job,
                Message::GiveUp {
                    stream: self.streams[job],
                    why,
                },
            );
            *self.states.slot(job) = SendState::Ended {
                retry: false,
                failure: Some(failure),
                later: false,
            };
        } else {
            self.fail_unannounced(job, failure);
        }
    }

    /// The new laptop was answered for `job` more than any move needs: it stops here, and the new
    /// laptop is told why.
    fn read_too_often(&mut self, job: usize) {
        self.give_up(
            job,
            GiveUpWhy::ReadTooOften,
            Failure::new(Cause::NotVerified, READ_TOO_OFTEN),
        );
    }

    /// A receipt for `raw` bytes of a block of `stream`, `written` or not. Always in this order:
    /// what the new laptop was answered is counted for the file's version, then checked against
    /// the cap, then (if written) the window grows by it. A receipt for an earlier attempt frees
    /// room only.
    fn receipted(&mut self, stream: u32, raw: u64, written: bool) {
        let Some(&job) = self.job_of.get(&stream) else {
            return;
        };
        self.answered[job] = self.answered[job].saturating_add(raw);
        let size = self.attempt_stamp[job].map_or(0, |s| s.size);
        if matches!(self.states[job], SendState::Open { .. })
            && self.answered[job] > answered_cap(size)
        {
            self.read_too_often(job);
            return;
        }
        if written {
            self.window = self.window.saturating_add(raw).min(self.in_flight_limit);
        }
    }

    /// A block of `job` could not be read: tell the new laptop to drop what it has of it.
    fn read_error(&mut self, job: usize, e: TransferError) {
        self.dirty = true;
        let SendState::Open { reader, .. } = &self.states[job] else {
            return;
        };
        let stamp = reader.header().stamp;
        let drive = matches!(&e, TransferError::Io(io) if is_drive_error(io));
        let retry = matches!(e, TransferError::ChangedWhileRead) && self.budget.try_again_later();
        self.push_end(
            job,
            Message::EndFile {
                stream: self.streams[job],
                stamp_after: stamp,
                changed: true,
            },
        );
        self.scheduler.finished(self.jobs[job].item);
        // Ended first, so stopping reading (if this was one error too many) leaves it be.
        *self.states.slot(job) = SendState::Ended {
            retry,
            failure: None,
            later: false,
        };
        let later = drive && self.read_failed(job);
        *self.states.slot(job) = SendState::Ended {
            retry,
            failure: (!retry && !later).then(|| Failure::of(&e, Side::Old)),
            later,
        };
    }

    /// Whether a file was started on this connection and the new laptop has not answered yet.
    fn awaiting_answer_to_start(&self) -> bool {
        self.states.iter().any(|s| {
            matches!(
                s,
                SendState::Open {
                    announced: true,
                    cleared: false,
                    ..
                }
            )
        })
    }

    /// Whether ends or skips are waiting to go on the main connection.
    fn has_control(&self) -> bool {
        !self.ends.is_empty() || !self.skips.is_empty()
    }

    /// Whether the other laptop understands `Hold`.
    fn speaks_hold(&self) -> bool {
        self.version >= HOLD_SINCE
    }

    /// Whether the move is held: this side's person paused it, or the new laptop's did.
    fn held(&self) -> bool {
        self.control.paused() || self.their_hold
    }

    /// This side's pause, to tell the new laptop if it changed since it was last told on this
    /// connection. With a laptop that cannot be told, this side holds itself, untold.
    fn hold_to_say(&mut self) -> Option<Message> {
        let paused = self.control.paused();
        if !self.speaks_hold() {
            self.meter.paused_here(paused);
            return None;
        }
        (paused != self.told_hold).then(|| self.say_hold(paused))
    }

    /// Still held: says this side's pause again, to be answered.
    fn hold_tick(&mut self) -> Option<Message> {
        let paused = self.control.paused();
        self.speaks_hold().then(|| self.say_hold(paused))
    }

    fn say_hold(&mut self, paused: bool) -> Message {
        self.told_hold = paused;
        self.holds_owed = self.holds_owed.saturating_add(1);
        self.meter.paused_here(paused);
        Message::Hold { paused }
    }

    /// Fingerprint comparisons still to work out: (job, the version, its file, the new laptop's
    /// fingerprint). One already worked out for that version is answered at once, reading
    /// nothing.
    fn take_checks(&mut self) -> Vec<(usize, crate::Stamp, PathBuf, [u8; 32])> {
        let mut out = Vec::new();
        for (job, theirs) in std::mem::take(&mut self.checks) {
            let Some(stamp) = self.attempt_stamp[job] else {
                self.checked(job, false);
                continue;
            };
            match self.fingerprint[job] {
                Some((at, key, mine)) if at == stamp && key == self.key_of(theirs) => {
                    self.judged(job, mine, theirs);
                }
                _ => out.push((job, stamp, self.jobs[job].source.clone(), theirs)),
            }
        }
        out
    }

    /// `job`'s fingerprint at `stamp` was worked out (`None`: it could not be): kept for that
    /// version, then compared.
    fn printed(
        &mut self,
        job: usize,
        stamp: crate::Stamp,
        mine: Option<[u8; 32]>,
        theirs: [u8; 32],
    ) {
        self.fingerprint[job] = Some((stamp, self.key_of(theirs), mine));
        if self.attempt_stamp[job] == Some(stamp) {
            self.judged(job, mine, theirs);
        } else {
            self.checked(job, false);
        }
    }

    /// The key to fingerprint with: from move protocol 4 the new laptop's `Have` holds a key,
    /// before it the new laptop's own fingerprint (compared as it is).
    fn key_of(&self, theirs: [u8; 32]) -> Option<[u8; 32]> {
        (self.version >= KEYED_SINCE).then_some(theirs)
    }

    /// `job`'s fingerprint (`mine`) against the new laptop's `Have`: before move protocol 4 the
    /// two are compared here; from 4 an identical file is the new laptop's to judge, so a file
    /// that could be read is offered with its print (`Print`), and one that could not is sent.
    fn judged(&mut self, job: usize, mine: Option<[u8; 32]>, theirs: [u8; 32]) {
        if self.version >= KEYED_SINCE {
            match mine {
                Some(print) => {
                    self.print_of.insert(job, print);
                    self.checked(job, true);
                }
                None => self.checked(job, false),
            }
        } else {
            self.checked(job, mine == Some(theirs));
        }
    }

    /// The comparison for `job` is done: an identical file is skipped, anything else is sent.
    fn checked(&mut self, job: usize, identical: bool) {
        if let SendState::Open { cleared, .. } = &mut *self.states.slot(job) {
            if identical && !self.no_skip.contains(&job) {
                self.skips.push(job);
            } else {
                *cleared = true;
            }
        }
    }

    /// A lane joined: its number.
    fn add_lane(&mut self) -> usize {
        self.lanes.push(VecDeque::new());
        self.lanes.len() - 1
    }

    fn take_dirty(&mut self) -> bool {
        std::mem::take(&mut self.dirty)
    }

    /// Whether a reply is owed on the main connection: an answer to a start, an end or a `Hold`,
    /// or a receipt for a block it carried.
    fn awaiting_main(&self) -> bool {
        self.holds_owed > 0
            || !self.lanes[0].is_empty()
            || self.states.iter().any(|s| {
                matches!(
                    s,
                    SendState::Open {
                        announced: true,
                        cleared: false,
                        ..
                    } | SendState::Ended { .. }
                        | SendState::Skipping
                )
            })
    }

    /// Every file answered for, nothing in flight, and nothing left to try again.
    fn all_done(&self) -> bool {
        self.in_flight == 0
            && self.ends.is_empty()
            && self.skips.is_empty()
            && self.states.iter().all(|s| matches!(s, SendState::Done(_)))
    }

    /// Handles a message from the new laptop on `lane`. Extra lanes carry only receipts.
    fn answer(&mut self, lane: usize, m: Message) -> Result<(), TransferError> {
        match m {
            Message::Refused { .. } => {
                let (stream, _, bytes) = self.lanes[lane]
                    .pop_front()
                    .ok_or_else(|| protocol("a refusal for nothing sent"))?;
                self.in_flight -= bytes;
                // The new laptop could not write a block of the current attempt, so the file
                // will not finish: stop sending it now and end it, rather than send the rest for
                // nothing.
                if let Some(&job) = self.job_of.get(&stream)
                    && let SendState::Open { reader, .. } = &self.states[job]
                {
                    let stamp_after = reader.header().stamp;
                    self.push_end(
                        job,
                        Message::EndFile {
                            stream,
                            stamp_after,
                            changed: true,
                        },
                    );
                    self.scheduler.finished(self.jobs[job].item);
                    *self.states.slot(job) = SendState::Ended {
                        retry: false,
                        failure: Some(Failure::new(
                            Cause::CouldNotWrite,
                            "the new laptop could not write part of it",
                        )),
                        later: false,
                    };
                    self.dirty = true;
                }
                Ok(())
            }
            Message::Receipt { written, .. } => {
                // Which block it answers, and its size, are this laptop's own record.
                let (stream, block, raw) = self.lanes[lane]
                    .pop_front()
                    .ok_or_else(|| protocol("a receipt for nothing sent"))?;
                self.on_the_way.remove(&(stream, block));
                self.in_flight -= raw;
                self.meter.block_confirmed(raw);
                if let Some(&job) = self.job_of.get(&stream)
                    && matches!(self.states[job], SendState::Open { .. })
                {
                    // A big file counts down on this laptop too, as the new laptop confirms it.
                    self.meter.progress(self.states.group(job), raw);
                }
                // Only the current attempt counts; a receipt for an earlier one frees room only.
                if let Some(&job) = self.job_of.get(&stream)
                    && let SendState::Open { sections, .. } = &mut *self.states.slot(job)
                {
                    let lane_no = u32::try_from(lane).unwrap_or(u32::MAX);
                    // In order per lane, as sent: which block a receipt answers is this laptop's
                    // own record, and an attempt only starts again once all of its blocks are
                    // answered. A mismatch would leave the file waiting for ever on both laptops,
                    // so it ends this lane (its blocks go back to the others) or, on the main
                    // connection, the connection (the move carries on from where it was).
                    if let Err(e) = sections.confirmed(lane_no, block) {
                        return Err(protocol(&format!(
                            "a receipt did not match the block sent on its lane ({e:?})"
                        )));
                    }
                }
                self.receipted(stream, raw, written);
                Ok(())
            }
            Message::FileDone { stream, ok } if lane == 0 => {
                if let Some(&job) = self.job_of.get(&stream) {
                    self.done(job, ok);
                }
                Ok(())
            }
            Message::SendAgain { stream, ticket } if lane == 0 => {
                if let Some(&job) = self.job_of.get(&stream) {
                    self.send_again(job, &ticket);
                }
                Ok(())
            }
            Message::Have { stream, same_size } if lane == 0 => {
                let Some(&job) = self.job_of.get(&stream) else {
                    return Ok(());
                };
                // Answered once per start: an answer not owed now is ignored (it could only make
                // this laptop read the file again for nothing).
                let owed = matches!(
                    self.states[job],
                    SendState::Open {
                        announced: true,
                        cleared: false,
                        ..
                    }
                ) && !self.checks.iter().any(|(j, _)| *j == job)
                    && !self.skips.contains(&job);
                if !owed {
                    return Ok(());
                }
                if self.too_many_versions(job) {
                    // Started only so the new laptop can say it kept changing: never read.
                    self.give_up(
                        job,
                        GiveUpWhy::ReadTooOften,
                        Failure::new(Cause::ChangedDuringMove, KEPT_CHANGING),
                    );
                    return Ok(());
                }
                match same_size {
                    // A same-size file is there: compare fingerprints, off the main loop (see
                    // `take_checks`).
                    Some(theirs) => self.checks.push((job, theirs)),
                    None => self.checked(job, false),
                }
                Ok(())
            }
            Message::Hold { paused } if lane == 0 && self.speaks_hold() => {
                // An answer to a `Hold` said, or the new laptop's own news: either way it is the
                // new laptop's pause as it stands.
                self.holds_owed = self.holds_owed.saturating_sub(1);
                self.their_hold = paused;
                self.meter.paused_there(paused);
                self.dirty = true;
                Ok(())
            }
            // The new laptop's person stopped the move: a stop, never a lost connection.
            Message::Stopping if lane == 0 && self.version >= STOP_SINCE => {
                Err(TransferError::StoppedThere)
            }
            _ => Err(protocol("unexpected message from the new laptop")),
        }
    }

    /// The new laptop asked, at the end of `job`, for the blocks `ticket` does not list. They are
    /// read again from the file, opened afresh read-only, only if it has not changed since it was
    /// read (else it ends as changed, and is sent again whole or reported, as any file that
    /// changed while being read). Ignored for a file not just ended.
    fn send_again(&mut self, job: usize, ticket: &ResumeTicket) {
        if !matches!(
            self.states[job],
            SendState::Ended {
                failure: None,
                later: false,
                retry: false,
            }
        ) {
            return;
        }
        self.dirty = true;
        // An honest new laptop's ticket carries exactly the file this laptop described (its
        // stamp, block size and count); one that does not is a new laptop that misbehaves, and the
        // file stops here (so it can never force a new attempt).
        let fits = self.attempt_stamp[job].is_some_and(|s| {
            let block_size = crate::block_size_for(s.size);
            s == ticket.stamp
                && ticket.block_size == block_size
                && ticket.done.count() == s.size.div_ceil(block_size)
        });
        if !fits {
            self.give_up(
                job,
                GiveUpWhy::AskedAgainTooOften,
                Failure::new(Cause::NotVerified, TICKET_WRONG),
            );
            return;
        }
        if self.asked_again[job] >= SEND_AGAIN_ROUNDS {
            // Asked more often in this attempt than an honest new laptop asks: this attempt stops
            // here and the file fails, said as it is (never as a file that changed).
            self.give_up(
                job,
                GiveUpWhy::AskedAgainTooOften,
                Failure::new(Cause::NotVerified, GAVE_UP),
            );
            return;
        }
        self.asked_again[job] += 1;
        let j = &self.jobs[job];
        let reopened =
            FileSender::open_with(&*self.opener, &j.source, Some(ticket), j.compressible).and_then(
                |reader| {
                    let sections = sections_for(reader.header(), &ticket.done.to_bools())
                        .ok_or_else(|| {
                            TransferError::Damaged("the file is too big to send".into())
                        })?;
                    Ok((reader, sections))
                },
            );
        match reopened {
            Ok((reader, sections)) => {
                self.scheduler.push_again(j.item, self.ended_tier[job]);
                *self.states.slot(job) = SendState::Open {
                    reader,
                    sections,
                    // The new laptop keeps the file open: no new start.
                    announced: true,
                    resumed_done: ticket.done.done_count(),
                    cleared: true,
                };
            }
            Err(e) => {
                let retry =
                    matches!(e, TransferError::ChangedSince) && self.budget.try_again_later();
                self.push_end(
                    job,
                    Message::EndFile {
                        stream: self.streams[job],
                        stamp_after: ticket.stamp,
                        changed: true,
                    },
                );
                *self.states.slot(job) = SendState::Ended {
                    retry,
                    failure: (!retry).then(|| Failure::of(&e, Side::Old)),
                    later: false,
                };
            }
        }
    }

    /// `lane` dropped: what it sent and was not confirmed goes back to be sent on the others, and
    /// the window starts again from the floor (a loss, as a dropped main connection is).
    fn lane_lost(&mut self, lane: usize) {
        let lane_no = u32::try_from(lane).unwrap_or(u32::MAX);
        self.spool_lane(lane);
        for (_, _, raw) in self.lanes[lane].drain(..) {
            self.in_flight -= raw;
        }
        self.window = self.floor();
        for job in 0..self.states.len() {
            if let SendState::Open { sections, .. } = &mut *self.states.slot(job) {
                sections.lane_lost(lane_no);
            }
        }
    }

    /// Keeps `lane`'s unconfirmed blocks to send again without reading the drive again, up to
    /// the in-flight limit in all (beyond it they are simply read again).
    fn spool_lane(&mut self, lane: usize) {
        for &(stream, b, _) in &self.lanes[lane] {
            let Some(wire) = self.on_the_way.remove(&(stream, b)) else {
                continue;
            };
            let Some(&job) = self.job_of.get(&stream) else {
                continue;
            };
            let Some(stamp) = self.attempt_stamp[job] else {
                continue;
            };
            let len = wire.len() as u64;
            if self.spooled.saturating_add(len) > self.in_flight_limit {
                continue;
            }
            self.spooled += len;
            if let Some((_, old)) = self.spool.insert((job, b), (stamp, wire)) {
                self.spooled = self.spooled.saturating_sub(old.len() as u64);
            }
        }
    }

    /// Counts a drive read error for `job`. Returns true when the file gets one more try after
    /// everything else; when errors come in a row, stops reading altogether.
    fn read_failed(&mut self, job: usize) -> bool {
        if self.budget.read_failed() {
            self.stop_reading();
            return false;
        }
        self.budget.try_again_later() && !self.retried.contains(&job)
    }

    /// The drive keeps failing: read nothing more. Files not yet read are reported as such, and
    /// files half-sent are dropped on the new laptop.
    fn stop_reading(&mut self) {
        self.dirty = true;
        self.scheduler = Scheduler::new(self.capacity);
        self.later.clear();
        for job in 0..self.states.len() {
            let mut state = self.states.slot(job);
            match &mut *state {
                SendState::Waiting => {
                    self.not_sent.push((self.jobs[job].item, Cause::Unreadable));
                    *state = SendState::Done(SendOutcome::Failed(Failure::new(
                        Cause::Unreadable,
                        STOPPED,
                    )))
                }
                SendState::Open {
                    announced: true,
                    reader,
                    ..
                } => {
                    let end = Message::EndFile {
                        stream: self.streams[job],
                        stamp_after: reader.header().stamp,
                        changed: true,
                    };
                    self.last_end[job] = Some(end.clone());
                    self.ends.push(end);
                    *state = SendState::Ended {
                        retry: false,
                        failure: Some(Failure::new(Cause::Unreadable, STOPPED)),
                        later: false,
                    };
                }
                SendState::Open { .. } => {
                    self.not_sent.push((self.jobs[job].item, Cause::Unreadable));
                    *state = SendState::Done(SendOutcome::Failed(Failure::new(
                        Cause::Unreadable,
                        STOPPED,
                    )));
                }
                _ => {}
            }
        }
    }

    fn done(&mut self, job: usize, ok: bool) {
        let item = self.jobs[job].item;
        let tier = self.jobs[job].tier;
        let state = self.states.take(job);
        *self.states.slot(job) = match (state, ok) {
            (SendState::Ended { later: true, .. }, _) => {
                // It could not be read: one more try after everything else.
                self.later.push(job);
                SendState::Waiting
            }
            // Already there only on the new laptop's word.
            (SendState::Skipping, true) => SendState::Done(SendOutcome::AlreadyThere),
            // Not accepted as already there (the new laptop could not use the copy it has): sent
            // after all, once.
            (SendState::Skipping, false)
                if self.version >= COPY_SINCE && self.no_skip.insert(job) =>
            {
                self.scheduler.push(item, tier);
                SendState::Waiting
            }
            (SendState::Skipping, false) => SendState::Done(SendOutcome::Failed(Failure::new(
                Cause::NotAccepted,
                "the new laptop did not accept it",
            ))),
            // Started past the versions any file is read in, and refused: it kept changing.
            (SendState::Open { .. }, false) if self.versions[job] > crate::MAX_ATTEMPTS => {
                self.scheduler.finished(item);
                SendState::Done(SendOutcome::Failed(Failure::new(
                    Cause::ChangedDuringMove,
                    KEPT_CHANGING,
                )))
            }
            (SendState::Ended { failure: None, .. }, true) => SendState::Done(SendOutcome::Arrived),
            (SendState::Ended { retry: true, .. }, false) => {
                // It changed while being read: send the whole file again.
                self.scheduler.push(item, tier);
                SendState::Waiting
            }
            (
                SendState::Ended {
                    failure: Some(why), ..
                },
                _,
            ) => SendState::Done(SendOutcome::Failed(why)),
            (SendState::Done(o), _) => SendState::Done(o),
            (_, true) => SendState::Done(SendOutcome::Arrived),
            (_, false) => {
                self.scheduler.finished(item);
                SendState::Done(SendOutcome::Failed(Failure::new(
                    Cause::NotAccepted,
                    "the new laptop did not accept it",
                )))
            }
        };
    }
}

/// A job's fingerprint as worked out: the version of the file, the new laptop's key it was worked
/// out with (move protocol 4), and the fingerprint (`None`: it could not be).
type Printed = (crate::Stamp, Option<[u8; 32]>, Option<[u8; 32]>);

/// One extra lane of the old laptop: asks the shared plan for blocks and sends them, reads
/// receipts only when they are owed, and otherwise sleeps until another lane changes something.
/// If it drops or is answered out of turn, its unconfirmed blocks go back to the others.
async fn send_lane<C: Channel>(
    state: &RefCell<&mut SenderSession>,
    changed: &Notify,
    control: &MoveControl,
    alive: &Cell<usize>,
    lane: usize,
    mut ch: C,
) {
    loop {
        // Both made before looking: a change by another lane, or a pause, carrying on or stop
        // made after this look, wakes this lane however it was told (or not told) to anyone.
        let wake = changed.notified();
        let seen = control.changes();
        let work = state.borrow_mut().work(lane);
        if state.borrow_mut().take_dirty() {
            changed.notify_waiters();
        }
        let lane_ok = match work {
            Work::Send { stream, wire } => {
                let mut sent = true;
                for piece in split_into_pieces(stream, &wire) {
                    if send(&mut ch, &piece).await.is_err() {
                        sent = false;
                        break;
                    }
                }
                sent
            }
            Work::Wait if !state.borrow().lanes[lane].is_empty() => {
                // Receipts are owed on this lane. If none comes for LANE_SILENCE the other end is
                // stuck (not just slow), so the lane is let go and its blocks go to the others.
                match tokio::time::timeout(LANE_SILENCE, recv(&mut ch))
                    .await
                    .unwrap_or(Err(TransferError::ConnectionDropped))
                {
                    Ok(m) => {
                        let answered = state.borrow_mut().answer(lane, m).is_ok();
                        state.borrow().states.check();
                        changed.notify_waiters();
                        answered
                    }
                    Err(_) => false,
                }
            }
            Work::Wait => {
                tokio::select! {
                    () = wake => {}
                    () = control.wait_change(seen) => {}
                }
                true
            }
        };
        if !lane_ok {
            state.borrow_mut().lane_lost(lane);
            alive.set(alive.get() - 1);
            changed.notify_waiters();
            return;
        }
    }
}

/// One extra lane of the new laptop: rejoins its own pieces, writes each whole block and answers
/// it on this lane. Closed if it drops or sends anything but pieces.
async fn receive_lane<C: Channel>(state: &RefCell<&mut ReceiverSession<'_>>, mut ch: C) {
    let mut slot: Slot = None;
    loop {
        let Ok(Message::Piece {
            stream,
            last,
            bytes,
        }) = recv(&mut ch).await
        else {
            // Dropped, or not a piece: this lane is closed.
            return;
        };
        let Ok(receipt) = state.borrow_mut().piece(&mut slot, stream, last, &bytes) else {
            // Pieces of two blocks mixed: this lane is closed.
            return;
        };
        if let Some(r) = receipt
            && send(&mut ch, &r).await.is_err()
        {
            return;
        }
    }
}

/// Lanes given all at once, as an inbox that then closes.
fn inbox_of<C>(lanes: Vec<C>) -> UnboundedReceiver<C> {
    let (tx, rx) = tokio::sync::mpsc::unbounded_channel();
    for lane in lanes {
        // The receiving end is held here, so nothing is lost.
        let _ = tx.send(lane);
    }
    rx
}

/// The section plan for a file, with blocks already done marked; `None` for a file with more
/// blocks than a plan can count (far past any real file).
fn sections_for(header: &crate::Header, done: &[bool]) -> Option<FileSections> {
    let blocks = u32::try_from(header.block_count).ok()?;
    Some(FileSections::new(blocks, header.block_size, done))
}

/// Compares fingerprints for same-size files on the blocking-thread pool, so hashing a big file
/// never stalls the lanes; each answer is settled before the main loop goes on.
async fn compare_fingerprints(state: &RefCell<&mut SenderSession>) {
    let checks = state.borrow_mut().take_checks();
    for (job, stamp, source, theirs) in checks {
        let opener = Arc::clone(&state.borrow().opener);
        let key = state.borrow().key_of(theirs);
        let mine =
            tokio::task::spawn_blocking(move || fingerprint_at(&*opener, &source, stamp, key))
                .await
                .ok()
                .flatten();
        state.borrow_mut().printed(job, stamp, mine, theirs);
    }
}

/// Runs blocking file work on the blocking-thread pool; `None` if it failed.
async fn off_the_loop(
    work: impl FnOnce() -> std::io::Result<[u8; 32]> + Send + 'static,
) -> Option<[u8; 32]> {
    tokio::task::spawn_blocking(work).await.ok()?.ok()
}

/// The BLAKE3 fingerprint of the file at `path` as it was when read at `stamp`, read through
/// `opener` (the same reader its blocks come through): exactly `stamp.size` bytes, then a look
/// for one more, its stamp checked before and after. `None` (so "not identical") if it is longer
/// or shorter, its stamp moved, or it could not be read: never more than `stamp.size + 1` bytes
/// are read.
fn fingerprint_at(
    opener: &dyn Opener,
    path: &Path,
    stamp: crate::Stamp,
    key: Option<[u8; 32]>,
) -> Option<[u8; 32]> {
    let mut source = opener.open(path).ok()?;
    if source.stamp().ok()? != stamp {
        return None;
    }
    let mut hasher = hasher_for(key);
    let read = std::io::copy(&mut Read::take(&mut source, stamp.size), &mut hasher).ok()?;
    if read != stamp.size || source.read(&mut [0u8; 1]).ok()? != 0 {
        return None;
    }
    if source.stamp().ok()? != stamp {
        return None;
    }
    Some(*hasher.finalize().as_bytes())
}

/// BLAKE3, keyed with the new laptop's key from move protocol 4, plain before it.
fn hasher_for(key: Option<[u8; 32]>) -> blake3::Hasher {
    match key {
        Some(key) => blake3::Hasher::new_keyed(&key),
        None => blake3::Hasher::new(),
    }
}

fn hash_reader(
    reader: &mut impl std::io::Read,
    key: Option<[u8; 32]>,
) -> std::io::Result<[u8; 32]> {
    let mut hasher = hasher_for(key);
    let mut buf = vec![0u8; 1024 * 1024];
    loop {
        let n = reader.read(&mut buf)?;
        if n == 0 {
            break;
        }
        hasher.update(&buf[..n]);
    }
    Ok(*hasher.finalize().as_bytes())
}

/// How a received file ended.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ReceiveOutcome {
    Finished(Finished),
    /// Copied and checked, and kept, but not finished as a visible, proven copy: filled in under
    /// its name and not yet shown (shown when PCTwin starts again), or a file PCTwin could not
    /// prove its own was found at its name and left as it is. Never reported as copied; `why`
    /// says, in plain words, what happened and what comes next.
    NeedsAction {
        finished: Finished,
        why: String,
    },
    /// An identical file was already there (its stored path); nothing was copied.
    AlreadyThere(String),
    Failed(Failure),
}

struct Incoming<'d> {
    item: ItemId,
    /// Its entry in the change journal.
    entry: u64,
    /// Blocks written and not yet checkpointed in the journal, and batches checkpointed so far.
    pending: Vec<(u64, [u8; 32])>,
    batches: u32,
    assembly: Option<Assembly<'d>>,
    block_size: u64,
    failure: Option<Failure>,
    destination: String,
    size: u64,
    /// The stored path of a same-size file already there, offered as possibly identical.
    same: Option<String>,
    /// Or an identical file this laptop already received in this move, to make this one from.
    copy_of: Option<CopyOf>,
    /// This laptop's own fingerprint of that file, once worked out.
    own_print: Option<[u8; 32]>,
    /// Blocks that arrived damaged (their own fingerprint did not match), over its attempt.
    damaged: u32,
    /// Times its blocks were asked for again (see [`SEND_AGAIN_ROUNDS`]).
    rounds: u8,
}

/// A file this laptop already received in this move, offered as possibly identical to a file
/// the old laptop started (copies cross once): where it is, and its fingerprint once worked out.
struct CopyOf {
    destination: String,
    stored: String,
    fingerprint: Option<[u8; 32]>,
}

/// A copy to make here, from a file already received, in place of sending it.
struct LocalCopy {
    stream: u32,
    file: Option<std::fs::File>,
    header: crate::Header,
    expected: [u8; 32],
    /// The key `expected` was worked out with (move protocol 4), or none.
    key: Option<[u8; 32]>,
}

/// The smallest file made from a copy already here: smaller ones cost more to compare than to
/// send.
const COPY_MIN: u64 = 1024 * 1024;
/// The most received files remembered as possible copies (each an ID in a list by size).
const MAX_COPY_SOURCES: usize = 100_000;
/// The most remembered files of one size looked at for a start.
const COPY_LOOKS: usize = 4;

/// Whose things a destination holds, for copies: a file is only ever made from one in the same
/// person's things, never across people. The signed-in person's folders are theirs together;
/// another account's place is that person's; a shared folder or a drive holds only its own.
#[derive(Debug, PartialEq, Eq)]
enum Whose<'a> {
    Mine,
    Account(&'a str),
    Only(&'a str),
}

fn whose<'a>(table: &'a Destinations, label: &'a str) -> Option<Whose<'a>> {
    Some(match table.place(label).ok()? {
        Approved::MyFolders => Whose::Mine,
        Approved::AnotherAccount { account_id } => Whose::Account(account_id),
        Approved::SharedFolder | Approved::ChosenDrive | Approved::OffloadDrive => {
            Whose::Only(label)
        }
    })
}

/// Makes the skipped file of `copy` here from the identical file already received: each block
/// read on the blocking-thread pool and written through the same checks as a block that arrived,
/// the whole compared with the fingerprint offered, then ended as a sent file ends. The answer to
/// send: done, or not accepted (the old laptop then sends it after all).
async fn copy_here(
    state: &RefCell<&mut ReceiverSession<'_>>,
    copy: LocalCopy,
) -> Result<Message, TransferError> {
    let LocalCopy {
        stream,
        mut file,
        header,
        expected,
        key,
    } = copy;
    let mut hasher = hasher_for(key);
    let mut whole = file.is_some();
    for index in 0..header.block_count {
        let Some(mut f) = file.take() else {
            whole = false;
            break;
        };
        let len = crate::block_len(&header, index);
        let read = tokio::task::spawn_blocking(move || {
            let mut data = vec![0u8; len];
            let read = f.read_exact(&mut data);
            (f, read.map(|()| data))
        })
        .await;
        let Ok((f, Ok(data))) = read else {
            whole = false;
            break;
        };
        file = Some(f);
        hasher.update(&data);
        if !state
            .borrow_mut()
            .copy_block(stream, Block::plain(index, data))
        {
            whole = false;
            break;
        }
    }
    // Exactly as long as it was, and as a whole what the old laptop has.
    whole = whole
        && file.is_some_and(|mut f| matches!(f.read(&mut [0u8; 1]), Ok(0)))
        && *hasher.finalize().as_bytes() == expected;
    state.borrow_mut().copied_here(stream, whole, header.stamp)
}

/// The new laptop's side of a move. A file starts only if the approved plan allows it (see
/// [`Allowance`]); every file lands through the safety gate in one of the approved destinations,
/// and every step of every write is recorded in the change journal first (planned before anything
/// is created, staged, verified, its name applied before it gets it, committed; or failed with
/// why, or existing for an identical file already there). If the journal cannot be written the
/// move stops ([`TransferError::Record`]): nothing is written that the journal does not know.
/// Keeps what it has across dropped connections: call [`run`](Self::run) again with a new
/// connection to continue.
pub struct ReceiverSession<'d> {
    table: &'d Destinations,
    journal: &'d dyn Ledger,
    /// The signed-in person's account on this laptop (who acts).
    account: String,
    allowance: Allowance,
    /// Bytes of blocks written so far, readable while the move runs (for the lane driver).
    written: Arc<AtomicU64>,
    /// What the move has counted, readable while it runs, and each planned file's group and
    /// step in it.
    meter: MoveMeter,
    steps: HashMap<ItemId, (Group, Step)>,
    /// The person's stop and pause.
    control: MoveControl,
    /// The move protocol version both laptops agreed.
    version: u16,
    /// On this connection: whether the old laptop was last told this side is paused.
    told_hold: bool,
    /// Why a file's start was refused, kept per file (bounded).
    refused: BTreeMap<ItemId, Failure>,
    /// Planned files whose latest start differed from the scan (size or changed time).
    changed: HashSet<ItemId>,
    /// A same-size file already here, to fingerprint before answering a start (off the loop),
    /// with the key to fingerprint it with (move protocol 4).
    to_fingerprint: Option<(u32, std::fs::File, Option<[u8; 32]>)>,
    /// This session's own secret, from which each file's key for comparing is made (move
    /// protocol 4): never sent, so a print the old laptop sends means nothing outside this move.
    same_secret: [u8; 32],
    /// A skipped start to make here from a copy already here (off the loop).
    to_copy: Option<u32>,
    /// Per approved place, whether each of its last checked files was stored correctly (newest
    /// last, at most [`BRAKE_WINDOW`]).
    stored_lately: HashMap<String, VecDeque<bool>>,
    /// Files received in this move that later ones may be made from, by size.
    copies: HashMap<u64, Vec<ItemId>>,
    copy_sources: usize,
    streams: BTreeMap<u32, Incoming<'d>>,
    /// Files picked up again after the app restarted, waiting for the old laptop to continue them.
    restored: BTreeMap<ItemId, Incoming<'d>>,
    /// How each file's last attempt ended, kept per file (never per stream), so nothing started
    /// later can rewrite the line of a file that landed.
    ended: BTreeMap<ItemId, Ended>,
    continued: u64,
}

/// How one file's last attempt ended.
struct Ended {
    /// The stream it ended on, to tell the old laptop again on the next connection.
    stream: u32,
    outcome: ReceiveOutcome,
    /// For a file that landed: its destination and size, to check again after copying.
    landed: Option<(String, u64)>,
}

impl Ended {
    /// It is on this laptop (copied, or already there): its line is final.
    fn spent(&self) -> bool {
        matches!(
            self.outcome,
            ReceiveOutcome::Finished(_)
                | ReceiveOutcome::NeedsAction { .. }
                | ReceiveOutcome::AlreadyThere(_)
        )
    }
}

impl<'d> ReceiverSession<'d> {
    /// Receives into the places in `table`, only what `allowance` (the approved plan) allows,
    /// recording every write in `journal`; `account` is the signed-in person's account on this
    /// laptop.
    pub fn new(
        table: &'d Destinations,
        allowance: Allowance,
        journal: &'d dyn Ledger,
        account: &str,
    ) -> Self {
        let meter = MoveMeter::new();
        let steps = plan_steps(&allowance, &meter);
        Self {
            table,
            journal,
            account: account.to_string(),
            allowance,
            written: Arc::new(AtomicU64::new(0)),
            meter,
            steps,
            control: MoveControl::new(),
            version: crate::MOVE_PROTOCOL.newest(),
            told_hold: false,
            refused: BTreeMap::new(),
            changed: HashSet::new(),
            to_fingerprint: None,
            same_secret: {
                let mut secret = [0u8; 32];
                // Without random numbers nothing is compared: every key is all zeros then, and
                // still never matched outside this laptop's own judgement.
                let _ = getrandom::fill(&mut secret);
                secret
            },
            to_copy: None,
            stored_lately: HashMap::new(),
            copies: HashMap::new(),
            copy_sources: 0,
            streams: BTreeMap::new(),
            restored: BTreeMap::new(),
            ended: BTreeMap::new(),
            continued: 0,
        }
    }

    /// A counter of bytes of blocks written, to read while the move runs (the lane driver's speed).
    pub fn meter(&self) -> Arc<AtomicU64> {
        self.written.clone()
    }

    /// Counts in `meter` from now on: every file of the approved plan, waiting, in the group of
    /// its destination. Give it before the first run.
    pub fn with_meter(mut self, meter: MoveMeter) -> Self {
        self.steps = plan_steps(&self.allowance, &meter);
        self.meter = meter;
        self
    }

    /// Stops and pauses when `control` says so (see [`MoveControl`]). A pause here is told to
    /// the old laptop with this side's next reply, and the old laptop holds; blocks already on
    /// their way are still written.
    pub fn with_control(mut self, control: MoveControl) -> Self {
        self.control = control;
        self
    }

    /// Speaks move protocol `version`, the one both laptops agreed
    /// ([`agree_version`](crate::agree_version)); the newest this release speaks unless set.
    /// Below [`HOLD_SINCE`](crate::HOLD_SINCE) the old laptop cannot be told of a pause here, so
    /// a pause here holds nothing and is not shown as held.
    pub fn with_protocol(mut self, version: u16) -> Self {
        self.version = version;
        self
    }

    /// What this move has counted so far (the same meter the caller may hold).
    pub fn move_meter(&self) -> MoveMeter {
        self.meter.clone()
    }

    /// This side's pause, to tell the old laptop with the next reply if it changed since it was
    /// last told on this connection.
    fn hold_to_say(&mut self) -> Option<Message> {
        let paused = self.control.paused();
        (self.version >= HOLD_SINCE && paused != self.told_hold).then(|| self.say_hold(paused))
    }

    fn say_hold(&mut self, paused: bool) -> Message {
        self.told_hold = paused;
        self.meter.paused_here(paused);
        Message::Hold { paused }
    }

    /// `item` (a file of the plan) is now at `step`.
    fn step(&mut self, item: ItemId, step: Step) {
        if let Some((group, at)) = self.steps.get_mut(&item) {
            self.meter.step(*group, *at, step);
            *at = step;
        }
    }

    /// The step `item` is at, from this session's state alone: copying while an attempt at it is
    /// open, else what its line says, else waiting. (Checking is only ever inside a landing.)
    fn step_now(&self, item: ItemId) -> Step {
        if let Some(s) = self.streams.values().find(|s| s.item == item) {
            // An attempt that failed here stays until the old laptop ends it: it is failed now
            // (the journal already says so), not copying.
            return if s.failure.is_some() || s.assembly.is_none() {
                Step::Failed
            } else {
                Step::Copying
            };
        }
        self.outcome(item)
            .map_or(Step::Waiting, |o| outcome_step(&o))
    }

    /// An attempt let go because another start took its place: its partial file is removed with
    /// it (already failed if it had failed), and its file is counted as it now stands.
    fn started_again(&mut self, old: Incoming<'d>) {
        self.end_attempt(&old);
        drop(old.assembly);
        let _ = self.journal.failed(old.entry, "it was started again");
        self.settle(old.item);
    }

    /// An attempt that ends unfinished: one that received nothing was no attempt (see
    /// [`Allowance::ended_unstarted`]).
    fn end_attempt(&mut self, old: &Incoming<'d>) {
        let nothing = old.assembly.as_ref().is_some_and(|a| a.blocks_done() == 0);
        if nothing && old.failure.is_none() {
            self.allowance.ended_unstarted(old.item);
        } else {
            self.allowance.ended(old.item, false);
        }
    }

    /// Counts `item` at the step its state says, after anything that may have changed it.
    fn settle(&mut self, item: ItemId) {
        let step = self.step_now(item);
        self.step(item, step);
    }

    /// Counts every planned file at the step its state says (when a run ends partway).
    fn settle_all(&mut self) {
        let items: Vec<ItemId> = self.steps.keys().copied().collect();
        for item in items {
            self.settle(item);
        }
    }

    /// In tests: every planned file is counted at the step its state says. (Plans of tests are
    /// small; a real move's, past [`CHECKED_PLAN`] files, is not looked through, so a debug build
    /// of the app stays quick.)
    fn check_steps(&self) {
        if cfg!(debug_assertions) && self.steps.len() <= CHECKED_PLAN {
            for (item, (_, at)) in &self.steps {
                debug_assert_eq!(
                    *at,
                    self.step_now(*item),
                    "a received file is counted at the wrong step"
                );
            }
        }
    }

    /// Files picked up partway after a dropped connection, rather than started again.
    pub fn continued(&self) -> u64 {
        self.continued
    }

    /// Files held partway (some blocks in, not finished), to continue on the next connection.
    pub fn partway(&self) -> u64 {
        self.streams
            .values()
            .chain(self.restored.values())
            .filter(|s| {
                s.failure.is_none() && s.assembly.as_ref().is_some_and(|a| a.blocks_done() > 0)
            })
            .count() as u64
    }

    /// After the app restarts (and [`recover`](crate::recover) ran): picks up again every file the
    /// journal has partly received from this plan's old laptop, so the old laptop continues each
    /// from what is really here. A file no longer in the approved plan is ended as failed and its
    /// partial file removed. Returns how many were picked up.
    pub fn restore(&mut self) -> Result<u64, TransferError> {
        let mut picked = 0;
        for e in self.journal.unfinished().map_err(record)? {
            // No more open at once than any move may hold; the rest stay as they are in the
            // journal, for a later start.
            if self.streams.len() + self.restored.len() >= MAX_OPEN_FILES {
                break;
            }
            let State::Staged { temp } = &e.state else {
                continue;
            };
            let w = &e.write;
            if w.source_laptop != self.allowance.source_laptop()
                || self.restored.contains_key(&w.item)
                || self.streams.values().any(|s| s.item == w.item)
            {
                continue;
            }
            let table: &'d Destinations = self.table;
            // A place not reachable now is left for later.
            let Ok(dest) = table.get(&w.destination) else {
                continue;
            };
            let here = dest.folder_identity("").ok().flatten().map(file_id);
            if w.place.is_some() && here != w.place {
                continue;
            }
            // Only what the approved plan says, where it says: the journal's word is checked too.
            if let Err(refusal) = self
                .allowance
                .admit(w.item, &w.destination, &w.path, w.size)
            {
                self.journal
                    .failed(e.id, &refusal.to_string())
                    .map_err(record)?;
                let _ = dest.remove_temp(temp);
                continue;
            }
            let header = crate::Header {
                size: w.size,
                block_size: w.block_size,
                block_count: w.size.div_ceil(w.block_size.max(1)),
                stamp: crate::Stamp {
                    size: w.size,
                    modified_ns: w.source_modified_ns,
                },
                source_file: w.source_file,
                mark: w.download_mark,
            };
            let claimed = self.journal.blocks(e.id).map_err(record)?;
            let resumed = IncomingPath::parse(&w.path)
                .map_err(|e| e.to_string())
                .and_then(|path| {
                    Assembly::resume(dest, &path, temp, header, &claimed).map_err(|e| e.to_string())
                });
            match resumed {
                Ok(assembly) => {
                    self.restored.insert(
                        w.item,
                        Incoming {
                            item: w.item,
                            entry: e.id,
                            pending: Vec::new(),
                            batches: 0,
                            block_size: w.block_size,
                            assembly: Some(assembly),
                            failure: None,
                            destination: w.destination.clone(),
                            size: w.size,
                            same: None,
                            copy_of: None,
                            own_print: None,
                            damaged: 0,
                            rounds: 0,
                        },
                    );
                    picked += 1;
                }
                Err(why) => {
                    self.allowance.ended(w.item, false);
                    self.journal.failed(e.id, &why).map_err(record)?;
                    let _ = dest.remove_temp(temp);
                }
            }
        }
        Ok(picked)
    }

    /// Records in the journal the blocks written since the last checkpoint, for every file being
    /// received (done by itself every few dozen blocks, and at each new connection). Best effort:
    /// a checkpoint that does not get through only means those blocks are sent again.
    /// Cancels the move on this side: every file not finished is recorded failed with `why` and
    /// its partly received file removed. (Simply dropping the session instead keeps them, for the
    /// move to continue after the app starts again.)
    pub fn cancel(mut self, why: &str) {
        self.checkpoint();
        let journal = self.journal;
        let mut ended = Vec::new();
        for s in self.streams.values_mut().chain(self.restored.values_mut()) {
            if let Some(assembly) = s.assembly.take() {
                drop(assembly);
                let _ = journal.failed(s.entry, why);
            }
            ended.push(s.item);
        }
        // The meter outlives the session: every attempt ends here, so each of its files is
        // failed there too, whatever was left of it.
        for item in ended {
            if !self.ended.get(&item).is_some_and(Ended::spent) {
                self.step(item, Step::Failed);
            }
        }
    }

    pub fn checkpoint(&mut self) {
        let journal = self.journal;
        for s in self.streams.values_mut().chain(self.restored.values_mut()) {
            flush_checkpoint(journal, s, true);
        }
    }

    /// How `item` ended: a file on this laptop keeps that line whatever came after it; otherwise
    /// the latest of its refusal or its last attempt's end.
    pub fn outcome(&self, item: ItemId) -> Option<ReceiveOutcome> {
        let ended = self.ended.get(&item);
        if let Some(e) = ended.filter(|e| e.spent()) {
            return Some(e.outcome.clone());
        }
        // A refusal is kept only while it is newer than the file's last end (see `end`).
        self.refused
            .get(&item)
            .map(|why| ReceiveOutcome::Failed(why.clone()))
            .or_else(|| ended.map(|e| e.outcome.clone()))
    }

    /// How many refusals are kept (one at most per planned file), for tests.
    #[doc(hidden)]
    pub fn refusals_kept(&self) -> usize {
        self.refused.len()
    }

    /// Whether `item` was sent as it is now, different from the scan (its size or its changed
    /// time). Judged at its latest start; for the report's "changed since the scan".
    pub fn changed_since_scan(&self, item: ItemId) -> bool {
        self.changed.contains(&item)
    }

    /// Receives over `ch` until the old laptop has sent everything, or the connection drops (call
    /// again with a new connection to continue).
    pub async fn run<C: Channel>(&mut self, ch: &mut C) -> Result<(), TransferError> {
        self.run_lanes(ch, Vec::<C>::new()).await
    }

    /// Receives over the main connection `main` and the extra `lanes` to the same old laptop.
    /// See [`run_joining`](Self::run_joining).
    pub async fn run_lanes<C: Channel, L: Channel>(
        &mut self,
        main: &mut C,
        lanes: Vec<L>,
    ) -> Result<(), TransferError> {
        self.run_joining(main, &mut inbox_of(lanes)).await
    }

    /// Receives over the main connection `main`, with extra lanes from the same old laptop
    /// joining through `joining` at any time during the move. Files are started, ended and
    /// skipped only on the main connection; extra lanes carry only blocks (as pieces) and their
    /// receipts, and each lane rejoins its own pieces. A lane that drops or sends anything but
    /// pieces is closed and the move carries on over the others; if the main connection drops,
    /// call again with a new one to continue. Lanes end with the move.
    pub async fn run_joining<C: Channel, L: Channel>(
        &mut self,
        main: &mut C,
        joining: &mut UnboundedReceiver<L>,
    ) -> Result<(), TransferError> {
        let control = self.control.clone();
        // A stop ends the run at once (before it starts, if it came first), wherever it waits.
        // The run is only ever left at a wait, never partway through handling a message, so the
        // session stays as a dropped connection leaves it: drop it to keep partly received files,
        // or `cancel` it.
        let ran = tokio::select! {
            biased;
            () = control.wait_stopped() => Err(TransferError::Stopped),
            ran = self.receive_joining(main, joining) => ran,
        };
        if matches!(ran, Err(TransferError::Stopped)) && self.version >= STOP_SINCE {
            tell_stopped(main).await;
        }
        ran
    }

    async fn receive_joining<C: Channel, L: Channel>(
        &mut self,
        main: &mut C,
        joining: &mut UnboundedReceiver<L>,
    ) -> Result<(), TransferError> {
        // What either laptop said about a pause was said on the last connection.
        self.told_hold = false;
        self.meter.paused_here(false);
        self.meter.paused_there(false);
        let state = RefCell::new(self);
        // Say what is already here, so the old laptop continues from there.
        let hello = state.borrow_mut().hello();
        state.borrow().check_steps();
        for m in &hello {
            send(main, m).await?;
        }
        let main_loop = async {
            // A block cut off by a drop is sent again whole: pieces start afresh each run.
            let mut slot: Slot = None;
            loop {
                let m = recv(main).await?;
                // Never held across a wait, so every lane can take its turn.
                let handled = state.borrow_mut().handle(m, &mut slot);
                if handled.is_err() {
                    // The run ends partway through a change: count every file as it is left.
                    state.borrow_mut().settle_all();
                }
                state.borrow().check_steps();
                let (mut replies, finished) = handled?;
                // A pause or carrying on here goes with the reply (the old laptop reads its
                // replies, so it hears it at once while it sends, and on its next word while it
                // is held).
                let hold = state.borrow_mut().hold_to_say();
                replies.extend(hold);
                for r in &replies {
                    send(main, r).await?;
                }
                // A start waiting on a same-size file's fingerprint: worked out on the
                // blocking-thread pool, so the lanes keep going meanwhile.
                let waiting = state.borrow_mut().to_fingerprint.take();
                if let Some((stream, mut file, key)) = waiting {
                    let own = off_the_loop(move || hash_reader(&mut file, key)).await;
                    state.borrow_mut().offered(stream, own);
                    // From move protocol 4 the key is sent, never this laptop's fingerprint.
                    let same_size = match key {
                        Some(key) => own.map(|_| key),
                        None => own,
                    };
                    send(main, &Message::Have { stream, same_size }).await?;
                }
                // A skip of a file made from a copy already here: copied block by block through
                // the same checks as a sent file, each block read on the blocking-thread pool.
                let copying = state.borrow_mut().take_copy();
                if let Some(copy) = copying {
                    let done = copy_here(&state, copy).await?;
                    send(main, &done).await?;
                }
                if finished {
                    return Ok(());
                }
            }
        };
        let extra = async {
            let mut running = FuturesUnordered::new();
            let mut open = true;
            loop {
                tokio::select! {
                    joined = joining.recv(), if open => match joined {
                        Some(ch) => running.push(receive_lane(&state, ch)),
                        None => open = false,
                    },
                    Some(()) = running.next(), if !running.is_empty() => {}
                    else => break,
                }
            }
        };
        tokio::pin!(main_loop);
        tokio::pin!(extra);
        tokio::select! {
            done = &mut main_loop => done,
            // Every extra lane closed and no more can join: carry on over the main connection.
            () = &mut extra => main_loop.await,
        }
    }

    /// What this side already has, then `Ready`.
    fn hello(&mut self) -> Vec<Message> {
        self.checkpoint();
        let dropped: Vec<u32> = self
            .streams
            .iter()
            .filter(|(_, s)| s.assembly.is_none() || s.failure.is_some())
            .map(|(k, _)| *k)
            .collect();
        for k in dropped {
            if let Some(old) = self.streams.remove(&k) {
                self.end_attempt(&old);
                // Counted as its line says, or waiting for the old laptop to start it again.
                self.settle(old.item);
            }
        }
        let mut list = Vec::new();
        for (stream, s) in &self.streams {
            if let Some(a) = &s.assembly {
                list.push(Message::ResumeFrom {
                    stream: *stream,
                    item: s.item,
                    ticket: a.resume_ticket(),
                });
            }
        }
        for (item, s) in &self.restored {
            if let Some(a) = &s.assembly {
                list.push(Message::ResumeFrom {
                    stream: 0,
                    item: *item,
                    ticket: a.resume_ticket(),
                });
            }
        }
        for (item, e) in &self.ended {
            // By the file from move protocol 4: a stream number may have been reused since.
            list.push(if self.version >= KEYED_SINCE {
                Message::ItemDone {
                    item: *item,
                    ok: e.spent(),
                }
            } else {
                Message::FileDone {
                    stream: e.stream,
                    ok: e.spent(),
                }
            });
        }
        list.push(Message::Ready);
        list
    }

    /// Handles one message from the main connection: the replies to send, and whether the old
    /// laptop has sent everything.
    fn handle(
        &mut self,
        m: Message,
        slot: &mut Slot,
    ) -> Result<(Vec<Message>, bool), TransferError> {
        let mut replies = Vec::new();
        match m {
            Message::StartFile {
                stream,
                item,
                destination,
                path,
                header,
                resumed_done,
            } => {
                // Only the approved plan says where a file lands: a start anywhere else is
                // refused before anything here is touched (nothing created, reserved or ended).
                if let Err(refusal) = self.allowance.check_place(item, &destination, &path) {
                    self.refuse(item, Failure::from(refusal));
                    replies.push(Message::FileDone { stream, ok: false });
                    return Ok((replies, false));
                }
                // Sent as it is now; its line says when that is not as it was at the scan. Judged
                // by its latest start, the one that lands.
                if self.allowance.changed_since_scan(item, header.stamp) {
                    self.changed.insert(item);
                } else {
                    self.changed.remove(&item);
                }
                // The copy is marked at least as strictly as the plan requires.
                let mut header = header;
                header.mark = self.allowance.mark_for(header.mark);
                // The old laptop continues from this side's ticket, which never claims more
                // than is here (it may carry only the earliest runs).
                let continuing = resumed_done > 0
                    && self.streams.get(&stream).is_some_and(|s| {
                        s.item == item
                            && s.assembly
                                .as_ref()
                                .is_some_and(|a| resumed_done <= a.blocks_done())
                    });
                // A file picked up again after the app restarted, continued by the old laptop with
                // exactly the description it started with (else it starts again).
                let restored = !continuing
                    && resumed_done > 0
                    && self.restored.get(&item).is_some_and(|s| {
                        s.assembly.as_ref().is_some_and(|a| {
                            resumed_done <= a.blocks_done() && a.header().same_file(&header)
                        })
                    });
                if restored && let Some(mut s) = self.restored.remove(&item) {
                    // Whichever said so, the copy keeps the stricter marker.
                    if let Some(a) = s.assembly.as_mut() {
                        a.keep_stricter_mark(header.mark);
                    }
                    if let Some(old) = self.streams.remove(&stream) {
                        self.started_again(old);
                    }
                    self.streams.insert(stream, s);
                }
                if continuing || restored {
                    self.settle(item);
                    self.continued += 1;
                    replies.push(Message::Have {
                        stream,
                        same_size: None,
                    });
                    return Ok((replies, false));
                }
                // A fresh start replaces anything kept on this stream, and any attempt at the same
                // file still open on another (one attempt at a file at a time). Partial files are
                // removed.
                let replaced: Vec<u32> = self
                    .streams
                    .iter()
                    .filter(|(k, s)| **k == stream || s.item == item)
                    .map(|(k, _)| *k)
                    .collect();
                let olds: Vec<Incoming<'d>> = replaced
                    .into_iter()
                    .filter_map(|k| self.streams.remove(&k))
                    .chain(self.restored.remove(&item))
                    .collect();
                for old in olds {
                    self.started_again(old);
                }
                let block_size = header.block_size;
                let size = header.size;
                // The plan is checked before anything is read, created or reserved here.
                let admitted = if resumed_done > 0 {
                    Err(Failure::new(
                        Cause::RefusedBySafetyCheck,
                        "the new laptop has no place to continue from",
                    ))
                } else if self.streams.len() + self.restored.len() >= MAX_OPEN_FILES {
                    Err(Failure::new(
                        Cause::RefusedBySafetyCheck,
                        "the old laptop started too many files at once",
                    ))
                } else {
                    self.allowance
                        .admit(item, &destination, &path, size)
                        .map_err(Failure::from)
                };
                if let Err(why) = admitted {
                    // Kept per file, not per start, so refusals cannot pile up.
                    self.refuse(item, why);
                    replies.push(Message::FileDone { stream, ok: false });
                    return Ok((replies, false));
                }
                // Someone else's laptop is never told what is here.
                let same = if self.allowance.offers_what_is_here() {
                    self.same_file(&destination, &path, size)
                } else {
                    None
                };
                let stored = same.as_ref().map(|(stored, _)| stored.clone());
                // Nothing like it at its place: an identical file this laptop already received in
                // this move, in the same person's things, may be made into this one here.
                let (copy_of, copy_file) = match &same {
                    None if self.version >= COPY_SINCE
                        && size >= COPY_MIN
                        && self.allowance.offers_what_is_here() =>
                    {
                        match self.copy_source(&destination, size) {
                            Some((c, f)) => (Some(c), Some(f)),
                            None => (None, None),
                        }
                    }
                    _ => (None, None),
                };
                let (assembly, entry) = match self.start(item, &destination, &path, header) {
                    Ok(started) => started,
                    Err(Start::Record(e)) => return Err(record(e)),
                    Err(Start::Refused(why)) => {
                        self.allowance.ended(item, false);
                        self.end(item, stream, ReceiveOutcome::Failed(why), None);
                        replies.push(Message::FileDone { stream, ok: false });
                        return Ok((replies, false));
                    }
                };
                // Every start is answered: nothing like it is here, or (once its fingerprint is
                // worked out off the main loop) the fingerprint of the same-size file that is.
                match same.map(|(_, f)| f).or(copy_file) {
                    Some(file) => {
                        self.to_fingerprint = Some((stream, file, self.same_key_for(item)));
                    }
                    None => replies.push(Message::Have {
                        stream,
                        same_size: None,
                    }),
                }
                self.streams.insert(
                    stream,
                    Incoming {
                        item,
                        entry,
                        pending: Vec::new(),
                        batches: 0,
                        assembly: Some(assembly),
                        block_size,
                        failure: None,
                        destination,
                        size,
                        same: stored,
                        copy_of,
                        own_print: None,
                        damaged: 0,
                        rounds: 0,
                    },
                );
                // A fresh attempt: nothing of an earlier one counts as written.
                if let Some((group, _)) = self.steps.get(&item) {
                    self.meter.attempt(*group);
                }
                self.settle(item);
            }
            Message::Piece {
                stream,
                last,
                bytes,
            } => {
                if let Some(r) = self.piece(slot, stream, last, &bytes)? {
                    replies.push(r);
                }
            }
            Message::EndFile {
                stream,
                stamp_after,
                changed,
            } => {
                let trailer = Trailer::new(stamp_after, changed);
                if let Some(again) = self.send_again(stream, &trailer) {
                    replies.push(again);
                    return Ok((replies, false));
                }
                replies.push(self.finish_stream(stream, trailer)?);
            }
            // From move protocol 4 this laptop judges: the old laptop's print against its own.
            Message::Print { stream, print } if self.version >= KEYED_SINCE => {
                let same = self
                    .streams
                    .get(&stream)
                    .is_some_and(|s| s.own_print == Some(print));
                if same {
                    return self.skipped(stream, replies);
                }
                replies.push(self.not_identical(stream));
            }
            Message::Skip { stream } if self.version >= KEYED_SINCE => {
                // Only a print is taken as "identical" now: a bare skip is never believed.
                replies.push(self.not_identical(stream));
            }
            Message::Skip { stream } => return self.skipped(stream, replies),
            Message::GiveUp { stream, why } => {
                // The old laptop stopped sending this file: it fails, with the true reason.
                let why = match why {
                    GiveUpWhy::AskedAgainTooOften => GAVE_UP,
                    GiveUpWhy::ReadTooOften => READ_TOO_OFTEN,
                };
                if let Some(s) = self.streams.remove(&stream) {
                    drop(s.assembly);
                    let _ = self.journal.failed(s.entry, why);
                    self.allowance.ended(s.item, false);
                    self.end(
                        s.item,
                        stream,
                        ReceiveOutcome::Failed(Failure::new(Cause::NotVerified, why)),
                        None,
                    );
                }
                replies.push(Message::FileDone { stream, ok: false });
            }
            Message::AllSent => return Ok((replies, true)),
            Message::Hold { paused } if self.version >= HOLD_SINCE => {
                // The old laptop's pause as it stands; every `Hold` is answered with this side's.
                self.meter.paused_there(paused);
                let here = self.control.paused();
                replies.push(self.say_hold(here));
            }
            // The old laptop's person stopped the move: a stop, never a lost connection.
            Message::Stopping if self.version >= STOP_SINCE => {
                return Err(TransferError::StoppedThere);
            }
            // A planned file the old laptop will not send, with why: its line says so. Kept only
            // for a file of the plan that is not open here and has not ended here.
            Message::NotSent { item, cause } if self.version >= STOP_SINCE => {
                let failure = Cause::heard(cause)
                    .ok_or_else(|| protocol("an unknown reason a file was not sent"))?;
                let open = self.streams.values().any(|s| s.item == item)
                    || self.restored.contains_key(&item);
                if self.allowance.destination(item).is_some()
                    && !open
                    && !self.ended.contains_key(&item)
                {
                    self.refuse(item, failure);
                }
            }
            _ => return Err(protocol("unexpected message from the old laptop")),
        }
        Ok((replies, false))
    }

    /// Adds a piece that came on one connection (with that connection's own `slot`); when it ends
    /// a block, checks and writes the block and returns the receipt naming it. Pieces are kept
    /// only for a file that is open, never past its block size, and only one block at a time per
    /// connection: an honest old laptop sends each block's pieces together, so pieces of two
    /// blocks mixed are refused (the connection ends) rather than held. Every block's last piece
    /// is answered, even for a file that is not open, so the old laptop's count stays right.
    /// Ends the file on `stream` as its last block came in: read back, checked and named through
    /// the gate (or failed, with why). The answer to send.
    fn finish_stream(&mut self, stream: u32, trailer: Trailer) -> Result<Message, TransferError> {
        let Some(s) = self.streams.remove(&stream) else {
            return Ok(Message::FileDone { stream, ok: false });
        };
        let outcome = match (s.assembly, s.failure) {
            (Some(mut a), None) => {
                let dest = self
                    .table
                    .get(&s.destination)
                    .map_err(|e| protocol(&e.to_string()))?;
                // Read back and checked now, before it gets its name.
                self.step(s.item, Step::Checking);
                self.meter.checking(Some(s.item));
                // Out of rounds and still not right though it never changed on the old laptop:
                // what is said depends on whose fault it is (Quarantine Design, step 3).
                let unchanged = !trailer.changed && trailer.stamp_after == a.header().stamp;
                let incomplete = a.blocks_done() < a.header().block_count;
                if unchanged
                    && a.read_differently()
                    && (incomplete || !matches!(a.check_stored(), Ok(StoredCheck::Matched)))
                {
                    self.meter.checking(None);
                    return self.set_aside(stream, s.item, s.entry, a);
                }
                let landed = if unchanged && incomplete && s.damaged >= 2 {
                    drop(a);
                    let why = Failure::new(Cause::NotVerified, SENT_DAMAGED);
                    let _ = self.journal.failed(s.entry, &why);
                    Ok(ReceiveOutcome::Failed(why))
                } else {
                    land(self.journal, dest, a, trailer, s.entry).map(|out| match out {
                        // Stored wrong again on this laptop, though the old laptop sent the
                        // same bytes each time.
                        ReceiveOutcome::Failed(f)
                            if unchanged && !incomplete && f.cause() == Cause::NotVerified =>
                        {
                            ReceiveOutcome::Failed(Failure::new(
                                Cause::CouldNotWrite,
                                NOT_STORED_HERE,
                            ))
                        }
                        out => out,
                    })
                };
                self.meter.checking(None);
                landed?
            }
            (_, Some(why)) => {
                // Recorded when it failed; recorded now if that did not get through.
                let _ = self.journal.failed(s.entry, &why);
                ReceiveOutcome::Failed(why)
            }
            (None, None) => {
                let why = Failure::new(Cause::NotVerified, "no file open");
                let _ = self.journal.failed(s.entry, &why);
                ReceiveOutcome::Failed(why)
            }
        };
        self.note_stored(&s.destination, &outcome);
        // Landed (kept on this laptop, never sent again), even when it still needs action.
        let ok = matches!(
            outcome,
            ReceiveOutcome::Finished(_) | ReceiveOutcome::NeedsAction { .. }
        );
        self.allowance.ended(s.item, ok);
        let landed = ok.then(|| (s.destination.clone(), s.size));
        self.end(s.item, stream, outcome, landed);
        Ok(Message::FileDone { stream, ok })
    }

    fn piece(
        &mut self,
        slot: &mut Slot,
        stream: u32,
        last: bool,
        bytes: &[u8],
    ) -> Result<Option<Message>, TransferError> {
        if slot.as_ref().is_some_and(|(held, _)| *held != stream) {
            return Err(protocol(
                "pieces of two blocks were mixed on one connection",
            ));
        }
        let mut written = None;
        let open = self
            .streams
            .get(&stream)
            .filter(|s| s.failure.is_none() && s.assembly.is_some())
            .map(|s| s.block_size);
        match open {
            // Not open (never started, refused, ended or failed): nothing is kept.
            None => *slot = None,
            Some(block_size) => {
                let limit = usize::try_from(block_size)
                    .unwrap_or(usize::MAX)
                    .saturating_add(BLOCK_WIRE_OVERHEAD);
                let buffer = &mut slot.get_or_insert_with(|| (stream, Vec::new())).1;
                if buffer.len().saturating_add(bytes.len()) > limit {
                    *slot = None;
                    self.fail_stream(
                        stream,
                        Failure::new(Cause::RefusedBySafetyCheck, "a block grew past its size"),
                    );
                } else {
                    buffer.extend_from_slice(bytes);
                    if last {
                        let whole = slot.take().map(|(_, b)| b).unwrap_or_default();
                        written = self.write_block(stream, &whole);
                    }
                }
            }
        }
        Ok(last.then_some(match written {
            Some((block, written)) => Message::Receipt {
                stream,
                block,
                written,
            },
            None => Message::Refused { stream },
        }))
    }

    /// Checks and writes one whole block of an open file; returns its number and whether it was
    /// written. A block that arrived damaged is not written and the file carries on (it is asked
    /// for again when the file ends; its number is `u64::MAX` if even that could not be read).
    /// `None` if it could not be written (then the file fails and its partial copy is removed).
    fn write_block(&mut self, stream: u32, whole: &[u8]) -> Option<(u64, bool)> {
        let journal = self.journal;
        let s = self.streams.get_mut(&stream)?;
        let accepted = Block::decode(whole, s.block_size).and_then(|b| {
            let hash = b.hash;
            let assembly = s
                .assembly
                .as_mut()
                .ok_or_else(|| protocol("no file open"))?;
            // A block sent again (it is already here) is confirmed, never recorded again, so
            // sending one block over and over cannot make the journal write over and over.
            let fresh = !assembly.has_block(b.index);
            let raw = b.data.len() as u64;
            let receipt = assembly.accept(b)?;
            Ok((receipt, fresh.then_some((hash, raw))))
        });
        match accepted {
            Ok((receipt, fresh)) => {
                self.written
                    .fetch_add(whole.len() as u64, Ordering::Relaxed);
                if let Some((hash, raw)) = fresh {
                    if let Some((group, _)) = self.steps.get(&s.item) {
                        self.meter.written(*group, raw);
                    }
                    s.pending.push((receipt.block, hash));
                    if s.pending.len() >= CHECKPOINT_BLOCKS {
                        flush_checkpoint(journal, s, false);
                    }
                }
                Some((receipt.block, true))
            }
            Err(TransferError::Damaged(_)) => {
                if let Some(s) = self.streams.get_mut(&stream) {
                    s.damaged = s.damaged.saturating_add(1);
                }
                Some((Block::number_in(whole).unwrap_or(u64::MAX), false))
            }
            Err(e) => {
                self.fail_stream(stream, Failure::of(&e, Side::New));
                None
            }
        }
    }

    /// At the end of the file on `stream`: if blocks are missing (they arrived damaged) or read
    /// back wrong from the drive, and it has not been asked for again [`SEND_AGAIN_ROUNDS`] times
    /// yet, asks for those blocks again and keeps the file open. `None` to end it now (it was
    /// stored right, it changed on the old laptop, or it is out of rounds).
    fn send_again(&mut self, stream: u32, trailer: &Trailer) -> Option<Message> {
        let s = self.streams.get_mut(&stream)?;
        if s.failure.is_some() || s.rounds >= SEND_AGAIN_ROUNDS {
            return None;
        }
        let a = s.assembly.as_mut()?;
        // A file that changed on the old laptop is never patched: it is sent again whole.
        if trailer.changed || trailer.stamp_after != a.header().stamp {
            return None;
        }
        let fixable = a.blocks_done() < a.header().block_count
            || matches!(a.check_stored(), Ok(StoredCheck::TakenBack(_)));
        if !fixable {
            return None;
        }
        s.rounds += 1;
        Some(Message::SendAgain {
            stream,
            ticket: a.resume_ticket(),
        })
    }

    /// The file on `stream` failed: its partial copy is removed (it ends when the old laptop ends
    /// it, and reports why).
    fn fail_stream(&mut self, stream: u32, why: Failure) {
        if let Some(s) = self.streams.get_mut(&stream) {
            s.assembly = None;
            // If this does not get through, recovery finds its partial file gone and records it.
            let _ = self.journal.failed(s.entry, &why);
            s.failure = Some(why);
            let item = s.item;
            self.settle(item);
        }
    }

    /// Records why a start was refused, once per file (and for a bounded number of files not in
    /// the plan), so refusals cannot pile up.
    fn refuse(&mut self, item: ItemId, why: Failure) {
        if self.refused.len() < MAX_REMEMBERED_REFUSALS || self.refused.contains_key(&item) {
            self.refused.insert(item, why);
        }
        self.settle(item);
    }

    /// Records how `item`'s attempt on `stream` ended. A file already on this laptop keeps its
    /// line (nothing can start it again, see [`Allowance::admit`]; this holds even if something
    /// did). Its older refusal, if any, is replaced by this end.
    fn end(
        &mut self,
        item: ItemId,
        stream: u32,
        outcome: ReceiveOutcome,
        landed: Option<(String, u64)>,
    ) -> bool {
        if self.ended.get(&item).is_some_and(Ended::spent) {
            return false;
        }
        self.refused.remove(&item);
        if let (ReceiveOutcome::Finished(_), Some((_, size))) = (&outcome, &landed) {
            self.meter.checked_bytes(*size);
            // A later file of the same size may be made from it.
            if *size >= COPY_MIN && self.copy_sources < MAX_COPY_SOURCES {
                self.copies.entry(*size).or_default().push(item);
                self.copy_sources += 1;
            }
        }
        self.ended.insert(
            item,
            Ended {
                stream,
                outcome,
                landed,
            },
        );
        self.settle(item);
        true
    }

    /// The latest file received in this move of exactly `size` bytes, in the same person's things
    /// as `destination` (see [`Whose`]), still there as it landed: where it is, opened to
    /// fingerprint.
    fn copy_source(&self, destination: &str, size: u64) -> Option<(CopyOf, std::fs::File)> {
        let wanted = whose(self.table, destination)?;
        for item in self.copies.get(&size)?.iter().rev().take(COPY_LOOKS) {
            let Some(Ended {
                outcome: ReceiveOutcome::Finished(finished),
                landed: Some((from, len)),
                ..
            }) = self.ended.get(item)
            else {
                continue;
            };
            if *len != size || whose(self.table, from).as_ref() != Some(&wanted) {
                continue;
            }
            let Ok(dest) = self.table.get(from) else {
                continue;
            };
            let Some((stored, now)) = IncomingPath::parse(&finished.final_path)
                .ok()
                .and_then(|path| dest.find(&path))
            else {
                continue;
            };
            if now != size {
                continue;
            }
            let Ok(file) = dest.open_read(&stored) else {
                continue;
            };
            return Some((
                CopyOf {
                    destination: from.clone(),
                    stored,
                    fingerprint: None,
                },
                file,
            ));
        }
        None
    }

    /// The old laptop's file on `stream` is identical to what is here: made here from a copy
    /// already received (see `copy_here`, answered once it is), or the partial copy dropped and
    /// the file already there kept. Always answered, so the old laptop records "already there"
    /// only on this laptop's word (a stream not open here is answered no).
    fn skipped(
        &mut self,
        stream: u32,
        mut replies: Vec<Message>,
    ) -> Result<(Vec<Message>, bool), TransferError> {
        let from_copy = self.streams.get(&stream).is_some_and(|s| {
            s.failure.is_none()
                && s.assembly.is_some()
                && s.copy_of.as_ref().is_some_and(|c| c.fingerprint.is_some())
        });
        if from_copy {
            self.to_copy = Some(stream);
            return Ok((replies, false));
        }
        // Identical to what is here: drop the partial copy and keep the original. Always
        // answered, so the old laptop records "already there" only on this laptop's word
        // (a stream not open here is answered no).
        let mut there = false;
        if let Some(s) = self.streams.remove(&stream) {
            // Its partial copy is removed first, then recorded.
            drop(s.assembly);
            let outcome = match s.same {
                Some(stored) => {
                    self.journal.existing(s.entry, &stored).map_err(record)?;
                    ReceiveOutcome::AlreadyThere(stored)
                }
                None => {
                    let why = Failure::new(Cause::RefusedBySafetyCheck, "skipped");
                    self.journal.failed(s.entry, &why).map_err(record)?;
                    ReceiveOutcome::Failed(why)
                }
            };
            // Already there: spent, like a file that landed.
            there = matches!(outcome, ReceiveOutcome::AlreadyThere(_));
            self.allowance.ended(s.item, there);
            if self.end(s.item, stream, outcome, None) && there {
                self.meter.already_there_bytes(s.size);
            }
        }
        replies.push(Message::FileDone { stream, ok: there });
        Ok((replies, false))
    }

    /// Counts a checked file towards its place's brake: a drive that could not store 3 of its
    /// last 20 checked files pauses the move (Quarantine Design, step 4). Other endings (a file
    /// that changed, a drive that went away) are not the drive's fault and are not counted.
    fn note_stored(&mut self, destination: &str, outcome: &ReceiveOutcome) {
        let stored = match outcome {
            ReceiveOutcome::Finished(_) => true,
            ReceiveOutcome::Failed(f) if f.why() == NOT_STORED_HERE => false,
            _ => return,
        };
        let lately = self
            .stored_lately
            .entry(destination.to_string())
            .or_default();
        lately.push_back(stored);
        while lately.len() > BRAKE_WINDOW {
            lately.pop_front();
        }
        if lately.iter().filter(|s| !**s).count() >= BRAKE_AFTER {
            lately.clear();
            self.control.brake(destination);
        }
    }

    /// Keeps what arrived for `item` beside its real name as a damaged copy, the old laptop's
    /// drive having read it differently each time: needs action, with where it is; the old laptop
    /// is told it did not arrive. Where the drive cannot keep it safely it fails, said plainly.
    fn set_aside(
        &mut self,
        stream: u32,
        item: ItemId,
        entry: u64,
        a: Assembly<'d>,
    ) -> Result<Message, TransferError> {
        let outcome = match a.set_aside() {
            Ok(kept) => {
                let why = format!("{READ_DIFFERENTLY} What arrived is kept as {kept}.");
                let _ = self
                    .journal
                    .failed(entry, &Failure::new(Cause::NotVerified, &why));
                ReceiveOutcome::NeedsAction {
                    finished: Finished {
                        final_path: kept.clone(),
                        sent_path: kept,
                        changes: Vec::new(),
                        mark_not_kept: None,
                        checked_from: None,
                    },
                    why,
                }
            }
            Err(_) => {
                let why = Failure::new(Cause::NotVerified, READ_DIFFERENTLY);
                let _ = self.journal.failed(entry, &why);
                ReceiveOutcome::Failed(why)
            }
        };
        // Spent: the same unsteady bytes are not asked for again in this move.
        self.allowance.ended(item, true);
        self.end(item, stream, outcome, None);
        Ok(Message::FileDone { stream, ok: false })
    }

    /// The old laptop's file on `stream` is not identical to what is here after all: its partial
    /// copy goes, nothing is recorded as its end, and the old laptop sends it (answered no).
    fn not_identical(&mut self, stream: u32) -> Message {
        if let Some(s) = self.streams.remove(&stream) {
            drop(s.assembly);
            let why = Failure::new(Cause::NotVerified, "not identical: it is sent instead");
            let _ = self.journal.failed(s.entry, &why);
            self.allowance.ended(s.item, false);
            self.settle(s.item);
        }
        Message::FileDone { stream, ok: false }
    }

    /// The key a file's fingerprint is compared under (move protocol 4): made from this session's
    /// secret and the file, so it is the same on every connection of the move and nowhere else.
    fn same_key_for(&self, item: ItemId) -> Option<[u8; 32]> {
        (self.version >= KEYED_SINCE)
            .then(|| *blake3::keyed_hash(&self.same_secret, item.to_hex().as_bytes()).as_bytes())
    }

    /// This laptop's own fingerprint for the start on `stream` (of the same-size file here, or
    /// of a copy already received), kept to judge the old laptop's print by.
    fn offered(&mut self, stream: u32, fingerprint: Option<[u8; 32]>) {
        if let Some(s) = self.streams.get_mut(&stream) {
            s.own_print = fingerprint;
            if let Some(c) = s.copy_of.as_mut() {
                c.fingerprint = fingerprint;
            }
        }
    }

    /// The copy to make here now, if the old laptop skipped a file offered as one already here.
    fn take_copy(&mut self) -> Option<LocalCopy> {
        let stream = self.to_copy.take()?;
        let s = self.streams.get(&stream)?;
        let c = s.copy_of.as_ref()?;
        Some(LocalCopy {
            stream,
            file: self
                .table
                .get(&c.destination)
                .ok()
                .and_then(|d| d.open_read(&c.stored).ok()),
            header: s.assembly.as_ref()?.header().clone(),
            expected: c.fingerprint?,
            key: self.same_key_for(s.item),
        })
    }

    /// Writes one block of a copy made here, through the same checks as a block that arrived;
    /// `false` if it could not be (the file has failed).
    fn copy_block(&mut self, stream: u32, block: Block) -> bool {
        let journal = self.journal;
        let Some(s) = self.streams.get_mut(&stream) else {
            return false;
        };
        let Some(assembly) = s.assembly.as_mut() else {
            return false;
        };
        let (hash, raw) = (block.hash, block.data.len() as u64);
        match assembly.accept(block) {
            Ok(receipt) => {
                if let Some((group, _)) = self.steps.get(&s.item) {
                    self.meter.written(*group, raw);
                }
                s.pending.push((receipt.block, hash));
                if s.pending.len() >= CHECKPOINT_BLOCKS {
                    flush_checkpoint(journal, s, false);
                }
                true
            }
            Err(e) => {
                self.fail_stream(stream, Failure::of(&e, Side::New));
                false
            }
        }
    }

    /// The copy on `stream` made here is complete and identical (`whole`): it ends as a sent file
    /// ends. If not, its partial copy is removed and it is not accepted, so the old laptop sends
    /// it after all.
    fn copied_here(
        &mut self,
        stream: u32,
        whole: bool,
        stamp: crate::Stamp,
    ) -> Result<Message, TransferError> {
        if whole {
            return self.finish_stream(stream, Trailer::new(stamp, false));
        }
        if let Some(s) = self.streams.remove(&stream) {
            drop(s.assembly);
            let why = Failure::new(
                Cause::NotVerified,
                "the copy already on this laptop could not be used",
            );
            let _ = self.journal.failed(s.entry, &why);
            self.allowance.ended(s.item, false);
            self.settle(s.item);
        }
        Ok(Message::FileDone { stream, ok: false })
    }

    /// A file already at the place `path` would land in, of exactly `size` bytes: its stored path,
    /// opened to fingerprint.
    fn same_file(
        &self,
        destination: &str,
        path: &str,
        size: u64,
    ) -> Option<(String, std::fs::File)> {
        let dest = self.table.get(destination).ok()?;
        let path = IncomingPath::parse(path).ok()?;
        let (stored, len) = dest.find(&path)?;
        if len != size {
            return None;
        }
        let file = dest.open_read(&stored).ok()?;
        Some((stored, file))
    }

    /// Checks every copied file again, a short while after copying: anything now missing or of
    /// the wrong size (often removed by security software) is reported failed, never copied.
    /// Returns the items that changed to failed.
    pub fn recheck(&mut self) -> Vec<ItemId> {
        let mut gone = Vec::new();
        for (item, e) in &mut self.ended {
            let ReceiveOutcome::Finished(finished) = &e.outcome else {
                continue;
            };
            let Some((destination, size)) = &e.landed else {
                continue;
            };
            let still_there = self
                .table
                .get(destination)
                .ok()
                .zip(IncomingPath::parse(&finished.final_path).ok())
                .and_then(|(dest, path)| dest.find(&path))
                .is_some_and(|(_, len)| len == *size);
            if !still_there {
                e.outcome = ReceiveOutcome::Failed(Failure::new(
                    Cause::RemovedAfterCopy,
                    "it was removed or changed soon after copying, often by security software",
                ));
                self.meter.unchecked_bytes(*size);
                gone.push(*item);
            }
        }
        for item in &gone {
            self.settle(*item);
        }
        gone
    }

    /// Plans the write in the journal, then creates its temporary file, then records it staged.
    fn start(
        &self,
        item: ItemId,
        destination: &str,
        path: &str,
        header: crate::Header,
    ) -> Result<(Assembly<'d>, u64), Start> {
        let refused =
            |e: GateError| Start::Refused(Failure::new(Cause::of_gate(&e), e.to_string()));
        let table: &'d Destinations = self.table;
        let dest = table.get(destination).map_err(refused)?;
        let place = table.place(destination).map_err(refused)?;
        let parsed = IncomingPath::parse(path).map_err(|e| refused(GateError::Path(e)))?;
        let (permission, for_account) = match place {
            Approved::MyFolders | Approved::ChosenDrive | Approved::OffloadDrive => {
                (Permission::OwnFolders, self.account.clone())
            }
            Approved::SharedFolder => (Permission::SharedFolder, EVERYONE.to_string()),
            Approved::AnotherAccount { account_id } => {
                (Permission::AdminHelper, account_id.clone())
            }
        };
        let write = PlannedWrite {
            download_mark: header.mark,
            item,
            source_laptop: self.allowance.source_laptop(),
            destination: destination.to_string(),
            path: path.to_string(),
            size: header.size,
            actor: Actor {
                acting_account: self.account.clone(),
                for_account,
                permission,
            },
            block_size: header.block_size,
            source_modified_ns: header.stamp.modified_ns,
            place: dest.folder_identity("").ok().flatten().map(file_id),
            source_file: header.source_file,
            partial_keep: self.allowance.partial_keep(),
        };
        let entry = self.journal.plan(&write).map_err(Start::Record)?;
        let assembly =
            match Assembly::start_tagged(dest, &parsed, header, &self.journal.temp_tag(entry)) {
                Ok(a) => a,
                Err(e) => {
                    let why = Failure::of(&e, Side::New);
                    self.journal.failed(entry, &why).map_err(Start::Record)?;
                    return Err(Start::Refused(why));
                }
            };
        let made: Vec<(String, Option<FileId>)> = assembly
            .created_folders()
            .iter()
            .map(|f| {
                (
                    f.clone(),
                    dest.folder_identity(f).ok().flatten().map(file_id),
                )
            })
            .collect();
        // If this fails the partial file goes with the assembly; recovery records the rest.
        self.journal
            .staged(entry, &assembly.temp_path(), &made)
            .map_err(Start::Record)?;
        Ok((assembly, entry))
    }
}

impl Drop for ReceiverSession<'_> {
    /// The app quitting (or the session ending any other way but [`cancel`](Self::cancel)):
    /// every partly received file the journal knows is kept on disk, with its blocks checkpointed,
    /// so the move continues after the app starts again. Nothing is thrown away by ending.
    fn drop(&mut self) {
        self.checkpoint();
        for s in self.streams.values_mut().chain(self.restored.values_mut()) {
            if s.failure.is_none()
                && let Some(assembly) = s.assembly.take()
            {
                assembly.persist();
            }
        }
    }
}

/// Said of a file the new laptop was answered for more than any move needs.
const READ_TOO_OFTEN: &str = "the new laptop asked for it again many more times than a move needs, so PCTwin stopped copying it. Your original is untouched";

/// Said of a file whose versions outnumbered the attempts any file gets.
const KEPT_CHANGING: &str = "it kept changing while it was being read; close the program using it and move it again. Your original is untouched";

/// Said of a file asked for again by a description that is not of the file sent.
const TICKET_WRONG: &str = "the new laptop asked for parts of it that do not match the file, so it was not finished. Your original is untouched";

/// The window after a loss (at least: see [`SenderSession::floor`]).
const FLOOR_BYTES: u64 = 1024 * 1024;

/// How many more times than once the new laptop may be answered for one file, per version. The
/// design's derivation is the whole file sent again in each of the two rounds
/// ([`SEND_AGAIN_ROUNDS`]) plus two restarts of the new laptop that each lose all of it: 4. It was
/// measured before it was set (15 October 2026): on a drive that stores every block wrong, with two
/// restarts during one file, the new laptop was answered 4.992 times the file, one block under a
/// cap of 4; so it is one more, a whole file of room for that very case. (Every other honest probe
/// measured at most 3: drops never count, since a lost block is never receipted.)
const REREAD_TIMES: u64 = SEND_AGAIN_ROUNDS as u64 + 3;

/// The most the new laptop may be answered for a file of `size` bytes, per version (as if at
/// least 1 MiB).
fn answered_cap(size: u64) -> u64 {
    size.max(1 << 20).saturating_mul(1 + REREAD_TIMES)
}

/// Said of a file whose blocks were asked for again more often than one attempt allows.
/// Quarantine Design, step 4: this many of a place's last [`BRAKE_WINDOW`] checked files not
/// stored correctly pause the move.
const BRAKE_AFTER: usize = 3;
const BRAKE_WINDOW: usize = 20;

/// Quarantine Design, step 3: the words for a file that would not come right.
const READ_DIFFERENTLY: &str = "This file read differently each time on your old laptop, so its drive may be failing. Your original is untouched.";
const NOT_STORED_HERE: &str =
    "This laptop could not store this file correctly. Your original is untouched.";
const SENT_DAMAGED: &str = "The old laptop sent it damaged each time. Your original is untouched.";

const GAVE_UP: &str = "parts of it kept arriving damaged or being stored wrong, so it was not finished. Your original is untouched";

/// How many times a file's missing or wrongly stored blocks are asked for again before it fails.
const SEND_AGAIN_ROUNDS: u8 = 2;

/// Blocks written before they are checkpointed in the journal together.
const CHECKPOINT_BLOCKS: usize = 64;
/// Every this many checkpoints, one is made durable at once (the others ride on the journal's next
/// durable step), so a long file's checkpoints never pile up only in memory.
const DURABLE_EVERY: u32 = 16;

/// Records a file's blocks written since its last checkpoint. Best effort: if it does not get
/// through, those blocks are only sent again after a restart.
fn flush_checkpoint(journal: &dyn Ledger, s: &mut Incoming<'_>, durable: bool) {
    if s.pending.is_empty() || s.assembly.is_none() {
        s.pending.clear();
        return;
    }
    s.batches = s.batches.wrapping_add(1);
    let durable = durable || s.batches.is_multiple_of(DURABLE_EVERY);
    let _ = journal.checkpoint(s.entry, &s.pending, durable);
    s.pending.clear();
}

/// Said of a checked file that could not be given its name just now.
const FINISHED_LATER: &str = "PCTwin finishes it the next time it starts";
/// Said of a file gone the moment it got its name.
const REMOVED_AT_ONCE: &str = "it was removed as soon as it was copied, often by security software";

/// Who a shared folder's files are for.
const EVERYONE: &str = "everyone";

/// Why a file did not start: refused (the file fails on its own), or the journal could not be
/// written (the move stops).
enum Start {
    Refused(Failure),
    Record(JournalError),
}

fn record(e: JournalError) -> TransferError {
    TransferError::Record(e.to_string())
}

fn file_id(id: pctwin_gate::FileId) -> FileId {
    id
}

/// The new laptop's step for a file that ended this way.
fn outcome_step(outcome: &ReceiveOutcome) -> Step {
    match outcome {
        ReceiveOutcome::Finished(_) => Step::Copied,
        ReceiveOutcome::AlreadyThere(_) => Step::AlreadyThere,
        ReceiveOutcome::NeedsAction { .. } => Step::NeedsAction,
        ReceiveOutcome::Failed(_) => Step::Failed,
    }
}

/// Plans up to this many files are checked against the meter after every message in debug builds.
const CHECKED_PLAN: usize = 256;

/// Every file of the approved plan, counted waiting in the group of its destination.
fn plan_steps(allowance: &Allowance, meter: &MoveMeter) -> HashMap<ItemId, (Group, Step)> {
    meter.reset();
    let steps = allowance
        .planned()
        .map(|(item, destination, size)| (item, (meter.plan(destination, size), Step::Waiting)))
        .collect();
    meter.plan_loaded();
    steps
}

/// Most names tried when other programs keep taking the free one first.
const MAX_NAME_TRIES: u32 = 64;

/// Lands a complete file through the journal: verified (every byte checked, the original
/// unchanged, all of it on disk), its name recorded before it gets it, then committed with what it
/// landed as. A file that cannot land is recorded failed with why; if the journal cannot be
/// written the move stops, and recovery finishes whatever the journal already has.
fn land(
    journal: &dyn Ledger,
    dest: &Destination,
    assembly: Assembly<'_>,
    trailer: Trailer,
    entry: u64,
) -> Result<ReceiveOutcome, TransferError> {
    let fail = |why: Failure| -> Result<ReceiveOutcome, TransferError> {
        journal.failed(entry, &why).map_err(record)?;
        Ok(ReceiveOutcome::Failed(why))
    };
    // Checked and on disk: from here only success or recovery ends it, never a throw-away. A
    // file that cannot be named now is kept for recovery to finish at the next start.
    let later = |sealed: pctwin_gate::Sealed<'_>, why: &GateError| {
        sealed.persist();
        Ok(ReceiveOutcome::Failed(Failure::new(
            Cause::of_gate(why),
            format!("{why}; {FINISHED_LATER}"),
        )))
    };
    let (size, block_size) = (assembly.header().size, assembly.header().block_size);
    let (mut sealed, fingerprint) = match assembly.seal(trailer) {
        Ok(sealed) => sealed,
        Err(e) => return fail(Failure::of(&e, Side::New)),
    };
    // Which file it is, so after a crash only this very file is ever taken as it.
    let sealed_as = sealed.identity().ok().map(file_id);
    // If the journal cannot take it, the checked file is still kept (never thrown away), for
    // recovery to finish once the journal works again.
    if let Err(e) = journal.verified_as(entry, fingerprint, sealed_as, sealed.checked_from()) {
        sealed.persist();
        return Err(record(e));
    }
    let mut tries = 0;
    let claimed = loop {
        tries += 1;
        if tries > MAX_NAME_TRIES {
            return later(sealed, &GateError::TooManyClashes);
        }
        let name = match sealed.next_name() {
            Ok(name) => name,
            Err(e) => return later(sealed, &e),
        };
        let whole = crate::naming::Whole {
            size,
            block_size,
            fingerprint,
        };
        // Recorded first, then named the way the drive allows (filled in, on a drive with no
        // safe rename).
        let named = match crate::naming::name_as(journal, entry, sealed, &name, whole) {
            Ok(named) => named,
            Err(broke) => {
                let (e, back) = *broke;
                back.persist();
                return Err(record(e));
            }
        };
        match named.claim {
            Claim::Named(c) => break c,
            // Another program took the name first: another name, recorded first again.
            Claim::Taken(back) => sealed = back,
            Claim::Failed(e, back) => return later(back, &e),
        }
    };
    // Committed while the temporary name still holds the file, then the temporary name goes.
    match dest.stat(&claimed.finished().final_path) {
        Ok(Some(stat)) => {
            if let Some(show) = claimed.show().cloned() {
                // Filled in under its name: committed while still hidden and tagged, then shown.
                let landed = Landed {
                    size: stat.len,
                    modified_ns: show.modified_ns,
                    file: Some(file_id(stat.id)),
                };
                let Some(check) = claimed.show_check() else {
                    return Err(record(pctwin_journal::JournalError::Storage(
                        "a filled-in file not held".into(),
                    )));
                };
                let end =
                    crate::naming::commit_and_show(journal, dest, entry, landed, &show, check)
                        .map_err(record)?;
                // Kept whatever the show came to: the journal has it, and recovery or the person
                // finishes it. Reported copied only once shown.
                let finished = claimed.keep();
                return Ok(match end {
                    crate::naming::ShowEnd::Shown => ReceiveOutcome::Finished(finished),
                    crate::naming::ShowEnd::Gone(why)
                    | crate::naming::ShowEnd::Left(why)
                    | crate::naming::ShowEnd::Later(why) => {
                        ReceiveOutcome::NeedsAction { finished, why }
                    }
                });
            } else {
                journal
                    .committed(
                        entry,
                        Landed {
                            size: stat.len,
                            modified_ns: stat.modified.map(crate::nanos),
                            file: Some(file_id(stat.id)),
                        },
                    )
                    .map_err(record)?;
            }
            Ok(ReceiveOutcome::Finished(claimed.keep()))
        }
        // Gone the moment it got its name: never reported copied.
        Ok(None) => fail(Failure::new(Cause::RemovedAfterCopy, REMOVED_AT_ONCE)),
        // Could not look just now: recovery finishes it (the journal has its name).
        Err(e) => Ok(ReceiveOutcome::Failed(Failure::new(
            Cause::of_io(&e, Side::New),
            format!("{e}; {FINISHED_LATER}"),
        ))),
    }
}

#[cfg(test)]
mod originals_limits {
    use super::*;
    use tokio::sync::mpsc;

    struct Mem {
        tx: mpsc::UnboundedSender<Vec<u8>>,
        rx: mpsc::UnboundedReceiver<Vec<u8>>,
    }

    fn pair() -> (Mem, Mem) {
        let (a_tx, b_rx) = mpsc::unbounded_channel();
        let (b_tx, a_rx) = mpsc::unbounded_channel();
        (Mem { tx: a_tx, rx: a_rx }, Mem { tx: b_tx, rx: b_rx })
    }

    impl Channel for Mem {
        async fn send(&mut self, data: &[u8]) -> Result<(), ChannelError> {
            self.tx.send(data.to_vec()).map_err(|_| ChannelError)
        }
        async fn recv(&mut self) -> Result<Vec<u8>, ChannelError> {
            self.rx.recv().await.ok_or(ChannelError)
        }
    }

    const NONCE: Nonce = [0x42; NONCE_LEN];

    type Looker = fn(&HashMap<ItemId, PathBuf>, &[ItemId]) -> Vec<(ItemId, OriginalNow)>;

    fn id(n: u8) -> ItemId {
        ItemId::from_hex(&format!("{n:02x}{}", "0".repeat(30))).unwrap()
    }

    fn present(n: u64) -> OriginalNow {
        OriginalNow::Present {
            size: n,
            modified_ns: Some(1),
            file: Some(FileId {
                volume: 1,
                index: std::num::NonZeroU64::new(n).unwrap(),
                born: None,
            }),
        }
    }

    /// Always says "present" for everything asked.
    fn all_present(_: &HashMap<ItemId, PathBuf>, items: &[ItemId]) -> Vec<(ItemId, OriginalNow)> {
        items.iter().map(|i| (*i, present(1))).collect()
    }

    fn sent_all(n: u8) -> HashMap<ItemId, PathBuf> {
        (1..=n).map(|i| (id(i), PathBuf::from("x"))).collect()
    }

    /// One request of `items`, served with `budget`; returns what the new laptop would read.
    async fn round(
        budget: &mut OriginalsBudget,
        items: Vec<ItemId>,
        answer: Looker,
    ) -> Vec<(ItemId, OriginalNow)> {
        let (mut new, mut old) = pair();
        new.send(
            &Message::CheckOriginals {
                nonce: NONCE,
                items,
            }
            .encode(),
        )
        .await
        .unwrap();
        serve_with(&mut old, &sent_all(10), budget, answer)
            .await
            .unwrap();
        let Message::Originals { nonce, answers } =
            Message::decode(&new.recv().await.unwrap()).unwrap()
        else {
            panic!("not an answer")
        };
        assert_eq!(nonce, NONCE, "the nonce is echoed, refused or not");
        answers
    }

    fn big() -> Duration {
        Duration::from_secs(60)
    }

    #[tokio::test]
    async fn within_the_limits_the_looks_are_answered() {
        let mut b = OriginalsBudget::with_limits(3, 2, big());
        let a = round(&mut b, vec![id(1), id(2)], all_present).await;
        assert!(a.iter().all(|(_, n)| *n == present(1)), "{a:?}");
        // Exactly the rest of the item allowance is still allowed.
        let a = round(&mut b, vec![id(3)], all_present).await;
        assert_eq!(a, vec![(id(3), present(1))]);
    }

    #[tokio::test]
    async fn over_the_item_cap_the_request_is_refused_as_cannot_look() {
        let mut b = OriginalsBudget::with_limits(3, 10, big());
        round(&mut b, vec![id(1), id(2)], all_present).await;
        let a = round(&mut b, vec![id(3), id(4)], all_present).await;
        assert_eq!(
            a,
            vec![
                (id(3), OriginalNow::CannotLook),
                (id(4), OriginalNow::CannotLook)
            ]
        );
        // And once refused, even a request that would have fitted stays refused.
        let a = round(&mut b, vec![id(1)], all_present).await;
        assert_eq!(a, vec![(id(1), OriginalNow::CannotLook)]);
    }

    #[tokio::test]
    async fn over_the_request_cap_the_request_is_refused_as_cannot_look() {
        let mut b = OriginalsBudget::with_limits(100, 2, big());
        round(&mut b, vec![id(1)], all_present).await;
        round(&mut b, vec![id(1)], all_present).await;
        let a = round(&mut b, vec![id(1)], all_present).await;
        assert_eq!(a, vec![(id(1), OriginalNow::CannotLook)]);
    }

    #[tokio::test]
    async fn a_look_that_takes_too_long_is_answered_cannot_look() {
        fn slow(_: &HashMap<ItemId, PathBuf>, items: &[ItemId]) -> Vec<(ItemId, OriginalNow)> {
            std::thread::sleep(Duration::from_millis(600));
            items.iter().map(|i| (*i, present(1))).collect()
        }
        let mut b = OriginalsBudget::with_limits(100, 100, Duration::from_millis(50));
        let a = round(&mut b, vec![id(1), id(2)], slow).await;
        assert_eq!(
            a,
            vec![
                (id(1), OriginalNow::CannotLook),
                (id(2), OriginalNow::CannotLook)
            ]
        );
    }

    #[tokio::test]
    async fn a_look_that_hung_holds_one_thread_and_no_new_look_starts_until_it_ends() {
        use std::sync::atomic::AtomicUsize;
        static CALLS: AtomicUsize = AtomicUsize::new(0);
        fn slow(_: &HashMap<ItemId, PathBuf>, items: &[ItemId]) -> Vec<(ItemId, OriginalNow)> {
            CALLS.fetch_add(1, Ordering::SeqCst);
            std::thread::sleep(Duration::from_millis(800));
            items.iter().map(|i| (*i, present(1))).collect()
        }
        fn quick(_: &HashMap<ItemId, PathBuf>, items: &[ItemId]) -> Vec<(ItemId, OriginalNow)> {
            CALLS.fetch_add(1, Ordering::SeqCst);
            items.iter().map(|i| (*i, present(1))).collect()
        }
        let mut b = OriginalsBudget::with_limits(100, 100, Duration::from_millis(50));
        let first = round(&mut b, vec![id(1)], slow).await;
        assert_eq!(first, vec![(id(1), OriginalNow::CannotLook)]);
        // The first look is still running: this one is refused without being started.
        let second = round(&mut b, vec![id(2)], quick).await;
        assert_eq!(second, vec![(id(2), OriginalNow::CannotLook)]);
        assert_eq!(
            CALLS.load(Ordering::SeqCst),
            1,
            "no second look was started"
        );
        // Once the hung look ends, looking works again.
        while b.look_running.load(Ordering::SeqCst) {
            tokio::time::sleep(Duration::from_millis(20)).await;
        }
        let third = round(&mut b, vec![id(3)], quick).await;
        assert_eq!(third, vec![(id(3), present(1))]);
    }

    #[test]
    fn the_session_limits_are_what_they_are_documented_to_be() {
        let b = OriginalsBudget::new();
        assert_eq!(b.items_left, MAX_ORIGINALS_PER_SESSION);
        assert_eq!(b.requests_left, MAX_ORIGINAL_REQUESTS_PER_SESSION);
        assert_eq!(b.time, ORIGINALS_REQUEST_TIME);
        assert!(ORIGINALS_REPLY_WAIT > ORIGINALS_REQUEST_TIME);
        const { assert!(MAX_ORIGINALS_PER_SESSION >= MAX_ORIGINALS_PER_REQUEST) };
    }

    #[tokio::test]
    async fn an_old_laptop_that_never_answers_is_an_error_not_a_confirmation() {
        let dir = tempfile::tempdir().unwrap();
        let journal = Journal::open(&dir.path().join("j.redb")).unwrap();
        let (mut new, _old) = pair();
        let r =
            check_originals_within(&mut new, &journal, &[id(1)], Duration::from_millis(50)).await;
        assert!(matches!(r, Err(TransferError::Protocol(_))), "{r:?}");
    }
}

#[cfg(test)]
mod outcomes {
    use super::*;
    use pctwin_record::{LaptopId, Record};

    /// A receiver for a plan with nothing in it: only its own record-keeping is under test.
    fn with_receiver(test: impl FnOnce(&mut ReceiverSession<'_>)) {
        let dir = tempfile::tempdir().unwrap();
        let journal = Journal::open(&dir.path().join("journal.redb")).unwrap();
        let table = Destinations::new();
        let record = Record::new(LaptopId::from_hex("00112233445566778899aabbccddeeff").unwrap());
        let approval = record.approve().unwrap();
        let allowance = Allowance::from_record(&record, &approval, |_| None).unwrap();
        let mut receiver = ReceiverSession::new(&table, allowance, &journal, "1001");
        test(&mut receiver);
    }

    fn id(n: u8) -> ItemId {
        ItemId::from_hex(&format!("{n:02x}{}", "0".repeat(30))).unwrap()
    }

    #[test]
    fn a_file_on_this_laptop_keeps_its_line_whatever_ends_after_it() {
        with_receiver(|r| {
            let there = ReceiveOutcome::AlreadyThere("Documents/a.txt".into());
            r.end(id(1), 0, there.clone(), None);
            // Even if something got past the plan's own check, nothing rewrites it.
            r.end(
                id(1),
                0,
                ReceiveOutcome::Failed(Failure::new(Cause::NotVerified, "it was started again")),
                None,
            );
            r.end(
                id(1),
                7,
                ReceiveOutcome::Failed(Failure::new(Cause::NotVerified, "it was started again")),
                None,
            );
            r.refuse(
                id(1),
                Failure::new(Cause::NotVerified, "it has already been moved"),
            );
            assert_eq!(r.outcome(id(1)), Some(there));
            // And the next connection is told it is done, by the file (a stream number may have
            // been given to another file since).
            assert_eq!(
                r.hello(),
                vec![
                    Message::ItemDone {
                        item: id(1),
                        ok: true
                    },
                    Message::Ready
                ]
            );
        });
    }

    #[test]
    fn a_file_not_on_this_laptop_shows_the_latest_of_its_end_and_its_refusal() {
        with_receiver(|r| {
            r.end(
                id(2),
                3,
                ReceiveOutcome::Failed(Failure::new(Cause::NotVerified, "first")),
                None,
            );
            r.refuse(id(2), Failure::new(Cause::NotVerified, "second"));
            assert_eq!(
                r.outcome(id(2)),
                Some(ReceiveOutcome::Failed(Failure::new(
                    Cause::NotVerified,
                    "second"
                )))
            );
            r.end(
                id(2),
                4,
                ReceiveOutcome::Failed(Failure::new(Cause::NotVerified, "third")),
                None,
            );
            assert_eq!(
                r.outcome(id(2)),
                Some(ReceiveOutcome::Failed(Failure::new(
                    Cause::NotVerified,
                    "third"
                )))
            );
        });
    }
}
