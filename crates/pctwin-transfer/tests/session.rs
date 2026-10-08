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
    let receiver = Mutex::new(ReceiverSession::new(&table));
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
    let mut receiver = ReceiverSession::new(&table);
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
        let receiver = Mutex::new(ReceiverSession::new(&table));
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
    let receiver = Mutex::new(ReceiverSession::new(&table));
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
    let receiver = Mutex::new(ReceiverSession::new(&table));
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
    let receiver = Mutex::new(ReceiverSession::new(&table));
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
    let receiver = Mutex::new(ReceiverSession::new(&table));
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
    let mut receiver = ReceiverSession::new(&table);
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
    let receiver = Mutex::new(ReceiverSession::new(&table));
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
    let receiver = Mutex::new(ReceiverSession::new(&table));
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
    let receiver = Mutex::new(ReceiverSession::new(&table));
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
    let mut receiver = ReceiverSession::new(table);
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
