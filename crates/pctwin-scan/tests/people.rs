//! Who is on the old laptop (Task List 1.4, "Who's moving?"): each person by the system's own
//! account ID, read from the list every system keeps without opening anyone's files. A name is only
//! a suggestion; system and service accounts are left out.

use std::path::Path;

use pctwin_scan::{
    Person, list_people, people_from_dscl, people_from_passwd, people_from_profile_list,
    uid_range_from_login_defs,
};

fn ids(people: &[Person]) -> Vec<&str> {
    people.iter().map(|p| p.account_id.as_str()).collect()
}

#[test]
fn windows_profiles_are_read_from_the_profile_list() {
    let reg = "\r
HKEY_LOCAL_MACHINE\\SOFTWARE\\Microsoft\\Windows NT\\CurrentVersion\\ProfileList\\S-1-5-18\r
    ProfileImagePath    REG_EXPAND_SZ    %systemroot%\\system32\\config\\systemprofile\r
\r
HKEY_LOCAL_MACHINE\\SOFTWARE\\Microsoft\\Windows NT\\CurrentVersion\\ProfileList\\S-1-5-19\r
    ProfileImagePath    REG_EXPAND_SZ    %systemroot%\\ServiceProfiles\\LocalService\r
\r
HKEY_LOCAL_MACHINE\\SOFTWARE\\Microsoft\\Windows NT\\CurrentVersion\\ProfileList\\S-1-5-21-111-222-333-1001\r
    ProfileImagePath    REG_EXPAND_SZ    C:\\Users\\Ada\r
\r
HKEY_LOCAL_MACHINE\\SOFTWARE\\Microsoft\\Windows NT\\CurrentVersion\\ProfileList\\S-1-5-21-111-222-333-1002\r
    ProfileImagePath    REG_EXPAND_SZ    C:\\Users\\Tunde-PC\r
\r
HKEY_LOCAL_MACHINE\\SOFTWARE\\Microsoft\\Windows NT\\CurrentVersion\\ProfileList\\S-1-5-21-111-222-333-1003.bak\r
    ProfileImagePath    REG_EXPAND_SZ    C:\\Users\\Old\r
\r
HKEY_LOCAL_MACHINE\\SOFTWARE\\Microsoft\\Windows NT\\CurrentVersion\\ProfileList\\S-1-12-1-444-555-666-777\r
    ProfileImagePath    REG_EXPAND_SZ    C:\\Users\\ChiomaWork\r
\r
End of search: 6 match(es) found.\r
";
    let people = people_from_profile_list(reg, Path::new(r"C:\Users\Ada"));
    // System and service profiles and Windows' backup copies of broken profiles (.bak) are left\n    // out; work and school (Entra ID) accounts are kept.
    assert_eq!(
        ids(&people),
        [
            "S-1-5-21-111-222-333-1001",
            "S-1-5-21-111-222-333-1002",
            "S-1-12-1-444-555-666-777"
        ]
    );
    assert_eq!(people[1].suggested_name, "Tunde-PC");
    assert_eq!(people[1].home, Path::new(r"C:\Users\Tunde-PC"));
    assert!(people[0].is_me && !people[1].is_me && !people[2].is_me);
}

#[test]
fn mac_accounts_are_read_from_the_directory_service() {
    let dscl = "\
NFSHomeDirectory: /var/root
RecordName: root
UniqueID: 0
-
NFSHomeDirectory: /var/empty
RecordName: _www
UniqueID: 70
-
NFSHomeDirectory: /Users/ada
RealName:
 Ada Obi
RecordName: ada
UniqueID: 501
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
NFSHomeDirectory: /Users/_mbsetupuser
RecordName: _mbsetupuser
UniqueID: 503
";
    let people = people_from_dscl(dscl, Path::new("/Users/tunde"));
    assert_eq!(ids(&people), ["501", "502"]);
    assert_eq!(people[0].suggested_name, "Ada Obi");
    assert_eq!(people[0].home, Path::new("/Users/ada"));
    assert!(!people[0].is_me && people[1].is_me);
}

#[test]
fn linux_accounts_are_read_from_passwd_in_the_normal_user_range() {
    let passwd = "\
root:x:0:0:root:/root:/bin/bash
daemon:x:1:1:daemon:/usr/sbin:/usr/sbin/nologin
ada:x:1000:1000:Ada Obi,,,:/home/ada:/bin/bash
backup-bot:x:1001:1001::/home/backup-bot:/usr/sbin/nologin
tunde:x:1002:1002::/home/tunde:/bin/zsh
builder:x:65000:65000::/home/builder:/bin/bash
ghost:x:1003:1003::/home/ghost:/bin/false
nobody:x:65534:65534:nobody:/nonexistent:/usr/sbin/nologin
";
    let people = people_from_passwd(passwd, (1000, 60000), Path::new("/home/ada"));
    assert_eq!(ids(&people), ["1000", "1002"]);
    assert_eq!(people[0].suggested_name, "Ada Obi");
    assert_eq!(people[1].suggested_name, "tunde");
    assert!(people[0].is_me);
}

#[test]
fn the_normal_user_range_follows_login_defs() {
    let defs = "# comment\nUID_MIN\t\t\t 500\nUID_MAX   29999\nGID_MIN 500\n";
    assert_eq!(uid_range_from_login_defs(Some(defs)), (500, 29999));
    assert_eq!(uid_range_from_login_defs(None), (1000, 60000));
    assert_eq!(
        uid_range_from_login_defs(Some("UID_MIN nonsense\n")),
        (1000, 60000)
    );
}

#[test]
fn this_laptop_lists_the_signed_in_person_exactly_once() {
    let people = list_people();
    assert_eq!(people.iter().filter(|p| p.is_me).count(), 1, "{people:#?}");
    for p in &people {
        assert!(!p.account_id.is_empty());
        assert!(p.home.is_absolute(), "{p:?}");
    }
}
