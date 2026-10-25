//! `ScanPerson`: an approved person's files, found by the same sweep that finds the mover's,
//! run as if that person were the one moving. One sweep serves every approved person of the
//! session ([`ScanOnce`], [`scan_people`]): the drives are walked once, not once per person. Only what the sweep places as theirs is kept;
//! shared things are already the mover's to move. Their names and places stay in the helper.

use std::collections::{BTreeMap, BTreeSet};
use std::path::PathBuf;
use std::sync::atomic::AtomicBool;
use std::time::{Duration, Instant};

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

/// One person's files as the helper found them, and how many of their places a Mac privacy
/// setting kept it out of (the helper needs Full Disk Access: the app then shows the steps).
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct PersonScan {
    pub found: Vec<(Item, Entry)>,
    pub privacy_blocked: u64,
    /// Places outside every home that only the administrators and the system may open, as the
    /// helper (which can read their permissions) found them. Kept in the helper, never sent: the
    /// mover asks about its own unreadable places and is told only yes or no for each
    /// (`Request::SystemOnly`).
    pub system_only: Vec<PathBuf>,
}

impl From<Vec<(Item, Entry)>> for PersonScan {
    fn from(found: Vec<(Item, Entry)>) -> Self {
        Self {
            found,
            privacy_blocked: 0,
            system_only: Vec::new(),
        }
    }
}

/// Scans `account`'s files on this laptop (the helper runs with an administrator's rights).
pub fn scan_person(laptop: LaptopId, account: &str) -> Result<PersonScan, Refusal> {
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
    // With an administrator's rights: on Windows, PowerShell by its system path and with nothing
    // of the starter's environment.
    #[cfg(windows)]
    let mut reader = match (
        pctwin_helper_os::system_folder(pctwin_helper_os::SystemFolder::Windows),
        pctwin_helper_os::minimal_env(),
    ) {
        (Ok(windows), Ok(env)) => pctwin_scan::system_reader_clean(
            windows.join(r"System32\WindowsPowerShell\v1.0\powershell.exe"),
            env,
        ),
        _ => return Err(Refusal::Unreadable),
    };
    #[cfg(not(windows))]
    let mut reader = pctwin_scan::system_reader();
    let stop = AtomicBool::new(false);
    let swept = pctwin_scan::sweep(
        facts,
        reader.as_mut(),
        &stop,
        &mut progress(),
        &SweepLimits::default(),
        None,
    );
    let privacy_blocked = swept
        .blind_spots
        .iter()
        .filter(|b| b.reason == pctwin_scan::BlindReason::MacPrivacy)
        .map(|b| b.count)
        .sum();
    Ok(PersonScan {
        found: entries_of(&swept, account),
        privacy_blocked,
        system_only: system_only(&swept),
    })
}

/// About once a minute while the helper sweeps, how far it has got, on the helper's own output
/// (counts only, never a name or a place): a sweep of a big drive takes many minutes with no
/// answer to send yet, and this says whether it is still moving or where it stopped.
fn progress() -> impl FnMut(&pctwin_scan::FrontierView<'_>) {
    let began = Instant::now();
    let mut said = began;
    move |view| {
        if said.elapsed() >= Duration::from_secs(60) {
            said = Instant::now();
            let so_far = view.found_so_far();
            eprintln!(
                "pctwin-helper: [{:>6.1} s] sweeping: {} files, {} bytes so far, {} folders waiting",
                began.elapsed().as_secs_f64(),
                so_far.files,
                so_far.bytes,
                view.pending()
            );
        }
    }
}

/// At most this many system-only places are told (each a folder's path, so the answer stays
/// small whatever the laptop holds).
pub const MAX_SYSTEM_ONLY: usize = 256;

/// The places `sweep` found to be the system's only by who may open them (owned by the
/// administrators or the system, nobody else allowed): never a rules-file corner, never inside a
/// home (the "inside a home" rule comes first), never a file.
pub fn system_only(sweep: &Sweep) -> Vec<PathBuf> {
    sweep
        .places
        .iter()
        .filter(|p| {
            p.placement.whose == pctwin_scan::Whose::System
                && p.placement.rule == pctwin_scan::Rule::OwnerOnly
        })
        .filter_map(|p| p.path.as_ref().map(PathBuf::from))
        .take(MAX_SYSTEM_ONLY)
        .collect()
}

/// Every approved person's files from one sweep of this laptop (the helper runs with an
/// administrator's rights): each approved person is placed and listed as a sweep made as them
/// would (see `pctwin_scan::sweep_people`), and each answer holds only that person's own files.
/// A person with no home here is refused, as one by one.
pub fn scan_people(
    laptop: LaptopId,
    accounts: &BTreeSet<String>,
) -> BTreeMap<String, Result<PersonScan, Refusal>> {
    let census = pctwin_scan::census_this_laptop();
    let people: BTreeSet<String> = accounts
        .iter()
        .filter(|a| census_as(census.clone(), a).is_some())
        .cloned()
        .collect();
    let mut out: BTreeMap<String, Result<PersonScan, Refusal>> = accounts
        .iter()
        .filter(|a| !people.contains(*a))
        .map(|a| (a.clone(), Err(Refusal::NoHome)))
        .collect();
    let Some(first) = people.iter().next() else {
        return out;
    };
    let Some(census) = census_as(census, first) else {
        return out;
    };
    let facts = SweepFacts {
        laptop,
        census,
        volumes: pctwin_scan::volumes_this_laptop(),
        // The system's list of special folders is the signed-in account's, not these people's:
        // their homes are placed by the "inside a home" rule instead.
        known_folders: Vec::new(),
        remembered: Vec::new(),
        rules: Rules::shipped(),
    };
    let mut reader = pctwin_scan::system_reader();
    let stop = AtomicBool::new(false);
    let swept = pctwin_scan::sweep_people(
        facts,
        &people,
        reader.as_mut(),
        &stop,
        &mut progress(),
        &SweepLimits::default(),
    );
    let privacy_blocked = swept
        .blind_spots
        .iter()
        .filter(|b| b.reason == pctwin_scan::BlindReason::MacPrivacy)
        .map(|b| b.count)
        .sum();
    let shared = system_only(&swept);
    for person in &people {
        out.insert(
            person.clone(),
            Ok(PersonScan {
                found: entries_of(&swept, person),
                privacy_blocked,
                system_only: shared.clone(),
            }),
        );
    }
    out
}

/// The session's scanner: the first `ScanPerson` sweeps once for every approved person (with
/// `all`, [`scan_people`] in the helper), and every `ScanPerson` is answered from that sweep, held
/// in the helper's own memory for the session. Nobody but an approved person is answered.
pub struct ScanOnce<F> {
    approved: BTreeSet<String>,
    all: F,
    found: Option<BTreeMap<String, Result<PersonScan, Refusal>>>,
}

impl<F> ScanOnce<F>
where
    F: FnMut(&BTreeSet<String>) -> BTreeMap<String, Result<PersonScan, Refusal>>,
{
    pub fn new(approved: BTreeSet<String>, all: F) -> Self {
        Self {
            approved,
            all,
            found: None,
        }
    }

    /// `account`'s files, from the session's one sweep (made now if this is the first ask).
    pub fn scan(&mut self, account: &str) -> Result<PersonScan, Refusal> {
        if !self.approved.contains(account) {
            return Err(Refusal::NotApproved);
        }
        let found = match &mut self.found {
            Some(found) => found,
            none => none.insert((self.all)(&self.approved)),
        };
        found.get(account).cloned().unwrap_or(Err(Refusal::NoHome))
    }
}
