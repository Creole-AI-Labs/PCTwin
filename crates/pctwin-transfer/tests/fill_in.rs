//! A move onto a drive with no safe rename (exFAT on a Mac; security design notes, "Drives with
//! no safe rename", decided 15 October 2026), forced here on the test machine's own drive through
//! a drive profile so every system runs it. Each file is filled in under its real name through
//! the journal; if the journal fails at any step, or the laptop stops part way through the copy,
//! recovery ends with the whole file under its name, nothing partly copied under a real name,
//! nothing of PCTwin's left behind, and an honest record.
// Tests make and remove files to test on; the no-file-changes fence is for the shipped code.
#![allow(clippy::disallowed_methods)]

use std::path::{Path, PathBuf};

use pctwin_gate::{Approved, Destination, Destinations, DriveKind, DriveProfile, Naming};
use pctwin_journal::{FileId, Journal, JournalError, Landed, Ledger, PlannedWrite, State};
use pctwin_record::ItemId;
use pctwin_transfer::{
    Channel, ChannelError, ReceiverSession, SendJob, SenderSession, Tier, TransferError,
};
use tokio::sync::mpsc;

mod common;

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

fn item(n: u8) -> ItemId {
    ItemId::from_hex(&format!("{n:02x}{}", "0".repeat(30))).unwrap()
}

fn data(len: usize) -> Vec<u8> {
    (0..len)
        .map(|i| (i as u8).wrapping_mul(13).wrapping_add(5))
        .collect()
}

/// The drive profile of a drive with no safe rename.
fn no_safe_rename() -> DriveProfile {
    DriveProfile::of(DriveKind::ExFat).with_naming(Naming::FillIn)
}

fn table_for(dest: Destination) -> Destinations {
    let mut table = Destinations::new();
    table
        .approve_opened("usb", Approved::ChosenDrive, dest)
        .unwrap();
    table
}

/// Sends `bytes` from a file on the "old laptop" folder `old` to `usb/<path>`.
async fn send(
    old: &Path,
    bytes: &[u8],
    path: &str,
    table: &Destinations,
    journal: &dyn Ledger,
) -> Result<(), TransferError> {
    let source = old.join("source.bin");
    std::fs::write(&source, bytes).unwrap();
    let job = SendJob {
        item: item(1),
        source,
        destination: "usb".into(),
        path: path.into(),
        compressible: false,
        tier: Tier::Rest,
    };
    let plan = common::approved_jobs(&[(&job, bytes.len() as u64)]);
    let mut receiver = ReceiverSession::new(table, plan, journal, "501");
    let mut sender = SenderSession::new(vec![job], 2);
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
    let (_, received) = tokio::time::timeout(std::time::Duration::from_secs(30), async {
        tokio::join!(send_side, receive_side)
    })
    .await
    .expect("the move hung");
    received
}

fn names(dir: &Path) -> Vec<String> {
    let mut out: Vec<String> = std::fs::read_dir(dir)
        .unwrap()
        .map(|e| e.unwrap().file_name().to_string_lossy().into_owned())
        .collect();
    out.sort();
    out
}

/// After everything: one committed write, its whole file under its name, and nothing else.
fn assert_landed_whole(journal: &Journal, dir: &Path, name: &str, bytes: &[u8]) {
    let all = journal.entries().unwrap();
    assert_eq!(all.len(), 1, "{all:?}");
    assert!(
        matches!(&all[0].state, State::Committed { final_path, .. } if final_path == name),
        "{:?}",
        all[0].state
    );
    assert!(journal.unfinished().unwrap().is_empty());
    assert_eq!(std::fs::read(dir.join(name)).unwrap(), bytes);
    assert_eq!(names(dir), [name]);
}

#[tokio::test]
async fn a_move_onto_a_drive_with_no_safe_rename_lands_whole_through_the_journal() {
    let old = tempfile::tempdir().unwrap();
    let usb = tempfile::tempdir().unwrap();
    let jdir = tempfile::tempdir().unwrap();
    let journal = Journal::open(&jdir.path().join("j.redb")).unwrap();
    let table = table_for(
        Destination::open(usb.path())
            .unwrap()
            .with_profile(no_safe_rename()),
    );
    let bytes = data(300_000);
    send(old.path(), &bytes, "a.bin", &table, &journal)
        .await
        .unwrap();
    assert_landed_whole(&journal, usb.path(), "a.bin", &bytes);
    let e = &journal.entries().unwrap()[0];
    let State::Committed { landed, .. } = &e.state else {
        unreachable!()
    };
    let stat = table.get("usb").unwrap().stat("a.bin").unwrap().unwrap();
    assert_eq!(landed.file, Some(stat.id));
    assert_eq!(landed.size, bytes.len() as u64);
}

