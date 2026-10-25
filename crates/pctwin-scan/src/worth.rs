//! How much each thing is worth, and which folders look like copies of each other: both from
//! what the scan already knows, both only to name, order and weigh. Worth never removes anything,
//! and a "looks like a copy" is only a suspicion: the move proves a copy only with the send-time
//! block fingerprints, within one person's things.

use std::collections::HashMap;

use pctwin_record::{Inclusion, Item, ItemId, ItemKind, ItemPath, LeftOutReason, Record, Storage};
use serde::{Deserialize, Serialize};

use crate::marks::MarkSample;
use crate::recognise::What;
use crate::rules::Kind;

/// How much something is worth to the person, most first.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum Worth {
    /// The person's own: lost if it is not moved.
    Irreplaceable,
    /// Can be fetched again (online-only files, installed programs and games).
    Redownloadable,
    /// Can be made again (downloaded packages, build output).
    Rebuildable,
    /// Caches, temporary files and bins.
    Junk,
}

/// Installers, which a download mark shows came from the internet and can come again.
const INSTALLERS: [&str; 8] = [
    ".exe",
    ".msi",
    ".msix",
    ".pkg",
    ".dmg",
    ".appimage",
    ".deb",
    ".rpm",
];

/// What `item` is worth, from what placed it (`kind`, the rules file's word), what the table
/// named it (`what`) and its folder's download-mark sample (`marks`). Anything the person made or
/// kept, unless something says otherwise, is irreplaceable: worth never guesses downwards. Only
/// an installer or disc image whose folder's sample is all downloaded is worth fetching again: a
/// downloaded letter is still the person's own.
pub fn worth(
    item: &Item,
    kind: Option<Kind>,
    what: Option<What>,
    marks: Option<MarkSample>,
) -> Worth {
    if let Inclusion::LeftOut {
        reason: LeftOutReason::CloudOnly,
    } = item.inclusion
    {
        return Worth::Redownloadable;
    }
    match kind {
        Some(Kind::Junk) => return Worth::Junk,
        Some(Kind::Rebuildable) => return Worth::Rebuildable,
        Some(Kind::System | Kind::App | Kind::OldSystem) => return Worth::Redownloadable,
        Some(Kind::AppData | Kind::KeepWhole) | None => {}
    }
    let downloaded = marks.is_some_and(|m| m.all_downloaded());
    let name = item
        .path
        .parts()
        .last()
        .map(|n| n.display().to_lowercase())
        .unwrap_or_default();
    match what {
        // Installed games download again; Outlook's offline mail comes back from the server.
        Some(What::InstalledGames | What::EmailCache) => Worth::Redownloadable,
        Some(What::DiscImage) if downloaded => Worth::Redownloadable,
        None if downloaded && INSTALLERS.iter().any(|s| name.ends_with(s)) => Worth::Redownloadable,
        _ => Worth::Irreplaceable,
    }
}

/// Whether `item` is found only on this laptop: not kept in a cloud folder and not online-only,
/// so if it is not moved it is gone. Such things are labelled "Only on this laptop" and move
/// before things also kept in a cloud.
pub fn only_on_this_laptop(item: &Item) -> bool {
    !matches!(item.place.storage, Storage::Cloud { .. })
        && item.inclusion
            != (Inclusion::LeftOut {
                reason: LeftOutReason::CloudOnly,
            })
}

/// Folders with at least this many files in them, all told, are compared; smaller ones match too
/// easily to mean anything.
pub const COPY_MIN_FILES: u64 = 3;

