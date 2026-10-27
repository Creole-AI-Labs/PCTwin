//! Cloud folders are found first; the catalogue only names them (Phase 2 adoption, changes 7
//! and 8).

use std::collections::HashMap;
use std::path::{Path, PathBuf};

use pctwin_catalogue::{Catalogue, test_key};
use pctwin_record::CloudProvider;
use pctwin_scan::clouds::{
    How, ProviderRow, RouteReader, Seen, System, cloud_storage_name, find_clouds, fuse_mounts_in,
    name_seen, provider_name, provider_name_in, shipped, sync_roots_from_reg,
};

const SID: &str = "S-1-5-21-1-2-3-1001";

fn reg_output() -> String {
    let base =
        r"HKEY_LOCAL_MACHINE\SOFTWARE\Microsoft\Windows\CurrentVersion\Explorer\SyncRootManager";
    [
        format!("{base}\\OneDrive!{SID}!Personal"),
        "    DisplayNameResource    REG_SZ    OneDrive - Personal".into(),
        String::new(),
        format!("{base}\\OneDrive!{SID}!Personal|0ABC12\\UserSyncRoots"),
        format!("    {SID}    REG_SZ    C:\\Users\\Ada\\OneDrive"),
        String::new(),
        format!("{base}\\OneDrive!{SID}!Business1|tenant\\UserSyncRoots"),
        format!("    {SID}    REG_EXPAND_SZ    C:\\Users\\Ada\\OneDrive - Contoso"),
        String::new(),
        format!("{base}\\Box!{SID}!ada@example.com\\UserSyncRoots"),
        format!("    {SID}    REG_SZ    C:\\Users\\Ada\\Box"),
        String::new(),
        format!("{base}\\AcmeVault!{SID}!x\\UserSyncRoots"),
        format!("    {SID}    REG_SZ    C:\\Users\\Ada\\Acme Vault"),
        String::new(),
        // Another person's sync root, and a value under a key of another SID.
        format!("{base}\\OneDrive!S-1-5-21-9!Personal\\UserSyncRoots"),
        "    S-1-5-21-9    REG_SZ    C:\\Users\\Bo\\OneDrive".into(),
        format!("{base}\\Dropbox!S-1-5-21-9!x\\UserSyncRoots"),
        format!("    {SID}    REG_SZ    C:\\Users\\Ada\\Forged"),
        // A longer SID that starts with this one is another person.
        format!("{base}\\Mega!{SID}0!x\\UserSyncRoots"),
        format!("    {SID}0    REG_SZ    C:\\Users\\Other\\MEGA"),
        // Malformed keys and empty values.
        format!("{base}\\NoBangs\\UserSyncRoots"),
        format!("    {SID}    REG_SZ    C:\\x"),
        format!("{base}\\!{SID}!x\\UserSyncRoots"),
        format!("    {SID}    REG_SZ    C:\\y"),
        format!("{base}\\Empty!{SID}!x\\UserSyncRoots"),
        format!("    {SID}    REG_SZ    "),
        format!("    {SID}    REG_DWORD    0x1"),
        "End of search: 9 match(es) found.".into(),
    ]
    .join("\r\n")
}

#[test]
fn windows_sync_roots_are_found_for_this_person_only() {
    let seen = sync_roots_from_reg(&reg_output(), SID);
    let got: Vec<(&str, &str, &Path)> = seen
        .iter()
        .map(|s| (s.provider.as_str(), s.account.as_str(), s.root.as_path()))
        .collect();
    assert_eq!(
        got,
        [
            (
                "OneDrive",
                "Personal|0ABC12",
                Path::new(r"C:\Users\Ada\OneDrive")
            ),
            (
                "OneDrive",
                "Business1|tenant",
                Path::new(r"C:\Users\Ada\OneDrive - Contoso")
            ),
            ("Box", "ada@example.com", Path::new(r"C:\Users\Ada\Box")),
            ("AcmeVault", "x", Path::new(r"C:\Users\Ada\Acme Vault")),
        ]
    );
    assert!(seen.iter().all(|s| s.how == How::SyncRoot));
    assert!(sync_roots_from_reg("ERROR: not found", SID).is_empty());
}

