//! Where the journal may be kept (Phase 2 plan 2.3, cases T23-008 and T23-009, adopted for redb
//! on 26 October 2026): only on a drive that is part of this laptop. A database's crash safety
//! rests on the drive honouring its flushes in order; a folder on another computer (a network
//! share, also one mapped to a drive letter) or a drive that can be pulled out may not, so the
//! journal is never opened there, and a drive that cannot be told apart from those is refused
//! too ("in doubt" counts as no).
//!
//! The drive is judged from the journal's folder itself: on Windows, the folder's real path (a
//! mapped network drive resolves to `\\?\UNC\server\share`), the bus its disk is attached by
//! ([`windows_placement`]: a USB hard disk reports itself "fixed", so the bus decides; security
//! review finding 4) and the system's list of volumes; on Linux and macOS, the folder's file
//! system as the system reports it, and the list of volumes for whether it is removable.

use std::path::Path;
use std::sync::OnceLock;

/// Whether a disk can be unplugged, as the disk itself says (Windows numbers its disks). The
/// journal never makes the system call itself and never links the crate that does (third
/// security review, finding 8): the app and the administrator helper install the real probe at
/// start ([`install_disk_probe`]); tests use stand-ins.
pub trait DiskProbe: Send + Sync {
    /// `Some(true)`: it can be unplugged; `Some(false)`: it cannot; `None`: it would not say.
    fn hotplug(&self, disk: u32) -> Option<bool>;
}

static PROBE: OnceLock<Box<dyn DiskProbe>> = OnceLock::new();

/// Gives the journal its disk probe, once for this program; a second one is ignored. Returns
/// whether this one was taken.
pub fn install_disk_probe(probe: Box<dyn DiskProbe>) -> bool {
    PROBE.set(probe).is_ok()
}

/// Whether a disk probe was installed in this program.
pub fn disk_probe_installed() -> bool {
    PROBE.get().is_some()
}

/// What the probe says of disk `disk`. With no probe installed (a test, a tool) the disk is not
/// asked, and the bus and the list of volumes decide alone, as they did before 27 October 2026;
/// the app and the helper always install one.
#[cfg(windows)]
fn hotplug_of(disk: u32) -> Option<bool> {
    match PROBE.get() {
        Some(probe) => probe.hotplug(disk),
        None => Some(false),
    }
}

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

/// The buses (Windows' `STORAGE_BUS_TYPE`) of disks inside the laptop: SCSI, ATAPI, ATA, RAID,
/// SAS, SATA, a virtual machine's own disk (`Virtual`, as cloud and Hyper-V machines report),
/// Storage Spaces, NVMe, storage-class memory and UFS.
pub const LOCAL_BUSES: [u16; 11] = [1, 2, 3, 8, 10, 11, 14, 16, 17, 18, 19];
/// The buses of disks that can be pulled out: USB, FireWire, SD cards and MMC cards.
pub const REMOVABLE_BUSES: [u16; 4] = [7, 4, 12, 13];
/// The buses of disks on another computer: iSCSI and Fibre Channel.
pub const NETWORK_BUSES: [u16; 2] = [9, 6];

/// Windows: decides the placement from the folder's real path, the bus of the disk its volume is
/// on (`None`: it could not be found, or the volume is on no disk this laptop lists), whether the
/// volume list shows the drive as removable (`None`: not listed) and whether the disk itself says
/// it can be unplugged or its medium taken out (`hotplug`; `None`: it would not say). Only a disk
/// positively on a bus inside the laptop, listed as not removable and saying it cannot be
/// unplugged, is local: an NVMe disk in a Thunderbolt or USB4 case reports the NVMe bus but can
/// be unplugged (second review, low finding). Anything else, including a disk file mounted as a
/// drive (`FileBackedVirtual`: the file could be anywhere) and an unknown bus, is not ("in doubt"
/// counts as no).
pub fn windows_placement(
    real: &str,
    bus: Option<u16>,
    listed_removable: Option<bool>,
    hotplug: Option<bool>,
) -> Placement {
    if classify(real, FsKind::Unknown, None) == Placement::Network {
        return Placement::Network;
    }
    match (bus, listed_removable, hotplug) {
        (Some(b), _, _) if NETWORK_BUSES.contains(&b) => Placement::Network,
        (Some(b), _, _) if REMOVABLE_BUSES.contains(&b) => Placement::Removable,
        (_, Some(true), _) | (_, _, Some(true)) => Placement::Removable,
        (Some(b), Some(false), Some(false)) if LOCAL_BUSES.contains(&b) => Placement::Local,
        _ => Placement::Unknown,
    }
}

