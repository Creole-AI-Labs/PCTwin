use std::path::{Path, PathBuf};

use serde::Serialize;

use crate::folders::same_place;

/// One person on the old laptop.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct Person {
    /// The system's own ID: a SID on Windows, the numeric user ID on Mac and Linux.
    pub account_id: String,
    /// A name to suggest. Only ever a suggestion: people are matched by `account_id`.
    pub suggested_name: String,
    pub home: PathBuf,
    /// The person signed in and running PCTwin.
    pub is_me: bool,
}

/// Everyone with an account on this laptop, read from the list the system keeps for everyone
/// (no administrator rights needed, nobody's files opened). System and service accounts are left
/// out.
pub fn list_people() -> Vec<Person> {
    let me = dirs::home_dir().unwrap_or_default();
    if cfg!(windows) {
        let out = run(
            "reg",
            &[
                "query",
                r"HKLM\SOFTWARE\Microsoft\Windows NT\CurrentVersion\ProfileList",
                "/s",
                "/v",
                "ProfileImagePath",
            ],
        );
        people_from_profile_list(&out, &me)
    } else if cfg!(target_os = "macos") {
        let out = run(
            "dscl",
            &[
                ".",
                "-readall",
                "/Users",
                "NFSHomeDirectory",
                "RealName",
                "UniqueID",
            ],
        );
        people_from_dscl(&out, &me)
    } else {
        // getent also lists accounts from a network directory; /etc/passwd is the fallback.
        let mut passwd = run("getent", &["passwd"]);
        if passwd.trim().is_empty() {
            passwd = std::fs::read_to_string("/etc/passwd").unwrap_or_default();
        }
        let defs = std::fs::read_to_string("/etc/login.defs").ok();
        people_from_passwd(&passwd, uid_range_from_login_defs(defs.as_deref()), &me)
    }
}

fn run(program: &str, args: &[&str]) -> String {
    std::process::Command::new(program)
        .args(args)
        .output()
        .map(|o| String::from_utf8_lossy(&o.stdout).into_owned())
        .unwrap_or_default()
}

/// Windows: `reg query ...\ProfileList /s /v ProfileImagePath`. Real people have local account
/// SIDs (`S-1-5-21-...`) or work and school (Entra ID) SIDs (`S-1-12-1-...`); `.bak` keys are
/// Windows' copies of broken profiles.
pub fn people_from_profile_list(output: &str, me: &Path) -> Vec<Person> {
    let system_drive = std::env::var("SystemDrive").unwrap_or_else(|_| "C:".into());
    let mut people = Vec::new();
    let mut sid: Option<String> = None;
    for line in output.lines() {
        let line = line.trim();
        if line.starts_with("HKEY_") {
            sid = line.rsplit('\\').next().map(str::to_string);
            continue;
        }
        let Some(rest) = line.strip_prefix("ProfileImagePath") else {
            continue;
        };
        let rest = rest.trim_start();
        let Some(value) = rest
            .strip_prefix("REG_EXPAND_SZ")
            .or_else(|| rest.strip_prefix("REG_SZ"))
        else {
            continue;
        };
        let Some(id) = sid.take() else { continue };
        let real =
            (id.starts_with("S-1-5-21-") || id.starts_with("S-1-12-1-")) && !id.ends_with(".bak");
        if !real {
            continue;
        }
        let path = value
            .trim()
            .replace("%SystemDrive%", &system_drive)
            .replace("%systemdrive%", &system_drive);
        let home = PathBuf::from(path);
        people.push(Person {
            suggested_name: last_name(&home),
            is_me: same_place(&home, me),
            account_id: id,
            home,
        });
    }
    people
}

fn last_name(home: &Path) -> String {
    home.file_name()
        .map(|n| n.to_string_lossy().into_owned())
        .unwrap_or_default()
}

/// macOS: `dscl . -readall /Users NFSHomeDirectory RealName UniqueID`. Records are separated by
/// a line holding `-`; a value may sit on the lines after its key. People have IDs from 501 and
/// names that don't start with `_`.
pub fn people_from_dscl(output: &str, me: &Path) -> Vec<Person> {
    let mut people = Vec::new();
    for record in output.split("\n-\n").chain(std::iter::once("")) {
        let mut fields: Vec<(String, String)> = Vec::new();
        for line in record.lines() {
            if line == "-" {
                continue;
            }
            if let Some(more) = line.strip_prefix(' ') {
                if let Some(last) = fields.last_mut() {
                    if !last.1.is_empty() {
                        last.1.push(' ');
                    }
                    last.1.push_str(more.trim());
                }
            } else if let Some((key, value)) = line.split_once(':') {
                fields.push((key.trim().to_string(), value.trim().to_string()));
            }
        }
        let get = |k: &str| {
            fields
                .iter()
                .find(|(key, _)| key == k)
                .map(|(_, v)| v.as_str())
        };
        let (Some(name), Some(uid), Some(home)) =
            (get("RecordName"), get("UniqueID"), get("NFSHomeDirectory"))
        else {
            continue;
        };
        let Ok(uid_number) = uid.parse::<i64>() else {
            continue;
        };
        if uid_number < 501 || name.starts_with('_') {
            continue;
        }
        let home = PathBuf::from(home);
        let real_name = get("RealName").filter(|n| !n.is_empty()).unwrap_or(name);
        people.push(Person {
            account_id: uid.to_string(),
            suggested_name: real_name.to_string(),
            is_me: same_place(&home, me),
            home,
        });
    }
    people
}

/// Linux: `name:x:uid:gid:full name,room,...:home:shell`. People have IDs in the normal user range
/// and a shell they can sign in with.
pub fn people_from_passwd(passwd: &str, (uid_min, uid_max): (u32, u32), me: &Path) -> Vec<Person> {
    passwd
        .lines()
        .filter_map(|line| {
            let f: Vec<&str> = line.split(':').collect();
            if f.len() < 7 {
                return None;
            }
            let uid: u32 = f[2].parse().ok()?;
            let shell = f[6].trim();
            let can_sign_in = !shell.ends_with("nologin") && !shell.ends_with("false");
            if uid < uid_min || uid > uid_max || !can_sign_in {
                return None;
            }
            let full = f[4].split(',').next().unwrap_or("").trim();
            let home = PathBuf::from(f[5]);
            Some(Person {
                account_id: uid.to_string(),
                suggested_name: if full.is_empty() { f[0] } else { full }.to_string(),
                is_me: same_place(&home, me),
                home,
            })
        })
        .collect()
}

/// The normal user ID range from `/etc/login.defs` (default 1000 to 60000).
pub fn uid_range_from_login_defs(defs: Option<&str>) -> (u32, u32) {
    let mut range = (1000, 60000);
    for line in defs.unwrap_or("").lines() {
        let mut words = line.split_whitespace();
        let (Some(key), Some(value)) = (words.next(), words.next()) else {
            continue;
        };
        let Ok(value) = value.parse::<u32>() else {
            continue;
        };
        match key {
            "UID_MIN" => range.0 = value,
            "UID_MAX" => range.1 = value,
            _ => {}
        }
    }
    range
}
