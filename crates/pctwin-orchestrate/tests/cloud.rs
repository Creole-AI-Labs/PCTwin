//! Cloud files, honest by default (Task List 2.6; acceptance T26-014, T26-015, T26-016).
// Tests make files to test on.
#![allow(clippy::disallowed_methods)]

mod common;

use common::*;
use pctwin_orchestrate::cloud::{
    CloudOutcome, Os, SyncFact, Whose, capture_scope, cloud_outcome, provider_name, sync_fact,
};
use pctwin_record::{CloudProvider, Inclusion, LeftOutReason, ManagedBy, Storage};
use pctwin_types::{ItemResult, ItemStatus, Reason};

/// T26-014: a file stored online only is never copied and never shown as copied: it is safe in
/// its cloud and needs one step, signing in on the new laptop.
#[test]
fn an_online_only_file_is_safe_in_its_cloud_and_needs_a_sign_in() {
    let item = online_only(person("S-1"), CloudProvider::OneDrive, &["trip.mp4"], 900);
    let out = cloud_outcome(&item, SyncFact::NotKnown);
    assert_eq!(
        out,
        CloudOutcome::SafeInCloud {
            provider: Some(CloudProvider::OneDrive)
        }
    );
    assert!(!out.copies());
    let result = out.planned_result().unwrap();
    assert_eq!(result, ItemResult::because(Reason::OnlineOnly));
    assert_eq!(result.status(), ItemStatus::NeedsAction);
    assert_eq!(
        out.words(&Whose::Mover).unwrap(),
        "Safe in OneDrive, not on this laptop. It comes back when you sign in to OneDrive on the new laptop."
    );
    assert_eq!(
        out.words(&Whose::Other { name: "Tunde" }).unwrap(),
        "Only in Tunde's OneDrive: Tunde signs in on the new laptop to get them."
    );
    // Whatever the sync fact says, an online-only file stays online only.
    for sync in [SyncFact::Uploaded, SyncFact::WaitingToUpload] {
        assert_eq!(cloud_outcome(&item, sync), out);
    }
}

/// An online-only file whose cloud PCTwin could not name still says where it is, without
/// guessing a name.
#[test]
fn an_online_only_file_in_an_unnamed_cloud_says_so_plainly() {
    let mut item = online_only(person("S-1"), CloudProvider::Other, &["a.txt"], 1);
    assert_eq!(
        cloud_outcome(&item, SyncFact::NotKnown),
        CloudOutcome::SafeInCloud { provider: None }
    );
    // A placeholder found outside a known cloud folder is still online only.
    item.place.storage = Storage::SystemDrive;
    let out = cloud_outcome(&item, SyncFact::NotKnown);
    assert_eq!(out, CloudOutcome::SafeInCloud { provider: None });
    assert_eq!(
        out.words(&Whose::Mover).unwrap(),
        "Safe in your cloud storage, not on this laptop. It comes back when you sign in to it on the new laptop."
    );
    assert_eq!(
        out.words(&Whose::Other { name: "Bea" }).unwrap(),
        "Only in Bea's cloud storage: Bea signs in on the new laptop to get them."
    );
}

/// T26-015: a file whose contents are on the laptop is copied like any other. Unless PCTwin
/// can tell it is uploaded, it counts as only on this laptop.
#[test]
fn a_file_on_the_laptop_is_copied_and_counts_as_only_here_unless_known_uploaded() {
    let item = cloud_file(person("S-1"), CloudProvider::Dropbox, &["draft.docx"], 10);
    for (sync, only_here) in [
        (SyncFact::WaitingToUpload, true),
        (SyncFact::NotKnown, true),
        (SyncFact::Uploaded, false),
    ] {
        let out = cloud_outcome(&item, sync);
        assert_eq!(
            out,
            CloudOutcome::CopyHere {
                provider: Some(CloudProvider::Dropbox),
                only_on_this_laptop: only_here
            }
        );
        assert!(out.copies());
        // The copy's own result comes from the move, never from the plan.
        assert_eq!(out.planned_result(), None);
    }
    assert_eq!(
        cloud_outcome(&item, SyncFact::WaitingToUpload)
            .words(&Whose::Mover)
            .unwrap(),
        "Only on this laptop: Dropbox may not have finished uploading it, so PCTwin copies it."
    );
    assert_eq!(
        cloud_outcome(&item, SyncFact::Uploaded).words(&Whose::Mover),
        None
    );
}

