//! Sending again (quarantine design, step 2): a block that arrives damaged is answered "not
//! written"; a file whose blocks are missing or read back wrong when it ends is asked for again,
//! only those blocks, at most twice, and named only once a read-back matches. A file that changed
//! on the old laptop meanwhile is sent again whole, as before. A drive that keeps storing it wrong
//! fails the file after the last round, and nothing of it is left behind.
// Tests make and remove files to test on; the no-file-changes fence is for the shipped code.
#![allow(clippy::disallowed_methods)]

use std::collections::{BTreeMap, BTreeSet, HashMap, HashSet};
use std::io::{Read, Seek, SeekFrom};
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicI64, AtomicU32, AtomicU64, AtomicUsize, Ordering};
use std::sync::{Arc, Mutex};
use std::time::Duration;

use pctwin_gate::{Approved, Destinations, FaultPlan, Flip};
use pctwin_record::ItemId;
use pctwin_scan::ReadPlan;
use pctwin_transfer::{
    BlockMap, Cause, Channel, ChannelError, Failure, FileSender, FsOpener, GiveUpWhy, Header,
    MAX_ATTEMPTS, MAX_OPEN_FILES, Message, Opener, ReceiveOutcome, ReceiverSession, ResumeTicket,
    Scheduler, SendJob, SendOutcome, SenderSession, Source, Stamp, Tier, block_size_for,
    split_into_pieces,
};
use proptest::prelude::*;

/// One move's scope in the journal, before and after the app restarts.
const SAME_MOVE: pctwin_journal::Scope = pctwin_journal::Scope(*b"the same move...");
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

async fn say(ch: &mut Mem, m: &Message) {
    ch.send(&m.encode()).await.unwrap();
}

