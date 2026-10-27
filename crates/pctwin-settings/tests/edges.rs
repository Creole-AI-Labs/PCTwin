//! Edges the mutation runs found untested: each test pins one boundary or condition.

mod common;

use std::collections::BTreeMap;

use pctwin_recipe::{Os, SettingValue};
use pctwin_settings::catalogue::{Catalogue, CatalogueError, STARTER, System};
use pctwin_settings::classify::{Ctx, Fate, FolderMap, Why, classify};
use pctwin_settings::codec::{self, Native};
use pctwin_settings::found::{Address, PrefKind, RegKind, discover_registry, setting_to_reg};
use pctwin_settings::map::{CAPTURE_FORMAT, Capture, PlanError};
use pctwin_settings::mechanism::Mechanisms;
use pctwin_settings::registry::{Hive, MemHive, RegValue};
use pctwin_settings::{
    Action, Arrival, Choice, Choices, Outcome, People, Reading, SettingsAdapter, Store, Target,
    plan,
};
use serde_json::{Value, json};

fn text(s: &str) -> SettingValue {
    SettingValue::Text(s.into())
}

fn starter() -> Value {
    serde_json::from_str(STARTER).unwrap()
}

fn load(v: &Value) -> Result<Catalogue, CatalogueError> {
    Catalogue::from_json(&serde_json::to_vec(v).unwrap())
}

fn with_row(row: Value) -> Value {
    let mut c = starter();
    c["settings"].as_array_mut().unwrap().push(row);
    c
}

fn ctx(m: &FolderMap) -> Ctx<'_> {
    Ctx {
        paths: m,
        same_screens: false,
        hints: pctwin_settings::catalogue::Catalogue::starter_hints(),
    }
}

// --- classify ---

#[test]
fn a_rewritten_path_takes_the_new_systems_separators() {
    let m = FolderMap(vec![(
        "C:\\Users\\ada\\code".into(),
        "/Users/ada/code".into(),
    )]);
    assert_eq!(
        classify(
            "Software\\V\\A",
            "Root",
            &text("C:\\Users\\ada\\code\\site\\index.html"),
            &ctx(&m)
        ),
        Fate::Rewritten(text("/Users/ada/code/site/index.html"))
    );
    let m = FolderMap(vec![(
        "/home/ada/code".into(),
        "C:\\Users\\ada\\code".into(),
    )]);
    assert_eq!(
        classify(
            "Software\\V\\A",
            "Root",
            &text("/home/ada/code/site/x"),
            &ctx(&m)
        ),
        Fate::Rewritten(text("C:\\Users\\ada\\code\\site\\x"))
    );
}

#[test]
fn a_list_nothing_changed_in_is_copied_as_it_is() {
    let m = FolderMap::default();
    let list = serde_json::to_string(&["spell", "grammar"]).unwrap();
    assert_eq!(
        classify("Software\\V\\A", "Plugins", &text(&list), &ctx(&m)),
        Fate::Copy(text(&list))
    );
}

#[test]
fn a_name_that_only_contains_exe_is_not_a_program() {
    let m = FolderMap(vec![(
        "C:\\Users\\ada\\Documents".into(),
        "C:\\Users\\ada\\Documents".into(),
    )]);
    for p in [
        "C:\\Users\\ada\\Documents\\setup.exe.notes.txt",
        "\"C:\\Users\\ada\\Documents\\setup.exe.notes.txt\"",
    ] {
        assert!(
            matches!(
                classify("Software\\V\\A", "Doc", &text(p), &ctx(&m)),
                Fate::Rewritten(_)
            ),
            "{p}"
        );
    }
    let m = FolderMap(vec![(
        "C:\\Users\\ada\\Documents".into(),
        "C:\\Users\\ada\\Documents".into(),
    )]);
    assert!(matches!(
        classify(
            "Software\\V\\A",
            "Doc",
            &text("file:///c%3A/Users/ada/Documents/a.exe.txt"),
            &ctx(&m)
        ),
        Fate::Rewritten(_)
    ));
    let m = FolderMap(vec![("/home/ada/P".into(), "/home/ada/P".into())]);
    assert!(matches!(
        classify(
            "Software\\V\\A",
            "Doc",
            &text("file:///home/ada/P/a.exe.txt"),
            &ctx(&m)
        ),
        Fate::Rewritten(_)
    ));
}

