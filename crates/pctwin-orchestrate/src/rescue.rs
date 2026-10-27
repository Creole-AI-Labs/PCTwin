//! Dead-laptop rescue, read only (Task List 2.6; acceptance T26-017 to T26-021).
//!
//! A laptop that will not start can still give up its files when its drive is connected to
//! another laptop (in a USB enclosure). PCTwin:
//!
//! - **knows the drive by its identity** ([`DriveIdentity`]: the volume's own ID, serial, file
//!   system and size, as the system reports them), never by its drive letter: a drive taken out
//!   and put back is checked again ([`reidentify`]); its rescue gets its own laptop ID, so its
//!   people are never mixed with anyone else's ([`crate::topology`]);
//! - **reads the drive only through the drive itself** ([`RescueDrive`]): the drive's top folder
//!   is opened once, and every folder and file below it is opened through its parent's handle,
//!   one name at a time, never through a link or any other redirection (a junction, a symbolic
//!   link, a mount point), and only if it is on the same volume as the top folder. A drive that
//!   holds a link to somewhere else (the new laptop's own folders, say) cannot lead the rescue
//!   there (third security review, finding 1);
//! - **finds the people and their folders from the drive itself** ([`people_on_drive`]): on a
//!   Windows drive each person is the account that owns their settings file (`NTUSER.DAT`), never
//!   their folder's name; a person whose account cannot be read is still listed, under their
//!   folder on this drive only, keyed by its exact name and which folder it is, so two people
//!   are never one; every folder at the top of each home is read, not only the usual six (apps'
//!   own data and any account state are counted, never moved), and what could not be read is
//!   listed;
//! - **only reads**: files are opened for reading and nothing is ever written to the drive (this
//!   crate cannot write files at all), and a drive chosen as both source and target is refused
//!   by the volume each target folder is really on, never by how its path is written
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
//! review, safety gate and transfer, read from the drive as a local source: each file is opened
//! with [`RescueScan::open`], through the drive's own handles, never by its path.
//!
//! What this does not prove: the scan lists each person's folder by its path once the folders
//! leading to it were opened and checked, and checks them again afterwards (a folder swapped in
//! between is left out). A drive that swaps a folder and swaps it back while it is listed could
//! put names from elsewhere into the plan, but never their contents: every file is read through
//! the drive's handles only.

use std::collections::BTreeMap;
use std::ffi::{OsStr, OsString};
use std::fs::File;
use std::io;
use std::path::{Component, Path, PathBuf};
use std::sync::Arc;

use cap_primitives::fs as cap;
use pctwin_record::{
    FolderRole, Inclusion, ItemId, ItemKind, ItemName, ItemPath, LaptopId, LeftOutReason,
    MAX_PERSONAL, Owner, Place, Record, Storage,
};

use crate::cloud::{Os, account_state_by_name};
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
    #[error("This is not the drive that was chosen, so it is not read. Choose the drive again.")]
    NotTheDrive,
    #[error(
        "PCTwin could not tell which drive this folder is on, so nothing is put there. Choose a folder on this laptop's own drive."
    )]
    TargetUnknown,
}

/// Refuses a rescue whose drive is also a place things would be written to, by the volume each
/// target folder is really on ([`volume_of`], following every link as a write would), never by
/// how its path is written: `D:\`, `\\?\D:\`, `\\.\D:\`, `\\localhost\D$\` and
/// `\\?\Volume{...}\` are one volume (third security review, finding 2). A target whose volume
/// cannot be told is refused too.
pub fn check_not_target(drive: &RescueDrive, targets: &[PathBuf]) -> Result<(), RescueError> {
    let volumes: Vec<Option<u64>> = targets.iter().map(|t| volume_of(t).ok()).collect();
    refuse_same_volume(drive.volume(), &volumes)
}

