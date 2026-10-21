//! Will the plan fit: each approved place's free space as its drive reports it, places on one
//! drive judged together, with room left for the system.
// Tests make folders to test on; the no-file-changes fence is for the shipped code.
#![allow(clippy::disallowed_methods)]

use std::collections::BTreeMap;

use pctwin_gate::{Approved, Destinations};
use pctwin_transfer::{Fit, KEEP_FREE};

#[test]
fn places_on_one_drive_are_judged_on_what_they_need_together() {
    let root = tempfile::tempdir().unwrap();
    let (a, b) = (root.path().join("a"), root.path().join("b"));
    std::fs::create_dir_all(&a).unwrap();
    std::fs::create_dir_all(&b).unwrap();
    let mut table = Destinations::new();
    table.approve("a", Approved::MyFolders, &a).unwrap();
    table.approve("b", Approved::SharedFolder, &b).unwrap();
    let needed = BTreeMap::from([("a".to_string(), 10u64), ("b".to_string(), 20u64)]);
    let fit = Fit::check(&needed, &table);
    assert_eq!(fit.places.len(), 2);
    for p in &fit.places {
        // Both folders are on the test's drive: each is judged on 30 bytes, never 10 or 20.
        assert_eq!(p.needed, 30, "{p:?}");
        let free = p.free.expect("the drive says how much is free");
        assert_eq!(p.fits(), Some(30 + KEEP_FREE <= free));
    }
    // A plan far bigger than any drive does not fit, and says by how much.
    let huge = BTreeMap::from([("a".to_string(), u64::MAX / 4)]);
    let fit = Fit::check(&huge, &table);
    assert_eq!(fit.fits(), Some(false));
    assert!(fit.places[0].short_by() > 0);
}

#[test]
fn a_place_not_approved_cannot_be_checked_and_never_counts_as_fitting() {
    let table = Destinations::new();
    let needed = BTreeMap::from([("nowhere".to_string(), 1u64)]);
    let fit = Fit::check(&needed, &table);
    assert_eq!(fit.places[0].free, None);
    assert_eq!(fit.fits(), None);
}
