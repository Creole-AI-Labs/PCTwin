use std::path::{Path, PathBuf};

use serde::Serialize;

use crate::folders::same_place;

/// A drive's file system.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "kebab-case")]
pub enum FileSystem {
    Ntfs,
    ReFs,
    Fat32,
    ExFat,
    Apfs,
    HfsPlus,
    Ext4,
    Btrfs,
    Xfs,
    Other(String),
}

/// Whether names that differ only in capital letters are the same name.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "kebab-case")]
pub enum CaseRule {
    IgnoresCase,
    KeepsCase,
    Unknown,
}

/// What a file system can keep, so a move knows what will survive on it.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
pub struct Keeps {
    /// The largest single file, or `None` for no practical limit.
    pub largest_file: Option<u64>,
    /// Owners and permissions on files.
    pub permissions: bool,
    /// Links and shortcuts that point elsewhere.
    pub links: bool,
    /// Extra hidden data attached to a file (Windows streams, such as download markers).
    pub extra_streams: bool,
    pub case: CaseRule,
    /// The smallest step a saved time can take, in nanoseconds.
    pub time_step_ns: u64,
}

impl FileSystem {
    /// Reads the name each system uses (`NTFS`, `vfat`, `msdos`, `apfs`, `ext4`, ...).
    pub fn from_name(name: &str) -> Self {
        match name.to_ascii_lowercase().as_str() {
            "ntfs" | "ntfs3" => Self::Ntfs,
            "refs" => Self::ReFs,
            "fat32" | "vfat" | "msdos" | "fat" => Self::Fat32,
            "exfat" => Self::ExFat,
            "apfs" => Self::Apfs,
            "hfs" | "hfs+" | "hfsplus" => Self::HfsPlus,
            "ext4" | "ext3" | "ext2" => Self::Ext4,
            "btrfs" => Self::Btrfs,
            "xfs" => Self::Xfs,
            _ => Self::Other(name.to_string()),
        }
    }

    pub fn keeps(&self) -> Keeps {
        let full = |case, time_step_ns| Keeps {
            largest_file: None,
            permissions: true,
            links: true,
            extra_streams: false,
            case,
            time_step_ns,
        };
        match self {
            Self::Ntfs | Self::ReFs => Keeps {
                extra_streams: true,
                ..full(CaseRule::IgnoresCase, 100)
            },
            Self::Fat32 => Keeps {
                largest_file: Some(4 * 1024 * 1024 * 1024 - 1),
                permissions: false,
                links: false,
                extra_streams: false,
                case: CaseRule::IgnoresCase,
                time_step_ns: 2_000_000_000,
            },
            Self::ExFat => Keeps {
                largest_file: None,
                permissions: false,
                links: false,
                extra_streams: false,
                case: CaseRule::IgnoresCase,
                time_step_ns: 10_000_000,
            },
            // APFS and HFS+ ignore case unless formatted as case-sensitive, which is rare.
            Self::Apfs => full(CaseRule::IgnoresCase, 1),
            Self::HfsPlus => full(CaseRule::IgnoresCase, 1_000_000_000),
            Self::Ext4 | Self::Btrfs | Self::Xfs => full(CaseRule::KeepsCase, 1),
            // Unknown types promise nothing.
            Self::Other(_) => Keeps {
                largest_file: None,
                permissions: false,
                links: false,
                extra_streams: false,
                case: CaseRule::Unknown,
                time_step_ns: 2_000_000_000,
            },
        }
    }
}

/// Another operating system installed on a drive.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "kebab-case")]
pub enum OtherSystem {
    Windows,
    MacOs,
    Linux,
}

/// One real drive on the laptop.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct Drive {
    pub name: String,
    pub mount: PathBuf,
    pub file_system: FileSystem,
    pub removable: bool,
    pub read_only: bool,
    pub total_bytes: u64,
    pub free_bytes: u64,
    /// The drive the running system started from.
    pub is_system: bool,
    /// Another installed system on this drive. It belongs to that system, so it is read only
    /// through rescue, with permission.
    pub other_system: Option<OtherSystem>,
}

