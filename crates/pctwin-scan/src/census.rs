//! The census: everyone on the old laptop, before anything is walked. Each person is known by the
//! system's own account ID (a SID on Windows, the numeric user ID on Mac and Linux), never by a
//! folder path, and falls in one of five kinds: the person moving, another person, an account
//! never used, a leftover from a removed account, and a work or school account.
//!
//! [`census_from`] holds every rule and reads nothing but the facts it is given, so each rule is
//! tested with text captured from real systems. [`census_this_laptop`] reads those facts from the
//! running system without administrator rights and without opening anyone's files.

use std::io::Read;
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};
use std::time::{Duration, Instant};

use serde::{Deserialize, Serialize};

use crate::folders::{is_within, real, same_place};
use crate::people::Person;

/// Which kind of person a resident is.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum PersonKind {
    /// The person signed in and running PCTwin.
    Me,
    /// Another person with an account and a home folder.
    Other,
    /// An account nobody has signed in to yet: there is no home folder.
    NeverUsed,
    /// Files left behind by an account that was removed.
    Leftover,
    /// A work or school account (Entra ID, a company domain, a network directory).
    Managed,
}

/// Where the census learned about a resident.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum FoundBy {
    /// The system's list of accounts.
    AccountList,
    /// The folder that holds everyone's home folders, or the system's list of them.
    ProfilesFolder,
    /// The owner of something met while looking through the laptop.
    OwnerSeen,
}

/// Which system the facts come from. The rules differ a little between them.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum SystemKind {
    Windows,
    Mac,
    Linux,
}

impl SystemKind {
    /// The system this program is running on.
    pub fn this() -> Self {
        if cfg!(windows) {
            SystemKind::Windows
        } else if cfg!(target_os = "macos") {
            SystemKind::Mac
        } else {
            SystemKind::Linux
        }
    }

    /// The normal range of user IDs for people when the system does not say otherwise.
    fn default_uid_range(self) -> (u32, u32) {
        match self {
            // Work accounts on a Mac get large IDs; nobody is 4294967294.
            SystemKind::Mac => (501, i32::MAX as u32),
            _ => (1000, 60000),
        }
    }
}

/// One person in the census.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Resident {
    /// SID on Windows, numeric uid on Mac/Linux. None only for a leftover folder whose account
    /// record is gone and whose owner could not be read.
    pub account_id: Option<String>,
    pub kind: PersonKind,
    /// A name to suggest. Only ever a suggestion: people are matched by `account_id`.
    pub suggested_name: String,
    pub home: Option<PathBuf>,
    /// Sorted, no duplicates.
    pub found_by: Vec<FoundBy>,
}

/// Everyone on the old laptop.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Census {
    /// The signed-in account ID.
    pub me: String,
    /// The folder that holds everyone's home folders, as the system reports it.
    pub profiles_root: Option<PathBuf>,
    /// The mover first, then by home folder, then by account ID.
    pub residents: Vec<Resident>,
    /// False if the system's account list could not be read. Then nobody is called a leftover or
    /// never used on a guess.
    pub account_list_read: bool,
    pub system: SystemKind,
    /// The user IDs people have on Mac and Linux (smallest, largest). Not used on Windows.
    pub uid_range: (u32, u32),
}

/// One account from the system's list.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Account {
    pub id: String,
    pub name: String,
    pub full_name: Option<String>,
    /// Where the account record says the home folder is (Mac and Linux; Windows keeps it in the
    /// profile list instead).
    pub home: Option<PathBuf>,
    /// A work or school account.
    pub managed: bool,
    /// Switched off (Windows): it cannot sign in now, but its person's files may still be here.
    #[serde(default)]
    pub switched_off: bool,
}

/// What the census is built from, read from the system or written by a test.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct CensusFacts {
    pub me: String,
    /// The accounts that can sign in. None when the list could not be read.
    pub accounts: Option<Vec<Account>>,
    pub profiles_root: Option<PathBuf>,
    /// The folders inside `profiles_root`, as listed.
    pub profile_folders: Vec<PathBuf>,
    /// Windows' profile list: every SID and its folder, service accounts included. Empty
    /// elsewhere.
    pub profile_entries: Vec<(String, PathBuf)>,
    /// The owner ID of a profile folder where it could be read. May be empty.
    pub folder_owners: Vec<(PathBuf, String)>,
    /// Home folders of system and service accounts, which are not people, so their folders are
    /// never taken for leftovers.
    pub other_homes: Vec<PathBuf>,
    /// Mac and Linux: the user IDs people have. None means the system's usual range.
    pub uid_range: Option<(u32, u32)>,
    pub system: SystemKind,
}

impl Census {
    /// The person moving.
    pub fn me(&self) -> Option<&Resident> {
        self.residents.iter().find(|r| r.kind == PersonKind::Me)
    }

