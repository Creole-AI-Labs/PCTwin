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
use std::ffi::OsString;
use std::io;
use std::path::{Path, PathBuf};

use std::fs::File;

use cap_primitives::fs as cap;
use pctwin_journal::FileId;

use pctwin_gate::{Approved, Destinations};
use pctwin_journal::Ledger;
use pctwin_record::{
    FolderRole, Inclusion, Item, ItemId, ItemKind, ItemName, ItemPath, LaptopId, ManagedBy, Owner,
    PersonTarget, Place, Portability, Record, Storage,
};
use pctwin_transfer::{
    Allowance, Channel, ChannelError, LandingPlace, Opener, ReceiveOutcome, ReceiverSession,
    SendJob, SenderSession, Source, Tier, TransferError, shown_path,
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

/// Opens the plain file `name` in the folder `dir` for reading, through the folder's handle, never
/// through a link. `None` when what is there is not one plain
/// file with one name: a link, a folder, a device, or a file with another name elsewhere on the
/// drive (a hard link could stand for a file the mover may not read, so it is never handed over).
fn open_plain(dir: &File, name: &Path) -> io::Result<Option<(File, FileId)>> {
    let mut options = cap::OpenOptions::new();
    // Never through a link at the name (cap-fs-ext's `follow`, which cap-primitives provides
    // under this name).
    options
        .read(true)
        ._cap_fs_ext_follow(cap::FollowSymlinks::No);
    let file = cap::open(dir, name, &options)?;
    let meta = file.metadata()?;
    if !meta.is_file() || meta.file_type().is_symlink() {
        return Ok(None);
    }
    let (id, links) = pctwin_gate::file_identity_and_links(&file)?;
    Ok((links == 1).then_some((file, id)))
}

/// One plain file found in the holding area: where it is (the full path, for the record, and its
/// parts below the holding area, for opening it again through folder handles), which file it is,
/// its path parts in the person's home, and its size.
struct Found {
    path: PathBuf,
    below: Vec<OsString>,
    id: FileId,
    parts: Vec<ItemName>,
    size: u64,
}

/// Every plain file under `dir` (at `path`, `below` the holding area), with its path parts;
/// anything else found is listed in `not_plain` and never entered. Every folder is entered
/// through its parent's handle, never through a link.
fn walk(
    dir: &File,
    path: &Path,
    below: &mut Vec<OsString>,
    parts: &mut Vec<ItemName>,
    files: &mut Vec<Found>,
    not_plain: &mut Vec<PathBuf>,
) -> io::Result<()> {
    for entry in cap::read_base_dir(dir)? {
        let name = entry?.file_name();
        let at = path.join(&name);
        let meta = cap::stat(dir, Path::new(&name), cap::FollowSymlinks::No)?;
        parts.push(name_of(&name));
        below.push(name.clone());
        if meta.file_type().is_symlink() {
            not_plain.push(at);
        } else if meta.is_dir() {
            let sub = cap::open_dir_nofollow(dir, Path::new(&name))?;
            walk(&sub, &at, below, parts, files, not_plain)?;
        } else if meta.is_file() {
            match open_plain(dir, Path::new(&name))? {
                Some((file, id)) => files.push(Found {
                    path: at,
                    below: below.clone(),
                    id,
                    parts: parts.clone(),
                    size: file.metadata()?.len(),
                }),
                None => not_plain.push(at),
            }
        } else {
            not_plain.push(at);
        }
        below.pop();
        parts.pop();
    }
    Ok(())
}

/// Opens only the files the walk found, each again through the holding area's folder handles
/// (never following a link, never leaving the holding area), and only if it is still the very
/// file found (the same file on the same drive, with one name). On Windows the hand-over may run
/// with an administrator's rights while the mover could still change the holding area (a folder
/// made, or a handle opened, before it was locked): whatever is swapped in after the walk, a link
/// or another file, is refused here, never read. Whatever the mover can still put there is a
/// plain file of its own making, which is the mover's to hand over anyway.
struct HeldOpener {
    root: File,
    files: BTreeMap<PathBuf, (Vec<OsString>, FileId)>,
}

impl HeldOpener {
    fn open_found(&self, path: &Path) -> io::Result<std::fs::File> {
        let refused = |why: &str| io::Error::new(io::ErrorKind::PermissionDenied, why.to_string());
        let (below, id) = self
            .files
            .get(path)
            .ok_or_else(|| refused("not a file the hand-over found"))?;
        let (name, folders) = below
            .split_last()
            .ok_or_else(|| refused("not a file the hand-over found"))?;
        let mut dir = self.root.try_clone()?;
        for folder in folders {
            dir = cap::open_dir_nofollow(&dir, Path::new(folder))?;
        }
        match open_plain(&dir, Path::new(name))? {
            Some((file, now)) if now == *id => Ok(file),
            _ => Err(refused("it changed after the hand-over looked at it")),
        }
    }
}

impl Opener for HeldOpener {
    fn open(&self, path: &Path) -> io::Result<Box<dyn Source>> {
        Ok(Box::new(self.open_found(path)?))
    }
}

/// What the walk of the holding area found, and the opener that reads only that.
struct Held {
    found: Vec<Found>,
    not_plain: Vec<PathBuf>,
    opener: HeldOpener,
}

/// Walks the place folders of `holding` that `places` names, through folder handles.
fn held(holding: &Path, places: &BTreeMap<String, String>) -> io::Result<Held> {
    let root = cap::open_ambient_dir(holding, cap_primitives::ambient_authority())?;
    let mut found = Vec::new();
    let mut not_plain = Vec::new();
    for (label, folder) in places {
        let meta = match cap::stat(&root, Path::new(label), cap::FollowSymlinks::No) {
            Ok(m) => m,
            Err(e) if e.kind() == io::ErrorKind::NotFound => continue,
            Err(e) => return Err(e),
        };
        let at = holding.join(label);
        if meta.file_type().is_symlink() || !meta.is_dir() {
            not_plain.push(at);
            continue;
        }
        let dir = cap::open_dir_nofollow(&root, Path::new(label))?;
        let mut below = vec![OsString::from(label)];
        let mut parts = vec![ItemName::from_text(folder)];
        walk(
            &dir,
            &at,
            &mut below,
            &mut parts,
            &mut found,
            &mut not_plain,
        )?;
    }
    let files = found
        .iter()
        .map(|f| (f.path.clone(), (f.below.clone(), f.id)))
        .collect();
    Ok(Held {
        found,
        not_plain,
        opener: HeldOpener { root, files },
    })
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
    let Held {
        found,
        mut not_plain,
        opener,
    } = held(holding, places)?;
    let mut record = Record::new(laptop);
    let mut sources: BTreeMap<ItemId, PathBuf> = BTreeMap::new();
    for Found {
        path, parts, size, ..
    } in found
    {
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
    // Nothing plain to hand over (only links, or nothing at all): nothing moves, and the record
    // is not made, as a mapping for someone with no things is refused.
    if record.items.is_empty() {
        return Ok(HandOver {
            files: Vec::new(),
            not_plain,
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
    let mut sender = SenderSession::new(jobs, 4).with_opener(std::sync::Arc::new(opener));
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

#[cfg(test)]
// Tests make, move and link files to test on; the no-file-changes fence is for the shipped code.
#[allow(clippy::disallowed_methods)]
mod held_tests {
    use super::*;
    use std::io::Read;

    /// `link` made a link to the folder `target`: a junction on Windows (which needs no special
    /// rights), a symbolic link elsewhere.
    fn link_folder(target: &Path, link: &Path) {
        #[cfg(windows)]
        {
            let out = std::process::Command::new("cmd")
                .args(["/C", "mklink", "/J"])
                .arg(link)
                .arg(target)
                .output()
                .unwrap();
            assert!(out.status.success(), "{out:?}");
        }
        #[cfg(unix)]
        std::os::unix::fs::symlink(target, link).unwrap();
    }

    fn documents() -> BTreeMap<String, String> {
        BTreeMap::from([("documents".to_string(), "Documents".to_string())])
    }

    /// A held folder of one file, `documents/sub/a.txt`, and a folder outside it holding a
    /// secret file of the same name.
    fn setup() -> (tempfile::TempDir, PathBuf, PathBuf) {
        let top = tempfile::tempdir().unwrap();
        let holding = top.path().join("holding");
        let sub = holding.join("documents").join("sub");
        std::fs::create_dir_all(&sub).unwrap();
        std::fs::write(sub.join("a.txt"), b"mine").unwrap();
        let outside = top.path().join("outside");
        std::fs::create_dir_all(&outside).unwrap();
        std::fs::write(outside.join("a.txt"), b"secret").unwrap();
        (top, holding, outside)
    }

    #[test]
    fn a_found_file_opens_through_the_held_folders() {
        let (_top, holding, _outside) = setup();
        let held = held(&holding, &documents()).unwrap();
        assert_eq!(held.found.len(), 1);
        let mut text = String::new();
        held.opener
            .open_found(&held.found[0].path)
            .unwrap()
            .read_to_string(&mut text)
            .unwrap();
        assert_eq!(text, "mine");
    }

    /// The Windows hand-over now may run with an administrator's rights: a folder swapped for a
    /// link to somewhere else after the walk (by a mover who kept a way in) is never followed.
    #[test]
    fn a_link_swapped_in_after_the_walk_is_never_followed() {
        let (_top, holding, outside) = setup();
        let held = held(&holding, &documents()).unwrap();
        let sub = holding.join("documents").join("sub");
        std::fs::rename(&sub, holding.join("documents").join("sub-old")).unwrap();
        link_folder(&outside, &sub);
        // Through the link, the name now finds the secret.
        assert_eq!(std::fs::read(sub.join("a.txt")).unwrap(), b"secret");
        assert!(held.opener.open_found(&held.found[0].path).is_err());
        assert!(held.opener.open(&held.found[0].path).is_err());
        // And a place nobody found is never opened.
        assert!(held.opener.open(&outside.join("a.txt")).is_err());
    }

    /// The whole place folder swapped for a link after the walk: refused the same way.
    #[test]
    fn a_place_folder_swapped_for_a_link_is_never_followed() {
        let (_top, holding, outside) = setup();
        let held = held(&holding, &documents()).unwrap();
        let docs = holding.join("documents");
        std::fs::rename(&docs, holding.join("documents-old")).unwrap();
        std::fs::create_dir_all(outside.join("sub")).unwrap();
        std::fs::write(outside.join("sub").join("a.txt"), b"secret").unwrap();
        link_folder(&outside, &docs);
        assert!(held.opener.open_found(&held.found[0].path).is_err());
    }

    /// Another file put at the found file's name after the walk is not the file found.
    #[test]
    fn another_file_at_the_name_after_the_walk_is_refused() {
        let (_top, holding, outside) = setup();
        let held = held(&holding, &documents()).unwrap();
        let a = holding.join("documents").join("sub").join("a.txt");
        std::fs::rename(&a, holding.join("moved.txt")).unwrap();
        std::fs::copy(outside.join("a.txt"), &a).unwrap();
        assert!(held.opener.open_found(&held.found[0].path).is_err());
    }

    /// A hard link swapped in at the found name (a second name of a file elsewhere) is refused.
    #[test]
    fn a_hard_link_swapped_in_after_the_walk_is_refused() {
        let (_top, holding, outside) = setup();
        let held = held(&holding, &documents()).unwrap();
        let a = holding.join("documents").join("sub").join("a.txt");
        std::fs::remove_file(&a).unwrap();
        std::fs::hard_link(outside.join("a.txt"), &a).unwrap();
        assert!(held.opener.open_found(&held.found[0].path).is_err());
    }

    /// At the walk itself: links and files with a second name are listed, never entered or
    /// read; the place folder itself as a link is never entered.
    #[test]
    fn links_and_second_names_found_by_the_walk_are_left() {
        let (_top, holding, outside) = setup();
        let docs = holding.join("documents");
        link_folder(&outside, &docs.join("linked"));
        std::fs::hard_link(outside.join("a.txt"), docs.join("second-name.txt")).unwrap();
        let first = held(&holding, &documents()).unwrap();
        assert_eq!(first.found.len(), 1, "only sub/a.txt");
        assert!(
            first.found[0]
                .path
                .ends_with(Path::new("sub").join("a.txt"))
        );
        assert_eq!(first.not_plain.len(), 2, "{:?}", first.not_plain);

        let (_top2, holding2, outside2) = setup();
        let docs2 = holding2.join("documents");
        std::fs::rename(&docs2, holding2.join("documents-old")).unwrap();
        link_folder(&outside2, &docs2);
        let held2 = held(&holding2, &documents()).unwrap();
        assert!(held2.found.is_empty());
        assert_eq!(held2.not_plain, vec![docs2]);
    }
}
