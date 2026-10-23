//! Whose is it, without asking (Product Spec section 6; Engineering Plan 9.16). The ladder, in
//! order:
//!
//! 1. The rules file: system and app places are the system's; junk and app data inside a home
//!    stay that person's but are only counted; rebuildable and keep-whole places keep the owner
//!    the rest of the ladder gives them, as one counted item.
//! 2. Inside a person's home: that person's.
//! 3. On a drive that keeps no owners (FAT, exFAT, a network share): the mover's on a laptop with
//!    one person, otherwise shared.
//! 4. A laptop with one person: everything else is the mover's.
//! 5. With two or more people, who could open it decides before the owner: everyone means shared;
//!    one person (and administrators) means that person; a few named people means shared,
//!    limited to them. The owner the drive records decides only when nobody else could open
//!    it, and an owner that is no longer an account goes to "From an old account".
//!
//! Every answer carries the rule that gave it, so the plan and the report can say why.

use std::collections::BTreeSet;

use serde::{Deserialize, Serialize};

use crate::access::AccessFact;
use crate::rules::{Kind, Row};

impl Placement {
    pub fn new(whose: Whose, rule: Rule) -> Self {
        Self {
            whose,
            kind: None,
            rule,
        }
    }
}

/// Whose a place is.
#[derive(Debug, Clone, PartialEq, Eq, Hash, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(tag = "whose", rename_all = "kebab-case")]
pub enum Whose {
    /// The person signed in and running the move.
    Mover,
    /// Another person, by account ID. Their things are never listed by name for the mover.
    Person { account_id: String },
    /// Everyone's, or (when `limited_to` is not empty) shared by these people only.
    Shared { limited_to: Vec<String> },
    /// Left by an account that no longer exists: it goes to the approving administrator, in a
    /// folder called "From an old account".
    FromOldAccount { account_id: Option<String> },
    /// The system's or the installed programs'.
    System,
    /// Who could open it could not be read, so it can't be placed: listed as a blind spot.
    Undecided,
}

/// The rule that placed something.
#[derive(Debug, Clone, PartialEq, Eq, Hash, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(tag = "rule", rename_all = "kebab-case")]
pub enum Rule {
    /// A row of the rules file (`places.txt:12 node_modules`).
    RulesFile { row: String },
    /// One of the person's special folders, found through the system.
    KnownFolder,
    /// The system's own folders and files among people's things (the default profile, folder
    /// settings files, bins).
    SystemFolder,
    /// Inside a person's home folder.
    InsideHome,
    /// The laptop has one person.
    OnePersonLaptop,
    /// The drive keeps no owners.
    NoOwnerDrive,
    /// Everyone on the laptop could open it.
    EveryoneCanRead,
    /// Only one person (and administrators) could open it.
    OnlyOnePerson,
    /// A few named people could open it.
    NamedPeople,
    /// Nobody but its owner (and administrators) could open it.
    OwnerOnly,
    /// Its owner is no longer an account on this laptop.
    UnresolvableOwner,
    /// Who could open it could not be read.
    AccessNotReadable,
}

/// Where something went and why.
#[derive(Debug, Clone, PartialEq, Eq, Hash, PartialOrd, Ord, Serialize, Deserialize)]
pub struct Placement {
    #[serde(flatten)]
    pub whose: Whose,
    /// What the rules file says it is, when it says.
    pub kind: Option<Kind>,
    #[serde(flatten)]
    pub rule: Rule,
}

/// Whose home a place is in.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum HomeOf {
    Mover,
    Person(String),
    /// A home folder whose account is gone (by its owner's ID, when that could be read).
    Leftover(Option<String>),
}

/// The people the ladder knows about.
pub trait People {
    /// The mover's account ID.
    fn mover(&self) -> &str;
    /// Whether `id` is an account on this laptop (or a leftover the census knows).
    fn is_account(&self, id: &str) -> bool;
    /// Two or more people use this laptop.
    fn several(&self) -> bool;
}

/// What the ladder looks at for one place.
#[derive(Debug, Clone)]
pub struct Facts<'a> {
    pub rule: Option<&'a Row>,
    pub home: Option<HomeOf>,
    pub access: &'a AccessFact,
    /// Whose the folder around it is, for rebuildable and keep-whole places (which keep their
    /// surroundings' owner). `None` means decide it here.
    pub around: Option<&'a Placement>,
}

