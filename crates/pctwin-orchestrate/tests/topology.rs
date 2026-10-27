//! Several people and several old laptops (Task List 2.6; acceptance T26-001 to T26-005, T26-027,
//! T26-028).

mod common;

use std::collections::BTreeMap;

use common::*;
use pctwin_orchestrate::topology::{
    LandsAt, SourcePerson, SourceResult, Topology, TopologyError, landing, provenance_folders,
};
use pctwin_record::{FolderRole, Inclusion, Item, LeftOutReason, PersonTarget, Record, Storage};

fn record(laptop_n: u8, items: Vec<Item>) -> Record {
    let mut r = Record::new(laptop(laptop_n));
    r.items = items;
    r
}

fn at(laptop_n: u8, owner: &str, parts: &[&str], size: u64) -> Item {
    item_in(
        &laptop(laptop_n),
        person(owner),
        FolderRole::Documents,
        Storage::SystemDrive,
        parts,
        size,
    )
}

fn sp(laptop_n: u8, account: &str) -> SourcePerson {
    SourcePerson {
        laptop: laptop(laptop_n),
        account_id: account.into(),
    }
}

fn existing(account: &str) -> PersonTarget {
    PersonTarget::ExistingAccount {
        account_id: account.into(),
    }
}

/// T26-002: two old Macs both have account 501, and two old Windows laptops cloned from one image
/// have the same account ID: they are different people until the person says otherwise. Equal
/// names are a suggestion at most.
#[test]
fn equal_account_ids_on_two_old_laptops_are_two_people() {
    let mut t = Topology::default();
    t.add_source(
        laptop(1),
        "Ada's MacBook",
        record(1, vec![at(1, "501", &["a.txt"], 1)]),
    )
    .unwrap();
    t.add_source(
        laptop(2),
        "Old iMac",
        record(2, vec![at(2, "501", &["b.txt"], 1)]),
    )
    .unwrap();
    let people = t.people();
    assert_eq!(people, [sp(1, "501"), sp(2, "501")]);
    // Mapping only the first leaves the second unmapped: nothing is assumed from the equal ID.
    t.map(sp(1, "501"), existing("S-NEW-1")).unwrap();
    assert_eq!(
        t.check_mapping(),
        Err(TopologyError::Unmapped {
            person: sp(2, "501")
        })
    );
    // Someone whose things are all left out needs no place to go.
    let mut u = Topology::default();
    let mut left = at(3, "S-X", &["x"], 1);
    left.inclusion = Inclusion::LeftOut {
        reason: LeftOutReason::ChosenByUser,
    };
    u.add_source(laptop(3), "PC", record(3, vec![left]))
        .unwrap();
    assert_eq!(u.check_mapping(), Ok(()));
    // Equal names only ever suggest.
    let names = BTreeMap::from([(sp(1, "501"), "ada"), (sp(2, "501"), "Ada")]);
    let targets = [("S-NEW-1", "Ada"), ("S-NEW-2", "Bob")];
    let s = t.suggest(&names, &targets);
    assert_eq!(s[&sp(1, "501")], Some("S-NEW-1".to_string()));
    assert_eq!(s[&sp(2, "501")], Some("S-NEW-1".to_string()));
    // ...and a suggestion is never a mapping.
    assert_eq!(t.target_of(&sp(2, "501")), None);
}

