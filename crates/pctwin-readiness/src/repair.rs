//! Focused repair and final changes (Task List 2.7 "Bring me up to date", 2.9 "Repair only what
//! failed"; plan 2.7 section 5; acceptance T27-008 to T27-010).
//!
//! - [`focus`]: "Fix these N items" takes exactly the items picked that are not done, and the
//!   dependencies they need that are not done; finished work is left as it is, and a dependency
//!   outside the approved plan is named for a fresh approval, never added quietly.
//!   [`FocusedRepair::jobs`] narrows the original 2.6 child jobs to those items for a new
//!   session, after the prerequisites are checked again (the old laptop connected, the space).
//!   The ledger keeps each item's earlier results, so a repair never erases a failure's history.
//! - [`final_changes`]: what changed on the old laptop since the approved plan, compared item by
//!   item (by item ID) against what landed: new files and changed files whose copy nobody touched
//!   are offered; a changed file whose copy was edited on the new laptop is a conflict that is
//!   never sent over it ([`Change::BothChanged`]); a file removed on the old laptop is never
//!   removed on the new one. The person reviews the delta ([`approve`]); the follow-up plan is
//!   the next revision of the record, approved again like any plan.
//! - [`three_way`]: a returned or loaned laptop compares the state at the move, the old laptop
//!   now and the new laptop now; it never copies back blindly. Changed on both sides into the
//!   same content is not a conflict (Unison's "false conflict").

use std::collections::{BTreeMap, BTreeSet};

use pctwin_orchestrate::jobs::{ChildJob, JobId, Plan, PlanError, Scope, seal};
use pctwin_record::{Fingerprint, Inclusion, Item, ItemId, ItemKind, Record};
use serde::{Deserialize, Serialize};

use crate::ledger::{Ledger, Selection, Standing};

/// Why a repair or a follow-up could not be made.
#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
pub enum RepairError {
    #[error("nothing was picked to fix")]
    NothingPicked,
    #[error("an item picked is not in this move")]
    UnknownItem(ItemId),
    #[error("an item picked was not selected to move")]
    NotSelected(ItemId),
    #[error("an item picked is already done")]
    AlreadyDone(ItemId),
    #[error("the old laptop is not connected")]
    SourceNotConnected,
    #[error("there is not enough space on the new laptop")]
    NotEnoughSpace,
    #[error("the new laptop's free space is not known")]
    SpaceUnknown,
    #[error("the repair is for an older version of the plan")]
    StalePlan,
    #[error("the repair plan could not be made: {0}")]
    Plan(#[from] PlanError),
}

/// The items one "Fix these" reruns.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct FocusedRepair {
    pub generation: u64,
    /// Picked items and the unfinished dependencies they need.
    pub items: BTreeSet<ItemId>,
    /// Dependencies already done: kept as they are, not run again.
    pub kept: BTreeSet<ItemId>,
    /// Dependencies outside the approved plan: they need the person's fresh approval first.
    pub needs_approval: BTreeSet<ItemId>,
}

/// What `items` need: for each item, the items it depends on (an import on its app, a project on
/// its linked files).
pub type Needs = BTreeMap<ItemId, BTreeSet<ItemId>>;

/// "Fix these": the picked items that are not done and what they need.
pub fn focus(
    ledger: &Ledger,
    picked: &BTreeSet<ItemId>,
    needs: &Needs,
) -> Result<FocusedRepair, RepairError> {
    if picked.is_empty() {
        return Err(RepairError::NothingPicked);
    }
    let mut r = FocusedRepair {
        generation: ledger.generation(),
        items: BTreeSet::new(),
        kept: BTreeSet::new(),
        needs_approval: BTreeSet::new(),
    };
    for id in picked {
        match ledger.line(*id) {
            None => return Err(RepairError::UnknownItem(*id)),
            Some(l) if l.selection != Selection::Selected => {
                return Err(RepairError::NotSelected(*id));
            }
            Some(_) if matches!(ledger.standing(*id), Some(Standing::Done(_))) => {
                return Err(RepairError::AlreadyDone(*id));
            }
            Some(_) => {}
        }
    }
    let mut stack: Vec<ItemId> = picked.iter().copied().collect();
    while let Some(id) = stack.pop() {
        if !r.items.insert(id) {
            continue;
        }
        for dep in needs.get(&id).into_iter().flatten() {
            match ledger.line(*dep) {
                None => return Err(RepairError::UnknownItem(*dep)),
                Some(l) if l.selection != Selection::Selected => {
                    r.needs_approval.insert(*dep);
                }
                Some(_) if matches!(ledger.standing(*dep), Some(Standing::Done(_))) => {
                    r.kept.insert(*dep);
                }
                Some(_) => stack.push(*dep),
            }
        }
    }
    Ok(r)
}

