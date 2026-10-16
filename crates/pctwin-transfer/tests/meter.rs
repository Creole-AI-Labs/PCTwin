//! The move meter and the stop. While a move runs, its counts can be read from outside: blocks and
//! bytes as they go, each file's step (waiting, copying, checking, copied, already there, failed)
//! per destination, and which file is being read back right now. Every count is counted, never
//! worked out, and the files in each step add up to the plan. A stop set from anywhere ends the
//! run at once, with its own error, and not one block is sent after it.
// Tests make and remove files to test on; the no-file-changes fence is for the shipped code.
#![allow(clippy::disallowed_methods)]

use std::sync::Mutex;
use std::time::Duration;

use pctwin_gate::{Approved, Destinations};
use pctwin_journal::{FileId, Journal, JournalError, Landed, Ledger, PlannedWrite};
use pctwin_record::ItemId;
use pctwin_transfer::{
    Channel, ChannelError, MeterReading, MoveControl, MoveMeter, ReceiveOutcome, ReceiverSession,
    SendJob, SendOutcome, SenderSession, Step, StepCounts, Tier, TransferError,
};
use tokio::sync::mpsc;

mod common;

struct Mem {
    tx: mpsc::UnboundedSender<Vec<u8>>,
    rx: mpsc::UnboundedReceiver<Vec<u8>>,
}

fn mem_pair() -> (Mem, Mem) {
    let (a_tx, b_rx) = mpsc::unbounded_channel();
    let (b_tx, a_rx) = mpsc::unbounded_channel();
    (Mem { tx: a_tx, rx: a_rx }, Mem { tx: b_tx, rx: b_rx })
}

impl Channel for Mem {
    async fn send(&mut self, data: &[u8]) -> Result<(), ChannelError> {
        tokio::task::yield_now().await;
        self.tx.send(data.to_vec()).map_err(|_| ChannelError)
    }
    async fn recv(&mut self) -> Result<Vec<u8>, ChannelError> {
        self.rx.recv().await.ok_or(ChannelError)
    }
}

/// A connection on which the old laptop never says anything.
struct Silent;

impl Channel for Silent {
    async fn send(&mut self, _: &[u8]) -> Result<(), ChannelError> {
        Ok(())
    }
    async fn recv(&mut self) -> Result<Vec<u8>, ChannelError> {
        std::future::pending().await
    }
}

fn id(n: u8) -> ItemId {
    ItemId::from_hex(&format!("{n:02x}{}", "0".repeat(30))).unwrap()
}

fn bytes(len: usize, seed: u8) -> Vec<u8> {
    (0..len)
        .map(|i| (i as u8).wrapping_mul(31).wrapping_add(seed))
        .collect()
}

struct World {
    old: tempfile::TempDir,
    mine: tempfile::TempDir,
    shared: tempfile::TempDir,
    _jdir: tempfile::TempDir,
    journal: Journal,
    table: Destinations,
}

fn world() -> World {
    let old = tempfile::tempdir().unwrap();
    let mine = tempfile::tempdir().unwrap();
    let shared = tempfile::tempdir().unwrap();
    let jdir = tempfile::tempdir().unwrap();
    let journal = Journal::open(&jdir.path().join("journal.redb")).unwrap();
    let mut table = Destinations::new();
    table
        .approve("me", Approved::MyFolders, mine.path())
        .unwrap();
    table
        .approve("shared", Approved::SharedFolder, shared.path())
        .unwrap();
    World {
        old,
        mine,
        shared,
        _jdir: jdir,
        journal,
        table,
    }
}

impl World {
    fn file(&self, n: u8, len: usize, destination: &str) -> SendJob {
        let source = self.old.path().join(format!("f{n}.bin"));
        std::fs::write(&source, bytes(len, n)).unwrap();
        SendJob {
            item: id(n),
            source,
            destination: destination.into(),
            path: format!("f{n}.bin"),
            compressible: false,
            tier: Tier::Rest,
        }
    }
}

/// The plan for `jobs`, each at the size its file has now.
fn plan(jobs: &[SendJob]) -> pctwin_transfer::Allowance {
    let sized: Vec<(&SendJob, u64)> = jobs
        .iter()
        .map(|j| (j, std::fs::metadata(&j.source).unwrap().len()))
        .collect();
    common::approved_jobs(&sized)
}

