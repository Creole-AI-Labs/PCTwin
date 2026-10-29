//! Undo of a move's settings, and what it would do, looked at now (R05, R06).
//!
//! Each setting is put back only if it still holds exactly what PCTwin set (the executor's
//! compare-and-write): a value the person changed after the move is kept, and said. Apps PCTwin
//! installed from a Windows Installer package are removed through the administrator helper
//! (Windows' own `msiexec /x`), only for a product the helper's own journal recorded installing
//! and only while it is still the version installed; another version is kept, and said
//! ([`AppUndo`], decided 28 October 2026). Apps installed by an installer program, and apps where
//! there is no helper, are listed for the person to remove in the system's settings if they want.
//! Files that were copied are not touched by this undo (the old laptop still has every
//! original).
//!
//! Lines are by group (the app or part of the system the settings belong to), as the plan showed
//! them.

use std::collections::BTreeMap;

use pctwin_executor::{Adapter, Undone, undo_effects};
use pctwin_journal::{EffectUndoOutcome, Journal, JournalError, Rebuild, Scope, StoredEffect};
use serde::Serialize;

use pctwin_apps::found::windows::{RawEntry, RegValue};
use pctwin_helper::Acted;
use pctwin_helper::broker::actions::{
    Action, ApprovedPlan, Hex16, Id, InstallDate, InstalledProduct, PlannedOperation,
    ProductVersion, UninstallWindowsInstallerProduct,
};
use pctwin_helper::broker::receipt::Outcome;
use pctwin_helper::broker::uninstall::UninstallResult;
use pctwin_helper::broker::{BrokerRequest, PROTOCOL};

use crate::install::{Helper, Machine};
use crate::run::{RemovableApp, Settled, UndoBook};

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
    /// An app PCTwin installed, still the version installed: PCTwin removes it.
    RemoveApp,
    /// An app PCTwin installed that is another version now: kept.
    KeepOtherVersion,
    /// An app PCTwin installed that is no longer there.
    AlreadyGone,
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
    /// An app PCTwin installed, removed and no longer in the system's list; whether a restart
    /// finishes it.
    Removed { restart_needed: bool },
    /// An app PCTwin installed that is another version now: kept.
    KeptOtherVersion,
    /// An app PCTwin installed that could not be removed this time (the prompt declined, or the
    /// removal failed or could not be confirmed): the person can remove it in the system's
    /// settings.
    NotRemoved,
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
    apps: &mut dyn AppUndo,
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
    // App data landed: removed again, each file only while it is still what PCTwin wrote.
    for (app, _) in &book.landed {
        if !out.iter().any(|l| l.name == *app) {
            out.push(PlanLine {
                name: app.clone(),
                plan: UndoPlan::PutBack,
            });
        }
    }
    out.extend(book.removable.iter().map(|a| PlanLine {
        name: a.name.clone(),
        plan: match a.settled {
            None => apps.plan(a),
            Some(settled) => settled_plan(settled),
        },
    }));
    out.extend(book.apps.iter().map(|a| PlanLine {
        name: a.clone(),
        plan: UndoPlan::RemoveYourself,
    }));
    Ok(out)
}

/// Undoes the move's settings through the executor (compare-and-write: a setting the person
/// changed since is kept), group by group, with `rebuild` (the journal's recorders) and
/// `adapter` (which must know `book`'s places). Each app whose removal ended is marked settled in
/// `book`, which the caller keeps: a later undo never asks for it again.
pub fn undo(
    rebuild: &Rebuild<'_>,
    book: &mut UndoBook,
    adapter: &mut dyn Adapter,
    data: Option<(&pctwin_grant::CaptureKey, &pctwin_gate::Destination)>,
    apps: &mut dyn AppUndo,
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
    // App data landed (2.5): each file removed only while it is still what PCTwin wrote; one the
    // person changed since is kept.
    if let Some((key, home)) = data {
        let rec = journal.imports(key);
        for (app, effect) in &book.landed {
            let result = match pctwin_special::api::undo(&rec, &permit, home, *effect) {
                Ok(pctwin_special::land::Undone::Removed) => Result::Restored,
                Ok(pctwin_special::land::Undone::AlreadyUndone) => Result::AsBefore,
                Ok(pctwin_special::land::Undone::KeptChanged { kept }) => {
                    Result::KeptChanged { kept: kept as u64 }
                }
                Ok(pctwin_special::land::Undone::NotDone(_)) | Err(_) => {
                    Result::NotDoneYet { left: 1 }
                }
            };
            out.push(Line {
                name: app.clone(),
                result,
            });
        }
    }
    // Apps PCTwin installed from a Windows Installer package: removed through the helper, each
    // checked gone afterwards.
    let pending: Vec<RemovableApp> = book
        .removable
        .iter()
        .filter(|a| a.settled.is_none())
        .cloned()
        .collect();
    let mut removed = apps.remove(book.scope, &pending).into_iter();
    for a in &mut book.removable {
        let result = match a.settled {
            Some(settled) => settled_result(settled),
            None => {
                let result = removed.next().unwrap_or(Result::NotRemoved);
                a.settled = settled_by(&result);
                result
            }
        };
        out.push(Line {
            name: a.name.clone(),
            result,
        });
    }
    out.extend(book.apps.iter().map(|a| Line {
        name: a.clone(),
        result: Result::RemoveYourself,
    }));
    Ok(out)
}

