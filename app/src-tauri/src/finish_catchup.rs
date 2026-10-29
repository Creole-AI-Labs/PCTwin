//! "Check the old laptop again" and "Bring me up to date" (Task List 2.7; screen R13; plan 2.7
//! section 5; acceptance T27-008, T27-009, T27-012).
//!
//! Armed from the finishing screens for one move ([`arm`]); then the person pairs and starts as for
//! any move. When the old laptop's fresh list arrives ([`plan_arrived`]), it is kept as the old
//! laptop as it is now (for the before-wipe list), and what changed since the move is worked out by
//! item ID against what landed on this laptop, as it is on disk now (`pctwin_readiness::repair`).
//! Nothing else of the full move runs: no app is installed, no setting changed.
//!
//! - Check: nothing moves; the plan is declined as soon as it is kept.
//! - Bring me up to date: the person reviews the changes ([`crate::finish::DeltaView`]) and
//!   approves them ([`approve`]); only those items are sent ([`left_out`] leaves every other item
//!   out). Nothing is ever sent over a copy edited on this laptop: such a change needs the
//!   person's choice, and "keep both" lands beside it (the transfer never overwrites).

use std::collections::{BTreeMap, BTreeSet};
use std::path::PathBuf;
use std::sync::{Mutex, OnceLock};

use pctwin_readiness::repair::{self, Change, Choice, Delta, Stamp, Target};
use pctwin_record::{ItemId, LaptopId, Record};
use tauri::{AppHandle, Emitter, Manager, Runtime};

use crate::finish::{
    CatchupChoice, CatchupPurpose, ChangeKey, ChoiceIn, DELTA_EVENT, DeltaLineView, DeltaView,
    FinishProblem, MAX_LINES,
};
use crate::finish_store::{self as store, Kept};

struct Pending {
    key: crate::moving::PlanKey,
    again: Record,
    delta: Delta,
    /// Once approved: the only items sent.
    only: Option<BTreeSet<ItemId>>,
}

struct Armed {
    history_id: String,
    purpose: CatchupPurpose,
    source: LaptopId,
    mover: String,
    baseline: Record,
    folder: PathBuf,
    pending: Option<Pending>,
}

fn armed() -> &'static Mutex<Option<Armed>> {
    static A: OnceLock<Mutex<Option<Armed>>> = OnceLock::new();
    A.get_or_init(|| Mutex::new(None))
}

fn lock() -> std::sync::MutexGuard<'static, Option<Armed>> {
    armed()
        .lock()
        .unwrap_or_else(std::sync::PoisonError::into_inner)
}

/// Readies the next move with this move's old laptop to be a check or a catch-up.
pub fn arm(kept: &Kept, folder: PathBuf, purpose: CatchupPurpose) -> Result<(), FinishProblem> {
    let mut a = lock();
    if a.as_ref().is_some_and(|x| x.pending.is_some()) {
        return Err(FinishProblem::Busy);
    }
    *a = Some(Armed {
        history_id: kept.history_id.clone(),
        purpose,
        source: kept.move_id.source,
        mover: kept.mover.clone(),
        baseline: kept.record.clone(),
        folder,
        pending: None,
    });
    Ok(())
}

/// Whether the move now starting with the old laptop `source` is a check or a catch-up, and which.
pub fn armed_for(source: LaptopId) -> Option<CatchupPurpose> {
    lock()
        .as_ref()
        .filter(|a| a.source == source)
        .map(|a| a.purpose)
}

/// The move ended: whatever was armed is done with.
pub fn disarm() {
    *lock() = None;
}

