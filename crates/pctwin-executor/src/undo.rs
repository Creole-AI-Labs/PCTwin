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
use pctwin_recipe::{Catalog, SettingValue, SettingsPlace, UndoDeclaration};

use crate::{Adapter, ours};

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
const KEPT_DAMAGED: &str =
    "the copy PCTwin kept of your earlier settings is damaged, so they were left as they are";

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
) -> Result<Vec<Undone>, JournalError> {
    let journal = rebuild.journal();
    let mut out = Vec::new();
    for e in journal.effects_of(scope)?.into_iter().rev() {
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
            resumed: before == Some(EffectUndo::Reversing),
        };
        let outcome = match &e.effect {
            StoredEffect::Install {
                catalog,
                package,
                before,
                undo,
            } => u.app(*catalog, package, before, undo, found)?,
            StoredEffect::ApplySetting { key, before, .. } => u.setting(key, before, found)?,
            StoredEffect::CopySettings { to, before, .. } => {
                let kept: BTreeMap<&str, [u8; 32]> =
                    before.iter().map(|f| (f.path.as_str(), f.hash)).collect();
                u.files(to, &kept, found)?
            }
            // Never claimed undone here: only its maker or service can undo it.
            StoredEffect::External { .. } => kept(ELSEWHERE),
            // Not this role's: skipped above.
            StoredEffect::Import { .. } => continue,
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
                "a setting's record holds another kind of observation".into(),
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
        if now != *applied {
            return Ok(kept(CHANGED_SETTING));
        }
        self.reversing()?;
        if self.adapter.write_setting(key, before.as_ref()).is_err() {
            return Ok(not_done(NOT_PUT_BACK));
        }
        Ok(match self.adapter.read_setting(key) {
            Ok(back) if back == *before => EffectUndoOutcome::Restored,
            _ => not_done(NOT_PUT_BACK),
        })
    }

    fn files(
        &mut self,
        place: &SettingsPlace,
        before: &BTreeMap<&str, [u8; 32]>,
        found: &Found,
    ) -> Result<EffectUndoOutcome, JournalError> {
        let Found::Files(written) = found else {
            return Err(JournalError::Damaged(
                "a settings copy's record holds another kind of observation".into(),
            ));
        };
        let written: BTreeMap<&str, [u8; 32]> =
            written.iter().map(|f| (f.path.as_str(), f.hash)).collect();
        let Ok(now) = self.adapter.read_settings_files(place) else {
            return Ok(not_done(NOT_READ));
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
            return Ok(match (changed, self.resumed) {
                (0, false) => EffectUndoOutcome::AsBefore,
                (0, true) => EffectUndoOutcome::Restored,
                (n, _) => EffectUndoOutcome::Kept { why: kept_why(n) },
            });
        }
        // Every earlier version is read (and checked) before anything is changed.
        let mut restores = Vec::with_capacity(to_restore.len());
        for path in &to_restore {
            let bytes = match before.get(path) {
                Some(hash) => match self.journal.kept_bytes(hash) {
                    Ok(b) => Some(b),
                    Err(JournalError::Damaged(_)) => return Ok(kept(KEPT_DAMAGED)),
                    Err(e) => return Err(e),
                },
                None => None,
            };
            restores.push((*path, bytes));
        }
        self.reversing()?;
        for (path, bytes) in &restores {
            if self
                .adapter
                .put_settings_file(place, path, bytes.as_deref())
                .is_err()
            {
                return Ok(not_done(NOT_PUT_BACK));
            }
        }
        // Read back: every file put back is exactly as it was before the move.
        let Ok(back) = self.adapter.read_settings_files(place) else {
            return Ok(not_done(NOT_READ));
        };
        let back: BTreeMap<String, [u8; 32]> = back
            .into_iter()
            .map(|f| (f.path, content_hash(&f.bytes)))
            .collect();
        if restores
            .iter()
            .any(|(path, _)| back.get(*path) != before.get(path))
        {
            return Ok(not_done(NOT_PUT_BACK));
        }
        Ok(if changed == 0 {
            EffectUndoOutcome::Restored
        } else {
            EffectUndoOutcome::Kept {
                why: kept_why(changed),
            }
        })
    }
}
