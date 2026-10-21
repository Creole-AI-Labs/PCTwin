//! Every volume the sweep looks at, and what kind it is: the laptop's own drives are walked;
//! removable drives, network folders and disc images are listed, not walked (Engineering Plan
//! 9.16, "Sweep order").

use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};
use std::time::{Duration, Instant};

use serde::{Deserialize, Serialize};

use crate::drives::{FileSystem, OtherSystem, is_real_mount, other_system};
use crate::folders::same_place;

/// What kind of volume it is, which decides whether the sweep walks it.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum VolumeKind {
    /// Part of the laptop: walked.
    Internal,
    /// A USB or external drive: "a separate drive: plug it into the new laptop". Listed, not
    /// walked, not included by default.
    Removable,
    /// A network folder: not on this laptop. Listed, not walked.
    Network,
    /// A disc or disc image. Listed, not walked.
    DiscImage,
    /// A drive holding another installed system: read only through rescue, with permission.
    OtherSystem,
}

/// Whether a drive spins (one walker) or not (up to four).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum DiskKind {
    Ssd,
    Hdd,
    Unknown,
}

impl DiskKind {
    /// How many walkers may read this disk at once: one on a hard disk (or when unknown, since a
    /// hard disk slows down badly when read in several places at once), up to four on an SSD.
    pub fn walkers(self, most: usize) -> usize {
        match self {
            DiskKind::Ssd => most.clamp(1, 4),
            DiskKind::Hdd | DiskKind::Unknown => 1,
        }
    }
}

/// One volume.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Volume {
    /// Where it is mounted (`D:\`, `/`, `/mnt/data`).
    pub mount: PathBuf,
    pub name: String,
    pub kind: VolumeKind,
    pub file_system: String,
    pub total_bytes: u64,
    pub free_bytes: u64,
    pub disk: DiskKind,
    /// The drive the running system started from.
    pub is_system: bool,
    /// Folders below the mount that are other volumes or the system's own views (`/proc`,
    /// `/System/Volumes`), never walked from here.
    pub skip: Vec<PathBuf>,
    /// macOS: the data volume behind `/` (`/System/Volumes/Data`); folders on it count as this
    /// volume.
    pub also: Vec<PathBuf>,
}

impl Volume {
    pub fn used_bytes(&self) -> u64 {
        self.total_bytes.saturating_sub(self.free_bytes)
    }

    /// The drive keeps owners and permissions.
    pub fn keeps_owners(&self) -> bool {
        FileSystem::from_name(&self.file_system).keeps().permissions
    }

    /// A short name for the drive, as the record's storage names it (`D:`, `/mnt/data`).
    pub fn label(&self) -> String {
        let text = self.mount.to_string_lossy();
        if cfg!(windows) {
            text.trim_end_matches(['\\', '/']).to_string()
        } else {
            text.into_owned()
        }
    }
}

/// The kind of a volume from its file system name and the system's own answers.
pub fn volume_kind(file_system: &str, removable: bool, other: Option<OtherSystem>) -> VolumeKind {
    let fs = file_system.to_ascii_lowercase();
    let network = [
        "nfs",
        "nfs4",
        "cifs",
        "smbfs",
        "smb3",
        "afpfs",
        "webdav",
        "fuse.sshfs",
        "9p",
        "davfs",
    ];
    if network.contains(&fs.as_str()) {
        VolumeKind::Network
    } else if ["iso9660", "udf", "cd9660", "cdfs"].contains(&fs.as_str()) {
        VolumeKind::DiscImage
    } else if removable {
        VolumeKind::Removable
    } else if other.is_some() {
        VolumeKind::OtherSystem
    } else {
        VolumeKind::Internal
    }
}

