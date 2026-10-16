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

/// The drive profile of a drive with no safe rename, where a filled-in file can be proven
/// PCTwin's after a restart (see `provable`).
fn no_safe_rename() -> DriveProfile {
    provable()
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
    assert!(journal.unshown().unwrap().is_empty(), "every copy is shown");
    assert_eq!(std::fs::read(dir.join(name)).unwrap(), bytes);
    assert_eq!(names(dir), [name]);
    assert!(
        !Destination::open(dir).unwrap().hidden(name).unwrap(),
        "the finished file is shown"
    );
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

/// A journal that stops working at one step, as a full or failing system drive would; or that
/// records one step and then the laptop stops dead (`dies_after`).
struct Breaks<'j> {
    inner: &'j Journal,
    at: &'static str,
    dies_after: &'static str,
}

impl Breaks<'_> {
    fn check(&self, step: &str) -> Result<(), JournalError> {
        if step == self.at {
            Err(JournalError::Storage("the drive is full".into()))
        } else {
            Ok(())
        }
    }
    /// After `step` is recorded: stops dead there if asked to.
    fn then(&self, step: &str, r: Result<(), JournalError>) -> Result<(), JournalError> {
        if r.is_ok() && step == self.dies_after {
            // Dead: nothing is tidied on the way out.
            std::process::abort();
        }
        r
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
        self.then("filled", self.inner.filled(id))
    }
    fn fill_cleared(&self, id: u64, left: Option<&str>) -> Result<(), JournalError> {
        self.check("fill-cleared")?;
        self.inner.fill_cleared(id, left)
    }
    fn committed_showing(
        &self,
        id: u64,
        landed: Landed,
        show: &pctwin_journal::Show,
    ) -> Result<(), JournalError> {
        self.check("committed")?;
        self.then("committed", self.inner.committed_showing(id, landed, show))
    }
    fn shown(&self, id: u64, left: Option<&str>) -> Result<(), JournalError> {
        self.check("shown")?;
        self.then("shown", self.inner.shown(id, left))
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
        "shown",
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
            dies_after: "",
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
    let State::Filling { fill, .. } = &e.state else {
        unreachable!()
    };
    let tag = fill.tag.born;
    let partial = std::fs::metadata(usb.join("c.bin")).unwrap().len();
    assert!(partial > 0 && partial < BIG as u64, "{partial}");
    let table = table_for(
        Destination::open(&usb)
            .unwrap()
            .with_profile(no_safe_rename()),
    );
    // Never seen as an ordinary file: hidden from the moment it was made (where the system can
    // hide a file; Linux cannot, and never fills files in on a real drive).
    let dest = table.get("usb").unwrap();
    if cfg!(any(windows, target_os = "macos")) {
        assert!(dest.hidden("c.bin").unwrap(), "a killed fill is visible");
    }
    let r = pctwin_transfer::recover(&journal, &table).unwrap();
    assert!(
        r.failed.is_empty() && r.left.is_empty() && r.notes.is_empty(),
        "{r:?}"
    );
    assert_landed_whole(&journal, &usb, "c.bin", &data(BIG));
    assert!(!dest.hidden("c.bin").unwrap(), "the finished file is shown");
    if tag.is_some() {
        assert_ne!(
            dest.born("c.bin").unwrap(),
            tag,
            "no tag left on the person's file"
        );
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

/// The laptop stops dead at each step of finishing a fill: recorded filled; committed (still
/// hidden and tagged); shown; given its modified time; given its creation date back; recorded
/// shown. Each run is a separate copy of this program, stopped dead there, so nothing is tidied.
/// Recovery always ends with exactly one copy: whole, shown, with its real dates (no tag), and
/// nothing left to show.
#[test]
fn a_laptop_that_stops_dead_at_any_step_of_finishing_a_fill_ends_with_one_shown_copy() {
    for at in [
        "journal:filled",
        "journal:committed",
        "show:shown",
        "show:modified",
        "show:created",
        "journal:shown",
    ] {
        let work = tempfile::tempdir().unwrap();
        let (_, usb, jdir) = dirs(work.path());
        let status = std::process::Command::new(std::env::current_exe().unwrap())
            .args([
                "--ignored",
                "--exact",
                "stops_dead_finishing",
                "--nocapture",
            ])
            .env("PCTWIN_KILL_WORK", work.path())
            .env("PCTWIN_DIE_AT", at)
            .status()
            .unwrap();
        assert!(!status.success(), "{at}: the laptop was meant to stop dead");
        let journal = Journal::open(&jdir.join("j.redb")).unwrap();
        let e = &journal.entries().unwrap()[0];
        let tag = match &e.state {
            State::Filled { fill, .. } => fill.tag.born,
            State::Committed { .. } => journal
                .show_pending(e.id)
                .unwrap()
                .and_then(|s| s.fill.tag.born),
            other => panic!("{at}: {other:?}"),
        };
        let table = table_for(Destination::open(&usb).unwrap().with_profile(provable()));
        let dest = table.get("usb").unwrap();
        let r = pctwin_transfer::recover(&journal, &table).unwrap();
        assert!(
            r.failed.is_empty() && r.left.is_empty() && r.notes.is_empty(),
            "{at}: {r:?}"
        );
        assert_landed_whole(&journal, &usb, "c.bin", &data(BIG));
        if tag.is_some() {
            assert_ne!(dest.born("c.bin").unwrap(), tag, "{at}: a tag left behind");
        }
        // Asked again, there is nothing left to do.
        let again = pctwin_transfer::recover(&journal, &table).unwrap();
        assert!(
            again.committed.is_empty() && again.shown.is_empty() && again.notes.is_empty(),
            "{at}: {again:?}"
        );
    }
}

/// The part run as a separate program by the test above: stops dead at PCTWIN_DIE_AT.
#[tokio::test]
#[ignore = "run by a_laptop_that_stops_dead_at_any_step_of_finishing_a_fill_ends_with_one_shown_copy"]
async fn stops_dead_finishing() {
    let work = PathBuf::from(std::env::var_os("PCTWIN_KILL_WORK").expect("run by its parent"));
    let at = std::env::var("PCTWIN_DIE_AT").unwrap();
    let (old, usb, jdir) = dirs(&work);
    let journal = Journal::open(&jdir.join("j.redb")).unwrap();
    let show_step = at.strip_prefix("show:").map(str::to_string);
    let dest = Destination::open(&usb)
        .unwrap()
        .with_profile(provable())
        .with_show_hook(move |step| {
            if show_step.as_deref() == Some(step) {
                std::process::abort();
            }
            Ok(())
        });
    let table = table_for(dest);
    let dies_after: &'static str = match at.strip_prefix("journal:") {
        Some("filled") => "filled",
        Some("committed") => "committed",
        Some("shown") => "shown",
        _ => "",
    };
    let breaks = Breaks {
        inner: &journal,
        at: "",
        dies_after,
    };
    let _ = send(&old, &data(BIG), "c.bin", &table, &breaks).await;
    unreachable!("the laptop was meant to stop dead at {at}");
}

/// The laptop is killed (by its process number, as a power cut or a crash would stop it) at a
/// random moment of a move onto a drive with no safe rename, again and again
/// (PCTWIN_KILL_RUNS, 100 unless set). After each, recovery never leaves a second copy, nor a
/// partial or hidden file under a real name, nor anything still to show.
#[test]
fn a_laptop_killed_at_random_moments_never_ends_with_a_second_copy() {
    use std::hash::{BuildHasher, RandomState};
    let runs: u32 = std::env::var("PCTWIN_KILL_RUNS")
        .ok()
        .and_then(|v| v.parse().ok())
        .unwrap_or(100);
    // The journal already exists, as on any laptop past its first start (a kill while redb first
    // makes its file is not what this tests).
    let child = |work: &Path| {
        let (_, _, jdir) = dirs(work);
        drop(Journal::open(&jdir.join("j.redb")).unwrap());
        std::process::Command::new(std::env::current_exe().unwrap())
            .args(["--ignored", "--exact", "moves_until_killed", "--nocapture"])
            .env("PCTWIN_KILL_WORK", work)
            .stdout(std::process::Stdio::null())
            .stderr(std::process::Stdio::null())
            .spawn()
            .unwrap()
    };
    // How long a whole move takes here, so the kills fall across all of it.
    let whole = {
        let work = tempfile::tempdir().unwrap();
        dirs(work.path());
        let started = std::time::Instant::now();
        assert!(child(work.path()).wait().unwrap().success());
        started.elapsed()
    };
    let random = RandomState::new();
    let (mut committed, mut earlier) = (0, 0);
    for run in 0..runs {
        let work = tempfile::tempdir().unwrap();
        let (_, usb, jdir) = dirs(work.path());
        let mut c = child(work.path());
        let at = whole.mul_f64((random.hash_one(run) % 1000) as f64 / 1000.0);
        std::thread::sleep(at);
        let _ = c.kill();
        let _ = c.wait();
        let journal = Journal::open(&jdir.join("j.redb")).unwrap();
        let table = table_for(Destination::open(&usb).unwrap().with_profile(provable()));
        let dest = table.get("usb").unwrap();
        let r = pctwin_transfer::recover(&journal, &table).unwrap();
        let real: Vec<String> = names(&usb)
            .into_iter()
            .filter(|n| !n.starts_with(".pctwin-"))
            .collect();
        assert!(real.len() <= 1, "run {run} ({at:?}): {real:?}, {r:?}");
        for name in &real {
            assert_eq!(
                std::fs::read(usb.join(name)).unwrap(),
                data(BIG),
                "run {run} ({at:?}): a partial copy under {name}"
            );
            assert!(!dest.hidden(name).unwrap(), "run {run}: {name} left hidden");
        }
        assert!(journal.unshown().unwrap().is_empty(), "run {run}: {r:?}");
        assert!(r.notes.is_empty(), "run {run}: {r:?}");
        match journal.entries().unwrap().first().map(|e| &e.state) {
            Some(State::Committed { .. }) => committed += 1,
            _ => earlier += 1,
        }
    }
    eprintln!(
        "{runs} kills: {committed} ended committed, {earlier} stopped earlier (kept or failed)"
    );
}

/// The part run as a separate program by the test above: a whole move, unless it is killed.
#[tokio::test]
#[ignore = "run by a_laptop_killed_at_random_moments_never_ends_with_a_second_copy"]
async fn moves_until_killed() {
    let work = PathBuf::from(std::env::var_os("PCTWIN_KILL_WORK").expect("run by its parent"));
    let (old, usb, jdir) = dirs(&work);
    let journal = Journal::open(&jdir.join("j.redb")).unwrap();
    let table = table_for(Destination::open(&usb).unwrap().with_profile(provable()));
    send(&old, &data(BIG), "c.bin", &table, &journal)
        .await
        .unwrap();
}

/// Runs the child that stops dead at `at` on a fresh place, and returns the folders and journal.
fn stopped_at(at: &str) -> (tempfile::TempDir, PathBuf, Journal) {
    let work = tempfile::tempdir().unwrap();
    let (_, usb, jdir) = dirs(work.path());
    let status = std::process::Command::new(std::env::current_exe().unwrap())
        .args([
            "--ignored",
            "--exact",
            "stops_dead_finishing",
            "--nocapture",
        ])
        .env("PCTWIN_KILL_WORK", work.path())
        .env("PCTWIN_DIE_AT", at)
        .status()
        .unwrap();
    assert!(!status.success(), "{at}: the laptop was meant to stop dead");
    let journal = Journal::open(&jdir.join("j.redb")).unwrap();
    (work, usb, journal)
}

/// Showing a committed copy fails part way (the drive errs as it is given its modified time):
/// the copy is finished, but its show stays pending, never recorded shown, and the next start
/// shows it with its real dates.
#[tokio::test]
async fn a_show_that_fails_part_way_is_finished_at_the_next_start() {
    let work = tempfile::tempdir().unwrap();
    let (old, usb, jdir) = dirs(work.path());
    let journal = Journal::open(&jdir.join("j.redb")).unwrap();
    let failing = Destination::open(&usb)
        .unwrap()
        .with_profile(provable())
        .with_show_hook(|step| {
            if step == "modified" {
                return Err(std::io::Error::other("the drive went away"));
            }
            Ok(())
        });
    send(&old, &data(BIG), "c.bin", &table_for(failing), &journal)
        .await
        .unwrap();
    let id = journal.entries().unwrap()[0].id;
    assert!(
        journal.show_pending(id).unwrap().is_some(),
        "a show that failed is never recorded shown"
    );
    let table = table_for(Destination::open(&usb).unwrap().with_profile(provable()));
    let r = pctwin_transfer::recover(&journal, &table).unwrap();
    assert_eq!(r.shown, [id], "{r:?}");
    assert_landed_whole(&journal, &usb, "c.bin", &data(BIG));
}

/// A copy committed while still hidden is never undone before it is shown: undo says it is still
/// being finished, and touches nothing; once recovery has shown it, it is like any other copy.
#[test]
fn a_copy_not_yet_shown_is_never_undone() {
    use pctwin_journal::UndoOutcome;
    let (_work, usb, journal) = stopped_at("journal:committed");
    let table = table_for(Destination::open(&usb).unwrap().with_profile(provable()));
    let id = journal.entries().unwrap()[0].id;
    assert!(journal.show_pending(id).unwrap().is_some());
    let report =
        pctwin_transfer::undo(&journal, &table, pctwin_transfer::Confirmed::none(&journal))
            .unwrap();
    assert!(
        matches!(&report.files[0].outcome, UndoOutcome::NotDone { why } if why.contains("still finishing")),
        "{report:?}"
    );
    assert_eq!(std::fs::read(usb.join("c.bin")).unwrap(), data(BIG));
    let r = pctwin_transfer::recover(&journal, &table).unwrap();
    assert_eq!(r.shown, [id], "{r:?}");
    assert!(journal.show_pending(id).unwrap().is_none());
}

/// A copy committed while still hidden, then replaced by another program's hidden file before
/// PCTwin came back to show it: that file is never touched, and the person is told, for good.
#[cfg(windows)]
#[test]
fn a_hidden_file_that_is_not_pctwin_s_is_never_shown_and_is_reported() {
    use std::io::Write;
    use std::os::windows::fs::OpenOptionsExt;
    let (_work, usb, journal) = stopped_at("journal:committed");
    let id = journal.entries().unwrap()[0].id;
    std::fs::remove_file(usb.join("c.bin")).unwrap();
    std::fs::OpenOptions::new()
        .write(true)
        .create_new(true)
        .attributes(0x2)
        .open(usb.join("c.bin"))
        .unwrap()
        .write_all(b"their hidden file")
        .unwrap();
    let table = table_for(Destination::open(&usb).unwrap().with_profile(provable()));
    let r = pctwin_transfer::recover(&journal, &table).unwrap();
    assert_eq!(r.notes.len(), 1, "{r:?}");
    assert!(r.notes[0].1.contains("c.bin"), "{r:?}");
    assert_eq!(journal.notes(id).unwrap().len(), 1, "kept for good");
    assert!(journal.unshown().unwrap().is_empty());
    let dest = table.get("usb").unwrap();
    assert!(dest.hidden("c.bin").unwrap(), "their file is left as it is");
    assert_eq!(
        std::fs::read(usb.join("c.bin")).unwrap(),
        b"their hidden file"
    );
}