/// What is checked again just before a repair runs.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Prerequisites {
    pub source_connected: bool,
    /// Whether the repair fits on the new laptop (`None`: not known).
    pub space_fits: Option<bool>,
}

/// The repair's work: child jobs narrowed to the items, and the files the transfer tries again.
#[derive(Debug, Clone)]
pub struct RepairJobs {
    pub plan: Plan,
    pub jobs: Vec<ChildJob>,
    /// Items no child job holds: plain files the move's "Try again" sends.
    pub by_transfer: BTreeSet<ItemId>,
}

impl FocusedRepair {
    /// The original plan's `jobs` narrowed to this repair, for the new `session`, after the
    /// prerequisites are checked again. `job_items` names each job's items.
    pub fn jobs(
        &self,
        ledger: &Ledger,
        original: &[ChildJob],
        job_items: &BTreeMap<JobId, BTreeSet<ItemId>>,
        session: u64,
        pre: Prerequisites,
    ) -> Result<RepairJobs, RepairError> {
        if self.generation != ledger.generation() {
            return Err(RepairError::StalePlan);
        }
        if !pre.source_connected {
            return Err(RepairError::SourceNotConnected);
        }
        match pre.space_fits {
            Some(true) => {}
            Some(false) => return Err(RepairError::NotEnoughSpace),
            None => return Err(RepairError::SpaceUnknown),
        }
        let mut covered = BTreeSet::new();
        let mut jobs = Vec::new();
        for job in original {
            let named = job_items.get(&job.id).cloned().unwrap_or_default();
            let affected: BTreeSet<ItemId> = named.intersection(&self.items).copied().collect();
            if affected.is_empty() {
                continue;
            }
            let scope = match &job.scope {
                Scope::Items { laptop, items } => {
                    let items: BTreeSet<ItemId> = items.intersection(&affected).copied().collect();
                    if items.is_empty() {
                        continue;
                    }
                    covered.extend(items.iter().copied());
                    Scope::Items {
                        laptop: *laptop,
                        items,
                    }
                }
                other => {
                    covered.extend(affected.iter().copied());
                    other.clone()
                }
            };
            jobs.push(ChildJob {
                id: job.id.clone(),
                worker: job.worker,
                source: job.source,
                session,
                scope,
                after: Vec::new(),
                essential: job.essential,
            });
        }
        // Keep only the waits between jobs that are both in the repair.
        let ids: BTreeSet<JobId> = jobs.iter().map(|j| j.id.clone()).collect();
        for j in &mut jobs {
            let orig = original
                .iter()
                .find(|o| o.id == j.id)
                .expect("made from it");
            j.after = orig
                .after
                .iter()
                .filter(|a| ids.contains(*a))
                .cloned()
                .collect();
        }
        let plan = seal(jobs.clone())?;
        let by_transfer = self.items.difference(&covered).copied().collect();
        Ok(RepairJobs {
            plan,
            jobs,
            by_transfer,
        })
    }
}

/// What a file was like: its size, modified time and, where known, its fingerprint.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct Stamp {
    pub size: u64,
    pub modified_ns: Option<i64>,
    pub digest: Option<[u8; 32]>,
}

/// Whether two stamps are of the same content.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Same {
    Yes,
    No,
    CannotTell,
}

impl Stamp {
    pub fn of(item: &Item) -> Self {
        Self {
            size: item.size_bytes,
            modified_ns: item.modified_ns,
            digest: None,
        }
    }

    /// Fingerprints decide when both have one; otherwise a different size, or a different
    /// modified time, is a change; equal sizes with a time missing cannot be told apart.
    pub fn same(&self, other: &Stamp) -> Same {
        if let (Some(a), Some(b)) = (self.digest, other.digest) {
            return if a == b { Same::Yes } else { Same::No };
        }
        if self.size != other.size {
            return Same::No;
        }
        match (self.modified_ns, other.modified_ns) {
            (Some(a), Some(b)) if a == b => Same::Yes,
            (Some(_), Some(_)) => Same::No,
            _ => Same::CannotTell,
        }
    }
}

/// An item's copy on the new laptop.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Target {
    /// It never landed.
    NotThere,
    /// It landed as `at_move`; `now` is what is there now (`None`: gone).
    Landed { at_move: Stamp, now: Option<Stamp> },
}

