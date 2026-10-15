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
    BlockMap, Channel, ChannelError, FileSender, GiveUpWhy, Message, ReceiveOutcome,
    ReceiverSession, ResumeTicket, Scheduler, SendJob, SendOutcome, SenderSession, Stamp, Tier,
    block_size_for, split_into_pieces,
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

/// The old laptop's side of an honest move where the person saves the file during a resend: the
/// first block of every attempt is damaged on the way; on the first attempt the file is saved
/// just before the block is sent again; on later attempts the block sent again is damaged once
/// more, so each attempt needs both of its rounds.
struct SavedDuringResend {
    inner: Mem,
    first: Option<u32>,
    sends: std::collections::BTreeMap<u32, u32>,
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
    let jdir = tempfile::tempdir().unwrap();
    let journal = pctwin_journal::Journal::open(&jdir.path().join("j.redb")).unwrap();
    // Every file reads back wrong from this drive.
    let table = table(&l, Some(Flip::OnRead { at: 0 }));
    let mut sender = SenderSession::new(jobs(&l), 1);
    {
        let mut receiver = ReceiverSession::new(&table, plan(&l), &journal, "1001");
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
    let mut receiver = ReceiverSession::new(&table, plan(&l), &journal, "1001");
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

#[tokio::test]
async fn reconnecting_with_a_resume_ticket_over_and_over_cannot_make_the_old_laptop_read_without_end()
 {
    let l = laptops(&[300_000]);
    let header = FileSender::open(&l.files[0].0, None, false)
        .unwrap()
        .header()
        .clone();
    let count = header.block_count;
    let mut sender = SenderSession::new(jobs(&l), 1);
    let mut stopped = false;
    for _ in 0..60 {
        let (mut old, mut new) = mem_pair();
        let header = header.clone();
        // Each connection: "I have none of it, carry on", take every block, then go away (the
        // connection ends with this script).
        let script = async move {
            say(
                &mut new,
                &Message::ResumeFrom {
                    stream: 0,
                    item: id(0),
                    ticket: ResumeTicket {
                        done: BlockMap::new(count),
                        block_size: header.block_size,
                        stamp: header.stamp,
                    },
                },
            )
            .await;
            say(&mut new, &Message::Ready).await;
            loop {
                match hear(&mut new).await {
                    Message::StartFile { .. } => {
                        say(
                            &mut new,
                            &Message::Have {
                                stream: 0,
                                same_size: None,
                            },
                        )
                        .await;
                    }
                    Message::Piece {
                        stream, last: true, ..
                    } => {
                        say(
                            &mut new,
                            &Message::Receipt {
                                stream,
                                block: 0,
                                written: true,
                            },
                        )
                        .await;
                    }
                    Message::Piece { .. } => {}
                    Message::GiveUp { why, .. } => return Some(why),
                    // The file's end, never answered: the connection goes.
                    _ => return None,
                }
            }
        };
        let (got, _) = tokio::time::timeout(std::time::Duration::from_secs(30), async {
            tokio::join!(script, sender.run(&mut old))
        })
        .await
        .expect("hung");
        if got == Some(GiveUpWhy::ReadTooOften) {
            stopped = true;
            break;
        }
    }
    assert!(stopped, "the old laptop never stopped reading the file");
    // Never more than the budget: 16 times the file (as if 1 MiB), plus a block.
    let budget_blocks = (16 * (1u64 << 20).max(header.size)).div_ceil(header.block_size);
    assert!(
        sender.blocks_sent() <= budget_blocks,
        "{} blocks read, budget {budget_blocks}",
        sender.blocks_sent()
    );
    match sender.outcome(id(0)) {
        Some(SendOutcome::Failed(why)) => assert!(why.contains("many more times"), "{why}"),
        None => {}
        other => panic!("{other:?}"),
    }
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
    let jdir = tempfile::tempdir().unwrap();
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
