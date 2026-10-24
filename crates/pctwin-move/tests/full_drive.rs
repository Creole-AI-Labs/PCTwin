//! A whole move into a drive that fills up part way (Task List 1.8): every file that landed is
//! whole and finished; every one that did not fit ends with the plain cause (no space), never as
//! finished; nothing partly written or temporary is left; the report never says everything was
//! copied; and once space is freed, moving again completes the rest with nothing sent twice.
//!
//! It needs a small real drive of its own, named by `PCTWIN_SMALL_DRIVE` (a folder on it the
//! test may fill and empty; CI makes one in the real-drives job). Without one it says so and
//! does nothing.
// Tests make files to test on; the no-file-changes fence is for the shipped code.
#![allow(clippy::disallowed_methods)]

use std::path::{Path, PathBuf};
use std::sync::Arc;
use std::sync::atomic::{AtomicU64, Ordering};
use std::time::Duration;

use pctwin_gate::{Approved, Destinations};
use pctwin_journal::Journal;
use pctwin_move::report::{FileResult, for_the_mover};
use pctwin_move::{Live, Preview, Steps, Stop, send_plan};
use pctwin_record::{
    FolderRole, Inclusion, Item, ItemId, ItemKind, ItemName, ItemPath, LaptopId, ManagedBy, Owner,
    PersonTarget, Place, Portability, Record, Storage,
};
use pctwin_transfer::{
    Cause, Channel, ChannelError, LandingPlace, ReceiveOutcome, role_label, shown_path,
};
use tokio::sync::mpsc;

const HANG_AFTER: Duration = Duration::from_secs(600);

async fn no_hang<T>(work: impl std::future::Future<Output = T>) -> T {
    match tokio::time::timeout(HANG_AFTER, work).await {
        Ok(done) => done,
        Err(_) => panic!("hung: still waiting after {HANG_AFTER:?}"),
    }
}

/// An in-memory connection that counts the bytes of files the old laptop sends.
struct Mem {
    tx: mpsc::UnboundedSender<Vec<u8>>,
    rx: mpsc::UnboundedReceiver<Vec<u8>>,
    sent: Arc<AtomicU64>,
}

impl Channel for Mem {
    async fn send(&mut self, data: &[u8]) -> Result<(), ChannelError> {
        tokio::task::yield_now().await;
        // Only the files' own pieces (not the list, the link test or the answers).
        if let Ok(pctwin_transfer::Message::Piece { bytes, .. }) =
            pctwin_transfer::Message::decode(data)
        {
            self.sent.fetch_add(bytes.len() as u64, Ordering::Relaxed);
        }
        self.tx.send(data.to_vec()).map_err(|_| ChannelError)
    }
    async fn recv(&mut self) -> Result<Vec<u8>, ChannelError> {
        self.rx.recv().await.ok_or(ChannelError)
    }
}

fn mem_pair() -> (Mem, Mem, Arc<AtomicU64>) {
    let (a_tx, b_rx) = mpsc::unbounded_channel();
    let (b_tx, a_rx) = mpsc::unbounded_channel();
    let sent = Arc::new(AtomicU64::new(0));
    (
        Mem {
            tx: a_tx,
            rx: a_rx,
            sent: sent.clone(),
        },
        Mem {
            tx: b_tx,
            rx: b_rx,
            sent: Arc::new(AtomicU64::new(0)),
        },
        sent,
    )
}

fn laptop() -> LaptopId {
    LaptopId::from_hex("00112233445566778899aabbccddeeff").unwrap()
}

fn photo(n: usize, size: u64) -> Item {
    let owner = Owner::Person {
        account_id: "1000".into(),
    };
    let place = Place {
        role: FolderRole::Pictures,
        storage: Storage::SystemDrive,
    };
    let path = ItemPath::new(vec![ItemName::from_text(&format!("photo {n:02}.jpg"))]).unwrap();
    Item {
        id: ItemId::derive(&laptop(), &owner, &place, &path),
        kind: ItemKind::File,
        owner,
        place,
        path,
        size_bytes: size,
        modified_ns: None,
        portability: Portability::Portable,
        managed_by: ManagedBy::Personal,
        download_bytes: None,
        inclusion: Inclusion::Included,
    }
}

/// Bytes that do not compress (a photo's), so what crosses is what is sent.
fn noise(len: usize, seed: u32) -> Vec<u8> {
    let mut x = seed.wrapping_mul(2_654_435_761) | 1;
    (0..len)
        .map(|_| {
            x ^= x << 13;
            x ^= x >> 17;
            x ^= x << 5;
            (x >> 24) as u8
        })
        .collect()
}

struct Person;

