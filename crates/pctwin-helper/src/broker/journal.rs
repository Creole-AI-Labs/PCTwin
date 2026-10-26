//! The broker's [`IntentJournal`] on the undo journal (`pctwin-journal`, its broker records):
//! the join of 2.2 and 2.3 (27 October 2026).
//!
//! Each privileged action's intent is written there, durably, before the action, and its outcome
//! before the answer. A record that cannot be read back into the helper's own types (an outcome
//! that does not parse, a record changed by hand, the journal's key not found) makes the journal
//! unavailable for that action: nothing is done, never a guess.

use pctwin_journal::{BrokerAction, BrokerRecorder, BrokerState};

use super::receipt::Outcome;
use super::{Intent, IntentJournal, JournalRef, JournalUnavailable, Recorded};

/// The broker's intents and outcomes, kept in the helper's own journal.
pub struct JournalIntents<'j> {
    record: BrokerRecorder<'j>,
}

impl<'j> JournalIntents<'j> {
    /// Over the broker's recorder of the helper's journal (made with the broker's own key).
    pub fn new(record: BrokerRecorder<'j>) -> Self {
        Self { record }
    }
}

impl IntentJournal for JournalIntents<'_> {
    fn lookup(
        &mut self,
        idempotency: &super::actions::Hex32,
    ) -> Result<Recorded, JournalUnavailable> {
        match self.record.lookup(&idempotency.0) {
            Ok(BrokerState::Nothing) => Ok(Recorded::Nothing),
            Ok(BrokerState::Unfinished(id)) => Ok(Recorded::Unfinished(JournalRef(id))),
            Ok(BrokerState::Finished(id, text)) => serde_json::from_str::<Outcome>(&text)
                .map(|o| Recorded::Finished(JournalRef(id), o))
                .map_err(|_| JournalUnavailable),
            Err(_) => Err(JournalUnavailable),
        }
    }

    fn record_intent(&mut self, intent: &Intent) -> Result<JournalRef, JournalUnavailable> {
        let action = serde_json::to_string(&intent.action).map_err(|_| JournalUnavailable)?;
        self.record
            .intend(&BrokerAction {
                key: intent.idempotency.0,
                plan_generation: intent.plan_generation,
                plan_digest: intent.plan_digest.0,
                action_id: intent.action_id.as_str().to_string(),
                action,
            })
            .map(JournalRef)
            .map_err(|_| JournalUnavailable)
    }

    fn record_outcome(
        &mut self,
        at: JournalRef,
        outcome: &Outcome,
    ) -> Result<(), JournalUnavailable> {
        let text = serde_json::to_string(outcome).map_err(|_| JournalUnavailable)?;
        self.record
            .outcome(at.0, &text)
            .map_err(|_| JournalUnavailable)
    }
}
