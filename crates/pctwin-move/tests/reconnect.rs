//! A move whose connection drops (Task List, the reconnect loop; C09, C10). Both laptops run the
//! app's way of moving (greeted, by the new laptop's rules) and the main connection is cut part
//! way: once the plan is approved the new laptop comes back on a new connection and both carry on
//! where they stopped, every file once; a stop while waiting stops at once; when the laptops can no
//! longer find each other the move ends with every file's honest result kept. Before the approval
//! a drop ends the move as it always did. The last test does it over real paired connections, with
//! the resume keys made from the pairing.
// Tests make files to test on; the no-file-changes fence is for the shipped code.
#![allow(clippy::disallowed_methods)]

use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicBool, AtomicU32, Ordering};
use std::sync::{Arc, Mutex};
use std::time::Duration;

use pctwin_gate::{Approved, Destinations};
use pctwin_journal::Journal;
use pctwin_move::landing::LandingRules;
use pctwin_move::reconnect::LinkState;
use pctwin_move::resume::{Comeback, Redial, Redialed};
use pctwin_move::{Ended, Here, Live, MoveError, Preview, Received, Sent, Steps, Stop, Who};
use pctwin_record::{
    FolderRole, Inclusion, Item, ItemKind, ItemName, ItemPath, LaptopId, ManagedBy, Owner,
    PersonTarget, Place, Portability, Record, Storage,
};
use pctwin_transfer::{
    Cause, Channel, ChannelError, LandingPlace, ReceiveOutcome, SendOutcome, TransferError,
    role_label, shown_path,
};
use tokio::sync::{Notify, mpsc};

/// Well past anything here takes (the waits between connections included): a test that waits
/// for ever still fails.
const HANG_AFTER: Duration = Duration::from_secs(120);

async fn no_hang<T>(work: impl std::future::Future<Output = T>) -> T {
    match tokio::time::timeout(HANG_AFTER, work).await {
        Ok(done) => done,
        Err(_) => panic!("hung: still waiting after {HANG_AFTER:?}"),
    }
}

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

/// The old laptop's end of the first connection, cut when `cut` says so (checked at each send
/// and receive) or when `now` is notified while it waits: the connection is let go at once, so the new laptop sees it
/// drop as a real one would, and it never works again.
struct Breaker<C> {
    inner: Option<C>,
    cut: Box<dyn Fn() -> bool + Send>,
    now: Arc<Notify>,
}

impl<C: Channel> Breaker<C> {
    fn new(inner: C, cut: impl Fn() -> bool + Send + 'static) -> Self {
        Self {
            inner: Some(inner),
            cut: Box::new(cut),
            now: Arc::new(Notify::new()),
        }
    }
}

impl<C: Channel> Channel for Breaker<C> {
    async fn send(&mut self, data: &[u8]) -> Result<(), ChannelError> {
        if (self.cut)() {
            self.inner = None;
        }
        match self.inner.as_mut() {
            Some(c) => c.send(data).await,
            None => Err(ChannelError),
        }
    }
    async fn recv(&mut self) -> Result<Vec<u8>, ChannelError> {
        let Some(c) = self.inner.as_mut() else {
            return Err(ChannelError);
        };
        let now = self.now.clone();
        let got = tokio::select! {
            got = c.recv() => got,
            () = now.notified() => Err(ChannelError),
        };
        // Once cut, whatever arrived is lost with the connection.
        if got.is_err() || (self.cut)() {
            self.inner = None;
            return Err(ChannelError);
        }
        got
    }
}

/// The new laptop's end of the first connection, that goes silent when `cut` says so (checked at
/// each send and receive): it neither reads nor writes again, yet stays open, so to the old laptop
/// the connection still looks alive (a laptop that wandered off the Wi-Fi, say).
struct Mute<C> {
    inner: C,
    cut: Box<dyn Fn() -> bool + Send>,
    dead: bool,
}

impl<C: Channel> Channel for Mute<C> {
    async fn send(&mut self, data: &[u8]) -> Result<(), ChannelError> {
        self.dead = self.dead || (self.cut)();
        if self.dead {
            return Err(ChannelError);
        }
        self.inner.send(data).await
    }
    async fn recv(&mut self) -> Result<Vec<u8>, ChannelError> {
        if self.dead {
            return Err(ChannelError);
        }
        let got = self.inner.recv().await;
        self.dead = self.dead || (self.cut)();
        if self.dead { Err(ChannelError) } else { got }
    }
}

/* ---------- The way back, in memory ---------- */

/// What the new laptop's tries to come back meet.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Way {
    /// The old laptop takes it back.
    Open,
    /// Nobody answers (try again later).
    Away,
    /// The keys ran out: the laptops must pair again.
    Over,
}

/// Shared by both ends: the plan each made its keys for, and how the way back is.
#[derive(Default)]
struct Wire {
    old_plan: Mutex<Option<[u8; 32]>>,
    new_plan: Mutex<Option<[u8; 32]>>,
    opened: AtomicBool,
    tries: AtomicU32,
}