#[test]
fn only_a_real_account_id_is_account_bound() {
    let m = FolderMap::default();
    assert_eq!(
        classify(
            "Software\\V\\A",
            "Label",
            &text("S-1-5-21-abc-def-ghi"),
            &ctx(&m)
        ),
        Fate::Copy(text("S-1-5-21-abc-def-ghi"))
    );
}

#[test]
fn random_looking_text_is_judged_from_32_characters_without_spaces() {
    use pctwin_settings::classify::looks_secret;
    let random32 = "Zq3xT9vB2mK7pL4wR8yN1cF6hJ5dG0sA";
    assert_eq!(random32.len(), 32);
    assert_eq!(looks_secret(random32), Some(Why::Secret));
    assert_eq!(looks_secret(&random32[..31]), None);
    assert_eq!(looks_secret("Zq3xT9vB2mK7pL4w R8yN1cF6hJ5dG0sA"), None);
    // A short hex hash (32 characters) is not taken for a key; a 40-character one is.
    assert_eq!(looks_secret("9f86d081884c7d659a2feaa0c55ad015"), None);
    assert_eq!(
        looks_secret("9f86d081884c7d659a2feaa0c55ad015a3bf4f1b"),
        Some(Why::Secret)
    );
}

// --- catalogue ---

fn refused(c: Value) -> bool {
    load(&c).is_err()
}

#[test]
fn catalogue_tables_exceptions_and_labels_must_be_plain() {
    let mut c = starter();
    c["tables"]["layouts"]["columns"][1] = json!("win dows");
    assert!(refused(c));
    let mut c = starter();
    c["exceptions"][0]["reason"] = json!("");
    assert!(refused(c));
    let mut c = starter();
    c["exceptions"][0]["store"] = json!("Software\\x\u{202E}");
    assert!(refused(c));
    let mut c = starter();
    c["labels"][0]["store"] = json!("");
    assert!(refused(c));
    let mut c = starter();
    c["labels"][0]["label"] = json!("x".repeat(121));
    assert!(refused(c));
    let mut c = starter();
    c["checked_on"] = json!("2026x10-27");
    assert!(refused(c));
}

fn gs_row(schema: &str, key: &str) -> Value {
    json!({"id": "x.g", "value": "bool", "places": {"gnome": {"mechanism": "gsettings", "schema": schema,
        "fields": [{"name": key, "kind": "bool"}], "codec": {"codec": "same"}}}})
}

#[test]
fn catalogue_place_names_are_checked_one_by_one() {
    assert!(!refused(with_row(gs_row("org.fake.app", "dark-mode"))));
    assert!(refused(with_row(gs_row("org fake", "dark-mode"))));
    assert!(refused(with_row(gs_row("org.fake.app", "dark mode"))));
    let mac = |d: &str| {
        json!({"id": "x.m", "value": "bool", "places": {"mac": {"mechanism": "mac-pref", "domain": d,
        "fields": [{"name": "K", "kind": "bool"}], "codec": {"codec": "same"}}}})
    };
    assert!(!refused(with_row(mac(&"a".repeat(128)))));
    assert!(refused(with_row(mac(&"a".repeat(129)))));
    let reg = |name: &str| {
        json!({"id": "x.r", "value": "bool", "places": {"windows": {"mechanism": "registry",
        "key": "Software\\Fake", "fields": [{"name": name, "kind": "dword"}], "codec": {"codec": "same"}}}})
    };
    assert!(!refused(with_row(reg("Name"))));
    assert!(refused(with_row(reg("Name "))));
    let lin = |n: [i64; 2]| {
        json!({"id": "x.l", "value": {"int": {"min": 0, "max": 10}}, "places": {"windows": {
        "mechanism": "registry", "key": "Software\\Fake", "fields": [{"name": "N", "kind": "dword"}],
        "codec": {"codec": "linear", "native": [0.0, 1.0], "neutral": n}}}})
    };
    assert!(!refused(with_row(lin([0, 10]))));
    assert!(refused(with_row(lin([5, 5]))));
    let flag = |mask: i64| {
        json!({"id": "x.f", "value": "bool", "places": {"windows": {"mechanism": "registry",
        "key": "Software\\Fake", "fields": [{"name": "F", "kind": "dword"}], "codec": {"codec": "flag", "mask": mask}, "read_only": true}}})
    };
    assert!(!refused(with_row(flag(1))));
    assert!(refused(with_row(flag(0))));
    let lit = |s: String| {
        json!({"id": "x.t", "value": {"choice": {"options": ["a"]}}, "places": {"windows": {"mechanism": "registry",
        "key": "Software\\Fake", "fields": [{"name": "T", "kind": "sz"}], "codec": {"codec": "table", "rows": [{"native": [s], "neutral": "a"}]}}}})
    };
    assert!(!refused(with_row(lit("x".repeat(256)))));
    assert!(refused(with_row(lit("x".repeat(257)))));
}

