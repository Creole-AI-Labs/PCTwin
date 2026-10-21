//! Pause and stop in the engine. A pause on either laptop stops new blocks at a block boundary:
//! what is already in flight (at most the in-flight limit, 32 MiB unless set) finishes, the other
//! laptop is told, and nothing treats the quiet as a dropped link. Carrying on picks up exactly
//! where it held, with every byte intact. A stop ends the run at once, during a pause too, and a
//! file cut off is never shown as done. A link that drops during a pause is still a drop.
// Tests make and remove files to test on; the no-file-changes fence is for the shipped code.
#![allow(clippy::disallowed_methods)]

use std::time::Duration;

use pctwin_gate::{Approved, Destinations};
use pctwin_journal::Journal;
use pctwin_record::ItemId;
use pctwin_transfer::{
    Channel, ChannelError, HOLD_SINCE, HOLD_TICK, Message, MoveControl, MoveMeter, ReceiveOutcome,
    ReceiverSession, SendJob, SendOutcome, SenderSession, Tier, TimeLeft, TransferError,
};
use tokio::sync::{mpsc, oneshot};

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

/// A new laptop that takes its time over every message it reads (a slow drive or a busy laptop).
struct Slow(Mem);

impl Channel for Slow {
    async fn send(&mut self, data: &[u8]) -> Result<(), ChannelError> {
        self.0.send(data).await
    }
    async fn recv(&mut self) -> Result<Vec<u8>, ChannelError> {
        tokio::time::sleep(Duration::from_millis(15)).await;
        self.0.recv().await
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
    _jdir: tempfile::TempDir,
    journal: Journal,
    table: Destinations,
}

fn world() -> World {
    let old = tempfile::tempdir().unwrap();
    let mine = tempfile::tempdir().unwrap();
    let jdir = tempfile::tempdir().unwrap();
    let journal = Journal::open(&jdir.path().join("journal.redb")).unwrap();
    let mut table = Destinations::new();
    table
        .approve("me", Approved::MyFolders, mine.path())
        .unwrap();
    World {
        old,
        mine,
        _jdir: jdir,
        journal,
        table,
    }
}

impl World {
    fn file(&self, n: u8, len: usize) -> SendJob {
        let source = self.old.path().join(format!("f{n}.bin"));
        std::fs::write(&source, bytes(len, n)).unwrap();
        SendJob {
            size: len as u64,
            item: id(n),
            source,
            destination: "me".into(),
            path: format!("f{n}.bin"),
            compressible: false,
            tier: Tier::Rest,
        }
    }

