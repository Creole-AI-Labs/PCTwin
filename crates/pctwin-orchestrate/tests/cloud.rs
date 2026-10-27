//! Cloud files, honest by default (Task List 2.6; acceptance T26-014, T26-015, T26-016).
// Tests make files to test on.
#![allow(clippy::disallowed_methods)]

mod common;

use common::*;
use pctwin_orchestrate::cloud::{
    CloudOutcome, Os, SyncFact, Whose, account_state_by_content, account_state_by_name,
    cloud_outcome, provider_name, sync_fact,
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

/// T26-016 and the third review's finding 4: a sync app's account state is recognised by what it
/// is, for any app, known or not; the catalogue of known places only adds to that.
#[test]
fn account_state_is_recognised_by_what_it_is() {
    use pctwin_orchestrate::cloud::AccountState::*;
    // Known places, below the home folder, whatever the case on Windows and macOS.
    for (os, parts) in [
        (
            Os::Windows,
            &[
                "AppData",
                "Local",
                "Microsoft",
                "OneDrive",
                "settings",
                "a.dat",
            ][..],
        ),
        (
            Os::Windows,
            &["appdata", "local", "microsoft", "onedrive", "x"][..],
        ),
        (
            Os::Windows,
            &["AppData", "Local", "Microsoft", "Credentials", "x"][..],
        ),
        (
            Os::Windows,
            &[
                "AppData",
                "Roaming",
                "Microsoft",
                "Protect",
                "S-1-5-21",
                "x",
            ][..],
        ),
        (Os::Mac, &["Library", "Keychains", "login.keychain-db"][..]),
        (Os::Linux, &[".config", "onedrive", "refresh_token"][..]),
    ] {
        assert_eq!(
            account_state_by_name(os, Some(parts), &["x"]),
            Some(KnownPlace),
            "{parts:?}"
        );
    }
    // Any app's own hidden files inside the folder it syncs, known app or not.
    for root in [
        &[".dropbox", "info.json"][..],
        &[".849C9593-D756", "x"][..],
        &[".stfolder"][..],
        &["a", ".sync", "x"][..],
    ] {
        assert_eq!(
            account_state_by_name(Os::Windows, None, root),
            Some(AppsOwnFile),
            "{root:?}"
        );
    }
    // Credential stores by their kind of name, wherever they are.
    for last in [
        "Cookies",
        "login.keychain-db",
        "work.keychain",
        "Login Data",
        "token.db",
        "msal.cache",
    ] {
        assert_eq!(
            account_state_by_name(Os::Mac, None, &["a", last]),
            Some(CredentialStore),
            "{last}"
        );
    }
    // The person's own files are not.
    for root in [
        &["Documents", "thesis.docx"][..],
        &["tokens of love.txt"][..],
        &["Login notes.txt"][..],
    ] {
        assert_eq!(
            account_state_by_name(Os::Windows, Some(&["OneDrive"]), root),
            None,
            "{root:?}"
        );
    }
    // Another system's places are not this one's.
    assert_eq!(
        account_state_by_name(Os::Windows, Some(&["Library", "Keychains", "x"]), &["x"]),
        None
    );
    // Linux names are exact: another case is another folder.
    assert_eq!(
        account_state_by_name(Os::Linux, Some(&[".Config", "onedrive", "x"]), &["x"]),
        None
    );

    // By what they hold.
    let mut dpapi = vec![
        1u8, 0, 0, 0, 0xD0, 0x8C, 0x9D, 0xDF, 0x01, 0x15, 0xD1, 0x11, 0x8C, 0x7A, 0x00, 0xC0, 0x4F,
        0xC2, 0x97, 0xEB,
    ];
    dpapi.extend_from_slice(&[7; 40]);
    assert_eq!(
        account_state_by_content("settings.dat", &dpapi),
        Some(CredentialStore)
    );
    let mut inside = b"header".to_vec();
    inside.extend_from_slice(&dpapi[4..]);
    assert_eq!(
        account_state_by_content("x.bin", &inside),
        Some(CredentialStore)
    );
    assert_eq!(
        account_state_by_content("any", b"kych\0\x01\0\0"),
        Some(CredentialStore)
    );
    assert_eq!(
        account_state_by_content("session.db", b"SQLite format 3\0rest"),
        Some(CredentialStore)
    );
    assert_eq!(
        account_state_by_content("Recipes.db", b"SQLite format 3\0rest"),
        None
    );
    assert_eq!(
        account_state_by_content("a.json", br#"{"Refresh_Token":"x"}"#),
        Some(Tokens)
    );
    assert_eq!(
        account_state_by_content("a.ini", b"access_token=abc"),
        Some(Tokens)
    );
    assert_eq!(
        account_state_by_content("a.ini", b"id_token=abc"),
        Some(Tokens)
    );
    assert_eq!(
        account_state_by_content("thesis.docx", b"PK\x03\x04 chapter one"),
        None
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

/// Phase 2 adoption, changes 7 and 8: the known sign-in places are a catalogue that loads whole,
/// and a place added only as a signed row is known with no code change.
#[test]
fn sign_in_places_are_catalogue_rows() {
    use pctwin_catalogue::{Catalogue, test_key};
    use pctwin_orchestrate::cloud::{
        AccountState, SignInPlace, account_state_by_name_with, shipped_sign_in_places,
    };
    let shipped = shipped_sign_in_places();
    assert!(shipped.report().is_clean(), "{:?}", shipped.report());
    assert!(shipped.rows().len() >= 17);

    // An app nobody listed: its settings folder is not known by name, and nothing in its name
    // marks it, so the general rules (what the file holds) decide when it is read.
    let acme = ["AppData", "Local", "Acme Sync", "state.bin"];
    assert_eq!(
        account_state_by_name_with(shipped, Os::Windows, Some(&acme), &["state.bin"]),
        None
    );
    // Its hidden folder inside what it syncs is still its own, with no row.
    assert_eq!(
        account_state_by_name_with(shipped, Os::Windows, None, &[".acmesync", "db"]),
        Some(AccountState::AppsOwnFile)
    );

    // One signed row later, the same place is known, on that system only.
    let bytes = br#"{"format":1,"catalogue":"sign-in-places","version":2,"rows":[
        {"system":"windows","place":["AppData","Local","Acme Sync"],"app":"Acme Sync"},
        {"system":"windows","place":["AppData","..","x"]},
        {"system":"beos","place":["x"]}
    ]}"#;
    let sig = test_key::sign(bytes, "sign-in-places", 2);
    let delivered =
        Catalogue::<SignInPlace>::signed(bytes, &sig, &test_key::keys(), shipped.version())
            .unwrap();
    assert_eq!(delivered.rows().len(), 1);
    assert_eq!(delivered.report().skipped.len(), 2);
    assert_eq!(
        account_state_by_name_with(&delivered, Os::Windows, Some(&acme), &["state.bin"]),
        Some(AccountState::KnownPlace)
    );
    assert_eq!(
        account_state_by_name_with(&delivered, Os::Mac, Some(&acme), &["state.bin"]),
        None
    );
    // A shorter path than the row is not inside it.
    assert_eq!(
        account_state_by_name_with(&delivered, Os::Windows, Some(&acme[..2]), &["x"]),
        None
    );
}

#[test]
fn a_sign_in_place_row_is_checked() {
    use pctwin_catalogue::Catalogue;
    use pctwin_orchestrate::cloud::SignInPlace;
    let long: Vec<String> = (0..17).map(|i| format!("\"f{i}\"")).collect();
    let text = format!(
        r#"{{"format":1,"catalogue":"sign-in-places","version":1,"rows":[
        {{"system":"linux","place":[]}},
        {{"system":"linux","place":[{}]}},
        {{"system":"linux","place":[""]}},
        {{"system":"linux","place":["."]}},
        {{"system":"linux","place":["a/b"]}},
        {{"system":"linux","place":["a\\b"]}},
        {{"system":"linux","place":["a\u0000"]}},
        {{"system":"linux","place":["{}"]}},
        {{"system":"linux","place":["ok"]}}
    ]}}"#,
        long.join(","),
        "x".repeat(256)
    );
    let c = Catalogue::<SignInPlace>::shipped(&text);
    let skipped: Vec<usize> = c.report().skipped.iter().map(|s| s.row).collect();
    assert_eq!(skipped, [0, 1, 2, 3, 4, 5, 6, 7]);
    assert_eq!(c.rows().len(), 1);
    let sixteen: Vec<String> = (0..16).map(|i| format!("\"f{i}\"")).collect();
    let ok = format!(
        r#"{{"format":1,"catalogue":"sign-in-places","version":1,"rows":[{{"system":"linux","place":[{}]}},{{"system":"linux","place":["{}"]}}]}}"#,
        sixteen.join(","),
        "x".repeat(255)
    );
    assert_eq!(Catalogue::<SignInPlace>::shipped(&ok).rows().len(), 2);
}
