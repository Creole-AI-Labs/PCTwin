//! An item's result with its reason (product notes: "every file and app that failed is named, with
//! the reason and a fix").
//!
//! An [`ItemResult`] can only be made in a shape the report can show honestly: an item that failed
//! or needs the person always carries a [`Reason`] that fits its status, and a finished or skipped
//! item carries none. Each reason's stable name is also the key of its translated text, which says
//! what happened and what to do next.

use serde::{Deserialize, Serialize};

use crate::ItemStatus;

/// Why an item did not finish. Each one belongs to exactly one status ([`Reason::status`]).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum Reason {
    // Failed: PCTwin could not finish it.
    /// The new laptop ran out of space.
    NotEnoughSpace,
    /// The file kept changing on the old laptop while it was being read.
    ChangedDuringMove,
    /// The old laptop would not let PCTwin read it.
    Unreadable,
    /// What arrived did not match the original, so it was not kept.
    NotVerified,
    /// The new laptop's safety check refused it (a path, name or size that was not allowed).
    RefusedBySafetyCheck,
    /// The name cannot exist on the new laptop's system and no safe new name was possible.
    NameNotAllowed,
    /// Installing the app did not succeed.
    InstallFailed,
    /// There is no version or equivalent for the new laptop's system.
    NoEquivalent,
    /// The drive it lands on is locked (BitLocker or another drive lock): nothing could be
    /// written there until it is unlocked.
    DriveLocked,

    // Needs action: it is in place, but the person must finish a step.
    /// The person must sign in.
    SignIn,
    /// The app must be activated with its licence on the new laptop.
    Activation,
    /// The system must give permission first.
    Permission,
    /// The file is online-only and must be downloaded by its cloud app.
    OnlineOnly,
    /// The app must be downloaded from its maker's site by the person.
    ManualInstall,
    /// The new laptop must restart to finish.
    Restart,
    /// A step PCTwin cannot do for the person; the report lists it as a checklist entry.
    FinishByHand,
    /// A folder that moves as one whole (a Photos Library, a project's history) arrived without
    /// every file inside it: those are listed in the report with why.
    PartsMissing,
    /// A setting that points at a moved file or folder that is not on the new laptop under the
    /// name the move gave it: the setting was left as it was, for the person to choose the file
    /// again.
    MovedFileMissing,
}

impl Reason {
    /// Every reason, in report order.
    pub const ALL: [Reason; 18] = [
        Reason::NotEnoughSpace,
        Reason::ChangedDuringMove,
        Reason::Unreadable,
        Reason::NotVerified,
        Reason::RefusedBySafetyCheck,
        Reason::NameNotAllowed,
        Reason::InstallFailed,
        Reason::NoEquivalent,
        Reason::DriveLocked,
        Reason::SignIn,
        Reason::Activation,
        Reason::Permission,
        Reason::OnlineOnly,
        Reason::ManualInstall,
        Reason::Restart,
        Reason::FinishByHand,
        Reason::PartsMissing,
        Reason::MovedFileMissing,
    ];

    /// The one status this reason belongs to.
    pub fn status(self) -> ItemStatus {
        match self {
            Reason::NotEnoughSpace
            | Reason::ChangedDuringMove
            | Reason::Unreadable
            | Reason::NotVerified
            | Reason::RefusedBySafetyCheck
            | Reason::NameNotAllowed
            | Reason::InstallFailed
            | Reason::NoEquivalent
            | Reason::DriveLocked => ItemStatus::Failed,
            Reason::SignIn
            | Reason::Activation
            | Reason::Permission
            | Reason::OnlineOnly
            | Reason::ManualInstall
            | Reason::Restart
            | Reason::FinishByHand
            | Reason::PartsMissing
            | Reason::MovedFileMissing => ItemStatus::NeedsAction,
        }
    }
}

/// Why an item result was refused.
#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
pub enum ResultError {
    #[error("a {0:?} item must say why")]
    MissingReason(ItemStatus),
    #[error("a {status:?} item cannot carry the reason {reason:?}")]
    ReasonDoesNotFit { status: ItemStatus, reason: Reason },
}

/// How one item ended, with its reason when it did not finish.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(try_from = "ResultWire", into = "ResultWire")]
pub struct ItemResult {
    status: ItemStatus,
    reason: Option<Reason>,
}

