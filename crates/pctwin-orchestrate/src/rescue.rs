//! Dead-laptop rescue, read only (Task List 2.6; acceptance T26-017 to T26-021).
//!
//! A laptop that will not start can still give up its files when its drive is connected to
//! another laptop (in a USB enclosure). PCTwin:
//!
//! - **knows the drive by its identity** ([`DriveIdentity`]: the volume's own ID, serial, file
//!   system and size, as the system reports them), never by its drive letter: a drive taken out
//!   and put back is checked again ([`reidentify`]); its rescue gets its own laptop ID, so its
//!   people are never mixed with anyone else's ([`crate::topology`]);
//! - **finds the people and their folders from the drive itself** ([`people_on_drive`]): on a
//!   Windows drive each person is the account that owns their settings file (`NTUSER.DAT`), never
//!   their folder's name; a person whose account cannot be read is still listed, under their
//!   folder on this drive only;
//! - **only reads**: files are opened for reading and nothing is ever written to the drive (this
//!   crate cannot write files at all), and a drive chosen as both source and target is refused
//!   ([`check_not_target`]). Only a drive the system mounted read-only is called untouched
//!   ([`ReadOnlyLevel`]): a drive mounted for writing can still be changed by the system itself;
//! - **locked drives**: a BitLocker drive is unlocked with Windows' own window and its recovery
//!   key; PCTwin never asks for the key, so it can never log or keep it ([`Lock`]); a Mac with
//!   Apple silicon or a T2 chip cannot be rescued by taking its drive out, and PCTwin says so;
//! - **a weak drive**: read with the careful plan: the most important first (documents, then
//!   photos, videos, music), each file once, a clean stop after repeated read errors; what could
//!   not be read is listed by the move as unreadable, never hidden.
//!
//! The rescued items are an ordinary plan ([`RescueScan::record`]) that goes through the normal
//! review, safety gate and transfer, read from the drive as a local source
//! ([`RescueScan::source_of`]).

use std::collections::BTreeMap;
use std::io;
use std::path::{Path, PathBuf};

use pctwin_record::{
    FolderRole, Inclusion, ItemId, ItemKind, LaptopId, MAX_PERSONAL, Owner, Place, Record, Storage,
};
use pctwin_scan::{AccessFact, AccessReader, Category, OtherSystem, ReadPlan, category};

/// A drive, as the system identifies it. No drive letter or mount point is part of it.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DriveIdentity {
    /// The volume's own ID (Windows: `\\?\Volume{...}\`; Linux and macOS: the file system UUID).
    pub volume_id: String,
    /// The file system's serial number, when the system gives one.
    pub serial: Option<String>,
    pub file_system: String,
    pub total_bytes: u64,
}

impl DriveIdentity {
    /// The laptop ID its rescue is recorded under: the same for the same drive, every time.
    pub fn laptop_id(&self) -> LaptopId {
        let mut h = blake3::Hasher::new();
        h.update(b"pctwin/v1/rescue-drive\0");
        for part in [
            self.volume_id.as_str(),
            self.serial.as_deref().unwrap_or("\u{0}none"),
            self.file_system.as_str(),
        ] {
            h.update(&(part.len() as u64).to_le_bytes());
            h.update(part.as_bytes());
        }
        h.update(&self.total_bytes.to_le_bytes());
        let hex = h.finalize().to_hex();
        LaptopId::from_hex(&hex[..32]).expect("32 hex digits are a laptop ID")
    }
}

/// Whether a drive put back is the one that was taken out.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Reinserted {
    Same,
    /// Another drive (perhaps in the same drive letter): its rescue starts again.
    Different,
}

pub fn reidentify(saved: &DriveIdentity, now: &DriveIdentity) -> Reinserted {
    if saved == now {
        Reinserted::Same
    } else {
        Reinserted::Different
    }
}

/// Reads `DeviceID <tab> SerialNumber <tab> FileSystem <tab> Capacity` (Windows' Win32_Volume).
pub fn identity_from_win32_volume(text: &str) -> Option<DriveIdentity> {
    let line = text.lines().next()?.trim_end_matches('\r');
    let f: Vec<&str> = line.split('\t').map(str::trim).collect();
    if f.len() < 4 || f[0].is_empty() {
        return None;
    }
    Some(DriveIdentity {
        volume_id: f[0].to_string(),
        serial: (!f[1].is_empty()).then(|| f[1].to_string()),
        file_system: f[2].to_string(),
        total_bytes: f[3].parse().ok()?,
    })
}

