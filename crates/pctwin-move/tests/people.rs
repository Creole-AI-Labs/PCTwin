//! A move of two people: the mover's own files from the disk, and an approved person's found and
//! read by the administrator helper (here a real helper session over a local connection, without
//! administrator rights). The other person's files are planned by handle, read only through the
//! helper, and land where the plan puts them, byte for byte.
// Tests make files to test on; the no-file-changes fence is for the shipped code.
#![allow(clippy::disallowed_methods)]

use std::collections::BTreeSet;
use std::net::{TcpListener, TcpStream};
use std::path::{Path, PathBuf};
use std::sync::{Arc, Mutex};
use std::time::Duration;

use pctwin_gate::{Approved, Destinations};
use pctwin_helper::{Client, Entry, Refusal, Session};
use pctwin_journal::Journal;
use pctwin_move::people::{HelperOpener, add_people, helper_source};
use pctwin_move::{Live, Preview, Steps, Stop, send_plan};
use pctwin_record::{
    FolderRole, Inclusion, Item, ItemId, ItemKind, ItemName, ItemPath, LaptopId, ManagedBy, Owner,
    PersonTarget, Place, Portability, Record, Storage,
};
use pctwin_transfer::{
    Channel, ChannelError, LandingPlace, ReceiveOutcome, SendOutcome, shown_path,
};
use tokio::sync::mpsc;

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

const ADA: &str = "1000";
const BEA: &str = "1001";

fn laptop() -> LaptopId {
    LaptopId::from_hex("00112233445566778899aabbccddeeff").unwrap()
}

fn item(owner: &str, name: &str, size: u64, modified_ns: Option<i64>) -> Item {
    let owner = Owner::Person {
        account_id: owner.into(),
    };
    let place = Place {
        role: FolderRole::Documents,
        storage: Storage::SystemDrive,
    };
    let path = ItemPath::new(vec![ItemName::from_text(name)]).unwrap();
    Item {
        id: ItemId::derive(&laptop(), &owner, &place, &path),
        kind: ItemKind::File,
        owner,
        place,
        path,
        size_bytes: size,
        modified_ns,
        portability: Portability::Portable,
        managed_by: ManagedBy::Personal,
        download_bytes: None,
        inclusion: Inclusion::Included,
    }
}

fn nanos(p: &Path) -> Option<i64> {
    let d = std::fs::metadata(p)
        .ok()?
        .modified()
        .ok()?
        .duration_since(std::time::UNIX_EPOCH)
        .ok()?;
    i64::try_from(d.as_nanos()).ok()
}

/// Ada's file goes to her Documents; Bea's to the place approved for her.
fn landing(i: &Item) -> LandingPlace {
    let place = match &i.owner {
        Owner::Person { account_id } if account_id == BEA => "for-bea",
        _ => "documents",
    };
    LandingPlace::new(place, shown_path(&i.path))
}

struct Person;

impl Steps for Person {
    async fn person(&mut self, _record: &Record, account_id: &str) -> Result<PersonTarget, Stop> {
        Ok(PersonTarget::ExistingAccount {
            account_id: account_id.into(),
        })
    }
    async fn approve(&mut self, _record: &Record, _preview: &Preview) -> Result<(), Stop> {
        Ok(())
    }
    fn landing(&self, item: &Item) -> Option<LandingPlace> {
        Some(landing(item))
    }
}

