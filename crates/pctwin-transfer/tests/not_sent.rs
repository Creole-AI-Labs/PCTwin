//! A planned file the old laptop never starts (gone since the scan, unreadable, stored online
//! only) is told to the new laptop with why, so the new laptop's report gives the true reason
//! rather than a blank. A laptop one version back is never told: its line stays empty, as before.
// Tests make and remove files to test on; the no-file-changes fence is for the shipped code.
#![allow(clippy::disallowed_methods)]

use pctwin_gate::{Approved, Destinations};
use pctwin_journal::Journal;
use pctwin_record::ItemId;
use pctwin_transfer::{
    Cause, Channel, ChannelError, Message, ReceiveOutcome, ReceiverSession, STOP_SINCE, SendJob,
    SendOutcome, SenderSession, Tier,
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

struct World {
    old: tempfile::TempDir,
    _mine: tempfile::TempDir,
    _jdir: tempfile::TempDir,
    journal: Journal,
    table: Destinations,
}

fn world() -> World {
    let old = tempfile::tempdir().unwrap();
    let mine = tempfile::tempdir().unwrap();
    let jdir = tempfile::tempdir().unwrap();
    let journal = Journal::open(&jdir.path().join("journal.redb")).unwrap();
    let mut table = Destinations::new();
    table
        .approve("me", Approved::MyFolders, mine.path())
        .unwrap();
    World {
        old,
        _mine: mine,
        _jdir: jdir,
        journal,
        table,
    }
}

impl World {
    fn file(&self, n: u8, bytes: &[u8]) -> SendJob {
        let source = self.old.path().join(format!("f{n}.bin"));
        std::fs::write(&source, bytes).unwrap();
        SendJob {
            size: bytes.len() as u64,
            item: id(n),
            source,
            destination: "me".into(),
            path: format!("f{n}.bin"),
            compressible: false,
            tier: Tier::Rest,
        }
    }
}

/// Moves `jobs` (file 2 removed after the plan was approved) on move protocol `version`.
async fn gone_since_the_scan(version: u16) -> (Option<SendOutcome>, Option<ReceiveOutcome>) {
    let w = world();
    let jobs = vec![w.file(1, b"still here"), w.file(2, b"about to go")];
    let sized: Vec<(&SendJob, u64)> = jobs.iter().map(|j| (j, j.size)).collect();
    let plan = common::approved_jobs(&sized);
    std::fs::remove_file(&jobs[1].source).unwrap();
    let mut sender = SenderSession::new(jobs, 2).with_protocol(version);
    let mut receiver =
        ReceiverSession::new(&w.table, plan, &w.journal, "1000").with_protocol(version);
    let (mut a, mut b) = mem_pair();
    let (sent, received) =
        common::no_hang(async { tokio::join!(sender.run(&mut a), receiver.run(&mut b)) }).await;
    sent.unwrap();
    received.unwrap();
    assert!(matches!(
        receiver.outcome(id(1)),
        Some(ReceiveOutcome::Finished(_))
    ));
    (sender.outcome(id(2)).cloned(), receiver.outcome(id(2)))
}

#[tokio::test]
async fn a_file_gone_since_the_scan_is_no_longer_there_on_both_laptops() {
    let (sent, received) = gone_since_the_scan(STOP_SINCE).await;
    assert!(
        matches!(&sent, Some(SendOutcome::Failed(why)) if why.cause() == Cause::NoLongerThere),
        "{sent:?}"
    );
    match &received {
        Some(ReceiveOutcome::Failed(why)) => {
            assert_eq!(why.cause(), Cause::NoLongerThere);
            assert!(why.contains("no longer on the old laptop"), "{why}");
        }
        other => panic!("{other:?}"),
    }
}

#[tokio::test]
async fn a_laptop_one_version_back_is_not_told_and_its_line_stays_empty() {
    let (sent, received) = gone_since_the_scan(STOP_SINCE - 1).await;
    assert!(
        matches!(&sent, Some(SendOutcome::Failed(why)) if why.cause() == Cause::NoLongerThere),
        "{sent:?}"
    );
    assert_eq!(received, None);
}

#[test]
fn not_sent_is_said_only_on_the_main_connection() {
    let m = Message::NotSent {
        item: id(7),
        cause: 0,
    };
    assert_eq!(Message::decode(&m.encode()).unwrap(), m);
    let mut on_a_stream = m.encode();
    on_a_stream[4] = 1;
    assert!(Message::decode(&on_a_stream).is_err());
    let mut longer = m.encode();
    longer.push(0);
    assert!(Message::decode(&longer).is_err());
}
