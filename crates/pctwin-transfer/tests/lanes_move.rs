//! A whole move over several lanes (Security Design A, "Lanes share one plan"): one plan feeds
//! every lane; a big file's sections go over several lanes at once and arrive whole; a lane that
//! drops loses nothing; the main connection dropping is picked up again over new lanes; a file
//! that changes while it is being sent arrives whole and current, never mixed with blocks of the
//! attempt before.
// Tests make and remove files to test on; the no-file-changes fence is for the shipped code.
#![allow(clippy::disallowed_methods)]

use std::collections::{BTreeSet, HashMap};
use std::path::PathBuf;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex};

use pctwin_gate::{Approved, Destinations};
use pctwin_record::ItemId;
use pctwin_transfer::{
    Channel, ChannelError, Message, ReceiverSession, SendJob, SendOutcome, SenderSession, Tier,
    block_size_for,
};
use tokio::sync::mpsc;

mod common;

const MIB: usize = 1024 * 1024;

type Hook = Box<dyn FnMut() + Send>;

/// One end of an in-memory connection that can be cut (both ends then fail, after what was already
/// delivered), counting the streams whose pieces it sent.
struct Mem {
    tx: mpsc::UnboundedSender<Vec<u8>>,
    rx: mpsc::UnboundedReceiver<Vec<u8>>,
    cut: Arc<AtomicBool>,
    sends: usize,
    cut_at: Option<usize>,
    hook: Option<(usize, Hook)>,
    streams: Arc<Mutex<BTreeSet<u32>>>,
    /// Each file started on this end, with the stream it was started on.
    starts: Arc<Mutex<Vec<(ItemId, u32)>>>,
}

fn pair() -> (Mem, Mem) {
    let (a_tx, b_rx) = mpsc::unbounded_channel();
    let (b_tx, a_rx) = mpsc::unbounded_channel();
    let cut = Arc::new(AtomicBool::new(false));
    let end = |tx, rx| Mem {
        tx,
        rx,
        cut: cut.clone(),
        sends: 0,
        cut_at: None,
        hook: None,
        streams: Arc::default(),
        starts: Arc::default(),
    };
    (end(a_tx, a_rx), end(b_tx, b_rx))
}

impl Channel for Mem {
    async fn send(&mut self, data: &[u8]) -> Result<(), ChannelError> {
        tokio::task::yield_now().await;
        self.sends += 1;
        if let Some((at, hook)) = &mut self.hook
            && *at == self.sends
        {
            hook();
        }
        if self.cut_at == Some(self.sends) {
            self.cut.store(true, Ordering::SeqCst);
        }
        if self.cut.load(Ordering::SeqCst) {
            return Err(ChannelError);
        }
        match Message::decode(data) {
            Ok(Message::Piece { stream, .. }) => {
                self.streams.lock().unwrap().insert(stream);
            }
            Ok(Message::StartFile { stream, item, .. }) => {
                self.starts.lock().unwrap().push((item, stream));
            }
            _ => {}
        }
        self.tx.send(data.to_vec()).map_err(|_| ChannelError)
    }

    async fn recv(&mut self) -> Result<Vec<u8>, ChannelError> {
        loop {
            if let Ok(m) = self.rx.try_recv() {
                return Ok(m);
            }
            if self.cut.load(Ordering::SeqCst) {
                return Err(ChannelError);
            }
            tokio::time::sleep(std::time::Duration::from_millis(1)).await;
        }
    }
}

fn pattern(len: usize, seed: u32) -> Vec<u8> {
    let mut x = seed.max(1);
    (0..len)
        .map(|_| {
            x ^= x << 13;
            x ^= x >> 17;
            x ^= x << 5;
            (x & 0xFF) as u8
        })
        .collect()
}

fn id(n: u8) -> ItemId {
    ItemId::from_hex(&format!("{n:02x}{}", "0".repeat(30))).unwrap()
}

struct Laptops {
    _old: tempfile::TempDir,
    new: tempfile::TempDir,
    files: Vec<(PathBuf, Vec<u8>)>,
}

