//! Planning: old values to new, with one declared outcome each (T24-004, T24-006, T24-007,
//! T24-008, T24-013, T24-014, T24-016, T24-023, T24-024).

use std::collections::BTreeMap;

use pctwin_recipe::{Os, SettingValue, Step};
use pctwin_settings::map::{
    APPROVE_AGAIN, Dependency, KEPT_STRONGER, MANAGED, NO_AUTO, NO_BROWSER, NO_PRINTER, OUTSIDE,
    PROTECTION_KEPT, inside,
};
use pctwin_settings::{
    Action, Arrival, Capture, Choice, Choices, Outcome, People, PlanError, Planned, Protected,
    Reading, SecurityControl, SettingId, SettingsPlan, System, Target, plan,
};

fn text(s: &str) -> SettingValue {
    SettingValue::Text(s.into())
}

fn mapped() -> Vec<(String, String)> {
    vec![("S-1-5-21-1-1001".into(), "S-1-5-21-2-1001".into())]
}

fn capture(system: System, values: Vec<(SettingId, SettingValue)>) -> Capture {
    let mut c = Capture::new(system);
    for (id, v) in values {
        c.readings.insert(id, Reading::Value(v));
    }
    c
}

fn target() -> Target {
    Target {
        roots: vec!["C:\\Users\\ada".into(), "/home/ada".into()],
        browsers: ["firefox".to_string(), "edge".to_string()].into(),
        printers: ["Office".to_string()].into(),
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
    plan(c, to, t, files, choices, &people).unwrap()
}

fn one(p: &SettingsPlan, id: SettingId) -> &Planned {
    p.settings.iter().find(|s| s.id == id).unwrap()
}

fn simple(c: &Capture, to: System) -> SettingsPlan {
    run(c, to, &target(), &BTreeMap::new(), &Choices::new())
}

#[test]
fn every_setting_on_the_list_gets_exactly_one_plan() {
    let p = simple(&Capture::new(System::Windows), System::Windows);
    assert_eq!(p.settings.len(), SettingId::ALL.len());
    for s in &p.settings {
        assert!(
            !matches!(s.action, Action::Apply(_)),
            "nothing read, nothing applied: {s:?}"
        );
    }
}

#[test]
fn same_system_values_are_exact_and_applied() {
    let c = capture(
        System::Windows,
        vec![
            (SettingId::Theme, text("dark")),
            (SettingId::TaskbarTheme, text("light")),
            (SettingId::PointerSpeed, SettingValue::Int(474)),
            (SettingId::TextSize, SettingValue::Int(150)),
        ],
    );
    let p = simple(&c, System::Windows);
    for (id, v) in [
        (SettingId::Theme, text("dark")),
        (SettingId::TaskbarTheme, text("light")),
        (SettingId::PointerSpeed, SettingValue::Int(474)),
        (SettingId::TextSize, SettingValue::Int(150)),
    ] {
        let s = one(&p, id);
        assert_eq!(s.outcome, Outcome::Exact, "{id:?}");
        assert_eq!(s.action, Action::Apply(v), "{id:?}");
    }
}

#[test]
fn a_value_already_there_is_not_written_again() {
    let c = capture(System::Windows, vec![(SettingId::Theme, text("dark"))]);
    let mut t = target();
    t.current
        .insert(SettingId::Theme, Reading::Value(text("dark")));
    let p = run(&c, System::Windows, &t, &BTreeMap::new(), &Choices::new());
    assert_eq!(one(&p, SettingId::Theme).action, Action::AlreadySet);
    assert_eq!(one(&p, SettingId::Theme).before, Some(text("dark")));
    assert_eq!(one(&p, SettingId::Theme).recipe(Os::Windows).unwrap(), None);
}

#[test]
fn t24_006_an_approximate_value_is_labelled_and_waits_for_the_persons_choice() {
    // A Mac's automatic appearance has no Windows equivalent.
    let c = capture(System::Mac, vec![(SettingId::Theme, text("auto"))]);
    let p = simple(&c, System::Windows);
    let s = one(&p, SettingId::Theme);
    assert_eq!(s.outcome, Outcome::Equivalent);
    assert_eq!(
        s.action,
        Action::NeedsChoice {
            options: vec![text("light"), text("dark")],
            why: NO_AUTO
        }
    );
    // The person picks dark: applied, still labelled equivalent.
    let choices: Choices = [(SettingId::Theme, Choice::Use(text("dark")))].into();
    let p = run(&c, System::Windows, &target(), &BTreeMap::new(), &choices);
    let s = one(&p, SettingId::Theme);
    assert_eq!(
        (s.outcome, &s.action),
        (Outcome::Equivalent, &Action::Apply(text("dark")))
    );
    // A pick that was not offered is ignored.
    let choices: Choices = [(SettingId::Theme, Choice::Use(text("auto")))].into();
    let p = run(&c, System::Windows, &target(), &BTreeMap::new(), &choices);
    assert!(matches!(
        one(&p, SettingId::Theme).action,
        Action::NeedsChoice { .. }
    ));
    // Automatic stays exact on a Mac.
    let p = simple(&c, System::Mac);
    assert_eq!(one(&p, SettingId::Theme).outcome, Outcome::Exact);
}

#[test]
fn pointer_speed_across_systems_is_an_equivalent_the_new_system_can_hold() {
    // A Mac slider position the Windows slider cannot hold exactly.
    let c = capture(
        System::Mac,
        vec![(SettingId::PointerSpeed, SettingValue::Int(229))],
    );
    let p = simple(&c, System::Windows);
    let s = one(&p, SettingId::PointerSpeed);
    assert_eq!(s.outcome, Outcome::Equivalent);
    assert_eq!(s.action, Action::Apply(SettingValue::Int(211)));
    // Nothing PCTwin writes on KDE: guided.
    let p = simple(&c, System::Kde);
    assert_eq!(one(&p, SettingId::PointerSpeed).outcome, Outcome::Guided);
}

#[test]
fn t24_007_a_desktop_picture_waits_for_its_file_and_is_never_applied_early() {
    let old = "C:\\Users\\ada\\OneDrive\\Pictures\\sea.jpg";
    let c = capture(System::Windows, vec![(SettingId::Wallpaper, text(old))]);
    let mut files = BTreeMap::new();
    files.insert(old.to_string(), Arrival::NotYet);
    let p = run(&c, System::Windows, &target(), &files, &Choices::new());
    let s = one(&p, SettingId::Wallpaper);
    assert_eq!(s.action, Action::Wait(Dependency::File(old.into())));
    assert_eq!(s.recipe(Os::Windows).unwrap(), None);
    // Arrived: applied at its new place.
    let new = "C:\\Users\\ada\\Pictures\\sea.jpg";
    files.insert(old.to_string(), Arrival::Arrived(new.into()));
    let p = run(&c, System::Windows, &target(), &files, &Choices::new());
    assert_eq!(
        one(&p, SettingId::Wallpaper).action,
        Action::Apply(text(new))
    );
    // Not part of the move: left.
    let p = simple(&c, System::Windows);
    assert!(matches!(
        one(&p, SettingId::Wallpaper).action,
        Action::Leave(_)
    ));
}

#[test]
fn t24_024_a_picture_outside_the_persons_folders_is_refused() {
    let old = "C:\\Users\\ada\\Pictures\\sea.jpg";
    let c = capture(System::Windows, vec![(SettingId::Wallpaper, text(old))]);
    for escaped in [
        "C:\\Windows\\System32\\evil.jpg",
        "C:\\Users\\adam\\Pictures\\sea.jpg",
        "C:\\Users\\ada",
        "C:\\Users\\ada\\..\\bob\\x.jpg",
    ] {
        let files: BTreeMap<_, _> = [(old.to_string(), Arrival::Arrived(escaped.into()))].into();
        let p = run(&c, System::Windows, &target(), &files, &Choices::new());
        assert_eq!(
            one(&p, SettingId::Wallpaper).action,
            Action::Leave(OUTSIDE),
            "{escaped}"
        );
    }
    assert!(inside(
        "c:\\users\\ADA\\x.jpg",
        &["C:\\Users\\ada\\".into()]
    ));
    assert!(inside("/home/ada/p/x.jpg", &["/home/ada/".into()]));
    assert!(!inside("/home/ADA/p/x.jpg", &["/home/ada".into()]));
    assert!(!inside("/home/ada/x.jpg", &["".into()]));
    assert!(!inside("/home/ada/x.jpg", &[]));
}

#[test]
fn t24_013_a_setting_the_organisation_manages_is_left_alone() {
    let c = capture(
        System::Windows,
        vec![
            (
                SettingId::Wallpaper,
                text("C:\\Users\\ada\\Pictures\\a.jpg"),
            ),
            (SettingId::Theme, text("dark")),
        ],
    );
    let mut t = target();
    t.managed.insert(SettingId::Theme);
    let p = run(&c, System::Windows, &t, &BTreeMap::new(), &Choices::new());
    let s = one(&p, SettingId::Theme);
    assert_eq!(
        (s.outcome, &s.action),
        (Outcome::Unsupported, &Action::Leave(MANAGED))
    );
    // The old laptop's managed values are not the person's choice either.
    let mut c = Capture::new(System::Windows);
    c.readings.insert(SettingId::Theme, Reading::Managed);
    let p = simple(&c, System::Windows);
    assert!(matches!(one(&p, SettingId::Theme).action, Action::Leave(_)));
}

#[test]
fn t24_014_and_t24_023_privacy_approvals_and_switched_off_protections_are_never_copied() {
    let mut c = Capture::new(System::Windows);
    c.protected.insert(Protected::CameraAccess);
    c.protected.insert(Protected::MicrophoneAccess);
    c.protected
        .insert(Protected::SecurityOff(SecurityControl::SmartScreen));
    let p = simple(&c, System::Windows);
    assert_eq!(
        p.protected,
        vec![
            (Protected::CameraAccess, APPROVE_AGAIN),
            (Protected::MicrophoneAccess, APPROVE_AGAIN),
            (
                Protected::SecurityOff(SecurityControl::SmartScreen),
                PROTECTION_KEPT
            ),
        ]
    );
    assert!(
        p.settings
            .iter()
            .all(|s| !matches!(s.action, Action::Apply(_)))
    );
}

#[test]
fn t24_004_settings_go_only_to_the_person_the_move_mapped() {
    let c = capture(System::Windows, vec![(SettingId::Theme, text("dark"))]);
    let m = mapped();
    // The same user name on the new laptop, but another account.
    let people = People {
        old_account: "S-1-5-21-1-1001",
        new_account: "S-1-5-21-2-1002",
        mapped: &m,
    };
    assert_eq!(
        plan(
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
fn t24_008_display_scaling_is_guided_and_positions_are_never_copied() {
    let c = capture(
        System::Windows,
        vec![(SettingId::DisplayScaling, SettingValue::Int(150))],
    );
    let p = simple(&c, System::Windows);
    let s = one(&p, SettingId::DisplayScaling);
    assert_eq!(s.outcome, Outcome::Guided);
    assert_eq!(s.action, Action::Guided(Some(SettingValue::Int(150))));
    assert_eq!(SettingId::from_key("display.positions"), None);
    assert_eq!(SettingId::from_key("display.layout"), None);
}

#[test]
fn t24_016_a_printer_that_is_not_there_must_be_chosen_again() {
    let c = capture(
        System::Windows,
        vec![(SettingId::DefaultPrinter, text("Home Laser"))],
    );
    let p = simple(&c, System::Windows);
    assert_eq!(
        one(&p, SettingId::DefaultPrinter).action,
        Action::NeedsChoice {
            options: vec![text("Office")],
            why: NO_PRINTER
        }
    );
    let c = capture(
        System::Windows,
        vec![(SettingId::DefaultPrinter, text("Office"))],
    );
    let p = simple(&c, System::Windows);
    assert_eq!(
        one(&p, SettingId::DefaultPrinter).action,
        Action::Guided(Some(text("Office")))
    );
}

#[test]
fn the_default_browser_is_handed_to_the_system_never_set() {
    let c = capture(
        System::Windows,
        vec![(SettingId::DefaultBrowser, text("firefox"))],
    );
    let p = simple(&c, System::Windows);
    let s = one(&p, SettingId::DefaultBrowser);
    assert_eq!(
        (s.outcome, &s.action),
        (Outcome::Exact, &Action::Handoff(text("firefox")))
    );
    assert_eq!(s.recipe(Os::Windows).unwrap(), None);
    // Not installed yet: waits for 2.1.
    let c = capture(
        System::Windows,
        vec![(SettingId::DefaultBrowser, text("chrome"))],
    );
    let p = simple(&c, System::Windows);
    assert_eq!(
        one(&p, SettingId::DefaultBrowser).action,
        Action::Wait(Dependency::App("chrome".into()))
    );
    // Safari on Windows: the person picks one the new laptop has.
    let c = capture(
        System::Mac,
        vec![(SettingId::DefaultBrowser, text("safari"))],
    );
    let p = simple(&c, System::Windows);
    assert_eq!(
        one(&p, SettingId::DefaultBrowser).action,
        Action::NeedsChoice {
            options: vec![text("edge"), text("firefox")],
            why: NO_BROWSER
        }
    );
    let choices: Choices = [(SettingId::DefaultBrowser, Choice::Use(text("edge")))].into();
    let p = run(&c, System::Windows, &target(), &BTreeMap::new(), &choices);
    let s = one(&p, SettingId::DefaultBrowser);
    assert_eq!(
        (s.outcome, &s.action),
        (Outcome::Equivalent, &Action::Handoff(text("edge")))
    );
}

#[test]
fn the_new_laptops_stronger_accessibility_setting_is_kept_unless_allowed() {
    let c = capture(
        System::Windows,
        vec![(SettingId::TextSize, SettingValue::Int(100))],
    );
    let mut t = target();
    t.current
        .insert(SettingId::TextSize, Reading::Value(SettingValue::Int(175)));
    let p = run(&c, System::Windows, &t, &BTreeMap::new(), &Choices::new());
    assert_eq!(
        one(&p, SettingId::TextSize).action,
        Action::NeedsChoice {
            options: vec![SettingValue::Int(100)],
            why: KEPT_STRONGER
        }
    );
    let allow: Choices = [(SettingId::TextSize, Choice::Allow)].into();
    let p = run(&c, System::Windows, &t, &BTreeMap::new(), &allow);
    assert_eq!(
        one(&p, SettingId::TextSize).action,
        Action::Apply(SettingValue::Int(100))
    );
    // Raising it needs no question.
    let c = capture(
        System::Windows,
        vec![(SettingId::TextSize, SettingValue::Int(200))],
    );
    let p = run(&c, System::Windows, &t, &BTreeMap::new(), &Choices::new());
    assert_eq!(
        one(&p, SettingId::TextSize).action,
        Action::Apply(SettingValue::Int(200))
    );
    // Fewer animations on the new laptop is kept too.
    let c = capture(
        System::Gnome,
        vec![(SettingId::ReduceMotion, SettingValue::Bool(false))],
    );
    let mut t = target();
    t.current.insert(
        SettingId::ReduceMotion,
        Reading::Value(SettingValue::Bool(true)),
    );
    let p = run(&c, System::Gnome, &t, &BTreeMap::new(), &Choices::new());
    assert!(matches!(
        one(&p, SettingId::ReduceMotion).action,
        Action::NeedsChoice { .. }
    ));
    // High contrast is always the person's own step.
    let c = capture(
        System::Windows,
        vec![(SettingId::HighContrast, SettingValue::Bool(true))],
    );
    let p = simple(&c, System::Windows);
    assert_eq!(one(&p, SettingId::HighContrast).outcome, Outcome::Guided);
}

#[test]
fn keep_leaves_the_new_laptops_value() {
    let c = capture(System::Windows, vec![(SettingId::Theme, text("dark"))]);
    let keep: Choices = [(SettingId::Theme, Choice::Keep)].into();
    let p = run(&c, System::Windows, &target(), &BTreeMap::new(), &keep);
    assert!(matches!(one(&p, SettingId::Theme).action, Action::Leave(_)));
}

#[test]
fn keyboard_layouts_are_only_added_never_taken_away() {
    let c = capture(
        System::Windows,
        vec![(SettingId::KeyboardLayouts, text("fr,us,ar"))],
    );
    let mut t = target();
    t.current
        .insert(SettingId::KeyboardLayouts, Reading::Value(text("us,de")));
    let p = run(&c, System::Gnome, &t, &BTreeMap::new(), &Choices::new());
    assert_eq!(
        one(&p, SettingId::KeyboardLayouts).action,
        Action::Apply(text("us,de,fr,ar"))
    );
    // A layout only Windows knows: guided on GNOME.
    let c = capture(
        System::Windows,
        vec![(SettingId::KeyboardLayouts, text("windows:0000041E"))],
    );
    let p = run(&c, System::Gnome, &t, &BTreeMap::new(), &Choices::new());
    assert_eq!(one(&p, SettingId::KeyboardLayouts).outcome, Outcome::Guided);
    // On Windows the person adds them in Settings.
    let c = capture(
        System::Windows,
        vec![(SettingId::KeyboardLayouts, text("fr"))],
    );
    let p = simple(&c, System::Windows);
    assert_eq!(one(&p, SettingId::KeyboardLayouts).outcome, Outcome::Guided);
}

#[test]
fn time_zone_goes_to_the_helper_and_startup_apps_are_their_own_choice() {
    let c = capture(
        System::Windows,
        vec![
            (SettingId::TimeZone, text("Africa/Lagos")),
            (SettingId::StartupApps, text("spotify,teams")),
        ],
    );
    let p = simple(&c, System::Windows);
    assert_eq!(
        one(&p, SettingId::TimeZone).action,
        Action::Privileged(text("Africa/Lagos"))
    );
    assert_eq!(
        one(&p, SettingId::StartupApps).action,
        Action::OptIn(text("spotify,teams"))
    );
    let allow: Choices = [(SettingId::StartupApps, Choice::Allow)].into();
    let p = run(&c, System::Windows, &target(), &BTreeMap::new(), &allow);
    assert_eq!(
        one(&p, SettingId::StartupApps).action,
        Action::Guided(Some(text("spotify,teams")))
    );
}

#[test]
fn not_applicable_on_the_new_system() {
    let c = capture(
        System::Windows,
        vec![(SettingId::TaskbarTheme, text("dark"))],
    );
    let p = simple(&c, System::Mac);
    assert_eq!(
        one(&p, SettingId::TaskbarTheme).outcome,
        Outcome::NotApplicable
    );
}

#[test]
fn a_hostile_capture_value_is_refused_before_planning() {
    let mut c = capture(
        System::Windows,
        vec![(SettingId::TextSize, SettingValue::Int(9000))],
    );
    let m = mapped();
    let people = People {
        old_account: "S-1-5-21-1-1001",
        new_account: "S-1-5-21-2-1001",
        mapped: &m,
    };
    assert!(matches!(
        plan(
            &c,
            System::Windows,
            &target(),
            &BTreeMap::new(),
            &Choices::new(),
            &people
        ),
        Err(PlanError::Schema(_))
    ));
    c.readings.clear();
    c.schema = 2;
    assert_eq!(
        plan(
            &c,
            System::Windows,
            &target(),
            &BTreeMap::new(),
            &Choices::new(),
            &people
        ),
        Err(PlanError::NewerSchema(2))
    );
}

#[test]
fn a_saved_capture_is_read_back_and_checked() {
    let c = capture(System::Gnome, vec![(SettingId::Theme, text("dark"))]);
    let bytes = serde_json::to_vec(&c).unwrap();
    assert_eq!(Capture::from_json(&bytes).unwrap(), c);
    let bad = String::from_utf8(bytes)
        .unwrap()
        .replace("\"dark\"", "\"blue\"");
    assert!(matches!(
        Capture::from_json(bad.as_bytes()),
        Err(PlanError::Schema(_))
    ));
    let newer = r#"{"schema":2,"system":"gnome","readings":{}}"#;
    assert_eq!(
        Capture::from_json(newer.as_bytes()),
        Err(PlanError::NewerSchema(2))
    );
    let zero = r#"{"schema":0,"system":"gnome","readings":{}}"#;
    assert_eq!(
        Capture::from_json(zero.as_bytes()),
        Err(PlanError::NewerSchema(0))
    );
    let unknown = r#"{"schema":1,"system":"gnome","readings":{"security.firewall":{"value":{"bool":false}}}}"#;
    assert!(matches!(
        Capture::from_json(unknown.as_bytes()),
        Err(PlanError::Damaged(_))
    ));
    assert_eq!(c.coverage(), (1, SettingId::ALL.len()));
}

#[test]
fn an_applied_setting_has_one_checked_recipe_step() {
    let c = capture(System::Windows, vec![(SettingId::Theme, text("dark"))]);
    let p = simple(&c, System::Windows);
    let r = one(&p, SettingId::Theme)
        .recipe(Os::Windows)
        .unwrap()
        .unwrap();
    assert_eq!(r.id.as_str(), "pctwin.settings.appearance.theme");
    assert_eq!(r.version, pctwin_settings::SCHEMA_VERSION);
    assert_eq!(
        r.steps,
        vec![Step::ApplySetting {
            key: "appearance.theme".into(),
            value: text("dark")
        }]
    );
    assert_eq!(r.platforms.len(), 2);
    assert!(r.platforms.iter().all(|pl| pl.os == Os::Windows));
}
