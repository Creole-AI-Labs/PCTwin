//! What a running move has measured, readable while it runs, and the switch that stops it.
//!
//! A [`MoveMeter`] is shared (cheap to clone) between a session and whoever shows the move: the
//! session counts as it works, and a reader on any thread takes a [`MeterReading`] at any time.
//! Every number in it was counted, never estimated. Block and byte counters are atomics; the
//! files in each step are kept per group (the destination each file lands in, as the approved
//! plan names it), with the bytes written of each file not yet through, behind a small lock the
//! move takes when a file changes step and when the new laptop writes a block.
//!
//! A [`MeterSnapshot`] is a reading with what the engine works out from it, and only from it: the
//! speed, measured over the last readings (see [`WINDOW`](crate::WINDOW)), and the [`TimeLeft`], which
//! is [`TimeLeft::NotKnownYet`] whenever there is not enough to say. The screens show these
//! figures; they never make one up. A time left can be very long (a slow link and a big move):
//! the screens cap what they show (for example "more than a day") rather than print it whole.
//!
//! A [`MoveControl`] is the person's stop and pause. A session checks it before each block it
//! sends. A stop ends its run with [`TransferError::Stopped`](crate::TransferError::Stopped) as
//! soon as it is set, from any thread, however long it was waiting. A pause holds the move at a
//! block boundary without ending the run, and the other laptop is told (see
//! [`Message::Hold`](crate::Message::Hold)).

use std::collections::{BTreeMap, HashMap};
use std::sync::atomic::{AtomicBool, AtomicU64, Ordering};
use std::sync::{Arc, Mutex, MutexGuard, PoisonError};
use tokio::time::Instant;

use pctwin_record::ItemId;
use serde::Serialize;
use tokio::sync::Notify;

use crate::pace::Pace;

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
    /// Kept on the new laptop, checked, but not finished under its real name: it needs the
    /// person (a copy filled in under its name and not yet shown, or one that landed beside a
    /// file PCTwin could not prove its own). Never counted copied.
    NeedsAction,
    /// It did not move. Its reason is in the session's outcome for it.
    Failed,
}

impl Step {
    /// The file is through: nothing more is left to do for it in this move (a file through can
    /// still go back, for example a failed attempt started again).
    fn through(self) -> bool {
        matches!(
            self,
            Step::Copied | Step::AlreadyThere | Step::NeedsAction | Step::Failed
        )
    }
}

/// Files in each step. They add up to `total`.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct StepCounts {
    pub total: u64,
    pub waiting: u64,
    pub copying: u64,
    pub checking: u64,
    pub copied: u64,
    pub already_there: u64,
    pub needs_action: u64,
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
            Step::NeedsAction => &mut self.needs_action,
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
        self.needs_action += other.needs_action;
        self.failed += other.failed;
    }

    /// Files not yet through: waiting, copying or checking.
    fn left(&self) -> u64 {
        self.waiting + self.copying + self.checking
    }
}

/// Everything the meter has counted, at one moment.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
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
    /// New laptop: the approved size of every file that is through (copied, already there,
    /// needing the person, or failed).
    pub bytes_through: u64,
    /// New laptop: bytes written of the files not yet through, in their current attempt, each
    /// counted up to its planned size. What is left to do is the plan less these and the bytes
    /// through.
    pub bytes_in_progress: u64,
    /// New laptop: the file being read back and checked right now, if one is.
    pub checking: Option<ItemId>,
    /// The move is held by this laptop's pause, and the other laptop has been told (or, with a
    /// laptop one version back that cannot be told, this one holds itself: the old laptop only).
    pub paused_here: bool,
    /// The move is held by the other laptop's pause, as it said.
    pub paused_there: bool,
}

/// How long the move has left, as the engine works it out (see the module docs).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(tag = "kind", rename_all = "kebab-case")]
pub enum TimeLeft {
    /// Not enough is measured to say: the first seconds of a move, or a laptop that does not know
    /// the plan's size (the old one). Never a guess.
    NotKnownYet,
    /// About this long: what is left over the measured speed (the screen rounds it).
    About { seconds: u64 },
    /// Nothing has moved for a while: waiting for the other laptop, a drive or the network.
    Waiting,
    /// Paused, here or on the other laptop. The speed is measured again once it carries on.
    Paused,
    /// Every file is through.
    Done,
}

