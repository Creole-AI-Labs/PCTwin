//! Where each file lands, worked out the same way on both laptops (Steps::landing: "it must be
//! the very function the old laptop sends with, so the old laptop never chooses a place").
//!
//! Only the new laptop knows its own folders, the names its person reads for a folder it lacks,
//! and the place it holds each other person's things in until the hand-over. So once the person
//! approves the plan, the new laptop sends those few facts ([`LandingRules`], one small message)
//! and the old laptop works out every file's place from them; the new laptop's allowance is made
//! from the very same rules, and anything announced elsewhere is refused there anyway.

use std::collections::BTreeMap;

use pctwin_record::{FolderRole, Inclusion, Item, ItemKind, Owner, Storage};
use pctwin_transfer::{
    Channel, LandingPlace, NewPlaces, TransferError, WholePlace, landing_for, role_label,
    shown_path,
};
use serde::{Deserialize, Serialize};

/// The message kind of the rules, kept apart from the transfer's (1 to 20), the inventory's (32,
/// 33, 35), the version's (34) and the link test's (36, 37).
const KIND: u8 = 39;
/// The message kind of the old laptop's word on who is moving.
const MOVER: u8 = 40;
/// The message kind of the old laptop's word on the other people moving: their names.
const PEOPLE: u8 = 41;
/// The longest account ID said (as the record allows).
const MAX_ACCOUNT_ID: usize = 256;
/// The most other people one move names (a laptop has a handful of accounts).
pub const MAX_PEOPLE: usize = 64;
/// The longest name said for a person, in bytes.
const MAX_NAME: usize = 256;
/// The rules are a handful of facts: far below the link's 64 KiB message limit.
const MAX_RULES: usize = 60 * 1024;

/// What the old laptop needs to place every file exactly as the new laptop will check it.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct LandingRules {
    /// The person moving: their account ID on the old laptop.
    pub mover: String,
    /// The new laptop's own special folders, and where each really is.
    pub roles: BTreeMap<FolderRole, Storage>,
    /// For a role the new laptop lacks, the folder inside the home that stands for it, in the
    /// person's language ("From your old laptop/Music").
    pub fallback: BTreeMap<FolderRole, String>,
    /// Each other approved person (by their account ID on the old laptop) to the place their
    /// things are held in until the hand-over.
    pub held: BTreeMap<String, String>,
    /// The mover's (and shared) folders, by role, the person chose to leave out at approval:
    /// none of their files is sent.
    #[serde(default)]
    pub left_out: std::collections::BTreeSet<FolderRole>,
    /// The mover's (and shared) folders, by role, the person sent to a folder they chose on the
    /// new laptop (M05): each lands in the place [`chosen_label`] names, approved there.
    #[serde(default)]
    pub chosen: std::collections::BTreeSet<FolderRole>,
    /// Several old laptops into one (Task List 2.6): the folder, in each of the new laptop's
    /// places, that this old laptop's mover's (and shared) files land in, in the person's
    /// language ("From Bea's laptop"), so they never mix with an earlier old laptop's. One plain
    /// folder name ([`own_folder_fine`]). Move protocol 7 and later
    /// ([`pctwin_transfer::OWN_FOLDER_SINCE`]); never sent to an older laptop.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub own_folder: Option<String>,
}

/// The longest own-folder name, in bytes.
pub const MAX_OWN_FOLDER: usize = 120;

/// Whether `name` may be an old laptop's own folder: one plain folder name (no folder marks,
/// nothing a system reads as special), as fallback names are checked. Every system's own name
/// rules are still checked by the safety gate where it lands.
pub fn own_folder_fine(name: &str) -> bool {
    let trimmed = name.trim();
    trimmed == name
        && !name.is_empty()
        && name.len() <= MAX_OWN_FOLDER
        && name != "."
        && name != ".."
        && !name.ends_with('.')
        && !name.chars().any(|c| {
            c.is_control() || matches!(c, '/' | '\\' | ':' | '*' | '?' | '"' | '<' | '>' | '|')
        })
}