// --- codec ---

fn row_place(
    c: &Catalogue,
    id: &str,
    s: System,
) -> (
    pctwin_settings::catalogue::Row,
    pctwin_settings::catalogue::Place,
) {
    let r = c.row(id).unwrap().clone();
    let p = r.places.get(&s).unwrap().clone();
    (r, p)
}

#[test]
fn codecs_at_their_edges() {
    let c = Catalogue::starter().unwrap();
    // High contrast is bit 1 of Flags.
    let (r, p) = row_place(&c, "accessibility.high-contrast", System::Windows);
    assert_eq!(
        codec::decode(&c, &r, &p, &[Some(Native::Int(127))]),
        Ok(Some(SettingValue::Bool(true)))
    );
    assert_eq!(
        codec::decode(&c, &r, &p, &[Some(Native::Int(126))]),
        Ok(Some(SettingValue::Bool(false)))
    );
    // Lookups match whole names: `us-intl` is not `us` with a suffix.
    let (r, p) = row_place(&c, "keyboard.layouts", System::Gnome);
    assert_eq!(
        codec::encode(&c, &r, &p, &text("us-intl")),
        Ok(vec![Some(Native::List(vec!["us+intl".into()]))])
    );
    let (r, p) = row_place(&c, "keyboard.layouts", System::Windows);
    assert_eq!(
        codec::decode(&c, &r, &p, &[Some(Native::List(vec!["00000409-1".into()]))]),
        Ok(Some(text("windows:00000409-1")))
    );
    assert!(codec::decode(&c, &r, &p, &[Some(Native::List(vec!["0000 0409".into()]))]).is_err());
    let (r, p) = row_place(&c, "time.zone", System::Windows);
    assert!(codec::encode(&c, &r, &p, &text("Africa/Lagos-x")).is_err());
    // Any value matches `{"any": true}`: a style with an odd tile value is still "fill".
    let (r, p) = row_place(&c, "desktop.wallpaper-style", System::Windows);
    assert_eq!(
        codec::decode(
            &c,
            &r,
            &p,
            &[
                Some(Native::Text("10".into())),
                Some(Native::Text("7".into()))
            ]
        ),
        Ok(Some(text("fill")))
    );
    // A registry text field holding a number matches its number.
    let (r, p) = row_place(&c, "pointer.primary-button", System::Windows);
    assert_eq!(
        codec::decode(&c, &r, &p, &[Some(Native::Int(1))]),
        Ok(Some(text("right")))
    );
    // A true/false setting kept as 0 or 1.
    let cat = load(&with_row(json!({"id": "x.b", "value": "bool", "places": {"windows": {"mechanism": "registry",
        "key": "Software\\Fake", "fields": [{"name": "B", "kind": "dword"}], "codec": {"codec": "same"}}}}))).unwrap();
    let (r, p) = row_place(&cat, "x.b", System::Windows);
    assert_eq!(
        codec::decode(&cat, &r, &p, &[Some(Native::Int(1))]),
        Ok(Some(SettingValue::Bool(true)))
    );
    assert_eq!(
        codec::decode(&cat, &r, &p, &[Some(Native::Int(0))]),
        Ok(Some(SettingValue::Bool(false)))
    );
    // A table matching a decimal native value.
    let cat = load(&with_row(json!({"id": "x.f", "value": {"choice": {"options": ["half"]}}, "places": {"mac": {"mechanism": "mac-pref",
        "domain": "com.fake", "fields": [{"name": "F", "kind": "float"}], "codec": {"codec": "table", "rows": [{"native": [0.5], "neutral": "half"}]}}}}))).unwrap();
    let (r, p) = row_place(&cat, "x.f", System::Mac);
    assert_eq!(
        codec::decode(&cat, &r, &p, &[Some(Native::Float(0.5))]),
        Ok(Some(text("half")))
    );
    assert!(codec::decode(&cat, &r, &p, &[Some(Native::Float(0.25))]).is_err());
}