    /// Accounts that exist: the mover, other people, accounts never used, and work or school
    /// accounts. Leftovers have no account.
    pub fn people_count(&self) -> usize {
        self.residents
            .iter()
            .filter(|r| r.kind != PersonKind::Leftover)
            .count()
    }

    pub fn by_account(&self, id: &str) -> Option<&Resident> {
        self.residents
            .iter()
            .find(|r| r.account_id.as_deref().is_some_and(|a| same_id(a, id)))
    }

    /// Whose home folder holds `path` (or is it). When homes sit inside each other, the deepest
    /// one wins.
    pub fn home_owner(&self, path: &Path) -> Option<&Resident> {
        self.residents
            .iter()
            .filter_map(|r| r.home.as_deref().map(|h| (r, h)))
            .filter(|(_, home)| is_within(path, home) || same_place(path, home))
            .max_by_key(|(_, home)| home.components().count())
            .map(|(r, _)| r)
    }

    /// Whether an ID looks like a real person's on this system: a local, domain or work account
    /// SID on Windows, a user ID in the normal user range on Mac and Linux. SYSTEM,
    /// Administrators, TrustedInstaller, root and service accounts are not people.
    pub fn is_person_id(&self, id: &str) -> bool {
        match self.system {
            SystemKind::Windows => is_person_sid(id),
            _ => id
                .parse::<u32>()
                .is_ok_and(|uid| uid >= self.uid_range.0 && uid <= self.uid_range.1),
        }
    }

    /// Records the owner of something met while looking through the laptop. A known person is
    /// marked as seen; a person nobody has heard of is added as a leftover with no folder; an ID
    /// that is not a person's is ignored.
    pub fn note_owner_seen(&mut self, id: &str) {
        let known = self
            .residents
            .iter_mut()
            .find(|r| r.account_id.as_deref().is_some_and(|a| same_id(a, id)));
        if let Some(resident) = known {
            if !resident.found_by.contains(&FoundBy::OwnerSeen) {
                resident.found_by.push(FoundBy::OwnerSeen);
                resident.found_by.sort();
            }
            return;
        }
        if !self.is_person_id(id) {
            return;
        }
        self.residents.push(Resident {
            account_id: Some(id.to_string()),
            kind: PersonKind::Leftover,
            suggested_name: "From an old account".into(),
            home: None,
            found_by: vec![FoundBy::OwnerSeen],
        });
        sort_residents(&mut self.residents, self.system);
    }

    /// The people list the rest of the scan uses: everyone with an account ID and a home folder.
    pub fn to_people(&self) -> Vec<Person> {
        self.residents
            .iter()
            .filter_map(|r| {
                Some(Person {
                    account_id: r.account_id.clone()?,
                    suggested_name: r.suggested_name.clone(),
                    home: r.home.clone()?,
                    is_me: r.kind == PersonKind::Me,
                })
            })
            .collect()
    }
}

/// Builds the census from facts. Reads nothing else, except where a folder exists outside the
/// profiles folder and, when the signed-in account is in no list at all, this person's home
/// folder.
pub fn census_from(facts: CensusFacts) -> Census {
    let system = facts.system;
    let uid_range = facts.uid_range.unwrap_or(system.default_uid_range());
    let account_list_read = facts.accounts.is_some();
    let accounts = facts.accounts.clone().unwrap_or_default();
    let places = Places::new(system, &facts);

    let mut residents = match system {
        SystemKind::Windows => windows_residents(&facts, &accounts, &places),
        _ => unix_residents(&accounts, account_list_read, &places),
    };
    let mut census_so_far = Census {
        me: facts.me.clone(),
        profiles_root: facts.profiles_root.clone(),
        residents: Vec::new(),
        account_list_read,
        system,
        uid_range,
    };
    residents.extend(leftover_folders(&facts, &accounts, &places, &census_so_far));

    // The mover is the account signed in, never a folder that looks like theirs.
    let me = residents.iter_mut().find(|r| {
        r.account_id
            .as_deref()
            .is_some_and(|a| same_id(a, &facts.me))
    });
    match me {
        Some(me) => {
            me.kind = PersonKind::Me;
            if me.home.is_none() {
                me.home = dirs::home_dir();
            }
        }
        None => {
            let home = dirs::home_dir();
            residents.push(Resident {
                account_id: Some(facts.me.clone()),
                kind: PersonKind::Me,
                suggested_name: home
                    .as_deref()
                    .and_then(Path::file_name)
                    .map(|n| n.to_string_lossy().into_owned())
                    .unwrap_or_else(|| facts.me.clone()),
                home,
                found_by: Vec::new(),
            });
        }
    }
    for r in &mut residents {
        r.found_by.sort();
        r.found_by.dedup();
    }
    sort_residents(&mut residents, system);
    census_so_far.residents = residents;
    census_so_far
}

