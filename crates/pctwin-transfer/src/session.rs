use std::collections::{BTreeMap, HashMap, HashSet, VecDeque};
use std::future::Future;
use std::path::PathBuf;
use std::sync::Arc;

use pctwin_gate::{Destinations, Finished, IncomingPath};
use pctwin_record::ItemId;

use crate::message::{Message, PieceBuffer, split_into_pieces};
use crate::queue::{Scheduler, Tier};
use crate::reading::{ReadBudget, is_drive_error};
use crate::{Assembly, Block, FileSender, FsOpener, Opener, ResumeTicket, Trailer, TransferError};
use pctwin_scan::ReadPlan;

/// Why the files not yet read were left: reading stopped to protect a failing drive.
const STOPPED: &str =
    "not read: the old drive kept failing, so PCTwin stopped reading it to protect it";

/// How much may be sent before the receiver confirms it (as Syncthing does): enough to keep the
/// connection busy, small enough that memory stays bounded.
const IN_FLIGHT_BYTES: u64 = 32 * 1024 * 1024;

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
        announced: bool,
        from_block: u64,
        /// The new laptop answered the start (nothing identical there), so blocks may go.
        cleared: bool,
    },
    /// The last block was sent; waiting for the new laptop's answer.
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

/// The old laptop's side of a move. Keeps its place across dropped connections: call
/// [`run`](Self::run) again with a new connection to continue.
pub struct SenderSession {
    jobs: Vec<SendJob>,
    states: Vec<SendState>,
    by_item: HashMap<ItemId, u32>,
    scheduler: Scheduler,
    capacity: usize,
    blocks_sent: u64,
    in_flight_limit: u64,
    /// Files found identical on the new laptop, to tell it to skip.
    skips: Vec<u32>,
    /// Files half-sent when reading stopped, to tell the new laptop to drop.
    aborts: Vec<u32>,
    opener: Arc<dyn Opener>,
    budget: ReadBudget,
    /// Files that could not be read, waiting for one more try after everything else.
    later: Vec<u32>,
    /// Files already given their second try.
    retried: HashSet<u32>,
}

impl SenderSession {
    /// `jobs` in the order planned; `capacity` files take turns at once.
    pub fn new(jobs: Vec<SendJob>, capacity: usize) -> Self {
        let mut scheduler = Scheduler::new(capacity);
        let mut by_item = HashMap::new();
        for (i, job) in jobs.iter().enumerate() {
            scheduler.push(job.item, job.tier);
            by_item.insert(job.item, u32::try_from(i).unwrap_or(u32::MAX));
        }
        let states = jobs.iter().map(|_| SendState::Waiting).collect();
        Self {
            jobs,
            states,
            by_item,
            scheduler,
            capacity,
            blocks_sent: 0,
            in_flight_limit: IN_FLIGHT_BYTES,
            skips: Vec::new(),
            aborts: Vec::new(),
            opener: Arc::new(FsOpener),
            budget: ReadBudget::for_plan(ReadPlan::Normal),
            later: Vec::new(),
            retried: HashSet::new(),
        }
    }

    /// Sends at most `bytes` before the new laptop confirms them (32 MiB unless set).
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

    /// Blocks sent so far, over every connection.
    pub fn blocks_sent(&self) -> u64 {
        self.blocks_sent
    }

    pub fn outcome(&self, item: ItemId) -> Option<&SendOutcome> {
        let i = *self.by_item.get(&item)? as usize;
        match &self.states[i] {
            SendState::Done(o) => Some(o),
            _ => None,
        }
    }

    /// The person asked for these to move first, during the move.
    pub fn ask_first(&mut self, items: &[ItemId]) {
        self.scheduler.ask_first(items);
    }

