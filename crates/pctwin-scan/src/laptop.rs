use std::path::PathBuf;

use pctwin_record::{FolderRole, Inclusion, ItemKind, LaptopId, Owner, Place, Record};
use serde::Serialize;

use crate::drives::{Drive, list_drives};
use crate::folders::{FolderLookup, find_special_folders, is_within};
use crate::measure::Size;
use crate::people::{Person, list_people};
use crate::scan::{Counts, scan_folder};

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
}

/// Why there is no scan.
#[derive(Debug, thiserror::Error)]
pub enum ScanError {
    #[error("PCTwin could not tell which account is signed in")]
    NoSignedInPerson,
}

/// Scans this laptop: who is on it, the signed-in person's special folders, the shared folder and
/// the drives. Only reads; no file is opened.
pub fn scan_this_laptop(laptop: LaptopId) -> Result<LaptopScan, ScanError> {
    build_scan(laptop, list_people(), find_special_folders(), list_drives())
}

/// Builds the scan from what was found. Other people's folders are not scanned (that needs their
/// sign-in or the administrator prompt); their size is shown only where the system lets it be
/// read.
pub fn build_scan(
    laptop: LaptopId,
    people: Vec<Person>,
    folders: Vec<FolderLookup>,
    drives: Vec<Drive>,
) -> Result<LaptopScan, ScanError> {
    let me = people
        .iter()
        .find(|p| p.is_me)
        .ok_or(ScanError::NoSignedInPerson)?
        .clone();
    let my_owner = Owner::Person {
        account_id: me.account_id.clone(),
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

    let mut record = Record::new(laptop);
    let mut counts = Counts::default();
    let mut links = 0;
    let mut unreadable = Vec::new();
    for folder in outer {
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
        counts.photos += scan.counts.photos;
        counts.videos += scan.counts.videos;
        counts.music += scan.counts.music;
        counts.documents += scan.counts.documents;
        counts.other += scan.counts.other;
        links += scan.links;
        unreadable.extend(scan.unreadable);
        record.items.extend(scan.items);
    }

    // What is theirs to move: included files and whole packages, not folder entries or anything
    // left out.
    let (bytes, files) = record
        .items
        .iter()
        .filter(|i| i.owner == my_owner && i.inclusion == Inclusion::Included)
        .filter(|i| !(i.kind == ItemKind::Folder && i.size_bytes == 0))
        .fold((0, 0), |(b, f), i| (b + i.size_bytes, f + 1));
    let my_size = if unreadable.is_empty() {
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
        record,
        counts,
        links,
        unreadable,
    })
}
