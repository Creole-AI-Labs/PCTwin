//! Saved Wi-Fi networks in the full move (Task List 2.4, decided 20 October 2026; move protocol
//! 8): the old laptop lists them by name, sends only the ticked ones that were read, with their
//! passwords, in one record the new laptop reads strictly against that list; every network ends
//! copied, needs-action with why, or skipped-by-user, and one already saved is never added over.

use pctwin_fullmove::extras::{Extras, ExtrasError};
use pctwin_fullmove::wifi::{
    NotAdded, RecordError, WifiDone, WifiHeld, WifiHere, WifiKind, WifiListed, WifiWhy,
    add_networks, decode_record, listed_ok, offered, parse_profiles,
};
use pctwin_settings::wifi::{Network, Secret, Security};
use pctwin_types::{ItemStatus, Reason};
use zeroize::Zeroizing;

fn net(ssid: &str, pw: Option<&str>) -> Network {
    Network {
        name: ssid.into(),
        ssid: ssid.into(),
        hidden: ssid.starts_with('h'),
        auto_connect: true,
        security: match pw {
            Some(_) => Security::Personal {
                auth: "WPA2PSK".into(),
                encryption: "AES".into(),
            },
            None => Security::Open,
        },
        secret: pw.map(|p| Secret::new(Zeroizing::new(p.into()))),
    }
}

fn org(ssid: &str) -> Network {
    Network {
        security: Security::Enterprise,
        ..net(ssid, None)
    }
}

/// The networks found on the old laptop (without passwords), and the same read with them.
fn found() -> Vec<Network> {
    vec![
        net("Home", Some("x")),
        net("Cafe", None),
        org("Office"),
        net("hidden-net", Some("x")),
        net("Gran", Some("x")),
    ]
}

fn read_all() -> Vec<Network> {
    vec![
        net("Home", Some("correct horse")),
        net("Cafe", None),
        net("hidden-net", Some("battery staple")),
        net("Gran", Some("never ticked pw")),
    ]
}

#[test]
fn only_ticked_networks_that_were_read_cross_and_they_cross_exactly() {
    let offer = offered(&found());
    assert_eq!(
        offer.iter().map(|(_, k)| *k).collect::<Vec<_>>(),
        [
            WifiKind::Password,
            WifiKind::Open,
            WifiKind::Organisation,
            WifiKind::Password,
            WifiKind::Password
        ]
    );
    // Home, Cafe and the hidden one ticked; Office cannot be (an organisation's); Gran not.
    let held = WifiHeld::new(&offer, &[0, 1, 2, 3], read_all());
    let listed = held.listed().to_vec();
    assert!(listed_ok(&listed));
    let ticked: Vec<bool> = listed.iter().map(|l| l.ticked).collect();
    assert_eq!(ticked, [true, true, false, true, false]);
    let read: Vec<bool> = listed.iter().map(|l| l.read).collect();
    assert_eq!(read, [true, true, false, true, false]);
    let record = held.record();
    // Gran's password was read here but never ticked: it is not in the record.
    assert!(
        !record
            .windows(b"never ticked pw".len())
            .any(|w| w == b"never ticked pw")
    );
    let got = decode_record(&record, &listed).unwrap();
    let names: Vec<(usize, &str)> = got.iter().map(|(at, n)| (*at, n.ssid.as_str())).collect();
    assert_eq!(names, [(0, "Home"), (1, "Cafe"), (3, "hidden-net")]);
    let pw: Vec<Option<Vec<u8>>> = got
        .iter()
        .map(|(_, n)| n.secret.as_ref().map(|s| s.to_session().to_vec()))
        .collect();
    assert_eq!(
        pw,
        [
            Some(b"correct horse".to_vec()),
            None,
            Some(b"battery staple".to_vec())
        ]
    );
    assert!(got[2].1.hidden && !got[0].1.hidden && got[0].1.auto_connect);
    assert_eq!(got[1].1.security, Security::Open);
    // Never printed.
    assert!(!format!("{held:?}").contains("correct horse"));
    // Nothing ticked: an empty record.
    let none = WifiHeld::new(&offer, &[], read_all());
    assert!(
        decode_record(&none.record(), none.listed())
            .unwrap()
            .is_empty()
    );
}

#[test]
fn a_password_wifi_does_not_allow_is_not_sent() {
    let offer = offered(&[net("Home", Some("x")), net("Hex", Some("x"))]);
    let read = vec![
        net("Home", Some("short")),
        net("Hex", Some(&"ab".repeat(32))),
    ];
    let held = WifiHeld::new(&offer, &[0, 1], read);
    assert!(!held.listed()[0].read, "fewer than 8 characters");
    assert!(held.listed()[1].read, "64 hex digits");
}

