//! Translating catalogue settings between laptops, with one declared outcome each (T24-004,
//! T24-006, T24-007, T24-008, T24-010, T24-013, T24-014, T24-016, T24-023, T24-024).

use std::collections::BTreeMap;

use pctwin_recipe::{Os, SettingValue, Step};
use pctwin_settings::catalogue::{Catalogue, Protected, SettingId};
use pctwin_settings::map::{
    APPROVE_AGAIN, CHOOSE, Capture, Dependency, KEPT_STRONGER, MANAGED, NO_APP, NO_DEVICE, OUTSIDE,
    PROTECTION_KEPT, PlanError,
};
use pctwin_settings::value::inside;
use pctwin_settings::{
    Action, Arrival, Choice, Choices, Outcome, People, Planned, Reading, SettingsPlan, System,
    Target, plan,
};

fn text(s: &str) -> SettingValue {
    SettingValue::Text(s.into())
}

fn id(s: &str) -> SettingId {
    SettingId::new(s).unwrap()
}

fn cat() -> Catalogue {
    Catalogue::starter().unwrap()
}

fn mapped() -> Vec<(String, String)> {
    vec![("S-1-5-21-1-1001".into(), "S-1-5-21-2-1001".into())]
}

fn capture(system: System, values: Vec<(&str, SettingValue)>) -> Capture {
    let mut c = Capture::new(system, 1);
    for (k, v) in values {
        c.readings.insert(id(k), Reading::Value(v));
    }
    c
}

fn target() -> Target {
    Target {
        roots: vec!["C:\\Users\\ada".into(), "/home/ada".into()],
        apps: ["firefox".to_string(), "edge".to_string()].into(),
        devices: ["Office".to_string()].into(),
        ..Target::default()
    }
}

fn run(
    c: &Capture,
    to: System,
    t: &Target,
    files: &BTreeMap<String, Arrival>,
    choices: &Choices,
) -> SettingsPlan {
    let m = mapped();
    let people = People {
        old_account: "S-1-5-21-1-1001",
        new_account: "S-1-5-21-2-1001",
        mapped: &m,
    };
    plan(&cat(), c, to, t, files, choices, &people).unwrap()
}

fn one<'a>(p: &'a SettingsPlan, k: &str) -> &'a Planned {
    p.settings.iter().find(|s| s.id.as_str() == k).unwrap()
}

fn simple(c: &Capture, to: System) -> SettingsPlan {
    run(c, to, &target(), &BTreeMap::new(), &Choices::new())
}

#[test]
fn every_row_gets_one_plan_and_nothing_unread_is_applied() {
    let p = simple(&Capture::new(System::Windows, 1), System::Windows);
    assert_eq!(p.settings.len(), cat().settings.len());
    assert!(
        p.settings
            .iter()
            .all(|s| !matches!(s.action, Action::Apply(_)))
    );
}

#[test]
fn same_system_values_are_exact() {
    let c = capture(
        System::Windows,
        vec![
            ("appearance.theme", text("dark")),
            ("appearance.taskbar-theme", text("light")),
            ("pointer.speed", SettingValue::Int(474)),
            ("accessibility.text-size", SettingValue::Int(150)),
        ],
    );
    let p = simple(&c, System::Windows);
    for (k, v) in [
        ("appearance.theme", text("dark")),
        ("appearance.taskbar-theme", text("light")),
        ("pointer.speed", SettingValue::Int(474)),
        ("accessibility.text-size", SettingValue::Int(150)),
    ] {
        assert_eq!(
            (one(&p, k).outcome, &one(&p, k).action),
            (Outcome::Exact, &Action::Apply(v)),
            "{k}"
        );
    }
}

#[test]
fn a_value_already_there_is_not_written_again() {
    let c = capture(System::Windows, vec![("appearance.theme", text("dark"))]);
    let mut t = target();
    t.current
        .insert(id("appearance.theme"), Reading::Value(text("dark")));
    let p = run(&c, System::Windows, &t, &BTreeMap::new(), &Choices::new());
    assert_eq!(one(&p, "appearance.theme").action, Action::AlreadySet);
    assert_eq!(
        one(&p, "appearance.theme").recipe(Os::Windows, 1).unwrap(),
        None
    );
}

