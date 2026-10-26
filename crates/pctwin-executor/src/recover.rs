//! Reconciling changes a crash interrupted (System Design S14: "On crash, reconcile exact
//! identities and observed content"; S01-P02 and Phase 2 plan 2.3, section 5: never replay an
//! installer blindly).
//!
//! An effect with an intent and no observation may or may not have happened. Recovery never
//! runs the change again: it asks the system what is there now, records that as the effect's
//! observation (marked as found after a restart), and works out the receipt from it and the
//! step's own check, exactly as a run would. An effect with an observation and no receipt gets
//! its receipt the same way. Where the system cannot tell now (a catalog tool that fails, a
//! setting that cannot be read), or an install may still be running, the effect is left exactly
//! as it is for next time, and said; nothing is changed while it is.
//!
//! Only the installs and settings roles' effects are reconciled here; an import's belongs to the
//! imports role. A recipe a crash stopped part way is not finished by recovery: running the plan
//! again does the rest, and an app already on the laptop is never installed again.

use pctwin_journal::{EffectLedger, EffectState, JournalError, Receipt, StoredEffect};
use pctwin_record::ItemId;

use crate::{Adapter, check_of, judge, look, ours};

/// What recovery did with one interrupted change.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Recovered {
    /// The effect's number in the journal.
    pub id: u64,
    pub item: ItemId,
    /// Its step in the recipe.
    pub step: u32,
    pub outcome: RecoveredAs,
}

/// How an interrupted change ended after a restart.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum RecoveredAs {
    /// Its receipt, worked out from what is on the laptop now.
    Receipt(Receipt),
    /// What is there now cannot be told yet (why, in plain words): left as it is for next time.
    Left(String),
}

/// Why an install is left for next time while it may still be running.
pub const STILL_INSTALLING: &str =
    "this app may still be installing; PCTwin will look again once it has finished";

/// Reconciles every unfinished installs or settings effect in `journal`, in the order intended,
/// by looking at the laptop through `adapter` (never changing it). An error means the journal
/// could not be read or written: nothing more may be changed.
pub fn recover(
    journal: &dyn EffectLedger,
    adapter: &mut dyn Adapter,
) -> Result<Vec<Recovered>, JournalError> {
    let mut out = Vec::new();
    for e in journal.unfinished_effects()? {
        if !ours(&e) {
            continue;
        }
        let left = |why: String| Recovered {
            id: e.id,
            item: e.planned.item,
            step: e.planned.step,
            outcome: RecoveredAs::Left(why),
        };
        let found = match &e.state {
            EffectState::Intended => {
                if let StoredEffect::Install {
                    catalog, package, ..
                } = &e.effect
                {
                    match adapter.install_running(*catalog, package) {
                        Ok(false) => {}
                        Ok(true) => {
                            out.push(left(STILL_INSTALLING.into()));
                            continue;
                        }
                        Err(why) => {
                            out.push(left(why));
                            continue;
                        }
                    }
                }
                match look(adapter, &e.effect) {
                    Ok(found) => {
                        journal.observed(e.role, e.id, &found, true)?;
                        found
                    }
                    Err(why) => {
                        out.push(left(why));
                        continue;
                    }
                }
            }
            EffectState::Observed { found, .. } => found.clone(),
            // Not unfinished; never listed as such.
            EffectState::Receipted { .. } => continue,
        };
        let passed = check_of(&e.effect).is_some_and(|c| adapter.verify(&c));
        let receipt = judge(&e.effect, &found, None, passed);
        journal.receipt(e.role, e.id, &receipt)?;
        out.push(Recovered {
            id: e.id,
            item: e.planned.item,
            step: e.planned.step,
            outcome: RecoveredAs::Receipt(receipt),
        });
    }
    Ok(out)
}