impl LandingRules {
    /// The rules as sent to a laptop speaking move protocol `version`: one before
    /// [`pctwin_transfer::OWN_FOLDER_SINCE`] cannot read an own folder, so none is named (its
    /// files land as they always did).
    pub fn for_version(mut self, version: u16) -> Self {
        if version < pctwin_transfer::OWN_FOLDER_SINCE {
            self.own_folder = None;
        }
        self
    }
}

/// The most names tried for one folder that moves as one whole ("Name (2)" to "Name (100)").
pub const MOST_WHOLE_NAMES: u32 = 100;

/// Where every folder that moves as one whole ([`ItemKind::Package`] in `items`, placed by
/// `rules`) is stored on this laptop. `meet(destination, path)` says how this laptop would store
/// a path there (its key: names converted and folded to the drive's case) and whether anything is
/// already there (see `pctwin_gate::Destination::meet`); `None` for a place not known here.
///
/// A whole an earlier move from this same old laptop already started or finished here carries on
/// in its own folder (gap 3, 28 October 2026): `earlier(destination)` lists the folders earlier
/// moves from this old laptop made there (the journal's record,
/// `pctwin_journal::Ledger::folders_made_from`), and one of this whole's names is its own only
/// when the journal says the write of one of this very whole's files made it and
/// `still(destination, path, identity)` finds that very folder there now. Its files that are
/// already there and the same are then found, not copied twice; the rest are copied into it. A
/// folder of the same name that anyone else made, or that was put in its place since, is never
/// taken as its own.
///
/// Otherwise each whole keeps its planned path unless that meets something already there or
/// another whole of this move by its key, else takes "Name (2)" and on, up to
/// [`MOST_WHOLE_NAMES`]; past that it has no place (its files are not received, each said with
/// why). Decided on this laptop alone: the old laptop announces every file at its planned path,
/// so nothing of it crosses, however many there are.
pub fn place_wholes(
    items: &[Item],
    rules: &LandingRules,
    meet: impl Fn(&str, &str) -> Option<(String, bool)>,
    earlier: impl Fn(&str) -> Vec<pctwin_journal::EarlierFolder>,
    still: impl Fn(&str, &str, pctwin_journal::FileId) -> bool,
) -> Vec<WholePlace> {
    let mut claimed: std::collections::HashSet<(String, String)> = Default::default();
    let mut made_before: BTreeMap<String, Vec<pctwin_journal::EarlierFolder>> = BTreeMap::new();
    let by_id: std::collections::HashMap<pctwin_record::ItemId, &Item> =
        items.iter().map(|i| (i.id, i)).collect();
    // Whether `file` is inside the whole `whole` on the old laptop.
    let inside = |file: &Item, whole: &Item| {
        let (f, w) = (file.path.parts(), whole.path.parts());
        file.owner == whole.owner
            && file.place == whole.place
            && f.len() > w.len()
            && f[..w.len()] == *w
    };
    let mut placed = Vec::new();
    for item in items {
        if item.kind != ItemKind::Package || item.inclusion != Inclusion::Included {
            continue;
        }
        let Some(place) = rules.landing(item) else {
            continue;
        };
        let (parent, name) = match place.path.rsplit_once('/') {
            Some((parent, name)) => (format!("{parent}/"), name.to_string()),
            None => (String::new(), place.path.clone()),
        };
        let (stem, ext) = match name.rsplit_once('.') {
            Some((stem, ext)) if !stem.is_empty() => (stem.to_string(), format!(".{ext}")),
            _ => (name.clone(), String::new()),
        };
        let candidates: Vec<String> = std::iter::once(place.path.clone())
            .chain((2..=MOST_WHOLE_NAMES).map(|n| format!("{parent}{stem} ({n}){ext}")))
            .collect();
        let before = made_before
            .entry(place.destination.clone())
            .or_insert_with(|| earlier(&place.destination));
        // Its own folder from an earlier move from this old laptop, first.
        let mut stored = None;
        let mut adopted = None;
        for candidate in &candidates {
            let Some(id) = before
                .iter()
                .filter(|e| e.folder == *candidate)
                .filter(|e| by_id.get(&e.item).is_some_and(|f| inside(f, item)))
                .find_map(|e| e.id)
            else {
                continue;
            };
            let Some((key, taken)) = meet(&place.destination, candidate) else {
                break;
            };
            let key = (place.destination.clone(), key);
            if taken && !claimed.contains(&key) && still(&place.destination, candidate, id) {
                claimed.insert(key);
                stored = Some(candidate.clone());
                adopted = Some(id);
                break;
            }
        }
        if stored.is_none() {
            for candidate in candidates {
                let Some((key, taken)) = meet(&place.destination, &candidate) else {
                    break;
                };
                let key = (place.destination.clone(), key);
                if !taken && !claimed.contains(&key) {
                    claimed.insert(key);
                    stored = Some(candidate);
                    break;
                }
            }
        }
        placed.push(WholePlace {
            destination: place.destination,
            planned: place.path,
            stored,
            adopted,
        });
    }
    placed
}

