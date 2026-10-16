//! What kind of drive an approved destination is on, read once from its open folder when it is
//! opened (security design notes, "Naming a finished file never replaces anything", decided 15
//! October 2026). The drive's type, never the laptop's system, decides which names it takes:
//! FAT, exFAT, NTFS, network and unknown drives get Windows' rules on every system; only known
//! POSIX drives (APFS, HFS+, ext4, XFS, btrfs, ZFS, F2FS, and Linux's in-memory file systems)
//! keep the system's own.
//!
//! The type is read from the folder handle itself, never from a path: Linux's `fstatfs` magic
//! number, macOS's `f_fstypename`. Windows has no safe way to read it from a handle with the
//! crates this workspace already uses (`winapi-util` wraps no volume call, and calling
//! `GetVolumeInformationByHandleW` directly needs unsafe code with buffers, which the workspace
//! forbids). Where the type cannot be read (Windows; a drive served by a program on Linux), the
//! drive is asked how it keeps time instead, once, on a probe file of PCTwin's own: FAT keeps a
//! file's modified time in two-second steps, exFAT and NTFS in hundredths or finer. So a FAT drive
//! is known everywhere, and its 4 GB limit holds on every system.
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

/// The modified time the time-step test sets: 3 July 2001, 04:05:07.01 UTC (an odd second and a
/// hundredth, which only a drive with two-second steps cannot keep).
const TIME_PROBE_AT: std::time::Duration = std::time::Duration::from_millis(994_133_107_010);

/// Whether a modified time set to `set` and read back as `got` shows FAT's two-second steps: the
/// read-back is an even whole second, and not what was set.
pub(crate) fn two_second_steps(set: std::time::SystemTime, got: std::time::SystemTime) -> bool {
    let Ok(d) = got.duration_since(std::time::UNIX_EPOCH) else {
        return false;
    };
    got != set && d.subsec_nanos() == 0 && d.as_secs() % 2 == 0
}

/// Asks the drive under `dir` how it keeps time, on a probe file of PCTwin's own made only if
/// free and removed at once: `Some(Fat)` for two-second steps, `None` if not (or the test could
/// not run: a folder PCTwin may not write to, say; nothing is then assumed).
fn kind_by_time_step(dir: &Dir) -> Option<DriveKind> {
    use std::hash::{BuildHasher, RandomState};
    use std::io::Write;
    let tag = RandomState::new().hash_one(std::process::id());
    let name = format!(".pctwin-probe-{tag:016x}-t");
    let mut options = cap_std::fs::OpenOptions::new();
    options.write(true).create_new(true);
    let set = std::time::UNIX_EPOCH + TIME_PROBE_AT;
    let kept = (|| {
        let mut f = dir.open_with(&name, &options).ok()?.into_std();
        f.write_all(TIME_PROBE).ok()?;
        f.set_modified(set).ok()?;
        f.sync_all().ok()?;
        drop(f);
        dir.open(&name)
            .ok()?
            .into_std()
            .metadata()
            .ok()?
            .modified()
            .ok()
    })();
    let _ = dir.remove_file(&name);
    kept.filter(|got| two_second_steps(set, *got))
        .map(|_| DriveKind::Fat)
}

/// The number of the drive the folder `dir` is on, as long as it stays plugged in.
pub(crate) fn volume_key(dir: &Dir) -> std::io::Result<u64> {
    #[cfg(unix)]
    {
        #[allow(
            clippy::useless_conversion,
            clippy::unnecessary_cast,
            reason = "the device number's width differs by target"
        )]
        Ok(rustix::fs::fstat(dir)?.st_dev as u64)
    }
    #[cfg(windows)]
    {
        Ok(
            winapi_util::file::information(dir.try_clone()?.into_std_file())?
                .volume_serial_number(),
        )
    }
    #[cfg(not(any(unix, windows)))]
    {
        let _ = dir;
        Ok(0)
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

#[cfg(not(any(target_os = "linux", target_os = "macos")))]
fn read_kind(_dir: &Dir) -> std::io::Result<DriveKind> {
    Ok(DriveKind::Unknown)
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::time::{Duration, UNIX_EPOCH};

    #[test]
    fn two_second_steps_are_told_from_finer_ones() {
        let set = UNIX_EPOCH + TIME_PROBE_AT;
        // FAT: rounded to an even whole second, either way.
        assert!(two_second_steps(
            set,
            UNIX_EPOCH + Duration::from_secs(994_133_106)
        ));
        assert!(two_second_steps(
            set,
            UNIX_EPOCH + Duration::from_secs(994_133_108)
        ));
        // exFAT keeps hundredths; NTFS, APFS and ext4 keep it exactly.
        assert!(!two_second_steps(set, set));
        assert!(!two_second_steps(
            set,
            UNIX_EPOCH + Duration::from_millis(994_133_107_000)
        ));
        assert!(!two_second_steps(
            set,
            UNIX_EPOCH + Duration::from_millis(994_133_106_500)
        ));
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
