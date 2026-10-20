//! What a running move has measured, readable while it runs, and the switch that stops it.
//!
//! A [`MoveMeter`] is shared (cheap to clone) between a session and whoever shows the move: the
//! session counts as it works, and a reader on any thread takes a [`MeterReading`] at any time.
//! Every number in it was counted, never estimated. Block and byte counters are atomics; the
//! files in each step are kept per group (the destination each file lands in, as the approved
//! plan names it) behind a small lock taken only when a file changes step.
//!
//! A [`MeterSnapshot`] is a reading with what the engine works out from it, and only from it: the
//! speed, measured over the last readings (see [`WINDOW`](crate::WINDOW)), and the [`TimeLeft`], which
//! is [`TimeLeft::NotKnownYet`] whenever there is not enough to say. The screens show these
//! figures; they never make one up.
//!
//! A [`MoveControl`] is the person's stop. A session checks it before each block it sends and
//! ends its run with [`TransferError::Stopped`](crate::TransferError::Stopped) as soon as it is
//! set, from any thread, however long it was waiting.

use std::collections::{BTreeMap, HashMap};
use std::sync::atomic::{AtomicBool, AtomicU64, Ordering};
use std::sync::{Arc, Mutex, MutexGuard, PoisonError};
use std::time::Instant;

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
    /// needing the person, or failed). What is left to do is the plan less these.
    pub bytes_through: u64,
    /// New laptop: the file being read back and checked right now, if one is.
    pub checking: Option<ItemId>,
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
    /// Bytes of the plan not yet through; `None` where the plan's size is not known (the old
    /// laptop).
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
    /// The readings the speed is measured from. Taken by readers only, never by the move.
    pace: Mutex<Pace>,
}

/// A running move's counters (see the module docs). Give one meter to one session.
#[derive(Debug, Clone, Default)]
pub struct MoveMeter(Arc<Inner>);

/// A planned file's place in the meter: its group, and its size in the plan.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct Group {
    index: u32,
    bytes: u64,
}

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
            bytes_through: files.bytes_through,
            checking: files.checking,
        }
    }

    /// A reading now, with the speed and time left (see [`snapshot_at`](Self::snapshot_at)).
    pub fn snapshot(&self) -> MeterSnapshot {
        self.snapshot_at(Instant::now())
    }

    /// A reading taken at `now`, with the speed measured over the readings taken so far (each
    /// snapshot is one; take one every second or so while the move runs) and the time left worked
    /// out from it:
    ///
    /// - [`TimeLeft::Done`] once every file is through;
    /// - [`TimeLeft::Waiting`] when nothing has moved for [`STALLED`](crate::STALLED);
    /// - [`TimeLeft::About`] the bytes left over the measured speed, once there is a speed
    ///   (readings over at least [`WARM_UP`](crate::WARM_UP)) and the plan's size is known;
    /// - else [`TimeLeft::NotKnownYet`].
    ///
    /// The bytes left are those of files not yet through, whole, so a file part-written counts in
    /// full until it is through: the figure leans long, never short.
    pub fn snapshot_at(&self, now: Instant) -> MeterSnapshot {
        let reading = self.reading();
        // One meter belongs to one session, so one of the two is always 0.
        let done = reading
            .bytes_confirmed
            .saturating_add(reading.bytes_written);
        let measured = self.pace().measure(now, done);
        let through = reading.files.left() == 0;
        let bytes_left = (reading.bytes_planned > 0 || through)
            .then(|| reading.bytes_planned.saturating_sub(reading.bytes_through));
        let time_left = if through {
            TimeLeft::Done
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
        *self.files() = Files::default();
        self.pace().reset();
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
        Group { index, bytes }
    }

    /// A file planned at `group` moved from step `from` to step `to`.
    pub(crate) fn step(&self, group: Group, from: Step, to: Step) {
        if from == to {
            return;
        }
        let mut files = self.files();
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

#[cfg(test)]
mod tests {
    use std::time::Duration;

    use super::*;

    const MB: u64 = 1_000_000;

    /// A new laptop's meter planned with `sizes`, each in group "me".
    fn planned(sizes: &[u64]) -> (MoveMeter, Vec<Group>) {
        let m = MoveMeter::new();
        let g = sizes.iter().map(|s| m.plan("me", *s)).collect();
        (m, g)
    }

    /// `n` seconds of snapshots from `*at`, `per_second` bytes written in each.
    fn seconds(m: &MoveMeter, at: &mut Instant, n: u32, per_second: u64) -> MeterSnapshot {
        let mut last = m.snapshot_at(*at);
        for _ in 0..n {
            *at += Duration::from_secs(1);
            m.written_bytes(per_second);
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
        let s = seconds(&m, &mut at, 4, 2 * MB);
        assert_eq!(s.time_left, TimeLeft::NotKnownYet, "4 seconds is too few");
        let s = seconds(&m, &mut at, 1, 2 * MB);
        assert_eq!(s.bytes_per_second, Some(2 * MB));
        // Nothing is through yet: all 150 MB are left, at 2 MB a second.
        assert_eq!(s.time_left, TimeLeft::About { seconds: 75 });
        // The first file is through: 50 MB left.
        m.step(g[0], Step::Copying, Step::Copied);
        let s = seconds(&m, &mut at, 1, 2 * MB);
        assert_eq!(s.bytes_left, Some(50 * MB));
        assert_eq!(s.time_left, TimeLeft::About { seconds: 25 });
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
    fn a_move_with_nothing_moving_says_waiting_not_a_time() {
        let (m, g) = planned(&[100 * MB]);
        m.step(g[0], Step::Waiting, Step::Copying);
        let mut at = Instant::now();
        seconds(&m, &mut at, 10, MB);
        let s = seconds(&m, &mut at, 10, 0);
        assert_eq!(s.time_left, TimeLeft::Waiting);
        let s = seconds(&m, &mut at, 1, MB);
        assert!(matches!(s.time_left, TimeLeft::About { .. }), "{s:?}");
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
        let s = seconds(&m, &mut at, 5, 100 * MB);
        assert_eq!(s.time_left, TimeLeft::About { seconds: 1 });
    }

    #[test]
    fn a_meter_taken_over_measures_its_speed_again() {
        let (m, g) = planned(&[100 * MB]);
        m.step(g[0], Step::Waiting, Step::Copying);
        let mut at = Instant::now();
        seconds(&m, &mut at, 10, MB);
        m.reset();
        let g = m.plan("me", 100 * MB);
        m.step(g, Step::Waiting, Step::Copying);
        let s = seconds(&m, &mut at, 1, MB);
        assert_eq!(s.bytes_per_second, None);
        assert_eq!(s.time_left, TimeLeft::NotKnownYet);
    }

    #[test]
    fn a_snapshot_serialises_with_the_names_the_screens_read() {
        let (m, g) = planned(&[3 * MB]);
        m.step(g[0], Step::Waiting, Step::Copying);
        let mut at = Instant::now();
        let s = seconds(&m, &mut at, 5, MB / 2);
        let json = serde_json::to_value(&s).unwrap();
        assert_eq!(
            json["timeLeft"],
            serde_json::json!({ "kind": "about", "seconds": 6 })
        );
        assert_eq!(json["bytesPerSecond"], serde_json::json!(MB / 2));
        assert_eq!(json["bytesLeft"], serde_json::json!(3 * MB));
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
                    }
                });
            }
            for _ in 0..64 {
                m.written_bytes(MB);
            }
        });
        assert_eq!(m.reading().bytes_written, 64 * MB);
    }
}
