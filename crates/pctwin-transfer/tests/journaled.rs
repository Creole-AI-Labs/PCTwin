//! The new laptop records every write in the change journal as it happens (Task List 1.6): an
//! entry is planned before anything is created, staged once its temporary file exists, verified
//! only after every byte arrived, the original did not change while it was read and the file is on
//! disk, applied with its real name before it gets it, and committed with what it landed as. A
//! failure is recorded with its plain reason; an identical file already there is recorded as
//! existing, never as written; and if the journal cannot be written, nothing more is written.

use std::path::PathBuf;

use pctwin_gate::{Approved, Destinations};
use pctwin_journal::{
    FileId, Journal, JournalError, Landed, Ledger, Permission, PlannedWrite, State,
};
use pctwin_record::{ItemId, LaptopId};
use pctwin_transfer::{
    Channel, ChannelError, ReceiveOutcome, ReceiverSession, SendJob, SenderSession, Tier,
    TransferError, block_size_for, fingerprint_reader,
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
    shared: tempfile::TempDir,
    _jdir: tempfile::TempDir,
    journal: Journal,
    table: Destinations,
}

fn world() -> World {
    let old = tempfile::tempdir().unwrap();
    let mine = tempfile::tempdir().unwrap();
    let shared = tempfile::tempdir().unwrap();
    let jdir = tempfile::tempdir().unwrap();
    let journal = Journal::open(&jdir.path().join("journal.redb")).unwrap();
    let mut table = Destinations::new();
    table
        .approve("me", Approved::MyFolders, mine.path())
        .unwrap();
    table
        .approve("shared", Approved::SharedFolder, shared.path())
        .unwrap();
    World {
        old,
        mine,
        shared,
        _jdir: jdir,
        journal,
        table,
    }
}

impl World {
    /// A file on the old laptop and its send job.
    fn file(&self, n: u8, len: usize, destination: &str, path: &str) -> (SendJob, Vec<u8>) {
        let source = self.old.path().join(format!("f{n}.bin"));
        let data = bytes(len, n);
        std::fs::write(&source, &data).unwrap();
        (
            SendJob {
                item: id(n),
                source,
                destination: destination.into(),
                path: path.into(),
                compressible: false,
                tier: Tier::Rest,
            },
            data,
        )
    }
}

async fn move_all(
    jobs: Vec<SendJob>,
    receiver: &mut ReceiverSession<'_>,
) -> (
    Result<(), TransferError>,
    Result<(), TransferError>,
    SenderSession,
) {
    let mut sender = SenderSession::new(jobs, 2);
    let (a, b) = mem_pair();
    // Each side's end closes when it stops, as a real link does when an app stops.
    let send_side = async {
        let mut a = a;
        sender.run(&mut a).await
    };
    let receive_side = async {
        let mut b = b;
        receiver.run(&mut b).await
    };
    let (sent, received) = tokio::time::timeout(std::time::Duration::from_secs(30), async {
        tokio::join!(send_side, receive_side)
    })
    .await
    .expect("the move hung");
    (sent, received, sender)
}

fn plan_for(jobs: &[(SendJob, Vec<u8>)]) -> pctwin_transfer::Allowance {
    let files: Vec<(ItemId, u64)> = jobs.iter().map(|(j, d)| (j.item, d.len() as u64)).collect();
    common::approved(&files)
}

fn only_entry(j: &Journal) -> pctwin_journal::Entry {
    let all = j.entries().unwrap();
    assert_eq!(all.len(), 1, "{all:?}");
    all.into_iter().next().unwrap()
}

