//! Where the journal may be kept (Phase 2 plan 2.3, cases T23-008 and T23-009, adopted for redb
//! on 26 October 2026): only on a drive that is part of this laptop. A database's crash safety
//! rests on the drive honouring its flushes in order; a folder on another computer (a network
//! share, also one mapped to a drive letter) or a drive that can be pulled out may not, so the
//! journal is never opened there, and a drive that cannot be told apart from those is refused
//! too ("in doubt" counts as no).
//!
//! The drive is judged from the journal's folder itself: on Windows, the folder's real path (a
//! mapped network drive resolves to `\\?\UNC\server\share`) and the system's list of volumes; on
//! Linux and macOS, the folder's file system as the system reports it, and the list of volumes
//! for whether it is removable.

use std::path::Path;

/// What kind of place a folder is on, for keeping the journal.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Placement {
    /// A drive that is part of this laptop: the journal may be kept here.
    Local,
    /// A folder on another computer.
    Network,
    /// A drive that can be pulled out (USB sticks, memory cards, external drives).
    Removable,
    /// It could not be told which: treated as not safe.
    Unknown,
}

impl Placement {
    /// Why the journal cannot be kept here, in plain words; `None` when it can.
    pub fn why_not(self) -> Option<&'static str> {
        match self {
            Placement::Local => None,
            Placement::Network => Some(
                "PCTwin keeps its record of the move on this laptop's own drive, not on a network drive",
            ),
            Placement::Removable => Some(
                "PCTwin keeps its record of the move on this laptop's own drive, not on a drive that can be unplugged",
            ),
            Placement::Unknown => Some(
                "PCTwin could not tell whether this drive is part of this laptop, so it did not keep its record of the move there",
            ),
        }
    }
}

/// What the file system says about a folder's drive.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FsKind {
    /// A local file system PCTwin knows.
    Local,
    /// A network file system.
    Network,
    /// Anything else (a file system served by a program, one not known).
    Unknown,
}

/// Decides the placement from the folder's real path (`real`, as the system resolved it), what
/// its file system says, and whether the list of volumes shows its drive as removable (`None`:
/// the drive is not in the list).
pub fn classify(real: &str, fs: FsKind, removable: Option<bool>) -> Placement {
    let windows_unc = real.starts_with(r"\\?\UNC\")
        || (real.starts_with(r"\\") && !real.starts_with(r"\\?\") && !real.starts_with(r"\\.\"));
    if windows_unc || fs == FsKind::Network {
        return Placement::Network;
    }
    match (fs, removable) {
        (_, Some(true)) => Placement::Removable,
        (FsKind::Local, _) => Placement::Local,
        _ => Placement::Unknown,
    }
}

/// The Linux file system kinds known to be local, by `statfs` magic number: ext2/3/4, XFS,
/// btrfs, ZFS, F2FS, tmpfs, ramfs and overlayfs (containers).
pub fn linux_kind(magic: u64) -> FsKind {
    match magic {
        0xEF53 | 0x5846_5342 | 0x9123_683E | 0x2FC1_2FC1 | 0xF2F5_2010 | 0x0102_1994
        | 0x8584_58f6 | 0x794C_7630 => FsKind::Local,
        // NFS, SMB, CIFS, SMB2, AFS, Ceph, 9p.
        0x6969 | 0x517B | 0xFF53_4D42 | 0xFE53_4D42 | 0x5346_414F | 0x00C3_6400 | 0x0102_1997 => {
            FsKind::Network
        }
        _ => FsKind::Unknown,
    }
}

/// Whether the volume list shows the drive holding `real` as removable: the listed volume whose
/// mount folder is the longest one `real` lies inside. `None` when no listed volume holds it.
pub fn removable_in(real: &str, volumes: &[(String, bool)]) -> Option<bool> {
    let fold = |s: &str| {
        if cfg!(windows) {
            s.trim_start_matches(r"\\?\")
                .to_lowercase()
                .replace('/', "\\")
        } else {
            s.to_string()
        }
    };
    let real = fold(real);
    let sep = if cfg!(windows) { '\\' } else { '/' };
    volumes
        .iter()
        .filter(|(mount, _)| {
            let m = fold(mount);
            let m = m.trim_end_matches(sep);
            real == m
                || real
                    .strip_prefix(m)
                    .is_some_and(|rest| rest.starts_with(sep))
        })
        .max_by_key(|(mount, _)| fold(mount).trim_end_matches(sep).len())
        .map(|(_, removable)| *removable)
}

/// Judges the drive of `folder` (which must exist).
pub fn judge(folder: &Path) -> Placement {
    let Ok(real) = std::fs::canonicalize(folder) else {
        return Placement::Unknown;
    };
    let real_text = real.to_string_lossy().into_owned();
    let volumes: Vec<(String, bool)> =
        sysinfo::Disks::new_with_refreshed_list_specifics(sysinfo::DiskRefreshKind::nothing())
            .iter()
            .map(|d| {
                (
                    d.mount_point().to_string_lossy().into_owned(),
                    d.is_removable(),
                )
            })
            .collect();
    classify(
        &real_text,
        fs_kind(&real),
        removable_in(&real_text, &volumes),
    )
}

#[cfg(windows)]
fn fs_kind(_real: &Path) -> FsKind {
    // A network share, also one mapped to a drive letter, shows in the real path itself (a
    // `\\?\UNC\` path, checked by `classify`); every other path is on one of this laptop's
    // volumes, and the volume list says whether that one can be unplugged.
    FsKind::Local
}

#[cfg(target_os = "linux")]
// `f_type` is `i64` on 64-bit Intel and ARM but narrower on some other processors, so the
// conversion is needed there though not here.
#[allow(clippy::useless_conversion)]
fn fs_kind(real: &Path) -> FsKind {
    match rustix::fs::statfs(real) {
        // The type's width differs between processors; its bits are the magic number.
        Ok(st) => linux_kind(i64::from(st.f_type) as u64),
        Err(_) => FsKind::Unknown,
    }
}

#[cfg(target_os = "macos")]
fn fs_kind(real: &Path) -> FsKind {
    /// `MNT_LOCAL` from macOS's `sys/mount.h`: the file system is stored on this computer.
    const MNT_LOCAL: u32 = 0x0000_1000;
    match rustix::fs::statfs(real) {
        Ok(st) if st.f_flags & MNT_LOCAL != 0 => FsKind::Local,
        Ok(_) => FsKind::Network,
        Err(_) => FsKind::Unknown,
    }
}

#[cfg(not(any(windows, target_os = "linux", target_os = "macos")))]
fn fs_kind(_real: &Path) -> FsKind {
    FsKind::Unknown
}