/// T26-001: several people on one old laptop each get their own scope and target; two people on
/// one old laptop can never be put into one account. The same person on two old laptops may be,
/// when the mover chooses it.
#[test]
fn people_on_one_old_laptop_never_share_an_account() {
    let mut t = Topology::default();
    t.add_source(
        laptop(1),
        "Family PC",
        record(
            1,
            vec![
                at(1, "S-ADA", &["a.txt"], 1),
                at(1, "S-TUNDE", &["t.txt"], 1),
            ],
        ),
    )
    .unwrap();
    t.add_source(
        laptop(2),
        "Ada's laptop",
        record(2, vec![at(2, "S-ADA2", &["x"], 1)]),
    )
    .unwrap();
    t.map(sp(1, "S-ADA"), existing("NEW-ADA")).unwrap();
    assert_eq!(
        t.map(sp(1, "S-TUNDE"), existing("NEW-ADA")),
        Err(TopologyError::AccountUsedTwice)
    );
    t.map(sp(1, "S-TUNDE"), existing("NEW-TUNDE")).unwrap();
    // Ada's other laptop into Ada's account: allowed, it is the mover's choice.
    t.map(sp(2, "S-ADA2"), existing("NEW-ADA")).unwrap();
    assert_eq!(t.check_mapping(), Ok(()));
    // Each person's scope is only their own items.
    let ada = t.scope(&sp(1, "S-ADA"));
    assert_eq!(ada.len(), 1);
    assert_eq!(ada[0], at(1, "S-ADA", &["a.txt"], 1).id);
    // A person not on that laptop cannot be mapped.
    assert_eq!(
        t.map(sp(2, "S-TUNDE"), existing("X")),
        Err(TopologyError::UnknownPerson)
    );
    // Remapping someone frees their old account for another.
    t.map(sp(1, "S-TUNDE"), PersonTarget::Skip).unwrap();
    t.map(sp(1, "S-ADA"), existing("NEW-TUNDE")).unwrap();
    // Two new accounts of the same name from one laptop are one account twice.
    let mut u = Topology::default();
    u.add_source(
        laptop(1),
        "PC",
        record(1, vec![at(1, "A", &["a"], 1), at(1, "B", &["b"], 1)]),
    )
    .unwrap();
    u.map(sp(1, "A"), PersonTarget::NewAccount { name: "Sam".into() })
        .unwrap();
    assert_eq!(
        u.map(sp(1, "B"), PersonTarget::NewAccount { name: "sam".into() }),
        Err(TopologyError::AccountUsedTwice)
    );
    // A laptop added twice is refused.
    assert_eq!(
        u.add_source(laptop(1), "PC", record(1, vec![])),
        Err(TopologyError::SourceTwice)
    );
    // A record from another laptop than it claims is refused.
    assert_eq!(
        u.add_source(laptop(3), "PC", record(4, vec![])),
        Err(TopologyError::WrongLaptop)
    );
}

