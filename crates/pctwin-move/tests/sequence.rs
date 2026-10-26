//! A whole move after pairing, both laptops in one test over an in-memory connection: the old
//! laptop sends its record and its files; the new laptop places each person, the person approves
//! the plan, and every file lands through the safety gate with each write in the change journal.
//! If the person does not approve, the move ends with nothing written.
// Tests make files to test on; the no-file-changes fence is for the shipped code.
#![allow(clippy::disallowed_methods)]

use std::path::{Path, PathBuf};
use std::time::Duration;

use pctwin_gate::{Approved, Destinations};
use pctwin_journal::Journal;
use pctwin_move::{Ended, Live, MoveError, NotSentWhy, Preview, STOPPED, Steps, Stop, send_plan};
use pctwin_record::{
    FolderRole, Inclusion, Item, ItemId, ItemKind, ItemName, ItemPath, LaptopId, LeftOutReason,
    ManagedBy, Owner, PersonTarget, Place, Portability, Record, Storage,
};
use pctwin_transfer::{
    Cause, Channel, ChannelError, LandingPlace, ReceiveOutcome, SendOutcome, TimeRange, role_label,
    shown_path,
};
use tokio::sync::mpsc;

/// How long a test waits before calling it a deadlock. Every exchange here takes well under a
/// second, so a busy runner never comes near it, while one that waits for ever still fails.
const HANG_AFTER: Duration = Duration::from_secs(300);

async fn no_hang<T>(work: impl std::future::Future<Output = T>) -> T {
    match tokio::time::timeout(HANG_AFTER, work).await {
        Ok(done) => done,
        Err(_) => panic!("hung: still waiting after {HANG_AFTER:?}, a deadlock, not a slow runner"),
    }
}

struct Mem {
    tx: mpsc::UnboundedSender<Vec<u8>>,
    rx: mpsc::UnboundedReceiver<Vec<u8>>,
}

/// The old laptop's side with the app: greets first (as the app does, before finding the
/// files), then sends by the new laptop's rules.
async fn send_greeted(
    link: &mut Mem,
    record: &Record,
    mover: &str,
    people: &[pctwin_move::landing::NamedPerson],
    source: impl Fn(&Item) -> Option<PathBuf>,
    live: &Live,
    opener: std::sync::Arc<dyn pctwin_transfer::Opener>,
) -> Result<(pctwin_move::Sent, pctwin_move::landing::LandingRules), MoveError> {
    let mut link = greeted(link, pctwin_move::greeting::Side::Old).await?;
    let who = pctwin_move::Who { mover, people };
    let mut no_way_back = pctwin_move::resume::NoComeback;
    pctwin_move::send_by_rules(
        &mut link,
        record,
        &who,
        source,
        live,
        opener,
        &mut no_way_back,
    )
    .await
}

/// The new laptop's side with the app: greets first, then receives by its rules.
async fn receive_greeted<S: Steps>(
    link: &mut Mem,
    steps: &mut S,
    rules: impl FnOnce(&mut S, &mut Destinations) -> pctwin_move::landing::LandingRules,
    table: &mut Destinations,
    journal: &dyn pctwin_journal::Ledger,
    account: &str,
    live: &Live,
) -> Result<pctwin_move::Received, MoveError> {
    let mut link = greeted(link, pctwin_move::greeting::Side::New).await?;
    let here = pctwin_move::Here { journal, account };
    let mut no_way_back = pctwin_move::resume::NoRedial;
    pctwin_move::receive_by_rules(&mut link, steps, rules, table, here, live, &mut no_way_back)
        .await
}

async fn greeted(
    link: &mut Mem,
    side: pctwin_move::greeting::Side,
) -> Result<pctwin_move::greeting::Greeted<'_, Mem>, MoveError> {
    use pctwin_move::greeting::{About, Power, greet};
    Ok(greet(link, &About::here(Power::Unknown, None), side)
        .await
        .map_err(pctwin_transfer::TransferError::from)?)
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

const ROLES: [(FolderRole, &str); 2] = [
    (FolderRole::Documents, "Documents"),
    (FolderRole::Pictures, "Pictures"),
];

fn laptop() -> LaptopId {
    LaptopId::from_hex("00112233445566778899aabbccddeeff").unwrap()
}

fn item(role: FolderRole, path: &[&str], kind: ItemKind, size: u64, inclusion: Inclusion) -> Item {
    let owner = Owner::Person {
        account_id: "1000".into(),
    };
    let place = Place {
        role,
        storage: Storage::SystemDrive,
    };
    let path = ItemPath::new(path.iter().map(|p| ItemName::from_text(p)).collect()).unwrap();
    Item {
        id: ItemId::derive(&laptop(), &owner, &place, &path),
        kind,
        owner,
        place,
        path,
        size_bytes: size,
        modified_ns: None,
        portability: Portability::Portable,
        managed_by: ManagedBy::Personal,
        download_bytes: None,
        inclusion,
    }
}

/// Both laptops: the old one's folders with three files in them (and one it leaves out), the new
/// one's empty folders, approved for receiving, and its change journal.
struct World {
    old: tempfile::TempDir,
    new: tempfile::TempDir,
    _jdir: tempfile::TempDir,
    journal: Journal,
    table: Destinations,
    record: Record,
}

fn write(path: &Path, bytes: &[u8]) {
    std::fs::create_dir_all(path.parent().unwrap()).unwrap();
    std::fs::write(path, bytes).unwrap();
}

const FILES: [(FolderRole, &[&str], &[u8]); 3] = [
    (FolderRole::Documents, &["letter.txt"], b"Dear Ada"),
    (
        FolderRole::Documents,
        &["Reports", "year.txt"],
        b"a good year, all told",
    ),
    (FolderRole::Pictures, &["beach.jpg"], b"sand and sea"),
];

fn world() -> World {
    let old = tempfile::tempdir().unwrap();
    let new = tempfile::tempdir().unwrap();
    let jdir = tempfile::tempdir().unwrap();
    let journal = Journal::open(&jdir.path().join("journal.redb")).unwrap();
    let mut items = vec![item(
        FolderRole::Documents,
        &["Reports"],
        ItemKind::Folder,
        0,
        Inclusion::Included,
    )];
    for (role, path, bytes) in FILES {
        let found = item(
            role,
            path,
            ItemKind::File,
            bytes.len() as u64,
            Inclusion::Included,
        );
        write(&source_in(old.path(), &found), bytes);
        items.push(found);
    }
    items.push(item(
        FolderRole::Documents,
        &["locked.txt"],
        ItemKind::File,
        3,
        Inclusion::LeftOut {
            reason: LeftOutReason::Unreadable,
        },
    ));
    let mut table = Destinations::new();
    for (role, name) in ROLES {
        let dir = new.path().join(name);
        std::fs::create_dir_all(&dir).unwrap();
        table
            .approve(role_label(role), Approved::MyFolders, &dir)
            .unwrap();
    }
    let mut record = Record::new(laptop());
    record.items = items;
    record.validate().unwrap();
    World {
        old,
        new,
        _jdir: jdir,
        journal,
        table,
        record,
    }
}

fn folder_name(role: FolderRole) -> &'static str {
    ROLES.iter().find(|(r, _)| *r == role).unwrap().1
}

fn source_in(root: &Path, item: &Item) -> PathBuf {
    let mut source = root.join(folder_name(item.place.role));
    for part in item.path.parts() {
        source.push(part.on_disk());
    }
    source
}

fn landing(item: &Item) -> LandingPlace {
    LandingPlace::new(role_label(item.place.role), shown_path(&item.path))
}

/// The new laptop's person, played by the test: each person on the old laptop goes to this
/// laptop's account `1001`, and the plan is approved only if `approves`.
struct Person {
    approves: bool,
    asked_about: Vec<String>,
    shown_plan: Option<Record>,
    preview: Option<Preview>,
}

impl Steps for Person {
    async fn person(&mut self, _record: &Record, account_id: &str) -> Result<PersonTarget, Stop> {
        self.asked_about.push(account_id.to_string());
        Ok(PersonTarget::ExistingAccount {
            account_id: "1001".into(),
        })
    }

    async fn approve(&mut self, record: &Record, preview: &Preview) -> Result<(), Stop> {
        self.shown_plan = Some(record.clone());
        self.preview = Some(preview.clone());
        if self.approves {
            Ok(())
        } else {
            Err(Stop::NotApproved)
        }
    }

    fn landing(&self, item: &Item) -> Option<LandingPlace> {
        Some(landing(item))
    }
}

fn person(approves: bool) -> Person {
    Person {
        approves,
        asked_about: Vec::new(),
        shown_plan: None,
        preview: None,
    }
}

