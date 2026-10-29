//! What a move leaves on the new laptop for finishing it (Task List 2.7), kept when the move
//! ends and read by the finishing commands ([`crate::finish`]).
//!
//! One folder per move, named by its My PCTwin history entry ID, beside the history, in PCTwin's
//! own data folder on the new laptop:
//!
//! - `move.json` ([`Kept`]): the approved plan as it concerns the mover (their own and shared
//!   items; another person's items are counted, never kept by name), each of the mover's results,
//!   the apps, settings and app data lines, and the old laptop's identity;
//! - `goodbye.json`: the before-wipe list, saved each time it is built (Task List 2.7: "save a
//!   copy on the new laptop");
//! - `old-now.json` ([`OldNow`]): the old laptop as it was when it was last looked at again;
//! - `setup.json` ([`Marks`]): the person's own marks on the finish-your-setup list;
//! - `handover.json` ([`Handovers`]): what was recorded of each handover step.
//!
//! Every file is written to a new temporary name, flushed and moved into place, then read back.
//! Nothing here is ever removed: retained history is never deleted to save room (a product non-negotiable).

use std::path::{Path, PathBuf};

use pctwin_readiness::ledger::MoveId;
use pctwin_record::{ItemId, Owner, Record};
use pctwin_types::{ItemStatus, Reason};
use serde::de::DeserializeOwned;
use serde::{Deserialize, Serialize};
use tauri::{AppHandle, Manager, Runtime};

/// The format of what is kept.
pub const FORMAT: u32 = 1;
/// The largest file read back (a plan of half a million files is far below it).
const MAX_FILE: u64 = 512 * 1024 * 1024;

/// One of the mover's file results, as the move's report gave it.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct KeptResult {
    pub item: ItemId,
    pub status: ItemStatus,
    pub reason: Option<Reason>,
}

/// What a line of the apps, settings and app data said.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum LineKind {
    App,
    Settings,
    Special,
}

/// One line of the apps, settings and app data (the outcome's rebuilt lines).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct KeptLine {
    pub kind: LineKind,
    /// The app's or part's name, as the person knows it.
    pub name: String,
    pub count: u64,
    pub status: ItemStatus,
    pub reason: Option<Reason>,
    /// The line's notes, by their keys (`sign-in`, `licence-key`, ...).
    pub notes: Vec<String>,
}

/// Another person in the move: counted, and named only as the mover already saw them named.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct OtherPerson {
    pub account_id: String,
    pub name: Option<String>,
    pub items: u64,
}

/// What one move left for finishing it.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Kept {
    pub format: u32,
    pub history_id: String,
    pub move_id: MoveId,
    /// The mover's account ID on the old laptop.
    pub mover: String,
    pub started_at: u64,
    pub ended_at: u64,
    /// The approved plan, as it concerns the mover ([`mover_only`]).
    pub record: Record,
    /// Every person in the move besides the mover.
    pub others: Vec<OtherPerson>,
    pub results: Vec<KeptResult>,
    pub lines: Vec<KeptLine>,
    /// The move's undo book, when its apps and settings can be undone.
    pub undo_id: Option<String>,
    /// The places the scan looked in, by their keys.
    pub searched: Vec<String>,
}

/// The old laptop as it was when last looked at again.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct OldNow {
    pub at: u64,
    /// Its plan, as it concerns the mover ([`mover_only`]).
    pub record: Record,
    pub others: Vec<OtherPerson>,
}

/// The person's marks on the finish-your-setup list, by task ID (hex).
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct Marks {
    pub done: std::collections::BTreeMap<String, u64>,
}

/// One recorded handover step.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct HandoverEntry {
    pub action: pctwin_readiness::handover::Action,
    /// When the person confirmed it (and undo was closed).
    pub confirmed_at: u64,
    /// What the person said happened, and when (`None`: not said yet).
    pub result: Option<(pctwin_readiness::handover::ActionResult, u64)>,
}

/// Every handover step recorded for a move.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct Handovers {
    pub entries: Vec<HandoverEntry>,
}