/// The label of the place the person chose for the folders of `role`: never one of the new
/// laptop's own folders' labels.
pub fn chosen_label(role: FolderRole) -> String {
    format!("chosen-{}", role_label(role))
}

impl LandingRules {
    /// Where `item` lands (`None`: it is not received here): the mover's and the shared things
    /// by the new laptop's folders ([`landing_for`]); another approved person's in their holding
    /// place, one folder per place label (`<label>/<path>`), as the hand-over reads it.
    pub fn landing(&self, item: &Item) -> Option<LandingPlace> {
        match &item.owner {
            Owner::Person { account_id } if *account_id != self.mover => {
                let label = self.held.get(account_id)?;
                Some(LandingPlace::new(
                    label.clone(),
                    format!("{}/{}", role_label(item.place.role), shown_path(&item.path)),
                ))
            }
            Owner::System => None,
            _ if self.left_out.contains(&item.place.role) => None,
            _ if self.chosen.contains(&item.place.role) => Some(self.in_own_folder(
                LandingPlace::new(chosen_label(item.place.role), shown_path(&item.path)),
            )),
            _ => {
                let places = NewPlaces::new(self.roles.clone());
                let fallback = |role: FolderRole| {
                    self.fallback
                        .get(&role)
                        .cloned()
                        .unwrap_or_else(|| format!("From your old laptop/{}", role_label(role)))
                };
                landing_for(item, &self.mover, &places, &fallback)
                    .place()
                    .map(|p| self.in_own_folder(p))
            }
        }
    }

    /// `place`, inside this old laptop's own folder when the rules name one.
    fn in_own_folder(&self, place: LandingPlace) -> LandingPlace {
        match &self.own_folder {
            Some(own) => LandingPlace::new(place.destination, format!("{own}/{}", place.path)),
            None => place,
        }
    }
}

/// The old laptop: says which of its people is moving (their account ID), right after the link
/// test, so the new laptop can place everyone before the person approves.
pub async fn send_mover(ch: &mut impl Channel, account_id: &str) -> Result<(), TransferError> {
    if account_id.is_empty() || account_id.len() > MAX_ACCOUNT_ID {
        return Err(TransferError::Protocol("not an account ID".into()));
    }
    let mut msg = vec![MOVER];
    msg.extend_from_slice(account_id.as_bytes());
    ch.send(&msg)
        .await
        .map_err(|_| TransferError::ConnectionDropped)
}

/// Another person whose things are in the move: their account ID on the old laptop and the name
/// their account shows there, so the new laptop can ask where their things go. Nothing of their
/// files is in it.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct NamedPerson {
    pub account_id: String,
    pub name: String,
}

fn fine_text(text: &str, max: usize) -> bool {
    !text.is_empty() && text.len() <= max && !text.chars().any(char::is_control)
}

/// The old laptop: names the other people whose things are in the move (none: an empty list),
/// right after saying who is moving.
pub async fn send_people(
    ch: &mut impl Channel,
    people: &[NamedPerson],
) -> Result<(), TransferError> {
    let fine = people.len() <= MAX_PEOPLE
        && people
            .iter()
            .all(|p| fine_text(&p.account_id, MAX_ACCOUNT_ID) && fine_text(&p.name, MAX_NAME));
    if !fine {
        return Err(TransferError::Protocol("not a list of people".into()));
    }
    let body = serde_json::to_vec(people)
        .map_err(|_| TransferError::Protocol("the people could not be written".into()))?;
    let mut msg = Vec::with_capacity(body.len() + 1);
    msg.push(PEOPLE);
    msg.extend_from_slice(&body);
    ch.send(&msg)
        .await
        .map_err(|_| TransferError::ConnectionDropped)
}