/// Folder comparisons by the rules of the system the facts come from (Windows paths are compared
/// by Windows rules wherever the census is built).
struct Places {
    system: SystemKind,
    folders: Vec<PathBuf>,
    other_homes: Vec<PathBuf>,
}

impl Places {
    fn new(system: SystemKind, facts: &CensusFacts) -> Self {
        let resolve = |p: &PathBuf| resolve(system, p);
        Places {
            system,
            folders: facts.profile_folders.iter().map(resolve).collect(),
            other_homes: facts.other_homes.iter().map(resolve).collect(),
        }
    }

    fn same(&self, a: &Path, b: &Path) -> bool {
        match self.system {
            SystemKind::Windows => windows_key(a) == windows_key(b),
            _ => same_place(a, b),
        }
    }

    fn listed(&self, path: &Path) -> bool {
        self.folders.iter().any(|f| self.same(f, path))
    }

    /// The folder is there: listed in the profiles folder, or found where it is.
    fn present(&self, path: &Path) -> bool {
        self.listed(path) || path.is_dir()
    }

    fn name(&self, path: &Path) -> String {
        match self.system {
            SystemKind::Windows => path
                .to_string_lossy()
                .trim_end_matches(['\\', '/'])
                .rsplit(['\\', '/'])
                .next()
                .unwrap_or_default()
                .to_string(),
            _ => path
                .file_name()
                .map(|n| n.to_string_lossy().into_owned())
                .unwrap_or_default(),
        }
    }
}

/// Mac and Linux: a home behind a link (Fedora Silverblue's /home -> /var/home) is compared by
/// where it really is. Windows paths are kept as written.
fn resolve(system: SystemKind, path: &Path) -> PathBuf {
    match system {
        SystemKind::Windows => path.to_path_buf(),
        _ => real(path),
    }
}