    /// The file as it landed under its real name on the new laptop, if it did.
    fn landed(&self, n: u8) -> Option<Vec<u8>> {
        std::fs::read(self.mine.path().join(format!("f{n}.bin"))).ok()
    }
}

fn plan(jobs: &[SendJob]) -> pctwin_transfer::Allowance {
    let sized: Vec<(&SendJob, u64)> = jobs
        .iter()
        .map(|j| (j, std::fs::metadata(&j.source).unwrap().len()))
        .collect();
    common::approved_jobs(&sized)
}

/// Waits until `cond` holds (judged by what happens, never by a deadline on a slow runner).
async fn until(mut cond: impl FnMut() -> bool) {
    common::no_hang(async {
        while !cond() {
            tokio::time::sleep(Duration::from_millis(2)).await;
        }
    })
    .await;
}

/// Long enough for the held old laptop to say it is still held twice.
fn two_ticks() -> Duration {
    HOLD_TICK * 2 + Duration::from_millis(300)
}

/// 64 blocks of 128 KiB.
const BIG: usize = 8 * 1024 * 1024;
const BLOCKS: u64 = 64;
const LIMIT: u64 = 512 * 1024;

/// Everything sent has been confirmed: nothing is in flight.
fn drained(m: &MoveMeter) -> bool {
    let r = m.reading();
    r.blocks_confirmed == r.blocks_sent
}

#[tokio::test]
async fn a_pause_on_the_old_laptop_holds_at_a_block_boundary_tells_the_new_one_and_carries_on_intact()
 {
    let w = world();
    let jobs = vec![w.file(1, BIG)];
    let (sm, rm, control) = (MoveMeter::new(), MoveMeter::new(), MoveControl::new());
    let mut sender = SenderSession::new(jobs.clone(), 1)
        .with_in_flight_limit(LIMIT)
        .with_meter(sm.clone())
        .with_control(control.clone());
    let mut receiver =
        ReceiverSession::new(&w.table, plan(&jobs), &w.journal, "1000").with_meter(rm.clone());
    let (mut a, mut b) = mem_pair();
    let person = async {
        until(|| sm.reading().blocks_sent >= 5).await;
        control.pause();
        let at_pause = sm.reading();
        // In flight when the pause came: at most the limit.
        assert!(
            at_pause.bytes_sent - at_pause.bytes_confirmed <= LIMIT,
            "{at_pause:?}"
        );
        // The new laptop is told, and what was in flight finishes.
        until(|| rm.reading().paused_there).await;
        until(|| drained(&sm)).await;
        let held = sm.reading();
        assert_eq!(
            held.blocks_sent, at_pause.blocks_sent,
            "a block started after the pause"
        );
        assert!(held.paused_here && !held.paused_there, "{held:?}");
        assert!(held.blocks_sent < BLOCKS);
        // Held for a while: nothing moves, and neither laptop gives up or calls it a drop.
        tokio::time::sleep(two_ticks()).await;
        assert_eq!(sm.reading().blocks_sent, held.blocks_sent);
        assert_eq!(sm.snapshot().time_left, TimeLeft::Paused);
        assert_eq!(rm.snapshot().time_left, TimeLeft::Paused);
        control.carry_on();
    };
    let ((sent, received), ()) = common::no_hang(async {
        tokio::join!(
            async { tokio::join!(sender.run(&mut a), receiver.run(&mut b)) },
            person
        )
    })
    .await;
    sent.unwrap();
    received.unwrap();
    assert_eq!(w.landed(1).as_deref(), Some(&bytes(BIG, 1)[..]));
    assert_eq!(sender.outcome(id(1)), Some(&SendOutcome::Arrived));
    assert!(matches!(
        receiver.outcome(id(1)),
        Some(ReceiveOutcome::Finished(_))
    ));
    // Picked up exactly where it held: not one block sent twice.
    assert_eq!(sm.reading().blocks_sent, BLOCKS);
    let (s, r) = (sm.reading(), rm.reading());
    assert!(!s.paused_here && !r.paused_there, "{s:?} {r:?}");
}

#[tokio::test]
async fn a_pause_while_the_new_laptop_is_slow_lets_in_flight_finish_and_still_holds() {
    let w = world();
    let len = 4 * 1024 * 1024;
    let jobs = vec![w.file(1, len)];
    let limit = 1024 * 1024;
    let (sm, rm, control) = (MoveMeter::new(), MoveMeter::new(), MoveControl::new());
    let mut sender = SenderSession::new(jobs.clone(), 1)
        .with_in_flight_limit(limit)
        .with_meter(sm.clone())
        .with_control(control.clone());
    let mut receiver =
        ReceiverSession::new(&w.table, plan(&jobs), &w.journal, "1000").with_meter(rm.clone());
    let (mut a, b) = mem_pair();
    let mut b = Slow(b);
    let person = async {
        // Paused while the new laptop is behind: blocks are sent and not yet confirmed.
        until(|| {
            let r = sm.reading();
            r.blocks_sent >= 10 && r.blocks_confirmed + 2 <= r.blocks_sent
        })
        .await;
        control.pause();
        let at_pause = sm.reading();
        assert!(at_pause.bytes_sent - at_pause.bytes_confirmed <= limit);
        until(|| drained(&sm) && rm.reading().paused_there).await;
        assert_eq!(sm.reading().blocks_sent, at_pause.blocks_sent);
        tokio::time::sleep(two_ticks()).await;
        assert_eq!(sm.reading().blocks_sent, at_pause.blocks_sent);
        control.carry_on();
    };
    let ((sent, received), ()) = common::no_hang(async {
        tokio::join!(
            async { tokio::join!(sender.run(&mut a), receiver.run(&mut b)) },
            person
        )
    })
    .await;
    sent.unwrap();
    received.unwrap();
    assert_eq!(w.landed(1).as_deref(), Some(&bytes(len, 1)[..]));
    assert_eq!(sm.reading().blocks_sent, 32);
}

#[tokio::test]
async fn a_pause_on_the_new_laptop_reaches_the_old_one_which_holds_until_it_carries_on() {
    let w = world();
    let jobs = vec![w.file(1, BIG)];
    let (sm, rm, here) = (MoveMeter::new(), MoveMeter::new(), MoveControl::new());
    let mut sender = SenderSession::new(jobs.clone(), 1)
        .with_in_flight_limit(LIMIT)
        .with_meter(sm.clone());
    let mut receiver = ReceiverSession::new(&w.table, plan(&jobs), &w.journal, "1000")
        .with_meter(rm.clone())
        .with_control(here.clone());
    let (mut a, mut b) = mem_pair();
    let person = async {
        until(|| sm.reading().blocks_sent >= 5).await;
        here.pause();
        until(|| sm.reading().paused_there && drained(&sm)).await;
        let held = sm.reading();
        assert!(held.blocks_sent < BLOCKS);
        assert!(rm.reading().paused_here);
        tokio::time::sleep(two_ticks()).await;
        assert_eq!(sm.reading().blocks_sent, held.blocks_sent);
        assert_eq!(sm.snapshot().time_left, TimeLeft::Paused);
        here.carry_on();
    };
    let ((sent, received), ()) = common::no_hang(async {
        tokio::join!(
            async { tokio::join!(sender.run(&mut a), receiver.run(&mut b)) },
            person
        )
    })
    .await;
    sent.unwrap();
    received.unwrap();
    assert_eq!(w.landed(1).as_deref(), Some(&bytes(BIG, 1)[..]));
    assert_eq!(sm.reading().blocks_sent, BLOCKS);
    assert!(!sm.reading().paused_there && !rm.reading().paused_here);
}

#[tokio::test]
async fn a_stop_mid_file_leaves_what_landed_landed_and_the_cut_file_never_done() {
    let w = world();
    // The small file goes first and lands; the big one is cut off.
    let jobs = vec![w.file(2, 1000), w.file(1, BIG)];
    let (sm, rm, control) = (MoveMeter::new(), MoveMeter::new(), MoveControl::new());
    let mut sender = SenderSession::new(jobs.clone(), 1)
        .with_in_flight_limit(LIMIT)
        .with_meter(sm.clone())
        .with_control(control.clone());
    let mut receiver =
        ReceiverSession::new(&w.table, plan(&jobs), &w.journal, "1000").with_meter(rm.clone());
    let (a, b) = mem_pair();
    let send_side = async {
        // Its end of the connection closes when it stops, as the app's does.
        let mut a = a;
        sender.run(&mut a).await
    };
    let receive_side = async {
        let mut b = b;
        receiver.run(&mut b).await
    };
    let stopper = async {
        until(|| rm.reading().files.copied == 1 && sm.reading().blocks_sent >= 6).await;
        control.stop();
        sm.reading().blocks_sent
    };
    let (sent, received, at_stop) =
        common::no_hang(async { tokio::join!(send_side, receive_side, stopper) }).await;
    assert!(matches!(sent, Err(TransferError::Stopped)), "{sent:?}");
    // Told it was a stop, never a lost connection.
    assert!(
        matches!(received, Err(TransferError::StoppedThere)),
        "{received:?}"
    );
    assert_eq!(sm.reading().blocks_sent, at_stop);
    // What landed is done; what was cut off is not, on either laptop, and has no name.
    assert!(matches!(
        receiver.outcome(id(2)),
        Some(ReceiveOutcome::Finished(_))
    ));
    assert_eq!(w.landed(2).as_deref(), Some(&bytes(1000, 2)[..]));
    assert_eq!(receiver.outcome(id(1)), None);
    assert_eq!(w.landed(1), None);
    assert_eq!(sender.outcome(id(1)), None);
    assert!(!matches!(
        sender.outcome(id(2)),
        Some(SendOutcome::Failed(_))
    ));
    let r = rm.reading();
    assert_eq!((r.files.copied, r.files.copying), (1, 1), "{r:?}");
}

#[tokio::test]
async fn a_stop_during_a_pause_ends_the_run_at_once_and_nothing_more_is_sent() {
    let w = world();
    let jobs = vec![w.file(1, BIG)];
    let (sm, rm, control) = (MoveMeter::new(), MoveMeter::new(), MoveControl::new());
    let mut sender = SenderSession::new(jobs.clone(), 1)
        .with_in_flight_limit(LIMIT)
        .with_meter(sm.clone())
        .with_control(control.clone());
    let mut receiver =
        ReceiverSession::new(&w.table, plan(&jobs), &w.journal, "1000").with_meter(rm.clone());
    let (a, b) = mem_pair();
    let send_side = async {
        let mut a = a;
        sender.run(&mut a).await
    };
    let receive_side = async {
        let mut b = b;
        receiver.run(&mut b).await
    };
    let person = async {
        until(|| sm.reading().blocks_sent >= 5).await;
        control.pause();
        until(|| rm.reading().paused_there && drained(&sm)).await;
        let held = sm.reading().blocks_sent;
        control.stop();
        held
    };
    let (sent, received, held) =
        common::no_hang(async { tokio::join!(send_side, receive_side, person) }).await;
    assert!(matches!(sent, Err(TransferError::Stopped)), "{sent:?}");
    // Told it was a stop, never a lost connection.
    assert!(
        matches!(received, Err(TransferError::StoppedThere)),
        "{received:?}"
    );
    assert_eq!(sm.reading().blocks_sent, held);
    assert_eq!(receiver.outcome(id(1)), None);
    assert_eq!(w.landed(1), None);
    assert_eq!(rm.reading().files.copied, 0);
}

#[tokio::test]
async fn a_stop_on_the_new_laptop_during_its_pause_is_its_stop_and_the_old_one_is_told() {
    let w = world();
    let jobs = vec![w.file(1, BIG)];
    let (sm, here) = (MoveMeter::new(), MoveControl::new());
    let mut sender = SenderSession::new(jobs.clone(), 1)
        .with_in_flight_limit(LIMIT)
        .with_meter(sm.clone());
    let mut receiver =
        ReceiverSession::new(&w.table, plan(&jobs), &w.journal, "1000").with_control(here.clone());
    let (a, b) = mem_pair();
    let send_side = async {
        let mut a = a;
        sender.run(&mut a).await
    };
    let receive_side = async {
        let mut b = b;
        receiver.run(&mut b).await
    };
    let person = async {
        until(|| sm.reading().blocks_sent >= 5).await;
        here.pause();
        until(|| sm.reading().paused_there && drained(&sm)).await;
        here.stop();
    };
    let (sent, received, ()) =
        common::no_hang(async { tokio::join!(send_side, receive_side, person) }).await;
    assert!(
        matches!(received, Err(TransferError::Stopped)),
        "{received:?}"
    );
    assert!(matches!(sent, Err(TransferError::StoppedThere)), "{sent:?}");
    assert_eq!(receiver.outcome(id(1)), None);
    assert_eq!(w.landed(1), None);
}

/// A laptop one version back is never sent `Stopping`: it sees the connection end, as before.
#[tokio::test]
async fn with_a_laptop_on_version_three_a_stop_is_a_drop_there() {
    let w = world();
    let jobs = vec![w.file(1, BIG)];
    let (sm, control) = (MoveMeter::new(), MoveControl::new());
    let mut sender = SenderSession::new(jobs.clone(), 1)
        .with_in_flight_limit(LIMIT)
        .with_meter(sm.clone())
        .with_control(control.clone())
        .with_protocol(3);
    let mut receiver =
        ReceiverSession::new(&w.table, plan(&jobs), &w.journal, "1000").with_protocol(3);
    let (a, b) = mem_pair();
    let send_side = async {
        let mut a = a;
        sender.run(&mut a).await
    };
    let receive_side = async {
        let mut b = b;
        receiver.run(&mut b).await
    };
    let person = async {
        until(|| sm.reading().blocks_sent >= 5).await;
        control.stop();
    };
    let (sent, received, ()) =
        common::no_hang(async { tokio::join!(send_side, receive_side, person) }).await;
    assert!(matches!(sent, Err(TransferError::Stopped)), "{sent:?}");
    assert!(
        matches!(received, Err(TransferError::ConnectionDropped)),
        "{received:?}"
    );
}

#[test]
fn stopping_is_said_only_on_the_main_connection_with_nothing_more() {
    let m = Message::Stopping;
    assert_eq!(Message::decode(&m.encode()).unwrap(), m);
    let good = m.encode();
    let mut on_a_stream = good.clone();
    on_a_stream[4] = 1;
    assert!(Message::decode(&on_a_stream).is_err());
    let mut longer = good.clone();
    longer.push(0);
    assert!(Message::decode(&longer).is_err());
}

#[tokio::test]
async fn a_link_that_drops_during_a_pause_is_a_drop_and_the_next_connection_carries_on_intact() {
    let w = world();
    let jobs = vec![w.file(1, BIG)];
    let (sm, rm, control) = (MoveMeter::new(), MoveMeter::new(), MoveControl::new());
    let mut sender = SenderSession::new(jobs.clone(), 1)
        .with_in_flight_limit(LIMIT)
        .with_meter(sm.clone())
        .with_control(control.clone());
    let mut receiver =
        ReceiverSession::new(&w.table, plan(&jobs), &w.journal, "1000").with_meter(rm.clone());
    let (a, b) = mem_pair();
    let (cut, cut_now) = oneshot::channel::<()>();
    let send_side = async {
        let mut a = a;
        sender.run(&mut a).await
    };
    // The new laptop's end of the link goes away (out of range, lid closed) during the pause.
    let receive_side = async {
        let mut b = b;
        tokio::select! {
            ran = receiver.run(&mut b) => Some(ran),
            _ = cut_now => None,
        }
    };
    let person = async {
        until(|| sm.reading().blocks_sent >= 5).await;
        control.pause();
        until(|| rm.reading().paused_there && drained(&sm)).await;
        let held = sm.reading().blocks_sent;
        cut.send(()).unwrap();
        held
    };
    let (sent, received, held) =
        common::no_hang(async { tokio::join!(send_side, receive_side, person) }).await;
    assert!(received.is_none(), "{received:?}");
    // The held old laptop finds out on its next word to the new one, and calls it a drop.
    assert!(
        matches!(sent, Err(TransferError::ConnectionDropped)),
        "{sent:?}"
    );
    assert_eq!(sm.reading().blocks_sent, held);

    // A new connection, still paused: it holds again (the new laptop is told again), then
    // carries on from exactly where it was.
    let (mut a, mut b) = mem_pair();
    let person = async {
        until(|| rm.reading().paused_there).await;
        tokio::time::sleep(two_ticks()).await;
        assert_eq!(sm.reading().blocks_sent, held);
        control.carry_on();
    };
    let ((sent, received), ()) = common::no_hang(async {
        tokio::join!(
            async { tokio::join!(sender.run(&mut a), receiver.run(&mut b)) },
            person
        )
    })
    .await;
    sent.unwrap();
    received.unwrap();
    assert_eq!(w.landed(1).as_deref(), Some(&bytes(BIG, 1)[..]));
    assert_eq!(sm.reading().blocks_sent, BLOCKS);
}

#[tokio::test]
async fn a_link_that_drops_during_the_new_laptops_pause_is_a_drop_there_too() {
    let w = world();
    let jobs = vec![w.file(1, BIG)];
    let (sm, here) = (MoveMeter::new(), MoveControl::new());
    let mut sender = SenderSession::new(jobs.clone(), 1)
        .with_in_flight_limit(LIMIT)
        .with_meter(sm.clone());
    let mut receiver =
        ReceiverSession::new(&w.table, plan(&jobs), &w.journal, "1000").with_control(here.clone());
    let (a, b) = mem_pair();
    let (cut, cut_now) = oneshot::channel::<()>();
    let send_side = async {
        let mut a = a;
        tokio::select! {
            ran = sender.run(&mut a) => Some(ran),
            _ = cut_now => None,
        }
    };
    let receive_side = async {
        let mut b = b;
        receiver.run(&mut b).await
    };
    let person = async {
        until(|| sm.reading().blocks_sent >= 5).await;
        here.pause();
        until(|| sm.reading().paused_there && drained(&sm)).await;
        cut.send(()).unwrap();
    };
    let (sent, received, ()) =
        common::no_hang(async { tokio::join!(send_side, receive_side, person) }).await;
    assert!(sent.is_none(), "{sent:?}");
    assert!(
        matches!(received, Err(TransferError::ConnectionDropped)),
        "{received:?}"
    );
}

#[tokio::test]
async fn with_a_laptop_on_version_two_a_pause_holds_here_and_says_nothing_it_would_not_understand()
{
    older_peer(2).await
}

#[tokio::test]
async fn with_a_laptop_on_version_three_a_pause_holds_here_and_says_nothing_it_would_not_understand()
 {
    older_peer(3).await
}

/// Both laptops on protocol `old`, which has no `Hold`.
async fn older_peer(old: u16) {
    assert!(old < HOLD_SINCE);
    let w = world();
    let jobs = vec![w.file(1, BIG)];
    let (sm, rm, control, here) = (
        MoveMeter::new(),
        MoveMeter::new(),
        MoveControl::new(),
        MoveControl::new(),
    );
    let mut sender = SenderSession::new(jobs.clone(), 1)
        .with_in_flight_limit(LIMIT)
        .with_meter(sm.clone())
        .with_control(control.clone())
        .with_protocol(old);
    let mut receiver = ReceiverSession::new(&w.table, plan(&jobs), &w.journal, "1000")
        .with_meter(rm.clone())
        .with_control(here.clone())
        .with_protocol(old);
    let (mut a, mut b) = mem_pair();
    let person = async {
        // The new laptop's pause cannot be told: it is not shown as paused, and does not hold.
        here.pause();
        until(|| sm.reading().blocks_sent >= 5).await;
        assert!(!rm.reading().paused_here);
        // The old laptop's own pause still holds it, untold.
        control.pause();
        until(|| drained(&sm)).await;
        let held = sm.reading().blocks_sent;
        tokio::time::sleep(two_ticks()).await;
        assert_eq!(sm.reading().blocks_sent, held);
        assert!(sm.reading().paused_here);
        assert!(!rm.reading().paused_there);
        control.carry_on();
    };
    let ((sent, received), ()) = common::no_hang(async {
        tokio::join!(
            async { tokio::join!(sender.run(&mut a), receiver.run(&mut b)) },
            person
        )
    })
    .await;
    sent.unwrap();
    received.unwrap();
    assert_eq!(w.landed(1).as_deref(), Some(&bytes(BIG, 1)[..]));
}

#[test]
fn hold_is_one_byte_of_meaning_on_the_main_connection_and_anything_else_is_refused() {
    for paused in [true, false] {
        let m = Message::Hold { paused };
        assert_eq!(Message::decode(&m.encode()).unwrap(), m);
    }
    let good = Message::Hold { paused: true }.encode();
    // Only on stream 0, only 0 or 1, nothing more.
    let mut on_a_stream = good.clone();
    on_a_stream[4] = 1;
    assert!(Message::decode(&on_a_stream).is_err());
    let mut bad_flag = good.clone();
    *bad_flag.last_mut().unwrap() = 2;
    assert!(Message::decode(&bad_flag).is_err());
    let mut longer = good.clone();
    longer.push(0);
    assert!(Message::decode(&longer).is_err());
    assert!(Message::decode(&good[..good.len() - 1]).is_err());
}

#[tokio::test]
async fn a_move_paused_before_it_starts_opens_and_starts_nothing_until_it_carries_on() {
    let w = world();
    let jobs = vec![w.file(2, 1000), w.file(1, BIG)];
    let (sm, rm, control) = (MoveMeter::new(), MoveMeter::new(), MoveControl::new());
    control.pause();
    let mut sender = SenderSession::new(jobs.clone(), 2)
        .with_in_flight_limit(LIMIT)
        .with_meter(sm.clone())
        .with_control(control.clone());
    let mut receiver =
        ReceiverSession::new(&w.table, plan(&jobs), &w.journal, "1000").with_meter(rm.clone());
    let (mut a, mut b) = mem_pair();
    let person = async {
        until(|| rm.reading().paused_there).await;
        tokio::time::sleep(two_ticks()).await;
        // Held from the start: no file was started on the new laptop, and nothing was sent.
        let (s, r) = (sm.reading(), rm.reading());
        assert_eq!(s.blocks_sent, 0);
        assert_eq!((r.files.waiting, r.files.copying), (2, 0), "{r:?}");
        assert_eq!(s.files.copying, 0, "{s:?}");
        control.carry_on();
    };
    let ((sent, received), ()) = common::no_hang(async {
        tokio::join!(
            async { tokio::join!(sender.run(&mut a), receiver.run(&mut b)) },
            person
        )
    })
    .await;
    sent.unwrap();
    received.unwrap();
    assert_eq!(w.landed(1).as_deref(), Some(&bytes(BIG, 1)[..]));
    assert_eq!(w.landed(2).as_deref(), Some(&bytes(1000, 2)[..]));
}

// ------------------------------------------------ pause with extra lanes (review, 21 October)

/// Extra lanes for both laptops.
fn lanes(n: usize) -> (Vec<Mem>, Vec<Mem>) {
    let (mut old, mut new) = (Vec::new(), Vec::new());
    for _ in 0..n {
        let (x, y) = mem_pair();
        old.push(x);
        new.push(y);
    }
    (old, new)
}

/// Eight rounds of pause, drain, hold, carry on, with three extra lanes, both laptops on protocol
/// `v`: nothing is sent while held, every carrying on wakes every lane, and both files land.
async fn lanes_pause(v: u16) {
    let w = world();
    let len = 16 * 1024 * 1024;
    let small = 3 * 1024 * 1024;
    let jobs = vec![w.file(1, len), w.file(2, small)];
    let (sm, control) = (MoveMeter::new(), MoveControl::new());
    let mut sender = SenderSession::new(jobs.clone(), 2)
        .with_in_flight_limit(LIMIT)
        .with_meter(sm.clone())
        .with_control(control.clone())
        .with_protocol(v);
    let mut receiver =
        ReceiverSession::new(&w.table, plan(&jobs), &w.journal, "1000").with_protocol(v);
    let written = receiver.meter();
    let (mut a, mut b) = mem_pair();
    let (old_lanes, new_lanes) = lanes(3);
    let person = async {
        // Rounds spread over the whole move, so some come after the old laptop's own share of
        // the big file is sent and only the lanes have blocks left.
        for round in 0..8u64 {
            until(|| sm.reading().blocks_sent >= 8 + round * 16).await;
            control.pause();
            until(|| drained(&sm)).await;
            let held = sm.reading().blocks_sent;
            tokio::time::sleep(HOLD_TICK + Duration::from_millis(300)).await;
            assert_eq!(
                sm.reading().blocks_sent,
                held,
                "v{v}: a block sent while held"
            );
            control.carry_on();
        }
    };
    let ((sent, received), ()) = common::unless_stuck(&written, (len + small) as u64, async {
        tokio::join!(
            async {
                tokio::join!(
                    sender.run_lanes(&mut a, old_lanes),
                    receiver.run_lanes(&mut b, new_lanes)
                )
            },
            person
        )
    })
    .await;
    sent.unwrap();
    received.unwrap();
    assert_eq!(w.landed(1).as_deref(), Some(&bytes(len, 1)[..]));
    assert_eq!(w.landed(2).as_deref(), Some(&bytes(small, 2)[..]));
}

#[tokio::test]
async fn with_extra_lanes_every_carrying_on_wakes_every_lane_on_version_two() {
    lanes_pause(2).await
}

#[tokio::test]
async fn with_extra_lanes_every_carrying_on_wakes_every_lane_on_version_three() {
    lanes_pause(3).await
}

#[tokio::test]
async fn with_extra_lanes_every_carrying_on_wakes_every_lane_on_this_version() {
    lanes_pause(HOLD_SINCE).await
}

/// Many quick pause and carry-on taps (a double-tap: pause, a few turns of the other tasks or
/// none, carry on) with three extra lanes on protocol `v`: after every tap the move moves again.
async fn taps(v: u16) {
    let w = world();
    let len = 48 * 1024 * 1024;
    let jobs = vec![w.file(1, len)];
    let (sm, control) = (MoveMeter::new(), MoveControl::new());
    let mut sender = SenderSession::new(jobs.clone(), 1)
        .with_in_flight_limit(LIMIT)
        .with_meter(sm.clone())
        .with_control(control.clone())
        .with_protocol(v);
    let mut receiver =
        ReceiverSession::new(&w.table, plan(&jobs), &w.journal, "1000").with_protocol(v);
    let written = receiver.meter();
    let (mut a, mut b) = mem_pair();
    let (old_lanes, new_lanes) = lanes(3);
    let person = async {
        let mut seed = 12345u64;
        for _ in 0..300 {
            let before = sm.reading().blocks_sent;
            if before >= 380 {
                break;
            }
            seed = seed
                .wrapping_mul(6364136223846793005)
                .wrapping_add(1442695040888963407);
            control.pause();
            for _ in 0..(seed >> 33) % 12 {
                tokio::task::yield_now().await;
            }
            control.carry_on();
            until(|| sm.reading().blocks_sent > before || sm.reading().files.copied == 1).await;
        }
    };
    let ((sent, received), ()) = common::unless_stuck(&written, len as u64, async {
        tokio::join!(
            async {
                tokio::join!(
                    sender.run_lanes(&mut a, old_lanes),
                    receiver.run_lanes(&mut b, new_lanes)
                )
            },
            person
        )
    })
    .await;
    sent.unwrap();
    received.unwrap();
    assert_eq!(w.landed(1).as_deref(), Some(&bytes(len, 1)[..]));
}

#[tokio::test]
async fn quick_pause_and_carry_on_taps_never_leave_a_lane_asleep_on_this_version() {
    taps(HOLD_SINCE).await
}

#[tokio::test]
async fn quick_pause_and_carry_on_taps_never_leave_a_lane_asleep_on_version_two() {
    taps(2).await
}

/// The old laptop's main connection, counting each time it says "still paused" twice in a row:
/// only its one-second tick does that, so between quick taps it means the main loop slept through
/// a carrying on.
struct Spy {
    inner: Mem,
    last_paused: bool,
    repeats: std::sync::Arc<std::sync::atomic::AtomicU32>,
}

impl Channel for Spy {
    async fn send(&mut self, data: &[u8]) -> Result<(), ChannelError> {
        if let Ok(Message::Hold { paused }) = Message::decode(data) {
            if paused && self.last_paused {
                self.repeats
                    .fetch_add(1, std::sync::atomic::Ordering::SeqCst);
            }
            self.last_paused = paused;
        }
        self.inner.send(data).await
    }
    async fn recv(&mut self) -> Result<Vec<u8>, ChannelError> {
        self.inner.recv().await
    }
}

/// With no extra lanes the main loop alone must notice every carrying on at once: it looks at
/// the pause before it decides to hold, so a carrying on made in between is never slept through
/// (a mutant that dropped that early look survived the tests with lanes, which hold on their own).
#[tokio::test]
async fn quick_taps_never_leave_the_main_connection_asleep() {
    let w = world();
    let len = 16 * 1024 * 1024;
    let jobs = vec![w.file(1, len)];
    let (sm, control) = (MoveMeter::new(), MoveControl::new());
    let mut sender = SenderSession::new(jobs.clone(), 1)
        .with_in_flight_limit(LIMIT)
        .with_meter(sm.clone())
        .with_control(control.clone())
        .with_protocol(HOLD_SINCE);
    let mut receiver =
        ReceiverSession::new(&w.table, plan(&jobs), &w.journal, "1000").with_protocol(HOLD_SINCE);
    let written = receiver.meter();
    let (a, mut b) = mem_pair();
    let repeats = std::sync::Arc::new(std::sync::atomic::AtomicU32::new(0));
    let mut a = Spy {
        inner: a,
        last_paused: false,
        repeats: repeats.clone(),
    };
    let person = async {
        let mut seed = 99u64;
        for _ in 0..200 {
            let before = sm.reading().blocks_sent;
            if sm.reading().files.copied == 1 {
                break;
            }
            seed = seed
                .wrapping_mul(6364136223846793005)
                .wrapping_add(1442695040888963407);
            control.pause();
            for _ in 0..(seed >> 33) % 12 {
                tokio::task::yield_now().await;
            }
            control.carry_on();
            until(|| sm.reading().blocks_sent > before || sm.reading().files.copied == 1).await;
        }
    };
    let ((sent, received), ()) = common::unless_stuck(&written, len as u64, async {
        tokio::join!(
            async { tokio::join!(sender.run(&mut a), receiver.run(&mut b)) },
            person
        )
    })
    .await;
    sent.unwrap();
    received.unwrap();
    assert_eq!(w.landed(1).as_deref(), Some(&bytes(len, 1)[..]));
    assert_eq!(
        repeats.load(std::sync::atomic::Ordering::SeqCst),
        0,
        "the main loop slept through a carrying on until its tick"
    );
}

// ------------------------------------------------ a hostile other laptop

/// A new laptop on protocol `v`, given `msgs` by an old laptop right after its hello: how its
/// run ended, and how many `Hold`s it said.
async fn new_laptop_given(v: u16, msgs: Vec<Message>) -> (Result<(), TransferError>, usize) {
    let w = world();
    let jobs = vec![w.file(1, 1000)];
    let mut receiver =
        ReceiverSession::new(&w.table, plan(&jobs), &w.journal, "1000").with_protocol(v);
    let (mut a, mut b) = mem_pair();
    let old = async move {
        while Message::decode(&a.recv().await.unwrap()).unwrap() != Message::Ready {}
        for m in &msgs {
            if a.send(&m.encode()).await.is_err() {
                break;
            }
        }
        // Whatever it says until it ends its run (closing its end).
        let mut holds = 0;
        while let Ok(m) = a.recv().await {
            if matches!(Message::decode(&m), Ok(Message::Hold { .. })) {
                holds += 1;
            }
        }
        holds
    };
    let (ran, holds) = common::no_hang(async {
        let new = async {
            let ran = receiver.run(&mut b).await;
            drop(b);
            ran
        };
        tokio::join!(new, old)
    })
    .await;
    (ran, holds)
}

#[tokio::test]
async fn a_hold_to_a_new_laptop_on_an_older_version_is_refused_and_never_answered() {
    for v in [2, 3] {
        let (ran, holds) = new_laptop_given(v, vec![Message::Hold { paused: true }]).await;
        assert!(
            matches!(ran, Err(TransferError::Protocol(_))),
            "v{v}: {ran:?}"
        );
        assert_eq!(holds, 0, "v{v}");
    }
    // On this version it is answered (and the old laptop then ends the move).
    let (ran, holds) = new_laptop_given(
        HOLD_SINCE,
        vec![Message::Hold { paused: true }, Message::AllSent],
    )
    .await;
    assert!(ran.is_ok(), "{ran:?}");
    assert_eq!(holds, 1);
}

/// An extra lane of a new laptop that answers its first whole block with a `Hold` instead of a
/// receipt, then keeps reading and says nothing more.
async fn lane_that_says_hold(mut lane: Mem) -> bool {
    let mut said = false;
    while let Ok(m) = lane.recv().await {
        if !said && matches!(Message::decode(&m), Ok(Message::Piece { last: true, .. })) {
            said = true;
            if lane
                .send(&Message::Hold { paused: true }.encode())
                .await
                .is_err()
            {
                return said;
            }
        }
    }
    said
}

#[tokio::test]
async fn a_hold_on_an_extra_lane_is_refused_and_never_pauses_the_old_laptop() {
    let w = world();
    // Two files in flight at once, so the extra lane takes one of its own.
    let len = 8 * 1024 * 1024;
    let small = 3 * 1024 * 1024;
    let jobs = vec![w.file(1, len), w.file(2, small)];
    let sm = MoveMeter::new();
    let mut sender = SenderSession::new(jobs.clone(), 2)
        .with_in_flight_limit(LIMIT)
        .with_meter(sm.clone());
    let mut receiver = ReceiverSession::new(&w.table, plan(&jobs), &w.journal, "1000");
    let written = receiver.meter();
    let (mut a, mut b) = mem_pair();
    let (x, y) = mem_pair();
    let mut seen_paused = false;
    let watch = async {
        while sm.reading().files.copied < 2 {
            seen_paused |= sm.reading().paused_there;
            tokio::task::yield_now().await;
        }
    };
    let ((sent, received, said), ()) =
        common::unless_stuck(&written, (len + small) as u64, async {
            tokio::join!(
                async {
                    tokio::join!(
                        sender.run_lanes(&mut a, vec![x]),
                        receiver.run(&mut b),
                        lane_that_says_hold(y)
                    )
                },
                watch
            )
        })
        .await;
    sent.unwrap();
    received.unwrap();
    assert!(said, "the lane was given a block to answer");
    assert!(
        !seen_paused,
        "a Hold on an extra lane paused the old laptop"
    );
    assert_eq!(w.landed(1).as_deref(), Some(&bytes(len, 1)[..]));
    assert_eq!(w.landed(2).as_deref(), Some(&bytes(small, 2)[..]));
}

/// One double-tap from another thread (pause, a few microseconds, carry on) once `at_block`
/// blocks are sent, with three extra lanes on protocol `v`; then no more taps: the move ends.
async fn one_double_tap(v: u16, spin_us: u64, at_block: u64) -> Result<(), TransferError> {
    let w = world();
    let len = 6 * 1024 * 1024;
    let jobs = vec![w.file(1, len)];
    let (sm, control) = (MoveMeter::new(), MoveControl::new());
    let mut sender = SenderSession::new(jobs.clone(), 1)
        .with_in_flight_limit(LIMIT)
        .with_meter(sm.clone())
        .with_control(control.clone())
        .with_protocol(v);
    let mut receiver =
        ReceiverSession::new(&w.table, plan(&jobs), &w.journal, "1000").with_protocol(v);
    let written = receiver.meter();
    let (mut a, mut b) = mem_pair();
    let (old_lanes, new_lanes) = lanes(3);
    let (c2, m2) = (control.clone(), sm.clone());
    let tapper = std::thread::spawn(move || {
        while m2.reading().blocks_sent < at_block {
            std::hint::spin_loop();
        }
        c2.pause();
        let t = std::time::Instant::now();
        while t.elapsed() < Duration::from_micros(spin_us) {
            std::hint::spin_loop();
        }
        c2.carry_on();
    });
    let (sent, received) = common::unless_stuck(&written, len as u64, async {
        tokio::join!(
            sender.run_lanes(&mut a, old_lanes),
            receiver.run_lanes(&mut b, new_lanes)
        )
    })
    .await;
    tapper.join().unwrap();
    sent?;
    received?;
    assert_eq!(w.landed(1).as_deref(), Some(&bytes(len, 1)[..]));
    Ok(())
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn a_double_tap_from_another_thread_never_leaves_the_move_asleep() {
    for v in [2, HOLD_SINCE] {
        for i in 0..30u64 {
            one_double_tap(v, (i * 37) % 400, 3 + (i * 7) % 40)
                .await
                .unwrap();
        }
    }
}