/// The new laptop: the other people, as the old laptop named them. Anything else, too many,
/// a name with control characters or one account named twice is refused.
pub async fn receive_people(ch: &mut impl Channel) -> Result<Vec<NamedPerson>, TransferError> {
    let msg = ch
        .recv()
        .await
        .map_err(|_| TransferError::ConnectionDropped)?;
    parse_people(&msg)
}

/// The other people, read from one message; anything else is refused.
pub fn parse_people(msg: &[u8]) -> Result<Vec<NamedPerson>, TransferError> {
    let bad = || TransferError::Protocol("expected the other people".into());
    let (&kind, body) = msg.split_first().ok_or_else(bad)?;
    if kind != PEOPLE {
        return Err(bad());
    }
    let people: Vec<NamedPerson> = serde_json::from_slice(body).map_err(|_| bad())?;
    let mut seen = std::collections::BTreeSet::new();
    let fine = people.len() <= MAX_PEOPLE
        && people.iter().all(|p| {
            fine_text(&p.account_id, MAX_ACCOUNT_ID)
                && fine_text(&p.name, MAX_NAME)
                && seen.insert(p.account_id.as_str())
        });
    if fine { Ok(people) } else { Err(bad()) }
}

/// The new laptop: who is moving, as the old laptop said; anything else is refused.
pub async fn receive_mover(ch: &mut impl Channel) -> Result<String, TransferError> {
    let msg = ch
        .recv()
        .await
        .map_err(|_| TransferError::ConnectionDropped)?;
    parse_mover(&msg)
}

/// The old laptop's word on who is moving, read from one message; anything else is refused.
pub fn parse_mover(msg: &[u8]) -> Result<String, TransferError> {
    let bad = || TransferError::Protocol("expected who is moving".into());
    let (&kind, body) = msg.split_first().ok_or_else(bad)?;
    if kind != MOVER || body.is_empty() || body.len() > MAX_ACCOUNT_ID {
        return Err(bad());
    }
    let id = std::str::from_utf8(body).map_err(|_| bad())?;
    if id.chars().any(char::is_control) {
        return Err(bad());
    }
    Ok(id.to_string())
}

/// The new laptop: sends the rules, once the person approved the plan.
pub async fn send_rules(ch: &mut impl Channel, rules: &LandingRules) -> Result<(), TransferError> {
    let body = serde_json::to_vec(rules)
        .map_err(|_| TransferError::Protocol("the landing rules could not be written".into()))?;
    if body.len() > MAX_RULES {
        return Err(TransferError::Protocol(
            "the landing rules are too long".into(),
        ));
    }
    let mut msg = Vec::with_capacity(body.len() + 1);
    msg.push(KIND);
    msg.extend_from_slice(&body);
    ch.send(&msg)
        .await
        .map_err(|_| TransferError::ConnectionDropped)
}

/// The old laptop: the rules, as the new laptop sent them; anything else is refused.
pub async fn receive_rules(ch: &mut impl Channel) -> Result<LandingRules, TransferError> {
    let msg = ch
        .recv()
        .await
        .map_err(|_| TransferError::ConnectionDropped)?;
    parse_rules(&msg)
}

/// The landing rules, read from one message; anything else is refused.
pub fn parse_rules(msg: &[u8]) -> Result<LandingRules, TransferError> {
    let bad = |why: &str| TransferError::Protocol(format!("landing rules: {why}"));
    let (&kind, body) = msg.split_first().ok_or_else(|| bad("empty"))?;
    if kind != KIND {
        return Err(bad("not the rules"));
    }
    if body.len() > MAX_RULES {
        return Err(bad("too long"));
    }
    let rules: LandingRules = serde_json::from_slice(body).map_err(|_| bad("not readable"))?;
    if rules
        .own_folder
        .as_deref()
        .is_some_and(|o| !own_folder_fine(o))
    {
        return Err(bad("not a folder name"));
    }
    Ok(rules)
}

