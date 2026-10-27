//! Conflict-aware undo of installs and settings (System Design S14, "Safe copied-file undo" last
//! paragraph and "Apps, imports and source policy"; problems S14-P07 and S14-P10; S02-P06;
//! research problem F06).
//!
//! Each effect of a move is undone in reverse order, each against what is on the laptop now,
//! never by replaying the record blindly:
//!
//! - **An app** is removed only if it was not on the laptop before the move, PCTwin saw it
//!   installed, and its recipe declares removal a safe undo touching no user data and no shared
//!   parts. It is removed through its own catalog, keeping the person's data (never a purge or a
//!   zap). An app that was there before is left. A removal that fails, or that the recipe does
//!   not allow, keeps the app and says why: the person needs to look at it.
//! - **A setting** is put back to its value from before the move (or removed, if it was not set)
//!   only if it still holds exactly what PCTwin set. If it holds anything else, the person (or
//!   an app) changed it since the move: their newer value is kept, and they are told.
//! - **Copied settings files**: each file is put back to its content from before the move (or
//!   removed, if there was none) only if its content hash is still exactly what PCTwin wrote.
//!   Any file changed since is kept; files that appeared since are never touched.
//!
//! Undo never trusts what a record says about what may be removed (security review finding 3):
//!
//! - every record is sealed with a key the journal file does not hold
//!   ([`pctwin_journal::keystore`]); a record whose seal is wrong is reported damaged and never
//!   drives anything ([`DAMAGED_RECORD`]);
//! - the undo rules (whether an app may be removed) come from the checked recipe the record
//!   names, looked up again now in the [`RecipeBook`], never from the record; and the record's
//!   app, setting or place must be exactly that recipe's step, or nothing is undone
//!   ([`NOT_THE_RECIPE`]); a recipe that cannot be found leaves it as it is ([`NO_RECIPE`]).
//!
//! Before any change, the effect's undo is recorded as part way ([`EffectUndo::Reversing`]), so
//! closing undo waits for it. After a crash, undoing a setting or settings files simply looks
//! again (the same rule gives the same answer, so it is safe to repeat); an interrupted app
//! removal is never run again by itself: it ends "not done this time", and asking for undo again
//! starts it afresh.

use std::collections::{BTreeMap, BTreeSet};

use pctwin_journal::{
    EffectEntry, EffectState, EffectUndo, EffectUndoOutcome, Found, Journal, JournalError,
    Presence, Rebuild, RebuildRole, Scope, StoredEffect, UndoPermit, content_hash,
};
use pctwin_recipe::{
    Catalog, Recipe, RecipeRef, SettingValue, SettingsPlace, Step, UndoDeclaration,
};

use crate::files::{self, Swap, Swapped};
use crate::{Adapter, SettingWrite, ours};

/// How undoing one effect ended.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Undone {
    /// The effect's number in the journal.
    pub id: u64,
    pub outcome: EffectUndoOutcome,
}

const CHANGED_SETTING: &str =
    "this setting was changed after the move, so your newer choice was kept";
const NOT_READ: &str = "PCTwin could not check this now; try undo again later";
const NOT_PUT_BACK: &str = "PCTwin could not put this back; try undo again later";
const NOT_REMOVED: &str =
    "PCTwin could not remove this app; you can remove it yourself in the system's settings";
const REMOVAL_STOPPED: &str =
    "removing this app was stopped part way; choose undo again to try once more";
const NOT_CHECKED: &str =
    "PCTwin has not finished checking this change yet; try undo again in a moment";
const ELSEWHERE: &str = "this was done with the app's maker or an online service; only they can \
     undo it, so PCTwin left it as it is";
/// How many times one file is tried in a row before undo stops for now (another program, such as
/// a backup or an indexer, may hold it for a moment).
const TRIES: u32 = 3;

/// Why a settings file could not be finished, naming where the person's file is meanwhile.
fn hidden_why(path: &str, private: &str) -> String {
    format!(
        "PCTwin could not finish putting back the settings file \"{path}\". Your file is safe: \
         it is in the same folder under the hidden name \"{private}\". Choose undo again to \
         finish, or rename that file back to \"{path}\" yourself"
    )
}

const KEPT_DAMAGED: &str =
    "the copy PCTwin kept of your earlier settings is damaged, so they were left as they are";
/// Why a record that was changed outside PCTwin was not undone.
pub const DAMAGED_RECORD: &str = "PCTwin's record of this change was changed outside PCTwin, so \
     undo did not trust it and left everything as it is";
