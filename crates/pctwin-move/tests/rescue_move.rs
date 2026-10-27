//! A rescued drive, end to end (Task List 2.6; acceptance T26-018): a dead laptop's drive,
//! connected to another laptop, is scanned read-only into an ordinary plan and moved through the
//! normal move, safety gate and journal as a local source, read carefully: the most important
//! first, each file once, a clean stop after repeated read errors, and every file that could not
//! be read listed as such, never as copied. Nothing on the drive changes.
// Tests make the fake drive's files; the no-file-changes fence is for the shipped code.
#![allow(clippy::disallowed_methods)]

use std::collections::BTreeMap;
use std::io;
use std::path::{Path, PathBuf};
use std::sync::{Arc, Mutex};
use std::time::Duration;

use pctwin_gate::{Approved, Destinations};
use pctwin_journal::Journal;
use pctwin_move::report::{FileResult, for_the_mover};
use pctwin_move::{Live, Preview, Steps, Stop, send_plan};
use pctwin_orchestrate::rescue::{DriveIdentity, scan_rescued_drive};
use pctwin_record::{FolderRole, Item, PersonTarget, Record};
use pctwin_scan::{AccessFact, AccessReader, UnixMode};
use pctwin_transfer::{
    Cause, Channel, ChannelError, LandingPlace, Opener, ReceiveOutcome, Source, role_label,
    shown_path,
};
use tokio::sync::mpsc;