#[cfg(test)]
mod tests {
    use super::*;
    use pctwin_record::{
        Inclusion, ItemId, ItemKind, ItemName, ItemPath, LaptopId, ManagedBy, Place, Portability,
    };

    fn item(owner: Owner, role: FolderRole, path: &[&str]) -> Item {
        let laptop = LaptopId::from_hex("00112233445566778899aabbccddeeff").unwrap();
        let place = Place {
            role,
            storage: Storage::SystemDrive,
        };
        let path = ItemPath::new(path.iter().map(|p| ItemName::from_text(p)).collect()).unwrap();
        Item {
            id: ItemId::derive(&laptop, &owner, &place, &path),
            kind: ItemKind::File,
            owner,
            place,
            path,
            size_bytes: 1,
            modified_ns: None,
            portability: Portability::Portable,
            managed_by: ManagedBy::Personal,
            download_bytes: None,
            inclusion: Inclusion::Included,
        }
    }

    fn rules() -> LandingRules {
        LandingRules {
            mover: "1000".into(),
            roles: BTreeMap::from([
                (FolderRole::Documents, Storage::SystemDrive),
                (FolderRole::Home, Storage::SystemDrive),
                (FolderRole::Public, Storage::SystemDrive),
            ]),
            fallback: BTreeMap::from([(FolderRole::Music, "Desde tu portátil/Música".into())]),
            held: BTreeMap::from([("1001".into(), "for-1".into())]),
            left_out: Default::default(),
            chosen: Default::default(),
            own_folder: None,
        }
    }

    #[test]
    fn a_folder_sent_to_a_place_the_person_chose_lands_there_and_nothing_else_moves() {
        let mut r = rules();
        r.chosen.insert(FolderRole::Pictures);
        let mine = |r: &LandingRules, role, path: &[&str]| {
            r.landing(&item(
                Owner::Person {
                    account_id: "1000".into(),
                },
                role,
                path,
            ))
        };
        assert_eq!(
            mine(&r, FolderRole::Pictures, &["Trip", "a.jpg"]),
            Some(LandingPlace::new(
                chosen_label(FolderRole::Pictures),
                "Trip/a.jpg"
            ))
        );
        assert_eq!(
            mine(&r, FolderRole::Documents, &["b.txt"]),
            Some(LandingPlace::new("documents", "b.txt"))
        );
        // Another person's pictures still wait in their holding place.
        let bea = item(
            Owner::Person {
                account_id: "1001".into(),
            },
            FolderRole::Pictures,
            &["c.jpg"],
        );
        assert_eq!(
            r.landing(&bea),
            Some(LandingPlace::new("for-1", "pictures/c.jpg"))
        );
        // Left out wins: a folder left out is not sent anywhere.
        r.left_out.insert(FolderRole::Pictures);
        assert_eq!(mine(&r, FolderRole::Pictures, &["d.jpg"]), None);
        // The label is never one of the new laptop's own folders.
        for role in [
            FolderRole::Pictures,
            FolderRole::Documents,
            FolderRole::Home,
        ] {
            assert_ne!(chosen_label(role), role_label(role));
        }
    }