/// A Windows path compared the Windows way: case ignored, either slash, no `\\?\`, no trailing
/// slash.
fn windows_key(path: &Path) -> String {
    let text = path.to_string_lossy().replace('/', "\\").to_lowercase();
    let text = text.strip_prefix(r"\\?\").unwrap_or(&text);
    text.trim_end_matches('\\').to_string()
}

fn same_id(a: &str, b: &str) -> bool {
    a.eq_ignore_ascii_case(b)
}

fn starts_with_ignoring_case(text: &str, start: &str) -> bool {
    text.len() >= start.len()
        && text.is_char_boundary(start.len())
        && text[..start.len()].eq_ignore_ascii_case(start)
}

/// Local and domain accounts (`S-1-5-21-...`) and work or school Entra ID accounts
/// (`S-1-12-1-...`). `.bak` entries are Windows' copies of broken profiles, not people.
fn is_person_sid(sid: &str) -> bool {
    (starts_with_ignoring_case(sid, "S-1-5-21-") || starts_with_ignoring_case(sid, "S-1-12-1-"))
        && !sid.to_ascii_lowercase().ends_with(".bak")
}

/// The domain part of a local or domain account SID: `S-1-5-21-a-b-c`.
fn sid_domain(sid: &str) -> Option<String> {
    if !starts_with_ignoring_case(sid, "S-1-5-21-") {
        return None;
    }
    let parts: Vec<&str> = sid.split('-').collect();
    (parts.len() >= 8).then(|| parts[..7].join("-").to_ascii_uppercase())
}

fn windows_residents(facts: &CensusFacts, accounts: &[Account], places: &Places) -> Vec<Resident> {
    let account_list_read = facts.accounts.is_some();
    let local_domain = accounts.iter().find_map(|a| sid_domain(&a.id));
    let person_entries: Vec<&(String, PathBuf)> = facts
        .profile_entries
        .iter()
        .filter(|(sid, _)| is_person_sid(sid))
        .collect();
    let entry_of = |id: &str| person_entries.iter().find(|(sid, _)| same_id(sid, id));

    let mut residents = Vec::new();
    for account in accounts {
        let entry = entry_of(&account.id);
        let home = entry
            .map(|(_, path)| path)
            .filter(|path| places.present(path))
            .cloned();
        // A switched-off account with no folder holds nothing and nobody (the built-in ones).
        if account.switched_off && home.is_none() {
            continue;
        }
        let mut found_by = vec![FoundBy::AccountList];
        if entry.is_some() {
            found_by.push(FoundBy::ProfilesFolder);
        }
        let kind = if account.managed {
            PersonKind::Managed
        } else if home.is_some() {
            PersonKind::Other
        } else {
            PersonKind::NeverUsed
        };
        residents.push(Resident {
            account_id: Some(account.id.clone()),
            kind,
            suggested_name: suggested_name(account),
            home,
            found_by,
        });
    }
    for (sid, path) in &person_entries {
        if accounts.iter().any(|a| same_id(&a.id, sid)) {
            continue;
        }
        let present = places.present(path);
        if !present && !same_id(sid, &facts.me) {
            // An entry whose folder is gone holds nothing to move.
            continue;
        }
        let kind = if starts_with_ignoring_case(sid, "S-1-12-1-") {
            PersonKind::Managed
        } else if !account_list_read {
            PersonKind::Other
        } else {
            match (&local_domain, sid_domain(sid)) {
                (Some(local), Some(domain)) if *local != domain => PersonKind::Managed,
                (Some(_), _) => PersonKind::Leftover,
                (None, _) => PersonKind::Other,
            }
        };
        residents.push(Resident {
            account_id: Some(sid.clone()),
            kind,
            suggested_name: places.name(path),
            home: present.then(|| path.clone()),
            found_by: vec![FoundBy::ProfilesFolder],
        });
    }
    residents
}

fn unix_residents(accounts: &[Account], account_list_read: bool, places: &Places) -> Vec<Resident> {
    accounts
        .iter()
        .map(|account| {
            let home = account.home.as_deref().map(|h| resolve(places.system, h));
            let listed = home.as_deref().is_some_and(|h| places.listed(h));
            let present = listed || home.as_deref().is_some_and(Path::is_dir);
            let mut found_by = vec![FoundBy::AccountList];
            if listed {
                found_by.push(FoundBy::ProfilesFolder);
            }
            let kind = if account.managed {
                PersonKind::Managed
            } else if present || !account_list_read {
                PersonKind::Other
            } else {
                PersonKind::NeverUsed
            };
            Resident {
                account_id: Some(account.id.clone()),
                kind,
                suggested_name: suggested_name(account),
                home: if present { home } else { None },
                found_by,
            }
        })
        .collect()
}

/// Folders in the profiles folder that belong to nobody the system knows: on Windows, no profile
/// entry points at them; on Mac and Linux, they are no account's home.
fn leftover_folders(
    facts: &CensusFacts,
    accounts: &[Account],
    places: &Places,
    census: &Census,
) -> Vec<Resident> {
    let mut leftovers = Vec::new();
    for folder in &places.folders {
        let name = places.name(folder);
        if is_system_folder(places.system, &name) {
            continue;
        }
        let referenced = match places.system {
            SystemKind::Windows => facts
                .profile_entries
                .iter()
                .any(|(_, path)| places.same(path, folder)),
            _ => accounts.iter().any(|a| {
                a.home
                    .as_deref()
                    .is_some_and(|h| places.same(&resolve(places.system, h), folder))
            }),
        };
        if referenced || places.other_homes.iter().any(|h| places.same(h, folder)) {
            continue;
        }
        let owner = facts
            .folder_owners
            .iter()
            .find(|(path, _)| places.same(&resolve(places.system, path), folder))
            .map(|(_, id)| id.as_str())
            .filter(|id| census.is_person_id(id))
            .filter(|id| !accounts.iter().any(|a| same_id(&a.id, id)));
        // Without the account list on a Mac or Linux laptop, a folder in the profiles folder is
        // most likely an account's home; it is only called a leftover with evidence.
        let kind = if places.system != SystemKind::Windows && !census.account_list_read {
            PersonKind::Other
        } else {
            PersonKind::Leftover
        };
        leftovers.push(Resident {
            account_id: owner.map(str::to_string),
            kind,
            suggested_name: name,
            home: Some(folder.clone()),
            found_by: vec![FoundBy::ProfilesFolder],
        });
    }
    leftovers
}

/// Folders in the profiles folder that never belong to a person.
fn is_system_folder(system: SystemKind, name: &str) -> bool {
    let lower = name.to_lowercase();
    match system {
        SystemKind::Windows => {
            [
                "public",
                "default",
                "default user",
                "all users",
                "desktop.ini",
            ]
            .contains(&lower.as_str())
                || lower.starts_with("defaultuser")
        }
        SystemKind::Mac => name.starts_with('.') || name == "Shared" || name == "Guest",
        SystemKind::Linux => name.starts_with('.') || name == "lost+found",
    }
}

fn suggested_name(account: &Account) -> String {
    account
        .full_name
        .as_deref()
        .map(str::trim)
        .filter(|n| !n.is_empty())
        .unwrap_or(&account.name)
        .to_string()
}

fn sort_residents(residents: &mut [Resident], system: SystemKind) {
    let home_key = |r: &Resident| match (&r.home, system) {
        (Some(home), SystemKind::Windows) => windows_key(home),
        (Some(home), _) => home.to_string_lossy().into_owned(),
        (None, _) => String::new(),
    };
    residents.sort_by(|a, b| {
        (
            a.kind != PersonKind::Me,
            a.home.is_none(),
            home_key(a),
            &a.account_id,
            &a.suggested_name,
        )
            .cmp(&(
                b.kind != PersonKind::Me,
                b.home.is_none(),
                home_key(b),
                &b.account_id,
                &b.suggested_name,
            ))
    });
}

// ---------------------------------------------------------------- reading the system's answers

/// Windows: `reg query "HKLM\...\ProfileList" /s`. Returns the profiles folder (the top-level
/// `ProfilesDirectory` value) and every subkey's SID with its `ProfileImagePath`, service
/// accounts and `.bak` copies included. `%SystemDrive%` and `%SystemRoot%` are filled in.
pub fn profile_list_from_reg(
    output: &str,
    system_drive: &str,
    system_root: &str,
) -> (Option<PathBuf>, Vec<(String, PathBuf)>) {
    let vars = [("systemdrive", system_drive), ("systemroot", system_root)];
    let mut root = None;
    let mut entries = Vec::new();
    // None while in the ProfileList key itself.
    let mut sid: Option<String> = None;
    for line in output.lines() {
        let line = line.trim_end_matches('\r');
        if line.starts_with("HKEY_") {
            let last = line.rsplit('\\').next().unwrap_or_default();
            sid = (!last.eq_ignore_ascii_case("ProfileList")).then(|| last.to_string());
            continue;
        }
        // "    Name    REG_TYPE    value": four spaces between the parts.
        let mut parts = line.trim_start().splitn(3, "    ");
        let (Some(name), Some(kind), Some(value)) = (parts.next(), parts.next(), parts.next())
        else {
            continue;
        };
        if kind != "REG_SZ" && kind != "REG_EXPAND_SZ" {
            continue;
        }
        let value = PathBuf::from(expand(value.trim(), &vars));
        match &sid {
            None if name.eq_ignore_ascii_case("ProfilesDirectory") => root = Some(value),
            Some(sid) if name.eq_ignore_ascii_case("ProfileImagePath") => {
                entries.push((sid.clone(), value));
            }
            _ => {}
        }
    }
    (root, entries)
}

/// Fills in `%NAME%` for the given names, ignoring case; anything else is left as written.
fn expand(text: &str, vars: &[(&str, &str)]) -> String {
    let mut out = String::new();
    let mut rest = text;
    while let Some(start) = rest.find('%') {
        let after = &rest[start + 1..];
        let Some(end) = after.find('%') else { break };
        let name = &after[..end];
        out.push_str(&rest[..start]);
        match vars.iter().find(|(v, _)| v.eq_ignore_ascii_case(name)) {
            Some((_, value)) => out.push_str(value),
            None => {
                out.push('%');
                out.push_str(name);
                out.push('%');
            }
        }
        rest = &after[end + 1..];
    }
    out.push_str(rest);
    out
}

/// Windows: `Get-LocalUser`, one account per line: SID, enabled (1 or 0), name and full name,
/// separated by tabs. Accounts that are switched off are kept and marked: the built-in
/// Administrator, Guest, DefaultAccount and WDAGUtilityAccount usually are and have no folder, so
/// they are nobody; a person's account switched off still has that person's files.
pub fn accounts_from_get_local_user(output: &str) -> Vec<Account> {
    output
        .lines()
        .filter_map(|line| {
            let mut f = line.trim_end_matches('\r').split('\t');
            let (id, enabled, name) = (f.next()?.trim(), f.next()?.trim(), f.next()?.trim());
            let full = f.next().unwrap_or("").trim();
            let enabled = enabled == "1" || enabled.eq_ignore_ascii_case("true");
            if !id.starts_with("S-") {
                return None;
            }
            Some(Account {
                id: id.to_string(),
                name: name.to_string(),
                full_name: (!full.is_empty()).then(|| full.to_string()),
                home: None,
                managed: false,
                switched_off: !enabled,
            })
        })
        .collect()
}

/// Windows: `whoami /user /fo csv /nh` gives `"name","SID"`; the SID is the last field.
pub fn sid_from_whoami(output: &str) -> Option<String> {
    let line = output.lines().map(str::trim).rfind(|l| !l.is_empty())?;
    let sid = line.rsplit(',').next()?.trim().trim_matches('"');
    sid.starts_with("S-").then(|| sid.to_string())
}

/// One record from `dscl . -readall`: each attribute with its values. Values on the key's own
/// line are separated by spaces; a value with spaces sits alone on a following line.
fn dscl_records(output: &str) -> Vec<Vec<(String, Vec<String>)>> {
    let mut records = Vec::new();
    let mut fields: Vec<(String, Vec<String>)> = Vec::new();
    for line in output.lines().map(|l| l.trim_end_matches('\r')) {
        if line == "-" {
            records.push(std::mem::take(&mut fields));
        } else if let Some(more) = line.strip_prefix(' ') {
            if let Some(last) = fields.last_mut() {
                last.1.push(more.trim().to_string());
            }
        } else if let Some((key, value)) = line.split_once(':') {
            let key = key.trim();
            let key = key.strip_prefix("dsAttrTypeNative:").unwrap_or(key);
            let values = value.split_whitespace().map(str::to_string).collect();
            fields.push((key.to_string(), values));
        }
    }
    records.push(fields);
    records.retain(|r| !r.is_empty());
    records
}

fn dscl_first<'a>(record: &'a [(String, Vec<String>)], key: &str) -> Option<&'a str> {
    record
        .iter()
        .find(|(k, _)| k == key)
        .and_then(|(_, v)| v.first())
        .map(String::as_str)
}