/// Every volume on this laptop.
pub fn volumes_this_laptop() -> Vec<Volume> {
    let system_root = if cfg!(windows) {
        PathBuf::from(format!(
            "{}\\",
            std::env::var("SystemDrive").unwrap_or_else(|_| "C:".into())
        ))
    } else {
        PathBuf::from("/")
    };
    let usb = if cfg!(windows) {
        windows_usb_letters()
    } else {
        Vec::new()
    };
    let disks = sysinfo::Disks::new_with_refreshed_list();
    let mut out: Vec<Volume> = Vec::new();
    for d in disks.list() {
        let fs = d.file_system().to_string_lossy().into_owned();
        let mount = d.mount_point().to_path_buf();
        if !is_real_mount(&fs, &mount) || out.iter().any(|v| same_place(&v.mount, &mount)) {
            continue;
        }
        let is_system = same_place(&mount, &system_root);
        let letter_on_usb = usb.iter().any(|l| same_place(&mount, Path::new(l)));
        let other = if is_system {
            None
        } else {
            other_system(&mount)
        };
        out.push(Volume {
            name: d.name().to_string_lossy().into_owned(),
            kind: volume_kind(&fs, d.is_removable() || letter_on_usb, other),
            file_system: fs,
            total_bytes: d.total_space(),
            free_bytes: d.available_space(),
            disk: match d.kind() {
                sysinfo::DiskKind::SSD => DiskKind::Ssd,
                sysinfo::DiskKind::HDD => DiskKind::Hdd,
                sysinfo::DiskKind::Unknown(_) => DiskKind::Unknown,
            },
            is_system,
            skip: Vec::new(),
            also: Vec::new(),
            mount,
        });
    }
    if cfg!(windows) {
        add_windows_network_and_discs(&mut out);
    }
    // Folders inside one volume that are the mount of another are walked as that one (or are
    // the system's own views, like /proc); every mount the system lists counts, not only the
    // drives above.
    let mut mounts: Vec<PathBuf> = out.iter().map(|v| v.mount.clone()).collect();
    mounts.extend(all_mount_points());
    for v in &mut out {
        v.skip.extend(
            mounts
                .iter()
                .filter(|m| !same_place(m, &v.mount) && crate::folders::is_within(m, &v.mount))
                .cloned(),
        );
        if !cfg!(windows) && v.is_system {
            for p in [
                "/proc", "/sys", "/dev", "/run", "/net", "/Volumes", "/Network",
            ] {
                v.skip.push(PathBuf::from(p));
            }
            if cfg!(target_os = "macos") {
                v.skip.push(PathBuf::from("/System/Volumes"));
                v.skip.push(PathBuf::from("/home"));
                v.also.push(PathBuf::from("/System/Volumes/Data"));
                // What the data volume shows through `/` (its firmlinks) is seen there, not
                // again from the data volume's root.
                let firmlinks = std::fs::read_to_string("/usr/share/firmlinks").unwrap_or_default();
                v.skip.extend(firmlink_targets(&firmlinks));
            }
        }
    }
    out
}

/// macOS: the data-volume folders `/usr/share/firmlinks` shows through `/` (lines like
/// `/Users<tab>Users`).
pub fn firmlink_targets(text: &str) -> Vec<PathBuf> {
    text.lines()
        .filter_map(|l| l.split('\t').nth(1))
        .map(str::trim)
        .filter(|t| !t.is_empty())
        .map(|t| PathBuf::from("/System/Volumes/Data").join(t))
        .collect()
}

/// Every mount point the system lists (Linux: `/proc/self/mounts`; elsewhere the drive list),
/// including ones that are not drives (`/proc`, a tmpfs, a FUSE folder).
fn all_mount_points() -> Vec<PathBuf> {
    if cfg!(target_os = "linux") {
        let text = std::fs::read_to_string("/proc/self/mounts").unwrap_or_default();
        mount_points_from_proc(&text)
    } else {
        sysinfo::Disks::new_with_refreshed_list()
            .list()
            .iter()
            .map(|d| d.mount_point().to_path_buf())
            .collect()
    }
}

/// Mount points from `/proc/self/mounts` (the second field, with `\040` for spaces and the like).
pub fn mount_points_from_proc(text: &str) -> Vec<PathBuf> {
    text.lines()
        .filter_map(|l| l.split(' ').nth(1))
        .map(unescape_octal)
        .map(PathBuf::from)
        .collect()
}

fn unescape_octal(field: &str) -> String {
    let bytes = field.as_bytes();
    let mut out = Vec::with_capacity(bytes.len());
    let mut i = 0;
    while i < bytes.len() {
        if bytes[i] == b'\\'
            && i + 3 < bytes.len()
            && bytes[i + 1..i + 4]
                .iter()
                .all(|b| (b'0'..=b'7').contains(b))
        {
            let v = (bytes[i + 1] - b'0') * 64 + (bytes[i + 2] - b'0') * 8 + (bytes[i + 3] - b'0');
            out.push(v);
            i += 4;
        } else {
            out.push(bytes[i]);
            i += 1;
        }
    }
    String::from_utf8_lossy(&out).into_owned()
}