/// Every included file arrives and lands where the plan says, byte for byte; both sides give one
/// typed outcome per file; the left-out file is listed as not sent, with its reason.
#[tokio::test]
async fn a_whole_move_lands_every_file() {
    let w = world();
    let plan = send_plan(&w.record, |i| Some(source_in(w.old.path(), i)), landing);
    let (mut old, mut new) = mem_pair();
    let mut steps = person(true);
    let quiet = Live::default();
    let quiet_old = Live::default();
    let (sent, received) = no_hang(async {
        tokio::join!(
            pctwin_move::send(&mut old, &w.record, plan, &quiet_old),
            pctwin_move::receive(&mut new, &mut steps, &w.table, &w.journal, "1001", &quiet),
        )
    })
    .await;
    let sent = sent.unwrap();
    let received = received.unwrap();

    let files: Vec<&Item> = w
        .record
        .items
        .iter()
        .filter(|i| i.kind == ItemKind::File && i.inclusion == Inclusion::Included)
        .collect();
    assert_eq!(sent.files.len(), files.len());
    for line in &sent.files {
        assert_eq!(line.outcome, Some(SendOutcome::Arrived), "{:?}", line.item);
    }
    let locked = w
        .record
        .items
        .iter()
        .find(|i| i.inclusion != Inclusion::Included)
        .unwrap();
    assert_eq!(sent.not_sent.len(), 1);
    assert_eq!(sent.not_sent[0].item, locked.id);
    assert_eq!(
        sent.not_sent[0].why,
        NotSentWhy::LeftOut {
            reason: LeftOutReason::Unreadable
        }
    );

    // The person was asked once about the one person on the old laptop, and saw that choice in
    // the plan they approved.
    assert_eq!(steps.asked_about, ["1000"]);
    let shown = steps.shown_plan.unwrap();
    // Before approving, the person saw whether it fits and about how long: from the plan's
    // size, the places' free space, and the link speed the test after pairing measured.
    let preview = steps.preview.unwrap();
    let total: u64 = FILES.iter().map(|(_, _, b)| b.len() as u64).sum();
    assert_eq!(preview.bytes, total);
    assert!(
        preview.bytes_per_second.is_some_and(|s| s > 0),
        "{preview:?}"
    );
    assert!(
        matches!(preview.time, TimeRange::Between { fastest, slowest } if fastest <= slowest),
        "{preview:?}"
    );
    assert_eq!(preview.fit.places.len(), ROLES.len());
    assert!(
        preview.fit.places.iter().all(|p| p.free.is_some()),
        "{preview:?}"
    );
    assert_eq!(
        shown.mapping.people.get("1000"),
        Some(&PersonTarget::ExistingAccount {
            account_id: "1001".into()
        })
    );
    assert_eq!(received.record, shown);

    // One outcome per included file, in the record's order.
    let ids: Vec<ItemId> = received.files.iter().map(|f| f.item).collect();
    assert_eq!(ids, files.iter().map(|i| i.id).collect::<Vec<_>>());
    for (line, (role, path, bytes)) in received.files.iter().zip(FILES) {
        let Some(ReceiveOutcome::Finished(done)) = &line.outcome else {
            panic!("{path:?} ended {:?}", line.outcome);
        };
        assert_eq!(done.final_path, path.join("/"));
        let landed = w.new.path().join(folder_name(role)).join(path.join("/"));
        assert_eq!(std::fs::read(landed).unwrap(), bytes);
    }
    let entries = w.journal.entries().unwrap();
    assert!(!entries.is_empty());
    // How each file was checked is kept with it, so the report after a restart can say it.
    for e in &entries {
        assert!(w.journal.checked_from(e.id).unwrap().is_some(), "{e:?}");
    }
}

/// A person who does not approve the plan stops the move before anything is written: no file
/// lands, the change journal stays empty, and the old laptop's send ends.
#[tokio::test]
async fn a_plan_not_approved_writes_nothing() {
    let w = world();
    let plan = send_plan(&w.record, |i| Some(source_in(w.old.path(), i)), landing);
    let (mut old, new) = mem_pair();
    let mut steps = person(false);
    let quiet = Live::default();
    let quiet_old = Live::default();
    let (sent, received) = no_hang(async {
        tokio::join!(
            pctwin_move::send(&mut old, &w.record, plan, &quiet_old),
            async {
                // The new laptop's end closes when it stops, as a real connection does.
                let mut new = new;
                pctwin_move::receive(&mut new, &mut steps, &w.table, &w.journal, "1001", &quiet)
                    .await
            }
        )
    })
    .await;
    assert!(
        matches!(received, Err(MoveError::Stopped(Stop::NotApproved))),
        "{:?}",
        received.map(|r| r.files)
    );
    assert!(
        sent.is_err(),
        "the old laptop's send ends when the new one stops"
    );
    assert!(w.journal.entries().unwrap().is_empty());
    for (_, name) in ROLES {
        let landed: Vec<_> = std::fs::read_dir(w.new.path().join(name))
            .unwrap()
            .collect();
        assert!(landed.is_empty(), "{name}: {landed:?}");
    }
}

/// A stop on the old laptop ends the move on both: each side reports how it ended, every file
/// the stop cut off is "failed: the move was stopped" (never copied), and the new laptop is told
/// it was a stop, not a lost connection.
#[tokio::test]
async fn a_stop_ends_the_move_on_both_with_every_cut_file_failed_as_stopped() {
    let w = world();
    let plan = send_plan(&w.record, |i| Some(source_in(w.old.path(), i)), landing);
    let (mut old, new) = mem_pair();
    let mut steps = person(true);
    let here = Live::default();
    let quiet = Live::default();
    // Stopped before the first block: nothing may land.
    here.control.stop();
    let (sent, received) = no_hang(async {
        tokio::join!(pctwin_move::send(&mut old, &w.record, plan, &here), async {
            let mut new = new;
            pctwin_move::receive(&mut new, &mut steps, &w.table, &w.journal, "1001", &quiet).await
        })
    })
    .await;
    let sent = sent.unwrap();
    let received = received.unwrap();
    assert_eq!(sent.ended, Ended::StoppedHere);
    assert_eq!(received.ended, Ended::StoppedThere);
    assert_eq!(sent.files.len(), FILES.len());
    for line in &sent.files {
        match &line.outcome {
            Some(SendOutcome::Failed(why)) => {
                assert_eq!(why.cause(), Cause::Stopped);
                assert_eq!(why.why(), STOPPED);
            }
            other => panic!("{:?} ended {other:?}", line.item),
        }
    }
    assert_eq!(received.files.len(), FILES.len());
    for line in &received.files {
        assert!(
            matches!(&line.outcome, Some(ReceiveOutcome::Failed(why)) if why.cause() == Cause::Stopped),
            "{:?} ended {:?}",
            line.item,
            line.outcome
        );
    }
    for (role, path, _) in FILES {
        let landed = w.new.path().join(folder_name(role)).join(path.join("/"));
        assert!(!landed.exists(), "{landed:?}");
    }
}

/// A file changed after the scan is sent as it is now, lands, and its line says "changed since
/// the scan"; files as they were at the scan say nothing of the sort.
#[tokio::test]
async fn a_file_changed_since_the_scan_lands_as_it_is_now_and_is_labelled() {
    let w = world();
    let letter = w
        .record
        .items
        .iter()
        .find(|i| i.path.parts().last() == Some(&ItemName::from_text("letter.txt")))
        .unwrap()
        .clone();
    // Edited after the scan: a little longer.
    write(&source_in(w.old.path(), &letter), b"Dear Ada, again");
    let plan = send_plan(&w.record, |i| Some(source_in(w.old.path(), i)), landing);
    let (mut old, mut new) = mem_pair();
    let mut steps = person(true);
    let (quiet_old, quiet_new) = (Live::default(), Live::default());
    let (sent, received) = no_hang(async {
        tokio::join!(
            pctwin_move::send(&mut old, &w.record, plan, &quiet_old),
            pctwin_move::receive(
                &mut new, &mut steps, &w.table, &w.journal, "1001", &quiet_new
            ),
        )
    })
    .await;
    sent.unwrap();
    let received = received.unwrap();
    for line in &received.files {
        assert!(
            matches!(line.outcome, Some(ReceiveOutcome::Finished(_))),
            "{:?}",
            line.outcome
        );
        assert_eq!(
            line.changed_since_scan,
            line.item == letter.id,
            "{:?}",
            line.item
        );
    }
    let landed = w.new.path().join("Documents").join("letter.txt");
    assert_eq!(std::fs::read(landed).unwrap(), b"Dear Ada, again");
}

/// Problems found before the move are predicted by the very rules the write uses: every name
/// the preview says will change lands changed, under exactly the predicted name, and every
/// other lands as sent. The old laptop's files sit under plain names; what is sent is the
/// record's name, as from another system.
#[tokio::test]
async fn every_predicted_name_is_the_name_it_lands_under() {
    let w = world();
    predict_then_compare(w.old.path(), &w.table, &w.journal).await;
}