/// A person on a Mac: an ID from 501 and a name that doesn't start with `_`.
fn dscl_is_person(record: &[(String, Vec<String>)]) -> bool {
    let uid = dscl_first(record, "UniqueID").and_then(|u| u.parse::<i64>().ok());
    let name = dscl_first(record, "RecordName").unwrap_or("_");
    uid.is_some_and(|u| u >= 501) && !name.starts_with('_')
}

/// macOS: `dscl . -readall /Users NFSHomeDirectory RealName UniqueID OriginalNodeName
/// AuthenticationAuthority`. A mobile account from a company directory has `OriginalNodeName`
/// or a `LocalCachedUser` authority: a work or school account.
pub fn accounts_from_dscl(output: &str) -> Vec<Account> {
    dscl_records(output)
        .iter()
        .filter(|r| dscl_is_person(r))
        .filter_map(|record| {
            let id = dscl_first(record, "UniqueID")?;
            let name = dscl_first(record, "RecordName")?;
            let full = record
                .iter()
                .find(|(k, _)| k == "RealName")
                .map(|(_, v)| v.join(" "))
                .filter(|n| !n.trim().is_empty());
            let authority = record
                .iter()
                .find(|(k, _)| k == "AuthenticationAuthority")
                .map(|(_, v)| v.join(" "))
                .unwrap_or_default();
            let managed = record
                .iter()
                .any(|(k, v)| k == "OriginalNodeName" && !v.is_empty())
                || authority.contains("LocalCachedUser");
            Some(Account {
                id: id.to_string(),
                name: name.to_string(),
                full_name: full,
                home: dscl_first(record, "NFSHomeDirectory").map(PathBuf::from),
                managed,
                switched_off: false,
            })
        })
        .collect()
}

