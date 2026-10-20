//! The census (the plan, "Census first"): everyone on the old laptop in five kinds, each by the
//! system's own account ID and never by path: the person moving, another person, an account never
//! used, a leftover from a removed account, and a work or school account.

use std::path::{Path, PathBuf};

use pctwin_scan::{
    Account, Census, CensusFacts, FoundBy, PersonKind, Resident, SystemKind, accounts_from_dscl,
    accounts_from_get_local_user, accounts_from_passwd, census_from, census_this_laptop,
    profile_list_from_reg, service_homes_from_dscl, service_homes_from_passwd, sid_from_whoami,
};

const DOMAIN: &str = "S-1-5-21-111-222-333";

fn sid(rid: u32) -> String {
    format!("{DOMAIN}-{rid}")
}

fn kinds(census: &Census) -> Vec<(Option<&str>, PersonKind)> {
    census
        .residents
        .iter()
        .map(|r| (r.account_id.as_deref(), r.kind))
        .collect()
}

fn named<'a>(census: &'a Census, name: &str) -> &'a Resident {
    census
        .residents
        .iter()
        .find(|r| r.suggested_name == name)
        .unwrap_or_else(|| panic!("nobody named {name}: {census:#?}"))
}

// ---------------------------------------------------------------- Windows

const REG_ENGLISH: &str = "\r
HKEY_LOCAL_MACHINE\\SOFTWARE\\Microsoft\\Windows NT\\CurrentVersion\\ProfileList\r
    Default    REG_EXPAND_SZ    %SystemDrive%\\Users\\Default\r
    ProfilesDirectory    REG_EXPAND_SZ    %SystemDrive%\\Users\r
    ProgramData    REG_EXPAND_SZ    %SystemDrive%\\ProgramData\r
    Public    REG_EXPAND_SZ    %SystemDrive%\\Users\\Public\r
\r
HKEY_LOCAL_MACHINE\\SOFTWARE\\Microsoft\\Windows NT\\CurrentVersion\\ProfileList\\S-1-5-18\r
    Flags    REG_DWORD    0xc\r
    ProfileImagePath    REG_EXPAND_SZ    %systemroot%\\system32\\config\\systemprofile\r
    RefCount    REG_DWORD    0x1\r
    Sid    REG_BINARY    010100000000000512000000\r
    State    REG_DWORD    0x0\r
\r
HKEY_LOCAL_MACHINE\\SOFTWARE\\Microsoft\\Windows NT\\CurrentVersion\\ProfileList\\S-1-5-19\r
    Flags    REG_DWORD    0x0\r
    ProfileImagePath    REG_EXPAND_SZ    %systemroot%\\ServiceProfiles\\LocalService\r
    State    REG_DWORD    0x0\r
\r
HKEY_LOCAL_MACHINE\\SOFTWARE\\Microsoft\\Windows NT\\CurrentVersion\\ProfileList\\S-1-5-21-111-222-333-1001\r
    ProfileImagePath    REG_EXPAND_SZ    C:\\Users\\Ada\r
    Flags    REG_DWORD    0x0\r
    FullProfile    REG_DWORD    0x1\r
\r
HKEY_LOCAL_MACHINE\\SOFTWARE\\Microsoft\\Windows NT\\CurrentVersion\\ProfileList\\S-1-5-21-111-222-333-1002\r
    ProfileImagePath    REG_EXPAND_SZ    %SystemDrive%\\Users\\Tunde\r
\r
HKEY_LOCAL_MACHINE\\SOFTWARE\\Microsoft\\Windows NT\\CurrentVersion\\ProfileList\\S-1-5-21-111-222-333-1003\r
    ProfileImagePath    REG_EXPAND_SZ    C:\\Users\\Old\r
\r
HKEY_LOCAL_MACHINE\\SOFTWARE\\Microsoft\\Windows NT\\CurrentVersion\\ProfileList\\S-1-5-21-111-222-333-1006.bak\r
    ProfileImagePath    REG_EXPAND_SZ    C:\\Users\\Kemi\r
\r
HKEY_LOCAL_MACHINE\\SOFTWARE\\Microsoft\\Windows NT\\CurrentVersion\\ProfileList\\S-1-12-1-444-555-666-777\r
    ProfileImagePath    REG_EXPAND_SZ    C:\\Users\\ChiomaWork\r
