use std::collections::BTreeMap;

use pctwin_record::{FolderRole, Inclusion, Item, ItemId, ItemKind};

/// How soon something moves. Earlier variants move first.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum Tier {
    /// The person asked for this first (Smart mode or their own picks), in the order asked.
    AskedFirst {
        rank: u32,
    },
    /// Documents, the desktop, settings and anything changed recently, so the new laptop is usable
    /// in minutes.
    Essential,
    Rest,
}

/// Files changed within this long count as recent.
const RECENT_NS: i64 = 30 * 86_400_000_000_000;

/// The order the plan's items move in, with each one's tier. Only included items are queued;
/// folder entries are not sent (their files are), but whole packages are. `asked` is what the
/// person asked to move first, in order; asking for a folder covers everything inside it.
/// `modified` gives each file's modified time (nanoseconds since 1970).
pub fn plan_order(
    items: &[Item],
    asked: &[ItemId],
    modified: &BTreeMap<ItemId, i64>,
    now_ns: i64,
) -> Vec<(ItemId, Tier)> {
    let asked_items: Vec<(u32, &Item)> = asked
        .iter()
        .enumerate()
        .filter_map(|(rank, id)| {
            let item = items.iter().find(|i| i.id == *id)?;
            Some((u32::try_from(rank).unwrap_or(u32::MAX), item))
        })
        .collect();
    let rank_of = |item: &Item| {
        asked_items
            .iter()
            .filter(|(_, a)| a.id == item.id || contains(a, item))
            .map(|(rank, _)| *rank)
            .min()
    };
    let mut queue: Vec<(Tier, i64, u64, ItemId)> = items
        .iter()
        .filter(|i| i.inclusion == Inclusion::Included)
        .filter(|i| !(i.kind == ItemKind::Folder && i.size_bytes == 0))
        .map(|i| {
            let changed = modified.get(&i.id).copied();
            let tier = match rank_of(i) {
                Some(rank) => Tier::AskedFirst { rank },
                None if is_essential(i, changed, now_ns) => Tier::Essential,
                None => Tier::Rest,
            };
            (tier, changed.unwrap_or(i64::MIN), i.size_bytes, i.id)
        })
        .collect();
    // Within a tier: newest first, then smaller first, then by ID so the order never varies.
    queue.sort_by(|a, b| {
        a.0.cmp(&b.0)
            .then(b.1.cmp(&a.1))
            .then(a.2.cmp(&b.2))
            .then(a.3.cmp(&b.3))
    });
    queue
        .into_iter()
        .map(|(tier, _, _, id)| (id, tier))
        .collect()
}

fn is_essential(item: &Item, modified: Option<i64>, now_ns: i64) -> bool {
    matches!(item.place.role, FolderRole::Documents | FolderRole::Desktop)
        || item.kind == ItemKind::Setting
        || modified.is_some_and(|m| m >= now_ns.saturating_sub(RECENT_NS))
}

/// Whether `inner` is inside the folder item `folder`: same owner and place, and its path starts
/// with the folder's whole path.
fn contains(folder: &Item, inner: &Item) -> bool {
    let (f, i) = (folder.path.parts(), inner.path.parts());
    folder.kind == ItemKind::Folder
        && folder.owner == inner.owner
        && folder.place == inner.place
        && i.len() > f.len()
        && i[..f.len()] == *f
}

/// Decides which file sends its next piece. Several files are in flight at once and take turns
/// piece by piece, so small files arrive quickly while a big one keeps going; only the most
/// important files in flight get turns, and a more important file starts at once (the others pause
/// where they are, losing nothing).
#[derive(Debug, Clone)]
pub struct Scheduler {
    capacity: usize,
    waiting: Vec<(ItemId, Tier)>,
    active: Vec<(ItemId, Tier)>,
    turn: usize,
    next_rank: u32,
}

impl Scheduler {
    /// `capacity` files are normally in flight at once.
    pub fn new(capacity: usize) -> Self {
        Self {
            capacity: capacity.max(1),
            waiting: Vec::new(),
            active: Vec::new(),
            turn: 0,
            next_rank: 0,
        }
    }

    /// Adds a file to wait its turn (call in [`plan_order`]'s order).
    pub fn push(&mut self, item: ItemId, tier: Tier) {
        if let Tier::AskedFirst { rank } = tier {
            self.next_rank = self.next_rank.max(rank.saturating_add(1));
        }
        self.waiting.push((item, tier));
        self.sort_waiting();
    }

    /// The person asked for these to move first, in this order, during the move.
    pub fn ask_first(&mut self, items: &[ItemId]) {
        for &item in items {
            let tier = Tier::AskedFirst {
                rank: self.next_rank,
            };
            self.next_rank = self.next_rank.saturating_add(1);
            if let Some(a) = self.active.iter_mut().find(|(i, _)| *i == item) {
                a.1 = a.1.min(tier);
            } else if let Some(w) = self.waiting.iter_mut().find(|(i, _)| *i == item) {
                w.1 = w.1.min(tier);
            } else {
                self.waiting.push((item, tier));
            }
        }
        self.sort_waiting();
    }

    /// The file whose next piece goes now, or `None` when everything is done.
    pub fn next_turn(&mut self) -> Option<ItemId> {
        while !self.waiting.is_empty()
            && (self.active.len() < self.capacity || self.waiting[0].1 < self.best_active())
        {
            let next = self.waiting.remove(0);
            self.active.push(next);
        }
        let best = self.best_active();
        let candidates: Vec<ItemId> = self
            .active
            .iter()
            .filter(|(_, t)| *t == best)
            .map(|(i, _)| *i)
            .collect();
        if candidates.is_empty() {
            return None;
        }
        let pick = candidates[self.turn % candidates.len()];
        self.turn = self.turn.wrapping_add(1);
        Some(pick)
    }

    /// The file is fully sent (or given up on).
    pub fn finished(&mut self, item: ItemId) {
        self.active.retain(|(i, _)| *i != item);
        self.waiting.retain(|(i, _)| *i != item);
    }

    fn best_active(&self) -> Tier {
        self.active
            .iter()
            .map(|(_, t)| *t)
            .min()
            .unwrap_or(Tier::Rest)
    }

    fn sort_waiting(&mut self) {
        // Stable: files of the same tier keep their planned order.
        self.waiting.sort_by_key(|(_, tier)| *tier);
    }
}