/// The old laptop's way back: connections the new laptop opened, until it says it is over.
struct Back {
    wire: Arc<Wire>,
    arrivals: mpsc::UnboundedReceiver<Mem>,
    /// What `open` answers (`None`: this laptop cannot listen).
    port: Option<u16>,
}

impl Comeback for Back {
    type Link = Mem;
    async fn open(&mut self, plan: &[u8; 32]) -> Option<u16> {
        *self.wire.old_plan.lock().unwrap() = Some(*plan);
        self.wire.opened.store(true, Ordering::SeqCst);
        self.port
    }
    async fn returned(&mut self) -> Option<Mem> {
        self.arrivals.recv().await
    }
}

/// The new laptop's way back.
struct Dial {
    wire: Arc<Wire>,
    to_old: Option<mpsc::UnboundedSender<Mem>>,
    way: Arc<Mutex<Way>>,
    port: Option<u16>,
}

impl Redial for Dial {
    type Link = Mem;
    fn prepare(&mut self, plan: &[u8; 32], port: u16) -> bool {
        *self.wire.new_plan.lock().unwrap() = Some(*plan);
        self.port = Some(port);
        true
    }
    async fn redial(&mut self) -> Result<Mem, Redialed> {
        self.wire.tries.fetch_add(1, Ordering::SeqCst);
        assert_eq!(
            self.port,
            Some(PORT),
            "the new laptop dials the port the old one said"
        );
        let way = *self.way.lock().unwrap();
        match way {
            Way::Open => {
                let (old, new) = mem_pair();
                self.to_old.as_ref().expect("still open").send(old).unwrap();
                Ok(new)
            }
            Way::Away => Err(Redialed::NotThere),
            Way::Over => {
                // Over on both laptops: the old one stops waiting too.
                self.to_old = None;
                Err(Redialed::Over)
            }
        }
    }
}

const PORT: u16 = 4242;

fn way_back(way: Way) -> (Back, Dial, Arc<Wire>, Arc<Mutex<Way>>) {
    let wire = Arc::new(Wire::default());
    let (tx, rx) = mpsc::unbounded_channel();
    let way = Arc::new(Mutex::new(way));
    (
        Back {
            wire: wire.clone(),
            arrivals: rx,
            port: Some(PORT),
        },
        Dial {
            wire: wire.clone(),
            to_old: Some(tx),
            way: way.clone(),
            port: None,
        },
        wire,
        way,
    )
}

/* ---------- The two laptops ---------- */

/// Files big enough to take many blocks each (128 KiB blocks), so a cut lands part way.
const FILES: usize = 4;
const FILE_BYTES: usize = 1_536 * 1024;

fn laptop() -> LaptopId {
    LaptopId::from_hex("00112233445566778899aabbccddeeff").unwrap()
}

fn item(path: &str, size: u64) -> Item {
    let owner = Owner::Person {
        account_id: "1000".into(),
    };
    let place = Place {
        role: FolderRole::Documents,
        storage: Storage::SystemDrive,
    };
    let path = ItemPath::new(vec![ItemName::from_text(path)]).unwrap();
    Item {
        id: pctwin_record::ItemId::derive(&laptop(), &owner, &place, &path),
        kind: ItemKind::File,
        owner,
        place,
        path,
        size_bytes: size,
        modified_ns: None,
        portability: Portability::Portable,
        managed_by: ManagedBy::Personal,
        download_bytes: None,
        inclusion: Inclusion::Included,
    }
}

struct World {
    old: tempfile::TempDir,
    new: tempfile::TempDir,
    _jdir: tempfile::TempDir,
    journal: Journal,
    table: Destinations,
    record: Record,
}

fn bytes_of(n: usize) -> Vec<u8> {
    (0..FILE_BYTES)
        .map(|i| (i.wrapping_mul(31) ^ (n * 7919)) as u8)
        .collect()
}

fn world() -> World {
    let old = tempfile::tempdir().unwrap();
    let new = tempfile::tempdir().unwrap();
    let jdir = tempfile::tempdir().unwrap();
    let journal = Journal::open(&jdir.path().join("journal.redb")).unwrap();
    let mut items = Vec::new();
    std::fs::create_dir_all(old.path().join("Documents")).unwrap();
    for n in 0..FILES {
        let name = format!("part-{n}.bin");
        std::fs::write(old.path().join("Documents").join(&name), bytes_of(n)).unwrap();
        items.push(item(&name, FILE_BYTES as u64));
    }
    let dir = new.path().join("Documents");
    std::fs::create_dir_all(&dir).unwrap();
    let mut table = Destinations::new();
    table
        .approve(role_label(FolderRole::Documents), Approved::MyFolders, &dir)
        .unwrap();
    let mut record = Record::new(laptop());
    record.items = items;
    record.validate().unwrap();
    World {
        old,
        new,
        _jdir: jdir,
        journal,
        table,
        record,
    }
}