/// The plan as it concerns the mover: their own items and shared ones. Another person's items
/// are left out (counted in [`others_of`]), so their file names are never kept for the mover.
pub fn mover_only(record: &Record, mover: &str) -> Record {
    let mut r = record.clone();
    r.items.retain(|i| match &i.owner {
        Owner::Person { account_id } => account_id == mover,
        Owner::Shared | Owner::System => true,
    });
    r.apps.found.retain(|a| match &a.owner {
        Owner::Person { account_id } => account_id == mover,
        Owner::Shared | Owner::System => true,
    });
    r
}

/// Whether an item of `owner` is the mover's own or shared.
pub fn mover_only_owner(owner: &Owner, mover: &str) -> bool {
    match owner {
        Owner::Person { account_id } => account_id == mover,
        Owner::Shared | Owner::System => true,
    }
}

/// Every other person with items in `record`, with how many, named as `named` names them.
pub fn others_of(
    record: &Record,
    mover: &str,
    named: &[pctwin_move::landing::NamedPerson],
) -> Vec<OtherPerson> {
    let mut by: std::collections::BTreeMap<String, u64> = Default::default();
    for i in &record.items {
        if let Owner::Person { account_id } = &i.owner
            && account_id != mover
        {
            *by.entry(account_id.clone()).or_default() += 1;
        }
    }
    by.into_iter()
        .map(|(account_id, items)| OtherPerson {
            name: named
                .iter()
                .find(|p| p.account_id == account_id)
                .map(|p| p.name.clone()),
            account_id,
            items,
        })
        .collect()
}

/// Whether `id` is a history entry ID ([`crate::history`]): digits and a dash only, so it never
/// names a folder outside this one.
pub fn plain_id(id: &str) -> bool {
    !id.is_empty() && id.len() <= 64 && id.bytes().all(|b| b.is_ascii_digit() || b == b'-')
}

/// The folder of every move's finishing records.
pub fn root<R: Runtime>(app: &AppHandle<R>) -> Option<PathBuf> {
    app.path().app_data_dir().ok().map(|d| d.join("finish"))
}

/// One move's folder.
pub fn folder(root: &Path, id: &str) -> Option<PathBuf> {
    plain_id(id).then(|| root.join(id))
}

/// Writes `value` as `name` in `folder`: a fresh temporary file, flushed, moved into place, read
/// back. An error unless what is there now is exactly it.
pub fn save<T: Serialize + DeserializeOwned + PartialEq>(
    folder: &Path,
    name: &str,
    value: &T,
) -> std::io::Result<()> {
    use std::io::Write;
    std::fs::create_dir_all(folder)?;
    let bytes = serde_json::to_vec(value).map_err(std::io::Error::other)?;
    let mut tag = [0u8; 8];
    getrandom::fill(&mut tag).map_err(|_| std::io::Error::other("no random source"))?;
    let tag: String = tag.iter().map(|b| format!("{b:02x}")).collect();
    let temp = folder.join(format!(".{name}.{tag}.part"));
    {
        let mut f = std::fs::OpenOptions::new()
            .write(true)
            .create_new(true)
            .open(&temp)?;
        f.write_all(&bytes)?;
        f.sync_all()?;
    }
    std::fs::rename(&temp, folder.join(name))?;
    if load::<T>(folder, name).as_ref() == Some(value) {
        Ok(())
    } else {
        Err(std::io::Error::other("what was kept did not read back"))
    }
}

/// Reads `name` in `folder` (`None`: not there, too large, or not readable).
pub fn load<T: DeserializeOwned>(folder: &Path, name: &str) -> Option<T> {
    let path = folder.join(name);
    let meta = std::fs::symlink_metadata(&path).ok()?;
    if !meta.is_file() || meta.len() > MAX_FILE {
        return None;
    }
    serde_json::from_slice(&std::fs::read(&path).ok()?).ok()
}

/// What a move left, by its history ID; a format from a newer PCTwin is not read.
pub fn kept(root: &Path, id: &str) -> Option<Kept> {
    let k: Kept = load(&folder(root, id)?, "move.json")?;
    (k.format <= FORMAT && k.history_id == id).then_some(k)
}

