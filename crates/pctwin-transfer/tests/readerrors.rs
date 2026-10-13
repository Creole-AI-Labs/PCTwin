//! Reading a weak or failing old drive (Task List 1.5, the ddrescue approach): what can be read is
//! copied first; a file that fails is skipped without retrying at once and tried once more at the
//! end (never on a drive already found weak); errors in a row mean a dying drive, so reading stops
//! cleanly and the rest is reported, not strained for; a missing file is not a drive error.
// Tests make and remove files to test on; the no-file-changes fence is for the shipped code.
#![allow(clippy::disallowed_methods)]

use std::collections::{BTreeSet, HashMap};
use std::io::{self, Read, Seek, SeekFrom};
use std::path::{Path, PathBuf};
use std::sync::{Arc, Mutex};

use pctwin_gate::{Approved, Destinations};
use pctwin_record::ItemId;
use pctwin_scan::ReadPlan;
use pctwin_transfer::{
    Channel, ChannelError, Opener, ReceiverSession, SendJob, SendOutcome, SenderSession, Source,
    Stamp, Tier,
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

/// How a file behaves when read.
#[derive(Clone, Copy, PartialEq, Eq)]
enum Fault {
    /// Every read fails (a bad area of the drive).
    Always,
    /// The first open's reads fail; later opens read fine (a marginal sector).
    FirstTime,
    /// The first open itself fails; later opens work.
    OpenFirstTime,
}

/// Opens real files, but makes the chosen ones fail, and logs every open in order.
#[derive(Default)]
struct Faulty {
    faults: HashMap<PathBuf, Fault>,
    opens: Mutex<Vec<PathBuf>>,
}

struct FailingRead;

impl Read for FailingRead {
    fn read(&mut self, _: &mut [u8]) -> io::Result<usize> {
        Err(io::Error::other("the drive could not read this area"))
    }
}

impl Seek for FailingRead {
    fn seek(&mut self, _: SeekFrom) -> io::Result<u64> {
        Ok(0)
    }
}

struct Wrapped<R> {
    inner: R,
    stamp: Stamp,
}

impl<R: Read> Read for Wrapped<R> {
    fn read(&mut self, buf: &mut [u8]) -> io::Result<usize> {
        self.inner.read(buf)
    }
}

impl<R: Seek> Seek for Wrapped<R> {
    fn seek(&mut self, to: SeekFrom) -> io::Result<u64> {
        self.inner.seek(to)
    }
}

impl<R: Read + Seek + Send> Source for Wrapped<R> {
    fn stamp(&mut self) -> io::Result<Stamp> {
        Ok(self.stamp)
    }
}

impl Opener for Faulty {
    fn open(&self, path: &Path) -> io::Result<Box<dyn Source>> {
        let previous = {
            let mut opens = self.opens.lock().unwrap();
            let n = opens.iter().filter(|p| p.as_path() == path).count();
            opens.push(path.to_path_buf());
            n
        };
        if self.faults.get(path) == Some(&Fault::OpenFirstTime) && previous == 0 {
            return Err(io::Error::other("the drive did not answer"));
        }
        let real = pctwin_transfer::FsOpener.open(path)?;
        let fails = match self.faults.get(path) {
            Some(Fault::Always) => true,
            Some(Fault::FirstTime) => previous == 0,
            Some(Fault::OpenFirstTime) | None => false,
        };
        if fails {
            let mut real = real;
            let stamp = real.stamp()?;
            return Ok(Box::new(Wrapped {
                inner: FailingRead,
                stamp,
            }));
        }
        Ok(real)
    }
}

struct Laptops {
    old: tempfile::TempDir,
    new: tempfile::TempDir,
}

fn id(n: u8) -> ItemId {
    ItemId::from_hex(&format!("{n:02x}{}", "0".repeat(30))).unwrap()
}

/// `count` files of 200 KB each, as jobs in that order.
fn setup(count: u8) -> (Laptops, Vec<SendJob>) {
    let l = Laptops {
        old: tempfile::tempdir().unwrap(),
        new: tempfile::tempdir().unwrap(),
    };
    let jobs = (0..count)
        .map(|n| {
            let source = l.old.path().join(format!("f{n}.bin"));
            std::fs::write(&source, vec![n; 200_000]).unwrap();
            SendJob {
                item: id(n),
                source,
                destination: "me".into(),
                path: format!("f{n}.bin"),
                compressible: false,
                tier: Tier::Rest,
            }
        })
        .collect();
    (l, jobs)
}

async fn run(
    l: &Laptops,
    jobs: Vec<SendJob>,
    faulty: Arc<Faulty>,
    plan: ReadPlan,
) -> SenderSession {
    let mut table = Destinations::new();
    table
        .approve("me", Approved::MyFolders, l.new.path())
        .unwrap();
    let files: Vec<(ItemId, u64)> = jobs.iter().map(|j| (j.item, 200_000)).collect();
    let mut sender = SenderSession::new(jobs, 2)
        .with_opener(faulty)
        .with_read_plan(plan);
    let mut receiver =
        ReceiverSession::new(&table, common::approved(&files), common::journal(), "1001");
    let (mut a, mut b) = mem_pair();
    let (sent, received) = tokio::time::timeout(std::time::Duration::from_secs(30), async {
        tokio::join!(sender.run(&mut a), receiver.run(&mut b))
    })
    .await
    .expect("the move hung");
    sent.unwrap();
    received.unwrap();
    sender
}

fn careful() -> ReadPlan {
    ReadPlan::Careful {
        most_important_first: true,
        read_each_file_once: true,
        stop_after_read_errors: 5,
    }
}

fn path_of(l: &Laptops, n: u8) -> PathBuf {
    l.old.path().join(format!("f{n}.bin"))
}

fn opens_of(f: &Faulty, p: &Path) -> usize {
    f.opens
        .lock()
        .unwrap()
        .iter()
        .filter(|o| o.as_path() == p)
        .count()
}

#[tokio::test]
async fn a_bad_file_fails_on_its_own_and_is_tried_once_more_at_the_end() {
    let (l, jobs) = setup(4);
    let mut faulty = Faulty::default();
    faulty.faults.insert(path_of(&l, 1), Fault::Always);
    let faulty = Arc::new(faulty);
    let s = run(&l, jobs, faulty.clone(), ReadPlan::Normal).await;
    for n in [0, 2, 3] {
        assert_eq!(s.outcome(id(n)), Some(&SendOutcome::Arrived), "{n}");
    }
    assert!(matches!(s.outcome(id(1)), Some(SendOutcome::Failed(_))));
    assert_eq!(
        opens_of(&faulty, &path_of(&l, 1)),
        2,
        "tried exactly once more"
    );
    assert!(!l.new.path().join("f1.bin").exists());
    let leftovers: Vec<_> = std::fs::read_dir(l.new.path())
        .unwrap()
        .map(|e| e.unwrap().file_name().to_string_lossy().into_owned())
        .filter(|n| n.ends_with(".part"))
        .collect();
    assert!(leftovers.is_empty(), "{leftovers:?}");
}

#[tokio::test]
async fn a_file_that_reads_on_the_second_try_arrives_after_everything_else_was_read() {
    let (l, jobs) = setup(4);
    let mut faulty = Faulty::default();
    faulty.faults.insert(path_of(&l, 0), Fault::FirstTime);
    let faulty = Arc::new(faulty);
    let s = run(&l, jobs, faulty.clone(), ReadPlan::Normal).await;
    assert_eq!(s.outcome(id(0)), Some(&SendOutcome::Arrived));
    assert_eq!(
        std::fs::read(l.new.path().join("f0.bin")).unwrap(),
        vec![0u8; 200_000]
    );
    // The second try came only after every other file had been read.
    let opens = faulty.opens.lock().unwrap().clone();
    let second_try = opens.iter().rposition(|p| *p == path_of(&l, 0)).unwrap();
    for n in 1..4 {
        let first = opens.iter().position(|p| *p == path_of(&l, n)).unwrap();
        assert!(first < second_try, "file {n} was read after the retry");
    }
}

#[tokio::test]
async fn on_a_weak_drive_a_failed_file_is_never_read_again() {
    let (l, jobs) = setup(3);
    let mut faulty = Faulty::default();
    faulty.faults.insert(path_of(&l, 1), Fault::FirstTime);
    let faulty = Arc::new(faulty);
    let s = run(&l, jobs, faulty.clone(), careful()).await;
    assert!(matches!(s.outcome(id(1)), Some(SendOutcome::Failed(_))));
    assert_eq!(opens_of(&faulty, &path_of(&l, 1)), 1);
    assert_eq!(s.outcome(id(2)), Some(&SendOutcome::Arrived));
}

#[tokio::test]
async fn errors_in_a_row_stop_reading_to_protect_a_dying_drive() {
    let (l, jobs) = setup(12);
    let mut faulty = Faulty::default();
    for n in 0..8 {
        faulty.faults.insert(path_of(&l, n), Fault::Always);
    }
    let faulty = Arc::new(faulty);
    let s = run(&l, jobs, faulty.clone(), careful()).await;
    // Five errors in a row: reading stops. The files after that were never touched.
    let touched: BTreeSet<PathBuf> = faulty.opens.lock().unwrap().iter().cloned().collect();
    for n in 8..12 {
        assert!(
            !touched.contains(&path_of(&l, n)),
            "file {n} was read after the stop"
        );
        match s.outcome(id(n)) {
            Some(SendOutcome::Failed(why)) => assert!(why.contains("stopped reading"), "{why}"),
            other => panic!("file {n}: {other:?}"),
        }
    }
    assert!(s.stopped_reading());
    let copied: Vec<_> = std::fs::read_dir(l.new.path()).unwrap().collect();
    assert!(copied.is_empty(), "{copied:?}");
}

#[tokio::test]
async fn scattered_errors_do_not_stop_the_move() {
    let (l, jobs) = setup(12);
    let mut faulty = Faulty::default();
    for n in [0, 2, 4, 6, 8, 10] {
        faulty.faults.insert(path_of(&l, n), Fault::Always);
    }
    let faulty = Arc::new(faulty);
    let s = run(&l, jobs, faulty, careful()).await;
    assert!(!s.stopped_reading());
    for n in [1, 3, 5, 7, 9, 11] {
        assert_eq!(s.outcome(id(n)), Some(&SendOutcome::Arrived), "{n}");
    }
}

#[tokio::test]
async fn missing_files_are_not_drive_errors() {
    let (l, mut jobs) = setup(30);
    for job in jobs.iter_mut().take(25) {
        job.source = l.old.path().join("gone").join("missing.bin");
    }
    let s = run(&l, jobs, Arc::new(Faulty::default()), careful()).await;
    assert!(!s.stopped_reading());
    for n in 25..30 {
        assert_eq!(s.outcome(id(n)), Some(&SendOutcome::Arrived), "{n}");
    }
}

#[tokio::test]
async fn a_file_that_will_not_open_at_first_is_tried_again_at_the_end() {
    let (l, jobs) = setup(3);
    let mut faulty = Faulty::default();
    faulty.faults.insert(path_of(&l, 1), Fault::OpenFirstTime);
    let faulty = Arc::new(faulty);
    let s = run(&l, jobs, faulty.clone(), ReadPlan::Normal).await;
    assert_eq!(s.outcome(id(1)), Some(&SendOutcome::Arrived));
    assert_eq!(opens_of(&faulty, &path_of(&l, 1)), 2);
}