/// Outside any cloud folder, nothing here applies.
#[test]
fn a_file_outside_a_cloud_folder_is_not_a_cloud_file() {
    let item = file(person("S-1"), &["a.txt"], 1);
    for sync in [
        SyncFact::Uploaded,
        SyncFact::WaitingToUpload,
        SyncFact::NotKnown,
    ] {
        let out = cloud_outcome(&item, sync);
        assert_eq!(out, CloudOutcome::NotInCloud { included: true });
        assert!(out.copies());
        assert_eq!(out.words(&Whose::Mover), None);
        assert_eq!(out.planned_result(), None);
    }
    // A file the person chose to leave out is not a cloud file either.
    let mut left = cloud_file(person("S-1"), CloudProvider::OneDrive, &["b.txt"], 1);
    left.inclusion = Inclusion::LeftOut {
        reason: LeftOutReason::ChosenByUser,
    };
    assert_eq!(
        cloud_outcome(&left, SyncFact::NotKnown),
        CloudOutcome::NotInCloud { included: false }
    );
    assert!(!cloud_outcome(&left, SyncFact::NotKnown).copies());
}

/// Work and school folders keep the rule: left out, "managed by your employer", whether the
/// file is on the laptop or online only.
#[test]
fn a_work_or_school_file_stays_with_the_employer() {
    let mut local = cloud_file(person("S-1"), CloudProvider::OneDrive, &["plan.xlsx"], 5);
    local.managed_by = ManagedBy::Organisation;
    local.inclusion = Inclusion::LeftOut {
        reason: LeftOutReason::WorkManaged,
    };
    let mut online = online_only(person("S-1"), CloudProvider::OneDrive, &["b.xlsx"], 5);
    online.managed_by = ManagedBy::Organisation;
    // Left out as work-managed by the scan, even where the item itself was not marked.
    let mut marked_only_by_reason = local.clone();
    marked_only_by_reason.managed_by = ManagedBy::Personal;
    for item in [&local, &online, &marked_only_by_reason] {
        for sync in [SyncFact::Uploaded, SyncFact::NotKnown] {
            let out = cloud_outcome(item, sync);
            assert_eq!(out, CloudOutcome::ManagedByEmployer);
            assert!(!out.copies());
            assert_eq!(
                out.planned_result(),
                Some(ItemResult::because(Reason::SignIn))
            );
            assert_eq!(
                out.words(&Whose::Mover).unwrap(),
                "This is managed by your employer or school, so it stays with them. Sign in to your work or school account on the new laptop to get it."
            );
        }
    }
}