/// What a later preview says of an app an earlier undo settled: never asked for again.
fn settled_plan(settled: Settled) -> UndoPlan {
    match settled {
        Settled::Removed | Settled::AlreadyGone => UndoPlan::AlreadyGone,
        Settled::KeptOtherVersion => UndoPlan::KeepOtherVersion,
    }
}

/// What a later undo says of an app an earlier undo settled: never asked for again.
fn settled_result(settled: Settled) -> Result {
    match settled {
        Settled::Removed | Settled::AlreadyGone => Result::AsBefore,
        Settled::KeptOtherVersion => Result::KeptOtherVersion,
    }
}

/// Whether `result` ends undo for an app (so it is never asked for again), and how.
fn settled_by(result: &Result) -> Option<Settled> {
    match result {
        Result::Removed { .. } => Some(Settled::Removed),
        Result::AsBefore => Some(Settled::AlreadyGone),
        Result::KeptOtherVersion => Some(Settled::KeptOtherVersion),
        _ => None,
    }
}

/* ---------- Apps PCTwin installed (decided 28 October 2026) ---------- */

/// Undo of the apps PCTwin installed: what it would do with each, and doing it.
pub trait AppUndo {
    /// What undo would do with `app`, looked at now. Nothing changes and nobody is asked.
    fn plan(&mut self, app: &RemovableApp) -> UndoPlan;
    /// Removes `apps` of the move `for_move` (its scope), one result each, in order.
    fn remove(&mut self, for_move: [u8; 16], apps: &[RemovableApp]) -> Vec<Result>;
}

/// Where PCTwin has no administrator helper to remove apps with: each is listed for the person
/// to remove in the system's settings.
pub struct ListOnly;

impl AppUndo for ListOnly {
    fn plan(&mut self, _: &RemovableApp) -> UndoPlan {
        UndoPlan::RemoveYourself
    }
    fn remove(&mut self, _: [u8; 16], apps: &[RemovableApp]) -> Vec<Result> {
        apps.iter().map(|_| Result::RemoveYourself).collect()
    }
}

/// Removing through the administrator helper (Windows): Windows' own `msiexec /x`, only for a
/// product the helper's own journal recorded installing, and only while it is the version
/// installed; then Windows' list is read again, and an app is said removed only when the list no
/// longer shows it.
pub struct HelperUndo<'a> {
    pub helper: &'a mut dyn Helper,
    pub machine: &'a dyn Machine,
    /// The one-use numbers of the helper's requests (the first also names this attempt).
    pub nonce: &'a mut dyn FnMut() -> Option<[u8; 16]>,
}

/// What Windows' list shows of a product now.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Look {
    /// There at the version PCTwin installed.
    Same,
    /// There at another version (or one that cannot be read).
    Other,
    Gone,
    /// The list could not be read (it is never empty on Windows): the helper looks itself.
    Unknown,
}

fn look(entries: &[RawEntry], product: &InstalledProduct) -> Look {
    if entries.is_empty() {
        return Look::Unknown;
    }
    let Some(e) = entries
        .iter()
        .find(|e| e.product_code().as_deref() == Some(product.code.as_str()))
    else {
        return Look::Gone;
    };
    let text = |name: &str| match e.values.get(name) {
        Some(RegValue::Text(v)) => Some(v.as_str()),
        _ => None,
    };
    // The version installed, written into the list on the same day: one installed again since is
    // the person's (review finding F1).
    let version = text("displayversion").and_then(ProductVersion::parse);
    let day = text("installdate").and_then(InstallDate::parse);
    if version.as_ref() == Some(&product.version) && day.as_ref() == Some(&product.installed_on) {
        Look::Same
    } else {
        Look::Other
    }
}

impl HelperUndo<'_> {
    /// The result of one removal, from the helper's answer and Windows' list read again.
    fn judged(&self, answer: std::result::Result<Acted, String>, p: &InstalledProduct) -> Result {
        let gone = look(&self.machine.entries(), p) == Look::Gone;
        match answer {
            Ok(Acted::Done(r)) => match r.outcome {
                Outcome::Uninstall(UninstallResult::Removed { restart_needed }) if gone => {
                    Result::Removed { restart_needed }
                }
                Outcome::Uninstall(UninstallResult::AlreadyGone) if gone => Result::AsBefore,
                // An earlier attempt the helper found finished after a crash (review finding F2).
                Outcome::AlreadyInPlace if gone => Result::Removed {
                    restart_needed: false,
                },
                Outcome::Uninstall(UninstallResult::KeptOtherVersion) => Result::KeptOtherVersion,
                _ => Result::NotRemoved,
            },
            _ => Result::NotRemoved,
        }
    }
}