fn source_in(root: &Path, item: &Item) -> PathBuf {
    root.join("Documents").join(shown_path(&item.path))
}

fn rules() -> LandingRules {
    LandingRules {
        mover: "1000".into(),
        roles: [(FolderRole::Documents, Storage::SystemDrive)].into(),
        fallback: Default::default(),
        held: Default::default(),
        left_out: Default::default(),
        chosen: Default::default(),
        wholes: Default::default(),
    }
}

/// The new laptop's person: approves, and (when given) runs `at_approval` as they do.
struct Person {
    rules: LandingRules,
    at_approval: Option<Box<dyn FnMut() + Send>>,
}

impl Steps for Person {
    async fn person(&mut self, _: &Record, _: &str) -> Result<PersonTarget, Stop> {
        Ok(PersonTarget::ExistingAccount {
            account_id: "1001".into(),
        })
    }
    async fn approve(&mut self, _: &Record, _: &Preview) -> Result<(), Stop> {
        if let Some(f) = self.at_approval.as_mut() {
            f();
        }
        Ok(())
    }
    fn landing(&self, item: &Item) -> Option<LandingPlace> {
        self.rules.landing(item)
    }
}

async fn greeted<C: Channel>(
    link: &mut C,
    side: pctwin_move::greeting::Side,
) -> Result<pctwin_move::greeting::Greeted<'_, C>, MoveError> {
    use pctwin_move::greeting::{About, Power, greet};
    Ok(greet(link, &About::here(Power::Unknown, None), side)
        .await
        .map_err(TransferError::from)?)
}

/// The old laptop's whole side, as the app runs it.
async fn old_side<C: Channel, B: Comeback>(
    link: &mut C,
    w: &World,
    live: &Live,
    back: &mut B,
) -> Result<Sent, MoveError> {
    let mut link = greeted(link, pctwin_move::greeting::Side::Old).await?;
    let who = Who {
        mover: "1000",
        people: &[],
    };
    let (sent, _) = pctwin_move::send_by_rules(
        &mut link,
        &w.record,
        &who,
        |i| Some(source_in(w.old.path(), i)),
        live,
        Arc::new(pctwin_transfer::FsOpener),
        back,
    )
    .await?;
    Ok(sent)
}

/// The new laptop's whole side, as the app runs it.
async fn new_side<C: Channel, R: Redial>(
    link: &mut C,
    w: &World,
    table: &mut Destinations,
    steps: &mut Person,
    live: &Live,
    redial: &mut R,
) -> Result<Received, MoveError> {
    let mut link = greeted(link, pctwin_move::greeting::Side::New).await?;
    pctwin_move::receive_by_rules(
        &mut link,
        steps,
        |s, _| s.rules.clone(),
        table,
        Here {
            journal: &w.journal,
            account: "1001",
        },
        live,
        redial,
    )
    .await
}

/// Every file the new laptop says it copied is on its drive, byte for byte.
fn landed_true(w: &World, received: &Received) -> usize {
    let mut copied = 0;
    for line in &received.files {
        if let Some(ReceiveOutcome::Finished(done)) = &line.outcome {
            let n: usize = done
                .final_path
                .trim_start_matches("part-")
                .trim_end_matches(".bin")
                .parse()
                .unwrap();
            let landed = std::fs::read(w.new.path().join("Documents").join(&done.final_path));
            assert_eq!(landed.unwrap(), bytes_of(n), "{}", done.final_path);
            copied += 1;
        }
    }
    copied
}

/// Waits (polling) until `live`'s connection is waiting for the other laptop.
async fn until_waiting(live: &Live) -> (bool, std::time::Instant) {
    loop {
        if let LinkState::Waiting { nothing_new, since } = live.connection.state() {
            return (nothing_new, since);
        }
        tokio::time::sleep(Duration::from_millis(5)).await;
    }
}

