//! Whose is it, without asking (Product Spec section 6; Engineering Plan 9.16): the rules file,
//! the access list boiled down to who could open a place, and the ladder that decides.

use std::collections::BTreeSet;

use pctwin_scan::{
    Access, AccessFact, HomeOf, Kind, People, PlaceFacts, Placement, Rule, Rules, System,
    UnixGroups, Whose, access_from_mode, access_from_sddl, place,
};

const ADA: &str = "S-1-5-21-111-222-333-1001";
const BEA: &str = "S-1-5-21-111-222-333-1002";
const GONE: &str = "S-1-5-21-111-222-333-1009";

struct Laptop {
    several: bool,
}

impl People for Laptop {
    fn mover(&self) -> &str {
        ADA
    }
    fn is_account(&self, id: &str) -> bool {
        id == ADA || id == BEA
    }
    fn several(&self) -> bool {
        self.several
    }
}

const TWO: Laptop = Laptop { several: true };
const ONE: Laptop = Laptop { several: false };

fn decide(access: AccessFact, people: &Laptop) -> Placement {
    place(
        &PlaceFacts {
            rule: None,
            home: None,
            access: &access,
            around: None,
        },
        people,
    )
}

fn known(sddl: &str) -> AccessFact {
    AccessFact::Known(access_from_sddl(sddl).expect("reads"))
}

fn shared() -> Whose {
    Whose::Shared {
        limited_to: Vec::new(),
    }
}

// --- The rules file ------------------------------------------------------------------------

#[test]
fn the_shipped_rules_file_reads_on_every_system() {
    let text = include_str!("../rules/places.txt");
    for system in [System::Windows, System::Mac, System::Linux] {
        let rules = Rules::parse(text, system).unwrap();
        assert!(rules.rows().len() > 10, "{system:?}");
    }
    Rules::shipped();
}

#[test]
fn a_broken_rules_file_is_refused_not_half_read() {
    let err = Rules::parse("windows | sysroot | Windows\n", System::Windows).unwrap_err();
    assert_eq!(err.line, 1);
    assert!(Rules::parse("windows | sky | Windows | system\n", System::Windows).is_err());
    assert!(Rules::parse("windows | root | A/B | system\n", System::Windows).is_err());
    assert!(Rules::parse("windows | root | Data | treasure\n", System::Windows).is_err());
}

#[test]
fn rules_find_system_app_junk_and_rebuildable_places() {
    let win = Rules::parse(include_str!("../rules/places.txt"), System::Windows).unwrap();
    assert_eq!(win.at_root("windows", true).unwrap().kind, Kind::System);
    assert_eq!(win.at_root("Program Files", true).unwrap().kind, Kind::App);
    // The Windows folder of another drive is not this system's.
    assert!(win.at_root("Windows", false).is_none());
    assert_eq!(win.at_root("$Recycle.Bin", false).unwrap().kind, Kind::Junk);
    assert!(win.at_root("Projects", true).is_none());
    assert_eq!(win.in_home(&["AppData"]).unwrap().kind, Kind::AppData);
    assert_eq!(
        win.in_home(&["appdata", "local", "temp"]).unwrap().kind,
        Kind::Junk
    );
    assert!(win.in_home(&["Documents"]).is_none());
    assert_eq!(
        win.anywhere("node_modules").unwrap().kind,
        Kind::Rebuildable
    );
    assert_eq!(win.anywhere(".git").unwrap().kind, Kind::KeepWhole);
    assert!(
        win.at_root("Windows", true)
            .unwrap()
            .label()
            .contains("Windows")
    );

    let linux = Rules::parse(include_str!("../rules/places.txt"), System::Linux).unwrap();
    // Linux keeps case: `Usr` is not `usr`.
    assert!(linux.at_root("Usr", true).is_none());
    assert_eq!(linux.at_root("usr", true).unwrap().kind, Kind::System);
    assert_eq!(linux.in_home(&[".cache"]).unwrap().kind, Kind::Junk);
    // Windows rows do not apply on Linux.
    assert!(linux.at_root("Windows", true).is_none());

    let mac = Rules::parse(include_str!("../rules/places.txt"), System::Mac).unwrap();
    assert_eq!(mac.in_home(&["Library"]).unwrap().kind, Kind::AppData);
    assert_eq!(mac.at_root("System", true).unwrap().kind, Kind::System);
}

// --- The access list -----------------------------------------------------------------------

