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
        let Ok(entries) = std::fs::read_dir(crate::sweep::for_listing(&dir)) else {
            m.unreadable.push(dir);
            continue;
        };
        for entry in entries {
            let Ok(entry) = entry else {
                m.unreadable.push(dir.clone());
                continue;
            };
            let path = dir.join(entry.file_name());
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

// Whether a file is stored online only: asked in one place for every reader of the person's files.
pub use pctwin_types::cloud::{
    cloud_only_mac, cloud_only_windows, is_cloud_only, is_icloud_stub_name,
};
