//! The named settings of Task List 2.4 (dark mode, wallpaper, mouse, display scaling, keyboard
//! layout, default browser) between every pair of Windows, Mac and GNOME: each gets one declared
//! outcome and one action, from the catalogue rows alone. The names are examples; the same plan
//! handles any row the catalogue holds.

use std::collections::BTreeMap;

use pctwin_recipe::SettingValue;
use pctwin_settings::catalogue::{Catalogue, SettingId};
use pctwin_settings::{
    Action, Arrival, Capture, Choices, Outcome, People, Reading, SettingsPlan, System, Target, plan,
};

const SYSTEMS: [System; 3] = [System::Windows, System::Mac, System::Gnome];

fn text(s: &str) -> SettingValue {
    SettingValue::Text(s.into())
}

/// The picture each system's laptop keeps, in its own spelling.
fn picture(s: System) -> &'static str {
    match s {
        System::Windows => "C:\\Users\\ada\\Pictures\\sea.jpg",
        System::Mac => "/Users/ada/Pictures/sea.jpg",
        _ => "/home/ada/Pictures/sea.jpg",
    }
}

/// What a laptop of system `s` reads for each named setting, as its own place can say it (a
/// setting with no place on `s` is not read at all).
fn capture(cat: &Catalogue, s: System) -> Capture {
    let mut c = Capture::new(s, cat.revision);
    let values = [
        ("appearance.theme", text("dark")),
        ("desktop.wallpaper", text(picture(s))),
        // Windows' slider has 20 steps: 474 is its tenth.
        (
            "pointer.speed",
            SettingValue::Int(if s == System::Windows { 474 } else { 500 }),
        ),
        ("display.scaling", SettingValue::Int(150)),
        ("display.arrangement", text("two screens")),
        ("keyboard.layouts", text("fr,us")),
        ("apps.default-browser", text("firefox")),
    ];
    for (k, v) in values {
        let id = SettingId::new(k).unwrap();
        if cat.row(k).unwrap().places.contains_key(&s) {
            c.readings.insert(id, Reading::Value(v));
        } else {
            c.readings.insert(id, Reading::NotBuilt);
        }
    }
    c
}

fn run(cat: &Catalogue, from: System, to: System) -> SettingsPlan {
    let t = Target {
        roots: vec![
            "C:\\Users\\ada".into(),
            "/Users/ada".into(),
            "/home/ada".into(),
        ],
        apps: ["firefox".to_string()].into(),
        ..Target::default()
    };
    let files: BTreeMap<String, Arrival> = [(
        picture(from).to_string(),
        Arrival::Arrived(picture(to).to_string()),
    )]
    .into();
    let m = vec![("old".to_string(), "new".to_string())];
    let people = People {
        old_account: "old",
        new_account: "new",
        mapped: &m,
    };
    plan(
        cat,
        &capture(cat, from),
        to,
        &t,
        &files,
        &Choices::new(),
        &people,
    )
    .unwrap()
}

/// The action's kind, for the table.
fn kind(a: &Action) -> &'static str {
    match a {
        Action::Apply(_) => "apply",
        Action::AlreadySet => "already-set",
        Action::NeedsChoice { .. } => "choose",
        Action::Handoff(_) => "handoff",
        Action::Guided(_) => "guided",
        Action::Privileged(_) => "privileged",
        Action::OptIn(_) => "opt-in",
        Action::Wait(_) => "wait",
        Action::Leave(_) => "leave",
    }
}

fn s(x: System) -> &'static str {
    match x {
        System::Windows => "windows",
        System::Mac => "mac",
        _ => "gnome",
    }
}

const NAMED: [&str; 7] = [
    "appearance.theme",
    "desktop.wallpaper",
    "pointer.speed",
    "display.scaling",
    "display.arrangement",
    "keyboard.layouts",
    "apps.default-browser",
];

