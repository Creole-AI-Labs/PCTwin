//! Copies cross once (Engineering Plan, copies; Security Design B): a file identical to one the
//! new laptop already received in this move, in the same person's things, is made there from that
//! one, through the same checks, and its bytes never cross the link again. Never across people;
//! never with a laptop on an older version; and if the copy already there cannot be used, the
//! file is sent after all.
// Tests make and remove files to test on; the no-file-changes fence is for the shipped code.
#![allow(clippy::disallowed_methods)]

use std::path::PathBuf;
use std::sync::{Arc, Mutex as StdMutex};

use pctwin_gate::{Approved, Destinations};
use pctwin_record::ItemId;
use pctwin_transfer::{
    Channel, ChannelError, Message, ReceiveOutcome, ReceiverSession, SendJob, SendOutcome,
    SenderSession, Tier, block_size_for,
};
use tokio::sync::mpsc;

mod common;

type Hook = Arc<StdMutex<Option<Box<dyn FnOnce() + Send>>>>;

/// One end of an in-memory connection; the old laptop's end can run a hook just before it says
/// its first `Skip`.
struct Mem {
    tx: mpsc::UnboundedSender<Vec<u8>>,
    rx: mpsc::UnboundedReceiver<Vec<u8>>,
    before_skip: Hook,
}

fn mem_pair(before_skip: Hook) -> (Mem, Mem) {
    let (a_tx, b_rx) = mpsc::unbounded_channel();
    let (b_tx, a_rx) = mpsc::unbounded_channel();
    (
        Mem {
            tx: a_tx,
            rx: a_rx,
            before_skip,
        },
        Mem {
            tx: b_tx,
            rx: b_rx,
            before_skip: Arc::new(StdMutex::new(None)),
        },
    )
}

impl Channel for Mem {
    async fn send(&mut self, data: &[u8]) -> Result<(), ChannelError> {
        tokio::task::yield_now().await;
        if let Ok(Message::Skip { .. }) = Message::decode(data)
            && let Some(hook) = self.before_skip.lock().unwrap().take()
        {
            hook();
        }
        self.tx.send(data.to_vec()).map_err(|_| ChannelError)
    }

    async fn recv(&mut self) -> Result<Vec<u8>, ChannelError> {
        self.rx.recv().await.ok_or(ChannelError)
    }
}

fn id(n: u8) -> ItemId {
    ItemId::from_hex(&format!("{n:02x}{}", "0".repeat(30))).unwrap()
}

fn pattern(len: usize, seed: u32) -> Vec<u8> {
    let mut x = seed.wrapping_mul(2_654_435_761).max(1);
    (0..len)
        .map(|_| {
            x ^= x << 13;
            x ^= x >> 17;
            x ^= x << 5;
            x as u8
        })
        .collect()
}

/// The new laptop's places: the signed-in person's folders ("me", "pictures"), and two shared
/// places that are not theirs together.
struct NewLaptop {
    me: tempfile::TempDir,
    pictures: tempfile::TempDir,
    shared: tempfile::TempDir,
    other: tempfile::TempDir,
}

impl NewLaptop {
    fn new() -> Self {
        Self {
            me: tempfile::tempdir().unwrap(),
            pictures: tempfile::tempdir().unwrap(),
            shared: tempfile::tempdir().unwrap(),
            other: tempfile::tempdir().unwrap(),
        }
    }

    fn table(&self) -> Destinations {
        let mut t = Destinations::new();
        t.approve("me", Approved::MyFolders, self.me.path()).unwrap();
        t.approve("pictures", Approved::MyFolders, self.pictures.path())
            .unwrap();
        t.approve("shared", Approved::SharedFolder, self.shared.path())
            .unwrap();
        t.approve("other", Approved::SharedFolder, self.other.path())
            .unwrap();
        t
    }

    fn root(&self, label: &str) -> PathBuf {
        match label {
            "me" => self.me.path(),
            "pictures" => self.pictures.path(),
            "shared" => self.shared.path(),
            _ => self.other.path(),
        }
        .to_path_buf()
    }
}