/// Why a change whose checked instructions cannot be found was not undone.
pub const NO_RECIPE: &str = "PCTwin could not find the checked instructions this change was made \
     with, so undo left it as it is";
/// Why a change that does not match the checked instructions it names was not undone.
pub const NOT_THE_RECIPE: &str = "this change does not match the checked instructions it names, \
     so undo left it as it is";

/// Where undo finds, again, the checked recipe a change was made with (in the app: the signed
/// recipe channel's local copy). Undo's rules come only from here.
pub trait RecipeBook {
    /// The recipe with exactly this ID and version, if it is known.
    fn recipe(&self, reference: &RecipeRef) -> Option<Recipe>;
}

impl RecipeBook for Vec<Recipe> {
    fn recipe(&self, reference: &RecipeRef) -> Option<Recipe> {
        self.iter().find(|r| r.reference() == *reference).cloned()
    }
}

/// The undo rule for effect `e`, from its recipe looked up again: an app's removal declaration
/// (`None` for a setting or files), or why undo leaves it.
fn rule_of(
    e: &EffectEntry,
    recipes: &dyn RecipeBook,
) -> Result<Option<UndoDeclaration>, &'static str> {
    let recipe = recipes.recipe(&e.planned.recipe).ok_or(NO_RECIPE)?;
    if recipe.validate().is_err() {
        return Err(NO_RECIPE);
    }
    let step = usize::try_from(e.planned.step)
        .ok()
        .and_then(|n| recipe.steps.get(n))
        .ok_or(NOT_THE_RECIPE)?;
    match (step, &e.effect) {
        (
            Step::Install { catalog, package },
            StoredEffect::Install {
                catalog: c,
                package: p,
                ..
            },
        ) if catalog == c && package == p => Ok(Some(recipe.undo.clone())),
        (
            Step::ApplySetting { key, value },
            StoredEffect::ApplySetting {
                key: k, value: v, ..
            },
        ) if key == k && value == v => Ok(None),
        (Step::CopySettings { from, to }, StoredEffect::CopySettings { from: f, to: t, .. })
            if from == f && to == t =>
        {
            Ok(None)
        }
        _ => Err(NOT_THE_RECIPE),
    }
}

/// Records an undo step through the effect's own role (the installs or settings recorder).
fn record(
    rebuild: &Rebuild<'_>,
    permit: &UndoPermit<'_>,
    role: RebuildRole,
    id: u64,
    undo: &EffectUndo,
) -> Result<(), JournalError> {
    match role {
        RebuildRole::Install => rebuild.installs.record_undo(permit, id, undo),
        RebuildRole::Settings => rebuild.settings.record_undo(permit, id, undo),
    }
}

/// Undoes every installs or settings effect of move `scope` not already undone, newest first,
/// with leave from `permit` (an import's undo is the imports role's). Each ends with an outcome recorded in the journal (except one whose own change is
/// not yet checked, which is only reported). An error means the journal could not be read or
/// written: undo stops there, and what was recorded part way is finished next time.
pub fn undo_effects(
    rebuild: &Rebuild<'_>,
    permit: &UndoPermit<'_>,
    adapter: &mut dyn Adapter,
    scope: Scope,
    recipes: &dyn RecipeBook,
) -> Result<Vec<Undone>, JournalError> {
    let journal = rebuild.journal();
    let checked = journal.effects_checked(scope)?;
    // A damaged record drives nothing: reported, never recorded as undone (it cannot be read).
    let mut out: Vec<Undone> = checked
        .damaged
        .iter()
        .map(|&id| Undone {
            id,
            outcome: kept(DAMAGED_RECORD),
        })
        .collect();
    for e in checked.effects.into_iter().rev() {
        let Some(role) = ours(&e) else {
            continue;
        };
        let before = journal.effect_undo_of(e.id)?;
        if let Some(EffectUndo::Done { outcome }) = &before
            && outcome.is_final()
        {
            out.push(Undone {
                id: e.id,
                outcome: outcome.clone(),
            });
            continue;
        }
        let EffectState::Receipted { found, .. } = &e.state else {
            out.push(Undone {
                id: e.id,
                outcome: EffectUndoOutcome::NotDone {
                    why: NOT_CHECKED.into(),
                },
            });
            continue;
        };
        let mut u = One {
            journal,
            rebuild,
            role,
            permit,
            adapter: &mut *adapter,
            entry: &e,
            resumed: matches!(
                before,
                Some(
                    EffectUndo::Reversing
                        | EffectUndo::Swapping { .. }
                        | EffectUndo::PutBack { .. }
                )
            ),
        };
        let rule = rule_of(&e, recipes);
        // A settings file a crash or an error left part way put back is settled first, whatever
        // else is true now: until then the person's file may be under its private name only, and
        // nothing else is recorded (closing undo waits).
        if let Some(EffectUndo::Swapping { path, private }) = &before
            && let Err(stuck) = u.settle_left(path, private, found, rule.is_ok())?
        {
            out.push(Undone {
                id: e.id,
                outcome: stuck,
            });
            continue;
        }
        let outcome = match (&e.effect, rule) {
            // Only its maker or service can undo it, whatever the recipe says.
            (StoredEffect::External { .. }, _) => kept(ELSEWHERE),
            (_, Err(why)) => kept(why),
            (
                StoredEffect::Install {
                    catalog,
                    package,
                    before,
                    ..
                },
                Ok(rule),
            ) => {
                // The recipe's own declaration, never the one stored with the record.
                let undo = rule.unwrap_or_default();
                u.app(*catalog, package, before, &undo, found)?
            }
            (effect, Ok(_)) => match effect {
                StoredEffect::ApplySetting { key, before, .. } => u.setting(key, before, found)?,
                StoredEffect::CopySettings { to, before, .. } => {
                    let kept: BTreeMap<&str, [u8; 32]> =
                        before.iter().map(|f| (f.path.as_str(), f.hash)).collect();
                    match u.files(to, &kept, found)? {
                        Ok(outcome) => outcome,
                        // A file is part way: said, and nothing recorded over it.
                        Err(stuck) => {
                            out.push(Undone {
                                id: e.id,
                                outcome: stuck,
                            });
                            continue;
                        }
                    }
                }
                // Not reached: an install, an external change and an import (not this role's,
                // skipped above) are matched before.
                _ => continue,
            },
        };
        record(
            rebuild,
            permit,
            role,
            e.id,
            &EffectUndo::Done {
                outcome: outcome.clone(),
            },
        )?;
        out.push(Undone { id: e.id, outcome });
    }
    Ok(out)
}