/// Linux: whether the device holding a block device, or any device on the way up to its bus,
/// says it can be removed. `dev` is the block device's real folder under `/sys/devices`; `read`
/// reads a file there. A block device's own `removable` says `1`; a bus device's (PCI behind an
/// external-facing port such as Thunderbolt or USB4, or USB) says `removable` (second review, low
/// finding: an NVMe disk in a Thunderbolt case has a block `removable` of 0).
pub fn linux_removable_chain(dev: &Path, read: &dyn Fn(&Path) -> Option<String>) -> bool {
    dev.ancestors()
        .take_while(|d| d.starts_with("/sys/devices") && *d != Path::new("/sys/devices"))
        .any(|d| read(&d.join("removable")).is_some_and(|v| matches!(v.trim(), "1" | "removable")))
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
                    d.is_removable() || device_removable(d.name()),
                )
            })
            .collect();
    #[cfg(windows)]
    let placement = {
        let found = windows_bus::bus_of(&real_text);
        let hotplug = found.and_then(|(_, disk)| hotplug_of(disk));
        windows_placement(
            &real_text,
            found.map(|(bus, _)| bus),
            removable_in(&real_text, &volumes),
            hotplug,
        )
    };
    #[cfg(not(windows))]
    let placement = classify(
        &real_text,
        fs_kind(&real),
        removable_in(&real_text, &volumes),
    );
    placement
}

/// Windows: the bus of the disk holding a path, from the system's storage list
/// (`root\Microsoft\Windows\Storage`: each partition's access paths and disk, each disk's bus),
/// through Windows Management Instrumentation with the `wmi` crate (safe Rust, no new unsafe).
#[cfg(windows)]
mod windows_bus {
    use serde::Deserialize;

    #[derive(Deserialize)]
    #[expect(non_snake_case, reason = "WMI's own names")]
    struct Partition {
        DiskNumber: u32,
        AccessPaths: Option<Vec<String>>,
    }

    #[derive(Deserialize)]
    #[expect(non_snake_case, reason = "WMI's own names")]
    struct Disk {
        Number: u32,
        BusType: u16,
    }

    /// The bus and number of the disk whose partition has the longest access path (a drive
    /// letter, or a folder a volume is mounted in) that holds `real`; `None` when none does or
    /// the system cannot say. Asked on a thread of its own, so the caller's own COM setup never
    /// matters.
    pub(super) fn bus_of(real: &str) -> Option<(u16, u32)> {
        let real = real.to_string();
        std::thread::spawn(move || ask(&real)).join().ok().flatten()
    }

    fn ask(real: &str) -> Option<(u16, u32)> {
        let fold = |s: &str| {
            s.trim_start_matches(r"\\?\")
                .to_lowercase()
                .replace('/', "\\")
        };
        let real = fold(real);
        let con =
            wmi::WMIConnection::with_namespace_path("ROOT\\Microsoft\\Windows\\Storage").ok()?;
        let partitions: Vec<Partition> = con
            .raw_query("SELECT DiskNumber, AccessPaths FROM MSFT_Partition")
            .ok()?;
        let (disk, _) = partitions
            .iter()
            .flat_map(|p| {
                p.AccessPaths
                    .iter()
                    .flatten()
                    .map(move |a| (p.DiskNumber, fold(a)))
            })
            .filter(|(_, a)| {
                let a = a.trim_end_matches('\\');
                !a.is_empty()
                    && (real == a || real.strip_prefix(a).is_some_and(|r| r.starts_with('\\')))
            })
            .max_by_key(|(_, a)| a.len())?;
        let disks: Vec<Disk> = con
            .raw_query("SELECT Number, BusType FROM MSFT_Disk")
            .ok()?;
        disks
            .iter()
            .find(|d| d.Number == disk)
            .map(|d| (d.BusType, d.Number))
    }
}

/// Linux: whether the device named `name` (`/dev/nvme1n1p2`, say) or a device above it says it
/// can be removed ([`linux_removable_chain`]).
#[cfg(target_os = "linux")]
fn device_removable(name: &std::ffi::OsStr) -> bool {
    let Some(short) = Path::new(name).file_name() else {
        return false;
    };
    let Ok(dev) = std::fs::canonicalize(Path::new("/sys/class/block").join(short)) else {
        return false;
    };
    linux_removable_chain(&dev, &|p| std::fs::read_to_string(p).ok())
}

/// Elsewhere the volume list alone says whether a drive can be removed (Windows asks the disk
/// itself in [`judge`]; macOS counts an ejectable disk, such as a Thunderbolt one, as removable).
#[cfg(not(target_os = "linux"))]
fn device_removable(_: &std::ffi::OsStr) -> bool {
    false
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
