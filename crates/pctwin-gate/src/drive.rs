//! What kind of drive an approved destination is on, read once from its open folder when it is
//! opened (security design notes, "Naming a finished file never replaces anything", decided 15
//! October 2026). The drive's type, never the laptop's system, decides which names it takes:
//! FAT, exFAT, NTFS, network and unknown drives get Windows' rules on every system; only known
//! POSIX drives (APFS, HFS+, ext4, XFS, btrfs, ZFS, F2FS, and Linux's in-memory file systems)
//! keep the system's own.
//!
//! The type is read from the folder handle itself: Linux's `fstatfs` magic number, macOS's
//! `f_fstypename`. Windows reads it from the volume list (`sysinfo`, already used by the scan, so
//! only safe calls of ours): each volume's mount folder is opened, and only a volume whose serial
//! number is the open folder's own counts; volumes that disagree make it unknown. Where the type
//! still cannot be read (a drive Windows does not list, a drive served by a program on Linux),
//! the drive is asked how it keeps a file's last-access time, on two probe files of PCTwin's own:
//! FAT keeps a date alone. Only an exact local midnight on both counts, so another program
//! reading the probes (which moves the time to now) never makes a drive FAT. A drive that is not
//! known to be FAT has no limit set; a file too big for a real FAT drive then fails safely when
//! its space is reserved.
//!
//! Each drive under a destination is judged on its own: a drive mounted inside an approved folder
//! gets its own rules, not the folder's.

use cap_std::fs::Dir;

use crate::{NameRules, Platform};

/// The largest file a FAT drive (FAT12, FAT16, FAT32) holds: 4 GiB less one byte.
pub const FAT_LARGEST_FILE: u64 = (1 << 32) - 1;

/// What the time-step test's probe file holds.
pub(crate) const TIME_PROBE: &[u8] = b"PCTwin drive test: file t. Safe to delete.
";

/// The last-access time the time test sets: 3 July 2001, 12:34:56 UTC. FAT keeps a file's
/// last-access time as a date alone (local midnight), exFAT and NTFS keep the time of day too (a
/// Windows exFAT drive keeps modified times in two-second steps like FAT, so only this tells them
/// apart there). No time zone puts local midnight at 56 seconds past a minute, so a FAT drive
/// never gives it back.
const TIME_PROBE_AT: std::time::Duration = std::time::Duration::from_secs(994_163_696);

/// Whether a last-access time set to `set` and read back as `got` shows FAT's date-only keeping:
/// exactly the local midnight that starts the day `set` falls on, in some time zone. That is a
/// whole quarter hour (every time zone is one) no later than `set` and less than a day before it.
/// Anything else (the time itself, exFAT's two-second step, or the time another program read the
/// file at) is not FAT.
pub(crate) fn date_only(set: std::time::SystemTime, got: std::time::SystemTime) -> bool {
    let Ok(before) = set.duration_since(got) else {
        return false;
    };
    let Ok(since) = got.duration_since(std::time::UNIX_EPOCH) else {
        return false;
    };
    before < std::time::Duration::from_secs(86_400)
        && since.subsec_nanos() == 0
        && since.as_secs() % 900 == 0
}

/// Whether two probes both read back a date alone, and the same one.
fn two_agree(
    set: std::time::SystemTime,
    a: Option<std::time::SystemTime>,
    b: Option<std::time::SystemTime>,
) -> bool {
    match (a, b) {
        (Some(a), Some(b)) => a == b && date_only(set, a),
        _ => false,
    }
}

/// Asks the drive under `dir` how it keeps time, on two probe files of PCTwin's own, each made
/// only if free and removed at once: `Some(Fat)` only if both kept the last-access time as the
/// same date alone; `None` otherwise (or if the test could not run: a folder PCTwin may not write
/// to, say; nothing is then assumed).
pub(crate) fn kind_by_time_step(dir: &Dir) -> Option<DriveKind> {
    let set = std::time::UNIX_EPOCH + TIME_PROBE_AT;
    let a = kept_access_time(dir, "t", set);
    let b = kept_access_time(dir, "u", set);
    two_agree(set, a, b).then_some(DriveKind::Fat)
}

