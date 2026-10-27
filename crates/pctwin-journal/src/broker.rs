//! The administrator helper's own record of its privileged actions (Phase 2.2 plan section 4,
//! steps 6, 10 and 12; joined to the 2.3 journal on 27 October 2026).
//!
//! The helper writes each action's intent here, durably, before it acts, and its outcome before it
//! answers. After a crash between the two, the next helper finds the intent with no outcome and
//! checks what is really on the laptop before anything is tried again: an action is never done
//! twice blindly. Each action is found by its one key (made by the helper from the plan, the
//! action's ID and the action itself), so the same action asked again is answered from here.
//!
//! The helper keeps its own journal, in its own administrators-only folder: the app's journal is
//! the app's (only one program opens a journal at a time). Records are sealed as effect records
//! are ([`crate::keystore`]): one changed by hand is damaged, never trusted. Only the broker's own
//! key makes the recorder ([`pctwin_grant::BrokerKey`]).
//!
//! What the action and its outcome say is the helper's own (this journal keeps them as the
//! helper's text, at most [`MAX_BROKER_TEXT`] bytes each, and reads them back only into the
//! helper's own checked types).

use pctwin_grant::BrokerKey;
use redb::{ReadableDatabase, ReadableTable, TableDefinition};
use serde::{Deserialize, Serialize};

use crate::keystore::TagKey;
use crate::{Journal, JournalError, damaged, storage, unreadable};

/// Each broker action, by number.
pub(crate) const BROKER: TableDefinition<u64, &[u8]> = TableDefinition::new("broker-actions");
/// Each action's number, by its one key.
pub(crate) const BROKER_KEYS: TableDefinition<[u8; 32], u64> = TableDefinition::new("broker-keys");
/// The next broker action number (in the `counters` table).
pub(crate) const NEXT_BROKER: &str = "next-broker";
/// The name broker records are sealed under.
const BROKER_SEAL: &str = "broker";

/// The longest action or outcome text kept.
pub const MAX_BROKER_TEXT: usize = 4096;

/// An action about to be taken: its one key, the plan it belongs to, its ID and the action itself
/// (as the helper writes it).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct BrokerAction {
    pub key: [u8; 32],
    pub plan_generation: u64,
    pub plan_digest: [u8; 32],
    pub action_id: String,
    pub action: String,
}

/// What the record holds for an action's key.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum BrokerState {
    /// Never recorded.
    Nothing,
    /// Its intent, with no outcome: the helper stopped before it could write one.
    Unfinished(u64),
    /// Finished, with its outcome (the helper's text).
    Finished(u64, String),
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
struct Stored {
    id: u64,
    action: BrokerAction,
    outcome: Option<String>,
}

/// The broker's way to write its record. Made with [`Journal::broker`].
#[derive(Debug)]
pub struct BrokerRecorder<'j> {
    journal: &'j Journal,
}

fn seal(key: &TagKey, id: u64, stored: &Stored) -> Result<Vec<u8>, JournalError> {
    let json = serde_json::to_vec(stored).map_err(storage)?;
    Ok([key.tag(BROKER_SEAL, id, &json).as_slice(), &json].concat())
}

fn unseal(key: &TagKey, id: u64, bytes: &[u8]) -> Result<Stored, JournalError> {
    let (tag, json) = bytes
        .split_first_chunk::<32>()
        .ok_or(damaged("a record is too short to be one of PCTwin's"))?;
    if !key.verify(BROKER_SEAL, id, json, tag) {
        return Err(damaged("a record was changed outside PCTwin"));
    }
    let stored: Stored = serde_json::from_slice(json).map_err(unreadable)?;
    if stored.id != id {
        return Err(damaged("a broker record is filed under another number"));
    }
    Ok(stored)
}

fn too_long(text: &str) -> bool {
    text.is_empty() || text.len() > MAX_BROKER_TEXT
}

impl Journal {
    /// The broker's recorder, made only with the broker's own key.
    pub fn broker<'j>(&'j self, _key: &'j BrokerKey) -> BrokerRecorder<'j> {
        BrokerRecorder { journal: self }
    }
}

impl BrokerRecorder<'_> {
    /// What the record holds for the action with this one key.
    pub fn lookup(&self, action_key: &[u8; 32]) -> Result<BrokerState, JournalError> {
        let j = self.journal;
        let tx = j.db.begin_read().map_err(storage)?;
        let keys = tx.open_table(BROKER_KEYS).map_err(storage)?;
        let Some(id) = keys.get(*action_key).map_err(storage)?.map(|v| v.value()) else {
            return Ok(BrokerState::Nothing);
        };
        let actions = tx.open_table(BROKER).map_err(storage)?;
        let bytes = actions
            .get(id)
            .map_err(storage)?
            .ok_or(damaged("a broker record is missing"))?;
        let stored = unseal(&j.record_key()?, id, bytes.value())?;
        if stored.action.key != *action_key {
            return Err(damaged("a broker record is filed under another key"));
        }
        Ok(match stored.outcome {
            None => BrokerState::Unfinished(id),
            Some(outcome) => BrokerState::Finished(id, outcome),
        })
    }

    /// Records an action about to be taken, durably, before it is; returns its number. Refused if
    /// this key was recorded before (it is looked up first, and answered from the record).
    pub fn intend(&self, action: &BrokerAction) -> Result<u64, JournalError> {
        if too_long(&action.action) || too_long(&action.action_id) {
            return Err(JournalError::TooLarge);
        }
        let j = self.journal;
        let key = j.record_key()?;
        let tx = j.db.begin_write().map_err(storage)?;
        let id = {
            let mut keys = tx.open_table(BROKER_KEYS).map_err(storage)?;
            if keys.get(action.key).map_err(storage)?.is_some() {
                return Err(JournalError::OutOfOrder {
                    from: "intended",
                    to: "intended",
                });
            }
            let mut counters = tx.open_table(crate::COUNTERS).map_err(storage)?;
            let id = counters
                .get(NEXT_BROKER)
                .map_err(storage)?
                .map_or(1, |v| v.value());
            counters.insert(NEXT_BROKER, id + 1).map_err(storage)?;
            let stored = Stored {
                id,
                action: action.clone(),
                outcome: None,
            };
            let mut actions = tx.open_table(BROKER).map_err(storage)?;
            actions
                .insert(id, seal(&key, id, &stored)?.as_slice())
                .map_err(storage)?;
            keys.insert(action.key, id).map_err(storage)?;
            id
        };
        tx.commit().map_err(storage)?;
        Ok(id)
    }

    /// Records action `id`'s outcome, durably, before the helper answers: once, and only for an
    /// action intended and not finished.
    pub fn outcome(&self, id: u64, outcome: &str) -> Result<(), JournalError> {
        if too_long(outcome) {
            return Err(JournalError::TooLarge);
        }
        let j = self.journal;
        let key = j.record_key()?;
        let tx = j.db.begin_write().map_err(storage)?;
        {
            let mut actions = tx.open_table(BROKER).map_err(storage)?;
            let mut stored = {
                let bytes = actions
                    .get(id)
                    .map_err(storage)?
                    .ok_or(JournalError::NoSuchEntry(id))?;
                unseal(&key, id, bytes.value())?
            };
            if stored.outcome.is_some() {
                return Err(JournalError::OutOfOrder {
                    from: "finished",
                    to: "finished",
                });
            }
            stored.outcome = Some(outcome.to_string());
            actions
                .insert(id, seal(&key, id, &stored)?.as_slice())
                .map_err(storage)?;
        }
        tx.commit().map_err(storage)
    }
}
