//! Several people and several old laptops in one move (Task List 2.6; acceptance T26-001 to
//! T26-005, T26-027, T26-028).
//!
//! - **Who is who**: a person is their account on one old laptop ([`SourcePerson`]): the laptop's
//!   own ID and the system's account ID there. Two old Macs both have account 501, and Windows
//!   laptops cloned from one image share account IDs, so an equal ID, number or name on two
//!   laptops never makes two people one. Names only ever suggest ([`Topology::suggest`]); the
//!   mover chooses ([`Topology::map`]). Two people from one laptop never go into one account.
//! - **Same names, two laptops**: when two old laptops would put something at the same place in
//!   the same account, each lands in its own "From <laptop>" folder ([`landing`]), even when the
//!   bytes are equal: equal bytes are never merged, because they do not say whose a thing is.
//!   Names are compared as the new laptop holds them (case and composed accents ignored); the
//!   exact names are kept.
//! - **Apps for everyone** are installed once; each person's own data stays theirs.
//! - **Each old laptop keeps its own** record, approval, change journal and undo, and its own
//!   session number: a late result from an older session, or for an item not in its approved
//!   plan, is refused ([`Topology::accept`]).
//! - **A change after approval** ([`Topology::refresh`]) holds only the affected items until the
//!   person approves again; everything else carries on. Where each person goes is part of what
//!   is approved ([`Topology::plan_fingerprint`]): a person put into another account after
//!   approval has their items held the same way until it is approved again (third security
//!   review, finding 5).

use std::collections::{BTreeMap, BTreeSet};

use pctwin_record::{
    Fingerprint, Inclusion, Item, ItemId, ItemKind, LaptopId, Owner, PersonTarget, Record,
};
use unicode_normalization::UnicodeNormalization;

/// One person: their account on one old laptop.
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct SourcePerson {
    pub laptop: LaptopId,
    /// The system's own account ID on that laptop.
    pub account_id: String,
}

#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
pub enum TopologyError {
    #[error("this old laptop is already part of the move")]
    SourceTwice,
    #[error("the list comes from another laptop than the one named")]
    WrongLaptop,
    #[error("this old laptop is not part of the move")]
    UnknownSource,
    #[error("that person has nothing on that old laptop")]
    UnknownPerson,
    #[error("two people from one old laptop cannot go into one account")]
    AccountUsedTwice,
    #[error("a person with things in the move has no place to go yet")]
    Unmapped { person: SourcePerson },
    #[error("the plan for this old laptop is not approved")]
    NotApproved,
    #[error("this comes from an older session with that laptop")]
    OldSession,
    #[error("this is not in the approved plan for that laptop")]
    NotInApprovedPlan,
    #[error("this changed after the plan was approved: approve it again first")]
    NeedsFreshApproval,
    #[error("the plan breaks a rule: {0}")]
    BadRecord(String),
}

#[derive(Debug, Clone)]
struct Source {
    label: String,
    record: Record,
    approved: Option<Approved>,
    session: u64,
}

/// What the person approved for one old laptop.
#[derive(Debug, Clone)]
struct Approved {
    /// The plan's fingerprint: the record's and where each person goes.
    fingerprint: PlanFingerprint,
    items: BTreeMap<ItemId, Item>,
    /// Where each of this laptop's people goes, by their account ID there.
    places: BTreeMap<String, PersonTarget>,
}

/// The fingerprint of one old laptop's plan as the person approves it: the record's own
/// fingerprint and where each of its people goes.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct PlanFingerprint(pub [u8; 32]);

/// A result an old laptop reports for one item, in one session.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SourceResult {
    pub laptop: LaptopId,
    pub session: u64,
    pub item: ItemId,
}

/// An app installed for everyone, found on one or more old laptops: installed once.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SharedInstall {
    pub name: String,
    pub from: Vec<(LaptopId, ItemId)>,
}

/// Every old laptop in the move, and where each person goes.
#[derive(Debug, Clone, Default)]
pub struct Topology {
    sources: BTreeMap<LaptopId, Source>,
    mapping: BTreeMap<SourcePerson, PersonTarget>,
}