\r
HKEY_LOCAL_MACHINE\\SOFTWARE\\Microsoft\\Windows NT\\CurrentVersion\\ProfileList\\S-1-5-21-999-888-777-1105\r
    ProfileImagePath    REG_EXPAND_SZ    C:\\Users\\jdoe.CORP\r
\r
HKEY_LOCAL_MACHINE\\SOFTWARE\\Microsoft\\Windows NT\\CurrentVersion\\ProfileList\\S-1-5-82-3006700770-424185619-1745488364-794895919-4004696415\r
    ProfileImagePath    REG_EXPAND_SZ    C:\\Users\\defaultapppool\r
\r
End of search: 37 match(es) found.\r
";

// `Get-LocalUser`: SID, enabled (1 or 0), name, full name.
const LOCAL_USERS_ENGLISH: &str = "\
S-1-5-21-111-222-333-500\t0\tAdministrator\t\r
S-1-5-21-111-222-333-1001\t1\tAda\tAda Obi\r
S-1-5-21-111-222-333-503\t0\tDefaultAccount\t\r
S-1-5-21-111-222-333-1005\t1\tEmeka\t\r
S-1-5-21-111-222-333-501\t0\tGuest\t\r
S-1-5-21-111-222-333-1002\t1\tTunde\t\r
S-1-5-21-111-222-333-504\t0\tWDAGUtilityAccount\t\r
";

fn english_folders() -> Vec<PathBuf> {
    [
        "Ada",
        "All Users",
        "ChiomaWork",
        "Default",
        "Default User",
        "defaultapppool",
        "defaultuser0",
        "desktop.ini",
        "jdoe.CORP",
        "Kemi",
        "Ngozi",
        "Old",
        "Public",
        "Tunde",
    ]
    .iter()
    .map(|n| PathBuf::from(format!(r"C:\Users\{n}")))
    .collect()
}

fn english_facts(me: &str, accounts: Option<Vec<Account>>) -> CensusFacts {
    let (root, entries) = profile_list_from_reg(REG_ENGLISH, "C:", r"C:\Windows");
    CensusFacts {
        me: me.into(),
        accounts,
        profiles_root: root,
        profile_folders: english_folders(),
        profile_entries: entries,
        folder_owners: Vec::new(),
        other_homes: Vec::new(),
        uid_range: None,
        system: SystemKind::Windows,
    }
}

#[test]
fn the_windows_profile_list_gives_the_profiles_folder_and_every_entry() {
    let (root, entries) = profile_list_from_reg(REG_ENGLISH, "C:", r"C:\Windows");
    assert_eq!(root.as_deref(), Some(Path::new(r"C:\Users")));
    // Every entry, service profiles and broken-profile copies included, with variables filled in.
    assert_eq!(entries.len(), 9, "{entries:#?}");
    assert!(entries.contains(&(
        "S-1-5-18".into(),
        r"C:\Windows\system32\config\systemprofile".into()
    )));
    assert!(entries.contains(&(sid(1002), r"C:\Users\Tunde".into())));
    assert!(entries.contains(&(format!("{}.bak", sid(1006)), r"C:\Users\Kemi".into())));
}

#[test]
fn local_accounts_that_cannot_sign_in_are_kept_and_marked() {
    let accounts = accounts_from_get_local_user(LOCAL_USERS_ENGLISH);
    assert_eq!(accounts.len(), 7);
    let on: Vec<&str> = accounts
        .iter()
        .filter(|a| !a.switched_off)
        .map(|a| a.id.as_str())
        .collect();
    assert_eq!(on, [sid(1001), sid(1005), sid(1002)]);
    let ada = accounts.iter().find(|a| a.name == "Ada").unwrap();
    assert_eq!(ada.full_name.as_deref(), Some("Ada Obi"));
    let emeka = accounts.iter().find(|a| a.name == "Emeka").unwrap();
    assert_eq!(emeka.full_name, None);
    assert!(accounts.iter().all(|a| !a.managed && a.home.is_none()));
}

