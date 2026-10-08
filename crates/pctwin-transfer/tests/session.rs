//! A whole move over a connection (Task List 1.5): files travel in priority order, taking turns;
//! every block is checked and receipted; no more than the in-flight limit is ever unconfirmed;
//! after a drop both sides keep what they have and the next connection continues each file from
//! exactly where it stopped (or starts a changed file again); a file that changes while it is
//! being sent arrives whole and current; a file for a place that was not approved fails on its
//! own. Run over an in-memory connection that can be cut, and once over a real paired link.

use std::path::{Path, PathBuf};
use std::sync::Arc;
use std::sync::atomic::{AtomicBool, AtomicI64, Ordering};

use pctwin_gate::{Approved, Destinations};
use pctwin_record::ItemId;
use pctwin_transfer::{
    Block, Channel, ChannelError, FileSender, Header, Message, ReceiveOutcome, ReceiverSession,
    SendJob, SendOutcome, SenderSession, Tier, block_size_for, split_into_pieces,
};
use tokio::sync::{Mutex, mpsc};

mod common;

type Hook = Box<dyn FnMut() + Send>;

/// One end of an in-memory connection. Cutting it breaks both ends like a dropped Wi-Fi link:
/// messages already delivered can still be read, then every call fails.
struct Mem {
    tx: mpsc::UnboundedSender<Vec<u8>>,
    rx: mpsc::UnboundedReceiver<Vec<u8>>,
    cut: Arc<AtomicBool>,
    sends: usize,
    /// Cut the connection at this send from this end.
    cut_at: Option<usize>,
    /// Run this at this send from this end (for example, to edit a file mid-move).
    hook: Option<(usize, Hook)>,
    /// Bytes of blocks sent and not yet confirmed, and the most ever seen.
    unconfirmed: Arc<AtomicI64>,
    most_unconfirmed: Arc<AtomicI64>,
    block_bytes: i64,
    /// Sizes of blocks sent and not yet confirmed, oldest first (receipts come back in order).
    fifo: std::collections::VecDeque<i64>,
}

fn mem_pair() -> (Mem, Mem) {
    let (a_tx, b_rx) = mpsc::unbounded_channel();
    let (b_tx, a_rx) = mpsc::unbounded_channel();
    let cut = Arc::new(AtomicBool::new(false));
    let unconfirmed = Arc::new(AtomicI64::new(0));
    let most = Arc::new(AtomicI64::new(0));
    let end = |tx, rx| Mem {
        tx,
        rx,
        cut: cut.clone(),
        sends: 0,
        cut_at: None,
        hook: None,
        unconfirmed: unconfirmed.clone(),
        most_unconfirmed: most.clone(),
        block_bytes: 0,
        fifo: std::collections::VecDeque::new(),
    };
    (end(a_tx, a_rx), end(b_tx, b_rx))
}

impl Channel for Mem {
    async fn send(&mut self, data: &[u8]) -> Result<(), ChannelError> {
        // Like a real network, the other side gets to run between messages.
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
        if let Ok(Message::Piece { last, bytes, .. }) = Message::decode(data) {
            self.block_bytes += bytes.len() as i64;
            if last {
                let now = self
                    .unconfirmed
                    .fetch_add(self.block_bytes, Ordering::SeqCst)
                    + self.block_bytes;
                self.most_unconfirmed.fetch_max(now, Ordering::SeqCst);
                self.fifo.push_back(self.block_bytes);
                self.block_bytes = 0;
            }
        }
        self.tx.send(data.to_vec()).map_err(|_| ChannelError)
    }