fn owner_of(laptop: &LaptopId, item: &Item) -> Option<SourcePerson> {
    match &item.owner {
        Owner::Person { account_id } => Some(SourcePerson {
            laptop: *laptop,
            account_id: account_id.clone(),
        }),
        Owner::Shared | Owner::System => None,
    }
}

/// A name as the new laptop compares it: composed, ignoring case.
fn compared(name: &str) -> String {
    name.nfc().collect::<String>().to_lowercase()
}

impl Topology {
    /// Adds an old laptop, by its own ID, with a label to show (only a label: never an identity).
    pub fn add_source(
        &mut self,
        laptop: LaptopId,
        label: &str,
        record: Record,
    ) -> Result<(), TopologyError> {
        if record.source_laptop != laptop {
            return Err(TopologyError::WrongLaptop);
        }
        if self.sources.contains_key(&laptop) {
            return Err(TopologyError::SourceTwice);
        }
        record
            .validate()
            .map_err(|e| TopologyError::BadRecord(e.to_string()))?;
        self.sources.insert(
            laptop,
            Source {
                label: label.to_string(),
                record,
                approved: None,
                session: 0,
            },
        );
        Ok(())
    }

    /// Everyone with something on any old laptop, each laptop's people apart.
    pub fn people(&self) -> Vec<SourcePerson> {
        let mut out = BTreeSet::new();
        for (laptop, s) in &self.sources {
            out.extend(s.record.items.iter().filter_map(|i| owner_of(laptop, i)));
        }
        out.into_iter().collect()
    }

    /// Suggests an account on the new laptop for each person whose name (`names`) equals an
    /// account's name (`targets`: ID and name), ignoring case. Only a suggestion: nothing is
    /// mapped.
    pub fn suggest(
        &self,
        names: &BTreeMap<SourcePerson, &str>,
        targets: &[(&str, &str)],
    ) -> BTreeMap<SourcePerson, Option<String>> {
        names
            .iter()
            .map(|(p, name)| {
                let hit = targets
                    .iter()
                    .find(|(_, t)| compared(t) == compared(name))
                    .map(|(id, _)| id.to_string());
                (p.clone(), hit)
            })
            .collect()
    }

    /// Puts `person` into `target`, as the mover chose.
    pub fn map(&mut self, person: SourcePerson, target: PersonTarget) -> Result<(), TopologyError> {
        if !self.people().contains(&person) {
            return Err(TopologyError::UnknownPerson);
        }
        let clash = self.mapping.iter().any(|(other, t)| {
            other.laptop == person.laptop
                && *other != person
                && match (t, &target) {
                    (
                        PersonTarget::ExistingAccount { account_id: a },
                        PersonTarget::ExistingAccount { account_id: b },
                    ) => a == b,
                    (
                        PersonTarget::NewAccount { name: a },
                        PersonTarget::NewAccount { name: b },
                    ) => compared(a) == compared(b),
                    _ => false,
                }
        });
        if clash {
            return Err(TopologyError::AccountUsedTwice);
        }
        self.mapping.insert(person, target);
        Ok(())
    }

    pub fn target_of(&self, person: &SourcePerson) -> Option<&PersonTarget> {
        self.mapping.get(person)
    }

    /// Where each of this old laptop's people goes now, by their account ID there.
    fn places_of(&self, laptop: &LaptopId) -> BTreeMap<String, PersonTarget> {
        self.mapping
            .iter()
            .filter(|(p, _)| p.laptop == *laptop)
            .map(|(p, t)| (p.account_id.clone(), t.clone()))
            .collect()
    }

    /// The fingerprint of this old laptop's plan as it stands: its record and where each of its
    /// people goes. Any change to either is a new plan to approve.
    pub fn plan_fingerprint(&self, laptop: &LaptopId) -> Option<PlanFingerprint> {
        let s = self.sources.get(laptop)?;
        Some(plan_fingerprint(
            &s.record.fingerprint(),
            &self.places_of(laptop),
        ))
    }

    /// The fingerprint of the plan last approved for this old laptop.
    pub fn approved_fingerprint(&self, laptop: &LaptopId) -> Option<PlanFingerprint> {
        self.sources
            .get(laptop)?
            .approved
            .as_ref()
            .map(|a| a.fingerprint)
    }