/// Reads `UUID FSTYPE SIZE` (Linux's `findmnt -n -b -o UUID,FSTYPE,SIZE`).
pub fn identity_from_findmnt(text: &str) -> Option<DriveIdentity> {
    let line = text.lines().next()?;
    // A drive with no UUID leaves that column blank: two fields, refused below.
    let f: Vec<&str> = line.split_whitespace().collect();
    if f.len() < 3 {
        return None;
    }
    Some(DriveIdentity {
        volume_id: f[0].to_string(),
        serial: None,
        file_system: f[1].to_string(),
        total_bytes: f[2].parse().ok()?,
    })
}

/// Why a drive cannot be rescued, or not as asked.
#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
pub enum RescueError {
    #[error(
        "This drive is the one being rescued, so nothing can be put on it. Choose a folder on another drive."
    )]
    SourceIsTarget,
    #[error("{}", .0.words())]
    Locked(Lock),
    #[error(
        "This drive does not hold a laptop's system and home folders, so it is not read as a rescue drive. If it is an ordinary USB drive, plug it into the new laptop instead."
    )]
    NoSystemFound,
    #[error("This drive could not be read: {0}")]
    Unreadable(String),
}

/// Refuses a rescue whose drive is also a place things would be written to: by identity when
/// the target's drive is known, and by place (a target inside the drive's mount) always.
pub fn check_not_target(
    source: &DriveIdentity,
    source_mount: &Path,
    targets: &[(PathBuf, Option<DriveIdentity>)],
) -> Result<(), RescueError> {
    let inside = |p: &Path| {
        if cfg!(windows) {
            let low = |x: &Path| x.to_string_lossy().to_lowercase().replace('/', "\\");
            low(p).starts_with(&low(source_mount))
        } else {
            p.starts_with(source_mount)
        }
    };
    for (path, id) in targets {
        if id.as_ref() == Some(source) || inside(path) {
            return Err(RescueError::SourceIsTarget);
        }
    }
    Ok(())
}

/// A drive that is locked.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Lock {
    BitLocker,
    FileVault,
    /// A Mac with Apple silicon or a T2 chip: the drive's keys never leave the Mac.
    AppleChip,
}

impl Lock {
    pub fn words(&self) -> &'static str {
        match self {
            Lock::BitLocker => {
                "This drive is locked with BitLocker. Unlock it in Windows first: open it in File Explorer and type its recovery key when Windows asks. The key is often saved in the owner's Microsoft account, at aka.ms/myrecoverykey. PCTwin never asks for the key and never keeps it."
            }
            Lock::FileVault => {
                "This Mac drive is locked with FileVault. Unlock it on a Mac with the owner's login password or recovery key first. PCTwin never asks for either and never keeps them."
            }
            Lock::AppleChip => {
                "A Mac with Apple silicon or a T2 chip keeps its drive locked to that Mac, so taking the drive out does not work. If the Mac still starts in Recovery, use Share Disk with a cable. If it does not start at all, only an Apple repair centre can help."
            }
        }
    }
}

/// BitLocker's "this volume is locked" (FVE_E_LOCKED_VOLUME).
const FVE_E_LOCKED_VOLUME: u32 = 0x8031_0000;

/// The lock an error from reading a drive means, if any.
pub fn lock_from_error(e: &io::Error) -> Option<Lock> {
    (e.raw_os_error() == Some(FVE_E_LOCKED_VOLUME as i32)).then_some(Lock::BitLocker)
}

/// How sure PCTwin can be that the drive does not change while it is read.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ReadOnlyLevel {
    /// The system mounted it read-only.
    MountedReadOnly,
    /// PCTwin only reads, but the system has it mounted for writing.
    ReadByPctwinOnly,
}

impl ReadOnlyLevel {
    pub fn untouched(&self) -> bool {
        *self == ReadOnlyLevel::MountedReadOnly
    }

    pub fn words(&self) -> &'static str {
        match self {
            ReadOnlyLevel::MountedReadOnly => {
                "The drive is connected read-only, so nothing on it can change."
            }
            ReadOnlyLevel::ReadByPctwinOnly => {
                "PCTwin only reads this drive. The system itself may still note small details on it, such as when a folder was last opened. For a drive that must not change at all, connect it read-only first."
            }
        }
    }
}

/// Whether the drive holding `path` is mounted read-only, as the system reports it. Not known
/// counts as mounted for writing.
pub fn read_only_level(path: &Path) -> ReadOnlyLevel {
    let disks = sysinfo::Disks::new_with_refreshed_list();
    let disk = disks
        .list()
        .iter()
        .filter(|d| path.starts_with(d.mount_point()))
        .max_by_key(|d| d.mount_point().as_os_str().len());
    match disk {
        Some(d) if d.is_read_only() => ReadOnlyLevel::MountedReadOnly,
        _ => ReadOnlyLevel::ReadByPctwinOnly,
    }
}

