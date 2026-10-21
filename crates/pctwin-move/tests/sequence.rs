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
    let long = format!("{}.txt", "x".repeat(300));
    let names = [
        "plain.txt",
        "CON.txt",
        "a:b?.txt",
        "LONGFO~1.txt",
        "spaces at the end. ",
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
        let source = w.old.path().join(format!("source-{n}.bin"));
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
            pctwin_move::receive(
                &mut new, &mut steps, &w.table, &w.journal, "1001", &quiet_new
            ),
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
        let predicted = w
            .table
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