/// The same on real drives (FAT32 and exFAT, mounted by the real-drives CI job and listed in
/// PCTWIN_FAT_DIRS as `kind=folder`, separated by `|`); with PCTWIN_REQUIRE_FAT set, a missing
/// list fails. Drive kinds in PCTWIN_FAT_REFUSES, where the gate refuses to name files for now,
/// are left to that job's own test. The runners' own drives (NTFS, APFS, ext4) are the test above.
#[tokio::test]
async fn every_predicted_name_is_the_name_it_lands_under_on_real_drives() {
    let Ok(list) = std::env::var("PCTWIN_FAT_DIRS") else {
        assert!(
            std::env::var_os("PCTWIN_REQUIRE_FAT").is_none(),
            "PCTWIN_REQUIRE_FAT is set but PCTWIN_FAT_DIRS is not"
        );
        return;
    };
    let refuses = std::env::var("PCTWIN_FAT_REFUSES").unwrap_or_default();
    for entry in list.split('|').filter(|e| !e.is_empty()) {
        let (kind, folder) = entry.split_once('=').unwrap_or(("unknown", entry));
        if refuses.split(',').any(|k| k.trim() == kind) {
            continue;
        }
        let place = Path::new(folder).join(format!("predict-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&place);
        std::fs::create_dir_all(place.join("Documents")).unwrap();
        std::fs::create_dir_all(place.join("Pictures")).unwrap();
        let mut table = Destinations::new();
        for (role, name) in ROLES {
            table
                .approve(role_label(role), Approved::MyFolders, &place.join(name))
                .unwrap();
        }
        let old = tempfile::tempdir().unwrap();
        let jdir = tempfile::tempdir().unwrap();
        let journal = Journal::open(&jdir.path().join("journal.redb")).unwrap();
        eprintln!("predict then compare on {kind} at {folder}");
        predict_then_compare(old.path(), &table, &journal).await;
        let _ = std::fs::remove_dir_all(&place);
    }
}

/// Every name the plan review predicts is the name the file lands under, with the same changes
/// listed (Engineering Plan 5.1, "Problems found before the move"): a wrong prediction fails.
async fn predict_then_compare(old_root: &Path, table: &Destinations, journal: &Journal) {
    let long = format!("{}.txt", "x".repeat(300));
    let names = [
        "plain.txt",
        "CON.txt",
        "a:b?.txt",
        "LONGFO~1.txt",
        "spaces at the end. ",
        "na\u{ef}ve caf\u{e9} \u{2713}.txt",
        // Names in other scripts and forms (Language Design follow-up): Arabic, Hebrew,
        // Devanagari, Chinese, an accent written as two characters, an emoji, direction marks.
        "\u{62a}\u{642}\u{631}\u{64a}\u{631}.txt",
        "\u{5d3}\u{5d5}\u{5d7}.txt",
        "\u{930}\u{93f}\u{92a}\u{94b}\u{930}\u{94d}\u{91f}.txt",
        "\u{62a5}\u{544a} 2026.txt",
        "Cafe\u{301} menu.txt",
        "\u{1f389} party.txt",
        "\u{200f}right to left\u{200e}.txt",
        long.as_str(),
    ];
    let mut record = Record::new(laptop());
    let mut sources = std::collections::BTreeMap::new();
    for (n, name) in names.iter().enumerate() {
        let found = item(
            FolderRole::Documents,
            &[name],
            ItemKind::File,
            5,
            Inclusion::Included,
        );
        let source = old_root.join(format!("source-{n}.bin"));
        write(&source, b"hello");
        sources.insert(found.id, source);
        record.items.push(found);
    }
    record.validate().unwrap();
    let plan = send_plan(&record, |i| sources.get(&i.id).cloned(), landing);
    let (mut old, mut new) = mem_pair();
    let mut steps = person(true);
    let (quiet_old, quiet_new) = (Live::default(), Live::default());
    let (sent, received) = no_hang(async {
        tokio::join!(
            pctwin_move::send(&mut old, &record, plan, &quiet_old),
            pctwin_move::receive(&mut new, &mut steps, table, journal, "1001", &quiet_new),
        )
    })
    .await;
    sent.unwrap();
    let received = received.unwrap();
    let preview = steps.preview.unwrap();
    let mut renamed = 0;
    for line in &received.files {
        let item = record.items.iter().find(|i| i.id == line.item).unwrap();
        let place = landing(item);
        let predicted = table
            .get(&place.destination)
            .unwrap()
            .predict(&place.path, item.size_bytes)
            .unwrap();
        let Some(ReceiveOutcome::Finished(done)) = &line.outcome else {
            panic!("{:?} ended {:?}", place.path, line.outcome);
        };
        let landed_name = done.final_path.rsplit('/').next().unwrap();
        assert_eq!(landed_name, predicted.stored_name, "{:?}", place.path);
        assert_eq!(done.changes, predicted.changes, "{:?}", place.path);
        if !predicted.changes.is_empty() {
            renamed += 1;
            assert!(preview.problems.renamed.examples.contains(&line.item));
        }
    }
    assert_eq!(preview.problems.renamed.count, renamed);
    // The long name needs shortening on every system.
    assert!(renamed >= 1);
    assert_eq!(preview.problems.too_big.count, 0);
    assert_eq!(preview.problems.refused.count, 0);
}

/// Reads every file as it is, except one another program has open.
struct OneBusy(PathBuf);

impl pctwin_transfer::Opener for OneBusy {
    fn open(&self, path: &Path) -> std::io::Result<Box<dyn pctwin_transfer::Source>> {
        if path == self.0 {
            return Err(std::io::Error::other(pctwin_transfer::InUse));
        }
        pctwin_transfer::FsOpener.open(path)
    }
}

/// A file another program had open fails "in use", never copied torn; "Try again" sends only
/// that file, and the report then shows it copied with everything else as it was.
#[tokio::test]
async fn try_again_brings_a_file_that_was_in_use() {
    use pctwin_move::report::{to_try_again, tried_again};
    let w = world();
    let beach = w
        .record
        .items
        .iter()
        .find(|i| {
            i.path
                .parts()
                .last()
                .is_some_and(|p| p.display() == "beach.jpg")
        })
        .unwrap();
    let busy = std::sync::Arc::new(OneBusy(source_in(w.old.path(), beach)));
    let plan = send_plan(&w.record, |i| Some(source_in(w.old.path(), i)), landing);
    let (live, old_live) = (Live::default(), Live::default());
    let (mut old, mut new) = mem_pair();
    let mut steps = person(true);
    let (sent, received) = no_hang(async {
        tokio::join!(
            pctwin_move::send_with(&mut old, &w.record, plan, &old_live, busy),
            pctwin_move::receive(&mut new, &mut steps, &w.table, &w.journal, "1001", &live),
        )
    })
    .await;
    sent.unwrap();
    let first = received.unwrap();
    let again = to_try_again(&first);
    assert_eq!(again, std::collections::BTreeSet::from([beach.id]));
    let line = first.files.iter().find(|f| f.item == beach.id).unwrap();
    assert!(
        matches!(&line.outcome, Some(ReceiveOutcome::Failed(f)) if f.cause() == Cause::InUse),
        "{:?}",
        line.outcome
    );

    // Try again: the program was closed; only that file is sent.
    let plan = send_plan(&w.record, |i| Some(source_in(w.old.path(), i)), landing).only(&again);
    assert_eq!(plan.jobs.len(), 1);
    let (mut old, mut new) = mem_pair();
    let mut steps = person(true);
    let (sent, received) = no_hang(async {
        tokio::join!(
            pctwin_move::send(&mut old, &w.record, plan, &old_live),
            pctwin_move::receive(&mut new, &mut steps, &w.table, &w.journal, "1001", &live),
        )
    })
    .await;
    assert_eq!(sent.unwrap().files.len(), 1);
    let merged = tried_again(&first, &received.unwrap(), &again);
    for line in &merged.files {
        assert!(
            matches!(line.outcome, Some(ReceiveOutcome::Finished(_))),
            "{:?}",
            line.outcome
        );
    }
    assert_eq!(
        std::fs::read(w.new.path().join("Pictures/beach.jpg")).unwrap(),
        b"sand and sea"
    );
}

/// A file that would land where the system starts programs at sign-in is shown before the plan
/// is approved (an app-data item named for a Startup folder), with the file as an example.
#[test]
fn a_file_landing_where_programs_start_at_sign_in_is_shown_before_approval() {
    let w = world();
    let mut record = Record::new(laptop());
    let startup = item(
        FolderRole::Documents,
        &[
            "Microsoft",
            "Windows",
            "Start Menu",
            "Programs",
            "Startup",
            "updater.exe",
        ],
        ItemKind::File,
        10,
        Inclusion::Included,
    );
    let letter = item(
        FolderRole::Documents,
        &["letter.txt"],
        ItemKind::File,
        10,
        Inclusion::Included,
    );
    record.items = vec![startup.clone(), letter];
    let found = pctwin_move::problems(&record, |i| Some(landing(i)), &w.table);
    assert_eq!(found.runs_at_sign_in.count, 1);
    assert_eq!(found.runs_at_sign_in.examples, vec![startup.id]);
}

/// The app's way: the old laptop does not choose where files go; the new laptop sends its
/// landing rules once the plan is approved, and both place every file the same way. Every file
/// lands where the new laptop's own rules put it.
#[tokio::test]
async fn files_land_where_the_new_laptop_s_rules_put_them() {
    use pctwin_move::landing::LandingRules;
    let mut w = world();
    let rules = LandingRules {
        mover: "1000".into(),
        roles: ROLES
            .iter()
            .map(|(r, _)| (*r, Storage::SystemDrive))
            .collect(),
        fallback: Default::default(),
        held: Default::default(),
        left_out: Default::default(),
        chosen: Default::default(),
    };
    struct ByRules {
        rules: LandingRules,
        told: Option<String>,
        named: Vec<String>,
    }
    impl Steps for ByRules {
        async fn person(&mut self, _: &Record, _: &str) -> Result<PersonTarget, Stop> {
            Ok(PersonTarget::ExistingAccount {
                account_id: "1001".into(),
            })
        }
        async fn approve(&mut self, _: &Record, _: &Preview) -> Result<(), Stop> {
            Ok(())
        }
        fn landing(&self, item: &Item) -> Option<LandingPlace> {
            self.rules.landing(item)
        }
        fn mover(&mut self, account_id: &str) {
            self.told = Some(account_id.to_string());
        }
        fn people(&mut self, people: &[pctwin_move::landing::NamedPerson]) {
            self.named = people.iter().map(|p| p.name.clone()).collect();
        }
    }
    let mut steps = ByRules {
        rules: rules.clone(),
        told: None,
        named: Vec::new(),
    };
    let (mut old, mut new) = mem_pair();
    let (old_live, new_live) = (Live::default(), Live::default());
    let mut table = std::mem::take(&mut w.table);
    let bea = [pctwin_move::landing::NamedPerson {
        account_id: "1001".into(),
        name: "Bea".into(),
    }];
    let (sent, received) = no_hang(async {
        tokio::join!(
            send_greeted(
                &mut old,
                &w.record,
                "1000",
                &bea,
                |i| Some(source_in(w.old.path(), i)),
                &old_live,
                std::sync::Arc::new(pctwin_transfer::FsOpener),
            ),
            receive_greeted(
                &mut new,
                &mut steps,
                |s, _| s.rules.clone(),
                &mut table,
                &w.journal,
                "1001",
                &new_live
            ),
        )
    })
    .await;
    let (sent, got) = sent.unwrap();
    assert_eq!(steps.told.as_deref(), Some("1000"), "told who is moving");
    assert_eq!(steps.named, ["Bea"], "told the other people's names");
    assert_eq!(
        got, rules,
        "the old laptop placed by the new laptop's rules"
    );
    for line in &sent.files {
        assert_eq!(line.outcome, Some(SendOutcome::Arrived), "{:?}", line.item);
    }
    for (line, (role, path, bytes)) in received.unwrap().files.iter().zip(FILES) {
        assert!(matches!(line.outcome, Some(ReceiveOutcome::Finished(_))));
        let landed = w.new.path().join(folder_name(role)).join(path.join("/"));
        assert_eq!(std::fs::read(landed).unwrap(), bytes);
    }
}

/// Another person's holding place is made only once the plan is approved (the app's helper makes
/// it then). Their files are announced there by the rules the new laptop sends, and the new
/// laptop accepts them there: what it accepts is made from those very rules, not from the places
/// it knew when the person approved.
#[tokio::test]
async fn another_persons_files_land_in_a_holding_place_made_after_the_approval() {
    use pctwin_move::landing::LandingRules;
    let mut w = world();
    // Here the mover is someone else ("2000"): every file in the world is another person's
    // ("1000"), with no place for them when the plan is approved.
    let before = LandingRules {
        mover: "2000".into(),
        roles: ROLES
            .iter()
            .map(|(r, _)| (*r, Storage::SystemDrive))
            .collect(),
        fallback: Default::default(),
        held: Default::default(),
        left_out: Default::default(),
        chosen: Default::default(),
    };
    struct Late {
        rules: LandingRules,
    }
    impl Steps for Late {
        async fn person(&mut self, _: &Record, _: &str) -> Result<PersonTarget, Stop> {
            Ok(PersonTarget::ExistingAccount {
                account_id: "1001".into(),
            })
        }
        async fn approve(&mut self, _: &Record, _: &Preview) -> Result<(), Stop> {
            Ok(())
        }
        fn landing(&self, item: &Item) -> Option<LandingPlace> {
            self.rules.landing(item)
        }
    }
    let holding = w.new.path().join("holding-1000");
    std::fs::create_dir_all(&holding).unwrap();
    let mut steps = Late {
        rules: before.clone(),
    };
    let (mut old, mut new) = mem_pair();
    let (old_live, new_live) = (Live::default(), Live::default());
    let mut table = std::mem::take(&mut w.table);
    let hold = holding.clone();
    let (sent, received) = no_hang(async {
        tokio::join!(
            send_greeted(
                &mut old,
                &w.record,
                "2000",
                &[],
                |i| Some(source_in(w.old.path(), i)),
                &old_live,
                std::sync::Arc::new(pctwin_transfer::FsOpener),
            ),
            receive_greeted(
                &mut new,
                &mut steps,
                |s, table| {
                    // After the approval: the holding place is made and approved, and the rules
                    // say to hold this person's things there.
                    table.approve_holding("for-1", "1001", &hold).unwrap();
                    let mut rules = s.rules.clone();
                    rules.held.insert("1000".into(), "for-1".into());
                    rules
                },
                &mut table,
                &w.journal,
                "1001",
                &new_live
            ),
        )
    })
    .await;
    let (sent, _) = sent.unwrap();
    for line in &sent.files {
        assert_eq!(line.outcome, Some(SendOutcome::Arrived), "{:?}", line.item);
    }
    for (line, (role, path, bytes)) in received.unwrap().files.iter().zip(FILES) {
        assert!(
            matches!(line.outcome, Some(ReceiveOutcome::Finished(_))),
            "{:?}",
            line.outcome
        );
        let landed = holding.join(role_label(role)).join(path.join("/"));
        assert_eq!(std::fs::read(landed).unwrap(), bytes);
    }
}

/// Another person's writes are journaled in their holding folder, never in the mover's journal:
/// read as the mover reads it, byte for byte, the mover's journal file names none of their files,
/// while the journal beside their files records every one (decided 21 October 2026: others'
/// files pass through PCTwin's engine, never shown to the mover).
#[tokio::test]
async fn the_movers_journal_names_none_of_another_persons_files() {
    use pctwin_move::held_journal::{HELD_JOURNAL, HeldPlaces, SplitLedger};
    use pctwin_move::landing::LandingRules;
    let mut w = world();
    let rules = LandingRules {
        mover: "2000".into(),
        roles: ROLES
            .iter()
            .map(|(r, _)| (*r, Storage::SystemDrive))
            .collect(),
        fallback: Default::default(),
        held: Default::default(),
        left_out: Default::default(),
        chosen: Default::default(),
    };
    struct Held {
        rules: LandingRules,
    }
    impl Steps for Held {
        async fn person(&mut self, _: &Record, _: &str) -> Result<PersonTarget, Stop> {
            Ok(PersonTarget::ExistingAccount {
                account_id: "1001".into(),
            })
        }
        async fn approve(&mut self, _: &Record, _: &Preview) -> Result<(), Stop> {
            Ok(())
        }
        fn landing(&self, item: &Item) -> Option<LandingPlace> {
            self.rules.landing(item)
        }
    }
    let holding = w.new.path().join("holding-1000");
    std::fs::create_dir_all(&holding).unwrap();
    let mut steps = Held { rules };
    let places = HeldPlaces::default();
    let ledger = SplitLedger::new(&w.journal, places.clone());
    let (mut old, mut new) = mem_pair();
    let (old_live, new_live) = (Live::default(), Live::default());
    let mut table = std::mem::take(&mut w.table);
    let hold = holding.clone();
    let (sent, received) = no_hang(async {
        tokio::join!(
            send_greeted(
                &mut old,
                &w.record,
                "2000",
                &[],
                |i| Some(source_in(w.old.path(), i)),
                &old_live,
                std::sync::Arc::new(pctwin_transfer::FsOpener),
            ),
            receive_greeted(
                &mut new,
                &mut steps,
                |s, table| {
                    table.approve_holding("for-1", "1001", &hold).unwrap();
                    places.hold("for-1", &hold);
                    let mut rules = s.rules.clone();
                    rules.held.insert("1000".into(), "for-1".into());
                    rules
                },
                &mut table,
                &ledger,
                "1001",
                &new_live
            ),
        )
    })
    .await;
    sent.unwrap();
    for (line, (role, path, bytes)) in received.unwrap().files.iter().zip(FILES) {
        assert!(
            matches!(line.outcome, Some(ReceiveOutcome::Finished(_))),
            "{:?}",
            line.outcome
        );
        let landed = holding.join(role_label(role)).join(path.join("/"));
        assert_eq!(std::fs::read(landed).unwrap(), bytes);
    }
    // Closed, as after the move, so the files are read as anyone would read them.
    drop(ledger);
    drop(w.journal);
    let has = |bytes: &[u8], name: &str| {
        bytes
            .windows(name.len())
            .any(|window| window == name.as_bytes())
    };
    let mine = std::fs::read(w._jdir.path().join("journal.redb")).unwrap();
    for name in FILES.iter().flat_map(|(_, path, _)| path.iter()) {
        assert!(!has(&mine, name), "the mover's journal names {name}");
    }
    let theirs = std::fs::read(holding.join(HELD_JOURNAL)).unwrap();
    for name in FILES.iter().flat_map(|(_, path, _)| path.iter()) {
        assert!(has(&theirs, name), "the holding journal misses {name}");
    }
}

/// M05: the person sent one folder to a place they chose on the new laptop. Once the plan is
/// approved, the chosen place is approved and the rules send that folder there; the rest land as
/// usual. A chosen place that could not be approved leaves its files failed, never put
/// somewhere else.
#[tokio::test]
async fn a_folder_sent_to_a_chosen_place_lands_there_or_fails_never_elsewhere() {
    use pctwin_move::landing::{LandingRules, chosen_label};
    for approved in [true, false] {
        let mut w = world();
        let mut rules = LandingRules {
            mover: "1000".into(),
            roles: ROLES
                .iter()
                .map(|(r, _)| (*r, Storage::SystemDrive))
                .collect(),
            fallback: Default::default(),
            held: Default::default(),
            left_out: Default::default(),
            chosen: Default::default(),
        };
        rules.chosen.insert(FolderRole::Pictures);
        struct Chose {
            rules: LandingRules,
        }
        impl Steps for Chose {
            async fn person(&mut self, _: &Record, _: &str) -> Result<PersonTarget, Stop> {
                Ok(PersonTarget::ExistingAccount {
                    account_id: "1001".into(),
                })
            }
            async fn approve(&mut self, _: &Record, _: &Preview) -> Result<(), Stop> {
                Ok(())
            }
            fn landing(&self, item: &Item) -> Option<LandingPlace> {
                self.rules.landing(item)
            }
        }
        let drive = tempfile::tempdir().unwrap();
        let mut steps = Chose { rules };
        let (mut old, mut new) = mem_pair();
        let (old_live, new_live) = (Live::default(), Live::default());
        let mut table = std::mem::take(&mut w.table);
        let (sent, received) = no_hang(async {
            tokio::join!(
                send_greeted(
                    &mut old,
                    &w.record,
                    "1000",
                    &[],
                    |i| Some(source_in(w.old.path(), i)),
                    &old_live,
                    std::sync::Arc::new(pctwin_transfer::FsOpener),
                ),
                receive_greeted(
                    &mut new,
                    &mut steps,
                    |s, table| {
                        if approved {
                            let place = pctwin_gate::Destination::open(drive.path()).unwrap();
                            table
                                .approve_chosen(&chosen_label(FolderRole::Pictures), place)
                                .unwrap();
                        }
                        s.rules.clone()
                    },
                    &mut table,
                    &w.journal,
                    "1001",
                    &new_live
                ),
            )
        })
        .await;
        sent.unwrap();
        for (line, (role, path, bytes)) in received.unwrap().files.iter().zip(FILES) {
            let usual = w.new.path().join(folder_name(role)).join(path.join("/"));
            let chosen = drive.path().join(path.join("/"));
            if role == FolderRole::Pictures {
                assert!(!usual.exists(), "never in the usual place");
                if approved {
                    assert!(matches!(line.outcome, Some(ReceiveOutcome::Finished(_))));
                    assert_eq!(std::fs::read(chosen).unwrap(), bytes);
                } else {
                    assert!(
                        !matches!(line.outcome, Some(ReceiveOutcome::Finished(_))),
                        "{:?}",
                        line.outcome
                    );
                    assert!(!chosen.exists());
                }
            } else {
                assert!(matches!(line.outcome, Some(ReceiveOutcome::Finished(_))));
                assert_eq!(std::fs::read(usual).unwrap(), bytes);
            }
        }
    }
}

/// Records the order the new laptop finished files in: which stream each start named, and each
/// "done" it answered, seen on its own connection.
struct Watched {
    inner: Mem,
    streams: std::sync::Arc<std::sync::Mutex<std::collections::HashMap<u32, ItemId>>>,
    finished: std::sync::Arc<std::sync::Mutex<Vec<ItemId>>>,
}

impl Channel for Watched {
    async fn send(&mut self, data: &[u8]) -> Result<(), ChannelError> {
        if let Ok(pctwin_transfer::Message::FileDone { stream, ok: true }) =
            pctwin_transfer::Message::decode(data)
            && let Some(item) = self.streams.lock().unwrap().get(&stream)
        {
            self.finished.lock().unwrap().push(*item);
        }
        self.inner.send(data).await
    }
    async fn recv(&mut self) -> Result<Vec<u8>, ChannelError> {
        let data = self.inner.recv().await?;
        if let Ok(pctwin_transfer::Message::StartFile { stream, item, .. }) =
            pctwin_transfer::Message::decode(&data)
        {
            self.streams.lock().unwrap().insert(stream, item);
        }
        Ok(data)
    }
}

/// A move of many pictures, of which the person's own essentials (the record's personal order)
/// are three near the end of the list: the order the new laptop finished them in, and how each
/// ended.
async fn finish_order(
    personal: &[usize],
) -> (Vec<ItemId>, Vec<ItemId>, Vec<Option<ReceiveOutcome>>) {
    let w = world();
    let mut record = w.record.clone();
    let pictures: Vec<Item> = (0..24)
        .map(|n| {
            let p = item(
                FolderRole::Pictures,
                &["Holiday", &format!("photo {n:02}.jpg")],
                ItemKind::File,
                64,
                Inclusion::Included,
            );
            write(&source_in(w.old.path(), &p), &[n as u8; 64]);
            p
        })
        .collect();
    record.items.extend(pictures.iter().cloned());
    record.personal = personal.iter().map(|n| pictures[*n].id).collect();
    record.validate().unwrap();
    let plan = send_plan(&record, |i| Some(source_in(w.old.path(), i)), landing);
    let (mut old, new) = mem_pair();
    let finished = std::sync::Arc::new(std::sync::Mutex::new(Vec::new()));
    let mut new = Watched {
        inner: new,
        streams: Default::default(),
        finished: finished.clone(),
    };
    let mut steps = person(true);
    let (quiet, quiet_old) = (Live::default(), Live::default());
    let (sent, received) = no_hang(async {
        tokio::join!(
            pctwin_move::send(&mut old, &record, plan, &quiet_old),
            pctwin_move::receive(&mut new, &mut steps, &w.table, &w.journal, "1001", &quiet),
        )
    })
    .await;
    sent.unwrap();
    let received = received.unwrap();
    let order = finished.lock().unwrap().clone();
    let outcomes = received.files.iter().map(|f| f.outcome.clone()).collect();
    (order, record.personal.clone(), outcomes)
}

/// The person's own essentials (Task List 1.5) are sent and finished on the new laptop before
/// anything else, in their order; everything still ends as it would.
#[tokio::test]
async fn the_persons_own_essentials_arrive_before_anything_else() {
    let (order, personal, outcomes) = finish_order(&[21, 19, 23]).await;
    // All of them before anything else (side by side, a few files are sent at once, so among
    // themselves they may end in any order).
    let first: std::collections::BTreeSet<ItemId> =
        order[..personal.len()].iter().copied().collect();
    let wanted: std::collections::BTreeSet<ItemId> = personal.iter().copied().collect();
    assert_eq!(first, wanted, "{order:?}");
    assert!(
        outcomes
            .iter()
            .all(|o| matches!(o, Some(ReceiveOutcome::Finished(_)))),
        "{outcomes:?}"
    );
}

/// With no essentials of the person's own, the move goes as it always did: nothing is put in
/// the personal tier, and every file ends as before.
#[tokio::test]
async fn a_move_with_no_personal_essentials_goes_as_before() {
    let (order, personal, outcomes) = finish_order(&[]).await;
    assert!(personal.is_empty());
    assert_eq!(order.len(), outcomes.len());
    assert!(
        outcomes
            .iter()
            .all(|o| matches!(o, Some(ReceiveOutcome::Finished(_)))),
        "{outcomes:?}"
    );
    let w = world();
    assert!(
        pctwin_move::move_order(&w.record)
            .iter()
            .all(|(_, tier)| *tier != pctwin_transfer::Tier::Personal)
    );
}

/// Gap found 24 October 2026, end to end: a folder that moves as one whole (here a Photos Library)
/// is sent as its files, each with its own result, and lands whole. The new laptop already has a
/// library of that name, so this one lands beside it under a name of its own, the one there left
/// exactly as it was: never merged.
#[tokio::test]
async fn a_whole_folder_is_sent_as_its_files_and_lands_beside_one_already_there() {
    use pctwin_move::landing::LandingRules;
    let mut w = world();
    let lib = "Photos Library.photoslibrary";
    let whole = item(
        FolderRole::Pictures,
        &[lib],
        ItemKind::Package,
        0,
        Inclusion::Included,
    );
    let inside: [(&[&str], &[u8]); 2] = [
        (&[lib, "originals", "A", "beach.heic"], b"the beach"),
        (&[lib, "database", "Photos.sqlite"], b"rows"),
    ];
    w.record.items.push(whole);
    for (path, bytes) in inside {
        let f = item(
            FolderRole::Pictures,
            path,
            ItemKind::File,
            bytes.len() as u64,
            Inclusion::Included,
        );
        write(&source_in(w.old.path(), &f), bytes);
        w.record.items.push(f);
    }
    // A second library on the old laptop already named like the first one's new name here (A).
    let second = "Photos Library (2).photoslibrary";
    w.record.items.push(item(
        FolderRole::Pictures,
        &[second],
        ItemKind::Package,
        0,
        Inclusion::Included,
    ));
    let other = item(
        FolderRole::Pictures,
        &[second, "originals", "b.heic"],
        ItemKind::File,
        5,
        Inclusion::Included,
    );
    write(&source_in(w.old.path(), &other), b"boats");
    w.record.items.push(other.clone());
    w.record.validate().unwrap();
    // The new laptop's own library, already there.
    let theirs = w
        .new
        .path()
        .join("Pictures")
        .join(lib)
        .join("database/Photos.sqlite");
    write(&theirs, b"the new laptop's own");

    let rules = LandingRules {
        mover: "1000".into(),
        roles: ROLES
            .iter()
            .map(|(r, _)| (*r, Storage::SystemDrive))
            .collect(),
        fallback: Default::default(),
        held: Default::default(),
        left_out: Default::default(),
        chosen: Default::default(),
    };
    struct Mine {
        rules: LandingRules,
    }
    impl Steps for Mine {
        async fn person(&mut self, _: &Record, _: &str) -> Result<PersonTarget, Stop> {
            Ok(PersonTarget::ExistingAccount {
                account_id: "1001".into(),
            })
        }
        async fn approve(&mut self, _: &Record, _: &Preview) -> Result<(), Stop> {
            Ok(())
        }
        fn landing(&self, item: &Item) -> Option<LandingPlace> {
            self.rules.landing(item)
        }
    }
    let mut steps = Mine { rules };
    let (mut old, mut new) = mem_pair();
    let (old_live, new_live) = (Live::default(), Live::default());
    let mut table = std::mem::take(&mut w.table);
    let (sent, received) = no_hang(async {
        tokio::join!(
            send_greeted(
                &mut old,
                &w.record,
                "1000",
                &[],
                |i| Some(source_in(w.old.path(), i)),
                &old_live,
                std::sync::Arc::new(pctwin_transfer::FsOpener),
            ),
            receive_greeted(
                &mut new,
                &mut steps,
                |s, _| s.rules.clone(),
                &mut table,
                &w.journal,
                "1001",
                &new_live
            ),
        )
    })
    .await;
    let (sent, said) = sent.unwrap();
    let received = received.unwrap();
    // Every file of the library was sent and has its own result.
    for line in &sent.files {
        assert_eq!(line.outcome, Some(SendOutcome::Arrived), "{:?}", line.item);
    }
    let ours = "Photos Library (2).photoslibrary".to_string();
    // Decided on this laptop alone: the old laptop was told nothing of the name it lands under.
    assert!(!serde_json::to_string(&said).unwrap().contains("(2)"));
    for (path, bytes) in inside {
        let id = w
            .record
            .items
            .iter()
            .find(|i| i.path.parts().len() == path.len() && shown_path(&i.path) == path.join("/"))
            .unwrap()
            .id;
        let line = received.files.iter().find(|f| f.item == id).unwrap();
        let Some(ReceiveOutcome::Finished(done)) = &line.outcome else {
            panic!("{path:?} ended {:?}", line.outcome);
        };
        let rest = path[1..].join("/");
        assert_eq!(done.final_path, format!("{ours}/{rest}"));
        let landed = w.new.path().join("Pictures").join(&ours).join(&rest);
        assert_eq!(std::fs::read(landed).unwrap(), bytes);
    }
    // The second library lands apart from both: never merged into the first one's new place.
    let line = received.files.iter().find(|f| f.item == other.id).unwrap();
    let Some(ReceiveOutcome::Finished(done)) = &line.outcome else {
        panic!("the second library ended {:?}", line.outcome);
    };
    assert_eq!(
        done.final_path,
        "Photos Library (2) (2).photoslibrary/originals/b.heic"
    );
    let first_new = w.new.path().join("Pictures").join(&ours);
    assert!(!first_new.join("originals/b.heic").exists());
    // The library already there is exactly as it was, and holds nothing of the move.
    assert_eq!(std::fs::read(&theirs).unwrap(), b"the new laptop's own");
    let there: Vec<_> = std::fs::read_dir(w.new.path().join("Pictures").join(lib))
        .unwrap()
        .flatten()
        .map(|e| e.file_name())
        .collect();
    assert_eq!(there, ["database"]);
}

/// A ledger that stops working at one step (`at`), as PCTwin stopping there would leave it.
struct StopsAt<'l> {
    inner: &'l dyn pctwin_journal::Ledger,
    at: &'static str,
}

impl StopsAt<'_> {
    fn check(&self, step: &str) -> Result<(), pctwin_journal::JournalError> {
        if step == self.at {
            Err(pctwin_journal::JournalError::Damaged("stopped here"))
        } else {
            Ok(())
        }
    }
}