#[test]
fn t24_006_a_meaning_that_changes_waits_for_the_persons_choice() {
    let c = capture(System::Mac, vec![("appearance.theme", text("auto"))]);
    let p = simple(&c, System::Windows);
    let s = one(&p, "appearance.theme");
    assert_eq!(s.outcome, Outcome::Equivalent);
    assert_eq!(
        s.action,
        Action::NeedsChoice {
            options: vec![text("light"), text("dark")],
            why: CHOOSE
        }
    );
    let choose = |v| -> Choices { [(id("appearance.theme"), Choice::Use(v))].into() };
    let p = run(
        &c,
        System::Windows,
        &target(),
        &BTreeMap::new(),
        &choose(text("dark")),
    );
    let s = one(&p, "appearance.theme");
    assert_eq!(
        (s.outcome, &s.action),
        (Outcome::Equivalent, &Action::Apply(text("dark")))
    );
    // A pick that was not offered changes nothing.
    let p = run(
        &c,
        System::Windows,
        &target(),
        &BTreeMap::new(),
        &choose(text("auto")),
    );
    assert!(matches!(
        one(&p, "appearance.theme").action,
        Action::NeedsChoice { .. }
    ));
    // A Mac keeps automatic exactly.
    assert_eq!(
        one(&simple(&c, System::Mac), "appearance.theme").outcome,
        Outcome::Exact
    );
}

#[test]
fn a_slider_becomes_the_nearest_position_the_new_system_holds() {
    let c = capture(System::Mac, vec![("pointer.speed", SettingValue::Int(229))]);
    let p = simple(&c, System::Windows);
    let s = one(&p, "pointer.speed");
    assert_eq!(
        (s.outcome, &s.action),
        (Outcome::Equivalent, &Action::Apply(SettingValue::Int(211)))
    );
    assert_eq!(
        one(&simple(&c, System::Kde), "pointer.speed").outcome,
        Outcome::Guided
    );
    // Every Windows step reads back as itself.
    for step in 1..=20i64 {
        let neutral = ((step - 1) as f64 * 1000.0 / 19.0).round() as i64;
        let c = capture(
            System::Windows,
            vec![("pointer.speed", SettingValue::Int(neutral))],
        );
        let p = simple(&c, System::Windows);
        assert_eq!(
            one(&p, "pointer.speed").outcome,
            Outcome::Exact,
            "step {step}"
        );
    }
}

#[test]
fn t24_007_a_desktop_picture_waits_for_its_file() {
    let old = "C:\\Users\\ada\\OneDrive\\Pictures\\sea.jpg";
    let c = capture(System::Windows, vec![("desktop.wallpaper", text(old))]);
    let mut files = BTreeMap::new();
    files.insert(old.to_string(), Arrival::NotYet);
    let p = run(&c, System::Windows, &target(), &files, &Choices::new());
    assert_eq!(
        one(&p, "desktop.wallpaper").action,
        Action::Wait(Dependency::File(old.into()))
    );
    let new = "C:\\Users\\ada\\Pictures\\sea.jpg";
    files.insert(old.to_string(), Arrival::Arrived(new.into()));
    let p = run(&c, System::Windows, &target(), &files, &Choices::new());
    assert_eq!(
        one(&p, "desktop.wallpaper").action,
        Action::Apply(text(new))
    );
}

#[test]
fn t24_024_a_picture_outside_the_persons_folders_is_refused() {
    let old = "C:\\Users\\ada\\Pictures\\sea.jpg";
    let c = capture(System::Windows, vec![("desktop.wallpaper", text(old))]);
    for escaped in [
        "C:\\Windows\\System32\\evil.jpg",
        "C:\\Users\\adam\\Pictures\\sea.jpg",
        "C:\\Users\\ada",
        "C:\\Users\\ada\\..\\bob\\x.jpg",
    ] {
        let files: BTreeMap<_, _> = [(old.to_string(), Arrival::Arrived(escaped.into()))].into();
        let p = run(&c, System::Windows, &target(), &files, &Choices::new());
        assert_eq!(
            one(&p, "desktop.wallpaper").action,
            Action::Leave(OUTSIDE),
            "{escaped}"
        );
    }
    assert!(inside(
        "c:\\users\\ADA\\x.jpg",
        &["C:\\Users\\ada\\".into()]
    ));
    assert!(!inside("/home/ADA/p/x.jpg", &["/home/ada".into()]));
    assert!(!inside("/home/ada/x.jpg", &["".into()]));
}

#[test]
fn t24_013_a_setting_the_organisation_manages_is_left_alone() {
    let c = capture(System::Windows, vec![("appearance.theme", text("dark"))]);
    let mut t = target();
    t.managed.insert(id("appearance.theme"));
    let p = run(&c, System::Windows, &t, &BTreeMap::new(), &Choices::new());
    let s = one(&p, "appearance.theme");
    assert_eq!(
        (s.outcome, &s.action),
        (Outcome::Unsupported, &Action::Leave(MANAGED))
    );
}