/// A person's account switched off (a child's account, say) still has that person's files: it is
/// another person, never "left from a removed account" (found by the how-and-why walk, 20 October
/// 2026). Switched-off accounts with no folder (the built-in ones) stay out.
#[test]
fn a_switched_off_account_with_a_folder_is_a_person_not_a_leftover() {
    let users = LOCAL_USERS_ENGLISH.replace("1002\t1\tTunde", "1002\t0\tTunde");
    assert_ne!(users, LOCAL_USERS_ENGLISH);
    let census = census_from(english_facts(
        &sid(1001),
        Some(accounts_from_get_local_user(&users)),
    ));
    let tunde = named(&census, "Tunde");
    assert_eq!(tunde.kind, PersonKind::Other);
    assert_eq!(tunde.account_id.as_deref(), Some(sid(1002).as_str()));
    assert_eq!(tunde.home.as_deref(), Some(Path::new(r"C:\Users\Tunde")));
}

#[test]
fn the_signed_in_account_is_read_from_whoami() {
    let out = "\"desktop-n0p1hj3\\dell\",\"S-1-5-21-751740092-431245858-612657413-1001\"\r\n";
    assert_eq!(
        sid_from_whoami(out).as_deref(),
        Some("S-1-5-21-751740092-431245858-612657413-1001")
    );
    assert_eq!(sid_from_whoami(""), None);
}

#[test]
fn windows_people_come_in_five_kinds() {
    let accounts = accounts_from_get_local_user(LOCAL_USERS_ENGLISH);
    let census = census_from(english_facts(&sid(1001), Some(accounts)));
    assert!(census.account_list_read);
    assert_eq!(
        census.profiles_root.as_deref(),
        Some(Path::new(r"C:\Users"))
    );

    let me = census.me().expect("the signed-in person");
    assert_eq!(me.account_id.as_deref(), Some(sid(1001).as_str()));
    assert_eq!(me.suggested_name, "Ada Obi");
    assert_eq!(census.residents[0].kind, PersonKind::Me);

    assert_eq!(named(&census, "Tunde").kind, PersonKind::Other);
    assert_eq!(
        named(&census, "Tunde").found_by,
        [FoundBy::AccountList, FoundBy::ProfilesFolder]
    );

    // Never signed in: an account, but no folder yet.
    let emeka = named(&census, "Emeka");
    assert_eq!(emeka.kind, PersonKind::NeverUsed);
    assert_eq!(emeka.home, None);
    assert_eq!(emeka.found_by, [FoundBy::AccountList]);

    // A profile of this laptop's own kind whose account was removed.
    let old = named(&census, "Old");
    assert_eq!(old.kind, PersonKind::Leftover);
    assert_eq!(old.account_id.as_deref(), Some(sid(1003).as_str()));
    assert_eq!(old.home.as_deref(), Some(Path::new(r"C:\Users\Old")));

    // Work or school: an Entra ID account, and an account from another domain.
    assert_eq!(named(&census, "ChiomaWork").kind, PersonKind::Managed);
    assert_eq!(named(&census, "jdoe.CORP").kind, PersonKind::Managed);
    assert_eq!(
        named(&census, "jdoe.CORP").account_id.as_deref(),
        Some("S-1-5-21-999-888-777-1105")
    );

    // A folder no profile entry points at: left over, owner unknown.
    let ngozi = named(&census, "Ngozi");
    assert_eq!(ngozi.kind, PersonKind::Leftover);
    assert_eq!(ngozi.account_id, None);
    assert_eq!(ngozi.found_by, [FoundBy::ProfilesFolder]);

    // Not people: system folders, a web server's app-pool profile, a broken-profile copy, and
    // built-in accounts that are switched off.
    for name in [
        "Public",
        "Default",
        "Default User",
        "All Users",
        "defaultuser0",
        "desktop.ini",
        "defaultapppool",
        "Kemi",
        "Administrator",
        "Guest",
        "DefaultAccount",
    ] {
        assert!(
            census.residents.iter().all(|r| r.suggested_name != name),
            "{name} is not a person: {census:#?}"
        );
    }
    assert_eq!(census.residents.len(), 7, "{census:#?}");
    // Me, Tunde, Emeka, ChiomaWork and jdoe have accounts; the two leftovers do not.
    assert_eq!(census.people_count(), 5);
}