impl pctwin_journal::Ledger for StopsAt<'_> {
    fn temp_tag(&self, id: u64) -> String {
        self.inner.temp_tag(id)
    }
    fn plan(&self, w: &pctwin_journal::PlannedWrite) -> Result<u64, pctwin_journal::JournalError> {
        self.check("plan")?;
        self.inner.plan(w)
    }
    fn staged(
        &self,
        id: u64,
        temp: &str,
        made: &[(String, Option<pctwin_journal::FileId>)],
    ) -> Result<(), pctwin_journal::JournalError> {
        self.check("staged")?;
        self.inner.staged(id, temp, made)
    }
    fn verified(
        &self,
        id: u64,
        fp: [u8; 32],
        file: Option<pctwin_journal::FileId>,
    ) -> Result<(), pctwin_journal::JournalError> {
        self.check("verified")?;
        self.inner.verified(id, fp, file)
    }
    fn verified_as(
        &self,
        id: u64,
        fp: [u8; 32],
        file: Option<pctwin_journal::FileId>,
        checked: Option<pctwin_journal::CheckedFrom>,
    ) -> Result<(), pctwin_journal::JournalError> {
        self.check("verified")?;
        self.inner.verified_as(id, fp, file, checked)
    }
    fn applied(&self, id: u64, final_path: &str) -> Result<(), pctwin_journal::JournalError> {
        self.check("applied")?;
        self.inner.applied(id, final_path)
    }
    fn filling(
        &self,
        id: u64,
        final_path: &str,
        tag: pctwin_journal::FillTag,
        proof: pctwin_journal::Proof,
    ) -> Result<(), pctwin_journal::JournalError> {
        self.check("filling")?;
        self.inner.filling(id, final_path, tag, proof)
    }
    fn fill_made(
        &self,
        id: u64,
        made: pctwin_journal::FileId,
    ) -> Result<(), pctwin_journal::JournalError> {
        self.inner.fill_made(id, made)
    }
    fn filled(&self, id: u64) -> Result<(), pctwin_journal::JournalError> {
        self.inner.filled(id)
    }
    fn fill_cleared(
        &self,
        id: u64,
        left: Option<&str>,
    ) -> Result<(), pctwin_journal::JournalError> {
        self.inner.fill_cleared(id, left)
    }
    fn committed_showing(
        &self,
        id: u64,
        landed: pctwin_journal::Landed,
        show: &pctwin_journal::Show,
    ) -> Result<(), pctwin_journal::JournalError> {
        self.check("committed")?;
        self.inner.committed_showing(id, landed, show)
    }
    fn shown(
        &self,
        id: u64,
        left: Option<&str>,
        file: Option<pctwin_journal::FileId>,
    ) -> Result<(), pctwin_journal::JournalError> {
        self.inner.shown(id, left, file)
    }
    fn committed(
        &self,
        id: u64,
        landed: pctwin_journal::Landed,
    ) -> Result<(), pctwin_journal::JournalError> {
        self.check("committed")?;
        self.inner.committed(id, landed)
    }
    fn existing(&self, id: u64, stored_path: &str) -> Result<(), pctwin_journal::JournalError> {
        self.inner.existing(id, stored_path)
    }
    fn failed(&self, id: u64, why: &str) -> Result<(), pctwin_journal::JournalError> {
        self.inner.failed(id, why)
    }
    fn kept_aside(
        &self,
        id: u64,
        why: &str,
        kept_path: &str,
        fingerprint: [u8; 32],
        landed: pctwin_journal::Landed,
    ) -> Result<(), pctwin_journal::JournalError> {
        self.inner
            .kept_aside(id, why, kept_path, fingerprint, landed)
    }
    fn checkpoint(
        &self,
        id: u64,
        blocks: &[(u64, [u8; 32])],
        durable: bool,
    ) -> Result<(), pctwin_journal::JournalError> {
        self.inner.checkpoint(id, blocks, durable)
    }
    fn blocks(&self, id: u64) -> Result<Vec<(u64, [u8; 32])>, pctwin_journal::JournalError> {
        self.inner.blocks(id)
    }
    fn unfinished(&self) -> Result<Vec<pctwin_journal::Entry>, pctwin_journal::JournalError> {
        self.inner.unfinished()
    }
}