    /// Sends everything not yet done over `ch`. Returns when every file is answered for, or with
    /// [`TransferError::ConnectionDropped`] (call again with a new connection to continue).
    pub async fn run(&mut self, ch: &mut impl Channel) -> Result<(), TransferError> {
        self.resume(ch).await?;
        let mut in_flight: VecDeque<u64> = VecDeque::new();
        loop {
            for stream in std::mem::take(&mut self.skips) {
                send(ch, &Message::Skip { stream }).await?;
                let i = stream as usize;
                self.scheduler.finished(self.jobs[i].item);
                self.states[i] = SendState::Done(SendOutcome::AlreadyThere);
            }
            for stream in std::mem::take(&mut self.aborts) {
                let i = stream as usize;
                if let SendState::Open { reader, .. } = &self.states[i] {
                    let stamp_after = reader.header().stamp;
                    send(
                        ch,
                        &Message::EndFile {
                            stream,
                            stamp_after,
                            changed: true,
                        },
                    )
                    .await?;
                    self.states[i] = SendState::Ended {
                        retry: false,
                        failure: Some(STOPPED.into()),
                        later: false,
                    };
                }
            }
            if in_flight.iter().sum::<u64>() >= self.in_flight_limit {
                let m = recv(ch).await?;
                self.answer(m, &mut in_flight)?;
                continue;
            }
            match self.scheduler.next_turn() {
                Some(item) => {
                    let stream = self.by_item[&item];
                    if !self.step(stream, ch, &mut in_flight).await? {
                        // This file waits for the new laptop's answer; listen for it.
                        let m = recv(ch).await?;
                        self.answer(m, &mut in_flight)?;
                    }
                }
                None => {
                    let waiting = self
                        .states
                        .iter()
                        .any(|s| matches!(s, SendState::Ended { .. }));
                    if waiting || !in_flight.is_empty() {
                        let m = recv(ch).await?;
                        self.answer(m, &mut in_flight)?;
                    } else if !self.later.is_empty() && !self.budget.stopped() {
                        // Everything readable is done: one more try for what could not be read.
                        for stream in std::mem::take(&mut self.later) {
                            self.retried.insert(stream);
                            let job = &self.jobs[stream as usize];
                            self.scheduler.push(job.item, job.tier);
                        }
                    } else {
                        break;
                    }
                }
            }
        }
        send(ch, &Message::AllSent).await
    }

    /// Reads what the new laptop already has and picks each file up from there.
    async fn resume(&mut self, ch: &mut impl Channel) -> Result<(), TransferError> {
        let mut tickets: BTreeMap<u32, ResumeTicket> = BTreeMap::new();
        loop {
            match recv(ch).await? {
                Message::ResumeFrom { stream, ticket } => {
                    tickets.insert(stream, ticket);
                }
                Message::FileDone { stream, ok } => self.done(stream, ok),
                Message::Ready => break,
                _ => return Err(protocol("expected the new laptop's resume list")),
            }
        }
        self.scheduler = Scheduler::new(self.capacity);
        for (i, job) in self.jobs.iter().enumerate() {
            let stream = u32::try_from(i).unwrap_or(u32::MAX);
            if matches!(self.states[i], SendState::Done(_)) {
                continue;
            }
            if self.later.contains(&stream) {
                // Still waiting for its second try at the end.
                continue;
            }
            let opener = self.opener.clone();
            let picked_up = tickets.get(&stream).and_then(|ticket| {
                FileSender::open_with(&*opener, &job.source, Some(ticket), job.compressible)
                    .ok()
                    .map(|reader| SendState::Open {
                        reader,
                        announced: false,
                        from_block: ticket.next_block,
                        cleared: false,
                    })
            });
            // Without a ticket, or if the file changed since, it starts again from the beginning.
            self.states[i] = picked_up.unwrap_or(SendState::Waiting);
            self.scheduler.push(job.item, job.tier);
        }
        Ok(())
    }

    /// Sends this file's next message. Returns false when it is waiting for the new laptop's
    /// answer to its start.
    async fn step(
        &mut self,
        stream: u32,
        ch: &mut impl Channel,
        in_flight: &mut VecDeque<u64>,
    ) -> Result<bool, TransferError> {
        let i = stream as usize;
        let job = self.jobs[i].clone();
        if matches!(self.states[i], SendState::Waiting) {
            match FileSender::open_with(&*self.opener, &job.source, None, job.compressible) {
                Ok(reader) => {
                    self.states[i] = SendState::Open {
                        reader,
                        announced: false,
                        from_block: 0,
                        cleared: false,
                    }
                }
                Err(e) => {
                    let drive = matches!(&e, TransferError::Io(io) if is_drive_error(io));
                    self.scheduler.finished(job.item);
                    self.states[i] = if drive && self.read_failed(stream) {
                        // One more try after everything else.
                        self.later.push(stream);
                        SendState::Waiting
                    } else {
                        SendState::Done(SendOutcome::Failed(e.to_string()))
                    };
                    return Ok(true);
                }
            }
        }
        let SendState::Open {
            reader,
            announced,
            from_block,
            cleared,
        } = &mut self.states[i]
        else {
            self.scheduler.finished(job.item);
            return Ok(true);
        };
        if !*announced {
            send(
                ch,
                &Message::StartFile {
                    stream,
                    item: job.item,
                    destination: job.destination.clone(),
                    path: job.path.clone(),
                    header: reader.header().clone(),
                    from_block: *from_block,
                },
            )
            .await?;
            *announced = true;
            return Ok(true);
        }
        if !*cleared {
            return Ok(false);
        }
        let stamp = reader.header().stamp;
        match reader.next_block() {
            Ok(Some(block)) => {
                self.budget.read_ok();
                let wire = block.encode();
                for piece in split_into_pieces(stream, &wire) {
                    send(ch, &piece).await?;
                }
                self.blocks_sent += 1;
                in_flight.push_back(wire.len() as u64);
                Ok(true)
            }
            Ok(None) => {
                let SendState::Open { reader, .. } =
                    std::mem::replace(&mut self.states[i], SendState::Waiting)
                else {
                    return Ok(true);
                };
                let trailer = reader.finish()?;
                send(
                    ch,
                    &Message::EndFile {
                        stream,
                        stamp_after: trailer.stamp_after,
                        changed: trailer.changed_while_read(),
                    },
                )
                .await?;
                // On a weak drive each file is read only once: a file that changed is not read again.
                let changed = trailer.changed_while_read();
                let again = changed && self.budget.try_again_later();
                self.states[i] = SendState::Ended {
                    retry: again,
                    failure: (changed && !again).then(|| "it changed while being read".to_string()),
                    later: false,
                };
                self.scheduler.finished(job.item);
                Ok(true)
            }
            Err(e) => {
                // Tell the new laptop to drop what it has of this file.
                let drive = matches!(&e, TransferError::Io(io) if is_drive_error(io));
                let retry =
                    matches!(e, TransferError::ChangedWhileRead) && self.budget.try_again_later();
                send(
                    ch,
                    &Message::EndFile {
                        stream,
                        stamp_after: stamp,
                        changed: true,
                    },
                )
                .await?;
                self.scheduler.finished(job.item);
                let later = drive && self.read_failed(stream);
                self.states[i] = SendState::Ended {
                    retry,
                    failure: (!retry && !later).then(|| e.to_string()),
                    later,
                };
                Ok(true)
            }
        }
    }

