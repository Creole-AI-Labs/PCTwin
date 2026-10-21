//! A laptop that stops dead part way through a copy onto a real FAT or exFAT drive, and the
//! drive then unplugged and plugged back in (security design notes, "Drives with no safe rename"
//! and "Proving a file is PCTwin's, per drive", decided 15 October 2026). CI makes real drives,
//! lists them in PCTWIN_FAT_DIRS (`kind=path`, separated by `|`) and runs this twice: with
//! PCTWIN_FAT_PHASE=before, which runs a separate copy of this program that is killed mid-copy
//! on every drive, then, after every drive is unmounted and mounted again, with
//! PCTWIN_FAT_PHASE=after, which runs recovery. PCTWIN_FAT_STATE is a folder off the drives that
//! survives between the two (the journal lives there, as on a laptop's own disk). After
//! recovery: nothing partly copied under a real name, an honest record, and nothing lost.
//! Without PCTWIN_FAT_DIRS this file has nothing to test.
// Tests make and remove files to test on; the no-file-changes fence is for the shipped code.
#![allow(clippy::disallowed_methods)]

use std::path::{Path, PathBuf};

use pctwin_gate::{Approved, Destination, Destinations, Naming};
use pctwin_journal::{Journal, State};
use pctwin_record::ItemId;
use pctwin_transfer::{Channel, ChannelError, ReceiverSession, SendJob, SenderSession, Tier};
use tokio::sync::mpsc;

mod common;

const SIZE: usize = 2 * 1024 * 1024;
const NAME: &str = "Photos/May?.jpg";

fn drives() -> Vec<(String, PathBuf)> {
    let Ok(list) = std::env::var("PCTWIN_FAT_DIRS") else {
        assert!(
            std::env::var_os("PCTWIN_REQUIRE_FAT").is_none(),
            "PCTWIN_REQUIRE_FAT is set but PCTWIN_FAT_DIRS is not"
        );
        return Vec::new();
    };
    list.split('|')
        .filter(|s| !s.is_empty())
        .map(|s| match s.split_once('=') {
            Some((kind, path)) => (kind.to_string(), PathBuf::from(path)),
            None => ("unknown".to_string(), PathBuf::from(s)),
        })
        .collect()
}

fn phase() -> String {
    std::env::var("PCTWIN_FAT_PHASE").unwrap_or_default()
}

fn state(kind: &str) -> PathBuf {
    let base = std::env::var_os("PCTWIN_FAT_STATE").map_or_else(std::env::temp_dir, PathBuf::from);
    let dir = base.join(format!("kill-{kind}"));
    std::fs::create_dir_all(&dir).unwrap();
    dir
}

fn data() -> Vec<u8> {
    (0..SIZE)
        .map(|i| (i as u8).wrapping_mul(29).wrapping_add(i as u8 >> 3))
        .collect()
}

fn table(folder: &Path) -> Destinations {
    let mut t = Destinations::new();
    t.approve("usb", Approved::ChosenDrive, folder).unwrap();
    t
}

/// The test folder on a drive (fresh in the before phase).
fn folder(drive: &Path, fresh: bool) -> PathBuf {
    let dir = drive.join("pctwin-fat-kill");
    if fresh {
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(&dir).unwrap();
    }
    dir
}

/// Every name under `dir`, apart from the Mac's own `._` attribute files.
fn files_under(dir: &Path) -> Vec<(String, Vec<u8>)> {
    let mut out = Vec::new();
    for e in std::fs::read_dir(dir).unwrap() {
        let e = e.unwrap();
        let name = e.file_name().to_string_lossy().into_owned();
        if name.starts_with("._") {
            continue;
        }
        if e.file_type().unwrap().is_dir() {
            for (n, b) in files_under(&e.path()) {
                out.push((format!("{name}/{n}"), b));
            }
        } else {
            out.push((name, std::fs::read(e.path()).unwrap()));
        }
    }
    out.sort();
    out
}

#[test]
fn a_copy_killed_mid_way_and_a_remount_leave_nothing_partial_and_lose_nothing() {
    match phase().as_str() {
        "after" => {
            for (kind, drive) in drives() {
                after(&kind, &drive);
            }
        }
        // Run alone (not by CI's two phases), the drive is not remounted in between.
        p => {
            for (kind, drive) in drives() {
                before(&kind, &drive);
                if p != "before" {
                    after(&kind, &drive);
                }
            }
        }
    }
}