/// The last-access time a probe file named for `which` keeps once it is set to `set`.
fn kept_access_time(
    dir: &Dir,
    which: &str,
    set: std::time::SystemTime,
) -> Option<std::time::SystemTime> {
    use std::hash::{BuildHasher, RandomState};
    use std::io::Write;
    let tag = RandomState::new().hash_one(std::process::id());
    let name = format!(".pctwin-probe-{tag:016x}-{which}");
    let mut options = cap_std::fs::OpenOptions::new();
    options.write(true).create_new(true);
    let kept = (|| {
        let mut f = dir.open_with(&name, &options).ok()?.into_std();
        f.write_all(TIME_PROBE).ok()?;
        f.sync_all().ok()?;
        f.set_times(std::fs::FileTimes::new().set_accessed(set))
            .ok()?;
        f.sync_all().ok()?;
        drop(f);
        dir.open(&name)
            .ok()?
            .into_std()
            .metadata()
            .ok()?
            .accessed()
            .ok()
    })();
    let _ = dir.remove_file(&name);
    kept
}

/// Which drive a folder is on, for as long as it stays mounted: the drive's number, and what
/// changes when that number is given to another drive (Linux and macOS hand a freed device number
/// to the next drive attached): its file system's type, and on Linux the mount's own number.
/// Read again every time a drive is looked up, so a drive put in another's place, even at the
/// same folder, is judged again rather than taking the old drive's answers.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub(crate) struct VolumeKey {
    number: u64,
    kind: u64,
    mount: u64,
}

/// The key of the drive the folder `dir` is on.
pub(crate) fn volume_key(dir: &Dir) -> std::io::Result<VolumeKey> {
    #[cfg(target_os = "linux")]
    {
        #[allow(
            clippy::useless_conversion,
            clippy::unnecessary_cast,
            clippy::cast_sign_loss,
            reason = "the device number's and the type's widths differ by target"
        )]
        {
            let folder = crate::folder_to_flush(dir)?;
            Ok(VolumeKey {
                number: rustix::fs::fstat(&folder)?.st_dev as u64,
                kind: (rustix::fs::fstatfs(&folder)?.f_type as u64) & 0xFFFF_FFFF,
                mount: mount_number(&folder),
            })
        }
    }
    #[cfg(target_os = "macos")]
    {
        use std::hash::{Hash, Hasher};
        #[allow(
            clippy::useless_conversion,
            clippy::unnecessary_cast,
            reason = "the device number's width differs by target"
        )]
        {
            let folder = crate::folder_to_flush(dir)?;
            let st = rustix::fs::fstatfs(&folder)?;
            let mut kind = std::collections::hash_map::DefaultHasher::new();
            st.f_fstypename
                .iter()
                .take_while(|c| **c != 0)
                .for_each(|c| c.hash(&mut kind));
            Ok(VolumeKey {
                number: rustix::fs::fstat(&folder)?.st_dev as u64,
                kind: kind.finish(),
                mount: 0,
            })
        }
    }
    #[cfg(windows)]
    {
        // A volume's serial number is set when it is formatted, so another drive has another.
        Ok(VolumeKey {
            number: winapi_util::file::information(dir.try_clone()?.into_std_file())?
                .volume_serial_number(),
            kind: 0,
            mount: 0,
        })
    }
    #[cfg(not(any(target_os = "linux", target_os = "macos", windows)))]
    {
        let _ = dir;
        Ok(VolumeKey {
            number: 0,
            kind: 0,
            mount: 0,
        })
    }
}

/// The number of the mount the open folder is on: the one Linux never gives again where it has
/// one (6.8 and later), else the one it may give again after an unmount; 0 if neither is known.
#[cfg(target_os = "linux")]
fn mount_number(folder: &std::fs::File) -> u64 {
    use rustix::fs::{AtFlags, StatxFlags, statx};
    // STATX_MNT_ID_UNIQUE, which rustix does not name yet.
    const UNIQUE: u32 = 0x4000;
    if let Ok(st) = statx(
        folder,
        "",
        AtFlags::EMPTY_PATH,
        StatxFlags::from_bits_retain(UNIQUE),
    ) && st.stx_mask & UNIQUE != 0
    {
        return st.stx_mnt_id;
    }
    match statx(folder, "", AtFlags::EMPTY_PATH, StatxFlags::MNT_ID) {
        Ok(st) if st.stx_mask & StatxFlags::MNT_ID.bits() != 0 => st.stx_mnt_id,
        _ => 0,
    }
}

/// The kind of file system an approved destination is on.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DriveKind {
    /// FAT12, FAT16 or FAT32 (Linux `vfat` and `msdos`, macOS `msdos`).
    Fat,
    ExFat,
    Ntfs,
    Apfs,
    HfsPlus,
    /// ext4, and ext2 and ext3, which share its number.
    Ext4,
    Xfs,
    Btrfs,
    Zfs,
    F2fs,
    /// A file system in memory (Linux `tmpfs`, `ramfs`): any name Linux allows.
    Memory,
    /// A folder on another computer (SMB, NFS, AFP, WebDAV): its server's rules are unknown.
    Network,
    /// Anything else, including a drive served by a program (FUSE): never assumed safe.
    Unknown,
}