/// Every named setting from every system to every system: (outcome, action) in the order of
/// [`NAMED`]. Same system first in each block.
#[rustfmt::skip]
const EXPECTED: [(&str, &str, [(Outcome, &str); 7]); 9] = {
    use Outcome::{Equivalent as Eq, Exact as Ex, Guided as G};
    [
        // Windows: theme, picture and mouse are written; scaling, screens and layouts are the
        // person's step on the system's page; the browser is handed to the system's own choice.
        ("windows", "windows", [(Ex, "apply"), (Ex, "apply"), (Ex, "apply"), (G, "guided"), (G, "guided"), (G, "guided"), (Ex, "handoff")]),
        ("windows", "mac",     [(Ex, "apply"), (G, "guided"), (Ex, "apply"), (G, "guided"), (G, "guided"), (G, "guided"), (Ex, "handoff")]),
        ("windows", "gnome",   [(Ex, "apply"), (Ex, "apply"), (Ex, "apply"), (G, "guided"), (G, "guided"), (Ex, "apply"), (Ex, "handoff")]),
        // A Mac's picture and scaling have no supported place to read: guided, never "moved".
        ("mac", "windows",     [(Ex, "apply"), (G, "guided"), (Eq, "apply"), (G, "guided"), (G, "guided"), (G, "guided"), (Ex, "handoff")]),
        ("mac", "mac",         [(Ex, "apply"), (G, "guided"), (Ex, "apply"), (G, "guided"), (G, "guided"), (G, "guided"), (Ex, "handoff")]),
        ("mac", "gnome",       [(Ex, "apply"), (G, "guided"), (Ex, "apply"), (G, "guided"), (G, "guided"), (Ex, "apply"), (Ex, "handoff")]),
        ("gnome", "windows",   [(Ex, "apply"), (Ex, "apply"), (Eq, "apply"), (G, "guided"), (G, "guided"), (G, "guided"), (Ex, "handoff")]),
        ("gnome", "mac",       [(Ex, "apply"), (G, "guided"), (Ex, "apply"), (G, "guided"), (G, "guided"), (G, "guided"), (Ex, "handoff")]),
        ("gnome", "gnome",     [(Ex, "apply"), (Ex, "apply"), (Ex, "apply"), (G, "guided"), (G, "guided"), (Ex, "apply"), (Ex, "handoff")]),
    ]
};

#[test]
fn every_named_setting_has_its_declared_outcome_between_every_pair_of_systems() {
    let cat = Catalogue::starter().unwrap();
    for (from, to, want) in EXPECTED {
        let sys = |n: &str| *SYSTEMS.iter().find(|x| s(**x) == n).unwrap();
        let p = run(&cat, sys(from), sys(to));
        for (k, (outcome, action)) in NAMED.iter().zip(want) {
            let one = p.settings.iter().find(|x| x.id.as_str() == *k).unwrap();
            assert_eq!(
                (one.outcome, kind(&one.action)),
                (outcome, action),
                "{k}: {from} to {to}: {:?}",
                one.action
            );
        }
    }
}

#[test]
fn a_picture_set_by_hand_is_shown_where_it_landed_and_a_mac_reading_claims_nothing() {
    let cat = Catalogue::starter().unwrap();
    let one = |from, to, k: &str| {
        run(&cat, from, to)
            .settings
            .into_iter()
            .find(|x| x.id.as_str() == k)
            .unwrap()
            .action
    };
    // The person sets the picture on the Mac: shown at its new place, not the old Windows path.
    assert_eq!(
        one(System::Windows, System::Mac, "desktop.wallpaper"),
        Action::Guided(Some(text("/Users/ada/Pictures/sea.jpg")))
    );
    // Written where the new system can hold it, at its new place.
    assert_eq!(
        one(System::Gnome, System::Windows, "desktop.wallpaper"),
        Action::Apply(text(picture(System::Windows)))
    );
    // Nothing was read on the Mac, so nothing old is shown or claimed.
    assert_eq!(
        one(System::Mac, System::Windows, "desktop.wallpaper"),
        Action::Guided(None)
    );
    assert_eq!(
        one(System::Mac, System::Gnome, "display.scaling"),
        Action::Guided(None)
    );
    // A position Windows' own slider holds is written exactly on Windows.
    assert_eq!(
        one(System::Windows, System::Windows, "pointer.speed"),
        Action::Apply(SettingValue::Int(474))
    );
}

#[test]
fn screens_are_always_arranged_by_the_person() {
    // The old laptop's screens include its own built-in one, which is never the new laptop's:
    // a saved arrangement (Windows' display configuration, a Mac's display preferences, GNOME's
    // monitors.xml) only ever applies to the exact screens it was saved for. So it is a guided
    // step everywhere, with the system's page where it has one.
    let cat = Catalogue::starter().unwrap();
    let row = cat.row("display.arrangement").unwrap();
    assert!(row.places.is_empty());
    assert!(row.pages.contains_key(&System::Windows) && row.pages.contains_key(&System::Mac));
}