#[tokio::test]
async fn a_received_file_is_recorded_step_by_step_and_committed_with_what_it_landed_as() {
    let w = world();
    let (job, data) = w.file(1, 300_000, "me", "Documents/report.pdf");
    let src_modified = std::fs::metadata(&job.source).unwrap().modified().unwrap();
    let mut receiver = ReceiverSession::new(
        &w.table,
        plan_for(&[(job.clone(), data.clone())]),
        &w.journal,
        "1001",
    );
    let (sent, received, _) = move_all(vec![job], &mut receiver).await;
    assert!(sent.is_ok() && received.is_ok());
    let e = only_entry(&w.journal);
    assert_eq!(e.write.item, id(1));
    assert_eq!(
        e.write.source_laptop,
        LaptopId::from_hex("00112233445566778899aabbccddeeff").unwrap()
    );
    assert_eq!(e.write.destination, "me");
    assert_eq!(e.write.path, "Documents/report.pdf");
    assert_eq!(e.write.size, data.len() as u64);
    assert_eq!(e.write.block_size, block_size_for(data.len() as u64));
    assert_eq!(e.write.actor.acting_account, "1001");
    assert_eq!(e.write.actor.for_account, "1001");
    assert_eq!(e.write.actor.permission, Permission::OwnFolders);
    let src_ns = src_modified
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap()
        .as_nanos() as i64;
    assert_eq!(e.write.source_modified_ns, Some(src_ns));
    let root = w
        .table
        .get("me")
        .unwrap()
        .folder_identity("")
        .unwrap()
        .unwrap();
    assert_eq!(
        e.write.place,
        Some(FileId {
            volume: root.volume,
            index: root.index
        })
    );
    let State::Committed {
        final_path,
        fingerprint,
        landed,
    } = e.state
    else {
        panic!("not committed: {:?}", e.state)
    };
    assert_eq!(final_path, "Documents/report.pdf");
    let on_disk = std::fs::read(w.mine.path().join("Documents/report.pdf")).unwrap();
    assert_eq!(on_disk, data);
    assert_eq!(
        Some(fingerprint),
        fingerprint_reader(
            &mut &on_disk[..],
            data.len() as u64,
            block_size_for(data.len() as u64)
        )
        .unwrap()
    );
    let stat = w
        .table
        .get("me")
        .unwrap()
        .stat("Documents/report.pdf")
        .unwrap()
        .unwrap();
    assert_eq!(
        landed,
        Landed {
            size: data.len() as u64,
            modified_ns: Some(src_ns),
            file: Some(FileId {
                volume: stat.id.volume,
                index: stat.id.index
            }),
        }
    );
    assert!(w.journal.unfinished().unwrap().is_empty());
    // Nothing but the file itself is left behind.
    assert_eq!(
        std::fs::read_dir(w.mine.path().join("Documents"))
            .unwrap()
            .count(),
        1
    );
}

#[tokio::test]
async fn the_shared_folder_is_recorded_as_written_for_everyone() {
    let w = world();
    let (job, data) = w.file(2, 1000, "shared", "Public/notes.txt");
    let mut receiver = ReceiverSession::new(
        &w.table,
        plan_for(&[(job.clone(), data)]),
        &w.journal,
        "1001",
    );
    move_all(vec![job], &mut receiver).await.1.unwrap();
    let e = only_entry(&w.journal);
    assert_eq!(e.write.actor.permission, Permission::SharedFolder);
    assert_eq!(e.write.actor.acting_account, "1001");
    assert!(w.shared.path().join("Public/notes.txt").is_file());
}

#[tokio::test]
async fn folders_made_for_a_file_are_recorded_and_folders_already_there_are_not() {
    let w = world();
    std::fs::create_dir(w.mine.path().join("Documents")).unwrap();
    let (job, data) = w.file(1, 1000, "me", "Documents/Tax/2026/a.txt");
    let mut receiver = ReceiverSession::new(
        &w.table,
        plan_for(&[(job.clone(), data)]),
        &w.journal,
        "1001",
    );
    move_all(vec![job], &mut receiver).await.1.unwrap();
    let mut made = w.journal.made_folders().unwrap();
    made.sort_by(|a, b| a.folder.cmp(&b.folder));
    let folders: Vec<&str> = made.iter().map(|m| m.folder.as_str()).collect();
    assert_eq!(folders, ["Documents/Tax", "Documents/Tax/2026"]);
    let dest = w.table.get("me").unwrap();
    for m in &made {
        let now = dest.folder_identity(&m.folder).unwrap().unwrap();
        assert_eq!(
            m.id,
            Some(FileId {
                volume: now.volume,
                index: now.index
            })
        );
    }
}

#[tokio::test]
async fn a_name_already_taken_is_never_replaced_and_the_journal_has_the_name_used() {
    let w = world();
    std::fs::create_dir(w.mine.path().join("Documents")).unwrap();
    std::fs::write(w.mine.path().join("Documents/a.txt"), b"mine").unwrap();
    let (job, data) = w.file(1, 1000, "me", "Documents/a.txt");
    let mut receiver = ReceiverSession::new(
        &w.table,
        plan_for(&[(job.clone(), data)]),
        &w.journal,
        "1001",
    );
    move_all(vec![job], &mut receiver).await.1.unwrap();
    let e = only_entry(&w.journal);
    assert!(
        matches!(e.state, State::Committed { ref final_path, .. } if final_path == "Documents/a (2).txt")
    );
    assert_eq!(
        std::fs::read(w.mine.path().join("Documents/a.txt")).unwrap(),
        b"mine"
    );
}