#[test]
fn the_mover_is_found_by_account_id_never_by_folder() {
    // Tunde is signed in, whatever folder the test itself runs from.
    let accounts = accounts_from_get_local_user(LOCAL_USERS_ENGLISH);
    let census = census_from(english_facts(&sid(1002), Some(accounts)));
    let me: Vec<&Resident> = census
        .residents
        .iter()
        .filter(|r| r.kind == PersonKind::Me)
        .collect();
    assert_eq!(me.len(), 1);
    assert_eq!(me[0].suggested_name, "Tunde");
    assert_eq!(named(&census, "Ada Obi").kind, PersonKind::Other);
}

#[test]
fn a_signed_in_account_missing_from_every_list_is_still_the_mover() {
    let census = census_from(english_facts("S-1-12-1-9-9-9-9", Some(Vec::new())));
    let me = census.me().expect("me");
    assert_eq!(me.account_id.as_deref(), Some("S-1-12-1-9-9-9-9"));
    assert_eq!(me.home, dirs::home_dir());
    assert_eq!(
        census
            .residents
            .iter()
            .filter(|r| r.kind == PersonKind::Me)
            .count(),
        1
    );
}

#[test]
fn without_the_account_list_nobody_is_called_leftover_or_never_used_from_a_guess() {
    let census = census_from(english_facts(&sid(1001), None));
    assert!(!census.account_list_read);
    assert_eq!(census.me().map(|r| r.kind), Some(PersonKind::Me));
    assert_eq!(named(&census, "Old").kind, PersonKind::Other);
    assert_eq!(named(&census, "Tunde").kind, PersonKind::Other);
    // The SID shape alone still says work or school.
    assert_eq!(named(&census, "ChiomaWork").kind, PersonKind::Managed);
    assert!(
        census
            .residents
            .iter()
            .all(|r| r.kind != PersonKind::NeverUsed)
    );
    assert!(
        census
            .residents
            .iter()
            .filter(|r| r.kind == PersonKind::Leftover)
            .all(|r| r.account_id.is_none()),
        "{census:#?}"
    );
}

#[test]
fn a_german_windows_reads_the_same_with_another_profiles_folder() {
    // Account names, the end-of-search line and folder names are in German; nothing depends on
    // them. The profiles folder was moved to D:.
    let reg = "\r
HKEY_LOCAL_MACHINE\\SOFTWARE\\Microsoft\\Windows NT\\CurrentVersion\\ProfileList\r
    Default    REG_EXPAND_SZ    D:\\Profile\\Default\r
    ProfilesDirectory    REG_EXPAND_SZ    D:\\Profile\r
    Public    REG_EXPAND_SZ    D:\\Profile\\Public\r
\r
HKEY_LOCAL_MACHINE\\SOFTWARE\\Microsoft\\Windows NT\\CurrentVersion\\ProfileList\\S-1-5-18\r
    ProfileImagePath    REG_EXPAND_SZ    %systemroot%\\system32\\config\\systemprofile\r
\r
HKEY_LOCAL_MACHINE\\SOFTWARE\\Microsoft\\Windows NT\\CurrentVersion\\ProfileList\\S-1-5-21-5-6-7-1001\r
    ProfileImagePath    REG_EXPAND_SZ    D:\\Profile\\Jürgen\r
\r
Suche beendet: 4 Übereinstimmung(en) gefunden.\r
";
    let users = "\
S-1-5-21-5-6-7-500\t0\tAdministrator\t\r
S-1-5-21-5-6-7-501\t0\tGast\t\r
S-1-5-21-5-6-7-1001\t1\tJürgen\tJürgen Weiß\r
S-1-5-21-5-6-7-1002\t1\tAnna\t\r
";
    let (root, entries) = profile_list_from_reg(reg, "C:", r"C:\Windows");
    assert_eq!(root.as_deref(), Some(Path::new(r"D:\Profile")));
    let census = census_from(CensusFacts {
        me: "S-1-5-21-5-6-7-1001".into(),
        accounts: Some(accounts_from_get_local_user(users)),
        profiles_root: root,
        profile_folders: ["Default", "Jürgen", "Öffentlich", "Public"]
            .iter()
            .map(|n| PathBuf::from(format!(r"D:\Profile\{n}")))
            .collect(),
        profile_entries: entries,
        folder_owners: Vec::new(),
        other_homes: Vec::new(),
        uid_range: None,
        system: SystemKind::Windows,
    });
    let me = census.me().expect("me");
    assert_eq!(me.suggested_name, "Jürgen Weiß");
    assert_eq!(me.home.as_deref(), Some(Path::new(r"D:\Profile\Jürgen")));
    assert_eq!(named(&census, "Anna").kind, PersonKind::NeverUsed);
    // A folder no entry points at, whatever its language.
    assert_eq!(named(&census, "Öffentlich").kind, PersonKind::Leftover);
    assert_eq!(
        kinds(&census)
            .iter()
            .filter(|(_, k)| *k == PersonKind::Me)
            .count(),
        1
    );
}