/// Places one thing.
pub fn place(facts: &Facts<'_>, people: &dyn People) -> Placement {
    // 1. The rules file.
    if let Some(row) = facts.rule {
        let rule = Rule::RulesFile { row: row.label() };
        let outside_homes = facts.home.is_none() && facts.around.is_none();
        let whose = match row.kind {
            Kind::System | Kind::App | Kind::Junk | Kind::OldSystem if outside_homes => {
                Whose::System
            }
            // Inside a person's things (a home, or a place already theirs): it stays theirs,
            // only counted.
            _ => match facts.around {
                Some(around) => around.whose.clone(),
                None => {
                    place(
                        &Facts {
                            rule: None,
                            ..facts.clone()
                        },
                        people,
                    )
                    .whose
                }
            },
        };
        return Placement {
            whose,
            kind: Some(row.kind),
            rule,
        };
    }
    if let Some(around) = facts.around {
        return Placement {
            kind: None,
            ..around.clone()
        };
    }
    // 2. Inside a home.
    if let Some(home) = &facts.home {
        let whose = match home {
            HomeOf::Mover => Whose::Mover,
            HomeOf::Person(id) => Whose::Person {
                account_id: id.clone(),
            },
            HomeOf::Leftover(id) => Whose::FromOldAccount {
                account_id: id.clone(),
            },
        };
        return Placement {
            whose,
            kind: None,
            rule: Rule::InsideHome,
        };
    }
    // 3. A drive that keeps no owners.
    if *facts.access == AccessFact::NoOwners {
        return Placement {
            whose: if people.several() {
                Whose::Shared {
                    limited_to: Vec::new(),
                }
            } else {
                Whose::Mover
            },
            kind: None,
            rule: Rule::NoOwnerDrive,
        };
    }
    // 4. One person.
    if !people.several() {
        return Placement {
            whose: Whose::Mover,
            kind: None,
            rule: Rule::OnePersonLaptop,
        };
    }
    // 5. Access first, then the owner.
    let AccessFact::Known(access) = facts.access else {
        return Placement {
            whose: Whose::Undecided,
            kind: None,
            rule: Rule::AccessNotReadable,
        };
    };
    if access.everyone {
        return Placement {
            whose: Whose::Shared {
                limited_to: Vec::new(),
            },
            kind: None,
            rule: Rule::EveryoneCanRead,
        };
    }
    let one = |id: &str, rule: Rule| -> Placement {
        let whose = if id == people.mover() {
            Whose::Mover
        } else if people.is_account(id) {
            Whose::Person {
                account_id: id.to_string(),
            }
        } else {
            return Placement {
                whose: Whose::FromOldAccount {
                    account_id: Some(id.to_string()),
                },
                kind: None,
                rule: Rule::UnresolvableOwner,
            };
        };
        Placement {
            whose,
            kind: None,
            rule,
        }
    };
    // Only people who exist read anything: a local group the system did not explain and an
    // account that is gone are left out, which can only make a place narrower (a thing never
    // lands where more people can open it than before).
    let readers: BTreeSet<&String> = access
        .people
        .iter()
        .filter(|id| id.as_str() == people.mover() || people.is_account(id))
        .collect();
    match readers.len() {
        0 => match &access.owner {
            Some(owner) if owner.starts_with("S-1-5-21-") || owner.starts_with("S-1-12-1-") => {
                one(owner, Rule::OwnerOnly)
            }
            Some(owner) if owner.parse::<u32>().is_ok_and(|uid| uid != 0) => {
                one(owner, Rule::OwnerOnly)
            }
            // Owned by the administrators or the system, and nobody else could open it: the
            // system's, never someone's "from an old account" (those are owners that are no
            // longer accounts; the administrators and the system are).
            Some(owner) if is_system_owner(owner) => Placement {
                whose: Whose::System,
                kind: None,
                rule: Rule::OwnerOnly,
            },
            // An owner that could not be read: nothing better is known.
            _ => Placement {
                whose: Whose::FromOldAccount { account_id: None },
                kind: None,
                rule: Rule::UnresolvableOwner,
            },
        },
        1 => one(
            readers.iter().next().map_or("", |s| s.as_str()),
            Rule::OnlyOnePerson,
        ),
        _ => Placement {
            whose: Whose::Shared {
                limited_to: readers.into_iter().cloned().collect(),
            },
            kind: None,
            rule: Rule::NamedPeople,
        },
    }
}

/// Whether `owner` (as an access list names it) is the administrators or the system itself, not
/// a person: `root` on macOS and Linux; on Windows the local system, local and network service,
/// the Administrators group and the system's own services (TrustedInstaller among them).
fn is_system_owner(owner: &str) -> bool {
    matches!(
        owner,
        "0" | "SY" | "BA" | "LS" | "NS" | "S-1-5-18" | "S-1-5-19" | "S-1-5-20" | "S-1-5-32-544"
    ) || owner.starts_with("S-1-5-80-")
}