impl AppUndo for HelperUndo<'_> {
    fn plan(&mut self, app: &RemovableApp) -> UndoPlan {
        match look(&self.machine.entries(), &app.removable.product) {
            Look::Same | Look::Unknown => UndoPlan::RemoveApp,
            Look::Other => UndoPlan::KeepOtherVersion,
            Look::Gone => UndoPlan::AlreadyGone,
        }
    }

    fn remove(&mut self, for_move: [u8; 16], apps: &[RemovableApp]) -> Vec<Result> {
        let entries = self.machine.entries();
        let mut out: Vec<Option<Result>> = Vec::with_capacity(apps.len());
        let mut wanted = Vec::new();
        for (at, a) in apps.iter().enumerate() {
            out.push(match look(&entries, &a.removable.product) {
                Look::Same | Look::Unknown => {
                    wanted.push(at);
                    None
                }
                Look::Other => Some(Result::KeptOtherVersion),
                Look::Gone => Some(Result::AsBefore),
            });
        }
        if !wanted.is_empty() {
            self.remove_through_helper(for_move, apps, &wanted, &mut out);
        }
        out.into_iter()
            .map(|r| r.unwrap_or(Result::NotRemoved))
            .collect()
    }
}

impl HelperUndo<'_> {
    /// One prompt for the plan of removals `wanted` (places in `apps`), then each asked for and
    /// judged into `out`.
    fn remove_through_helper(
        &mut self,
        for_move: [u8; 16],
        apps: &[RemovableApp],
        wanted: &[usize],
        out: &mut [Option<Result>],
    ) {
        // Each attempt names its own actions, so a removal that failed is tried again next time
        // instead of being answered from the helper's record of the last attempt.
        let Some(tag) = (self.nonce)() else {
            return;
        };
        let tag: String = tag[..6].iter().map(|b| format!("{b:02x}")).collect();
        let mut operations = Vec::with_capacity(wanted.len());
        for (n, &at) in wanted.iter().enumerate() {
            let Some(action_id) = Id::parse(&format!("uninstall.{n}.{tag}")) else {
                return;
            };
            let r = &apps[at].removable;
            operations.push((
                at,
                PlannedOperation {
                    action_id,
                    action: Action::UninstallWindowsInstallerProduct(
                        UninstallWindowsInstallerProduct {
                            product_code: r.product.code,
                            installed_by: r.installed_by,
                            for_move: Hex16(for_move),
                        },
                    ),
                },
            ));
        }
        let plan = ApprovedPlan {
            generation: 1,
            operations: operations.iter().map(|(_, o)| o.clone()).collect(),
        };
        if self.helper.start(&plan).is_err() {
            // Declined, or the helper could not start: nothing was removed.
            return;
        }
        for (at, op) in &operations {
            let answer = match (self.nonce)() {
                Some(nonce) => self.helper.act(&BrokerRequest {
                    protocol: PROTOCOL,
                    audience: op.action.audience(),
                    request_id: op.action_id.clone(),
                    plan_generation: plan.generation,
                    plan_digest: plan.digest(),
                    action_id: op.action_id.clone(),
                    action: op.action.clone(),
                    nonce: Hex16(nonce),
                }),
                None => Err("no one-use number".into()),
            };
            out[*at] = Some(self.judged(answer, &apps[*at].removable.product));
        }
        self.helper.bye();
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Review finding F1: only a removal that ended settles an app (removed, already gone, kept
    /// because it changed); one not removed, or left for the person, is asked for again.
    #[test]
    fn only_an_ended_removal_settles_an_app() {
        assert_eq!(
            settled_by(&Result::Removed {
                restart_needed: true
            }),
            Some(Settled::Removed)
        );
        assert_eq!(settled_by(&Result::AsBefore), Some(Settled::AlreadyGone));
        assert_eq!(
            settled_by(&Result::KeptOtherVersion),
            Some(Settled::KeptOtherVersion)
        );
        // A settled app is said as it was left, and never asked for again.
        assert_eq!(settled_plan(Settled::Removed), UndoPlan::AlreadyGone);
        assert_eq!(settled_plan(Settled::AlreadyGone), UndoPlan::AlreadyGone);
        assert_eq!(
            settled_plan(Settled::KeptOtherVersion),
            UndoPlan::KeepOtherVersion
        );
        assert_eq!(settled_result(Settled::Removed), Result::AsBefore);
        assert_eq!(settled_result(Settled::AlreadyGone), Result::AsBefore);
        assert_eq!(
            settled_result(Settled::KeptOtherVersion),
            Result::KeptOtherVersion
        );
        for open in [
            Result::NotRemoved,
            Result::RemoveYourself,
            Result::NotDoneYet { left: 1 },
            Result::Restored,
        ] {
            assert_eq!(settled_by(&open), None, "{open:?}");
        }
    }
}
