//! Where each file lands, worked out the same way on both laptops (Steps::landing: "it must be
//! the very function the old laptop sends with, so the old laptop never chooses a place").
//!
//! Only the new laptop knows its own folders, the names its person reads for a folder it lacks,
//! and the place it holds each other person's things in until the hand-over. So once the person
//! approves the plan, the new laptop sends those few facts ([`LandingRules`], one small message)
//! and the old laptop works out every file's place from them; the new laptop's allowance is made
//! from the very same rules, and anything announced elsewhere is refused there anyway.

use std::collections::BTreeMap;

use pctwin_record::{FolderRole, Item, Owner, Storage};
use pctwin_transfer::{
    Channel, LandingPlace, NewPlaces, TransferError, landing_for, role_label, shown_path,
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
            _ => {
                let places = NewPlaces::new(self.roles.clone());
                let fallback = |role: FolderRole| {
                    self.fallback
                        .get(&role)
                        .cloned()
                        .unwrap_or_else(|| format!("From your old laptop/{}", role_label(role)))
                };
                landing_for(item, &self.mover, &places, &fallback).place()
            }
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
    let bad = |why: &str| TransferError::Protocol(format!("landing rules: {why}"));
    let (&kind, body) = msg.split_first().ok_or_else(|| bad("empty"))?;
    if kind != KIND {
        return Err(bad("not the rules"));
    }
    if body.len() > MAX_RULES {
        return Err(bad("too long"));
    }
    serde_json::from_slice(body).map_err(|_| bad("not readable"))
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
}