/// How a rescued person is known.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum PersonIdentity {
    /// By the account ID the drive records (a Windows SID, a Unix user number).
    Account(String),
    /// Their account could not be read: known only by their folder on this drive.
    FolderOnly,
}

/// One person found on a rescued drive.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RescuedPerson {
    /// Their folder's name, to show (never an identity).
    pub label: String,
    pub home: PathBuf,
    pub identity: PersonIdentity,
}

impl RescuedPerson {
    /// The account ID their things are recorded under on this drive's rescue.
    pub fn account_id(&self) -> String {
        match &self.identity {
            PersonIdentity::Account(id) => id.clone(),
            PersonIdentity::FolderOnly => format!("folder:{}", self.label),
        }
    }
}

/// Folders under the home folders that are not people.
fn not_a_person(system: OtherSystem, name: &str) -> bool {
    let n = name.to_lowercase();
    n.starts_with('.')
        || match system {
            OtherSystem::Windows => matches!(
                n.as_str(),
                "public" | "default" | "default user" | "all users" | "defaultapppool"
            ),
            OtherSystem::MacOs => matches!(n.as_str(), "shared" | "guest"),
            OtherSystem::Linux => matches!(n.as_str(), "lost+found"),
        }
}

fn homes_folder(root: &Path, system: OtherSystem) -> PathBuf {
    match system {
        OtherSystem::Windows | OtherSystem::MacOs => root.join("Users"),
        OtherSystem::Linux => root.join("home"),
    }
}

/// An account ID that names a person (a local, Microsoft or work account), not the system.
fn is_person_sid(sid: &str) -> bool {
    sid.starts_with("S-1-5-21-") || sid.starts_with("S-1-12-1-")
}

/// Everyone with a home folder on the drive at `root`, in name order.
pub fn people_on_drive(
    root: &Path,
    system: OtherSystem,
    reader: &mut dyn AccessReader,
) -> Vec<RescuedPerson> {
    let Ok(entries) = std::fs::read_dir(homes_folder(root, system)) else {
        return Vec::new();
    };
    let mut people: Vec<RescuedPerson> = entries
        .flatten()
        .filter(|e| e.file_type().is_ok_and(|t| t.is_dir()))
        .filter_map(|e| {
            let label = e.file_name().to_string_lossy().into_owned();
            if not_a_person(system, &label) {
                return None;
            }
            let home = e.path();
            let identity = match system {
                OtherSystem::Windows => {
                    let hive = home.join("NTUSER.DAT");
                    match hive.is_file().then(|| reader.read(&hive, None)) {
                        Some(AccessFact::Known(a)) => a
                            .owner
                            .filter(|o| is_person_sid(o))
                            .map_or(PersonIdentity::FolderOnly, PersonIdentity::Account),
                        _ => PersonIdentity::FolderOnly,
                    }
                }
                OtherSystem::MacOs | OtherSystem::Linux => unix_owner(&home),
            };
            Some(RescuedPerson {
                label,
                home,
                identity,
            })
        })
        .collect();
    people.sort_by(|a, b| a.label.cmp(&b.label));
    people
}

#[cfg(unix)]
fn unix_owner(home: &Path) -> PersonIdentity {
    use std::os::unix::fs::MetadataExt;
    match std::fs::symlink_metadata(home) {
        Ok(m) if m.uid() != 0 => PersonIdentity::Account(m.uid().to_string()),
        _ => PersonIdentity::FolderOnly,
    }
}

#[cfg(not(unix))]
fn unix_owner(_home: &Path) -> PersonIdentity {
    // A Mac or Linux drive read from Windows: its owners' numbers are not visible here.
    PersonIdentity::FolderOnly
}

/// Each person's own folders on a drive of `system`, by their names on disk (the names on disk
/// are the same in every language on Windows and macOS).
fn own_folders(system: OtherSystem) -> &'static [(&'static str, FolderRole)] {
    match system {
        OtherSystem::Windows => &[
            ("Desktop", FolderRole::Desktop),
            ("Documents", FolderRole::Documents),
            ("Downloads", FolderRole::Downloads),
            ("Pictures", FolderRole::Pictures),
            ("Music", FolderRole::Music),
            ("Videos", FolderRole::Videos),
        ],
        OtherSystem::MacOs => &[
            ("Desktop", FolderRole::Desktop),
            ("Documents", FolderRole::Documents),
            ("Downloads", FolderRole::Downloads),
            ("Pictures", FolderRole::Pictures),
            ("Music", FolderRole::Music),
            ("Movies", FolderRole::Videos),
        ],
        OtherSystem::Linux => &[
            ("Desktop", FolderRole::Desktop),
            ("Documents", FolderRole::Documents),
            ("Downloads", FolderRole::Downloads),
            ("Pictures", FolderRole::Pictures),
            ("Music", FolderRole::Music),
            ("Videos", FolderRole::Videos),
        ],
    }
}