/// macOS: the home folders of system and service accounts in the same `dscl` output.
pub fn service_homes_from_dscl(output: &str) -> Vec<PathBuf> {
    dscl_records(output)
        .iter()
        .filter(|r| !dscl_is_person(r))
        .filter_map(|r| dscl_first(r, "NFSHomeDirectory").map(PathBuf::from))
        .collect()
}

/// One `passwd` line: name, uid, full name, home, and whether the shell lets anyone sign in.
fn passwd_record(line: &str) -> Option<(&str, u32, &str, &str, bool)> {
    let f: Vec<&str> = line.trim_end_matches('\r').split(':').collect();
    if f.len() < 7 {
        return None;
    }
    let uid: u32 = f[2].parse().ok()?;
    let shell = f[6].trim();
    let can_sign_in = !shell.ends_with("nologin") && !shell.ends_with("false");
    let full = f[4].split(',').next().unwrap_or("").trim();
    Some((f[0], uid, full, f[5], can_sign_in))
}

/// Linux: people from `getent passwd`, which also lists accounts from a network directory. An
/// account in `getent` but not in the laptop's own `/etc/passwd` comes from that directory: a work
/// or school account. People sign in with a real shell and have IDs in the normal user range;
/// directory accounts often have larger IDs, so for them only the lower end applies.
pub fn accounts_from_passwd(
    getent: &str,
    local: &str,
    (uid_min, uid_max): (u32, u32),
) -> Vec<Account> {
    let local_ids: Vec<u32> = local
        .lines()
        .filter_map(passwd_record)
        .map(|r| r.1)
        .collect();
    let mut accounts: Vec<Account> = Vec::new();
    for (name, uid, full, home, can_sign_in) in getent.lines().filter_map(passwd_record) {
        let managed = !local_ids.is_empty() && !local_ids.contains(&uid);
        let in_range = uid >= uid_min && (uid <= uid_max || managed);
        if !in_range || !can_sign_in || accounts.iter().any(|a| a.id == uid.to_string()) {
            continue;
        }
        accounts.push(Account {
            id: uid.to_string(),
            name: name.to_string(),
            full_name: (!full.is_empty()).then(|| full.to_string()),
            home: Some(PathBuf::from(home)),
            managed,
            switched_off: false,
        });
    }
    accounts
}

