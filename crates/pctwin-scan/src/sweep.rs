//! The drive-wide sweep for the person moving (Engineering Plan 9.16; System Designs S13,
//! "Finding everything on the old laptop"). Only reads, and never opens a person's files.
//!
//! In order: the person's special folders; then every volume, breadth first to depth 3, then deep
//! into the places that are the mover's or shared. System, app, junk and rebuildable places are
//! counted, not listed. Other people's homes are counted where the system lets this person see
//! them, never listed by name. Removable drives, network folders and disc images are listed, not
//! walked; links and junctions are never followed; a file reached by several hard links counts
//! once; online-only files are judged from their attributes and never downloaded; a folder that
//! can't be read is listed with the reason, never shown as empty.
//!
//! Every volume ends with a coverage line (how full the drive says it is against what was seen,
//! with the system's own space named) and the blind spots keep their reasons, so the top line is
//! arithmetic plus a named list: "Everything on this laptop is accounted for" only when the sweep
//! finished and nothing is left unexplained.

use std::collections::{BTreeMap, BTreeSet, HashSet, VecDeque};
use std::ffi::OsString;
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::mpsc::channel;
use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant};

use pctwin_record::{
    FolderRole, Inclusion, Item, ItemId, ItemKind, ItemName, ItemPath, LaptopId, LeftOutReason,
    ManagedBy, Owner, Place, Portability, Record, Storage,
};
use serde::{Deserialize, Serialize};

use crate::access::AccessFact;
use crate::acl::{AccessReader, UnixMode};
use crate::census::{Census, PersonKind};
use crate::folders::{FolderLookup, is_within, same_place};
use crate::measure::{is_cloud_only, is_icloud_stub_name};
use crate::rules::{Kind, Rules};
use crate::scan::{
    Counts, Package, Stamp, category, count, count_library, is_clutter, item_name, left_out,
    package_kind, scan_managed_folder,
};
use crate::volumes::{Volume, VolumeKind};
use crate::whose::{Facts, HomeOf, People, Placement, Rule, Whose, place as place_by_ladder};

/// The system's own folders in the profiles folder: counted, never anyone's.
const SYSTEM_PROFILES: [&str; 3] = ["Default", "Default User", "All Users"];

/// How far the first, breadth-first pass goes below each volume's root.
pub const SHALLOW_DEPTH: u16 = 3;

/// A gap between how full a drive says it is and what was seen is flagged only when it is more
/// than 5 GB and more than a tenth of what the drive says is used.
pub const UNEXPLAINED_FLOOR: u64 = 5 * 1024 * 1024 * 1024;

/// Windows lists no link count, so finding a file with several names takes a handle per file
/// (attributes only). That is done for files of 1 MiB or more, where counting twice would move the
/// coverage line; smaller files with several names may be counted twice (macOS and Linux check
/// every file, from the listing).
pub const HARD_LINK_CHECK_MIN: u64 = 1024 * 1024;

/// Limits that keep the sweep honest about time and memory.
#[derive(Debug, Clone)]
pub struct SweepLimits {
    /// Save the frontier after this many entries...
    pub checkpoint_entries: u64,
    /// ...or this long, whichever comes first.
    pub checkpoint_every: Duration,
    /// The most walkers on one SSD.
    pub most_walkers: usize,
    /// A place with more items than this is counted as one item (the record that lists any
    /// number of items comes in a later version).
    pub place_items: usize,
}

impl Default for SweepLimits {
    fn default() -> Self {
        Self {
            checkpoint_entries: 10_000,
            checkpoint_every: Duration::from_secs(5),
            most_walkers: 4,
            place_items: 250_000,
        }
    }
}

/// What the sweep is told about the laptop.
#[derive(Debug, Clone)]
pub struct SweepFacts {
    pub laptop: LaptopId,
    pub census: Census,
    pub volumes: Vec<Volume>,
    /// The mover's special folders, as the system reports them.
    pub known_folders: Vec<FolderLookup>,
    /// Places the mover's own records mention (recent files, pinned folders), to find drives
    /// that are not connected.
    pub remembered: Vec<PathBuf>,
    pub rules: Rules,
}

/// What happens to a place.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum Fate {
    /// In the move record: it moves unless the person unticks it.
    Moves,
    /// App data this version does not move yet: listed with its size so nobody wipes the old
    /// laptop believing it moved.
    FoundNotMovedInThisVersion,
    /// The system's, an app's or junk: counted only.
    Counted,
    /// Another person's, or from an old account: it moves only with an administrator's approval.
    NotYours,
}

/// One place the sweep found.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct FoundPlace {
    /// The volume's mount (`D:`, `/`).
    pub volume: String,
    /// Where it is. Never given for another person's things.
    pub path: Option<String>,
    pub placement: Placement,
    pub fate: Fate,
    pub files: u64,
    pub bytes: u64,
}

/// Why a place could not be looked at.
#[derive(Debug, Clone, PartialEq, Eq, Hash, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(tag = "reason", rename_all = "kebab-case")]
pub enum BlindReason {
    /// Another person, who was not approved: their things are not looked at.
    AnotherPerson { account_id: Option<String> },
    /// Left by an old account, and this person may not open it.
    FromOldAccount,
    /// This person's permissions don't allow it.
    NoPermission,
    /// A Mac privacy setting refused it (System Settings, Privacy and Security).
    MacPrivacy,
    /// The drive is locked (BitLocker, FileVault).
    LockedDrive,
    /// A drive this person used is not connected.
    NotConnected,
    /// Another installed system: read only through rescue, with permission.
    OtherSystem,
    /// Who could open it could not be read, so it was not placed.
    AccessNotReadable,
    /// Another program holds it.
    InUse,
    /// It changed while the sweep was looking (renamed, moved or removed): what is there now
    /// was not seen, so it is never counted as looked at. Scanning again sees it.
    ChangedDuringScan,
    /// The drive says this much more is used than the sweep could see.
    Unexplained { bytes: u64 },
    /// The system refused for another reason (the system's error number).
    Refused { code: Option<i32> },
}

/// A place the sweep could not look at, and why.
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
pub struct BlindSpot {
    pub volume: Option<String>,
    /// Where it is. Never given for another person's things.
    pub path: Option<String>,
    #[serde(flatten)]
    pub reason: BlindReason,
    /// How many places this line stands for: blind spots that can't be told apart (another
    /// person's folders, which are never named) are one line, but every place still counts.
    #[serde(default = "one")]
    pub count: u64,
}

fn one() -> u64 {
    1
}

/// Adds a blind spot, counting it when the same line is already there.
fn add_blind(spots: &mut BTreeMap<BlindKey, u64>, spot: BlindSpot) {
    *spots
        .entry(BlindKey {
            volume: spot.volume,
            path: spot.path,
            reason: spot.reason,
        })
        .or_insert(0) += spot.count.max(1);
}

/// A blind spot without its count, as the sweep keeps them while it runs.
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
struct BlindKey {
    volume: Option<String>,
    path: Option<String>,
    reason: BlindReason,
}

/// Saves a map as a list of pairs (JSON maps take only text keys).
mod pairs {
    use std::collections::BTreeMap;

    use serde::{Deserialize, Deserializer, Serialize, Serializer};

    pub fn serialize<K: Serialize, V: Serialize, S: Serializer>(
        map: &BTreeMap<K, V>,
        s: S,
    ) -> Result<S::Ok, S::Error> {
        s.collect_seq(map.iter())
    }

    pub fn deserialize<'de, K, V, D>(d: D) -> Result<BTreeMap<K, V>, D::Error>
    where
        K: Deserialize<'de> + Ord,
        V: Deserialize<'de>,
        D: Deserializer<'de>,
    {
        Ok(Vec::<(K, V)>::deserialize(d)?.into_iter().collect())
    }
}

/// A named kind of space on a drive.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum Space {
    /// The person's own things and shared things.
    People,
    /// The operating system.
    TheSystem,
    /// Installed apps.
    Apps,
    /// What apps keep for one person.
    AppData,
    /// Caches, temporary files and bins.
    Junk,
    /// Things that can be made again.
    Rebuildable,
    /// Other people's things and things from old accounts, as far as they could be counted.
    OtherPeople,
    /// Parts of the system no one but the system may look into (restore points, snapshots).
    SystemKeepsToItself,
}

/// One line of a volume's coverage.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct SpaceLine {
    pub space: Space,
    /// `None` when the size could not be seen.
    pub bytes: Option<u64>,
}

/// How full one volume is, against what the sweep saw there.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct VolumeReport {
    pub mount: String,
    pub name: String,
    pub kind: VolumeKind,
    pub file_system: String,
    /// The drive the running system started from.
    pub is_system: bool,
    pub walked: bool,
    pub walkers: usize,
    pub used_bytes: u64,
    pub seen_bytes: u64,
    pub lines: Vec<SpaceLine>,
    pub unexplained_bytes: u64,
    /// The gap is big enough to be a blind spot.
    pub flagged: bool,
    /// Shares its space with this volume (one file system mounted in several places); its gap
    /// is judged there.
    pub same_space_as: Option<String>,
}

/// The line at the top of the person's summary.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "line", rename_all = "kebab-case")]
pub enum TopLine {
    /// "Everything on this laptop is accounted for."
    Everything,
    /// "Accounted for except 3 places."
    AccountedForExcept { places: usize },
    /// "Couldn't check: the scan did not finish."
    DidNotFinish,
}

impl TopLine {
    /// The words a person reads.
    pub fn words(&self) -> String {
        match self {
            TopLine::Everything => "Everything on this laptop is accounted for".into(),
            TopLine::AccountedForExcept { places: 1 } => "Accounted for except 1 place".into(),
            TopLine::AccountedForExcept { places } => {
                format!("Accounted for except {places} places")
            }
            TopLine::DidNotFinish => "Couldn't check: the scan did not finish".into(),
        }
    }
}

/// Everything the sweep found.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Sweep {
    pub census: Census,
    pub volumes: Vec<VolumeReport>,
    pub places: Vec<FoundPlace>,
    pub blind_spots: Vec<BlindSpot>,
    /// The mover's things and shared things, as move-record items.
    pub record: Record,
    /// For every item in the record, the place (and so the rule) that placed it.
    pub placed_by: Vec<(ItemId, usize)>,
    pub counts: Counts,
    /// Links and junctions: counted, never followed.
    pub links: u64,
    pub finished: bool,
    pub top_line: TopLine,
    /// Where to carry on from, when the sweep was stopped.
    pub frontier: Option<Frontier>,
}

impl Sweep {
    /// The rule that placed an item.
    pub fn placement_of(&self, id: &ItemId) -> Option<&Placement> {
        self.placed_by
            .iter()
            .find(|(i, _)| i == id)
            .and_then(|(_, p)| self.places.get(*p))
            .map(|p| &p.placement)
    }
}

/// Where a stopped sweep carries on from: the folders still to list and what was found so far.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Frontier {
    state: State,
    in_flight: Vec<Job>,
}

impl Frontier {
    /// Folders still to list.
    pub fn pending(&self) -> usize {
        pending(&self.state) + self.in_flight.len()
    }

    /// Reads a frontier saved with [`FrontierView::to_json`].
    pub fn from_json(bytes: &[u8]) -> Option<Self> {
        serde_json::from_slice(bytes).ok()
    }
}

fn pending(state: &State) -> usize {
    state.shallow.iter().map(VecDeque::len).sum::<usize>()
        + state.deep.iter().map(VecDeque::len).sum::<usize>()
}

/// The frontier as it stands, lent to the checkpoint without copying it. Saving it is the
/// caller's choice ([`FrontierView::save`] or [`FrontierView::to_json`]).
pub struct FrontierView<'a> {
    state: &'a State,
    in_flight: Vec<Job>,
}

impl FrontierView<'_> {
    /// Folders still to list.
    pub fn pending(&self) -> usize {
        pending(self.state) + self.in_flight.len()
    }

    /// An owned copy, to resume from.
    pub fn save(&self) -> Frontier {
        Frontier {
            state: self.state.clone(),
            in_flight: self.in_flight.clone(),
        }
    }

    /// The frontier in its saved form.
    pub fn to_json(&self) -> Vec<u8> {
        #[derive(Serialize)]
        struct Saved<'b> {
            state: &'b State,
            in_flight: &'b [Job],
        }
        serde_json::to_vec(&Saved {
            state: self.state,
            in_flight: &self.in_flight,
        })
        .unwrap_or_default()
    }
}

