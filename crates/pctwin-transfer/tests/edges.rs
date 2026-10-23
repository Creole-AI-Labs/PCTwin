//! Edges of a move the PR 45 review named: two files the plan puts at the same place, two places
//! that differ only by case, and an old laptop that floods the new one with starts it refuses.
//! Nothing is overwritten, every file keeps an honest line, and refusals never pile up.
// Tests make and remove files to test on; the no-file-changes fence is for the shipped code.
#![allow(clippy::disallowed_methods)]

use std::path::PathBuf;

use pctwin_gate::{Approved, Destinations};
use pctwin_record::ItemId;
use pctwin_transfer::{
    Channel, ChannelError, FileSender, Message, ReceiveOutcome, ReceiverSession, SendJob,
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

/// Two files with different bytes, sent to `paths` in one approved folder; what landed, by
/// stored path.
async fn two_files_to(paths: [&str; 2]) -> (tempfile::TempDir, Vec<Option<ReceiveOutcome>>) {
    let old = tempfile::tempdir_in(env!("CARGO_TARGET_TMPDIR")).unwrap();
    let new = tempfile::tempdir_in(env!("CARGO_TARGET_TMPDIR")).unwrap();
    let mut table = Destinations::new();
    table
        .approve("me", Approved::MyFolders, new.path())
        .unwrap();
    let jobs: Vec<SendJob> = paths
        .iter()
        .enumerate()
        .map(|(n, path)| {
            let source: PathBuf = old.path().join(format!("f{n}.txt"));
            std::fs::write(&source, format!("file number {n}")).unwrap();
            SendJob {
                size: 13,
                item: id(n as u8),
                source,
                destination: "me".into(),
                path: (*path).into(),
                compressible: true,
                tier: Tier::Rest,
            }
        })
        .collect();
    let sizes: Vec<(&SendJob, u64)> = jobs.iter().map(|j| (j, j.size)).collect();
    let mut receiver = ReceiverSession::new(
        &table,
        common::approved_jobs(&sizes),
        common::journal(),
        "1001",
    );
    let mut sender = SenderSession::new(jobs.clone(), 1);
    let (mut a, mut b) = mem_pair();
    let landed = receiver.meter();
    let (s, r) = common::unless_stuck(&landed, 26, async {
        tokio::join!(sender.run(&mut a), receiver.run(&mut b))
    })
    .await;
    assert!(s.is_ok() && r.is_ok());
    for n in 0..2u8 {
        assert_eq!(sender.outcome(id(n)), Some(&SendOutcome::Arrived));
    }
    let outcomes = (0..2u8).map(|n| receiver.outcome(id(n))).collect();
    (new, outcomes)
}

/// Every file landed whole under its own name: none written over another. Each file reads back
/// its own words, which only two files can do; the names are compared as given, since on a drive
/// that tells case apart (Linux) two names differing only in case are two places.
fn each_landed_whole(new: &tempfile::TempDir, outcomes: &[Option<ReceiveOutcome>]) {
    let mut names = Vec::new();
    for (n, o) in outcomes.iter().enumerate() {
        let Some(ReceiveOutcome::Finished(done)) = o else {
            panic!("file {n} ended {o:?}");
        };
        assert_eq!(
            std::fs::read_to_string(new.path().join(&done.final_path)).unwrap(),
            format!("file number {n}")
        );
        names.push(done.final_path.clone());
    }
    names.dedup();
    assert_eq!(names.len(), 2, "two files, two names: {outcomes:?}");
}

#[tokio::test]
async fn two_files_for_the_same_place_both_land_and_neither_is_written_over() {
    let (new, outcomes) = two_files_to(["Docs/same.txt", "Docs/same.txt"]).await;
    each_landed_whole(&new, &outcomes);
}

#[tokio::test]
async fn places_differing_only_by_case_both_land_on_any_drive() {
    let (new, outcomes) = two_files_to(["Docs/Photo.txt", "docs/photo.TXT"]).await;
    each_landed_whole(&new, &outcomes);
}

/// An old laptop that starts, again and again, files the plan does not hold: each is refused,
/// the move still ends well, and what is kept of the refusals stays within the plan's own size.
#[tokio::test]
async fn a_flood_of_refused_starts_never_piles_up() {
    let old = tempfile::tempdir_in(env!("CARGO_TARGET_TMPDIR")).unwrap();
    let new = tempfile::tempdir_in(env!("CARGO_TARGET_TMPDIR")).unwrap();
    let mut table = Destinations::new();
    table
        .approve("me", Approved::MyFolders, new.path())
        .unwrap();
    let source = old.path().join("f0.txt");
    std::fs::write(&source, b"file number 0").unwrap();
    let job = SendJob {
        size: 13,
        item: id(0),
        source: source.clone(),
        destination: "me".into(),
        path: "Docs/f0.txt".into(),
        compressible: true,
        tier: Tier::Rest,
    };
    let mut receiver = ReceiverSession::new(
        &table,
        common::approved_jobs(&[(&job, 13)]),
        common::journal(),
        "1001",
    );
    let header = FileSender::open(&source, None, true)
        .unwrap()
        .header()
        .clone();
    let (mut old_end, mut new_end) = mem_pair();
    let script = async {
        assert_eq!(
            Message::decode(&old_end.recv().await.unwrap()).unwrap(),
            Message::Ready
        );
        for n in 0..5000u32 {
            let start = Message::StartFile {
                stream: n,
                // Never the planned file: item 1 onwards, each a stranger to the plan.
                item: ItemId::from_hex(&format!("{:08x}{}", n + 1, "1".repeat(24))).unwrap(),
                destination: "me".into(),
                path: format!("Docs/flood-{n}.txt"),
                header: header.clone(),
                resumed_done: 0,
            };
            old_end.send(&start.encode()).await.unwrap();
            assert_eq!(
                Message::decode(&old_end.recv().await.unwrap()).unwrap(),
                Message::FileDone {
                    stream: n,
                    ok: false
                }
            );
        }
        old_end.send(&Message::AllSent.encode()).await.unwrap();
    };
    let ((), r) = common::no_hang(async { tokio::join!(script, receiver.run(&mut new_end)) }).await;
    r.unwrap();
    assert!(std::fs::read_dir(new.path()).unwrap().next().is_none());
    // Nothing of the flood is the plan's: its own file has no line.
    assert_eq!(receiver.outcome(id(0)), None);
    // Each refusal is kept to say why (at most a fixed number, 1,024), never one per start.
    assert!(
        receiver.refusals_kept() <= 1024,
        "{}",
        receiver.refusals_kept()
    );
}