// --- map ---

fn people_ok() -> Vec<(String, String)> {
    vec![("a".into(), "a".into())]
}

fn plan_with(
    cat: &Catalogue,
    c: &Capture,
    to: System,
    t: &Target,
    ch: &Choices,
) -> Result<pctwin_settings::SettingsPlan, PlanError> {
    let m = people_ok();
    let people = People {
        old_account: "a",
        new_account: "a",
        mapped: &m,
    };
    let files: BTreeMap<String, Arrival> = BTreeMap::new();
    plan(cat, c, to, t, &files, ch, &people)
}

fn cap(system: System, k: &str, v: SettingValue, cat: &Catalogue) -> Capture {
    let mut c = Capture::new(system, 1);
    c.readings
        .insert(cat.row(k).unwrap().id.clone(), Reading::Value(v));
    c
}

#[test]
fn captures_carry_their_format() {
    let cat = Catalogue::starter().unwrap();
    let c = cap(System::Windows, "appearance.theme", text("dark"), &cat);
    assert_eq!(c.format, CAPTURE_FORMAT);
    assert_eq!(
        Capture::from_json(&serde_json::to_vec(&c).unwrap(), &cat).unwrap(),
        c
    );
    let mut zero = c.clone();
    zero.format = 0;
    assert_eq!(
        Capture::from_json(&serde_json::to_vec(&zero).unwrap(), &cat),
        Err(PlanError::NewerFormat(0))
    );
    assert!(matches!(
        plan_with(
            &cat,
            &zero,
            System::Windows,
            &Target::default(),
            &Choices::new()
        ),
        Err(PlanError::NewerFormat(0))
    ));
}

#[test]
fn the_time_zone_to_another_system_is_an_equivalent() {
    let cat = Catalogue::starter().unwrap();
    let c = cap(System::Windows, "time.zone", text("Africa/Lagos"), &cat);
    let one = |to| {
        plan_with(&cat, &c, to, &Target::default(), &Choices::new())
            .unwrap()
            .settings
            .into_iter()
            .find(|s| s.id.as_str() == "time.zone")
            .unwrap()
    };
    assert_eq!(one(System::Windows).outcome, Outcome::Exact);
    assert_eq!(one(System::Mac).outcome, Outcome::Equivalent);
}

#[test]
fn a_true_false_value_the_new_system_cannot_hold_offers_the_one_it_can() {
    let cat = load(&with_row(
        json!({"id": "x.only-on", "value": "bool", "places": {"windows": {"mechanism": "registry",
        "key": "Software\\Fake", "fields": [{"name": "B", "kind": "dword"}],
        "codec": {"codec": "table", "rows": [{"native": [1], "neutral": true}]}}}}),
    ))
    .unwrap();
    let c = cap(System::Mac, "x.only-on", SettingValue::Bool(false), &cat);
    let p = plan_with(
        &cat,
        &c,
        System::Windows,
        &Target::default(),
        &Choices::new(),
    )
    .unwrap();
    let s = p
        .settings
        .iter()
        .find(|s| s.id.as_str() == "x.only-on")
        .unwrap();
    assert_eq!(
        s.action,
        Action::NeedsChoice {
            options: vec![SettingValue::Bool(true)],
            why: pctwin_settings::map::CHOOSE
        }
    );
}

#[test]
fn the_persons_pick_of_a_nearby_value_is_used() {
    let cat = Catalogue::starter().unwrap();
    let c = cap(System::Mac, "pointer.speed", SettingValue::Int(229), &cat);
    let id = cat.row("pointer.speed").unwrap().id.clone();
    let ch: Choices = [(id, Choice::Use(SettingValue::Int(263)))].into();
    let p = plan_with(&cat, &c, System::Windows, &Target::default(), &ch).unwrap();
    let s = p
        .settings
        .iter()
        .find(|s| s.id.as_str() == "pointer.speed")
        .unwrap();
    assert_eq!(s.action, Action::Apply(SettingValue::Int(263)));
    // A pick the new system cannot hold is ignored.
    let id = cat.row("pointer.speed").unwrap().id.clone();
    let ch: Choices = [(id, Choice::Use(SettingValue::Int(264)))].into();
    let p = plan_with(&cat, &c, System::Windows, &Target::default(), &ch).unwrap();
    let s = p
        .settings
        .iter()
        .find(|s| s.id.as_str() == "pointer.speed")
        .unwrap();
    assert_eq!(s.action, Action::Apply(SettingValue::Int(211)));
}