/// Once the plan is approved, a drop part way through is picked up again on a new connection
/// without pairing again: both laptops made their keys for the very same plan, the move carries
/// on and finishes, every file lands once and is counted once on both laptops, and both screens
/// are back to connected.
#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn a_drop_after_the_approval_is_picked_up_again_and_every_file_lands_once() {
    let mut w = world();
    let mut table = std::mem::take(&mut w.table);
    let (mut back, mut dial, wire, _) = way_back(Way::Open);
    let (old_live, new_live) = (Live::default(), Live::default());
    // Cut once the first file is through on the old laptop, so some are done and some are not.
    let meter = old_live.meter.clone();
    let cut_once = Arc::new(AtomicBool::new(false));
    let cut = {
        let cut_once = cut_once.clone();
        move || meter.reading().files.copied >= 1 && !cut_once.swap(true, Ordering::SeqCst)
    };
    let (old, mut new) = mem_pair();
    let mut old = Breaker::new(old, cut);
    let mut steps = Person {
        rules: rules(),
        at_approval: None,
    };
    let (sent, received) = no_hang(async {
        tokio::join!(
            old_side(&mut old, &w, &old_live, &mut back),
            new_side(&mut new, &w, &mut table, &mut steps, &new_live, &mut dial)
        )
    })
    .await;
    let (sent, received) = (sent.unwrap(), received.unwrap());
    assert!(cut_once.load(Ordering::SeqCst), "the connection was cut");
    assert!(
        wire.tries.load(Ordering::SeqCst) >= 1,
        "the new laptop came back"
    );
    let (old_plan, new_plan) = (
        *wire.old_plan.lock().unwrap(),
        *wire.new_plan.lock().unwrap(),
    );
    assert!(old_plan.is_some(), "the old laptop made keys for the plan");
    assert_eq!(
        old_plan, new_plan,
        "both laptops made keys for the same plan"
    );

    assert_eq!(sent.ended, Ended::Finished);
    assert_eq!(received.ended, Ended::Finished);
    for line in &sent.files {
        assert_eq!(line.outcome, Some(SendOutcome::Arrived), "{:?}", line.item);
    }
    assert_eq!(landed_true(&w, &received), FILES);
    // Counted once on both laptops: nothing twice, however the cut fell.
    let total = (FILES * FILE_BYTES) as u64;
    let new_reading = new_live.meter.reading();
    assert_eq!(new_reading.files.total, FILES as u64);
    assert_eq!(new_reading.files.copied, FILES as u64);
    assert_eq!(new_reading.files.failed, 0);
    assert_eq!(new_reading.bytes_checked, total);
    let old_reading = old_live.meter.reading();
    assert_eq!(old_reading.files.copied, FILES as u64);
    assert_eq!(new_live.connection.state(), LinkState::Connected);
    assert_eq!(old_live.connection.state(), LinkState::Connected);
}

/// A stop while waiting for the other laptop stops the move at once on that laptop, with what
/// was already copied kept and every other file failed as stopped (never copied).
#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn a_stop_while_waiting_stops_at_once_and_keeps_what_moved() {
    let mut w = world();
    let mut table = std::mem::take(&mut w.table);
    let (mut back, mut dial, _wire, _) = way_back(Way::Away);
    let (old_live, new_live) = (Live::default(), Live::default());
    let meter = old_live.meter.clone();
    let (old, mut new) = mem_pair();
    let mut old = Breaker::new(old, move || meter.reading().files.copied >= 1);
    let mut steps = Person {
        rules: rules(),
        at_approval: None,
    };
    let stopper = async {
        // Each laptop's person stops once their screen says it is waiting.
        until_waiting(&new_live).await;
        new_live.control.stop();
        until_waiting(&old_live).await;
        old_live.control.stop();
    };
    let (sent, received, ()) = no_hang(async {
        tokio::join!(
            old_side(&mut old, &w, &old_live, &mut back),
            new_side(&mut new, &w, &mut table, &mut steps, &new_live, &mut dial),
            stopper
        )
    })
    .await;
    let (sent, received) = (sent.unwrap(), received.unwrap());
    assert_eq!(received.ended, Ended::StoppedHere);
    assert_eq!(sent.ended, Ended::StoppedHere);
    let copied = landed_true(&w, &received);
    assert!(copied >= 1, "what moved before the drop is kept");
    for line in &received.files {
        match &line.outcome {
            Some(ReceiveOutcome::Finished(_)) => {}
            Some(ReceiveOutcome::Failed(f)) => assert_eq!(f.cause(), Cause::Stopped),
            other => panic!("{other:?}"),
        }
    }
    for line in &sent.files {
        match &line.outcome {
            Some(SendOutcome::Arrived) => {}
            Some(SendOutcome::Failed(f)) => assert_eq!(f.cause(), Cause::Stopped),
            other => panic!("{other:?}"),
        }
    }
}