fn sample() -> (Vec<WifiListed>, Zeroizing<Vec<u8>>) {
    let held = WifiHeld::new(&offered(&found()), &[0, 1, 3], read_all());
    (held.listed().to_vec(), held.record())
}

#[test]
fn a_record_from_the_other_laptop_is_read_strictly() {
    let (listed, record) = sample();
    assert_eq!(decode_record(&record, &listed).unwrap().len(), 3);
    let with = |f: &dyn Fn(&mut Vec<u8>)| {
        let mut r = record.to_vec();
        f(&mut r);
        decode_record(&r, &listed).err()
    };
    // Another format, a byte too many or too few.
    assert_eq!(with(&|r| r[0] = 2), Some(RecordError::Unreadable));
    assert_eq!(with(&|r| r.push(0)), Some(RecordError::Unreadable));
    assert_eq!(
        with(&|r| {
            r.pop();
        }),
        Some(RecordError::Unreadable)
    );
    // A network at a place not listed, or not ticked (Office, 2; Gran, 4).
    for at in [2u16, 4, 99] {
        assert_eq!(
            with(&|r| r[3..5].copy_from_slice(&at.to_be_bytes())),
            Some(RecordError::NotTicked),
            "{at}"
        );
    }
    // A network sent twice.
    let (l, one) = {
        let held = WifiHeld::new(&offered(&found()), &[0], read_all());
        (held.listed().to_vec(), held.record())
    };
    let body = one[3..].to_vec();
    let mut twice = vec![1u8, 0, 2];
    twice.extend_from_slice(&body);
    twice.extend_from_slice(&body);
    assert_eq!(
        decode_record(&twice, &l).err(),
        Some(RecordError::NotTicked)
    );
    // The listed name changed under it (an SSID the person never saw).
    let mut l2 = l.clone();
    l2[0].name = "Other".into();
    assert_eq!(
        decode_record(&one, &l2).err(),
        Some(RecordError::Unreadable)
    );
    // The same network listed as open but sent with a password, and the other way round.
    let mut l3 = l.clone();
    l3[0].kind = WifiKind::Open;
    assert_eq!(
        decode_record(&one, &l3).err(),
        Some(RecordError::Unreadable)
    );
    // Unknown flags.
    let flags_at = 3 + 2 + 2 + "Home".len() + 2 + "Home".len();
    assert_eq!(
        decode_record(
            &{
                let mut r = one.to_vec();
                r[flags_at] |= 4;
                r
            },
            &l
        )
        .err(),
        Some(RecordError::Unreadable)
    );
    // More networks than were listed.
    assert_eq!(
        with(&|r| r[1..3].copy_from_slice(&999u16.to_be_bytes())),
        Some(RecordError::Unreadable)
    );
    // An empty record: nothing.
    assert!(decode_record(&[], &listed).unwrap().is_empty());
}

/// A laptop's saved networks, as a stand-in for the system's own tools.
#[derive(Default)]
struct Here {
    saved: Vec<String>,
    unseen: bool,
    refuse: Vec<String>,
    profile: bool,
    added: Vec<String>,
}

impl WifiHere for Here {
    fn saved(&mut self) -> Result<Vec<String>, pctwin_fullmove::wifi::NotSeen> {
        if self.unseen {
            Err(pctwin_fullmove::wifi::NotSeen)
        } else {
            Ok(self.saved.clone())
        }
    }
    fn add(&mut self, nets: &[&Network]) -> Vec<Result<(), NotAdded>> {
        nets.iter()
            .map(|n| {
                self.added.push(n.ssid.clone());
                if self.profile {
                    Err(NotAdded::InstallProfile)
                } else if self.refuse.contains(&n.ssid) {
                    Err(NotAdded::Failed)
                } else {
                    Ok(())
                }
            })
            .collect()
    }
}

fn words(done: &[WifiDone]) -> Vec<(String, ItemStatus, Option<WifiWhy>)> {
    done.iter()
        .map(|d| (d.name.clone(), d.result.status(), d.why))
        .collect()
}

