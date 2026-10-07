use std::path::{Path, PathBuf};

use serde::Serialize;

/// What a folder holds, found without opening any file.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize)]
pub struct Measure {
    pub files: u64,
    /// Folders inside the measured folder (not counting itself).
    pub folders: u64,
    /// Full size of every file, including files that are only in the cloud.
    pub bytes: u64,
    /// Links and junctions, counted but never followed.
    pub links: u64,
    /// Files whose contents are only in the cloud (OneDrive, iCloud); never downloaded by a scan.
    pub cloud_only_files: u64,
    pub cloud_only_bytes: u64,
    /// Folders and files that could not be read, listed rather than hidden.
    pub unreadable: Vec<PathBuf>,
}

/// A size to show, honest about what could not be read.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(tag = "kind", rename_all = "kebab-case")]
pub enum Size {
    /// Everything was read.
    Known { bytes: u64, files: u64 },
    /// Some parts could not be read, so the real size is at least this.
    AtLeast { bytes: u64, files: u64 },
    /// The folder itself could not be read (it belongs to someone else, or isn't there).
    Unreadable,
}

impl Size {
    pub fn of(path: &Path) -> Self {
        let m = measure(path);
        if m.unreadable.iter().any(|p| p == path) {
            Size::Unreadable
        } else if m.unreadable.is_empty() {
            Size::Known {
                bytes: m.bytes,
                files: m.files,
            }
        } else {
            Size::AtLeast {
                bytes: m.bytes,
                files: m.files,
            }
        }
    }
}

/// Walks `root` reading only folder listings and file details, so no cloud file is downloaded and
/// no file is opened. Links and junctions are never followed.
pub fn measure(root: &Path) -> Measure {
    let mut m = Measure::default();
    let mut pending = vec![root.to_path_buf()];
    while let Some(dir) = pending.pop() {
        let Ok(entries) = std::fs::read_dir(&dir) else {
            m.unreadable.push(dir);
            continue;
        };
        for entry in entries {
            let Ok(entry) = entry else {
                m.unreadable.push(dir.clone());
                continue;
            };
            let path = entry.path();
            // The kind and details come from the listing itself and never follow a link.
            let (Ok(kind), Ok(meta)) = (entry.file_type(), entry.metadata()) else {
                m.unreadable.push(path);
                continue;
            };
            if kind.is_symlink() {
                m.links += 1;
            } else if kind.is_dir() {
                m.folders += 1;
                pending.push(path);
            } else {
                m.files += 1;
                let stub = is_icloud_stub_name(&entry.file_name().to_string_lossy());
                if stub {
                    // A stand-in for a file in iCloud; its real size isn't known here.
                    m.cloud_only_files += 1;
                    continue;
                }
                m.bytes += meta.len();
                if is_cloud_only(&meta) {
                    m.cloud_only_files += 1;
                    m.cloud_only_bytes += meta.len();
                }
            }
        }
    }
    m
}

#[cfg(windows)]
fn is_cloud_only(meta: &std::fs::Metadata) -> bool {
    use std::os::windows::fs::MetadataExt;
    cloud_only_windows(meta.file_attributes())
}

#[cfg(target_os = "macos")]
fn is_cloud_only(meta: &std::fs::Metadata) -> bool {
    use std::os::macos::fs::MetadataExt;
    cloud_only_mac(meta.st_flags())
}

#[cfg(not(any(windows, target_os = "macos")))]
fn is_cloud_only(_meta: &std::fs::Metadata) -> bool {
    false
}

/// Windows: the file's contents are not on the laptop (OneDrive Files On-Demand and other cloud
/// folders), from its attributes. Reading such a file would download it.
pub fn cloud_only_windows(attributes: u32) -> bool {
    const OFFLINE: u32 = 0x1000;
    const RECALL_ON_OPEN: u32 = 0x4_0000;
    const RECALL_ON_DATA_ACCESS: u32 = 0x40_0000;
    attributes & (OFFLINE | RECALL_ON_OPEN | RECALL_ON_DATA_ACCESS) != 0
}

/// macOS 14 and later: the file is "dataless", its contents only in iCloud (`SF_DATALESS`).
pub fn cloud_only_mac(flags: u32) -> bool {
    const SF_DATALESS: u32 = 0x4000_0000;
    flags & SF_DATALESS != 0
}

/// Before macOS 14, a file evicted to iCloud was replaced by a hidden stand-in named
/// `.<name>.icloud`.
pub fn is_icloud_stub_name(name: &str) -> bool {
    name.strip_prefix('.')
        .and_then(|n| n.strip_suffix(".icloud"))
        .is_some_and(|inner| !inner.is_empty())
}