#[tokio::test]
async fn an_identical_file_already_there_is_recorded_as_existing_not_written() {
    let w = world();
    let (job, data) = w.file(1, 5000, "me", "a.txt");
    std::fs::write(w.mine.path().join("a.txt"), &data).unwrap();
    let mut receiver = ReceiverSession::new(
        &w.table,
        plan_for(&[(job.clone(), data)]),
        &w.journal,
        "1001",
    );
    move_all(vec![job], &mut receiver).await.1.unwrap();
    let e = only_entry(&w.journal);
    assert_eq!(
        e.state,
        State::Existing {
            stored_path: "a.txt".into()
        }
    );
    assert_eq!(std::fs::read_dir(w.mine.path()).unwrap().count(), 1);
}

#[tokio::test]
async fn a_file_outside_the_plan_writes_nothing_and_records_nothing() {
    let w = world();
    let (job, _) = w.file(1, 1000, "me", "a.txt");
    let (other, other_data) = w.file(2, 1000, "me", "b.txt");
    let mut receiver = ReceiverSession::new(
        &w.table,
        plan_for(&[(other, other_data)]),
        &w.journal,
        "1001",
    );
    move_all(vec![job], &mut receiver).await.1.unwrap();
    assert!(w.journal.entries().unwrap().is_empty());
    assert_eq!(std::fs::read_dir(w.mine.path()).unwrap().count(), 0);
}

#[tokio::test]
async fn a_file_that_changed_while_read_is_recorded_failed_never_verified() {
    use std::sync::Arc;
    // Grows by a byte between being read and being ended: the end says it changed.
    struct Growing(PathBuf);
    impl pctwin_transfer::Opener for Growing {
        fn open(
            &self,
            path: &std::path::Path,
        ) -> std::io::Result<Box<dyn pctwin_transfer::Source>> {
            let f = std::fs::File::open(path)?;
            let grow = self.0.clone();
            Ok(Box::new(GrowOnStamp { f, grow, calls: 0 }))
        }
    }
    struct GrowOnStamp {
        f: std::fs::File,
        grow: PathBuf,
        calls: u32,
    }
    impl std::io::Read for GrowOnStamp {
        fn read(&mut self, b: &mut [u8]) -> std::io::Result<usize> {
            self.f.read(b)
        }
    }
    impl std::io::Seek for GrowOnStamp {
        fn seek(&mut self, p: std::io::SeekFrom) -> std::io::Result<u64> {
            self.f.seek(p)
        }
    }
    impl pctwin_transfer::Source for GrowOnStamp {
        fn stamp(&mut self) -> std::io::Result<pctwin_transfer::Stamp> {
            self.calls += 1;
            let mut s = self.f.stamp()?;
            if self.calls > 1 {
                s.modified_ns = s.modified_ns.map(|n| n + 1);
                let _ = &self.grow;
            }
            Ok(s)
        }
    }
    let w = world();
    let (job, data) = w.file(1, 1000, "me", "a.txt");
    let mut receiver = ReceiverSession::new(
        &w.table,
        plan_for(&[(job.clone(), data)]),
        &w.journal,
        "1001",
    );
    let mut sender =
        SenderSession::new(vec![job.clone()], 1).with_opener(Arc::new(Growing(job.source.clone())));
    let (mut a, mut b) = mem_pair();
    let _ = tokio::time::timeout(std::time::Duration::from_secs(30), async {
        tokio::join!(sender.run(&mut a), receiver.run(&mut b))
    })
    .await
    .expect("hung");
    let all = w.journal.entries().unwrap();
    assert!(!all.is_empty());
    for e in all {
        let State::Failed { reached, .. } = &e.state else {
            panic!("{:?}", e.state)
        };
        assert!(matches!(**reached, State::Staged { .. }), "{reached:?}");
    }
    assert_eq!(std::fs::read_dir(w.mine.path()).unwrap().count(), 0);
}