async fn hear(ch: &mut Mem) -> Message {
    Message::decode(&ch.recv().await.unwrap()).unwrap()
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

fn blocks_of(len: usize) -> u64 {
    (len as u64).div_ceil(block_size_for(len as u64))
}

struct Laptops {
    _old: tempfile::TempDir,
    new: tempfile::TempDir,
    files: Vec<(PathBuf, Vec<u8>)>,
}

/// The old laptop's files, of these sizes, all landing in "me".
fn laptops(sizes: &[usize]) -> Laptops {
    let old = tempfile::tempdir_in(env!("CARGO_TARGET_TMPDIR")).unwrap();
    let files = sizes
        .iter()
        .enumerate()
        .map(|(i, len)| {
            let p = old.path().join(format!("f{i}.bin"));
            let bytes = pattern(*len, i as u32 + 7);
            std::fs::write(&p, &bytes).unwrap();
            (p, bytes)
        })
        .collect();
    Laptops {
        _old: old,
        new: tempfile::tempdir_in(env!("CARGO_TARGET_TMPDIR")).unwrap(),
        files,
    }
}

fn jobs(l: &Laptops) -> Vec<SendJob> {
    (0..l.files.len())
        .map(|n| SendJob {
            size: 0,
            item: id(n as u8),
            source: l.files[n].0.clone(),
            destination: "me".into(),
            path: format!("Documents/f{n}.bin"),
            // Sent as they are, so a test can damage a block's contents on the wire.
            compressible: false,
            tier: Tier::Rest,
        })
        .collect()
}

fn plan(l: &Laptops) -> pctwin_transfer::Allowance {
    let jobs = jobs(l);
    let files: Vec<(&SendJob, u64)> = jobs
        .iter()
        .enumerate()
        .map(|(n, j)| (j, l.files[n].1.len() as u64))
        .collect();
    common::approved_jobs(&files)
}

fn table(l: &Laptops, faults: Option<Flip>) -> Destinations {
    let mut t = Destinations::new();
    t.approve("me", Approved::MyFolders, l.new.path()).unwrap();
    if let Some(flip) = faults {
        t.fake_faults(
            "me",
            FaultPlan {
                flip: Some(flip),
                ..Default::default()
            },
        )
        .unwrap();
    }
    t
}

fn landed(l: &Laptops, n: usize) -> PathBuf {
    l.new.path().join(format!("Documents/f{n}.bin"))
}

async fn move_all(sender: &mut SenderSession, receiver: &mut ReceiverSession<'_>) -> (bool, bool) {
    let (mut a, mut b) = mem_pair();
    let (sent, received) = tokio::time::timeout(std::time::Duration::from_secs(60), async {
        tokio::join!(sender.run(&mut a), receiver.run(&mut b))
    })
    .await
    .expect("the move hung");
    (sent.is_ok(), received.is_ok())
}

#[tokio::test]
async fn a_block_stored_wrong_is_sent_again_and_the_file_arrives() {
    let l = laptops(&[300_000, 70_000]);
    let table = table(&l, Some(Flip::OnWriteOnce));
    let mut sender = SenderSession::new(jobs(&l), 2);
    let mut receiver = ReceiverSession::new(&table, plan(&l), common::journal(), "1001");
    assert_eq!(move_all(&mut sender, &mut receiver).await, (true, true));
    for n in 0..2 {
        assert_eq!(sender.outcome(id(n as u8)), Some(&SendOutcome::Arrived));
        assert!(matches!(
            receiver.outcome(id(n as u8)),
            Some(ReceiveOutcome::Finished(_))
        ));
        assert_eq!(std::fs::read(landed(&l, n)).unwrap(), l.files[n].1);
    }
    // Exactly the one block stored wrong was sent again, not its whole file.
    let total: u64 = l.files.iter().map(|(_, b)| blocks_of(b.len())).sum();
    assert_eq!(sender.blocks_sent(), total + 1);
}

#[tokio::test]
async fn a_drive_that_keeps_storing_it_wrong_fails_the_file_after_two_more_rounds() {
    let l = laptops(&[70_000]);
    let table = table(&l, Some(Flip::OnEveryWrite));
    let mut sender = SenderSession::new(jobs(&l), 1);
    let mut receiver = ReceiverSession::new(&table, plan(&l), common::journal(), "1001");
    assert_eq!(move_all(&mut sender, &mut receiver).await, (true, true));
    assert!(matches!(
        sender.outcome(id(0)),
        Some(SendOutcome::Failed(_))
    ));
    match receiver.outcome(id(0)) {
        Some(ReceiveOutcome::Failed(why)) => {
            assert!(why.contains("could not store this file correctly"), "{why}");
        }
        other => panic!("expected the file to fail, got {other:?}"),
    }
    // Every block once, then every block again in each of the two rounds: never more.
    assert_eq!(sender.blocks_sent(), 3 * blocks_of(l.files[0].1.len()));
    assert!(!landed(&l, 0).exists());
    let left: Vec<_> = std::fs::read_dir(l.new.path().join("Documents"))
        .map(|d| d.map(|e| e.unwrap().file_name()).collect())
        .unwrap_or_default();
    assert!(left.is_empty(), "nothing left behind: {left:?}");
}

/// The test plays the old laptop for one file: the start, then each block (the one at `damage`
/// with a byte of its contents changed on the way), then the end. Returns what the new laptop
/// answered to each block, and to the end.
async fn send_file(
    old: &mut Mem,
    fs: &mut FileSender,
    blocks: &[u64],
    damage: Option<u64>,
) -> (Vec<Message>, Message) {
    let mut answers = Vec::new();
    for &b in blocks {
        let mut wire = fs.block_at(b).unwrap().encode();
        if damage == Some(b) {
            // Past the block's own header: its contents, as sent uncompressed.
            let at = wire.len() - 1;
            wire[at] ^= 0x5A;
        }
        for piece in split_into_pieces(0, &wire) {
            say(old, &piece).await;
        }
        answers.push(hear(old).await);
    }
    let stamp = fs.header().stamp;
    say(
        old,
        &Message::EndFile {
            stream: 0,
            stamp_after: stamp,
            changed: false,
        },
    )
    .await;
    (answers, hear(old).await)
}

#[tokio::test]
async fn a_block_damaged_on_the_way_is_not_written_and_is_asked_for_again() {
    let l = laptops(&[300_000]);
    let table = table(&l, None);
    let mut receiver = ReceiverSession::new(&table, plan(&l), common::journal(), "1001");
    let mut fs = FileSender::open(&l.files[0].0, None, false).unwrap();
    let header = fs.header().clone();
    let count = header.block_count;
    assert!(count >= 3, "{count}");
    let (mut old, mut new) = mem_pair();
    let script = async {
        assert_eq!(hear(&mut old).await, Message::Ready);
        say(
            &mut old,
            &Message::StartFile {
                stream: 0,
                item: id(0),
                destination: "me".into(),
                path: "Documents/f0.bin".into(),
                header: header.clone(),
                resumed_done: 0,
            },
        )
        .await;
        assert!(matches!(hear(&mut old).await, Message::Have { .. }));
        let all: Vec<u64> = (0..count).collect();
        let (answers, end) = send_file(&mut old, &mut fs, &all, Some(1)).await;
        for (b, a) in answers.iter().enumerate() {
            let written = b != 1;
            assert_eq!(
                *a,
                Message::Receipt {
                    stream: 0,
                    block: b as u64,
                    written
                },
                "block {b}"
            );
        }
        // Only block 1 is asked for again.
        let Message::SendAgain { stream: 0, ticket } = end else {
            panic!("expected to be asked again, got {end:?}");
        };
        assert_eq!(ticket.done.done_count(), count - 1);
        assert!(!ticket.done.contains(1));
        assert_eq!(ticket.stamp, header.stamp);
        let (answers, end) = send_file(&mut old, &mut fs, &[1], None).await;
        assert_eq!(
            answers,
            [Message::Receipt {
                stream: 0,
                block: 1,
                written: true
            }]
        );
        assert_eq!(
            end,
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
    assert_eq!(std::fs::read(landed(&l, 0)).unwrap(), l.files[0].1);
}

#[tokio::test]
async fn a_file_damaged_on_the_way_three_times_fails_and_leaves_nothing() {
    let l = laptops(&[70_000]);
    let table = table(&l, None);
    let mut receiver = ReceiverSession::new(&table, plan(&l), common::journal(), "1001");
    let mut fs = FileSender::open(&l.files[0].0, None, false).unwrap();
    let header = fs.header().clone();
    let (mut old, mut new) = mem_pair();
    let script = async {
        assert_eq!(hear(&mut old).await, Message::Ready);
        say(
            &mut old,
            &Message::StartFile {
                stream: 0,
                item: id(0),
                destination: "me".into(),
                path: "Documents/f0.bin".into(),
                header: header.clone(),
                resumed_done: 0,
            },
        )
        .await;
        assert!(matches!(hear(&mut old).await, Message::Have { .. }));
        let all: Vec<u64> = (0..header.block_count).collect();
        let (_, end) = send_file(&mut old, &mut fs, &all, Some(0)).await;
        assert!(matches!(end, Message::SendAgain { .. }), "{end:?}");
        let (_, end) = send_file(&mut old, &mut fs, &[0], Some(0)).await;
        assert!(matches!(end, Message::SendAgain { .. }), "{end:?}");
        let (_, end) = send_file(&mut old, &mut fs, &[0], Some(0)).await;
        assert_eq!(
            end,
            Message::FileDone {
                stream: 0,
                ok: false
            },
            "no third round"
        );
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
        Some(ReceiveOutcome::Failed(_))
    ));
    assert!(!landed(&l, 0).exists());
}

#[tokio::test]
async fn a_file_changed_on_the_old_laptop_before_it_is_sent_again_is_not_patched() {
    let l = laptops(&[70_000]);
    let header = FileSender::open(&l.files[0].0, None, false)
        .unwrap()
        .header()
        .clone();
    let mut sender = SenderSession::new(jobs(&l), 1);
    let (mut old, mut new) = mem_pair();
    let script = async {
        say(&mut new, &Message::Ready).await;
        assert!(matches!(
            hear(&mut new).await,
            Message::StartFile { stream: 0, .. }
        ));
        say(
            &mut new,
            &Message::Have {
                stream: 0,
                same_size: None,
            },
        )
        .await;
        // Every block, each receipted as written.
        for b in 0..header.block_count {
            loop {
                if let Message::Piece { last: true, .. } = hear(&mut new).await {
                    break;
                }
            }
            say(
                &mut new,
                &Message::Receipt {
                    stream: 0,
                    block: b,
                    written: true,
                },
            )
            .await;
        }
        assert!(matches!(
            hear(&mut new).await,
            Message::EndFile { changed: false, .. }
        ));
        // The file changes on the old laptop, then the new laptop asks for block 0 again.
        std::fs::write(&l.files[0].0, pattern(l.files[0].1.len() + 1, 99)).unwrap();
        let mut done = BlockMap::new(header.block_count);
        for b in 1..header.block_count {
            done.insert(b).unwrap();
        }
        say(
            &mut new,
            &Message::SendAgain {
                stream: 0,
                ticket: ResumeTicket {
                    done,
                    block_size: header.block_size,
                    stamp: header.stamp,
                },
            },
        )
        .await;
        // No block of the changed file is sent into the old one: it ends as changed, so the
        // whole file is sent again from the start.
        let answer = hear(&mut new).await;
        drop(new);
        answer
    };
    let (answer, _) = tokio::time::timeout(std::time::Duration::from_secs(30), async {
        tokio::join!(script, sender.run(&mut old))
    })
    .await
    .expect("hung");
    assert_eq!(
        answer,
        Message::EndFile {
            stream: 0,
            stamp_after: Stamp {
                size: header.size,
                modified_ns: header.stamp.modified_ns
            },
            changed: true
        }
    );
}

#[test]
fn the_new_messages_read_back_as_written() {
    let mut done = BlockMap::new(10);
    for b in [0, 1, 2, 5, 9] {
        done.insert(b).unwrap();
    }
    for m in [
        Message::Receipt {
            stream: 3,
            block: 7,
            written: false,
        },
        Message::Receipt {
            stream: 3,
            block: 7,
            written: true,
        },
        Message::GiveUp {
            stream: 9,
            why: GiveUpWhy::ReadTooOften,
        },
        Message::SendAgain {
            stream: 4,
            ticket: ResumeTicket {
                done,
                block_size: 65_536,
                stamp: Stamp {
                    size: 600_000,
                    modified_ns: Some(-5),
                },
            },
        },
    ] {
        assert_eq!(Message::decode(&m.encode()).unwrap(), m);
    }
    // A receipt with a flag that is neither yes nor no is refused.
    let mut bad = Message::Receipt {
        stream: 1,
        block: 1,
        written: true,
    }
    .encode();
    *bad.last_mut().unwrap() = 2;
    assert!(Message::decode(&bad).is_err());
}

#[test]
fn this_release_speaks_the_move_protocol_that_can_send_again() {
    // Sending again arrived in version 2; every later version keeps it.
    assert!(pctwin_transfer::MOVE_PROTOCOL.oldest() >= 2);
    assert!(pctwin_transfer::MOVE_PROTOCOL.newest() >= 2);
}

/// Plays the new laptop against a real old laptop: takes every piece of every block until the
/// end, answering each block with a receipt. Returns the blocks taken and the end.
async fn take_blocks_until_end(new: &mut Mem) -> (u64, Message) {
    let mut blocks = 0;
    loop {
        match hear(new).await {
            Message::Piece {
                stream, last: true, ..
            } => {
                blocks += 1;
                say(
                    new,
                    &Message::Receipt {
                        stream,
                        block: 0,
                        written: true,
                    },
                )
                .await;
            }
            Message::Piece { .. } => {}
            end @ Message::EndFile { .. } => return (blocks, end),
            other => panic!("unexpected {other:?}"),
        }
    }
}

#[tokio::test]
async fn a_new_laptop_that_keeps_asking_again_is_answered_twice_then_the_file_fails() {
    let l = laptops(&[300_000]);
    let header = FileSender::open(&l.files[0].0, None, false)
        .unwrap()
        .header()
        .clone();
    let count = header.block_count;
    let mut sender = SenderSession::new(jobs(&l), 1);
    let (mut old, mut new) = mem_pair();
    let script = async {
        say(&mut new, &Message::Ready).await;
        assert!(matches!(
            hear(&mut new).await,
            Message::StartFile { stream: 0, .. }
        ));
        say(
            &mut new,
            &Message::Have {
                stream: 0,
                same_size: None,
            },
        )
        .await;
        let (first, _) = take_blocks_until_end(&mut new).await;
        assert_eq!(first, count);
        let ask = Message::SendAgain {
            stream: 0,
            ticket: ResumeTicket {
                done: BlockMap::new(count),
                block_size: header.block_size,
                stamp: header.stamp,
            },
        };
        for round in 0..2 {
            say(&mut new, &ask).await;
            let (got, end) = take_blocks_until_end(&mut new).await;
            assert_eq!(got, count, "round {round}");
            assert!(
                matches!(end, Message::EndFile { changed: false, .. }),
                "{end:?}"
            );
        }
        // A third time in this attempt: nothing more is read or sent, and the old laptop says it
        // stops (never that the file changed).
        say(&mut new, &ask).await;
        assert_eq!(
            hear(&mut new).await,
            Message::GiveUp {
                stream: 0,
                why: GiveUpWhy::AskedAgainTooOften
            }
        );
        say(
            &mut new,
            &Message::FileDone {
                stream: 0,
                ok: false,
            },
        )
        .await;
        assert_eq!(hear(&mut new).await, Message::AllSent);
    };
    let ((), r) = tokio::time::timeout(std::time::Duration::from_secs(60), async {
        tokio::join!(script, sender.run(&mut old))
    })
    .await
    .expect("hung");
    r.unwrap();
    assert_eq!(sender.blocks_sent(), 3 * count);
    match sender.outcome(id(0)) {
        Some(SendOutcome::Failed(why)) => assert!(why.contains("kept arriving damaged"), "{why}"),
        other => panic!("{other:?}"),
    }
}

#[tokio::test]
async fn being_asked_again_for_a_file_not_just_ended_changes_nothing() {
    let l = laptops(&[300_000, 300_000]);
    let header = FileSender::open(&l.files[0].0, None, false)
        .unwrap()
        .header()
        .clone();
    let count = header.block_count;
    let empty = ResumeTicket {
        done: BlockMap::new(count),
        block_size: header.block_size,
        stamp: header.stamp,
    };
    let mut sender = SenderSession::new(jobs(&l), 1);
    let (mut old, mut new) = mem_pair();
    let script = async {
        say(&mut new, &Message::Ready).await;
        assert!(matches!(
            hear(&mut new).await,
            Message::StartFile { stream: 0, .. }
        ));
        // Not ended yet: ignored.
        say(
            &mut new,
            &Message::SendAgain {
                stream: 0,
                ticket: empty.clone(),
            },
        )
        .await;
        say(
            &mut new,
            &Message::Have {
                stream: 0,
                same_size: None,
            },
        )
        .await;
        let (got, end) = take_blocks_until_end(&mut new).await;
        assert_eq!(got, count);
        assert!(matches!(
            end,
            Message::EndFile {
                stream: 0,
                changed: false,
                ..
            }
        ));
        say(
            &mut new,
            &Message::FileDone {
                stream: 0,
                ok: true,
            },
        )
        .await;
        // Already done: ignored too.
        assert!(matches!(
            hear(&mut new).await,
            Message::StartFile { stream: 1, .. }
        ));
        say(
            &mut new,
            &Message::SendAgain {
                stream: 0,
                ticket: empty.clone(),
            },
        )
        .await;
        say(
            &mut new,
            &Message::Have {
                stream: 1,
                same_size: None,
            },
        )
        .await;
        let (got, end) = take_blocks_until_end(&mut new).await;
        assert_eq!(got, count);
        assert!(matches!(end, Message::EndFile { stream: 1, .. }));
        say(
            &mut new,
            &Message::FileDone {
                stream: 1,
                ok: true,
            },
        )
        .await;
        assert_eq!(hear(&mut new).await, Message::AllSent);
    };
    let ((), r) = tokio::time::timeout(std::time::Duration::from_secs(60), async {
        tokio::join!(script, sender.run(&mut old))
    })
    .await
    .expect("hung");
    r.unwrap();
    assert_eq!(sender.blocks_sent(), 2 * count);
}

/// Damages the first block of every file once, on the way.
struct DamageFirstBlockOfEach {
    inner: Mem,
    seen: HashSet<u32>,
}

impl Channel for DamageFirstBlockOfEach {
    async fn send(&mut self, data: &[u8]) -> Result<(), ChannelError> {
        if let Ok(Message::Piece {
            stream,
            last: true,
            mut bytes,
        }) = Message::decode(data)
            && self.seen.insert(stream)
        {
            let at = bytes.len() - 1;
            bytes[at] ^= 0x5A;
            let m = Message::Piece {
                stream,
                last: true,
                bytes,
            };
            return self.inner.send(&m.encode()).await;
        }
        self.inner.send(data).await
    }

    async fn recv(&mut self) -> Result<Vec<u8>, ChannelError> {
        self.inner.recv().await
    }
}

#[tokio::test]
async fn many_files_each_asked_again_never_crowd_out_the_rest() {
    // More files than the new laptop keeps open at once, each needing a block again.
    const FILES: usize = 80;
    let l = laptops(&vec![10_000usize; FILES]);
    let table = table(&l, None);
    let mut sender = SenderSession::new(jobs(&l), 1);
    let mut receiver = ReceiverSession::new(&table, plan(&l), common::journal(), "1001");
    let (a, mut b) = mem_pair();
    let mut a = DamageFirstBlockOfEach {
        inner: a,
        seen: Default::default(),
    };
    let (s, r) = tokio::time::timeout(std::time::Duration::from_secs(120), async {
        tokio::join!(sender.run(&mut a), receiver.run(&mut b))
    })
    .await
    .expect("hung");
    s.unwrap();
    r.unwrap();
    for n in 0..FILES {
        assert!(
            matches!(
                receiver.outcome(id(n as u8)),
                Some(ReceiveOutcome::Finished(_))
            ),
            "file {n}: {:?}",
            receiver.outcome(id(n as u8))
        );
        assert_eq!(std::fs::read(landed(&l, n)).unwrap(), l.files[n].1);
    }
}

/// The person, each time a failing drive pauses the move: carries on (here, to see every file's
/// own ending), until `done`. How many times the move paused.
async fn carry_on_whenever_braked(
    control: &pctwin_transfer::MoveControl,
    done: &std::cell::Cell<bool>,
) -> usize {
    let mut brakes = 0;
    while !done.get() {
        if control.braked().is_some() {
            brakes += 1;
            control.carry_on();
        }
        tokio::time::sleep(std::time::Duration::from_millis(2)).await;
    }
    brakes
}

#[tokio::test]
async fn on_a_drive_that_reads_every_file_back_wrong_each_file_fails_for_that_reason() {
    const FILES: usize = 80;
    let l = laptops(&vec![10_000usize; FILES]);
    let table = table(&l, Some(Flip::OnRead { at: 0 }));
    let control = pctwin_transfer::MoveControl::new();
    let mut sender = SenderSession::new(jobs(&l), 1);
    let mut receiver = ReceiverSession::new(&table, plan(&l), common::journal(), "1001")
        .with_control(control.clone());
    let done = std::cell::Cell::new(false);
    let ((s, r), brakes) = {
        let (mut a, mut b) = mem_pair();
        tokio::time::timeout(std::time::Duration::from_secs(120), async {
            tokio::join!(
                async {
                    let ran = tokio::join!(sender.run(&mut a), receiver.run(&mut b));
                    done.set(true);
                    ran
                },
                carry_on_whenever_braked(&control, &done)
            )
        })
        .await
        .expect("hung")
    };
    s.unwrap();
    r.unwrap();
    // The failing drive paused the move again and again (the person carried on each time).
    assert!(brakes >= 1, "the drive never paused the move");
    for n in 0..FILES {
        match receiver.outcome(id(n as u8)) {
            Some(ReceiveOutcome::Failed(why)) => {
                assert!(
                    why.contains("could not store this file correctly"),
                    "file {n}: {why}"
                );
            }
            other => panic!("file {n}: {other:?}"),
        }
    }
}

/// The old laptop's side of an honest move where the person saves the file during a resend: the
/// first block of every attempt is damaged on the way; on the first attempt the file is saved
/// just before the block is sent again; on later attempts the block sent again is damaged once
/// more, so each attempt needs both of its rounds.
struct SavedDuringResend {
    inner: Mem,
    first: Option<u32>,
    sends: BTreeMap<u32, u32>,
    src: PathBuf,
}

impl Channel for SavedDuringResend {
    async fn send(&mut self, data: &[u8]) -> Result<(), ChannelError> {
        match Message::decode(data) {
            Ok(Message::StartFile { stream, .. }) if self.first.is_none() => {
                self.first = Some(stream);
            }
            Ok(Message::Piece {
                stream,
                last: true,
                mut bytes,
            }) => {
                let n = self.sends.entry(stream).or_default();
                *n += 1;
                let first = self.first == Some(stream);
                if first && *n == 4 {
                    std::fs::write(&self.src, pattern(300_001, 1234)).unwrap();
                }
                if *n == 1 || (!first && *n == 4) {
                    let at = bytes.len() - 1;
                    bytes[at] ^= 0x5A;
                    let m = Message::Piece {
                        stream,
                        last: true,
                        bytes,
                    };
                    return self.inner.send(&m.encode()).await;
                }
            }
            _ => {}
        }
        self.inner.send(data).await
    }

    async fn recv(&mut self) -> Result<Vec<u8>, ChannelError> {
        self.inner.recv().await
    }
}

#[tokio::test]
async fn a_file_saved_during_a_resend_starts_again_with_its_own_rounds_and_arrives() {
    let l = laptops(&[300_000]);
    let table = table(&l, None);
    let mut sender = SenderSession::new(jobs(&l), 1);
    let mut receiver = ReceiverSession::new(&table, plan(&l), common::journal(), "1001");
    let (a, mut b) = mem_pair();
    let mut a = SavedDuringResend {
        inner: a,
        first: None,
        sends: Default::default(),
        src: l.files[0].0.clone(),
    };
    let (s, r) = tokio::time::timeout(std::time::Duration::from_secs(60), async {
        tokio::join!(sender.run(&mut a), receiver.run(&mut b))
    })
    .await
    .expect("hung");
    s.unwrap();
    r.unwrap();
    assert_eq!(sender.outcome(id(0)), Some(&SendOutcome::Arrived));
    assert!(matches!(
        receiver.outcome(id(0)),
        Some(ReceiveOutcome::Finished(_))
    ));
    assert_eq!(
        std::fs::read(landed(&l, 0)).unwrap(),
        std::fs::read(&l.files[0].0).unwrap()
    );
}

/// The new laptop's main connection, cut as soon as the old laptop starts its second round.
struct CutAfterSecondRound {
    inner: Option<Mem>,
    asked: u32,
}

impl Channel for CutAfterSecondRound {
    async fn send(&mut self, data: &[u8]) -> Result<(), ChannelError> {
        let Some(inner) = self.inner.as_mut() else {
            return Err(ChannelError);
        };
        if let Ok(Message::SendAgain { .. }) = Message::decode(data) {
            self.asked += 1;
        }
        inner.send(data).await
    }

    async fn recv(&mut self) -> Result<Vec<u8>, ChannelError> {
        let Some(inner) = self.inner.as_mut() else {
            return Err(ChannelError);
        };
        let d = inner.recv().await?;
        if self.asked >= 2 && matches!(Message::decode(&d), Ok(Message::Piece { .. })) {
            self.inner = None;
            return Err(ChannelError);
        }
        Ok(d)
    }
}

#[tokio::test]
async fn after_the_new_laptop_restarts_a_file_out_of_rounds_fails_with_the_true_reason() {
    let l = laptops(&[300_000]);
    let jdir = tempfile::tempdir_in(env!("CARGO_TARGET_TMPDIR")).unwrap();
    let journal = pctwin_journal::Journal::open(&jdir.path().join("j.redb")).unwrap();
    // Every file reads back wrong from this drive.
    let table = table(&l, Some(Flip::OnRead { at: 0 }));
    let mut sender = SenderSession::new(jobs(&l), 1);
    {
        let mut receiver =
            ReceiverSession::new(&table, plan(&l), &journal, "1001").with_scope(SAME_MOVE);
        let (mut a, b) = mem_pair();
        let mut b = CutAfterSecondRound {
            inner: Some(b),
            asked: 0,
        };
        let _ = tokio::time::timeout(std::time::Duration::from_secs(60), async {
            tokio::join!(sender.run(&mut a), receiver.run(&mut b))
        })
        .await
        .expect("hung");
        assert_eq!(b.asked, 2);
        // The app on the new laptop quits here.
    }
    pctwin_transfer::recover(&journal, &table).unwrap();
    let mut receiver =
        ReceiverSession::new(&table, plan(&l), &journal, "1001").with_scope(SAME_MOVE);
    receiver.restore().unwrap();
    let (mut a, mut b) = mem_pair();
    let (s, r) = tokio::time::timeout(std::time::Duration::from_secs(60), async {
        tokio::join!(sender.run(&mut a), receiver.run(&mut b))
    })
    .await
    .expect("hung");
    s.unwrap();
    r.unwrap();
    match receiver.outcome(id(0)) {
        Some(ReceiveOutcome::Failed(why)) => {
            assert!(
                !why.contains("changed"),
                "never said to have changed: {why}"
            );
            assert!(why.contains("Your original is untouched"), "{why}");
        }
        other => panic!("expected the file to fail, got {other:?}"),
    }
    match sender.outcome(id(0)) {
        Some(SendOutcome::Failed(why)) => assert!(why.contains("kept arriving damaged"), "{why}"),
        other => panic!("{other:?}"),
    }
    assert!(!landed(&l, 0).exists());
    // Nothing partly received is left on the drive or open in the journal.
    let left: Vec<_> = std::fs::read_dir(l.new.path().join("Documents"))
        .map(|d| d.map(|e| e.unwrap().file_name()).collect())
        .unwrap_or_default();
    assert!(left.is_empty(), "{left:?}");
    assert!(
        pctwin_journal::Ledger::unfinished(&journal)
            .unwrap()
            .is_empty()
    );
}

fn order(s: &mut Scheduler) -> Vec<ItemId> {
    let mut out = Vec::new();
    while let Some(&x) = s.in_flight().first() {
        out.push(x);
        s.finished(x);
    }
    out
}

#[test]
fn a_file_asked_again_goes_ahead_of_its_own_importance_only() {
    let (x, y, z, w, v, u) = (id(1), id(2), id(3), id(4), id(5), id(6));
    let mut s = Scheduler::new(1);
    s.push(x, Tier::AskedFirst { rank: 0 });
    s.push(y, Tier::AskedFirst { rank: 1 });
    s.push(z, Tier::Rest);
    assert_eq!(s.in_flight(), vec![x]);
    s.push_again(w, Tier::Rest);
    s.push_again(v, Tier::AskedFirst { rank: 5 });
    s.push_again(u, Tier::Essential);
    s.finished(x);
    assert_eq!(order(&mut s), vec![y, v, u, w, z]);
    // One more important than every file in flight goes in at once.
    let mut s = Scheduler::new(1);
    s.push(z, Tier::Rest);
    s.push(w, Tier::Rest);
    assert_eq!(s.in_flight(), vec![z]);
    s.push_again(v, Tier::Essential);
    assert_eq!(s.in_flight()[0], v);
    // A file raised by the person keeps that place: its tier now is what it ends with.
    let mut s = Scheduler::new(1);
    s.push(z, Tier::Rest);
    s.ask_first(&[z]);
    assert!(matches!(s.tier_of(z), Some(Tier::AskedFirst { .. })));
}

#[tokio::test]
async fn a_ticket_that_does_not_describe_the_file_sent_stops_it_and_never_starts_it_again() {
    let l = laptops(&[300_000]);
    let header = FileSender::open(&l.files[0].0, None, false)
        .unwrap()
        .header()
        .clone();
    let count = header.block_count;
    let mut sender = SenderSession::new(jobs(&l), 1);
    let (mut old, mut new) = mem_pair();
    let script = async {
        say(&mut new, &Message::Ready).await;
        assert!(matches!(
            hear(&mut new).await,
            Message::StartFile { stream: 0, .. }
        ));
        say(
            &mut new,
            &Message::Have {
                stream: 0,
                same_size: None,
            },
        )
        .await;
        let (got, _) = take_blocks_until_end(&mut new).await;
        assert_eq!(got, count);
        // A ticket claiming the file is other than the one described: as if it changed.
        let mut forged = header.stamp;
        forged.size += 1;
        say(
            &mut new,
            &Message::SendAgain {
                stream: 0,
                ticket: ResumeTicket {
                    done: BlockMap::new(count),
                    block_size: header.block_size,
                    stamp: forged,
                },
            },
        )
        .await;
        assert_eq!(
            hear(&mut new).await,
            Message::GiveUp {
                stream: 0,
                why: GiveUpWhy::AskedAgainTooOften
            }
        );
        say(
            &mut new,
            &Message::FileDone {
                stream: 0,
                ok: false,
            },
        )
        .await;
        // Never started again.
        assert_eq!(hear(&mut new).await, Message::AllSent);
    };
    let ((), r) = tokio::time::timeout(std::time::Duration::from_secs(60), async {
        tokio::join!(script, sender.run(&mut old))
    })
    .await
    .expect("hung");
    r.unwrap();
    assert_eq!(sender.blocks_sent(), count);
}

/// A new laptop that reconnects with "I have none of it, carry on" each time and answers every
/// block, but never the end: what it is answered is counted for the file's version, so the old
/// laptop stops at the cap, says why, and reads nothing more of it (invariant 3).
#[tokio::test]
async fn reconnecting_with_a_resume_ticket_over_and_over_stops_at_the_answered_cap() {
    let l = laptops(&[300_000]);
    let header = header_of(&l, 0);
    let reads = Reads::default();
    let mut sender = SenderSession::new(jobs(&l), 1).with_opener(Arc::new(reads.clone()));
    let mut stopped_after = None;
    for c in 0..60 {
        let header = header.clone();
        let (got, _) = connection(&mut sender, async move |new: &mut Mem| {
            say(
                new,
                &Message::ResumeFrom {
                    stream: 0,
                    item: id(0),
                    ticket: empty_ticket(&header),
                },
            )
            .await;
            say(new, &Message::Ready).await;
            loop {
                match hear(new).await {
                    Message::StartFile { stream, .. } => {
                        answer_start(new, stream, None).await;
                    }
                    Message::Piece {
                        stream, last: true, ..
                    } => receipt(new, stream, true).await,
                    Message::Piece { .. } => {}
                    Message::GiveUp { why, .. } => return Some(why),
                    // The file's end, never answered: the connection goes.
                    _ => return None,
                }
            }
        })
        .await;
        if got.is_some() {
            assert_eq!(got, Some(GiveUpWhy::ReadTooOften));
            stopped_after = Some(c);
            break;
        }
    }
    assert!(stopped_after.is_some(), "the old laptop never stopped");
    let read = reads.blocks();
    assert!(
        read <= cap_of(header.size) + WINDOW,
        "{read} bytes read, cap {} plus the window",
        cap_of(header.size)
    );
    // Reconnecting again reads nothing more of it: its stop is said again, and answered.
    let (end, ok) = connection(&mut sender, async |new: &mut Mem| {
        say(
            new,
            &Message::ResumeFrom {
                stream: 0,
                item: id(0),
                ticket: empty_ticket(&header),
            },
        )
        .await;
        say(new, &Message::Ready).await;
        let end = hear(new).await;
        if let Message::GiveUp { stream, .. } = end {
            done(new, stream, false).await;
            assert_eq!(hear(new).await, Message::AllSent);
        }
        end
    })
    .await;
    assert!(matches!(end, Message::GiveUp { .. }), "{end:?}");
    assert!(ok);
    assert_eq!(reads.blocks(), read, "nothing more was read");
    match sender.outcome(id(0)) {
        Some(SendOutcome::Failed(why)) => {
            assert!(why.contains("asked for it again many more times"), "{why}");
            assert!(why.contains("Your original is untouched"), "{why}");
        }
        other => panic!("{other:?}"),
    }
}

/// The same new laptop answering nothing: every connection after the first reads at most the
/// floor and one block (invariant 7), however often it reconnects.
#[tokio::test]
async fn reconnecting_without_answering_reads_at_most_the_floor_and_a_block_each_time() {
    let l = laptops(&[8 << 20]);
    let header = header_of(&l, 0);
    let reads = Reads::default();
    let mut sender = SenderSession::new(jobs(&l), 1).with_opener(Arc::new(reads.clone()));
    let mut per = Vec::new();
    for _ in 0..12 {
        let before = reads.blocks();
        let header = header.clone();
        let (blocks, _) = connection(&mut sender, async move |new: &mut Mem| {
            say(
                new,
                &Message::ResumeFrom {
                    stream: 0,
                    item: id(0),
                    ticket: empty_ticket(&header),
                },
            )
            .await;
            say(new, &Message::Ready).await;
            let (stream, _) = start_of(&hear(new).await);
            answer_start(new, stream, None).await;
            unanswered_blocks(new).await
        })
        .await;
        per.push((blocks, reads.blocks() - before));
    }
    // The first connection may fill the whole window (8 MiB here, the whole file).
    assert_eq!(per[0].1, 8 << 20);
    for (c, (blocks, read)) in per.iter().enumerate().skip(1) {
        assert!(
            read <= &(FLOOR + BLOCK),
            "connection {c}: {read} bytes read"
        );
        // Every block read was sent; blocks a drop cut off are sent again from what was kept,
        // without reading the drive again, so more can be sent than read.
        assert!(
            *blocks * BLOCK >= *read,
            "connection {c}: {read} bytes read, {blocks} blocks sent"
        );
    }
    assert_eq!(
        sender.outcome(id(0)),
        None,
        "nothing ends it: the person can stop"
    );
}

#[tokio::test]
async fn an_answer_to_a_start_is_taken_once() {
    let l = laptops(&[300_000]);
    let fingerprint = *blake3::hash(&l.files[0].1).as_bytes();
    let mut sender = SenderSession::new(jobs(&l), 1);
    let (mut old, mut new) = mem_pair();
    let script = async {
        say(&mut new, &Message::Ready).await;
        assert!(matches!(
            hear(&mut new).await,
            Message::StartFile { stream: 0, .. }
        ));
        say(
            &mut new,
            &Message::Have {
                stream: 0,
                same_size: None,
            },
        )
        .await;
        // Once sending, later answers (here, "an identical file is here") are not owed: ignored,
        // so the file is neither skipped nor read again to compare.
        for _ in 0..40 {
            say(
                &mut new,
                &Message::Have {
                    stream: 0,
                    same_size: Some(fingerprint),
                },
            )
            .await;
        }
        let (_, end) = take_blocks_until_end(&mut new).await;
        assert!(
            matches!(end, Message::EndFile { changed: false, .. }),
            "{end:?}"
        );
        say(
            &mut new,
            &Message::FileDone {
                stream: 0,
                ok: true,
            },
        )
        .await;
        assert_eq!(hear(&mut new).await, Message::AllSent);
    };
    let ((), r) = tokio::time::timeout(std::time::Duration::from_secs(60), async {
        tokio::join!(script, sender.run(&mut old))
    })
    .await
    .expect("hung");
    r.unwrap();
    assert_eq!(sender.outcome(id(0)), Some(&SendOutcome::Arrived));
}

/// A connection that goes away just as the old laptop starts a file, while `lose` is set.
struct LoseStart {
    inner: Option<Mem>,
    lose: bool,
}

impl Channel for LoseStart {
    async fn send(&mut self, data: &[u8]) -> Result<(), ChannelError> {
        let Some(inner) = self.inner.as_mut() else {
            return Err(ChannelError);
        };
        if self.lose && matches!(Message::decode(data), Ok(Message::StartFile { .. })) {
            self.inner = None;
            return Err(ChannelError);
        }
        inner.send(data).await
    }

    async fn recv(&mut self) -> Result<Vec<u8>, ChannelError> {
        let Some(inner) = self.inner.as_mut() else {
            return Err(ChannelError);
        };
        inner.recv().await
    }
}

#[tokio::test]
async fn files_whose_start_is_lost_to_drops_again_and_again_still_arrive() {
    let l = laptops(&[300_000usize; 6]);
    let jdir = tempfile::tempdir_in(env!("CARGO_TARGET_TMPDIR")).unwrap();
    let journal = pctwin_journal::Journal::open(&jdir.path().join("j.redb")).unwrap();
    let table = table(&l, None);
    let mut sender = SenderSession::new(jobs(&l), 4);
    let mut receiver = ReceiverSession::new(&table, plan(&l), &journal, "1001");
    for c in 0..=6 {
        let (a, mut b) = mem_pair();
        let mut a = LoseStart {
            inner: Some(a),
            lose: c < 6,
        };
        let (s, _) = tokio::time::timeout(std::time::Duration::from_secs(60), async {
            tokio::join!(sender.run(&mut a), receiver.run(&mut b))
        })
        .await
        .expect("hung");
        if c == 6 {
            s.unwrap();
        }
    }
    for n in 0..6 {
        assert_eq!(
            sender.outcome(id(n as u8)),
            Some(&SendOutcome::Arrived),
            "file {n}"
        );
        assert_eq!(std::fs::read(landed(&l, n)).unwrap(), l.files[n].1);
    }
}

/// Opens files whose stamp moves on every time a read of them ends, as a file being written to
/// all the time does.
#[derive(Default)]
struct AlwaysChanging {
    version: std::sync::Arc<std::sync::atomic::AtomicI64>,
}

struct AlwaysChangingSource {
    inner: Box<dyn pctwin_transfer::Source>,
    version: std::sync::Arc<std::sync::atomic::AtomicI64>,
    seen: bool,
}

impl std::io::Read for AlwaysChangingSource {
    fn read(&mut self, buf: &mut [u8]) -> std::io::Result<usize> {
        self.inner.read(buf)
    }
}

impl std::io::Seek for AlwaysChangingSource {
    fn seek(&mut self, to: std::io::SeekFrom) -> std::io::Result<u64> {
        self.inner.seek(to)
    }
}

impl pctwin_transfer::Source for AlwaysChangingSource {
    fn stamp(&mut self) -> std::io::Result<Stamp> {
        use std::sync::atomic::Ordering;
        let mut s = self.inner.stamp()?;
        let v = if self.seen {
            self.version.fetch_add(1, Ordering::SeqCst) + 1
        } else {
            self.seen = true;
            self.version.load(Ordering::SeqCst)
        };
        s.modified_ns = Some(1_000_000_000 + v);
        Ok(s)
    }
}

impl pctwin_transfer::Opener for AlwaysChanging {
    fn open(&self, path: &std::path::Path) -> std::io::Result<Box<dyn pctwin_transfer::Source>> {
        Ok(Box::new(AlwaysChangingSource {
            inner: pctwin_transfer::FsOpener.open(path)?,
            version: self.version.clone(),
            seen: false,
        }))
    }
}

#[tokio::test]
async fn a_file_that_never_stops_changing_ends_with_the_true_reason() {
    let l = laptops(&[300_000]);
    let table = table(&l, None);
    let mut sender =
        SenderSession::new(jobs(&l), 1).with_opener(std::sync::Arc::new(AlwaysChanging::default()));
    let mut receiver = ReceiverSession::new(&table, plan(&l), common::journal(), "1001");
    let (mut a, mut b) = mem_pair();
    let (s, r) = tokio::time::timeout(std::time::Duration::from_secs(60), async {
        tokio::join!(sender.run(&mut a), receiver.run(&mut b))
    })
    .await
    .expect("hung");
    s.unwrap();
    r.unwrap();
    match receiver.outcome(id(0)) {
        Some(ReceiveOutcome::Failed(why)) => assert!(why.contains("kept changing"), "{why}"),
        other => panic!("{other:?}"),
    }
    assert!(!landed(&l, 0).exists());
    // Sent whole at most as often as the new laptop allows attempts.
    let count = (300_000u64).div_ceil(block_size_for(300_000));
    assert!(sender.blocks_sent() <= u64::from(pctwin_transfer::MAX_ATTEMPTS) * count);
}

// =============================================================================================
// The old laptop reads only as much as the move needs (Security Design; quarantine design,
// "Bounding the old laptop's reads"). Every read the new laptop can cause is paid for by progress
// the old laptop recorded itself, plus a small constant per connection:
// - after a loss its window starts again from a floor and grows only by blocks receipted written;
// - what the new laptop is answered for a file, per version, is capped;
// - a file's fingerprint is worked out once per version, through the opener;
// - the versions it reads are bounded by what it saw itself.
// =============================================================================================

const MIB: u64 = 1 << 20;
/// The old laptop's window while nothing is lost (as on main).
const WINDOW: u64 = 32 * MIB;
/// The block of every file in these tests (all under 250 MiB).
const BLOCK: u64 = 128 * 1024;
/// The window after a loss, for files of such blocks.
const FLOOR: u64 = MIB;
/// How many more times than once a file may be answered, per version (REREAD_TIMES).
const REREADS: u64 = 5;

/// What the new laptop may be answered for a file of `size` bytes, per version.
fn cap_of(size: u64) -> u64 {
    (1 + REREADS) * size.max(MIB)
}

fn header_of(l: &Laptops, n: usize) -> Header {
    FileSender::open(&l.files[n].0, None, false)
        .unwrap()
        .header()
        .clone()
}

fn empty_ticket(h: &Header) -> ResumeTicket {
    ResumeTicket {
        done: BlockMap::new(h.block_count),
        block_size: h.block_size,
        stamp: h.stamp,
    }
}

fn start_of(m: &Message) -> (u32, Header) {
    match m {
        Message::StartFile { stream, header, .. } => (*stream, header.clone()),
        other => panic!("expected a start, got {other:?}"),
    }
}

async fn answer_start(new: &mut Mem, stream: u32, same_size: Option<[u8; 32]>) {
    say(new, &Message::Have { stream, same_size }).await;
}

async fn receipt(new: &mut Mem, stream: u32, written: bool) {
    say(
        new,
        &Message::Receipt {
            stream,
            block: 0,
            written,
        },
    )
    .await;
}

async fn done(new: &mut Mem, stream: u32, ok: bool) {
    say(new, &Message::FileDone { stream, ok }).await;
}

/// Takes blocks, receipting each as written, until the old laptop says something else: how many
/// blocks, and what it said.
async fn blocks_then(new: &mut Mem) -> (u64, Message) {
    let mut blocks = 0;
    loop {
        match hear(new).await {
            Message::Piece {
                stream, last: true, ..
            } => {
                blocks += 1;
                receipt(new, stream, true).await;
            }
            Message::Piece { .. } => {}
            other => return (blocks, other),
        }
    }
}

/// Takes exactly `n` blocks without answering them.
async fn take_blocks(new: &mut Mem, n: u64) {
    let mut got = 0;
    while got < n {
        if let Message::Piece { last: true, .. } = hear(new).await {
            got += 1;
        }
    }
}

/// Takes blocks without answering any, until nothing more comes: how many.
async fn unanswered_blocks(new: &mut Mem) -> u64 {
    let mut blocks = 0;
    while let Ok(Ok(d)) = tokio::time::timeout(Duration::from_millis(300), new.recv()).await {
        if let Ok(Message::Piece { last: true, .. }) = Message::decode(&d) {
            blocks += 1;
        }
    }
    blocks
}

/// One connection to `sender`, with the new laptop played by `script`: what the script returns,
/// and whether the old laptop's run ended well. The connection goes when the script ends.
async fn connection<T>(
    sender: &mut SenderSession,
    script: impl AsyncFnOnce(&mut Mem) -> T,
) -> (T, bool) {
    let (mut old, new) = mem_pair();
    let new_laptop = async move {
        let mut new = new;
        script(&mut new).await
    };
    let (t, r) = tokio::time::timeout(Duration::from_secs(120), async {
        tokio::join!(new_laptop, sender.run(&mut old))
    })
    .await
    .expect("the move hung");
    (t, r.is_ok())
}

// ---------------------------------------------------------------- reading, counted

/// A file that changes as it is read (a log being written to): each open sees the version now,
/// and asking for its stamp again (as the end of a read does) finds it one version on, until it
/// settles after `settles_after` changes (never, if `None`).
#[derive(Default)]
struct Changes {
    version: AtomicI64,
    settles_after: Option<i64>,
    /// The versions whose bytes were read.
    read: Mutex<BTreeSet<i64>>,
}

/// Opens files from the disk, counting every open and every byte read. Blocks are always read
/// from their own place (after a seek); a fingerprint is read from the start without one.
#[derive(Clone, Default)]
struct Reads {
    opens: Arc<AtomicU64>,
    blocks: Arc<AtomicU64>,
    prints: Arc<AtomicU64>,
    changes: Option<Arc<Changes>>,
    /// Every source says it is this, whatever the file on disk is now (a file that grew while
    /// its size and time did not show it).
    claims: Option<Stamp>,
    /// Sources open now, and the most open at once.
    live: Arc<AtomicI64>,
    most_live: Arc<AtomicI64>,
}

impl Reads {
    fn changing(settles_after: Option<i64>) -> Self {
        Self {
            changes: Some(Arc::new(Changes {
                settles_after,
                ..Default::default()
            })),
            ..Default::default()
        }
    }

    fn opens(&self) -> u64 {
        self.opens.load(Ordering::SeqCst)
    }

    fn blocks(&self) -> u64 {
        self.blocks.load(Ordering::SeqCst)
    }

    fn prints(&self) -> u64 {
        self.prints.load(Ordering::SeqCst)
    }

    fn most_live(&self) -> i64 {
        self.most_live.load(Ordering::SeqCst)
    }

    fn versions_read(&self) -> usize {
        self.changes
            .as_ref()
            .map_or(0, |c| c.read.lock().unwrap().len())
    }
}

struct Counted {
    inner: Box<dyn Source>,
    reads: Reads,
    seeked: bool,
    /// The version it was opened at (for a changing file).
    opened_at: Option<i64>,
}

impl Read for Counted {
    fn read(&mut self, buf: &mut [u8]) -> std::io::Result<usize> {
        let n = self.inner.read(buf)?;
        let counter = if self.seeked {
            &self.reads.blocks
        } else {
            &self.reads.prints
        };
        counter.fetch_add(n as u64, Ordering::SeqCst);
        if n > 0
            && let (Some(c), Some(v)) = (&self.reads.changes, self.opened_at)
        {
            c.read.lock().unwrap().insert(v);
        }
        Ok(n)
    }
}

impl Seek for Counted {
    fn seek(&mut self, to: SeekFrom) -> std::io::Result<u64> {
        self.seeked = true;
        self.inner.seek(to)
    }
}

impl Drop for Counted {
    fn drop(&mut self) {
        self.reads.live.fetch_sub(1, Ordering::SeqCst);
    }
}

impl Source for Counted {
    fn stamp(&mut self) -> std::io::Result<Stamp> {
        let mut s = match self.reads.claims {
            Some(s) => s,
            None => self.inner.stamp()?,
        };
        if let Some(c) = &self.reads.changes {
            let v = match self.opened_at {
                None => {
                    let v = c.version.load(Ordering::SeqCst);
                    self.opened_at = Some(v);
                    v
                }
                Some(_) => {
                    let now = c.version.load(Ordering::SeqCst);
                    if c.settles_after.is_none_or(|at| now < at) {
                        c.version.fetch_add(1, Ordering::SeqCst) + 1
                    } else {
                        now
                    }
                }
            };
            s.modified_ns = Some(1_000_000_000 + v);
        }
        Ok(s)
    }
}

impl Opener for Reads {
    fn open(&self, path: &Path) -> std::io::Result<Box<dyn Source>> {
        self.opens.fetch_add(1, Ordering::SeqCst);
        let inner = FsOpener.open(path)?;
        let now = self.live.fetch_add(1, Ordering::SeqCst) + 1;
        self.most_live.fetch_max(now, Ordering::SeqCst);
        Ok(Box::new(Counted {
            inner,
            reads: self.clone(),
            seeked: false,
            opened_at: None,
        }))
    }
}

// ---------------------------------------------------------------- a flaky link, real laptops

/// How the old laptop's end of one connection (or lane) breaks. Blocks are counted over the
/// whole connection, every lane together.
#[derive(Clone, Copy, Debug)]
enum Cut {
    /// It never breaks.
    Clean,
    /// After `k` blocks everything sent is lost, and it drops once nothing more comes back.
    LoseAfter(u32),
    /// After `k` blocks it closes: what was sent before arrives.
    CloseAfter(u32),
    /// After `k` receipts on it, receipts are lost on the way, and it drops once nothing more
    /// comes back.
    ReceiptsLostAfter(u32),
    /// The old laptop's skip is lost, and it drops.
    LoseSkip,
}

/// What the old laptop's ends of the connections saw: its own view of what was answered.
#[derive(Default)]
struct OldView {
    /// Each stream's description, from its start.
    headers: HashMap<u32, Header>,
    /// Raw bytes of blocks answered (receipted either way), and of those, receipted written.
    answered: u64,
    written: u64,
    /// Connections and lanes that dropped.
    losses: u64,
}

/// The raw length of block `block` of the file `h` describes (0 for a block outside it).
fn raw_len(h: &Header, block: u64) -> u64 {
    let start = block.saturating_mul(h.block_size);
    if block >= h.block_count {
        return 0;
    }
    h.block_size.min(h.size - start)
}

struct OldEnd {
    inner: Option<Mem>,
    cut: Cut,
    blocks: Arc<AtomicU32>,
    receipts: u32,
    lost: bool,
    view: Arc<Mutex<OldView>>,
}

impl OldEnd {
    fn new(inner: Mem, cut: Cut, blocks: Arc<AtomicU32>, view: Arc<Mutex<OldView>>) -> Self {
        Self {
            inner: Some(inner),
            cut,
            blocks,
            receipts: 0,
            lost: false,
            view,
        }
    }

    fn cut_off(&mut self) -> ChannelError {
        if self.inner.take().is_some() {
            self.view.lock().unwrap().losses += 1;
        }
        ChannelError
    }
}

impl Channel for OldEnd {
    async fn send(&mut self, data: &[u8]) -> Result<(), ChannelError> {
        if self.inner.is_none() {
            return Err(ChannelError);
        }
        let m = Message::decode(data).ok();
        if let Some(Message::StartFile { stream, header, .. }) = &m {
            self.view
                .lock()
                .unwrap()
                .headers
                .insert(*stream, header.clone());
        }
        let piece = matches!(m, Some(Message::Piece { .. }));
        let last = matches!(m, Some(Message::Piece { last: true, .. }));
        if self.lost {
            return Ok(());
        }
        let sent = self.blocks.load(Ordering::SeqCst);
        match self.cut {
            Cut::LoseAfter(k) if piece && sent >= k => {
                self.lost = true;
                return Ok(());
            }
            Cut::CloseAfter(k) if piece && sent >= k => return Err(self.cut_off()),
            Cut::LoseSkip if matches!(m, Some(Message::Skip { .. })) => {
                return Err(self.cut_off());
            }
            _ => {}
        }
        let inner = self.inner.as_mut().ok_or(ChannelError)?;
        inner.send(data).await?;
        if last {
            self.blocks.fetch_add(1, Ordering::SeqCst);
        }
        Ok(())
    }

    async fn recv(&mut self) -> Result<Vec<u8>, ChannelError> {
        loop {
            let quiet =
                self.lost || matches!(self.cut, Cut::ReceiptsLostAfter(k) if self.receipts >= k);
            let inner = self.inner.as_mut().ok_or(ChannelError)?;
            let got = if quiet {
                match tokio::time::timeout(Duration::from_millis(40), inner.recv()).await {
                    Ok(got) => got,
                    Err(_) => return Err(self.cut_off()),
                }
            } else {
                inner.recv().await
            };
            let data = got?;
            if let Ok(Message::Receipt {
                stream,
                block,
                written,
            }) = Message::decode(&data)
            {
                if matches!(self.cut, Cut::ReceiptsLostAfter(k) if self.receipts >= k) {
                    // Lost on the way.
                    continue;
                }
                self.receipts += 1;
                let mut v = self.view.lock().unwrap();
                let raw = v.headers.get(&stream).map_or(0, |h| raw_len(h, block));
                v.answered += raw;
                if written {
                    v.written += raw;
                }
            }
            return Ok(data);
        }
    }
}

/// What is already on the new laptop where each file lands.
#[derive(Clone, Copy, Debug, PartialEq)]
enum Prefill {
    None,
    /// A file of the same size with other contents.
    Different,
    /// An identical file.
    Identical,
}

struct Flaky {
    sizes: Vec<u64>,
    /// Files of zeros (else a pattern).
    zeros: bool,
    compressible: bool,
    /// Extra lanes per connection.
    lanes: usize,
    in_flight: Option<u64>,
    plan: ReadPlan,
    prefill: Prefill,
    /// How each connection breaks, main and lanes: these first, then `then` for every later one.
    cuts: Vec<(Cut, Cut)>,
    then: (Cut, Cut),
    max_conns: usize,
}

impl Flaky {
    fn new(sizes: &[u64], then: (Cut, Cut)) -> Self {
        Self {
            sizes: sizes.to_vec(),
            zeros: false,
            compressible: false,
            lanes: 0,
            in_flight: None,
            plan: ReadPlan::Normal,
            prefill: Prefill::None,
            cuts: Vec::new(),
            then,
            max_conns: 5000,
        }
    }
}

/// One connection's reads and what was answered on it, as the old laptop saw them.
#[derive(Debug, Default, Clone, Copy)]
struct Conn {
    blocks: u64,
    answered: u64,
    written: u64,
    losses: u64,
}

struct Moved {
    conns: Vec<Conn>,
    reads: Reads,
    sent: Vec<Option<SendOutcome>>,
    got: Vec<Option<ReceiveOutcome>>,
    /// Per file: the bytes now on the new laptop for it (its copy, or the file already there).
    landed: Vec<Option<Vec<u8>>>,
    files: Vec<Vec<u8>>,
}

impl Moved {
    fn losses(&self) -> u64 {
        self.conns.iter().map(|c| c.losses).sum()
    }

    /// Invariant 6: every file arrived whole (or was found already there, identical).
    fn assert_arrived(&self, identical: bool) {
        for n in 0..self.files.len() {
            self.assert_arrived_one(n, identical);
        }
    }

    /// The one way a file that did not change may end otherwise today: the new laptop refused a
    /// start it counted as a fresh attempt (its continuation is its own change).
    fn assert_refused_as_new_attempts(&self, n: usize) {
        assert_eq!(
            self.sent[n],
            Some(SendOutcome::Failed(Failure::new(
                Cause::NotAccepted,
                "the new laptop did not accept it"
            ))),
            "file {n}"
        );
        match &self.got[n] {
            Some(ReceiveOutcome::Failed(why)) => assert!(why.contains("kept changing"), "{why}"),
            other => panic!("file {n}: {other:?}"),
        }
    }

    fn assert_arrived_one(&self, n: usize, identical: bool) {
        let file = &self.files[n];
        {
            if identical {
                assert_eq!(self.sent[n], Some(SendOutcome::AlreadyThere), "file {n}");
                assert!(
                    matches!(self.got[n], Some(ReceiveOutcome::AlreadyThere(_))),
                    "file {n}: {:?}",
                    self.got[n]
                );
            } else {
                assert_eq!(self.sent[n], Some(SendOutcome::Arrived), "file {n}");
                assert!(
                    matches!(self.got[n], Some(ReceiveOutcome::Finished(_))),
                    "file {n}: {:?}",
                    self.got[n]
                );
            }
            assert!(
                self.landed[n].as_ref() == Some(file),
                "file {n}: landed bytes differ"
            );
        }
    }

    /// Invariant 1: what is read and not answered at a loss is at most the floor and one block,
    /// plus what was receipted written since the loss before (the first connection starts with
    /// the whole `window`). A connection that ends well leaves nothing unanswered.
    fn assert_lost_reads_bounded(&self, floor: u64, block: u64, window: u64) {
        for (c, conn) in self.conns.iter().enumerate() {
            let lost = conn.blocks.saturating_sub(conn.answered);
            if conn.losses == 0 {
                assert_eq!(lost, 0, "connection {c} ended well: {conn:?}");
                continue;
            }
            let first = if c == 0 { window } else { floor };
            let allowed = first + block + (conn.losses - 1) * (floor + block) + conn.written;
            assert!(
                lost <= allowed,
                "connection {c}: {lost} bytes read and lost, allowed {allowed}: {conn:?}"
            );
        }
    }

    /// Invariant 7: a connection after the first that made no progress read at most the floor
    /// and one block at each of its losses (one, without extra lanes).
    fn assert_no_progress_bounded(&self, floor: u64, block: u64) {
        for (c, conn) in self.conns.iter().enumerate().skip(1) {
            if conn.written == 0 && conn.losses > 0 {
                assert!(
                    conn.blocks <= conn.losses * (floor + block),
                    "connection {c} made no progress yet read {}: {conn:?}",
                    conn.blocks
                );
            }
        }
    }

    /// Invariant 8: all read is at most, per file version, six times the file (as if at least
    /// 1 MiB: once, four more answered, one fingerprint), plus the floor and a block per loss,
    /// plus one window.
    fn assert_reads_bounded(&self, floor: u64, block: u64, window: u64) {
        let read = self.reads.blocks() + self.reads.prints();
        let per_version: u64 = self
            .files
            .iter()
            .map(|f| (2 + REREADS) * (f.len() as u64).max(MIB))
            .sum();
        let allowed = per_version + self.losses() * (floor + block) + window;
        assert!(
            read <= allowed,
            "{read} bytes read, allowed {allowed} ({} losses)",
            self.losses()
        );
    }
}

/// Moves `f.sizes` over a link that breaks as `f` says, until the old laptop's run ends well.
async fn flaky(f: Flaky) -> Moved {
    let old = tempfile::tempdir_in(env!("CARGO_TARGET_TMPDIR")).unwrap();
    let new = tempfile::tempdir_in(env!("CARGO_TARGET_TMPDIR")).unwrap();
    let mut files = Vec::new();
    for (i, size) in f.sizes.iter().enumerate() {
        let len = usize::try_from(*size).unwrap();
        let bytes = if f.zeros {
            vec![0u8; len]
        } else {
            pattern(len, i as u32 + 7)
        };
        std::fs::write(old.path().join(format!("f{i}.bin")), &bytes).unwrap();
        if f.prefill != Prefill::None {
            let p = new.path().join(format!("Documents/f{i}.bin"));
            std::fs::create_dir_all(p.parent().unwrap()).unwrap();
            let there = match f.prefill {
                Prefill::Identical => bytes.clone(),
                _ => pattern(len, 999 + i as u32),
            };
            std::fs::write(&p, there).unwrap();
        }
        files.push(bytes);
    }
    let jobs: Vec<SendJob> = (0..files.len())
        .map(|n| SendJob {
            size: 0,
            item: id(n as u8),
            source: old.path().join(format!("f{n}.bin")),
            destination: "me".into(),
            path: format!("Documents/f{n}.bin"),
            compressible: f.compressible,
            tier: Tier::Rest,
        })
        .collect();
    let sized: Vec<(&SendJob, u64)> = jobs
        .iter()
        .enumerate()
        .map(|(n, j)| (j, files[n].len() as u64))
        .collect();
    let allowance = common::approved_jobs(&sized);
    let mut table = Destinations::new();
    table
        .approve("me", Approved::MyFolders, new.path())
        .unwrap();
    let reads = Reads::default();
    let mut sender = SenderSession::new(jobs.clone(), 4)
        .with_opener(Arc::new(reads.clone()))
        .with_read_plan(f.plan);
    if let Some(limit) = f.in_flight {
        sender = sender.with_in_flight_limit(limit);
    }
    let mut receiver = ReceiverSession::new(&table, allowance, common::journal(), "1001");
    let view = Arc::new(Mutex::new(OldView::default()));
    let mut conns = Vec::new();
    for c in 0.. {
        assert!(c < f.max_conns, "the move never finished");
        let (main_cut, lane_cut) = f.cuts.get(c).copied().unwrap_or(f.then);
        let snapshot = |v: &OldView| (v.answered, v.written, v.losses);
        let (a0, w0, l0) = snapshot(&view.lock().unwrap());
        let b0 = reads.blocks();
        let blocks = Arc::new(AtomicU32::new(0));
        let (a, mut b) = mem_pair();
        let mut a = OldEnd::new(a, main_cut, blocks.clone(), view.clone());
        let mut old_lanes = Vec::new();
        let mut new_lanes = Vec::new();
        for _ in 0..f.lanes {
            let (x, y) = mem_pair();
            old_lanes.push(OldEnd::new(x, lane_cut, blocks.clone(), view.clone()));
            new_lanes.push(y);
        }
        let (s, _) = tokio::time::timeout(Duration::from_secs(300), async {
            tokio::join!(
                sender.run_lanes(&mut a, old_lanes),
                receiver.run_lanes(&mut b, new_lanes)
            )
        })
        .await
        .expect("a connection hung");
        drop(a);
        let (a1, w1, l1) = snapshot(&view.lock().unwrap());
        conns.push(Conn {
            blocks: reads.blocks() - b0,
            answered: a1 - a0,
            written: w1 - w0,
            losses: l1 - l0,
        });
        if s.is_ok() {
            break;
        }
    }
    let landed = (0..files.len())
        .map(|n| match receiver.outcome(id(n as u8)) {
            Some(ReceiveOutcome::Finished(done)) => {
                std::fs::read(new.path().join(&done.final_path)).ok()
            }
            Some(ReceiveOutcome::AlreadyThere(stored)) => {
                std::fs::read(new.path().join(stored)).ok()
            }
            _ => None,
        })
        .collect();
    Moved {
        conns,
        sent: (0..files.len())
            .map(|n| sender.outcome(id(n as u8)).cloned())
            .collect(),
        got: (0..files.len())
            .map(|n| receiver.outcome(id(n as u8)))
            .collect(),
        landed,
        reads,
        files,
    }
}

// ---------------------------------------------------------------- the earned window (A)

/// A clean first connection keeps the whole window (as on main); after a loss it starts from the
/// floor and grows by exactly what is receipted written, never by a receipt "not written".
#[tokio::test]
async fn the_window_is_whole_until_a_loss_then_earned_from_the_floor() {
    let size = 40 << 20;
    let old = tempfile::tempdir_in(env!("CARGO_TARGET_TMPDIR")).unwrap();
    let src = old.path().join("f0.bin");
    std::fs::write(&src, vec![0u8; size]).unwrap();
    let job = SendJob {
        size: 0,
        item: id(0),
        source: src,
        destination: "me".into(),
        path: "Documents/f0.bin".into(),
        compressible: false,
        tier: Tier::Rest,
    };
    let mut sender = SenderSession::new(vec![job], 1);
    let fresh = async |new: &mut Mem| {
        say(new, &Message::Ready).await;
        let (stream, _) = start_of(&hear(new).await);
        answer_start(new, stream, None).await;
        stream
    };
    // Nothing lost yet: the whole window goes (32 MiB of 128 KiB blocks).
    let (sent, _) = connection(&mut sender, async |new: &mut Mem| {
        fresh(new).await;
        unanswered_blocks(new).await
    })
    .await;
    assert_eq!(sent * BLOCK, WINDOW, "{sent} blocks before the first loss");
    // After the loss: the floor.
    let (sent, _) = connection(&mut sender, async |new: &mut Mem| {
        fresh(new).await;
        unanswered_blocks(new).await
    })
    .await;
    assert_eq!(sent * BLOCK, FLOOR, "{sent} blocks after a loss");
    // Each block receipted written lets one more go in its place and one more besides.
    let (sent, _) = connection(&mut sender, async |new: &mut Mem| {
        let stream = fresh(new).await;
        take_blocks(new, FLOOR / BLOCK).await;
        for _ in 0..4 {
            receipt(new, stream, true).await;
            take_blocks(new, 2).await;
        }
        FLOOR / BLOCK + 8 + unanswered_blocks(new).await
    })
    .await;
    assert_eq!(sent, FLOOR / BLOCK + 8);
    // A receipt "not written" frees its place only.
    let (sent, _) = connection(&mut sender, async |new: &mut Mem| {
        let stream = fresh(new).await;
        take_blocks(new, FLOOR / BLOCK).await;
        receipt(new, stream, false).await;
        FLOOR / BLOCK + unanswered_blocks(new).await
    })
    .await;
    assert_eq!(sent, FLOOR / BLOCK + 1);
}

/// A lane that drops is a loss too: the window starts again from the floor. (Files under
/// 64 MiB are never split between lanes, so several files keep every lane busy.)
#[tokio::test]
async fn a_lane_that_drops_starts_the_window_again_from_the_floor() {
    let mut f = Flaky::new(&[4 << 20; 6], (Cut::Clean, Cut::LoseAfter(2)));
    f.lanes = 3;
    let m = flaky(f).await;
    m.assert_arrived(false);
    assert_eq!(m.conns.len(), 1, "the main connection carried the rest");
    assert_eq!(m.conns[0].losses, 3, "every lane dropped: {:?}", m.conns[0]);
    m.assert_lost_reads_bounded(FLOOR, BLOCK, WINDOW);
    m.assert_reads_bounded(FLOOR, BLOCK, WINDOW);
}

/// A lane lost while the main connection carries on is a loss at once: from then on only the
/// floor is in flight, not the whole window. (A file of 72 MiB, so the lane takes its back half;
/// the lane's far end is already gone, so its first block cannot be sent.)
#[tokio::test]
async fn a_lane_lost_mid_connection_drops_the_window_to_the_floor() {
    let old = tempfile::tempdir_in(env!("CARGO_TARGET_TMPDIR")).unwrap();
    let src = old.path().join("f0.bin");
    std::fs::write(&src, vec![0u8; 72 << 20]).unwrap();
    let job = SendJob {
        size: 0,
        item: id(0),
        source: src,
        destination: "me".into(),
        path: "Documents/f0.bin".into(),
        compressible: false,
        tier: Tier::Rest,
    };
    let mut sender = SenderSession::new(vec![job], 1);
    let (mut main, mut new) = mem_pair();
    let (lane, gone) = mem_pair();
    drop(gone);
    let script = async {
        say(&mut new, &Message::Ready).await;
        let (stream, _) = start_of(&hear(&mut new).await);
        answer_start(&mut new, stream, None).await;
        let sent = unanswered_blocks(&mut new).await;
        drop(new);
        sent
    };
    let (sent, _) = tokio::time::timeout(Duration::from_secs(60), async {
        tokio::join!(script, sender.run_lanes(&mut main, vec![lane]))
    })
    .await
    .expect("hung");
    // Without the loss it would be the whole window (256 blocks); a few blocks may have gone on
    // the main connection before the lane's first one failed.
    assert!(
        sent * BLOCK <= FLOOR + 16 * BLOCK,
        "{sent} blocks went on the main connection after the lane was lost"
    );
}

// ---------------------------------------------------------------- round 1

/// A new laptop that asks for the file again with a ticket claiming it changed (it did not):
/// the old laptop stops after the first read, never sending it whole again.
async fn forged_stamp_loop(size: usize, loops: u64) {
    let l = laptops(&[size]);
    let reads = Reads::default();
    let mut sender = SenderSession::new(jobs(&l), 1).with_opener(Arc::new(reads.clone()));
    let (stopped, _) = connection(&mut sender, async |new: &mut Mem| {
        say(new, &Message::Ready).await;
        for _ in 0..loops {
            let start = hear(new).await;
            let Message::StartFile { stream, header, .. } = start else {
                return start;
            };
            answer_start(new, stream, None).await;
            let (_, end) = blocks_then(new).await;
            assert!(
                matches!(end, Message::EndFile { changed: false, .. }),
                "{end:?}"
            );
            let mut forged = empty_ticket(&header);
            forged.stamp.size += 1;
            say(
                new,
                &Message::SendAgain {
                    stream,
                    ticket: forged,
                },
            )
            .await;
            let reply = hear(new).await;
            done(new, stream, false).await;
            if !matches!(reply, Message::EndFile { .. }) {
                assert_eq!(hear(new).await, Message::AllSent);
                return reply;
            }
        }
        Message::AllSent
    })
    .await;
    assert!(
        matches!(
            stopped,
            Message::GiveUp {
                why: GiveUpWhy::AskedAgainTooOften,
                ..
            }
        ),
        "{stopped:?}"
    );
    assert_eq!(reads.blocks(), size as u64, "read once, never again");
}

#[tokio::test]
async fn a_forged_stamp_on_send_again_stops_the_file_after_one_read() {
    forged_stamp_loop(300_000, 25).await;
}

#[tokio::test]
async fn a_long_forged_stamp_loop_stops_the_file_after_one_read() {
    forged_stamp_loop(2 << 20, 400).await;
}

#[tokio::test]
async fn send_again_after_the_old_laptop_gave_up_reads_nothing() {
    let l = laptops(&[300_000]);
    let header = header_of(&l, 0);
    let reads = Reads::default();
    let mut sender = SenderSession::new(jobs(&l), 1).with_opener(Arc::new(reads.clone()));
    let ((), ok) = connection(&mut sender, async |new: &mut Mem| {
        say(new, &Message::Ready).await;
        let (stream, _) = start_of(&hear(new).await);
        answer_start(new, stream, None).await;
        let _ = blocks_then(new).await;
        let ask = Message::SendAgain {
            stream,
            ticket: empty_ticket(&header),
        };
        for _ in 0..2 {
            say(new, &ask).await;
            let (got, _) = blocks_then(new).await;
            assert_eq!(got, header.block_count);
        }
        say(new, &ask).await;
        assert_eq!(
            hear(new).await,
            Message::GiveUp {
                stream,
                why: GiveUpWhy::AskedAgainTooOften
            }
        );
        // Again and again, before and after answering.
        for _ in 0..5 {
            say(new, &ask).await;
        }
        done(new, stream, false).await;
        for _ in 0..5 {
            say(new, &ask).await;
        }
        done(new, stream, true).await;
        assert_eq!(hear(new).await, Message::AllSent);
    })
    .await;
    assert!(ok);
    assert_eq!(reads.blocks(), 3 * header.size);
    assert!(
        matches!(sender.outcome(id(0)), Some(SendOutcome::Failed(w)) if w.contains("untouched"))
    );
}

#[tokio::test]
async fn a_file_answered_not_accepted_never_starts_a_new_attempt() {
    for mid_round in [false, true] {
        let l = laptops(&[300_000]);
        let header = header_of(&l, 0);
        let reads = Reads::default();
        let mut sender = SenderSession::new(jobs(&l), 1).with_opener(Arc::new(reads.clone()));
        let ((), ok) = connection(&mut sender, async |new: &mut Mem| {
            say(new, &Message::Ready).await;
            let (stream, _) = start_of(&hear(new).await);
            answer_start(new, stream, None).await;
            let _ = blocks_then(new).await;
            if mid_round {
                say(
                    new,
                    &Message::SendAgain {
                        stream,
                        ticket: empty_ticket(&header),
                    },
                )
                .await;
                take_blocks(new, 1).await;
                done(new, stream, false).await;
                receipt(new, stream, true).await;
            } else {
                done(new, stream, false).await;
            }
            loop {
                match hear(new).await {
                    Message::Piece {
                        stream, last: true, ..
                    } => receipt(new, stream, true).await,
                    Message::Piece { .. } => {}
                    Message::StartFile { .. } => panic!("a new attempt started"),
                    Message::EndFile { stream, .. } | Message::GiveUp { stream, .. } => {
                        done(new, stream, false).await;
                    }
                    Message::AllSent => break,
                    other => panic!("unexpected {other:?}"),
                }
            }
        })
        .await;
        assert!(ok);
        assert!(reads.blocks() <= 2 * header.size, "{}", reads.blocks());
        assert_eq!(
            sender.outcome(id(0)),
            Some(&SendOutcome::Failed(Failure::new(
                Cause::NotAccepted,
                "the new laptop did not accept it"
            )))
        );
    }
}

// ---------------------------------------------------------------- round 2: crowding

fn tier_of(n: usize, mixed: bool) -> Tier {
    if !mixed {
        return Tier::Rest;
    }
    match n % 4 {
        0 => Tier::AskedFirst {
            rank: (n / 4) as u32,
        },
        1 => Tier::Personal,
        2 => Tier::Essential,
        _ => Tier::Rest,
    }
}

/// The jobs in planned order: most important first (stable within a tier).
fn jobs_with(l: &Laptops, mixed: bool) -> Vec<SendJob> {
    let mut v: Vec<(Tier, SendJob)> = jobs(l)
        .into_iter()
        .enumerate()
        .map(|(n, mut j)| {
            j.tier = tier_of(n, mixed);
            (j.tier, j)
        })
        .collect();
    v.sort_by_key(|(t, _)| *t);
    v.into_iter().map(|(_, j)| j).collect()
}

/// Damages the first block of each stream once (whichever lane carries it).
struct DamageOnce {
    inner: Mem,
    seen: Arc<Mutex<HashSet<u32>>>,
    damage: bool,
}

impl Channel for DamageOnce {
    async fn send(&mut self, data: &[u8]) -> Result<(), ChannelError> {
        if self.damage
            && let Ok(Message::Piece {
                stream,
                last: true,
                mut bytes,
            }) = Message::decode(data)
            && self.seen.lock().unwrap().insert(stream)
        {
            let at = bytes.len() - 1;
            bytes[at] ^= 0x5A;
            let m = Message::Piece {
                stream,
                last: true,
                bytes,
            };
            return self.inner.send(&m.encode()).await;
        }
        self.inner.send(data).await
    }

    async fn recv(&mut self) -> Result<Vec<u8>, ChannelError> {
        self.inner.recv().await
    }
}

/// The new laptop's main connection, counting the files open on it at once.
struct CountOpen {
    inner: Mem,
    open: HashSet<u32>,
    most: Arc<AtomicUsize>,
}

impl Channel for CountOpen {
    async fn send(&mut self, data: &[u8]) -> Result<(), ChannelError> {
        if let Ok(Message::FileDone { stream, .. }) = Message::decode(data) {
            self.open.remove(&stream);
        }
        self.inner.send(data).await
    }

    async fn recv(&mut self) -> Result<Vec<u8>, ChannelError> {
        let d = self.inner.recv().await?;
        if let Ok(Message::StartFile { stream, .. }) = Message::decode(&d) {
            self.open.insert(stream);
            self.most.fetch_max(self.open.len(), Ordering::SeqCst);
        }
        Ok(d)
    }
}

/// `files` small files, `capacity` in flight, over the main connection and `lanes` more. Returns
/// the most open at once on the new laptop and how many files ended which way.
async fn crowd(
    files: usize,
    capacity: usize,
    lanes: usize,
    mixed: bool,
    flip: Option<Flip>,
) -> (usize, BTreeMap<String, usize>) {
    let l = laptops(&vec![10_000usize; files]);
    let table = table(&l, flip);
    let mut sender = SenderSession::new(jobs_with(&l, mixed), capacity);
    let control = pctwin_transfer::MoveControl::new();
    let mut receiver = ReceiverSession::new(&table, plan(&l), common::journal(), "1001")
        .with_control(control.clone());
    let seen = Arc::new(Mutex::new(HashSet::new()));
    let damage = flip.is_none();
    let (a, b) = mem_pair();
    let mut old_main = DamageOnce {
        inner: a,
        seen: seen.clone(),
        damage,
    };
    let most = Arc::new(AtomicUsize::new(0));
    let mut new_main = CountOpen {
        inner: b,
        open: HashSet::new(),
        most: most.clone(),
    };
    let mut old_lanes = Vec::new();
    let mut new_lanes = Vec::new();
    for _ in 0..lanes {
        let (a, b) = mem_pair();
        old_lanes.push(DamageOnce {
            inner: a,
            seen: seen.clone(),
            damage,
        });
        new_lanes.push(b);
    }
    let done = std::cell::Cell::new(false);
    let ((s, r), _) = tokio::time::timeout(Duration::from_secs(240), async {
        tokio::join!(
            async {
                let ran = tokio::join!(
                    sender.run_lanes(&mut old_main, old_lanes),
                    receiver.run_lanes(&mut new_main, new_lanes)
                );
                done.set(true);
                ran
            },
            carry_on_whenever_braked(&control, &done)
        )
    })
    .await
    .expect("hung");
    s.unwrap();
    r.unwrap();
    let mut why: BTreeMap<String, usize> = BTreeMap::new();
    for n in 0..files {
        let k = match receiver.outcome(id(n as u8)) {
            Some(ReceiveOutcome::Finished(_)) => "finished".to_string(),
            Some(ReceiveOutcome::Failed(w)) => format!("failed: {w}"),
            other => format!("{other:?}"),
        };
        *why.entry(k).or_default() += 1;
    }
    (most.load(Ordering::SeqCst), why)
}

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn files_asked_again_never_crowd_out_the_rest_at_any_capacity() {
    for (files, capacity, lanes) in [
        (200usize, 4usize, 3usize),
        (200, 8, 3),
        (250, 16, 7),
        (250, 32, 7),
    ] {
        let (most, why) = crowd(files, capacity, lanes, true, None).await;
        assert!(most <= MAX_OPEN_FILES, "capacity {capacity}: {most} open");
        assert_eq!(
            why,
            BTreeMap::from([("finished".to_string(), files)]),
            "capacity {capacity}, lanes {lanes}"
        );
    }
}

/// Pinned as it is today: with nearly as many files in flight as the new laptop keeps open, files
/// asked again can fill it, and the files then started are refused "too many at once" (each with
/// that reason, none left without one).
#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn at_capacity_63_files_asked_again_can_fill_the_new_laptop() {
    let (_, why) = crowd(250, 63, 7, false, None).await;
    let refused = "failed: the old laptop started too many files at once".to_string();
    assert_eq!(why.values().sum::<usize>(), 250);
    assert!(
        why.keys().all(|k| k == "finished" || *k == refused),
        "{why:?}"
    );
}

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn on_a_drive_that_reads_back_wrong_crowded_files_fail_for_that_reason() {
    for (files, capacity, lanes) in [(200usize, 4usize, 3usize), (200, 16, 7)] {
        let (_, why) = crowd(files, capacity, lanes, true, Some(Flip::OnRead { at: 0 })).await;
        assert_eq!(why.len(), 1, "{why:?}");
        let (k, n) = why.iter().next().unwrap();
        assert!(k.contains("could not store this file correctly"), "{k}");
        assert_eq!(*n, files);
    }
}

// ---------------------------------------------------------------- round 4: reconnects

/// The new laptop's first `Ready` of a connection, with these resume tickets first.
async fn hello(new: &mut Mem, tickets: &[(ItemId, ResumeTicket)]) {
    for (item, ticket) in tickets {
        say(
            new,
            &Message::ResumeFrom {
                stream: 0,
                item: *item,
                ticket: ticket.clone(),
            },
        )
        .await;
    }
    say(new, &Message::Ready).await;
}

/// Each connection starts the file afresh, takes every block and answers none of its ends: the
/// same version is answered over and over, so the cap ends it, and nothing more is read.
#[tokio::test]
async fn reconnecting_alone_reads_up_to_the_answered_cap_then_nothing() {
    let l = laptops(&[300_000]);
    let reads = Reads::default();
    let mut sender = SenderSession::new(jobs(&l), 1).with_opener(Arc::new(reads.clone()));
    let mut stopped = None;
    for c in 0..60 {
        let (end, _) = connection(&mut sender, async |new: &mut Mem| {
            hello(new, &[]).await;
            let (stream, _) = start_of(&hear(new).await);
            answer_start(new, stream, None).await;
            blocks_then(new).await.1
        })
        .await;
        if let Message::GiveUp { why, .. } = end {
            assert_eq!(why, GiveUpWhy::ReadTooOften);
            stopped = Some(c);
            break;
        }
        assert!(matches!(end, Message::EndFile { .. }), "{end:?}");
    }
    let c = stopped.expect("never stopped");
    // 300 KB a time, as if 1 MiB: five times that answered, never a whole time more.
    assert!(c >= 16, "stopped too early, after {c}");
    assert!(reads.blocks() <= cap_of(300_000) + WINDOW);
    let before = reads.blocks();
    let (_, _) = connection(&mut sender, async |new: &mut Mem| {
        hello(new, &[]).await;
        let end = hear(new).await;
        assert!(matches!(end, Message::GiveUp { .. }), "{end:?}");
    })
    .await;
    assert_eq!(reads.blocks(), before);
}

/// Rounds asked again are bounded per attempt, and reconnecting to start again keeps the count of
/// what was answered (same version), so across reconnects the cap holds.
#[tokio::test]
async fn the_bound_holds_across_reconnects() {
    let l = laptops(&[300_000]);
    let header = header_of(&l, 0);
    let reads = Reads::default();
    let mut sender = SenderSession::new(jobs(&l), 1).with_opener(Arc::new(reads.clone()));
    let mut stopped = false;
    for _ in 0..40 {
        let (gave_up, _) = connection(&mut sender, async |new: &mut Mem| {
            hello(new, &[]).await;
            let start = hear(new).await;
            let Message::StartFile { stream, .. } = start else {
                return true;
            };
            answer_start(new, stream, None).await;
            if let (_, Message::GiveUp { .. }) = blocks_then(new).await {
                return true;
            }
            // Two rounds, then the connection goes (a third would be refused).
            for _ in 0..2 {
                say(
                    new,
                    &Message::SendAgain {
                        stream,
                        ticket: empty_ticket(&header),
                    },
                )
                .await;
                if let (_, Message::GiveUp { .. }) = blocks_then(new).await {
                    return true;
                }
            }
            false
        })
        .await;
        if gave_up {
            stopped = true;
            break;
        }
    }
    assert!(stopped);
    assert!(
        reads.blocks() <= cap_of(header.size) + WINDOW,
        "{}",
        reads.blocks()
    );
    assert!(matches!(
        sender.outcome(id(0)),
        None | Some(SendOutcome::Failed(_))
    ));
}

/// A resume ticket on every reconnect ("none of it here"): bounded by the cap.
async fn reconnect_with_ticket(plan: ReadPlan, unfit: bool) -> (u64, Reads, SenderSession) {
    let l = laptops(&[300_000]);
    let header = header_of(&l, 0);
    let mut ticket = empty_ticket(&header);
    if unfit {
        ticket.block_size *= 2;
    }
    let reads = Reads::default();
    let mut sender = SenderSession::new(jobs(&l), 1)
        .with_opener(Arc::new(reads.clone()))
        .with_read_plan(plan);
    for _ in 0..40 {
        let ticket = ticket.clone();
        let (end, _) = connection(&mut sender, async move |new: &mut Mem| {
            hello(new, &[(id(0), ticket)]).await;
            let start = hear(new).await;
            let Message::StartFile { stream, .. } = start else {
                return start;
            };
            answer_start(new, stream, None).await;
            blocks_then(new).await.1
        })
        .await;
        if !matches!(end, Message::EndFile { .. }) {
            assert!(
                matches!(
                    end,
                    Message::GiveUp {
                        why: GiveUpWhy::ReadTooOften,
                        ..
                    }
                ),
                "{end:?}"
            );
            break;
        }
    }
    (header.size, reads, sender)
}

#[tokio::test]
async fn reconnecting_with_a_valid_empty_ticket_is_bounded_by_the_cap() {
    let (size, reads, sender) = reconnect_with_ticket(ReadPlan::Normal, false).await;
    assert!(
        reads.blocks() <= cap_of(size) + WINDOW,
        "{}",
        reads.blocks()
    );
    assert!(reads.blocks() >= cap_of(size), "honest room was left");
    assert!(sender.outcome(id(0)).is_none() || !sender.stopped_reading());
}

#[tokio::test]
async fn reconnecting_with_a_ticket_on_a_weak_drive_is_bounded_by_the_cap() {
    let careful = ReadPlan::Careful {
        most_important_first: true,
        read_each_file_once: true,
        stop_after_read_errors: 5,
    };
    let (size, reads, sender) = reconnect_with_ticket(careful, false).await;
    assert!(
        reads.blocks() <= cap_of(size) + WINDOW,
        "{}",
        reads.blocks()
    );
    assert!(!sender.stopped_reading());
}

#[tokio::test]
async fn reconnecting_with_a_ticket_that_does_not_fit_is_bounded_by_the_cap() {
    let (size, reads, _) = reconnect_with_ticket(ReadPlan::Normal, true).await;
    assert!(
        reads.blocks() <= cap_of(size) + WINDOW,
        "{}",
        reads.blocks()
    );
}

/// Given up on (asked again once too often), then reconnecting with a ticket: never opened again;
/// the stop is said again on each connection until it is answered.
#[tokio::test]
async fn a_file_given_up_on_stays_given_up_across_reconnects() {
    let l = laptops(&[300_000]);
    let header = header_of(&l, 0);
    let reads = Reads::default();
    let mut sender = SenderSession::new(jobs(&l), 1).with_opener(Arc::new(reads.clone()));
    let ((), _) = connection(&mut sender, async |new: &mut Mem| {
        hello(new, &[]).await;
        let (stream, _) = start_of(&hear(new).await);
        answer_start(new, stream, None).await;
        let _ = blocks_then(new).await;
        for round in 0..3 {
            say(
                new,
                &Message::SendAgain {
                    stream,
                    ticket: empty_ticket(&header),
                },
            )
            .await;
            let (_, end) = blocks_then(new).await;
            assert_eq!(matches!(end, Message::GiveUp { .. }), round == 2, "{end:?}");
        }
        // Never answered: the connection goes.
    })
    .await;
    let opens = reads.opens();
    for _ in 0..5 {
        let (first, _) = connection(&mut sender, async |new: &mut Mem| {
            hello(new, &[(id(0), empty_ticket(&header))]).await;
            hear(new).await
        })
        .await;
        assert!(
            matches!(
                first,
                Message::GiveUp {
                    why: GiveUpWhy::AskedAgainTooOften,
                    ..
                }
            ),
            "{first:?}"
        );
    }
    let ((), ok) = connection(&mut sender, async |new: &mut Mem| {
        hello(new, &[(id(0), empty_ticket(&header))]).await;
        let Message::GiveUp { stream, .. } = hear(new).await else {
            panic!("expected the stop again")
        };
        done(new, stream, false).await;
        assert_eq!(hear(new).await, Message::AllSent);
    })
    .await;
    assert!(ok);
    assert_eq!(reads.blocks(), 3 * header.size, "read for the rounds only");
    assert_eq!(reads.opens(), opens, "never opened again");
    match sender.outcome(id(0)) {
        Some(SendOutcome::Failed(why)) => assert!(why.contains("kept arriving damaged"), "{why}"),
        other => panic!("{other:?}"),
    }
}

/// Asking again on every reconnect gains nothing once the file is given up on: the first
/// connection reads it three times (once and two rounds), later ones read none of it.
#[tokio::test]
async fn reconnects_after_a_file_was_given_up_on_read_none_of_it() {
    let l = laptops(&[300_000]);
    let header = header_of(&l, 0);
    let reads = Reads::default();
    let mut sender = SenderSession::new(jobs(&l), 1).with_opener(Arc::new(reads.clone()));
    let mut per = Vec::new();
    for _ in 0..4 {
        let before = reads.blocks();
        let ((), _) = connection(&mut sender, async |new: &mut Mem| {
            hello(new, &[]).await;
            let Message::StartFile { stream, .. } = hear(new).await else {
                return;
            };
            answer_start(new, stream, None).await;
            let _ = blocks_then(new).await;
            for _ in 0..3 {
                say(
                    new,
                    &Message::SendAgain {
                        stream,
                        ticket: empty_ticket(&header),
                    },
                )
                .await;
                if let (_, Message::GiveUp { .. }) = blocks_then(new).await {
                    return;
                }
            }
        })
        .await;
        per.push((reads.blocks() - before) / header.size);
    }
    assert_eq!(per, vec![3, 0, 0, 0]);
}

/// "A same-size file is here" said again and again, on one connection and on the next: the file
/// is fingerprinted once for its version, through the opener.
#[tokio::test]
async fn a_flood_of_have_reads_the_file_exactly_once() {
    // The exchange before move protocol 4: the fingerprint itself, and a bare skip.
    let l = laptops(&[2 << 20]);
    let reads = Reads::default();
    let mut sender = SenderSession::new(jobs(&l), 1)
        .with_protocol(3)
        .with_opener(Arc::new(reads.clone()));
    let other = [7u8; 32];
    for last in [false, true] {
        let ((), _) = connection(&mut sender, async |new: &mut Mem| {
            hello(new, &[]).await;
            let (stream, _) = start_of(&hear(new).await);
            for _ in 0..40 {
                answer_start(new, stream, Some(other)).await;
            }
            let (_, end) = blocks_then(new).await;
            assert!(matches!(end, Message::EndFile { .. }), "{end:?}");
            if last {
                done(new, stream, true).await;
                assert_eq!(hear(new).await, Message::AllSent);
            }
        })
        .await;
    }
    assert_eq!(
        reads.prints(),
        2 << 20,
        "one fingerprint, of the whole file"
    );
    assert_eq!(sender.outcome(id(0)), Some(&SendOutcome::Arrived));
}

/// The old laptop's main connection drops the moment it sends a start, while `lose` is set.
struct LoseStartOf {
    inner: Option<Mem>,
    lose: bool,
}

impl Channel for LoseStartOf {
    async fn send(&mut self, data: &[u8]) -> Result<(), ChannelError> {
        let Some(inner) = self.inner.as_mut() else {
            return Err(ChannelError);
        };
        if self.lose && matches!(Message::decode(data), Ok(Message::StartFile { .. })) {
            self.inner = None;
            return Err(ChannelError);
        }
        inner.send(data).await
    }

    async fn recv(&mut self) -> Result<Vec<u8>, ChannelError> {
        let Some(inner) = self.inner.as_mut() else {
            return Err(ChannelError);
        };
        inner.recv().await
    }
}

#[tokio::test]
async fn files_whose_starts_are_lost_arrive_however_many_times() {
    for (lost, capacity, files) in [
        (3u32, 1usize, 1usize),
        (4, 1, 1),
        (5, 1, 1),
        (6, 1, 1),
        (4, 4, 6),
    ] {
        let l = laptops(&vec![300_000usize; files]);
        let table = table(&l, None);
        let mut sender = SenderSession::new(jobs(&l), capacity);
        let mut receiver = ReceiverSession::new(&table, plan(&l), common::journal(), "1001");
        for c in 0..=lost {
            let (a, mut b) = mem_pair();
            let mut a = LoseStartOf {
                inner: Some(a),
                lose: c < lost,
            };
            let (s, _) = tokio::time::timeout(Duration::from_secs(60), async {
                tokio::join!(sender.run(&mut a), receiver.run(&mut b))
            })
            .await
            .expect("hung");
            assert_eq!(s.is_ok(), c == lost);
        }
        for n in 0..files {
            assert_eq!(sender.outcome(id(n as u8)), Some(&SendOutcome::Arrived));
            assert_eq!(std::fs::read(landed(&l, n)).unwrap(), l.files[n].1);
        }
    }
}

/// A ticket with the right stamp and the wrong shape (block size or count): it does not describe
/// the file sent, so the file stops, said truthfully, with nothing more read.
#[tokio::test]
async fn send_again_with_a_ticket_of_the_wrong_shape_stops_the_file_truthfully() {
    for case in 0..3 {
        let l = laptops(&[300_000]);
        let header = header_of(&l, 0);
        let reads = Reads::default();
        let mut sender = SenderSession::new(jobs(&l), 1).with_opener(Arc::new(reads.clone()));
        let (end, ok) = connection(&mut sender, async |new: &mut Mem| {
            hello(new, &[]).await;
            let (stream, _) = start_of(&hear(new).await);
            answer_start(new, stream, None).await;
            let _ = blocks_then(new).await;
            let mut t = empty_ticket(&header);
            match case {
                0 => t.block_size *= 2,
                1 => t.done = BlockMap::new(header.block_count + 1),
                _ => t.done = BlockMap::new(header.block_count - 1),
            }
            say(new, &Message::SendAgain { stream, ticket: t }).await;
            let (blocks, end) = blocks_then(new).await;
            assert_eq!(blocks, 0);
            done(new, stream, false).await;
            assert_eq!(hear(new).await, Message::AllSent);
            end
        })
        .await;
        assert!(ok);
        assert!(
            matches!(
                end,
                Message::GiveUp {
                    why: GiveUpWhy::AskedAgainTooOften,
                    ..
                }
            ),
            "case {case}: {end:?}"
        );
        assert_eq!(reads.blocks(), header.size);
        assert_eq!(reads.opens(), 1, "never opened again");
        match sender.outcome(id(0)) {
            Some(SendOutcome::Failed(why)) => {
                assert!(!why.contains("damaged"), "case {case}: {why}");
                assert!(why.contains("do not match the file"), "case {case}: {why}");
                assert!(
                    why.contains("Your original is untouched"),
                    "case {case}: {why}"
                );
            }
            other => panic!("case {case}: {other:?}"),
        }
    }
}

/// An honest file that changes on every read, real laptops: at most four versions are read, and
/// both laptops say it kept changing.
#[tokio::test]
async fn a_file_that_always_changes_is_read_in_at_most_four_versions() {
    let l = laptops(&[300_000]);
    let table = table(&l, None);
    let reads = Reads::changing(None);
    let mut sender = SenderSession::new(jobs(&l), 1).with_opener(Arc::new(reads.clone()));
    let mut receiver = ReceiverSession::new(&table, plan(&l), common::journal(), "1001");
    assert_eq!(move_all(&mut sender, &mut receiver).await, (true, true));
    assert!(reads.versions_read() <= MAX_ATTEMPTS as usize);
    assert!(reads.blocks() <= u64::from(MAX_ATTEMPTS) * 300_000);
    match sender.outcome(id(0)) {
        Some(SendOutcome::Failed(why)) => assert!(why.contains("kept changing"), "{why}"),
        other => panic!("{other:?}"),
    }
    match receiver.outcome(id(0)) {
        Some(ReceiveOutcome::Failed(why)) => assert!(why.contains("kept changing"), "{why}"),
        other => panic!("{other:?}"),
    }
}

#[tokio::test]
async fn a_file_that_changes_then_settles_arrives() {
    for changes in [1i64, 2, 3] {
        let l = laptops(&[300_000]);
        let table = table(&l, None);
        let reads = Reads::changing(Some(changes));
        let mut sender = SenderSession::new(jobs(&l), 1).with_opener(Arc::new(reads.clone()));
        let mut receiver = ReceiverSession::new(&table, plan(&l), common::journal(), "1001");
        assert_eq!(move_all(&mut sender, &mut receiver).await, (true, true));
        assert_eq!(
            sender.outcome(id(0)),
            Some(&SendOutcome::Arrived),
            "{changes}"
        );
        assert!(matches!(
            receiver.outcome(id(0)),
            Some(ReceiveOutcome::Finished(_))
        ));
    }
}

/// A new laptop that keeps starting a file that really changes before each round (and never
/// refuses a start): the old laptop reads at most four versions by its own count, then stops.
#[tokio::test]
async fn versions_are_bounded_by_what_the_old_laptop_saw_itself() {
    let l = laptops(&[300_000]);
    let path = l.files[0].0.clone();
    let reads = Reads::default();
    let mut sender = SenderSession::new(jobs(&l), 1).with_opener(Arc::new(reads.clone()));
    let (starts, ok) = connection(&mut sender, async |new: &mut Mem| {
        hello(new, &[]).await;
        let mut starts = Vec::new();
        loop {
            match hear(new).await {
                Message::StartFile { stream, header, .. } => {
                    answer_start(new, stream, None).await;
                    let (got, end) = blocks_then(new).await;
                    starts.push(got);
                    if let Message::GiveUp { .. } = end {
                        done(new, stream, false).await;
                        continue;
                    }
                    // The file is written to now (its time moves on).
                    let f = std::fs::File::options().write(true).open(&path).unwrap();
                    f.set_modified(
                        std::time::SystemTime::now()
                            + Duration::from_secs(10 * starts.len() as u64),
                    )
                    .unwrap();
                    drop(f);
                    say(
                        new,
                        &Message::SendAgain {
                            stream,
                            ticket: empty_ticket(&header),
                        },
                    )
                    .await;
                    let reply = hear(new).await;
                    assert!(matches!(reply, Message::EndFile { changed: true, .. }));
                    done(new, stream, false).await;
                }
                Message::AllSent => return starts,
                other => panic!("unexpected {other:?}"),
            }
        }
    })
    .await;
    assert!(ok);
    let read: Vec<_> = starts.iter().filter(|b| **b > 0).collect();
    assert!(read.len() <= MAX_ATTEMPTS as usize, "{starts:?}");
    assert!(reads.blocks() <= u64::from(MAX_ATTEMPTS) * 300_000);
    match sender.outcome(id(0)) {
        Some(SendOutcome::Failed(why)) => assert!(why.contains("kept changing"), "{why}"),
        other => panic!("{other:?}"),
    }
}

/// Fails the reads of the `n`th open of a file (a marginal sector that reads on a later try).
struct FailNth {
    n: u64,
    opens: AtomicU64,
}

struct BadRead {
    stamp: Stamp,
}

impl Read for BadRead {
    fn read(&mut self, _: &mut [u8]) -> std::io::Result<usize> {
        Err(std::io::Error::other("the drive could not read this area"))
    }
}

impl Seek for BadRead {
    fn seek(&mut self, _: SeekFrom) -> std::io::Result<u64> {
        Ok(0)
    }
}

impl Source for BadRead {
    fn stamp(&mut self) -> std::io::Result<Stamp> {
        Ok(self.stamp)
    }
}

impl Opener for FailNth {
    fn open(&self, path: &Path) -> std::io::Result<Box<dyn Source>> {
        let k = self.opens.fetch_add(1, Ordering::SeqCst) + 1;
        let mut real = FsOpener.open(path)?;
        if k == self.n {
            let stamp = real.stamp()?;
            return Ok(Box::new(BadRead { stamp }));
        }
        Ok(real)
    }
}

/// Honest probes keep their outcomes: three starts lost, then a marginal sector on the fourth
/// try, which reads on the promised try after everything else.
#[tokio::test]
async fn lost_starts_then_a_marginal_sector_still_arrive() {
    let l = laptops(&[300_000]);
    let table = table(&l, None);
    let opener = Arc::new(FailNth {
        n: 4,
        opens: AtomicU64::new(0),
    });
    let mut sender = SenderSession::new(jobs(&l), 1).with_opener(opener);
    let mut receiver = ReceiverSession::new(&table, plan(&l), common::journal(), "1001");
    for c in 0..6 {
        let (a, mut b) = mem_pair();
        let mut a = LoseStartOf {
            inner: Some(a),
            lose: c < 3,
        };
        let (s, _) = tokio::time::timeout(Duration::from_secs(60), async {
            tokio::join!(sender.run(&mut a), receiver.run(&mut b))
        })
        .await
        .expect("hung");
        if s.is_ok() {
            break;
        }
    }
    assert_eq!(sender.outcome(id(0)), Some(&SendOutcome::Arrived));
    assert_eq!(std::fs::read(landed(&l, 0)).unwrap(), l.files[0].1);
}

/// Honest probes keep their outcomes: after a whole-file retry, the new attempt has its own two
/// rounds, each sending only the block asked for.
#[tokio::test]
async fn after_a_whole_file_retry_the_new_attempt_has_its_own_rounds() {
    let l = laptops(&[300_000]);
    let src = l.files[0].0.clone();
    let mut sender = SenderSession::new(jobs(&l), 1);
    let one_missing = |h: &Header| {
        let mut done = BlockMap::new(h.block_count);
        for b in 1..h.block_count {
            done.insert(b).unwrap();
        }
        ResumeTicket {
            done,
            block_size: h.block_size,
            stamp: h.stamp,
        }
    };
    let ((r1, r2), ok) = connection(&mut sender, async |new: &mut Mem| {
        hello(new, &[]).await;
        let (s0, h0) = start_of(&hear(new).await);
        answer_start(new, s0, None).await;
        let _ = blocks_then(new).await;
        say(
            new,
            &Message::SendAgain {
                stream: s0,
                ticket: one_missing(&h0),
            },
        )
        .await;
        take_blocks(new, 1).await;
        std::fs::write(&src, pattern(300_001, 1234)).unwrap();
        receipt(new, s0, true).await;
        let end = hear(new).await;
        assert!(
            matches!(end, Message::EndFile { changed: true, .. }),
            "{end:?}"
        );
        done(new, s0, false).await;
        let (s1, h1) = start_of(&hear(new).await);
        assert_ne!(s1, s0);
        answer_start(new, s1, None).await;
        let _ = blocks_then(new).await;
        say(
            new,
            &Message::SendAgain {
                stream: s1,
                ticket: one_missing(&h1),
            },
        )
        .await;
        let (r1, _) = blocks_then(new).await;
        say(
            new,
            &Message::SendAgain {
                stream: s1,
                ticket: one_missing(&h1),
            },
        )
        .await;
        let (r2, end) = blocks_then(new).await;
        assert!(
            matches!(end, Message::EndFile { changed: false, .. }),
            "{end:?}"
        );
        done(new, s1, true).await;
        assert_eq!(hear(new).await, Message::AllSent);
        (r1, r2)
    })
    .await;
    assert!(ok);
    assert_eq!((r1, r2), (1, 1));
    assert_eq!(sender.outcome(id(0)), Some(&SendOutcome::Arrived));
}

/// Honest probes keep their outcomes: a SendAgain racing the file's end reads nothing once the
/// end is answered.
#[tokio::test]
async fn send_again_racing_the_answer_to_the_end() {
    for (again_first, ok) in [(true, true), (true, false), (false, true), (false, false)] {
        let l = laptops(&[300_000]);
        let header = header_of(&l, 0);
        let reads = Reads::default();
        let mut sender = SenderSession::new(jobs(&l), 1).with_opener(Arc::new(reads.clone()));
        let ((), run_ok) = connection(&mut sender, async |new: &mut Mem| {
            hello(new, &[]).await;
            let (stream, _) = start_of(&hear(new).await);
            answer_start(new, stream, None).await;
            let _ = blocks_then(new).await;
            let again = Message::SendAgain {
                stream,
                ticket: empty_ticket(&header),
            };
            let answer = Message::FileDone { stream, ok };
            let (a, b) = if again_first {
                (again, answer)
            } else {
                (answer, again)
            };
            say(new, &a).await;
            say(new, &b).await;
            loop {
                match hear(new).await {
                    Message::Piece {
                        stream, last: true, ..
                    } => receipt(new, stream, true).await,
                    Message::Piece { .. } => {}
                    Message::EndFile { stream, .. } | Message::GiveUp { stream, .. } => {
                        done(new, stream, false).await;
                    }
                    Message::AllSent => break,
                    other => panic!("unexpected {other:?}"),
                }
            }
        })
        .await;
        assert!(run_ok);
        if !again_first {
            assert_eq!(reads.blocks(), header.size, "nothing read after the answer");
        }
        assert!(sender.outcome(id(0)).is_some());
    }
}

/// The new laptop's main connection, cut as soon as the old laptop starts sending after the new
/// laptop's `after`-th SendAgain.
struct CutAfterRound {
    inner: Option<Mem>,
    asked: u32,
    after: u32,
}

impl Channel for CutAfterRound {
    async fn send(&mut self, data: &[u8]) -> Result<(), ChannelError> {
        let Some(inner) = self.inner.as_mut() else {
            return Err(ChannelError);
        };
        if let Ok(Message::SendAgain { .. }) = Message::decode(data) {
            self.asked += 1;
        }
        inner.send(data).await
    }

    async fn recv(&mut self) -> Result<Vec<u8>, ChannelError> {
        let Some(inner) = self.inner.as_mut() else {
            return Err(ChannelError);
        };
        let d = inner.recv().await?;
        if self.asked >= self.after && matches!(Message::decode(&d), Ok(Message::Piece { .. })) {
            self.inner = None;
            return Err(ChannelError);
        }
        Ok(d)
    }
}

/// Honest probes keep their outcomes: the new laptop restarts after one or two rounds on a drive
/// that reads every file back wrong; the file fails with the true reason on both laptops.
#[tokio::test]
async fn a_restart_after_one_or_two_rounds_keeps_the_true_reason() {
    for after in [1u32, 2] {
        let l = laptops(&[300_000]);
        let jdir = tempfile::tempdir_in(env!("CARGO_TARGET_TMPDIR")).unwrap();
        let journal = pctwin_journal::Journal::open(&jdir.path().join("j.redb")).unwrap();
        let table = table(&l, Some(Flip::OnRead { at: 0 }));
        let mut sender = SenderSession::new(jobs(&l), 1);
        {
            let mut receiver =
                ReceiverSession::new(&table, plan(&l), &journal, "1001").with_scope(SAME_MOVE);
            let (mut a, b) = mem_pair();
            let mut b = CutAfterRound {
                inner: Some(b),
                asked: 0,
                after,
            };
            let _ = tokio::time::timeout(Duration::from_secs(60), async {
                tokio::join!(sender.run(&mut a), receiver.run(&mut b))
            })
            .await
            .expect("hung");
            assert_eq!(b.asked, after);
        }
        pctwin_transfer::recover(&journal, &table).unwrap();
        let mut receiver =
            ReceiverSession::new(&table, plan(&l), &journal, "1001").with_scope(SAME_MOVE);
        receiver.restore().unwrap();
        let (mut a, mut b) = mem_pair();
        let (s, r) = tokio::time::timeout(Duration::from_secs(60), async {
            tokio::join!(sender.run(&mut a), receiver.run(&mut b))
        })
        .await
        .expect("hung");
        s.unwrap();
        r.unwrap();
        // The old laptop says it gave up; the new laptop, which saw its drive store the file
        // wrong, gives the true reason.
        let sent = match sender.outcome(id(0)) {
            Some(SendOutcome::Failed(why)) => why.clone(),
            other => panic!("{other:?}"),
        };
        assert!(
            sent.contains("kept arriving damaged"),
            "after {after}: {sent}"
        );
        let received = match receiver.outcome(id(0)) {
            Some(ReceiveOutcome::Failed(why)) => why,
            other => panic!("{other:?}"),
        };
        assert!(
            received.contains("could not store this file correctly"),
            "after {after}: {received}"
        );
        assert!(!landed(&l, 0).exists());
    }
}

// ---------------------------------------------------------------- round 5 and honest links

/// One 128 KiB block of progress per connection, everything else in flight lost (the review's
/// worst honest case): every file arrives, and what is read stays within the bounds.
async fn one_block_per_connection(size: u64) {
    let m = flaky(Flaky::new(&[size], (Cut::LoseAfter(1), Cut::Clean))).await;
    m.assert_arrived(false);
    m.assert_lost_reads_bounded(FLOOR, BLOCK, WINDOW);
    m.assert_reads_bounded(FLOOR, BLOCK, WINDOW);
}

#[tokio::test]
async fn one_block_per_connection_small_files_arrive_within_the_bounds() {
    for size in [300_000, MIB, 2 * MIB, 8 * MIB] {
        one_block_per_connection(size).await;
    }
}

#[tokio::test]
async fn one_block_per_connection_a_32_mib_file_arrives_within_the_bounds() {
    one_block_per_connection(32 * MIB).await;
}

#[tokio::test]
async fn one_block_per_connection_a_64_mib_file_arrives_within_the_bounds() {
    let m = flaky(Flaky::new(&[64 * MIB], (Cut::LoseAfter(1), Cut::Clean))).await;
    m.assert_arrived(false);
    m.assert_lost_reads_bounded(FLOOR, BLOCK, WINDOW);
    m.assert_reads_bounded(FLOOR, BLOCK, WINDOW);
    // About 0.6 GiB, where main read 12 GiB.
    let read = m.reads.blocks();
    assert!(read < 700 * MIB, "{} MiB read", read / MIB);
}

/// Honest flaky links: half a MiB of progress per connection, everything else in flight lost.
/// Reads stay within the file, the first window, and the floor, a block and the progress per
/// connection.
#[tokio::test]
async fn honest_flaky_links_arrive_reading_little_more_than_the_file() {
    let progress = 4; // blocks: 0.5 MiB
    for size in [8 * MIB, 32 * MIB, 64 * MIB] {
        let m = flaky(Flaky::new(&[size], (Cut::LoseAfter(progress), Cut::Clean))).await;
        m.assert_arrived(false);
        m.assert_lost_reads_bounded(FLOOR, BLOCK, WINDOW);
        let conns = m.conns.len() as u64;
        let allowed =
            size + size.min(WINDOW) + conns * (FLOOR + BLOCK + u64::from(progress) * BLOCK);
        assert!(
            m.reads.blocks() <= allowed,
            "{size}: {} read, allowed {allowed}",
            m.reads.blocks()
        );
    }
}

/// The same on three lanes, everything lost on the lanes and the main cut cleanly. (Several
/// files, so every lane carries blocks.)
#[tokio::test]
async fn honest_flaky_lanes_arrive_within_the_bounds() {
    for sizes in [vec![8 * MIB; 4], vec![2 * MIB; 8]] {
        // Whether a lane carries a block before the main connection is cut depends on how the
        // runner schedules them: every run must keep the bounds, and the scenario runs again
        // (a few times at most) until the lanes really dropped too, so it is never passed
        // without being tried.
        let mut lanes_dropped = false;
        for _ in 0..5 {
            let mut f = Flaky::new(&sizes, (Cut::CloseAfter(12), Cut::LoseAfter(4)));
            f.lanes = 3;
            let m = flaky(f).await;
            m.assert_arrived(false);
            m.assert_lost_reads_bounded(FLOOR, BLOCK, WINDOW);
            m.assert_reads_bounded(FLOOR, BLOCK, WINDOW);
            if m.losses() > m.conns.len() as u64 {
                lanes_dropped = true;
                break;
            }
        }
        assert!(
            lanes_dropped,
            "lanes dropped as well as the main connection, in one of five runs"
        );
    }
}

/// A 64 MiB file of zeros, sent compressed: what is lost at each loss is bounded in raw bytes
/// read, not in the few bytes each block takes on the wire.
#[tokio::test]
async fn an_all_zeros_file_is_bounded_in_raw_bytes() {
    let mut f = Flaky::new(&[64 * MIB], (Cut::LoseAfter(4), Cut::Clean));
    f.zeros = true;
    f.compressible = true;
    let m = flaky(f).await;
    m.assert_arrived(false);
    m.assert_lost_reads_bounded(FLOOR, BLOCK, WINDOW);
    m.assert_reads_bounded(FLOOR, BLOCK, WINDOW);
}

/// On a weak drive the window after a loss is one block.
#[tokio::test]
async fn on_a_weak_drive_the_floor_is_one_block() {
    let mut f = Flaky::new(&[8 * MIB], (Cut::LoseAfter(1), Cut::Clean));
    f.plan = ReadPlan::Careful {
        most_important_first: true,
        read_each_file_once: true,
        stop_after_read_errors: 5,
    };
    let m = flaky(f).await;
    m.assert_arrived(false);
    m.assert_lost_reads_bounded(BLOCK, BLOCK, WINDOW);
}

/// The skip is lost with the connection, the file identical: it is compared again for free on the
/// next connection, and both laptops end with it already there.
#[tokio::test]
async fn a_skip_lost_to_a_drop_still_ends_already_there() {
    for size in [300_000, 2 * MIB] {
        let mut f = Flaky::new(&[size], (Cut::Clean, Cut::Clean));
        f.prefill = Prefill::Identical;
        f.cuts = vec![(Cut::LoseSkip, Cut::Clean)];
        let m = flaky(f).await;
        m.assert_arrived(true);
        assert_eq!(m.reads.prints(), size, "fingerprinted once");
        assert_eq!(m.reads.blocks(), 0);
    }
}

/// A file that grew after its start without its size or time showing it: its fingerprint reads
/// exactly the size it started with and one byte more, and the answer is "not identical", even
/// though the start of it matches the file on the new laptop.
#[tokio::test]
async fn a_file_that_grew_after_its_start_is_not_identical() {
    let l = laptops(&[300_000]);
    let stamp = header_of(&l, 0).stamp;
    let same = *blake3::hash(&l.files[0].1).as_bytes();
    let mut longer = l.files[0].1.clone();
    longer.extend_from_slice(b"more");
    std::fs::write(&l.files[0].0, &longer).unwrap();
    let reads = Reads {
        claims: Some(stamp),
        ..Default::default()
    };
    let mut sender = SenderSession::new(jobs(&l), 1).with_opener(Arc::new(reads.clone()));
    let (next, _) = connection(&mut sender, async |new: &mut Mem| {
        hello(new, &[]).await;
        let (stream, _) = start_of(&hear(new).await);
        answer_start(new, stream, Some(same)).await;
        hear(new).await
    })
    .await;
    assert!(
        matches!(next, Message::Piece { .. }),
        "sent, not skipped: {next:?}"
    );
    assert_eq!(reads.prints(), 300_000 + 1);
}

// ---------------------------------------------------------------- fingerprints (C), skips

/// Invariant 4: the fingerprint is read through the opener, once per version, however often the
/// file is started at that version.
#[tokio::test]
async fn a_fingerprint_is_read_through_the_opener_once_per_version() {
    // The exchange before move protocol 4: the fingerprint itself, and a bare skip.
    let l = laptops(&[2 << 20]);
    let same = *blake3::hash(&l.files[0].1).as_bytes();
    let reads = Reads::default();
    let mut sender = SenderSession::new(jobs(&l), 1)
        .with_protocol(3)
        .with_opener(Arc::new(reads.clone()));
    // The skip is never answered: the connection goes, three times.
    for _ in 0..3 {
        let (said, _) = connection(&mut sender, async |new: &mut Mem| {
            hello(new, &[]).await;
            let (stream, _) = start_of(&hear(new).await);
            answer_start(new, stream, Some(same)).await;
            hear(new).await
        })
        .await;
        assert!(matches!(said, Message::Skip { .. }), "{said:?}");
        // Invariant 9: not already there until the new laptop says so.
        assert_eq!(sender.outcome(id(0)), None);
    }
    let ((), ok) = connection(&mut sender, async |new: &mut Mem| {
        hello(new, &[]).await;
        let (stream, _) = start_of(&hear(new).await);
        answer_start(new, stream, Some(same)).await;
        assert_eq!(hear(new).await, Message::Skip { stream });
        done(new, stream, true).await;
        assert_eq!(hear(new).await, Message::AllSent);
    })
    .await;
    assert!(ok);
    assert_eq!(
        reads.prints(),
        2 << 20,
        "one fingerprint, through the opener"
    );
    assert_eq!(reads.blocks(), 0);
    assert_eq!(sender.outcome(id(0)), Some(&SendOutcome::AlreadyThere));
}

/// Invariant 9: a skip the new laptop does not accept is never "already there" (with a laptop on
/// version 3, it fails).
#[tokio::test]
async fn a_skip_answered_not_accepted_is_not_already_there() {
    let l = laptops(&[300_000]);
    let same = *blake3::hash(&l.files[0].1).as_bytes();
    let mut sender = SenderSession::new(jobs(&l), 1).with_protocol(3);
    let ((), ok) = connection(&mut sender, async |new: &mut Mem| {
        hello(new, &[]).await;
        let (stream, _) = start_of(&hear(new).await);
        answer_start(new, stream, Some(same)).await;
        assert_eq!(hear(new).await, Message::Skip { stream });
        done(new, stream, false).await;
        assert_eq!(hear(new).await, Message::AllSent);
    })
    .await;
    assert!(ok);
    assert!(
        matches!(sender.outcome(id(0)), Some(SendOutcome::Failed(_))),
        "{:?}",
        sender.outcome(id(0))
    );
}

/// From version 4 a skip not accepted (the new laptop could not use the copy it has) is sent
/// after all, once, and never skipped again: it arrives.
#[tokio::test]
async fn a_skip_not_accepted_is_sent_after_all_and_never_skipped_again() {
    let l = laptops(&[300_000]);
    let same = *blake3::hash(&l.files[0].1).as_bytes();
    let mut sender = SenderSession::new(jobs(&l), 1);
    let ((), ok) = connection(&mut sender, async |new: &mut Mem| {
        hello(new, &[]).await;
        let (stream, _) = start_of(&hear(new).await);
        answer_start(new, stream, Some(same)).await;
        assert!(
            matches!(hear(new).await, Message::Print { stream: s, .. } if s == stream),
            "offered with its print, never skipped on its own word"
        );
        done(new, stream, false).await;
        // Started again; offered the same fingerprint, it is sent this time.
        let (stream, _) = start_of(&hear(new).await);
        answer_start(new, stream, Some(same)).await;
        let (blocks, end) = blocks_then(new).await;
        assert_eq!(blocks, 3);
        assert!(matches!(end, Message::EndFile { .. }), "{end:?}");
        done(new, stream, true).await;
        assert_eq!(hear(new).await, Message::AllSent);
    })
    .await;
    assert!(ok);
    assert_eq!(sender.outcome(id(0)), Some(&SendOutcome::Arrived));
}

/// The new laptop answers every skip, and believes none: with a laptop on move protocol 3 it
/// never offers what is here (an older laptop would get its own fingerprint), so even a file
/// that is already there is answered "no" and sent.
#[tokio::test]
async fn the_new_laptop_answers_every_skip() {
    let l = laptops(&[300_000]);
    std::fs::create_dir_all(l.new.path().join("Documents")).unwrap();
    std::fs::write(landed(&l, 0), &l.files[0].1).unwrap();
    let table = table(&l, None);
    let mut receiver =
        ReceiverSession::new(&table, plan(&l), common::journal(), "1001").with_protocol(3);
    let header = header_of(&l, 0);
    let (mut old, mut new) = mem_pair();
    let script = async {
        assert_eq!(hear(&mut old).await, Message::Ready);
        say(&mut old, &Message::Skip { stream: 9 }).await;
        assert_eq!(
            hear(&mut old).await,
            Message::FileDone {
                stream: 9,
                ok: false
            }
        );
        say(
            &mut old,
            &Message::StartFile {
                stream: 0,
                item: id(0),
                destination: "me".into(),
                path: "Documents/f0.bin".into(),
                header,
                resumed_done: 0,
            },
        )
        .await;
        assert_eq!(
            hear(&mut old).await,
            Message::Have {
                stream: 0,
                same_size: None
            }
        );
        say(&mut old, &Message::Skip { stream: 0 }).await;
        assert_eq!(
            hear(&mut old).await,
            Message::FileDone {
                stream: 0,
                ok: false
            }
        );
        say(&mut old, &Message::AllSent).await;
    };
    let ((), r) = tokio::time::timeout(Duration::from_secs(30), async {
        tokio::join!(script, receiver.run(&mut new))
    })
    .await
    .expect("hung");
    r.unwrap();
    assert!(!matches!(
        receiver.outcome(id(0)),
        Some(ReceiveOutcome::AlreadyThere(_))
    ));
}

// ---------------------------------------------------------------- the answered cap (B)

/// Exactly at the cap is still allowed; one receipt past it stops the file. A file of exactly
/// 1 MiB, answered whole as many times as the cap allows: the next time its first receipt stops
/// it.
#[tokio::test]
async fn the_cap_allows_exactly_its_times_and_stops_at_the_next_receipt() {
    let l = laptops(&[MIB as usize]);
    let reads = Reads::default();
    let mut sender = SenderSession::new(jobs(&l), 1).with_opener(Arc::new(reads.clone()));
    for c in 0..1 + REREADS {
        let (end, _) = connection(&mut sender, async |new: &mut Mem| {
            hello(new, &[]).await;
            let (stream, _) = start_of(&hear(new).await);
            answer_start(new, stream, None).await;
            blocks_then(new).await
        })
        .await;
        assert_eq!(end.0, MIB / BLOCK, "connection {c}");
        assert!(
            matches!(end.1, Message::EndFile { .. }),
            "connection {c}: {:?}",
            end.1
        );
    }
    let (end, _) = connection(&mut sender, async |new: &mut Mem| {
        hello(new, &[]).await;
        let (stream, _) = start_of(&hear(new).await);
        answer_start(new, stream, None).await;
        // The window after a loss: the floor (all of this file); one receipt goes.
        take_blocks(new, MIB / BLOCK).await;
        receipt(new, stream, true).await;
        hear(new).await
    })
    .await;
    assert!(
        matches!(
            end,
            Message::GiveUp {
                why: GiveUpWhy::ReadTooOften,
                ..
            }
        ),
        "{end:?}"
    );
    assert_eq!(reads.blocks(), (2 + REREADS) * MIB);
}

/// A new laptop that starts again and again (losing everything each time) until the cap: both
/// laptops record the file failed, with the reason, and nothing more of it is read.
#[tokio::test]
async fn at_the_cap_both_laptops_record_why_and_nothing_more_is_read() {
    /// The old laptop's main connection, dropped the moment it sends a file's end.
    struct DropAtEnd(Option<Mem>);

    impl Channel for DropAtEnd {
        async fn send(&mut self, data: &[u8]) -> Result<(), ChannelError> {
            let Some(inner) = self.0.as_mut() else {
                return Err(ChannelError);
            };
            if matches!(Message::decode(data), Ok(Message::EndFile { .. })) {
                self.0 = None;
                return Err(ChannelError);
            }
            inner.send(data).await
        }

        async fn recv(&mut self) -> Result<Vec<u8>, ChannelError> {
            self.0.as_mut().ok_or(ChannelError)?.recv().await
        }
    }

    let l = laptops(&[MIB as usize]);
    let table = table(&l, None);
    let reads = Reads::default();
    let mut sender = SenderSession::new(jobs(&l), 1).with_opener(Arc::new(reads.clone()));
    let mut last = None;
    for _ in 0..12 {
        // A new laptop that lost everything (a fresh record each time).
        let mut receiver = ReceiverSession::new(&table, plan(&l), common::journal(), "1001");
        let (a, mut b) = mem_pair();
        let mut a = DropAtEnd(Some(a));
        let (s, _) = tokio::time::timeout(Duration::from_secs(60), async {
            tokio::join!(sender.run(&mut a), receiver.run(&mut b))
        })
        .await
        .expect("hung");
        last = Some(receiver.outcome(id(0)));
        if s.is_ok() {
            break;
        }
    }
    match sender.outcome(id(0)) {
        Some(SendOutcome::Failed(why)) => {
            assert!(why.contains("asked for it again many more times"), "{why}");
        }
        other => panic!("{other:?}"),
    }
    match last.flatten() {
        Some(ReceiveOutcome::Failed(why)) => {
            assert!(why.contains("asked for it again many more times"), "{why}");
        }
        other => panic!("{other:?}"),
    }
    assert!(reads.blocks() <= cap_of(MIB) + WINDOW, "{}", reads.blocks());
}

// ---------------------------------------------------------------- any honest flaky link

/// The loss modes of the property test.
fn mode_cuts(mode: u8, k: u32) -> (Cut, Cut) {
    match mode {
        // Everything in flight is lost.
        0 => (Cut::LoseAfter(k), Cut::LoseAfter(k)),
        // A clean cut: what was sent arrives.
        1 => (Cut::CloseAfter(k), Cut::CloseAfter(k)),
        // Only the receipts are lost.
        2 => (Cut::ReceiptsLostAfter(k), Cut::ReceiptsLostAfter(k)),
        // Only the lanes drop.
        3 => (Cut::Clean, Cut::LoseAfter(k)),
        // Only the main connection drops.
        _ => (Cut::LoseAfter(k), Cut::Clean),
    }
}

proptest! {
    #![proptest_config(ProptestConfig { cases: 24, ..ProptestConfig::default() })]

    /// On any honest flaky link (every connection writes at least one whole new block, except
    /// at most one that makes none after the first), every file arrives, and invariants 1, 6, 7
    /// and 8 hold with a small in-flight limit.
    #[test]
    fn the_bounds_hold_on_any_honest_flaky_link(
        sizes in prop::collection::vec(prop_oneof![
            Just(0u64), Just(1), Just(BLOCK - 1), Just(BLOCK), Just(BLOCK + 1),
            (2u64..6).prop_map(|k| k * BLOCK),
            (0u64..3).prop_map(|d| 2 * MIB - 1 + d),
            (0u64..3).prop_map(|d| 4 * MIB - 1 + d),
        ], 1..=3),
        lanes in prop_oneof![Just(0usize), Just(1), Just(7)],
        mode in 0u8..5,
        progress in prop_oneof![Just(1u32), 1u32..12],
        prefill in 0u8..3,
        idle in any::<bool>(),
    ) {
        let window = 2 * MIB;
        let prefill = match prefill {
            0 => Prefill::None,
            1 => Prefill::Identical,
            _ => Prefill::Different,
        };
        let rt = tokio::runtime::Builder::new_current_thread().enable_all().build().unwrap();
        rt.block_on(async {
            // Several files, so that extra lanes carry blocks (files under 64 MiB are never
            // split between lanes).
            let mut f = Flaky::new(&sizes, mode_cuts(mode, progress));
            f.lanes = lanes;
            f.in_flight = Some(window);
            f.prefill = prefill;
            if idle {
                // The second connection makes no progress at all.
                f.cuts = vec![mode_cuts(mode, progress), (Cut::LoseAfter(0), Cut::LoseAfter(0))];
            }
            f.max_conns = 400;
            let m = flaky(f).await;
            // An empty file is identical to any empty file.
            for (n, size) in sizes.iter().enumerate() {
                let identical =
                    prefill == Prefill::Identical || (prefill != Prefill::None && *size == 0);
                if sizes.len() > 1 && m.sent[n] != Some(SendOutcome::Arrived) && !identical {
                    // With several files sharing a connection's progress, a file can get no
                    // block on a connection, and the new laptop then counts its next start as a
                    // fresh attempt until it refuses it as "kept changing". That is the new
                    // laptop's continuation, its own change in the Task List; it is the only
                    // way such a file may end otherwise, and nothing is read for it after.
                    m.assert_refused_as_new_attempts(n);
                } else {
                    m.assert_arrived_one(n, identical);
                }
            }
            m.assert_lost_reads_bounded(FLOOR, BLOCK, window);
            m.assert_no_progress_bounded(FLOOR.min(window), BLOCK);
            m.assert_reads_bounded(FLOOR, BLOCK, window);
        });
    }
}

// ---------------------------------------------------------------- the Test-Reviewer's survivors

/// Takes blocks, receipting each "not written", until the old laptop says something else.
async fn blocks_unwritten_then(new: &mut Mem) -> (u64, Message) {
    let mut blocks = 0;
    loop {
        match hear(new).await {
            Message::Piece {
                stream, last: true, ..
            } => {
                blocks += 1;
                receipt(new, stream, false).await;
            }
            Message::Piece { .. } => {}
            other => return (blocks, other),
        }
    }
}

/// A new laptop that answers every block "not written", connection after connection (each
/// starting afresh, or each with a resume ticket saying it has none): every answer counts, so the
/// cap stops it, and nothing more is read.
async fn answered_not_written_until_the_cap(with_ticket: bool) {
    let l = laptops(&[300_000]);
    let header = header_of(&l, 0);
    let reads = Reads::default();
    let mut sender = SenderSession::new(jobs(&l), 1).with_opener(Arc::new(reads.clone()));
    let tickets = if with_ticket {
        vec![(id(0), empty_ticket(&header))]
    } else {
        Vec::new()
    };
    let mut stopped = false;
    for _ in 0..60 {
        let (end, _) = connection(&mut sender, async |new: &mut Mem| {
            hello(new, &tickets).await;
            let (stream, _) = start_of(&hear(new).await);
            answer_start(new, stream, None).await;
            blocks_unwritten_then(new).await.1
        })
        .await;
        if let Message::GiveUp { why, .. } = end {
            assert_eq!(why, GiveUpWhy::ReadTooOften);
            stopped = true;
            break;
        }
        assert!(matches!(end, Message::EndFile { .. }), "{end:?}");
    }
    assert!(stopped, "answers \"not written\" were never counted");
    assert!(
        reads.blocks() <= cap_of(header.size) + WINDOW,
        "{}",
        reads.blocks()
    );
    let before = reads.blocks();
    let ((), ok) = connection(&mut sender, async |new: &mut Mem| {
        hello(new, &tickets).await;
        let Message::GiveUp { stream, why } = hear(new).await else {
            panic!("expected the stop again")
        };
        assert_eq!(why, GiveUpWhy::ReadTooOften);
        done(new, stream, false).await;
        assert_eq!(hear(new).await, Message::AllSent);
    })
    .await;
    assert!(ok);
    assert_eq!(reads.blocks(), before, "nothing more was read");
    match sender.outcome(id(0)) {
        Some(SendOutcome::Failed(why)) => {
            assert!(why.contains("asked for it again many more times"), "{why}");
        }
        other => panic!("{other:?}"),
    }
}

#[tokio::test]
async fn blocks_answered_not_written_count_toward_the_cap_across_reconnects() {
    answered_not_written_until_the_cap(false).await;
}

#[tokio::test]
async fn blocks_answered_not_written_count_toward_the_cap_with_resume_tickets() {
    answered_not_written_until_the_cap(true).await;
}

/// Real laptops: every block arrives damaged (so the new laptop answers each "not written"), and
/// the new laptop loses everything each time (a fresh record per connection, the connection
/// going as the old laptop ends the file). Both laptops end with the cap's reason.
#[tokio::test]
async fn blocks_damaged_on_every_connection_stop_at_the_cap_on_both_laptops() {
    /// Damages every block, and drops the moment the old laptop ends a file.
    struct DamageAllDropAtEnd(Option<Mem>);

    impl Channel for DamageAllDropAtEnd {
        async fn send(&mut self, data: &[u8]) -> Result<(), ChannelError> {
            let Some(inner) = self.0.as_mut() else {
                return Err(ChannelError);
            };
            match Message::decode(data) {
                Ok(Message::EndFile { .. }) => {
                    self.0 = None;
                    Err(ChannelError)
                }
                Ok(Message::Piece {
                    stream,
                    last: true,
                    mut bytes,
                }) => {
                    let at = bytes.len() - 1;
                    bytes[at] ^= 0x5A;
                    let m = Message::Piece {
                        stream,
                        last: true,
                        bytes,
                    };
                    inner.send(&m.encode()).await
                }
                _ => inner.send(data).await,
            }
        }

        async fn recv(&mut self) -> Result<Vec<u8>, ChannelError> {
            self.0.as_mut().ok_or(ChannelError)?.recv().await
        }
    }

    let l = laptops(&[MIB as usize]);
    let table = table(&l, None);
    let reads = Reads::default();
    let mut sender = SenderSession::new(jobs(&l), 1).with_opener(Arc::new(reads.clone()));
    let mut last = None;
    for _ in 0..20 {
        let mut receiver = ReceiverSession::new(&table, plan(&l), common::journal(), "1001");
        let (a, mut b) = mem_pair();
        let mut a = DamageAllDropAtEnd(Some(a));
        let (s, _) = tokio::time::timeout(Duration::from_secs(60), async {
            tokio::join!(sender.run(&mut a), receiver.run(&mut b))
        })
        .await
        .expect("hung");
        last = Some(receiver.outcome(id(0)));
        if s.is_ok() {
            break;
        }
    }
    for why in [
        match sender.outcome(id(0)) {
            Some(SendOutcome::Failed(why)) => why.clone(),
            other => panic!("{other:?}"),
        },
        match last.flatten() {
            Some(ReceiveOutcome::Failed(why)) => why,
            other => panic!("{other:?}"),
        },
    ] {
        assert!(why.contains("asked for it again many more times"), "{why}");
    }
    assert!(reads.blocks() <= cap_of(MIB) + WINDOW, "{}", reads.blocks());
}

/// The file at `l`'s `n`th source, answered whole `times` times on fresh starts that the new
/// laptop never ends, then `partly` of its blocks once more.
async fn answer_again_and_again(sender: &mut SenderSession, times: u64, partly: u64) {
    for _ in 0..times {
        let (end, _) = connection(sender, async |new: &mut Mem| {
            hello(new, &[]).await;
            let (stream, _) = start_of(&hear(new).await);
            answer_start(new, stream, None).await;
            blocks_then(new).await.1
        })
        .await;
        assert!(matches!(end, Message::EndFile { .. }), "{end:?}");
    }
    let ((), _) = connection(sender, async |new: &mut Mem| {
        hello(new, &[]).await;
        let (stream, _) = start_of(&hear(new).await);
        answer_start(new, stream, None).await;
        take_blocks(new, FLOOR / BLOCK).await;
        for _ in 0..partly {
            receipt(new, stream, true).await;
        }
        // Let the receipts be read before the connection goes.
        tokio::time::sleep(Duration::from_millis(200)).await;
    })
    .await;
}

/// What was answered counts for one version only: a file answered close to the cap, then changed
/// on the old laptop, arrives as its new version.
#[tokio::test]
async fn a_new_version_starts_its_count_again_and_arrives() {
    let l = laptops(&[MIB as usize]);
    let table = table(&l, None);
    let mut sender = SenderSession::new(jobs(&l), 1);
    // Version 1: five and a half times answered (the cap is six).
    answer_again_and_again(&mut sender, 1 + REREADS - 1, 4).await;
    // The file changes on the old laptop.
    let changed = pattern(MIB as usize, 4242);
    std::fs::write(&l.files[0].0, &changed).unwrap();
    let f = std::fs::File::options()
        .write(true)
        .open(&l.files[0].0)
        .unwrap();
    f.set_modified(std::time::SystemTime::now() + Duration::from_secs(60))
        .unwrap();
    drop(f);
    let mut receiver = ReceiverSession::new(&table, plan(&l), common::journal(), "1001");
    assert_eq!(move_all(&mut sender, &mut receiver).await, (true, true));
    assert_eq!(sender.outcome(id(0)), Some(&SendOutcome::Arrived));
    assert_eq!(std::fs::read(landed(&l, 0)).unwrap(), changed);
}

/// Receipts that land after the cap stopped a file give one stop, never another; and receipts that
/// land after a file ended for another reason never change that reason.
#[tokio::test]
async fn receipts_after_a_stop_never_stop_it_again_or_change_why() {
    // Exactly at the cap, then a connection with the floor in flight.
    let l = laptops(&[MIB as usize]);
    let mut sender = SenderSession::new(jobs(&l), 1);
    answer_again_and_again(&mut sender, 1 + REREADS, 0).await;
    let (said, ok) = connection(&mut sender, async |new: &mut Mem| {
        hello(new, &[]).await;
        let (stream, _) = start_of(&hear(new).await);
        answer_start(new, stream, None).await;
        take_blocks(new, FLOOR / BLOCK).await;
        // Every block in flight answered: the first is over the cap, the rest land after.
        for _ in 0..FLOOR / BLOCK {
            receipt(new, stream, true).await;
        }
        let mut said = Vec::new();
        loop {
            match hear(new).await {
                m @ (Message::GiveUp { stream, .. } | Message::EndFile { stream, .. }) => {
                    said.push(m);
                    done(new, stream, false).await;
                }
                Message::AllSent => return said,
                other => panic!("unexpected {other:?}"),
            }
        }
    })
    .await;
    assert!(ok);
    assert_eq!(
        said,
        vec![Message::GiveUp {
            stream: start_stream(&said),
            why: GiveUpWhy::ReadTooOften
        }]
    );
    match sender.outcome(id(0)) {
        Some(SendOutcome::Failed(why)) => {
            assert!(why.contains("asked for it again many more times"), "{why}");
        }
        other => panic!("{other:?}"),
    }

    // Close to the cap, then the new laptop cannot write a block: the file ends for that reason,
    // and the receipts landing after it (past the cap) change nothing.
    let l = laptops(&[MIB as usize]);
    let mut sender = SenderSession::new(jobs(&l), 1);
    answer_again_and_again(&mut sender, 1 + REREADS, 0).await;
    let (said, ok) = connection(&mut sender, async |new: &mut Mem| {
        hello(new, &[]).await;
        let (stream, _) = start_of(&hear(new).await);
        answer_start(new, stream, None).await;
        take_blocks(new, FLOOR / BLOCK).await;
        say(new, &Message::Refused { stream }).await;
        for _ in 1..FLOOR / BLOCK {
            receipt(new, stream, true).await;
        }
        let mut said = Vec::new();
        loop {
            match hear(new).await {
                m @ (Message::GiveUp { stream, .. } | Message::EndFile { stream, .. }) => {
                    said.push(m);
                    done(new, stream, false).await;
                }
                Message::AllSent => return said,
                other => panic!("unexpected {other:?}"),
            }
        }
    })
    .await;
    assert!(ok);
    assert_eq!(said.len(), 1, "{said:?}");
    assert!(
        matches!(said[0], Message::EndFile { changed: true, .. }),
        "{said:?}"
    );
    assert_eq!(
        sender.outcome(id(0)),
        Some(&SendOutcome::Failed(Failure::new(
            Cause::CouldNotWrite,
            "the new laptop could not write part of it"
        )))
    );
}

fn start_stream(said: &[Message]) -> u32 {
    match said.first() {
        Some(Message::GiveUp { stream, .. } | Message::EndFile { stream, .. }) => *stream,
        _ => u32::MAX,
    }
}

/// A file whose stamp moves while its fingerprint is read (it was written to meanwhile) is not
/// identical, even though the bytes read match the file on the new laptop.
#[tokio::test]
async fn a_file_that_changed_while_its_fingerprint_was_read_is_not_identical() {
    let l = laptops(&[300_000]);
    let same = *blake3::hash(&l.files[0].1).as_bytes();
    let reads = Reads::changing(None);
    let mut sender = SenderSession::new(jobs(&l), 1).with_opener(Arc::new(reads.clone()));
    let (next, _) = connection(&mut sender, async |new: &mut Mem| {
        hello(new, &[]).await;
        let (stream, _) = start_of(&hear(new).await);
        answer_start(new, stream, Some(same)).await;
        hear(new).await
    })
    .await;
    assert!(
        matches!(next, Message::Piece { .. }),
        "sent, not skipped: {next:?}"
    );
    assert_eq!(reads.prints(), 300_000, "the whole fingerprint was read");
    assert_ne!(sender.outcome(id(0)), Some(&SendOutcome::AlreadyThere));
}

/// A reconnect with resume tickets for more files than may be in flight: the old laptop opens
/// only as many as may be in flight; the rest wait for room, and every one still arrives.
#[tokio::test]
async fn a_reconnect_with_many_tickets_opens_only_what_may_be_in_flight() {
    const FILES: usize = 70;
    const CAPACITY: usize = 4;
    let l = laptops(&vec![10_000usize; FILES]);
    let tickets: Vec<(ItemId, ResumeTicket)> = (0..FILES)
        .map(|n| (id(n as u8), empty_ticket(&header_of(&l, n))))
        .collect();
    let reads = Reads::default();
    let mut sender = SenderSession::new(jobs(&l), CAPACITY).with_opener(Arc::new(reads.clone()));
    let ((), _) = connection(&mut sender, async |new: &mut Mem| {
        hello(new, &tickets).await;
        // The starts of the files in flight come; nothing is answered.
        let mut starts = 0;
        while let Ok(Ok(d)) = tokio::time::timeout(Duration::from_millis(300), new.recv()).await {
            if let Ok(Message::StartFile { .. }) = Message::decode(&d) {
                starts += 1;
            }
        }
        assert_eq!(starts, CAPACITY);
    })
    .await;
    assert!(
        reads.opens() <= CAPACITY as u64,
        "{} files opened for {CAPACITY} in flight",
        reads.opens()
    );
    assert!(
        reads.most_live() <= CAPACITY as i64,
        "{}",
        reads.most_live()
    );
    // And each still arrives, a few at a time.
    let table = table(&l, None);
    let mut receiver = ReceiverSession::new(&table, plan(&l), common::journal(), "1001");
    assert_eq!(move_all(&mut sender, &mut receiver).await, (true, true));
    for n in 0..FILES {
        assert_eq!(
            sender.outcome(id(n as u8)),
            Some(&SendOutcome::Arrived),
            "{n}"
        );
    }
    assert!(
        reads.most_live() <= CAPACITY as i64,
        "{}",
        reads.most_live()
    );
}

// ---------------------------------------------------------------- tickets are used once

/// Opens from the disk; the first read of `path` at `offset` (after a seek there) fails as a
/// drive error, once.
struct FailOnceAt {
    path: PathBuf,
    offset: u64,
    fired: Arc<std::sync::atomic::AtomicBool>,
}

struct FailOnceSource {
    inner: Box<dyn Source>,
    arm: Option<(u64, Arc<std::sync::atomic::AtomicBool>)>,
    pos: u64,
}

impl Read for FailOnceSource {
    fn read(&mut self, buf: &mut [u8]) -> std::io::Result<usize> {
        if let Some((at, fired)) = &self.arm
            && self.pos == *at
            && !fired.swap(true, Ordering::SeqCst)
        {
            return Err(std::io::Error::other("the drive could not read this area"));
        }
        let n = self.inner.read(buf)?;
        self.pos += n as u64;
        Ok(n)
    }
}

impl Seek for FailOnceSource {
    fn seek(&mut self, to: SeekFrom) -> std::io::Result<u64> {
        self.pos = self.inner.seek(to)?;
        Ok(self.pos)
    }
}

impl Source for FailOnceSource {
    fn stamp(&mut self) -> std::io::Result<Stamp> {
        self.inner.stamp()
    }
}

impl Opener for FailOnceAt {
    fn open(&self, path: &Path) -> std::io::Result<Box<dyn Source>> {
        let inner = FsOpener.open(path)?;
        let arm = (path == self.path).then(|| (self.offset, self.fired.clone()));
        Ok(Box::new(FailOnceSource { inner, arm, pos: 0 }))
    }
}

fn fail_once_at(path: &Path, offset: u64) -> Arc<FailOnceAt> {
    Arc::new(FailOnceAt {
        path: path.to_path_buf(),
        offset,
        fired: Arc::new(std::sync::atomic::AtomicBool::new(false)),
    })
}

/// A ticket saying the new laptop has `done` of the file `h` describes.
fn ticket_with(h: &Header, done: std::ops::Range<u64>) -> ResumeTicket {
    let mut map = BlockMap::new(h.block_count);
    for b in done {
        map.insert(b).unwrap();
    }
    ResumeTicket {
        done: map,
        block_size: h.block_size,
        stamp: h.stamp,
    }
}

/// Within one connection: a file continued from its ticket, then a read error ends it as changed
/// and the new laptop drops what it had. Its one more try after everything else is a new attempt,
/// on a new stream, from the beginning: the ticket was used once and is gone.
#[tokio::test]
async fn the_retry_after_a_read_error_never_reuses_the_ticket() {
    let l = laptops(&[600_000]);
    let h = header_of(&l, 0);
    let mut sender =
        SenderSession::new(jobs(&l), 1).with_opener(fail_once_at(&l.files[0].0, 3 * BLOCK));
    let ticket = ticket_with(&h, 0..3);
    let (starts, ok) = connection(&mut sender, async |new: &mut Mem| {
        hello(new, &[(id(0), ticket.clone())]).await;
        let mut starts = Vec::new();
        loop {
            match hear(new).await {
                Message::StartFile {
                    stream,
                    resumed_done,
                    ..
                } => {
                    starts.push((stream, resumed_done));
                    answer_start(new, stream, None).await;
                }
                Message::Piece {
                    stream, last: true, ..
                } => receipt(new, stream, true).await,
                Message::Piece { .. } => {}
                // A file ended "changed": the new laptop drops what it had of it.
                Message::EndFile {
                    stream, changed, ..
                } => done(new, stream, !changed).await,
                Message::AllSent => break,
                other => panic!("{other:?}"),
            }
        }
        starts
    })
    .await;
    assert!(ok);
    assert_eq!(starts.len(), 2, "{starts:?}");
    assert_eq!(starts[0].1, 3, "first continued from the ticket");
    assert_eq!(starts[1].1, 0, "the retry starts again whole");
    assert_ne!(starts[1].0, starts[0].0, "the retry has a new stream");
    assert_eq!(sender.outcome(id(0)), Some(&SendOutcome::Arrived));
}

/// A skip never answered (lost with the connection) starts again on the next connection whatever
/// the ticket says, and is compared again for free.
#[tokio::test]
async fn an_unanswered_skip_ignores_the_ticket() {
    // The exchange before move protocol 4: the fingerprint itself, and a bare skip.
    let l = laptops(&[600_000]);
    let h = header_of(&l, 0);
    let same = *blake3::hash(&l.files[0].1).as_bytes();
    let reads = Reads::default();
    let mut sender = SenderSession::new(jobs(&l), 1)
        .with_protocol(3)
        .with_opener(Arc::new(reads.clone()));
    let ((), _) = connection(&mut sender, async |new: &mut Mem| {
        hello(new, &[]).await;
        let (stream, _) = start_of(&hear(new).await);
        answer_start(new, stream, Some(same)).await;
        let m = hear(new).await;
        assert!(matches!(m, Message::Skip { .. }), "{m:?}");
        // The connection goes before the skip is answered.
    })
    .await;
    let prints = reads.prints();
    let ticket = ticket_with(&h, 0..3);
    let (resumed, ok) = connection(&mut sender, async |new: &mut Mem| {
        hello(new, &[(id(0), ticket.clone())]).await;
        let Message::StartFile {
            stream,
            resumed_done,
            ..
        } = hear(new).await
        else {
            panic!("expected a start")
        };
        answer_start(new, stream, Some(same)).await;
        assert_eq!(hear(new).await, Message::Skip { stream });
        done(new, stream, true).await;
        assert_eq!(hear(new).await, Message::AllSent);
        resumed_done
    })
    .await;
    assert!(ok);
    assert_eq!(resumed, 0, "the ticket of an unanswered skip was used");
    assert_eq!(reads.prints(), prints, "compared again for free");
    assert_eq!(reads.blocks(), 0);
    assert_eq!(sender.outcome(id(0)), Some(&SendOutcome::AlreadyThere));
}

/// The same retry on lanes: blocks of the attempt that ended are still on their way on a lane
/// when the file starts again, and they are of the old attempt's stream, never the new one, so
/// they cannot land in it. (A file of 72 MiB, so a lane takes its back half; the main connection
/// meets the read error.)
#[tokio::test]
async fn blocks_still_on_a_lane_from_an_ended_attempt_cannot_land_in_its_retry() {
    let old = tempfile::tempdir_in(env!("CARGO_TARGET_TMPDIR")).unwrap();
    let src = old.path().join("f0.bin");
    std::fs::write(&src, vec![0u8; 72 << 20]).unwrap();
    let h = FileSender::open(&src, None, false)
        .unwrap()
        .header()
        .clone();
    let job = SendJob {
        size: 0,
        item: id(0),
        source: src.clone(),
        destination: "me".into(),
        path: "Documents/f0.bin".into(),
        compressible: false,
        tier: Tier::Rest,
    };
    let mut sender = SenderSession::new(vec![job], 1).with_opener(fail_once_at(&src, 20 * BLOCK));
    let ticket = ticket_with(&h, 0..3);
    let (mut main, new) = mem_pair();
    let (lane, mut new_lane) = mem_pair();
    let ended = tokio::sync::Notify::new();
    let main_side = async {
        // Owned here, so the connection goes when this side is done.
        let mut new = new;
        hello(&mut new, &[(id(0), ticket.clone())]).await;
        let (first, _) = start_of(&hear(&mut new).await);
        answer_start(&mut new, first, None).await;
        loop {
            match hear(&mut new).await {
                Message::Piece {
                    stream, last: true, ..
                } => receipt(&mut new, stream, true).await,
                Message::Piece { .. } => {}
                Message::EndFile {
                    stream,
                    changed: true,
                    ..
                } => {
                    done(&mut new, stream, false).await;
                    // Now the lane goes, its blocks still unanswered.
                    ended.notify_one();
                }
                Message::StartFile {
                    stream,
                    resumed_done,
                    ..
                } => return (first, stream, resumed_done),
                other => panic!("unexpected {other:?}"),
            }
        }
    };
    let lane_side = async {
        let mut streams = BTreeSet::new();
        loop {
            tokio::select! {
                got = new_lane.recv() => match got.map(|d| Message::decode(&d)) {
                    Ok(Ok(Message::Piece { stream, .. })) => {
                        streams.insert(stream);
                    }
                    _ => break,
                },
                () = ended.notified() => break,
            }
        }
        drop(new_lane);
        streams
    };
    let ((first, again, resumed_done), on_the_lane, _) =
        tokio::time::timeout(Duration::from_secs(60), async {
            tokio::join!(
                main_side,
                lane_side,
                sender.run_lanes(&mut main, vec![lane])
            )
        })
        .await
        .expect("hung");
    assert!(
        !on_the_lane.is_empty(),
        "the lane carried blocks of the first attempt"
    );
    assert_eq!(on_the_lane, BTreeSet::from([first]));
    assert_eq!(resumed_done, 0, "the retry starts again whole");
    assert_ne!(again, first, "the retry has a new stream");
}

/// An old laptop whose drive reads the file differently each time it is opened (its stamp never
/// changes): the first byte comes back different on every open.
struct Unsteady(std::sync::Arc<std::sync::atomic::AtomicU8>);

struct UnsteadyFile {
    file: std::fs::File,
    flip: u8,
    at: u64,
}

impl std::io::Read for UnsteadyFile {
    fn read(&mut self, buf: &mut [u8]) -> std::io::Result<usize> {
        let n = self.file.read(buf)?;
        if self.at == 0 && n > 0 {
            buf[0] ^= self.flip;
        }
        self.at += n as u64;
        Ok(n)
    }
}

impl std::io::Seek for UnsteadyFile {
    fn seek(&mut self, pos: std::io::SeekFrom) -> std::io::Result<u64> {
        self.at = self.file.seek(pos)?;
        Ok(self.at)
    }
}

impl pctwin_transfer::Source for UnsteadyFile {
    fn stamp(&mut self) -> std::io::Result<pctwin_transfer::Stamp> {
        self.file.stamp()
    }

    fn identity(&mut self) -> std::io::Result<Option<pctwin_journal::FileId>> {
        self.file.identity()
    }
}

impl pctwin_transfer::Opener for Unsteady {
    fn open(&self, path: &std::path::Path) -> std::io::Result<Box<dyn pctwin_transfer::Source>> {
        let flip = self
            .0
            .fetch_add(1, std::sync::atomic::Ordering::SeqCst)
            .wrapping_add(1);
        Ok(Box::new(UnsteadyFile {
            file: std::fs::File::open(path)?,
            flip,
            at: 0,
        }))
    }
}

/// Quarantine Design, step 3: the old laptop's drive reads the file differently each time and
/// this laptop's drive keeps the block wrong, so it never comes right. What arrived is kept
/// beside the real name as a damaged copy, never under it, and the file needs action, saying the
/// old drive may be failing.
#[tokio::test]
async fn a_file_the_old_drive_reads_differently_is_set_aside_as_a_damaged_copy() {
    let l = laptops(&[70_000]);
    let table = table(&l, Some(Flip::OnEveryWrite));
    let mut sender = SenderSession::new(jobs(&l), 1).with_opener(Arc::new(Unsteady(
        std::sync::Arc::new(std::sync::atomic::AtomicU8::new(0)),
    )));
    let mut receiver = ReceiverSession::new(&table, plan(&l), common::journal(), "1001");
    assert_eq!(move_all(&mut sender, &mut receiver).await, (true, true));
    assert!(matches!(
        sender.outcome(id(0)),
        Some(SendOutcome::Failed(_))
    ));
    let Some(ReceiveOutcome::NeedsAction { finished, why }) = receiver.outcome(id(0)) else {
        panic!(
            "expected a damaged copy kept, got {:?}",
            receiver.outcome(id(0))
        );
    };
    assert!(why.contains("read differently"), "{why}");
    assert_eq!(
        finished.final_path,
        "Documents/f0.bin (damaged copy).pctwin-damaged"
    );
    assert!(l.new.path().join(&finished.final_path).exists());
    assert!(!landed(&l, 0).exists(), "never under the real name");
}

/// Undo takes the damaged copy away too, like any copy the move made, once the old laptop says
/// the original is still there unchanged; a copy the person changed since is kept.
#[tokio::test]
async fn undo_removes_a_damaged_copy_but_keeps_one_changed_since() {
    for changed in [false, true] {
        let l = laptops(&[70_000]);
        let table = table(&l, Some(Flip::OnEveryWrite));
        let journal = common::journal();
        let mut sender = SenderSession::new(jobs(&l), 1).with_opener(Arc::new(Unsteady(
            std::sync::Arc::new(std::sync::atomic::AtomicU8::new(0)),
        )));
        let mut receiver = ReceiverSession::new(&table, plan(&l), journal, "1001");
        assert_eq!(move_all(&mut sender, &mut receiver).await, (true, true));
        let Some(ReceiveOutcome::NeedsAction { finished, .. }) = receiver.outcome(id(0)) else {
            panic!("expected a damaged copy kept");
        };
        let kept = l.new.path().join(&finished.final_path);
        assert!(kept.exists());
        if changed {
            std::fs::write(&kept, b"the person's own words now").unwrap();
        }
        let items = pctwin_transfer::undo_items(journal).unwrap();
        assert_eq!(
            items,
            vec![id(0)],
            "undo asks about the damaged copy's original"
        );
        let sent: HashMap<ItemId, PathBuf> =
            jobs(&l).into_iter().map(|j| (j.item, j.source)).collect();
        let (mut new, mut old) = mem_pair();
        let serve = async {
            let mut budget = pctwin_transfer::OriginalsBudget::new();
            pctwin_transfer::serve_originals(&mut old, &sent, &mut budget)
                .await
                .unwrap();
        };
        let (originals, ()) = common::no_hang(async {
            tokio::join!(
                pctwin_transfer::check_originals(&mut new, journal, &items),
                serve
            )
        })
        .await;
        let report = pctwin_transfer::undo(journal, &table, originals.unwrap()).unwrap();
        assert_eq!(report.files.len(), 1, "{report:?}");
        if changed {
            assert!(
                matches!(
                    report.files[0].outcome,
                    pctwin_journal::UndoOutcome::Kept { .. }
                ),
                "{report:?}"
            );
            assert!(kept.exists(), "the person's changes are kept");
        } else {
            // Gone from where it was: deleted, or on a Mac put in the Trash (and seen there).
            match &report.files[0].outcome {
                pctwin_journal::UndoOutcome::Deleted => {}
                pctwin_journal::UndoOutcome::Trashed { at } if cfg!(target_os = "macos") => {
                    assert!(std::path::Path::new(at).exists(), "{report:?}");
                }
                other => panic!("{other:?}: {report:?}"),
            }
            assert!(!kept.exists(), "the damaged copy is gone");
        }
    }
}

/// Quarantine Design, step 4: a drive that could not store 3 of its last checked files pauses the
/// move and names the place, so the person can choose another or stop; it never runs on through
/// every file.
#[tokio::test]
async fn a_drive_that_keeps_failing_pauses_the_move_and_says_which() {
    let l = laptops(&[70_000; 6]);
    let table = table(&l, Some(Flip::OnEveryWrite));
    let control = pctwin_transfer::MoveControl::new();
    let mut sender = SenderSession::new(jobs(&l), 1);
    let mut receiver = ReceiverSession::new(&table, plan(&l), common::journal(), "1001")
        .with_control(control.clone());
    let (mut a, mut b) = mem_pair();
    let person = async {
        while control.braked().is_none() {
            tokio::time::sleep(std::time::Duration::from_millis(5)).await;
        }
        // The person reads why it paused, and stops.
        control.stop();
    };
    let (_, received, ()) = tokio::time::timeout(std::time::Duration::from_secs(120), async {
        tokio::join!(sender.run(&mut a), receiver.run(&mut b), person)
    })
    .await
    .expect("the move hung");
    // Ended by the person's stop.
    assert!(received.is_err());
    assert_eq!(control.braked().as_deref(), Some("me"));
    let failed = (0..6u8)
        .filter(|n| matches!(receiver.outcome(id(*n)), Some(ReceiveOutcome::Failed(why)) if why.contains("could not store")))
        .count();
    assert_eq!(
        failed, 3,
        "paused at the third, never ran on through every file"
    );
}

/// Quarantine Design, step 4, with many files on their way at once: every file is sent and
/// checked side by side, so when the third of them fails and the move pauses, the ends of the
/// others are already on the wire. None of them is checked, named or written any more at that
/// place: each is set down partway, unfinished (never shown as arrived), to be picked up from
/// there once the person carries on.
#[tokio::test]
async fn a_drive_that_pauses_the_move_takes_nothing_more_of_the_files_on_their_way() {
    const FILES: usize = 12;
    // One block each, and several (blocks of them on their way too).
    for size in [70_000, 600_000] {
        let l = laptops(&[size; FILES]);
        let table = table(&l, Some(Flip::OnEveryWrite));
        let control = pctwin_transfer::MoveControl::new();
        let mut sender = SenderSession::new(jobs(&l), FILES);
        let mut receiver = ReceiverSession::new(&table, plan(&l), common::journal(), "1001")
            .with_control(control.clone());
        let written = receiver.meter();
        let (mut a, mut b) = mem_pair();
        let person = async {
            while control.braked().is_none() {
                tokio::time::sleep(std::time::Duration::from_millis(5)).await;
            }
            let at_pause = written.load(std::sync::atomic::Ordering::SeqCst);
            // Long enough for everything already on its way to arrive and be answered.
            tokio::time::sleep(std::time::Duration::from_millis(300)).await;
            assert_eq!(
                written.load(std::sync::atomic::Ordering::SeqCst),
                at_pause,
                "written there after the pause"
            );
            control.stop();
        };
        let (_, received, ()) = tokio::time::timeout(std::time::Duration::from_secs(120), async {
            tokio::join!(sender.run(&mut a), receiver.run(&mut b), person)
        })
        .await
        .expect("the move hung");
        assert_eq!(control.braked().as_deref(), Some("me"));
        let mut failed = 0;
        for n in 0..FILES {
            match receiver.outcome(id(n as u8)) {
                Some(ReceiveOutcome::Failed(why)) if why.contains("could not store") => failed += 1,
                None => {}
                other => panic!("file {n} ended after the pause: {other:?}"),
            }
            if !matches!(
                receiver.outcome(id(n as u8)),
                Some(ReceiveOutcome::Finished(_))
            ) {
                assert!(!landed(&l, n).exists(), "file {n} was named all the same");
            }
        }
        assert_eq!(failed, 3, "paused at the third, nothing more ended there");
        // Ended by the person's stop, never run on to the end.
        assert!(received.is_err());
        // A file set down and never picked up again is never counted as arrived, on either
        // laptop: the new laptop has no ending for it (the report says it did not arrive), and
        // the old laptop never heard it arrived.
        for n in 0..FILES {
            if receiver.outcome(id(n as u8)).is_none() {
                assert!(
                    !matches!(
                        sender.outcome(id(n as u8)),
                        Some(SendOutcome::Arrived | SendOutcome::AlreadyThere)
                    ),
                    "file {n}: {:?}",
                    sender.outcome(id(n as u8))
                );
            }
        }
    }
}

/// The same, carrying on each time the drive pauses the move: every file set down is picked up
/// where it was and ends for its own reason, and the move ends (never stuck on a file set down).
#[tokio::test]
async fn files_set_down_by_a_pause_are_picked_up_when_the_person_carries_on() {
    const FILES: usize = 12;
    let l = laptops(&[70_000; FILES]);
    let table = table(&l, Some(Flip::OnEveryWrite));
    let control = pctwin_transfer::MoveControl::new();
    let mut sender = SenderSession::new(jobs(&l), FILES);
    let mut receiver = ReceiverSession::new(&table, plan(&l), common::journal(), "1001")
        .with_control(control.clone());
    let done = std::cell::Cell::new(false);
    let ((s, r), brakes) = {
        let (mut a, mut b) = mem_pair();
        tokio::time::timeout(std::time::Duration::from_secs(120), async {
            tokio::join!(
                async {
                    let ran = tokio::join!(sender.run(&mut a), receiver.run(&mut b));
                    done.set(true);
                    ran
                },
                carry_on_whenever_braked(&control, &done)
            )
        })
        .await
        .expect("hung")
    };
    s.unwrap();
    r.unwrap();
    assert!(brakes >= 1, "the drive never paused the move");
    for n in 0..FILES {
        match receiver.outcome(id(n as u8)) {
            Some(ReceiveOutcome::Failed(why)) => {
                assert!(why.contains("could not store"), "file {n}: {why}");
            }
            other => panic!("file {n}: {other:?}"),
        }
    }
}

/// Blocks a drop cut off are kept and sent again without reading the drive again: on a weak
/// drive every block is read once, however often the connection drops.
#[tokio::test]
async fn blocks_cut_off_by_a_drop_are_never_read_twice_on_a_weak_drive() {
    let size = 8 * MIB;
    let mut f = Flaky::new(&[size], (Cut::Clean, Cut::Clean));
    f.cuts = vec![
        (Cut::LoseAfter(3), Cut::Clean),
        (Cut::LoseAfter(2), Cut::Clean),
    ];
    f.plan = ReadPlan::Careful {
        most_important_first: true,
        read_each_file_once: true,
        stop_after_read_errors: 5,
    };
    let m = flaky(f).await;
    m.assert_arrived(false);
    assert!(m.losses() >= 2, "the connection dropped twice");
    assert_eq!(
        m.reads.blocks(),
        size,
        "every byte read from the drive once"
    );
}

/// "Done" from the new laptop for a file still being sent, before its end was said, is not
/// evidence it arrived: never recorded as arrived.
#[tokio::test]
async fn a_done_before_the_file_was_ended_is_not_taken_as_arrived() {
    let l = laptops(&[300_000]);
    let mut sender = SenderSession::new(jobs(&l), 1);
    let ((), _) = connection(&mut sender, async |new: &mut Mem| {
        hello(new, &[]).await;
        let (stream, _) = start_of(&hear(new).await);
        answer_start(new, stream, None).await;
        done(new, stream, true).await;
    })
    .await;
    assert_ne!(sender.outcome(id(0)), Some(&SendOutcome::Arrived));
}

/// A new laptop that misbehaves on purpose (PR 52 residual): on every connection it earns the
/// window with receipts, then stops answering and drops, over and over, on the main connection
/// and on lanes alike. Every connection's lost reads stay within the floor and a block per loss
/// on top of what was written, and the whole move within its answered-reads bound.
#[tokio::test]
async fn a_new_laptop_that_earns_the_window_then_drops_it_stays_within_the_bounds() {
    for sizes in [vec![8 * MIB], vec![4 * MIB; 3]] {
        let mut f = Flaky::new(
            &sizes,
            (Cut::ReceiptsLostAfter(6), Cut::ReceiptsLostAfter(3)),
        );
        f.lanes = 2;
        // Earning a little more each time, so the window is grown before it is dropped.
        f.cuts = (1..8)
            .map(|k| (Cut::ReceiptsLostAfter(2 * k), Cut::ReceiptsLostAfter(k)))
            .collect();
        let m = flaky(f).await;
        m.assert_arrived(false);
        assert!(m.losses() >= 1, "it dropped: {:?}", m.conns);
        m.assert_lost_reads_bounded(FLOOR, BLOCK, WINDOW);
        m.assert_reads_bounded(FLOOR, BLOCK, WINDOW);
    }
}

/// What the new laptop says next, past any word of its pause.
async fn hear_past_holds(ch: &mut Mem) -> Message {
    loop {
        match hear(ch).await {
            Message::Hold { .. } => continue,
            m => return m,
        }
    }
}

/// Every file left under `dir`, at any depth.
fn files_under(dir: &Path) -> Vec<PathBuf> {
    let mut found = Vec::new();
    let mut todo = vec![dir.to_path_buf()];
    while let Some(d) = todo.pop() {
        for e in std::fs::read_dir(&d).into_iter().flatten().flatten() {
            let p = e.path();
            if p.is_dir() {
                todo.push(p);
            } else {
                found.push(p);
            }
        }
    }
    found
}

/// Brake follow-up (23 October 2026): a file the old laptop gives up on while its place's drive
/// holds the move paused keeps its part-copy as it is there, since nothing more is written to that
/// drive until the person carries on; it is cleared once they do.
#[tokio::test]
async fn a_file_given_up_at_a_paused_place_is_not_cleared_there_until_the_person_carries_on() {
    let l = laptops(&[70_000; 4]);
    let table = table(&l, Some(Flip::OnEveryWrite));
    let control = pctwin_transfer::MoveControl::new();
    let mut receiver = ReceiverSession::new(&table, plan(&l), common::journal(), "1001")
        .with_control(control.clone());
    let (mut old, mut new) = mem_pair();
    let start = |stream: u32, n: usize, header: Header| Message::StartFile {
        stream,
        item: id(n as u8),
        destination: "me".into(),
        path: format!("Documents/f{n}.bin"),
        header,
        resumed_done: 0,
    };
    let script = async {
        assert_eq!(hear(&mut old).await, Message::Ready);
        // File 3 starts first and is part way when the drive pauses the move.
        let mut late = FileSender::open(&l.files[3].0, None, false).unwrap();
        say(&mut old, &start(9, 3, late.header().clone())).await;
        assert!(matches!(hear(&mut old).await, Message::Have { .. }));
        for piece in split_into_pieces(9, &late.block_at(0).unwrap().encode()) {
            say(&mut old, &piece).await;
        }
        assert!(matches!(hear(&mut old).await, Message::Receipt { .. }));
        brake_me(&mut old, &l).await;
        assert_eq!(control.braked().as_deref(), Some("me"));
        let before = files_under(l.new.path());
        assert_eq!(before.len(), 1, "file 3's part-copy: {before:?}");
        // The old laptop gives up on file 3 while the move is paused: its part-copy stays.
        say(
            &mut old,
            &Message::GiveUp {
                stream: 9,
                why: GiveUpWhy::ReadTooOften,
            },
        )
        .await;
        assert_eq!(
            hear_past_holds(&mut old).await,
            Message::FileDone {
                stream: 9,
                ok: false
            }
        );
        assert_eq!(files_under(l.new.path()), before, "nothing changed there");
        control.carry_on();
        say(&mut old, &Message::AllSent).await;
    };
    let ((), r) = tokio::time::timeout(std::time::Duration::from_secs(30), async {
        tokio::join!(script, receiver.run(&mut new))
    })
    .await
    .expect("hung");
    r.unwrap();
    assert!(matches!(
        receiver.outcome(id(3)),
        Some(ReceiveOutcome::Failed(_))
    ));
    assert_eq!(
        files_under(l.new.path()),
        Vec::<PathBuf>::new(),
        "cleared once carried on"
    );
}

/// The test plays the old laptop: three files the drive of "me" stores wrong, each sent whole,
/// asked for again and given up on (streams 0 to 2), so the drive pauses the move.
async fn brake_me(old: &mut Mem, l: &Laptops) {
    for n in 0..3 {
        let stream = n as u32;
        let mut fs = FileSender::open(&l.files[n].0, None, false).unwrap();
        say(
            old,
            &Message::StartFile {
                stream,
                item: id(n as u8),
                destination: "me".into(),
                path: format!("Documents/f{n}.bin"),
                header: fs.header().clone(),
                resumed_done: 0,
            },
        )
        .await;
        assert!(matches!(hear_past_holds(old).await, Message::Have { .. }));
        for b in 0..fs.header().block_count {
            for piece in split_into_pieces(stream, &fs.block_at(b).unwrap().encode()) {
                say(old, &piece).await;
            }
            assert!(matches!(
                hear_past_holds(old).await,
                Message::Receipt { .. }
            ));
        }
        let stamp = fs.header().stamp;
        say(
            old,
            &Message::EndFile {
                stream,
                stamp_after: stamp,
                changed: false,
            },
        )
        .await;
        let asked = hear_past_holds(old).await;
        assert!(matches!(asked, Message::SendAgain { .. }), "{asked:?}");
        say(
            old,
            &Message::GiveUp {
                stream,
                why: GiveUpWhy::AskedAgainTooOften,
            },
        )
        .await;
        assert_eq!(
            hear_past_holds(old).await,
            Message::FileDone { stream, ok: false }
        );
    }
}

/// Brake follow-up (23 October 2026): an old laptop on move protocol 3 cannot be told of a pause
/// (no `Hold`, no setting down), so a file it starts at the paused place is refused before anything
/// is written there, and ends failed with that true reason and what to do, never "not checked".
#[tokio::test]
async fn an_older_old_laptop_is_told_no_for_a_paused_place_with_the_true_reason() {
    let l = laptops(&[70_000; 4]);
    let table = table(&l, Some(Flip::OnEveryWrite));
    let control = pctwin_transfer::MoveControl::new();
    let mut receiver = ReceiverSession::new(&table, plan(&l), common::journal(), "1001")
        .with_control(control.clone())
        .with_protocol(3);
    let (mut old, mut new) = mem_pair();
    let script = async {
        assert_eq!(hear(&mut old).await, Message::Ready);
        brake_me(&mut old, &l).await;
        assert_eq!(control.braked().as_deref(), Some("me"));
        let fs = FileSender::open(&l.files[3].0, None, false).unwrap();
        say(
            &mut old,
            &Message::StartFile {
                stream: 3,
                item: id(3),
                destination: "me".into(),
                path: "Documents/f3.bin".into(),
                header: fs.header().clone(),
                resumed_done: 0,
            },
        )
        .await;
        assert_eq!(
            hear(&mut old).await,
            Message::FileDone {
                stream: 3,
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
    match receiver.outcome(id(3)) {
        Some(ReceiveOutcome::Failed(why)) => {
            assert!(why.contains("paused the move"), "{why}");
            assert!(why.contains("again"), "{why}");
        }
        other => panic!("{other:?}"),
    }
    assert!(!landed(&l, 3).exists());
}

/// Exit-gate review (24 October 2026): a folder that moves as one whole is never written into a
/// folder someone else made, even one put there after this laptop placed the whole: the file is
/// refused before anything is made inside it, with why. Its own folder, made by its first file,
/// takes the rest.
#[tokio::test]
async fn a_whole_is_never_written_into_a_folder_planted_after_it_was_placed() {
    for planted in [true, false] {
        let l = laptops(&[70_000, 70_000]);
        let table = table(&l, None);
        let plan = plan(&l).with_wholes(vec![pctwin_transfer::WholePlace {
            destination: "me".into(),
            planned: "Documents".into(),
            stored: Some("Documents".into()),
        }]);
        if planted {
            std::fs::create_dir_all(l.new.path().join("Documents")).unwrap();
        }
        let mut sender = SenderSession::new(jobs(&l), 1);
        let mut receiver = ReceiverSession::new(&table, plan, common::journal(), "1001");
        move_all(&mut sender, &mut receiver).await;
        for n in 0..2 {
            let got = receiver.outcome(id(n as u8));
            if planted {
                match got {
                    Some(ReceiveOutcome::Failed(why)) => {
                        assert!(why.contains("already there"), "{why}")
                    }
                    other => panic!("file {n}: {other:?}"),
                }
            } else {
                assert!(matches!(got, Some(ReceiveOutcome::Finished(_))), "{got:?}");
                assert_eq!(std::fs::read(landed(&l, n)).unwrap(), l.files[n].1);
            }
        }
        if planted {
            assert_eq!(
                std::fs::read_dir(l.new.path().join("Documents"))
                    .unwrap()
                    .count(),
                0,
                "nothing was made in the folder already there"
            );
        }
    }
}

/// Fresh-context review of TRANSFER (24 October 2026): an old laptop that starts the same file
/// over and over without sending a block is refused once the file's starts run out, so it cannot
/// make the new laptop write a journal entry and a temporary file for ever.
#[tokio::test]
async fn a_file_started_over_and_over_without_blocks_is_refused_in_the_end() {
    let l = laptops(&[70_000]);
    let table = table(&l, None);
    let dir = tempfile::tempdir().unwrap();
    let journal = pctwin_journal::Journal::open(&dir.path().join("journal.redb")).unwrap();
    let mut receiver = ReceiverSession::new(&table, plan(&l), &journal, "1001");
    let (mut old, mut new) = mem_pair();
    let header = FileSender::open(&l.files[0].0, None, false)
        .unwrap()
        .header()
        .clone();
    let script = async {
        assert_eq!(hear(&mut old).await, Message::Ready);
        for n in 0..=pctwin_transfer::MAX_STARTS_PER_FILE {
            say(
                &mut old,
                &Message::StartFile {
                    stream: 0,
                    item: id(0),
                    destination: "me".into(),
                    path: "Documents/f0.bin".into(),
                    header: header.clone(),
                    resumed_done: 0,
                },
            )
            .await;
            let heard = hear_past_holds(&mut old).await;
            if n < pctwin_transfer::MAX_STARTS_PER_FILE {
                assert!(matches!(heard, Message::Have { .. }), "{n}: {heard:?}");
            } else {
                assert_eq!(
                    heard,
                    Message::FileDone {
                        stream: 0,
                        ok: false
                    }
                );
            }
        }
        say(&mut old, &Message::AllSent).await;
    };
    let ((), r) = tokio::time::timeout(std::time::Duration::from_secs(60), async {
        tokio::join!(script, receiver.run(&mut new))
    })
    .await
    .expect("hung");
    r.unwrap();
    match receiver.outcome(id(0)) {
        Some(ReceiveOutcome::Failed(why)) => {
            assert_eq!(why.cause(), Cause::RefusedBySafetyCheck, "{why}");
        }
        other => panic!("{other:?}"),
    }
    let entries = journal.entries().unwrap();
    assert!(
        entries.len() <= pctwin_transfer::MAX_STARTS_PER_FILE as usize,
        "{} journal entries",
        entries.len()
    );
    assert_eq!(files_under(l.new.path()), Vec::<PathBuf>::new());
}

/// Fresh-context review of TRANSFER (24 October 2026): a part-copy kept at a paused place (see
/// above) is still an open file here, so it counts toward [`MAX_OPEN_FILES`]: with one kept, the
/// old laptop can open one file fewer elsewhere.
#[tokio::test]
async fn a_part_copy_kept_at_a_paused_place_counts_as_an_open_file() {
    let l = laptops(&[70_000; 4]);
    let elsewhere = tempfile::tempdir_in(env!("CARGO_TARGET_TMPDIR")).unwrap();
    let tiny = l._old.path().join("tiny.bin");
    std::fs::write(&tiny, b"tiny").unwrap();
    let mut table = table(&l, Some(Flip::OnEveryWrite));
    table
        .approve("other", Approved::MyFolders, elsewhere.path())
        .unwrap();
    let mut jobs = jobs(&l);
    let mut sizes: Vec<u64> = l.files.iter().map(|f| f.1.len() as u64).collect();
    for k in 0..MAX_OPEN_FILES {
        jobs.push(SendJob {
            size: 0,
            item: id(10 + k as u8),
            source: tiny.clone(),
            destination: "other".into(),
            path: format!("Documents/g{k}.bin"),
            compressible: false,
            tier: Tier::Rest,
        });
        sizes.push(4);
    }
    let files: Vec<(&SendJob, u64)> = jobs.iter().zip(sizes).collect();
    let control = pctwin_transfer::MoveControl::new();
    let mut receiver = ReceiverSession::new(
        &table,
        common::approved_jobs(&files),
        common::journal(),
        "1001",
    )
    .with_control(control.clone());
    let (mut old, mut new) = mem_pair();
    let tiny_header = FileSender::open(&tiny, None, false)
        .unwrap()
        .header()
        .clone();
    let script = async {
        assert_eq!(hear(&mut old).await, Message::Ready);
        // File 3 is part way when the drive of "me" pauses the move, and is given up on there.
        let mut late = FileSender::open(&l.files[3].0, None, false).unwrap();
        say(
            &mut old,
            &Message::StartFile {
                stream: 9,
                item: id(3),
                destination: "me".into(),
                path: "Documents/f3.bin".into(),
                header: late.header().clone(),
                resumed_done: 0,
            },
        )
        .await;
        assert!(matches!(hear(&mut old).await, Message::Have { .. }));
        for piece in split_into_pieces(9, &late.block_at(0).unwrap().encode()) {
            say(&mut old, &piece).await;
        }
        assert!(matches!(hear(&mut old).await, Message::Receipt { .. }));
        brake_me(&mut old, &l).await;
        say(
            &mut old,
            &Message::GiveUp {
                stream: 9,
                why: GiveUpWhy::ReadTooOften,
            },
        )
        .await;
        assert_eq!(
            hear_past_holds(&mut old).await,
            Message::FileDone {
                stream: 9,
                ok: false
            }
        );
        // Its part-copy is kept, so only MAX_OPEN_FILES - 1 more open elsewhere.
        for k in 0..MAX_OPEN_FILES {
            let stream = 100 + k as u32;
            say(
                &mut old,
                &Message::StartFile {
                    stream,
                    item: id(10 + k as u8),
                    destination: "other".into(),
                    path: format!("Documents/g{k}.bin"),
                    header: tiny_header.clone(),
                    resumed_done: 0,
                },
            )
            .await;
            let heard = hear_past_holds(&mut old).await;
            if k + 1 < MAX_OPEN_FILES {
                assert!(matches!(heard, Message::Have { .. }), "{k}: {heard:?}");
            } else {
                assert_eq!(heard, Message::FileDone { stream, ok: false });
            }
        }
        control.carry_on();
        say(&mut old, &Message::AllSent).await;
    };
    let ((), r) = tokio::time::timeout(std::time::Duration::from_secs(60), async {
        tokio::join!(script, receiver.run(&mut new))
    })
    .await
    .expect("hung");
    r.unwrap();
    match receiver.outcome(id(10 + (MAX_OPEN_FILES - 1) as u8)) {
        Some(ReceiveOutcome::Failed(why)) => assert!(why.contains("too many files"), "{why}"),
        other => panic!("{other:?}"),
    }
}