/// When the laptops can no longer come back to each other without pairing again (the keys ran
/// out of time or tries), the move ends on both: what moved is kept, every other file is failed
/// because the move ended (never copied), and the new laptop's screen says it is over.
#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn when_the_way_back_is_over_the_move_ends_with_every_result_kept() {
    let mut w = world();
    let mut table = std::mem::take(&mut w.table);
    let (mut back, mut dial, wire, _) = way_back(Way::Over);
    let (old_live, new_live) = (Live::default(), Live::default());
    let meter = old_live.meter.clone();
    let (old, mut new) = mem_pair();
    let mut old = Breaker::new(old, move || meter.reading().files.copied >= 1);
    let mut steps = Person {
        rules: rules(),
        at_approval: None,
    };
    let (sent, received) = no_hang(async {
        tokio::join!(
            old_side(&mut old, &w, &old_live, &mut back),
            new_side(&mut new, &w, &mut table, &mut steps, &new_live, &mut dial)
        )
    })
    .await;
    let (sent, received) = (sent.unwrap(), received.unwrap());
    assert_eq!(
        wire.tries.load(Ordering::SeqCst),
        1,
        "no more tries once over"
    );
    assert_eq!(received.ended, Ended::LinkLost);
    assert_eq!(sent.ended, Ended::LinkLost);
    assert_eq!(new_live.connection.state(), LinkState::Over);
    assert_eq!(old_live.connection.state(), LinkState::Over);
    let copied = landed_true(&w, &received);
    // The cut (the same as in the other tests) falls part way: some files done, some not.
    assert!((1..FILES).contains(&copied), "{copied} copied");
    for line in &received.files {
        match &line.outcome {
            Some(ReceiveOutcome::Finished(_)) => {}
            Some(ReceiveOutcome::Failed(f)) => {
                assert_eq!(f.cause(), Cause::MoveEnded);
                assert_eq!(f.why(), pctwin_move::LINK_LOST);
            }
            other => panic!("{other:?}"),
        }
    }
    let mut arrived = 0;
    for line in &sent.files {
        match &line.outcome {
            Some(SendOutcome::Arrived) => arrived += 1,
            Some(SendOutcome::Failed(f)) => assert_eq!(f.cause(), Cause::MoveEnded),
            other => panic!("{other:?}"),
        }
    }
    assert!(arrived >= 1);
}

/// The old laptop that could not listen for a way back says so: a drop then ends the move at
/// once on both laptops, with every result kept, and the new laptop never tries to come back.
#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn with_no_way_back_a_drop_ends_the_move_with_every_result_kept() {
    let mut w = world();
    let mut table = std::mem::take(&mut w.table);
    let (mut back, mut dial, wire, _) = way_back(Way::Open);
    back.port = None;
    let (old_live, new_live) = (Live::default(), Live::default());
    let meter = old_live.meter.clone();
    let (old, mut new) = mem_pair();
    let mut old = Breaker::new(old, move || meter.reading().files.copied >= 1);
    let mut steps = Person {
        rules: rules(),
        at_approval: None,
    };
    let (sent, received) = no_hang(async {
        tokio::join!(
            old_side(&mut old, &w, &old_live, &mut back),
            new_side(&mut new, &w, &mut table, &mut steps, &new_live, &mut dial)
        )
    })
    .await;
    let (sent, received) = (sent.unwrap(), received.unwrap());
    assert_eq!(wire.tries.load(Ordering::SeqCst), 0);
    assert_eq!(received.ended, Ended::LinkLost);
    assert_eq!(sent.ended, Ended::LinkLost);
    assert!(landed_true(&w, &received) >= 1);
}

/// Before the plan is approved there are no keys to come back with: a drop ends the move as it
/// always did, as a lost connection, and nobody tries to come back.
#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn a_drop_before_the_approval_ends_the_move_as_before() {
    let mut w = world();
    let mut table = std::mem::take(&mut w.table);
    let (mut back, mut dial, wire, _) = way_back(Way::Open);
    let (old_live, new_live) = (Live::default(), Live::default());
    let (old, mut new) = mem_pair();
    let approving = Arc::new(AtomicBool::new(false));
    let cut = approving.clone();
    let mut old = Breaker::new(old, move || cut.load(Ordering::SeqCst));
    let now = old.now.clone();
    let mut steps = Person {
        rules: rules(),
        // The connection drops while the person is approving.
        at_approval: Some(Box::new(move || {
            approving.store(true, Ordering::SeqCst);
            now.notify_one();
        })),
    };
    let (sent, received) = no_hang(async {
        tokio::join!(
            old_side(&mut old, &w, &old_live, &mut back),
            new_side(&mut new, &w, &mut table, &mut steps, &new_live, &mut dial)
        )
    })
    .await;
    assert!(
        matches!(
            sent,
            Err(MoveError::Transfer(TransferError::ConnectionDropped))
        ),
        "{:?}",
        sent.map(|s| s.ended)
    );
    assert!(
        matches!(
            received,
            Err(MoveError::Transfer(TransferError::ConnectionDropped))
        ),
        "{:?}",
        received.map(|r| r.ended)
    );
    assert!(
        !wire.opened.load(Ordering::SeqCst),
        "no keys before the approval"
    );
    assert_eq!(wire.tries.load(Ordering::SeqCst), 0);
}