/// A big file (80 MiB: enough to split into two sections) and a few small ones.
fn laptops() -> Laptops {
    let old = tempfile::tempdir().unwrap();
    let files = [(80 * MIB, 7u32), (300_000, 1), (40_000, 2), (2_000_000, 3)]
        .iter()
        .enumerate()
        .map(|(i, (len, seed))| {
            let p = old.path().join(format!("f{i}.bin"));
            let bytes = pattern(*len, *seed);
            std::fs::write(&p, &bytes).unwrap();
            (p, bytes)
        })
        .collect();
    Laptops {
        _old: old,
        new: tempfile::tempdir().unwrap(),
        files,
    }
}

fn table(l: &Laptops) -> Destinations {
    let mut t = Destinations::new();
    t.approve("me", Approved::MyFolders, l.new.path()).unwrap();
    t
}

fn jobs(l: &Laptops) -> Vec<SendJob> {
    (0..l.files.len())
        .map(|n| SendJob {
            item: id(n as u8),
            source: l.files[n].0.clone(),
            destination: "me".into(),
            path: format!("f{n}.bin"),
            compressible: false,
            tier: Tier::Rest,
        })
        .collect()
}

fn plan(l: &Laptops) -> pctwin_transfer::Allowance {
    let files: Vec<(ItemId, u64)> = (0..l.files.len())
        .map(|n| (id(n as u8), l.files[n].1.len() as u64))
        .collect();
    common::approved(&files)
}

fn total_blocks(l: &Laptops) -> u64 {
    l.files
        .iter()
        .map(|(_, b)| (b.len() as u64).div_ceil(block_size_for(b.len() as u64)))
        .sum()
}

fn assert_arrived(l: &Laptops) {
    for (n, (_, bytes)) in l.files.iter().enumerate() {
        let got = std::fs::read(l.new.path().join(format!("f{n}.bin"))).unwrap();
        assert!(got == *bytes, "file {n} differs");
    }
}

/// Connections for a move: the main one and `extra` lanes, as (old laptop's ends, new laptop's).
fn connections(extra: usize) -> (Mem, Vec<Mem>, Mem, Vec<Mem>) {
    let (old_main, new_main) = pair();
    let (old_lanes, new_lanes): (Vec<Mem>, Vec<Mem>) = (0..extra).map(|_| pair()).unzip();
    (old_main, old_lanes, new_main, new_lanes)
}

async fn run(
    sender: &mut SenderSession,
    receiver: &mut ReceiverSession<'_>,
    (mut old_main, old_lanes, mut new_main, new_lanes): (Mem, Vec<Mem>, Mem, Vec<Mem>),
) -> (bool, bool) {
    let (s, r) = tokio::time::timeout(std::time::Duration::from_secs(120), async {
        tokio::join!(
            sender.run_lanes(&mut old_main, old_lanes),
            receiver.run_lanes(&mut new_main, new_lanes)
        )
    })
    .await
    .expect("the move hung");
    (s.is_ok(), r.is_ok())
}

#[tokio::test]
async fn a_big_file_goes_over_several_lanes_at_once_and_everything_arrives() {
    let l = laptops();
    let table = table(&l);
    let mut sender = SenderSession::new(jobs(&l), 4);
    let mut receiver = ReceiverSession::new(&table, plan(&l), common::journal(), "1001");
    let conns = connections(2);
    let carried: Vec<_> = std::iter::once(&conns.0)
        .chain(&conns.1)
        .map(|m| m.streams.clone())
        .collect();
    assert_eq!(run(&mut sender, &mut receiver, conns).await, (true, true));
    assert_arrived(&l);
    for n in 0..l.files.len() {
        assert_eq!(sender.outcome(id(n as u8)), Some(&SendOutcome::Arrived));
    }
    // The big file (stream 0) went over more than one lane. (Smaller files are never split, so a
    // lane may rightly find nothing left for it.)
    let lanes_with_big = carried
        .iter()
        .filter(|s| s.lock().unwrap().contains(&0))
        .count();
    assert!(
        lanes_with_big >= 2,
        "the big file used {lanes_with_big} lane(s)"
    );
}

