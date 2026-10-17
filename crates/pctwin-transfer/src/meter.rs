//! What a running move has measured, readable while it runs, and the switch that stops it.
//!
//! A [`MoveMeter`] is shared (cheap to clone) between a session and whoever shows the move: the
//! session counts as it works, and a reader on any thread takes a [`MeterReading`] at any time.
//! Every number in it was counted, never estimated. Block and byte counters are atomics; the
//! files in each step are kept per group (the destination each file lands in, as the approved
//! plan names it) behind a small lock taken only when a file changes step.
//!
//! A [`MoveControl`] is the person's stop. A session checks it before each block it sends and
//! ends its run with [`TransferError::Stopped`](crate::TransferError::Stopped) as soon as it is
//! set, from any thread, however long it was waiting.

use std::collections::{BTreeMap, HashMap};
use std::sync::atomic::{AtomicBool, AtomicU64, Ordering};
use std::sync::{Arc, Mutex, MutexGuard, PoisonError};

use pctwin_record::ItemId;
use tokio::sync::Notify;

/// Where one file is, as this laptop knows it.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Step {
    /// Not started.
    Waiting,
    /// Being sent (old laptop) or written (new laptop).
    Copying,
    /// Being read back and checked (new laptop), or sent whole and waiting for the new laptop to
    /// say it checked it (old laptop).
    Checking,
    /// Checked and under its real name on the new laptop (on the old laptop: the new laptop said
    /// so).
    Copied,
    /// An identical file was already on the new laptop; nothing was copied.
    AlreadyThere,
    /// It did not move. Its reason is in the session's outcome for it.
    Failed,
}

/// Files in each step. They add up to `total`.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct StepCounts {
    pub total: u64,
    pub waiting: u64,
    pub copying: u64,
    pub checking: u64,
    pub copied: u64,
    pub already_there: u64,
    pub failed: u64,
}

impl StepCounts {
    fn slot(&mut self, step: Step) -> &mut u64 {
        match step {
            Step::Waiting => &mut self.waiting,
            Step::Copying => &mut self.copying,
            Step::Checking => &mut self.checking,
            Step::Copied => &mut self.copied,
            Step::AlreadyThere => &mut self.already_there,
            Step::Failed => &mut self.failed,
        }
    }

    fn add(&mut self, other: &StepCounts) {
        self.total += other.total;
        self.waiting += other.waiting;
        self.copying += other.copying;
        self.checking += other.checking;
        self.copied += other.copied;
        self.already_there += other.already_there;
        self.failed += other.failed;
    }
}

/// Everything the meter has counted, at one moment.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct MeterReading {
    /// The whole move: every group together.
    pub files: StepCounts,
    /// Per group, by its name (the destination label of the approved plan).
    pub groups: BTreeMap<String, StepCounts>,
    /// Old laptop: blocks sent, and blocks the new laptop confirmed, over every connection and
    /// lane (a block sent again counts again).
    pub blocks_sent: u64,
    pub blocks_confirmed: u64,
    /// Old laptop: bytes of those blocks (before compression).
    pub bytes_sent: u64,
    pub bytes_confirmed: u64,
    /// New laptop: bytes of the files' blocks written so far, over every attempt (not yet
    /// checked; a block that arrives again for the same attempt counts once).
    pub bytes_written: u64,
    /// New laptop: the size of every file read back, checked and under its real name.
    pub bytes_checked: u64,
    /// New laptop: the size of every file found already there.
    pub bytes_already_there: u64,
    /// New laptop: the approved plan's size, every file at its approved size.
    pub bytes_planned: u64,
    /// New laptop: the file being read back and checked right now, if one is.
    pub checking: Option<ItemId>,
}

#[derive(Debug, Default)]
struct Files {
    groups: Vec<(String, StepCounts)>,
    by_name: HashMap<String, u32>,
    checking: Option<ItemId>,
}

#[derive(Debug, Default)]
struct Inner {
    blocks_sent: AtomicU64,
    blocks_confirmed: AtomicU64,
    bytes_sent: AtomicU64,
    bytes_confirmed: AtomicU64,
    bytes_written: AtomicU64,
    bytes_checked: AtomicU64,
    bytes_already_there: AtomicU64,
    bytes_planned: AtomicU64,
    files: Mutex<Files>,
}

/// A running move's counters (see the module docs). Give one meter to one session.
#[derive(Debug, Clone, Default)]
pub struct MoveMeter(Arc<Inner>);

/// A group's place in the meter.
pub(crate) type Group = u32;

impl MoveMeter {
    pub fn new() -> Self {
        Self::default()
    }

    /// Everything counted so far.
    pub fn reading(&self) -> MeterReading {
        let i = &self.0;
        let files = self.files();
        let mut total = StepCounts::default();
        let mut groups = BTreeMap::new();
        for (name, counts) in &files.groups {
            total.add(counts);
            groups
                .entry(name.clone())
                .or_insert_with(StepCounts::default)
                .add(counts);
        }
        MeterReading {
            files: total,
            groups,
            blocks_sent: i.blocks_sent.load(Ordering::Acquire),
            blocks_confirmed: i.blocks_confirmed.load(Ordering::Acquire),
            bytes_sent: i.bytes_sent.load(Ordering::Acquire),
            bytes_confirmed: i.bytes_confirmed.load(Ordering::Acquire),
            bytes_written: i.bytes_written.load(Ordering::Acquire),
            bytes_checked: i.bytes_checked.load(Ordering::Acquire),
            bytes_already_there: i.bytes_already_there.load(Ordering::Acquire),
            bytes_planned: i.bytes_planned.load(Ordering::Acquire),
            checking: files.checking,
        }
    }

