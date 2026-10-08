use std::collections::BTreeMap;

use pctwin_gate::{Approved, Destinations, GateError};
use pctwin_record::{CloudProvider, FolderRole, Item, Owner, Storage};
use pctwin_scan::FolderLookup;

/// The destination label for a role's folder on the new laptop.
pub fn role_label(role: FolderRole) -> &'static str {
    match role {
        FolderRole::Home => "home",
        FolderRole::Desktop => "desktop",
        FolderRole::Documents => "documents",
        FolderRole::Downloads => "downloads",
        FolderRole::Pictures => "pictures",
        FolderRole::Music => "music",
        FolderRole::Videos => "videos",
        FolderRole::Public => "public",
        FolderRole::AppData => "app-data",
        FolderRole::Other => "other",
    }
}

/// The new laptop's own special folders: which roles it has, and where each one really is.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct NewPlaces {
    roles: BTreeMap<FolderRole, Storage>,
}

impl NewPlaces {
    pub fn new(roles: BTreeMap<FolderRole, Storage>) -> Self {
        Self { roles }
    }

    pub fn has(&self, role: FolderRole) -> bool {
        self.roles.contains_key(&role)
    }
}

/// Approves each special folder the new laptop found as a destination labelled by its role (the
/// shared Public folder as shared, the rest as the signed-in person's), and returns what it has.
pub fn approve_new_places(
    table: &mut Destinations,
    found: &[FolderLookup],
) -> Result<NewPlaces, GateError> {
    let mut roles = BTreeMap::new();
    for f in found {
        let FolderLookup::Found(folder) = f else {
            continue;
        };
        let kind = if folder.role == FolderRole::Public {
            Approved::SharedFolder
        } else {
            Approved::MyFolders
        };
        table.approve(role_label(folder.role), kind, &folder.path)?;
        roles.insert(folder.role, folder.storage.clone());
    }
    Ok(NewPlaces { roles })
}

/// Where one item goes on the new laptop.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Landing {
    /// Copy it to `path` inside the approved destination `destination`.
    Copy { destination: String, path: String },
    /// Both laptops keep this folder in the same cloud service, which will bring it over; copying
    /// it as well would upload everything twice. The person can choose to copy it anyway.
    LeaveToCloud { provider: CloudProvider },
    /// Another person's (or the system's) things: only through the administrator helper.
    NeedsAdministrator,
    /// The new laptop has neither this folder nor a home folder to put it in.
    NoPlace,
}

/// Where `item` lands. `me` is the signed-in person's account ID on the old laptop; `fallback`
/// gives the folder (in the person's language) for a role the new laptop doesn't have, such as
/// "From your old laptop/Music".
pub fn landing_for(
    item: &Item,
    me: &str,
    places: &NewPlaces,
    fallback: &dyn Fn(FolderRole) -> String,
) -> Landing {
    let mine = matches!(&item.owner, Owner::Person { account_id } if account_id == me);
    let shared = item.owner == Owner::Shared;
    if !mine && !shared {
        return Landing::NeedsAdministrator;
    }
    let role = if shared {
        FolderRole::Public
    } else {
        item.place.role
    };
    let path = item
        .path
        .parts()
        .iter()
        .map(|n| n.display())
        .collect::<Vec<_>>()
        .join("/");
    match places.roles.get(&role) {
        Some(storage) => {
            if let (Storage::Cloud { provider: old }, Storage::Cloud { provider: new }) =
                (&item.place.storage, storage)
                && old == new
            {
                return Landing::LeaveToCloud { provider: *old };
            }
            Landing::Copy {
                destination: role_label(role).to_string(),
                path,
            }
        }
        None if places.has(FolderRole::Home) => Landing::Copy {
            destination: role_label(FolderRole::Home).to_string(),
            path: format!("{}/{path}", fallback(role)),
        },
        None => Landing::NoPlace,
    }
}