/// Every count is one of the steps, and they add up.
fn adds_up(c: &StepCounts) -> bool {
    c.waiting + c.copying + c.checking + c.copied + c.already_there + c.failed == c.total
}

fn counts(total: u64, step: Step) -> StepCounts {
    let mut c = StepCounts {
        total,
        ..StepCounts::default()
    };
    match step {
        Step::Waiting => c.waiting = total,
        Step::Copying => c.copying = total,
        Step::Checking => c.checking = total,
        Step::Copied => c.copied = total,
        Step::AlreadyThere => c.already_there = total,
        Step::Failed => c.failed = total,
    }
    c
}

/// A journal that takes a reading of the meter at the moment a file is recorded as checked (its
/// read-back is done, its name not yet given).
struct Watching<'j> {
    inner: &'j Journal,
    meter: MoveMeter,
    at_verified: Mutex<Vec<MeterReading>>,
}

impl Ledger for Watching<'_> {
    fn temp_tag(&self, id: u64) -> String {
        self.inner.temp_tag(id)
    }
    fn plan(&self, w: &PlannedWrite) -> Result<u64, JournalError> {
        self.inner.plan(w)
    }
    fn staged(
        &self,
        id: u64,
        temp: &str,
        made: &[(String, Option<FileId>)],
    ) -> Result<(), JournalError> {
        self.inner.staged(id, temp, made)
    }
    fn verified(&self, id: u64, fp: [u8; 32], file: Option<FileId>) -> Result<(), JournalError> {
        self.at_verified.lock().unwrap().push(self.meter.reading());
        self.inner.verified(id, fp, file)
    }
    fn applied(&self, id: u64, final_path: &str) -> Result<(), JournalError> {
        self.inner.applied(id, final_path)
    }
    fn committed(&self, id: u64, landed: Landed) -> Result<(), JournalError> {
        self.inner.committed(id, landed)
    }
    fn existing(&self, id: u64, stored_path: &str) -> Result<(), JournalError> {
        self.inner.existing(id, stored_path)
    }
    fn failed(&self, id: u64, why: &str) -> Result<(), JournalError> {
        self.inner.failed(id, why)
    }
    fn checkpoint(
        &self,
        id: u64,
        blocks: &[(u64, [u8; 32])],
        durable: bool,
    ) -> Result<(), JournalError> {
        self.inner.checkpoint(id, blocks, durable)
    }
    fn blocks(&self, id: u64) -> Result<Vec<(u64, [u8; 32])>, JournalError> {
        self.inner.blocks(id)
    }
    fn unfinished(&self) -> Result<Vec<pctwin_journal::Entry>, JournalError> {
        self.inner.unfinished()
    }
}