/// Windows: drive letters on USB disks, which can show as fixed drives.
fn windows_usb_letters() -> Vec<String> {
    let script = "[Console]::OutputEncoding=[Text.Encoding]::UTF8; \
        Get-Partition | Where-Object DriveLetter | ForEach-Object { \
          $d = Get-Disk -Number $_.DiskNumber; if ([string]$d.BusType -eq 'USB') { [string]$_.DriveLetter + ':\\' } }";
    run_powershell(script)
        .map(|out| {
            out.lines()
                .map(str::trim)
                .filter(|l| !l.is_empty())
                .map(str::to_string)
                .collect()
        })
        .unwrap_or_default()
}

/// Windows: network drive letters and discs, which the drive list leaves out.
fn add_windows_network_and_discs(out: &mut Vec<Volume>) {
    let script = "[Console]::OutputEncoding=[Text.Encoding]::UTF8; \
        Get-CimInstance Win32_LogicalDisk | ForEach-Object { \
          [string]$_.DeviceID + [char]9 + [string]$_.DriveType + [char]9 + [string]$_.FileSystem + [char]9 + \
          [string]$_.Size + [char]9 + [string]$_.FreeSpace + [char]9 + [string]$_.VolumeName }";
    let Ok(text) = run_powershell(script) else {
        return;
    };
    for v in logical_disks(&text) {
        if !out.iter().any(|o| same_place(&o.mount, &v.mount)) {
            out.push(v);
        }
    }
}

/// Reads `DeviceID <tab> DriveType <tab> FileSystem <tab> Size <tab> FreeSpace <tab> Name` lines
/// (Win32_LogicalDisk), keeping network drives (type 4) and discs (type 5).
pub fn logical_disks(text: &str) -> Vec<Volume> {
    text.lines()
        .filter_map(|line| {
            let f: Vec<&str> = line.trim_end_matches('\r').split('\t').collect();
            if f.len() < 6 {
                return None;
            }
            let kind = match f[1].trim() {
                "4" => VolumeKind::Network,
                "5" => VolumeKind::DiscImage,
                _ => return None,
            };
            Some(Volume {
                mount: PathBuf::from(format!("{}\\", f[0].trim())),
                name: f[5].trim().to_string(),
                kind,
                file_system: f[2].trim().to_string(),
                total_bytes: f[3].trim().parse().unwrap_or(0),
                free_bytes: f[4].trim().parse().unwrap_or(0),
                disk: DiskKind::Unknown,
                is_system: false,
                skip: Vec::new(),
                also: Vec::new(),
            })
        })
        .collect()
}

/// Runs a PowerShell script with a time limit, without a window.
pub(crate) fn run_powershell(script: &str) -> Result<String, String> {
    let exe = std::env::var_os("SystemRoot")
        .map(|r| PathBuf::from(r).join(r"System32\WindowsPowerShell\v1.0\powershell.exe"))
        .unwrap_or_else(|| PathBuf::from("powershell"));
    let mut command = Command::new(exe);
    command
        .args(["-NoProfile", "-NonInteractive", "-Command", script])
        .stdin(Stdio::null())
        .stdout(Stdio::piped())
        .stderr(Stdio::null());
    #[cfg(windows)]
    {
        use std::os::windows::process::CommandExt;
        const CREATE_NO_WINDOW: u32 = 0x0800_0000;
        command.creation_flags(CREATE_NO_WINDOW);
    }
    let mut child = command.spawn().map_err(|e| e.to_string())?;
    let started = Instant::now();
    loop {
        match child.try_wait() {
            Ok(Some(_)) => break,
            Ok(None) if started.elapsed() > Duration::from_secs(30) => {
                let _ = child.kill();
                let _ = child.wait();
                return Err("took too long".into());
            }
            Ok(None) => std::thread::sleep(Duration::from_millis(50)),
            Err(e) => return Err(e.to_string()),
        }
    }
    let output = child.wait_with_output().map_err(|e| e.to_string())?;
    if !output.status.success() {
        return Err("reported a problem".into());
    }
    Ok(String::from_utf8_lossy(&output.stdout).into_owned())
}