#[test]
fn the_shipped_catalogue_reads_whole_and_names_the_usual_services() {
    let rows = shipped();
    assert!(rows.report().is_clean(), "{:?}", rows.report());
    assert_eq!(provider_name(CloudProvider::OneDrive), Some("OneDrive"));
    assert_eq!(provider_name(CloudProvider::ICloud), Some("iCloud Drive"));
    assert_eq!(provider_name(CloudProvider::Dropbox), Some("Dropbox"));
    assert_eq!(
        provider_name(CloudProvider::GoogleDrive),
        Some("Google Drive")
    );
    assert_eq!(
        provider_name(CloudProvider::new("box").unwrap()),
        Some("Box")
    );
    assert_eq!(provider_name(CloudProvider::Other), None);
    assert_eq!(
        provider_name(CloudProvider::new("acmevault").unwrap()),
        None
    );
}

/// What was found and named: provider ID, work or not, named or not.
fn summary(found: &[pctwin_scan::clouds::FoundCloud]) -> Vec<(String, bool, bool)> {
    found
        .iter()
        .map(|f| (f.provider.as_str().to_string(), f.work, f.named))
        .collect()
}

#[test]
fn every_sync_root_is_found_and_one_with_no_row_is_found_not_yet_named() {
    let seen = sync_roots_from_reg(&reg_output(), SID);
    let found = name_seen(shipped(), seen);
    assert_eq!(
        summary(&found),
        [
            ("one-drive".into(), false, true),
            ("one-drive".into(), true, true),
            ("box".into(), false, true),
            // No row: still found, under an ID from its own name, the person's own.
            ("acmevault".into(), false, false),
        ]
    );
    assert_eq!(found[3].found_name, "AcmeVault");
}

/// The new-row proof: a service added only as a signed row is named, with its work rule, with no
/// code change.
#[test]
fn a_new_signed_row_names_a_found_service_with_no_code_change() {
    let bytes = br#"{"format":1,"catalogue":"cloud-providers","version":2,"rows":[
        {"id":"acme-vault","name":"Acme Vault","found-as":["AcmeVault"],"work-accounts":["corp-"]},
        {"id":"other","name":"Nope","found-as":["x"]},
        {"id":"Bad Id","name":"Nope"},
        {"id":"y","name":"","found-as":["y"]},
        {"id":"z","name":"Z","found-as":["--"]},
        {"id":"q","name":"Q","routes":[{"registry":{"system":"windows","key":"HKLM\\SOFTWARE\\X","value":"Path"}}]},
        {"id":"r","name":"R","routes":[{"folder":{"system":"linux","base":"home","path":[".."]}}]},
        {"id":"s","name":"S","routes":[{"env":{"system":"windows","name":"A B"}}]},
        {"id":"t","name":"T","routes":[{"teleport":{"system":"windows"}}]}
    ]}"#;
    let sig = test_key::sign(bytes, "cloud-providers", 2);
    let rows = Catalogue::<ProviderRow>::signed(bytes, &sig, &test_key::keys(), 1).unwrap();
    assert_eq!(rows.rows().len(), 1);
    let skipped: Vec<usize> = rows.report().skipped.iter().map(|s| s.row).collect();
    assert_eq!(skipped, [1, 2, 3, 4, 5, 6, 7, 8]);
    let seen = vec![
        Seen {
            provider: "AcmeVault".into(),
            account: "Corp-Contoso".into(),
            root: PathBuf::from("/a"),
            how: How::SyncRoot,
        },
        Seen {
            provider: "acme vault".into(),
            account: "me".into(),
            root: PathBuf::from("/b"),
            how: How::CloudStorageFolder,
        },
    ];
    let found = name_seen(&rows, seen);
    assert_eq!(
        summary(&found),
        [
            ("acme-vault".into(), true, true),
            ("acme-vault".into(), false, true)
        ]
    );
    assert_eq!(
        provider_name_in(&rows, CloudProvider::new("acme-vault").unwrap()),
        Some("Acme Vault")
    );
}