#[tokio::test]
async fn both_meters_count_every_file_and_byte_by_destination_and_can_be_read_during_the_move() {
    let w = world();
    let jobs = vec![
        w.file(1, 3 * 1024 * 1024, "me"),
        w.file(2, 1000, "me"),
        w.file(3, 200_000, "shared"),
    ];
    let sizes: u64 = 3 * 1024 * 1024 + 1000 + 200_000;
    let send_meter = MoveMeter::new();
    let receive_meter = MoveMeter::new();
    let mut sender = SenderSession::new(jobs.clone(), 2)
        .with_in_flight_limit(512 * 1024)
        .with_meter(send_meter.clone());
    let mut receiver = ReceiverSession::new(&w.table, plan(&jobs), &w.journal, "1000")
        .with_meter(receive_meter.clone());
    // Before anything moves: every planned file is waiting, in its destination's group.
    let before = receive_meter.reading();
    assert_eq!(before.files, counts(3, Step::Waiting));
    assert_eq!(before.groups["me"], counts(2, Step::Waiting));
    assert_eq!(before.groups["shared"], counts(1, Step::Waiting));
    assert_eq!(before.bytes_planned, sizes);
    assert_eq!(send_meter.reading().files, counts(3, Step::Waiting));

    let (mut a, mut b) = mem_pair();
    let written = receiver.meter();
    let mut seen_send = Vec::new();
    let mut seen_receive = Vec::new();
    let watch = async {
        loop {
            let (s, r) = (send_meter.reading(), receive_meter.reading());
            assert!(adds_up(&s.files) && adds_up(&r.files), "{s:?} {r:?}");
            for c in s.groups.values().chain(r.groups.values()) {
                assert!(adds_up(c), "{c:?}");
            }
            let done = r.files.copied == 3 && s.files.copied == 3;
            seen_send.push(s);
            seen_receive.push(r);
            if done {
                return;
            }
            tokio::task::yield_now().await;
        }
    };
    let ((sent, received), ()) = common::unless_stuck(&written, sizes, async {
        tokio::join!(
            async { tokio::join!(sender.run(&mut a), receiver.run(&mut b)) },
            watch
        )
    })
    .await;
    sent.unwrap();
    received.unwrap();

    // Read during the move: the counts moved while it ran, and files were seen part-way.
    assert!(seen_send.iter().any(|s| s.files.copying > 0));
    assert!(seen_receive.iter().any(|r| r.files.copying > 0));
    // The old laptop counts a file sent whole as checking until the new laptop answers for it.
    assert!(seen_send.iter().any(|s| s.files.checking > 0));
    assert!(
        seen_receive
            .windows(2)
            .any(|p| p[0].bytes_written < p[1].bytes_written && p[1].bytes_written < sizes)
    );
    // At the end.
    let s = send_meter.reading();
    let r = receive_meter.reading();
    assert_eq!(s.files, counts(3, Step::Copied));
    assert_eq!(r.files, counts(3, Step::Copied));
    assert_eq!(r.groups["me"], counts(2, Step::Copied));
    assert_eq!(r.groups["shared"], counts(1, Step::Copied));
    assert_eq!(s.groups["shared"], counts(1, Step::Copied));
    assert_eq!(r.bytes_checked, sizes);
    assert_eq!(r.bytes_written, sizes);
    assert_eq!(r.bytes_already_there, 0);
    assert_eq!(r.checking, None);
    // The sender's counters are the meter's: one count, read two ways.
    assert_eq!(s.blocks_sent, sender.blocks_sent());
    assert_eq!(s.blocks_confirmed, sender.blocks_confirmed());
    assert_eq!(s.blocks_sent, s.blocks_confirmed);
    assert_eq!(s.bytes_sent, sizes);
    assert_eq!(s.bytes_confirmed, sizes);
    for j in &jobs {
        assert_eq!(sender.outcome(j.item), Some(&SendOutcome::Arrived));
    }
}

#[tokio::test]
async fn the_file_being_read_back_is_shown_as_checking_until_it_is_checked() {
    let w = world();
    let jobs = vec![w.file(1, 300_000, "me"), w.file(2, 5000, "shared")];
    let meter = MoveMeter::new();
    let watching = Watching {
        inner: &w.journal,
        meter: meter.clone(),
        at_verified: Mutex::new(Vec::new()),
    };
    let mut sender = SenderSession::new(jobs.clone(), 1);
    let mut receiver =
        ReceiverSession::new(&w.table, plan(&jobs), &watching, "1000").with_meter(meter.clone());
    let (mut a, mut b) = mem_pair();
    let written = receiver.meter();
    let (sent, received) = common::unless_stuck(&written, 305_000, async {
        tokio::join!(sender.run(&mut a), receiver.run(&mut b))
    })
    .await;
    sent.unwrap();
    received.unwrap();
    let at = watching.at_verified.lock().unwrap().clone();
    assert_eq!(at.len(), 2);
    // Each file, at the moment its read-back finished, was the one being checked, counted as
    // checking in its own group, and not yet counted as checked bytes.
    let first = &at[0];
    assert_eq!(first.checking, Some(id(1)));
    assert_eq!(first.groups["me"].checking, 1);
    assert_eq!(first.files.checking, 1);
    assert_eq!(first.bytes_checked, 0);
    let second = &at[1];
    assert_eq!(second.checking, Some(id(2)));
    assert_eq!(second.groups["shared"].checking, 1);
    assert_eq!(second.groups["me"], counts(1, Step::Copied));
    assert_eq!(second.bytes_checked, 300_000);
    let end = meter.reading();
    assert_eq!(end.checking, None);
    assert_eq!(end.files, counts(2, Step::Copied));
}