/// Undoing one effect.
struct One<'a, 'j> {
    journal: &'a Journal,
    rebuild: &'a Rebuild<'j>,
    role: RebuildRole,
    permit: &'a UndoPermit<'j>,
    adapter: &'a mut dyn Adapter,
    entry: &'a EffectEntry,
    /// Its undo was recorded part way before (a crash): Reversing is already on disk.
    resumed: bool,
}

fn not_done(why: &str) -> EffectUndoOutcome {
    EffectUndoOutcome::NotDone { why: why.into() }
}

fn kept(why: &str) -> EffectUndoOutcome {
    EffectUndoOutcome::Kept { why: why.into() }
}

impl One<'_, '_> {
    /// Records the undo part way, before the first change (once).
    fn reversing(&mut self) -> Result<(), JournalError> {
        if !self.resumed {
            record(
                self.rebuild,
                self.permit,
                self.role,
                self.entry.id,
                &EffectUndo::Reversing,
            )?;
            self.resumed = true;
        }
        Ok(())
    }

    fn app(
        &mut self,
        catalog: Catalog,
        package: &str,
        before: &Presence,
        undo: &UndoDeclaration,
        found: &Found,
    ) -> Result<EffectUndoOutcome, JournalError> {
        // There before the move, or never seen installed: nothing of PCTwin's to remove.
        if *before != Presence::Absent || !matches!(found, Found::App(Presence::Present { .. })) {
            return Ok(EffectUndoOutcome::AsBefore);
        }
        if let Some(why) = undo.why_kept() {
            return Ok(kept(why));
        }
        let Ok(now) = self.adapter.app_presence(catalog, package) else {
            return Ok(not_done(NOT_READ));
        };
        match (now, self.resumed) {
            (Presence::Absent, true) => return Ok(EffectUndoOutcome::Uninstalled),
            (Presence::Absent, false) => return Ok(EffectUndoOutcome::AsBefore),
            // Never run a removal again by itself after a crash.
            (Presence::Present { .. }, true) => return Ok(not_done(REMOVAL_STOPPED)),
            (Presence::Present { .. }, false) => {}
        }
        self.reversing()?;
        let removed = self.adapter.uninstall(catalog, package);
        let gone = matches!(
            self.adapter.app_presence(catalog, package),
            Ok(Presence::Absent)
        );
        Ok(if removed.is_ok() && gone {
            EffectUndoOutcome::Uninstalled
        } else {
            kept(NOT_REMOVED)
        })
    }