/// A journal that stops working at one step, as a full or failing system drive would.
struct Breaks<'j> {
    inner: &'j Journal,
    at: &'static str,
    /// Every step asked for, in order.
    calls: std::cell::RefCell<Vec<String>>,
}

impl Ledger for Breaks<'_> {
    fn checkpoint(
        &self,
        id: u64,
        blocks: &[(u64, [u8; 32])],
        durable: bool,
    ) -> Result<(), JournalError> {
        self.check("checkpoint")?;
        self.inner.checkpoint(id, blocks, durable)
    }
    fn blocks(&self, id: u64) -> Result<Vec<(u64, [u8; 32])>, JournalError> {
        self.inner.blocks(id)
    }
    fn unfinished(&self) -> Result<Vec<pctwin_journal::Entry>, JournalError> {
        self.inner.unfinished()
    }
    fn temp_tag(&self, id: u64) -> String {
        self.inner.temp_tag(id)
    }
    fn plan(&self, w: &PlannedWrite) -> Result<u64, JournalError> {
        self.check("plan")?;
        self.inner.plan(w)
    }
    fn staged(
        &self,
        id: u64,
        temp: &str,
        made: &[(String, Option<FileId>)],
    ) -> Result<(), JournalError> {
        self.check("staged")?;
        self.inner.staged(id, temp, made)
    }
    fn verified(&self, id: u64, fp: [u8; 32], file: Option<FileId>) -> Result<(), JournalError> {
        self.check("verified")?;
        self.inner.verified(id, fp, file)
    }
    fn applied(&self, id: u64, final_path: &str) -> Result<(), JournalError> {
        self.check("applied")?;
        self.inner.applied(id, final_path)
    }
    fn committed(&self, id: u64, landed: Landed) -> Result<(), JournalError> {
        self.check("committed")?;
        self.inner.committed(id, landed)
    }
    fn existing(&self, id: u64, stored: &str) -> Result<(), JournalError> {
        self.check("existing")?;
        self.inner.existing(id, stored)
    }
    fn failed(&self, id: u64, why: &str) -> Result<(), JournalError> {
        self.check("failed")?;
        self.inner.failed(id, why)
    }
}

impl Breaks<'_> {
    fn check(&self, step: &str) -> Result<(), JournalError> {
        self.calls.borrow_mut().push(step.to_string());
        if step == self.at {
            Err(JournalError::Storage("the drive is full".into()))
        } else {
            Ok(())
        }
    }
}

#[tokio::test]
async fn when_the_journal_cannot_be_written_the_move_stops_and_nothing_unrecorded_is_left() {
    for at in ["plan", "staged", "verified", "applied"] {
        let w = world();
        let (job, data) = w.file(1, 1000, "me", "a.txt");
        let breaks = Breaks {
            inner: &w.journal,
            at,
            calls: Default::default(),
        };
        let mut receiver =
            ReceiverSession::new(&w.table, plan_for(&[(job.clone(), data)]), &breaks, "1001");
        let (_, received, _) = move_all(vec![job], &mut receiver).await;
        assert!(
            matches!(received, Err(TransferError::Record(_))),
            "{at}: {received:?}"
        );
        // Nothing under a real name that the journal does not know, and no file announced as
        // copied.
        assert!(!w.mine.path().join("a.txt").exists(), "{at}");
        assert!(
            !matches!(receiver.outcome(id(1)), Some(ReceiveOutcome::Finished(_))),
            "{at}"
        );
        drop(receiver);
        // Nothing was even tried after the step that could not be recorded.
        let calls = breaks.calls.borrow();
        assert_eq!(
            calls.last().map(String::as_str),
            Some(at),
            "{at}: {calls:?}"
        );
        assert_eq!(
            calls.iter().filter(|c| *c == at).count(),
            1,
            "{at}: {calls:?}"
        );
        drop(calls);
        // Not even a temporary file is left; and once the journal works again, recovery ends
        // whatever it had as failed, never committed.
        assert_eq!(std::fs::read_dir(w.mine.path()).unwrap().count(), 0, "{at}");
        let r = pctwin_transfer::recover(&w.journal, &w.table).unwrap();
        assert!(r.committed.is_empty(), "{at}");
        for e in w.journal.entries().unwrap() {
            assert!(
                matches!(e.state, State::Failed { .. }),
                "{at}: {:?}",
                e.state
            );
        }
    }
}