fn before(kind: &str, drive: &Path) {
    let dir = folder(drive, true);
    let st = state(kind);
    let _ = std::fs::remove_file(st.join("j.redb"));
    let naming = table(&dir)
        .get("usb")
        .unwrap()
        .check_drive()
        .unwrap()
        .naming;
    println!("{kind}: this drive names files: {naming:?}");
    let status = std::process::Command::new(std::env::current_exe().unwrap())
        .args(["--ignored", "--exact", "killed_mid_copy", "--nocapture"])
        .env("PCTWIN_KILL_DRIVE", &dir)
        .env("PCTWIN_KILL_STATE", &st)
        .env(
            "PCTWIN_KILL_FILL",
            if naming == Naming::FillIn { "1" } else { "0" },
        )
        .status()
        .unwrap();
    assert!(!status.success(), "{kind}: the copy was meant to stop dead");
    let journal = Journal::open(&st.join("j.redb")).unwrap();
    let e = &journal.entries().unwrap()[0];
    println!("{kind}: stopped at {:?}", step(&e.state));
    let dest = Destination::open(&dir).unwrap();
    for (name, bytes) in files_under(&dir) {
        let hidden = dest.hidden(&name).unwrap_or(false);
        println!(
            "{kind}: on the drive before the remount: {name} ({} bytes, hidden: {hidden})",
            bytes.len()
        );
        // A copy killed while it was filled in under its name is never seen as an ordinary file:
        // it is hidden from the moment it is made.
        let pctwin_own = name
            .rsplit('/')
            .next()
            .unwrap_or("")
            .starts_with(".pctwin-");
        if !pctwin_own && bytes != data() {
            assert!(
                hidden,
                "{kind}: a partly copied file is visible under {name}"
            );
        }
    }
}

fn step(s: &State) -> &'static str {
    match s {
        State::Planned => "planned",
        State::Staged { .. } => "staged",
        State::Verified { .. } => "verified",
        State::Applied { .. } => "applied",
        State::Filling { .. } => "filling",
        State::Filled { .. } => "filled",
        State::Committed { .. } => "committed",
        State::Existing { .. } => "existing",
        State::Failed { .. } => "failed",
    }
}

fn after(kind: &str, drive: &Path) {
    let dir = folder(drive, false);
    let st = state(kind);
    let journal = Journal::open(&st.join("j.redb")).unwrap();
    let table = table(&dir);
    let r = pctwin_transfer::recover(&journal, &table).unwrap();
    println!("{kind}: recovery: {r:?}");
    let want = data();
    // Nothing lost: the original is still on the "old laptop".
    assert_eq!(
        std::fs::read(st.join("source.bin")).unwrap(),
        want,
        "{kind}"
    );
    let e = &journal.entries().unwrap()[0];
    println!("{kind}: now {:?}", step(&e.state));
    for (name, bytes) in files_under(&dir) {
        let pctwin_own = name
            .rsplit('/')
            .next()
            .unwrap_or("")
            .starts_with(".pctwin-");
        println!(
            "{kind}: on the drive after recovery: {name} ({} bytes)",
            bytes.len()
        );
        // Nothing partly copied under a real name: every file under a real name is the whole
        // file. (PCTwin's own `.pctwin-` names may hold a partial copy kept to continue.)
        if !pctwin_own {
            assert_eq!(
                bytes.len(),
                want.len(),
                "{kind}: a partial copy under {name}"
            );
            assert!(bytes == want, "{kind}: other bytes under {name}");
        }
    }
    // An honest record: committed only with the whole file under its recorded name; anything
    // else is unfinished with a plain reason, or kept to continue.
    match &e.state {
        State::Committed { final_path, .. } => {
            assert_eq!(std::fs::read(dir.join(final_path)).unwrap(), want, "{kind}");
            assert_eq!(r.committed, [e.id], "{kind}");
            // Committed means the person's copy: visible, nothing left to show, nothing to say.
            let dest = Destination::open(&dir).unwrap();
            assert!(
                !dest.hidden(final_path).unwrap(),
                "{kind}: committed but left hidden: {r:?}"
            );
            assert!(journal.unshown().unwrap().is_empty(), "{kind}: {r:?}");
            assert!(journal.notes(e.id).unwrap().is_empty(), "{kind}: {r:?}");
        }
        State::Failed { why, .. } => assert!(!why.is_empty(), "{kind}"),
        State::Staged { .. } => assert!(
            r.resumable.contains(&e.id) || r.left.iter().any(|(id, _)| *id == e.id),
            "{kind}: {r:?}"
        ),
        other => assert!(
            r.left
                .iter()
                .any(|(id, why)| *id == e.id && !why.is_empty()),
            "{kind}: left at {} without a reason: {r:?}",
            step(other)
        ),
    }
}