#[test]
fn mac_cloud_storage_folders_are_split_into_service_and_account() {
    for (name, provider, account) in [
        ("OneDrive-Personal", "OneDrive", "Personal"),
        (
            "OneDrive-SharedLibraries-Contoso",
            "OneDrive",
            "SharedLibraries-Contoso",
        ),
        (
            "GoogleDrive-ada@example.com",
            "GoogleDrive",
            "ada@example.com",
        ),
        ("Dropbox (Contoso)", "Dropbox", "(Contoso)"),
        ("Dropbox", "Dropbox", ""),
        ("Box-Box", "Box", "Box"),
        ("SomethingNew", "SomethingNew", ""),
    ] {
        assert_eq!(
            cloud_storage_name(name),
            (provider.to_string(), account.to_string()),
            "{name}"
        );
    }
    let seen = ["Box-Box", "SomethingNew-ada"]
        .iter()
        .map(|n| {
            let (provider, account) = cloud_storage_name(n);
            Seen {
                provider,
                account,
                root: PathBuf::from(n),
                how: How::CloudStorageFolder,
            }
        })
        .collect();
    assert_eq!(
        summary(&name_seen(shipped(), seen)),
        [
            ("box".into(), false, true),
            ("somethingnew".into(), false, false)
        ]
    );
}

#[test]
fn linux_fuse_mounts_inside_the_home_are_found() {
    let mounts = "/dev/sda1 / ext4 rw 0 0\n\
remote: /home/ada/My\\040Cloud fuse.rclone rw,nosuid 0 0\n\
onedriver /home/ada/OneDrive fuse.onedriver rw 0 0\n\
gvfsd-fuse /run/user/1000/gvfs fuse.gvfsd-fuse rw 0 0\n\
x /home/ada fuse.sshfs rw 0 0\n\
tmpfs /home/ada/tmp tmpfs rw 0 0\n\
short\n";
    let seen = fuse_mounts_in(mounts, Path::new("/home/ada"));
    let got: Vec<(&str, &Path)> = seen
        .iter()
        .map(|s| (s.provider.as_str(), s.root.as_path()))
        .collect();
    assert_eq!(
        got,
        [
            ("rclone", Path::new("/home/ada/My Cloud")),
            ("onedriver", Path::new("/home/ada/OneDrive")),
        ]
    );
    assert!(seen.iter().all(|s| s.how == How::Mount));
}

/// Routes a test gives.
#[derive(Default)]
struct Fake {
    env: HashMap<String, PathBuf>,
    reg: Vec<(String, PathBuf)>,
    json: HashMap<PathBuf, Vec<(String, PathBuf)>>,
}

impl RouteReader for Fake {
    fn env(&self, name: &str) -> Option<PathBuf> {
        self.env.get(name).cloned()
    }
    fn registry(&self, key: &str, value: &str) -> Vec<(String, PathBuf)> {
        if key == r"HKCU\Software\Microsoft\OneDrive\Accounts" && value == "UserFolder" {
            self.reg.clone()
        } else {
            Vec::new()
        }
    }
    fn json_accounts(&self, file: &Path) -> Vec<(String, PathBuf)> {
        self.json.get(file).cloned().unwrap_or_default()
    }
    fn local_app_data(&self) -> Option<PathBuf> {
        Some(PathBuf::from("/la"))
    }
    fn app_data(&self) -> Option<PathBuf> {
        None
    }
}