    /// Every person with something included has a place to go.
    pub fn check_mapping(&self) -> Result<(), TopologyError> {
        for laptop in self.sources.keys() {
            self.check_mapping_of(laptop)?;
        }
        Ok(())
    }

    fn check_mapping_of(&self, laptop: &LaptopId) -> Result<(), TopologyError> {
        let s = self
            .sources
            .get(laptop)
            .ok_or(TopologyError::UnknownSource)?;
        for item in &s.record.items {
            if item.inclusion != Inclusion::Included {
                continue;
            }
            if let Some(person) = owner_of(laptop, item)
                && !self.mapping.contains_key(&person)
            {
                return Err(TopologyError::Unmapped { person });
            }
        }
        Ok(())
    }

    /// One person's own included items: their scope, and nobody else's.
    pub fn scope(&self, person: &SourcePerson) -> Vec<ItemId> {
        self.sources
            .get(&person.laptop)
            .map(|s| {
                s.record
                    .items
                    .iter()
                    .filter(|i| i.inclusion == Inclusion::Included)
                    .filter(|i| owner_of(&person.laptop, i).as_ref() == Some(person))
                    .map(|i| i.id)
                    .collect()
            })
            .unwrap_or_default()
    }

    /// Apps installed for everyone, grouped by name across old laptops: each is installed once.
    /// The name only groups them for the plan; the app installer (2.1) checks each one's maker
    /// before anything is installed.
    pub fn shared_installs(&self) -> Vec<SharedInstall> {
        let mut by_name: BTreeMap<String, SharedInstall> = BTreeMap::new();
        for (laptop, s) in &self.sources {
            for i in &s.record.items {
                if i.kind != ItemKind::App || i.owner != Owner::Shared {
                    continue;
                }
                let Some(last) = i.path.parts().last() else {
                    continue;
                };
                let name = last.display().to_string();
                by_name
                    .entry(compared(&name))
                    .or_insert_with(|| SharedInstall {
                        name,
                        from: Vec::new(),
                    })
                    .from
                    .push((*laptop, i.id));
            }
        }
        by_name.into_values().collect()
    }

    /// The name of this old laptop's own change journal on the new laptop.
    pub fn journal_name(&self, laptop: &LaptopId) -> Option<String> {
        self.sources
            .contains_key(laptop)
            .then(|| format!("move-from-{}", laptop.to_hex()))
    }

    /// What undo for this old laptop's move may touch: only its own items.
    pub fn undo_scope(&self, laptop: &LaptopId) -> Vec<ItemId> {
        self.sources
            .get(laptop)
            .map(|s| {
                s.record
                    .items
                    .iter()
                    .filter(|i| i.inclusion == Inclusion::Included)
                    .map(|i| i.id)
                    .collect()
            })
            .unwrap_or_default()
    }

    pub fn record(&self, laptop: &LaptopId) -> Option<&Record> {
        self.sources.get(laptop).map(|s| &s.record)
    }

    /// The person approves this old laptop's current plan.
    pub fn approve(&mut self, laptop: &LaptopId) -> Result<(), TopologyError> {
        self.check_mapping_of(laptop)?;
        let s = self
            .sources
            .get(laptop)
            .ok_or(TopologyError::UnknownSource)?;
        s.record
            .validate()
            .map_err(|e| TopologyError::BadRecord(e.to_string()))?;
        let items = s.record.items.iter().map(|i| (i.id, i.clone())).collect();
        let places = self.places_of(laptop);
        let s = self
            .sources
            .get_mut(laptop)
            .ok_or(TopologyError::UnknownSource)?;
        s.approved = Some(Approved {
            fingerprint: plan_fingerprint(&s.record.fingerprint(), &places),
            items,
            places,
        });
        Ok(())
    }

    /// Starts a new session with an old laptop; results from any earlier one are refused from
    /// now on.
    pub fn begin_session(&mut self, laptop: &LaptopId) -> Result<u64, TopologyError> {
        let s = self
            .sources
            .get_mut(laptop)
            .ok_or(TopologyError::UnknownSource)?;
        s.session += 1;
        Ok(s.session)
    }