/// Every file in the world is another person's ("1000"), held for account "1001" in `holding`
/// under the label `for-1`; the move stops at the journal step `at`.
async fn held_move_stopping_at(w: &mut World, holding: &Path, at: &'static str) {
    use pctwin_move::held_journal::{HeldPlaces, SplitLedger};
    use pctwin_move::landing::LandingRules;
    let rules = LandingRules {
        mover: "2000".into(),
        roles: ROLES
            .iter()
            .map(|(r, _)| (*r, Storage::SystemDrive))
            .collect(),
        fallback: Default::default(),
        held: Default::default(),
        left_out: Default::default(),
        chosen: Default::default(),
    };
    struct Held {
        rules: LandingRules,
    }
    impl Steps for Held {
        async fn person(&mut self, _: &Record, _: &str) -> Result<PersonTarget, Stop> {
            Ok(PersonTarget::ExistingAccount {
                account_id: "1001".into(),
            })
        }
        async fn approve(&mut self, _: &Record, _: &Preview) -> Result<(), Stop> {
            Ok(())
        }
        fn landing(&self, item: &Item) -> Option<LandingPlace> {
            self.rules.landing(item)
        }
    }
    std::fs::create_dir_all(holding).unwrap();
    let mut steps = Held { rules };
    let places = HeldPlaces::default();
    let split = SplitLedger::new(&w.journal, places.clone());
    let ledger = StopsAt { inner: &split, at };
    let (mut old, new) = mem_pair();
    let (old_live, new_live) = (Live::default(), Live::default());
    let mut table = std::mem::take(&mut w.table);
    let (_, received) = no_hang(async {
        tokio::join!(
            send_greeted(
                &mut old,
                &w.record,
                "2000",
                &[],
                |i| Some(source_in(w.old.path(), i)),
                &old_live,
                std::sync::Arc::new(pctwin_transfer::FsOpener),
            ),
            async {
                // Stopped: this end goes away, as a stopped PCTwin's would, so the old laptop
                // stops waiting.
                let mut new = new;
                let received = receive_greeted(
                    &mut new,
                    &mut steps,
                    |s, table| {
                        table.approve_holding("for-1", "1001", holding).unwrap();
                        places.hold("for-1", holding);
                        let mut rules = s.rules.clone();
                        rules.held.insert("1000".into(), "for-1".into());
                        rules
                    },
                    &mut table,
                    &ledger,
                    "1001",
                    &new_live,
                )
                .await;
                drop(new);
                received
            },
        )
    })
    .await;
    // PCTwin stopped: the move did not finish.
    assert!(received.is_err(), "the move should have stopped at {at}");
}