/// What landed on this laptop for each of the old laptop's items, as the journal recorded it and
/// as the file is on disk now (found through this laptop's own places only; a place the person
/// chose for a move is not looked up, so its items cannot be told apart and need a choice).
fn targets<R: Runtime>(app: &AppHandle<R>, source: LaptopId) -> BTreeMap<ItemId, Target> {
    let mut out = BTreeMap::new();
    let Some(j) =
        crate::moving::journal_path(app).and_then(|p| pctwin_journal::Journal::open(&p).ok())
    else {
        return out;
    };
    let Ok(entries) = j.entries() else {
        return out;
    };
    let table = crate::moving::this_laptops_places().ok().map(|(t, _)| t);
    for e in entries {
        if e.write.source_laptop != source || e.write.scope == pctwin_journal::Scope::QUICK_SEND {
            continue;
        }
        if matches!(j.undo_of(e.id), Ok(Some(pctwin_journal::Undo::Done { .. }))) {
            out.insert(e.write.item, Target::NotThere);
            continue;
        }
        let (stored, at_move) = match &e.state {
            pctwin_journal::State::Committed {
                final_path, landed, ..
            } => (
                final_path.clone(),
                Stamp {
                    size: landed.size,
                    modified_ns: landed.modified_ns,
                    digest: None,
                },
            ),
            pctwin_journal::State::Existing { stored_path } => (
                stored_path.clone(),
                Stamp {
                    size: e.write.size,
                    modified_ns: None,
                    digest: None,
                },
            ),
            _ => continue,
        };
        let now = table
            .as_ref()
            .and_then(|t| t.get(&e.write.destination).ok())
            .and_then(|d| d.shown_path().map(std::path::Path::to_path_buf))
            .map(|root| stored.split('/').fold(root, |p, n| p.join(n)))
            .map(|path| match std::fs::symlink_metadata(&path) {
                Ok(m) if m.is_file() => Some(Stamp {
                    size: m.len(),
                    modified_ns: m
                        .modified()
                        .ok()
                        .and_then(|t| t.duration_since(std::time::UNIX_EPOCH).ok())
                        .and_then(|d| i64::try_from(d.as_nanos()).ok()),
                    digest: None,
                }),
                _ => None,
            });
        // Where it is cannot be found: it cannot be told whether it changed (never "gone").
        let now = match now {
            Some(n) => n,
            None => Some(Stamp {
                size: at_move.size,
                modified_ns: None,
                digest: None,
            }),
        };
        out.insert(e.write.item, Target::Landed { at_move, now });
    }
    out
}

fn change_key(c: Change) -> ChangeKey {
    match c {
        Change::New => ChangeKey::New,
        Change::Updated => ChangeKey::Updated,
        Change::BothChanged => ChangeKey::BothChanged,
        Change::TargetGone => ChangeKey::GoneOnNewLaptop,
        Change::GoneAtSource => ChangeKey::GoneOnOldLaptop,
        Change::CannotTell => ChangeKey::CannotTell,
    }
}

fn label(r: &Record, id: ItemId) -> Option<String> {
    let i = r.items.iter().find(|i| i.id == id)?;
    Some(
        i.path
            .parts()
            .iter()
            .map(|p| p.display())
            .collect::<Vec<_>>()
            .join("/"),
    )
}

/// The old laptop's fresh list arrived, for a move armed as a check or catch-up: kept as the old
/// laptop now, its changes worked out and sent to the screens.
pub fn plan_arrived<R: Runtime>(
    app: &AppHandle<R>,
    record: &Record,
    named: &[pctwin_move::landing::NamedPerson],
    key: crate::moving::PlanKey,
) {
    let mut a = lock();
    let Some(armed) = a.as_mut().filter(|x| x.source == record.source_laptop) else {
        return;
    };
    let again = store::mover_only(record, &armed.mover);
    let now = store::OldNow {
        at: store::now(),
        record: again.clone(),
        others: store::others_of(record, &armed.mover, named),
    };
    let _ = store::save(&armed.folder, "old-now.json", &now);
    let Ok(delta) = repair::final_changes(&armed.baseline, &again, &targets(app, armed.source))
    else {
        return;
    };
    let others: u64 = now.others.iter().map(|p| p.items).sum();
    let lines: Vec<DeltaLineView> = delta
        .lines
        .iter()
        .map(|l| {
            let change = change_key(l.change);
            DeltaLineView {
                id: l.item.to_hex(),
                label: label(&again, l.item)
                    .or_else(|| label(&armed.baseline, l.item))
                    .unwrap_or_default(),
                change,
                needs_choice: change.needs_choice(),
            }
        })
        .collect();
    let view = DeltaView {
        id: armed.history_id.clone(),
        purpose: armed.purpose,
        digest: delta.digest().iter().map(|b| format!("{b:02x}")).collect(),
        more: (lines.len() as u64).saturating_sub(MAX_LINES as u64),
        lines: lines.into_iter().take(MAX_LINES).collect(),
        others,
    };
    armed.pending = Some(Pending {
        key,
        again,
        delta,
        only: None,
    });
    let _ = app.emit(DELTA_EVENT, view);
}