    async fn recv(&mut self) -> Result<Vec<u8>, ChannelError> {
        loop {
            if let Ok(m) = self.rx.try_recv() {
                // As the sender sees it: a receipt confirms the oldest unconfirmed block.
                if let Ok(Message::Receipt { .. }) = Message::decode(&m)
                    && let Some(len) = self.fifo.pop_front()
                {
                    self.unconfirmed.fetch_sub(len, Ordering::SeqCst);
                }
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
    new_mine: tempfile::TempDir,
    new_shared: tempfile::TempDir,
    files: Vec<(PathBuf, Vec<u8>)>,
}

fn laptops() -> Laptops {
    let old = tempfile::tempdir().unwrap();
    let files: Vec<(PathBuf, Vec<u8>)> = [(4_000, 1), (300_000, 2), (2_500_000, 3)]
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
        new_mine: tempfile::tempdir().unwrap(),
        new_shared: tempfile::tempdir().unwrap(),
        files,
    }
}

fn table(l: &Laptops) -> Destinations {
    let mut t = Destinations::new();
    t.approve("me", Approved::MyFolders, l.new_mine.path())
        .unwrap();
    t.approve("shared", Approved::SharedFolder, l.new_shared.path())
        .unwrap();
    t
}

fn jobs(l: &Laptops) -> Vec<SendJob> {
    let job = |n: u8, destination: &str, path: &str| SendJob {
        item: id(n),
        source: l.files[n as usize].0.clone(),
        destination: destination.into(),
        path: path.into(),
        compressible: true,
        tier: Tier::Rest,
    };
    vec![
        job(0, "me", "Documents/small.bin"),
        job(1, "shared", "Public/medium.bin"),
        job(2, "me", "Videos/big.bin"),
    ]
}

/// The plan the person approved: exactly the three files, at their sizes.
fn plan(l: &Laptops) -> pctwin_transfer::Allowance {
    let files: Vec<(ItemId, u64)> = (0..3u8)
        .map(|n| (id(n), l.files[n as usize].1.len() as u64))
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
    assert_eq!(
        std::fs::read(l.new_mine.path().join("Documents/small.bin")).unwrap(),
        l.files[0].1
    );
    assert_eq!(
        std::fs::read(l.new_shared.path().join("Public/medium.bin")).unwrap(),
        l.files[1].1
    );
    assert_eq!(
        std::fs::read(l.new_mine.path().join("Videos/big.bin")).unwrap(),
        l.files[2].1
    );
}

async fn run_both(
    sender: &Mutex<SenderSession>,
    receiver: &Mutex<ReceiverSession<'_>>,
    (mut a, mut b): (Mem, Mem),
) -> (bool, bool) {
    let mut s = sender.lock().await;
    let mut r = receiver.lock().await;
    // A move that hangs fails here instead of waiting forever.
    let (sent, received) = tokio::time::timeout(std::time::Duration::from_secs(30), async {
        tokio::join!(s.run(&mut a), r.run(&mut b))
    })
    .await
    .expect("the move hung");
    (sent.is_ok(), received.is_ok())
}

#[tokio::test]
async fn every_file_arrives_in_its_approved_place() {
    let l = laptops();
    let table = table(&l);
    let sender = Mutex::new(SenderSession::new(jobs(&l), 2));
    let receiver = Mutex::new(ReceiverSession::new(&table, plan(&l)));
    assert_eq!(run_both(&sender, &receiver, mem_pair()).await, (true, true));
    assert_arrived(&l);
    let s = sender.lock().await;
    for n in 0..3 {
        assert_eq!(s.outcome(id(n)), Some(&SendOutcome::Arrived), "{n}");
    }
    assert_eq!(s.blocks_sent(), total_blocks(&l));
    // Copies keep the original's modified time.
    for (src, dst) in [
        (&l.files[0].0, l.new_mine.path().join("Documents/small.bin")),
        (&l.files[1].0, l.new_shared.path().join("Public/medium.bin")),
        (&l.files[2].0, l.new_mine.path().join("Videos/big.bin")),
    ] {
        assert_eq!(
            std::fs::metadata(src).unwrap().modified().unwrap(),
            std::fs::metadata(dst).unwrap().modified().unwrap()
        );
    }
    let r = receiver.lock().await;
    assert!(
        matches!(r.outcome(id(2)), Some(ReceiveOutcome::Finished(f)) if f.final_path == "Videos/big.bin")
    );
}

#[tokio::test]
async fn never_more_than_the_in_flight_limit_is_unconfirmed() {
    let l = laptops();
    let table = table(&l);
    let limit: u64 = 256 * 1024;
    let mut sender = SenderSession::new(jobs(&l), 2).with_in_flight_limit(limit);
    let mut receiver = ReceiverSession::new(&table, plan(&l));
    let (mut a, mut b) = mem_pair();
    let most = a.most_unconfirmed.clone();
    let (sent, received) = tokio::time::timeout(std::time::Duration::from_secs(30), async {
        tokio::join!(sender.run(&mut a), receiver.run(&mut b))
    })
    .await
    .unwrap();
    sent.unwrap();
    received.unwrap();
    assert_arrived(&l);
    // A new block goes only while less than the limit is unconfirmed, so the most ever
    // unconfirmed is under the limit plus one block.
    let one_block = 131_072 + 46;
    let peak = most.load(Ordering::SeqCst);
    assert!(peak < (limit + one_block) as i64, "{peak}");
    // And the limit really was reached, so it was really tested.
    assert!(peak >= limit as i64, "{peak}");
}

#[tokio::test]
async fn a_dropped_connection_continues_each_file_from_where_it_stopped() {
    // Cut at many points, including part way through a block.
    let mut some_partway = false;
    for cut in [8usize, 15, 22, 23, 24, 31, 40, 41, 55] {
        let l = laptops();
        let table = table(&l);
        let sender = Mutex::new(SenderSession::new(jobs(&l), 2));
        let receiver = Mutex::new(ReceiverSession::new(&table, plan(&l)));
        let (mut a, b) = mem_pair();
        a.cut_at = Some(cut);
        assert_eq!(
            run_both(&sender, &receiver, (a, b)).await,
            (false, false),
            "cut {cut}"
        );
        let before = sender.lock().await.blocks_sent();
        let partway = receiver.lock().await.partway();
        some_partway |= partway > 0;

        assert_eq!(
            run_both(&sender, &receiver, mem_pair()).await,
            (true, true),
            "cut {cut}"
        );
        assert_arrived(&l);
        // Every file held partway was continued, not started over: only the blocks lost in the cut
        // were sent again.
        assert_eq!(
            receiver.lock().await.continued(),
            partway,
            "cut {cut}: a partway file was started over"
        );
        let sent = sender.lock().await.blocks_sent();
        assert!(
            sent - before < total_blocks(&l),
            "cut {cut}: {sent} sent, {before} before"
        );
    }
    assert!(
        some_partway,
        "no cut left a file partway, so resuming was not tested"
    );
}

#[tokio::test]
async fn a_file_changed_during_the_break_is_sent_again_whole() {
    let l = laptops();
    let table = table(&l);
    let sender = Mutex::new(SenderSession::new(jobs(&l), 1));
    let receiver = Mutex::new(ReceiverSession::new(&table, plan(&l)));
    let (mut a, b) = mem_pair();
    a.cut_at = Some(30);
    let _ = run_both(&sender, &receiver, (a, b)).await;

    // The big file is edited while the laptops are apart.
    let edited = pattern(2_600_000, 99);
    std::fs::write(&l.files[2].0, &edited).unwrap();
    assert_eq!(run_both(&sender, &receiver, mem_pair()).await, (true, true));
    assert_eq!(
        std::fs::read(l.new_mine.path().join("Videos/big.bin")).unwrap(),
        edited
    );
}

#[tokio::test]
async fn a_file_changed_while_being_sent_arrives_whole_and_current() {
    let l = laptops();
    let table = table(&l);
    let sender = Mutex::new(SenderSession::new(jobs(&l), 1));
    let receiver = Mutex::new(ReceiverSession::new(&table, plan(&l)));
    let (mut a, b) = mem_pair();
    // Part way through the big file, another program rewrites it (same size, new contents).
    let big = l.files[2].0.clone();
    let edited = pattern(2_500_000, 77);
    let write = edited.clone();
    a.hook = Some((
        20,
        Box::new(move || {
            std::fs::write(&big, &write).unwrap();
            let f = std::fs::OpenOptions::new().write(true).open(&big).unwrap();
            f.set_modified(std::time::SystemTime::now() + std::time::Duration::from_secs(5))
                .unwrap();
        }),
    ));
    assert_eq!(run_both(&sender, &receiver, (a, b)).await, (true, true));
    assert_eq!(
        std::fs::read(l.new_mine.path().join("Videos/big.bin")).unwrap(),
        edited
    );
    assert_eq!(
        sender.lock().await.outcome(id(2)),
        Some(&SendOutcome::Arrived)
    );
}

#[tokio::test]
async fn a_file_for_a_place_that_was_not_approved_fails_on_its_own() {
    let l = laptops();
    let table = table(&l);
    let mut jobs = jobs(&l);
    jobs[1].destination = "somewhere-else".into();
    let sender = Mutex::new(SenderSession::new(jobs, 2));
    let receiver = Mutex::new(ReceiverSession::new(&table, plan(&l)));
    assert_eq!(run_both(&sender, &receiver, mem_pair()).await, (true, true));
    let s = sender.lock().await;
    assert_eq!(s.outcome(id(0)), Some(&SendOutcome::Arrived));
    assert!(matches!(s.outcome(id(1)), Some(SendOutcome::Failed(_))));
    assert_eq!(s.outcome(id(2)), Some(&SendOutcome::Arrived));
    assert!(!l.new_shared.path().join("Public/medium.bin").exists());
}

#[tokio::test]
async fn a_source_file_that_cannot_be_read_fails_on_its_own() {
    let l = laptops();
    let table = table(&l);
    let mut jobs = jobs(&l);
    jobs[0].source = Path::new("does-not-exist.bin").to_path_buf();
    let sender = Mutex::new(SenderSession::new(jobs, 2));
    let receiver = Mutex::new(ReceiverSession::new(&table, plan(&l)));
    assert_eq!(run_both(&sender, &receiver, mem_pair()).await, (true, true));
    let s = sender.lock().await;
    assert!(matches!(s.outcome(id(0)), Some(SendOutcome::Failed(_))));
    assert_eq!(s.outcome(id(2)), Some(&SendOutcome::Arrived));
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn a_move_runs_over_a_real_paired_link() {
    use std::time::{Duration, Instant};
    let l = laptops();
    let table = table(&l);
    let config = pctwin_link::LinkConfig {
        step_timeout: Duration::from_secs(5),
        handshake_timeout: Duration::from_secs(5),
        silence_penalty: Duration::ZERO,
        connect_timeout: Duration::from_secs(5),
    };
    let host = pctwin_link::Host::bind("127.0.0.1:0".parse().unwrap(), config)
        .await
        .unwrap();
    let addr = host.local_addr().unwrap();
    let rotating = std::sync::Arc::new(std::sync::Mutex::new(
        pctwin_pairing::RotatingSender::new(Instant::now()).unwrap(),
    ));
    let code = {
        let mut s = rotating.lock().unwrap();
        s.tick(Instant::now()).unwrap();
        pctwin_pairing::PairingCode::parse(&s.code().unwrap()).unwrap()
    };
    // The old laptop hosts; the new laptop connects with the code and the person picks the number.
    let guest =
        tokio::spawn(async move { pctwin_link::connect(addr, &code, config).await.unwrap() });
    let pending_host = host.next_peer(&rotating).await.unwrap();
    let pending_guest = guest.await.unwrap();
    let number = pending_guest.match_number();
    let mut old_link = pending_host.choose(number, Instant::now()).await.unwrap();
    let mut new_link = pending_guest.approval().await.unwrap();

    let mut sender = SenderSession::new(jobs(&l), 2);
    let mut receiver = ReceiverSession::new(&table, plan(&l));
    let (sent, received) = tokio::time::timeout(Duration::from_secs(60), async {
        tokio::join!(sender.run(&mut old_link), receiver.run(&mut new_link))
    })
    .await
    .unwrap();
    sent.unwrap();
    received.unwrap();
    assert_arrived(&l);
}

#[tokio::test]
async fn a_file_the_new_laptop_already_has_is_not_copied_again() {
    let l = laptops();
    let table = table(&l);
    // The new laptop already holds an identical copy of the medium file.
    std::fs::create_dir_all(l.new_shared.path().join("Public")).unwrap();
    std::fs::write(l.new_shared.path().join("Public/medium.bin"), &l.files[1].1).unwrap();
    let sender = Mutex::new(SenderSession::new(jobs(&l), 2));
    let receiver = Mutex::new(ReceiverSession::new(&table, plan(&l)));
    assert_eq!(run_both(&sender, &receiver, mem_pair()).await, (true, true));
    let s = sender.lock().await;
    assert_eq!(s.outcome(id(1)), Some(&SendOutcome::AlreadyThere));
    assert_eq!(s.outcome(id(0)), Some(&SendOutcome::Arrived));
    let medium_blocks = 300_000u64.div_ceil(131_072);
    assert_eq!(s.blocks_sent(), total_blocks(&l) - medium_blocks);
    let left: Vec<_> = std::fs::read_dir(l.new_shared.path().join("Public"))
        .unwrap()
        .map(|e| e.unwrap().file_name())
        .collect();
    assert_eq!(left, ["medium.bin"], "no second copy, no leftovers");
    assert!(matches!(
        receiver.lock().await.outcome(id(1)),
        Some(ReceiveOutcome::AlreadyThere(p)) if p == "Public/medium.bin"
    ));
}

#[tokio::test]
async fn a_different_file_with_the_same_name_and_size_is_kept_and_the_new_one_copied_beside_it() {
    let l = laptops();
    let table = table(&l);
    std::fs::create_dir_all(l.new_shared.path().join("Public")).unwrap();
    let theirs = pattern(300_000, 555);
    std::fs::write(l.new_shared.path().join("Public/medium.bin"), &theirs).unwrap();
    let sender = Mutex::new(SenderSession::new(jobs(&l), 2));
    let receiver = Mutex::new(ReceiverSession::new(&table, plan(&l)));
    assert_eq!(run_both(&sender, &receiver, mem_pair()).await, (true, true));
    assert_eq!(
        sender.lock().await.outcome(id(1)),
        Some(&SendOutcome::Arrived)
    );
    assert_eq!(
        std::fs::read(l.new_shared.path().join("Public/medium.bin")).unwrap(),
        theirs
    );
    assert_eq!(
        std::fs::read(l.new_shared.path().join("Public/medium (2).bin")).unwrap(),
        l.files[1].1
    );
}

#[tokio::test]
async fn a_file_removed_soon_after_copying_is_reported_not_copied() {
    let l = laptops();
    let table = table(&l);
    let sender = Mutex::new(SenderSession::new(jobs(&l), 2));
    let receiver = Mutex::new(ReceiverSession::new(&table, plan(&l)));
    assert_eq!(run_both(&sender, &receiver, mem_pair()).await, (true, true));
    // Security software removes one copy, and another is cut short.
    std::fs::remove_file(l.new_mine.path().join("Documents/small.bin")).unwrap();
    std::fs::OpenOptions::new()
        .write(true)
        .open(l.new_mine.path().join("Videos/big.bin"))
        .unwrap()
        .set_len(10)
        .unwrap();
    let mut r = receiver.lock().await;
    let mut gone = r.recheck();
    gone.sort();
    let mut expected = vec![id(0), id(2)];
    expected.sort();
    assert_eq!(gone, expected);
    assert!(matches!(r.outcome(id(0)), Some(ReceiveOutcome::Failed(_))));
    assert!(matches!(
        r.outcome(id(1)),
        Some(ReceiveOutcome::Finished(_))
    ));
    // Checking again finds nothing new.
    assert!(r.recheck().is_empty());
}

/// The test plays the old laptop itself, sending exactly the messages it chooses.
async fn say(ch: &mut Mem, m: &Message) {
    ch.send(&m.encode()).await.unwrap();
}

async fn hear(ch: &mut Mem) -> Message {
    Message::decode(&ch.recv().await.unwrap()).unwrap()
}

/// A new laptop that received blocks 2, 0 and 1 of the big file (in that order, as sections over
/// lanes may arrive) before the connection dropped. Each receipt names the block just written.
async fn three_blocks_then_a_drop<'d>(
    l: &Laptops,
    table: &'d Destinations,
) -> (ReceiverSession<'d>, Header, Vec<Block>) {
    let mut receiver = ReceiverSession::new(table, plan(l));
    let mut fs = FileSender::open(&l.files[2].0, None, true).unwrap();
    let header = fs.header().clone();
    let blocks: Vec<Block> = std::iter::from_fn(|| fs.next_block().unwrap()).collect();
    let (mut old, mut new) = mem_pair();
    let script = async {
        assert_eq!(hear(&mut old).await, Message::Ready);
        say(&mut old, &start(&header, 0, id(2))).await;
        assert!(matches!(
            hear(&mut old).await,
            Message::Have { stream: 0, .. }
        ));
        for b in [2usize, 0, 1] {
            for piece in split_into_pieces(0, &blocks[b].encode()) {
                say(&mut old, &piece).await;
            }
            assert_eq!(
                hear(&mut old).await,
                Message::Receipt {
                    stream: 0,
                    block: b as u64
                }
            );
        }
        old.cut.store(true, Ordering::SeqCst);
    };
    let ((), r) = tokio::time::timeout(std::time::Duration::from_secs(30), async {
        tokio::join!(script, receiver.run(&mut new))
    })
    .await
    .expect("hung");
    assert!(r.is_err(), "the connection dropped");
    (receiver, header, blocks)
}

fn start(header: &Header, resumed_done: u64, item: ItemId) -> Message {
    Message::StartFile {
        stream: 0,
        item,
        destination: "me".into(),
        path: "Videos/big.bin".into(),
        header: header.clone(),
        resumed_done,
    }
}

/// On the next connection the old laptop asks to continue: the new laptop says what it has, then
/// answers the start. Returns that answer.
async fn continue_with(
    receiver: &mut ReceiverSession<'_>,
    header: &Header,
    claim: Message,
) -> Message {
    let (mut old, mut new) = mem_pair();
    let script = async {
        match hear(&mut old).await {
            Message::ResumeFrom { stream: 0, ticket } => {
                assert_eq!(ticket.done.done_count(), 3);
                assert!((0..3).all(|b| ticket.done.contains(b)));
                assert_eq!(ticket.block_size, header.block_size);
            }
            other => panic!("expected the resume ticket, got {other:?}"),
        }
        assert_eq!(hear(&mut old).await, Message::Ready);
        say(&mut old, &claim).await;
        let answer = hear(&mut old).await;
        old.cut.store(true, Ordering::SeqCst);
        answer
    };
    let (answer, _) = tokio::time::timeout(std::time::Duration::from_secs(30), async {
        tokio::join!(script, receiver.run(&mut new))
    })
    .await
    .expect("hung");
    answer
}

#[tokio::test]
async fn an_old_laptop_continues_only_a_file_that_matches_what_is_here() {
    let l = laptops();
    let table = table(&l);
    let failed = Message::FileDone {
        stream: 0,
        ok: false,
    };
    // Honest: the same file, continuing from what the ticket said.
    let (mut r, header, _) = three_blocks_then_a_drop(&l, &table).await;
    let answer = continue_with(&mut r, &header, start(&header, 3, id(2))).await;
    assert!(
        matches!(
            answer,
            Message::Have {
                stream: 0,
                same_size: None
            }
        ),
        "{answer:?}"
    );
    assert_eq!(r.continued(), 1);
    // Claiming more blocks than are here: not continued.
    let (mut r, header, _) = three_blocks_then_a_drop(&l, &table).await;
    assert_eq!(
        continue_with(&mut r, &header, start(&header, 1000, id(2))).await,
        failed
    );
    assert_eq!(r.continued(), 0);
    // Continuing under this stream, but for another file: not continued.
    let (mut r, header, _) = three_blocks_then_a_drop(&l, &table).await;
    assert_eq!(
        continue_with(&mut r, &header, start(&header, 3, id(7))).await,
        failed
    );
    assert_eq!(r.continued(), 0);
}

/// Every file and folder under `dir`, temporary ones included.
fn everything_under(dir: &Path) -> Vec<PathBuf> {
    let mut out = Vec::new();
    for e in std::fs::read_dir(dir).unwrap() {
        let p = e.unwrap().path();
        if p.is_dir() {
            out.extend(everything_under(&p));
        }
        out.push(p);
    }
    out
}

#[tokio::test]
async fn the_new_laptop_starts_only_what_the_approved_plan_allows() {
    let l = laptops();
    let table = table(&l);
    let mut receiver = ReceiverSession::new(&table, plan(&l));
    let fs = FileSender::open(&l.files[2].0, None, true).unwrap();
    let honest = fs.header().clone();
    // The same file, announced as 1 TiB: far more than the plan allows for it.
    let mut huge = honest.clone();
    huge.size = 1 << 40;
    huge.block_size = 16 * 1024 * 1024;
    huge.block_count = huge.size.div_ceil(huge.block_size);
    let (mut old, mut new) = mem_pair();
    let failed = |stream| Message::FileDone { stream, ok: false };
    let script = async {
        assert_eq!(hear(&mut old).await, Message::Ready);
        // Not part of the plan at all.
        let mut stranger = start(&honest, 0, id(9));
        if let Message::StartFile { stream, .. } = &mut stranger {
            *stream = 5;
        }
        say(&mut old, &stranger).await;
        assert_eq!(hear(&mut old).await, failed(5));
        // In the plan, but announced far bigger than approved.
        say(&mut old, &start(&huge, 0, id(2))).await;
        assert_eq!(hear(&mut old).await, failed(0));
        old.cut.store(true, Ordering::SeqCst);
    };
    let ((), _) = tokio::time::timeout(std::time::Duration::from_secs(30), async {
        tokio::join!(script, receiver.run(&mut new))
    })
    .await
    .expect("hung");
    // Nothing was created or reserved for either.
    assert!(everything_under(l.new_mine.path()).is_empty());
    assert!(
        matches!(receiver.outcome(id(2)), Some(ReceiveOutcome::Failed(why)) if why.contains("grew"))
    );
    assert!(
        matches!(receiver.outcome(id(9)), Some(ReceiveOutcome::Failed(why)) if why.contains("plan"))
    );
}

/// Sends `blocks` (by index) of stream 0 over `lanes` together: one piece at a time from each lane
/// in turn, so pieces of different blocks of the same file are interleaved in time.
async fn interleave(lanes: &mut [&mut Mem], plan: &[Vec<usize>], blocks: &[Block]) {
    let mut queues: Vec<Vec<Message>> = plan
        .iter()
        .map(|bs| {
            bs.iter()
                .flat_map(|b| split_into_pieces(0, &blocks[*b].encode()))
                .collect()
        })
        .collect();
    for q in &mut queues {
        q.reverse();
    }
    while queues.iter().any(|q| !q.is_empty()) {
        for (lane, q) in lanes.iter_mut().zip(&mut queues) {
            if let Some(m) = q.pop() {
                say(lane, &m).await;
            }
        }
    }
}

#[tokio::test]
async fn a_file_sent_in_sections_over_three_lanes_arrives_whole() {
    let l = laptops();
    let table = table(&l);
    let mut receiver = ReceiverSession::new(&table, plan(&l));
    let mut fs = FileSender::open(&l.files[2].0, None, true).unwrap();
    let header = fs.header().clone();
    let blocks: Vec<Block> = std::iter::from_fn(|| fs.next_block().unwrap()).collect();
    let n = blocks.len();
    assert!(n >= 12);
    let (mut old, mut new) = mem_pair();
    let (mut old2, new2) = mem_pair();
    let (mut old3, new3) = mem_pair();
    let extra = [new2, new3];
    // Main sends the middle, lane 2 the start, lane 3 the end: three sections at once.
    let plan_of = vec![
        (n / 3..2 * n / 3).collect::<Vec<_>>(),
        (0..n / 3).collect(),
        (2 * n / 3..n).collect(),
    ];
    let script = async {
        assert_eq!(hear(&mut old).await, Message::Ready);
        say(&mut old, &start(&header, 0, id(2))).await;
        assert!(matches!(
            hear(&mut old).await,
            Message::Have { stream: 0, .. }
        ));
        interleave(&mut [&mut old, &mut old2, &mut old3], &plan_of, &blocks).await;
        // Each lane's receipts come back on that lane, in the order it sent, naming each block.
        for (lane, sent) in [&mut old, &mut old2, &mut old3].into_iter().zip(&plan_of) {
            for b in sent {
                assert_eq!(
                    hear(lane).await,
                    Message::Receipt {
                        stream: 0,
                        block: *b as u64
                    }
                );
            }
        }
        say(
            &mut old,
            &Message::EndFile {
                stream: 0,
                stamp_after: header.stamp,
                changed: false,
            },
        )
        .await;
        assert_eq!(
            hear(&mut old).await,
            Message::FileDone {
                stream: 0,
                ok: true
            }
        );
        say(&mut old, &Message::AllSent).await;
    };
    let ((), r) = tokio::time::timeout(std::time::Duration::from_secs(30), async {
        tokio::join!(script, receiver.run_lanes(&mut new, extra.into()))
    })
    .await
    .expect("hung");
    r.unwrap();
    assert_eq!(
        std::fs::read(l.new_mine.path().join("Videos/big.bin")).unwrap(),
        l.files[2].1
    );
}

#[tokio::test]
async fn a_lane_that_drops_mid_block_loses_nothing_and_the_rest_goes_on_the_main_lane() {
    let l = laptops();
    let table = table(&l);
    let mut receiver = ReceiverSession::new(&table, plan(&l));
    let mut fs = FileSender::open(&l.files[2].0, None, true).unwrap();
    let header = fs.header().clone();
    let blocks: Vec<Block> = std::iter::from_fn(|| fs.next_block().unwrap()).collect();
    let (mut old, mut new) = mem_pair();
    let (mut old2, new2) = mem_pair();
    let extra = [new2];
    let script = async {
        assert_eq!(hear(&mut old).await, Message::Ready);
        say(&mut old, &start(&header, 0, id(2))).await;
        assert!(matches!(
            hear(&mut old).await,
            Message::Have { stream: 0, .. }
        ));
        // Lane 2 sends blocks 0 and 1 whole and half of block 2, then drops.
        for b in [0usize, 1] {
            for piece in split_into_pieces(0, &blocks[b].encode()) {
                say(&mut old2, &piece).await;
            }
        }
        let pieces = split_into_pieces(0, &blocks[2].encode());
        assert!(pieces.len() >= 2);
        say(&mut old2, &pieces[0]).await;
        for b in [0u64, 1] {
            assert_eq!(
                hear(&mut old2).await,
                Message::Receipt {
                    stream: 0,
                    block: b
                }
            );
        }
        old2.cut.store(true, Ordering::SeqCst);
        // Everything not confirmed goes on the main lane, starting with the cut-off block.
        for (b, block) in blocks.iter().enumerate().skip(2) {
            for piece in split_into_pieces(0, &block.encode()) {
                say(&mut old, &piece).await;
            }
            assert_eq!(
                hear(&mut old).await,
                Message::Receipt {
                    stream: 0,
                    block: b as u64
                }
            );
        }
        say(
            &mut old,
            &Message::EndFile {
                stream: 0,
                stamp_after: header.stamp,
                changed: false,
            },
        )
        .await;
        assert_eq!(
            hear(&mut old).await,
            Message::FileDone {
                stream: 0,
                ok: true
            }
        );
        say(&mut old, &Message::AllSent).await;
    };
    let ((), r) = tokio::time::timeout(std::time::Duration::from_secs(30), async {
        tokio::join!(script, receiver.run_lanes(&mut new, extra.into()))
    })
    .await
    .expect("hung");
    r.unwrap();
    assert_eq!(
        std::fs::read(l.new_mine.path().join("Videos/big.bin")).unwrap(),
        l.files[2].1
    );
}

#[tokio::test]
async fn an_extra_lane_cannot_start_end_or_skip_files() {
    let l = laptops();
    let table = table(&l);
    let mut receiver = ReceiverSession::new(&table, plan(&l));
    let fs = FileSender::open(&l.files[2].0, None, true).unwrap();
    let header = fs.header().clone();
    let (mut old, mut new) = mem_pair();
    let (mut old2, new2) = mem_pair();
    let extra = [new2];
    let mut fs = FileSender::open(&l.files[2].0, None, true).unwrap();
    let blocks: Vec<Block> = std::iter::from_fn(|| fs.next_block().unwrap()).collect();
    let script = async {
        assert_eq!(hear(&mut old).await, Message::Ready);
        // A start on an extra lane is refused there: that lane is closed and nothing starts.
        say(&mut old2, &start(&header, 0, id(2))).await;
        tokio::time::sleep(std::time::Duration::from_millis(100)).await;
        // Had it started, block 1 sent now would be written and named in the receipt.
        for piece in split_into_pieces(0, &blocks[1].encode()) {
            say(&mut old, &piece).await;
        }
        assert_eq!(
            hear(&mut old).await,
            Message::Receipt {
                stream: 0,
                block: 0
            }
        );
        say(&mut old, &Message::AllSent).await;
    };
    let ((), r) = tokio::time::timeout(std::time::Duration::from_secs(30), async {
        tokio::join!(script, receiver.run_lanes(&mut new, extra.into()))
    })
    .await
    .expect("hung");
    r.unwrap();
    assert!(receiver.outcome(id(2)).is_none());
    assert!(everything_under(l.new_mine.path()).is_empty());
}

#[tokio::test]
async fn pieces_for_a_file_never_started_are_still_answered() {
    // Otherwise the old laptop would wait for those receipts for ever.
    let l = laptops();
    let table = table(&l);
    let mut receiver = ReceiverSession::new(&table, plan(&l));
    let mut fs = FileSender::open(&l.files[2].0, None, true).unwrap();
    let block = fs.next_block().unwrap().unwrap();
    let (mut old, mut new) = mem_pair();
    let (mut old2, new2) = mem_pair();
    let extra = [new2];
    let script = async {
        assert_eq!(hear(&mut old).await, Message::Ready);
        for lane in [&mut old, &mut old2] {
            for piece in split_into_pieces(9, &block.encode()) {
                say(lane, &piece).await;
            }
            assert_eq!(
                hear(lane).await,
                Message::Receipt {
                    stream: 9,
                    block: 0
                }
            );
        }
        say(&mut old, &Message::AllSent).await;
    };
    let ((), r) = tokio::time::timeout(std::time::Duration::from_secs(30), async {
        tokio::join!(script, receiver.run_lanes(&mut new, extra.into()))
    })
    .await
    .expect("hung");
    r.unwrap();
    assert!(everything_under(l.new_mine.path()).is_empty());
}

#[tokio::test]
async fn pieces_from_two_lanes_for_one_file_are_kept_apart() {
    let l = laptops();
    let table = table(&l);
    let mut receiver = ReceiverSession::new(&table, plan(&l));
    let mut fs = FileSender::open(&l.files[2].0, None, true).unwrap();
    let header = fs.header().clone();
    let blocks: Vec<Block> = std::iter::from_fn(|| fs.next_block().unwrap()).collect();
    let n = blocks.len();
    let (mut old, mut new) = mem_pair();
    let (mut old2, new2) = mem_pair();
    let (mut old3, new3) = mem_pair();
    let extra = [new2, new3];
    let script = async {
        assert_eq!(hear(&mut old).await, Message::Ready);
        say(&mut old, &start(&header, 0, id(2))).await;
        assert!(matches!(
            hear(&mut old).await,
            Message::Have { stream: 0, .. }
        ));
        // Lane 2 is part way through block 0 ...
        let first = split_into_pieces(0, &blocks[0].encode());
        let (last, before) = first.split_last().unwrap();
        for piece in before {
            say(&mut old2, piece).await;
        }
        tokio::time::sleep(std::time::Duration::from_millis(100)).await;
        // ... when lane 3 sends the last block whole.
        for piece in split_into_pieces(0, &blocks[n - 1].encode()) {
            say(&mut old3, &piece).await;
        }
        assert_eq!(
            hear(&mut old3).await,
            Message::Receipt {
                stream: 0,
                block: (n - 1) as u64
            }
        );
        say(&mut old2, last).await;
        assert_eq!(
            hear(&mut old2).await,
            Message::Receipt {
                stream: 0,
                block: 0
            }
        );
        for (b, block) in blocks.iter().enumerate().take(n - 1).skip(1) {
            for piece in split_into_pieces(0, &block.encode()) {
                say(&mut old, &piece).await;
            }
            assert_eq!(
                hear(&mut old).await,
                Message::Receipt {
                    stream: 0,
                    block: b as u64
                }
            );
        }
        say(
            &mut old,
            &Message::EndFile {
                stream: 0,
                stamp_after: header.stamp,
                changed: false,
            },
        )
        .await;
        assert_eq!(
            hear(&mut old).await,
            Message::FileDone {
                stream: 0,
                ok: true
            }
        );
        say(&mut old, &Message::AllSent).await;
    };
    let ((), r) = tokio::time::timeout(std::time::Duration::from_secs(30), async {
        tokio::join!(script, receiver.run_lanes(&mut new, extra.into()))
    })
    .await
    .expect("hung");
    r.unwrap();
    assert_eq!(
        std::fs::read(l.new_mine.path().join("Videos/big.bin")).unwrap(),
        l.files[2].1
    );
}

#[tokio::test]
async fn an_old_laptop_cannot_hold_more_than_the_open_file_limit() {
    // A plan of many small files; a hostile old laptop starts them all and never finishes any,
    // which would keep space reserved for every one.
    let l = laptops();
    let table = table(&l);
    let n = pctwin_transfer::MAX_OPEN_FILES as u8 + 1;
    let files: Vec<(ItemId, u64)> = (0..n).map(|k| (id(k), 1000)).collect();
    let mut receiver = ReceiverSession::new(&table, common::approved(&files));
    let header = |size| Header {
        size,
        block_size: 131_072,
        block_count: 1,
        stamp: pctwin_transfer::Stamp {
            size,
            modified_ns: None,
        },
    };
    let (mut old, mut new) = mem_pair();
    let script = async {
        assert_eq!(hear(&mut old).await, Message::Ready);
        for k in 0..n {
            let mut m = start(&header(1000), 0, id(k));
            if let Message::StartFile { stream, path, .. } = &mut m {
                *stream = u32::from(k);
                *path = format!("Documents/f{k}.bin");
            }
            say(&mut old, &m).await;
            let answer = hear(&mut old).await;
            if k + 1 < n {
                assert!(matches!(answer, Message::Have { .. }), "{k}: {answer:?}");
            } else {
                assert_eq!(
                    answer,
                    Message::FileDone {
                        stream: u32::from(k),
                        ok: false
                    }
                );
            }
        }
        old.cut.store(true, Ordering::SeqCst);
    };
    let ((), _) = tokio::time::timeout(std::time::Duration::from_secs(30), async {
        tokio::join!(script, receiver.run(&mut new))
    })
    .await
    .expect("hung");
    assert!(matches!(
        receiver.outcome(id(n - 1)),
        Some(ReceiveOutcome::Failed(why)) if why.contains("too many")
    ));
    // Nothing was created for the one refused.
    let made = everything_under(l.new_mine.path())
        .iter()
        .filter(|p| p.is_file())
        .count();
    assert_eq!(made, usize::from(n) - 1);
}

#[tokio::test]
async fn files_refused_at_their_start_do_not_count_toward_the_open_limit() {
    let l = laptops();
    let table = table(&l);
    let mut receiver = ReceiverSession::new(&table, plan(&l));
    let fs = FileSender::open(&l.files[2].0, None, true).unwrap();
    let header = fs.header().clone();
    let (mut old, mut new) = mem_pair();
    let script = async {
        assert_eq!(hear(&mut old).await, Message::Ready);
        // Many files the plan does not have: each refused, and each holding nothing.
        for k in 0..(pctwin_transfer::MAX_OPEN_FILES as u32 + 6) {
            let mut m = start(&header, 0, id(200));
            if let Message::StartFile { stream, .. } = &mut m {
                *stream = 100 + k;
            }
            say(&mut old, &m).await;
            assert_eq!(
                hear(&mut old).await,
                Message::FileDone {
                    stream: 100 + k,
                    ok: false
                }
            );
        }
        // A file of the plan still starts.
        say(&mut old, &start(&header, 0, id(2))).await;
        assert!(matches!(
            hear(&mut old).await,
            Message::Have { stream: 0, .. }
        ));
        old.cut.store(true, Ordering::SeqCst);
    };
    let ((), _) = tokio::time::timeout(std::time::Duration::from_secs(30), async {
        tokio::join!(script, receiver.run(&mut new))
    })
    .await
    .expect("hung");
}

/// The small file (one block), started on `stream`.
fn small_start(l: &Laptops, stream: u32) -> (Message, Vec<Block>) {
    let mut fs = FileSender::open(&l.files[0].0, None, true).unwrap();
    let header = fs.header().clone();
    let blocks: Vec<Block> = std::iter::from_fn(|| fs.next_block().unwrap()).collect();
    let start = Message::StartFile {
        stream,
        item: id(0),
        destination: "me".into(),
        path: "Documents/small.bin".into(),
        header,
        resumed_done: 0,
    };
    (start, blocks)
}

/// Sends a whole small file on `stream` and ends it; returns the answer to the end.
async fn send_small(old: &mut Mem, l: &Laptops, stream: u32) -> Message {
    let (start, blocks) = small_start(l, stream);
    say(old, &start).await;
    match hear(old).await {
        Message::Have { .. } => {}
        refused => return refused,
    }
    for piece in split_into_pieces(stream, &blocks[0].encode()) {
        say(old, &piece).await;
    }
    let _receipt = hear(old).await;
    let Message::StartFile { header, .. } = start else {
        unreachable!()
    };
    say(
        old,
        &Message::EndFile {
            stream,
            stamp_after: header.stamp,
            changed: false,
        },
    )
    .await;
    hear(old).await
}

fn files_under(dir: &Path) -> usize {
    everything_under(dir).iter().filter(|p| p.is_file()).count()
}

#[tokio::test]
async fn an_approved_file_lands_once_however_often_it_is_started() {
    // The review's repro: the same approved file started again and again on new streams.
    let l = laptops();
    let table = table(&l);
    let mut receiver = ReceiverSession::new(&table, plan(&l));
    let (mut old, mut new) = mem_pair();
    let script = async {
        assert_eq!(hear(&mut old).await, Message::Ready);
        assert_eq!(
            send_small(&mut old, &l, 0).await,
            Message::FileDone {
                stream: 0,
                ok: true
            }
        );
        for k in 1..14u32 {
            assert_eq!(
                send_small(&mut old, &l, 100 + k).await,
                Message::FileDone {
                    stream: 100 + k,
                    ok: false
                },
                "start {k}"
            );
        }
        say(&mut old, &Message::AllSent).await;
    };
    let ((), r) = tokio::time::timeout(std::time::Duration::from_secs(30), async {
        tokio::join!(script, receiver.run(&mut new))
    })
    .await
    .expect("hung");
    r.unwrap();
    assert_eq!(files_under(l.new_mine.path()), 1, "one copy, not fourteen");
}

#[tokio::test]
async fn a_second_start_of_a_file_replaces_the_first_and_nothing_piles_up() {
    let l = laptops();
    let table = table(&l);
    let mut receiver = ReceiverSession::new(&table, plan(&l));
    let (mut old, mut new) = mem_pair();
    let script = async {
        assert_eq!(hear(&mut old).await, Message::Ready);
        // Started on several streams without ever finishing: each replaces the one before,
        // and after a few tries the file is refused.
        let mut answers = Vec::new();
        for k in 0..(pctwin_transfer::MAX_ATTEMPTS + 2) {
            let (start, _) = small_start(&l, 10 + k);
            say(&mut old, &start).await;
            answers.push(hear(&mut old).await);
        }
        let started = answers
            .iter()
            .filter(|a| matches!(a, Message::Have { .. }))
            .count();
        assert_eq!(
            started,
            pctwin_transfer::MAX_ATTEMPTS as usize,
            "{answers:?}"
        );
        // Only one partial file is ever kept.
        assert!(files_under(l.new_mine.path()) <= 1);
        say(&mut old, &Message::AllSent).await;
    };
    let ((), r) = tokio::time::timeout(std::time::Duration::from_secs(30), async {
        tokio::join!(script, receiver.run(&mut new))
    })
    .await
    .expect("hung");
    r.unwrap();
    assert!(matches!(
        receiver.outcome(id(0)),
        Some(ReceiveOutcome::Failed(why)) if why.contains("kept changing")
    ));
}

#[tokio::test]
async fn pieces_of_two_files_mixed_on_one_connection_end_it() {
    // An honest old laptop sends each block's pieces together; mixing them is refused rather
    // than buffered.
    let l = laptops();
    let table = table(&l);
    let mut receiver = ReceiverSession::new(&table, plan(&l));
    let (mut old, mut new) = mem_pair();
    let script = async {
        assert_eq!(hear(&mut old).await, Message::Ready);
        let (start, blocks) = small_start(&l, 0);
        say(&mut old, &start).await;
        let _have = hear(&mut old).await;
        let mut fs = FileSender::open(&l.files[2].0, None, true).unwrap();
        let big = fs.next_block().unwrap().unwrap();
        say(&mut old, &self::start(fs.header(), 0, id(2)).clone()).await;
        let pieces = split_into_pieces(0, &big.encode());
        say(&mut old, &pieces[0]).await;
        // A piece of another stream while that block is half sent.
        say(&mut old, &split_into_pieces(7, &blocks[0].encode())[0]).await;
    };
    let ((), r) = tokio::time::timeout(std::time::Duration::from_secs(30), async {
        tokio::join!(script, receiver.run(&mut new))
    })
    .await
    .expect("hung");
    assert!(r.is_err(), "mixing pieces must end the connection");
}

#[tokio::test]
async fn pieces_for_a_file_that_is_not_open_are_answered_and_not_kept() {
    let l = laptops();
    let table = table(&l);
    let mut receiver = ReceiverSession::new(&table, plan(&l));
    let (mut old, mut new) = mem_pair();
    let script = async {
        assert_eq!(hear(&mut old).await, Message::Ready);
        // Thousands of pieces for a stream never started, the last one ending a "block".
        for _ in 0..2000 {
            say(
                &mut old,
                &Message::Piece {
                    stream: 77,
                    last: false,
                    bytes: vec![0; 1000],
                },
            )
            .await;
        }
        say(
            &mut old,
            &Message::Piece {
                stream: 77,
                last: true,
                bytes: vec![0; 10],
            },
        )
        .await;
        assert_eq!(
            hear(&mut old).await,
            Message::Receipt {
                stream: 77,
                block: 0
            }
        );
        // A real file still goes through.
        assert_eq!(
            send_small(&mut old, &l, 0).await,
            Message::FileDone {
                stream: 0,
                ok: true
            }
        );
        say(&mut old, &Message::AllSent).await;
    };
    let ((), r) = tokio::time::timeout(std::time::Duration::from_secs(30), async {
        tokio::join!(script, receiver.run(&mut new))
    })
    .await
    .expect("hung");
    r.unwrap();
}

#[tokio::test]
async fn refused_starts_do_not_pile_up_in_what_the_new_laptop_remembers() {
    let l = laptops();
    let table = table(&l);
    let mut receiver = ReceiverSession::new(&table, plan(&l));
    let fs = FileSender::open(&l.files[2].0, None, true).unwrap();
    let header = fs.header().clone();
    let (mut old, mut new) = mem_pair();
    let script = async {
        assert_eq!(hear(&mut old).await, Message::Ready);
        for k in 0..300u32 {
            let mut m = start(&header, 0, id(200));
            if let Message::StartFile { stream, .. } = &mut m {
                *stream = 1000 + k;
            }
            say(&mut old, &m).await;
            let _refused = hear(&mut old).await;
        }
        old.cut.store(true, Ordering::SeqCst);
    };
    let ((), _) = tokio::time::timeout(std::time::Duration::from_secs(60), async {
        tokio::join!(script, receiver.run(&mut new))
    })
    .await
    .expect("hung");
    // On the next connection, the new laptop's list of what it has does not repeat them.
    let (mut old, mut new) = mem_pair();
    let listing = async {
        let mut said = 0;
        loop {
            match hear(&mut old).await {
                Message::Ready => break,
                _ => said += 1,
            }
        }
        old.cut.store(true, Ordering::SeqCst);
        said
    };
    let (said, _) = tokio::join!(listing, receiver.run(&mut new));
    assert_eq!(said, 0);
}

#[tokio::test]
async fn a_block_bigger_than_its_files_block_size_fails_that_file() {
    let l = laptops();
    let table = table(&l);
    let mut receiver = ReceiverSession::new(&table, plan(&l));
    let (mut old, mut new) = mem_pair();
    let script = async {
        assert_eq!(hear(&mut old).await, Message::Ready);
        let (start, _) = small_start(&l, 0);
        let Message::StartFile { header, .. } = &start else {
            unreachable!()
        };
        let stamp = header.stamp;
        let block_size = header.block_size as usize;
        say(&mut old, &start).await;
        assert!(matches!(hear(&mut old).await, Message::Have { .. }));
        // Pieces past the file's own block size, never ending.
        let mut sent = 0;
        while sent <= block_size + 46 {
            say(
                &mut old,
                &Message::Piece {
                    stream: 0,
                    last: false,
                    bytes: vec![0; 60 * 1024],
                },
            )
            .await;
            sent += 60 * 1024;
        }
        // The overflowing block was dropped at once, so this connection is free for the next
        // block: a real block of another file goes straight through.
        let mut medium = FileSender::open(&l.files[1].0, None, true).unwrap();
        let mut m = self::start(medium.header(), 0, id(1));
        if let Message::StartFile { stream, path, .. } = &mut m {
            *stream = 1;
            *path = "Public/medium.bin".into();
        }
        say(&mut old, &m).await;
        assert!(matches!(
            hear(&mut old).await,
            Message::Have { stream: 1, .. }
        ));
        let first = medium.next_block().unwrap().unwrap();
        for piece in split_into_pieces(1, &first.encode()) {
            say(&mut old, &piece).await;
        }
        assert_eq!(
            hear(&mut old).await,
            Message::Receipt {
                stream: 1,
                block: 0
            }
        );
        say(
            &mut old,
            &Message::EndFile {
                stream: 0,
                stamp_after: stamp,
                changed: false,
            },
        )
        .await;
        assert_eq!(
            hear(&mut old).await,
            Message::FileDone {
                stream: 0,
                ok: false
            }
        );
        say(&mut old, &Message::AllSent).await;
    };
    let ((), r) = tokio::time::timeout(std::time::Duration::from_secs(30), async {
        tokio::join!(script, receiver.run(&mut new))
    })
    .await
    .expect("hung");
    r.unwrap();
    // Nothing of the overflowed file was left behind.
    assert!(!l.new_mine.path().join("Documents/small.bin").exists());
    // The only file is the second file's partial copy.
    assert_eq!(files_under(l.new_mine.path()), 1);
}

#[tokio::test]
async fn a_file_found_already_there_is_spent_too() {
    let l = laptops();
    let table = table(&l);
    // An identical copy is already on the new laptop.
    let there = l.new_mine.path().join("Documents");
    std::fs::create_dir_all(&there).unwrap();
    std::fs::write(there.join("small.bin"), &l.files[0].1).unwrap();
    let mut receiver = ReceiverSession::new(&table, plan(&l));
    let (mut old, mut new) = mem_pair();
    let script = async {
        assert_eq!(hear(&mut old).await, Message::Ready);
        let (start, _) = small_start(&l, 0);
        say(&mut old, &start).await;
        assert!(matches!(
            hear(&mut old).await,
            Message::Have {
                same_size: Some(_),
                ..
            }
        ));
        say(&mut old, &Message::Skip { stream: 0 }).await;
        // Started again on another stream: refused, it is already there.
        let (again, _) = small_start(&l, 5);
        say(&mut old, &again).await;
        assert_eq!(
            hear(&mut old).await,
            Message::FileDone {
                stream: 5,
                ok: false
            }
        );
        say(&mut old, &Message::AllSent).await;
    };
    let ((), r) = tokio::time::timeout(std::time::Duration::from_secs(30), async {
        tokio::join!(script, receiver.run(&mut new))
    })
    .await
    .expect("hung");
    r.unwrap();
    assert_eq!(files_under(l.new_mine.path()), 1);
}