#[test]
fn every_network_ends_with_one_honest_word() {
    let (mut listed, record) = sample();
    // Home was read; pretend the hidden one's password was not.
    let arrived = decode_record(&record, &listed).unwrap();
    listed.push(WifiListed {
        name: "Lab".into(),
        kind: WifiKind::Password,
        ticked: true,
        read: false,
    });
    listed.push(WifiListed {
        name: "Odd".into(),
        kind: WifiKind::Other,
        ticked: false,
        read: false,
    });
    let mut here = Here {
        saved: vec!["Cafe".into()],
        refuse: vec!["hidden-net".into()],
        ..Here::default()
    };
    let done = add_networks(&listed, Some(&arrived), &mut here);
    use ItemStatus::*;
    assert_eq!(
        words(&done),
        [
            ("Home".into(), Copied, None),
            ("Cafe".into(), NeedsAction, Some(WifiWhy::AlreadyHere)),
            ("Office".into(), NeedsAction, Some(WifiWhy::Organisation)),
            ("hidden-net".into(), NeedsAction, Some(WifiWhy::NotAdded)),
            ("Gran".into(), SkippedByUser, None),
            ("Lab".into(), NeedsAction, Some(WifiWhy::NoPassword)),
            ("Odd".into(), NeedsAction, Some(WifiWhy::NotThisKind)),
        ]
    );
    // A network already saved here is never added over.
    assert_eq!(here.added, ["Home", "hidden-net"]);
    for d in &done {
        if d.result.status() == NeedsAction {
            assert_eq!(d.result.reason(), Some(Reason::FinishByHand));
        }
    }
}

#[test]
fn nothing_is_added_when_what_is_here_cannot_be_seen_or_nothing_arrived() {
    let (listed, record) = sample();
    let arrived = decode_record(&record, &listed).unwrap();
    let mut here = Here {
        unseen: true,
        ..Here::default()
    };
    let done = add_networks(&listed, Some(&arrived), &mut here);
    assert!(here.added.is_empty());
    assert_eq!(done[0].why, Some(WifiWhy::NotAdded));
    // The record never came: each ticked network that was read did not arrive.
    let mut here = Here::default();
    let done = add_networks(&listed, None, &mut here);
    assert!(here.added.is_empty());
    assert_eq!(done[0].why, Some(WifiWhy::DidNotArrive));
    assert_eq!(done[4].result.status(), ItemStatus::SkippedByUser);
    // A Mac: the profile not installed is the person's step.
    let mut here = Here {
        profile: true,
        ..Here::default()
    };
    let done = add_networks(&listed, Some(&arrived), &mut here);
    assert_eq!(done[0].why, Some(WifiWhy::InstallProfile));
    assert_eq!(done[0].result.status(), ItemStatus::NeedsAction);
}

#[test]
fn the_list_crosses_in_the_extras_and_is_read_strictly() {
    let (listed, _) = sample();
    let mut x = Extras::empty(pctwin_settings::System::Windows);
    // None: the bytes do not name it, so an older new laptop reads them.
    assert!(!String::from_utf8(x.encode()).unwrap().contains("wifi"));
    x.wifi = Some(listed.clone());
    assert_eq!(Extras::decode(&x.encode()).unwrap(), x);
    type Change = Box<dyn Fn(&mut Vec<WifiListed>)>;
    let bad: Vec<Change> = vec![
        Box::new(|l| l[2].ticked = true),
        Box::new(|l| {
            l[4].read = true;
        }),
        Box::new(|l| l[1].name = "Home".into()),
        Box::new(|l| l[0].name = "a\u{7}".into()),
        Box::new(|l| l[0].name = "x".repeat(33)),
        Box::new(|l| l[0].name = String::new()),
        Box::new(|l| {
            let one = l[0].clone();
            l.clear();
            for i in 0..=pctwin_fullmove::wifi::MAX_NETWORKS {
                l.push(WifiListed {
                    name: format!("n{i}"),
                    ..one.clone()
                });
            }
        }),
    ];
    for (i, f) in bad.iter().enumerate() {
        let mut y = x.clone();
        f(y.wifi.as_mut().unwrap());
        assert_eq!(
            Extras::decode(&y.encode()),
            Err(ExtrasError::Limit),
            "case {i}"
        );
    }
}

#[test]
fn the_helpers_profiles_are_kept_only_for_ticked_networks() {
    let xml = |ssid: &str, key: &str| {
        Zeroizing::new(format!(
            "<?xml version=\"1.0\"?><WLANProfile xmlns=\"http://www.microsoft.com/networking/WLAN/profile/v1\"><name>{ssid}</name><SSIDConfig><SSID><name>{ssid}</name></SSID></SSIDConfig><connectionMode>auto</connectionMode><MSM><security><authEncryption><authentication>WPA2PSK</authentication><encryption>AES</encryption><useOneX>false</useOneX></authEncryption><sharedKey><keyType>passPhrase</keyType><protected>false</protected><keyMaterial>{key}</keyMaterial></sharedKey></security></MSM></WLANProfile>"
        ))
    };
    let got = parse_profiles(
        vec![
            xml("Home", "correct horse"),
            xml("Gran", "other pw 1"),
            Zeroizing::new("<x/>".into()),
        ],
        &["Home".to_string()],
    );
    assert_eq!(got.len(), 1);
    assert_eq!(got[0].ssid, "Home");
    assert_eq!(
        got[0].secret.as_ref().unwrap().to_session().to_vec(),
        b"correct horse"
    );
}
