//! Cloud files, honest by default (Task List 2.6; acceptance T26-014 to T26-016).
//!
//! A file in a personal cloud folder ends one of three ways:
//!
//! - **stored online only** (its contents are not on the laptop): never downloaded, never copied
//!   and never shown as copied. It is safe in its cloud and needs one step: signing in on the new
//!   laptop ([`CloudOutcome::SafeInCloud`], needs action).
//! - **on the laptop**: copied like any other file. A file waiting to upload exists only here, so
//!   unless PCTwin can tell it has been uploaded it counts as "only on this laptop"
//!   ([`CloudOutcome::CopyHere`]). PCTwin cannot yet tell (see [`sync_fact`]), so today every such
//!   file counts as only here: the honest default.
//! - **work or school** ("OneDrive - Company"): left out, "managed by your employer"
//!   ([`CloudOutcome::ManagedByEmployer`]).
//!
//! A sync app's own sign-in state (its tokens and settings) is never copied in place of signing
//! in: [`sign_in_state`] names those folders so no later step that copies app data can take them.
//! Today app data is only counted, never listed, so none can be copied in the first place.

use pctwin_record::{CloudProvider, Inclusion, Item, LeftOutReason, ManagedBy, Storage};
use pctwin_types::{ItemResult, Reason};

/// Whether a file on the laptop has been uploaded to its cloud.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SyncFact {
    /// The cloud has this exact version.
    Uploaded,
    /// The cloud app has not uploaded it yet: it exists only on this laptop.
    WaitingToUpload,
    /// PCTwin could not tell. Treated as waiting to upload.
    NotKnown,
}

/// Whether the file described by `meta` has been uploaded to its cloud.
///
/// Always [`SyncFact::NotKnown`] for now. On Windows the "in sync" state of a cloud file is kept
/// in its reparse data and read through the Cloud Files functions (`CfGetPlaceholderStateFrom...`),
/// which can only be called with `unsafe` code; the standard library does not expose the reparse
/// tag. macOS keeps it in `NSURLUbiquitousItemIsUploadedKey`, reachable only through Objective-C.
/// No safe route exists, so rather than add a new unsafe function this answers "not known", and
/// the plan treats every such file as only on this laptop and copies it. Nothing is lost by this;
/// the "only on the old laptop" list is longer than it might be.
pub fn sync_fact(_meta: &std::fs::Metadata) -> SyncFact {
    SyncFact::NotKnown
}

/// How one item in (or from) a cloud folder ends.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CloudOutcome {
    /// Not a cloud matter: nothing here applies. `included` is whether the move copies it (an
    /// ordinary file is copied as always; one left out for another reason is not).
    NotInCloud { included: bool },
    /// Its contents are on the laptop: copied like any other file. `only_on_this_laptop` unless
    /// PCTwin knows the cloud has this version.
    CopyHere {
        provider: Option<CloudProvider>,
        only_on_this_laptop: bool,
    },
    /// Stored online only: safe in its cloud, never copied; comes back when the person signs in
    /// on the new laptop. `None`: a cloud PCTwin cannot name.
    SafeInCloud { provider: Option<CloudProvider> },
    /// A work or school folder: it stays with the employer or school.
    ManagedByEmployer,
}

/// Whose item it is, as the words name them.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Whose<'a> {
    /// The person moving.
    Mover,
    /// Another person, by the name the screens show for them.
    Other { name: &'a str },
}

/// The provider's own name, or `None` for one PCTwin does not know.
pub fn provider_name(provider: CloudProvider) -> Option<&'static str> {
    match provider {
        CloudProvider::OneDrive => Some("OneDrive"),
        CloudProvider::ICloud => Some("iCloud Drive"),
        CloudProvider::Dropbox => Some("Dropbox"),
        CloudProvider::GoogleDrive => Some("Google Drive"),
        CloudProvider::Other => None,
    }
}

fn named(provider: CloudProvider) -> Option<CloudProvider> {
    provider_name(provider).map(|_| provider)
}

/// How `item` ends, given whether its contents are uploaded (`sync`, used only for a file on the
/// laptop).
pub fn cloud_outcome(item: &Item, sync: SyncFact) -> CloudOutcome {
    if item.managed_by == ManagedBy::Organisation {
        return CloudOutcome::ManagedByEmployer;
    }
    let provider = match item.place.storage {
        Storage::Cloud { provider } => named(provider),
        _ => None,
    };
    match item.inclusion {
        Inclusion::LeftOut {
            reason: LeftOutReason::CloudOnly,
        } => CloudOutcome::SafeInCloud { provider },
        Inclusion::LeftOut {
            reason: LeftOutReason::WorkManaged,
        } => CloudOutcome::ManagedByEmployer,
        Inclusion::LeftOut { .. } => CloudOutcome::NotInCloud { included: false },
        Inclusion::Included => match item.place.storage {
            Storage::Cloud { .. } => CloudOutcome::CopyHere {
                provider,
                only_on_this_laptop: sync != SyncFact::Uploaded,
            },
            _ => CloudOutcome::NotInCloud { included: true },
        },
    }
}