impl DriveKind {
    /// The kind of file system Linux's `statfs` reports by its magic number (`f_type`).
    /// `vfat` and `msdos` share one number; a drive served through FUSE (exfat-fuse, ntfs-3g)
    /// reports FUSE's own number, so what it really is cannot be told: unknown.
    pub fn from_linux_magic(magic: u64) -> Self {
        match magic {
            0x4d44 => Self::Fat,
            0x2011_BAB0 => Self::ExFat,
            // ntfs3, and the old read-only ntfs driver.
            0x7366_746e | 0x5346_544e => Self::Ntfs,
            0x482b => Self::HfsPlus,
            0xEF53 => Self::Ext4,
            0x5846_5342 => Self::Xfs,
            0x9123_683E => Self::Btrfs,
            0x2FC1_2FC1 => Self::Zfs,
            0xF2F5_2010 => Self::F2fs,
            // tmpfs, ramfs.
            0x0102_1994 | 0x8584_58f6 => Self::Memory,
            // NFS, SMB, CIFS, SMB2.
            0x6969 | 0x517B | 0xFF53_4D42 | 0xFE53_4D42 => Self::Network,
            _ => Self::Unknown,
        }
    }

    /// The kind of file system Windows names for a volume (`GetVolumeInformationW`), matched
    /// exactly: anything else is unknown.
    pub fn from_windows_name(name: &str) -> Self {
        match name.to_ascii_uppercase().as_str() {
            "FAT" | "FAT12" | "FAT16" | "FAT32" => Self::Fat,
            "EXFAT" => Self::ExFat,
            "NTFS" => Self::Ntfs,
            _ => Self::Unknown,
        }
    }

    /// The kind of file system macOS's `statfs` names (`f_fstypename`), matched loosely: since
    /// macOS 14 FAT and exFAT drives are served from user space, and a later system may name
    /// them differently, so a name that contains `msdos` or `exfat` counts.
    pub fn from_macos_type(name: &str) -> Self {
        let name = name.to_ascii_lowercase();
        let has = |part: &str| name.contains(part);
        if has("exfat") {
            Self::ExFat
        } else if has("msdos") || name == "fat" || name == "vfat" || has("fat32") {
            Self::Fat
        } else if has("ntfs") {
            Self::Ntfs
        } else if has("apfs") {
            Self::Apfs
        } else if name == "hfs" || has("hfsplus") || has("hfs+") {
            Self::HfsPlus
        } else if name == "zfs" {
            Self::Zfs
        } else if ["smbfs", "nfs", "afpfs", "webdav", "cifs", "ftp"]
            .iter()
            .any(|n| name == *n)
        {
            Self::Network
        } else {
            Self::Unknown
        }
    }

    /// Whether this file system keeps any name its system allows (a known POSIX file system).
    fn keeps_posix_names(self) -> bool {
        matches!(
            self,
            Self::Apfs
                | Self::HfsPlus
                | Self::Ext4
                | Self::Xfs
                | Self::Btrfs
                | Self::Zfs
                | Self::F2fs
                | Self::Memory
        )
    }

    /// Whether this file system tells `a.jpg` and `A.jpg` apart. APFS and HFS+ usually do not;
    /// anything unknown is taken not to.
    fn tells_case_apart(self) -> bool {
        matches!(
            self,
            Self::Ext4 | Self::Xfs | Self::Btrfs | Self::Zfs | Self::F2fs | Self::Memory
        )
    }
}

/// What PCTwin knows about the drive an approved destination is on, worked out when the
/// destination opens: its kind, the name rules it gets, and the largest file it holds.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct DriveProfile {
    kind: DriveKind,
    rules: NameRules,
    case_apart: bool,
    /// Whether to try keeping download markers (the drive's own answer still decides).
    markers: bool,
}

impl DriveProfile {
    /// The profile of a drive of `kind` on this system.
    pub fn of(kind: DriveKind) -> Self {
        Self::on(kind, Platform::host())
    }

    /// The profile of a drive of `kind` on `system`. Windows refuses its forbidden names on
    /// every drive, so there the rules are always Windows'.
    pub fn on(kind: DriveKind, system: Platform) -> Self {
        let posix = system != Platform::Windows;
        Self {
            kind,
            rules: if posix && kind.keeps_posix_names() {
                NameRules::Posix
            } else {
                NameRules::Windows
            },
            case_apart: posix && kind.tells_case_apart(),
            markers: true,
        }
    }