#[tokio::test]
async fn an_extra_lane_that_drops_mid_move_loses_nothing() {
    let l = laptops();
    let table = table(&l);
    let mut sender = SenderSession::new(jobs(&l), 4);
    let mut receiver = ReceiverSession::new(&table, plan(&l), common::journal(), "1001");
    let mut conns = connections(2);
    // The extra lanes drop after carrying some pieces (some blocks unconfirmed). Which lane gets
    // work varies, so both are set to drop; at least one really does.
    for lane in &mut conns.1 {
        lane.cut_at = Some(60);
    }
    let dropped: Vec<_> = conns.1.iter().map(|m| m.cut.clone()).collect();
    assert_eq!(run(&mut sender, &mut receiver, conns).await, (true, true));
    assert!(
        dropped.iter().any(|d| d.load(Ordering::SeqCst)),
        "a lane really dropped"
    );
    assert_arrived(&l);
    // The blocks the lane had not had confirmed were really sent again on the others.
    assert!(sender.blocks_sent() > total_blocks(&l));
}

#[tokio::test]
async fn the_main_connection_dropping_is_picked_up_again_over_new_lanes() {
    let l = laptops();
    let table = table(&l);
    let mut sender = SenderSession::new(jobs(&l), 4);
    let mut receiver = ReceiverSession::new(&table, plan(&l), common::journal(), "1001");
    let mut conns = connections(2);
    // Early, while the big file is still on its way (the main connection's 8th message).
    conns.0.cut_at = Some(8);
    // Every lane goes with it, as when the Wi-Fi drops.
    let cut = conns.0.cut.clone();
    for lane in &mut conns.1 {
        lane.cut = cut.clone();
    }
    for lane in &mut conns.3 {
        lane.cut = cut.clone();
    }
    assert_eq!(run(&mut sender, &mut receiver, conns).await, (false, false));
    let before = sender.blocks_sent();
    assert!(before > 0);
    assert_eq!(
        run(&mut sender, &mut receiver, connections(2)).await,
        (true, true)
    );
    assert_arrived(&l);
    // Picked up where it stopped, not sent again from the start.
    assert!(receiver.continued() >= 1);
}

#[tokio::test]
async fn a_file_changed_while_sent_over_lanes_arrives_whole_and_current() {
    let l = laptops();
    let table = table(&l);
    let mut sender = SenderSession::new(jobs(&l), 4);
    let mut receiver = ReceiverSession::new(&table, plan(&l), common::journal(), "1001");
    let mut conns = connections(2);
    let starts = conns.0.starts.clone();
    // Partway through, another program saves a new version of the big file (same size).
    let big = l.files[0].0.clone();
    let newer = pattern(80 * MIB, 99);
    let written = newer.clone();
    // On the main connection, which always carries blocks.
    conns.0.hook = Some((
        8,
        Box::new(move || {
            std::fs::write(&big, &written).unwrap();
            let f = std::fs::OpenOptions::new().write(true).open(&big).unwrap();
            f.set_modified(std::time::SystemTime::now() + std::time::Duration::from_secs(5))
                .unwrap();
        }),
    ));
    assert_eq!(run(&mut sender, &mut receiver, conns).await, (true, true));
    let got = std::fs::read(l.new.path().join("f0.bin")).unwrap();
    assert!(got == newer, "the copy is not the current version");
    // The change was really noticed and the file sent again ...
    assert!(sender.blocks_sent() > total_blocks(&l));
    // ... as a new attempt on a new stream, so no block of the first attempt still travelling
    // on a slow lane could ever be written into the second.
    let big: Vec<u32> = starts
        .lock()
        .unwrap()
        .iter()
        .filter(|(item, _)| *item == id(0))
        .map(|(_, s)| *s)
        .collect();
    assert_eq!(big.len(), 2, "{big:?}");
    assert_ne!(big[0], big[1]);
    assert_eq!(sender.outcome(id(0)), Some(&SendOutcome::Arrived));
}