/// A journal that stops working at one step, as a full or failing system drive would.
struct Breaks<'j> {
    inner: &'j Journal,
    at: &'static str,
}

impl Breaks<'_> {
    fn check(&self, step: &str) -> Result<(), JournalError> {
        if step == self.at {
            Err(JournalError::Storage("the drive is full".into()))
        } else {
            Ok(())
        }
    }
}

impl Ledger for Breaks<'_> {
    fn temp_tag(&self, id: u64) -> String {
        self.inner.temp_tag(id)
    }
    fn plan(&self, w: &PlannedWrite) -> Result<u64, JournalError> {
        self.inner.plan(w)
    }
    fn staged(
        &self,
        id: u64,
        temp: &str,
        made: &[(String, Option<FileId>)],
    ) -> Result<(), JournalError> {
        self.inner.staged(id, temp, made)
    }
    fn verified(&self, id: u64, fp: [u8; 32], file: Option<FileId>) -> Result<(), JournalError> {
        self.inner.verified(id, fp, file)
    }
    fn applied(&self, id: u64, final_path: &str) -> Result<(), JournalError> {
        self.check("applied")?;
        self.inner.applied(id, final_path)
    }
    fn filling(
        &self,
        id: u64,
        final_path: &str,
        tag: pctwin_journal::FillTag,
        proof: pctwin_journal::Proof,
    ) -> Result<(), JournalError> {
        self.check("filling")?;
        self.inner.filling(id, final_path, tag, proof)
    }
    fn fill_made(&self, id: u64, made: FileId) -> Result<(), JournalError> {
        self.check("fill-made")?;
        self.inner.fill_made(id, made)
    }
    fn filled(&self, id: u64) -> Result<(), JournalError> {
        self.check("filled")?;
        self.inner.filled(id)
    }
    fn fill_cleared(&self, id: u64) -> Result<(), JournalError> {
        self.check("fill-cleared")?;
        self.inner.fill_cleared(id)
    }
    fn committed(&self, id: u64, landed: Landed) -> Result<(), JournalError> {
        self.check("committed")?;
        self.inner.committed(id, landed)
    }
    fn existing(&self, id: u64, stored: &str) -> Result<(), JournalError> {
        self.inner.existing(id, stored)
    }
    fn failed(&self, id: u64, why: &str) -> Result<(), JournalError> {
        self.inner.failed(id, why)
    }
    fn checkpoint(
        &self,
        id: u64,
        blocks: &[(u64, [u8; 32])],
        durable: bool,
    ) -> Result<(), JournalError> {
        self.inner.checkpoint(id, blocks, durable)
    }
    fn blocks(&self, id: u64) -> Result<Vec<(u64, [u8; 32])>, JournalError> {
        self.inner.blocks(id)
    }
    fn unfinished(&self) -> Result<Vec<pctwin_journal::Entry>, JournalError> {
        self.inner.unfinished()
    }
}

/// A drive with no safe rename where a file filled in is provable after a restart: by its
/// creation-time tag where the system sets one (Windows, macOS), else (Linux) by an identity the
/// drive keeps.
fn provable() -> DriveProfile {
    let kind = if cfg!(any(windows, target_os = "macos")) {
        DriveKind::ExFat
    } else {
        DriveKind::Ext4
    };
    DriveProfile::of(kind).with_naming(Naming::FillIn)
}