/// What changed for one item since the approved plan.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum Change {
    /// New on the old laptop (or changed, and its copy never landed).
    New,
    /// Changed on the old laptop; its copy on the new laptop is as it landed.
    Updated,
    /// Changed on the old laptop, and its copy was edited on the new laptop: a conflict.
    BothChanged,
    /// Changed on the old laptop, and its copy was removed on the new laptop.
    TargetGone,
    /// Removed on the old laptop. Nothing is removed on the new laptop.
    GoneAtSource,
    /// Whether it changed cannot be told (no modified time): for the person to look at.
    CannotTell,
}

/// One line of the delta.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct DeltaLine {
    pub item: ItemId,
    pub change: Change,
}

/// What changed since the approved plan, for the person to review.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Delta {
    pub approved: Fingerprint,
    pub again: Fingerprint,
    pub lines: Vec<DeltaLine>,
}

impl Delta {
    /// Binds the person's review to exactly this delta.
    pub fn digest(&self) -> [u8; 32] {
        let mut h = blake3::Hasher::new();
        h.update(b"pctwin/v1/final-changes\0");
        h.update(&serde_json::to_vec(self).expect("a delta always serialises"));
        *h.finalize().as_bytes()
    }
}

/// Why final changes could not be made or approved.
#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
pub enum DeltaError {
    #[error("the new scan is of another laptop")]
    OtherLaptop,
    #[error("the changes reviewed are not these")]
    Stale,
    #[error("this item would be sent over a copy edited on the new laptop")]
    WouldOverwrite(ItemId),
    #[error("this item needs a choice first")]
    NeedsChoice(ItemId),
    #[error("this item is not in the changes")]
    NotInDelta(ItemId),
}

fn moving_file(i: &Item) -> bool {
    i.kind == ItemKind::File && i.inclusion == Inclusion::Included
}

/// What changed between the approved plan and a new scan `again` of the same old laptop, against
/// what landed on the new laptop (`target`, by item ID).
pub fn final_changes(
    approved: &Record,
    again: &Record,
    target: &BTreeMap<ItemId, Target>,
) -> Result<Delta, DeltaError> {
    if approved.source_laptop != again.source_laptop {
        return Err(DeltaError::OtherLaptop);
    }
    let before: BTreeMap<ItemId, &Item> = approved
        .items
        .iter()
        .filter(|i| moving_file(i))
        .map(|i| (i.id, i))
        .collect();
    let now: BTreeSet<ItemId> = again.items.iter().map(|i| i.id).collect();
    let mut lines = Vec::new();
    for i in again.items.iter().filter(|i| moving_file(i)) {
        let change = match before.get(&i.id) {
            None => Some(Change::New),
            Some(p) => match Stamp::of(p).same(&Stamp::of(i)) {
                Same::Yes => None,
                Same::CannotTell => Some(Change::CannotTell),
                Same::No => Some(
                    match target.get(&i.id).copied().unwrap_or(Target::NotThere) {
                        Target::NotThere => Change::New,
                        Target::Landed { now: None, .. } => Change::TargetGone,
                        Target::Landed {
                            at_move,
                            now: Some(n),
                        } => match at_move.same(&n) {
                            Same::Yes => Change::Updated,
                            Same::No | Same::CannotTell => Change::BothChanged,
                        },
                    },
                ),
            },
        };
        if let Some(change) = change {
            lines.push(DeltaLine { item: i.id, change });
        }
    }
    for id in before.keys().filter(|id| !now.contains(id)) {
        lines.push(DeltaLine {
            item: *id,
            change: Change::GoneAtSource,
        });
    }
    Ok(Delta {
        approved: approved.fingerprint(),
        again: again.fingerprint(),
        lines,
    })
}

/// The person's choice for one line.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum Choice {
    /// Send it (only for a new file or a copy nobody touched).
    Send,
    /// Send it to land beside the copy on the new laptop, under its own name.
    KeepBoth,
    /// Keep what is on the new laptop; send nothing.
    KeepNewLaptopVersion,
}

/// The reviewed follow-up: the next plan, and how each file sent lands.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct FollowUp {
    pub record: Record,
    /// Sent as the newer version of a copy nobody touched (undo keeps the earlier one).
    pub newer: BTreeSet<ItemId>,
    /// Sent to land beside the copy on the new laptop, never over it.
    pub beside: BTreeSet<ItemId>,
}