#[test]
fn t24_014_and_t24_023_approvals_and_protections_are_listed_never_copied() {
    let mut c = Capture::new(System::Windows, 1);
    c.protected.insert(Protected::CameraAccess);
    c.protected.insert(Protected::SmartScreenOff);
    let p = simple(&c, System::Windows);
    assert_eq!(
        p.protected,
        vec![
            (Protected::CameraAccess, APPROVE_AGAIN),
            (Protected::SmartScreenOff, PROTECTION_KEPT)
        ]
    );
}

#[test]
fn t24_004_settings_go_only_to_the_mapped_person() {
    let c = capture(System::Windows, vec![("appearance.theme", text("dark"))]);
    let m = mapped();
    let people = People {
        old_account: "S-1-5-21-1-1001",
        new_account: "S-1-5-21-2-1002",
        mapped: &m,
    };
    assert_eq!(
        plan(
            &cat(),
            &c,
            System::Windows,
            &target(),
            &BTreeMap::new(),
            &Choices::new(),
            &people
        ),
        Err(PlanError::OwnerMismatch)
    );
}

#[test]
fn t24_008_display_scaling_is_guided() {
    let c = capture(
        System::Windows,
        vec![("display.scaling", SettingValue::Int(150))],
    );
    let s = one(&simple(&c, System::Windows), "display.scaling").clone();
    assert_eq!(
        (s.outcome, s.action),
        (
            Outcome::Guided,
            Action::Guided(Some(SettingValue::Int(150)))
        )
    );
    assert!(cat().row("display.positions").is_none());
}

#[test]
fn t24_016_a_missing_printer_is_chosen_again() {
    let c = capture(
        System::Windows,
        vec![("devices.default-printer", text("Home Laser"))],
    );
    assert_eq!(
        one(&simple(&c, System::Windows), "devices.default-printer").action,
        Action::NeedsChoice {
            options: vec![text("Office")],
            why: NO_DEVICE
        }
    );
}

#[test]
fn the_default_browser_is_handed_to_the_system_never_set() {
    let c = capture(
        System::Windows,
        vec![("apps.default-browser", text("firefox"))],
    );
    let s = one(&simple(&c, System::Windows), "apps.default-browser").clone();
    assert_eq!(
        (s.outcome, &s.action),
        (Outcome::Exact, &Action::Handoff(text("firefox")))
    );
    assert_eq!(s.recipe(Os::Windows, 1).unwrap(), None);
    let c = capture(
        System::Windows,
        vec![("apps.default-browser", text("chrome"))],
    );
    assert_eq!(
        one(&simple(&c, System::Windows), "apps.default-browser").action,
        Action::Wait(Dependency::App("chrome".into()))
    );
    let c = capture(System::Mac, vec![("apps.default-browser", text("safari"))]);
    assert_eq!(
        one(&simple(&c, System::Windows), "apps.default-browser").action,
        Action::NeedsChoice {
            options: vec![text("edge"), text("firefox")],
            why: NO_APP
        }
    );
}

#[test]
fn the_new_laptops_stronger_accessibility_is_kept_unless_allowed() {
    let c = capture(
        System::Windows,
        vec![("accessibility.text-size", SettingValue::Int(100))],
    );
    let mut t = target();
    t.current.insert(
        id("accessibility.text-size"),
        Reading::Value(SettingValue::Int(175)),
    );
    let p = run(&c, System::Windows, &t, &BTreeMap::new(), &Choices::new());
    assert_eq!(
        one(&p, "accessibility.text-size").action,
        Action::NeedsChoice {
            options: vec![SettingValue::Int(100)],
            why: KEPT_STRONGER
        }
    );
    let allow: Choices = [(id("accessibility.text-size"), Choice::Allow)].into();
    let p = run(&c, System::Windows, &t, &BTreeMap::new(), &allow);
    assert_eq!(
        one(&p, "accessibility.text-size").action,
        Action::Apply(SettingValue::Int(100))
    );
    let c = capture(
        System::Gnome,
        vec![("accessibility.reduce-motion", SettingValue::Bool(false))],
    );
    let mut t = target();
    t.current.insert(
        id("accessibility.reduce-motion"),
        Reading::Value(SettingValue::Bool(true)),
    );
    let p = run(&c, System::Gnome, &t, &BTreeMap::new(), &Choices::new());
    assert!(matches!(
        one(&p, "accessibility.reduce-motion").action,
        Action::NeedsChoice { .. }
    ));
}

