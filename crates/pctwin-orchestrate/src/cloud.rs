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
//! in; the person signs in again on the new laptop. It is recognised by what it is, not by a list
//! of apps (third security review, finding 4; Phase 2 adoption, changes 7 and 8: named lists are
//! examples, PCTwin works from what it finds): by its names ([`account_state_by_name`]: anything
//! hidden inside a synced folder, keychains, cookie, login and token databases) and by what it
//! holds ([`account_state_by_content`]: Windows data-protection blobs, keychains, token text). A
//! short catalogue of known places only adds to that. Anything PCTwin cannot tell is not taken:
//! keeping local copies ([`crate::local_copy`]) brings down only files that pass both checks.

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

/// Why something is a sync app's own account state (its sign-in, tokens, settings), which PCTwin
/// never copies: the person signs in again on the new laptop instead.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AccountState {
    /// A place a known app keeps its sign-in state (the catalogue: it only adds places, it is
    /// never the whole rule).
    KnownPlace,
    /// A hidden file or folder a sync app keeps for itself in the folder it syncs (`.dropbox`,
    /// OneDrive's hidden folders), or one the system marks as the app's own (hidden and system).
    AppsOwnFile,
    /// A store of passwords or keys: a Windows data-protection blob, a macOS keychain, a
    /// browser's or app's cookie, login or token database.
    CredentialStore,
    /// Text holding sign-in tokens (`refresh_token`, `access_token`, `id_token`).
    Tokens,
}

/// Places known apps keep their sign-in state, below a person's home folder. Examples that add
/// to the general rules ([`account_state_by_name`], [`account_state_by_content`]); an app not
/// listed here is still recognised by those.
const KNOWN_PLACES: [(Os, &[&str]); 17] = [
    (Os::Windows, &["AppData", "Local", "Microsoft", "OneDrive"]),
    (Os::Windows, &["AppData", "Local", "Dropbox"]),
    (Os::Windows, &["AppData", "Roaming", "Dropbox"]),
    (Os::Windows, &["AppData", "Local", "Google", "DriveFS"]),
    (
        Os::Windows,
        &["AppData", "Roaming", "Apple Computer", "iCloud"],
    ),
    (
        Os::Windows,
        &["AppData", "Local", "Apple Inc", "iCloudDrive"],
    ),
    (
        Os::Windows,
        &["AppData", "Local", "Microsoft", "Credentials"],
    ),
    (
        Os::Windows,
        &["AppData", "Roaming", "Microsoft", "Credentials"],
    ),
    (Os::Windows, &["AppData", "Roaming", "Microsoft", "Protect"]),
    (Os::Windows, &["AppData", "Local", "Microsoft", "Vault"]),
    (Os::Mac, &["Library", "Keychains"]),
    (Os::Mac, &["Library", "Application Support", "CloudDocs"]),
    (
        Os::Mac,
        &[
            "Library",
            "Group Containers",
            "UBF8T346G9.OneDriveSyncClientSuite",
        ],
    ),
    (
        Os::Mac,
        &["Library", "Containers", "com.microsoft.OneDrive-mac"],
    ),
    (
        Os::Mac,
        &["Library", "Application Support", "Google", "DriveFS"],
    ),
    (Os::Linux, &[".config", "onedrive"]),
    (Os::Linux, &[".local", "share", "keyrings"]),
];

/// File names that are credential stores whatever app made them (browsers' and apps' cookie,
/// login and token databases; keychains), compared ignoring case.
const STORE_NAMES: [&str; 8] = [
    "cookies",
    "login data",
    "web data",
    "token.db",
    "tokens.db",
    "refresh_token",
    "credentials.json",
    "msal.cache",
];

/// Whether a path is account state, by its names alone (before anything is read): `under_home`
/// is the path's parts below the person's home folder, when it is there; `under_root` its parts
/// below the synced folder it is in. Any hidden part inside the synced folder, any part that is a
/// known place, a keychain or a credential store's name, counts.
pub fn account_state_by_name(
    os: Os,
    under_home: Option<&[&str]>,
    under_root: &[&str],
) -> Option<AccountState> {
    let same = |a: &str, b: &str| match os {
        Os::Windows | Os::Mac => a.eq_ignore_ascii_case(b),
        Os::Linux => a == b,
    };
    if let Some(parts) = under_home
        && KNOWN_PLACES.iter().any(|(o, place)| {
            *o == os
                && parts.len() >= place.len()
                && place.iter().zip(parts).all(|(p, n)| same(p, n))
        })
    {
        return Some(AccountState::KnownPlace);
    }
    if under_root.iter().any(|n| n.starts_with('.')) {
        return Some(AccountState::AppsOwnFile);
    }
    let last = under_root
        .last()
        .map(|n| n.to_ascii_lowercase())
        .unwrap_or_default();
    if last.ends_with(".keychain") || last.ends_with(".keychain-db") {
        return Some(AccountState::CredentialStore);
    }
    if STORE_NAMES.contains(&last.as_str()) {
        return Some(AccountState::CredentialStore);
    }
    None
}

/// The first bytes of a Windows data-protection (DPAPI) blob: its version and the provider's ID.
const DPAPI: [u8; 20] = [
    0x01, 0x00, 0x00, 0x00, 0xD0, 0x8C, 0x9D, 0xDF, 0x01, 0x15, 0xD1, 0x11, 0x8C, 0x7A, 0x00, 0xC0,
    0x4F, 0xC2, 0x97, 0xEB,
];

/// Whether a file is account state by what it holds: `name` is its file name, `head` its first
/// bytes (a few kilobytes are enough). A data-protection blob, a keychain, a database or text
/// holding sign-in tokens counts, whatever app made it.
pub fn account_state_by_content(name: &str, head: &[u8]) -> Option<AccountState> {
    if head.starts_with(&DPAPI) || head.windows(DPAPI.len() - 4).any(|w| w == &DPAPI[4..]) {
        return Some(AccountState::CredentialStore);
    }
    if head.starts_with(b"kych") {
        return Some(AccountState::CredentialStore);
    }
    let lower_name = name.to_ascii_lowercase();
    if head.starts_with(b"SQLite format 3\0")
        && [
            "cookie",
            "token",
            "login",
            "credential",
            "auth",
            "session",
            "account",
        ]
        .iter()
        .any(|w| lower_name.contains(w))
    {
        return Some(AccountState::CredentialStore);
    }
    let text = String::from_utf8_lossy(head).to_ascii_lowercase();
    if ["refresh_token", "access_token", "id_token"]
        .iter()
        .any(|k| text.contains(k))
    {
        return Some(AccountState::Tokens);
    }
    None
}