/// Folders of one person's things that look like copies of each other: the same names and sizes
/// all the way down (dates are not compared). Each group lists the folders, in the record's
/// order; the first is not "the original", only the first found.
pub fn look_like_copies(record: &Record) -> Vec<Vec<ItemId>> {
    // Each folder's children, by the folder's own ID: (name, size or the child folder's ID).
    enum Child<'a> {
        File(&'a str, u64),
        Folder(&'a str, ItemId),
    }
    let mut children: HashMap<ItemId, Vec<Child<'_>>> = HashMap::new();
    for i in &record.items {
        let parts = i.path.parts();
        let Some((name, parent)) = parts.split_last() else {
            continue;
        };
        let Ok(parent) = ItemPath::new(parent.to_vec()) else {
            continue;
        };
        let folder = ItemId::derive(&record.source_laptop, &i.owner, &i.place, &parent);
        let child = match i.kind {
            ItemKind::Folder => Child::Folder(name.display(), i.id),
            _ => Child::File(name.display(), i.size_bytes),
        };
        children.entry(folder).or_default().push(child);
    }
    // Deepest first, so every folder's children have their signatures before it.
    let mut folders: Vec<&Item> = record
        .items
        .iter()
        .filter(|i| i.kind == ItemKind::Folder)
        .collect();
    folders.sort_by_key(|i| std::cmp::Reverse(i.path.parts().len()));
    let mut signature: HashMap<ItemId, ([u8; 32], u64)> = HashMap::new();
    for f in &folders {
        let mut lines: Vec<String> = Vec::new();
        let mut files = 0u64;
        for c in children.get(&f.id).map(Vec::as_slice).unwrap_or_default() {
            match c {
                Child::File(name, size) => {
                    files += 1;
                    lines.push(format!("f {} {size}", name.to_lowercase()));
                }
                Child::Folder(name, id) => {
                    let (sig, n) = signature.get(id).copied().unwrap_or(([0; 32], 0));
                    files += n;
                    lines.push(format!(
                        "d {} {}",
                        name.to_lowercase(),
                        blake3::Hash::from(sig).to_hex()
                    ));
                }
            }
        }
        lines.sort();
        let mut h = blake3::Hasher::new_derive_key("pctwin 2026-10-21 folder signature");
        for l in &lines {
            h.update(l.as_bytes());
            h.update(b"\n");
        }
        signature.insert(f.id, (h.finalize().into(), files));
    }
    // Grouped within one person's things (owner), never across people.
    let mut groups: HashMap<(String, [u8; 32]), Vec<ItemId>> = HashMap::new();
    for f in &folders {
        let (sig, files) = signature[&f.id];
        if files >= COPY_MIN_FILES {
            groups
                .entry((format!("{:?}", f.owner), sig))
                .or_default()
                .push(f.id);
        }
    }
    let order: HashMap<ItemId, usize> = record
        .items
        .iter()
        .enumerate()
        .map(|(n, i)| (i.id, n))
        .collect();
    let mut out: Vec<Vec<ItemId>> = groups
        .into_values()
        .filter(|g| g.len() > 1)
        .map(|mut g| {
            g.sort_by_key(|id| order[id]);
            g
        })
        .collect();
    // A copy inside a copy is the outer copy's: keep only groups not wholly inside another's
    // folders.
    out.sort_by_key(|g| order[&g[0]]);
    let inside = |id: &ItemId, outer: &[ItemId]| {
        let item = &record.items[order[id]];
        outer.iter().any(|o| {
            let o = &record.items[order[o]];
            o.owner == item.owner
                && o.place == item.place
                && item.path.parts().len() > o.path.parts().len()
                && item.path.parts().starts_with(o.path.parts())
        })
    };
    let all = out.clone();
    out.retain(|g| {
        !all.iter()
            .any(|other| other != g && g.iter().all(|id| inside(id, other)))
    });
    out
}

#[cfg(test)]
mod tests {
    use super::*;
    use pctwin_record::{
        FolderRole, ItemName, LaptopId, ManagedBy, Owner, Place, Portability, Storage,
    };

    fn file(inclusion: Inclusion) -> Item {
        let owner = Owner::Shared;
        let place = Place {
            role: FolderRole::Documents,
            storage: Storage::SystemDrive,
        };
        let path = ItemPath::new(vec![ItemName::from_text("a.txt")]).unwrap();
        let laptop = LaptopId::from_hex("00112233445566778899aabbccddeeff").unwrap();
        Item {
            id: ItemId::derive(&laptop, &owner, &place, &path),
            kind: ItemKind::File,
            owner,
            place,
            path,
            size_bytes: 1,
            modified_ns: None,
            portability: Portability::Portable,
            managed_by: ManagedBy::Personal,
            download_bytes: None,
            inclusion,
        }
    }

    #[test]
    fn worth_comes_from_what_is_known_and_never_guesses_downwards() {
        let mine = file(Inclusion::Included);
        assert_eq!(worth(&mine, None, None, None), Worth::Irreplaceable);
        assert_eq!(worth(&mine, Some(Kind::Junk), None, None), Worth::Junk);
        assert_eq!(
            worth(&mine, Some(Kind::Rebuildable), None, None),
            Worth::Rebuildable
        );
        assert_eq!(
            worth(&mine, Some(Kind::App), None, None),
            Worth::Redownloadable
        );
        assert_eq!(
            worth(&mine, Some(Kind::KeepWhole), None, None),
            Worth::Irreplaceable
        );
        assert_eq!(
            worth(&mine, None, Some(What::InstalledGames), None),
            Worth::Redownloadable
        );
        // Outlook's offline mail comes back from the server; an archive and a vault do not.
        assert_eq!(
            worth(&mine, None, Some(What::EmailCache), None),
            Worth::Redownloadable
        );
        for own in [What::EmailArchive, What::PasswordVault, What::CodeProject] {
            assert_eq!(worth(&mine, None, Some(own), None), Worth::Irreplaceable);
        }
        // An old backup is still the person's own.
        assert_eq!(
            worth(&mine, None, Some(What::TimeMachine), None),
            Worth::Irreplaceable
        );
        let online = file(Inclusion::LeftOut {
            reason: LeftOutReason::CloudOnly,
        });
        assert_eq!(worth(&online, None, None, None), Worth::Redownloadable);
    }

    fn named(name: &str) -> Item {
        let mut i = file(Inclusion::Included);
        i.path = ItemPath::new(vec![pctwin_record::ItemName::from_text(name)]).unwrap();
        i
    }

    #[test]
    fn only_a_downloaded_installer_is_worth_fetching_again() {
        let all = Some(MarkSample {
            looked: 3,
            marked: 3,
        });
        let some = Some(MarkSample {
            looked: 3,
            marked: 2,
        });
        let setup = named("Setup.EXE");
        assert_eq!(worth(&setup, None, None, all), Worth::Redownloadable);
        // Not every file in its folder came from the internet: it may be the person's own.
        assert_eq!(worth(&setup, None, None, some), Worth::Irreplaceable);
        assert_eq!(worth(&setup, None, None, None), Worth::Irreplaceable);
        // A downloaded letter is still the person's own.
        assert_eq!(
            worth(&named("letter.pdf"), None, None, all),
            Worth::Irreplaceable
        );
        assert_eq!(
            worth(&named("ubuntu.iso"), None, Some(What::DiscImage), all),
            Worth::Redownloadable
        );
        assert_eq!(
            worth(&named("family.iso"), None, Some(What::DiscImage), None),
            Worth::Irreplaceable
        );
    }

    #[test]
    fn only_what_no_cloud_keeps_is_only_on_this_laptop() {
        let mut local = file(Inclusion::Included);
        assert!(only_on_this_laptop(&local));
        local.place.storage = Storage::Cloud {
            provider: pctwin_record::CloudProvider::OneDrive,
        };
        assert!(!only_on_this_laptop(&local));
        let online = file(Inclusion::LeftOut {
            reason: LeftOutReason::CloudOnly,
        });
        assert!(!only_on_this_laptop(&online));
    }
}