#[tokio::test]
async fn one_lane_and_several_lanes_move_the_same_files() {
    // Several lanes are only faster, never different: the same files, the same outcomes.
    let l = laptops();
    let t1 = table(&l);
    let mut sender = SenderSession::new(jobs(&l), 4);
    let mut receiver = ReceiverSession::new(&t1, plan(&l), common::journal(), "1001");
    assert_eq!(
        run(&mut sender, &mut receiver, connections(0)).await,
        (true, true)
    );
    assert_arrived(&l);
    let one: HashMap<u8, Option<SendOutcome>> = (0..4u8)
        .map(|n| (n, sender.outcome(id(n)).cloned()))
        .collect();
    let l2 = laptops();
    let table2 = table(&l2);
    let mut sender2 = SenderSession::new(jobs(&l2), 4);
    let mut receiver2 = ReceiverSession::new(&table2, plan(&l2), common::journal(), "1001");
    assert_eq!(
        run(&mut sender2, &mut receiver2, connections(3)).await,
        (true, true)
    );
    assert_arrived(&l2);
    for n in 0..4u8 {
        assert_eq!(sender2.outcome(id(n)).cloned(), one[&n]);
    }
}

#[tokio::test]
async fn a_lane_that_joins_during_the_move_takes_a_share_of_it() {
    // Many small files, so there is always work left for a lane that joins late.
    let old = tempfile::tempdir().unwrap();
    let files: Vec<(PathBuf, Vec<u8>)> = (0..40u32)
        .map(|i| {
            let p = old.path().join(format!("f{i}.bin"));
            let bytes = pattern(300_000, i + 1);
            std::fs::write(&p, &bytes).unwrap();
            (p, bytes)
        })
        .collect();
    let l = Laptops {
        _old: old,
        new: tempfile::tempdir().unwrap(),
        files,
    };
    let table = table(&l);
    let mut sender = SenderSession::new(jobs(&l), 8);
    let mut receiver = ReceiverSession::new(&table, plan(&l), common::journal(), "1001");
    let (mut old_main, mut new_main) = pair();
    let (old_in, mut old_joining) = mpsc::unbounded_channel();
    let (new_in, mut new_joining) = mpsc::unbounded_channel();
    let (old_lane, new_lane) = pair();
    let carried = old_lane.streams.clone();
    // The lane is opened while the move is under way: at the main connection's 20th message.
    let mut lanes = Some((old_lane, new_lane));
    old_main.hook = Some((
        20,
        Box::new(move || {
            if let Some((o, n)) = lanes.take() {
                new_in.send(n).unwrap();
                old_in.send(o).unwrap();
            }
        }),
    ));
    let (s, r) = tokio::time::timeout(std::time::Duration::from_secs(60), async {
        tokio::join!(
            sender.run_joining(&mut old_main, &mut old_joining),
            receiver.run_joining(&mut new_main, &mut new_joining)
        )
    })
    .await
    .expect("the move hung");
    s.unwrap();
    r.unwrap();
    assert_arrived(&l);
    assert!(
        !carried.lock().unwrap().is_empty(),
        "the late lane carried nothing"
    );
}