/// A whole move onto each real drive, never stopped: reported copied in the same move, and the
/// file visible under its name with nothing left to show (on a drive with no safe rename it is
/// filled in under its name, so this proves the show works on the real drive).
#[tokio::test]
async fn a_whole_move_onto_a_real_drive_lands_visible_and_reported_copied() {
    if phase() == "after" {
        return;
    }
    for (kind, drive) in drives() {
        let dir = drive.join("pctwin-fat-live");
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(&dir).unwrap();
        let st = state(&format!("live-{kind}"));
        let _ = std::fs::remove_file(st.join("j.redb"));
        let journal = Journal::open(&st.join("j.redb")).unwrap();
        let table = table(&dir);
        let naming = table.get("usb").unwrap().check_drive().unwrap().naming;
        if naming == Naming::NoSafeMethod {
            println!("{kind}: refused, as it must be");
            continue;
        }
        let source = st.join("source.bin");
        std::fs::write(&source, data()).unwrap();
        let job = SendJob {
            size: 0,
            item: ItemId::from_hex(&format!("02{}", "0".repeat(30))).unwrap(),
            source,
            destination: "usb".into(),
            path: NAME.into(),
            compressible: false,
            tier: Tier::Rest,
        };
        let plan = common::approved_jobs(&[(&job, SIZE as u64)]);
        let mut receiver = ReceiverSession::new(&table, plan, &journal, "501");
        let mut sender = SenderSession::new(vec![job.clone()], 2);
        let (a_tx, b_rx) = mpsc::unbounded_channel();
        let (b_tx, a_rx) = mpsc::unbounded_channel();
        let mut a = Mem {
            tx: a_tx,
            rx: a_rx,
            die_after: None,
            got: 0,
        };
        let mut b = Mem {
            tx: b_tx,
            rx: b_rx,
            die_after: None,
            got: 0,
        };
        let (_, received) = tokio::join!(sender.run(&mut a), receiver.run(&mut b));
        received.unwrap();
        let outcome = receiver.outcome(job.item);
        println!("{kind}: names by {naming:?}; outcome {outcome:?}");
        let Some(pctwin_transfer::ReceiveOutcome::Finished(done)) = outcome else {
            panic!("{kind}: not reported copied: {outcome:?}");
        };
        let dest = table.get("usb").unwrap();
        assert_eq!(
            std::fs::read(dir.join(&done.final_path)).unwrap(),
            data(),
            "{kind}"
        );
        assert!(
            !dest.hidden(&done.final_path).unwrap(),
            "{kind}: left hidden"
        );
        assert!(journal.unshown().unwrap().is_empty(), "{kind}");
        let id = journal.entries().unwrap()[0].id;
        assert!(journal.notes(id).unwrap().is_empty(), "{kind}");
    }
}

struct Mem {
    tx: mpsc::UnboundedSender<Vec<u8>>,
    rx: mpsc::UnboundedReceiver<Vec<u8>>,
    /// The receiving end stops dead after this many messages (none: never).
    die_after: Option<usize>,
    got: usize,
}

impl Channel for Mem {
    async fn send(&mut self, data: &[u8]) -> Result<(), ChannelError> {
        tokio::task::yield_now().await;
        self.tx.send(data.to_vec()).map_err(|_| ChannelError)
    }
    async fn recv(&mut self) -> Result<Vec<u8>, ChannelError> {
        let m = self.rx.recv().await.ok_or(ChannelError)?;
        self.got += 1;
        if self.die_after.is_some_and(|n| self.got >= n) {
            std::process::abort();
        }
        Ok(m)
    }
}

/// The part run as a separate program: a move onto the drive that is killed part way through
/// the copy (while the file is filled in under its name, on a drive with no safe rename; else
/// while it is received).
#[tokio::test]
#[ignore = "run by a_copy_killed_mid_way_and_a_remount_leave_nothing_partial_and_lose_nothing"]
async fn killed_mid_copy() {
    let dir = PathBuf::from(std::env::var_os("PCTWIN_KILL_DRIVE").expect("run by its parent"));
    let st = PathBuf::from(std::env::var_os("PCTWIN_KILL_STATE").unwrap());
    let fill = std::env::var("PCTWIN_KILL_FILL").as_deref() == Ok("1");
    let journal = Journal::open(&st.join("j.redb")).unwrap();
    let mut dest = Destination::open(&dir).unwrap();
    if fill {
        dest = dest.with_fill_hook(256 * 1024, |written| {
            if written >= 768 * 1024 {
                // Dead: nothing is tidied on the way out.
                std::process::abort();
            }
            Ok(())
        });
    }
    let mut table = Destinations::new();
    table
        .approve_opened("usb", Approved::ChosenDrive, dest)
        .unwrap();
    let source = st.join("source.bin");
    std::fs::write(&source, data()).unwrap();
    let job = SendJob {
        size: 0,
        item: ItemId::from_hex(&format!("01{}", "0".repeat(30))).unwrap(),
        source,
        destination: "usb".into(),
        path: NAME.into(),
        compressible: false,
        tier: Tier::Rest,
    };
    let plan = common::approved_jobs(&[(&job, SIZE as u64)]);
    let mut receiver = ReceiverSession::new(&table, plan, &journal, "501");
    let mut sender = SenderSession::new(vec![job], 2);
    let (a_tx, b_rx) = mpsc::unbounded_channel();
    let (b_tx, a_rx) = mpsc::unbounded_channel();
    let a = Mem {
        tx: a_tx,
        rx: a_rx,
        die_after: None,
        got: 0,
    };
    // Where the file is not filled in, the receiver stops dead part way through the pieces.
    let b = Mem {
        tx: b_tx,
        rx: b_rx,
        die_after: (!fill).then_some(8),
        got: 0,
    };
    let send_side = async {
        let mut a = a;
        sender.run(&mut a).await
    };
    let receive_side = async {
        let mut b = b;
        receiver.run(&mut b).await
    };
    let _ = tokio::join!(send_side, receive_side);
    // A fill-in drive that finished without its hook (a file too small) still stops dead here.
    std::process::abort();
}