    /// The drive's kind.
    pub fn kind(&self) -> DriveKind {
        self.kind
    }

    /// The name rules every file and folder name written to this drive follows.
    pub fn name_rules(&self) -> NameRules {
        self.rules
    }

    /// The largest single file this drive holds, if it has a limit below what any system
    /// stores ([`FAT_LARGEST_FILE`] on FAT), so the plan can refuse a file before the move.
    pub fn largest_file(&self) -> Option<u64> {
        (self.kind == DriveKind::Fat).then_some(FAT_LARGEST_FILE)
    }

    /// For tests: a drive that cannot keep download markers (FAT and exFAT on Windows, some
    /// network drives), which no test machine has. Files still land; the finished file says
    /// which marker was lost.
    #[doc(hidden)]
    #[must_use]
    pub fn without_markers(mut self) -> Self {
        self.markers = false;
        self
    }

    pub(crate) fn tries_markers(&self) -> bool {
        self.markers
    }

    /// Whether names that differ only in capital letters are different names here.
    pub(crate) fn tells_case_apart(&self) -> bool {
        self.case_apart
    }

    /// The profile of the drive the open folder `dir` is on. A drive whose type cannot be read
    /// is asked how it keeps time (FAT is found that way); otherwise it is unknown, and gets the
    /// strictest rules.
    pub(crate) fn read(dir: &Dir) -> Self {
        let kind = match read_kind(dir).unwrap_or(DriveKind::Unknown) {
            DriveKind::Unknown => kind_by_time_step(dir).unwrap_or(DriveKind::Unknown),
            kind => kind,
        };
        Self::of(kind)
    }
}

#[cfg(target_os = "linux")]
fn read_kind(dir: &Dir) -> std::io::Result<DriveKind> {
    let folder = crate::folder_to_flush(dir)?;
    let st = rustix::fs::fstatfs(&folder)?;
    #[allow(
        clippy::cast_sign_loss,
        clippy::useless_conversion,
        clippy::unnecessary_cast,
        reason = "the drive types are 32-bit numbers in a field whose width differs by target"
    )]
    let magic = (st.f_type as u64) & 0xFFFF_FFFF;
    Ok(DriveKind::from_linux_magic(magic))
}

#[cfg(target_os = "macos")]
fn read_kind(dir: &Dir) -> std::io::Result<DriveKind> {
    let folder = crate::folder_to_flush(dir)?;
    let st = rustix::fs::fstatfs(&folder)?;
    let name: Vec<u8> = st
        .f_fstypename
        .iter()
        .take_while(|c| **c != 0)
        .map(|c| c.to_ne_bytes()[0])
        .collect();
    Ok(DriveKind::from_macos_type(&String::from_utf8_lossy(&name)))
}

/// Windows: the volume list names each volume's file system. Only a volume whose mount folder
/// opens onto the same volume serial number as the open folder counts; none, or volumes that
/// disagree, leave it unknown.
#[cfg(windows)]
fn read_kind(dir: &Dir) -> std::io::Result<DriveKind> {
    let serial = volume_key(dir)?.number;
    let disks =
        sysinfo::Disks::new_with_refreshed_list_specifics(sysinfo::DiskRefreshKind::nothing());
    let volumes: Vec<(u64, String)> = disks
        .list()
        .iter()
        .filter_map(|disk| {
            let mount = winapi_util::Handle::from_path_any(disk.mount_point()).ok()?;
            let info = winapi_util::file::information(&mount).ok()?;
            Some((
                info.volume_serial_number(),
                disk.file_system().to_string_lossy().into_owned(),
            ))
        })
        .collect();
    Ok(kind_of_volume(serial, &volumes))
}

/// The kind of the volume numbered `serial`, from the volumes listed with their numbers and file
/// system names: only those with that number count, and if they disagree it is unknown.
#[cfg(any(windows, test))]
pub(crate) fn kind_of_volume(serial: u64, volumes: &[(u64, String)]) -> DriveKind {
    let mut found = None;
    for (number, name) in volumes {
        if *number != serial {
            continue;
        }
        let kind = DriveKind::from_windows_name(name);
        match found {
            None => found = Some(kind),
            Some(seen) if seen == kind => {}
            Some(_) => return DriveKind::Unknown,
        }
    }
    found.unwrap_or(DriveKind::Unknown)
}