#[tokio::test]
async fn a_whole_move_over_real_lanes_that_the_driver_opens_and_closes() {
    use std::sync::atomic::AtomicU64;
    use std::time::Instant;
    let l = laptops();
    let table = table(&l);
    let config = pctwin_link::LinkConfig {
        step_timeout: std::time::Duration::from_secs(5),
        handshake_timeout: std::time::Duration::from_secs(5),
        silence_penalty: std::time::Duration::ZERO,
        connect_timeout: std::time::Duration::from_secs(5),
    };
    // Pair over a real connection, as the app does.
    let host = pctwin_link::Host::bind("127.0.0.1:0".parse().unwrap(), config)
        .await
        .unwrap();
    let addr = host.local_addr().unwrap();
    let rotating = Arc::new(Mutex::new(
        pctwin_pairing::RotatingSender::new(Instant::now()).unwrap(),
    ));
    let code = {
        let mut s = rotating.lock().unwrap();
        s.tick(Instant::now()).unwrap();
        pctwin_pairing::PairingCode::parse(&s.code().unwrap()).unwrap()
    };
    let guest =
        tokio::spawn(async move { pctwin_link::connect(addr, &code, config).await.unwrap() });
    let pending_host = host.next_peer(&rotating).await.unwrap();
    let pending_guest = guest.await.unwrap();
    let number = pending_guest.match_number();
    let mut old_link = pending_host.choose(number, Instant::now()).await.unwrap();
    let mut new_link = pending_guest.approval().await.unwrap();

    // The old laptop serves lanes; the new laptop's driver opens them as the tuner asks.
    let old_keys = old_link.take_lane_keys().unwrap();
    let mut listener = host.into_lanes(old_keys, old_link.peer_addr());
    let mut opener = pctwin_transfer::LinkLanes::new(&mut new_link, config).unwrap();
    let (old_tx, mut old_in) = mpsc::unbounded_channel();
    let (new_tx, mut new_in) = mpsc::unbounded_channel();
    let mut sender = SenderSession::new(jobs(&l), 4);
    let mut receiver = ReceiverSession::new(&table, plan(&l), common::journal(), "1001");
    let meter: Arc<AtomicU64> = receiver.meter();
    let done = Arc::new(AtomicBool::new(false));
    let mut driver = pctwin_transfer::LaneDriver::new(std::time::Duration::from_millis(100));

    let old_side = async {
        tokio::select! {
            sent = sender.run_joining(&mut old_link, &mut old_in) => sent,
            () = pctwin_transfer::accept_lanes(&mut listener, &old_tx) => {
                panic!("the old laptop stopped accepting lanes during the move")
            }
        }
    };
    let new_side = async {
        let received = receiver.run_joining(&mut new_link, &mut new_in).await;
        done.store(true, Ordering::SeqCst);
        received
    };
    let driving = driver.run(
        &mut opener,
        &new_tx,
        |_| meter.load(Ordering::SeqCst),
        || done.load(Ordering::SeqCst),
    );
    let (sent, received, ()) = tokio::time::timeout(std::time::Duration::from_secs(120), async {
        tokio::join!(old_side, new_side, driving)
    })
    .await
    .expect("the move hung");
    sent.unwrap();
    received.unwrap();
    assert_arrived(&l);
    // The tuner always tries a second lane first, so at least one real lane was opened.
    assert!(opener.opened() >= 1);
    assert_eq!(driver.lanes(), 0, "every lane closed with the move");
}

/// A lane that goes silent: whatever is sent is swallowed and nothing ever comes back.
struct Silent;

impl Channel for Silent {
    async fn send(&mut self, _data: &[u8]) -> Result<(), ChannelError> {
        Ok(())
    }

    async fn recv(&mut self) -> Result<Vec<u8>, ChannelError> {
        std::future::pending().await
    }
}

#[tokio::test]
async fn a_lane_that_goes_silent_is_let_go_and_its_blocks_go_to_the_others() {
    // The new laptop's end of one lane stops answering (hung, not disconnected): the old laptop
    // gives up on it after LANE_SILENCE and sends its blocks on the main connection.
    tokio::time::pause();
    let l = laptops();
    let table = table(&l);
    let mut sender = SenderSession::new(jobs(&l), 4);
    let mut receiver = ReceiverSession::new(&table, plan(&l), common::journal(), "1001");
    let (mut old_main, mut new_main) = pair();
    let (old_tx, mut old_in) = mpsc::unbounded_channel::<Silent>();
    old_tx.send(Silent).unwrap();
    drop(old_tx);
    let (_new_tx, mut new_in) = mpsc::unbounded_channel::<Mem>();
    let (s, r) = tokio::time::timeout(std::time::Duration::from_secs(600), async {
        tokio::join!(sender.run_joining(&mut old_main, &mut old_in), async {
            let r = receiver.run_joining(&mut new_main, &mut new_in).await;
            drop(_new_tx);
            r
        })
    })
    .await
    .expect("the move hung on the silent lane");
    s.unwrap();
    r.unwrap();
    assert_arrived(&l);
}
