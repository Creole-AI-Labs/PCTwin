//! Undo of a move's settings, and what it would do, looked at now (R05, R06).
//!
//! Each setting is put back only if it still holds exactly what PCTwin set (the executor's
//! compare-and-write): a value the person changed after the move is kept, and said. Apps PCTwin
//! installed are not removed by PCTwin yet: each is listed for the person to remove in the
//! system's settings if they want. Files that were copied are not touched by this undo (the old
//! laptop still has every original).
//!
//! Lines are by group (the app or part of the system the settings belong to), as the plan showed
//! them.

use std::collections::BTreeMap;

use pctwin_executor::{Adapter, Undone, undo_effects};
use pctwin_journal::{EffectUndoOutcome, Journal, JournalError, Rebuild, Scope, StoredEffect};
use serde::Serialize;

use crate::run::UndoBook;

/// What undo will do with one group, looked at now.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "kebab-case")]
pub enum UndoPlan {
    /// Every setting still holds what PCTwin set: each is put back.
    PutBack,
    /// Some were changed since the move: those are kept, the rest put back.
    KeepChanged,
    /// An app PCTwin installed: the person removes it in the system's settings if they want.
    RemoveYourself,
}

/// One line of the preview.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct PlanLine {
    pub name: String,
    pub plan: UndoPlan,
}

/// How undoing one group ended.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(tag = "kind", rename_all = "kebab-case")]
pub enum Result {
    /// Every setting is back as it was before the move.
    Restored,
    /// Nothing to put back: it was already as before.
    AsBefore,
    /// Some were kept because they changed after the move (how many); the rest are back.
    KeptChanged { kept: u64 },
    /// Some could not be done this time (how many); undo tries them again next time.
    NotDoneYet { left: u64 },
    /// An app: left for the person to remove.
    RemoveYourself,
}

/// One line of the undo report.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct Line {
    pub name: String,
    pub result: Result,
}

/// The settings of `book`'s move as the journal holds them: (executor key, what PCTwin set).
fn applied(
    journal: &Journal,
    scope: Scope,
) -> std::result::Result<Vec<(u64, String, pctwin_recipe::SettingValue)>, JournalError> {
    Ok(journal
        .effects_of(scope)?
        .into_iter()
        .filter_map(|e| match e.effect {
            StoredEffect::ApplySetting { key, value, .. } => Some((e.id, key, value)),
            _ => None,
        })
        .collect())
}

/// What undo would do now, group by group, reading each setting here through `adapter` (which
/// must know `book`'s places). Nothing is changed.
pub fn preview(
    journal: &Journal,
    book: &UndoBook,
    adapter: &mut dyn Adapter,
) -> std::result::Result<Vec<PlanLine>, JournalError> {
    let mut groups: BTreeMap<String, bool> = BTreeMap::new();
    for (_, key, set) in applied(journal, Scope(book.scope))? {
        let label = book.labels.get(&key).cloned().unwrap_or_default();
        let still = matches!(adapter.read_setting(&key), Ok(Some(v)) if v == set);
        let changed = groups.entry(label).or_default();
        *changed |= !still;
    }
    let mut out: Vec<PlanLine> = groups
        .into_iter()
        .map(|(name, changed)| PlanLine {
            name,
            plan: if changed {
                UndoPlan::KeepChanged
            } else {
                UndoPlan::PutBack
            },
        })
        .collect();
    out.extend(book.apps.iter().map(|a| PlanLine {
        name: a.clone(),
        plan: UndoPlan::RemoveYourself,
    }));
    Ok(out)
}

/// Undoes the move's settings through the executor (compare-and-write: a setting the person
/// changed since is kept), group by group, with `rebuild` (the journal's recorders) and
/// `adapter` (which must know `book`'s places).
pub fn undo(
    rebuild: &Rebuild<'_>,
    book: &UndoBook,
    adapter: &mut dyn Adapter,
) -> std::result::Result<Vec<Line>, JournalError> {
    let journal = rebuild.journal();
    let scope = Scope(book.scope);
    let keys: BTreeMap<u64, String> = applied(journal, scope)?
        .into_iter()
        .map(|(id, key, _)| (id, key))
        .collect();
    let permit = journal.begin_undo()?;
    let undone = undo_effects(rebuild, &permit, adapter, scope, &book.recipes)?;
    // (restored, as before, kept, not done) per group.
    let mut groups: BTreeMap<String, (u64, u64, u64, u64)> = BTreeMap::new();
    for Undone { id, outcome } in undone {
        let Some(key) = keys.get(&id) else { continue };
        let label = book.labels.get(key).cloned().unwrap_or_default();
        let g = groups.entry(label).or_default();
        match outcome {
            EffectUndoOutcome::Restored | EffectUndoOutcome::Uninstalled => g.0 += 1,
            EffectUndoOutcome::AsBefore => g.1 += 1,
            EffectUndoOutcome::Kept { .. } => g.2 += 1,
            EffectUndoOutcome::NotDone { .. } => g.3 += 1,
        }
    }
    let mut out: Vec<Line> = groups
        .into_iter()
        .map(|(name, (restored, _as_before, kept, left))| Line {
            name,
            result: if left > 0 {
                Result::NotDoneYet { left }
            } else if kept > 0 {
                Result::KeptChanged { kept }
            } else if restored > 0 {
                Result::Restored
            } else {
                Result::AsBefore
            },
        })
        .collect();
    out.extend(book.apps.iter().map(|a| Line {
        name: a.clone(),
        result: Result::RemoveYourself,
    }));
    Ok(out)
}