/// One reading of the meter, with the speed and time left worked out from it.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct MeterSnapshot {
    pub reading: MeterReading,
    /// Bytes a second, measured over about the last 20 seconds of readings (old laptop: bytes
    /// the new laptop confirmed; new laptop: bytes written). `None` until enough is measured.
    pub bytes_per_second: Option<u64>,
    /// Bytes of the plan not yet written or through; `None` until the plan is loaded, and where
    /// its size is not known (the old laptop).
    pub bytes_left: Option<u64>,
    pub time_left: TimeLeft,
}

#[derive(Debug, Default)]
struct Files {
    groups: Vec<(String, StepCounts)>,
    by_name: HashMap<String, u32>,
    checking: Option<ItemId>,
    /// See [`MeterReading::bytes_through`].
    bytes_through: u64,
    /// Per planned file not yet through: bytes written in its current attempt (uncapped).
    written: HashMap<u32, u64>,
    /// See [`MeterReading::bytes_in_progress`].
    bytes_in_progress: u64,
    /// The next planned file's number.
    next_file: u32,
    /// The whole plan is loaded: only then can the move be done.
    loaded: bool,
}

impl Files {
    /// `group`'s file no longer counts what it wrote (a new attempt, or it left copying and
    /// checking).
    fn forget_written(&mut self, group: Group) {
        if let Some(w) = self.written.remove(&group.file) {
            self.bytes_in_progress = self.bytes_in_progress.saturating_sub(w.min(group.bytes));
        }
    }
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
    paused_here: AtomicBool,
    paused_there: AtomicBool,
    files: Mutex<Files>,
    /// The readings the speed is measured from. Taken by readers only, never by the move.
    pace: Mutex<Pace>,
}

/// A running move's counters (see the module docs). Give one meter to one session.
#[derive(Debug, Clone, Default)]
pub struct MoveMeter(Arc<Inner>);

/// A planned file's place in the meter: its group, its own number, and its size in the plan.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct Group {
    index: u32,
    file: u32,
    bytes: u64,
}

impl MoveMeter {
    pub fn new() -> Self {
        Self::default()
    }

    /// Everything counted so far.
    pub fn reading(&self) -> MeterReading {
        self.reading_and_loaded().0
    }

    /// The reading, and whether the whole plan is loaded.
    fn reading_and_loaded(&self) -> (MeterReading, bool) {
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
        let reading = MeterReading {
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
            bytes_through: files.bytes_through,
            bytes_in_progress: files.bytes_in_progress,
            checking: files.checking,
            paused_here: i.paused_here.load(Ordering::Acquire),
            paused_there: i.paused_there.load(Ordering::Acquire),
        };
        (reading, files.loaded)
    }

    /// A reading now, with the speed and time left (see [`snapshot_at`](Self::snapshot_at)).
    pub fn snapshot(&self) -> MeterSnapshot {
        self.snapshot_at(Instant::now())
    }

    /// A reading taken at `now`, with the speed measured over the readings taken so far (each
    /// snapshot is one; take one every second or so while the move runs) and the time left worked
    /// out from it:
    ///
    /// - [`TimeLeft::Done`] once the plan is loaded and every file of it is through;
    /// - [`TimeLeft::Waiting`] when nothing has moved for [`STALLED`](crate::STALLED);
    /// - [`TimeLeft::About`] the bytes left over the measured speed, once there is a speed
    ///   (readings over at least [`WARM_UP`](crate::WARM_UP)) and the plan's size is known;
    /// - else [`TimeLeft::NotKnownYet`].
    ///
    /// The bytes left are the plan less the files through and the bytes written of those not
    /// yet through (each up to its planned size), so a big file counts down as it is written.
    /// Before the plan is loaded nothing is known, and the move is never done.
    pub fn snapshot_at(&self, now: Instant) -> MeterSnapshot {
        let (reading, loaded) = self.reading_and_loaded();
        // One meter belongs to one session, so one of the two is always 0.
        let done = reading
            .bytes_confirmed
            .saturating_add(reading.bytes_written);
        let through = loaded && reading.files.left() == 0;
        let paused = reading.paused_here || reading.paused_there;
        let measured = {
            let mut pace = self.pace();
            if paused {
                // Paused time is not measured: the speed starts again from nothing after it.
                pace.reset();
            }
            pace.measure(now, done)
        };
        let bytes_left = (loaded && (reading.bytes_planned > 0 || through)).then(|| {
            reading
                .bytes_planned
                .saturating_sub(reading.bytes_through)
                .saturating_sub(reading.bytes_in_progress)
        });
        let time_left = if through {
            TimeLeft::Done
        } else if paused {
            TimeLeft::Paused
        } else if measured.stalled {
            TimeLeft::Waiting
        } else {
            match (bytes_left, measured.speed) {
                (Some(left), Some(speed)) if speed > 0 => TimeLeft::About {
                    seconds: left.div_ceil(speed).max(1),
                },
                _ => TimeLeft::NotKnownYet,
            }
        };
        MeterSnapshot {
            reading,
            bytes_per_second: measured.speed,
            bytes_left,
            time_left,
        }
    }