/// T26-003 and T26-004: two old laptops with the same names in the same place: each copy lands
/// in its own "From <laptop>" folder, even when the bytes are equal; names that do not clash land
/// as usual. Names are compared as the new laptop holds them (case, composed accents).
#[test]
fn clashing_names_from_two_laptops_each_get_their_own_folder() {
    let mut t = Topology::default();
    let a1 = at(1, "S-ADA", &["Report.docx"], 100);
    let a2 = at(1, "S-ADA", &["Only A.txt"], 1);
    let a3 = at(1, "S-ADA", &["Caf\u{e9}", "menu.txt"], 1);
    let b1 = at(2, "S-ADA2", &["report.DOCX"], 100);
    let b2 = at(2, "S-ADA2", &["Cafe\u{301}", "menu.txt"], 1);
    let b3 = at(2, "S-ADA2", &["Only B.txt"], 1);
    t.add_source(
        laptop(1),
        "Laptop",
        record(1, vec![a1.clone(), a2.clone(), a3.clone()]),
    )
    .unwrap();
    t.add_source(
        laptop(2),
        "Laptop",
        record(2, vec![b1.clone(), b2.clone(), b3.clone()]),
    )
    .unwrap();
    t.map(sp(1, "S-ADA"), existing("NEW-ADA")).unwrap();
    t.map(sp(2, "S-ADA2"), existing("NEW-ADA")).unwrap();
    let folders = provenance_folders(&t);
    // Equal labels are told apart in a fixed order.
    assert_eq!(folders[&laptop(1)], "From Laptop");
    assert_eq!(folders[&laptop(2)], "From Laptop (2)");
    let lands = landing(&t);
    assert_eq!(
        lands[&(laptop(1), a1.id)],
        LandsAt::FromLaptop("From Laptop".into())
    );
    assert_eq!(
        lands[&(laptop(2), b1.id)],
        LandsAt::FromLaptop("From Laptop (2)".into())
    );
    assert_eq!(
        lands[&(laptop(1), a3.id)],
        LandsAt::FromLaptop("From Laptop".into())
    );
    assert_eq!(
        lands[&(laptop(2), b2.id)],
        LandsAt::FromLaptop("From Laptop (2)".into())
    );
    assert_eq!(lands[&(laptop(1), a2.id)], LandsAt::AsUsual);
    assert_eq!(lands[&(laptop(2), b3.id)], LandsAt::AsUsual);
    // A person who is skipped lands nowhere.
    let mut sk = Topology::default();
    let kept = at(1, "X", &["k"], 1);
    let skipped = at(2, "Y", &["k"], 1);
    sk.add_source(laptop(1), "A", record(1, vec![kept.clone()]))
        .unwrap();
    sk.add_source(laptop(2), "B", record(2, vec![skipped.clone()]))
        .unwrap();
    sk.map(sp(1, "X"), existing("N")).unwrap();
    sk.map(sp(2, "Y"), PersonTarget::Skip).unwrap();
    let l = landing(&sk);
    assert_eq!(l.len(), 1);
    assert_eq!(l[&(laptop(1), kept.id)], LandsAt::AsUsual);
    // Two people's things never clash with each other.
    let mut u = Topology::default();
    u.add_source(laptop(1), "A", record(1, vec![at(1, "X", &["same"], 1)]))
        .unwrap();
    u.add_source(laptop(2), "B", record(2, vec![at(2, "Y", &["same"], 1)]))
        .unwrap();
    u.map(sp(1, "X"), existing("NX")).unwrap();
    u.map(sp(2, "Y"), existing("NY")).unwrap();
    assert!(landing(&u).values().all(|l| *l == LandsAt::AsUsual));
    // A file on one laptop with the name of a folder on the other clashes too.
    let mut v = Topology::default();
    let f = at(1, "X", &["Notes"], 1);
    let g = at(2, "Y", &["Notes", "inside.txt"], 1);
    v.add_source(laptop(1), "A", record(1, vec![f.clone()]))
        .unwrap();
    v.add_source(laptop(2), "B", record(2, vec![g.clone()]))
        .unwrap();
    v.map(sp(1, "X"), existing("N")).unwrap();
    v.map(sp(2, "Y"), existing("N")).unwrap();
    let l = landing(&v);
    assert_eq!(l[&(laptop(1), f.id)], LandsAt::FromLaptop("From A".into()));
    assert_eq!(l[&(laptop(2), g.id)], LandsAt::FromLaptop("From B".into()));
    // Different folders (roles) never clash; nor do items left out or skipped people.
    let mut w = Topology::default();
    let mut pic = at(2, "Y", &["same"], 1);
    pic.place.role = FolderRole::Pictures;
    let mut left = at(2, "Y", &["other"], 1);
    left.inclusion = Inclusion::LeftOut {
        reason: LeftOutReason::ChosenByUser,
    };
    w.add_source(
        laptop(1),
        "A",
        record(1, vec![at(1, "X", &["same"], 1), at(1, "X", &["other"], 1)]),
    )
    .unwrap();
    w.add_source(laptop(2), "B", record(2, vec![pic, left]))
        .unwrap();
    w.map(sp(1, "X"), existing("N")).unwrap();
    w.map(sp(2, "Y"), existing("N")).unwrap();
    assert!(landing(&w).values().all(|l| *l == LandsAt::AsUsual));
}

