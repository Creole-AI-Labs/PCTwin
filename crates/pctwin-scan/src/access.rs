//! Who could open a place on the old laptop, read from its owner and permissions and boiled down
//! to the few facts the ownership rules use (Engineering Plan 9.16, "Access lists are
//! normalised"). Nothing here opens a file.
//!
//! - Administrators, SYSTEM, TrustedInstaller, CREATOR OWNER, Owner Rights and the system's
//!   service and app accounts carry no signal: they can open everything, so they say nothing
//!   about whose a thing is.
//! - Users, Authenticated Users, Everyone, INTERACTIVE, Guests and Anonymous mean everyone; on a
//!   Mac the `staff` group means everyone; on Linux "other" read means everyone.
//! - A Unix group that is one person's own private group means that person.

use std::collections::{BTreeMap, BTreeSet};

use serde::{Deserialize, Serialize};

/// Who could open a place, after the noise is taken out.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct Access {
    /// Everyone on the laptop can open it.
    pub everyone: bool,
    /// The accounts (or groups the system did not explain) that can open it, by ID, besides the
    /// administrators.
    pub people: BTreeSet<String>,
    /// The owner the drive records, by ID.
    pub owner: Option<String>,
}

/// What could be learned about a place's access.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "kebab-case")]
pub enum AccessFact {
    Known(Access),
    /// The drive keeps no owners or permissions (FAT, exFAT, a network share).
    NoOwners,
    /// The permissions could not be read, with the reason.
    Unknown {
        why: String,
    },
}

// ------------------------------------------------------------------------------------------
// Windows: security descriptors in SDDL text, as `Get-Acl`'s `Sddl` gives them.

/// Well-known accounts and groups that carry no signal.
fn windows_noise(sid: &str) -> bool {
    matches!(
        sid,
        "BA" | "SY"
            | "CO"
            | "OW"
            | "CG"
            | "AC"
            | "NS"
            | "LS"
            | "SO"
            | "BO"
            | "PO"
            | "SU"
            | "S-1-5-32-544"
            | "S-1-5-18"
            | "S-1-5-19"
            | "S-1-5-20"
            | "S-1-3-0"
            | "S-1-3-1"
            | "S-1-3-4"
            | "S-1-5-6"
            | "S-1-5-32-549"
            | "S-1-5-32-550"
            | "S-1-5-32-551"
    ) || sid.starts_with("S-1-5-80-")
        || sid.starts_with("S-1-5-82-")
        || sid.starts_with("S-1-5-83-")
        || sid.starts_with("S-1-5-90-")
        || sid.starts_with("S-1-5-96-")
        || sid.starts_with("S-1-15-")
        || is_rid(sid, 512)
}

/// Accounts and groups that mean everyone who can sign in.
fn windows_everyone(sid: &str) -> bool {
    matches!(
        sid,
        "WD" | "AU"
            | "BU"
            | "IU"
            | "BG"
            | "AN"
            | "RD"
            | "PU"
            | "S-1-1-0"
            | "S-1-5-11"
            | "S-1-5-32-545"
            | "S-1-5-32-546"
            | "S-1-5-4"
            | "S-1-5-7"
            | "S-1-5-32-555"
            | "S-1-5-32-547"
    ) || is_rid(sid, 513)
        || is_rid(sid, 545)
}

/// A machine or domain account ending in `rid` (`S-1-5-21-a-b-c-513`, the "None" or "Domain
/// Users" group every account belongs to).
fn is_rid(sid: &str, rid: u32) -> bool {
    sid.starts_with("S-1-5-21-")
        && sid.matches('-').count() == 7
        && sid.rsplit('-').next() == Some(rid.to_string().as_str())
}

/// Whether an access mask (`FA`, `GRGX`, `0x1200a9`) lets its holder read the contents.
fn grants_read(rights: &str) -> bool {
    if let Some(hex) = rights
        .strip_prefix("0x")
        .or_else(|| rights.strip_prefix("0X"))
    {
        let Ok(mask) = u32::from_str_radix(hex, 16) else {
            return false;
        };
        const READ_DATA: u32 = 0x1;
        const GENERIC_ALL: u32 = 0x1000_0000;
        const GENERIC_READ: u32 = 0x8000_0000;
        return mask & (READ_DATA | GENERIC_ALL | GENERIC_READ) != 0;
    }
    let bytes = rights.as_bytes();
    bytes
        .chunks(2)
        .any(|c| matches!(c, b"GA" | b"GR" | b"FA" | b"FR" | b"KA" | b"KR" | b"CC"))
}

/// Reads a security descriptor in SDDL (`O:S-1-5-21-..G:..D:PAI(A;OICI;FA;;;SY)...`). Returns
/// `None` when it is not one.
pub fn access_from_sddl(sddl: &str) -> Option<Access> {
    let sddl = sddl.trim();
    let owner = section(sddl, "O:").map(str::to_string);
    let Some(dacl) = section(sddl, "D:") else {
        // No access list at all: everyone may open it.
        return Some(Access {
            everyone: true,
            people: BTreeSet::new(),
            owner,
        });
    };
    if dacl.starts_with("NO_ACCESS_CONTROL") {
        return Some(Access {
            everyone: true,
            people: BTreeSet::new(),
            owner,
        });
    }
    let mut access = Access {
        owner,
        ..Access::default()
    };
    let mut denied: BTreeSet<String> = BTreeSet::new();
    let mut rest = dacl;
    while let Some(open) = rest.find('(') {
        let close = rest[open..].find(')')? + open;
        let ace = &rest[open + 1..close];
        rest = &rest[close + 1..];
        let fields: Vec<&str> = ace.split(';').collect();
        if fields.len() < 6 {
            return None;
        }
        let (kind, flags, rights, sid) = (fields[0], fields[1], fields[2], fields[5]);
        // An entry only passed on to what is inside says nothing about this place itself.
        if flags.as_bytes().chunks(2).any(|f| f == b"IO") || !grants_read(rights) {
            continue;
        }
        let allow = matches!(kind, "A" | "XA" | "ZA");
        let deny = matches!(kind, "D" | "XD");
        if windows_noise(sid) {
            continue;
        }
        if deny {
            denied.insert(sid.to_string());
        } else if allow {
            if windows_everyone(sid) {
                access.everyone = true;
            } else {
                access.people.insert(sid.to_string());
            }
        }
    }
    access.people.retain(|p| !denied.contains(p));
    Some(access)
}

