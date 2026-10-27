use std::ffi::OsStr;
use std::path::{Path, PathBuf};

use pctwin_record::{
    Inclusion, Item, ItemId, ItemKind, ItemName, ItemPath, LaptopId, LeftOutReason, ManagedBy,
    Owner, Place, Portability,
};
use std::collections::HashMap;
use std::sync::OnceLock;

use pctwin_catalogue::{Catalogue, Row};
use serde::{Deserialize, Serialize};

use crate::measure::{is_cloud_only, is_icloud_stub_name, measure};
use crate::rules::{Kind, Rules};

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
    pub(crate) fn of(meta: &std::fs::Metadata) -> Self {
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
    scan_managed_folder(laptop, owner, place, root, ManagedBy::Personal)
}

/// As [`scan_folder`], for a folder `managed` by its owner or by an employer or school. A work
/// or school folder's items are marked as theirs and left out by default, with that plain
/// reason; the person can still choose to include them when reviewing the plan.
pub fn scan_managed_folder(
    laptop: &LaptopId,
    owner: &Owner,
    place: &Place,
    root: &Path,
    managed: ManagedBy,
) -> Scan {
    scan_managed_folder_with(laptop, owner, place, root, managed, &Rules::shipped())
}

/// As [`scan_managed_folder`], with the folders the rules file names anywhere (`.git`,
/// `node_modules`) taken from `rules`.
pub(crate) fn scan_managed_folder_with(
    laptop: &LaptopId,
    owner: &Owner,
    place: &Place,
    root: &Path,
    managed: ManagedBy,
    rules: &Rules,
) -> Scan {
    let mut scan = scan_folder_items(laptop, owner, place, root, rules);
    if managed == ManagedBy::Organisation {
        for item in &mut scan.items {
            item.managed_by = ManagedBy::Organisation;
            if item.inclusion == Inclusion::Included {
                item.inclusion = left_out(LeftOutReason::WorkManaged);
            }
        }
    }
    scan
}