/// The journal fails at each step of a fill in turn; the move stops; recovery then ends with
/// the whole file under its name and nothing else left.
#[tokio::test]
async fn a_journal_that_fails_at_any_step_of_a_fill_is_recovered_whole() {
    for at in [
        "filling",
        "fill-made",
        "filled",
        "fill-cleared",
        "committed",
    ] {
        let old = tempfile::tempdir().unwrap();
        let usb = tempfile::tempdir().unwrap();
        let jdir = tempfile::tempdir().unwrap();
        let journal = Journal::open(&jdir.path().join("j.redb")).unwrap();
        let table = table_for(
            Destination::open(usb.path())
                .unwrap()
                .with_profile(provable()),
        );
        let bytes = data(200_000);
        let breaks = Breaks {
            inner: &journal,
            at,
        };
        let sent = send(old.path(), &bytes, "b.bin", &table, &breaks).await;
        if at != "fill-cleared" {
            assert!(sent.is_err(), "{at}: the move went on without its journal");
        }
        let r = pctwin_transfer::recover(&journal, &table).unwrap();
        assert!(r.failed.is_empty() && r.left.is_empty(), "{at}: {r:?}");
        assert_landed_whole(&journal, usb.path(), "b.bin", &bytes);
    }
}

/// The laptop stops dead part way through filling a file in (this test runs itself again as a
/// separate program that is killed mid-copy, so nothing is tidied on the way out, as with a
/// power cut). Recovery removes PCTwin's own partial copy from the real name and names the
/// checked file again: the whole file, nothing partial, nothing left.
#[test]
fn a_laptop_that_stops_dead_mid_fill_is_recovered_whole() {
    let work = tempfile::tempdir().unwrap();
    let (old, usb, jdir) = dirs(work.path());
    let status = std::process::Command::new(std::env::current_exe().unwrap())
        .args(["--ignored", "--exact", "stops_dead_mid_fill", "--nocapture"])
        .env("PCTWIN_KILL_WORK", work.path())
        .status()
        .unwrap();
    assert!(!status.success(), "the copy was meant to stop dead");
    // Stopped mid-copy: PCTwin's partial copy is under the real name, the journal says filling.
    let journal = Journal::open(&jdir.join("j.redb")).unwrap();
    let e = &journal.entries().unwrap()[0];
    assert!(
        matches!(&e.state, State::Filling { fill, .. } if fill.made.is_some()),
        "{:?}",
        e.state
    );
    let partial = std::fs::metadata(usb.join("c.bin")).unwrap().len();
    assert!(partial > 0 && partial < BIG as u64, "{partial}");
    let table = table_for(
        Destination::open(&usb)
            .unwrap()
            .with_profile(no_safe_rename()),
    );
    let r = pctwin_transfer::recover(&journal, &table).unwrap();
    // Linux keeps no creation time it can set, so nothing proves the partial copy PCTwin's after
    // a restart there: it is left and the person told, and the file lands beside it.
    if cfg!(any(windows, target_os = "macos")) {
        assert!(
            r.failed.is_empty() && r.left.is_empty() && r.notes.is_empty(),
            "{r:?}"
        );
        assert_landed_whole(&journal, &usb, "c.bin", &data(BIG));
    } else {
        assert_eq!(r.notes.len(), 1, "{r:?}");
        assert!(r.notes[0].1.contains("c.bin"), "{r:?}");
        assert_eq!(std::fs::read(usb.join("c (2).bin")).unwrap(), data(BIG));
        assert_eq!(names(&usb), ["c (2).bin", "c.bin"]);
    }
    let _ = old;
}

const BIG: usize = 400_000;

fn dirs(work: &Path) -> (PathBuf, PathBuf, PathBuf) {
    let [old, usb, j] = ["old", "usb", "journal"].map(|d| work.join(d));
    for d in [&old, &usb, &j] {
        std::fs::create_dir_all(d).unwrap();
    }
    (old, usb, j)
}

/// The part run as a separate program by the test above: a move that is killed part way
/// through filling its file in.
#[tokio::test]
#[ignore = "run by a_laptop_that_stops_dead_mid_fill_is_recovered_whole"]
async fn stops_dead_mid_fill() {
    let work = PathBuf::from(std::env::var_os("PCTWIN_KILL_WORK").expect("run by its parent"));
    let (old, usb, jdir) = dirs(&work);
    let journal = Journal::open(&jdir.join("j.redb")).unwrap();
    let table = table_for(
        Destination::open(&usb)
            .unwrap()
            .with_profile(no_safe_rename())
            .with_fill_hook(64 * 1024, |written| {
                if written >= 128 * 1024 {
                    // Dead: no clean-up of any kind runs.
                    std::process::abort();
                }
                Ok(())
            }),
    );
    let _ = send(&old, &data(BIG), "c.bin", &table, &journal).await;
    unreachable!("the copy was meant to stop dead");
}