#[test]
fn administrators_system_and_creator_owner_say_nothing() {
    let a = access_from_sddl(&format!(
        "O:{ADA}G:{ADA}D:PAI(A;OICI;FA;;;SY)(A;OICI;FA;;;BA)(A;OICIIO;FA;;;CO)(A;;FA;;;OW)\
         (A;OICI;FA;;;S-1-5-80-956008885-3418522649-1831038044-1853292631-2271478464)(A;OICI;FA;;;{ADA})"
    ))
    .unwrap();
    assert!(!a.everyone);
    assert_eq!(a.people, BTreeSet::from([ADA.to_string()]));
    assert_eq!(a.owner.as_deref(), Some(ADA));
}

#[test]
fn users_authenticated_users_everyone_and_interactive_mean_everyone() {
    for who in [
        "BU",
        "AU",
        "WD",
        "IU",
        "S-1-1-0",
        "S-1-5-32-545",
        "S-1-5-11",
        "S-1-5-4",
    ] {
        let a = access_from_sddl(&format!("O:BAD:(A;OICI;0x1200a9;;;{who})")).unwrap();
        assert!(a.everyone, "{who}");
        assert!(a.people.is_empty(), "{who}");
    }
    // The "None" group every local account belongs to.
    let a = access_from_sddl("O:BAD:(A;;FR;;;S-1-5-21-111-222-333-513)").unwrap();
    assert!(a.everyone);
}

#[test]
fn rights_that_do_not_read_and_entries_only_passed_on_say_nothing() {
    // Traverse and read-attributes only (0x100080), write only (0x2), and an inherit-only entry.
    let a = access_from_sddl(&format!(
        "O:{ADA}D:(A;;0x100080;;;WD)(A;;0x2;;;BU)(A;OICIIO;FA;;;{BEA})(A;;FA;;;{ADA})"
    ))
    .unwrap();
    assert!(!a.everyone);
    assert_eq!(a.people, BTreeSet::from([ADA.to_string()]));
    // Generic read written as letters, and a denied person removed.
    let a = access_from_sddl(&format!(
        "O:BAD:(A;;GRGX;;;{BEA})(A;;FA;;;{ADA})(D;;FR;;;{BEA})"
    ))
    .unwrap();
    assert_eq!(a.people, BTreeSet::from([ADA.to_string()]));
}

#[test]
fn no_access_list_means_everyone_and_bad_text_is_not_an_answer() {
    assert!(access_from_sddl("O:BAG:SY").unwrap().everyone);
    assert!(
        access_from_sddl("O:BAD:NO_ACCESS_CONTROL")
            .unwrap()
            .everyone
    );
    assert!(access_from_sddl("O:BAD:(A;;FA").is_none());
    assert!(access_from_sddl("O:BAD:(A;FA)").is_none());
    // An empty list: nobody but the owner's own rights.
    let a = access_from_sddl(&format!("O:{BEA}D:P")).unwrap();
    assert!(!a.everyone && a.people.is_empty());
}

fn groups() -> UnixGroups {
    let passwd = "root:x:0:0::/root:/bin/bash\nada:x:1000:1000::/home/ada:/bin/bash\n\
                  bea:x:1001:1001::/home/bea:/bin/bash\ncal:x:1002:1002::/home/cal:/bin/bash\n";
    let group = "root:x:0:\nada:x:1000:\nbea:x:1001:\ncal:x:1002:\nfamily:x:2000:ada,bea\n\
                 sudo:x:27:ada\nusers:x:100:\n";
    UnixGroups::from_files(group, passwd, false)
}

#[test]
fn linux_other_read_means_everyone_and_a_private_group_means_its_person() {
    let g = groups();
    assert!(access_from_mode(1000, 1000, 0o644, &g).everyone);
    let a = access_from_mode(1000, 1000, 0o750, &g);
    assert!(!a.everyone);
    assert_eq!(a.people, BTreeSet::from(["1000".to_string()]));
    let a = access_from_mode(1000, 2000, 0o750, &g);
    assert_eq!(
        a.people,
        BTreeSet::from(["1000".to_string(), "1001".to_string()])
    );
    // The administrators' group carries no signal; the users group is everyone.
    assert_eq!(
        access_from_mode(1000, 27, 0o750, &g).people,
        BTreeSet::from(["1000".to_string()])
    );
    assert!(access_from_mode(1000, 100, 0o750, &g).everyone);
    // Owner-only.
    let a = access_from_mode(1002, 1002, 0o700, &g);
    assert_eq!(a.people, BTreeSet::from(["1002".to_string()]));
    assert_eq!(a.owner.as_deref(), Some("1002"));
}

