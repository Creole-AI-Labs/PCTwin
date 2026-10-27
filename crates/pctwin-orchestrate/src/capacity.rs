//! A smaller new laptop (Task List 2.6; acceptance T26-010 to T26-012).
//!
//! - [`budget`]: every real volume on the new laptop is budgeted with everything that lands on
//!   it, not just the files: app downloads and unpacking, the change journal, partly copied files
//!   kept to continue from ([`NeedKind`]), with the same room always kept free as the move keeps
//!   ([`KEEP_FREE`]). A volume whose free space is not known never "fits".
//! - [`propose`]: when a volume is short, PCTwin proposes what to keep elsewhere: first what is
//!   already safe in a cloud (left there), then a separately approved, connected offload drive
//!   with room, then leaving it on the old laptop; largest first; never the person's essentials;
//!   only as much as the shortfall. Whatever is still short is said, never forced.
//! - [`Decisions`]: a proposal changes nothing until the person approves it; the selection
//!   itself never changes, and nothing is deleted to make room.
//! - [`account`]: after the move every selected item is in exactly one place ([`Outcome`]). An
//!   offload drive that is not connected is "on the offload drive, not connected", never on the
//!   new laptop; an item with no result is missing, never dropped from the count; what was left
//!   on the old laptop joins the "still only on the old laptop" list.

use std::collections::BTreeMap;

use pctwin_record::ItemId;

/// Space left free on each drive after the move, as the move itself keeps
/// (`pctwin_transfer::KEEP_FREE`).
pub const KEEP_FREE: u64 = 1024 * 1024 * 1024;

/// One real volume on the new laptop, by the label the plan uses for it.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Volume {
    pub id: String,
    /// Free space as its drive reports it (`None`: not known).
    pub free: Option<u64>,
}

/// What takes space on a volume.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum NeedKind {
    /// The files themselves.
    Files,
    /// Apps' installers downloaded to reinstall them.
    AppDownloads,
    /// Room installers need while they unpack.
    Unpacking,
    /// The change journal that makes undo and resume possible.
    Journal,
    /// Partly copied files kept to continue from after an interruption.
    PartlyCopied,
}

/// Some bytes of one kind on one volume.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Need {
    pub volume: String,
    pub kind: NeedKind,
    pub bytes: u64,
}

/// How one volume fares.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct VolumeBudget {
    pub volume: String,
    pub by_kind: BTreeMap<NeedKind, u64>,
    pub total: u64,
    pub free: Option<u64>,
    /// `Some(true)`: fits with [`KEEP_FREE`] to spare; `None`: free space not known.
    pub fits: Option<bool>,
    /// How much it is short by (0 when it fits or is not known).
    pub short_by: u64,
}

/// Each volume named by `volumes` or `needs`, in that order, against what lands on it.
pub fn budget(volumes: &[Volume], needs: &[Need]) -> Vec<VolumeBudget> {
    let mut order: Vec<String> = volumes.iter().map(|v| v.id.clone()).collect();
    for n in needs {
        if !order.contains(&n.volume) {
            order.push(n.volume.clone());
        }
    }
    order
        .into_iter()
        .map(|volume| {
            let mut by_kind = BTreeMap::new();
            for n in needs.iter().filter(|n| n.volume == volume) {
                let e = by_kind.entry(n.kind).or_insert(0u64);
                *e = e.saturating_add(n.bytes);
            }
            let total = by_kind.values().fold(0u64, |t, b| t.saturating_add(*b));
            let free = volumes.iter().find(|v| v.id == volume).and_then(|v| v.free);
            let wanted = total.saturating_add(KEEP_FREE);
            VolumeBudget {
                fits: free.map(|f| wanted <= f),
                short_by: free.map_or(0, |f| wanted.saturating_sub(f)),
                volume,
                by_kind,
                total,
                free,
            }
        })
        .collect()
}

/// Something that could be kept elsewhere to make room.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Candidate {
    pub item: ItemId,
    /// The volume it would land on.
    pub volume: String,
    pub bytes: u64,
    /// One of the person's own essentials: never proposed.
    pub essential: bool,
    /// Already safe in a cloud (an online-only file the person wanted a copy of): it can simply
    /// stay there.
    pub safe_in_cloud: bool,
}

/// Where something kept off the new laptop goes.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum Choice {
    /// It stays in its cloud.
    LeaveInCloud,
    /// To the approved offload drive.
    Offload,
    /// It stays on the old laptop.
    LeaveOnOldLaptop,
}

/// An offload drive: a separate destination the person approves on its own.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct OffloadDrive {
    pub label: String,
    pub free: Option<u64>,
    /// The person approved it as a destination.
    pub approved: bool,
    pub connected: bool,
}

/// What PCTwin proposes, and what would still be short.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct Proposal {
    pub choices: BTreeMap<ItemId, Choice>,
    /// Volumes still short after every possible proposal, by how much.
    pub still_short: BTreeMap<String, u64>,
}

