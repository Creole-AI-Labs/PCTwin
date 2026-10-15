//! Sending again (quarantine design, step 2): a block that arrives damaged is answered "not
//! written"; a file whose blocks are missing or read back wrong when it ends is asked for again,
//! only those blocks, at most twice, and named only once a read-back matches. A file that changed
//! on the old laptop meanwhile is sent again whole, as before. A drive that keeps storing it wrong
//! fails the file after the last round, and nothing of it is left behind.
// Tests make and remove files to test on; the no-file-changes fence is for the shipped code.
#![allow(clippy::disallowed_methods)]

use std::path::PathBuf;

use pctwin_gate::{Approved, Destinations, FaultPlan, Flip};
use pctwin_record::ItemId;
use pctwin_transfer::{
    BlockMap, Channel, ChannelError, FileSender, Message, ReceiveOutcome, ReceiverSession,
    ResumeTicket, SendJob, SendOutcome, SenderSession, Stamp, Tier, block_size_for,
    split_into_pieces,
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
    let old = tempfile::tempdir().unwrap();
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
        new: tempfile::tempdir().unwrap(),
        files,
    }
}

fn jobs(l: &Laptops) -> Vec<SendJob> {
    (0..l.files.len())
        .map(|n| SendJob {
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
    assert_eq!(pctwin_transfer::MOVE_PROTOCOL.newest(), 2);
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
        // A third time: nothing more is read or sent, and the file ends.
        say(&mut new, &ask).await;
        let (got, end) = take_blocks_until_end(&mut new).await;
        assert_eq!(got, 0);
        assert!(
            matches!(end, Message::EndFile { changed: true, .. }),
            "{end:?}"
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
    assert!(matches!(
        sender.outcome(id(0)),
        Some(SendOutcome::Failed(_))
    ));
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
    seen: std::collections::HashSet<u32>,
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

#[tokio::test]
async fn on_a_drive_that_reads_every_file_back_wrong_each_file_fails_for_that_reason() {
    const FILES: usize = 80;
    let l = laptops(&vec![10_000usize; FILES]);
    let table = table(&l, Some(Flip::OnRead { at: 0 }));
    let mut sender = SenderSession::new(jobs(&l), 1);
    let mut receiver = ReceiverSession::new(&table, plan(&l), common::journal(), "1001");
    let (s, r) = {
        let (mut a, mut b) = mem_pair();
        tokio::time::timeout(std::time::Duration::from_secs(120), async {
            tokio::join!(sender.run(&mut a), receiver.run(&mut b))
        })
        .await
        .expect("hung")
    };
    s.unwrap();
    r.unwrap();
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