#[test]
fn on_a_mac_staff_means_everyone_and_admin_says_nothing() {
    let g = UnixGroups::from_files(
        "staff:*:20:root\nadmin:*:80:root,ada\n",
        "ada:*:501:20::0:0:Ada:/Users/ada:/bin/zsh\n",
        true,
    );
    assert!(access_from_mode(501, 20, 0o750, &g).everyone);
    let a = access_from_mode(501, 80, 0o750, &g);
    assert!(!a.everyone);
    assert_eq!(a.people, BTreeSet::from(["501".to_string()]));
}

// --- The ladder ----------------------------------------------------------------------------

#[test]
fn photos_on_d_owned_by_one_person_but_readable_by_users_are_shared() {
    // D:\Photos made by Bea: the drive's default list lets Users read it.
    let p = decide(
        known(&format!(
            "O:{BEA}D:AI(A;OICI;FA;;;SY)(A;OICI;FA;;;BA)(A;OICI;0x1200a9;;;BU)(A;ID;0x1301bf;;;AU)(A;OICIID;FA;;;{BEA})"
        )),
        &TWO,
    );
    assert_eq!(p.whose, shared());
    assert_eq!(p.rule, Rule::EveryoneCanRead);
}

#[test]
fn readable_only_by_one_person_and_administrators_is_that_persons() {
    let only_bea = format!("O:BAD:P(A;OICI;FA;;;SY)(A;OICI;FA;;;BA)(A;OICI;FA;;;{BEA})");
    let p = decide(known(&only_bea), &TWO);
    assert_eq!(
        p.whose,
        Whose::Person {
            account_id: BEA.into()
        }
    );
    assert_eq!(p.rule, Rule::OnlyOnePerson);
    // An administrators-owned copy follows the access rule, not its owner.
    let only_ada = format!("O:BAD:P(A;OICI;FA;;;BA)(A;OICI;FA;;;{ADA})");
    assert_eq!(decide(known(&only_ada), &TWO).whose, Whose::Mover);
}

#[test]
fn access_decides_before_the_owner() {
    // Owned by Ada (the mover) but readable by everyone: shared, not Ada's.
    let p = decide(
        known(&format!("O:{ADA}D:(A;;FA;;;{ADA})(A;;FR;;;WD)")),
        &TWO,
    );
    assert_eq!(p.whose, shared());
    // Owned by Ada but readable only by Bea: Bea's.
    let p = decide(known(&format!("O:{ADA}D:P(A;;FA;;;{BEA})")), &TWO);
    assert_eq!(
        p.whose,
        Whose::Person {
            account_id: BEA.into()
        }
    );
}

#[test]
fn a_few_named_people_share_it_limited_to_them() {
    let p = decide(
        known(&format!("O:{ADA}D:P(A;;FA;;;{ADA})(A;;FR;;;{BEA})")),
        &TWO,
    );
    assert_eq!(
        p.whose,
        Whose::Shared {
            limited_to: vec![ADA.into(), BEA.into()]
        }
    );
    assert_eq!(p.rule, Rule::NamedPeople);
}

#[test]
fn the_owner_decides_only_for_owner_only_access_and_a_gone_owner_goes_to_an_old_account() {
    let p = decide(known(&format!("O:{BEA}D:P(A;;FA;;;BA)")), &TWO);
    assert_eq!(
        p.whose,
        Whose::Person {
            account_id: BEA.into()
        }
    );
    assert_eq!(p.rule, Rule::OwnerOnly);
    let p = decide(
        known(&format!("O:{GONE}D:P(A;;FA;;;{GONE})(A;;FA;;;SY)")),
        &TWO,
    );
    assert_eq!(
        p.whose,
        Whose::FromOldAccount {
            account_id: Some(GONE.into())
        }
    );
    assert_eq!(p.rule, Rule::UnresolvableOwner);
    // Owned by the administrators (or root), and only they and the system could open it: the
    // system's. The administrators are an account, never gone, so not "from an old account".
    let p = decide(known("O:BAD:P(A;;FA;;;BA)(A;;FA;;;SY)"), &TWO);
    assert_eq!(p.whose, Whose::System);
    assert_eq!(p.rule, Rule::OwnerOnly);
    let root_only = access_from_mode(0, 0, 0o700, &groups());
    let p = decide(AccessFact::Known(root_only), &TWO);
    assert_eq!(p.whose, Whose::System);
    // An owner that could not be read at all: nothing better is known.
    let p = decide(known("D:P(A;;FA;;;BA)(A;;FA;;;SY)"), &TWO);
    assert_eq!(p.whose, Whose::FromOldAccount { account_id: None });
}