/// Every file under `dir`, by its path below it.
fn files_under(dir: &Path) -> Vec<PathBuf> {
    let mut out = Vec::new();
    let Ok(entries) = std::fs::read_dir(dir) else {
        return out;
    };
    for e in entries.flatten() {
        let p = e.path();
        if p.is_dir() {
            out.extend(files_under(&p));
        } else {
            out.push(p);
        }
    }
    out
}

fn is_temp(p: &Path) -> bool {
    p.file_name()
        .is_some_and(|n| pctwin_gate::is_temp_name(&n.to_string_lossy()))
}

/// PCTwin stops between a write into another person's holding place and its naming. At the next
/// start, recovery finds that holding place's journal (remembered with no file name), proves the
/// copy again and names it, with counts only coming back; the mover's own journal still names
/// none of their files.
#[tokio::test]
async fn a_held_write_stopped_before_its_naming_is_finished_by_recovery() {
    use pctwin_move::held_journal::{HELD_JOURNAL, HeldPlace, recover_held, unfinished_held};
    let mut w = world();
    let holding = w.new.path().join("holding-1000");
    held_move_stopping_at(&mut w, &holding, "committed").await;
    let place = HeldPlace {
        label: "for-1".into(),
        account: "1001".into(),
        folder: holding.clone(),
    };
    let before = files_under(&holding);
    assert!(before.iter().any(|p| is_temp(p)), "{before:?}");
    assert!(unfinished_held(std::slice::from_ref(&place)) >= 1);
    let done = recover_held(std::slice::from_ref(&place));
    assert!(done.put_in_place >= 1, "{done:?}");
    assert_eq!(done.left_to_hand_over, 0);
    // What is still unfinished is only what was part copied, kept to carry on.
    assert_eq!(
        unfinished_held(std::slice::from_ref(&place)),
        done.kept_to_continue
    );
    // Every file now under its own name is whole; the temporary files left are only those kept.
    let after = files_under(&holding);
    let named: Vec<&PathBuf> = after
        .iter()
        .filter(|p| !is_temp(p) && !p.ends_with(HELD_JOURNAL))
        .collect();
    assert_eq!(named.len() as u64, done.put_in_place, "{after:?}");
    for f in named {
        let bytes = std::fs::read(f).unwrap();
        assert!(
            FILES.iter().any(|(_, _, b)| *b == bytes.as_slice()),
            "{f:?}"
        );
    }
    assert_eq!(
        after.iter().filter(|p| is_temp(p)).count() as u64,
        done.kept_to_continue,
        "{after:?}"
    );
    // Run again: nothing more is named or failed.
    let again = recover_held(&[place]);
    assert_eq!((again.put_in_place, again.not_finished), (0, 0));
    drop(w.journal);
    let mine = std::fs::read(w._jdir.path().join("journal.redb")).unwrap();
    for name in FILES.iter().flat_map(|(_, path, _)| path.iter()) {
        assert!(
            !mine.windows(name.len()).any(|x| x == name.as_bytes()),
            "the mover's journal names {name}"
        );
    }
}