/// One file on the old laptop and where it lands.
struct File {
    bytes: Vec<u8>,
    destination: &'static str,
    path: &'static str,
}

struct Move {
    _old: tempfile::TempDir,
    new: NewLaptop,
    files: Vec<File>,
    jobs: Vec<SendJob>,
}

/// The files on the old laptop; the first is sent first, alone, so it has landed before the
/// others start.
fn a_move(files: Vec<File>) -> Move {
    let old = tempfile::tempdir().unwrap();
    let jobs = files
        .iter()
        .enumerate()
        .map(|(n, f)| {
            let source = old.path().join(format!("f{n}.bin"));
            std::fs::write(&source, &f.bytes).unwrap();
            SendJob {
                size: f.bytes.len() as u64,
                item: id(n as u8),
                source,
                destination: f.destination.into(),
                path: f.path.into(),
                compressible: false,
                tier: if n == 0 { Tier::Essential } else { Tier::Rest },
            }
        })
        .collect();
    Move {
        _old: old,
        new: NewLaptop::new(),
        files,
        jobs,
    }
}

fn blocks(len: usize) -> u64 {
    (len as u64).div_ceil(block_size_for(len as u64))
}

struct Ran {
    sent: SendOutcomes,
    blocks_sent: u64,
    received: Vec<Option<ReceiveOutcome>>,
}

type SendOutcomes = Vec<Option<SendOutcome>>;

async fn run(m: &Move, version: u16, before_skip: Option<Box<dyn FnOnce() + Send>>) -> Ran {
    let table = m.new.table();
    let sizes: Vec<(&SendJob, u64)> = m.jobs.iter().map(|j| (j, j.size)).collect();
    let mut receiver = ReceiverSession::new(
        &table,
        common::approved_jobs(&sizes),
        common::journal(),
        "1001",
    )
    .with_protocol(version);
    let mut sender = SenderSession::new(m.jobs.clone(), 1).with_protocol(version);
    let (mut a, mut b) = mem_pair(Arc::new(StdMutex::new(before_skip)));
    let total: u64 = m.files.iter().map(|f| f.bytes.len() as u64).sum();
    let landed = receiver.meter();
    let (s, r) = common::unless_stuck(&landed, total, async {
        tokio::join!(sender.run(&mut a), receiver.run(&mut b))
    })
    .await;
    assert!(s.is_ok() && r.is_ok(), "the move ran to its end");
    for f in &m.files {
        assert_eq!(
            std::fs::read(m.new.root(f.destination).join(f.path)).unwrap(),
            f.bytes,
            "{} arrived whole",
            f.path
        );
    }
    let n = m.jobs.len() as u8;
    Ran {
        sent: (0..n).map(|i| sender.outcome(id(i)).cloned()).collect(),
        blocks_sent: sender.blocks_sent(),
        received: (0..n).map(|i| receiver.outcome(id(i))).collect(),
    }
}

const BIG: usize = 2_500_000;

#[tokio::test]
async fn a_copy_in_one_person_s_things_crosses_the_link_once() {
    let photo = pattern(BIG, 7);
    let m = a_move(vec![
        File {
            bytes: photo.clone(),
            destination: "pictures",
            path: "Pictures/beach.jpg",
        },
        File {
            bytes: photo.clone(),
            destination: "me",
            path: "Documents/Copy of beach.jpg",
        },
        // Same size, different bytes: sent, never taken for a copy.
        File {
            bytes: pattern(BIG, 8),
            destination: "me",
            path: "Documents/other.jpg",
        },
    ]);
    let ran = run(&m, 4, None).await;
    assert_eq!(ran.blocks_sent, 2 * blocks(BIG), "the copy's blocks never crossed");
    assert_eq!(ran.sent[0], Some(SendOutcome::Arrived));
    assert_eq!(ran.sent[1], Some(SendOutcome::AlreadyThere));
    assert_eq!(ran.sent[2], Some(SendOutcome::Arrived));
    // On the new laptop the copy is a file copied and checked like any other.
    assert!(matches!(ran.received[1], Some(ReceiveOutcome::Finished(_))));
    let left: Vec<_> = std::fs::read_dir(m.new.me.path().join("Documents"))
        .unwrap()
        .map(|e| e.unwrap().file_name().into_string().unwrap())
        .collect();
    assert_eq!(left.len(), 2, "no leftovers: {left:?}");
}