#[test]
fn profile_entries_and_folders_are_matched_ignoring_case() {
    let mut facts = english_facts(
        &sid(1001),
        Some(accounts_from_get_local_user(LOCAL_USERS_ENGLISH)),
    );
    facts.profile_folders = facts
        .profile_folders
        .iter()
        .map(|p| PathBuf::from(p.to_string_lossy().to_uppercase()))
        .collect();
    let census = census_from(facts);
    // Only NGOZI is unreferenced; Tunde's folder is still his.
    let leftovers: Vec<&str> = census
        .residents
        .iter()
        .filter(|r| r.kind == PersonKind::Leftover && r.account_id.is_none())
        .map(|r| r.suggested_name.as_str())
        .collect();
    assert_eq!(leftovers, ["NGOZI"]);
    assert_eq!(named(&census, "Tunde").kind, PersonKind::Other);
}

#[test]
fn windows_person_ids_are_local_or_work_accounts() {
    let census = census_from(english_facts(&sid(1001), Some(Vec::new())));
    for id in [
        sid(1001),
        "S-1-12-1-1-2-3-4".into(),
        "s-1-5-21-1-2-3-1009".into(),
    ] {
        assert!(census.is_person_id(&id), "{id}");
    }
    for id in [
        "S-1-5-18",
        "S-1-5-32-544",
        "S-1-5-80-956008885-3418522649-1831038044-1853292631-2271478464",
        "S-1-5-82-3006700770-424185619-1745488364-794895919-4004696415",
        "S-1-3-0",
    ] {
        assert!(!census.is_person_id(id), "{id}");
    }
}

#[test]
fn owners_seen_during_the_walk_are_added_once() {
    let mut census = census_from(english_facts(
        &sid(1001),
        Some(accounts_from_get_local_user(LOCAL_USERS_ENGLISH)),
    ));
    let before = census.residents.len();
    census.note_owner_seen(&sid(1002));
    census.note_owner_seen(&sid(1002));
    assert_eq!(
        named(&census, "Tunde").found_by,
        [
            FoundBy::AccountList,
            FoundBy::ProfilesFolder,
            FoundBy::OwnerSeen
        ]
    );
    // SYSTEM, Administrators and TrustedInstaller carry no person.
    census.note_owner_seen("S-1-5-18");
    census.note_owner_seen("S-1-5-32-544");
    census.note_owner_seen("S-1-5-80-956008885-3418522649-1831038044-1853292631-2271478464");
    assert_eq!(census.residents.len(), before);
    // A person nobody has heard of: a leftover with no folder.
    census.note_owner_seen(&sid(1010));
    census.note_owner_seen(&sid(1010));
    assert_eq!(census.residents.len(), before + 1);
    let new = census.by_account(&sid(1010)).expect("added");
    assert_eq!(new.kind, PersonKind::Leftover);
    assert_eq!(new.home, None);
    assert_eq!(new.found_by, [FoundBy::OwnerSeen]);
    assert_eq!(census.residents[0].kind, PersonKind::Me);
}

// ---------------------------------------------------------------- macOS

const DSCL: &str = "\
AuthenticationAuthority: ;ShadowHash;HASHLIST:<SALTED-SHA512-PBKDF2,SRP-RFC5054-4096-SHA512-PBKDF2> ;Kerberosv5;;ada@LKDC:SHA1.4C2E;LKDC:SHA1.4C2E; ;SecureToken;
NFSHomeDirectory: /Users/ada
RealName:
 Ada Obi
RecordName: ada
UniqueID: 501
-
NFSHomeDirectory: /var/empty
RecordName: _www
UniqueID: 70
-
NFSHomeDirectory: /Users/Guest
RealName: Guest User
RecordName: Guest
UniqueID: 201
-
AuthenticationAuthority:
 ;LocalCachedUser;/Active Directory/CORP/corp.example.com:jdoe:9A1B2C3D
 ;ShadowHash;HASHLIST:<SALTED-SHA512-PBKDF2>