#[test]
fn an_app_name_is_matched_whole() {
    let cat = Catalogue::starter().unwrap();
    let c = cap(
        System::Gnome,
        "apps.default-browser",
        text("firefox-esr"),
        &cat,
    );
    let t = Target {
        apps: ["firefox".to_string()].into(),
        ..Target::default()
    };
    let p = plan_with(&cat, &c, System::Windows, &t, &Choices::new()).unwrap();
    let s = p
        .settings
        .iter()
        .find(|s| s.id.as_str() == "apps.default-browser")
        .unwrap();
    assert!(matches!(s.action, Action::NeedsChoice { .. }));
}

// --- found / generic ---

fn reg(key: &str, name: &str) -> Address {
    Address::Registry {
        key: key.into(),
        name: name.into(),
        kind: RegKind::Sz,
    }
}

#[test]
fn only_the_persons_own_stores_may_be_written() {
    assert!(reg("Software\\Fake\\App", "A").writable());
    assert!(reg("Control Panel", "A").writable());
    assert!(reg("Control Panel\\Desktop", "A").writable());
    for (k, n) in [
        ("Environment", "Path"),
        ("Keyboard Layout\\Preload", "1"),
        ("Software\\Microsoft\\Office", "A"),
        ("Software\\Fake\\App", ""),
        ("Software\\Fake\\App", "A\u{202E}"),
        ("Software\\Fake\\..\\App", "A"),
        ("Software\\Fake\\App\\Policies", "A"),
    ] {
        assert!(!reg(k, n).writable(), "{k} {n}");
    }
    let mac = |d: &str, k: &str| Address::MacPref {
        domain: d.into(),
        key: k.into(),
        kind: PrefKind::Bool,
    };
    assert!(mac("com.fake.app", "K").writable());
    assert!(mac(&"a".repeat(128), "K").writable());
    assert!(!mac(&"a".repeat(129), "K").writable());
    assert!(!mac("com fake", "K").writable());
    assert!(!mac("com.fake.app", "").writable());
    assert!(!mac("com.apple.loginwindow", "K").writable());
    let dconf = |p: &str, k: &str| Address::Dconf {
        path: p.into(),
        key: k.into(),
    };
    assert!(dconf("/org/fake/app/", "dark-mode").writable());
    assert!(!dconf("/org/fake/app", "dark-mode").writable());
    assert!(!dconf("org/fake/app/", "dark-mode").writable());
    assert!(!dconf("/org/fake/app/", "").writable());
    assert!(!dconf("/org/fake/app/", "a/b").writable());
    assert!(!dconf("/org/gnome/desktop/privacy/", "x").writable());
}

#[test]
fn binary_values_up_to_the_limit_are_written_back() {
    let hex = "ab".repeat(1024);
    assert_eq!(
        setting_to_reg(RegKind::Binary, &text(&hex)),
        Some(RegValue::Binary(vec![0xab; 1024]))
    );
    assert_eq!(
        setting_to_reg(RegKind::Binary, &text(&"ab".repeat(1025))),
        None
    );
    assert_eq!(setting_to_reg(RegKind::Binary, &text("abc")), None);
}

#[test]
fn deep_trees_stop_at_the_depth_limit_and_control_panel_keys_are_labelled() {
    let mut h = MemHive::default();
    let mut key = "Software\\Fake".to_string();
    for n in 0..16 {
        key.push_str(&format!("\\L{n}"));
        h.put(&key, "V", RegValue::Dword(n));
    }
    h.put(
        "Control Panel\\Colors",
        "Window",
        RegValue::Sz("255 255 255".into()),
    );
    let d = discover_registry(&h);
    let fake = d
        .found
        .iter()
        .filter(|f| f.address.store().starts_with("Software\\Fake"))
        .count();
    assert!(fake > 0 && fake < 16, "{fake}");
    let colors = d
        .found
        .iter()
        .find(|f| f.address.name() == "Window")
        .unwrap();
    assert_eq!(colors.label, "Control Panel Colors");
}