impl CloudOutcome {
    /// Whether the move copies it.
    pub fn copies(&self) -> bool {
        match self {
            CloudOutcome::CopyHere { .. } => true,
            CloudOutcome::SafeInCloud { .. } | CloudOutcome::ManagedByEmployer => false,
            CloudOutcome::NotInCloud { included } => *included,
        }
    }

    /// The result the plan already knows: needs action for what stays in a cloud. A copy's own
    /// result comes only from the move itself (`None`).
    pub fn planned_result(&self) -> Option<ItemResult> {
        match self {
            CloudOutcome::SafeInCloud { .. } => Some(ItemResult::because(Reason::OnlineOnly)),
            CloudOutcome::ManagedByEmployer => Some(ItemResult::because(Reason::SignIn)),
            CloudOutcome::CopyHere { .. } | CloudOutcome::NotInCloud { .. } => None,
        }
    }

    /// The plain words for the plan and the report, or `None` when there is nothing to say.
    pub fn words(&self, whose: &Whose<'_>) -> Option<String> {
        let name = |p: &Option<CloudProvider>| p.and_then(provider_name);
        match (self, whose) {
            (CloudOutcome::NotInCloud { .. }, _) => None,
            (
                CloudOutcome::CopyHere {
                    only_on_this_laptop: false,
                    ..
                },
                _,
            ) => None,
            (CloudOutcome::CopyHere { provider, .. }, _) => Some(format!(
                "Only on this laptop: {} may not have finished uploading it, so PCTwin copies it.",
                name(provider).unwrap_or("the cloud app")
            )),
            (CloudOutcome::SafeInCloud { provider }, Whose::Mover) => Some(match name(provider) {
                Some(p) => format!(
                    "Safe in {p}, not on this laptop. It comes back when you sign in to {p} on the new laptop."
                ),
                None => "Safe in your cloud storage, not on this laptop. It comes back when you sign in to it on the new laptop.".to_string(),
            }),
            (CloudOutcome::SafeInCloud { provider }, Whose::Other { name: who }) => {
                Some(format!(
                    "Only in {who}'s {}: {who} signs in on the new laptop to get them.",
                    name(provider).unwrap_or("cloud storage")
                ))
            }
            (CloudOutcome::ManagedByEmployer, _) => Some(
                "This is managed by your employer or school, so it stays with them. Sign in to your work or school account on the new laptop to get it.".to_string(),
            ),
        }
    }
}

/// The system an old laptop's home folder comes from.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Os {
    Windows,
    Mac,
    Linux,
}

/// Where each sync app keeps its sign-in state and settings, below a person's home folder.
const SIGN_IN_STATE: [(Os, &[&str], CloudProvider); 15] = [
    (
        Os::Windows,
        &["AppData", "Local", "Microsoft", "OneDrive"],
        CloudProvider::OneDrive,
    ),
    (
        Os::Windows,
        &["AppData", "Local", "Dropbox"],
        CloudProvider::Dropbox,
    ),
    (
        Os::Windows,
        &["AppData", "Roaming", "Dropbox"],
        CloudProvider::Dropbox,
    ),
    (
        Os::Windows,
        &["AppData", "Local", "Google", "DriveFS"],
        CloudProvider::GoogleDrive,
    ),
    (
        Os::Windows,
        &["AppData", "Roaming", "Apple Computer", "iCloud"],
        CloudProvider::ICloud,
    ),
    (
        Os::Windows,
        &["AppData", "Local", "Apple Inc", "iCloudDrive"],
        CloudProvider::ICloud,
    ),
    (
        Os::Mac,
        &["Library", "Application Support", "CloudDocs"],
        CloudProvider::ICloud,
    ),
    (
        Os::Mac,
        &[
            "Library",
            "Group Containers",
            "UBF8T346G9.OneDriveSyncClientSuite",
        ],
        CloudProvider::OneDrive,
    ),
    (
        Os::Mac,
        &["Library", "Containers", "com.microsoft.OneDrive-mac"],
        CloudProvider::OneDrive,
    ),
    (
        Os::Mac,
        &["Library", "Application Support", "OneDrive"],
        CloudProvider::OneDrive,
    ),
    (Os::Mac, &[".dropbox"], CloudProvider::Dropbox),
    (
        Os::Mac,
        &["Library", "Application Support", "Google", "DriveFS"],
        CloudProvider::GoogleDrive,
    ),
    (Os::Linux, &[".config", "onedrive"], CloudProvider::OneDrive),
    (Os::Linux, &[".dropbox"], CloudProvider::Dropbox),
    (Os::Linux, &[".config", "rclone"], CloudProvider::Other),
];

/// Whether `under_home` (folders below a person's home folder, then a name) is a sync app's own
/// sign-in state, which is never copied: the person signs in again on the new laptop. Names are
/// compared ignoring case on Windows and macOS, exactly on Linux.
pub fn sign_in_state(os: Os, under_home: &[&str]) -> Option<CloudProvider> {
    let same = |a: &str, b: &str| match os {
        Os::Windows | Os::Mac => a.eq_ignore_ascii_case(b),
        Os::Linux => a == b,
    };
    SIGN_IN_STATE
        .iter()
        .find(|(o, prefix, _)| {
            *o == os
                && under_home.len() >= prefix.len()
                && prefix.iter().zip(under_home).all(|(p, n)| same(p, n))
        })
        .map(|(_, _, provider)| *provider)
}
