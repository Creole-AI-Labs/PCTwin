//! Step 3: fetching a download in a separate, locked-down process (ADOPTION change 1).
//!
//! The app never downloads an installer itself. It asks the fetch process
//! (`pctwin-fetch`, [`process`]) for exactly one download ([`download::Request`]) and reads back
//! one answer ([`download::Outcome`]). That process:
//!
//! - **Windows:** runs at *low integrity*: its program file is PCTwin's own copy marked low with
//!   Windows' own `icacls`, so Windows starts it below the person's level. It can write only in
//!   its own low folder (`LocalLow`), cannot change the person's files or PCTwin, and cannot
//!   reach the person's other programs. It can still read what the person can read: full
//!   isolation (an AppContainer) needs a system call PCTwin would have to fence, awaiting the
//!   user's approval, so it is not used.
//! - **Mac:** runs inside the system's own sandbox (`sandbox-exec` with a profile that allows
//!   only the network and writing in its staging folder) (not yet proven on a real Mac).
//! - **Linux:** runs as a separate process with an empty environment and no confinement beyond
//!   that yet (not built yet: Landlock).
//! - starts with an empty environment, in its own staging folder, and is stopped when its time
//!   is up;
//! - keeps the download's origin on it ([`marks`]): Windows' mark of the web, the Mac's
//!   quarantine flag; it never removes one;
//! - never opens or runs what it downloads.
//!
//! The rules it follows are [`policy`] (where a download may come from) and [`download`] (how).

pub mod download;
pub mod marks;
pub mod policy;
pub mod process;

pub use download::{Expected, NetworkCost, Outcome, Request};

/// What the network costs now, as the system says. Windows asks its own connection profile;
/// elsewhere the system offers no simple answer, so it is unknown (asked of the person).
pub fn network_cost() -> NetworkCost {
    #[cfg(windows)]
    {
        use windows::Networking::Connectivity::{NetworkCostType, NetworkInformation};
        let cost = NetworkInformation::GetInternetConnectionProfile()
            .and_then(|p| p.GetConnectionCost())
            .and_then(|c| {
                Ok((
                    c.NetworkCostType()?,
                    c.Roaming()?,
                    c.OverDataLimit()?,
                    c.ApproachingDataLimit()?,
                ))
            });
        match cost {
            Ok((NetworkCostType::Unrestricted, false, false, false)) => NetworkCost::Unmetered,
            Ok((NetworkCostType::Unknown, ..)) | Err(_) => NetworkCost::Unknown,
            Ok(_) => NetworkCost::Metered,
        }
    }
    #[cfg(not(windows))]
    {
        NetworkCost::Unknown
    }
}