    #[test]
    fn each_kind_of_owner_lands_where_the_new_laptop_checks_it() {
        let r = rules();
        let mine = |role, path: &[&str]| {
            r.landing(&item(
                Owner::Person {
                    account_id: "1000".into(),
                },
                role,
                path,
            ))
        };
        assert_eq!(
            mine(FolderRole::Documents, &["Letters", "a.txt"]),
            Some(LandingPlace::new("documents", "Letters/a.txt"))
        );
        // A folder the new laptop lacks: in the home, under the person's own words.
        assert_eq!(
            mine(FolderRole::Music, &["song.mp3"]),
            Some(LandingPlace::new(
                "home",
                "Desde tu portátil/Música/song.mp3"
            ))
        );
        // Another approved person: in their holding place, under the place label.
        let bea = item(
            Owner::Person {
                account_id: "1001".into(),
            },
            FolderRole::Pictures,
            &["beach.jpg"],
        );
        assert_eq!(
            r.landing(&bea),
            Some(LandingPlace::new("for-1", "pictures/beach.jpg"))
        );
        // A person not approved, and the system's own: nowhere.
        let carl = item(
            Owner::Person {
                account_id: "1002".into(),
            },
            FolderRole::Documents,
            &["x.txt"],
        );
        assert_eq!(r.landing(&carl), None);
        assert_eq!(
            r.landing(&item(Owner::System, FolderRole::Other, &["y"])),
            None
        );
        // Shared things go to the shared folder.
        assert_eq!(
            r.landing(&item(Owner::Shared, FolderRole::Other, &["menu.txt"])),
            Some(LandingPlace::new("public", "menu.txt"))
        );
    }

