//! Handing another person's files over (Security Design part C, `HandOver`): from the holding
//! area, where the move left them, into that person's own folders, through the very same write
//! path a move uses (the sender reading the holding area, the receiver landing through the safety
//! gate with every step in the change journal): nothing at a name is ever replaced, no link is
//! followed, and every file is read back before it is called landed.
//!
//! The holding area has one folder per place (`<holding>/<place label>/...`); `places` says
//! which folder of the person's home each label lands in (made by the safety gate if missing). On Mac and Linux the administrator helper
//! runs this as the person themselves, so it writes with their rights only; on Windows the
//! helper runs it with its own.

use std::collections::BTreeMap;
use std::path::{Path, PathBuf};

use pctwin_gate::{Approved, Destinations};
use pctwin_journal::Ledger;
use pctwin_record::{
    FolderRole, Inclusion, Item, ItemId, ItemKind, ItemName, ItemPath, LaptopId, ManagedBy, Owner,
    PersonTarget, Place, Portability, Record, Storage,
};
use pctwin_transfer::{
    Allowance, Channel, ChannelError, LandingPlace, ReceiveOutcome, ReceiverSession, SendJob,
    SenderSession, Tier, TransferError, shown_path,
};
use tokio::sync::mpsc;

/// The usual places a person's files land in inside their home, by the move's place labels:
/// the system's own folder names, and one folder for everything else.
/// What a hand-over left for its person to read once ("Your things are ready", M32): counts
/// only. Kept in the person's own app data folder, where PCTwin finds it when it next opens.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ReadyNote {
    /// In their own folders now.
    pub landed: u64,
    /// Kept aside, because a file of that name was already there (or it needs a step).
    pub kept: u64,
    /// Could not be handed over.
    pub failed: u64,
}

/// The note's file inside an app data folder (`data_dir`, the person's own): the same folder
/// the app keeps its data in (`<data>/com.creoleailabs.pctwin`).
pub fn ready_note_path(data_dir: &Path) -> PathBuf {
    data_dir
        .join("com.creoleailabs.pctwin")
        .join("things-ready.json")
}

impl ReadyNote {
    /// A note as PCTwin wrote it, or `None` for anything else (too long, or other fields).
    pub fn read(bytes: &[u8]) -> Option<ReadyNote> {
        if bytes.len() > 4096 {
            return None;
        }
        serde_json::from_slice(bytes).ok()
    }

    /// This note and `more` together (two hand-overs before the person looked).
    pub fn and(self, more: ReadyNote) -> ReadyNote {
        ReadyNote {
            landed: self.landed.saturating_add(more.landed),
            kept: self.kept.saturating_add(more.kept),
            failed: self.failed.saturating_add(more.failed),
        }
    }
}

pub fn usual_places() -> BTreeMap<String, String> {
    let videos = if cfg!(target_os = "macos") {
        "Movies"
    } else {
        "Videos"
    };
    [
        ("desktop", "Desktop"),
        ("documents", "Documents"),
        ("downloads", "Downloads"),
        ("pictures", "Pictures"),
        ("music", "Music"),
        ("videos", videos),
        ("home", "From your old laptop"),
        ("public", "From your old laptop"),
        ("other", "From your old laptop"),
    ]
    .into_iter()
    .map(|(l, f)| (l.to_string(), f.to_string()))
    .collect()
}

/// The one approved place of a hand-over: the person's home.
const HOME: &str = "home";

/// How one held file ended.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct HandedOver {
    /// Its path in the holding area, below its place's folder.
    pub held: PathBuf,
    pub outcome: Option<ReceiveOutcome>,
}

/// What the hand-over came to, and what in the holding area was not a plain file (left as it
/// was, never followed).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct HandOver {
    pub files: Vec<HandedOver>,
    pub not_plain: Vec<PathBuf>,
}

/// The two ends of the hand-over, in this process.
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

fn name_of(name: &std::ffi::OsStr) -> ItemName {
    #[cfg(unix)]
    {
        use std::os::unix::ffi::OsStrExt;
        ItemName::from_unix_bytes(name.as_bytes())
    }
    #[cfg(windows)]
    {
        use std::os::windows::ffi::OsStrExt;
        ItemName::from_windows_wide(&name.encode_wide().collect::<Vec<u16>>())
    }
}

/// Every plain file under `dir`, with its path parts below it; anything else found is listed
/// in `not_plain` and never entered.
fn walk(
    dir: &Path,
    parts: &mut Vec<ItemName>,
    files: &mut Vec<(PathBuf, Vec<ItemName>, u64)>,
    not_plain: &mut Vec<PathBuf>,
) -> std::io::Result<()> {
    for entry in std::fs::read_dir(dir)? {
        let entry = entry?;
        let path = entry.path();
        let meta = std::fs::symlink_metadata(&path)?;
        parts.push(name_of(&entry.file_name()));
        if meta.file_type().is_symlink() {
            not_plain.push(path);
        } else if meta.is_dir() {
            walk(&path, parts, files, not_plain)?;
        } else if meta.is_file() {
            files.push((path, parts.clone(), meta.len()));
        } else {
            not_plain.push(path);
        }
        parts.pop();
    }
    Ok(())
}