#[test]
fn keyboard_layouts_are_only_added_never_taken_away() {
    let c = capture(
        System::Windows,
        vec![("keyboard.layouts", text("fr,us,ar"))],
    );
    let mut t = target();
    t.current
        .insert(id("keyboard.layouts"), Reading::Value(text("us,de")));
    let p = run(&c, System::Gnome, &t, &BTreeMap::new(), &Choices::new());
    assert_eq!(
        one(&p, "keyboard.layouts").action,
        Action::Apply(text("us,de,fr,ar"))
    );
    let c = capture(
        System::Windows,
        vec![("keyboard.layouts", text("windows:0000041e"))],
    );
    assert_eq!(
        one(
            &run(&c, System::Gnome, &t, &BTreeMap::new(), &Choices::new()),
            "keyboard.layouts"
        )
        .outcome,
        Outcome::Guided
    );
}

#[test]
fn t24_010_right_to_left_and_non_ascii_values_carry_exactly() {
    let path = "C:\\Users\\أحمد\\الصور\\بحر.jpg";
    let c = capture(System::Windows, vec![("desktop.wallpaper", text(path))]);
    let files: BTreeMap<_, _> = [(
        path.to_string(),
        Arrival::Arrived("C:\\Users\\ada\\صور\\بحر.jpg".into()),
    )]
    .into();
    let p = run(&c, System::Windows, &target(), &files, &Choices::new());
    assert_eq!(
        one(&p, "desktop.wallpaper").action,
        Action::Apply(text("C:\\Users\\ada\\صور\\بحر.jpg"))
    );
    let c = capture(
        System::Windows,
        vec![("devices.default-printer", text("طابعة المكتب"))],
    );
    let mut t = target();
    t.devices.insert("طابعة المكتب".into());
    let p = run(&c, System::Windows, &t, &BTreeMap::new(), &Choices::new());
    assert_eq!(
        one(&p, "devices.default-printer").action,
        Action::Guided(Some(text("طابعة المكتب")))
    );
}

#[test]
fn time_zone_goes_to_the_helper_and_startup_apps_are_their_own_choice() {
    let c = capture(
        System::Windows,
        vec![
            ("time.zone", text("Africa/Lagos")),
            ("apps.startup", text("Spotify,Microsoft Teams")),
        ],
    );
    let p = simple(&c, System::Windows);
    assert_eq!(
        one(&p, "time.zone").action,
        Action::Privileged(text("Africa/Lagos"))
    );
    assert_eq!(
        one(&p, "apps.startup").action,
        Action::OptIn(text("Spotify,Microsoft Teams"))
    );
}

#[test]
fn a_hostile_or_unknown_capture_is_refused() {
    let m = mapped();
    let people = People {
        old_account: "S-1-5-21-1-1001",
        new_account: "S-1-5-21-2-1001",
        mapped: &m,
    };
    let c = capture(
        System::Windows,
        vec![("accessibility.text-size", SettingValue::Int(9000))],
    );
    assert!(matches!(
        plan(
            &cat(),
            &c,
            System::Windows,
            &target(),
            &BTreeMap::new(),
            &Choices::new(),
            &people
        ),
        Err(PlanError::Value(_))
    ));
    let mut c = Capture::new(System::Windows, 1);
    c.readings.insert(
        id("security.firewall"),
        Reading::Value(SettingValue::Bool(false)),
    );
    assert_eq!(
        plan(
            &cat(),
            &c,
            System::Windows,
            &target(),
            &BTreeMap::new(),
            &Choices::new(),
            &people
        ),
        Err(PlanError::UnknownSetting("security.firewall".into()))
    );
    let bytes = serde_json::to_vec(&c).unwrap();
    assert!(Capture::from_json(&bytes, &cat()).is_err());
    let mut c = Capture::new(System::Windows, 1);
    c.format = 2;
    assert_eq!(
        Capture::from_json(&serde_json::to_vec(&c).unwrap(), &cat()),
        Err(PlanError::NewerFormat(2))
    );
}

#[test]
fn an_applied_setting_has_one_checked_recipe_step() {
    let c = capture(System::Windows, vec![("appearance.theme", text("dark"))]);
    let r = one(&simple(&c, System::Windows), "appearance.theme")
        .recipe(Os::Windows, 1)
        .unwrap()
        .unwrap();
    assert_eq!(r.id.as_str(), "pctwin.settings.appearance.theme");
    assert_eq!(
        r.steps,
        vec![Step::ApplySetting {
            key: "appearance.theme".into(),
            value: text("dark")
        }]
    );
}