impl ItemResult {
    /// A finished item: copied, reinstalled or converted, and verified.
    pub fn done(status: ItemStatus) -> Result<Self, ResultError> {
        Self::new(status, None)
    }

    /// The person chose to leave it out.
    pub fn skipped_by_user() -> Self {
        Self {
            status: ItemStatus::SkippedByUser,
            reason: None,
        }
    }

    /// An item that did not finish; its status follows from the reason.
    pub fn because(reason: Reason) -> Self {
        Self {
            status: reason.status(),
            reason: Some(reason),
        }
    }

    /// Checks that the reason fits the status.
    pub fn new(status: ItemStatus, reason: Option<Reason>) -> Result<Self, ResultError> {
        match (status, reason) {
            (ItemStatus::Failed | ItemStatus::NeedsAction, None) => {
                Err(ResultError::MissingReason(status))
            }
            (_, Some(reason)) if reason.status() != status => {
                Err(ResultError::ReasonDoesNotFit { status, reason })
            }
            _ => Ok(Self { status, reason }),
        }
    }

    pub fn status(&self) -> ItemStatus {
        self.status
    }

    pub fn reason(&self) -> Option<Reason> {
        self.reason
    }
}

#[derive(Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
struct ResultWire {
    status: ItemStatus,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    reason: Option<Reason>,
}

impl TryFrom<ResultWire> for ItemResult {
    type Error = ResultError;
    fn try_from(w: ResultWire) -> Result<Self, ResultError> {
        Self::new(w.status, w.reason)
    }
}

impl From<ItemResult> for ResultWire {
    fn from(r: ItemResult) -> Self {
        Self {
            status: r.status,
            reason: r.reason,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn unfinished_items_must_say_why() {
        for status in [ItemStatus::Failed, ItemStatus::NeedsAction] {
            assert_eq!(
                ItemResult::new(status, None),
                Err(ResultError::MissingReason(status))
            );
        }
    }

    #[test]
    fn finished_and_skipped_items_carry_no_reason() {
        for status in [
            ItemStatus::Copied,
            ItemStatus::Reinstalled,
            ItemStatus::Converted,
            ItemStatus::SkippedByUser,
        ] {
            assert!(ItemResult::new(status, None).is_ok());
            for reason in Reason::ALL {
                assert!(ItemResult::new(status, Some(reason)).is_err());
            }
        }
    }

    #[test]
    fn every_reason_fits_exactly_one_status() {
        for reason in Reason::ALL {
            let fits: Vec<ItemStatus> = ItemStatus::ALL
                .into_iter()
                .filter(|s| ItemResult::new(*s, Some(reason)).is_ok())
                .collect();
            assert_eq!(fits, [reason.status()]);
            assert_eq!(ItemResult::because(reason).status(), reason.status());
        }
    }

    #[test]
    fn saved_results_are_checked_when_read() {
        let r = ItemResult::because(Reason::NotEnoughSpace);
        let json = serde_json::to_string(&r).expect("serialise");
        assert_eq!(json, r#"{"status":"failed","reason":"not-enough-space"}"#);
        assert_eq!(serde_json::from_str::<ItemResult>(&json).expect("read"), r);
        assert_eq!(
            serde_json::to_string(&ItemResult::skipped_by_user()).expect("serialise"),
            r#"{"status":"skipped-by-user"}"#
        );
        for bad in [
            r#"{"status":"failed"}"#,
            r#"{"status":"copied","reason":"sign-in"}"#,
            r#"{"status":"needs-action","reason":"not-verified"}"#,
            r#"{"status":"copied","extra":1}"#,
        ] {
            assert!(serde_json::from_str::<ItemResult>(bad).is_err(), "{bad}");
        }
    }

    #[test]
    fn reason_names_are_stable_translation_keys() {
        let names: Vec<String> = Reason::ALL
            .iter()
            .map(|r| serde_json::to_string(r).expect("serialise"))
            .collect();
        assert_eq!(
            names.join(","),
            "\"not-enough-space\",\"changed-during-move\",\"unreadable\",\"not-verified\",\
             \"refused-by-safety-check\",\"name-not-allowed\",\"install-failed\",\"no-equivalent\",\
             \"drive-locked\",\"sign-in\",\"activation\",\"permission\",\"online-only\",\"manual-install\",\
             \"restart\",\"finish-by-hand\",\"parts-missing\",\"moved-file-missing\""
        );
    }
}