#[cfg(not(any(target_os = "linux", target_os = "macos", windows)))]
fn read_kind(_dir: &Dir) -> std::io::Result<DriveKind> {
    Ok(DriveKind::Unknown)
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::time::{Duration, UNIX_EPOCH};

    fn at(secs: i64) -> std::time::SystemTime {
        UNIX_EPOCH + Duration::from_secs(u64::try_from(secs).unwrap())
    }

    #[test]
    fn a_date_only_access_time_is_exactly_the_local_midnight_before_the_time_set() {
        let set = UNIX_EPOCH + TIME_PROBE_AT;
        let set_secs = 994_163_696i64;
        // FAT: the local midnight that starts the day the time set falls on, in every time zone
        // (offsets are whole quarter hours, from 12 hours behind to 14 ahead, an hour more
        // either way for summer time).
        let mut zones = 0;
        for quarter_hours in -13 * 4..=15 * 4i64 {
            let offset = quarter_hours * 900;
            let local = set_secs + offset;
            let midnight = local - local.rem_euclid(86_400) - offset;
            assert!(date_only(set, at(midnight)), "{quarter_hours}: {midnight}");
            zones += 1;
        }
        assert_eq!(zones, 113);
        // exFAT keeps it to two seconds; NTFS, APFS and ext4 exactly.
        assert!(!date_only(set, set));
        assert!(!date_only(set, set - Duration::from_secs(1)));
        assert!(!date_only(set, set + Duration::from_millis(1500)));
        // Another program read it: the time is now, or any time but that midnight.
        assert!(!date_only(set, std::time::SystemTime::now()));
        assert!(!date_only(set, at(1_792_108_800))); // a midnight in 2026
        assert!(!date_only(set, at(994_118_400 + 86_400))); // the next UTC midnight
        assert!(!date_only(set, at(994_118_400 - 2 * 86_400))); // two days before
        assert!(!date_only(set, at(994_118_400 - 16 * 3600))); // before any time zone's midnight
        assert!(!date_only(set, at(994_118_400 + 7 * 60))); // not a quarter hour
        assert!(!date_only(set, at(994_118_400) + Duration::from_micros(1)));
        assert!(!date_only(set, at(994_163_700))); // a quarter hour after the time set
    }

    /// Only the volume with the folder's own number counts; none, or two that disagree (two
    /// volumes can share a number, a copied drive say), leave it unknown, never FAT.
    #[test]
    fn the_volume_kind_is_the_one_with_the_folder_s_number() {
        let v = |n: u64, name: &str| (n, name.to_string());
        let list = [v(1, "NTFS"), v(2, "FAT32"), v(3, "exFAT")];
        assert_eq!(kind_of_volume(2, &list), DriveKind::Fat);
        assert_eq!(kind_of_volume(1, &list), DriveKind::Ntfs);
        assert_eq!(kind_of_volume(3, &list), DriveKind::ExFat);
        assert_eq!(kind_of_volume(9, &list), DriveKind::Unknown);
        assert_eq!(kind_of_volume(1, &[]), DriveKind::Unknown);
        // The same volume listed at two folders: the same answer.
        assert_eq!(
            kind_of_volume(2, &[v(2, "FAT32"), v(1, "NTFS"), v(2, "FAT32")]),
            DriveKind::Fat
        );
        // Two volumes with one number that disagree: unknown, whichever comes first.
        assert_eq!(
            kind_of_volume(2, &[v(2, "FAT32"), v(2, "NTFS")]),
            DriveKind::Unknown
        );
        assert_eq!(
            kind_of_volume(2, &[v(2, "NTFS"), v(2, "FAT32")]),
            DriveKind::Unknown
        );
    }

    /// Two probes must both read back a date alone, and the same one: a single probe another
    /// program touched never decides.
    #[test]
    fn fat_needs_two_probes_that_agree() {
        let set = UNIX_EPOCH + TIME_PROBE_AT;
        let midnight = at(994_118_400);
        let other = at(994_118_400 - 3600);
        assert!(two_agree(set, Some(midnight), Some(midnight)));
        assert!(!two_agree(set, Some(midnight), Some(other)));
        assert!(!two_agree(set, Some(midnight), None));
        assert!(!two_agree(set, None, Some(midnight)));
        assert!(!two_agree(set, Some(midnight), Some(set)));
        assert!(!two_agree(set, Some(set), Some(set)));
    }

    /// The test machine's own drive is not FAT, and the test leaves nothing behind.
    #[test]
    fn the_time_step_test_finds_no_fat_here_and_leaves_nothing() {
        let root = tempfile::tempdir().unwrap();
        let dir = Dir::open_ambient_dir(root.path(), cap_std::ambient_authority()).unwrap();
        assert_eq!(kind_by_time_step(&dir), None);
        assert_eq!(std::fs::read_dir(root.path()).unwrap().count(), 0);
    }
}
