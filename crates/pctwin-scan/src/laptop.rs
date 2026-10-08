use std::path::PathBuf;

use pctwin_record::{FolderRole, Inclusion, ItemKind, LaptopId, Owner, Place, Record};
use serde::Serialize;

use crate::drives::{Drive, list_drives};
use crate::folders::{FolderLookup, find_special_folders, is_within};
use crate::measure::Size;
use crate::people::{Person, list_people};
use crate::scan::{Counts, scan_folder};
use crate::state::{DoneFolder, ScanState};

/// One person and how much they have, as honestly as can be told.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct PersonSummary {
    pub person: Person,
    pub size: Size,
}

/// Everything the scan found on the old laptop.
#[derive(Debug, Clone)]
pub struct LaptopScan {
    pub people: Vec<PersonSummary>,
    pub folders: Vec<FolderLookup>,
    pub drives: Vec<Drive>,
    /// The signed-in person's special folders and the shared ones, as move-record items.
    pub record: Record,
    pub counts: Counts,
    pub links: u64,
    pub unreadable: Vec<PathBuf>,
    /// What to save so a restart can resume and a later scan can tell what changed.
    pub state: ScanState,
}

/// Why there is no scan.
#[derive(Debug, thiserror::Error)]
pub enum ScanError {
    #[error("PCTwin could not tell which account is signed in")]
    NoSignedInPerson,
}

/// Whether to go on after a folder is saved.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Flow {
    Continue,
    Stop,
}

/// Scans this laptop: who is on it, the signed-in person's special folders, the shared folder and
/// the drives. Only reads; no file is opened.
pub fn scan_this_laptop(laptop: LaptopId) -> Result<LaptopScan, ScanError> {
    build_scan(laptop, list_people(), find_special_folders(), list_drives())
}

/// Builds the scan from what was found, in one go.
pub fn build_scan(
    laptop: LaptopId,
    people: Vec<Person>,
    folders: Vec<FolderLookup>,
    drives: Vec<Drive>,
) -> Result<LaptopScan, ScanError> {
    build_scan_resumable(laptop, people, folders, drives, None, &mut |_| {
        Flow::Continue
    })
}

/// Builds the scan, calling `after_folder` with the state to save each time a folder is done; it
/// can stop the scan there. With `previous` (an unfinished scan of the same laptop by the same
/// person), folders already done are not scanned again. Other people's folders are not scanned
/// (that needs their sign-in); their size is shown only where the system lets it be read.
pub fn build_scan_resumable(
    laptop: LaptopId,
    people: Vec<Person>,
    folders: Vec<FolderLookup>,
    drives: Vec<Drive>,
    previous: Option<&ScanState>,
    after_folder: &mut dyn FnMut(&ScanState) -> Flow,
) -> Result<LaptopScan, ScanError> {
    let me = people
        .iter()
        .find(|p| p.is_me)
        .ok_or(ScanError::NoSignedInPerson)?
        .clone();
    let my_owner = Owner::Person {
        account_id: me.account_id.clone(),
    };
    let mut state = match previous {
        Some(p) if p.laptop == laptop && p.person == me.account_id && !p.finished => p.clone(),
        _ => ScanState::new(laptop, me.account_id.clone()),
    };

    // The home folder holds the others, so it is not scanned on its own; a special folder inside
    // another one (Pictures inside Documents) is scanned once, as part of the outer one.
    let found: Vec<_> = folders
        .iter()
        .filter_map(|f| match f {
            FolderLookup::Found(found) if found.role != FolderRole::Home => Some(found),
            _ => None,
        })
        .collect();
    let outer: Vec<_> = found
        .iter()
        .filter(|f| {
            !found
                .iter()
                .any(|other| other.path != f.path && is_within(&f.path, &other.path))
        })
        .collect();

    let mut stopped = false;
    for folder in outer {
        let path_text = folder.path.to_string_lossy().into_owned();
        if state.done.iter().any(|d| d.path == path_text) {
            continue;
        }
        let owner = if folder.role == FolderRole::Public {
            Owner::Shared
        } else {
            my_owner.clone()
        };
        let place = Place {
            role: folder.role,
            storage: folder.storage.clone(),
        };
        let scan = scan_folder(&laptop, &owner, &place, &folder.path);
        state.record.items.extend(scan.items);
        state.stamps.extend(scan.stamps);
        state.done.push(DoneFolder {
            role: folder.role,
            path: path_text,
            counts: scan.counts,
            links: scan.links,
            unreadable: scan
                .unreadable
                .iter()
                .map(|p| p.to_string_lossy().into_owned())
                .collect(),
        });
        if after_folder(&state) == Flow::Stop {
            stopped = true;
            break;
        }
    }
    state.finished = !stopped;

    let mut counts = Counts::default();
    let mut links = 0;
    let mut unreadable = Vec::new();
    for d in &state.done {
        counts.photos += d.counts.photos;
        counts.videos += d.counts.videos;
        counts.music += d.counts.music;
        counts.documents += d.counts.documents;
        counts.other += d.counts.other;
        links += d.links;
        unreadable.extend(d.unreadable.iter().map(PathBuf::from));
    }

    // What is theirs to move: included files and whole packages, not folder entries or anything
    // left out.
    let (bytes, files) = state
        .record
        .items
        .iter()
        .filter(|i| i.owner == my_owner && i.inclusion == Inclusion::Included)
        .filter(|i| !(i.kind == ItemKind::Folder && i.size_bytes == 0))
        .fold((0, 0), |(b, f), i| (b + i.size_bytes, f + 1));
    let my_size = if unreadable.is_empty() && state.finished {
        Size::Known { bytes, files }
    } else {
        Size::AtLeast { bytes, files }
    };
    let summaries = people
        .into_iter()
        .map(|person| PersonSummary {
            size: if person.is_me {
                my_size
            } else {
                Size::of(&person.home)
            },
            person,
        })
        .collect();

    Ok(LaptopScan {
        people: summaries,
        folders,
        drives,
        record: state.record.clone(),
        counts,
        links,
        unreadable,
        state,
    })
}