/// What to keep elsewhere so each short volume in `budgets` fits.
pub fn propose(
    budgets: &[VolumeBudget],
    candidates: &[Candidate],
    offload: Option<&OffloadDrive>,
) -> Proposal {
    let mut proposal = Proposal::default();
    // Room on the offload drive, kept free as on any drive; only an approved, connected drive
    // whose space is known.
    let mut offload_room = offload
        .filter(|d| d.approved && d.connected)
        .and_then(|d| d.free)
        .map_or(0, |f| f.saturating_sub(KEEP_FREE));
    for b in budgets {
        if b.fits == Some(true) {
            continue;
        }
        let mut short = b.short_by;
        let mut here: Vec<&Candidate> = candidates
            .iter()
            .filter(|c| c.volume == b.volume && !c.essential)
            .collect();
        // Largest first; equal sizes in a fixed order.
        here.sort_by(|x, y| y.bytes.cmp(&x.bytes).then(x.item.cmp(&y.item)));
        for pass_cloud in [true, false] {
            for c in &here {
                if short == 0 {
                    break;
                }
                if proposal.choices.contains_key(&c.item) || c.safe_in_cloud != pass_cloud {
                    continue;
                }
                let choice = if c.safe_in_cloud {
                    Choice::LeaveInCloud
                } else if c.bytes <= offload_room {
                    offload_room -= c.bytes;
                    Choice::Offload
                } else {
                    Choice::LeaveOnOldLaptop
                };
                proposal.choices.insert(c.item, choice);
                short = short.saturating_sub(c.bytes);
            }
        }
        if short > 0 {
            proposal.still_short.insert(b.volume.clone(), short);
        }
    }
    proposal
}

/// What the person approved to keep off the new laptop.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct Decisions {
    pub choices: BTreeMap<ItemId, Choice>,
}

impl Decisions {
    /// Whether these decisions make every volume fit. A volume of unknown free space never does;
    /// leaving something in the cloud counts only for what is safe in a cloud.
    pub fn covers(&self, budgets: &[VolumeBudget], candidates: &[Candidate]) -> bool {
        budgets.iter().all(|b| {
            if b.fits == Some(true) {
                return true;
            }
            if b.free.is_none() {
                return false;
            }
            let freed = candidates
                .iter()
                .filter(|c| c.volume == b.volume)
                .filter(|c| match self.choices.get(&c.item) {
                    Some(Choice::LeaveInCloud) => c.safe_in_cloud,
                    Some(Choice::Offload | Choice::LeaveOnOldLaptop) => true,
                    None => false,
                })
                .fold(0u64, |t, c| t.saturating_add(c.bytes));
            freed >= b.short_by
        })
    }
}

/// Where one selected item is after the move.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum Outcome {
    OnNewLaptop,
    OnOffloadDrive,
    /// On the offload drive, which is not connected: not on the new laptop.
    OffloadDriveNotConnected,
    LeftInCloud,
    /// Left on the old laptop.
    OnlyOnOldLaptop,
    /// No verified result: not on the new laptop.
    Missing,
}

impl Outcome {
    pub fn words(&self) -> &'static str {
        match self {
            Outcome::OnNewLaptop => "On the new laptop.",
            Outcome::OnOffloadDrive => "On the offload drive.",
            Outcome::OffloadDriveNotConnected => {
                "On the offload drive, which is not connected now. Connect it to open these."
            }
            Outcome::LeftInCloud => "Left in the cloud, where they are safe.",
            Outcome::OnlyOnOldLaptop => {
                "Still only on the old laptop: keep it until these are moved."
            }
            Outcome::Missing => {
                "Not on the new laptop: PCTwin has no result for these. Try the move again for them."
            }
        }
    }
}

/// Every selected item in exactly one place.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct Accounting {
    pub by_outcome: BTreeMap<Outcome, Vec<ItemId>>,
    /// Joins the list of what is still only on the old laptop.
    pub still_only_on_old_laptop: Vec<ItemId>,
}

/// Where each of `selected` ended: `decisions` approved before the move, `verified` the move's
/// results (true: written and checked), `offload_connected` whether the offload drive is there
/// now.
pub fn account(
    selected: &[ItemId],
    decisions: &Decisions,
    verified: &BTreeMap<ItemId, bool>,
    offload_connected: bool,
) -> Accounting {
    let mut acc = Accounting::default();
    for item in selected {
        let ok = verified.get(item) == Some(&true);
        let outcome = match decisions.choices.get(item) {
            Some(Choice::LeaveInCloud) => Outcome::LeftInCloud,
            Some(Choice::LeaveOnOldLaptop) => {
                acc.still_only_on_old_laptop.push(*item);
                Outcome::OnlyOnOldLaptop
            }
            Some(Choice::Offload) if ok && offload_connected => Outcome::OnOffloadDrive,
            Some(Choice::Offload) if ok => Outcome::OffloadDriveNotConnected,
            None if ok => Outcome::OnNewLaptop,
            Some(Choice::Offload) | None => Outcome::Missing,
        };
        acc.by_outcome.entry(outcome).or_default().push(*item);
    }
    acc
}