/// Approves the reviewed `delta` (`seen` is the digest of the delta the person reviewed): new
/// files and untouched copies are sent unless the person keeps the new laptop's version; a
/// conflict, a removed copy and an unclear change need a choice; nothing is ever sent over a copy
/// edited on the new laptop; a file removed on the old laptop is left alone.
pub fn approve(
    delta: &Delta,
    seen: [u8; 32],
    approved: &Record,
    again: &Record,
    choices: &BTreeMap<ItemId, Choice>,
) -> Result<FollowUp, DeltaError> {
    if seen != delta.digest()
        || approved.fingerprint() != delta.approved
        || again.fingerprint() != delta.again
    {
        return Err(DeltaError::Stale);
    }
    if let Some(stray) = choices
        .keys()
        .find(|id| !delta.lines.iter().any(|l| l.item == **id))
    {
        return Err(DeltaError::NotInDelta(*stray));
    }
    let mut newer = BTreeSet::new();
    let mut beside = BTreeSet::new();
    let mut send = BTreeSet::new();
    for l in &delta.lines {
        let choice = choices.get(&l.item).copied();
        match (l.change, choice) {
            (Change::GoneAtSource, _) | (_, Some(Choice::KeepNewLaptopVersion)) => {}
            (Change::New, None | Some(Choice::Send)) => {
                send.insert(l.item);
            }
            (Change::Updated, None | Some(Choice::Send)) => {
                send.insert(l.item);
                newer.insert(l.item);
            }
            (_, Some(Choice::KeepBoth)) => {
                send.insert(l.item);
                beside.insert(l.item);
            }
            (Change::BothChanged | Change::TargetGone | Change::CannotTell, Some(Choice::Send)) => {
                return Err(DeltaError::WouldOverwrite(l.item));
            }
            (Change::BothChanged | Change::TargetGone | Change::CannotTell, None) => {
                return Err(DeltaError::NeedsChoice(l.item));
            }
        }
    }
    let mut record = approved.next_revision();
    record.items = again
        .items
        .iter()
        .filter(|i| send.contains(&i.id))
        .cloned()
        .collect();
    Ok(FollowUp {
        record,
        newer,
        beside,
    })
}

/// What a returned or loaned laptop's item needs (T27-010).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum Merge {
    /// Nothing changed (or it is gone on both).
    Unchanged,
    /// Changed only on the new laptop: bring it back over the unchanged old copy.
    BringBack,
    /// Changed only on the old laptop: keep it there; nothing comes back.
    KeepOld,
    /// Changed on both into the same content.
    SameOnBoth,
    /// Changed on both differently, or changed on one and removed on the other: keep both.
    Conflict,
    /// Removed on the new laptop, unchanged on the old: ask before removing anything.
    RemovedOnNewLaptop,
    /// Removed on the old laptop, unchanged on the new: ask before bringing it back.
    RemovedOnOldLaptop,
    /// Cannot be told: for the person to look at.
    CannotTell,
}

/// Compares the state at the move (`base`, `None` when the item was made later), the old laptop
/// now and the new laptop now.
pub fn three_way(base: Option<Stamp>, old_now: Option<Stamp>, new_now: Option<Stamp>) -> Merge {
    let changed = |s: &Stamp| match base {
        None => Same::No,
        Some(b) => match b.same(s) {
            Same::Yes => Same::Yes,
            Same::No => Same::No,
            Same::CannotTell => Same::CannotTell,
        },
    };
    match (old_now, new_now) {
        (None, None) => Merge::Unchanged,
        (Some(_), None) if base.is_none() => Merge::KeepOld,
        (None, Some(_)) if base.is_none() => Merge::BringBack,
        (Some(o), None) => match changed(&o) {
            Same::Yes => Merge::RemovedOnNewLaptop,
            Same::No => Merge::Conflict,
            Same::CannotTell => Merge::CannotTell,
        },
        (None, Some(n)) => match changed(&n) {
            Same::Yes => Merge::RemovedOnOldLaptop,
            Same::No => Merge::Conflict,
            Same::CannotTell => Merge::CannotTell,
        },
        (Some(o), Some(n)) => match (changed(&o), changed(&n)) {
            (Same::CannotTell, _) | (_, Same::CannotTell) => Merge::CannotTell,
            (Same::Yes, Same::Yes) => Merge::Unchanged,
            (Same::Yes, Same::No) => Merge::BringBack,
            (Same::No, Same::Yes) => Merge::KeepOld,
            (Same::No, Same::No) => match o.same(&n) {
                Same::Yes => Merge::SameOnBoth,
                Same::No => Merge::Conflict,
                Same::CannotTell => Merge::CannotTell,
            },
        },
    }
}