NFSHomeDirectory: /Users/jdoe
OriginalNodeName:
 /Active Directory/CORP/corp.example.com
RealName: John Doe
RecordName: jdoe
UniqueID: 1234567890
-
NFSHomeDirectory: /var/empty
RecordName: nobody
UniqueID: -2
-
NFSHomeDirectory: /Users/tunde
RealName: Tunde
RecordName: tunde
UniqueID: 502
-
NFSHomeDirectory: /Users/sam
RecordName: sam
UniqueID: 503
-
NFSHomeDirectory: /var/setup
RecordName: _mbsetupuser
UniqueID: 248
";

fn mac_facts(accounts: Option<Vec<Account>>) -> CensusFacts {
    CensusFacts {
        me: "501".into(),
        accounts,
        profiles_root: Some("/Users".into()),
        profile_folders: [
            ".localized",
            "Deleted Users",
            "Guest",
            "Shared",
            "ada",
            "jdoe",
            "oldkid",
            "tunde",
        ]
        .iter()
        .map(|n| Path::new("/Users").join(n))
        .collect(),
        profile_entries: Vec::new(),
        folder_owners: vec![
            ("/Users/Deleted Users".into(), "0".into()),
            ("/Users/Shared".into(), "0".into()),
            ("/Users/oldkid".into(), "504".into()),
            ("/Users/ada".into(), "501".into()),
        ],
        other_homes: service_homes_from_dscl(DSCL),
        uid_range: None,
        system: SystemKind::Mac,
    }
}

#[test]
fn mac_accounts_include_mobile_work_accounts() {
    let accounts = accounts_from_dscl(DSCL);
    let ids: Vec<&str> = accounts.iter().map(|a| a.id.as_str()).collect();
    assert_eq!(ids, ["501", "1234567890", "502", "503"]);
    assert_eq!(accounts[0].full_name.as_deref(), Some("Ada Obi"));
    assert_eq!(accounts[0].home.as_deref(), Some(Path::new("/Users/ada")));
    assert!(!accounts[0].managed && accounts[1].managed && !accounts[2].managed);
}

#[test]
fn mac_people_come_in_five_kinds() {
    let census = census_from(mac_facts(Some(accounts_from_dscl(DSCL))));
    assert_eq!(
        census.me().map(|r| r.suggested_name.as_str()),
        Some("Ada Obi")
    );
    assert_eq!(named(&census, "Tunde").kind, PersonKind::Other);
    assert_eq!(named(&census, "John Doe").kind, PersonKind::Managed);
    assert_eq!(named(&census, "sam").kind, PersonKind::NeverUsed);
    assert_eq!(named(&census, "sam").home, None);

    let deleted = named(&census, "Deleted Users");
    assert_eq!(deleted.kind, PersonKind::Leftover);
    assert_eq!(deleted.account_id, None, "owned by root, so no person");
    let oldkid = named(&census, "oldkid");
    assert_eq!(oldkid.kind, PersonKind::Leftover);
    assert_eq!(oldkid.account_id.as_deref(), Some("504"));

    for name in ["Shared", "Guest", ".localized", "Guest User"] {
        assert!(
            census.residents.iter().all(|r| r.suggested_name != name),
            "{name}"
        );
    }
    assert_eq!(census.residents.len(), 6, "{census:#?}");
    assert_eq!(census.people_count(), 4);
    assert!(census.is_person_id("501") && census.is_person_id("1234567890"));
    assert!(!census.is_person_id("0") && !census.is_person_id("4294967294"));
}

// ---------------------------------------------------------------- Linux

const GETENT: &str = "\
root:x:0:0:root:/root:/bin/bash
daemon:x:1:1:daemon:/usr/sbin:/usr/sbin/nologin
ada:x:1000:1000:Ada Obi,,,:/home/ada:/bin/bash
tunde:x:1001:1001::/home/tunde:/bin/zsh
svc-backup:x:1002:1002::/home/svc-backup:/usr/sbin/nologin
newbie:x:1003:1003::/home/newbie:/bin/bash
nobody:x:65534:65534:nobody:/nonexistent:/usr/sbin/nologin
jdoe:*:1500001:1500001:John Doe:/home/jdoe:/bin/bash
ada:x:1000:1000:Ada Obi,,,:/home/ada:/bin/bash
";