struct Mem {
    tx: mpsc::UnboundedSender<Vec<u8>>,
    rx: mpsc::UnboundedReceiver<Vec<u8>>,
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

fn mem_pair() -> (Mem, Mem) {
    let (a_tx, b_rx) = mpsc::unbounded_channel();
    let (b_tx, a_rx) = mpsc::unbounded_channel();
    (Mem { tx: a_tx, rx: a_rx }, Mem { tx: b_tx, rx: b_rx })
}

const ADA: &str = "S-1-5-21-1-2-3-1001";

/// The drive records Ada's settings file as hers.
struct Owners;

impl AccessReader for Owners {
    fn read(&mut self, _path: &Path, _unix: Option<UnixMode>) -> AccessFact {
        AccessFact::Known(pctwin_scan::Access {
            everyone: false,
            people: Default::default(),
            owner: Some(ADA.into()),
        })
    }
}

fn touch(p: &Path, bytes: &[u8]) {
    std::fs::create_dir_all(p.parent().unwrap()).unwrap();
    std::fs::write(p, bytes).unwrap();
}

/// A dead Windows laptop's drive with Ada's things.
fn drive(files: usize) -> tempfile::TempDir {
    let d = tempfile::tempdir().unwrap();
    let r = d.path();
    touch(&r.join("Windows/System32/config/SYSTEM"), b"");
    touch(&r.join("Users/Ada/NTUSER.DAT"), b"hive");
    for n in 0..files {
        touch(
            &r.join(format!("Users/Ada/Documents/letter {n:02}.txt")),
            format!("letter {n}").repeat(50).as_bytes(),
        );
    }
    touch(&r.join("Users/Ada/Pictures/beach.jpg"), &[7u8; 3000]);
    d
}

fn identity() -> DriveIdentity {
    DriveIdentity {
        volume_id: r"\\?\Volume{11111111-2222-3333-4444-555555555555}\".into(),
        // The drive the test's folders are on, as the system's volume list names it.
        serial: Some(
            pctwin_orchestrate::rescue::volume_of(&std::env::temp_dir())
                .unwrap()
                .to_string(),
        ),
        file_system: "NTFS".into(),
        total_bytes: 128_000_000_000,
    }
}

struct Ada;

impl Steps for Ada {
    async fn person(&mut self, _: &Record, _: &str) -> Result<PersonTarget, Stop> {
        Ok(PersonTarget::ExistingAccount {
            account_id: "NEW-ADA".into(),
        })
    }
    async fn approve(&mut self, _: &Record, _: &Preview) -> Result<(), Stop> {
        Ok(())
    }
    fn landing(&self, item: &Item) -> Option<LandingPlace> {
        Some(LandingPlace::new(
            role_label(item.place.role),
            shown_path(&item.path),
        ))
    }
}

/// Reads the drive, failing for the files named (as a failing drive does), and counts how often
/// each file is opened.
struct Flaky {
    bad: Vec<PathBuf>,
    opens: Mutex<BTreeMap<PathBuf, u32>>,
    /// The rescue, whose own handles every file is read through.
    scan: pctwin_orchestrate::rescue::RescueScan,
}

impl Opener for Flaky {
    fn open(&self, path: &Path) -> io::Result<Box<dyn Source>> {
        *self
            .opens
            .lock()
            .unwrap()
            .entry(path.to_path_buf())
            .or_default() += 1;
        if self.bad.iter().any(|b| b == path) {
            return Err(io::Error::other(
                "the drive could not read this part of the disk",
            ));
        }
        Ok(Box::new(self.scan.open(path)?))
    }
}

struct Moved {
    received: pctwin_move::Received,
    sent: pctwin_move::Sent,
    record: Record,
    dests: tempfile::TempDir,
    opens: BTreeMap<PathBuf, u32>,
    sources: BTreeMap<pctwin_record::ItemId, PathBuf>,
}

async fn rescue_move(d: &Path, bad: impl Fn(&Path) -> bool) -> Moved {
    let scan = scan_rescued_drive(d, &identity(), &mut Owners).unwrap();
    let record = scan.record.clone();
    let dests = tempfile::tempdir().unwrap();
    let mut table = Destinations::new();
    for role in [FolderRole::Documents, FolderRole::Pictures] {
        let dir = dests.path().join(role_label(role));
        std::fs::create_dir_all(&dir).unwrap();
        table
            .approve(role_label(role), Approved::MyFolders, &dir)
            .unwrap();
    }
    let sources: BTreeMap<_, _> = record
        .items
        .iter()
        .filter_map(|i| scan.source_of(i).map(|p| (i.id, p)))
        .collect();
    let plan = send_plan(
        &record,
        |i| scan.source_of(i),
        |i| LandingPlace::new(role_label(i.place.role), shown_path(&i.path)),
    )
    .read_with(scan.read_plan);
    let opener = Arc::new(Flaky {
        bad: sources.values().filter(|p| bad(p)).cloned().collect(),
        opens: Mutex::new(BTreeMap::new()),
        scan: scan.clone(),
    });
    let jdir = tempfile::tempdir().unwrap();
    let journal = Journal::open(&jdir.path().join("journal.redb")).unwrap();
    let (mut a, mut b) = mem_pair();
    let (quiet, quiet_old) = (Live::default(), Live::default());
    let mut steps = Ada;
    let (s, r) = tokio::time::timeout(Duration::from_secs(300), async {
        tokio::join!(
            pctwin_move::send_with(&mut a, &record, plan, &quiet_old, opener.clone()),
            pctwin_move::receive(&mut b, &mut steps, &table, &journal, "NEW-ADA", &quiet),
        )
    })
    .await
    .expect("hung");
    Moved {
        sent: s.unwrap(),
        received: r.unwrap(),
        record,
        dests,
        opens: opener.opens.lock().unwrap().clone(),
        sources,
    }
}

fn snapshot(root: &Path) -> Vec<(PathBuf, u64, std::time::SystemTime)> {
    let mut out = Vec::new();
    let mut pending = vec![root.to_path_buf()];
    while let Some(dir) = pending.pop() {
        for e in std::fs::read_dir(&dir).unwrap() {
            let e = e.unwrap();
            // Through the file itself: a folder listing's copy of the details can lag behind.
            let m = std::fs::symlink_metadata(e.path()).unwrap();
            if m.is_dir() {
                pending.push(e.path());
            }
            out.push((e.path(), m.len(), m.modified().unwrap()));
        }
    }
    out.sort();
    out
}

/// A healthy rescued drive: everything arrives exactly, through the normal move; nothing on the
/// drive changes.
#[tokio::test]
async fn a_rescued_drive_moves_like_any_old_laptop() {
    let d = drive(4);
    let before = snapshot(d.path());
    let m = rescue_move(d.path(), |_| false).await;
    let mut copied = 0;
    for line in &m.received.files {
        let item = m.record.items.iter().find(|i| i.id == line.item).unwrap();
        match &line.outcome {
            Some(ReceiveOutcome::Finished(done)) => {
                let landed = m
                    .dests
                    .path()
                    .join(role_label(item.place.role))
                    .join(&done.final_path);
                assert_eq!(
                    std::fs::read(landed).unwrap(),
                    std::fs::read(&m.sources[&item.id]).unwrap()
                );
                copied += 1;
            }
            other => panic!("{:?}: {other:?}", item.path),
        }
    }
    assert_eq!(copied, 5);
    let report = for_the_mover(&m.received, ADA);
    assert!(report.own.iter().all(|l| l.result == FileResult::Copied));
    // Each file was opened once.
    assert!(m.opens.values().all(|n| *n == 1), "{:?}", m.opens);
    assert_eq!(snapshot(d.path()), before, "nothing on the drive changed");
}

/// A failing drive: a file that cannot be read is opened once, never again, and ends
/// "unreadable"; everything else still arrives; nothing that failed is called copied.
#[tokio::test]
async fn a_failing_read_is_tried_once_and_listed() {
    let d = drive(6);
    let m = rescue_move(d.path(), |p| p.ends_with("letter 02.txt")).await;
    let mut unreadable = Vec::new();
    for line in &m.received.files {
        let item = m.record.items.iter().find(|i| i.id == line.item).unwrap();
        match &line.outcome {
            Some(ReceiveOutcome::Finished(_)) => {}
            Some(ReceiveOutcome::Failed(f)) => {
                assert_eq!(f.cause(), Cause::Unreadable, "{f:?}");
                unreadable.push(shown_path(&item.path));
            }
            other => panic!("{:?}: {other:?}", item.path),
        }
    }
    assert_eq!(unreadable, ["letter 02.txt"]);
    let bad = m
        .sources
        .values()
        .find(|p| p.ends_with("letter 02.txt"))
        .unwrap();
    assert_eq!(m.opens[bad], 1, "read once, never hammered");
    let report = for_the_mover(&m.received, ADA);
    assert_eq!(
        report
            .own
            .iter()
            .filter(|l| l.result == FileResult::Failed)
            .count(),
        1
    );
}

/// A drive that keeps failing: after five read errors in a row PCTwin stops reading cleanly;
/// every file not read is listed as unreadable, none as copied, and none was opened twice.
#[tokio::test]
async fn a_drive_that_keeps_failing_is_not_strained() {
    let d = drive(12);
    let m = rescue_move(d.path(), |p| p.to_string_lossy().contains("letter")).await;
    let mut finished = 0;
    for line in &m.received.files {
        let item = m.record.items.iter().find(|i| i.id == line.item).unwrap();
        match &line.outcome {
            Some(ReceiveOutcome::Finished(_)) => {
                finished += 1;
                assert!(!shown_path(&item.path).contains("letter"));
            }
            Some(ReceiveOutcome::Failed(f)) => assert_eq!(f.cause(), Cause::Unreadable),
            // Never sent, so it never arrived: the report says "did not arrive", never copied.
            None => {}
            other => panic!("{:?}: {other:?}", item.path),
        }
    }
    assert!(finished <= 1, "only the picture could have arrived");
    // The old laptop keeps the list of what could not be read: every letter, by its cause.
    for f in &m.sent.files {
        let item = m.record.items.iter().find(|i| i.id == f.item).unwrap();
        if shown_path(&item.path).contains("letter") {
            match &f.outcome {
                Some(pctwin_transfer::SendOutcome::Failed(fail)) => {
                    assert_eq!(fail.cause(), Cause::Unreadable, "{:?}", item.path)
                }
                other => panic!("{:?}: {other:?}", item.path),
            }
        }
    }
    let report = for_the_mover(&m.received, ADA);
    assert!(
        report
            .own
            .iter()
            .filter(|l| l.path.contains("letter"))
            .all(|l| l.result == FileResult::Failed)
    );
    let letters_opened: u32 = m
        .opens
        .iter()
        .filter(|(p, _)| p.to_string_lossy().contains("letter"))
        .map(|(_, n)| *n)
        .sum();
    assert!(
        letters_opened <= 5 + 4,
        "stopped after five errors in a row (and at most the four already in flight): {letters_opened}"
    );
    assert!(m.opens.values().all(|n| *n == 1), "{:?}", m.opens);
}

/// "Try again" for some files keeps the careful reading of the drive.
#[test]
fn trying_again_keeps_the_careful_read() {
    let d = drive(1);
    let scan = scan_rescued_drive(d.path(), &identity(), &mut Owners).unwrap();
    let plan = send_plan(
        &scan.record,
        |i| scan.source_of(i),
        |i| LandingPlace::new(role_label(i.place.role), shown_path(&i.path)),
    );
    assert_eq!(plan.read_plan, pctwin_move::ReadPlan::Normal);
    let careful = plan.read_with(scan.read_plan);
    let again = careful.only(&scan.record.items.iter().map(|i| i.id).take(1).collect());
    assert_eq!(again.read_plan, scan.read_plan);
}