    fn setting(
        &mut self,
        key: &str,
        before: &Option<SettingValue>,
        found: &Found,
    ) -> Result<EffectUndoOutcome, JournalError> {
        let Found::Setting(applied) = found else {
            return Err(JournalError::Damaged(
                "a setting's record holds another kind of observation",
            ));
        };
        let Ok(now) = self.adapter.read_setting(key) else {
            return Ok(not_done(NOT_READ));
        };
        if now == *before {
            return Ok(if self.resumed {
                EffectUndoOutcome::Restored
            } else {
                EffectUndoOutcome::AsBefore
            });
        }
        let Some(expect) = applied.as_ref().filter(|_| now == *applied) else {
            return Ok(kept(CHANGED_SETTING));
        };
        self.reversing()?;
        // Compared and written in one step: a newer choice made since the look above is kept.
        match self.adapter.write_setting_if(key, expect, before.as_ref()) {
            Ok(SettingWrite::Written) => {}
            Ok(SettingWrite::Changed) => return Ok(kept(CHANGED_SETTING)),
            Err(_) => return Ok(not_done(NOT_PUT_BACK)),
        }
        Ok(match self.adapter.read_setting(key) {
            Ok(back) if back == *before => EffectUndoOutcome::Restored,
            // Still PCTwin's: the write did not take.
            Ok(back) if back == *applied => not_done(NOT_PUT_BACK),
            // Neither: someone set it again right after; theirs is kept.
            Ok(_) => kept(CHANGED_SETTING),
            Err(_) => not_done(NOT_READ),
        })
    }

    /// Puts one settings file back as `swap` says (its private name already recorded as
    /// [`EffectUndo::Swapping`]), trying a few times in a row; only when that says the file is
    /// settled is [`EffectUndo::PutBack`] recorded. `Err` with what to tell the person (naming
    /// where their file is) when it could not be finished: the record stays part way.
    fn settle(
        &mut self,
        place: &SettingsPlace,
        path: &str,
        swap: &Swap<'_>,
    ) -> Result<Result<Swapped, EffectUndoOutcome>, JournalError> {
        let mut tries = 0;
        let swapped = loop {
            match self.adapter.replace_settings_file(place, path, swap) {
                Ok(s) => break s,
                Err(_) => {
                    tries += 1;
                    if tries >= TRIES {
                        return Ok(Err(not_done(&hidden_why(path, swap.private))));
                    }
                    std::thread::sleep(std::time::Duration::from_millis(50 * u64::from(tries)));
                }
            }
        };
        record(
            self.rebuild,
            self.permit,
            self.role,
            self.entry.id,
            &EffectUndo::PutBack {
                path: path.to_string(),
                private: swap.private.to_string(),
            },
        )?;
        Ok(Ok(swapped))
    }

    /// Settles the settings file a crash or an error left part way put back (recorded as
    /// [`EffectUndo::Swapping`]): put back as planned when its earlier version can be had and
    /// the checked instructions still allow it, otherwise only brought out of hiding. Undo then
    /// looks at the place afresh.
    fn settle_left(
        &mut self,
        path: &str,
        private: &str,
        found: &Found,
        allowed: bool,
    ) -> Result<Result<(), EffectUndoOutcome>, JournalError> {
        let (StoredEffect::CopySettings { to, before, .. }, Found::Files(written)) =
            (&self.entry.effect, found)
        else {
            return Err(JournalError::Damaged(
                "an undo record names a file PCTwin did not write",
            ));
        };
        let Some(ours) = written.iter().find(|f| f.path == path).map(|f| f.hash) else {
            return Err(JournalError::Damaged(
                "an undo record names a file PCTwin did not write",
            ));
        };
        let kept: BTreeMap<&str, [u8; 32]> =
            before.iter().map(|f| (f.path.as_str(), f.hash)).collect();
        let earlier = if allowed {
            self.earlier(&kept, path)?.ok()
        } else {
            None
        };
        let swap = Swap {
            private,
            ours,
            earlier: earlier.as_ref().and_then(|b| b.as_deref()),
            only_unhide: earlier.is_none(),
        };
        Ok(self.settle(to, path, &swap)?.map(|_| ()))
    }