fn local_passwd() -> String {
    GETENT
        .lines()
        .filter(|l| !l.starts_with("jdoe:"))
        .map(|l| format!("{l}\n"))
        .collect()
}

fn linux_facts(accounts: Option<Vec<Account>>) -> CensusFacts {
    CensusFacts {
        me: "1000".into(),
        accounts,
        profiles_root: Some("/home".into()),
        profile_folders: [
            "ada",
            "jdoe",
            "lost+found",
            "oldie",
            "shared-stuff",
            "svc-backup",
            "tunde",
        ]
        .iter()
        .map(|n| Path::new("/home").join(n))
        .collect(),
        profile_entries: Vec::new(),
        folder_owners: vec![
            ("/home/oldie".into(), "1004".into()),
            ("/home/shared-stuff".into(), "0".into()),
            ("/home/svc-backup".into(), "1002".into()),
        ],
        other_homes: service_homes_from_passwd(GETENT, (1000, 2_000_000)),
        uid_range: Some((1000, 2_000_000)),
        system: SystemKind::Linux,
    }
}

#[test]
fn linux_accounts_from_a_network_directory_are_work_accounts() {
    let accounts = accounts_from_passwd(GETENT, &local_passwd(), (1000, 2_000_000));
    let ids: Vec<&str> = accounts.iter().map(|a| a.id.as_str()).collect();
    // Services that cannot sign in are left out; a listed-twice account counts once.
    assert_eq!(ids, ["1000", "1001", "1003", "1500001"]);
    assert_eq!(accounts[0].full_name.as_deref(), Some("Ada Obi"));
    assert_eq!(accounts[1].full_name, None);
    assert!(accounts[3].managed && !accounts[0].managed);
    // Directory accounts often have IDs above the local range; they are still people.
    let narrow = accounts_from_passwd(GETENT, &local_passwd(), (1000, 60000));
    assert_eq!(narrow.len(), 4);
}

#[test]
fn linux_people_come_in_five_kinds() {
    let range = (1000, 2_000_000);
    let census = census_from(linux_facts(Some(accounts_from_passwd(
        GETENT,
        &local_passwd(),
        range,
    ))));
    assert_eq!(
        census.me().map(|r| r.suggested_name.as_str()),
        Some("Ada Obi")
    );
    assert_eq!(named(&census, "tunde").kind, PersonKind::Other);
    assert_eq!(named(&census, "John Doe").kind, PersonKind::Managed);
    assert_eq!(named(&census, "newbie").kind, PersonKind::NeverUsed);
    assert_eq!(named(&census, "oldie").kind, PersonKind::Leftover);
    assert_eq!(named(&census, "oldie").account_id.as_deref(), Some("1004"));
    assert_eq!(named(&census, "shared-stuff").account_id, None);
    // A service's home and lost+found are not leftovers.
    for name in ["svc-backup", "lost+found"] {
        assert!(
            census.residents.iter().all(|r| r.suggested_name != name),
            "{name}"
        );
    }
    assert_eq!(census.residents.len(), 6, "{census:#?}");
    assert!(census.is_person_id("1500001") && !census.is_person_id("999"));
}

#[test]
fn the_linux_user_range_defaults_from_the_system_kind() {
    let mut facts = linux_facts(Some(Vec::new()));
    facts.uid_range = None;
    let census = census_from(facts);
    assert!(census.is_person_id("1000") && census.is_person_id("60000"));
    assert!(!census.is_person_id("65534") && !census.is_person_id("0"));
}

#[cfg(unix)]
#[test]
fn homes_behind_a_link_are_matched_to_their_real_folder() {
    // Fedora Silverblue and Kinoite: /home is a link to /var/home.
    let dir = tempfile::tempdir().unwrap();
    std::fs::create_dir_all(dir.path().join("var/home/ada")).unwrap();
    std::os::unix::fs::symlink(dir.path().join("var/home"), dir.path().join("home")).unwrap();
    let root = std::fs::canonicalize(dir.path().join("home")).unwrap();
    let census = census_from(CensusFacts {
        me: "1000".into(),
        accounts: Some(vec![Account {
            id: "1000".into(),
            name: "ada".into(),
            full_name: None,
            home: Some(dir.path().join("home/ada")),
            managed: false,
        }]),
        profiles_root: Some(root.clone()),
        profile_folders: vec![root.join("ada")],
        profile_entries: Vec::new(),
        folder_owners: Vec::new(),
        other_homes: Vec::new(),
        uid_range: None,
        system: SystemKind::Linux,
    });
    assert_eq!(census.residents.len(), 1, "{census:#?}");
    assert_eq!(
        census.residents[0].found_by,
        [FoundBy::AccountList, FoundBy::ProfilesFolder]
    );
}