/// The rule [`check_not_target`] applies: every target's volume must be known, and none may be
/// the rescued drive's.
pub fn refuse_same_volume(source: u64, targets: &[Option<u64>]) -> Result<(), RescueError> {
    for t in targets {
        match t {
            None => return Err(RescueError::TargetUnknown),
            Some(v) if *v == source => return Err(RescueError::SourceIsTarget),
            Some(_) => {}
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

/// Which volume a file is on and which file it is there, read from its open handle (Windows: the
/// volume's serial number and the file's index; Linux and macOS: the device and inode numbers).
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct FileKey {
    pub volume: u64,
    pub file: u64,
}

/// Reads the [`FileKey`] of an open file.
pub fn file_key(f: &File) -> io::Result<FileKey> {
    #[cfg(windows)]
    {
        let info = winapi_util::file::information(f)?;
        Ok(FileKey {
            volume: info.volume_serial_number(),
            file: info.file_index(),
        })
    }
    #[cfg(unix)]
    {
        use std::os::unix::fs::MetadataExt;
        let m = f.metadata()?;
        Ok(FileKey {
            volume: m.dev(),
            file: m.ino(),
        })
    }
}

/// Whether what an open handle is on redirects somewhere else: on Windows any reparse point
/// (a junction, a symbolic link, a mount point, a file stored by a cloud app); elsewhere a link.
fn redirects(meta: &std::fs::Metadata) -> bool {
    #[cfg(windows)]
    {
        use std::os::windows::fs::MetadataExt;
        const FILE_ATTRIBUTE_REPARSE_POINT: u32 = 0x400;
        meta.file_attributes() & FILE_ATTRIBUTE_REPARSE_POINT != 0 || meta.file_type().is_symlink()
    }
    #[cfg(not(windows))]
    {
        meta.file_type().is_symlink()
    }
}

/// The volume serial number a drive's identity names, as Windows' volume list gives it (a whole
/// number) or as `vol` prints it (`1234-ABCD`).
pub fn serial_number(serial: &str) -> Option<u64> {
    let s = serial.trim();
    if let Ok(n) = s.parse::<u64>() {
        return Some(n);
    }
    let (hi, lo) = s.split_once('-')?;
    if hi.len() != 4 || lo.len() != 4 {
        return None;
    }
    let hi = u64::from_str_radix(hi, 16).ok()?;
    let lo = u64::from_str_radix(lo, 16).ok()?;
    Some((hi << 16) | lo)
}

/// A rescued drive, opened at its top folder. Everything below is opened through this handle, one
/// name at a time: never through a link or other redirection, and only on the drive's own
/// volume.
#[derive(Debug)]
pub struct RescueDrive {
    root: PathBuf,
    top: File,
    volume: u64,
}

/// Why something on the drive was not opened.
fn refused(why: &str) -> io::Error {
    io::Error::new(io::ErrorKind::PermissionDenied, why.to_owned())
}

impl RescueDrive {
    /// Opens the drive at `root`, which must be the drive `identity` names. On Windows the
    /// identity's serial number is required and must be the serial of the volume `root` is
    /// really on; a drive whose identity names no serial is refused. Linux and macOS give a
    /// volume's file system ID, which an open folder does not show; there everything below is
    /// checked against the volume of the folder opened here.
    pub fn open(root: &Path, identity: &DriveIdentity) -> Result<Self, RescueError> {
        let top =
            cap::open_ambient_dir(root, cap_primitives::ambient_authority()).map_err(|e| {
                match lock_from_error(&e) {
                    Some(lock) => RescueError::Locked(lock),
                    None => RescueError::Unreadable(e.to_string()),
                }
            })?;
        let key = file_key(&top).map_err(|e| RescueError::Unreadable(e.to_string()))?;
        if cfg!(windows) {
            let named = identity.serial.as_deref().and_then(serial_number);
            if named != Some(key.volume) {
                return Err(RescueError::NotTheDrive);
            }
        }
        Ok(Self {
            root: root.to_path_buf(),
            top,
            volume: key.volume,
        })
    }

    /// The drive's top folder, as it was named (for showing, and the paths of the plan).
    pub fn root(&self) -> &Path {
        &self.root
    }

    /// The volume the drive is on.
    pub fn volume(&self) -> u64 {
        self.volume
    }

    /// Checks an opened handle: no redirection, on this drive's volume.
    fn check(&self, f: &File) -> io::Result<FileKey> {
        if redirects(&f.metadata()?) {
            return Err(refused("it leads somewhere else (a link)"));
        }
        let key = file_key(f)?;
        if key.volume != self.volume {
            return Err(refused("it is on another drive"));
        }
        Ok(key)
    }

    /// One name: a single plain part, never `..`, `.`, a drive or a separator.
    fn one_name(name: &OsStr) -> io::Result<&Path> {
        let p = Path::new(name);
        let mut c = p.components();
        match (c.next(), c.next()) {
            (Some(Component::Normal(_)), None) => Ok(p),
            _ => Err(refused("not one name")),
        }
    }

    /// Opens the folders `names` one below the other from the top, each through its parent's
    /// handle. Returns the last folder and the key of every folder opened.
    fn folders(&self, names: &[&OsStr]) -> io::Result<(File, Vec<FileKey>)> {
        let mut dir = self.top.try_clone()?;
        let mut keys = Vec::with_capacity(names.len());
        for name in names {
            let next = cap::open_dir_nofollow(&dir, Self::one_name(name)?)?;
            keys.push(self.check(&next)?);
            dir = next;
        }
        Ok((dir, keys))
    }

    /// Opens the plain file `names` (its folders, then its own name) for reading.
    pub fn open_file(&self, names: &[&OsStr]) -> io::Result<File> {
        let (last, folders) = names.split_last().ok_or_else(|| refused("no name"))?;
        let (dir, _) = self.folders(folders)?;
        let mut options = cap::OpenOptions::new();
        options
            .read(true)
            ._cap_fs_ext_follow(cap::FollowSymlinks::No);
        let file = cap::open(&dir, Self::one_name(last)?, &options)?;
        self.check(&file)?;
        if !file.metadata()?.is_file() {
            return Err(refused("not a plain file"));
        }
        Ok(file)
    }

    /// The names of what is in the folder `names`, read through its handle.
    fn list(&self, names: &[&OsStr]) -> io::Result<Vec<OsString>> {
        let (dir, _) = self.folders(names)?;
        let mut out = Vec::new();
        for entry in cap::read_base_dir(&dir)? {
            out.push(entry?.file_name());
        }
        Ok(out)
    }

    /// The parts of `path` below the drive's top folder, or `None` for a path elsewhere.
    fn parts_of<'a>(&self, path: &'a Path) -> Option<Vec<&'a OsStr>> {
        let below = path.strip_prefix(&self.root).ok()?;
        below
            .components()
            .map(|c| match c {
                Component::Normal(n) => Some(n),
                _ => None,
            })
            .collect()
    }
}

/// Which volume `path` (a folder that will be written to) is really on: the nearest folder of it
/// that exists, opened following every link, as the system would when writing there.
pub fn volume_of(path: &Path) -> io::Result<u64> {
    let mut last = io::Error::new(io::ErrorKind::NotFound, "no part of the path exists");
    for p in path.ancestors() {
        if p.as_os_str().is_empty() {
            break;
        }
        let mut o = std::fs::OpenOptions::new();
        o.read(true);
        #[cfg(windows)]
        {
            use std::os::windows::fs::OpenOptionsExt;
            // Folders open only with backup semantics; no access to the contents is asked.
            const FILE_FLAG_BACKUP_SEMANTICS: u32 = 0x0200_0000;
            o.access_mode(0).custom_flags(FILE_FLAG_BACKUP_SEMANTICS);
        }
        match o.open(p) {
            Ok(f) => return file_key(&f).map(|k| k.volume),
            Err(e) => last = e,
        }
    }
    Err(last)
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
    /// Their folder's name, to show (never an identity; names that are not valid text show with
    /// a replacement mark).
    pub label: String,
    /// Their folder's name exactly as the drive holds it.
    pub name: OsString,
    pub home: PathBuf,
    /// Which folder it is on the drive.
    pub folder: FileKey,
    pub identity: PersonIdentity,
}

/// A name's exact bytes as text: hex of its UTF-16 units on Windows, of its bytes elsewhere.
fn exact_hex(name: &OsStr) -> String {
    let mut out = String::new();
    #[cfg(windows)]
    {
        use std::os::windows::ffi::OsStrExt;
        for unit in name.encode_wide() {
            out.push_str(&format!("{unit:04x}"));
        }
    }
    #[cfg(unix)]
    {
        use std::os::unix::ffi::OsStrExt;
        for b in name.as_bytes() {
            out.push_str(&format!("{b:02x}"));
        }
    }
    out
}

impl RescuedPerson {
    /// The account ID their things are recorded under on this drive's rescue: the account the
    /// drive records, or else their folder's exact name and which folder it is, so two people
    /// whose names look the same (or differ only in characters that are not valid text) are
    /// never one.
    pub fn account_id(&self) -> String {
        match &self.identity {
            PersonIdentity::Account(id) => id.clone(),
            PersonIdentity::FolderOnly => {
                format!("folder:{}:{:x}", exact_hex(&self.name), self.folder.file)
            }
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

fn homes_name(system: OtherSystem) -> &'static str {
    match system {
        OtherSystem::Windows | OtherSystem::MacOs => "Users",
        OtherSystem::Linux => "home",
    }
}

/// An account ID that names a person (a local, Microsoft or work account), not the system.
fn is_person_sid(sid: &str) -> bool {
    sid.starts_with("S-1-5-21-") || sid.starts_with("S-1-12-1-")
}

/// Everyone with a home folder on `drive`, in name order. A home folder that is a link, or is on
/// another volume, is not a person on this drive and is left out.
pub fn people_on_drive(
    drive: &RescueDrive,
    system: OtherSystem,
    reader: &mut dyn AccessReader,
) -> Vec<RescuedPerson> {
    let homes = OsStr::new(homes_name(system));
    let Ok(names) = drive.list(&[homes]) else {
        return Vec::new();
    };
    let mut people: Vec<RescuedPerson> = names
        .into_iter()
        .filter_map(|name| {
            let label = name.to_string_lossy().into_owned();
            if not_a_person(system, &label) {
                return None;
            }
            let (home_dir, keys) = drive.folders(&[homes, &name]).ok()?;
            let folder = *keys.last()?;
            let home = drive.root().join(homes).join(&name);
            let identity = match system {
                OtherSystem::Windows => {
                    let hive = OsStr::new("NTUSER.DAT");
                    // The settings file must be a plain file in this folder, on this drive,
                    // before its owner is read.
                    let plain = drive.open_file(&[homes, &name, hive]).is_ok();
                    match plain.then(|| reader.read(&home.join(hive), None)) {
                        Some(AccessFact::Known(a)) => a
                            .owner
                            .filter(|o| is_person_sid(o))
                            .map_or(PersonIdentity::FolderOnly, PersonIdentity::Account),
                        _ => PersonIdentity::FolderOnly,
                    }
                }
                OtherSystem::MacOs | OtherSystem::Linux => unix_owner(&home_dir),
            };
            Some(RescuedPerson {
                label,
                name,
                home,
                folder,
                identity,
            })
        })
        .collect();
    people.sort_by(|a, b| a.label.cmp(&b.label).then_with(|| a.name.cmp(&b.name)));
    people
}

#[cfg(unix)]
fn unix_owner(home: &File) -> PersonIdentity {
    use std::os::unix::fs::MetadataExt;
    match home.metadata() {
        Ok(m) if m.uid() != 0 => PersonIdentity::Account(m.uid().to_string()),
        _ => PersonIdentity::FolderOnly,
    }
}

#[cfg(not(unix))]
fn unix_owner(_home: &File) -> PersonIdentity {
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

/// The shared folder of a drive of `system`, when it has one (below the top folder).
fn shared_folder(system: OtherSystem) -> Option<[&'static str; 2]> {
    match system {
        OtherSystem::Windows => Some(["Users", "Public"]),
        OtherSystem::MacOs => Some(["Users", "Shared"]),
        OtherSystem::Linux => None,
    }
}

/// One folder to read on the drive: whose, under which role, its names from the top, and, for a
/// folder at the top of a home that is not one of the usual six, its own name (its things are
/// recorded below the home under that name).
struct Place_ {
    owner: Owner,
    role: FolderRole,
    names: Vec<OsString>,
    below_home: Option<OsString>,
}

/// A folder at the top of a home that holds apps' own data: read and counted, never moved.
fn is_app_data(system: OtherSystem, top: &str) -> bool {
    top.starts_with('.')
        || match system {
            OtherSystem::Windows => top.eq_ignore_ascii_case("AppData"),
            OtherSystem::MacOs => top.eq_ignore_ascii_case("Library"),
            OtherSystem::Linux => false,
        }
}

/// A name from the drive as the plan records it.
fn name_of(name: &OsStr) -> ItemName {
    #[cfg(windows)]
    {
        use std::os::windows::ffi::OsStrExt;
        let wide: Vec<u16> = name.encode_wide().collect();
        ItemName::from_windows_wide(&wide)
    }
    #[cfg(unix)]
    {
        use std::os::unix::ffi::OsStrExt;
        ItemName::from_unix_bytes(name.as_bytes())
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
    /// Folders that could not be read (another person's without permission, damaged areas, a
    /// link that leads off the drive).
    pub unreadable: Vec<PathBuf>,
    roots: BTreeMap<(String, FolderRole), PathBuf>,
    drive: Arc<RescueDrive>,
}

impl RescueScan {
    /// Where an item of the plan is on the drive: the name the move knows it by. It is read only
    /// through [`RescueScan::open`], never by this path.
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

    /// Opens a file of the plan (named as [`RescueScan::source_of`] names it) for reading,
    /// through the drive's own handles: every folder on the way is opened through its parent,
    /// never through a link, and must be on the drive's volume, as must the file.
    pub fn open(&self, path: &Path) -> io::Result<File> {
        let parts = self
            .drive
            .parts_of(path)
            .ok_or_else(|| refused("it is not on the rescued drive"))?;
        self.drive.open_file(&parts)
    }

    /// The drive this scan read.
    pub fn drive(&self) -> &RescueDrive {
        &self.drive
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
/// file details are read. The drive is opened with [`RescueDrive::open`], so a folder that is
/// not the drive `identity` names is refused.
pub fn scan_rescued_drive(
    root: &Path,
    identity: &DriveIdentity,
    reader: &mut dyn AccessReader,
) -> Result<RescueScan, RescueError> {
    let drive = Arc::new(RescueDrive::open(root, identity)?);
    let system = pctwin_scan::other_system(root).ok_or(RescueError::NoSystemFound)?;
    let laptop = identity.laptop_id();
    let people = people_on_drive(&drive, system, reader);
    let mut record = Record::new(laptop);
    let mut unreadable = Vec::new();
    let mut roots = BTreeMap::new();
    let storage = Storage::OtherDrive {
        drive: "rescue".into(),
    };
    let homes = OsString::from(homes_name(system));
    // Every folder at the top of each person's home is read, not only the usual six: the usual
    // ones under their own role, every other one below the home under its own name (Phase 2
    // adoption, changes 7 and 8: PCTwin works from what it finds). What could not be read is
    // listed as unreadable.
    let mut places: Vec<Place_> = Vec::new();
    for p in &people {
        let owner = Owner::Person {
            account_id: p.account_id(),
        };
        let home = [homes.clone(), p.name.clone()];
        let names = match drive.list(&[homes.as_os_str(), p.name.as_os_str()]) {
            Ok(names) => names,
            Err(_) => {
                unreadable.push(p.home.clone());
                continue;
            }
        };
        let mut names = names;
        names.sort();
        for name in names {
            let own = own_folders(system)
                .iter()
                .find(|(n, _)| OsStr::new(n) == name.as_os_str())
                .map(|(_, role)| *role);
            let mut at = home.to_vec();
            at.push(name.clone());
            places.push(Place_ {
                owner: owner.clone(),
                role: own.unwrap_or(FolderRole::Home),
                names: at,
                below_home: own.is_none().then_some(name),
            });
        }
    }
    if let Some(shared) = shared_folder(system) {
        places.push(Place_ {
            owner: Owner::Shared,
            role: FolderRole::Public,
            names: shared.iter().map(OsString::from).collect(),
            below_home: None,
        });
    }
    let os = match system {
        OtherSystem::Windows => Os::Windows,
        OtherSystem::MacOs => Os::Mac,
        OtherSystem::Linux => Os::Linux,
    };
    // (category rank, place order, path) for the most-important-first order.
    let mut order: Vec<(u8, usize, ItemId)> = Vec::new();
    for (n, place_) in places.into_iter().enumerate() {
        let Place_ {
            owner,
            role,
            names,
            below_home,
        } = place_;
        let names: Vec<&OsStr> = names.iter().map(OsString::as_os_str).collect();
        let folder = names
            .iter()
            .fold(drive.root().to_path_buf(), |p, n| p.join(n));
        // Every folder on the way opened through the drive's handles first, and again after the
        // listing: a folder that is a link, is on another drive, or was swapped meanwhile is
        // left out, as unreadable.
        let before = match drive.folders(&names) {
            Ok((_, keys)) => keys,
            Err(e) if e.kind() == io::ErrorKind::NotFound => continue,
            // A plain file at the top of the home is not a folder to read.
            Err(_) if below_home.is_some() && drive.open_file(&names).is_ok() => continue,
            Err(_) => {
                unreadable.push(folder);
                continue;
            }
        };
        let place = Place {
            role,
            storage: storage.clone(),
        };
        let scan = pctwin_scan::scan_folder(&laptop, &owner, &place, &folder);
        if drive.folders(&names).map(|(_, k)| k).ok() != Some(before) {
            unreadable.push(folder);
            continue;
        }
        unreadable.extend(scan.unreadable);
        let home_parts: Vec<String> = names
            .iter()
            .skip(2)
            .map(|n| n.to_string_lossy().into_owned())
            .collect();
        for mut item in scan.items {
            if let Some(name) = &below_home {
                // Recorded below the home under the folder's own name, so two such folders'
                // things never share a name.
                let mut parts = vec![name_of(name)];
                parts.extend(item.path.parts().iter().cloned());
                let Ok(path) = ItemPath::new(parts) else {
                    unreadable.push(folder.clone());
                    continue;
                };
                item.path = path;
                item.id = ItemId::derive(&laptop, &owner, &place, &item.path);
            }
            // A sync app's or the system's own account state is never moved (third review,
            // finding 4), nor is an app's own data (counted only, for now).
            let shown: Vec<String> = item
                .path
                .parts()
                .iter()
                .map(|p| p.display().to_string())
                .collect();
            let mut under_home: Vec<&str> = home_parts.iter().map(String::as_str).collect();
            if below_home.is_none() {
                under_home.extend(shown.iter().map(String::as_str));
            } else {
                under_home = shown.iter().map(String::as_str).collect();
            }
            let apps_own = under_home
                .first()
                .is_some_and(|top| is_app_data(system, top));
            if (apps_own || account_state_by_name(os, Some(&under_home), &under_home).is_some())
                && item.inclusion == Inclusion::Included
            {
                item.inclusion = Inclusion::LeftOut {
                    reason: LeftOutReason::System,
                };
            }
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
        let root = if below_home.is_some() {
            folder.parent().map(Path::to_path_buf).unwrap_or(folder)
        } else {
            folder
        };
        roots.insert((owner_key(&owner), role), root);
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
        drive,
    })
}
