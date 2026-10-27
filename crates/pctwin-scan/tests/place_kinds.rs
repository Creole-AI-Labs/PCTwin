//! Kinds of place are rows too (Phase 2 adoption, changes 7 and 8): a new kind with its own
//! traits and markers works from a signed row with no code change, and a newer table never stops
//! an older app.

use pctwin_catalogue::{Catalogue, test_key};
use pctwin_scan::{Recogniser, System, Trait, What, WhatRow};

#[test]
fn a_new_signed_kind_names_places_with_no_code_change() {
    let bytes = br#"{"format":1,"catalogue":"place-kinds","version":2,"rows":[
        {"kind":{"id":"drone-footage","name":"Drone footage","traits":["fetch-again-if-downloaded"]}},
        {"marker":{"system":"any","shape":"holds","marker":"DJI_0001.MP4","what":"drone-footage"}},
        {"marker":{"system":"any","shape":"file-suffix","marker":".dfr","what":"drone-footage"}},
        {"marker":{"system":"any","shape":"folder","marker":"Spare","what":"not-a-kind"}},
        {"kind":{"id":"later","name":"Later","traits":["teleport"]}}
    ]}"#;
    let sig = test_key::sign(bytes, "place-kinds", 2);
    let table = Catalogue::<WhatRow>::signed(bytes, &sig, &test_key::keys(), 1).unwrap();
    let r = Recogniser::from_catalogue(&table, System::Windows);
    let drone = What::new("drone-footage").unwrap();
    assert_eq!(
        r.folder("Flight 3", ["dji_0001.mp4"]).map(|w| w.0),
        Some(drone)
    );
    assert_eq!(r.file("route.DFR").map(|w| w.0), Some(drone));
    assert!(r.has(drone, Trait::FetchAgainIfDownloaded));
    assert_eq!(r.kind(drone).unwrap().name, "Drone footage");
    // A marker of a kind no row gives, and a kind with a trait this version does not know: skipped
    // and reported, the rest still works.
    let (report, unknown) = r.report();
    assert_eq!(report.skipped.len(), 1);
    assert_eq!(unknown, [(4, What::new("not-a-kind").unwrap())]);
    assert_eq!(r.folder("Spare", []), None);
    // The shipped kinds are not in this table: nothing is named by them here.
    assert_eq!(r.folder("DCIM", []), None);
}