#[tokio::test]
async fn copies_are_never_made_across_people_or_shared_places() {
    let photo = pattern(BIG, 9);
    let m = a_move(vec![
        File {
            bytes: photo.clone(),
            destination: "me",
            path: "beach.jpg",
        },
        // Another place that is not the person's own: sent in full, never made from theirs.
        File {
            bytes: photo.clone(),
            destination: "shared",
            path: "beach.jpg",
        },
        File {
            bytes: photo.clone(),
            destination: "other",
            path: "beach.jpg",
        },
    ]);
    let ran = run(&m, 4, None).await;
    assert_eq!(ran.blocks_sent, 3 * blocks(BIG));
    assert!(
        ran.sent
            .iter()
            .all(|o| *o == Some(SendOutcome::Arrived))
    );
}

#[tokio::test]
async fn with_a_laptop_on_version_three_every_file_is_sent() {
    let photo = pattern(BIG, 10);
    let m = a_move(vec![
        File {
            bytes: photo.clone(),
            destination: "me",
            path: "a.jpg",
        },
        File {
            bytes: photo,
            destination: "me",
            path: "b.jpg",
        },
    ]);
    let ran = run(&m, 3, None).await;
    assert_eq!(ran.blocks_sent, 2 * blocks(BIG));
}

#[tokio::test]
async fn small_files_are_sent_not_compared() {
    let note = pattern(200_000, 11);
    let m = a_move(vec![
        File {
            bytes: note.clone(),
            destination: "me",
            path: "a.txt",
        },
        File {
            bytes: note,
            destination: "me",
            path: "b.txt",
        },
    ]);
    let ran = run(&m, 4, None).await;
    assert_eq!(ran.blocks_sent, 2 * blocks(200_000));
}

#[tokio::test]
async fn a_copy_that_changed_before_it_was_used_is_sent_after_all() {
    let photo = pattern(BIG, 12);
    let m = a_move(vec![
        File {
            bytes: photo.clone(),
            destination: "me",
            path: "beach.jpg",
        },
        File {
            bytes: photo.clone(),
            destination: "me",
            path: "Copy of beach.jpg",
        },
    ]);
    // Between the offer and the copy, someone edits the copy already there (same size).
    let already = m.new.me.path().join("beach.jpg");
    let edited = pattern(BIG, 13);
    let hook: Box<dyn FnOnce() + Send> = Box::new(move || std::fs::write(already, edited).unwrap());
    let table = m.new.table();
    let sizes: Vec<(&SendJob, u64)> = m.jobs.iter().map(|j| (j, j.size)).collect();
    let mut receiver = ReceiverSession::new(
        &table,
        common::approved_jobs(&sizes),
        common::journal(),
        "1001",
    );
    let mut sender = SenderSession::new(m.jobs.clone(), 1);
    let (mut a, mut b) = mem_pair(Arc::new(StdMutex::new(Some(hook))));
    let landed = receiver.meter();
    let (s, r) = common::unless_stuck(&landed, 2 * BIG as u64, async {
        tokio::join!(sender.run(&mut a), receiver.run(&mut b))
    })
    .await;
    assert!(s.is_ok() && r.is_ok());
    // Sent after all, whole, and never made from the edited file.
    assert_eq!(
        std::fs::read(m.new.me.path().join("Copy of beach.jpg")).unwrap(),
        photo
    );
    assert_eq!(sender.outcome(id(1)), Some(&SendOutcome::Arrived));
    assert_eq!(sender.blocks_sent(), 2 * blocks(BIG));
    assert!(matches!(
        receiver.outcome(id(1)),
        Some(ReceiveOutcome::Finished(_))
    ));
}