fn scan_folder_items(
    laptop: &LaptopId,
    owner: &Owner,
    place: &Place,
    root: &Path,
    rules: &Rules,
) -> Scan {
    let mut scan = Scan::default();
    // Each pending folder carries its path inside `root`, below the top its item's index, and
    // whether it is inside a folder that moves as one whole (a package, a project's history):
    // there nothing is looked for by name and nothing is counted by kind (the whole was counted
    // as itself).
    let mut pending: Vec<(PathBuf, Vec<ItemName>, Option<usize>, bool)> =
        vec![(root.to_path_buf(), Vec::new(), None, false)];
    while let Some((dir, parts, item, whole)) = pending.pop() {
        let Ok(entries) = std::fs::read_dir(crate::sweep::for_listing(&dir)) else {
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
            let path = dir.join(entry.file_name());
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
                modified_ns: None,
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
                let row = (!whole).then(|| rules.anywhere(&name)).flatten();
                let package = (!whole).then(|| package_kind(&name)).flatten();
                if row.is_some_and(|r| r.kind == Kind::Rebuildable) {
                    // Made again on the new laptop: one item with its whole size, left out.
                    let size = measure(&path).bytes;
                    let mut it = make(ItemKind::Folder, size, left_out(LeftOutReason::Rebuildable));
                    it.portability = Portability::Rebuildable;
                    scan.items.push(it);
                } else if package.is_some() || row.is_some_and(|r| r.kind == Kind::KeepWhole) {
                    // Moves as one whole, as its files, below one package item.
                    match package {
                        Some(Package::PhotosLibrary) => count_library(&path, &mut scan.counts),
                        Some(Package::Document) => scan.counts.documents += 1,
                        _ => {}
                    }
                    scan.items
                        .push(make(ItemKind::Package, 0, Inclusion::Included));
                    pending.push((path, item_parts, Some(scan.items.len() - 1), true));
                } else {
                    scan.items
                        .push(make(ItemKind::Folder, 0, Inclusion::Included));
                    pending.push((path, item_parts, Some(scan.items.len() - 1), whole));
                }
            } else {
                let stub = is_icloud_stub_name(&name);
                let cloud_only = stub || is_cloud_only(&meta);
                let category_name = if stub {
                    name.trim_start_matches('.').trim_end_matches(".icloud")
                } else {
                    &name
                };
                if !whole {
                    count(category(category_name), &mut scan.counts);
                }
                let inclusion = if cloud_only {
                    // Its contents are in the cloud, not on this laptop; the scan never fetches it.
                    left_out(LeftOutReason::CloudOnly)
                } else {
                    Inclusion::Included
                };
                let size = if stub { 0 } else { meta.len() };
                let mut item = make(ItemKind::File, size, inclusion);
                if !stub {
                    let stamp = Stamp::of(&meta);
                    // Carried in the record too, so the move can tell a file changed since the scan.
                    item.modified_ns = stamp.modified_ns;
                    scan.stamps.push((item.id, stamp));
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
pub(crate) fn is_clutter(name: &str) -> bool {
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

pub(crate) fn left_out(reason: LeftOutReason) -> Inclusion {
    Inclusion::LeftOut { reason }
}

/// A name exactly as the system gave it.
pub(crate) fn item_name(name: &OsStr) -> ItemName {
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
pub(crate) enum Package {
    App,
    Document,
    PhotosLibrary,
    Other,
}

pub(crate) fn package_kind(name: &str) -> Option<Package> {
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

/// What kind of file a name says it is, by its extension, as the scan counts it ([`Counts`]). The
/// after-move check counts the same way, so the two can be held side by side. Which extensions
/// are which kind is data: the `file-categories` catalogue ([`CategoryRow`]); a file whose kind
/// no row names is [`Category::Other`], counted and moved like any other and ranked after the
/// named kinds when the most important things go first (the general rule: never dropped).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum Category {
    Photo,
    Video,
    Music,
    Document,
    Other,
}

/// One row of the `file-categories` catalogue: a kind of file and its extensions.
#[derive(Debug, Clone, PartialEq, Eq, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct CategoryRow {
    pub category: Category,
    /// Lower case, without the dot.
    pub extensions: Vec<String>,
}

impl Row for CategoryRow {
    const CATALOGUE: &'static str = "file-categories";
    const FIELDS: &'static [&'static str] = &["category", "extensions"];
    fn check(&self) -> Result<(), String> {
        if self.category == Category::Other {
            return Err("`other` is for what no row names".into());
        }
        let ok = self.extensions.len() <= 1000
            && self.extensions.iter().all(|e| {
                !e.is_empty()
                    && e.len() <= 16
                    && e.bytes()
                        .all(|b| b.is_ascii_lowercase() || b.is_ascii_digit())
            });
        if ok {
            Ok(())
        } else {
            Err("extensions are lower-case letters and digits".into())
        }
    }
}

/// The extensions of a catalogue, ready to look up: the first row naming an extension wins.
#[derive(Debug, Clone, Default)]
pub struct Categories(HashMap<String, Category>);

impl Categories {
    pub fn from_catalogue(rows: &Catalogue<CategoryRow>) -> Self {
        let mut map = HashMap::new();
        for row in rows.rows() {
            for e in &row.extensions {
                map.entry(e.clone()).or_insert(row.category);
            }
        }
        Self(map)
    }

    /// The [`Category`] of a file called `name`.
    pub fn of(&self, name: &str) -> Category {
        extension(name)
            .and_then(|e| self.0.get(&e).copied())
            .unwrap_or(Category::Other)
    }
}

/// The rows compiled into this build.
pub fn shipped_categories() -> &'static Catalogue<CategoryRow> {
    static ROWS: OnceLock<Catalogue<CategoryRow>> = OnceLock::new();
    ROWS.get_or_init(|| Catalogue::shipped(include_str!("../catalogues/file-categories.json")))
}

/// The [`Category`] of a file called `name`, by the shipped rows.
pub fn category(name: &str) -> Category {
    static MAP: OnceLock<Categories> = OnceLock::new();
    MAP.get_or_init(|| Categories::from_catalogue(shipped_categories()))
        .of(name)
}

pub(crate) fn count(category: Category, counts: &mut Counts) {
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
pub(crate) fn count_library(library: &Path, counts: &mut Counts) {
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
