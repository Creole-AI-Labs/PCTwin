//! Third security review, finding 5: where each person goes is part of what was approved. Putting
//! a person into another account after approval holds that person's items until the plan is
//! approved again; everyone else's carry on.

mod common;

use common::*;
use pctwin_orchestrate::topology::{SourcePerson, SourceResult, Topology, TopologyError};
use pctwin_record::{FolderRole, Item, PersonTarget, Record, Storage};

fn at(owner: &str, parts: &[&str]) -> Item {
    item_in(
        &laptop(1),
        person(owner),
        FolderRole::Documents,
        Storage::SystemDrive,
        parts,
        1,
    )
}

fn sp(account: &str) -> SourcePerson {
    SourcePerson {
        laptop: laptop(1),
        account_id: account.into(),
    }
}

fn existing(account: &str) -> PersonTarget {
    PersonTarget::ExistingAccount {
        account_id: account.into(),
    }
}

#[test]
fn a_person_put_into_another_account_after_approval_needs_fresh_approval() {
    let ada = at("S-ADA", &["thesis.docx"]);
    let tunde = at("S-TUNDE", &["song.mp3"]);
    let mut r = Record::new(laptop(1));
    r.items = vec![ada.clone(), tunde.clone()];
    let mut t = Topology::default();
    t.add_source(laptop(1), "Old PC", r).unwrap();
    t.map(sp("S-ADA"), existing("S-NEW-ADA")).unwrap();
    t.map(sp("S-TUNDE"), existing("S-NEW-TUNDE")).unwrap();
    t.approve(&laptop(1)).unwrap();
    let session = t.begin_session(&laptop(1)).unwrap();
    assert!(t.may_move(&laptop(1), &ada.id));

    // After approval, Ada is put into another account (a guest's, say).
    t.map(sp("S-ADA"), existing("S-GUEST")).unwrap();
    assert!(
        !t.may_move(&laptop(1), &ada.id),
        "Ada's things would go to an account nobody approved"
    );
    assert_eq!(
        t.accept(&SourceResult {
            laptop: laptop(1),
            session,
            item: ada.id,
        }),
        Err(TopologyError::NeedsFreshApproval)
    );
    // Tunde's place did not change: his things carry on.
    assert!(t.may_move(&laptop(1), &tunde.id));

    // Putting Ada back where she was approved to go is the approved plan again.
    t.map(sp("S-ADA"), existing("S-NEW-ADA")).unwrap();
    assert!(t.may_move(&laptop(1), &ada.id));

    // A new place, approved again, moves.
    t.map(sp("S-ADA"), existing("S-GUEST")).unwrap();
    t.approve(&laptop(1)).unwrap();
    assert!(t.may_move(&laptop(1), &ada.id));
}

#[test]
fn the_approved_fingerprint_covers_where_each_person_goes() {
    let mut r = Record::new(laptop(1));
    r.items = vec![at("S-ADA", &["a.txt"])];
    let mut t = Topology::default();
    t.add_source(laptop(1), "Old PC", r).unwrap();
    t.map(sp("S-ADA"), existing("S-ONE")).unwrap();
    let one = t.plan_fingerprint(&laptop(1)).unwrap();
    t.map(sp("S-ADA"), existing("S-TWO")).unwrap();
    let two = t.plan_fingerprint(&laptop(1)).unwrap();
    assert_ne!(one, two);
    t.map(sp("S-ADA"), PersonTarget::NewAccount { name: "Ada".into() })
        .unwrap();
    assert_ne!(t.plan_fingerprint(&laptop(1)).unwrap(), two);
    t.approve(&laptop(1)).unwrap();
    assert_eq!(
        t.approved_fingerprint(&laptop(1)),
        t.plan_fingerprint(&laptop(1))
    );
}

#[test]
fn every_part_of_where_people_go_counts() {
    let mut r = Record::new(laptop(1));
    r.items = vec![at("S-ADA", &["a.txt"]), at("S-TUNDE", &["b.txt"])];
    let mut t = Topology::default();
    t.add_source(laptop(1), "Old PC", r).unwrap();
    // An account that exists and a new one of the same name are two places.
    t.map(sp("S-ADA"), existing("Ada")).unwrap();
    t.map(sp("S-TUNDE"), existing("T")).unwrap();
    let existing_ada = t.plan_fingerprint(&laptop(1)).unwrap();
    t.map(sp("S-ADA"), PersonTarget::NewAccount { name: "Ada".into() })
        .unwrap();
    assert_ne!(t.plan_fingerprint(&laptop(1)).unwrap(), existing_ada);
    // Two people swapping places is a new plan.
    t.map(sp("S-ADA"), existing("X")).unwrap();
    t.map(sp("S-TUNDE"), existing("Y")).unwrap();
    let before = t.plan_fingerprint(&laptop(1)).unwrap();
    t.map(sp("S-ADA"), existing("Y2")).unwrap();
    t.map(sp("S-TUNDE"), existing("X")).unwrap();
    t.map(sp("S-ADA"), existing("Y")).unwrap();
    assert_ne!(t.plan_fingerprint(&laptop(1)).unwrap(), before);
}

#[test]
fn another_old_laptops_people_do_not_hold_this_ones_plan() {
    let one = at("S-ADA", &["a.txt"]);
    let mut r1 = Record::new(laptop(1));
    r1.items = vec![one.clone()];
    let mut r2 = Record::new(laptop(2));
    r2.items = vec![item_in(
        &laptop(2),
        person("S-BOB"),
        FolderRole::Documents,
        Storage::SystemDrive,
        &["b.txt"],
        1,
    )];
    let mut t = Topology::default();
    t.add_source(laptop(1), "PC one", r1).unwrap();
    t.add_source(laptop(2), "PC two", r2).unwrap();
    t.map(sp("S-ADA"), existing("S-NEW-ADA")).unwrap();
    let bob = SourcePerson {
        laptop: laptop(2),
        account_id: "S-BOB".into(),
    };
    t.map(bob.clone(), existing("S-NEW-BOB")).unwrap();
    t.approve(&laptop(1)).unwrap();
    let fp = t.plan_fingerprint(&laptop(1)).unwrap();
    t.map(bob, existing("S-OTHER")).unwrap();
    assert_eq!(t.plan_fingerprint(&laptop(1)).unwrap(), fp);
    assert!(t.may_move(&laptop(1), &one.id));
}

#[test]
fn who_goes_there_counts_not_only_where() {
    let fp = |who: &str| {
        let mut r = Record::new(laptop(1));
        r.items = vec![at("S-ADA", &["a.txt"]), at("S-TUNDE", &["b.txt"])];
        let mut t = Topology::default();
        t.add_source(laptop(1), "Old PC", r).unwrap();
        t.map(sp(who), existing("S-NEW")).unwrap();
        t.plan_fingerprint(&laptop(1)).unwrap()
    };
    assert_ne!(fp("S-ADA"), fp("S-TUNDE"));
}
