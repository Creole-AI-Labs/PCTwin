//! `ScanPerson`: an approved person's files, found by the same sweep that finds the mover's,
//! run as if that person were the one moving. Only what the sweep places as theirs is kept;
//! shared things are already the mover's to move. Their names and places stay in the helper.

use std::path::PathBuf;
use std::sync::atomic::AtomicBool;

use pctwin_record::{Inclusion, Item, ItemKind, LaptopId, Owner};
use pctwin_scan::{Census, PersonKind, Rules, Sweep, SweepFacts, SweepLimits};

use crate::protocol::Refusal;
use crate::read::Entry;

/// `census` as seen by `account`, another real person with a home folder on this laptop: they
/// become the one moving, the mover one of the others (`None`: no such person).
pub fn census_as(mut census: Census, account: &str) -> Option<Census> {
    let i = census.residents.iter().position(|r| {
        r.account_id.as_deref() == Some(account) && r.kind == PersonKind::Other && r.home.is_some()
    })?;
    for r in &mut census.residents {
        if r.kind == PersonKind::Me {
            r.kind = PersonKind::Other;
        }
    }
    census.residents[i].kind = PersonKind::Me;
    census.me = account.to_string();
    Some(census)
}

/// The files `sweep` (run as `account`) placed as `account`'s own and includes: each as a record
/// item, and where it is and as the sweep saw it.
pub fn entries_of(sweep: &Sweep, account: &str) -> Vec<(Item, Entry)> {
    let owner = Owner::Person {
        account_id: account.to_string(),
    };
    let place_of: std::collections::HashMap<_, _> = sweep.placed_by.iter().copied().collect();
    sweep
        .record
        .items
        .iter()
        .filter(|i| {
            i.kind == ItemKind::File && i.inclusion == Inclusion::Included && i.owner == owner
        })
        .filter_map(|i| {
            let mut path: PathBuf = sweep.item_roots.get(place_of.get(&i.id)?)?.clone();
            for part in i.path.parts() {
                path.push(part.on_disk());
            }
            Some((
                i.clone(),
                Entry {
                    path,
                    size: i.size_bytes,
                    modified_ns: i.modified_ns,
                },
            ))
        })
        .collect()
}

/// Scans `account`'s files on this laptop (the helper runs with an administrator's rights).
pub fn scan_person(laptop: LaptopId, account: &str) -> Result<Vec<(Item, Entry)>, Refusal> {
    let census = census_as(pctwin_scan::census_this_laptop(), account).ok_or(Refusal::NoHome)?;
    let facts = SweepFacts {
        laptop,
        census,
        volumes: pctwin_scan::volumes_this_laptop(),
        // The system's list of special folders is the signed-in account's, not this person's:
        // their home is placed by the "inside a home" rule instead.
        known_folders: Vec::new(),
        remembered: Vec::new(),
        rules: Rules::shipped(),
    };
    let mut reader = pctwin_scan::system_reader();
    let stop = AtomicBool::new(false);
    let swept = pctwin_scan::sweep(
        facts,
        reader.as_mut(),
        &stop,
        &mut |_| {},
        &SweepLimits::default(),
        None,
    );
    Ok(entries_of(&swept, account))
}