impl Steps for Person {
    async fn person(&mut self, _: &Record, _: &str) -> Result<PersonTarget, Stop> {
        Ok(PersonTarget::ExistingAccount {
            account_id: "1001".into(),
        })
    }
    // Approved even though it does not all fit: the person is told and goes ahead.
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

/// Free bytes on the drive holding `dir`, as the gate reads them.
fn free_on(dir: &Path) -> u64 {
    let mut table = Destinations::new();
    table.approve("probe", Approved::MyFolders, dir).unwrap();
    table
        .get("probe")
        .unwrap()
        .available_bytes()
        .expect("the drive's free space")
}

/// One move of `record` into `dest`: the new laptop's outcome per file, and how many bytes the
/// old laptop sent.
async fn move_once(
    record: &Record,
    old: &Path,
    dest: &Path,
    journal: &Journal,
) -> (pctwin_move::Received, u64) {
    let mut table = Destinations::new();
    table
        .approve(role_label(FolderRole::Pictures), Approved::MyFolders, dest)
        .unwrap();
    let plan = send_plan(
        record,
        |i| Some(old.join(shown_path(&i.path))),
        |i| LandingPlace::new(role_label(i.place.role), shown_path(&i.path)),
    );
    let (mut a, mut b, sent) = mem_pair();
    let (quiet, quiet_old) = (Live::default(), Live::default());
    let mut steps = Person;
    let (s, r) = no_hang(async {
        tokio::join!(
            pctwin_move::send(&mut a, record, plan, &quiet_old),
            pctwin_move::receive(&mut b, &mut steps, &table, journal, "1001", &quiet),
        )
    })
    .await;
    s.unwrap();
    (r.unwrap(), sent.load(Ordering::Relaxed))
}

#[tokio::test]
async fn a_drive_that_fills_up_part_way_ends_honestly_and_the_rest_moves_once_freed() {
    let Some(drive) = std::env::var_os("PCTWIN_SMALL_DRIVE").map(PathBuf::from) else {
        eprintln!("not reachable: no small drive (PCTWIN_SMALL_DRIVE) to fill");
        return;
    };
    let dest = drive.join("full-drive-test");
    let _ = std::fs::remove_dir_all(&dest);
    std::fs::create_dir_all(&dest).unwrap();
    let filler = drive.join("full-drive-filler.bin");
    let _ = std::fs::remove_file(&filler);

    const FILES: usize = 10;
    const SIZE: usize = 600 * 1024;
    // Room for about half of them: the rest of the drive is filled first.
    let room = (FILES as u64 * SIZE as u64) / 2;
    let free = free_on(&drive);
    assert!(
        free > room + 1024 * 1024,
        "the drive is too small: {free} bytes free"
    );
    std::fs::write(&filler, vec![0u8; (free - room) as usize]).unwrap();

    let old = tempfile::tempdir().unwrap();
    let jdir = tempfile::tempdir().unwrap();
    let journal = Journal::open(&jdir.path().join("journal.redb")).unwrap();
    let mut record = Record::new(laptop());
    for n in 0..FILES {
        let item = photo(n, SIZE as u64);
        std::fs::write(
            old.path().join(shown_path(&item.path)),
            noise(SIZE, n as u32),
        )
        .unwrap();
        record.items.push(item);
    }
    record.validate().unwrap();

    // The first move: the drive fills up part way.
    let (first, _) = move_once(&record, old.path(), &dest, &journal).await;
    let mut landed = Vec::new();
    for (line, item) in first.files.iter().zip(&record.items) {
        match &line.outcome {
            Some(ReceiveOutcome::Finished(done)) => {
                let on_disk = std::fs::read(dest.join(&done.final_path)).unwrap();
                assert_eq!(on_disk.len(), SIZE, "{} landed partly", done.final_path);
                landed.push(item.id);
            }
            Some(ReceiveOutcome::Failed(f)) => {
                assert_eq!(f.cause(), Cause::NoSpace, "{f:?}");
            }
            other => panic!("{:?}: {other:?}", item.path),
        }
    }
    assert!(!landed.is_empty(), "nothing fitted");
    assert!(
        landed.len() < FILES,
        "everything fitted: the drive never filled"
    );
    // Nothing partly written or temporary is left: only the files that landed, whole.
    let names: Vec<String> = std::fs::read_dir(&dest)
        .unwrap()
        .flatten()
        .map(|e| e.file_name().to_string_lossy().into_owned())
        .collect();
    assert_eq!(names.len(), landed.len(), "left on the drive: {names:?}");
    // The report never says everything was copied.
    let report = for_the_mover(&first, "1000");
    let failed = report
        .own
        .iter()
        .filter(|l| l.result == FileResult::Failed)
        .count();
    assert_eq!(failed, FILES - landed.len());
    assert!(
        report
            .own
            .iter()
            .filter(|l| l.result == FileResult::Failed)
            .all(|l| l.why.as_ref().is_some_and(|(c, _)| *c == Cause::NoSpace)),
        "{:?}",
        report.own
    );

    // Space is freed and the move is made again: the rest arrives, and what landed the first
    // time is not sent again.
    std::fs::remove_file(&filler).unwrap();
    let (second, sent) = move_once(&record, old.path(), &dest, &journal).await;
    for (line, item) in second.files.iter().zip(&record.items) {
        match &line.outcome {
            Some(ReceiveOutcome::Finished(_)) => assert!(!landed.contains(&item.id)),
            Some(ReceiveOutcome::AlreadyThere(_)) => assert!(landed.contains(&item.id)),
            other => panic!("{:?}: {other:?}", item.path),
        }
    }
    let missing = (FILES - landed.len()) as u64 * SIZE as u64;
    assert!(
        sent < missing + missing / 4 + 256 * 1024,
        "{sent} bytes sent for {missing} bytes missing: something was sent twice"
    );
    for item in &record.items {
        let n: usize = shown_path(&item.path)[6..8].parse().unwrap();
        assert_eq!(
            std::fs::read(dest.join(shown_path(&item.path))).unwrap(),
            noise(SIZE, n as u32)
        );
    }
    let _ = std::fs::remove_dir_all(&dest);
}