#[test]
fn a_laptop_with_one_person_gives_everything_else_to_the_mover() {
    let p = decide(known(&format!("O:{BEA}D:P(A;;FA;;;{BEA})")), &ONE);
    assert_eq!(p.whose, Whose::Mover);
    assert_eq!(p.rule, Rule::OnePersonLaptop);
    assert_eq!(
        decide(AccessFact::Unknown { why: "x".into() }, &ONE).whose,
        Whose::Mover
    );
}

#[test]
fn drives_without_owners_go_to_the_mover_alone_or_are_shared() {
    assert_eq!(decide(AccessFact::NoOwners, &ONE).whose, Whose::Mover);
    let p = decide(AccessFact::NoOwners, &TWO);
    assert_eq!(p.whose, shared());
    assert_eq!(p.rule, Rule::NoOwnerDrive);
}

#[test]
fn unreadable_access_on_a_shared_laptop_is_never_guessed() {
    let p = decide(
        AccessFact::Unknown {
            why: "denied".into(),
        },
        &TWO,
    );
    assert_eq!(p.whose, Whose::Undecided);
    assert_eq!(p.rule, Rule::AccessNotReadable);
}

#[test]
fn inside_a_home_means_that_person_whatever_the_access() {
    let everyone = AccessFact::Known(Access {
        everyone: true,
        people: BTreeSet::new(),
        owner: None,
    });
    for (home, want) in [
        (HomeOf::Mover, Whose::Mover),
        (
            HomeOf::Person(BEA.into()),
            Whose::Person {
                account_id: BEA.into(),
            },
        ),
        (
            HomeOf::Leftover(Some(GONE.into())),
            Whose::FromOldAccount {
                account_id: Some(GONE.into()),
            },
        ),
    ] {
        let p = place(
            &PlaceFacts {
                rule: None,
                home: Some(home),
                access: &everyone,
                around: None,
            },
            &TWO,
        );
        assert_eq!(p.whose, want);
        assert_eq!(p.rule, Rule::InsideHome);
    }
}

#[test]
fn the_rules_file_decides_first_and_says_which_row() {
    let rules = Rules::parse(include_str!("../rules/places.txt"), System::Windows).unwrap();
    let everyone = known("O:BAD:(A;;FR;;;WD)");
    let p = place(
        &PlaceFacts {
            rule: rules.at_root("Windows", true),
            home: None,
            access: &everyone,
            around: None,
        },
        &TWO,
    );
    assert_eq!(p.whose, Whose::System);
    assert_eq!(p.kind, Some(Kind::System));
    assert!(matches!(&p.rule, Rule::RulesFile { row } if row.contains("Windows")));
    // node_modules inside the mover's project stays the mover's, counted as one item.
    let project = Placement::new(Whose::Mover, Rule::OnePersonLaptop);
    let p = place(
        &PlaceFacts {
            rule: rules.anywhere("node_modules"),
            home: None,
            access: &everyone,
            around: Some(&project),
        },
        &TWO,
    );
    assert_eq!(p.whose, Whose::Mover);
    assert_eq!(p.kind, Some(Kind::Rebuildable));
    // App data in another person's home stays theirs.
    let p = place(
        &PlaceFacts {
            rule: rules.in_home(&["AppData"]),
            home: Some(HomeOf::Person(BEA.into())),
            access: &everyone,
            around: None,
        },
        &TWO,
    );
    assert_eq!(
        p.whose,
        Whose::Person {
            account_id: BEA.into()
        }
    );
    assert_eq!(p.kind, Some(Kind::AppData));
}

#[test]
fn placements_say_why_in_their_saved_form() {
    let p = Placement::new(
        Whose::Shared {
            limited_to: vec![ADA.into()],
        },
        Rule::NamedPeople,
    );
    let json = serde_json::to_value(&p).unwrap();
    assert_eq!(json["whose"], "shared");
    assert_eq!(json["rule"], "named-people");
    let back: Placement = serde_json::from_value(json).unwrap();
    assert_eq!(back, p);
}