    #[test]
    fn a_folder_the_person_left_out_gets_no_place_but_other_people_keep_theirs() {
        let mut r = rules();
        r.left_out.insert(FolderRole::Documents);
        let mine = item(
            Owner::Person {
                account_id: "1000".into(),
            },
            FolderRole::Documents,
            &["a.txt"],
        );
        assert_eq!(r.landing(&mine), None);
        let pictures = item(
            Owner::Person {
                account_id: "1000".into(),
            },
            FolderRole::Pictures,
            &["b.jpg"],
        );
        assert!(r.landing(&pictures).is_some());
        let bea = item(
            Owner::Person {
                account_id: "1001".into(),
            },
            FolderRole::Documents,
            &["c.txt"],
        );
        assert_eq!(
            r.landing(&bea),
            Some(LandingPlace::new("for-1", "documents/c.txt"))
        );
        // Rules from a laptop that says nothing about it leave nothing out.
        let without: LandingRules =
            serde_json::from_str(r#"{"mover":"1000","roles":{},"fallback":{},"held":{}}"#).unwrap();
        assert!(without.left_out.is_empty());
    }

    /// Several old laptops into one (Task List 2.6): with an own folder named, every file of the
    /// mover's and the shared ones lands inside it in its place; another person's held things do
    /// not; an older laptop is never sent one, and a name that is not one plain folder is refused.
    #[test]
    fn an_old_laptops_own_folder_holds_its_movers_files_only() {
        let mut r = rules();
        r.own_folder = Some("From Bea's laptop".into());
        let mine = item(
            Owner::Person {
                account_id: "1000".into(),
            },
            FolderRole::Documents,
            &["Work", "a.txt"],
        );
        let plain = rules().landing(&mine).unwrap();
        assert_eq!(
            r.landing(&mine),
            Some(LandingPlace::new(
                plain.destination.clone(),
                format!("From Bea's laptop/{}", plain.path)
            ))
        );
        let shared = item(Owner::Shared, FolderRole::Other, &["menu.txt"]);
        assert_eq!(
            r.landing(&shared),
            Some(LandingPlace::new("public", "From Bea's laptop/menu.txt"))
        );
        let bea = item(
            Owner::Person {
                account_id: "1001".into(),
            },
            FolderRole::Documents,
            &["c.txt"],
        );
        assert_eq!(
            r.landing(&bea),
            Some(LandingPlace::new("for-1", "documents/c.txt"))
        );
        // A place the person chose holds it too.
        let mut chose = r.clone();
        chose.chosen.insert(FolderRole::Documents);
        assert_eq!(
            chose.landing(&mine),
            Some(LandingPlace::new(
                chosen_label(FolderRole::Documents),
                "From Bea's laptop/Work/a.txt"
            ))
        );
        // Left out stays left out.
        let mut out = r.clone();
        out.left_out.insert(FolderRole::Documents);
        assert_eq!(out.landing(&mine), None);
        // Version 7 and later read it; an older laptop is never sent one, so it lands as always.
        assert_eq!(
            r.clone()
                .for_version(pctwin_transfer::OWN_FOLDER_SINCE)
                .own_folder,
            r.own_folder
        );
        let older = r.clone().for_version(pctwin_transfer::OWN_FOLDER_SINCE - 1);
        assert_eq!(older.own_folder, None);
        assert_eq!(older.landing(&mine), Some(plain));
        // Without one, the rules are written exactly as before (an older laptop reads them).
        let json = serde_json::to_string(&rules()).unwrap();
        assert!(!json.contains("own_folder"), "{json}");
        // Read back, and only a plain folder name is taken.
        let msg = |own: &str| {
            let mut r = rules();
            r.own_folder = Some(own.into());
            let mut m = vec![KIND];
            m.extend_from_slice(&serde_json::to_vec(&r).unwrap());
            m
        };
        assert_eq!(
            parse_rules(&msg("From Bea's laptop"))
                .unwrap()
                .own_folder
                .as_deref(),
            Some("From Bea's laptop")
        );
        for bad in [
            "", " From", "From ", ".", "..", "a/b", "a\\b", "C:", "x.", "a\u{0}b", "a*b", "a?b",
            "a\"b", "a<b", "a>b", "a|b",
        ] {
            assert!(!own_folder_fine(bad), "{bad:?}");
            assert!(parse_rules(&msg(bad)).is_err(), "{bad:?}");
        }
        assert!(own_folder_fine(&"x".repeat(MAX_OWN_FOLDER)));
        assert!(!own_folder_fine(&"x".repeat(MAX_OWN_FOLDER + 1)));
        assert!(own_folder_fine("De l’ordinateur de Bea"));
    }

    #[tokio::test]
    async fn the_rules_cross_whole_and_anything_else_is_refused() {
        use tokio::sync::mpsc;
        struct Mem {
            tx: mpsc::UnboundedSender<Vec<u8>>,
            rx: mpsc::UnboundedReceiver<Vec<u8>>,
        }
        impl Channel for Mem {
            async fn send(&mut self, data: &[u8]) -> Result<(), pctwin_transfer::ChannelError> {
                self.tx
                    .send(data.to_vec())
                    .map_err(|_| pctwin_transfer::ChannelError)
            }
            async fn recv(&mut self) -> Result<Vec<u8>, pctwin_transfer::ChannelError> {
                self.rx.recv().await.ok_or(pctwin_transfer::ChannelError)
            }
        }
        let (a_tx, b_rx) = mpsc::unbounded_channel();
        let (b_tx, a_rx) = mpsc::unbounded_channel();
        let (mut a, mut b) = (Mem { tx: a_tx, rx: a_rx }, Mem { tx: b_tx, rx: b_rx });
        send_rules(&mut a, &rules()).await.unwrap();
        assert_eq!(receive_rules(&mut b).await.unwrap(), rules());
        // Not the rules, or rules with a field PCTwin does not write: refused.
        a.send(&[KIND, b'{', b'}']).await.unwrap();
        assert!(receive_rules(&mut b).await.is_err());
        a.send(&[1, 2, 3]).await.unwrap();
        assert!(receive_rules(&mut b).await.is_err());
        let mut odd = vec![KIND];
        odd.extend_from_slice(br#"{"mover":"1","roles":{},"fallback":{},"held":{},"extra":1}"#);
        a.send(&odd).await.unwrap();
        assert!(receive_rules(&mut b).await.is_err());

        // The other people: named whole, and anything odd refused on both sides.
        let bea = NamedPerson {
            account_id: "1001".into(),
            name: "Bea".into(),
        };
        send_people(&mut a, std::slice::from_ref(&bea))
            .await
            .unwrap();
        assert_eq!(receive_people(&mut b).await.unwrap(), vec![bea.clone()]);
        send_people(&mut a, &[]).await.unwrap();
        assert_eq!(receive_people(&mut b).await.unwrap(), vec![]);
        let control = NamedPerson {
            account_id: "1002".into(),
            name: "Be\na".into(),
        };
        assert!(send_people(&mut a, &[control]).await.is_err());
        let mut twice = vec![PEOPLE];
        twice
            .extend_from_slice(br#"[{"account_id":"1","name":"A"},{"account_id":"1","name":"B"}]"#);
        a.send(&twice).await.unwrap();
        assert!(receive_people(&mut b).await.is_err());
        let mut tagged = vec![PEOPLE];
        tagged.extend_from_slice(br#"[{"account_id":"1","name":"A","files":["x"]}]"#);
        a.send(&tagged).await.unwrap();
        assert!(receive_people(&mut b).await.is_err());
        let many: Vec<NamedPerson> = (0..=MAX_PEOPLE)
            .map(|n| NamedPerson {
                account_id: n.to_string(),
                name: "P".into(),
            })
            .collect();
        assert!(send_people(&mut a, &many).await.is_err());
    }

    /// Every whole of a move is placed against what is already here AND every other whole of
    /// the move, by the name this laptop would store it under: a renamed whole never lands on
    /// another one's planned name (A), and two names this drive takes as one (`X.app`, `x.app`
    /// where case is not told apart) never become one folder (B).
    #[test]
    fn wholes_are_placed_against_what_is_here_and_each_other() {
        let me = || Owner::Person {
            account_id: "1000".into(),
        };
        let whole = |path: &[&str]| {
            let mut i = item(me(), FolderRole::Documents, path);
            i.kind = ItemKind::Package;
            i.size_bytes = 0;
            i
        };
        let items = vec![
            whole(&["Photos.photoslibrary"]),
            whole(&["Photos (2).photoslibrary"]),
            whole(&["X.app"]),
            whole(&["x.app"]),
            whole(&["proj", ".git"]),
            whole(&["Trip.pages"]),
        ];
        let here = ["Photos.photoslibrary", "proj/.git"];
        // A drive that does not tell case apart: keys folded to one case.
        let meet =
            |d: &str, p: &str| (d == "documents").then(|| (p.to_lowercase(), here.contains(&p)));
        let placed = place_wholes(&items, &rules(), meet, |_| Vec::new(), |_, _, _| false);
        let stored: Vec<Option<&str>> = placed.iter().map(|w| w.stored.as_deref()).collect();
        assert_eq!(
            stored,
            [
                Some("Photos (2).photoslibrary"),
                Some("Photos (2) (2).photoslibrary"),
                Some("X.app"),
                Some("x (2).app"),
                Some("proj/.git (2)"),
                Some("Trip.pages"),
            ]
        );
        assert!(placed.iter().all(|w| w.destination == "documents"));
        // Where case is told apart, X.app and x.app are two folders, each under its own name.
        let apart =
            |d: &str, p: &str| (d == "documents").then(|| (p.to_string(), here.contains(&p)));
        let placed = place_wholes(&items, &rules(), apart, |_| Vec::new(), |_, _, _| false);
        assert_eq!(placed[3].stored.as_deref(), Some("x.app"));
    }

    /// However many wholes meet something already here, the move still starts: each gets a name
    /// of its own on this laptop (nothing of it is said to the old laptop), and one that can be
    /// given none is not received, said with why, never merged.
    #[test]
    fn a_thousand_wholes_already_here_each_land_apart_and_one_with_no_free_name_has_no_place() {
        let me = || Owner::Person {
            account_id: "1000".into(),
        };
        let items: Vec<Item> = (0..1000)
            .map(|n| {
                let name = format!("project {n}");
                let mut i = item(me(), FolderRole::Documents, &[&name, ".git"]);
                i.kind = ItemKind::Package;
                i.size_bytes = 0;
                i
            })
            .collect();
        let placed = place_wholes(
            &items,
            &rules(),
            |_, p: &str| Some((p.to_string(), p.ends_with("/.git"))),
            |_| Vec::new(),
            |_, _, _| false,
        );
        assert_eq!(placed.len(), 1000);
        assert!(
            placed
                .iter()
                .all(|w| w.stored.as_deref() == Some(&*format!("{} (2)", w.planned)))
        );
        // The rules said to the old laptop stay as small as ever.
        assert!(serde_json::to_vec(&rules()).unwrap().len() < 1024);
        // Every name taken: no place, never the one already there.
        let none = place_wholes(
            &items[..1],
            &rules(),
            |_, p: &str| Some((p.to_string(), true)),
            |_| Vec::new(),
            |_, _, _| false,
        );
        assert_eq!(none[0].stored, None);
    }
}