/// While the other laptop cannot be reached, the new laptop tries again by itself; "Try now"
/// tries at once rather than after the wait; tries that bring nothing back soon say "nothing new
/// has arrived"; and once the other laptop answers, the move carries on and finishes.
#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn try_now_tries_at_once_and_idle_tries_say_nothing_new_has_arrived() {
    let mut w = world();
    let mut table = std::mem::take(&mut w.table);
    let (mut back, mut dial, wire, way) = way_back(Way::Away);
    let (old_live, new_live) = (Live::default(), Live::default());
    let meter = old_live.meter.clone();
    let cut_once = Arc::new(AtomicBool::new(false));
    let cut = {
        let cut_once = cut_once.clone();
        move || meter.reading().files.copied >= 1 && !cut_once.swap(true, Ordering::SeqCst)
    };
    let (old, mut new) = mem_pair();
    let mut old = Breaker::new(old, cut);
    let mut steps = Person {
        rules: rules(),
        at_approval: None,
    };
    let person = async {
        // The drop came after progress: not "nothing new" yet.
        let (nothing_new, since) = until_waiting(&new_live).await;
        assert!(!nothing_new);
        // Each "Try now" is a try at once (the waits grow to 2 and 4 seconds by themselves).
        for n in 1..=3u32 {
            let started = std::time::Instant::now();
            while wire.tries.load(Ordering::SeqCst) < n {
                new_live.connection.try_now();
                tokio::time::sleep(Duration::from_millis(5)).await;
            }
            assert!(started.elapsed() < Duration::from_millis(1_900), "try {n}");
        }
        // Two tries in a row that brought nothing back: the screen says so (and Stop is there).
        loop {
            if let LinkState::Waiting {
                nothing_new,
                since: s,
            } = new_live.connection.state()
            {
                assert_eq!(s, since, "still the same drop");
                if nothing_new {
                    break;
                }
            }
            tokio::time::sleep(Duration::from_millis(5)).await;
        }
        // The other laptop answers again: the move carries on.
        *way.lock().unwrap() = Way::Open;
        new_live.connection.try_now();
    };
    let (sent, received, ()) = no_hang(async {
        tokio::join!(
            old_side(&mut old, &w, &old_live, &mut back),
            new_side(&mut new, &w, &mut table, &mut steps, &new_live, &mut dial),
            person
        )
    })
    .await;
    let (sent, received) = (sent.unwrap(), received.unwrap());
    assert_eq!(sent.ended, Ended::Finished);
    assert_eq!(received.ended, Ended::Finished);
    assert_eq!(landed_true(&w, &received), FILES);
}

/* ---------- Over real paired connections ---------- */

/// Pairs two laptops over loopback, as the app does: the old laptop's link and the new one's.
async fn paired(config: pctwin_link::LinkConfig) -> (pctwin_link::Link, pctwin_link::Link) {
    use std::time::Instant;
    let host = pctwin_link::Host::bind("127.0.0.1:0".parse().unwrap(), config)
        .await
        .unwrap();
    let addr = host.local_addr().unwrap();
    let rotating = Arc::new(std::sync::Mutex::new(
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
    let (old, new) = tokio::join!(
        pending_host.choose(number, Instant::now()),
        pending_guest.approval()
    );
    // Pairing's own listener closes once paired, as in the app.
    drop(host);
    (old.unwrap(), new.unwrap())
}

/// Over real connections: after the approval the old laptop listens for the way back on its own,
/// both laptops make their resume keys from the pairing for the approved plan, and after a drop the
/// new laptop reconnects with them (no code) and the move finishes, every file once.
#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn over_real_connections_a_dropped_move_resumes_with_its_keys_and_finishes() {
    let config = pctwin_link::LinkConfig {
        step_timeout: Duration::from_secs(5),
        handshake_timeout: Duration::from_secs(5),
        silence_penalty: Duration::ZERO,
        connect_timeout: Duration::from_secs(5),
    };
    let (mut old_link, mut new_link) = paired(config).await;
    let mut back = pctwin_move::resume::NetComeback::new(&mut old_link, config);
    let mut dial = pctwin_move::resume::NetRedial::new(&mut new_link, config);
    let mut w = world();
    let mut table = std::mem::take(&mut w.table);
    let (old_live, new_live) = (Live::default(), Live::default());
    let meter = old_live.meter.clone();
    let cut_once = Arc::new(AtomicBool::new(false));
    let cut = {
        let cut_once = cut_once.clone();
        move || meter.reading().files.copied >= 1 && !cut_once.swap(true, Ordering::SeqCst)
    };
    let mut old = Breaker::new(old_link, cut);
    let mut steps = Person {
        rules: rules(),
        at_approval: None,
    };
    let (sent, received) = no_hang(async {
        tokio::join!(
            old_side(&mut old, &w, &old_live, &mut back),
            new_side(
                &mut new_link,
                &w,
                &mut table,
                &mut steps,
                &new_live,
                &mut dial
            )
        )
    })
    .await;
    let (sent, received) = (sent.unwrap(), received.unwrap());
    assert!(cut_once.load(Ordering::SeqCst), "the connection was cut");
    assert_eq!(sent.ended, Ended::Finished);
    assert_eq!(received.ended, Ended::Finished);
    assert_eq!(landed_true(&w, &received), FILES);
    let reading = new_live.meter.reading();
    assert_eq!(reading.files.copied, FILES as u64);
    assert_eq!(reading.bytes_checked, (FILES * FILE_BYTES) as u64);
}

/* ---------- A stop while waiting on the other laptop, before any file moves ---------- */

/// The other laptop's end, kept open and silent (or just reading), for as long as the test runs.
async fn silent(mut link: Mem) {
    while link.recv().await.is_ok() {}
}

/// Runs `work` with the other laptop's `peer` beside it, which never ends by itself.
async fn beside<T>(
    peer: impl std::future::Future<Output = ()>,
    work: impl std::future::Future<Output = T>,
) -> T {
    tokio::select! {
        got = work => got,
        () = peer => panic!("the other laptop's end closed"),
    }
}

/// Stops `live`'s move a moment from now.
async fn stop_soon(live: &Live) {
    tokio::time::sleep(Duration::from_millis(200)).await;
    live.control.stop();
}

/// A stop during the greeting ends it at once, as a stop, however the other laptop drags it out.
#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn a_stop_during_the_greeting_ends_it_at_once() {
    let (mut new, old) = mem_pair();
    let live = Live::default();
    let greeting = pctwin_move::or_stop(&live.control, async {
        greeted(&mut new, pctwin_move::greeting::Side::New)
            .await
            .map(|_| ())
    });
    let got = no_hang(beside(silent(old), async {
        tokio::join!(greeting, stop_soon(&live)).0
    }))
    .await;
    assert!(
        matches!(got, Err(MoveError::Stopped(Stop::Cancelled))),
        "{got:?}"
    );
}

