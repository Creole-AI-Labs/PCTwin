//! Continuing a file after the app restarts (Task List 1.6, 1.5): the new laptop keeps each
//! landed block's fingerprint in the journal (in batches), and after a restart picks the partly
//! received file up again, trusting no block until its bytes in the file match its fingerprint
//! (as Syncthing re-hashes temporary files), so the old laptop sends only what is really missing.

use std::path::PathBuf;
use std::sync::Arc;
use std::sync::atomic::{AtomicBool, Ordering};

use pctwin_gate::{Approved, Destinations, temp_name};
use pctwin_journal::{Actor, FileId, Journal, Permission, PlannedWrite, State};
use pctwin_record::{ItemId, LaptopId};
use pctwin_transfer::{
    Channel, ChannelError, ReceiverSession, SendJob, SenderSession, Tier, block_size_for, recover,
};
use tokio::sync::mpsc;

mod common;

struct Mem {
    tx: mpsc::UnboundedSender<Vec<u8>>,
    rx: mpsc::UnboundedReceiver<Vec<u8>>,
    cut: Arc<AtomicBool>,
    sends: usize,
    cut_at: Option<usize>,
}

fn mem_pair(cut_at: Option<usize>) -> (Mem, Mem) {
    let (a_tx, b_rx) = mpsc::unbounded_channel();
    let (b_tx, a_rx) = mpsc::unbounded_channel();
    let cut = Arc::new(AtomicBool::new(false));
    (
        Mem {
            tx: a_tx,
            rx: a_rx,
            cut: cut.clone(),
            sends: 0,
            cut_at,
        },
        Mem {
            tx: b_tx,
            rx: b_rx,
            cut,
            sends: 0,
            cut_at: None,
        },
    )
}

