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
    send_outcome(old, bytes, path, table, journal).await.0
}

/// As [`send`], with what the new laptop reports for the file.
async fn send_outcome(
    old: &Path,
    bytes: &[u8],
    path: &str,
    table: &Destinations,
    journal: &dyn Ledger,
) -> (
    Result<(), TransferError>,
    Option<pctwin_transfer::ReceiveOutcome>,
) {
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
    let outcome = receiver.outcome(item(1));
    (received, outcome)
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
    // The identity kept is the shown file's (on a Mac it carries the real creation date, given
    // back after the commit), so Undo knows the file as it is now.
    if let State::Committed { landed, .. } = &all[0].state {
        let now = Destination::open(dir).unwrap().stat(name).unwrap().unwrap();
        assert_eq!(
            landed.file,
            Some(now.id),
            "the identity kept is the file's now"
        );
    }
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
        // Another program takes the name just after it is recorded, so the next one is about to
        // be recorded: stop dead before it is.
        let race = std::env::var_os("PCTWIN_TAKE_THEN_DIE").map(PathBuf::from);
        if race.is_some() && FILLINGS.fetch_add(1, std::sync::atomic::Ordering::SeqCst) == 1 {
            std::process::abort();
        }
        let r = self.then("filling", self.inner.filling(id, final_path, tag, proof));
        if let Some(usb) = race
            && r.is_ok()
        {
            their_empty_hidden(&usb.join(final_path));
        }
        r
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
        let r = self.then("committed", self.inner.committed_showing(id, landed, show));
        // Another program puts its own file in place of PCTwin's just after the commit (set
        // only in a separate copy of this program, so no other test sees it).
        if let Some(usb) = std::env::var_os("PCTWIN_REPLACE_AFTER_COMMIT").map(PathBuf::from)
            && r.is_ok()
        {
            let at = usb.join(&show.fill.final_path);
            std::fs::remove_file(&at).unwrap();
            their_empty_hidden(&at);
        }
        // Or PCTwin's file is simply removed there (security software, say).
        if let Some(usb) = std::env::var_os("PCTWIN_REMOVE_AFTER_COMMIT").map(PathBuf::from)
            && r.is_ok()
        {
            std::fs::remove_file(usb.join(&show.fill.final_path)).unwrap();
        }
        r
    }
    fn shown(&self, id: u64, left: Option<&str>, file: Option<FileId>) -> Result<(), JournalError> {
        self.check("shown")?;
        self.then("shown", self.inner.shown(id, left, file))
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
/// checked file again: the whole file, nothing partial, nothing left. On Windows, where the tag
/// that proves the partial copy can be inherited by another program's file, the partial copy is
/// left and the whole copy lands beside it, with a note.
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
    if cfg!(windows) {
        // On Windows a tag never removes anything (another program's file at a freed name can
        // inherit it): the partial copy stays as it is, the whole copy lands beside it, and the
        // person is told.
        assert!(r.failed.is_empty() && !r.notes.is_empty(), "{r:?}");
        assert_eq!(std::fs::metadata(usb.join("c.bin")).unwrap().len(), partial);
        let all = journal.entries().unwrap();
        let State::Committed { final_path, .. } = &all[0].state else {
            panic!("{:?}", all[0].state)
        };
        assert_ne!(final_path, "c.bin");
        assert_eq!(std::fs::read(usb.join(final_path)).unwrap(), data(BIG));
        let _ = (old, tag);
        return;
    }
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

/// How many fills a test run has recorded (for a stop just before the second).
static FILLINGS: std::sync::atomic::AtomicU32 = std::sync::atomic::AtomicU32::new(0);

/// Makes `path` as another program's empty file, marked hidden where the system has a mark.
fn their_empty_hidden(path: &Path) {
    #[cfg(windows)]
    {
        use std::os::windows::fs::OpenOptionsExt;
        std::fs::OpenOptions::new()
            .write(true)
            .create_new(true)
            .attributes(0x2)
            .open(path)
            .unwrap();
    }
    #[cfg(not(windows))]
    {
        std::fs::write(path, b"").unwrap();
    }
    #[cfg(target_os = "macos")]
    {
        let done = std::process::Command::new("chflags")
            .arg("hidden")
            .arg(path)
            .status()
            .unwrap();
        assert!(done.success());
    }
}

/// Runs a move of c.bin in a separate copy of this program, stopped dead as `env` says.
fn stopped_naming(work: &Path, env: (&str, &str)) {
    let status = std::process::Command::new(std::env::current_exe().unwrap())
        .args(["--ignored", "--exact", "stops_dead_naming", "--nocapture"])
        .env("PCTWIN_KILL_WORK", work)
        .env(env.0, env.1)
        .status()
        .unwrap();
    assert!(!status.success(), "the laptop was meant to stop dead");
}

/// After recovery: another program's empty file is still at c.bin, untouched; PCTwin's copy is
/// whole under another name, committed; and the person is told about c.bin in plain words.
fn assert_theirs_kept(journal: &Journal, usb: &Path, r: &pctwin_transfer::Recovered) {
    assert_eq!(
        std::fs::read(usb.join("c.bin")).unwrap(),
        b"",
        "their file at c.bin was removed or taken over: {r:?}"
    );
    let e = &journal.entries().unwrap()[0];
    let State::Committed { final_path, .. } = &e.state else {
        panic!("{:?} {r:?}", e.state);
    };
    assert_ne!(final_path, "c.bin");
    assert_eq!(std::fs::read(usb.join(final_path)).unwrap(), data(BIG));
    assert!(
        r.notes.iter().any(|(_, n)| n.ends_with("c.bin")),
        "c.bin left without a word: {r:?}"
    );
}

/// The review's probe: the laptop stops dead right after the journal records the name it is
/// about to fill in, before the file is made; then another program makes an empty hidden file at
/// that name. What PCTwin meant to do proves nothing: recovery leaves that file exactly as it
/// is, says so, and puts the copy beside it.
#[test]
fn an_empty_hidden_file_made_after_pctwin_stopped_is_never_removed_or_taken() {
    let work = tempfile::tempdir().unwrap();
    let (_, usb, jdir) = dirs(work.path());
    stopped_naming(work.path(), ("PCTWIN_DIE_AT", "journal:filling"));
    let journal = Journal::open(&jdir.join("j.redb")).unwrap();
    let e = journal.entries().unwrap()[0].clone();
    assert!(
        matches!(&e.state, State::Filling { fill, .. } if fill.made.is_none()),
        "{:?}",
        e.state
    );
    assert!(
        !usb.join("c.bin").exists(),
        "PCTwin made nothing at the name"
    );
    their_empty_hidden(&usb.join("c.bin"));
    let table = table_for(Destination::open(&usb).unwrap().with_profile(provable()));
    let r = pctwin_transfer::recover(&journal, &table).unwrap();
    assert_theirs_kept(&journal, &usb, &r);
}

/// The other way in: another program makes an empty hidden file at the name just after PCTwin
/// recorded it, so the name is taken, and the laptop stops dead before it records the next name. Recovery still
/// leaves that file, says so, and puts the copy beside it.
#[test]
fn a_taken_name_s_file_is_never_removed_after_a_stop_before_the_next_name() {
    let work = tempfile::tempdir().unwrap();
    let (_, usb, jdir) = dirs(work.path());
    stopped_naming(work.path(), ("PCTWIN_TAKE_THEN_DIE", usb.to_str().unwrap()));
    let journal = Journal::open(&jdir.join("j.redb")).unwrap();
    let e = journal.entries().unwrap()[0].clone();
    assert!(
        matches!(&e.state, State::Filling { fill, .. } if fill.final_path == "c.bin" && fill.made.is_none()),
        "{:?}",
        e.state
    );
    let table = table_for(Destination::open(&usb).unwrap().with_profile(provable()));
    let r = pctwin_transfer::recover(&journal, &table).unwrap();
    assert_theirs_kept(&journal, &usb, &r);
}

/// Another program puts its own file in place of PCTwin's just after the copy is committed and
/// before it is shown (on Windows its file even takes PCTwin's tag: a file made at a freed name
/// gets the old one's creation time). That file is never shown or touched, and the person is
/// told in the same move, never that the file was copied.
#[test]
fn a_file_replaced_before_it_is_shown_is_reported_in_the_same_move() {
    let work = tempfile::tempdir().unwrap();
    let (_, usb, jdir) = dirs(work.path());
    let status = std::process::Command::new(std::env::current_exe().unwrap())
        .args(["--ignored", "--exact", "moves_and_reports", "--nocapture"])
        .env("PCTWIN_KILL_WORK", work.path())
        .env("PCTWIN_REPLACE_AFTER_COMMIT", &usb)
        .status()
        .unwrap();
    assert!(status.success());
    let said = std::fs::read_to_string(work.path().join("outcome.txt")).unwrap();
    assert!(said.starts_with("Some(NeedsAction"), "{said}");
    assert!(said.contains("could not confirm"), "{said}");
    assert_eq!(
        std::fs::read(usb.join("c.bin")).unwrap(),
        b"",
        "their file was touched"
    );
    let journal = Journal::open(&jdir.join("j.redb")).unwrap();
    let id = journal.entries().unwrap()[0].id;
    assert_eq!(journal.notes(id).unwrap().len(), 1, "kept for good");
}

/// PCTwin's file is removed from its name just after the copy is committed, before it is shown:
/// the person is told in the same move, and the journal keeps the same words for good.
#[test]
fn a_file_removed_before_it_is_shown_is_reported_and_noted() {
    let work = tempfile::tempdir().unwrap();
    let (_, usb, jdir) = dirs(work.path());
    let status = std::process::Command::new(std::env::current_exe().unwrap())
        .args(["--ignored", "--exact", "moves_and_reports", "--nocapture"])
        .env("PCTWIN_KILL_WORK", work.path())
        .env("PCTWIN_REMOVE_AFTER_COMMIT", &usb)
        .status()
        .unwrap();
    assert!(status.success());
    let said = std::fs::read_to_string(work.path().join("outcome.txt")).unwrap();
    assert!(said.starts_with("Some(NeedsAction"), "{said}");
    assert!(said.contains("removed from its name"), "{said}");
    let journal = Journal::open(&jdir.join("j.redb")).unwrap();
    let id = journal.entries().unwrap()[0].id;
    let notes = journal.notes(id).unwrap();
    assert!(
        notes.len() == 1 && notes[0].contains("removed from its name"),
        "{notes:?}"
    );
}

/// The part run as a separate program by the tests above: a whole move, its outcome written to
/// outcome.txt.
#[tokio::test]
#[ignore = "run by a_file_replaced_before_it_is_shown_is_reported_in_the_same_move"]
async fn moves_and_reports() {
    let work = PathBuf::from(std::env::var_os("PCTWIN_KILL_WORK").expect("run by its parent"));
    let (old, usb, jdir) = dirs(&work);
    let journal = Journal::open(&jdir.join("j.redb")).unwrap();
    let table = table_for(Destination::open(&usb).unwrap().with_profile(provable()));
    let breaks = Breaks {
        inner: &journal,
        at: "",
        dies_after: "",
    };
    let (sent, outcome) = send_outcome(&old, &data(BIG), "c.bin", &table, &breaks).await;
    sent.unwrap();
    std::fs::write(work.join("outcome.txt"), format!("{outcome:?}")).unwrap();
}

/// The part run as a separate program by the two tests above.
#[tokio::test]
#[ignore = "run by the tests of a stop while naming"]
async fn stops_dead_naming() {
    let work = PathBuf::from(std::env::var_os("PCTWIN_KILL_WORK").expect("run by its parent"));
    let (old, usb, jdir) = dirs(&work);
    let journal = Journal::open(&jdir.join("j.redb")).unwrap();
    let table = table_for(Destination::open(&usb).unwrap().with_profile(provable()));
    let dies_after = if std::env::var("PCTWIN_DIE_AT").as_deref() == Ok("journal:filling") {
        "filling"
    } else {
        ""
    };
    let breaks = Breaks {
        inner: &journal,
        at: "",
        dies_after,
    };
    let _ = send(&old, &data(BIG), "c.bin", &table, &breaks).await;
    unreachable!("the laptop was meant to stop dead");
}

/// The laptop is killed (by its process number, as a power cut or a crash would stop it) at a
/// random moment of a move onto a drive with no safe rename, again and again
/// (PCTWIN_KILL_RUNS, 100 unless set). After each, recovery never leaves a second copy, nor a
/// partial or hidden file under a real name, nor anything still to show. The one thing it may
/// leave is an empty file nothing proves is PCTwin's (a stop just after it was made), and it says
/// so.
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
    let (mut committed, mut earlier, mut left_empty) = (0, 0, 0);
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
        let (whole, other): (Vec<&String>, Vec<&String>) = real
            .iter()
            .partition(|n| std::fs::read(usb.join(n)).unwrap() == data(BIG));
        assert!(whole.len() <= 1, "run {run} ({at:?}): {real:?}, {r:?}");
        for name in &whole {
            assert!(!dest.hidden(name).unwrap(), "run {run}: {name} left hidden");
        }
        // A stop between making the file and recording it made (or before its first byte) leaves
        // an empty file nothing proves is PCTwin's. It is left, and said, and the copy lands
        // beside it: nothing is lost, and no copy is made twice.
        for name in &other {
            assert!(
                std::fs::metadata(usb.join(name)).unwrap().len() == 0,
                "run {run} ({at:?}): a partial copy under {name}: {r:?}"
            );
            assert!(
                r.notes.iter().any(|(_, n)| n.ends_with(name.as_str())),
                "run {run}: {name} left without a word: {r:?}"
            );
        }
        if other.is_empty() {
            assert!(r.notes.is_empty(), "run {run}: {r:?}");
        } else {
            left_empty += 1;
        }
        assert!(journal.unshown().unwrap().is_empty(), "run {run}: {r:?}");
        match journal.entries().unwrap().first().map(|e| &e.state) {
            Some(State::Committed { .. }) => committed += 1,
            _ => earlier += 1,
        }
    }
    eprintln!(
        "{runs} kills: {committed} ended committed, {earlier} stopped earlier (kept or failed), {left_empty} left an empty file and said so"
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
    let (sent, outcome) =
        send_outcome(&old, &data(BIG), "c.bin", &table_for(failing), &journal).await;
    sent.unwrap();
    // Told in the same move: never reported copied while it is still hidden.
    match outcome {
        Some(pctwin_transfer::ReceiveOutcome::NeedsAction { why, .. }) => {
            assert!(why.contains("starts again"), "{why}");
        }
        other => panic!("reported {other:?}"),
    }
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

/// Moves every creation time of every file in `dir` by `by` seconds, as macOS does to a FAT
/// drive mounted again under another time zone or summer-time offset.
#[cfg(windows)]
fn shift_creation_times(dir: &Path, by: i64) {
    use std::os::windows::fs::FileTimesExt;
    for e in std::fs::read_dir(dir).unwrap() {
        let p = e.unwrap().path();
        if !p.is_file() {
            continue;
        }
        let born = std::fs::metadata(&p).unwrap().created().unwrap();
        let moved = if by >= 0 {
            born + std::time::Duration::from_secs(by.unsigned_abs())
        } else {
            born - std::time::Duration::from_secs(by.unsigned_abs())
        };
        let f = std::fs::OpenOptions::new().write(true).open(&p).unwrap();
        f.set_times(std::fs::FileTimes::new().set_created(moved))
            .unwrap();
    }
}

/// A stop after the commit, or after the fill was recorded filled, then the drive mounted again
/// under another offset (the review's probe: every time on the drive an hour on, or five and a
/// half). Recovery still knows PCTwin's copy by its shifted tag and its whole bytes: it ends
/// shown, visible, committed, with nothing left hidden, pending or said.
#[cfg(windows)]
#[test]
fn a_copy_is_still_finished_after_the_drive_s_time_offset_changed() {
    for (at, by) in [
        ("journal:committed", 3600),
        ("journal:filled", 5 * 3600 + 1800),
    ] {
        let (_work, usb, journal) = stopped_at(at);
        shift_creation_times(&usb, by);
        let table = table_for(Destination::open(&usb).unwrap().with_profile(provable()));
        let r = pctwin_transfer::recover(&journal, &table).unwrap();
        assert!(r.notes.is_empty() && r.left.is_empty(), "{at}: {r:?}");
        assert_landed_whole(&journal, &usb, "c.bin", &data(BIG));
    }
}

/// After a stop at Filled, the same bytes and the same tag come back at the name as a new file
/// record (as a drive that numbers files by where they sit gives after a remount, or a file
/// whose number changes as it fills): it is PCTwin's copy by its tag and its whole bytes, so
/// recovery finishes it, shows it with its real dates, and leaves nothing hidden or pending. (The
/// review's probe: before, the number recorded when it was made was asked for, and the copy was
/// committed and left hidden for good.)
#[cfg(windows)]
#[test]
fn a_copy_whose_file_number_changed_is_still_finished_and_shown() {
    use std::io::Write;
    use std::os::windows::fs::{FileTimesExt, OpenOptionsExt};
    let (_work, usb, journal) = stopped_at("journal:filled");
    let e = journal.entries().unwrap()[0].clone();
    let State::Filled { fill, .. } = &e.state else {
        panic!("{:?}", e.state);
    };
    let tag = fill.tag.born.unwrap();
    let at = usb.join("c.bin");
    let content = std::fs::read(&at).unwrap();
    std::fs::remove_file(&at).unwrap();
    let mut f = std::fs::OpenOptions::new()
        .write(true)
        .create_new(true)
        .attributes(0x2)
        .open(&at)
        .unwrap();
    f.write_all(&content).unwrap();
    let born = std::time::UNIX_EPOCH
        + std::time::Duration::new(u64::try_from(tag.secs).unwrap(), tag.nanos);
    f.set_times(std::fs::FileTimes::new().set_created(born))
        .unwrap();
    f.sync_all().unwrap();
    drop(f);
    let table = table_for(Destination::open(&usb).unwrap().with_profile(provable()));
    let r = pctwin_transfer::recover(&journal, &table).unwrap();
    assert!(r.notes.is_empty() && r.left.is_empty(), "{r:?}");
    assert_landed_whole(&journal, &usb, "c.bin", &data(BIG));
    let dest = table.get("usb").unwrap();
    assert_ne!(
        dest.born("c.bin").unwrap(),
        Some(tag),
        "the tag left behind"
    );
}

/// Recovery finishes a file recorded filled before a stop, but its show then fails part way (the
/// drive errs): recovery says so, in plain words, and the show stays pending for the next start,
/// never dropped as if the copy were finished and visible.
#[test]
fn recovery_says_when_a_show_it_starts_fails() {
    let (_work, usb, journal) = stopped_at("journal:filled");
    let failing = Destination::open(&usb)
        .unwrap()
        .with_profile(provable())
        .with_show_hook(|step| {
            if step == "modified" {
                return Err(std::io::Error::other("the drive went away"));
            }
            Ok(())
        });
    let table = table_for(failing);
    let r = pctwin_transfer::recover(&journal, &table).unwrap();
    let id = journal.entries().unwrap()[0].id;
    // Not a finished, visible copy yet: never listed as committed, only as waiting.
    assert!(r.committed.is_empty(), "{r:?}");
    assert!(
        r.left
            .iter()
            .any(|(i, why)| *i == id && why.contains("starts again")),
        "the failed show was dropped: {r:?}"
    );
    assert!(journal.show_pending(id).unwrap().is_some());
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

/// Windows hands a file made at a name freed moments before the old file's creation time
/// ("file-name tunnelling"), so another program's file at the name carries PCTwin's tag. After a
/// stop mid-fill, another program replaces PCTwin's hidden partial copy with its own file holding
/// the first bytes of the same data: recovery leaves it as it is, puts the copy beside it, and
/// says why (found by the fresh review of 20 October 2026).
#[cfg(windows)]
#[test]
fn another_programs_file_with_pctwins_tag_is_never_removed_on_windows() {
    let work = tempfile::tempdir().unwrap();
    let (_old, usb, jdir) = dirs(work.path());
    let status = std::process::Command::new(std::env::current_exe().unwrap())
        .args(["--ignored", "--exact", "stops_dead_mid_fill", "--nocapture"])
        .env("PCTWIN_KILL_WORK", work.path())
        .status()
        .unwrap();
    assert!(!status.success());
    let journal = Journal::open(&jdir.join("j.redb")).unwrap();
    let e = &journal.entries().unwrap()[0];
    let State::Filling { fill, .. } = &e.state else {
        panic!("{:?}", e.state)
    };
    assert_eq!(fill.proof, pctwin_journal::Proof::Token);
    let tag = fill.tag.born.expect("a tag was given");
    let path = usb.join("c.bin");
    std::fs::remove_file(&path).unwrap();
    let theirs = data(BIG)[..5000].to_vec();
    std::fs::write(&path, &theirs).unwrap();
    let table = table_for(
        Destination::open(&usb)
            .unwrap()
            .with_profile(no_safe_rename()),
    );
    let dest = table.get("usb").unwrap();
    assert_eq!(
        dest.born("c.bin").unwrap(),
        Some(tag),
        "their file inherited PCTwin's tag, as on a real Windows drive"
    );
    let r = pctwin_transfer::recover(&journal, &table).unwrap();
    assert_eq!(
        std::fs::read(&path).unwrap(),
        theirs,
        "their file is untouched"
    );
    assert!(!r.notes.is_empty(), "the person is told: {r:?}");
}