fn choice_of(c: CatchupChoice) -> Choice {
    match c {
        CatchupChoice::Send => Choice::Send,
        CatchupChoice::KeepBoth => Choice::KeepBoth,
        CatchupChoice::KeepNewLaptopVersion => Choice::KeepNewLaptopVersion,
    }
}

/// The person approved the reviewed changes (`seen`, the digest of the list shown): only those
/// items are sent. With nothing to send, the plan is declined and nothing moves.
pub fn approve<R: Runtime>(
    app: &AppHandle<R>,
    kept: &Kept,
    seen: [u8; 32],
    choices: &[ChoiceIn],
) -> Result<(), FinishProblem> {
    let mut a = lock();
    let armed = a
        .as_mut()
        .filter(|x| x.history_id == kept.history_id && x.purpose == CatchupPurpose::CatchUp)
        .ok_or(FinishProblem::NotAllowed)?;
    let pending = armed.pending.as_mut().ok_or(FinishProblem::Changed)?;
    let mut picked = BTreeMap::new();
    for c in choices {
        let id = ItemId::from_hex(&c.id).map_err(|_| FinishProblem::Changed)?;
        picked.insert(id, choice_of(c.choice));
    }
    let follow = repair::approve(
        &pending.delta,
        seen,
        &armed.baseline,
        &pending.again,
        &picked,
    )
    .map_err(|e| match e {
        repair::DeltaError::NeedsChoice(_) | repair::DeltaError::WouldOverwrite(_) => {
            FinishProblem::NotAllowed
        }
        _ => FinishProblem::Changed,
    })?;
    let only: BTreeSet<ItemId> = follow.record.items.iter().map(|i| i.id).collect();
    let send = !only.is_empty();
    pending.only = Some(only);
    let key = pending.key.clone();
    drop(a);
    app.state::<crate::moving::AppMoving>()
        .answer_catch_up(&key, send)
        .map_err(|_| FinishProblem::Changed)
}

/// For an approved catch-up of the old laptop `source`: every item of `record` not approved to be
/// sent (`None`: not a catch-up). A check approves nothing, so everything is left out.
pub fn left_out(record: &Record) -> Option<Vec<ItemId>> {
    let a = lock();
    let armed = a.as_ref().filter(|x| x.source == record.source_laptop)?;
    let only = armed
        .pending
        .as_ref()
        .and_then(|p| p.only.clone())
        .unwrap_or_default();
    Some(
        record
            .items
            .iter()
            .map(|i| i.id)
            .filter(|id| !only.contains(id))
            .collect(),
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn every_change_has_its_own_key() {
        let all = [
            Change::New,
            Change::Updated,
            Change::BothChanged,
            Change::TargetGone,
            Change::GoneAtSource,
            Change::CannotTell,
        ];
        let keys: BTreeSet<String> = all
            .into_iter()
            .map(|c| format!("{:?}", change_key(c)))
            .collect();
        assert_eq!(keys.len(), all.len());
        // A change that would touch a copy not as it landed always needs the person's choice.
        for c in [Change::BothChanged, Change::TargetGone, Change::CannotTell] {
            assert!(change_key(c).needs_choice(), "{c:?}");
        }
        for c in [Change::New, Change::Updated, Change::GoneAtSource] {
            assert!(!change_key(c).needs_choice(), "{c:?}");
        }
    }
}