// ---------------------------------------------------------------- the census itself

#[test]
fn the_deepest_home_owns_a_path() {
    let accounts = vec![
        Account {
            id: "1000".into(),
            name: "ada".into(),
            full_name: None,
            home: Some("/data".into()),
            managed: false,
            switched_off: false,
        },
        Account {
            id: "1001".into(),
            name: "kid".into(),
            full_name: None,
            home: Some("/data/kid".into()),
            managed: false,
            switched_off: false,
        },
    ];
    let census = census_from(CensusFacts {
        me: "1000".into(),
        accounts: Some(accounts),
        profiles_root: Some("/home".into()),
        profile_folders: vec!["/data".into(), "/data/kid".into()],
        profile_entries: Vec::new(),
        folder_owners: Vec::new(),
        other_homes: Vec::new(),
        uid_range: None,
        system: SystemKind::Linux,
    });
    let owner = |p: &str| {
        census
            .home_owner(Path::new(p))
            .and_then(|r| r.account_id.clone())
    };
    assert_eq!(owner("/data/kid/notes.txt").as_deref(), Some("1001"));
    assert_eq!(owner("/data/kid").as_deref(), Some("1001"));
    assert_eq!(owner("/data/kiddo/x").as_deref(), Some("1000"));
    assert_eq!(owner("/elsewhere"), None);
}

#[test]
fn the_old_people_list_is_everyone_with_an_account_and_a_folder() {
    let census = census_from(english_facts(
        &sid(1001),
        Some(accounts_from_get_local_user(LOCAL_USERS_ENGLISH)),
    ));
    let people = census.to_people();
    let ids: Vec<&str> = people.iter().map(|p| p.account_id.as_str()).collect();
    // Never-used Emeka has no folder; the Ngozi folder has no account.
    assert_eq!(people.len(), 5, "{ids:?}");
    assert!(!ids.contains(&sid(1005).as_str()));
    assert_eq!(people.iter().filter(|p| p.is_me).count(), 1);
    assert!(people[0].is_me && people[0].account_id == sid(1001));
}

#[test]
fn the_census_order_is_the_same_every_time() {
    let facts = || {
        english_facts(
            &sid(1002),
            Some(accounts_from_get_local_user(LOCAL_USERS_ENGLISH)),
        )
    };
    let a = census_from(facts());
    let mut shuffled = facts();
    shuffled.profile_folders.reverse();
    shuffled.profile_entries.reverse();
    let b = census_from(shuffled);
    assert_eq!(a, b);
    assert_eq!(a.residents[0].kind, PersonKind::Me);
}

#[test]
fn the_census_is_saved_and_read_back_in_plain_words() {
    let census = census_from(mac_facts(Some(accounts_from_dscl(DSCL))));
    let json = serde_json::to_string(&census).unwrap();
    assert!(
        json.contains("\"never-used\"") && json.contains("\"profiles-folder\""),
        "{json}"
    );
    let back: Census = serde_json::from_str(&json).unwrap();
    assert_eq!(back, census);
}

#[test]
fn this_laptop_has_exactly_one_mover_found_by_account_id() {
    let census = census_this_laptop();
    let me: Vec<&Resident> = census
        .residents
        .iter()
        .filter(|r| r.kind == PersonKind::Me)
        .collect();
    assert_eq!(me.len(), 1, "{census:#?}");
    assert_eq!(me[0].account_id.as_deref(), Some(census.me.as_str()));
    assert!(!census.me.is_empty());
    for r in &census.residents {
        if let Some(home) = &r.home {
            assert!(home.is_absolute(), "{r:?}");
        }
    }
    let root = census.profiles_root.as_ref().expect("profiles folder");
    assert!(root.is_dir(), "{root:?}");
}