/// The new laptop waits for the old laptop's list as long as the old laptop's person takes; a
/// stop here ends that wait at once, and nothing is asked of the person after it.
#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn a_stop_while_waiting_for_the_list_ends_it_at_once() {
    let mut w = world();
    let mut table = std::mem::take(&mut w.table);
    let (old, mut new) = mem_pair();
    let live = Live::default();
    let asked = Arc::new(AtomicBool::new(false));
    let asked_too = asked.clone();
    let mut steps = Person {
        rules: rules(),
        at_approval: Some(Box::new(move || asked_too.store(true, Ordering::SeqCst))),
    };
    let old_side = async {
        // Greets, then finds nothing for ever.
        let mut old = old;
        let _greeted = greeted(&mut old, pctwin_move::greeting::Side::Old).await;
        silent(old).await;
    };
    let mut no_way_back = pctwin_move::resume::NoRedial;
    let got = no_hang(beside(old_side, async {
        tokio::join!(
            new_side(
                &mut new,
                &w,
                &mut table,
                &mut steps,
                &live,
                &mut no_way_back
            ),
            stop_soon(&live)
        )
        .0
    }))
    .await;
    assert!(
        matches!(got, Err(MoveError::Stopped(Stop::Cancelled))),
        "{:?}",
        got.map(|r| r.ended)
    );
    assert!(!asked.load(Ordering::SeqCst));
}

/// The old laptop waits for the new laptop's rules as long as its person takes to approve; a stop
/// here ends that wait at once.
#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn a_stop_while_waiting_for_the_approval_ends_it_at_once() {
    let w = world();
    let (mut old, new) = mem_pair();
    let live = Live::default();
    let new_side = async {
        // Greets, then reads whatever comes and never approves.
        let mut new = new;
        let _greeted = greeted(&mut new, pctwin_move::greeting::Side::New).await;
        silent(new).await;
    };
    let mut no_way_back = pctwin_move::resume::NoComeback;
    let got = no_hang(beside(new_side, async {
        tokio::join!(
            old_side(&mut old, &w, &live, &mut no_way_back),
            stop_soon(&live)
        )
        .0
    }))
    .await;
    assert!(
        matches!(got, Err(MoveError::Stopped(Stop::Cancelled))),
        "{:?}",
        got.map(|s| s.ended)
    );
}

/// The new laptop gives up on a connection that still looks alive to the old laptop (it went
/// silent rather than closing) and comes back on a new one: the old laptop drops the old one at
/// once and carries on over the new one. At every point of the move the cut falls, both laptops
/// finish, every file lands once, and both meters count exactly the plan.
#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn a_return_while_the_old_connection_still_looks_alive_carries_on_wherever_it_falls() {
    let total = (FILES * FILE_BYTES) as u64;
    for at in [
        128 * 1024,
        2 * 1024 * 1024,
        4 * 1024 * 1024,
        total - 128 * 1024,
    ] {
        let mut w = world();
        let mut table = std::mem::take(&mut w.table);
        let (mut back, mut dial, wire, _) = way_back(Way::Open);
        let (old_live, new_live) = (Live::default(), Live::default());
        let meter = new_live.meter.clone();
        let fell = Arc::new(AtomicBool::new(false));
        let cut = {
            let fell = fell.clone();
            move || meter.reading().bytes_written >= at && !fell.swap(true, Ordering::SeqCst)
        };
        let (mut old, new) = mem_pair();
        let mut new = Mute {
            inner: new,
            cut: Box::new(cut),
            dead: false,
        };
        let mut steps = Person {
            rules: rules(),
            at_approval: None,
        };
        let (sent, received) = no_hang(async {
            tokio::join!(
                old_side(&mut old, &w, &old_live, &mut back),
                new_side(&mut new, &w, &mut table, &mut steps, &new_live, &mut dial)
            )
        })
        .await;
        let (sent, received) = (sent.unwrap(), received.unwrap());
        assert!(fell.load(Ordering::SeqCst), "cut at {at}");
        assert_eq!(wire.tries.load(Ordering::SeqCst), 1, "cut at {at}");
        assert_eq!(sent.ended, Ended::Finished, "cut at {at}");
        assert_eq!(received.ended, Ended::Finished, "cut at {at}");
        for line in &sent.files {
            assert_eq!(line.outcome, Some(SendOutcome::Arrived), "cut at {at}");
        }
        assert_eq!(landed_true(&w, &received), FILES, "cut at {at}");
        let new_reading = new_live.meter.reading();
        assert_eq!(new_reading.files.copied, FILES as u64, "cut at {at}");
        assert_eq!(new_reading.files.failed, 0, "cut at {at}");
        assert_eq!(new_reading.bytes_checked, total, "cut at {at}");
        let old_reading = old_live.meter.reading();
        assert_eq!(old_reading.files.copied, FILES as u64, "cut at {at}");
        assert_eq!(old_reading.bytes_checked, total, "cut at {at}");
    }
}

