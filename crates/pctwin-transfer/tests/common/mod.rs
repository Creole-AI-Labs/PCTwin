//! Shared by the move tests: a real approved plan for the files a test moves, built and approved
//! the way the app does, so a receiver only ever gets an allowance from an approval.

use pctwin_record::{
    FolderRole, Inclusion, Item, ItemId, ItemKind, ItemName, ItemPath, LaptopId, ManagedBy, Owner,
    PersonTarget, Place, Portability, Record, Storage,
};
use pctwin_transfer::Allowance;

/// The approved plan for these files (each with its size), as the new laptop holds it.
#[allow(dead_code)]
pub fn approved(files: &[(ItemId, u64)]) -> Allowance {
    let laptop = LaptopId::from_hex("00112233445566778899aabbccddeeff").unwrap();
    let mut record = Record::new(laptop);
    for (n, (id, size)) in files.iter().enumerate() {
        record.items.push(Item {
            id: *id,
            kind: ItemKind::File,
            owner: Owner::Person {
                account_id: "1000".into(),
            },
            place: Place {
                role: FolderRole::Documents,
                storage: Storage::SystemDrive,
            },
            path: ItemPath::new(vec![ItemName::from_text(&format!("file{n}"))]).unwrap(),
            size_bytes: *size,
            portability: Portability::Portable,
            managed_by: ManagedBy::Personal,
            download_bytes: None,
            inclusion: Inclusion::Included,
        });
    }
    record.mapping.people.insert(
        "1000".into(),
        PersonTarget::ExistingAccount {
            account_id: "1001".into(),
        },
    );
    let approval = record.approve().unwrap();
    Allowance::from_record(&record, &approval).unwrap()
}

/// A change journal for a test's receiver, kept until the test run ends.
#[allow(dead_code)]
pub fn journal() -> &'static pctwin_journal::Journal {
    let dir = Box::leak(Box::new(tempfile::tempdir().unwrap()));
    Box::leak(Box::new(
        pctwin_journal::Journal::open(&dir.path().join("journal.redb")).unwrap(),
    ))
}

/// Which file this is on its drive, as the gate tells files apart (the gate's own tests prove
/// that), worked out here independently of the crate under test.
#[allow(dead_code)]
pub fn identity_of(path: &std::path::Path) -> pctwin_journal::FileId {
    pctwin_gate::file_identity(&std::fs::File::open(path).unwrap()).unwrap()
}

/// A connection for a test: whole messages in order, in memory.
struct ConfirmedMem {
    tx: tokio::sync::mpsc::UnboundedSender<Vec<u8>>,
    rx: tokio::sync::mpsc::UnboundedReceiver<Vec<u8>>,
}

impl pctwin_transfer::Channel for ConfirmedMem {
    async fn send(&mut self, data: &[u8]) -> Result<(), pctwin_transfer::ChannelError> {
        self.tx
            .send(data.to_vec())
            .map_err(|_| pctwin_transfer::ChannelError)
    }
    async fn recv(&mut self) -> Result<Vec<u8>, pctwin_transfer::ChannelError> {
        self.rx.recv().await.ok_or(pctwin_transfer::ChannelError)
    }
}

/// A check of the originals for `journal`, made by the real `check_originals` against a scripted
/// old laptop that replies with exactly `answers` (each request gets the answers for the items
/// in it). The items asked about are the ones in `answers`; any other item has no answer.
#[allow(dead_code)]
pub async fn confirmed(
    journal: &pctwin_journal::Journal,
    answers: Vec<(ItemId, pctwin_transfer::OriginalNow)>,
) -> pctwin_transfer::Confirmed {
    use pctwin_transfer::{Channel, Message, check_originals};
    let items: Vec<ItemId> = answers.iter().map(|(i, _)| *i).collect();
    let (a_tx, b_rx) = tokio::sync::mpsc::unbounded_channel();
    let (b_tx, a_rx) = tokio::sync::mpsc::unbounded_channel();
    let mut new = ConfirmedMem { tx: a_tx, rx: a_rx };
    let mut old = ConfirmedMem { tx: b_tx, rx: b_rx };
    let script = async {
        let mut covered = 0;
        let mut seen = std::collections::HashSet::new();
        let unique = items.iter().filter(|i| seen.insert(**i)).count();
        while covered < unique {
            let bytes = old.recv().await.unwrap();
            let Message::CheckOriginals {
                nonce,
                items: asked,
            } = Message::decode(&bytes).unwrap()
            else {
                panic!("expected a request about the originals")
            };
            covered += asked.len();
            let reply: Vec<_> = answers
                .iter()
                .filter(|(i, _)| asked.contains(i))
                .copied()
                .collect();
            old.send(
                &Message::Originals {
                    nonce,
                    answers: reply,
                }
                .encode(),
            )
            .await
            .unwrap();
        }
    };
    let (token, ()) = tokio::join!(check_originals(&mut new, journal, &items), script);
    token.unwrap()
}

/// [`confirmed`] for a test that is not async (it runs its own small runtime, so do not call it
/// from inside one).
#[allow(dead_code)]
pub fn confirmed_blocking(
    journal: &pctwin_journal::Journal,
    answers: Vec<(ItemId, pctwin_transfer::OriginalNow)>,
) -> pctwin_transfer::Confirmed {
    tokio::runtime::Builder::new_current_thread()
        .enable_time()
        .build()
        .unwrap()
        .block_on(confirmed(journal, answers))
}