    /// Whether this item may move now: approved, and unchanged since.
    pub fn may_move(&self, laptop: &LaptopId, item: &ItemId) -> bool {
        self.held(laptop, item) == Ok(())
    }

    fn held(&self, laptop: &LaptopId, item: &ItemId) -> Result<(), TopologyError> {
        let s = self
            .sources
            .get(laptop)
            .ok_or(TopologyError::UnknownSource)?;
        let Some(approved) = &s.approved else {
            return Err(TopologyError::NotApproved);
        };
        let Some(was) = approved.items.get(item) else {
            return Err(TopologyError::NotInApprovedPlan);
        };
        let Some(now) = s.record.items.iter().find(|i| i.id == *item) else {
            return Err(TopologyError::NeedsFreshApproval);
        };
        if now != was {
            return Err(TopologyError::NeedsFreshApproval);
        }
        // Its person must still go where the approval said.
        if let Some(person) = owner_of(laptop, now)
            && self.mapping.get(&person) != approved.places.get(&person.account_id)
        {
            return Err(TopologyError::NeedsFreshApproval);
        }
        Ok(())
    }

    /// Accepts a result only from the current session with its laptop, for an item that laptop's
    /// approved plan holds unchanged.
    pub fn accept(&self, result: &SourceResult) -> Result<(), TopologyError> {
        let s = self
            .sources
            .get(&result.laptop)
            .ok_or(TopologyError::UnknownSource)?;
        if result.session != s.session {
            return Err(TopologyError::OldSession);
        }
        self.held(&result.laptop, &result.item)
    }

    /// A newer list from an old laptop. Returns the items that differ from what was approved
    /// (added, removed or changed), which now wait for a fresh approval.
    pub fn refresh(
        &mut self,
        laptop: &LaptopId,
        record: Record,
    ) -> Result<Vec<ItemId>, TopologyError> {
        if record.source_laptop != *laptop {
            return Err(TopologyError::WrongLaptop);
        }
        record
            .validate()
            .map_err(|e| TopologyError::BadRecord(e.to_string()))?;
        let s = self
            .sources
            .get_mut(laptop)
            .ok_or(TopologyError::UnknownSource)?;
        let before: BTreeMap<ItemId, Item> = match &s.approved {
            Some(a) => a.items.clone(),
            None => s.record.items.iter().map(|i| (i.id, i.clone())).collect(),
        };
        let after: BTreeMap<ItemId, &Item> = record.items.iter().map(|i| (i.id, i)).collect();
        let mut affected: BTreeSet<ItemId> = BTreeSet::new();
        for (id, was) in &before {
            if after.get(id).is_none_or(|now| *now != was) {
                affected.insert(*id);
            }
        }
        affected.extend(after.keys().filter(|id| !before.contains_key(id)));
        s.record = record;
        Ok(affected.into_iter().collect())
    }
}

/// The fingerprint of a record's revision and where each person goes, each part written with
/// its length so no two plans write the same bytes.
fn plan_fingerprint(
    record: &Fingerprint,
    places: &BTreeMap<String, PersonTarget>,
) -> PlanFingerprint {
    let mut h = blake3::Hasher::new();
    h.update(b"pctwin/v1/approved-plan:");
    h.update(record.to_hex().as_bytes());
    let mut part = |b: &[u8]| {
        h.update(&(b.len() as u64).to_le_bytes());
        h.update(b);
    };
    for (account, target) in places {
        part(account.as_bytes());
        match target {
            PersonTarget::ExistingAccount { account_id } => {
                part(b"existing");
                part(account_id.as_bytes());
            }
            PersonTarget::NewAccount { name } => {
                part(b"new");
                part(name.as_bytes());
            }
            PersonTarget::Skip => part(b"skip"),
        }
    }
    PlanFingerprint(*h.finalize().as_bytes())
}

/// Where an item lands in its account's folder.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum LandsAt {
    /// Where it always would.
    AsUsual,
    /// Inside a folder of this name (in the same place), because another old laptop has
    /// something with the same name there.
    FromLaptop(String),
}

