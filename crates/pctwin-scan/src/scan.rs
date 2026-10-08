use std::ffi::OsStr;
use std::path::{Path, PathBuf};

use pctwin_record::{
    Inclusion, Item, ItemId, ItemKind, ItemName, ItemPath, LaptopId, LeftOutReason, ManagedBy,
    Owner, Place, Portability,
};
use serde::Serialize;

use crate::measure::{is_cloud_only, is_icloud_stub_name, measure};

/// What scanning one folder found.
#[derive(Debug, Clone, Default)]
pub struct Scan {
    /// Every file and folder, including what is left out (with the reason).
    pub items: Vec<Item>,
    pub counts: Counts,
    /// Links and junctions: counted, never followed, not yet recorded as items.
    pub links: u64,
    /// Folders and files that could not be read.
    pub unreadable: Vec<PathBuf>,
    /// Size and modified time of each file, to tell later what changed.
    pub stamps: Vec<(ItemId, Stamp)>,
}

/// What a later scan compares to tell whether a file changed (as Syncthing and rsync do).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, serde::Deserialize)]
pub struct Stamp {
    pub size: u64,
    /// Nanoseconds since 1970, when the system gives it.
    pub modified_ns: Option<i64>,
}

impl Stamp {
    fn of(meta: &std::fs::Metadata) -> Self {
        let modified_ns =
            meta.modified()
                .ok()
                .map(|t| match t.duration_since(std::time::UNIX_EPOCH) {
                    Ok(d) => i64::try_from(d.as_nanos()).unwrap_or(i64::MAX),
                    Err(e) => -i64::try_from(e.duration().as_nanos()).unwrap_or(i64::MAX),
                });
        Self {
            size: meta.len(),
            modified_ns,
        }
    }
}

/// What matters to people, counted at scan time so the new laptop can be checked after the move.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, serde::Deserialize)]
pub struct Counts {
    pub photos: u64,
    pub videos: u64,
    pub music: u64,
    pub documents: u64,
    pub other: u64,
}

/// Scans `root` (one special folder of `owner`, at `place`) into move-record items. Only folder
/// listings and file details are read: no file is opened, nothing in the cloud is downloaded and
/// links are never followed.
pub fn scan_folder(laptop: &LaptopId, owner: &Owner, place: &Place, root: &Path) -> Scan {
    let mut scan = Scan::default();
    // Each pending folder carries its path inside `root` and, below the top, its item's index.
    let mut pending: Vec<(PathBuf, Vec<ItemName>, Option<usize>)> =
        vec![(root.to_path_buf(), Vec::new(), None)];
    while let Some((dir, parts, item)) = pending.pop() {
        let Ok(entries) = std::fs::read_dir(&dir) else {
            if let Some(i) = item {
                scan.items[i].inclusion = left_out(LeftOutReason::Unreadable);
            }
            scan.unreadable.push(dir);
            continue;
        };
        for entry in entries {
            let Ok(entry) = entry else {
                scan.unreadable.push(dir.clone());
                continue;
            };
            let path = entry.path();
            let file_name = entry.file_name();
            let mut item_parts = parts.clone();
            item_parts.push(item_name(&file_name));
            let Ok(item_path) = ItemPath::new(item_parts.clone()) else {
                scan.unreadable.push(path);
                continue;
            };
            let name = file_name.to_string_lossy();
            let make = |kind, size_bytes, inclusion| Item {
                id: ItemId::derive(laptop, owner, place, &item_path),
                kind,
                owner: owner.clone(),
                place: place.clone(),
                path: item_path.clone(),
                size_bytes,
                portability: Portability::Portable,
                managed_by: ManagedBy::Personal,
                download_bytes: None,
                inclusion,
            };
            let (Ok(kind), Ok(meta)) = (entry.file_type(), entry.metadata()) else {
                scan.items
                    .push(make(ItemKind::File, 0, left_out(LeftOutReason::Unreadable)));
                scan.unreadable.push(path);
                continue;
            };
            if is_clutter(&name) {
                // The system's own clutter: recorded so the report can account for it, never
                // moved, never counted, and never looked inside.
                let (kind, size) = if kind.is_dir() {
                    (ItemKind::Folder, 0)
                } else {
                    (ItemKind::File, meta.len())
                };
                scan.items
                    .push(make(kind, size, left_out(LeftOutReason::System)));
            } else if kind.is_symlink() {
                scan.links += 1;
            } else if kind.is_dir() {
                if let Some(package) = package_kind(&name) {
                    let size = measure(&path).bytes;
                    scan.items
                        .push(make(package.item_kind(), size, Inclusion::Included));
                    match package {
                        Package::PhotosLibrary => count_library(&path, &mut scan.counts),
                        Package::Document => scan.counts.documents += 1,
                        Package::App | Package::Other => {}
                    }
                } else {
                    scan.items
                        .push(make(ItemKind::Folder, 0, Inclusion::Included));
                    pending.push((path, item_parts, Some(scan.items.len() - 1)));
                }
            } else {
                let stub = is_icloud_stub_name(&name);
                let cloud_only = stub || is_cloud_only(&meta);
                let category_name = if stub {
                    name.trim_start_matches('.').trim_end_matches(".icloud")
                } else {
                    &name
                };
                count(category(category_name), &mut scan.counts);
                let inclusion = if cloud_only {
                    // Its contents are in the cloud, not on this laptop; the scan never fetches it.
                    left_out(LeftOutReason::CloudOnly)
                } else {
                    Inclusion::Included
                };
                let size = if stub { 0 } else { meta.len() };
                let item = make(ItemKind::File, size, inclusion);
                if !stub {
                    scan.stamps.push((item.id, Stamp::of(&meta)));
                }
                scan.items.push(item);
            }
        }
    }
    scan
}