/// The history ID of the newest move kept here.
pub fn newest(root: &Path) -> Option<String> {
    let mut ids: Vec<(u128, String)> = std::fs::read_dir(root)
        .ok()?
        .flatten()
        .filter_map(|e| {
            let id = e.file_name().to_string_lossy().into_owned();
            let ms = id.split('-').next()?.parse::<u128>().ok()?;
            (plain_id(&id) && e.path().join("move.json").is_file()).then_some((ms, id))
        })
        .collect();
    ids.sort();
    ids.pop().map(|(_, id)| id)
}

/// Seconds since 1970 now.
pub fn now() -> u64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_secs())
        .unwrap_or(0)
}

#[cfg(test)]
mod tests {
    use super::*;
    use pctwin_record::{
        FolderRole, Inclusion, Item, ItemKind, ItemName, ItemPath, LaptopId, ManagedBy, Place,
        Portability, Storage,
    };

    fn item(owner: Owner, name: &str) -> Item {
        let laptop = LaptopId::from_hex("00112233445566778899aabbccddeeff").unwrap();
        let place = Place {
            role: FolderRole::Documents,
            storage: Storage::SystemDrive,
        };
        let path = ItemPath::new(vec![ItemName::from_text(name)]).unwrap();
        Item {
            id: ItemId::derive(&laptop, &owner, &place, &path),
            kind: ItemKind::File,
            owner,
            place,
            path,
            size_bytes: 1,
            modified_ns: Some(1),
            portability: Portability::Portable,
            managed_by: ManagedBy::Personal,
            download_bytes: None,
            inclusion: Inclusion::Included,
        }
    }

    fn person(id: &str) -> Owner {
        Owner::Person {
            account_id: id.into(),
        }
    }

    /// Another person's file names are never kept for the mover: counted only.
    #[test]
    fn other_people_s_items_are_counted_never_kept() {
        let mut r = Record::new(LaptopId::from_hex("00112233445566778899aabbccddeeff").unwrap());
        r.items = vec![
            item(person("1000"), "mine.txt"),
            item(Owner::Shared, "shared.txt"),
            item(person("1001"), "bea-diary.txt"),
            item(person("1001"), "bea-2.txt"),
        ];
        let kept = mover_only(&r, "1000");
        let json = serde_json::to_string(&kept).unwrap();
        assert!(!json.contains("bea"), "{json}");
        assert_eq!(kept.items.len(), 2);
        let named = [pctwin_move::landing::NamedPerson {
            account_id: "1001".into(),
            name: "Bea".into(),
        }];
        assert_eq!(
            others_of(&r, "1000", &named),
            [OtherPerson {
                account_id: "1001".into(),
                name: Some("Bea".into()),
                items: 2
            }]
        );
    }

    /// Only history IDs become folder names, never a path outside.
    #[test]
    fn only_history_ids_name_a_folder() {
        let root = Path::new("finish");
        assert!(folder(root, "1792627200000-0").is_some());
        for bad in ["", "../x", "a", "1/2", "C:\\x"] {
            assert!(folder(root, bad).is_none(), "{bad}");
        }
    }

    /// Saved, read back whole, no temporary file left; the newest move is found by its time.
    #[test]
    fn saved_records_read_back_and_the_newest_is_found() {
        let dir = tempfile::tempdir().unwrap();
        let root = dir.path();
        let a = folder(root, "100-0").unwrap();
        let b = folder(root, "200-1").unwrap();
        let marks = Marks {
            done: [("ab".to_string(), 5)].into(),
        };
        save(&a, "setup.json", &marks).unwrap();
        assert_eq!(load::<Marks>(&a, "setup.json"), Some(marks));
        assert_eq!(std::fs::read_dir(&a).unwrap().count(), 1);
        std::fs::create_dir_all(&b).unwrap();
        std::fs::write(a.join("move.json"), b"{}").unwrap();
        std::fs::write(b.join("move.json"), b"{}").unwrap();
        assert_eq!(newest(root).as_deref(), Some("200-1"));
        // A record that is not one is not read.
        assert!(kept(root, "200-1").is_none());
    }
}