/// The text of one `O:`, `G:`, `D:` or `S:` section.
fn section<'a>(sddl: &'a str, tag: &str) -> Option<&'a str> {
    let start = find_section(sddl, tag)? + tag.len();
    let end = ["O:", "G:", "D:", "S:"]
        .iter()
        .filter_map(|t| find_section(&sddl[start..], t).map(|i| i + start))
        .min()
        .unwrap_or(sddl.len());
    Some(&sddl[start..end])
}

/// Where a section tag starts, outside any `( ... )` entry.
fn find_section(text: &str, tag: &str) -> Option<usize> {
    let mut depth = 0usize;
    let bytes = text.as_bytes();
    for i in 0..bytes.len() {
        match bytes[i] {
            b'(' => depth += 1,
            b')' => depth = depth.saturating_sub(1),
            _ if depth == 0 && text[i..].starts_with(tag) => return Some(i),
            _ => {}
        }
    }
    None
}

// ------------------------------------------------------------------------------------------
// macOS and Linux: owner, group and mode bits.

/// The groups on a Mac or Linux laptop, as far as the ownership rules need them.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct UnixGroups {
    /// Group ID to its name and the user IDs listed as members.
    pub groups: BTreeMap<u32, (String, BTreeSet<u32>)>,
    /// Each person's user ID to their name and main group.
    pub users: BTreeMap<u32, (String, u32)>,
    pub mac: bool,
}

impl UnixGroups {
    /// Reads `/etc/group` and `/etc/passwd` lines (`getent group` and `getent passwd` give the
    /// same shape).
    pub fn from_files(group: &str, passwd: &str, mac: bool) -> Self {
        let mut out = Self {
            mac,
            ..Self::default()
        };
        for line in passwd.lines() {
            let f: Vec<&str> = line.split(':').collect();
            if f.len() >= 4
                && let (Ok(uid), Ok(gid)) = (f[2].parse::<u32>(), f[3].parse::<u32>())
            {
                out.users.insert(uid, (f[0].to_string(), gid));
            }
        }
        for line in group.lines() {
            let f: Vec<&str> = line.split(':').collect();
            if f.len() >= 3
                && let Ok(gid) = f[2].parse::<u32>()
            {
                let members = f
                    .get(3)
                    .map(|m| {
                        m.split(',')
                            .filter_map(|name| {
                                out.users
                                    .iter()
                                    .find(|(_, (n, _))| n == name.trim())
                                    .map(|(uid, _)| *uid)
                            })
                            .collect()
                    })
                    .unwrap_or_default();
                out.groups.insert(gid, (f[0].to_string(), members));
            }
        }
        out
    }

    /// Everyone in group `gid`: its listed members and everyone whose main group it is.
    fn members(&self, gid: u32) -> BTreeSet<u32> {
        let mut m = self
            .groups
            .get(&gid)
            .map(|(_, members)| members.clone())
            .unwrap_or_default();
        m.extend(
            self.users
                .iter()
                .filter(|(_, (_, main))| *main == gid)
                .map(|(uid, _)| *uid),
        );
        m
    }

    fn name(&self, gid: u32) -> Option<&str> {
        self.groups.get(&gid).map(|(n, _)| n.as_str())
    }
}

/// Who could open a place with owner `uid`, group `gid` and permission bits `mode`. On a folder,
/// read means its list of names can be read.
pub fn access_from_mode(uid: u32, gid: u32, mode: u32, groups: &UnixGroups) -> Access {
    let mut access = Access {
        owner: Some(uid.to_string()),
        ..Access::default()
    };
    if mode & 0o004 != 0 {
        access.everyone = true;
    }
    if mode & 0o400 != 0 && uid != 0 {
        access.people.insert(uid.to_string());
    }
    if mode & 0o040 != 0 {
        let name = groups.name(gid).unwrap_or("");
        let admins = matches!(name, "admin" | "wheel" | "root" | "sudo" | "adm")
            || gid == 0
            || (groups.mac && gid == 80);
        let everyone_group = (groups.mac && (gid == 20 || name == "staff"))
            || matches!(name, "users" | "everyone")
            || (groups.mac && gid == 12);
        if everyone_group {
            access.everyone = true;
        } else if !admins {
            // A person's own private group (Linux makes one per person) means that person.
            let members = groups.members(gid);
            for m in members {
                if m != 0 {
                    access.people.insert(m.to_string());
                }
            }
            if groups
                .users
                .get(&uid)
                .is_some_and(|(n, main)| *main == gid && n == name)
            {
                access.people.insert(uid.to_string());
            }
        }
    }
    access
}