/// Linux: the home folders of system and service accounts (no sign-in shell, or an ID outside
/// the normal user range).
pub fn service_homes_from_passwd(getent: &str, (uid_min, uid_max): (u32, u32)) -> Vec<PathBuf> {
    getent
        .lines()
        .filter_map(passwd_record)
        .filter(|(_, uid, _, _, can_sign_in)| !can_sign_in || *uid < uid_min || *uid > uid_max)
        .map(|(_, _, _, home, _)| PathBuf::from(home))
        .collect()
}

// ---------------------------------------------------------------- reading this laptop

/// Reads the census of the running laptop. Needs no administrator rights and opens nobody's
/// files: only the system's own lists and the names of the folders in the profiles folder.
pub fn census_this_laptop() -> Census {
    census_from(match SystemKind::this() {
        SystemKind::Windows => windows_facts(),
        SystemKind::Mac => mac_facts(),
        SystemKind::Linux => linux_facts(),
    })
}

fn windows_facts() -> CensusFacts {
    let system_root = std::env::var("SystemRoot").unwrap_or_else(|_| r"C:\Windows".into());
    let system_drive = std::env::var("SystemDrive").unwrap_or_else(|_| "C:".into());
    let system32 = Path::new(&system_root).join("System32");
    let (profiles_root, profile_entries) = reg_profile_list(&system32)
        .map(|out| profile_list_from_reg(&out, &system_drive, &system_root))
        .unwrap_or_default();

    let script = "[Console]::OutputEncoding=[Text.Encoding]::UTF8; \
        Get-LocalUser | ForEach-Object { $_.SID.Value + [char]9 + [int]$_.Enabled + [char]9 + \
        $_.Name + [char]9 + $_.FullName }";
    let mut powershell =
        pctwin_types::process::quiet(system32.join(r"WindowsPowerShell\v1.0\powershell.exe"));
    powershell.args(["-NoProfile", "-NonInteractive", "-Command", script]);
    let accounts = run(powershell)
        .ok()
        .filter(|out| !out.trim().is_empty())
        .map(|out| accounts_from_get_local_user(&out));

    let mut whoami = pctwin_types::process::quiet(system32.join("whoami.exe"));
    whoami.args(["/user", "/fo", "csv", "/nh"]);
    let me = run(whoami)
        .ok()
        .and_then(|out| sid_from_whoami(&out))
        .or_else(|| {
            // Without whoami, the profile entry whose folder is this person's home.
            let home = dirs::home_dir()?;
            profile_entries
                .iter()
                .find(|(sid, path)| is_person_sid(sid) && windows_key(path) == windows_key(&home))
                .map(|(sid, _)| sid.clone())
        })
        .unwrap_or_default();

    CensusFacts {
        me,
        accounts,
        profile_folders: profiles_root
            .as_deref()
            .map(list_folders)
            .unwrap_or_default(),
        profiles_root,
        profile_entries,
        folder_owners: Vec::new(),
        other_homes: Vec::new(),
        uid_range: None,
        system: SystemKind::Windows,
    }
}

/// `reg query` writes in the console's code page, which is not UTF-8 on most laptops: a profile
/// folder named `Jürgen` or `张伟` would come back garbled. Switching the code page to UTF-8
/// first keeps every name intact.
#[cfg(windows)]
fn reg_profile_list(system32: &Path) -> Result<String, String> {
    use std::os::windows::process::CommandExt;
    let chcp = system32.join("chcp.com");
    let reg = system32.join("reg.exe");
    let mut cmd = pctwin_types::process::quiet(system32.join("cmd.exe"));
    cmd.raw_arg(format!(
        r#"/d /c ""{}" 65001 >nul & "{}" query "HKLM\SOFTWARE\Microsoft\Windows NT\CurrentVersion\ProfileList" /s""#,
        chcp.display(),
        reg.display()
    ));
    run(cmd)
}

#[cfg(not(windows))]
fn reg_profile_list(_system32: &Path) -> Result<String, String> {
    Err("the Windows profile list exists only on Windows".into())
}

fn mac_facts() -> CensusFacts {
    let mut dscl = pctwin_types::process::quiet("/usr/bin/dscl");
    dscl.args([
        ".",
        "-readall",
        "/Users",
        "NFSHomeDirectory",
        "RealName",
        "UniqueID",
        "OriginalNodeName",
        "AuthenticationAuthority",
    ]);
    let out = run(dscl).ok().filter(|o| !o.trim().is_empty());
    let root = PathBuf::from("/Users");
    let folders = list_folders(&root);
    CensusFacts {
        me: unix_me(),
        accounts: out.as_deref().map(accounts_from_dscl),
        other_homes: out
            .as_deref()
            .map(service_homes_from_dscl)
            .unwrap_or_default(),
        folder_owners: owners(&folders),
        profile_folders: folders,
        profiles_root: Some(root),
        profile_entries: Vec::new(),
        uid_range: None,
        system: SystemKind::Mac,
    }
}