    fn answer(&mut self, m: Message, in_flight: &mut VecDeque<u64>) -> Result<(), TransferError> {
        match m {
            Message::Receipt { .. } => {
                in_flight.pop_front();
                Ok(())
            }
            Message::FileDone { stream, ok } => {
                self.done(stream, ok);
                Ok(())
            }
            Message::Have { stream, same_size } => {
                let i = stream as usize;
                let source = self.jobs.get(i).map(|j| j.source.clone());
                if let (Some(SendState::Open { cleared, .. }), Some(source)) =
                    (self.states.get_mut(i), source)
                {
                    let identical = same_size
                        .is_some_and(|theirs| hash_file(&source).is_ok_and(|mine| mine == theirs));
                    if identical {
                        self.skips.push(stream);
                    } else {
                        *cleared = true;
                    }
                }
                Ok(())
            }
            _ => Err(protocol("unexpected message from the new laptop")),
        }
    }

    /// Counts a drive read error for `stream`. Returns true when the file gets one more try after
    /// everything else; when errors come in a row, stops reading altogether.
    fn read_failed(&mut self, stream: u32) -> bool {
        if self.budget.read_failed() {
            self.stop_reading();
            return false;
        }
        self.budget.try_again_later() && !self.retried.contains(&stream)
    }

    /// The drive keeps failing: read nothing more. Files not yet read are reported as such, and
    /// files half-sent are dropped on the new laptop.
    fn stop_reading(&mut self) {
        self.scheduler = Scheduler::new(self.capacity);
        self.later.clear();
        for (i, state) in self.states.iter_mut().enumerate() {
            match state {
                SendState::Waiting => *state = SendState::Done(SendOutcome::Failed(STOPPED.into())),
                SendState::Open { announced, .. } => {
                    if *announced {
                        self.aborts.push(u32::try_from(i).unwrap_or(u32::MAX));
                    } else {
                        *state = SendState::Done(SendOutcome::Failed(STOPPED.into()));
                    }
                }
                _ => {}
            }
        }
    }

