//! Shared types used across the PCTwin engine and app.

use serde::{Deserialize, Serialize};

/// How a single selected item ended up after a move.
///
/// Every selected item ends with exactly one of these. A success count must
/// never hide items that ended in any other status.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum ItemStatus {
    /// The data was moved from the old laptop.
    Copied,
    /// The app was installed fresh from a trusted catalog.
    Reinstalled,
    /// A setting, file or app was translated to its equivalent on the new system.
    Converted,
    /// The person must finish a step, such as signing in.
    NeedsAction,
    /// The person chose to leave it out.
    SkippedByUser,
    /// It did not complete; the reason is shown to the person.
    Failed,
}

impl ItemStatus {
    /// Every status, in the order reports list them.
    pub const ALL: [ItemStatus; 6] = [
        ItemStatus::Copied,
        ItemStatus::Reinstalled,
        ItemStatus::Converted,
        ItemStatus::NeedsAction,
        ItemStatus::SkippedByUser,
        ItemStatus::Failed,
    ];

    /// True only when the item is fully done on the new laptop with nothing left for the person to do.
    pub fn is_done(self) -> bool {
        matches!(
            self,
            ItemStatus::Copied | ItemStatus::Reinstalled | ItemStatus::Converted
        )
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn serialises_with_stable_names() {
        let names: Vec<String> = ItemStatus::ALL
            .iter()
            .map(|s| serde_json::to_string(s).expect("serialise"))
            .collect();
        assert_eq!(
            names,
            [
                "\"copied\"",
                "\"reinstalled\"",
                "\"converted\"",
                "\"needs-action\"",
                "\"skipped-by-user\"",
                "\"failed\""
            ]
        );
    }

    #[test]
    fn round_trips_every_status() {
        for status in ItemStatus::ALL {
            let json = serde_json::to_string(&status).expect("serialise");
            let back: ItemStatus = serde_json::from_str(&json).expect("deserialise");
            assert_eq!(back, status);
        }
    }

    #[test]
    fn only_completed_work_counts_as_done() {
        let done: Vec<ItemStatus> = ItemStatus::ALL
            .into_iter()
            .filter(|s| s.is_done())
            .collect();
        assert_eq!(
            done,
            [
                ItemStatus::Copied,
                ItemStatus::Reinstalled,
                ItemStatus::Converted
            ]
        );
    }

    #[test]
    fn unknown_status_is_rejected() {
        assert!(serde_json::from_str::<ItemStatus>("\"done\"").is_err());
    }
}