/// Hands over everything in `holding` into the person's `home`, each place label's files into
/// the folder of `home` that `places` names for it, as the signed-in `account`, recording every
/// write in `journal`.
pub async fn hand_over(
    holding: &Path,
    home: &Path,
    places: &BTreeMap<String, String>,
    journal: &dyn Ledger,
    account: &str,
) -> Result<HandOver, TransferError> {
    let laptop =
        LaptopId::from_hex(&"0".repeat(32)).map_err(|e| TransferError::Protocol(e.to_string()))?;
    let owner = Owner::Person {
        account_id: account.to_string(),
    };
    let place = Place {
        role: FolderRole::Other,
        storage: Storage::SystemDrive,
    };
    let mut table = Destinations::new();
    table
        .approve(HOME, Approved::MyFolders, home)
        .map_err(TransferError::Gate)?;
    let mut found = Vec::new();
    let mut not_plain = Vec::new();
    for (label, folder) in places {
        let held = holding.join(label);
        if !held.is_dir() {
            continue;
        }
        let mut parts = vec![ItemName::from_text(folder)];
        walk(&held, &mut parts, &mut found, &mut not_plain)?;
    }
    let mut record = Record::new(laptop);
    let mut sources: BTreeMap<ItemId, PathBuf> = BTreeMap::new();
    for (path, parts, size) in found {
        let Ok(item_path) = ItemPath::new(parts) else {
            not_plain.push(path);
            continue;
        };
        let id = ItemId::derive(&laptop, &owner, &place, &item_path);
        sources.insert(id, path);
        record.items.push(Item {
            id,
            kind: ItemKind::File,
            owner: owner.clone(),
            place: place.clone(),
            path: item_path,
            size_bytes: size,
            modified_ns: None,
            portability: Portability::Portable,
            managed_by: ManagedBy::Personal,
            download_bytes: None,
            inclusion: Inclusion::Included,
        });
    }
    // Each item's path starts with its place's folder in the home: the gate makes it if missing.
    let landing =
        |i: &Item| -> Option<LandingPlace> { Some(LandingPlace::new(HOME, shown_path(&i.path))) };
    // Every file is this person's, landing in their own account.
    record.mapping.people.insert(
        account.to_string(),
        PersonTarget::ExistingAccount {
            account_id: account.to_string(),
        },
    );
    let approval = record
        .approve()
        .map_err(|e| TransferError::Protocol(e.to_string()))?;
    let allowance = Allowance::from_record(&record, &approval, landing)
        .map_err(|e| TransferError::Protocol(e.to_string()))?;
    let jobs: Vec<SendJob> = record
        .items
        .iter()
        .filter_map(|i| {
            let place = landing(i)?;
            Some(SendJob {
                item: i.id,
                source: sources.get(&i.id)?.clone(),
                destination: place.destination,
                path: place.path,
                compressible: false,
                tier: Tier::Rest,
                size: i.size_bytes,
            })
        })
        .collect();
    let (a_tx, b_rx) = mpsc::unbounded_channel();
    let (b_tx, a_rx) = mpsc::unbounded_channel();
    let (mut a, mut b) = (Mem { tx: a_tx, rx: a_rx }, Mem { tx: b_tx, rx: b_rx });
    let mut sender = SenderSession::new(jobs, 4);
    let mut receiver = ReceiverSession::new(&table, allowance, journal, account);
    let (sent, received) = tokio::join!(sender.run(&mut a), receiver.run(&mut b));
    sent?;
    received?;
    let files = record
        .items
        .iter()
        .map(|i| HandedOver {
            held: sources[&i.id]
                .strip_prefix(holding)
                .unwrap_or(&sources[&i.id])
                .to_path_buf(),
            outcome: receiver.outcome(i.id),
        })
        .collect();
    Ok(HandOver { files, not_plain })
}

#[cfg(test)]
mod ready_tests {
    use super::*;

    #[test]
    fn a_note_is_only_what_pctwin_writes_and_two_add_up() {
        let one = ReadyNote {
            landed: 3,
            kept: 1,
            failed: 0,
        };
        let bytes = serde_json::to_vec(&one).unwrap();
        assert_eq!(ReadyNote::read(&bytes), Some(one));
        assert_eq!(
            one.and(one),
            ReadyNote {
                landed: 6,
                kept: 2,
                failed: 0
            }
        );
        assert_eq!(ReadyNote::read(br#"{"landed":1,"names":["x"]}"#), None);
        assert_eq!(ReadyNote::read(&vec![b' '; 5000]), None);
        assert!(
            ready_note_path(Path::new("/d")).ends_with("com.creoleailabs.pctwin/things-ready.json")
        );
    }
}