    /// Never fails: a reader that panicked while holding the lock cannot have left the counts half
    /// changed (each change is whole under the lock), so they are still right.
    fn files(&self) -> MutexGuard<'_, Files> {
        self.0.files.lock().unwrap_or_else(PoisonError::into_inner)
    }

    /// As [`files`](Self::files): a reading is whole under the lock.
    fn pace(&self) -> MutexGuard<'_, Pace> {
        self.0.pace.lock().unwrap_or_else(PoisonError::into_inner)
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
        i.paused_here.store(false, Ordering::Release);
        i.paused_there.store(false, Ordering::Release);
        *self.files() = Files::default();
        self.pace().reset();
    }

    /// The move is (or is no longer) held by this laptop's pause.
    pub(crate) fn paused_here(&self, paused: bool) {
        self.0.paused_here.store(paused, Ordering::Release);
    }

    /// The other laptop said its person paused (or carried on).
    pub(crate) fn paused_there(&self, paused: bool) {
        self.0.paused_there.store(paused, Ordering::Release);
    }

    /// One more planned file of `bytes` in the group named `name`, waiting; returns its place.
    pub(crate) fn plan(&self, name: &str, bytes: u64) -> Group {
        self.0.bytes_planned.fetch_add(bytes, Ordering::AcqRel);
        let mut files = self.files();
        let index = match files.by_name.get(name) {
            Some(g) => *g,
            None => {
                let g = u32::try_from(files.groups.len()).unwrap_or(u32::MAX);
                files.groups.push((name.to_string(), StepCounts::default()));
                files.by_name.insert(name.to_string(), g);
                g
            }
        };
        if let Some((_, c)) = files.groups.get_mut(index as usize) {
            c.total += 1;
            c.waiting += 1;
        }
        let file = files.next_file;
        files.next_file = files.next_file.saturating_add(1);
        Group { index, file, bytes }
    }

    /// Every file of the plan is planned: from now on the move can be done.
    pub(crate) fn plan_loaded(&self) {
        self.files().loaded = true;
    }

    /// A fresh attempt at `group`'s file starts: what an earlier one wrote counts no more.
    pub(crate) fn attempt(&self, group: Group) {
        self.files().forget_written(group);
    }

    /// A file planned at `group` moved from step `from` to step `to`.
    pub(crate) fn step(&self, group: Group, from: Step, to: Step) {
        if from == to {
            return;
        }
        let mut files = self.files();
        if !matches!(to, Step::Copying | Step::Checking) {
            files.forget_written(group);
        }
        let Some((_, c)) = files.groups.get_mut(group.index as usize) else {
            return;
        };
        let was = c.slot(from);
        *was = was.saturating_sub(1);
        *c.slot(to) += 1;
        match (from.through(), to.through()) {
            (false, true) => files.bytes_through = files.bytes_through.saturating_add(group.bytes),
            (true, false) => files.bytes_through = files.bytes_through.saturating_sub(group.bytes),
            _ => {}
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

    /// `bytes` more of `group`'s file written (a block written for the first time in this
    /// attempt).
    pub(crate) fn written(&self, group: Group, bytes: u64) {
        self.0.bytes_written.fetch_add(bytes, Ordering::AcqRel);
        self.progress(group, bytes);
    }

    /// `bytes` more of `group`'s file through in this attempt (written on the new laptop,
    /// confirmed on the old one), so the time left counts down inside a big file.
    pub(crate) fn progress(&self, group: Group, bytes: u64) {
        let mut files = self.files();
        let was = files.written.get(&group.file).copied().unwrap_or(0);
        let now = was.saturating_add(bytes);
        files.written.insert(group.file, now);
        let more = now.min(group.bytes) - was.min(group.bytes);
        files.bytes_in_progress = files.bytes_in_progress.saturating_add(more);
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
    pause: AtomicBool,
    /// Every stop, pause and carrying on counts one more, so a waiter that looked before a
    /// pause and a carrying on (back to back) still sees that something changed.
    changes: AtomicU64,
    wake: Notify,
    /// The approved places whose drives are not keeping files correctly, when that paused the
    /// move, in the order they did (each once; at most one per approved place).
    braked: std::sync::Mutex<Vec<String>>,
}

/// The person's stop and pause for a running move (see the module docs). Cheap to clone; set it
/// from any thread. A stop stays set: a session given it stops every run at once. A pause lasts
/// until [`carry_on`](Self::carry_on), across connections.
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
        self.changed();
    }

    pub fn stopped(&self) -> bool {
        self.0.stop.load(Ordering::Acquire)
    }

    /// Pauses the move at the next block boundary: no new block is sent, those already in flight
    /// (at most the in-flight limit) finish, and the other laptop is told. The run does not end;
    /// [`carry_on`](Self::carry_on) picks up exactly where it held.
    pub fn pause(&self) {
        self.0.pause.store(true, Ordering::Release);
        self.changed();
    }

    /// Ends this laptop's pause (the other laptop's own pause, if it made one, still holds), and
    /// any brake: the person has chosen to carry on.
    pub fn carry_on(&self) {
        if let Ok(mut b) = self.0.braked.lock() {
            b.clear();
        }
        self.0.pause.store(false, Ordering::Release);
        self.changed();
    }

    /// Pauses the move because the drive of the approved place `destination` is not keeping
    /// files correctly (Quarantine Design, step 4); the screen says so and offers another place
    /// or stopping (X07). Files already checked are fine. A second place that pauses it too is
    /// held as well: neither takes another write until the person carries on.
    pub(crate) fn brake(&self, destination: &str) {
        if let Ok(mut b) = self.0.braked.lock()
            && !b.iter().any(|d| d == destination)
        {
            b.push(destination.to_string());
        }
        self.pause();
    }

    /// The approved place whose drive paused the move first, if one did (the screen names it).
    pub fn braked(&self) -> Option<String> {
        self.0.braked.lock().ok().and_then(|b| b.first().cloned())
    }

    /// Whether the approved place `destination` paused the move and still holds it: nothing more
    /// is written there until the person carries on.
    pub(crate) fn is_braked(&self, destination: &str) -> bool {
        let b = self
            .0
            .braked
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner);
        b.iter().any(|d| d == destination)
    }

    /// Counts the change, then wakes every waiter (after the count, so none misses it).
    fn changed(&self) {
        self.0.changes.fetch_add(1, Ordering::AcqRel);
        self.0.wake.notify_waiters();
    }

    /// How many stops, pauses and carryings on there have been: look before deciding to wait,
    /// then wait with [`wait_change`](Self::wait_change).
    pub(crate) fn changes(&self) -> u64 {
        self.0.changes.load(Ordering::Acquire)
    }

    /// Whether this laptop's person has paused the move.
    pub fn paused(&self) -> bool {
        self.0.pause.load(Ordering::Acquire)
    }

    /// Returns once there was a stop, pause or carrying on after `seen` (from
    /// [`changes`](Self::changes)): at once if there was one already, even a pause and a
    /// carrying on that left the move as it was.
    pub(crate) async fn wait_change(&self, seen: u64) {
        loop {
            // Made before looking, as in `wait_stopped`.
            let woken = self.0.wake.notified();
            if self.changes() != seen {
                return;
            }
            woken.await;
        }
    }

    /// Returns once the move is stopped (at once if it already is).
    pub async fn wait_stopped(&self) {
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

#[cfg(test)]
mod tests {
    use std::time::Duration;

    use super::*;

    const MB: u64 = 1_000_000;

    /// Fresh-context review of TRANSFER (24 October 2026): a second place whose drive pauses the
    /// move is held as well, never in place of the first; carrying on ends both.
    #[test]
    fn a_second_braked_place_is_held_as_well_as_the_first() {
        let c = MoveControl::new();
        c.brake("me");
        c.brake("photos");
        c.brake("me");
        assert!(c.is_braked("me"));
        assert!(c.is_braked("photos"));
        assert!(!c.is_braked("music"));
        assert_eq!(c.braked().as_deref(), Some("me"));
        assert!(c.paused());
        c.carry_on();
        assert!(!c.is_braked("me") && !c.is_braked("photos"));
        assert_eq!(c.braked(), None);
    }

    /// A new laptop's meter with the plan `sizes` loaded, each in group "me".
    fn planned(sizes: &[u64]) -> (MoveMeter, Vec<Group>) {
        let m = MoveMeter::new();
        let g = sizes.iter().map(|s| m.plan("me", *s)).collect();
        m.plan_loaded();
        (m, g)
    }

    /// `n` seconds of snapshots from `*at`, `per_second` bytes of `file` written in each.
    fn seconds(
        m: &MoveMeter,
        file: Group,
        at: &mut Instant,
        n: u32,
        per_second: u64,
    ) -> MeterSnapshot {
        let mut last = m.snapshot_at(*at);
        for _ in 0..n {
            *at += Duration::from_secs(1);
            m.written(file, per_second);
            last = m.snapshot_at(*at);
        }
        last
    }

    #[test]
    fn time_left_is_not_known_until_measured_then_is_what_is_left_over_the_speed() {
        let (m, g) = planned(&[100 * MB, 50 * MB]);
        let mut at = Instant::now();
        let s = m.snapshot_at(at);
        assert_eq!(s.time_left, TimeLeft::NotKnownYet);
        assert_eq!(s.bytes_per_second, None);
        assert_eq!(s.bytes_left, Some(150 * MB));
        m.step(g[0], Step::Waiting, Step::Copying);
        let s = seconds(&m, g[0], &mut at, 4, 2 * MB);
        assert_eq!(s.time_left, TimeLeft::NotKnownYet, "4 seconds is too few");
        let s = seconds(&m, g[0], &mut at, 1, 2 * MB);
        assert_eq!(s.bytes_per_second, Some(2 * MB));
        // 10 MB of the first file written: 140 MB left, at 2 MB a second.
        assert_eq!(s.bytes_left, Some(140 * MB));
        assert_eq!(s.time_left, TimeLeft::About { seconds: 70 });
        // The first file is through, and 2 MB of the second written: 48 MB left.
        m.step(g[0], Step::Copying, Step::Copied);
        m.step(g[1], Step::Waiting, Step::Copying);
        let s = seconds(&m, g[1], &mut at, 1, 2 * MB);
        assert_eq!(s.bytes_left, Some(48 * MB));
        assert_eq!(s.time_left, TimeLeft::About { seconds: 24 });
    }

    #[test]
    fn a_big_file_counts_down_while_it_is_written_never_past_its_size() {
        let (m, g) = planned(&[100 * MB]);
        m.step(g[0], Step::Waiting, Step::Copying);
        let mut at = Instant::now();
        let s = seconds(&m, g[0], &mut at, 5, 2 * MB);
        assert_eq!(s.time_left, TimeLeft::About { seconds: 45 });
        let s = seconds(&m, g[0], &mut at, 1, 2 * MB);
        assert_eq!(s.time_left, TimeLeft::About { seconds: 44 });
        let s = seconds(&m, g[0], &mut at, 5, 2 * MB);
        assert_eq!(s.time_left, TimeLeft::About { seconds: 39 });
        // Blocks written again (or a file that grew) never count past the planned size.
        m.written(g[0], 500 * MB);
        assert_eq!(m.snapshot_at(at).bytes_left, Some(0));
        // Being read back and checked still counts what was written.
        m.step(g[0], Step::Copying, Step::Checking);
        assert_eq!(m.snapshot_at(at).bytes_left, Some(0));
    }

    #[test]
    fn a_file_started_again_or_failed_no_longer_counts_what_it_had_written() {
        let (m, g) = planned(&[10 * MB, 20 * MB]);
        let at = Instant::now();
        m.step(g[0], Step::Waiting, Step::Copying);
        m.written(g[0], 4 * MB);
        assert_eq!(m.snapshot_at(at).bytes_left, Some(26 * MB));
        // A fresh attempt starts from nothing.
        m.attempt(g[0]);
        assert_eq!(m.snapshot_at(at).bytes_left, Some(30 * MB));
        m.written(g[0], 3 * MB);
        assert_eq!(m.snapshot_at(at).bytes_left, Some(27 * MB));
        // Failed: through, at its planned size; what it wrote counts no more.
        m.step(g[0], Step::Copying, Step::Failed);
        assert_eq!(m.snapshot_at(at).bytes_left, Some(20 * MB));
        // Started again from failed: all of it is left again.
        m.step(g[0], Step::Failed, Step::Copying);
        assert_eq!(m.snapshot_at(at).bytes_left, Some(30 * MB));
        // Back to waiting (its attempt let go): what it wrote counts no more either.
        m.written(g[0], 5 * MB);
        m.step(g[0], Step::Copying, Step::Waiting);
        assert_eq!(m.snapshot_at(at).bytes_left, Some(30 * MB));
    }

    #[test]
    fn one_file_written_past_its_size_takes_nothing_off_the_others() {
        let (m, g) = planned(&[10 * MB, 20 * MB]);
        let at = Instant::now();
        m.step(g[0], Step::Waiting, Step::Copying);
        m.written(g[0], 30 * MB);
        let s = m.snapshot_at(at);
        assert_eq!(s.reading.bytes_in_progress, 10 * MB);
        assert_eq!(s.bytes_left, Some(20 * MB));
        // And forgetting it takes off only what it counted.
        m.step(g[1], Step::Waiting, Step::Copying);
        m.written(g[1], 5 * MB);
        m.attempt(g[0]);
        assert_eq!(m.snapshot_at(at).bytes_left, Some(25 * MB));
    }

    #[test]
    fn files_that_fail_or_need_the_person_are_through_and_leave_nothing_to_wait_for() {
        let (m, g) = planned(&[10 * MB, 20 * MB, 30 * MB]);
        let at = Instant::now();
        m.step(g[0], Step::Waiting, Step::Failed);
        m.step(g[1], Step::Waiting, Step::NeedsAction);
        let s = m.snapshot_at(at);
        assert_eq!(s.reading.bytes_through, 30 * MB);
        assert_eq!(s.bytes_left, Some(30 * MB));
        // A failed attempt started again is not through any more.
        m.step(g[0], Step::Failed, Step::Copying);
        assert_eq!(m.snapshot_at(at).bytes_left, Some(40 * MB));
        m.step(g[0], Step::Copying, Step::Copied);
        m.step(g[2], Step::Waiting, Step::AlreadyThere);
        let s = m.snapshot_at(at);
        assert_eq!(s.bytes_left, Some(0));
        assert_eq!(s.time_left, TimeLeft::Done);
    }

    #[test]
    fn a_meter_with_no_plan_loaded_never_says_done() {
        let s = MoveMeter::new().snapshot();
        assert_eq!(s.time_left, TimeLeft::NotKnownYet, "{s:?}");
        assert_eq!(s.bytes_left, None);
        // Files planned, the plan not yet whole: not done either.
        let m = MoveMeter::new();
        let g = m.plan("me", 0);
        m.step(g, Step::Waiting, Step::Copied);
        assert_eq!(m.snapshot().time_left, TimeLeft::NotKnownYet);
        // Taken over: nothing is planned again until the new plan is loaded.
        let (m, g) = planned(&[MB]);
        m.step(g[0], Step::Waiting, Step::Copied);
        assert_eq!(m.snapshot().time_left, TimeLeft::Done);
        m.reset();
        let s = m.snapshot();
        assert_eq!(s.time_left, TimeLeft::NotKnownYet, "{s:?}");
        assert_eq!(s.bytes_left, None);
    }

    #[test]
    fn a_plan_with_nothing_in_it_or_only_empty_files_ends_done() {
        let (m, _) = planned(&[]);
        assert_eq!(m.snapshot().time_left, TimeLeft::Done);
        let (m, g) = planned(&[0, 0]);
        assert_eq!(m.snapshot().time_left, TimeLeft::NotKnownYet);
        m.step(g[0], Step::Waiting, Step::Copied);
        m.step(g[1], Step::Waiting, Step::AlreadyThere);
        let s = m.snapshot();
        assert_eq!(s.time_left, TimeLeft::Done);
        assert_eq!(s.bytes_left, Some(0));
    }

    #[test]
    fn a_move_with_nothing_moving_says_waiting_not_a_time() {
        let (m, g) = planned(&[100 * MB]);
        m.step(g[0], Step::Waiting, Step::Copying);
        let mut at = Instant::now();
        seconds(&m, g[0], &mut at, 10, MB);
        let s = seconds(&m, g[0], &mut at, 10, 0);
        assert_eq!(s.time_left, TimeLeft::Waiting);
        let s = seconds(&m, g[0], &mut at, 1, MB);
        assert!(matches!(s.time_left, TimeLeft::About { .. }), "{s:?}");
    }

    #[test]
    fn a_pause_here_or_there_says_paused_and_the_speed_is_measured_again_after_it() {
        let (m, g) = planned(&[100 * MB]);
        m.step(g[0], Step::Waiting, Step::Copying);
        let mut at = Instant::now();
        seconds(&m, g[0], &mut at, 10, MB);
        for (here, there) in [(true, false), (false, true), (true, true)] {
            m.paused_here(here);
            m.paused_there(there);
            // A long pause is paused, never "waiting" and never a time.
            let s = seconds(&m, g[0], &mut at, 30, 0);
            assert_eq!(s.time_left, TimeLeft::Paused, "{here} {there}");
        }
        m.paused_here(false);
        m.paused_there(false);
        // Carried on: nothing from before the pause counts, so not known until measured again.
        let s = seconds(&m, g[0], &mut at, 4, 4 * MB);
        assert_eq!(s.time_left, TimeLeft::NotKnownYet);
        let s = seconds(&m, g[0], &mut at, 1, 4 * MB);
        assert_eq!(s.bytes_per_second, Some(4 * MB));
        // 30 MB written of 100: 70 MB left at 4 MB a second.
        assert_eq!(s.time_left, TimeLeft::About { seconds: 18 });
        // Done is done, paused or not.
        m.paused_here(true);
        m.step(g[0], Step::Copying, Step::Copied);
        assert_eq!(m.snapshot_at(at).time_left, TimeLeft::Done);
        assert_eq!(
            serde_json::to_value(TimeLeft::Paused).unwrap(),
            serde_json::json!({ "kind": "paused" })
        );
    }

    #[test]
    fn a_pause_and_a_carrying_on_back_to_back_still_wake_a_waiter_that_looked_before() {
        use futures_util::FutureExt;
        let c = MoveControl::new();
        let seen = c.changes();
        assert!(
            c.wait_change(seen).now_or_never().is_none(),
            "nothing changed yet"
        );
        // Paused and carried on before the waiter looks again: the move is as it was, but it
        // changed, so the waiter is not left asleep.
        c.pause();
        c.carry_on();
        assert!(!c.paused());
        assert!(c.wait_change(seen).now_or_never().is_some());
        // A waiter already asleep is woken by each of them, from another thread.
        let rt = tokio::runtime::Builder::new_current_thread()
            .enable_time()
            .build()
            .unwrap();
        for act in [MoveControl::pause, MoveControl::carry_on, MoveControl::stop] {
            let seen = c.changes();
            let other = c.clone();
            let t = std::thread::spawn(move || {
                std::thread::sleep(Duration::from_millis(20));
                act(&other);
            });
            // Generous: it is woken within moments; this only turns a missed wake into a failure.
            let woken = rt.block_on(async {
                tokio::time::timeout(Duration::from_secs(60), c.wait_change(seen)).await
            });
            assert!(woken.is_ok(), "a waiter was left asleep");
            t.join().unwrap();
        }
    }

    #[test]
    fn a_laptop_that_does_not_know_the_plans_size_measures_speed_but_never_a_time() {
        // The old laptop plans its files at size 0.
        let (m, g) = planned(&[0, 0]);
        m.step(g[0], Step::Waiting, Step::Copying);
        let mut at = Instant::now();
        let mut s = m.snapshot_at(at);
        for _ in 0..10 {
            at += Duration::from_secs(1);
            m.block_confirmed(MB);
            s = m.snapshot_at(at);
        }
        assert_eq!(s.bytes_per_second, Some(MB));
        assert_eq!(s.bytes_left, None);
        assert_eq!(s.time_left, TimeLeft::NotKnownYet);
    }

    #[test]
    fn the_last_bytes_still_take_a_second_not_none() {
        let (m, g) = planned(&[MB]);
        m.step(g[0], Step::Waiting, Step::Copying);
        let mut at = Instant::now();
        let s = seconds(&m, g[0], &mut at, 5, MB / 10);
        assert_eq!(s.bytes_left, Some(MB / 2));
        assert_eq!(s.time_left, TimeLeft::About { seconds: 5 });
        let s = seconds(&m, g[0], &mut at, 5, MB / 10);
        assert_eq!(s.bytes_left, Some(0));
        assert_eq!(s.time_left, TimeLeft::About { seconds: 1 });
    }

    #[test]
    fn a_meter_taken_over_measures_its_speed_again() {
        let (m, g) = planned(&[100 * MB]);
        m.step(g[0], Step::Waiting, Step::Copying);
        let mut at = Instant::now();
        seconds(&m, g[0], &mut at, 10, MB);
        m.reset();
        let g = m.plan("me", 100 * MB);
        m.plan_loaded();
        m.step(g, Step::Waiting, Step::Copying);
        let s = seconds(&m, g, &mut at, 1, MB);
        assert_eq!(s.bytes_per_second, None);
        assert_eq!(s.time_left, TimeLeft::NotKnownYet);
    }

    #[test]
    fn a_snapshot_serialises_with_the_names_the_screens_read() {
        let (m, g) = planned(&[3 * MB]);
        m.step(g[0], Step::Waiting, Step::Copying);
        let mut at = Instant::now();
        let s = seconds(&m, g[0], &mut at, 5, MB / 2);
        let json = serde_json::to_value(&s).unwrap();
        assert_eq!(
            json["timeLeft"],
            serde_json::json!({ "kind": "about", "seconds": 1 })
        );
        assert_eq!(json["bytesPerSecond"], serde_json::json!(MB / 2));
        assert_eq!(json["bytesLeft"], serde_json::json!(MB / 2));
        assert_eq!(json["reading"]["files"]["copying"], serde_json::json!(1));
        assert_eq!(
            json["reading"]["groups"]["me"]["total"],
            serde_json::json!(1)
        );
        assert_eq!(
            json["reading"]["bytesWritten"],
            serde_json::json!(5 * MB / 2)
        );
        assert_eq!(json["reading"]["checking"], serde_json::Value::Null);
        for (t, kind) in [
            (TimeLeft::NotKnownYet, "not-known-yet"),
            (TimeLeft::Waiting, "waiting"),
            (TimeLeft::Done, "done"),
        ] {
            assert_eq!(
                serde_json::to_value(t).unwrap(),
                serde_json::json!({ "kind": kind })
            );
        }
    }

    #[test]
    fn readers_on_other_threads_take_snapshots_while_the_move_counts() {
        let (m, g) = planned(&[64 * MB]);
        m.step(g[0], Step::Waiting, Step::Copying);
        let start = Instant::now();
        std::thread::scope(|s| {
            for _ in 0..4 {
                let m = m.clone();
                s.spawn(move || {
                    for i in 0..1000u64 {
                        let snap = m.snapshot_at(start + Duration::from_millis(i));
                        assert!(snap.reading.bytes_written <= 64 * MB);
                        assert!(snap.bytes_left.is_some_and(|l| l <= 64 * MB));
                    }
                });
            }
            for _ in 0..64 {
                m.written(g[0], MB);
            }
        });
        assert_eq!(m.reading().bytes_written, 64 * MB);
        assert_eq!(m.snapshot().bytes_left, Some(0));
    }
}
