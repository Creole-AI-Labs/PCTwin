use std::cell::{Cell, RefCell};
use std::collections::{BTreeMap, HashMap, HashSet, VecDeque};
use std::future::Future;
use std::path::PathBuf;
use std::sync::Arc;

use pctwin_gate::{Destinations, Finished, IncomingPath};
use pctwin_record::ItemId;

use crate::message::{Message, PieceBuffer, split_into_pieces};
use crate::queue::{Scheduler, Tier};
use crate::reading::{ReadBudget, is_drive_error};
use crate::sections::FileSections;
use crate::{
    Allowance, Assembly, Block, FileSender, FsOpener, Opener, ResumeTicket, Trailer, TransferError,
};
use pctwin_scan::ReadPlan;
use tokio::sync::Notify;

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
        self.run_lanes(ch, &mut [] as &mut [C]).await
    }

    /// Sends everything not yet done over the main connection `main` and any extra `lanes` to
    /// the same new laptop. Starts, ends and skips go on the main connection; blocks go on every
    /// lane, a big file's sections over several at once, most important files first. A lane that
    /// drops gives its unconfirmed blocks back to be sent on the others; if the main connection
    /// drops, call again with a new one to continue.
    pub async fn run_lanes<C: Channel>(
        &mut self,
        main: &mut C,
        lanes: &mut [C],
    ) -> Result<(), TransferError> {
        self.resume(main).await?;
        self.lanes = (0..=lanes.len()).map(|_| VecDeque::new()).collect();
        self.in_flight = 0;
        let state = RefCell::new(self);
        let changed = Notify::new();
        // Extra lanes still running.
        let alive = Cell::new(lanes.len());
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
        let extra = futures_util::future::join_all(lanes.iter_mut().enumerate().map(|(n, ch)| {
            let (state, changed, alive) = (&state, &changed, &alive);
            let lane = n + 1;
            async move {
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
                                if send(ch, &piece).await.is_err() {
                                    sent = false;
                                    break;
                                }
                            }
                            sent
                        }
                        Work::Wait if !state.borrow().lanes[lane].is_empty() => {
                            // Receipts are owed on this lane, so waiting for one cannot hang.
                            match recv(ch).await {
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
                        // Dropped, or answered out of turn: this lane is closed and its
                        // unconfirmed blocks go to the others.
                        state.borrow_mut().lane_lost(lane);
                        alive.set(alive.get() - 1);
                        changed.notify_waiters();
                        return;
                    }
                }
            }
        }));
        tokio::pin!(main_loop);
        tokio::pin!(extra);
        tokio::select! {
            done = &mut main_loop => done,
            _ = &mut extra => main_loop.await,
        }
    }

    /// Reads what the new laptop already has and picks each file up from there.
    async fn resume(&mut self, ch: &mut impl Channel) -> Result<(), TransferError> {
        let mut tickets: BTreeMap<usize, ResumeTicket> = BTreeMap::new();
        loop {
            match recv(ch).await? {
                Message::ResumeFrom { stream, ticket } => {
                    if let Some(&job) = self.job_of.get(&stream) {
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

    /// Whether ends or skips are waiting to go on the main connection.
    fn has_control(&self) -> bool {
        !self.ends.is_empty() || !self.skips.is_empty()
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
                let source = self.jobs[job].source.clone();
                if let SendState::Open { cleared, .. } = &mut self.states[job] {
                    let identical = same_size
                        .is_some_and(|theirs| hash_file(&source).is_ok_and(|mine| mine == theirs));
                    if identical {
                        self.skips.push(job);
                    } else {
                        *cleared = true;
                    }
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

/// The section plan for a file, with blocks already done marked; `None` for a file with more
/// blocks than a plan can count (far past any real file).
fn sections_for(header: &crate::Header, done: &[bool]) -> Option<FileSections> {
    let blocks = u32::try_from(header.block_count).ok()?;
    Some(FileSections::new(blocks, header.block_size, done))
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
    assembly: Option<Assembly<'d>>,
    block_size: u64,
    failure: Option<String>,
    destination: String,
    size: u64,
    /// The stored path of a same-size file already there, offered as possibly identical.
    same: Option<String>,
}

/// The new laptop's side of a move. A file starts only if the approved plan allows it (see
/// [`Allowance`]); every file lands through the safety gate in one of the approved destinations. Keeps what it has across dropped connections: call [`run`](Self::run)
/// again with a new connection to continue.
pub struct ReceiverSession<'d> {
    table: &'d Destinations,
    allowance: Allowance,
    streams: BTreeMap<u32, Incoming<'d>>,
    done: BTreeMap<u32, (ItemId, ReceiveOutcome)>,
    continued: u64,
    /// For each finished file: its destination and size, to check again after copying.
    landed: BTreeMap<u32, (String, u64)>,
}

impl<'d> ReceiverSession<'d> {
    /// Receives into the places in `table`, only what `allowance` (the approved plan) allows.
    pub fn new(table: &'d Destinations, allowance: Allowance) -> Self {
        Self {
            table,
            allowance,
            streams: BTreeMap::new(),
            done: BTreeMap::new(),
            continued: 0,
            landed: BTreeMap::new(),
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
            .filter(|s| {
                s.failure.is_none() && s.assembly.as_ref().is_some_and(|a| a.blocks_done() > 0)
            })
            .count() as u64
    }

    pub fn outcome(&self, item: ItemId) -> Option<&ReceiveOutcome> {
        self.done.values().find(|(i, _)| *i == item).map(|(_, o)| o)
    }

    /// Receives over `ch` until the old laptop has sent everything, or the connection drops (call
    /// again with a new connection to continue).
    pub async fn run<C: Channel>(&mut self, ch: &mut C) -> Result<(), TransferError> {
        self.run_lanes(ch, &mut [] as &mut [C]).await
    }

    /// Receives over the main connection `main` and any extra `lanes` to the same old laptop.
    /// Files are started, ended and skipped only on the main connection; extra lanes carry only
    /// blocks (as pieces) and their receipts, and each lane rejoins its own pieces. A lane that
    /// drops or sends anything but pieces is closed and the move carries on over the others; if
    /// the main connection drops, call again with a new one to continue.
    pub async fn run_lanes<C: Channel>(
        &mut self,
        main: &mut C,
        lanes: &mut [C],
    ) -> Result<(), TransferError> {
        let state = RefCell::new(self);
        // Say what is already here, so the old laptop continues from there.
        let hello = state.borrow_mut().hello();
        for m in &hello {
            send(main, m).await?;
        }
        let main_loop = async {
            // A block cut off by a drop is sent again whole: pieces start afresh each run.
            let mut pieces: HashMap<u32, PieceBuffer> = HashMap::new();
            loop {
                let m = recv(main).await?;
                // Never held across a wait, so every lane can take its turn.
                let (replies, finished) = state.borrow_mut().handle(m, &mut pieces)?;
                for r in &replies {
                    send(main, r).await?;
                }
                if finished {
                    return Ok(());
                }
            }
        };
        let extra = futures_util::future::join_all(lanes.iter_mut().map(|ch| {
            let state = &state;
            async move {
                let mut pieces: HashMap<u32, PieceBuffer> = HashMap::new();
                loop {
                    let Ok(Message::Piece {
                        stream,
                        last,
                        bytes,
                    }) = recv(ch).await
                    else {
                        // Dropped, or not a piece: this lane is closed.
                        return;
                    };
                    let receipt = state.borrow_mut().piece(&mut pieces, stream, last, &bytes);
                    if let Some(r) = receipt
                        && send(ch, &r).await.is_err()
                    {
                        return;
                    }
                }
            }
        }));
        tokio::pin!(main_loop);
        tokio::pin!(extra);
        tokio::select! {
            done = &mut main_loop => done,
            // Every extra lane closed: carry on over the main connection alone.
            _ = &mut extra => main_loop.await,
        }
    }

    /// What this side already has, then `Ready`.
    fn hello(&mut self) -> Vec<Message> {
        self.streams
            .retain(|_, s| s.assembly.is_some() && s.failure.is_none());
        let mut list = Vec::new();
        for (stream, s) in &self.streams {
            if let Some(a) = &s.assembly {
                list.push(Message::ResumeFrom {
                    stream: *stream,
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
        pieces: &mut HashMap<u32, PieceBuffer>,
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
                if continuing {
                    self.continued += 1;
                    replies.push(Message::Have {
                        stream,
                        same_size: None,
                    });
                    return Ok((replies, false));
                }
                // A fresh start replaces anything kept (the partial file is removed).
                self.streams.remove(&stream);
                let block_size = header.block_size;
                let size = header.size;
                let same = self.same_file(&destination, &path, size);
                let started = if resumed_done > 0 {
                    Err("the new laptop has no place to continue from".to_string())
                } else if self.streams.len() >= MAX_OPEN_FILES {
                    Err("the old laptop started too many files at once".to_string())
                } else if let Err(refused) = self.allowance.admit(item, size) {
                    // Checked before anything is created or reserved on this laptop.
                    Err(refused.to_string())
                } else {
                    self.start(&destination, &path, header)
                };
                let (assembly, failure) = match started {
                    Ok(a) => (Some(a), None),
                    Err(why) => (None, Some(why)),
                };
                if let Some(why) = failure {
                    // Nothing is kept for a file refused at its start, so it holds no place.
                    self.done
                        .insert(stream, (item, ReceiveOutcome::Failed(why)));
                    replies.push(Message::FileDone { stream, ok: false });
                    return Ok((replies, false));
                } else {
                    // Every start is answered: here is a same-size file's fingerprint, or
                    // nothing like it is here.
                    let same_size = same.as_ref().map(|(_, hash)| *hash);
                    replies.push(Message::Have { stream, same_size });
                }
                self.streams.insert(
                    stream,
                    Incoming {
                        item,
                        assembly,
                        block_size,
                        failure,
                        destination,
                        size,
                        same: same.map(|(stored, _)| stored),
                    },
                );
            }
            Message::Piece {
                stream,
                last,
                bytes,
            } => {
                if let Some(r) = self.piece(pieces, stream, last, &bytes) {
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
                let outcome = match (s.assembly, s.failure) {
                    (Some(a), None) => match a.finish(Trailer::new(stamp_after, changed)) {
                        Ok(f) => ReceiveOutcome::Finished(f),
                        Err(e) => ReceiveOutcome::Failed(e.to_string()),
                    },
                    (_, Some(why)) => ReceiveOutcome::Failed(why),
                    (None, None) => ReceiveOutcome::Failed("no file open".into()),
                };
                let ok = matches!(outcome, ReceiveOutcome::Finished(_));
                if ok {
                    self.landed.insert(stream, (s.destination.clone(), s.size));
                }
                self.done.insert(stream, (s.item, outcome));
                replies.push(Message::FileDone { stream, ok });
            }
            Message::Skip { stream } => {
                // Identical to what is here: drop the partial copy and keep the original.
                if let Some(s) = self.streams.remove(&stream) {
                    let outcome = match s.same {
                        Some(stored) => ReceiveOutcome::AlreadyThere(stored),
                        None => ReceiveOutcome::Failed("skipped".into()),
                    };
                    self.done.insert(stream, (s.item, outcome));
                }
            }
            Message::AllSent => return Ok((replies, true)),
            _ => return Err(protocol("unexpected message from the old laptop")),
        }
        Ok((replies, false))
    }

    /// Adds a piece that came on one lane (with that lane's own `pieces`); when it ends a block,
    /// checks and writes the block and returns the receipt naming it. Every block's last piece is
    /// answered, even for a file that failed or was never started.
    fn piece(
        &mut self,
        pieces: &mut HashMap<u32, PieceBuffer>,
        stream: u32,
        last: bool,
        bytes: &[u8],
    ) -> Option<Message> {
        let buffer = pieces.entry(stream).or_default();
        let whole = buffer.add(bytes, last);
        let mut written = 0;
        if let Some(s) = self.streams.get_mut(&stream) {
            match whole {
                Ok(Some(whole)) if s.failure.is_none() => {
                    let accepted = Block::decode(&whole, s.block_size).and_then(|b| {
                        s.assembly
                            .as_mut()
                            .ok_or_else(|| protocol("no file open"))?
                            .accept(b)
                    });
                    match accepted {
                        Ok(receipt) => written = receipt.block,
                        Err(e) => {
                            s.failure = Some(e.to_string());
                            // Dropping the assembly removes the partial file.
                            s.assembly = None;
                        }
                    }
                }
                Ok(_) => {}
                Err(e) => {
                    s.failure = Some(e.to_string());
                    s.assembly = None;
                }
            }
        }
        last.then_some(Message::Receipt {
            stream,
            block: written,
        })
    }

    /// A file already at the place `path` would land in, of exactly `size` bytes: its stored path
    /// and fingerprint.
    fn same_file(&self, destination: &str, path: &str, size: u64) -> Option<(String, [u8; 32])> {
        let dest = self.table.get(destination).ok()?;
        let path = IncomingPath::parse(path).ok()?;
        let (stored, len) = dest.find(&path)?;
        if len != size {
            return None;
        }
        let hash = hash_reader(&mut dest.open_read(&stored).ok()?).ok()?;
        Some((stored, hash))
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

    fn start(
        &self,
        destination: &str,
        path: &str,
        header: crate::Header,
    ) -> Result<Assembly<'d>, String> {
        let table: &'d Destinations = self.table;
        let dest = table.get(destination).map_err(|e| e.to_string())?;
        let path = IncomingPath::parse(path).map_err(|e| e.to_string())?;
        Assembly::start(dest, &path, header).map_err(|e| e.to_string())
    }
}