/// A label made safe as a folder name: no separators or reserved characters, not too long.
fn folder_safe(label: &str) -> String {
    let cleaned: String = label
        .chars()
        .map(|c| {
            if c.is_control() || "\\/:*?\"<>|".contains(c) {
                ' '
            } else {
                c
            }
        })
        .collect();
    let mut s: String = cleaned
        .split_whitespace()
        .collect::<Vec<_>>()
        .join(" ")
        .chars()
        .take(60)
        .collect();
    while s.ends_with(['.', ' ']) {
        s.pop();
    }
    if s.is_empty() {
        "another laptop".to_string()
    } else {
        s
    }
}

/// The "From <laptop>" folder name for each old laptop; equal labels are numbered in the order of
/// the laptops' IDs.
pub fn provenance_folders(t: &Topology) -> BTreeMap<LaptopId, String> {
    let mut used: BTreeMap<String, u32> = BTreeMap::new();
    t.sources
        .iter()
        .map(|(laptop, s)| {
            let base = format!("From {}", folder_safe(&s.label));
            let n = used.entry(compared(&base)).or_insert(0);
            *n += 1;
            let name = if *n == 1 {
                base
            } else {
                format!("{base} ({n})")
            };
            (*laptop, name)
        })
        .collect()
}

/// Where each included item of a mapped person (or shared) lands.
pub fn landing(t: &Topology) -> BTreeMap<(LaptopId, ItemId), LandsAt> {
    // The account on the new laptop an item goes into, as a key.
    let target_key = |laptop: &LaptopId, item: &Item| -> Option<String> {
        match &item.owner {
            Owner::Shared => Some("shared".to_string()),
            Owner::System => None,
            Owner::Person { account_id } => {
                let person = SourcePerson {
                    laptop: *laptop,
                    account_id: account_id.clone(),
                };
                match t.mapping.get(&person)? {
                    PersonTarget::ExistingAccount { account_id } => {
                        Some(format!("account:{account_id}"))
                    }
                    PersonTarget::NewAccount { name } => Some(format!("new:{}", compared(name))),
                    PersonTarget::Skip => None,
                }
            }
        }
    };
    // Each landing item: (laptop, item, place key, path as compared).
    let mut landing_items = Vec::new();
    for (laptop, s) in &t.sources {
        for item in &s.record.items {
            if item.inclusion != Inclusion::Included {
                continue;
            }
            let Some(target) = target_key(laptop, item) else {
                continue;
            };
            let place = format!("{target}|{:?}", item.place.role);
            let path: Vec<String> = item
                .path
                .parts()
                .iter()
                .map(|p| compared(p.display()))
                .collect();
            landing_items.push((*laptop, item.id, place, path));
        }
    }
    // Per laptop and place: the full paths, and every folder above them.
    let mut files: BTreeMap<(LaptopId, &str), BTreeSet<&[String]>> = BTreeMap::new();
    let mut folders: BTreeMap<(LaptopId, &str), BTreeSet<&[String]>> = BTreeMap::new();
    for (laptop, _, place, path) in &landing_items {
        files
            .entry((*laptop, place.as_str()))
            .or_default()
            .insert(path.as_slice());
        let f = folders.entry((*laptop, place.as_str())).or_default();
        for n in 1..path.len() {
            f.insert(&path[..n]);
        }
    }
    let names = provenance_folders(t);
    landing_items
        .iter()
        .map(|(laptop, id, place, path)| {
            let clashes = t.sources.keys().filter(|o| *o != laptop).any(|other| {
                let key = (*other, place.as_str());
                let of = files.get(&key);
                let od = folders.get(&key);
                of.is_some_and(|f| f.contains(path.as_slice()))
                    || od.is_some_and(|d| d.contains(path.as_slice()))
                    || of.is_some_and(|f| (1..path.len()).any(|n| f.contains(&path[..n])))
            });
            let at = if clashes {
                LandsAt::FromLaptop(names[laptop].clone())
            } else {
                LandsAt::AsUsual
            };
            ((*laptop, *id), at)
        })
        .collect()
}