#[test]
fn a_catalogue_key_in_another_domain_is_not_the_catalogues() {
    use pctwin_settings::generic::covered;
    let cat = Catalogue::starter().unwrap();
    let elsewhere = Address::MacPref {
        domain: "com.fake.app".into(),
        key: "AppleInterfaceStyle".into(),
        kind: PrefKind::String,
    };
    assert!(!covered(&cat, System::Mac, &elsewhere));
    let there = Address::MacPref {
        domain: "NSGlobalDomain".into(),
        key: "AppleInterfaceStyle".into(),
        kind: PrefKind::String,
    };
    assert!(covered(&cat, System::Mac, &there));
}

#[test]
fn a_secret_inside_a_desktop_list_is_found() {
    use pctwin_settings::found::parse_dconf_dump;
    use pctwin_settings::generic::plan_found;
    let d = parse_dconf_dump(
        "[org/fake/app]\naccounts=['ghp_0123456789abcdefghijABCDEFGHIJ012345']\nnames=['a', 'b']\n",
    );
    let m = FolderMap::default();
    let p = plan_found(
        &Catalogue::starter().unwrap(),
        System::Gnome,
        System::Gnome,
        &d,
        &ctx(&m),
    );
    assert!(
        p.not_moved
            .iter()
            .any(|n| n.name == "accounts" && n.why == Why::Secret),
        "{:?}",
        p.not_moved
    );
    assert!(p.moves.iter().any(|m| m.address.name() == "names"));
}

// --- store ---

fn store(cat: Catalogue, h: MemHive) -> Store {
    Store::new(
        cat,
        Mechanisms {
            system: System::Windows,
            user: Some(Box::new(h)),
            machine: None,
            runner: None,
            localtime: None,
        },
    )
}

#[test]
fn only_this_systems_protected_things_are_looked_for() {
    let mut h = MemHive::default();
    h.put("Software\\Microsoft\\Windows\\CurrentVersion\\CapabilityAccessManager\\ConsentStore\\webcam\\App1", "Value", RegValue::Sz("Allow".into()));
    let c = store(Catalogue::starter().unwrap(), h.clone()).capture();
    assert!(
        c.protected
            .contains(&pctwin_settings::catalogue::Protected::CameraAccess)
    );
    // The same probe written for a Mac is not run on Windows.
    let mut v = starter();
    for p in v["protected"].as_array_mut().unwrap() {
        p["system"] = json!("mac");
    }
    let c = store(load(&v).unwrap(), h).capture();
    assert!(c.protected.is_empty());
}

#[test]
fn extra_stores_are_scanned_for_settings_without_a_row_only_on_their_system() {
    let mut h = MemHive::default();
    h.put(
        "Control Panel\\Keyboard",
        "KeyboardSpeed",
        RegValue::Sz("31".into()),
    );
    let c = store(Catalogue::starter().unwrap(), h.clone()).capture();
    assert!(
        c.unsupported.iter().any(|f| f.name == "KeyboardSpeed"),
        "{:?}",
        c.unsupported
    );
    let mut v = starter();
    for d in v["discover"].as_array_mut().unwrap() {
        d["system"] = json!("mac");
    }
    let c = store(load(&v).unwrap(), h).capture();
    assert!(!c.unsupported.iter().any(|f| f.name == "KeyboardSpeed"));
}

#[test]
fn the_settings_adapter_checks_nothing_but_settings() {
    use pctwin_executor::Adapter;
    let mut a = SettingsAdapter {
        store: store(Catalogue::starter().unwrap(), MemHive::default()),
        platform: pctwin_recipe::Platform {
            os: Os::Windows,
            arch: pctwin_recipe::Arch::X86_64,
        },
    };
    assert!(!a.verify(&pctwin_recipe::Check::PackageInstalled {
        catalog: pctwin_recipe::Catalog::Winget,
        package: "X.Y".into()
    }));
}