#[tokio::test]
async fn files_that_did_not_move_are_counted_failed_on_both_laptops_and_identical_ones_already_there()
 {
    let w = world();
    let grew = w.file(1, 1000, "me");
    let there = w.file(2, 4000, "shared");
    let fine = w.file(3, 2000, "me");
    let jobs = vec![grew.clone(), there.clone(), fine.clone()];
    let allowance = plan(&jobs);
    // After the plan was approved, file 1 grew far past what the plan allows.
    std::fs::write(&grew.source, bytes(5 * 1024 * 1024, 1)).unwrap();
    // File 2 is already on the new laptop, identical.
    std::fs::write(w.shared.path().join("f2.bin"), bytes(4000, 2)).unwrap();

    let send_meter = MoveMeter::new();
    let receive_meter = MoveMeter::new();
    let mut sender = SenderSession::new(jobs, 2).with_meter(send_meter.clone());
    let mut receiver = ReceiverSession::new(&w.table, allowance, &w.journal, "1000")
        .with_meter(receive_meter.clone());
    let (mut a, mut b) = mem_pair();
    let written = receiver.meter();
    let (sent, received) = common::unless_stuck(&written, 6000, async {
        tokio::join!(sender.run(&mut a), receiver.run(&mut b))
    })
    .await;
    sent.unwrap();
    received.unwrap();
    assert!(matches!(
        receiver.outcome(grew.item),
        Some(ReceiveOutcome::Failed(_))
    ));
    assert!(matches!(
        receiver.outcome(there.item),
        Some(ReceiveOutcome::AlreadyThere(_))
    ));
    let r = receive_meter.reading();
    let s = send_meter.reading();
    for m in [&r, &s] {
        assert_eq!(m.files.total, 3);
        assert_eq!(m.files.failed, 1, "{m:?}");
        assert_eq!(m.files.already_there, 1, "{m:?}");
        assert_eq!(m.files.copied, 1, "{m:?}");
        assert_eq!(m.groups["me"].failed, 1);
        assert_eq!(m.groups["shared"].already_there, 1);
    }
    assert_eq!(r.bytes_checked, 2000);
    assert_eq!(r.bytes_already_there, 4000);
}

#[tokio::test]
async fn a_stop_ends_the_old_laptops_run_at_once_and_not_one_block_is_sent_after_it() {
    let w = world();
    // 64 blocks of 128 KiB.
    let len = 8 * 1024 * 1024;
    let jobs = vec![w.file(1, len, "me")];
    let meter = MoveMeter::new();
    let control = MoveControl::new();
    let mut sender = SenderSession::new(jobs.clone(), 1)
        .with_in_flight_limit(512 * 1024)
        .with_meter(meter.clone())
        .with_control(control.clone());
    let mut receiver = ReceiverSession::new(&w.table, plan(&jobs), &w.journal, "1000");
    let written = receiver.meter();
    let (a, b) = mem_pair();
    let send_side = async {
        let mut a = a;
        // Its end of the connection closes when it stops, as the app's does.
        sender.run(&mut a).await
    };
    let receive_side = async {
        let mut b = b;
        receiver.run(&mut b).await
    };
    let stopper = async {
        loop {
            let n = meter.reading().blocks_sent;
            if n >= 5 {
                control.stop();
                return n;
            }
            tokio::task::yield_now().await;
        }
    };
    let (sent, received, at_stop) = tokio::time::timeout(Duration::from_secs(60), async {
        tokio::join!(send_side, receive_side, stopper)
    })
    .await
    .expect("a stopped move ends");
    assert!(matches!(sent, Err(TransferError::Stopped)), "{sent:?}");
    assert!(
        matches!(received, Err(TransferError::ConnectionDropped)),
        "{received:?}"
    );
    assert_eq!(meter.reading().blocks_sent, at_stop);
    assert_eq!(sender.blocks_sent(), at_stop);
    assert!(written.load(std::sync::atomic::Ordering::Acquire) < len as u64);
    // Stopped stays stopped: another run ends at once, sending nothing.
    let (mut a2, _b2) = mem_pair();
    let again = tokio::time::timeout(Duration::from_secs(10), sender.run(&mut a2))
        .await
        .expect("ends at once");
    assert!(matches!(again, Err(TransferError::Stopped)), "{again:?}");
    assert_eq!(sender.blocks_sent(), at_stop);
    // The file is not counted as copied anywhere.
    assert_eq!(meter.reading().files.copied, 0);
}