/// Every real drive, with its type. Temporary and system-internal mounts are left out.
pub fn list_drives() -> Vec<Drive> {
    let system_root = system_root();
    let disks = sysinfo::Disks::new_with_refreshed_list();
    let mut drives: Vec<Drive> = disks
        .list()
        .iter()
        .filter_map(|d| {
            let fs_name = d.file_system().to_string_lossy().into_owned();
            let mount = d.mount_point().to_path_buf();
            if !is_real_mount(&fs_name, &mount) {
                return None;
            }
            let is_system = same_place(&mount, &system_root);
            Some(Drive {
                name: d.name().to_string_lossy().into_owned(),
                file_system: FileSystem::from_name(&fs_name),
                removable: d.is_removable(),
                read_only: d.is_read_only(),
                total_bytes: d.total_space(),
                free_bytes: d.available_space(),
                other_system: if is_system {
                    None
                } else {
                    other_system(&mount)
                },
                is_system,
                mount,
            })
        })
        .collect();
    // The same drive can be listed more than once (Linux bind mounts); keep the first.
    let mut seen = Vec::new();
    drives.retain(|d| {
        let fresh = !seen.iter().any(|m: &PathBuf| same_place(m, &d.mount));
        seen.push(d.mount.clone());
        fresh
    });
    drives
}

fn system_root() -> PathBuf {
    if cfg!(windows) {
        let drive = std::env::var("SystemDrive").unwrap_or_else(|_| "C:".into());
        PathBuf::from(format!("{drive}\\"))
    } else {
        PathBuf::from("/")
    }
}

/// Whether a mount is a real drive, not a temporary, virtual or system-internal one.
pub fn is_real_mount(file_system: &str, mount: &Path) -> bool {
    const VIRTUAL: &[&str] = &[
        "tmpfs",
        "ramfs",
        "proc",
        "sysfs",
        "devtmpfs",
        "devpts",
        "devfs",
        "squashfs",
        "overlay",
        "efivarfs",
        "autofs",
        "cgroup",
        "cgroup2",
        "securityfs",
        "debugfs",
        "tracefs",
        "pstore",
        "bpf",
        "mqueue",
        "hugetlbfs",
        "configfs",
        "fusectl",
        "binfmt_misc",
        "nsfs",
        "rpc_pipefs",
        "nfsd",
        "selinuxfs",
        "nullfs",
        "fuse.portal",
        "fuse.gvfsd-fuse",
        "fuse.snapfuse",
    ];
    if VIRTUAL.contains(&file_system.to_ascii_lowercase().as_str()) {
        return false;
    }
    let m = mount.to_string_lossy();
    // macOS's internal volumes behind "/" (VM, Preboot, Update, Data) are part of the system
    // drive; Linux keeps virtual and container mounts under these folders.
    let internal = [
        "/System/Volumes/",
        "/proc",
        "/sys",
        "/dev",
        "/snap/",
        "/var/lib/docker/",
        "/var/snap/",
    ];
    let removable_media = m.starts_with("/run/media/");
    removable_media || !internal.iter().any(|p| m.starts_with(p)) && !m.starts_with("/run/")
}

/// Another installed system on the drive at `root`, found by its own marker files.
pub fn other_system(root: &Path) -> Option<OtherSystem> {
    if root
        .join("Windows")
        .join("System32")
        .join("config")
        .join("SYSTEM")
        .is_file()
    {
        Some(OtherSystem::Windows)
    } else if root
        .join("System/Library/CoreServices/SystemVersion.plist")
        .is_file()
    {
        Some(OtherSystem::MacOs)
    } else if root.join("etc/os-release").exists() {
        Some(OtherSystem::Linux)
    } else {
        None
    }
}