/// The shared folder of a drive of `system`, when it has one.
fn shared_folder(root: &Path, system: OtherSystem) -> Option<PathBuf> {
    match system {
        OtherSystem::Windows => Some(root.join("Users").join("Public")),
        OtherSystem::MacOs => Some(root.join("Users").join("Shared")),
        OtherSystem::Linux => None,
    }
}

/// What a rescued drive gives: an ordinary plan, read carefully.
#[derive(Debug, Clone)]
pub struct RescueScan {
    pub system: OtherSystem,
    pub people: Vec<RescuedPerson>,
    /// The plan: every person's folders, recorded under the drive's own laptop ID, with the
    /// most important things first in its order.
    pub record: Record,
    /// Always careful: a dead laptop's drive is never trusted to be healthy.
    pub read_plan: ReadPlan,
    /// Folders that could not be read (another person's without permission, damaged areas).
    pub unreadable: Vec<PathBuf>,
    roots: BTreeMap<(String, FolderRole), PathBuf>,
}

impl RescueScan {
    /// Where an item of the plan is on the drive, to read it from.
    pub fn source_of(&self, item: &pctwin_record::Item) -> Option<PathBuf> {
        let mut path = self
            .roots
            .get(&(owner_key(&item.owner), item.place.role))?
            .clone();
        for part in item.path.parts() {
            path.push(part.on_disk());
        }
        Some(path)
    }
}

/// An owner as a key.
fn owner_key(owner: &Owner) -> String {
    match owner {
        Owner::Person { account_id } => format!("person:{account_id}"),
        Owner::Shared => "shared".into(),
        Owner::System => "system".into(),
    }
}

/// The careful plan every rescue reads with.
const RESCUE_READ: ReadPlan = ReadPlan::Careful {
    most_important_first: true,
    read_each_file_once: true,
    stop_after_read_errors: 5,
};

fn rank(c: Category) -> u8 {
    match c {
        Category::Document => 0,
        Category::Photo => 1,
        Category::Video => 2,
        Category::Music => 3,
        Category::Other => 4,
    }
}

/// Scans the drive at `root` (identified as `identity`) into a plan. Only folder listings and
/// file details are read.
pub fn scan_rescued_drive(
    root: &Path,
    identity: &DriveIdentity,
    reader: &mut dyn AccessReader,
) -> Result<RescueScan, RescueError> {
    if let Err(e) = std::fs::read_dir(root) {
        return Err(match lock_from_error(&e) {
            Some(lock) => RescueError::Locked(lock),
            None => RescueError::Unreadable(e.to_string()),
        });
    }
    let system = pctwin_scan::other_system(root).ok_or(RescueError::NoSystemFound)?;
    let laptop = identity.laptop_id();
    let people = people_on_drive(root, system, reader);
    let mut record = Record::new(laptop);
    let mut unreadable = Vec::new();
    let mut roots = BTreeMap::new();
    let storage = Storage::OtherDrive {
        drive: "rescue".into(),
    };
    let mut places: Vec<(Owner, FolderRole, PathBuf)> = Vec::new();
    for p in &people {
        let owner = Owner::Person {
            account_id: p.account_id(),
        };
        for (name, role) in own_folders(system) {
            places.push((owner.clone(), *role, p.home.join(name)));
        }
    }
    if let Some(shared) = shared_folder(root, system) {
        places.push((Owner::Shared, FolderRole::Public, shared));
    }
    // (category rank, place order, path) for the most-important-first order.
    let mut order: Vec<(u8, usize, ItemId)> = Vec::new();
    for (n, (owner, role, folder)) in places.into_iter().enumerate() {
        if !folder.is_dir() {
            continue;
        }
        let place = Place {
            role,
            storage: storage.clone(),
        };
        let scan = pctwin_scan::scan_folder(&laptop, &owner, &place, &folder);
        unreadable.extend(scan.unreadable);
        for item in scan.items {
            if item.kind == ItemKind::File && item.inclusion == Inclusion::Included {
                let name = item
                    .path
                    .parts()
                    .last()
                    .map(|p| p.display().to_string())
                    .unwrap_or_default();
                order.push((rank(category(&name)), n, item.id));
            }
            record.items.push(item);
        }
        roots.insert((owner_key(&owner), role), folder);
    }
    order.sort();
    record.personal = order
        .into_iter()
        .map(|(_, _, id)| id)
        .take(MAX_PERSONAL)
        .collect();
    record
        .validate()
        .map_err(|e| RescueError::Unreadable(e.to_string()))?;
    Ok(RescueScan {
        system,
        people,
        record,
        read_plan: RESCUE_READ,
        unreadable,
        roots,
    })
}