/// T26-005: an app installed for everyone is installed once, while each person's own data stays
/// their own.
#[test]
fn a_shared_app_is_installed_once_and_data_stays_per_person() {
    use pctwin_record::{ItemKind, Owner};
    let mut t = Topology::default();
    let mut app1 = at(1, "S-ADA", &["Firefox"], 0);
    app1.kind = ItemKind::App;
    app1.owner = Owner::Shared;
    let mut app2 = app1.clone();
    app2.id = at(2, "S-ADA2", &["Firefox"], 0).id;
    let ada_data = at(1, "S-ADA", &["profile"], 5);
    // An app installed for one person only is not installed for everyone.
    let mut own_app = at(1, "S-ADA", &["Spotify"], 0);
    own_app.kind = ItemKind::App;
    let tunde_data = at(1, "S-TUNDE", &["profile"], 5);
    t.add_source(
        laptop(1),
        "PC",
        record(
            1,
            vec![
                app1.clone(),
                ada_data.clone(),
                tunde_data.clone(),
                own_app.clone(),
            ],
        ),
    )
    .unwrap();
    t.add_source(laptop(2), "Laptop", record(2, vec![app2.clone()]))
        .unwrap();
    let installs = t.shared_installs();
    assert_eq!(installs.len(), 1, "one install for the machine");
    assert_eq!(installs[0].name, "Firefox");
    assert_eq!(installs[0].from.len(), 2);
    // Data stays with its person.
    assert_eq!(t.scope(&sp(1, "S-ADA")), [ada_data.id, own_app.id]);
    assert_eq!(t.scope(&sp(1, "S-TUNDE")), [tunde_data.id]);
}

/// Each old laptop keeps its own record, its own journal and its own undo.
#[test]
fn each_old_laptop_keeps_its_own_record_journal_and_undo() {
    let mut t = Topology::default();
    let a = at(1, "X", &["a"], 1);
    let b = at(2, "Y", &["b"], 1);
    t.add_source(laptop(1), "A", record(1, vec![a.clone()]))
        .unwrap();
    t.add_source(laptop(2), "B", record(2, vec![b.clone()]))
        .unwrap();
    assert_ne!(t.journal_name(&laptop(1)), t.journal_name(&laptop(2)));
    assert_eq!(
        t.journal_name(&laptop(1)).unwrap(),
        format!("move-from-{}", laptop(1).to_hex())
    );
    assert_eq!(t.journal_name(&laptop(9)), None);
    assert_eq!(t.undo_scope(&laptop(1)), [a.id]);
    assert_eq!(t.undo_scope(&laptop(2)), [b.id]);
    assert_eq!(t.record(&laptop(2)).unwrap().items, [b]);
}

/// T26-027: each old laptop has its own session number; a late result from an old session, or a
/// result naming another laptop or an item not in its approved plan, is refused.
#[test]
fn a_late_result_from_an_old_session_is_refused() {
    let mut t = Topology::default();
    let a = at(1, "X", &["a"], 1);
    let b = at(2, "Y", &["b"], 1);
    t.add_source(laptop(1), "A", record(1, vec![a.clone()]))
        .unwrap();
    t.add_source(laptop(2), "B", record(2, vec![b.clone()]))
        .unwrap();
    t.map(sp(1, "X"), existing("N1")).unwrap();
    t.map(sp(2, "Y"), existing("N2")).unwrap();
    t.approve(&laptop(1)).unwrap();
    t.approve(&laptop(2)).unwrap();
    let old = t.begin_session(&laptop(1)).unwrap();
    let new = t.begin_session(&laptop(1)).unwrap();
    assert!(new > old);
    let other = t.begin_session(&laptop(2)).unwrap();
    let result = |laptop_n: u8, session: u64, item| SourceResult {
        laptop: laptop(laptop_n),
        session,
        item,
    };
    assert_eq!(
        t.accept(&result(1, old, a.id)),
        Err(TopologyError::OldSession)
    );
    assert_eq!(t.accept(&result(1, new, a.id)), Ok(()));
    // Laptop 2's session number means nothing for laptop 1.
    assert_eq!(t.accept(&result(2, other, b.id)), Ok(()));
    assert_eq!(
        t.accept(&result(1, other.max(new) + 1, a.id)),
        Err(TopologyError::OldSession)
    );
    assert_eq!(
        t.accept(&result(1, new, b.id)),
        Err(TopologyError::NotInApprovedPlan)
    );
    assert_eq!(
        t.accept(&result(9, new, a.id)),
        Err(TopologyError::UnknownSource)
    );
    assert_eq!(
        t.begin_session(&laptop(9)),
        Err(TopologyError::UnknownSource)
    );
}