fn linux_facts() -> CensusFacts {
    let local = std::fs::read_to_string("/etc/passwd").unwrap_or_default();
    let mut getent = pctwin_types::process::quiet("getent");
    getent.arg("passwd");
    let getent = run(getent)
        .ok()
        .filter(|o| !o.trim().is_empty())
        .unwrap_or_else(|| local.clone());
    let defs = std::fs::read_to_string("/etc/login.defs").ok();
    let range = crate::people::uid_range_from_login_defs(defs.as_deref());
    let read = !getent.trim().is_empty();
    // Fedora Silverblue and Kinoite keep homes in /var/home behind a /home link.
    let root = real(Path::new("/home"));
    let folders = list_folders(&root);
    CensusFacts {
        me: unix_me(),
        accounts: read.then(|| accounts_from_passwd(&getent, &local, range)),
        other_homes: service_homes_from_passwd(&getent, range),
        folder_owners: owners(&folders),
        profile_folders: folders,
        profiles_root: Some(root),
        profile_entries: Vec::new(),
        uid_range: Some(range),
        system: SystemKind::Linux,
    }
}

/// The signed-in user ID: `id -u`, or the owner of this person's home folder.
fn unix_me() -> String {
    let mut id = pctwin_types::process::quiet("id");
    id.arg("-u");
    run(id)
        .ok()
        .map(|o| o.trim().to_string())
        .filter(|o| !o.is_empty())
        .or_else(|| dirs::home_dir().and_then(|h| owner_of(&h)))
        .unwrap_or_default()
}

/// The folders inside the profiles folder, by name only. A folder that cannot be listed gives an
/// empty list; the account list still finds everyone with an account.
fn list_folders(root: &Path) -> Vec<PathBuf> {
    let Ok(entries) = std::fs::read_dir(root) else {
        return Vec::new();
    };
    let mut folders: Vec<PathBuf> = entries
        .filter_map(Result::ok)
        .map(|e| e.path())
        .filter(|p| p.is_dir())
        .collect();
    folders.sort();
    folders
}

fn owners(folders: &[PathBuf]) -> Vec<(PathBuf, String)> {
    folders
        .iter()
        .filter_map(|f| owner_of(f).map(|id| (f.clone(), id)))
        .collect()
}

#[cfg(unix)]
fn owner_of(path: &Path) -> Option<String> {
    use std::os::unix::fs::MetadataExt;
    std::fs::symlink_metadata(path)
        .ok()
        .map(|m| m.uid().to_string())
}

#[cfg(not(unix))]
fn owner_of(_path: &Path) -> Option<String> {
    None
}

/// The longest any one system tool may take: a minute, because PowerShell alone can take most
/// of 15 seconds to start on a slow or busy laptop (seen under load in review).
const TOOL_LIMIT: Duration = Duration::from_secs(60);

/// Runs a system tool with a time limit, without a console window on Windows. The answer is read
/// while the tool runs, so a long answer can never stall it.
fn run(mut command: Command) -> Result<String, String> {
    command
        .stdin(Stdio::null())
        .stdout(Stdio::piped())
        .stderr(Stdio::null());
    #[cfg(windows)]
    {
        use std::os::windows::process::CommandExt;
        const CREATE_NO_WINDOW: u32 = 0x0800_0000;
        command.creation_flags(CREATE_NO_WINDOW);
    }
    let program = command.get_program().to_string_lossy().into_owned();
    let mut child = command
        .spawn()
        .map_err(|e| format!("{program} could not start: {e}"))?;
    let mut stdout = child
        .stdout
        .take()
        .ok_or_else(|| format!("{program} gave no answer"))?;
    let reader = std::thread::spawn(move || {
        let mut answer = Vec::new();
        let _ = stdout.read_to_end(&mut answer);
        answer
    });
    let started = Instant::now();
    let status = loop {
        match child.try_wait() {
            Ok(Some(status)) => break status,
            Ok(None) if started.elapsed() > TOOL_LIMIT => {
                let _ = child.kill();
                let _ = child.wait();
                return Err(format!("{program} took too long"));
            }
            Ok(None) => std::thread::sleep(Duration::from_millis(20)),
            Err(e) => return Err(format!("{program} failed: {e}")),
        }
    };
    let answer = reader
        .join()
        .map_err(|_| format!("{program}'s answer could not be read"))?;
    if !status.success() {
        return Err(format!("{program} reported a problem"));
    }
    Ok(String::from_utf8_lossy(&answer).into_owned())
}