impl Channel for Mem {
    async fn send(&mut self, data: &[u8]) -> Result<(), ChannelError> {
        tokio::task::yield_now().await;
        self.sends += 1;
        if self.cut_at == Some(self.sends) {
            self.cut.store(true, Ordering::SeqCst);
        }
        if self.cut.load(Ordering::SeqCst) {
            return Err(ChannelError);
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

fn id(n: u8) -> ItemId {
    ItemId::from_hex(&format!("{n:02x}{}", "0".repeat(30))).unwrap()
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

struct World {
    _old: tempfile::TempDir,
    mine: tempfile::TempDir,
    _jdir: tempfile::TempDir,
    journal: Journal,
    table: Destinations,
    source: PathBuf,
    data: Vec<u8>,
}

/// The old laptop the approved plan is for (see `common::approved`).
const LAPTOP: &str = "00112233445566778899aabbccddeeff";

/// 100 blocks of 128 KiB.
const SIZE: usize = 100 * 128 * 1024 - 7;

fn world() -> World {
    let old = tempfile::tempdir().unwrap();
    let mine = tempfile::tempdir().unwrap();
    let jdir = tempfile::tempdir().unwrap();
    let journal = Journal::open(&jdir.path().join("journal.redb")).unwrap();
    let mut table = Destinations::new();
    table
        .approve("me", Approved::MyFolders, mine.path())
        .unwrap();
    let data = pattern(SIZE, 9);
    let source = old.path().join("big.bin");
    std::fs::write(&source, &data).unwrap();
    World {
        _old: old,
        mine,
        _jdir: jdir,
        journal,
        table,
        source,
        data,
    }
}

impl World {
    fn job(&self) -> SendJob {
        SendJob {
            item: id(1),
            source: self.source.clone(),
            destination: "me".into(),
            path: "Videos/big.bin".into(),
            compressible: false,
            tier: Tier::Rest,
        }
    }
    fn plan(&self) -> pctwin_transfer::Allowance {
        common::approved(&[(id(1), SIZE as u64)])
    }
    fn receiver(&self) -> ReceiverSession<'_> {
        ReceiverSession::new(&self.table, self.plan(), &self.journal, "1001")
    }
    fn block_size(&self) -> u64 {
        block_size_for(SIZE as u64)
    }
    fn modified_ns(&self) -> Option<i64> {
        let t = std::fs::metadata(&self.source).unwrap().modified().unwrap();
        Some(t.duration_since(std::time::UNIX_EPOCH).unwrap().as_nanos() as i64)
    }
    /// What the app left behind when it stopped with `blocks` of the file in its temporary file
    /// and checkpointed: the entry staged, the file, the claims.
    fn left_behind(&self, blocks: &[u64], claims: &[(u64, [u8; 32])]) -> u64 {
        self.left_behind_as(LAPTOP, self.block_size(), blocks, claims)
    }

    fn left_behind_as(
        &self,
        laptop: &str,
        block_size: u64,
        blocks: &[u64],
        claims: &[(u64, [u8; 32])],
    ) -> u64 {
        let place = self
            .table
            .get("me")
            .unwrap()
            .folder_identity("")
            .unwrap()
            .unwrap();
        let entry = self
            .journal
            .plan(&PlannedWrite {
                item: id(1),
                source_laptop: LaptopId::from_hex(laptop).unwrap(),
                destination: "me".into(),
                path: "Videos/big.bin".into(),
                size: SIZE as u64,
                actor: Actor {
                    acting_account: "1001".into(),
                    for_account: "1001".into(),
                    permission: Permission::OwnFolders,
                },
                block_size,
                source_modified_ns: self.modified_ns(),
                source_file: None,
                partial_keep: Default::default(),
                place: Some(FileId {
                    volume: place.volume,
                    index: place.index,
                }),
            })
            .unwrap();
        let temp = format!("Videos/{}", temp_name(&self.journal.temp_tag(entry)));
        std::fs::create_dir_all(self.mine.path().join("Videos")).unwrap();
        // Reserved to full size, with zeros where nothing landed.
        let mut file = vec![0u8; SIZE];
        let bs = self.block_size() as usize;
        for b in blocks {
            let at = *b as usize * bs;
            let end = (at + bs).min(SIZE);
            file[at..end].copy_from_slice(&self.data[at..end]);
        }
        std::fs::write(self.mine.path().join(&temp), &file).unwrap();
        self.journal.staged(entry, &temp, &[]).unwrap();
        self.journal.checkpoint(entry, claims, true).unwrap();
        entry
    }
    fn hash(&self, b: u64) -> [u8; 32] {
        let bs = self.block_size() as usize;
        let at = b as usize * bs;
        *blake3::hash(&self.data[at..(at + bs).min(SIZE)]).as_bytes()
    }
}

async fn send_all(sender: &mut SenderSession, receiver: &mut ReceiverSession<'_>) {
    let (mut a, mut b) = mem_pair(None);
    let (sent, received) = tokio::time::timeout(std::time::Duration::from_secs(60), async {
        tokio::join!(sender.run(&mut a), receiver.run(&mut b))
    })
    .await
    .expect("the move hung");
    sent.unwrap();
    received.unwrap();
}

#[tokio::test]
async fn after_a_restart_only_blocks_proven_in_the_file_are_kept_and_the_rest_are_sent() {
    let w = world();
    let landed: Vec<u64> = (0..40).collect();
    let mut claims: Vec<(u64, [u8; 32])> = landed.iter().map(|b| (*b, w.hash(*b))).collect();
    // Claimed, but its bytes never reached the disk (still zeros).
    claims.push((60, w.hash(60)));
    // Claimed with a fingerprint the bytes there do not have.
    claims[5].1 = [0xAB; 32];
    let entry = w.left_behind(&landed, &claims);
    let rec = recover(&w.journal, &w.table).unwrap();
    assert_eq!(rec.resumable, [entry]);
    let mut receiver = w.receiver();
    assert_eq!(receiver.restore().unwrap(), 1);
    assert_eq!(receiver.partway(), 1);
    let mut sender = SenderSession::new(vec![w.job()], 1);
    send_all(&mut sender, &mut receiver).await;
    // 39 blocks proven in the file; the other 61 sent.
    assert_eq!(sender.blocks_sent(), 61);
    assert_eq!(receiver.continued(), 1);
    assert_eq!(
        std::fs::read(w.mine.path().join("Videos/big.bin")).unwrap(),
        w.data
    );
    assert!(matches!(
        w.journal.entry(entry).unwrap().unwrap().state,
        State::Committed { .. }
    ));
    assert!(w.journal.blocks(entry).unwrap().is_empty());
    assert_eq!(
        std::fs::read_dir(w.mine.path().join("Videos"))
            .unwrap()
            .count(),
        1
    );
}

#[tokio::test]
async fn a_file_changed_on_the_old_laptop_since_is_started_again_not_mixed() {
    let w = world();
    let landed: Vec<u64> = (0..40).collect();
    let claims: Vec<(u64, [u8; 32])> = landed.iter().map(|b| (*b, w.hash(*b))).collect();
    let entry = w.left_behind(&landed, &claims);
    // The original was edited while the new laptop was off.
    let mut edited = w.data.clone();
    edited[3] ^= 0xFF;
    std::fs::write(&w.source, &edited).unwrap();
    recover(&w.journal, &w.table).unwrap();
    let mut receiver = w.receiver();
    receiver.restore().unwrap();
    let mut sender = SenderSession::new(vec![w.job()], 1);
    send_all(&mut sender, &mut receiver).await;
    assert_eq!(sender.blocks_sent(), 100);
    assert_eq!(
        std::fs::read(w.mine.path().join("Videos/big.bin")).unwrap(),
        edited
    );
    assert!(matches!(
        w.journal.entry(entry).unwrap().unwrap().state,
        State::Failed { .. }
    ));
    assert_eq!(
        std::fs::read_dir(w.mine.path().join("Videos"))
            .unwrap()
            .count(),
        1
    );
}

#[tokio::test]
async fn a_file_no_longer_in_the_approved_plan_is_not_picked_up_again() {
    let w = world();
    let claims: Vec<(u64, [u8; 32])> = (0..10).map(|b| (b, w.hash(b))).collect();
    let entry = w.left_behind(&(0..10).collect::<Vec<_>>(), &claims);
    let other_plan = common::approved(&[(id(2), 10)]);
    let mut receiver = ReceiverSession::new(&w.table, other_plan, &w.journal, "1001");
    assert_eq!(receiver.restore().unwrap(), 0);
    assert!(matches!(
        w.journal.entry(entry).unwrap().unwrap().state,
        State::Failed { .. }
    ));
    assert_eq!(
        std::fs::read_dir(w.mine.path().join("Videos"))
            .unwrap()
            .count(),
        0
    );
}

#[tokio::test]
async fn blocks_are_checkpointed_as_they_land_and_a_crash_mid_move_continues_after_restart() {
    let w = world();
    let mut receiver = w.receiver();
    let mut sender = SenderSession::new(vec![w.job()], 1);
    // Cut partway (each block goes as a few pieces).
    let (mut a, mut b) = mem_pair(Some(300));
    let _ = tokio::time::timeout(std::time::Duration::from_secs(60), async {
        tokio::join!(sender.run(&mut a), receiver.run(&mut b))
    })
    .await
    .expect("hung");
    let entry = w.journal.unfinished().unwrap()[0].id;
    let checkpointed = w.journal.blocks(entry).unwrap().len();
    assert!(checkpointed >= 64, "{checkpointed}");
    // The app stops without any clean-up: nothing it would do on the way out happens.
    std::mem::forget(receiver);
    drop(sender);
    // Both apps start again: the new laptop's recovery, then a new move from scratch.
    let rec = recover(&w.journal, &w.table).unwrap();
    assert_eq!(rec.resumable, [entry]);
    let mut receiver = w.receiver();
    assert_eq!(receiver.restore().unwrap(), 1);
    let mut sender = SenderSession::new(vec![w.job()], 1);
    send_all(&mut sender, &mut receiver).await;
    assert!(
        sender.blocks_sent() <= 100 - checkpointed as u64,
        "{} sent",
        sender.blocks_sent()
    );
    assert_eq!(
        std::fs::read(w.mine.path().join("Videos/big.bin")).unwrap(),
        w.data
    );
    assert!(matches!(
        w.journal.entry(entry).unwrap().unwrap().state,
        State::Committed { .. }
    ));
}

#[tokio::test]
async fn a_file_from_another_old_laptop_or_another_folder_is_left_alone() {
    let w = world();
    let claims: Vec<(u64, [u8; 32])> = (0..10).map(|b| (b, w.hash(b))).collect();
    let blocks: Vec<u64> = (0..10).collect();
    let theirs = w.left_behind_as(
        "ffeeddccbbaa99887766554433221100",
        w.block_size(),
        &blocks,
        &claims,
    );
    let mut receiver = w.receiver();
    assert_eq!(receiver.restore().unwrap(), 0);
    assert!(matches!(
        w.journal.entry(theirs).unwrap().unwrap().state,
        State::Staged { .. }
    ));
    // Another folder under the same label: not touched either.
    let w2 = world();
    let ours = w2.left_behind(&blocks, &claims);
    let other = tempfile::tempdir().unwrap();
    let mut table = Destinations::new();
    table
        .approve("me", Approved::MyFolders, other.path())
        .unwrap();
    let mut receiver = ReceiverSession::new(&table, w2.plan(), &w2.journal, "1001");
    assert_eq!(receiver.restore().unwrap(), 0);
    assert!(matches!(
        w2.journal.entry(ours).unwrap().unwrap().state,
        State::Staged { .. }
    ));
    assert_eq!(
        std::fs::read_dir(w2.mine.path().join("Videos"))
            .unwrap()
            .count(),
        1
    );
}

#[tokio::test]
async fn a_partial_file_that_cannot_be_picked_up_again_is_failed_and_removed() {
    let w = world();
    // Recorded in a block size this file is never sent in.
    let entry = w.left_behind_as(LAPTOP, 4096, &[], &[]);
    let mut receiver = w.receiver();
    assert_eq!(receiver.restore().unwrap(), 0);
    assert!(matches!(
        w.journal.entry(entry).unwrap().unwrap().state,
        State::Failed { .. }
    ));
    assert_eq!(
        std::fs::read_dir(w.mine.path().join("Videos"))
            .unwrap()
            .count(),
        0
    );
    // And it can start again from the beginning.
    let mut sender = SenderSession::new(vec![w.job()], 1);
    send_all(&mut sender, &mut receiver).await;
    assert_eq!(
        std::fs::read(w.mine.path().join("Videos/big.bin")).unwrap(),
        w.data
    );
}

#[tokio::test]
async fn blocks_short_of_a_batch_are_checkpointed_when_the_next_connection_starts() {
    let w = world();
    let mut receiver = w.receiver();
    let mut sender = SenderSession::new(vec![w.job()], 1);
    // A few blocks, then a drop.
    let (mut a, mut b) = mem_pair(Some(40));
    let _ = tokio::time::timeout(std::time::Duration::from_secs(60), async {
        tokio::join!(sender.run(&mut a), receiver.run(&mut b))
    })
    .await
    .expect("hung");
    let entry = w.journal.unfinished().unwrap()[0].id;
    let landed = w.journal.blocks(entry).unwrap().len();
    assert!(landed < 64, "{landed}");
    assert_eq!(receiver.partway(), 1);
    // The next connection drops at once, but the new laptop recorded what it had first.
    let (mut a, mut b) = mem_pair(Some(1));
    let _ = tokio::time::timeout(std::time::Duration::from_secs(60), async {
        tokio::join!(sender.run(&mut a), receiver.run(&mut b))
    })
    .await
    .expect("hung");
    assert!(w.journal.blocks(entry).unwrap().len() > landed);
}

#[tokio::test]
async fn a_restored_file_is_never_continued_with_another_description() {
    use pctwin_transfer::{Header, Message, Stamp};
    let w = world();
    let claims: Vec<(u64, [u8; 32])> = (0..10).map(|b| (b, w.hash(b))).collect();
    let entry = w.left_behind(&(0..10).collect::<Vec<_>>(), &claims);
    let mut receiver = w.receiver();
    assert_eq!(receiver.restore().unwrap(), 1);
    // Claims to continue it, but describes a different original (another modified time).
    let header = Header {
        size: SIZE as u64,
        block_size: w.block_size(),
        block_count: (SIZE as u64).div_ceil(w.block_size()),
        stamp: Stamp {
            size: SIZE as u64,
            modified_ns: w.modified_ns().map(|n| n + 1),
        },
    };
    let (mut old, mut new) = mem_pair(None);
    let script = async {
        let hear = |m: Vec<u8>| Message::decode(&m).unwrap();
        let mut offered = None;
        loop {
            match hear(old.recv().await.unwrap()) {
                Message::ResumeFrom { item, ticket, .. } => {
                    assert_eq!(ticket.done.done_count(), 10);
                    offered = Some(item);
                }
                Message::Ready => break,
                _ => {}
            }
        }
        assert_eq!(offered, Some(id(1)));
        let start = Message::StartFile {
            stream: 5,
            item: id(1),
            destination: "me".into(),
            path: "Videos/big.bin".into(),
            header: header.clone(),
            resumed_done: 10,
        };
        old.send(&start.encode()).await.unwrap();
        let answer = hear(old.recv().await.unwrap());
        old.cut.store(true, Ordering::SeqCst);
        answer
    };
    let (answer, _) = tokio::time::timeout(std::time::Duration::from_secs(30), async {
        tokio::join!(script, receiver.run(&mut new))
    })
    .await
    .expect("hung");
    assert_eq!(
        answer,
        Message::FileDone {
            stream: 5,
            ok: false
        }
    );
    // Its partial file is never mixed with another original's blocks: it is gone.
    assert!(matches!(
        w.journal.entry(entry).unwrap().unwrap().state,
        State::Failed { .. }
    ));
    assert_eq!(
        std::fs::read_dir(w.mine.path().join("Videos"))
            .unwrap()
            .count(),
        0
    );
}

#[tokio::test]
async fn quitting_keeps_every_partly_received_file_for_the_next_start() {
    let w = world();
    let mut receiver = w.receiver();
    let mut sender = SenderSession::new(vec![w.job()], 1);
    let (mut a, mut b) = mem_pair(Some(300));
    let _ = tokio::time::timeout(std::time::Duration::from_secs(60), async {
        tokio::join!(sender.run(&mut a), receiver.run(&mut b))
    })
    .await
    .expect("hung");
    let entry = w.journal.unfinished().unwrap()[0].id;
    // A normal quit: the session ends, nothing leaked.
    drop(receiver);
    drop(sender);
    let rec = recover(&w.journal, &w.table).unwrap();
    assert_eq!(rec.resumable, [entry]);
    let mut receiver = w.receiver();
    assert_eq!(receiver.restore().unwrap(), 1);
    let mut sender = SenderSession::new(vec![w.job()], 1);
    send_all(&mut sender, &mut receiver).await;
    assert!(sender.blocks_sent() < 100, "{}", sender.blocks_sent());
    assert_eq!(
        std::fs::read(w.mine.path().join("Videos/big.bin")).unwrap(),
        w.data
    );
}

#[tokio::test]
async fn cancelling_records_every_unfinished_file_failed_and_removes_its_partial_file() {
    let w = world();
    let mut receiver = w.receiver();
    let mut sender = SenderSession::new(vec![w.job()], 1);
    let (mut a, mut b) = mem_pair(Some(300));
    let _ = tokio::time::timeout(std::time::Duration::from_secs(60), async {
        tokio::join!(sender.run(&mut a), receiver.run(&mut b))
    })
    .await
    .expect("hung");
    let entry = w.journal.unfinished().unwrap()[0].id;
    receiver.cancel("you cancelled the move");
    match w.journal.entry(entry).unwrap().unwrap().state {
        State::Failed { why, .. } => assert_eq!(why, "you cancelled the move"),
        other => panic!("{other:?}"),
    }
    assert_eq!(
        std::fs::read_dir(w.mine.path().join("Videos"))
            .unwrap()
            .count(),
        0
    );
}

#[tokio::test]
async fn no_more_partly_received_files_are_picked_up_than_may_be_open_at_once() {
    let w = world();
    let place = w
        .table
        .get("me")
        .unwrap()
        .folder_identity("")
        .unwrap()
        .unwrap();
    let n = pctwin_transfer::MAX_OPEN_FILES + 6;
    let files: Vec<(ItemId, u64)> = (0..n)
        .map(|k| {
            (
                ItemId::from_hex(&format!("{:04x}{}", k + 16, "0".repeat(28))).unwrap(),
                10,
            )
        })
        .collect();
    std::fs::create_dir_all(w.mine.path().join("Many")).unwrap();
    for (item, size) in &files {
        let entry = w
            .journal
            .plan(&PlannedWrite {
                item: *item,
                source_laptop: LaptopId::from_hex(LAPTOP).unwrap(),
                destination: "me".into(),
                path: format!("Many/{}.txt", item.to_hex()),
                size: *size,
                actor: Actor {
                    acting_account: "1001".into(),
                    for_account: "1001".into(),
                    permission: Permission::OwnFolders,
                },
                block_size: block_size_for(*size),
                source_modified_ns: None,
                source_file: None,
                partial_keep: Default::default(),
                place: Some(FileId {
                    volume: place.volume,
                    index: place.index,
                }),
            })
            .unwrap();
        let temp = format!("Many/{}", temp_name(&w.journal.temp_tag(entry)));
        std::fs::write(w.mine.path().join(&temp), b"").unwrap();
        w.journal.staged(entry, &temp, &[]).unwrap();
    }
    let mut receiver = ReceiverSession::new(&w.table, common::approved(&files), &w.journal, "1001");
    assert_eq!(
        receiver.restore().unwrap(),
        pctwin_transfer::MAX_OPEN_FILES as u64
    );
    // The rest stay in the journal, untouched, for a later start.
    let staged = w
        .journal
        .unfinished()
        .unwrap()
        .iter()
        .filter(|e| matches!(e.state, State::Staged { .. }))
        .count();
    assert_eq!(staged, n);
    // With every place taken by the files picked up, a new start is refused, not let in.
    use pctwin_transfer::{Header, Message, Stamp};
    let (mut old, mut new) = mem_pair(None);
    let last = files[n - 1].0;
    let script = async {
        loop {
            let m = Message::decode(&old.recv().await.unwrap()).unwrap();
            if m == Message::Ready {
                break;
            }
        }
        let start = Message::StartFile {
            stream: 99,
            item: last,
            destination: "me".into(),
            path: format!("Many/{}.txt", last.to_hex()),
            header: Header {
                size: 10,
                block_size: block_size_for(10),
                block_count: 1,
                stamp: Stamp {
                    size: 10,
                    modified_ns: None,
                },
            },
            resumed_done: 0,
        };
        old.send(&start.encode()).await.unwrap();
        let answer = Message::decode(&old.recv().await.unwrap()).unwrap();
        old.cut.store(true, Ordering::SeqCst);
        answer
    };
    let (answer, _) = tokio::time::timeout(std::time::Duration::from_secs(30), async {
        tokio::join!(script, receiver.run(&mut new))
    })
    .await
    .expect("hung");
    assert_eq!(
        answer,
        Message::FileDone {
            stream: 99,
            ok: false
        }
    );
}