/// T26-028: an old laptop that changed after the plan was approved: only the affected items wait
/// for a fresh approval; the rest carry on.
#[test]
fn a_change_after_approval_holds_only_the_affected_items() {
    let mut t = Topology::default();
    let a = at(1, "X", &["a"], 1);
    let b = at(1, "X", &["b"], 1);
    let c = at(1, "X", &["c"], 1);
    t.add_source(
        laptop(1),
        "A",
        record(1, vec![a.clone(), b.clone(), c.clone()]),
    )
    .unwrap();
    t.map(sp(1, "X"), existing("N")).unwrap();
    // Nothing moves before approval.
    let s = t.begin_session(&laptop(1)).unwrap();
    let r = |item| SourceResult {
        laptop: laptop(1),
        session: s,
        item,
    };
    assert_eq!(t.accept(&r(a.id)), Err(TopologyError::NotApproved));
    t.approve(&laptop(1)).unwrap();
    for i in [&a, &b, &c] {
        assert!(t.may_move(&laptop(1), &i.id));
    }
    // b changed, c was removed, d is new.
    let mut b2 = b.clone();
    b2.size_bytes = 2;
    let d = at(1, "X", &["d"], 1);
    let changed = t
        .refresh(
            &laptop(1),
            record(1, vec![a.clone(), b2.clone(), d.clone()]),
        )
        .unwrap();
    let mut want = vec![b.id, c.id, d.id];
    want.sort();
    assert_eq!(changed, want);
    assert!(t.may_move(&laptop(1), &a.id), "unchanged items carry on");
    for i in [&b, &c, &d] {
        assert!(!t.may_move(&laptop(1), &i.id));
    }
    assert_eq!(t.accept(&r(a.id)), Ok(()));
    assert_eq!(t.accept(&r(b.id)), Err(TopologyError::NeedsFreshApproval));
    // The fresh approval covers the new revision.
    t.approve(&laptop(1)).unwrap();
    assert!(t.may_move(&laptop(1), &b.id));
    assert!(t.may_move(&laptop(1), &d.id));
    assert!(!t.may_move(&laptop(1), &c.id), "removed since");
    // A refresh with no change holds nothing.
    let same = t.record(&laptop(1)).unwrap().clone();
    assert_eq!(t.refresh(&laptop(1), same).unwrap(), []);
    assert!(t.may_move(&laptop(1), &a.id));
    // A refresh from another laptop's record is refused.
    assert_eq!(
        t.refresh(&laptop(1), record(2, vec![])),
        Err(TopologyError::WrongLaptop)
    );
}

/// A laptop's label comes from that laptop, so it is made safe before it names a folder: no
/// separators or reserved characters, no trailing dots, not too long, never empty.
#[test]
fn a_laptop_label_is_made_safe_as_a_folder_name() {
    let mut t = Topology::default();
    t.add_source(laptop(1), "Ada/../Windows:\u{7}", record(1, vec![]))
        .unwrap();
    t.add_source(laptop(2), "...", record(2, vec![])).unwrap();
    t.add_source(laptop(3), &"x".repeat(100), record(3, vec![]))
        .unwrap();
    t.add_source(laptop(4), "Tunde's PC. ", record(4, vec![]))
        .unwrap();
    let f = provenance_folders(&t);
    assert_eq!(f[&laptop(1)], "From Ada .. Windows");
    assert_eq!(f[&laptop(2)], "From another laptop");
    assert_eq!(f[&laptop(3)], format!("From {}", "x".repeat(60)));
    assert_eq!(f[&laptop(4)], "From Tunde's PC");
}