#[test]
fn a_key_made_once_is_listed_once() {
    use pctwin_settings::found::write_found;
    let mut m = Mechanisms {
        system: System::Windows,
        user: Some(Box::new(MemHive::default())),
        machine: None,
        runner: None,
        localtime: None,
    };
    let mut created = Vec::new();
    for n in ["A", "B"] {
        write_found(
            &mut m,
            &reg("Software\\Fake\\App", n),
            Some(&text("x")),
            &mut created,
        )
        .unwrap();
    }
    assert_eq!(
        created,
        vec![
            "Software".to_string(),
            "Software\\Fake".into(),
            "Software\\Fake\\App".into()
        ]
    );
    let h: &dyn Hive = m.user.as_deref().unwrap();
    assert!(h.key_exists("Software\\Fake\\App").unwrap());
}

// --- files ---

#[test]
fn xml_with_a_document_type_is_not_read_and_two_removals_both_happen() {
    use pctwin_recipe::SettingsRoot;
    use pctwin_settings::files::{FileCtx, FileOutcome, FoundFile, plan_file};
    let cat = Catalogue::starter().unwrap();
    let m = FolderMap::default();
    let running = Default::default();
    let versions = Default::default();
    let renames = Default::default();
    let fctx = FileCtx {
        classify: ctx(&m),
        from: System::Gnome,
        to: System::Gnome,
        home_old: "/home/ada",
        home_new: "/home/ada",
        running: &running,
        versions: &versions,
        folder_renames: &renames,
    };
    let f = FoundFile {
        root: SettingsRoot::Config,
        rel: "fakeapp/a.xml".into(),
        label: "fakeapp".into(),
        format: Some(pctwin_settings::catalogue::FileFormat::Xml),
    };
    let dtd = "<!DOCTYPE x [<!ENTITY a \"aaaa\">]><x>&a;</x>";
    assert_eq!(
        plan_file(&cat, &f, dtd.as_bytes(), &fctx),
        FileOutcome::NotMoved(Why::Kind)
    );
    let two = r#"<app><opt name="password" value="x"/><keep name="theme" value="ocean"/><opt name="apiToken" value="y"/></app>"#;
    match plan_file(&cat, &f, two.as_bytes(), &fctx) {
        FileOutcome::Move { bytes, .. } => {
            assert_eq!(
                String::from_utf8(bytes).unwrap(),
                r#"<app><keep name="theme" value="ocean"/></app>"#
            );
        }
        other => panic!("{other:?}"),
    }
}

// --- second round ---

#[test]
fn an_unused_table_must_still_be_well_formed() {
    let mut c = starter();
    c["tables"]["extra"] = json!({"columns": ["neutral", "bad name"], "rows": [["a", "b"]]});
    assert!(refused(c));
    let mut c = starter();
    c["tables"]["extra"] = json!({"columns": ["neutral", "other"], "rows": [["a", "b"]]});
    assert!(!refused(c));
}

#[test]
fn a_key_that_was_there_before_is_never_listed_as_made() {
    use pctwin_settings::found::write_found;
    let mut h = MemHive::default();
    h.put("Software\\Fake", "x", RegValue::Dword(1));
    h.delete("Software\\Fake", "x").unwrap();
    let mut m = Mechanisms {
        system: System::Windows,
        user: Some(Box::new(h)),
        machine: None,
        runner: None,
        localtime: None,
    };
    let mut created = Vec::new();
    write_found(
        &mut m,
        &reg("Software\\Fake\\App", "A"),
        Some(&text("x")),
        &mut created,
    )
    .unwrap();
    assert_eq!(created, vec!["Software\\Fake\\App".to_string()]);
}