#[tokio::test]
async fn a_journal_that_fails_only_at_the_last_step_leaves_the_file_for_recovery_to_commit() {
    let w = world();
    let (job, data) = w.file(1, 1000, "me", "a.txt");
    let breaks = Breaks {
        inner: &w.journal,
        at: "committed",
        calls: Default::default(),
    };
    {
        let mut receiver = ReceiverSession::new(
            &w.table,
            plan_for(&[(job.clone(), data.clone())]),
            &breaks,
            "1001",
        );
        let (_, received, _) = move_all(vec![job], &mut receiver).await;
        assert!(matches!(received, Err(TransferError::Record(_))));
        assert!(!matches!(
            receiver.outcome(id(1)),
            Some(ReceiveOutcome::Finished(_))
        ));
    }
    let e = only_entry(&w.journal);
    let State::Applied { temp, .. } = &e.state else {
        panic!("{:?}", e.state)
    };
    // Its temporary file is named after the journal and the entry, so recovery knows it.
    assert_eq!(temp, &pctwin_gate::temp_name(&w.journal.temp_tag(e.id)));
    let r = pctwin_transfer::recover(&w.journal, &w.table).unwrap();
    assert_eq!(r.committed, [e.id]);
    assert_eq!(std::fs::read(w.mine.path().join("a.txt")).unwrap(), data);
}

/// Damages the first block's piece, then cuts the connection before the old laptop can end the
/// file, as a laptop that goes away would.
struct Damaging {
    inner: Mem,
    pieces: usize,
    sends: usize,
    cut_at: usize,
}

impl Channel for Damaging {
    async fn send(&mut self, data: &[u8]) -> Result<(), ChannelError> {
        self.sends += 1;
        if self.sends >= self.cut_at {
            return Err(ChannelError);
        }
        let mut data = data.to_vec();
        if let Ok(pctwin_transfer::Message::Piece { .. }) = pctwin_transfer::Message::decode(&data)
        {
            self.pieces += 1;
            if self.pieces == 1 {
                let last = data.len() - 1;
                data[last] ^= 0xFF;
            }
        }
        self.inner.send(&data).await
    }
    async fn recv(&mut self) -> Result<Vec<u8>, ChannelError> {
        if self.sends + 1 >= self.cut_at {
            return Err(ChannelError);
        }
        self.inner.recv().await
    }
}

#[tokio::test]
async fn a_file_that_fails_is_recorded_with_its_reason_even_if_its_end_never_comes() {
    let w = world();
    let (job, data) = w.file(1, 1000, "me", "a.txt");
    let mut receiver = ReceiverSession::new(
        &w.table,
        plan_for(&[(job.clone(), data)]),
        &w.journal,
        "1001",
    );
    let mut sender = SenderSession::new(vec![job], 1);
    let (a, b) = mem_pair();
    // Sends: the start, the damaged block, then the end, which never goes.
    let send_side = async {
        let mut a = Damaging {
            inner: a,
            pieces: 0,
            sends: 0,
            cut_at: 3,
        };
        sender.run(&mut a).await
    };
    let receive_side = async {
        let mut b = b;
        receiver.run(&mut b).await
    };
    let _ = tokio::time::timeout(std::time::Duration::from_secs(30), async {
        tokio::join!(send_side, receive_side)
    })
    .await
    .expect("hung");
    let e = only_entry(&w.journal);
    let State::Failed { why, .. } = &e.state else {
        panic!("{:?}", e.state)
    };
    assert!(why.contains("damaged"), "{why}");
    assert_eq!(std::fs::read_dir(w.mine.path()).unwrap().count(), 0);
}

#[tokio::test]
async fn a_file_that_cannot_be_started_here_is_recorded_failed_with_why() {
    let w = world();
    std::fs::write(w.mine.path().join("Documents"), b"a file in the way").unwrap();
    let (job, data) = w.file(1, 1000, "me", "Documents/a.txt");
    let mut receiver = ReceiverSession::new(
        &w.table,
        plan_for(&[(job.clone(), data)]),
        &w.journal,
        "1001",
    );
    move_all(vec![job], &mut receiver).await.1.unwrap();
    let e = only_entry(&w.journal);
    let State::Failed { why, reached } = &e.state else {
        panic!("{:?}", e.state)
    };
    assert!(why.contains("in the way"), "{why}");
    assert_eq!(**reached, State::Planned);
    assert_eq!(
        std::fs::read(w.mine.path().join("Documents")).unwrap(),
        b"a file in the way"
    );
}
