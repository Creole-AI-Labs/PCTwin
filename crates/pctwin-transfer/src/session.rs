use std::cell::{Cell, RefCell};
use std::collections::{BTreeMap, HashMap, HashSet, VecDeque};
use std::future::Future;
use std::path::PathBuf;
use std::sync::Arc;
use std::sync::atomic::{AtomicU64, Ordering};

use pctwin_gate::{Approved, Claim, Destination, Destinations, Finished, IncomingPath};
use pctwin_journal::{
    Actor, FileId, JournalError, Landed, Ledger, Permission, PlannedWrite, State,
};
use pctwin_record::ItemId;

use crate::message::{BLOCK_WIRE_OVERHEAD, Message, split_into_pieces};
use crate::queue::{Scheduler, Tier};
use crate::reading::{ReadBudget, is_drive_error};
use crate::sections::FileSections;
use crate::{
    Allowance, Assembly, Block, FileSender, FsOpener, Opener, ResumeTicket, Trailer, TransferError,
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
}

/// How a sent file ended.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum SendOutcome {
    /// The new laptop confirmed it is complete under its real name.
    Arrived,
    /// The new laptop already had an identical file there, so it was not sent.
    AlreadyThere,
    Failed(String),
}

enum SendState {
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
        failure: Option<String>,
        /// It could not be read; try it once more after everything else.
        later: bool,
    },
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
    /// Per job.
    states: Vec<SendState>,
    /// Per job: the stream of its current attempt.
    streams: Vec<u32>,
    /// Stream to job, for current attempts only.
    job_of: HashMap<u32, usize>,
    next_stream: u32,
    by_item: HashMap<ItemId, usize>,
    scheduler: Scheduler,
    capacity: usize,
    blocks_sent: u64,
    in_flight_limit: u64,
    /// Jobs found identical on the new laptop, to tell it to skip.
    skips: Vec<usize>,
    /// Ends to send on the main connection (a file finished, or failed partway).
    ends: Vec<Message>,
    opener: Arc<dyn Opener>,
    budget: ReadBudget,
    /// Jobs that could not be read, waiting for one more try after everything else.
    later: Vec<usize>,
    /// Jobs already given their second try.
    retried: HashSet<usize>,
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
        let states = jobs.iter().map(|_| SendState::Waiting).collect();
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
            blocks_sent: 0,
            in_flight_limit: IN_FLIGHT_BYTES,
            skips: Vec::new(),
            ends: Vec::new(),
            opener: Arc::new(FsOpener),
            budget: ReadBudget::for_plan(ReadPlan::Normal),
            later: Vec::new(),
            retried: HashSet::new(),
            lanes: Vec::new(),
            in_flight: 0,
            dirty: false,
            checks: Vec::new(),
            attempted: HashSet::new(),
        }
    }

    /// Sends at most `bytes` (over every lane together) before the new laptop confirms them
    /// (32 MiB unless set).
    pub fn with_in_flight_limit(mut self, bytes: u64) -> Self {
        self.in_flight_limit = bytes.max(1);
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
        self
    }

    /// Whether reading stopped because the old drive kept failing.
    pub fn stopped_reading(&self) -> bool {
        self.budget.stopped()
    }

    /// Blocks sent so far, over every connection and lane.
    pub fn blocks_sent(&self) -> u64 {
        self.blocks_sent
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
        self.resume(main).await?;
        self.lanes = vec![VecDeque::new()];
        self.in_flight = 0;
        let state = RefCell::new(self);
        let changed = Notify::new();
        // Extra lanes still running.
        let alive = Cell::new(0usize);
        let main_loop = async {
            loop {
                // Registered before looking, so a change made meanwhile is not missed.
                let wake = changed.notified();
                // The state is never borrowed across a wait, so every lane can take its turn.
                let control = state.borrow_mut().control();
                if !control.is_empty() {
                    for m in &control {
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
                    compare_fingerprints(&state).await;
                    changed.notify_waiters();
                } else if state.borrow().all_done() {
                    break;
                } else if alive.get() > 0 {
                    // Other lanes are still at work: wait until one of them changes something.
                    wake.await;
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
                            running.push(send_lane(&state, &changed, &alive, lane, ch));
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
                Message::Ready => break,
                _ => return Err(protocol("expected the new laptop's resume list")),
            }
        }
        self.scheduler = Scheduler::new(self.capacity);
        self.ends.clear();
        self.skips.clear();
        for job in 0..self.jobs.len() {
            if matches!(self.states[job], SendState::Done(_)) || self.later.contains(&job) {
                // Done, or still waiting for its second try at the end.
                continue;
            }
            let picked_up = tickets.get(&job).and_then(|ticket| {
                let j = &self.jobs[job];
                let reader =
                    FileSender::open_with(&*self.opener, &j.source, Some(ticket), j.compressible)
                        .ok()?;
                let sections = sections_for(reader.header(), &ticket.done.to_bools())?;
                Some(SendState::Open {
                    reader,
                    sections,
                    announced: false,
                    resumed_done: ticket.done.done_count(),
                    cleared: false,
                })
            });
            // Without a ticket, or if the file changed since, it starts again from the beginning.
            self.states[job] = picked_up.unwrap_or(SendState::Waiting);
            self.scheduler
                .push(self.jobs[job].item, self.jobs[job].tier);
        }
        Ok(())
    }

    /// Messages for the main connection: skips, ends, and starts of the files now in flight.
    fn control(&mut self) -> Vec<Message> {
        let mut out = Vec::new();
        for job in std::mem::take(&mut self.skips) {
            out.push(Message::Skip {
                stream: self.streams[job],
            });
            self.scheduler.finished(self.jobs[job].item);
            self.states[job] = SendState::Done(SendOutcome::AlreadyThere);
        }
        out.append(&mut self.ends);
        // A file ends once every block is confirmed, so the new laptop has them all first.
        for job in 0..self.states.len() {
            let complete = matches!(
                &self.states[job],
                SendState::Open { sections, cleared: true, .. } if sections.is_done()
            );
            if complete
                && let SendState::Open { reader, .. } =
                    std::mem::replace(&mut self.states[job], SendState::Waiting)
            {
                out.push(self.end(job, reader));
            }
        }
        // Everything readable is done: one more try for what could not be read.
        let idle = !self
            .states
            .iter()
            .any(|s| matches!(s, SendState::Open { .. } | SendState::Ended { .. }))
            && self.in_flight == 0;
        if idle && !self.later.is_empty() && !self.budget.stopped() {
            for job in std::mem::take(&mut self.later) {
                self.retried.insert(job);
                self.scheduler
                    .push(self.jobs[job].item, self.jobs[job].tier);
            }
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
            } = &mut self.states[job]
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

    /// Opens a waiting file for a new attempt, with a new stream.
    fn open(&mut self, job: usize) {
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
                self.states[job] = SendState::Open {
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
                self.states[job] = if drive && self.read_failed(job) {
                    // One more try after everything else.
                    self.later.push(job);
                    SendState::Waiting
                } else {
                    SendState::Done(SendOutcome::Failed(e.to_string()))
                };
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

    /// Every block of `job` is confirmed: its end, after checking it did not change while read.
    fn end(&mut self, job: usize, reader: FileSender) -> Message {
        let stream = self.streams[job];
        let stamp = reader.header().stamp;
        self.scheduler.finished(self.jobs[job].item);
        match reader.finish() {
            Ok(trailer) => {
                // On a weak drive each file is read only once: a file that changed is not read again.
                let changed = trailer.changed_while_read();
                let again = changed && self.budget.try_again_later();
                self.states[job] = SendState::Ended {
                    retry: again,
                    failure: (changed && !again).then(|| "it changed while being read".to_string()),
                    later: false,
                };
                Message::EndFile {
                    stream,
                    stamp_after: trailer.stamp_after,
                    changed,
                }
            }
            Err(e) => {
                self.states[job] = SendState::Ended {
                    retry: false,
                    failure: Some(e.to_string()),
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
    fn work(&mut self, lane: usize) -> Work {
        if self.budget.stopped() || self.in_flight >= self.in_flight_limit {
            return Work::Wait;
        }
        let lane_no = u32::try_from(lane).unwrap_or(u32::MAX);
        for item in self.scheduler.in_flight() {
            let job = self.by_item[&item];
            let stream = self.streams[job];
            let SendState::Open {
                reader,
                sections,
                cleared: true,
                ..
            } = &mut self.states[job]
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
            match reader.block_at(u64::from(b)) {
                Ok(block) => {
                    self.budget.read_ok();
                    let wire = block.encode();
                    let bytes = wire.len() as u64;
                    self.lanes[lane].push_back((stream, b, bytes));
                    self.in_flight += bytes;
                    self.blocks_sent += 1;
                    return Work::Send { stream, wire };
                }
                Err(e) => self.read_error(job, e),
            }
        }
        Work::Wait
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
        self.ends.push(Message::EndFile {
            stream: self.streams[job],
            stamp_after: stamp,
            changed: true,
        });
        self.scheduler.finished(self.jobs[job].item);
        // Ended first, so stopping reading (if this was one error too many) leaves it be.
        self.states[job] = SendState::Ended {
            retry,
            failure: None,
            later: false,
        };
        let later = drive && self.read_failed(job);
        self.states[job] = SendState::Ended {
            retry,
            failure: (!retry && !later).then(|| e.to_string()),
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

    /// Fingerprint comparisons waiting to run: (job, the new laptop's fingerprint).
    fn take_checks(&mut self) -> Vec<(usize, std::path::PathBuf, [u8; 32])> {
        std::mem::take(&mut self.checks)
            .into_iter()
            .map(|(job, theirs)| (job, self.jobs[job].source.clone(), theirs))
            .collect()
    }

    /// The comparison for `job` is done: an identical file is skipped, anything else is sent.
    fn checked(&mut self, job: usize, identical: bool) {
        if let SendState::Open { cleared, .. } = &mut self.states[job] {
            if identical {
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

    /// Whether a reply is owed on the main connection: an answer to a start or an end, or a
    /// receipt for a block it carried.
    fn awaiting_main(&self) -> bool {
        !self.lanes[0].is_empty()
            || self.states.iter().any(|s| {
                matches!(
                    s,
                    SendState::Open {
                        announced: true,
                        cleared: false,
                        ..
                    } | SendState::Ended { .. }
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
                    self.ends.push(Message::EndFile {
                        stream,
                        stamp_after: reader.header().stamp,
                        changed: true,
                    });
                    self.scheduler.finished(self.jobs[job].item);
                    self.states[job] = SendState::Ended {
                        retry: false,
                        failure: Some("the new laptop could not write part of it".into()),
                        later: false,
                    };
                    self.dirty = true;
                }
                Ok(())
            }
            Message::Receipt { .. } => {
                let (stream, block, bytes) = self.lanes[lane]
                    .pop_front()
                    .ok_or_else(|| protocol("a receipt for nothing sent"))?;
                self.in_flight -= bytes;
                // Only the current attempt counts; a receipt for an earlier one frees room only.
                if let Some(&job) = self.job_of.get(&stream)
                    && let SendState::Open { sections, .. } = &mut self.states[job]
                {
                    let lane_no = u32::try_from(lane).unwrap_or(u32::MAX);
                    // In order per lane, as sent; a mismatch can only mean the new laptop
                    // failed the file, which it reports when the file ends.
                    let _ = sections.confirmed(lane_no, block);
                }
                Ok(())
            }
            Message::FileDone { stream, ok } if lane == 0 => {
                if let Some(&job) = self.job_of.get(&stream) {
                    self.done(job, ok);
                }
                Ok(())
            }
            Message::Have { stream, same_size } if lane == 0 => {
                let Some(&job) = self.job_of.get(&stream) else {
                    return Ok(());
                };
                match same_size {
                    // A same-size file is there: compare fingerprints, off the main loop (see
                    // `take_checks`).
                    Some(theirs) => self.checks.push((job, theirs)),
                    None => self.checked(job, false),
                }
                Ok(())
            }
            _ => Err(protocol("unexpected message from the new laptop")),
        }
    }

    /// `lane` dropped: what it sent and was not confirmed goes back to be sent on the others.
    fn lane_lost(&mut self, lane: usize) {
        let lane_no = u32::try_from(lane).unwrap_or(u32::MAX);
        for (_, _, bytes) in self.lanes[lane].drain(..) {
            self.in_flight -= bytes;
        }
        for state in &mut self.states {
            if let SendState::Open { sections, .. } = state {
                sections.lane_lost(lane_no);
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
        for (job, state) in self.states.iter_mut().enumerate() {
            match state {
                SendState::Waiting => *state = SendState::Done(SendOutcome::Failed(STOPPED.into())),
                SendState::Open {
                    announced: true,
                    reader,
                    ..
                } => {
                    self.ends.push(Message::EndFile {
                        stream: self.streams[job],
                        stamp_after: reader.header().stamp,
                        changed: true,
                    });
                    *state = SendState::Ended {
                        retry: false,
                        failure: Some(STOPPED.into()),
                        later: false,
                    };
                }
                SendState::Open { .. } => {
                    *state = SendState::Done(SendOutcome::Failed(STOPPED.into()));
                }
                _ => {}
            }
        }
    }

    fn done(&mut self, job: usize, ok: bool) {
        let item = self.jobs[job].item;
        let tier = self.jobs[job].tier;
        let state = std::mem::replace(&mut self.states[job], SendState::Waiting);
        self.states[job] = match (state, ok) {
            (SendState::Ended { later: true, .. }, _) => {
                // It could not be read: one more try after everything else.
                self.later.push(job);
                SendState::Waiting
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
                SendState::Done(SendOutcome::Failed(
                    "the new laptop did not accept it".into(),
                ))
            }
        };
    }
}

/// One extra lane of the old laptop: asks the shared plan for blocks and sends them, reads
/// receipts only when they are owed, and otherwise sleeps until another lane changes something.
/// If it drops or is answered out of turn, its unconfirmed blocks go back to the others.
async fn send_lane<C: Channel>(
    state: &RefCell<&mut SenderSession>,
    changed: &Notify,
    alive: &Cell<usize>,
    lane: usize,
    mut ch: C,
) {
    loop {
        let wake = changed.notified();
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
                        changed.notify_waiters();
                        answered
                    }
                    Err(_) => false,
                }
            }
            Work::Wait => {
                wake.await;
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
    for (job, source, theirs) in checks {
        let mine = off_the_loop(move || hash_file(&source)).await;
        state.borrow_mut().checked(job, mine == Some(theirs));
    }
}

/// Runs blocking file work on the blocking-thread pool; `None` if it failed.
async fn off_the_loop(
    work: impl FnOnce() -> std::io::Result<[u8; 32]> + Send + 'static,
) -> Option<[u8; 32]> {
    tokio::task::spawn_blocking(work).await.ok()?.ok()
}

/// The BLAKE3 fingerprint of a whole file.
fn hash_file(path: &std::path::Path) -> std::io::Result<[u8; 32]> {
    let mut file = std::fs::File::open(path)?;
    hash_reader(&mut file)
}

fn hash_reader(reader: &mut impl std::io::Read) -> std::io::Result<[u8; 32]> {
    let mut hasher = blake3::Hasher::new();
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
    /// An identical file was already there (its stored path); nothing was copied.
    AlreadyThere(String),
    Failed(String),
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
    failure: Option<String>,
    destination: String,
    size: u64,
    /// The stored path of a same-size file already there, offered as possibly identical.
    same: Option<String>,
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
    /// Why a file's start was refused, kept per file (bounded).
    refused: BTreeMap<ItemId, String>,
    /// A same-size file already here, to fingerprint before answering a start (off the loop).
    to_fingerprint: Option<(u32, std::fs::File)>,
    streams: BTreeMap<u32, Incoming<'d>>,
    /// Files picked up again after the app restarted, waiting for the old laptop to continue them.
    restored: BTreeMap<ItemId, Incoming<'d>>,
    done: BTreeMap<u32, (ItemId, ReceiveOutcome)>,
    continued: u64,
    /// For each finished file: its destination and size, to check again after copying.
    landed: BTreeMap<u32, (String, u64)>,
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
        Self {
            table,
            journal,
            account: account.to_string(),
            allowance,
            written: Arc::new(AtomicU64::new(0)),
            refused: BTreeMap::new(),
            to_fingerprint: None,
            streams: BTreeMap::new(),
            restored: BTreeMap::new(),
            done: BTreeMap::new(),
            continued: 0,
            landed: BTreeMap::new(),
        }
    }

    /// A counter of bytes of blocks written, to read while the move runs (the lane driver's speed).
    pub fn meter(&self) -> Arc<AtomicU64> {
        self.written.clone()
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
            if let Err(refusal) = self.allowance.admit(w.item, w.size) {
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
        for s in self.streams.values_mut().chain(self.restored.values_mut()) {
            if let Some(assembly) = s.assembly.take() {
                drop(assembly);
                let _ = journal.failed(s.entry, why);
            }
        }
    }

    pub fn checkpoint(&mut self) {
        let journal = self.journal;
        for s in self.streams.values_mut().chain(self.restored.values_mut()) {
            flush_checkpoint(journal, s, true);
        }
    }

    pub fn outcome(&self, item: ItemId) -> Option<ReceiveOutcome> {
        self.done
            .values()
            .find(|(i, _)| *i == item)
            .map(|(_, o)| o.clone())
            .or_else(|| {
                self.refused
                    .get(&item)
                    .map(|why| ReceiveOutcome::Failed(why.clone()))
            })
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
        let state = RefCell::new(self);
        // Say what is already here, so the old laptop continues from there.
        let hello = state.borrow_mut().hello();
        for m in &hello {
            send(main, m).await?;
        }
        let main_loop = async {
            // A block cut off by a drop is sent again whole: pieces start afresh each run.
            let mut slot: Slot = None;
            loop {
                let m = recv(main).await?;
                // Never held across a wait, so every lane can take its turn.
                let (replies, finished) = state.borrow_mut().handle(m, &mut slot)?;
                for r in &replies {
                    send(main, r).await?;
                }
                // A start waiting on a same-size file's fingerprint: worked out on the
                // blocking-thread pool, so the lanes keep going meanwhile.
                let waiting = state.borrow_mut().to_fingerprint.take();
                if let Some((stream, mut file)) = waiting {
                    let same_size = off_the_loop(move || hash_reader(&mut file)).await;
                    send(main, &Message::Have { stream, same_size }).await?;
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
                self.allowance.ended(old.item, false);
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
        for (stream, (_, outcome)) in &self.done {
            list.push(Message::FileDone {
                stream: *stream,
                ok: matches!(
                    outcome,
                    ReceiveOutcome::Finished(_) | ReceiveOutcome::AlreadyThere(_)
                ),
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
                self.done.remove(&stream);
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
                            resumed_done <= a.blocks_done() && *a.header() == header
                        })
                    });
                if restored && let Some(s) = self.restored.remove(&item) {
                    if let Some(old) = self.streams.remove(&stream) {
                        self.allowance.ended(old.item, false);
                        drop(old.assembly);
                        let _ = self.journal.failed(old.entry, "it was started again");
                    }
                    self.streams.insert(stream, s);
                }
                if continuing || restored {
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
                    self.allowance.ended(old.item, false);
                    // Its partial file is removed with it (already failed if it had failed).
                    drop(old.assembly);
                    let _ = self.journal.failed(old.entry, "it was started again");
                }
                let block_size = header.block_size;
                let size = header.size;
                // The plan is checked before anything is read, created or reserved here.
                let admitted = if resumed_done > 0 {
                    Err("the new laptop has no place to continue from".to_string())
                } else if self.streams.len() + self.restored.len() >= MAX_OPEN_FILES {
                    Err("the old laptop started too many files at once".to_string())
                } else {
                    self.allowance.admit(item, size).map_err(|r| r.to_string())
                };
                if let Err(why) = admitted {
                    // Kept per file, not per start, so refusals cannot pile up.
                    self.refuse(item, why);
                    replies.push(Message::FileDone { stream, ok: false });
                    return Ok((replies, false));
                }
                let same = self.same_file(&destination, &path, size);
                let stored = same.as_ref().map(|(stored, _)| stored.clone());
                let (assembly, entry) = match self.start(item, &destination, &path, header) {
                    Ok(started) => started,
                    Err(Start::Record(e)) => return Err(record(e)),
                    Err(Start::Refused(why)) => {
                        self.allowance.ended(item, false);
                        self.done
                            .insert(stream, (item, ReceiveOutcome::Failed(why)));
                        replies.push(Message::FileDone { stream, ok: false });
                        return Ok((replies, false));
                    }
                };
                // Every start is answered: nothing like it is here, or (once its fingerprint is
                // worked out off the main loop) the fingerprint of the same-size file that is.
                match same {
                    Some((_, file)) => self.to_fingerprint = Some((stream, file)),
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
                    },
                );
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
                let Some(s) = self.streams.remove(&stream) else {
                    replies.push(Message::FileDone { stream, ok: false });
                    return Ok((replies, false));
                };
                let trailer = Trailer::new(stamp_after, changed);
                let outcome = match (s.assembly, s.failure) {
                    (Some(a), None) => {
                        let dest = self
                            .table
                            .get(&s.destination)
                            .map_err(|e| protocol(&e.to_string()))?;
                        land(self.journal, dest, a, trailer, s.entry)?
                    }
                    (_, Some(why)) => {
                        // Recorded when it failed; recorded now if that did not get through.
                        let _ = self.journal.failed(s.entry, &why);
                        ReceiveOutcome::Failed(why)
                    }
                    (None, None) => {
                        let why = "no file open".to_string();
                        let _ = self.journal.failed(s.entry, &why);
                        ReceiveOutcome::Failed(why)
                    }
                };
                let ok = matches!(outcome, ReceiveOutcome::Finished(_));
                self.allowance.ended(s.item, ok);
                if ok {
                    self.landed.insert(stream, (s.destination.clone(), s.size));
                }
                self.done.insert(stream, (s.item, outcome));
                replies.push(Message::FileDone { stream, ok });
            }
            Message::Skip { stream } => {
                // Identical to what is here: drop the partial copy and keep the original.
                if let Some(s) = self.streams.remove(&stream) {
                    // Its partial copy is removed first, then recorded.
                    drop(s.assembly);
                    let outcome = match s.same {
                        Some(stored) => {
                            self.journal.existing(s.entry, &stored).map_err(record)?;
                            ReceiveOutcome::AlreadyThere(stored)
                        }
                        None => {
                            let why = "skipped".to_string();
                            self.journal.failed(s.entry, &why).map_err(record)?;
                            ReceiveOutcome::Failed(why)
                        }
                    };
                    // Already there: spent, like a file that landed.
                    self.allowance
                        .ended(s.item, matches!(outcome, ReceiveOutcome::AlreadyThere(_)));
                    self.done.insert(stream, (s.item, outcome));
                }
            }
            Message::AllSent => return Ok((replies, true)),
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
                    self.fail_stream(stream, "a block grew past its size".into());
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
            Some(block) => Message::Receipt { stream, block },
            None => Message::Refused { stream },
        }))
    }

    /// Checks and writes one whole block of an open file; returns its number, or `None` if it was
    /// refused (then the file fails and its partial copy is removed).
    fn write_block(&mut self, stream: u32, whole: &[u8]) -> Option<u64> {
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
            let receipt = assembly.accept(b)?;
            Ok((receipt, fresh.then_some(hash)))
        });
        match accepted {
            Ok((receipt, fresh)) => {
                self.written
                    .fetch_add(whole.len() as u64, Ordering::Relaxed);
                if let Some(hash) = fresh {
                    s.pending.push((receipt.block, hash));
                    if s.pending.len() >= CHECKPOINT_BLOCKS {
                        flush_checkpoint(journal, s, false);
                    }
                }
                Some(receipt.block)
            }
            Err(e) => {
                self.fail_stream(stream, e.to_string());
                None
            }
        }
    }

    /// The file on `stream` failed: its partial copy is removed (it ends when the old laptop ends
    /// it, and reports why).
    fn fail_stream(&mut self, stream: u32, why: String) {
        if let Some(s) = self.streams.get_mut(&stream) {
            s.assembly = None;
            // If this does not get through, recovery finds its partial file gone and records it.
            let _ = self.journal.failed(s.entry, &why);
            s.failure = Some(why);
        }
    }

    /// Records why a start was refused, once per file (and for a bounded number of files not in
    /// the plan), so refusals cannot pile up.
    fn refuse(&mut self, item: ItemId, why: String) {
        if self.refused.len() < MAX_REMEMBERED_REFUSALS || self.refused.contains_key(&item) {
            self.refused.insert(item, why);
        }
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
        for (stream, (item, outcome)) in &mut self.done {
            let ReceiveOutcome::Finished(finished) = outcome else {
                continue;
            };
            let Some((destination, size)) = self.landed.get(stream) else {
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
                *outcome = ReceiveOutcome::Failed(
                    "it was removed or changed soon after copying, often by security software"
                        .into(),
                );
                gone.push(*item);
            }
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
        let refused = |e: &dyn std::fmt::Display| Start::Refused(e.to_string());
        let table: &'d Destinations = self.table;
        let dest = table.get(destination).map_err(|e| refused(&e))?;
        let place = table.place(destination).map_err(|e| refused(&e))?;
        let parsed = IncomingPath::parse(path).map_err(|e| refused(&e))?;
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
            source_file: None,
            partial_keep: self.allowance.partial_keep(),
        };
        let entry = self.journal.plan(&write).map_err(Start::Record)?;
        let assembly =
            match Assembly::start_tagged(dest, &parsed, header, &self.journal.temp_tag(entry)) {
                Ok(a) => a,
                Err(e) => {
                    let why = e.to_string();
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
    Refused(String),
    Record(JournalError),
}

fn record(e: JournalError) -> TransferError {
    TransferError::Record(e.to_string())
}

fn file_id(id: pctwin_gate::FileId) -> FileId {
    FileId {
        volume: id.volume,
        index: id.index,
    }
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
    let fail = |why: String| -> Result<ReceiveOutcome, TransferError> {
        journal.failed(entry, &why).map_err(record)?;
        Ok(ReceiveOutcome::Failed(why))
    };
    // Checked and on disk: from here only success or recovery ends it, never a throw-away. A
    // file that cannot be named now is kept for recovery to finish at the next start.
    let later = |sealed: pctwin_gate::Sealed<'_>, why: &dyn std::fmt::Display| {
        sealed.persist();
        Ok(ReceiveOutcome::Failed(format!("{why}; {FINISHED_LATER}")))
    };
    let (mut sealed, fingerprint) = match assembly.seal(trailer) {
        Ok(sealed) => sealed,
        Err(e) => return fail(e.to_string()),
    };
    // Which file it is, so after a crash only this very file is ever taken as it.
    let sealed_as = sealed.identity().ok().map(file_id);
    // If the journal cannot take it, the checked file is still kept (never thrown away), for
    // recovery to finish once the journal works again.
    if let Err(e) = journal.verified(entry, fingerprint, sealed_as) {
        sealed.persist();
        return Err(record(e));
    }
    let mut tries = 0;
    let claimed = loop {
        tries += 1;
        if tries > MAX_NAME_TRIES {
            return later(sealed, &pctwin_gate::GateError::TooManyClashes);
        }
        let name = match sealed.next_name() {
            Ok(name) => name,
            Err(e) => return later(sealed, &e),
        };
        if let Err(e) = journal.applied(entry, &name) {
            sealed.persist();
            return Err(record(e));
        }
        match sealed.claim_as(&name) {
            Claim::Named(c) => break c,
            // Another program took the name first: another name, recorded first again.
            Claim::Taken(back) => sealed = back,
            Claim::Failed(e, back) => return later(back, &e),
        }
    };
    // Committed while the temporary name still holds the file, then the temporary name goes.
    match dest.stat(&claimed.finished().final_path) {
        Ok(Some(stat)) => {
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
            Ok(ReceiveOutcome::Finished(claimed.keep()))
        }
        // Gone the moment it got its name: never reported copied.
        Ok(None) => fail(REMOVED_AT_ONCE.into()),
        // Could not look just now: recovery finishes it (the journal has its name).
        Err(e) => Ok(ReceiveOutcome::Failed(format!("{e}; {FINISHED_LATER}"))),
    }
}