#[test]
fn ini_values_in_brackets_are_not_sections_and_xml_text_is_judged() {
    use pctwin_recipe::SettingsRoot;
    use pctwin_settings::catalogue::FileFormat;
    use pctwin_settings::files::{FileCtx, FileOutcome, FoundFile, plan_file};
    let cat = Catalogue::starter().unwrap();
    let m = FolderMap::default();
    let (running, versions, renames) = (Default::default(), Default::default(), Default::default());
    let fctx = FileCtx {
        classify: ctx(&m),
        from: System::Gnome,
        to: System::Gnome,
        home_old: "/home/ada",
        home_new: "/home/ada",
        running: &running,
        versions: &versions,
        folder_renames: &renames,
    };
    let f = |rel: &str, fmt| FoundFile {
        root: SettingsRoot::Config,
        rel: rel.into(),
        label: "fakeapp".into(),
        format: Some(fmt),
    };
    let ini = "[a]\nitems = [x, y]\npassword = p\n";
    match plan_file(
        &cat,
        &f("fakeapp/a.ini", FileFormat::Ini),
        ini.as_bytes(),
        &fctx,
    ) {
        FileOutcome::Move { bytes, left, .. } => {
            assert_eq!(String::from_utf8(bytes).unwrap(), "[a]\nitems = [x, y]\n");
            assert!(left.iter().any(|l| l.field == "a/password"));
        }
        o => panic!("{o:?}"),
    }
    let xml = "<app><theme>ocean</theme><apiKey>abc</apiKey></app>";
    match plan_file(
        &cat,
        &f("fakeapp/a.xml", FileFormat::Xml),
        xml.as_bytes(),
        &fctx,
    ) {
        FileOutcome::Move { bytes, .. } => assert_eq!(
            String::from_utf8(bytes).unwrap(),
            "<app><theme>ocean</theme></app>"
        ),
        o => panic!("{o:?}"),
    }
}

#[test]
fn a_handoff_page_names_only_the_exact_app() {
    use pctwin_settings::handoff::page;
    let cat = Catalogue::starter().unwrap();
    let row = cat.row("apps.default-browser").unwrap();
    let names = vec!["Firefox-308046B0AF4A39CB".to_string()];
    assert_eq!(
        page(
            &cat,
            row,
            System::Windows,
            Some(&text("firefox-esr")),
            &names
        )
        .as_deref(),
        Some("ms-settings:defaultapps")
    );
}

#[test]
fn value_shapes_at_their_limits() {
    use pctwin_settings::value::{ValueType, device_name, full_path};
    let list = ValueType::List { max: 3 };
    assert!(
        list.check("k", &text(&format!("{},b", "a".repeat(1100))))
            .is_err()
    );
    assert!(list.check("k", &text("a,b,c")).is_ok());
    let opts: Vec<String> = (0..65).map(|i| format!("o{i}")).collect();
    assert!(
        !ValueType::Choice {
            options: opts.clone()
        }
        .sound()
    );
    assert!(
        ValueType::Choice {
            options: opts[..64].to_vec()
        }
        .sound()
    );
    assert!(device_name("Office Printer"));
    assert!(!device_name("Office Printer "));
    assert!(!device_name("Office/Printer"));
    assert!(!device_name(""));
    assert!(!full_path("C:\\"));
    assert!(full_path("C:\\x"));
}

#[test]
fn a_list_longer_than_the_text_limit_is_refused_even_with_good_items() {
    use pctwin_settings::value::ValueType;
    let item = "a".repeat(120);
    let nine = [item.as_str(); 9].join(",");
    assert!(nine.len() > 1024);
    assert!(
        ValueType::List { max: 16 }
            .check("k", &text(&nine))
            .is_err()
    );
    let eight = [item.as_str(); 8].join(",");
    assert!(
        ValueType::List { max: 16 }
            .check("k", &text(&eight))
            .is_ok()
    );
}

#[test]
fn start_up_entries_are_read_by_name_without_the_default_value() {
    let mut h = MemHive::default();
    h.put(
        "Software\\Microsoft\\Windows\\CurrentVersion\\Run",
        "",
        RegValue::Sz("x".into()),
    );
    h.put(
        "Software\\Microsoft\\Windows\\CurrentVersion\\Run",
        "Spotify",
        RegValue::Sz("\"C:\\x.exe\"".into()),
    );
    let mut st = store(Catalogue::starter().unwrap(), h);
    assert_eq!(st.read("apps.startup"), Ok(Some(text("Spotify"))));
}

#[test]
fn token_forms_at_their_shortest() {
    use pctwin_settings::classify::looks_secret;
    let sixteen = "ghp_012345678901";
    assert_eq!(sixteen.len(), 16);
    assert_eq!(looks_secret(sixteen), Some(Why::Secret));
    assert_eq!(looks_secret("ghp_01234567890"), None);
    assert_eq!(looks_secret("eyJh.eyJz.c2ln"), Some(Why::Secret));
    assert_eq!(looks_secret("eyJ.eyJz.c2ln"), None);
}