#[tokio::test]
async fn an_approved_person_s_files_move_through_the_helper() {
    let old = tempfile::tempdir().unwrap();
    let new = tempfile::tempdir().unwrap();
    let jdir = tempfile::tempdir().unwrap();
    // Ada's file, on the disk as the mover can read it.
    let ada_file = old.path().join("ada.txt");
    std::fs::write(&ada_file, b"Ada's notes").unwrap();
    // Bea's files, which only the helper reads.
    let bea_home = old.path().join("bea");
    std::fs::create_dir_all(&bea_home).unwrap();
    let bea_files: Vec<(PathBuf, &[u8])> = vec![
        (bea_home.join("letter.txt"), b"Dear Bea"),
        (bea_home.join("big.bin"), &[7u8; 300_000]),
    ];
    for (p, bytes) in &bea_files {
        std::fs::write(p, bytes).unwrap();
    }
    let found: Vec<(Item, Entry)> = bea_files
        .iter()
        .map(|(p, bytes)| {
            let name = p.file_name().unwrap().to_str().unwrap();
            (
                item(BEA, name, bytes.len() as u64, nanos(p)),
                Entry {
                    path: p.clone(),
                    size: bytes.len() as u64,
                    modified_ns: nanos(p),
                },
            )
        })
        .collect();

    // The helper: one session over a local connection.
    let listener = TcpListener::bind("127.0.0.1:0").unwrap();
    let addr = listener.local_addr().unwrap();
    let helper = std::thread::spawn(move || {
        let (mut stream, _) = listener.accept().unwrap();
        let mut scanner = move |account: &str| -> Result<Vec<(Item, Entry)>, Refusal> {
            assert_eq!(account, BEA);
            Ok(found.clone())
        };
        Session::new(
            "tok".into(),
            BTreeSet::from([BEA.to_string()]),
            &mut scanner,
        )
        .serve(&mut stream)
        .unwrap();
    });
    let stream = TcpStream::connect(addr).unwrap();
    stream
        .set_read_timeout(Some(Duration::from_secs(60)))
        .unwrap();
    let mut client = Client::open(stream, "tok").unwrap();
    let bea = client.scan_person(BEA).unwrap();
    assert_eq!(bea.files, 2);

    // The record: Ada's own, and Bea's from the helper.
    let mut record = Record::new(laptop());
    record
        .items
        .push(item(ADA, "ada.txt", 11, nanos(&ada_file)));
    let handles = add_people(&mut record, std::slice::from_ref(&bea));
    record.validate().unwrap();
    let plan = send_plan(
        &record,
        |i| match handles.get(&i.id) {
            Some(h) => Some(helper_source(*h)),
            None => Some(ada_file.clone()),
        },
        landing,
    );
    // Bea's files are planned by handle, never by a path.
    for job in &plan.jobs {
        let by_helper = handles.contains_key(&job.item);
        assert_eq!(
            job.source
                .to_string_lossy()
                .starts_with("pctwin-helper-file:"),
            by_helper
        );
    }
    let client = Arc::new(Mutex::new(client));
    let opener = Arc::new(HelperOpener::new(
        Arc::clone(&client),
        std::slice::from_ref(&bea),
    ));

    let mut table = Destinations::new();
    std::fs::create_dir_all(new.path().join("docs")).unwrap();
    std::fs::create_dir_all(new.path().join("bea")).unwrap();
    table
        .approve("documents", Approved::MyFolders, &new.path().join("docs"))
        .unwrap();
    table
        .approve("for-bea", Approved::SharedFolder, &new.path().join("bea"))
        .unwrap();
    let journal = Journal::open(&jdir.path().join("journal.redb")).unwrap();
    let (mut a, mut b) = mem_pair();
    let (here, there) = (Live::default(), Live::default());
    let mut steps = Person;
    let (sent, received) = tokio::join!(
        pctwin_move::send_with(&mut a, &record, plan, &here, opener),
        pctwin_move::receive(&mut b, &mut steps, &table, &journal, ADA, &there),
    );
    let (sent, received) = (sent.unwrap(), received.unwrap());
    for line in &sent.files {
        assert_eq!(line.outcome, Some(SendOutcome::Arrived), "{:?}", line.item);
    }
    for line in &received.files {
        assert!(
            matches!(line.outcome, Some(ReceiveOutcome::Finished(_))),
            "{:?}",
            line.outcome
        );
    }
    assert_eq!(
        std::fs::read(new.path().join("docs/ada.txt")).unwrap(),
        b"Ada's notes"
    );
    for (p, bytes) in &bea_files {
        let landed = new.path().join("bea").join(p.file_name().unwrap());
        assert_eq!(&std::fs::read(landed).unwrap(), bytes);
    }
    // The mover's report: their own file by name, Bea as counts only, none of her names.
    let report = pctwin_move::report::for_the_mover(&received, ADA);
    assert_eq!(report.own.len(), 1);
    assert_eq!(report.own[0].path, "ada.txt");
    let bea = &report.people[BEA];
    assert_eq!((bea.copied, bea.failed, bea.bytes_copied), (2, 0, 300_008));
    let shown = format!("{report:?}");
    for name in ["letter", "big.bin"] {
        assert!(!shown.contains(name), "{name} named in {shown}");
    }
    // The move is over: the helper ends.
    let client = Arc::try_unwrap(client).ok().unwrap().into_inner().unwrap();
    client.bye().unwrap();
    helper.join().unwrap();
}
