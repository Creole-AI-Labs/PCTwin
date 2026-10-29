//! What the helper hands back after an action: a receipt of what it observed, signed with a key
//! only this session knows, because it crosses from the helper's process to the app's and is kept
//! in the app's records. It reports; it never authorises anything.
//!
//! What the signature does not prove (security review, 26 October 2026; later roles must not rely
//! on it): the receipt key is derived from the session token, and the app holds that token too.
//! So a receipt shows it was not changed on its way or in the app's records by anyone without the
//! token; it is no proof against the app itself, which could make one. Nothing may treat a
//! receipt as permission or as evidence the app could not have forged.

use serde::{Deserialize, Serialize};

use super::actions::{Hex16, Hex32, Id, InstalledProduct};
use super::restore::RestorePointResult;
use super::uninstall::UninstallResult;

/// What really happened.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case", deny_unknown_fields)]
pub enum Outcome {
    /// Done and observed; whether the laptop must restart to finish it.
    Done { restart_needed: bool },
    /// A Windows Installer package installed, and the helper saw its product appear in the
    /// system's list (absent before, present after, at this version): the record undo needs to
    /// remove it, kept in the helper's own journal.
    Installed {
        restart_needed: bool,
        product: InstalledProduct,
        /// The move the install was for: only an undo of that move may remove it.
        for_move: Hex16,
    },
    /// How removing an app PCTwin installed ended, when it did not fail.
    Uninstall(UninstallResult),
    /// A restore point's honest result.
    RestorePoint(RestorePointResult),
    /// Found already in place when an unfinished earlier attempt was checked: not done again.
    AlreadyInPlace,
    /// Part of it happened (a system action that could not be stopped was stopped short, or
    /// failed half-way): the system's code only.
    Partial { code: i64 },
    /// It failed and nothing was changed, as far as the system says: its code only.
    Failed { code: i64 },
}

/// The signed receipt.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case", deny_unknown_fields)]
pub struct Receipt {
    pub request_id: Id,
    pub action_id: Id,
    pub plan_generation: u64,
    pub plan_digest: Hex32,
    /// The action's one key, made by the helper from the plan, the action's ID and the action
    /// itself (never one the app chose).
    pub idempotency: Hex32,
    pub outcome: Outcome,
    /// Where the change is in the undo journal.
    pub journal: u64,
    /// Whether the outcome was written to the journal (false: it will be checked again before
    /// anything is retried).
    pub recorded: bool,
    /// Whether this is an earlier result handed back again, not a new action.
    pub replayed: bool,
    pub mac: Hex32,
}

/// This session's receipt key, from its token (which only the app and this helper know).
pub fn receipt_key(session_token: &str) -> [u8; 32] {
    blake3::derive_key(
        "PCTwin helper receipt key 2026-10-26 v1",
        session_token.as_bytes(),
    )
}

impl Receipt {
    fn body(&self) -> Vec<u8> {
        let mut unsigned = self.clone();
        unsigned.mac = Hex32::default();
        serde_json::to_vec(&unsigned).unwrap_or_default()
    }

    pub fn signed(mut self, key: &[u8; 32]) -> Self {
        self.mac = Hex32(*blake3::keyed_hash(key, &self.body()).as_bytes());
        self
    }

    /// Whether this receipt was signed with `key` and not changed since (compared in constant
    /// time).
    pub fn verify(&self, key: &[u8; 32]) -> bool {
        blake3::keyed_hash(key, &self.body()) == blake3::Hash::from_bytes(self.mac.0)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn receipt() -> Receipt {
        Receipt {
            request_id: Id::parse("r1").unwrap(),
            action_id: Id::parse("a1").unwrap(),
            plan_generation: 4,
            plan_digest: Hex32([1; 32]),
            idempotency: Hex32([2; 32]),
            outcome: Outcome::Done {
                restart_needed: true,
            },
            journal: 9,
            recorded: true,
            replayed: false,
            mac: Hex32::default(),
        }
    }

    #[test]
    fn a_receipt_is_checked_against_its_session() {
        let key = receipt_key("token-a");
        let r = receipt().signed(&key);
        assert_ne!(r.mac, Hex32::default());
        assert!(r.verify(&key));
        assert!(!r.verify(&receipt_key("token-b")));
        assert_ne!(receipt_key("token-a"), receipt_key("token-b"));
        let wire = serde_json::to_string(&r).unwrap();
        let back: Receipt = serde_json::from_str(&wire).unwrap();
        assert!(back.verify(&key));
    }

    #[test]
    fn a_changed_receipt_fails_its_check() {
        let key = receipt_key("t");
        let r = receipt().signed(&key);
        let mut changes: Vec<Receipt> = Vec::new();
        let mut c = r.clone();
        c.outcome = Outcome::Failed { code: 1 };
        changes.push(c);
        let mut c = r.clone();
        c.plan_generation = 5;
        changes.push(c);
        let mut c = r.clone();
        c.replayed = true;
        changes.push(c);
        let mut c = r.clone();
        c.recorded = false;
        changes.push(c);
        let mut c = r.clone();
        c.request_id = Id::parse("r2").unwrap();
        changes.push(c);
        let mut c = r.clone();
        c.mac.0[0] ^= 1;
        changes.push(c);
        for c in changes {
            assert!(!c.verify(&key), "{c:?}");
        }
    }
}
