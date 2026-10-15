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
//! `GetVolumeInformationByHandleW` directly needs unsafe code, which the workspace forbids), so
//! there the type is [`DriveKind::Unknown`]: harmless for names, since Windows applies its own
//! rules on every drive anyway.

use cap_std::fs::Dir;

use crate::{NameRules, Platform};

/// The largest file a FAT drive (FAT12, FAT16, FAT32) holds: 4 GiB less one byte.
pub const FAT_LARGEST_FILE: u64 = (1 << 32) - 1;

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
    /// is unknown, and gets the strictest rules.
    pub(crate) fn read(dir: &Dir) -> Self {
        Self::of(read_kind(dir).unwrap_or(DriveKind::Unknown))
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