/// Strangers on the network holding connections to the old laptop's way back open, from more
/// addresses than it serves at once, cannot end a dropped move: the new laptop comes back and the
/// move finishes.
// A Mac answers only on 127.0.0.1, and the strangers need addresses of their own.
#[cfg(not(target_os = "macos"))]
#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn strangers_jamming_the_way_back_cannot_end_the_move() {
    /// The real way back, telling the test where it listens.
    struct Told {
        inner: pctwin_move::resume::NetComeback,
        port: Arc<Mutex<Option<u16>>>,
    }
    impl Comeback for Told {
        type Link = pctwin_link::Link;
        async fn open(&mut self, plan: &[u8; 32]) -> Option<u16> {
            let port = self.inner.open(plan).await;
            *self.port.lock().unwrap() = port;
            port
        }
        async fn returned(&mut self) -> Option<pctwin_link::Link> {
            self.inner.returned().await
        }
    }
    async fn hold_open(from: std::net::IpAddr, port: Arc<Mutex<Option<u16>>>) {
        use tokio::io::AsyncReadExt;
        loop {
            let at = *port.lock().unwrap();
            if let Some(at) = at {
                let socket = tokio::net::TcpSocket::new_v4().unwrap();
                socket.bind(std::net::SocketAddr::new(from, 0)).unwrap();
                let to = std::net::SocketAddr::from(([127, 0, 0, 1], at));
                if let Ok(mut held) = socket.connect(to).await {
                    let mut sink = [0u8; 64];
                    let _ =
                        tokio::time::timeout(Duration::from_secs(5), held.read(&mut sink)).await;
                }
            }
            tokio::time::sleep(Duration::from_millis(20)).await;
        }
    }
    let config = pctwin_link::LinkConfig {
        step_timeout: Duration::from_secs(1),
        handshake_timeout: Duration::from_secs(5),
        silence_penalty: Duration::from_millis(200),
        connect_timeout: Duration::from_secs(1),
    };
    let (mut old_link, mut new_link) = paired(config).await;
    let port = Arc::new(Mutex::new(None));
    let mut back = Told {
        inner: pctwin_move::resume::NetComeback::new(&mut old_link, config),
        port: port.clone(),
    };
    let mut dial = pctwin_move::resume::NetRedial::new(&mut new_link, config);
    let strangers: Vec<_> = (2..=9u8)
        .map(|n| {
            tokio::spawn(hold_open(
                std::net::IpAddr::from([127, 0, 0, n]),
                port.clone(),
            ))
        })
        .collect();
    let mut w = world();
    let mut table = std::mem::take(&mut w.table);
    let (old_live, new_live) = (Live::default(), Live::default());
    let meter = old_live.meter.clone();
    let cut_once = Arc::new(AtomicBool::new(false));
    let cut = {
        let cut_once = cut_once.clone();
        move || meter.reading().files.copied >= 1 && !cut_once.swap(true, Ordering::SeqCst)
    };
    let mut old = Breaker::new(old_link, cut);
    let mut steps = Person {
        rules: rules(),
        at_approval: None,
    };
    let (sent, received) = no_hang(async {
        tokio::join!(
            old_side(&mut old, &w, &old_live, &mut back),
            new_side(
                &mut new_link,
                &w,
                &mut table,
                &mut steps,
                &new_live,
                &mut dial
            )
        )
    })
    .await;
    for s in strangers {
        s.abort();
    }
    let (sent, received) = (sent.unwrap(), received.unwrap());
    assert!(cut_once.load(Ordering::SeqCst), "the connection was cut");
    assert_eq!(sent.ended, Ended::Finished);
    assert_eq!(received.ended, Ended::Finished);
    assert_eq!(landed_true(&w, &received), FILES);
}