    fn done(&mut self, stream: u32, ok: bool) {
        let i = stream as usize;
        let Some(job) = self.jobs.get(i) else { return };
        let item = job.item;
        let tier = job.tier;
        let state = std::mem::replace(&mut self.states[i], SendState::Waiting);
        self.states[i] = match (state, ok) {
            (SendState::Ended { later: true, .. }, _) => {
                // It could not be read: one more try after everything else.
                self.later.push(stream);
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
    buffer: PieceBuffer,
    block_size: u64,
    failure: Option<String>,
    destination: String,
    size: u64,
    /// The stored path of a same-size file already there, offered as possibly identical.
    same: Option<String>,
}

/// The new laptop's side of a move. Every file lands through the safety gate in one of the
/// approved destinations. Keeps what it has across dropped connections: call [`run`](Self::run)
/// again with a new connection to continue.
pub struct ReceiverSession<'d> {
    table: &'d Destinations,
    streams: BTreeMap<u32, Incoming<'d>>,
    done: BTreeMap<u32, (ItemId, ReceiveOutcome)>,
    continued: u64,
    /// For each finished file: its destination and size, to check again after copying.
    landed: BTreeMap<u32, (String, u64)>,
}

impl<'d> ReceiverSession<'d> {
    pub fn new(table: &'d Destinations) -> Self {
        Self {
            table,
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
                s.failure.is_none()
                    && s.assembly
                        .as_ref()
                        .is_some_and(|a| a.resume_ticket().next_block > 0)
            })
            .count() as u64
    }

    pub fn outcome(&self, item: ItemId) -> Option<&ReceiveOutcome> {
        self.done.values().find(|(i, _)| *i == item).map(|(_, o)| o)
    }

    /// Receives over `ch` until the old laptop has sent everything, or the connection drops (call
    /// again with a new connection to continue).
    pub async fn run(&mut self, ch: &mut impl Channel) -> Result<(), TransferError> {
        // Say what is already here, so the old laptop continues from there.
        self.streams
            .retain(|_, s| s.assembly.is_some() && s.failure.is_none());
        let mut list = Vec::new();
        for (stream, s) in &mut self.streams {
            // A block cut off by the drop is sent again whole.
            s.buffer = PieceBuffer::default();
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
        for m in &list {
            send(ch, m).await?;
        }
        send(ch, &Message::Ready).await?;

        loop {
            match recv(ch).await? {
                Message::StartFile {
                    stream,
                    item,
                    destination,
                    path,
                    header,
                    from_block,
                } => {
                    self.done.remove(&stream);
                    let continuing = from_block > 0
                        && self.streams.get(&stream).is_some_and(|s| {
                            s.item == item
                                && s.assembly
                                    .as_ref()
                                    .is_some_and(|a| a.resume_ticket().next_block == from_block)
                        });
                    if continuing {
                        self.continued += 1;
                        send(
                            ch,
                            &Message::Have {
                                stream,
                                same_size: None,
                            },
                        )
                        .await?;
                        continue;
                    }
                    // A fresh start replaces anything kept (the partial file is removed).
                    self.streams.remove(&stream);
                    let block_size = header.block_size;
                    let size = header.size;
                    let same = self.same_file(&destination, &path, size);
                    let started = if from_block > 0 {
                        Err("the new laptop has no place to continue from".to_string())
                    } else {
                        self.start(&destination, &path, header)
                    };
                    let (assembly, failure) = match started {
                        Ok(a) => (Some(a), None),
                        Err(why) => (None, Some(why)),
                    };
                    if let Some(why) = &failure {
                        self.done
                            .insert(stream, (item, ReceiveOutcome::Failed(why.clone())));
                        send(ch, &Message::FileDone { stream, ok: false }).await?;
                    } else {
                        // Every start is answered: here is a same-size file's fingerprint, or
                        // nothing like it is here.
                        let same_size = same.as_ref().map(|(_, hash)| *hash);
                        send(ch, &Message::Have { stream, same_size }).await?;
                    }
                    self.streams.insert(
                        stream,
                        Incoming {
                            item,
                            assembly,
                            buffer: PieceBuffer::default(),
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
                    let Some(s) = self.streams.get_mut(&stream) else {
                        // Pieces of a file never started: nothing to keep, but still answered.
                        if last {
                            send(
                                ch,
                                &Message::Receipt {
                                    stream,
                                    next_block: 0,
                                },
                            )
                            .await?;
                        }
                        continue;
                    };
                    match s.buffer.add(&bytes, last) {
                        Ok(Some(whole)) if s.failure.is_none() => {
                            let accepted = Block::decode(&whole, s.block_size).and_then(|b| {
                                s.assembly
                                    .as_mut()
                                    .ok_or_else(|| protocol("no file open"))?
                                    .accept(b)
                            });
                            if let Err(e) = accepted {
                                s.failure = Some(e.to_string());
                                // Dropping the assembly removes the partial file.
                                s.assembly = None;
                            }
                        }
                        Ok(_) => {}
                        Err(e) => {
                            s.failure = Some(e.to_string());
                            s.assembly = None;
                        }
                    }
                    if last {
                        let next_block = s
                            .assembly
                            .as_ref()
                            .map_or(0, |a| a.resume_ticket().next_block);
                        send(ch, &Message::Receipt { stream, next_block }).await?;
                    }
                }
                Message::EndFile {
                    stream,
                    stamp_after,
                    changed,
                } => {
                    let Some(s) = self.streams.remove(&stream) else {
                        send(ch, &Message::FileDone { stream, ok: false }).await?;
                        continue;
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
                    send(ch, &Message::FileDone { stream, ok }).await?;
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
                Message::AllSent => return Ok(()),
                _ => return Err(protocol("unexpected message from the old laptop")),
            }
        }
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