/// T26-016 and the third review's finding 4: only a sync app's allowed place (the person's own
/// synced files) is ever in scope; its sign-in state, settings and the system's credential stores
/// never are, whatever they are called.
#[test]
fn only_the_persons_own_synced_files_are_in_scope() {
    let never: [(Os, &[&str]); 20] = [
        (
            Os::Windows,
            &[
                "AppData",
                "Local",
                "Microsoft",
                "OneDrive",
                "settings",
                "Personal",
                "a.dat",
            ],
        ),
        (
            Os::Windows,
            &["appdata", "local", "microsoft", "onedrive", "x"],
        ),
        (Os::Windows, &["AppData", "Local", "Dropbox", "info.json"]),
        (Os::Windows, &["AppData", "Roaming", "Dropbox", "host.db"]),
        (
            Os::Windows,
            &["AppData", "Local", "Google", "DriveFS", "a", "b"],
        ),
        (
            Os::Windows,
            &["AppData", "Roaming", "Apple Computer", "iCloud", "x"],
        ),
        // What a list of things never to take missed (third review, finding 4):
        (
            Os::Windows,
            &["APPDAT~1", "Local", "Microsoft", "OneDrive", "x"],
        ),
        (Os::Windows, &["ONEDRI~1", "x"]),
        (Os::Windows, &["OneDrive.", "x"]),
        (Os::Windows, &["OneDrive ", "x"]),
        (
            Os::Windows,
            &[
                "AppData",
                "Local",
                "Packages",
                "AppleInc.iCloud_nzyj5cx40ttqa",
                "LocalState",
                "x",
            ],
        ),
        (
            Os::Windows,
            &["AppData", "Local", "Microsoft", "Credentials", "x"],
        ),
        (
            Os::Windows,
            &["AppData", "Roaming", "Microsoft", "Credentials", "x"],
        ),
        (Os::Mac, &["Library", "Keychains", "login.keychain-db"]),
        (
            Os::Mac,
            &[
                "Library",
                "Application Support",
                "CloudDocs",
                "session",
                "db",
            ],
        ),
        (
            Os::Mac,
            &[
                "Library",
                "Group Containers",
                "UBF8T346G9.OneDriveSyncClientSuite",
                "x",
            ],
        ),
        (Os::Mac, &[".dropbox", "info.json"]),
        (
            Os::Mac,
            &["Library", "CloudStorage", "GoogleDrive-", "My Drive", "x"],
        ),
        (Os::Linux, &[".config", "onedrive", "refresh_token"]),
        (Os::Linux, &["onedrive", "x"]),
    ];
    for (os, parts) in never {
        assert_eq!(capture_scope(os, parts), None, "{os:?} {parts:?}");
    }
    let mine: [(Os, &[&str], CloudProvider); 13] = [
        (
            Os::Windows,
            &["OneDrive", "Documents", "a.txt"],
            CloudProvider::OneDrive,
        ),
        (Os::Windows, &["onedrive", "a.txt"], CloudProvider::OneDrive),
        (
            Os::Windows,
            &["OneDrive - Personal", "a.txt"],
            CloudProvider::OneDrive,
        ),
        (Os::Windows, &["Dropbox", "x"], CloudProvider::Dropbox),
        (Os::Windows, &["My Drive", "x"], CloudProvider::GoogleDrive),
        (Os::Windows, &["iCloudDrive", "x"], CloudProvider::ICloud),
        (
            Os::Windows,
            &["Pictures", "iCloud Photos", "x"],
            CloudProvider::ICloud,
        ),
        (
            Os::Mac,
            &[
                "Library",
                "Mobile Documents",
                "com~apple~CloudDocs",
                "a.txt",
            ],
            CloudProvider::ICloud,
        ),
        (
            Os::Mac,
            &["Library", "CloudStorage", "OneDrive-Personal", "a.txt"],
            CloudProvider::OneDrive,
        ),
        (
            Os::Mac,
            &["Library", "CloudStorage", "Dropbox", "a.txt"],
            CloudProvider::Dropbox,
        ),
        (
            Os::Mac,
            &[
                "Library",
                "CloudStorage",
                "GoogleDrive-ada@example.com",
                "My Drive",
                "x",
            ],
            CloudProvider::GoogleDrive,
        ),
        (Os::Linux, &["OneDrive", "x"], CloudProvider::OneDrive),
        (Os::Linux, &["Dropbox", "x"], CloudProvider::Dropbox),
    ];
    for (os, parts, provider) in mine {
        assert_eq!(capture_scope(os, parts), Some(provider), "{os:?} {parts:?}");
    }
    // The allowed folder itself is a place, not a thing to take; another system's places are not
    // this one's; an account folder must name its account.
    assert_eq!(capture_scope(Os::Windows, &["OneDrive"]), None);
    assert_eq!(capture_scope(Os::Windows, &[]), None);
    assert_eq!(capture_scope(Os::Linux, &["iCloudDrive", "x"]), None);
    assert_eq!(
        capture_scope(
            Os::Mac,
            &["Library", "CloudStorage", "GoogleDrive-a.", "My Drive", "x"]
        ),
        None
    );
    assert_eq!(
        capture_scope(
            Os::Mac,
            &["Library", "CloudStorage", "googledrive-a", "My Drive", "x"]
        ),
        Some(CloudProvider::GoogleDrive)
    );
}

#[test]
fn providers_have_their_own_names() {
    assert_eq!(provider_name(CloudProvider::OneDrive), Some("OneDrive"));
    assert_eq!(provider_name(CloudProvider::ICloud), Some("iCloud Drive"));
    assert_eq!(provider_name(CloudProvider::Dropbox), Some("Dropbox"));
    assert_eq!(
        provider_name(CloudProvider::GoogleDrive),
        Some("Google Drive")
    );
    assert_eq!(provider_name(CloudProvider::Other), None);
}

/// PCTwin cannot yet read whether a file on the laptop is uploaded without new unsafe code, so
/// a real file always reads as "not known" (and is therefore copied and kept on the list of what
/// is only on the old laptop).
#[test]
fn a_real_files_upload_state_reads_as_not_known() {
    let dir = tempfile::tempdir().unwrap();
    let p = dir.path().join("a.txt");
    std::fs::write(&p, b"x").unwrap();
    assert_eq!(
        sync_fact(&std::fs::metadata(&p).unwrap()),
        SyncFact::NotKnown
    );
}