#[test]
fn a_stop_from_another_thread_ends_a_new_laptop_that_is_waiting_for_the_old_one() {
    let w = world();
    let jobs = vec![w.file(1, 1000, "me")];
    let control = MoveControl::new();
    let mut receiver = ReceiverSession::new(&w.table, plan(&jobs), &w.journal, "1000")
        .with_control(control.clone());
    let stopper = {
        let control = control.clone();
        std::thread::spawn(move || {
            std::thread::sleep(Duration::from_millis(200));
            control.stop();
        })
    };
    let rt = tokio::runtime::Builder::new_current_thread()
        .enable_time()
        .build()
        .unwrap();
    let got = rt.block_on(async {
        tokio::time::timeout(Duration::from_secs(30), receiver.run(&mut Silent)).await
    });
    stopper.join().unwrap();
    let got = got.expect("the stop woke the waiting run");
    assert!(matches!(got, Err(TransferError::Stopped)), "{got:?}");
    assert!(control.stopped());
    // A stop set before the run starts ends it before anything is said.
    let again = rt.block_on(async {
        tokio::time::timeout(Duration::from_secs(10), receiver.run(&mut Silent)).await
    });
    assert!(
        matches!(again, Ok(Err(TransferError::Stopped))),
        "{again:?}"
    );
}

/// A connection that never makes the old laptop wait to send, and stops the move from inside a
/// send once `after` blocks have gone: everything the old laptop does next happens in the same
/// turn, before any other future runs.
struct StopsWhileSending {
    inner: Mem,
    meter: MoveMeter,
    control: MoveControl,
    after: u64,
}

impl Channel for StopsWhileSending {
    async fn send(&mut self, data: &[u8]) -> Result<(), ChannelError> {
        if self.meter.reading().blocks_sent >= self.after {
            self.control.stop();
        }
        self.inner.tx.send(data.to_vec()).map_err(|_| ChannelError)
    }
    async fn recv(&mut self) -> Result<Vec<u8>, ChannelError> {
        self.inner.rx.recv().await.ok_or(ChannelError)
    }
}

#[tokio::test]
async fn a_stop_made_while_the_old_laptop_is_busy_sending_stops_the_very_next_block() {
    let w = world();
    // 64 blocks of 128 KiB, all of which fit in the window at once.
    let len = 8 * 1024 * 1024;
    let jobs = vec![w.file(1, len, "me")];
    let meter = MoveMeter::new();
    let control = MoveControl::new();
    let mut sender = SenderSession::new(jobs.clone(), 1)
        .with_meter(meter.clone())
        .with_control(control.clone());
    let mut receiver = ReceiverSession::new(&w.table, plan(&jobs), &w.journal, "1000");
    let (a, b) = mem_pair();
    let send_side = async {
        let mut a = StopsWhileSending {
            inner: a,
            meter: meter.clone(),
            control: control.clone(),
            after: 3,
        };
        sender.run(&mut a).await
    };
    let receive_side = async {
        let mut b = b;
        receiver.run(&mut b).await
    };
    let (sent, _) = tokio::time::timeout(Duration::from_secs(60), async {
        tokio::join!(send_side, receive_side)
    })
    .await
    .expect("a stopped move ends");
    assert!(matches!(sent, Err(TransferError::Stopped)), "{sent:?}");
    // The block being sent when the stop came was finished; not one more was started.
    assert_eq!(meter.reading().blocks_sent, 3);
}

#[tokio::test]
async fn a_file_gone_soon_after_copying_is_counted_failed_and_its_bytes_no_longer_checked() {
    let w = world();
    let jobs = vec![w.file(1, 3000, "me"), w.file(2, 7000, "me")];
    let meter = MoveMeter::new();
    let mut sender = SenderSession::new(jobs.clone(), 2);
    let mut receiver =
        ReceiverSession::new(&w.table, plan(&jobs), &w.journal, "1000").with_meter(meter.clone());
    let (mut a, mut b) = mem_pair();
    let written = receiver.meter();
    let (sent, received) = common::unless_stuck(&written, 10_000, async {
        tokio::join!(sender.run(&mut a), receiver.run(&mut b))
    })
    .await;
    sent.unwrap();
    received.unwrap();
    assert_eq!(meter.reading().bytes_checked, 10_000);
    // Security software removes the second file soon after it landed.
    std::fs::remove_file(w.mine.path().join("f2.bin")).unwrap();
    assert_eq!(receiver.recheck(), vec![id(2)]);
    let r = meter.reading();
    assert_eq!(r.files.copied, 1);
    assert_eq!(r.files.failed, 1);
    assert_eq!(r.groups["me"].failed, 1);
    assert_eq!(r.bytes_checked, 3000);
}