// ------------------------------------------------------------------------------------------
// The sweep's working state, saved whole in a frontier.

#[derive(Debug, Clone, Serialize, Deserialize)]
enum Zone {
    /// Outside everyone's home, not yet placed. Inside a rules-file place that holds a home
    /// (`/var` holding `/var/home`), what is not a home stays in that place.
    Explore { inside: Option<usize> },
    /// The folder that holds everyone's homes.
    Profiles,
    /// Listed item by item into the record.
    Itemise { base: usize, place: usize },
    /// A person's app data: each program's folder is one counted place.
    AppData { home: usize, place: usize },
    /// Counted only (names never kept).
    Count { place: usize },
    /// Counted into one record item.
    Collapse { item: usize, place: usize },
    /// Another person's home, or one from an old account: counted in one go by a walker, so no
    /// name below it is ever kept, queued or saved. A stopped count starts again from its root.
    Whole { place: usize },
    /// An older copy of Windows (`Windows.old`): the system's, except its `Users` folder.
    OldSystem { place: usize },
    /// The homes inside an older copy of Windows: each is placed by its owner, like a folder
    /// among today's homes that no account explains.
    OldProfiles { place: usize },
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct Job {
    volume: usize,
    rel: Vec<ItemName>,
    depth: u16,
    zone: Zone,
    /// The record item for this folder, to mark it if it can't be read.
    item: Option<usize>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct PlaceMeta {
    /// For places listed into the record.
    record_place: Option<Place>,
    owner: Option<Owner>,
    space: Space,
    private: bool,
    items: usize,
    collapsed: Option<usize>,
    /// The folder item the place starts at, when it starts at a folder.
    root_item: Option<usize>,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
struct VolumeSeen {
    seen: u64,
    lines: BTreeMap<Space, (u64, bool)>,
    walked: bool,
    walkers: usize,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct State {
    census: Census,
    record: Record,
    places: Vec<FoundPlace>,
    meta: Vec<PlaceMeta>,
    item_place: Vec<usize>,
    removed: BTreeSet<usize>,
    #[serde(with = "pairs")]
    blind: BTreeMap<BlindKey, u64>,
    counts: Counts,
    links: u64,
    seen_keys: HashSet<(u64, u64)>,
    vols: Vec<VolumeSeen>,
    known_done: Vec<usize>,
    known_started: bool,
    /// Folders still to list, per volume: the breadth-first pass, then the deep one.
    shallow: Vec<VecDeque<Job>>,
    deep: Vec<VecDeque<Job>>,
    stamps: Vec<(ItemId, Stamp)>,
    /// Places for loose files, by volume, folder and placement.
    loose: Vec<((usize, String, Placement), usize)>,
}

// ------------------------------------------------------------------------------------------
// Listing one folder (done by the walkers, in parallel).

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum EntryKind {
    File,
    Dir,
    Link,
}

#[derive(Debug)]
struct Entry {
    name: OsString,
    kind: Option<EntryKind>,
    len: u64,
    cloud_only: bool,
    stamp: Option<Stamp>,
    unix: Option<UnixMode>,
    dev: Option<u64>,
    /// A file reached by several hard links: its identity, to count it once.
    key: Option<(u64, u64)>,
    /// What the file takes on the disk (macOS and Linux: its blocks), for the coverage line; a
    /// sparse file takes less than its length.
    alloc: Option<u64>,
}

/// What one walker found counting a whole place in one go: totals only, never a name.
#[derive(Debug, Default)]
struct Whole {
    files: u64,
    bytes: u64,
    seen: u64,
    /// The place itself could not be listed.
    root_refused: Option<Refusal>,
    /// Something inside it could not be listed.
    inside_refused: bool,
    /// The owners met inside (Unix user IDs), for the census.
    owners: BTreeSet<u32>,
}

/// What a walker hands back.
enum Listing {
    Dir(Result<Vec<Entry>, Refusal>),
    Whole(Whole),
}

/// Counts everything below `root` without keeping a name: links are not followed and other
/// file systems mounted inside are not entered. It always runs to the end, even when the sweep
/// is asked to stop: a half-done count could only be resumed by keeping the names inside, so a
/// stop waits for it instead (one home at most per walker).
fn count_whole(root: &Path) -> Whole {
    let mut w = Whole::default();
    let root_dev = dev_of(root);
    let mut pending = vec![for_listing(root)];
    let mut first = true;
    while let Some(dir) = pending.pop() {
        let reader = match std::fs::read_dir(&dir) {
            Ok(r) => r,
            Err(e) => {
                if first {
                    w.root_refused = Some(Refusal::of(&e));
                } else {
                    w.inside_refused = true;
                }
                first = false;
                continue;
            }
        };
        first = false;
        for entry in reader {
            let Ok(entry) = entry else {
                w.inside_refused = true;
                break;
            };
            let (Ok(kind), Ok(meta)) = (entry.file_type(), entry.metadata()) else {
                w.inside_refused = true;
                continue;
            };
            let (unix, dev, _) = unix_facts(&meta);
            if let Some(u) = unix {
                w.owners.insert(u.uid);
            }
            if kind.is_symlink() {
                continue;
            }
            if kind.is_dir() {
                if dev.is_some() && dev != root_dev {
                    continue;
                }
                pending.push(entry.path());
            } else if !is_cloud_only(&meta) {
                w.files += 1;
                w.bytes += meta.len();
                w.seen += allocated(&meta).unwrap_or(meta.len());
            }
        }
    }
    w
}

#[cfg(unix)]
fn allocated(meta: &std::fs::Metadata) -> Option<u64> {
    use std::os::unix::fs::MetadataExt;
    Some(meta.blocks().saturating_mul(512))
}

#[cfg(not(unix))]
fn allocated(_meta: &std::fs::Metadata) -> Option<u64> {
    None
}

/// Why a folder could not be listed.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Refusal {
    NoPermission,
    MacPrivacy,
    Locked,
    InUse,
    Gone,
    Other(Option<i32>),
}

impl Refusal {
    fn of(e: &std::io::Error) -> Self {
        let code = e.raw_os_error();
        if cfg!(windows) {
            match code {
                Some(5) => Refusal::NoPermission,
                Some(2) | Some(3) => Refusal::Gone,
                Some(32) | Some(33) => Refusal::InUse,
                // FVE_E_LOCKED_VOLUME: a BitLocker drive that is locked.
                Some(-2_144_272_384) => Refusal::Locked,
                _ => Refusal::Other(code),
            }
        } else {
            match code {
                // macOS answers "operation not permitted" (EPERM) when a privacy setting
                // refuses, and "permission denied" (EACCES) for ordinary permissions.
                Some(1) if cfg!(target_os = "macos") => Refusal::MacPrivacy,
                Some(1) | Some(13) => Refusal::NoPermission,
                Some(2) | Some(20) => Refusal::Gone,
                Some(16) | Some(26) => Refusal::InUse,
                _ => Refusal::Other(code),
            }
        }
    }

    fn reason(self) -> Option<BlindReason> {
        match self {
            Refusal::NoPermission => Some(BlindReason::NoPermission),
            Refusal::MacPrivacy => Some(BlindReason::MacPrivacy),
            Refusal::Locked => Some(BlindReason::LockedDrive),
            Refusal::InUse => Some(BlindReason::InUse),
            // Listed a moment ago and gone now: renamed or moved while the sweep looked, so
            // whatever is there now was not seen.
            Refusal::Gone => Some(BlindReason::ChangedDuringScan),
            Refusal::Other(code) => Some(BlindReason::Refused { code }),
        }
    }
}

/// Why a folder the system refused to list is a blind spot, from the system's error. A folder
/// that was listed and is gone a moment later changed during the scan (renamed or moved): what is
/// there now was not seen. On a Mac a privacy refusal ("operation not permitted") is told apart
/// from ordinary permissions ("permission denied").
pub fn blind_reason_for(error: &std::io::Error) -> Option<BlindReason> {
    Refusal::of(error).reason()
}

fn list_dir(path: &Path, want_keys: bool) -> Result<Vec<Entry>, Refusal> {
    let reader = std::fs::read_dir(path).map_err(|e| Refusal::of(&e))?;
    let mut out = Vec::new();
    for entry in reader {
        let entry = entry.map_err(|e| Refusal::of(&e))?;
        let name = entry.file_name();
        let (Ok(kind), Ok(meta)) = (entry.file_type(), entry.metadata()) else {
            out.push(Entry {
                name,
                kind: None,
                len: 0,
                cloud_only: false,
                stamp: None,
                unix: None,
                dev: None,
                key: None,
                alloc: None,
            });
            continue;
        };
        let kind = if kind.is_symlink() {
            EntryKind::Link
        } else if kind.is_dir() {
            EntryKind::Dir
        } else {
            EntryKind::File
        };
        let (unix, dev, mut key) = unix_facts(&meta);
        if want_keys
            && kind == EntryKind::File
            && key.is_none()
            && meta.len() >= HARD_LINK_CHECK_MIN
        {
            key = windows_key(&entry.path());
        }
        out.push(Entry {
            name,
            kind: Some(kind),
            len: meta.len(),
            cloud_only: kind == EntryKind::File && is_cloud_only(&meta),
            stamp: (kind == EntryKind::File).then(|| Stamp::of(&meta)),
            unix,
            dev,
            key,
            alloc: (kind == EntryKind::File)
                .then(|| allocated(&meta))
                .flatten(),
        });
    }
    Ok(out)
}

#[cfg(unix)]
fn unix_facts(meta: &std::fs::Metadata) -> (Option<UnixMode>, Option<u64>, Option<(u64, u64)>) {
    use std::os::unix::fs::MetadataExt;
    let key = (meta.nlink() > 1 && !meta.is_dir()).then(|| (meta.dev(), meta.ino()));
    (
        Some(UnixMode {
            uid: meta.uid(),
            gid: meta.gid(),
            mode: meta.mode(),
        }),
        Some(meta.dev()),
        key,
    )
}

#[cfg(not(unix))]
fn unix_facts(_meta: &std::fs::Metadata) -> (Option<UnixMode>, Option<u64>, Option<(u64, u64)>) {
    (None, None, None)
}

/// Windows: a file's identity when it has more than one name, read through a handle opened for
/// its attributes only (no data access, links not followed, online-only files never recalled).
#[cfg(windows)]
fn windows_key(path: &Path) -> Option<(u64, u64)> {
    use std::os::windows::fs::OpenOptionsExt;
    const FILE_READ_ATTRIBUTES: u32 = 0x80;
    const FILE_FLAG_BACKUP_SEMANTICS: u32 = 0x0200_0000;
    const FILE_FLAG_OPEN_REPARSE_POINT: u32 = 0x0020_0000;
    const FILE_FLAG_OPEN_NO_RECALL: u32 = 0x0010_0000;
    let file = std::fs::OpenOptions::new()
        .access_mode(FILE_READ_ATTRIBUTES)
        .share_mode(7)
        .custom_flags(
            FILE_FLAG_BACKUP_SEMANTICS | FILE_FLAG_OPEN_REPARSE_POINT | FILE_FLAG_OPEN_NO_RECALL,
        )
        .open(path)
        .ok()?;
    let info = winapi_util::file::information(&file).ok()?;
    (info.number_of_links() > 1).then(|| (info.volume_serial_number(), info.file_index()))
}

#[cfg(not(windows))]
fn windows_key(_path: &Path) -> Option<(u64, u64)> {
    None
}

// ------------------------------------------------------------------------------------------

struct Ladder<'a> {
    census: &'a Census,
}

impl People for Ladder<'_> {
    fn mover(&self) -> &str {
        &self.census.me
    }
    fn is_account(&self, id: &str) -> bool {
        self.census
            .by_account(id)
            .is_some_and(|r| r.kind != PersonKind::Leftover)
    }
    fn several(&self) -> bool {
        self.census.people_count() >= 2
    }
}

/// Sweeps this laptop for the signed-in person. `stop` is checked before each folder;
/// `checkpoint` is given the frontier every few seconds and when the sweep stops; `resume`
/// carries on from a saved frontier.
pub fn sweep_this_laptop(
    laptop: LaptopId,
    stop: &AtomicBool,
    checkpoint: &mut dyn FnMut(&FrontierView<'_>),
    resume: Option<Frontier>,
) -> Sweep {
    let census = crate::census::census_this_laptop();
    let home = census
        .me()
        .and_then(|r| r.home.clone())
        .or_else(dirs::home_dir)
        .unwrap_or_default();
    let usage = crate::usage::read_usage(&home);
    let mut remembered: Vec<PathBuf> = usage.recent.into_iter().map(|(p, _)| p).collect();
    remembered.extend(usage.pinned);
    let facts = SweepFacts {
        laptop,
        census,
        volumes: crate::volumes::volumes_this_laptop(),
        known_folders: crate::folders::find_special_folders(),
        remembered,
        rules: Rules::shipped(),
    };
    let mut reader = crate::acl::system_reader();
    sweep(
        facts,
        reader.as_mut(),
        stop,
        checkpoint,
        &SweepLimits::default(),
        resume,
    )
}

/// The sweep, from what it is told about the laptop.
pub fn sweep(
    facts: SweepFacts,
    access: &mut dyn AccessReader,
    stop: &AtomicBool,
    checkpoint: &mut dyn FnMut(&FrontierView<'_>),
    limits: &SweepLimits,
    resume: Option<Frontier>,
) -> Sweep {
    let mut s = Sweeper {
        state: match resume {
            Some(f) => {
                let mut state = f.state;
                for job in f.in_flight {
                    let q = if job.depth <= SHALLOW_DEPTH {
                        &mut state.shallow
                    } else {
                        &mut state.deep
                    };
                    if let Some(q) = q.get_mut(job.volume) {
                        q.push_front(job);
                    }
                }
                state
            }
            None => State {
                census: facts.census.clone(),
                record: Record::new(facts.laptop),
                places: Vec::new(),
                meta: Vec::new(),
                item_place: Vec::new(),
                removed: BTreeSet::new(),
                blind: BTreeMap::new(),
                counts: Counts::default(),
                links: 0,
                seen_keys: HashSet::new(),
                vols: vec![VolumeSeen::default(); facts.volumes.len()],
                known_done: Vec::new(),
                known_started: false,
                shallow: vec![VecDeque::new(); facts.volumes.len()],
                deep: vec![VecDeque::new(); facts.volumes.len()],
                stamps: Vec::new(),
                loose: Vec::new(),
            },
        },
        facts: &facts,
        access,
        limits,
        known_paths: Vec::new(),
        devs: Vec::new(),
        last_save: Duration::ZERO,
    };
    s.prepare();
    let finished = s.run(stop, checkpoint);
    s.finish(finished)
}

struct Sweeper<'a> {
    state: State,
    facts: &'a SweepFacts,
    access: &'a mut dyn AccessReader,
    limits: &'a SweepLimits,
    known_paths: Vec<PathBuf>,
    /// For each volume, the device numbers that are part of it (Unix).
    devs: Vec<Vec<u64>>,
    /// How long the last checkpoint took.
    last_save: Duration,
}

impl Sweeper<'_> {
    fn prepare(&mut self) {
        self.known_paths = outer_known(&self.facts.known_folders)
            .iter()
            .map(|(_, p)| p.clone())
            .collect();
        self.devs = self
            .facts
            .volumes
            .iter()
            .map(|v| {
                std::iter::once(&v.mount)
                    .chain(v.also.iter())
                    .filter_map(|p| dev_of(p))
                    .collect()
            })
            .collect();
        if self.state.known_started {
            return;
        }
        self.state.known_started = true;
        // Every volume starts with its root, breadth first.
        for (i, v) in self.facts.volumes.iter().enumerate() {
            if v.kind == VolumeKind::Internal {
                self.state.vols[i].walked = true;
                let zone = self.root_zone(i);
                self.state.shallow[i].push_back(Job {
                    volume: i,
                    rel: Vec::new(),
                    depth: 0,
                    zone,
                    item: None,
                });
                // macOS: the data volume behind `/`. Most of it is seen through `/` (Users,
                // Applications, Library); what an administrator made at its root is not, so its
                // root is listed too.
                for also in &v.also {
                    let Ok(inner) = also.strip_prefix(&v.mount) else {
                        continue;
                    };
                    if !also.is_dir() {
                        continue;
                    }
                    self.state.shallow[i].push_back(Job {
                        volume: i,
                        rel: inner.iter().map(item_name).collect(),
                        depth: 0,
                        zone: Zone::Explore { inside: None },
                        item: None,
                    });
                }
            } else if v.kind == VolumeKind::OtherSystem {
                add_blind(
                    &mut self.state.blind,
                    BlindSpot {
                        volume: Some(v.label()),
                        path: None,
                        reason: BlindReason::OtherSystem,
                        count: 1,
                    },
                );
            }
        }
        // Drives the person's own records remember that are not connected.
        let mut missing = BTreeSet::new();
        for p in &self.facts.remembered {
            if self.facts.volumes.iter().any(|v| is_within(p, &v.mount)) {
                continue;
            }
            if let Some(root) = drive_root(p)
                && !root.exists()
            {
                missing.insert(root.to_string_lossy().into_owned());
            }
        }
        for root in missing {
            add_blind(
                &mut self.state.blind,
                BlindSpot {
                    volume: Some(root),
                    path: None,
                    reason: BlindReason::NotConnected,
                    count: 1,
                },
            );
        }
    }

    /// How a volume's root is walked. A volume that is someone's home (a ZFS dataset or a
    /// systemd-homed home per person) is walked as that home, once; one inside a home belongs to
    /// that person; anything else is explored.
    fn root_zone(&mut self, volume: usize) -> Zone {
        let mount = self.facts.volumes[volume].mount.clone();
        let owner = self
            .state
            .census
            .residents
            .iter()
            .filter(|r| r.home.as_ref().is_some_and(|h| is_within(&mount, h)))
            .max_by_key(|r| r.home.as_ref().map_or(0, |h| h.components().count()))
            .map(|r| (r.kind, r.account_id.clone(), r.home.clone()));
        let Some((kind, id, home)) = owner else {
            return self.mount_context(volume);
        };
        let exact = home.as_ref().is_some_and(|h| same_place(h, &mount));
        let shown = Some(mount.to_string_lossy().into_owned());
        if kind == PersonKind::Me {
            let me = Owner::Person {
                account_id: self.state.census.me.clone(),
            };
            let storage = self.storage(volume);
            let (role, rule) = if exact {
                (FolderRole::Home, Rule::InsideHome)
            } else {
                (FolderRole::Other, Rule::InsideHome)
            };
            let p = self.new_place(
                volume,
                shown,
                Placement::new(Whose::Mover, rule),
                Fate::Moves,
                PlaceMeta {
                    record_place: Some(Place { role, storage }),
                    owner: Some(me),
                    space: Space::People,
                    private: false,
                    items: 0,
                    collapsed: None,
                    root_item: None,
                },
            );
            return Zone::Itemise { base: 0, place: p };
        }
        let whose = match kind {
            PersonKind::Leftover => Whose::FromOldAccount { account_id: id },
            _ => Whose::Person {
                account_id: id.unwrap_or_default(),
            },
        };
        let p = self.new_place(
            volume,
            None,
            Placement::new(whose, Rule::InsideHome),
            Fate::NotYours,
            PlaceMeta {
                record_place: None,
                owner: None,
                space: Space::OtherPeople,
                private: true,
                items: 0,
                collapsed: None,
                root_item: None,
            },
        );
        Zone::Whole { place: p }
    }

    /// A volume mounted inside another (`/var`, `/opt` or `/home` as their own file systems, a
    /// folder-mounted Windows volume) is what that place would be: the profiles folder, or
    /// whatever the rules file says the folder it is mounted at is. Elsewhere it is explored.
    fn mount_context(&mut self, volume: usize) -> Zone {
        let v = self.facts.volumes[volume].clone();
        if self
            .state
            .census
            .profiles_root
            .as_ref()
            .is_some_and(|r| same_place(r, &v.mount))
        {
            return Zone::Profiles;
        }
        let parent = self
            .facts
            .volumes
            .iter()
            .filter(|o| !same_place(&o.mount, &v.mount) && is_within(&v.mount, &o.mount))
            .max_by_key(|o| o.mount.components().count())
            .cloned();
        let Some(parent) = parent else {
            return Zone::Explore { inside: None };
        };
        let Ok(inner) = v.mount.strip_prefix(&parent.mount) else {
            return Zone::Explore { inside: None };
        };
        for (i, part) in inner.iter().enumerate() {
            let name = part.to_string_lossy();
            let row = if i == 0 {
                self.facts.rules.at_root(&name, parent.is_system)
            } else {
                None
            }
            .or_else(|| self.facts.rules.anywhere(&name));
            let Some(row) = row else { continue };
            if !matches!(
                row.kind,
                Kind::System | Kind::App | Kind::Junk | Kind::OldSystem
            ) {
                continue;
            }
            let placement = Placement {
                whose: Whose::System,
                kind: Some(row.kind),
                rule: Rule::RulesFile { row: row.label() },
            };
            let p = self.start_place(volume, &v.mount, placement);
            return Zone::Count { place: p };
        }
        Zone::Explore { inside: None }
    }

    fn mount_text(&self, volume: usize) -> String {
        self.facts.volumes[volume].label()
    }

    fn abs(&self, volume: usize, rel: &[ItemName]) -> PathBuf {
        let mut p = self.facts.volumes[volume].mount.clone();
        for n in rel {
            p.push(n.on_disk());
        }
        p
    }

    fn storage(&self, volume: usize) -> Storage {
        let v = &self.facts.volumes[volume];
        if v.is_system {
            Storage::SystemDrive
        } else {
            Storage::OtherDrive { drive: v.label() }
        }
    }

    fn several(&self) -> bool {
        self.state.census.people_count() >= 2
    }

    fn new_place(
        &mut self,
        volume: usize,
        path: Option<String>,
        placement: Placement,
        fate: Fate,
        meta: PlaceMeta,
    ) -> usize {
        self.state.places.push(FoundPlace {
            volume: self.mount_text(volume),
            path,
            placement,
            fate,
            files: 0,
            bytes: 0,
        });
        self.state.meta.push(meta);
        self.state.places.len() - 1
    }

    /// Counts a listed file: its length toward the place, what it takes on the disk toward the
    /// volume's coverage line.
    fn count_entry(&mut self, volume: usize, place: usize, entry: &Entry) {
        self.count_seen(
            volume,
            place,
            entry.len,
            entry.alloc.unwrap_or(entry.len),
            entry.key,
        );
    }

    fn count_bytes(&mut self, volume: usize, place: usize, len: u64, key: Option<(u64, u64)>) {
        self.count_seen(volume, place, len, len, key);
    }

    fn count_seen(
        &mut self,
        volume: usize,
        place: usize,
        len: u64,
        seen: u64,
        key: Option<(u64, u64)>,
    ) {
        if let Some(k) = key
            && !self.state.seen_keys.insert(k)
        {
            return;
        }
        let space = self.state.meta[place].space;
        let v = &mut self.state.vols[volume];
        v.seen += seen;
        v.lines.entry(space).or_insert((0, true)).0 += seen;
        self.state.places[place].files += 1;
        self.state.places[place].bytes += len;
    }

    fn blind(&mut self, volume: usize, path: Option<String>, reason: BlindReason) {
        let label = self.mount_text(volume);
        add_blind(
            &mut self.state.blind,
            BlindSpot {
                volume: Some(label),
                path,
                reason,
                count: 1,
            },
        );
    }

    // --- the special folders -----------------------------------------------------------

    fn known_folders(&mut self, stop: &AtomicBool) -> bool {
        let me = Owner::Person {
            account_id: self.state.census.me.clone(),
        };
        let outer = outer_known(&self.facts.known_folders);
        for (i, folder) in outer.iter().enumerate() {
            if self.state.known_done.contains(&i) {
                continue;
            }
            if stop.load(Ordering::SeqCst) {
                return false;
            }
            let FolderLookup::Found(found) = &self.facts.known_folders[folder.0] else {
                continue;
            };
            let shared = found.role == FolderRole::Public;
            let owner = if shared { Owner::Shared } else { me.clone() };
            let rplace = Place {
                role: found.role,
                storage: found.storage.clone(),
            };
            let volume = self.volume_of(&found.path);
            let placement = Placement::new(
                if shared {
                    Whose::Shared {
                        limited_to: Vec::new(),
                    }
                } else {
                    Whose::Mover
                },
                Rule::KnownFolder,
            );
            let place = self.new_place(
                volume.unwrap_or(0),
                Some(found.path.to_string_lossy().into_owned()),
                placement,
                Fate::Moves,
                PlaceMeta {
                    record_place: Some(rplace.clone()),
                    owner: Some(owner.clone()),
                    space: Space::People,
                    private: false,
                    items: 0,
                    collapsed: None,
                    root_item: None,
                },
            );
            if let Some(v) = volume {
                self.state.places[place].volume = self.mount_text(v);
            }
            let scan = scan_managed_folder(
                &self.facts.laptop,
                &owner,
                &rplace,
                &found.path,
                found.managed_by,
            );
            for item in scan.items {
                if item.kind == ItemKind::File
                    && item.inclusion != left_out(LeftOutReason::CloudOnly)
                {
                    if let Some(v) = volume {
                        self.count_bytes(v, place, item.size_bytes, None);
                    }
                } else if item.kind != ItemKind::File && item.size_bytes > 0 {
                    // A package: one item.
                    if let Some(v) = volume {
                        self.count_bytes(v, place, item.size_bytes, None);
                    }
                }
                self.push_item(item, place);
            }
            self.state.stamps.extend(scan.stamps);
            add_counts(&mut self.state.counts, &scan.counts);
            self.state.links += scan.links;
            for dir in scan.unreadable {
                let why = std::fs::read_dir(for_listing(&dir))
                    .err()
                    .map(|e| Refusal::of(&e))
                    .unwrap_or(Refusal::Other(None));
                // A special folder the system names but that is not there holds nothing.
                if why == Refusal::Gone && same_place(&dir, &found.path) {
                    continue;
                }
                if let Some(reason) = why.reason() {
                    let label = volume.map(|v| self.mount_text(v));
                    add_blind(
                        &mut self.state.blind,
                        BlindSpot {
                            volume: label,
                            path: Some(dir.to_string_lossy().into_owned()),
                            reason,
                            count: 1,
                        },
                    );
                }
            }
            self.state.known_done.push(i);
        }
        true
    }

    fn volume_of(&self, path: &Path) -> Option<usize> {
        self.facts
            .volumes
            .iter()
            .enumerate()
            .filter(|(_, v)| is_within(path, &v.mount))
            .max_by_key(|(_, v)| v.mount.components().count())
            .map(|(i, _)| i)
    }

    fn push_item(&mut self, item: Item, place: usize) -> usize {
        self.state.record.items.push(item);
        self.state.item_place.push(place);
        self.state.meta[place].items += 1;
        self.state.record.items.len() - 1
    }

    // --- the walk ----------------------------------------------------------------------

    fn run(&mut self, stop: &AtomicBool, checkpoint: &mut dyn FnMut(&FrontierView<'_>)) -> bool {
        if !self.known_folders(stop) {
            checkpoint(&self.view(Vec::new()));
            return false;
        }
        let mut since = Instant::now();
        let mut entries: u64 = 0;
        let volumes = self.facts.volumes.len();
        for deep in [false, true] {
            for v in 0..volumes {
                let walkers = self.facts.volumes[v].disk.walkers(self.limits.most_walkers);
                self.state.vols[v].walkers = self.state.vols[v].walkers.max(walkers);
                if !self.walk_volume(v, deep, walkers, stop, checkpoint, &mut since, &mut entries) {
                    checkpoint(&self.view(Vec::new()));
                    return false;
                }
            }
        }
        true
    }

    fn take_job(&mut self, v: usize, deep: bool) -> Option<Job> {
        let queue = if deep {
            &mut self.state.deep
        } else {
            &mut self.state.shallow
        };
        queue.get_mut(v)?.pop_front()
    }

    #[allow(clippy::too_many_arguments)]
    fn walk_volume(
        &mut self,
        v: usize,
        deep: bool,
        walkers: usize,
        stop: &AtomicBool,
        checkpoint: &mut dyn FnMut(&FrontierView<'_>),
        since: &mut Instant,
        entries: &mut u64,
    ) -> bool {
        let want_keys = cfg!(windows);
        let (job_send, job_recv) = channel::<(usize, PathBuf, bool, bool)>();
        let job_recv = Arc::new(Mutex::new(job_recv));
        let (done_send, done_recv) = channel::<(usize, Listing)>();
        let mut stopped = false;
        std::thread::scope(|scope| {
            for _ in 0..walkers {
                let job_recv = Arc::clone(&job_recv);
                let done_send = done_send.clone();
                scope.spawn(move || {
                    loop {
                        let next = match job_recv.lock() {
                            Ok(r) => r.recv(),
                            Err(_) => break,
                        };
                        let Ok((ticket, path, keys, whole)) = next else {
                            break;
                        };
                        let listed = if whole {
                            Listing::Whole(count_whole(&path))
                        } else {
                            Listing::Dir(list_dir(&path, keys && want_keys))
                        };
                        if done_send.send((ticket, listed)).is_err() {
                            break;
                        }
                    }
                });
            }
            drop(done_send);
            let mut in_flight: BTreeMap<usize, Job> = BTreeMap::new();
            let mut ticket = 0usize;
            loop {
                while in_flight.len() < walkers && !stopped {
                    if stop.load(Ordering::SeqCst) {
                        stopped = true;
                        break;
                    }
                    let Some(job) = self.take_job(v, deep) else {
                        break;
                    };
                    let path = for_listing(&self.abs(job.volume, &job.rel));
                    let keys = matches!(job.zone, Zone::Itemise { .. });
                    let whole = matches!(job.zone, Zone::Whole { .. });
                    ticket += 1;
                    if job_send.send((ticket, path, keys, whole)).is_err() {
                        stopped = true;
                        self.requeue(job);
                        break;
                    }
                    in_flight.insert(ticket, job);
                }
                if in_flight.is_empty() {
                    break;
                }
                let Ok((t, listed)) = done_recv.recv() else {
                    break;
                };
                let Some(job) = in_flight.remove(&t) else {
                    continue;
                };
                match &listed {
                    Listing::Dir(Ok(list)) => *entries += list.len() as u64,
                    Listing::Whole(w) => *entries += w.files,
                    Listing::Dir(Err(_)) => {}
                }
                self.process(job, listed);
                // Every 5 seconds or 10,000 entries, but never more than a fifth of the time:
                // saving grows with what was found, and a save that took longer than the gap
                // would otherwise leave no time for the walk itself.
                let due = *entries >= self.limits.checkpoint_entries
                    || since.elapsed() >= self.limits.checkpoint_every;
                if due && since.elapsed() >= self.last_save * 4 {
                    let view = self.view(in_flight.values().cloned().collect());
                    let started = Instant::now();
                    checkpoint(&view);
                    self.last_save = started.elapsed();
                    *entries = 0;
                    *since = Instant::now();
                }
            }
            drop(job_send);
            // Folders listed but not processed go back on the frontier.
            for (_, job) in in_flight {
                self.requeue(job);
            }
        });
        !stopped
    }

    fn requeue(&mut self, job: Job) {
        let v = job.volume;
        if job.depth <= SHALLOW_DEPTH {
            self.state.shallow[v].push_front(job);
        } else {
            self.state.deep[v].push_front(job);
        }
    }

    fn view(&self, in_flight: Vec<Job>) -> FrontierView<'_> {
        FrontierView {
            state: &self.state,
            in_flight,
        }
    }

    fn enqueue(&mut self, job: Job) {
        let v = job.volume;
        if job.depth <= SHALLOW_DEPTH {
            self.state.shallow[v].push_back(job);
        } else {
            self.state.deep[v].push_back(job);
        }
    }

    fn child(&self, job: &Job, name: &ItemName, zone: Zone, item: Option<usize>) -> Job {
        let mut rel = job.rel.clone();
        rel.push(name.clone());
        Job {
            volume: job.volume,
            rel,
            depth: job.depth.saturating_add(1),
            zone,
            item,
        }
    }

    fn process(&mut self, job: Job, listed: Listing) {
        let listed = match listed {
            Listing::Whole(w) => {
                self.whole(job, w);
                return;
            }
            Listing::Dir(listed) => listed,
        };
        let entries = match listed {
            Ok(e) => e,
            Err(why) => {
                self.refused(&job, why);
                return;
            }
        };
        match job.zone.clone() {
            Zone::Explore { inside } => self.explore(&job, entries, inside),
            Zone::Profiles => self.profiles(&job, entries),
            Zone::Itemise { base, place } => self.itemise(&job, entries, base, place),
            Zone::AppData { home, place } => self.app_data(&job, entries, home, place),
            Zone::Count { place } => self.count(&job, entries, place),
            Zone::Collapse { item, place } => self.collapse(&job, entries, item, place),
            Zone::OldSystem { place } => self.old_system(&job, entries, place),
            Zone::OldProfiles { place } => self.old_profiles(&job, entries, place),
            // Counted by a walker; never listed here.
            Zone::Whole { .. } => {}
        }
    }

    /// The count of another person's place (or one from an old account) came back: totals only.
    fn whole(&mut self, job: Job, w: Whole) {
        let Zone::Whole { place } = job.zone else {
            return;
        };
        for uid in &w.owners {
            let id = uid.to_string();
            if self.state.census.by_account(&id).is_none() {
                self.state.census.note_owner_seen(&id);
            }
        }
        let space = self.state.meta[place].space;
        let v = &mut self.state.vols[job.volume];
        v.seen += w.seen;
        v.lines.entry(space).or_insert((0, true)).0 += w.seen;
        self.state.places[place].files += w.files;
        self.state.places[place].bytes += w.bytes;
        if w.root_refused.is_some() || w.inside_refused {
            let reason = self.private_reason(place);
            self.blind(job.volume, None, reason);
        }
    }

    /// The blind-spot reason for another person's place: never with a name or path.
    fn private_reason(&self, place: usize) -> BlindReason {
        match &self.state.places[place].placement.whose {
            Whose::Person { account_id } => BlindReason::AnotherPerson {
                account_id: Some(account_id.clone()),
            },
            Whose::FromOldAccount { .. } => BlindReason::FromOldAccount,
            Whose::Undecided => BlindReason::AccessNotReadable,
            _ => BlindReason::AnotherPerson { account_id: None },
        }
    }

    /// A folder that could not be listed: listed as a blind spot with its reason, never as empty.
    fn refused(&mut self, job: &Job, why: Refusal) {
        if let Some(i) = job.item {
            self.state.record.items[i].inclusion = left_out(LeftOutReason::Unreadable);
        }
        let Some(reason) = why.reason() else {
            return;
        };
        let path = self
            .abs(job.volume, &job.rel)
            .to_string_lossy()
            .into_owned();
        let place = match &job.zone {
            Zone::Itemise { place, .. }
            | Zone::AppData { place, .. }
            | Zone::Count { place }
            | Zone::Collapse { place, .. } => Some(*place),
            Zone::Explore { inside } => *inside,
            Zone::Whole { place } | Zone::OldSystem { place } | Zone::OldProfiles { place } => {
                Some(*place)
            }
            Zone::Profiles => None,
        };
        if let Some(p) = place {
            let meta = &self.state.meta[p];
            if meta.private {
                let reason = self.private_reason(p);
                self.blind(job.volume, None, reason);
                return;
            }
            if matches!(meta.space, Space::TheSystem | Space::Apps | Space::Junk)
                && why != Refusal::Gone
            {
                // The system's own corners: named on the volume's line, size unknown. (One that
                // changed during the scan was not seen at all, so it is a blind spot below.)
                let line = self.state.vols[job.volume]
                    .lines
                    .entry(Space::SystemKeepsToItself)
                    .or_insert((0, false));
                line.1 = false;
                return;
            }
        }
        self.blind(job.volume, Some(path), reason);
    }

    fn skipped(&self, job: &Job, entry: &Entry, path: &Path) -> bool {
        let v = &self.facts.volumes[job.volume];
        if v.skip.iter().any(|s| same_place(s, path)) {
            return true;
        }
        // Another volume mounted here is walked as that volume, never from here (every mount
        // inside this volume is in `skip`). A folder with its own device number that is no mount
        // (a btrfs sub-volume) is part of this volume and is walked; only the device number of
        // another listed volume marks a mount the list missed.
        if let Some(dev) = entry.dev
            && entry.kind == Some(EntryKind::Dir)
            && !self.devs[job.volume].contains(&dev)
            && self
                .devs
                .iter()
                .enumerate()
                .any(|(i, d)| i != job.volume && d.contains(&dev))
        {
            return true;
        }
        false
    }

    fn note_owner(&mut self, entry: &Entry) {
        if let Some(u) = entry.unix {
            let id = u.uid.to_string();
            if self.state.census.by_account(&id).is_none() {
                self.state.census.note_owner_seen(&id);
            }
        }
    }

    /// Whether a folder holds someone's home, the profiles folder or a special folder further
    /// down (so it must be explored, not counted or listed whole).
    fn holds_people(&self, path: &Path) -> bool {
        self.state
            .census
            .residents
            .iter()
            .filter_map(|r| r.home.as_ref())
            .any(|h| is_within(h, path))
            || self
                .state
                .census
                .profiles_root
                .as_ref()
                .is_some_and(|r| is_within(r, path))
            || self.known_paths.iter().any(|k| is_within(k, path))
    }

    fn explore(&mut self, job: &Job, entries: Vec<Entry>, inside: Option<usize>) {
        let is_system = self.facts.volumes[job.volume].is_system;
        let keeps_owners = self.facts.volumes[job.volume].keeps_owners();
        let mut loose: BTreeMap<Placement, usize> = BTreeMap::new();
        for entry in entries {
            let name = item_name(&entry.name);
            let text = name.display().to_string();
            let path = self.abs(job.volume, &job.rel).join(&entry.name);
            if self.skipped(job, &entry, &path) {
                continue;
            }
            self.note_owner(&entry);
            let Some(kind) = entry.kind else {
                self.blind(
                    job.volume,
                    Some(path.to_string_lossy().into_owned()),
                    BlindReason::NoPermission,
                );
                continue;
            };
            if kind == EntryKind::Link {
                self.state.links += 1;
                continue;
            }
            if self.known_paths.iter().any(|k| same_place(k, &path)) {
                continue;
            }
            if kind == EntryKind::Dir {
                if self
                    .state
                    .census
                    .profiles_root
                    .as_ref()
                    .is_some_and(|r| same_place(r, &path))
                {
                    let child = self.child(job, &name, Zone::Profiles, None);
                    self.enqueue(child);
                    continue;
                }
                if let Some(home) = self.home_here(&path) {
                    self.enter_home(job, &name, &path, home);
                    continue;
                }
            }
            let row = if job.depth == 0 {
                self.facts.rules.at_root(&text, is_system)
            } else {
                None
            }
            .or_else(|| {
                (kind == EntryKind::Dir)
                    .then(|| self.facts.rules.anywhere(&text))
                    .flatten()
            });
            if kind == EntryKind::Dir
                && let Some(r) = row
                && r.kind == Kind::OldSystem
            {
                let placement = Placement {
                    whose: Whose::System,
                    kind: Some(Kind::OldSystem),
                    rule: Rule::RulesFile { row: r.label() },
                };
                let p = self.start_place(job.volume, &path, placement);
                let child = self.child(job, &name, Zone::OldSystem { place: p }, None);
                self.enqueue(child);
                continue;
            }
            if kind == EntryKind::Dir && self.holds_people(&path) {
                // Someone's home or a special folder is further down: keep exploring. Inside a
                // rules-file place (`/var` holding `/var/home`), the rest stays in that place.
                let inside = match row {
                    Some(r) => {
                        let placement = place_by_ladder(
                            &Facts {
                                rule: Some(r),
                                home: None,
                                access: &AccessFact::Unknown {
                                    why: "a rules-file place".into(),
                                },
                                around: None,
                            },
                            &Ladder {
                                census: &self.state.census,
                            },
                        );
                        Some(self.start_place(job.volume, &path, placement))
                    }
                    None => inside,
                };
                let child = self.child(job, &name, Zone::Explore { inside }, None);
                self.enqueue(child);
                continue;
            }
            if let Some(p) = inside {
                // Inside a rules-file place: counted with it.
                match kind {
                    EntryKind::Dir => {
                        let child = self.child(job, &name, Zone::Count { place: p }, None);
                        self.enqueue(child);
                    }
                    _ => self.count_entry(job.volume, p, &entry),
                }
                continue;
            }
            let fact = if !keeps_owners {
                AccessFact::NoOwners
            } else if self.several()
                && row.is_none_or(|r| !matches!(r.kind, Kind::System | Kind::App | Kind::Junk))
            {
                self.read_access(&path, entry.unix)
            } else {
                AccessFact::Unknown {
                    why: "not needed".into(),
                }
            };
            let placement = place_by_ladder(
                &Facts {
                    rule: row,
                    home: None,
                    access: &fact,
                    around: None,
                },
                &Ladder {
                    census: &self.state.census,
                },
            );
            match kind {
                EntryKind::Dir => {
                    let p = self.start_place(job.volume, &path, placement);
                    self.enter_place(job, &name, p, 0);
                }
                EntryKind::File => {
                    let p = match loose.get(&placement) {
                        Some(p) => *p,
                        None => {
                            let dir = self.abs(job.volume, &job.rel);
                            let p = self.start_place(job.volume, &dir, placement.clone());
                            loose.insert(placement, p);
                            p
                        }
                    };
                    self.file(job, &entry, &name, p, 0);
                }
                EntryKind::Link => {}
            }
        }
    }

    /// Reads who could open a place, noting an owner the census did not know.
    fn read_access(&mut self, path: &Path, unix: Option<UnixMode>) -> AccessFact {
        let fact = self.access.read(path, unix);
        if let AccessFact::Known(a) = &fact
            && let Some(o) = &a.owner
            && self.state.census.by_account(o).is_none()
        {
            self.state.census.note_owner_seen(o);
        }
        fact
    }

    /// Goes into a folder that starts place `p`: listed into the record, counted as one item
    /// (rebuildable and keep-whole places), or only counted.
    fn enter_place(&mut self, job: &Job, name: &ItemName, p: usize, base: usize) {
        let collapse = matches!(
            self.state.places[p].placement.kind,
            Some(Kind::Rebuildable | Kind::KeepWhole)
        );
        if self.state.meta[p].private {
            // Another person's folder outside the homes (or one whose access this person may not
            // even read): never walked, never named; a blind spot on its drive.
            let reason = self.private_reason(p);
            self.blind(job.volume, None, reason);
            return;
        }
        if self.state.meta[p].owner.is_none() {
            let child = self.child(job, name, Zone::Count { place: p }, None);
            self.enqueue(child);
            return;
        }
        let item = self.make_item(job, name, p, base, ItemKind::Folder, 0, Inclusion::Included);
        let Some(item) = item else {
            let child = self.child(job, name, Zone::Count { place: p }, None);
            self.enqueue(child);
            return;
        };
        let i = self.push_item(item, p);
        self.state.meta[p].root_item = Some(i);
        let zone = if collapse {
            Zone::Collapse { item: i, place: p }
        } else {
            Zone::Itemise { base, place: p }
        };
        let child = self.child(job, name, zone, Some(i));
        self.enqueue(child);
    }

    /// Starts a place for something outside everyone's home.
    fn start_place(&mut self, volume: usize, path: &Path, placement: Placement) -> usize {
        let (fate, space, private, owner) = match (&placement.whose, placement.kind) {
            (Whose::System, Some(Kind::App)) => (Fate::Counted, Space::Apps, false, None),
            (Whose::System, Some(Kind::Junk)) => (Fate::Counted, Space::Junk, false, None),
            (Whose::System, _) => (Fate::Counted, Space::TheSystem, false, None),
            (Whose::Mover, k) => (
                Fate::Moves,
                space_of(k),
                false,
                Some(Owner::Person {
                    account_id: self.state.census.me.clone(),
                }),
            ),
            (Whose::Shared { .. }, k) => (Fate::Moves, space_of(k), false, Some(Owner::Shared)),
            (Whose::Person { .. } | Whose::FromOldAccount { .. } | Whose::Undecided, _) => {
                (Fate::NotYours, Space::OtherPeople, true, None)
            }
        };
        let record_place = owner.as_ref().map(|_| Place {
            role: FolderRole::Other,
            storage: self.storage(volume),
        });
        let shown = (!private).then(|| path.to_string_lossy().into_owned());
        self.new_place(
            volume,
            shown,
            placement,
            fate,
            PlaceMeta {
                record_place,
                owner,
                space,
                private,
                items: 0,
                collapsed: None,
                root_item: None,
            },
        )
    }

    #[allow(clippy::too_many_arguments)]
    fn make_item(
        &self,
        job: &Job,
        name: &ItemName,
        place: usize,
        base: usize,
        kind: ItemKind,
        size: u64,
        inclusion: Inclusion,
    ) -> Option<Item> {
        let meta = &self.state.meta[place];
        let (Some(rplace), Some(owner)) = (&meta.record_place, &meta.owner) else {
            return None;
        };
        let mut parts: Vec<ItemName> = job.rel.get(base..).unwrap_or(&[]).to_vec();
        parts.push(name.clone());
        let path = ItemPath::new(parts).ok()?;
        Some(Item {
            id: ItemId::derive(&self.facts.laptop, owner, rplace, &path),
            kind,
            owner: owner.clone(),
            place: rplace.clone(),
            path,
            size_bytes: size,
            portability: Portability::Portable,
            managed_by: ManagedBy::Personal,
            download_bytes: None,
            inclusion,
        })
    }

    fn home_here(&self, path: &Path) -> Option<HomeOf> {
        let r = self
            .state
            .census
            .residents
            .iter()
            .find(|r| r.home.as_ref().is_some_and(|h| same_place(h, path)))?;
        Some(match r.kind {
            PersonKind::Me => HomeOf::Mover,
            PersonKind::Leftover => HomeOf::Leftover(r.account_id.clone()),
            _ => HomeOf::Person(r.account_id.clone().unwrap_or_default()),
        })
    }

    fn enter_home(&mut self, job: &Job, name: &ItemName, path: &Path, home: HomeOf) {
        let placement = place_by_ladder(
            &Facts {
                rule: None,
                home: Some(home.clone()),
                access: &AccessFact::Unknown {
                    why: "inside a home".into(),
                },
                around: None,
            },
            &Ladder {
                census: &self.state.census,
            },
        );
        let base = job.rel.len() + 1;
        match home {
            HomeOf::Mover => {
                let storage = self.storage(job.volume);
                let p = self.new_place(
                    job.volume,
                    Some(path.to_string_lossy().into_owned()),
                    placement,
                    Fate::Moves,
                    PlaceMeta {
                        record_place: Some(Place {
                            role: FolderRole::Home,
                            storage,
                        }),
                        owner: Some(Owner::Person {
                            account_id: self.state.census.me.clone(),
                        }),
                        space: Space::People,
                        private: false,
                        items: 0,
                        collapsed: None,
                        root_item: None,
                    },
                );
                let child = self.child(job, name, Zone::Itemise { base, place: p }, None);
                self.enqueue(child);
            }
            _ => {
                let p = self.new_place(
                    job.volume,
                    None,
                    placement,
                    Fate::NotYours,
                    PlaceMeta {
                        record_place: None,
                        owner: None,
                        space: Space::OtherPeople,
                        private: true,
                        items: 0,
                        collapsed: None,
                        root_item: None,
                    },
                );
                let child = self.child(job, name, Zone::Whole { place: p }, None);
                self.enqueue(child);
            }
        }
    }

    fn profiles(&mut self, job: &Job, entries: Vec<Entry>) {
        for entry in entries {
            let name = item_name(&entry.name);
            let path = self.abs(job.volume, &job.rel).join(&entry.name);
            // A home that is its own volume (a ZFS dataset or a systemd-homed home) is walked
            // as that volume, once.
            if self.skipped(job, &entry, &path) {
                continue;
            }
            self.note_owner(&entry);
            match entry.kind {
                Some(EntryKind::Link) => {
                    self.state.links += 1;
                    continue;
                }
                Some(EntryKind::Dir) => {}
                Some(EntryKind::File) => {
                    // desktop.ini and the like are the system's; anything else lying among the
                    // homes is placed like a file at a drive's root.
                    let text = name.display().to_string();
                    if is_clutter(&text) || text.to_ascii_lowercase().starts_with("ntuser") {
                        let p = self.system_place(job.volume, &path, Space::TheSystem);
                        self.count_entry(job.volume, p, &entry);
                    } else {
                        let p = self.loose_place(job, &entry, &path);
                        self.file(job, &entry, &name, p, 0);
                    }
                    continue;
                }
                None => {
                    self.blind(
                        job.volume,
                        Some(path.to_string_lossy().into_owned()),
                        BlindReason::NoPermission,
                    );
                    continue;
                }
            }
            if self.known_paths.iter().any(|k| same_place(k, &path)) {
                continue;
            }
            if let Some(home) = self.home_here(&path) {
                self.enter_home(job, &name, &path, home);
                continue;
            }
            if self.known_paths.iter().any(|k| is_within(k, &path))
                || self
                    .state
                    .census
                    .residents
                    .iter()
                    .filter_map(|r| r.home.as_ref())
                    .any(|h| is_within(h, &path))
            {
                let child = self.child(job, &name, Zone::Explore { inside: None }, None);
                self.enqueue(child);
                continue;
            }
            // The system's own profiles (Default, All Users) are counted, and Public (when the
            // system did not name it) is placed like any folder everyone can open. Anything else
            // here is somebody's home folder that the census could not tie to an account: it is
            // never explored by name. Its owner decides: the mover's own old profile
            // (`ada.OLD`) stays the mover's; anyone else's is counted, never named, and goes to
            // the approver as "From an old account" (or to that person, if they still exist).
            let text = name.display().to_string();
            if SYSTEM_PROFILES
                .iter()
                .any(|n| n.eq_ignore_ascii_case(&text))
                || text.to_ascii_lowercase().starts_with("defaultuser")
            {
                let p = self.system_place(job.volume, &path, Space::TheSystem);
                let child = self.child(job, &name, Zone::Count { place: p }, None);
                self.enqueue(child);
            } else if text.eq_ignore_ascii_case("Public") || text.eq_ignore_ascii_case("Shared") {
                let child = self.child(job, &name, Zone::Explore { inside: None }, None);
                self.enqueue(child);
            } else {
                self.unexplained_home(job, &entry, &name, &path);
            }
        }
    }

    /// A folder among the homes that no account explains: whose it is comes from its owner.
    fn unexplained_home(&mut self, job: &Job, entry: &Entry, name: &ItemName, path: &Path) {
        let owner = match self.read_access(path, entry.unix) {
            AccessFact::Known(a) => a.owner,
            AccessFact::NoOwners | AccessFact::Unknown { .. } => None,
        };
        if owner.as_deref() == Some(self.state.census.me.as_str()) {
            let placement = Placement::new(Whose::Mover, Rule::InsideHome);
            let p = self.start_place(job.volume, path, placement);
            self.enter_place(job, name, p, 0);
            return;
        }
        let whose = match owner {
            Some(id)
                if self
                    .state
                    .census
                    .by_account(&id)
                    .is_some_and(|r| r.kind != PersonKind::Leftover) =>
            {
                Whose::Person { account_id: id }
            }
            Some(id) if self.state.census.is_person_id(&id) => Whose::FromOldAccount {
                account_id: Some(id),
            },
            _ => Whose::FromOldAccount { account_id: None },
        };
        self.private_home(job, name, whose);
    }

    /// Counts another person's home (or one from an old account) in one go, never by name.
    fn private_home(&mut self, job: &Job, name: &ItemName, whose: Whose) {
        let p = self.new_place(
            job.volume,
            None,
            Placement::new(whose, Rule::InsideHome),
            Fate::NotYours,
            PlaceMeta {
                record_place: None,
                owner: None,
                space: Space::OtherPeople,
                private: true,
                items: 0,
                collapsed: None,
                root_item: None,
            },
        );
        let child = self.child(job, name, Zone::Whole { place: p }, None);
        self.enqueue(child);
    }

    /// The place for loose files placed `placement` in the folder `job` lists (one per folder and
    /// placement). Another person's loose files are counted there without names.
    fn loose_for(&mut self, job: &Job, placement: Placement) -> usize {
        let dir = self.abs(job.volume, &job.rel);
        let key = (
            job.volume,
            dir.to_string_lossy().into_owned(),
            placement.clone(),
        );
        if let Some(p) = self
            .state
            .loose
            .iter()
            .find(|(k, _)| *k == key)
            .map(|(_, p)| *p)
        {
            return p;
        }
        let p = self.start_place(job.volume, &dir, placement);
        self.state.loose.push((key, p));
        p
    }

    /// The place for a loose file found among the homes, placed like one at a drive's root.
    fn loose_place(&mut self, job: &Job, entry: &Entry, path: &Path) -> usize {
        let fact = if !self.facts.volumes[job.volume].keeps_owners() {
            AccessFact::NoOwners
        } else if self.several() {
            self.read_access(path, entry.unix)
        } else {
            AccessFact::Unknown {
                why: "not needed".into(),
            }
        };
        let placement = place_by_ladder(
            &Facts {
                rule: None,
                home: None,
                access: &fact,
                around: None,
            },
            &Ladder {
                census: &self.state.census,
            },
        );
        self.loose_for(job, placement)
    }

    fn system_place(&mut self, volume: usize, path: &Path, space: Space) -> usize {
        let shown = Some(path.to_string_lossy().into_owned());
        if let Some(p) = self
            .state
            .places
            .iter()
            .position(|pl| pl.path == shown && pl.placement.whose == Whose::System)
        {
            return p;
        }
        self.new_place(
            volume,
            shown,
            Placement::new(Whose::System, Rule::SystemFolder),
            Fate::Counted,
            counted_meta(space),
        )
    }

    fn itemise(&mut self, job: &Job, entries: Vec<Entry>, base: usize, place: usize) {
        if let Some(item) = self.state.meta[place].collapsed {
            // This place grew too big to list: counted as one item from here on.
            self.collapse(job, entries, item, place);
            return;
        }
        let in_home =
            self.state.meta[place].record_place.as_ref().map(|p| p.role) == Some(FolderRole::Home);
        let home_parts: Vec<String> = job
            .rel
            .get(base..)
            .unwrap_or(&[])
            .iter()
            .map(|n| n.display().to_string())
            .collect();
        for entry in entries {
            let name = item_name(&entry.name);
            let text = name.display().to_string();
            let path = self.abs(job.volume, &job.rel).join(&entry.name);
            if self.skipped(job, &entry, &path) {
                continue;
            }
            if self.known_paths.iter().any(|k| same_place(k, &path)) {
                continue;
            }
            self.note_owner(&entry);
            let Some(kind) = entry.kind else {
                if let Some(item) = self.make_item(
                    job,
                    &name,
                    place,
                    base,
                    ItemKind::File,
                    0,
                    left_out(LeftOutReason::Unreadable),
                ) {
                    self.push_item(item, place);
                }
                self.blind(
                    job.volume,
                    Some(path.to_string_lossy().into_owned()),
                    BlindReason::NoPermission,
                );
                continue;
            };
            if is_clutter(&text) {
                let (k, size) = if kind == EntryKind::Dir {
                    (ItemKind::Folder, 0)
                } else {
                    (ItemKind::File, entry.len)
                };
                if kind == EntryKind::File {
                    self.count_entry(job.volume, place, &entry);
                }
                if let Some(item) = self.make_item(
                    job,
                    &name,
                    place,
                    base,
                    k,
                    size,
                    left_out(LeftOutReason::System),
                ) {
                    self.push_item(item, place);
                }
                if kind == EntryKind::Dir {
                    // The system's bins and indexes: counted, never listed.
                    let p = self.system_place(job.volume, &path, Space::Junk);
                    let child = self.child(job, &name, Zone::Count { place: p }, None);
                    self.enqueue(child);
                }
                continue;
            }
            match kind {
                EntryKind::Link => self.state.links += 1,
                EntryKind::File => {
                    // Inside a home, a file the rules file names (NTUSER.DAT) is the system's.
                    if in_home
                        && self
                            .facts
                            .rules
                            .in_home(&with(&home_parts, &text))
                            .is_some_and(|r| r.kind == Kind::System)
                    {
                        let p = self.system_place(job.volume, &path, Space::TheSystem);
                        self.count_entry(job.volume, p, &entry);
                        continue;
                    }
                    // macOS and Linux list each file's owner and mode, so a file one person
                    // keeps to themselves inside a shared folder is told apart for free
                    // (Windows would need a call per file; there the folder decides).
                    if entry.unix.is_some()
                        && let Some(narrower) =
                            self.narrower(job, &path, entry.unix, place, in_home)
                    {
                        let p = self.loose_for(job, narrower);
                        self.file(job, &entry, &name, p, base);
                        continue;
                    }
                    self.file(job, &entry, &name, place, base);
                }
                EntryKind::Dir => {
                    self.itemise_dir(job, &entry, &name, &path, place, base, in_home, &home_parts)
                }
            }
        }
        self.maybe_collapse(place);
    }

    #[allow(clippy::too_many_arguments)]
    fn itemise_dir(
        &mut self,
        job: &Job,
        entry: &Entry,
        name: &ItemName,
        path: &Path,
        here: usize,
        base: usize,
        in_home: bool,
        home_parts: &[String],
    ) {
        let text = name.display().to_string();
        // Someone else's home is never listed as part of another place.
        if let Some(home) = self.home_here(path)
            && home != HomeOf::Mover
        {
            self.enter_home(job, name, path, home);
            return;
        }
        // With two or more people, below a shared place, who could open a folder is read before
        // anything else is decided about it (a project's history, a package, a plain folder): one
        // person's own corner of a family folder is theirs. The answer can only narrow what the
        // place around allows, never widen it.
        let mut here = here;
        let mut fresh = false;
        if let Some(narrower) = self.narrower(job, path, entry.unix, here, in_home) {
            here = self.start_place(job.volume, path, narrower);
            if self.state.meta[here].private {
                let reason = self.private_reason(here);
                self.blind(job.volume, None, reason);
                return;
            }
            fresh = true;
        }
        let around = self.state.places[here].placement.clone();
        // App data, junk and the system's own files inside a home.
        if in_home && let Some(row) = self.facts.rules.in_home(&with(home_parts, &text)) {
            let placement = place_with(row, &around, &self.state.census);
            match row.kind {
                Kind::AppData => {
                    let p = self.new_place(
                        job.volume,
                        Some(path.to_string_lossy().into_owned()),
                        placement,
                        Fate::FoundNotMovedInThisVersion,
                        counted_meta(Space::AppData),
                    );
                    let child = self.child(
                        job,
                        name,
                        Zone::AppData {
                            home: base,
                            place: p,
                        },
                        None,
                    );
                    self.enqueue(child);
                    return;
                }
                Kind::Junk | Kind::System | Kind::App | Kind::OldSystem => {
                    let p = self.new_place(
                        job.volume,
                        Some(path.to_string_lossy().into_owned()),
                        placement,
                        Fate::Counted,
                        counted_meta(if row.kind == Kind::Junk {
                            Space::Junk
                        } else {
                            Space::TheSystem
                        }),
                    );
                    let child = self.child(job, name, Zone::Count { place: p }, None);
                    self.enqueue(child);
                    return;
                }
                Kind::Rebuildable | Kind::KeepWhole => {}
            }
        }
        if let Some(row) = self.facts.rules.anywhere(&text) {
            // node_modules, .git and the like: one item, keeping its surroundings' owner.
            let placement = place_with(row, &around, &self.state.census);
            let mut meta = self.state.meta[here].clone();
            meta.space = space_of(Some(row.kind));
            meta.items = 0;
            meta.collapsed = None;
            meta.root_item = None;
            let p = self.new_place(
                job.volume,
                Some(path.to_string_lossy().into_owned()),
                placement,
                Fate::Moves,
                meta,
            );
            self.enter_place_at(job, name, p, base, true);
            return;
        }
        if let Some(package) = package_kind(&text) {
            if let Some(item) = self.make_item(
                job,
                name,
                here,
                base,
                package.item_kind(),
                0,
                Inclusion::Included,
            ) {
                match package {
                    Package::PhotosLibrary => count_library(path, &mut self.state.counts),
                    Package::Document => self.state.counts.documents += 1,
                    Package::App | Package::Other => {}
                }
                let i = self.push_item(item, here);
                let child = self.child(
                    job,
                    name,
                    Zone::Collapse {
                        item: i,
                        place: here,
                    },
                    None,
                );
                self.enqueue(child);
            }
            return;
        }
        if fresh {
            // The folder starts its own (narrower) place.
            self.enter_place_at(job, name, here, base, false);
            return;
        }
        // In the mover's home, each top folder is its own place ("code", "Projects").
        if in_home && job.rel.len() == base {
            let mut meta = self.state.meta[here].clone();
            meta.items = 0;
            meta.collapsed = None;
            meta.root_item = None;
            let p = self.new_place(
                job.volume,
                Some(path.to_string_lossy().into_owned()),
                around,
                Fate::Moves,
                meta,
            );
            self.enter_place_at(job, name, p, base, false);
            return;
        }
        let item = self.make_item(
            job,
            name,
            here,
            base,
            ItemKind::Folder,
            0,
            Inclusion::Included,
        );
        let item = item.map(|i| self.push_item(i, here));
        let child = self.child(job, name, Zone::Itemise { base, place: here }, item);
        self.enqueue(child);
    }

    /// Below a shared place on a laptop with two or more people: the placement for something
    /// inside it, when it is narrower than the place around (fewer people could open it).
    fn narrower(
        &mut self,
        _job: &Job,
        path: &Path,
        unix: Option<UnixMode>,
        here: usize,
        in_home: bool,
    ) -> Option<Placement> {
        let around = self.state.places[here].placement.clone();
        if in_home
            || !matches!(around.whose, Whose::Shared { .. })
            || !self.several()
            || !self.facts.volumes[_job.volume].keeps_owners()
        {
            return None;
        }
        let fact = self.read_access(path, unix);
        let placement = place_by_ladder(
            &Facts {
                rule: None,
                home: None,
                access: &fact,
                around: None,
            },
            &Ladder {
                census: &self.state.census,
            },
        );
        let whose = narrowed(&placement.whose, &around.whose, &self.state.census.me)?;
        Some(Placement {
            whose,
            kind: None,
            rule: placement.rule,
        })
    }

    /// Goes into a folder that starts place `p`, below `base`.
    fn enter_place_at(
        &mut self,
        job: &Job,
        name: &ItemName,
        p: usize,
        base: usize,
        collapse: bool,
    ) {
        if self.state.meta[p].private {
            let reason = self.private_reason(p);
            self.blind(job.volume, None, reason);
            return;
        }
        if self.state.meta[p].owner.is_none() {
            let child = self.child(job, name, Zone::Count { place: p }, None);
            self.enqueue(child);
            return;
        }
        let Some(item) =
            self.make_item(job, name, p, base, ItemKind::Folder, 0, Inclusion::Included)
        else {
            let child = self.child(job, name, Zone::Count { place: p }, None);
            self.enqueue(child);
            return;
        };
        let i = self.push_item(item, p);
        self.state.meta[p].root_item = Some(i);
        let zone = if collapse {
            Zone::Collapse { item: i, place: p }
        } else {
            Zone::Itemise { base, place: p }
        };
        let child = self.child(job, name, zone, Some(i));
        self.enqueue(child);
    }

    fn file(&mut self, job: &Job, entry: &Entry, name: &ItemName, place: usize, base: usize) {
        let text = name.display().to_string();
        let stub = is_icloud_stub_name(&text);
        let cloud_only = stub || entry.cloud_only;
        if !cloud_only {
            self.count_entry(job.volume, place, entry);
        } else {
            self.state.places[place].files += 1;
        }
        if self.state.meta[place].owner.is_none() {
            return;
        }
        let category_name = if stub {
            text.trim_start_matches('.')
                .trim_end_matches(".icloud")
                .to_string()
        } else {
            text.clone()
        };
        count(category(&category_name), &mut self.state.counts);
        let inclusion = if cloud_only {
            left_out(LeftOutReason::CloudOnly)
        } else {
            Inclusion::Included
        };
        let size = if stub { 0 } else { entry.len };
        if let Some(item) = self.make_item(job, name, place, base, ItemKind::File, size, inclusion)
        {
            if let (false, Some(stamp)) = (stub, entry.stamp) {
                self.state.stamps.push((item.id, stamp));
            }
            self.push_item(item, place);
        }
    }

    /// A place that starts at a folder and holds more items than the limit is counted as one
    /// item (that folder) from here on: the record that lists any number of items comes in a
    /// later version.
    fn maybe_collapse(&mut self, place: usize) {
        let meta = &self.state.meta[place];
        let Some(root) = meta.root_item else {
            return;
        };
        if meta.items <= self.limits.place_items || meta.collapsed.is_some() {
            return;
        }
        let mut total = 0u64;
        let mut inside = Vec::new();
        for (i, p) in self.state.item_place.iter().enumerate() {
            if *p == place && i != root && !self.state.removed.contains(&i) {
                let item = &self.state.record.items[i];
                if item.inclusion == Inclusion::Included {
                    total += item.size_bytes;
                }
                inside.push(i);
            }
        }
        self.state.removed.extend(inside);
        let item = &mut self.state.record.items[root];
        item.size_bytes = total;
        item.inclusion = Inclusion::Included;
        self.state.meta[place].collapsed = Some(root);
    }

    fn app_data(&mut self, job: &Job, entries: Vec<Entry>, home: usize, place: usize) {
        let roots = [
            "Local",
            "Roaming",
            "LocalLow",
            "Application Support",
            "Containers",
            "Group Containers",
            "share",
        ];
        let home_parts: Vec<String> = job
            .rel
            .get(home..)
            .unwrap_or(&[])
            .iter()
            .map(|n| n.display().to_string())
            .collect();
        let around = self.state.places[place].placement.clone();
        for entry in entries {
            let name = item_name(&entry.name);
            let text = name.display().to_string();
            let path = self.abs(job.volume, &job.rel).join(&entry.name);
            if self.skipped(job, &entry, &path)
                || self.known_paths.iter().any(|k| same_place(k, &path))
            {
                continue;
            }
            match entry.kind {
                Some(EntryKind::Link) => {
                    self.state.links += 1;
                    continue;
                }
                Some(EntryKind::File) => {
                    self.count_entry(job.volume, place, &entry);
                    continue;
                }
                None => continue,
                Some(EntryKind::Dir) => {}
            }
            if let Some(row) = self.facts.rules.in_home(&with(&home_parts, &text)) {
                let census = self.state.census.clone();
                let placement = place_with(row, &around, &census);
                let space = if row.kind == Kind::Junk {
                    Space::Junk
                } else {
                    Space::AppData
                };
                let p = self.new_place(
                    job.volume,
                    Some(path.to_string_lossy().into_owned()),
                    placement,
                    Fate::Counted,
                    counted_meta(space),
                );
                let child = self.child(job, &name, Zone::Count { place: p }, None);
                self.enqueue(child);
                continue;
            }
            if job.rel.len() == home + 1 && roots.iter().any(|r| r.eq_ignore_ascii_case(&text)) {
                let child = self.child(job, &name, Zone::AppData { home, place }, None);
                self.enqueue(child);
                continue;
            }
            // One program's folder: one counted place, "found, not moved in this version".
            let p = self.new_place(
                job.volume,
                Some(path.to_string_lossy().into_owned()),
                around.clone(),
                Fate::FoundNotMovedInThisVersion,
                counted_meta(Space::AppData),
            );
            let child = self.child(job, &name, Zone::Count { place: p }, None);
            self.enqueue(child);
        }
    }

    /// An older copy of Windows: counted as the system's, except its `Users` folder, which holds
    /// the old homes.
    fn old_system(&mut self, job: &Job, entries: Vec<Entry>, place: usize) {
        for entry in entries {
            let name = item_name(&entry.name);
            let path = self.abs(job.volume, &job.rel).join(&entry.name);
            if self.skipped(job, &entry, &path) {
                continue;
            }
            match entry.kind {
                Some(EntryKind::Dir) if name.display().eq_ignore_ascii_case("Users") => {
                    let child = self.child(job, &name, Zone::OldProfiles { place }, None);
                    self.enqueue(child);
                }
                Some(EntryKind::Dir) => {
                    let child = self.child(job, &name, Zone::Count { place }, None);
                    self.enqueue(child);
                }
                Some(EntryKind::File) => self.count_entry(job.volume, place, &entry),
                Some(EntryKind::Link) => self.state.links += 1,
                None => self.system_corner(job.volume),
            }
        }
    }

    /// The homes in an older copy of Windows: each is placed by its owner (the mover's own stays
    /// the mover's; anyone else's is counted, never named, "From an old account").
    fn old_profiles(&mut self, job: &Job, entries: Vec<Entry>, place: usize) {
        for entry in entries {
            let name = item_name(&entry.name);
            let path = self.abs(job.volume, &job.rel).join(&entry.name);
            if self.skipped(job, &entry, &path) {
                continue;
            }
            match entry.kind {
                Some(EntryKind::Dir) => {
                    let text = name.display().to_string();
                    if SYSTEM_PROFILES
                        .iter()
                        .any(|n| n.eq_ignore_ascii_case(&text))
                    {
                        let child = self.child(job, &name, Zone::Count { place }, None);
                        self.enqueue(child);
                    } else if text.eq_ignore_ascii_case("Public") {
                        // The old shared folder: everyone's things, placed like any folder.
                        let child = self.child(job, &name, Zone::Explore { inside: None }, None);
                        self.enqueue(child);
                    } else {
                        self.unexplained_home(job, &entry, &name, &path);
                    }
                }
                Some(EntryKind::File) => self.count_entry(job.volume, place, &entry),
                Some(EntryKind::Link) => self.state.links += 1,
                None => self.system_corner(job.volume),
            }
        }
    }

    /// Notes a corner of the system no one but the system may look into.
    fn system_corner(&mut self, volume: usize) {
        self.state.vols[volume]
            .lines
            .entry(Space::SystemKeepsToItself)
            .or_insert((0, false))
            .1 = false;
    }

    fn count(&mut self, job: &Job, entries: Vec<Entry>, place: usize) {
        for entry in entries {
            let path = self.abs(job.volume, &job.rel).join(&entry.name);
            if self.skipped(job, &entry, &path)
                || self.known_paths.iter().any(|k| same_place(k, &path))
            {
                continue;
            }
            self.note_owner(&entry);
            match entry.kind {
                Some(EntryKind::Dir) => {
                    let name = item_name(&entry.name);
                    let child = self.child(job, &name, Zone::Count { place }, None);
                    self.enqueue(child);
                }
                Some(EntryKind::File) => {
                    if !(entry.cloud_only || is_icloud_stub_name(&entry.name.to_string_lossy())) {
                        self.count_entry(job.volume, place, &entry);
                    }
                }
                Some(EntryKind::Link) => self.state.links += 1,
                None => {
                    // Something whose details could not be read: never dropped silently. In
                    // another person's place it is their blind spot (no name); in the system's
                    // it is the system's own corner; otherwise it is listed.
                    let meta = &self.state.meta[place];
                    if meta.private {
                        let job = Job {
                            zone: Zone::Count { place },
                            ..job.clone()
                        };
                        self.refused(&job, Refusal::NoPermission);
                    } else if matches!(meta.space, Space::TheSystem | Space::Apps | Space::Junk) {
                        self.state.vols[job.volume]
                            .lines
                            .entry(Space::SystemKeepsToItself)
                            .or_insert((0, false))
                            .1 = false;
                    } else {
                        self.blind(
                            job.volume,
                            Some(path.to_string_lossy().into_owned()),
                            BlindReason::NoPermission,
                        );
                    }
                }
            }
        }
    }

    fn collapse(&mut self, job: &Job, entries: Vec<Entry>, item: usize, place: usize) {
        for entry in entries {
            let path = self.abs(job.volume, &job.rel).join(&entry.name);
            if self.skipped(job, &entry, &path)
                || self.known_paths.iter().any(|k| same_place(k, &path))
            {
                continue;
            }
            match entry.kind {
                Some(EntryKind::Dir) => {
                    let name = item_name(&entry.name);
                    let child = self.child(job, &name, Zone::Collapse { item, place }, None);
                    self.enqueue(child);
                }
                Some(EntryKind::File) => {
                    if !(entry.cloud_only || is_icloud_stub_name(&entry.name.to_string_lossy())) {
                        let fresh = entry.key.is_none_or(|k| !self.state.seen_keys.contains(&k));
                        self.count_entry(job.volume, place, &entry);
                        if fresh {
                            self.state.record.items[item].size_bytes += entry.len;
                        }
                    }
                }
                Some(EntryKind::Link) => self.state.links += 1,
                None => {
                    self.blind(
                        job.volume,
                        Some(path.to_string_lossy().into_owned()),
                        BlindReason::NoPermission,
                    );
                }
            }
        }
    }

    // --- the end -----------------------------------------------------------------------

    fn finish(mut self, finished: bool) -> Sweep {
        let frontier = (!finished).then(|| self.view(Vec::new()).save());
        // Volumes that share one file system's space (btrfs sub-volumes mounted apart, APFS
        // volumes in one container) all report the same used space: they are judged together,
        // on the one mounted highest, against everything seen on any of them.
        let shares = |a: &Volume, b: &Volume| {
            a.kind == VolumeKind::Internal
                && b.kind == VolumeKind::Internal
                && a.total_bytes > 0
                && a.total_bytes == b.total_bytes
                && a.free_bytes == b.free_bytes
                && a.file_system.eq_ignore_ascii_case(&b.file_system)
        };
        let n = self.facts.volumes.len();
        let vols = &self.facts.volumes;
        let lead: Vec<usize> = (0..n)
            .map(|i| {
                (0..n)
                    .filter(|&j| j == i || shares(&vols[i], &vols[j]))
                    .min_by_key(|&j| (vols[j].mount.components().count(), j))
                    .unwrap_or(i)
            })
            .collect();
        let mut group_seen = vec![0u64; n];
        for i in 0..n {
            group_seen[lead[i]] += self.state.vols[i].seen;
        }
        let mut volumes = Vec::new();
        for (i, v) in self.facts.volumes.iter().enumerate() {
            let seen = &self.state.vols[i];
            let used = v.used_bytes();
            let walked = seen.walked && v.kind == VolumeKind::Internal;
            let unexplained = if walked && lead[i] == i {
                used.saturating_sub(group_seen[i])
            } else {
                0
            };
            let flagged = walked && unexplained > UNEXPLAINED_FLOOR.max(used / 10);
            if flagged && finished {
                add_blind(
                    &mut self.state.blind,
                    BlindSpot {
                        volume: Some(v.label()),
                        path: None,
                        reason: BlindReason::Unexplained { bytes: unexplained },
                        count: 1,
                    },
                );
            }
            volumes.push(VolumeReport {
                mount: v.label(),
                name: v.name.clone(),
                kind: v.kind,
                file_system: v.file_system.clone(),
                is_system: v.is_system,
                walked,
                walkers: seen.walkers,
                used_bytes: used,
                seen_bytes: seen.seen,
                lines: seen
                    .lines
                    .iter()
                    .map(|(space, (bytes, known))| SpaceLine {
                        space: *space,
                        bytes: known.then_some(*bytes),
                    })
                    .collect(),
                unexplained_bytes: unexplained,
                flagged,
                same_space_as: (lead[i] != i).then(|| self.facts.volumes[lead[i]].label()),
            });
        }
        let removed = &self.state.removed;
        let mut items = Vec::new();
        let mut placed_by = Vec::new();
        for (i, item) in self.state.record.items.iter().enumerate() {
            if removed.contains(&i) {
                continue;
            }
            placed_by.push((item.id, self.state.item_place[i]));
            items.push(item.clone());
        }
        // The same item can be reached twice only through a path the system reports twice;
        // keep the first.
        let mut ids = HashSet::new();
        let mut keep = Vec::new();
        let mut keep_placed = Vec::new();
        for (item, placed) in items.into_iter().zip(placed_by) {
            if ids.insert(item.id) {
                keep.push(item);
                keep_placed.push(placed);
            }
        }
        let mut record = self.state.record.clone();
        record.items = keep;
        let blind_spots: Vec<BlindSpot> = self
            .state
            .blind
            .iter()
            .map(|(k, n)| BlindSpot {
                volume: k.volume.clone(),
                path: k.path.clone(),
                reason: k.reason.clone(),
                count: *n,
            })
            .collect();
        let places: u64 = blind_spots.iter().map(|b| b.count).sum();
        let top_line = if !finished {
            TopLine::DidNotFinish
        } else if places == 0 {
            TopLine::Everything
        } else {
            TopLine::AccountedForExcept {
                places: usize::try_from(places).unwrap_or(usize::MAX),
            }
        };
        Sweep {
            census: self.state.census.clone(),
            volumes,
            places: self.state.places.clone(),
            blind_spots,
            record,
            placed_by: keep_placed,
            counts: self.state.counts,
            links: self.state.links,
            finished,
            top_line,
            frontier,
        }
    }
}

/// Who could open a place, as a set.
enum Readers {
    Everyone,
    These(BTreeSet<String>),
    /// This person can't judge (a gone account's, unreadable access).
    NotJudged,
}

fn readers(w: &Whose, me: &str) -> Readers {
    match w {
        Whose::Mover => Readers::These(BTreeSet::from([me.to_string()])),
        Whose::Person { account_id } => Readers::These(BTreeSet::from([account_id.clone()])),
        Whose::Shared { limited_to } if limited_to.is_empty() => Readers::Everyone,
        Whose::Shared { limited_to } => Readers::These(limited_to.iter().cloned().collect()),
        Whose::System => Readers::Everyone,
        Whose::FromOldAccount { .. } | Whose::Undecided => Readers::NotJudged,
    }
}

/// Below the shared place `parent`, whose `child` is: only the people both lists allow (it lands
/// where no more people can open it than could before, and no fewer than its own list
/// allows of the place's people). `None` when that is the place's own answer.
fn narrowed(child: &Whose, parent: &Whose, me: &str) -> Option<Whose> {
    if !matches!(parent, Whose::Shared { .. }) {
        return None;
    }
    let around = readers(parent, me);
    let own = match readers(child, me) {
        Readers::NotJudged => return Some(child.clone()),
        r => r,
    };
    let both: BTreeSet<String> = match (&around, &own) {
        (Readers::Everyone, Readers::Everyone) => return None,
        (Readers::Everyone, Readers::These(c)) => c.clone(),
        (Readers::These(_), Readers::Everyone) => return None,
        (Readers::These(p), Readers::These(c)) => {
            let both: BTreeSet<String> = p.intersection(c).cloned().collect();
            if both == *p {
                return None;
            }
            if both.is_empty() {
                // Nobody the place allows may open it: someone else's, never listed here.
                return Some(match c.iter().next() {
                    Some(only) if c.len() == 1 => Whose::Person {
                        account_id: only.clone(),
                    },
                    _ => Whose::Undecided,
                });
            }
            both
        }
        (Readers::NotJudged, _) | (_, Readers::NotJudged) => return None,
    };
    let mut ids = both.into_iter();
    Some(match (ids.next(), ids.next()) {
        (Some(one), None) if one == me => Whose::Mover,
        (Some(one), None) => Whose::Person { account_id: one },
        (Some(a), Some(b)) => {
            let mut limited_to = vec![a, b];
            limited_to.extend(ids);
            Whose::Shared { limited_to }
        }
        (None, _) => Whose::Undecided,
    })
}

fn with<'a>(parts: &'a [String], last: &'a str) -> Vec<&'a str> {
    parts
        .iter()
        .map(String::as_str)
        .chain(std::iter::once(last))
        .collect()
}

fn place_with(row: &crate::rules::Row, around: &Placement, census: &Census) -> Placement {
    place_by_ladder(
        &Facts {
            rule: Some(row),
            home: None,
            access: &AccessFact::Unknown {
                why: "inside a placed folder".into(),
            },
            around: Some(around),
        },
        &Ladder { census },
    )
}

fn counted_meta(space: Space) -> PlaceMeta {
    PlaceMeta {
        record_place: None,
        owner: None,
        space,
        private: false,
        items: 0,
        collapsed: None,
        root_item: None,
    }
}

fn space_of(kind: Option<Kind>) -> Space {
    match kind {
        Some(Kind::Rebuildable) => Space::Rebuildable,
        Some(Kind::AppData) => Space::AppData,
        Some(Kind::Junk) => Space::Junk,
        Some(Kind::System | Kind::OldSystem) => Space::TheSystem,
        Some(Kind::App) => Space::Apps,
        Some(Kind::KeepWhole) | None => Space::People,
    }
}

fn add_counts(total: &mut Counts, more: &Counts) {
    total.photos += more.photos;
    total.videos += more.videos;
    total.music += more.music;
    total.documents += more.documents;
    total.other += more.other;
}

/// The special folders scanned on their own: the home holds the others, and a folder inside
/// another is scanned as part of the outer one.
fn outer_known(folders: &[FolderLookup]) -> Vec<(usize, PathBuf)> {
    let found: Vec<(usize, PathBuf)> = folders
        .iter()
        .enumerate()
        .filter_map(|(i, f)| match f {
            FolderLookup::Found(found) if found.role != FolderRole::Home => {
                Some((i, found.path.clone()))
            }
            _ => None,
        })
        .collect();
    found
        .iter()
        .filter(|(_, p)| {
            !found
                .iter()
                .any(|(_, other)| other != p && is_within(p, other))
        })
        .cloned()
        .collect()
}

/// The path the system lists a folder through. On Windows that is the `\\?\` form, which takes
/// every name exactly as it is on the disk: a folder named `notes.` or `CON` is listed under its
/// real name instead of one Windows tidied up (which would not exist).
pub(crate) fn for_listing(path: &Path) -> PathBuf {
    #[cfg(windows)]
    {
        use std::os::windows::ffi::{OsStrExt, OsStringExt};
        // The `\\?\` form takes no `/`: turn them round first (name by name, exactly).
        let wide: Vec<u16> = path
            .as_os_str()
            .encode_wide()
            .map(|u| {
                if u == u16::from(b'/') {
                    u16::from(b'\\')
                } else {
                    u
                }
            })
            .collect();
        let path = PathBuf::from(std::ffi::OsString::from_wide(&wide));
        let text = path.as_os_str().to_string_lossy();
        if text.starts_with(r"\\?\") || text.starts_with(r"\\.\") {
            return path;
        }
        if let Some(rest) = text.strip_prefix(r"\\") {
            return PathBuf::from(format!(r"\\?\UNC\{rest}"));
        }
        if path.is_absolute() {
            let mut long = std::ffi::OsString::from(r"\\?\");
            long.push(path.as_os_str());
            return PathBuf::from(long);
        }
        path
    }
    #[cfg(not(windows))]
    {
        path.to_path_buf()
    }
}

#[cfg(unix)]
fn dev_of(path: &Path) -> Option<u64> {
    use std::os::unix::fs::MetadataExt;
    std::fs::symlink_metadata(path).ok().map(|m| m.dev())
}

#[cfg(not(unix))]
fn dev_of(_path: &Path) -> Option<u64> {
    None
}

/// The root of the drive a remembered path is on (`E:\`, `/Volumes/Backup`, `/media/ada/USB`).
fn drive_root(path: &Path) -> Option<PathBuf> {
    if cfg!(windows) {
        let mut c = path.components();
        let prefix = c.next()?;
        Some(PathBuf::from(format!(
            "{}\\",
            prefix.as_os_str().to_string_lossy()
        )))
    } else {
        let parts: Vec<_> = path.components().collect();
        let text: Vec<String> = parts
            .iter()
            .map(|c| c.as_os_str().to_string_lossy().into_owned())
            .collect();
        let take = match text.get(1).map(String::as_str) {
            Some("Volumes") | Some("mnt") => 3,
            Some("media") => 4,
            Some("run") if text.get(2).map(String::as_str) == Some("media") => 5,
            _ => return None,
        };
        (parts.len() >= take).then(|| parts[..take].iter().collect())
    }
}