#[test]
fn routes_add_folders_a_service_records_and_each_folder_is_kept_once() {
    let mut fake = Fake::default();
    fake.env
        .insert("OneDrive".into(), PathBuf::from("/h/OneDrive"));
    fake.env
        .insert("OneDriveCommercial".into(), PathBuf::from("/h/Work"));
    fake.reg = vec![
        ("Personal".into(), PathBuf::from("/h/OneDrive")),
        ("Business1".into(), PathBuf::from("/h/OneDrive - Contoso")),
    ];
    fake.json.insert(
        PathBuf::from("/la/Dropbox/info.json"),
        vec![
            ("personal".into(), PathBuf::from("/h/Dropbox")),
            ("business".into(), PathBuf::from("/h/Dropbox (Contoso)")),
        ],
    );
    let seen = vec![Seen {
        provider: "OneDrive".into(),
        account: "Business1".into(),
        root: PathBuf::from("/h/Work"),
        how: How::SyncRoot,
    }];
    let exists = |p: &Path| p != Path::new("/h/iCloudDrive");
    let found = find_clouds(
        shipped(),
        System::Windows,
        Path::new("/h"),
        seen,
        &fake,
        &exists,
    );
    let got: Vec<(&str, &Path, bool, How)> = found
        .iter()
        .map(|f| (f.provider.as_str(), f.root.as_path(), f.work, f.how))
        .collect();
    assert_eq!(
        got,
        [
            ("one-drive", Path::new("/h/Work"), true, How::SyncRoot),
            ("one-drive", Path::new("/h/OneDrive"), false, How::Route),
            (
                "one-drive",
                Path::new("/h/OneDrive - Contoso"),
                true,
                How::Route
            ),
            ("dropbox", Path::new("/h/Dropbox"), false, How::Route),
            (
                "dropbox",
                Path::new("/h/Dropbox (Contoso)"),
                true,
                How::Route
            ),
        ]
    );
    // The same routes on a Mac lead only to the Mac's own folders.
    let mac = find_clouds(
        shipped(),
        System::Mac,
        Path::new("/h"),
        Vec::new(),
        &fake,
        &|_: &Path| true,
    );
    let roots: Vec<&Path> = mac.iter().map(|f| f.root.as_path()).collect();
    assert_eq!(
        roots,
        [
            Path::new("/h/Library/Mobile Documents/com~apple~CloudDocs"),
            Path::new("/h/Dropbox"),
        ]
    );
}

/// A folder one way calls personal and another calls work is a work folder.
#[test]
fn a_folder_any_way_calls_work_is_work() {
    let fake = Fake {
        reg: vec![("Business1".into(), PathBuf::from("/h/OneDrive"))],
        ..Fake::default()
    };
    let seen = vec![Seen {
        provider: "OneDrive".into(),
        account: "Personal".into(),
        root: PathBuf::from("/h/OneDrive"),
        how: How::SyncRoot,
    }];
    let found = find_clouds(
        shipped(),
        System::Windows,
        Path::new("/h"),
        seen,
        &fake,
        &|_: &Path| true,
    );
    assert_eq!(found.len(), 2, "{found:?}");
    assert!(found[0].work);
    assert_eq!(found[0].how, How::SyncRoot);
}

/// Reads this laptop's cloud folders and prints counts only (never names or paths). Run by hand:
/// `cargo test -p pctwin-scan --test clouds -- --ignored --nocapture`.
#[test]
#[ignore = "reads the real laptop"]
fn count_this_laptops_clouds() {
    let facts = pctwin_scan::facts_from_system();
    let found = &facts.found_clouds;
    let named = found.iter().filter(|f| f.named).count();
    let work = found.iter().filter(|f| f.work).count();
    let by_sync_root = found.iter().filter(|f| f.how == How::SyncRoot).count();
    let by_route = found.iter().filter(|f| f.how == How::Route).count();
    println!(
        "clouds found {}, named {named}, not yet named {}, work {work}, by sync root {by_sync_root}, by route {by_route}",
        found.len(),
        found.len() - named
    );
}

#[test]
fn work_accounts_follow_the_row() {
    let one = shipped()
        .rows()
        .iter()
        .find(|r| r.id == CloudProvider::OneDrive)
        .unwrap();
    for personal in ["", "Personal", "personal", "Personal|0ABC", " Personal "] {
        assert!(!one.is_work(personal), "{personal:?}");
    }
    for work in [
        "Business1",
        "Business1|t",
        "Contoso",
        "PersonalStuff",
        "SharedLibraries-Contoso",
    ] {
        assert!(one.is_work(work), "{work:?}");
    }
    let rows = Catalogue::<ProviderRow>::shipped(
        r#"{"format":1,"catalogue":"cloud-providers","version":1,"rows":[
            {"id":"a","name":"A","work-accounts":["corp",""]},
            {"id":"b","name":"B"}
        ]}"#,
    );
    let (a, b) = (&rows.rows()[0], &rows.rows()[1]);
    assert!(a.is_work("Corp-1"));
    assert!(!a.is_work("me"));
    assert!(!a.is_work(""));
    assert!(!b.is_work("Corp-1"));
}