/// A holding place the mover may no longer open (handed over, or locked for it) is left to its
/// hand-over; one that is gone is nothing to do.
#[test]
fn a_holding_place_the_mover_cannot_open_is_left_to_its_hand_over() {
    use pctwin_move::held_journal::{HELD_JOURNAL, HeldPlace, recover_held};
    let dir = tempfile::tempdir().unwrap();
    let gone = HeldPlace {
        label: "for-1".into(),
        account: "1001".into(),
        folder: dir.path().join("gone"),
    };
    assert_eq!(
        recover_held(std::slice::from_ref(&gone)),
        Default::default()
    );
    // Something other than a journal at the journal's name is never opened.
    let odd = dir.path().join("odd");
    std::fs::create_dir_all(odd.join(HELD_JOURNAL)).unwrap();
    let odd = HeldPlace {
        folder: odd,
        ..gone
    };
    assert_eq!(recover_held(&[odd]).left_to_hand_over, 1);
}

/// A hand-over after PCTwin stopped mid-move, before any recovery: a temporary file the move
/// left is never handed over (the person never gets half a file); it is listed as unfinished.
#[tokio::test]
async fn a_hand_over_after_a_stop_never_carries_an_unfinished_file() {
    let mut w = world();
    let holding = w.new.path().join("holding-1000");
    held_move_stopping_at(&mut w, &holding, "committed").await;
    let left: Vec<PathBuf> = files_under(&holding)
        .into_iter()
        .filter(|p| is_temp(p))
        .collect();
    assert!(!left.is_empty());
    let home = tempfile::tempdir().unwrap();
    let jdir = tempfile::tempdir().unwrap();
    let journal = Journal::open(&jdir.path().join("handover.journal")).unwrap();
    let places = std::collections::BTreeMap::from([
        ("documents".to_string(), "Documents".to_string()),
        ("pictures".to_string(), "Pictures".to_string()),
    ]);
    let done = pctwin_move::handover::hand_over(&holding, home.path(), &places, &journal, "1001")
        .await
        .unwrap();
    assert_eq!(done.unfinished.len(), left.len(), "{:?}", done.unfinished);
    for f in files_under(home.path()) {
        assert!(!is_temp(&f), "{f:?} handed over");
        assert!(
            !f.to_string_lossy().contains(".pctwin-"),
            "{f:?} handed over"
        );
    }
    for f in &done.files {
        assert!(!is_temp(&f.held), "{:?}", f.held);
    }
}

/// Task List 1.8, sleep: the move is part way and the laptop sleeps for an hour (paused by the
/// person first, or not): both its clocks jump (the time the engine's waits run on, and the
/// calendar). Then the move either finishes with every file whole or ends honestly (each file that
/// did not arrive said so): never a file said arrived that did not, never a hang. How each side
/// ended.
async fn sleeps_an_hour_mid_move(paused: bool) -> (Ended, Ended) {
    let mut w = world();
    // A file big enough to be part way when the laptop sleeps.
    let big = item(
        FolderRole::Documents,
        &["video.bin"],
        ItemKind::File,
        6 * 1024 * 1024,
        Inclusion::Included,
    );
    let bytes: Vec<u8> = (0..big.size_bytes).map(|i| (i * 31 % 251) as u8).collect();
    write(&source_in(w.old.path(), &big), &bytes);
    w.record.items.push(big.clone());
    w.record.validate().unwrap();
    let plan = send_plan(&w.record, |i| Some(source_in(w.old.path(), i)), landing);
    let (mut old, mut new) = mem_pair();
    let mut steps = person(true);
    let (old_live, new_live) = (Live::default(), Live::default());
    let sleep = async {
        // Part way: some of the file confirmed, not all.
        while old_live.meter.snapshot().reading.bytes_confirmed < 1024 * 1024 {
            tokio::time::sleep(Duration::from_millis(1)).await;
        }
        if paused {
            old_live.control.pause();
            tokio::time::sleep(Duration::from_millis(50)).await;
        }
        // An hour asleep: both clocks move on together.
        pctwin_pairing::clock::move_calendar(Duration::from_secs(3600));
        tokio::time::advance(Duration::from_secs(3600)).await;
        if paused {
            old_live.control.carry_on();
        }
    };
    let ((sent, received), ()) = no_hang(async {
        tokio::join!(
            async {
                tokio::join!(
                    pctwin_move::send(&mut old, &w.record, plan, &old_live),
                    pctwin_move::receive(
                        &mut new, &mut steps, &w.table, &w.journal, "1001", &new_live
                    ),
                )
            },
            sleep
        )
    })
    .await;
    // Whatever each side came to, nothing is said to have arrived that did not.
    let landed_ok = |item: &Item, expect: &[u8]| {
        let at = w
            .new
            .path()
            .join(folder_name(item.place.role))
            .join(shown_path(&item.path));
        std::fs::read(at).is_ok_and(|b| b == expect)
    };
    let source_of = |item: &Item| std::fs::read(source_in(w.old.path(), item)).unwrap();
    if let Ok(sent) = &sent {
        for line in &sent.files {
            let item = w.record.items.iter().find(|i| i.id == line.item).unwrap();
            if line.outcome == Some(SendOutcome::Arrived) {
                assert!(
                    landed_ok(item, &source_of(item)),
                    "{:?} said arrived",
                    item.path
                );
            }
        }
    }
    if let Ok(received) = &received {
        for line in &received.files {
            let item = w.record.items.iter().find(|i| i.id == line.item).unwrap();
            if matches!(line.outcome, Some(ReceiveOutcome::Finished(_))) {
                assert!(
                    landed_ok(item, &source_of(item)),
                    "{:?} said copied",
                    item.path
                );
            }
        }
        // It carried on: every file finished, the big one whole.
        if received.ended == pctwin_move::Ended::Finished {
            assert!(landed_ok(&big, &bytes));
        }
    }
    // One way or the other, the move came to an end on both laptops (no hang), and said how.
    let ended = |r: Result<Ended, &MoveError>| r.unwrap_or(Ended::LinkLost);
    (
        ended(sent.as_ref().map(|s| s.ended)),
        ended(received.as_ref().map(|r| r.ended)),
    )
}