/// Files and folders the system makes for itself: folder thumbnails and settings, the recycle bin
/// and Trash, search and version indexes, Office's temporary lock files and the `._` files macOS
/// leaves on other drives. Backup and migration tools skip these too.
fn is_clutter(name: &str) -> bool {
    let lower = name.to_lowercase();
    matches!(
        lower.as_str(),
        "thumbs.db"
            | "ehthumbs.db"
            | "desktop.ini"
            | ".ds_store"
            | "$recycle.bin"
            | "system volume information"
            | ".trash"
            | ".trashes"
            | ".spotlight-v100"
            | ".fseventsd"
            | ".temporaryitems"
            | ".documentrevisions-v100"
    ) || lower.starts_with("~$")
        || lower.starts_with("._")
}

fn left_out(reason: LeftOutReason) -> Inclusion {
    Inclusion::LeftOut { reason }
}

/// A name exactly as the system gave it.
fn item_name(name: &OsStr) -> ItemName {
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
    #[cfg(not(any(windows, unix)))]
    {
        ItemName::from_text(&name.to_string_lossy())
    }
}

/// macOS folders that act as one file. macOS also decides this from its own type database, which
/// is not read here; these are the common kinds.
#[derive(Debug, Clone, Copy)]
enum Package {
    App,
    Document,
    PhotosLibrary,
    Other,
}

impl Package {
    fn item_kind(self) -> ItemKind {
        match self {
            Package::App => ItemKind::App,
            _ => ItemKind::Folder,
        }
    }
}

fn package_kind(name: &str) -> Option<Package> {
    let ext = extension(name)?;
    Some(match ext.as_str() {
        "app" => Package::App,
        "pages" | "numbers" | "key" | "rtfd" => Package::Document,
        "photoslibrary" => Package::PhotosLibrary,
        "bundle" | "framework" | "plugin" | "kext" | "musiclibrary" | "imovielibrary"
        | "fcpbundle" | "logicx" | "band" | "photoboothlibrary" | "aplibrary" => Package::Other,
        _ => return None,
    })
}

fn extension(name: &str) -> Option<String> {
    let (stem, ext) = name.rsplit_once('.')?;
    (!stem.is_empty() && !ext.is_empty()).then(|| ext.to_lowercase())
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Category {
    Photo,
    Video,
    Music,
    Document,
    Other,
}

fn category(name: &str) -> Category {
    let Some(ext) = extension(name) else {
        return Category::Other;
    };
    match ext.as_str() {
        "jpg" | "jpeg" | "jpe" | "png" | "gif" | "bmp" | "tif" | "tiff" | "heic" | "heif"
        | "webp" | "avif" | "jxl" | "dng" | "cr2" | "cr3" | "crw" | "nef" | "nrw" | "arw"
        | "srf" | "sr2" | "raf" | "orf" | "rw2" | "pef" | "srw" | "x3f" | "3fr" | "erf" | "kdc"
        | "dcr" | "mos" | "mef" | "iiq" => Category::Photo,
        "mp4" | "m4v" | "mov" | "avi" | "mkv" | "wmv" | "webm" | "3gp" | "mts" | "m2ts" | "mpg"
        | "mpeg" | "flv" => Category::Video,
        "mp3" | "m4a" | "aac" | "flac" | "wav" | "ogg" | "oga" | "opus" | "wma" | "aiff"
        | "aif" | "alac" | "mid" | "midi" => Category::Music,
        "pdf" | "doc" | "docx" | "xls" | "xlsx" | "ppt" | "pptx" | "odt" | "ods" | "odp"
        | "rtf" | "txt" | "md" | "csv" | "pages" | "numbers" | "key" | "epub" => Category::Document,
        _ => Category::Other,
    }
}

fn count(category: Category, counts: &mut Counts) {
    match category {
        Category::Photo => counts.photos += 1,
        Category::Video => counts.videos += 1,
        Category::Music => counts.music += 1,
        Category::Document => counts.documents += 1,
        Category::Other => counts.other += 1,
    }
}

/// Counts the photos and videos in a Photos library from its originals (`originals` since
/// Photos 5, `Masters` before), never its thumbnails or previews, and without opening its
/// database.
fn count_library(library: &Path, counts: &mut Counts) {
    let mut pending: Vec<PathBuf> = ["originals", "Masters"]
        .iter()
        .map(|d| library.join(d))
        .collect();
    while let Some(dir) = pending.pop() {
        let Ok(entries) = std::fs::read_dir(&dir) else {
            continue;
        };
        for entry in entries.flatten() {
            let Ok(kind) = entry.file_type() else {
                continue;
            };
            if kind.is_dir() {
                pending.push(entry.path());
            } else if kind.is_file() {
                match category(&entry.file_name().to_string_lossy()) {
                    Category::Photo => counts.photos += 1,
                    Category::Video => counts.videos += 1,
                    _ => {}
                }
            }
        }
    }
}