    /// Puts the settings files of one copy back. `Err` with what to tell the person when a file
    /// is left part way (nothing is recorded over it).
    fn files(
        &mut self,
        place: &SettingsPlace,
        before: &BTreeMap<&str, [u8; 32]>,
        found: &Found,
    ) -> Result<Result<EffectUndoOutcome, EffectUndoOutcome>, JournalError> {
        let Found::Files(written) = found else {
            return Err(JournalError::Damaged(
                "a settings copy's record holds another kind of observation",
            ));
        };
        let written: BTreeMap<&str, [u8; 32]> =
            written.iter().map(|f| (f.path.as_str(), f.hash)).collect();
        let Ok(now) = self.adapter.read_settings_files(place) else {
            return Ok(Ok(not_done(NOT_READ)));
        };
        let now: BTreeMap<String, [u8; 32]> = now
            .into_iter()
            .map(|f| (f.path, content_hash(&f.bytes)))
            .collect();
        let paths: BTreeSet<&str> = before.keys().chain(written.keys()).copied().collect();
        let mut to_restore = Vec::new();
        let mut changed = 0usize;
        for path in paths {
            let current = now.get(path);
            if current == before.get(path) {
                continue;
            }
            if current == written.get(path) {
                to_restore.push(path);
            } else {
                changed += 1;
            }
        }
        let kept_why = |changed: usize| {
            format!(
                "{changed} of these settings files changed after the move, so your newer versions \
                 were kept; the rest were put back"
            )
        };
        if to_restore.is_empty() {
            return Ok(Ok(match (changed, self.resumed) {
                (0, false) => EffectUndoOutcome::AsBefore,
                (0, true) => EffectUndoOutcome::Restored,
                (n, _) => EffectUndoOutcome::Kept { why: kept_why(n) },
            }));
        }
        // Every earlier version is read (and checked) before anything is changed.
        let mut restores = Vec::with_capacity(to_restore.len());
        for path in &to_restore {
            match self.earlier(before, path)? {
                Ok(bytes) => restores.push((*path, bytes)),
                Err(outcome) => return Ok(Ok(outcome)),
            }
        }
        // Each file is put back only if it is still PCTwin's version, checked and replaced in
        // one step that cannot lose a save made meanwhile; its private name is recorded first.
        let mut put_back = Vec::with_capacity(restores.len());
        for (path, bytes) in &restores {
            let Some(ours) = written.get(path) else {
                // PCTwin's copy left no file here, so there is nothing of its own to swap: the
                // earlier file is not made again over whatever may come; said, not hidden.
                changed += 1;
                continue;
            };
            let Ok(private) = files::private_name() else {
                return Ok(Ok(not_done(NOT_PUT_BACK)));
            };
            record(
                self.rebuild,
                self.permit,
                self.role,
                self.entry.id,
                &EffectUndo::Swapping {
                    path: (*path).to_string(),
                    private: private.clone(),
                },
            )?;
            self.resumed = true;
            let swap = Swap {
                private: &private,
                ours: *ours,
                earlier: bytes.as_deref(),
                only_unhide: false,
            };
            match self.settle(place, path, &swap)? {
                Ok(Swapped::Restored) => put_back.push(*path),
                // Gone with nothing before it: as it was before the move.
                Ok(Swapped::Gone) if bytes.is_none() => put_back.push(*path),
                // Changed or removed meanwhile: the person's is kept.
                Ok(Swapped::Kept | Swapped::Gone) => changed += 1,
                // Left part way: nothing is recorded over it.
                Err(stuck) => return Ok(Err(stuck)),
            }
        }
        let restores: Vec<_> = restores
            .into_iter()
            .filter(|(path, _)| put_back.contains(path))
            .collect();
        // Read back: every file put back is exactly as it was before the move.
        let Ok(back) = self.adapter.read_settings_files(place) else {
            return Ok(Ok(not_done(NOT_READ)));
        };
        let back: BTreeMap<String, [u8; 32]> = back
            .into_iter()
            .map(|f| (f.path, content_hash(&f.bytes)))
            .collect();
        if restores
            .iter()
            .any(|(path, _)| back.get(*path) != before.get(path))
        {
            return Ok(Ok(not_done(NOT_PUT_BACK)));
        }
        Ok(Ok(if changed == 0 {
            EffectUndoOutcome::Restored
        } else {
            EffectUndoOutcome::Kept {
                why: kept_why(changed),
            }
        }))
    }

    /// The earlier version of `path` (none when there was no file), read and checked from what
    /// the journal kept; `Err` with the outcome when that copy is damaged.
    fn earlier(
        &self,
        before: &BTreeMap<&str, [u8; 32]>,
        path: &str,
    ) -> Result<Result<Option<Vec<u8>>, EffectUndoOutcome>, JournalError> {
        match before.get(path) {
            Some(hash) => match self.journal.kept_bytes(hash) {
                Ok(b) => Ok(Ok(Some(b))),
                Err(JournalError::Damaged(_)) => Ok(Err(kept(KEPT_DAMAGED))),
                Err(e) => Err(e),
            },
            None => Ok(Ok(None)),
        }
    }
}