    /// Never fails: a reader that panicked while holding the lock cannot have left the counts half
    /// changed (each change is whole under the lock), so they are still right.
    fn files(&self) -> MutexGuard<'_, Files> {
        self.0.files.lock().unwrap_or_else(PoisonError::into_inner)
    }

    /// Starts counting again from nothing (a session takes a meter over).
    pub(crate) fn reset(&self) {
        let i = &self.0;
        for a in [
            &i.blocks_sent,
            &i.blocks_confirmed,
            &i.bytes_sent,
            &i.bytes_confirmed,
            &i.bytes_written,
            &i.bytes_checked,
            &i.bytes_already_there,
            &i.bytes_planned,
        ] {
            a.store(0, Ordering::Release);
        }
        *self.files() = Files::default();
    }

    /// One more planned file in the group named `name`, waiting; returns the group.
    pub(crate) fn plan(&self, name: &str, bytes: u64) -> Group {
        self.0.bytes_planned.fetch_add(bytes, Ordering::AcqRel);
        let mut files = self.files();
        let group = match files.by_name.get(name) {
            Some(g) => *g,
            None => {
                let g = Group::try_from(files.groups.len()).unwrap_or(Group::MAX);
                files.groups.push((name.to_string(), StepCounts::default()));
                files.by_name.insert(name.to_string(), g);
                g
            }
        };
        if let Some((_, c)) = files.groups.get_mut(group as usize) {
            c.total += 1;
            c.waiting += 1;
        }
        group
    }

    /// A file of `group` moved from step `from` to step `to`.
    pub(crate) fn step(&self, group: Group, from: Step, to: Step) {
        if from == to {
            return;
        }
        let mut files = self.files();
        if let Some((_, c)) = files.groups.get_mut(group as usize) {
            let was = c.slot(from);
            *was = was.saturating_sub(1);
            *c.slot(to) += 1;
        }
    }

    /// `item` is being read back and checked now (or, with `None`, nothing is).
    pub(crate) fn checking(&self, item: Option<ItemId>) {
        self.files().checking = item;
    }

    pub(crate) fn block_sent(&self, bytes: u64) {
        self.0.blocks_sent.fetch_add(1, Ordering::AcqRel);
        self.0.bytes_sent.fetch_add(bytes, Ordering::AcqRel);
    }

    pub(crate) fn block_confirmed(&self, bytes: u64) {
        self.0.blocks_confirmed.fetch_add(1, Ordering::AcqRel);
        self.0.bytes_confirmed.fetch_add(bytes, Ordering::AcqRel);
    }

    pub(crate) fn blocks_sent(&self) -> u64 {
        self.0.blocks_sent.load(Ordering::Acquire)
    }

    pub(crate) fn blocks_confirmed(&self) -> u64 {
        self.0.blocks_confirmed.load(Ordering::Acquire)
    }

    pub(crate) fn written_bytes(&self, bytes: u64) {
        self.0.bytes_written.fetch_add(bytes, Ordering::AcqRel);
    }

    pub(crate) fn checked_bytes(&self, bytes: u64) {
        self.0.bytes_checked.fetch_add(bytes, Ordering::AcqRel);
    }

    /// A file counted as checked was found gone or changed soon after.
    pub(crate) fn unchecked_bytes(&self, bytes: u64) {
        let _ = self
            .0
            .bytes_checked
            .try_update(Ordering::AcqRel, Ordering::Acquire, |n| {
                Some(n.saturating_sub(bytes))
            });
    }

    pub(crate) fn already_there_bytes(&self, bytes: u64) {
        self.0
            .bytes_already_there
            .fetch_add(bytes, Ordering::AcqRel);
    }
}

#[derive(Debug, Default)]
struct ControlInner {
    stop: AtomicBool,
    wake: Notify,
}

/// The person's stop for a running move (see the module docs). Cheap to clone; set it from any
/// thread. Once set it stays set: a session given it stops every run at once.
#[derive(Debug, Clone, Default)]
pub struct MoveControl(Arc<ControlInner>);

impl MoveControl {
    pub fn new() -> Self {
        Self::default()
    }

    /// Stops the move: no more blocks are sent, and the session's run ends with
    /// [`TransferError::Stopped`](crate::TransferError::Stopped).
    pub fn stop(&self) {
        self.0.stop.store(true, Ordering::Release);
        self.0.wake.notify_waiters();
    }

    pub fn stopped(&self) -> bool {
        self.0.stop.load(Ordering::Acquire)
    }

    /// Returns once the move is stopped.
    pub(crate) async fn wait_stopped(&self) {
        loop {
            // Made before looking: a stop made after this wakes it even before it is first
            // awaited (tokio's `notify_waiters` guarantee), so no stop is missed.
            let woken = self.0.wake.notified();
            if self.stopped() {
                return;
            }
            woken.await;
        }
    }
}