/// The person paused before the laptop slept an hour: it carries on, every file whole.
#[tokio::test(start_paused = true)]
async fn a_laptop_paused_then_asleep_an_hour_carries_on_with_every_file_whole() {
    assert_eq!(
        sleeps_an_hour_mid_move(true).await,
        (Ended::Finished, Ended::Finished)
    );
}

/// The laptop slept an hour in the middle of sending, unpaused: whatever it comes to (it carries
/// on, or ends as a lost connection), nothing is said to have arrived that did not, and it ends.
#[tokio::test(start_paused = true)]
async fn a_laptop_asleep_an_hour_mid_send_carries_on_or_ends_honestly() {
    let ends = sleeps_an_hour_mid_move(false).await;
    assert!(
        matches!(ends.1, Ended::Finished | Ended::LinkLost),
        "{ends:?}"
    );
}

/// The old laptop's end of the connection, which says one thing out of place: right after the
/// last of the first file it starts is sent, a `FileDone` (only the new laptop ever says that).
struct StrayAfter {
    inner: Mem,
    /// The first file started, and its stream.
    first: Option<(ItemId, u32)>,
    said: bool,
}

impl Channel for StrayAfter {
    async fn send(&mut self, data: &[u8]) -> Result<(), ChannelError> {
        use pctwin_transfer::Message;
        let heard = Message::decode(data).ok();
        self.inner.send(data).await?;
        match heard {
            Some(Message::StartFile { stream, item, .. }) if self.first.is_none() => {
                self.first = Some((item, stream));
            }
            Some(Message::EndFile { stream, .. })
                if self.first.is_some_and(|(_, s)| s == stream) && !self.said =>
            {
                self.said = true;
                let stray = Message::FileDone { stream, ok: true };
                self.inner.send(&stray.encode()).await?;
            }
            _ => {}
        }
        Ok(())
    }
    async fn recv(&mut self) -> Result<Vec<u8>, ChannelError> {
        self.inner.recv().await
    }
}

/// Fresh-context review of TRANSFER (24 October 2026): one message out of place halfway through
/// ends the move, but never loses the report of what already landed: the file that landed is
/// copied (and on the disk), and every file the end cut off failed, with why.
#[tokio::test]
async fn a_message_out_of_place_mid_move_keeps_the_report_of_what_landed() {
    let w = world();
    let plan = send_plan(&w.record, |i| Some(source_in(w.old.path(), i)), landing);
    let (old, new) = mem_pair();
    let mut old = StrayAfter {
        inner: old,
        first: None,
        said: false,
    };
    let mut steps = person(true);
    let (quiet, quiet_old) = (Live::default(), Live::default());
    let (_, received) = no_hang(async {
        tokio::join!(
            pctwin_move::send(&mut old, &w.record, plan, &quiet_old),
            async {
                // The new laptop lets the connection go when its side ends, as a real one does.
                let mut new = new;
                pctwin_move::receive(&mut new, &mut steps, &w.table, &w.journal, "1001", &quiet)
                    .await
            }
        )
    })
    .await;
    let received = received.expect("the report is kept");
    let (first, _) = old.first.expect("a file was started");
    assert_eq!(received.ended, Ended::Broke);
    assert_eq!(received.files.len(), FILES.len());
    let mut cut = 0;
    for (line, (role, path, bytes)) in received.files.iter().zip(FILES) {
        let landed = w.new.path().join(folder_name(role)).join(path.join("/"));
        match &line.outcome {
            Some(ReceiveOutcome::Finished(_)) => {
                assert_eq!(std::fs::read(&landed).unwrap(), bytes);
            }
            Some(ReceiveOutcome::Failed(why)) => {
                assert_ne!(
                    line.item, first,
                    "the first file landed before the stray message"
                );
                assert_eq!(why.cause(), Cause::MoveEnded, "{why}");
                assert!(why.why().starts_with(pctwin_move::BROKE_OFF), "{why}");
                assert!(!landed.exists(), "{landed:?}");
                cut += 1;
            }
            other => panic!("{path:?} ended {other:?}"),
        }
    }
    assert!(
        matches!(
            received
                .files
                .iter()
                .find(|f| f.item == first)
                .map(|f| &f.outcome),
            Some(Some(ReceiveOutcome::Finished(_)))
        ),
        "the first file is reported copied"
    );
    assert!(
        cut > 0,
        "the stray message came before the last file landed"
    );
}

/// Another person's files, with no place when the plan is approved, held in a place made after
/// the approval (as in the test above), the person answering each approval with `second` after
/// the first. Every file but the first is `size` in the plan (if given). What the move came to, and every
/// preview the person was shown.
async fn held_after_approval(
    size: Option<u64>,
    second: Result<(), Stop>,
) -> (
    Result<pctwin_move::Received, MoveError>,
    Vec<Preview>,
    World,
) {
    use pctwin_move::landing::LandingRules;
    let mut w = world();
    if let Some(size) = size {
        for i in w
            .record
            .items
            .iter_mut()
            .filter(|i| i.kind == ItemKind::File)
            .skip(1)
        {
            i.size_bytes = size;
        }
    }
    let before = LandingRules {
        mover: "2000".into(),
        roles: ROLES
            .iter()
            .map(|(r, _)| (*r, Storage::SystemDrive))
            .collect(),
        fallback: Default::default(),
        held: Default::default(),
        left_out: Default::default(),
        chosen: Default::default(),
    };
    struct Counting {
        rules: LandingRules,
        shown: Vec<Preview>,
        second: Result<(), Stop>,
    }
    impl Steps for Counting {
        async fn person(&mut self, _: &Record, _: &str) -> Result<PersonTarget, Stop> {
            Ok(PersonTarget::ExistingAccount {
                account_id: "1001".into(),
            })
        }
        async fn approve(&mut self, _: &Record, preview: &Preview) -> Result<(), Stop> {
            self.shown.push(preview.clone());
            if self.shown.len() == 1 {
                Ok(())
            } else {
                self.second
            }
        }
        fn landing(&self, item: &Item) -> Option<LandingPlace> {
            self.rules.landing(item)
        }
    }
    let holding = w.new.path().join("holding-1000");
    std::fs::create_dir_all(&holding).unwrap();
    let mut steps = Counting {
        rules: before,
        shown: Vec::new(),
        second,
    };
    let (mut old, new) = mem_pair();
    let (old_live, new_live) = (Live::default(), Live::default());
    let mut table = std::mem::take(&mut w.table);
    let (_, received) = no_hang(async {
        tokio::join!(
            send_greeted(
                &mut old,
                &w.record,
                "2000",
                &[],
                |i| Some(source_in(w.old.path(), i)),
                &old_live,
                std::sync::Arc::new(pctwin_transfer::FsOpener),
            ),
            async {
                let mut new = new;
                receive_greeted(
                    &mut new,
                    &mut steps,
                    |s, table| {
                        table.approve_holding("for-1", "1001", &holding).unwrap();
                        let mut rules = s.rules.clone();
                        rules.held.insert("1000".into(), "for-1".into());
                        s.rules = rules.clone();
                        rules
                    },
                    &mut table,
                    &w.journal,
                    "1001",
                    &new_live,
                )
                .await
            },
        )
    })
    .await;
    (received, steps.shown, w)
}

/// Fresh-context review of TRANSFER (24 October 2026): what the person approved is what is sent.
/// Another person's files have no place when the plan is approved, so the preview could not count
/// where they land; once their place is made, if it does not fit where the person saw it fit,
/// the person sees the plan as it will run and approves it again. Here they stop, and nothing is
/// written.
#[tokio::test]
async fn a_plan_that_no_longer_fits_once_everyone_is_placed_is_approved_again() {
    let huge = 1u64 << 50;
    let (received, shown, w) = held_after_approval(Some(huge), Err(Stop::NotApproved)).await;
    assert!(
        matches!(received, Err(MoveError::Stopped(Stop::NotApproved))),
        "{:?}",
        received.map(|r| r.ended)
    );
    assert_eq!(shown.len(), 2, "approved again");
    assert!(shown[0].fit.places.iter().all(|p| p.fits() != Some(false)));
    assert!(shown[1].fit.places.iter().any(|p| p.fits() == Some(false)));
    assert!(shown[1].bytes >= huge, "{:?}", shown[1]);
    assert!(w.journal.entries().unwrap().is_empty());
}

/// And when everyone's place still fits, nothing is asked again: the files land.
#[tokio::test]
async fn a_plan_that_still_fits_once_everyone_is_placed_is_not_asked_again() {
    let (received, shown, _w) = held_after_approval(None, Ok(())).await;
    assert_eq!(shown.len(), 1);
    let received = received.unwrap();
    assert_eq!(received.ended, Ended::Finished);
    assert!(
        received
            .files
            .iter()
            .all(|f| matches!(f.outcome, Some(ReceiveOutcome::Finished(_)))),
        "{:?}",
        received.files
    );
}
