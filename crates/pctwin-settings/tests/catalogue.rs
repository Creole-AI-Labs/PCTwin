//! The catalogue is data, checked whole when read (T24-001, T24-002, T24-003, T24-013); a new
//! setting added only as a row is read, applied, read back and undone with no code change.

mod common;

use common::{SCOPE, actor, journal, plan_of, rebuild, undo};
use pctwin_journal::EffectUndoOutcome;
use pctwin_recipe::{Arch, Os, Platform, SettingValue};
use pctwin_settings::catalogue::{Catalogue, CatalogueError, STARTER, System};
use pctwin_settings::map::{Arrival, Choices, People, Reading, plan};
use pctwin_settings::mechanism::Mechanisms;
use pctwin_settings::registry::{Hive, MemHive, RegValue};
use pctwin_settings::{Action, SettingsAdapter, Store};
use pctwin_types::ItemStatus;
use serde_json::{Value, json};

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

fn refused(row: Value) -> String {
    match load(&with_row(row)) {
        Err(CatalogueError::Row { why, .. }) => why,
        other => panic!("not refused: {other:?}"),
    }
}

fn reg_row(id: &str, key: &str, name: &str) -> Value {
    json!({"id": id, "value": {"name": {}}, "places": {"windows": {
        "mechanism": "registry", "key": key, "fields": [{"name": name, "kind": "sz"}], "codec": {"codec": "same"}}}})
}

#[test]
fn the_starter_catalogue_reads_and_names_its_examples_as_rows() {
    let c = Catalogue::starter().unwrap();
    assert!(c.row("appearance.theme").is_some());
    assert!(c.row("apps.default-browser").is_some());
    assert!(c.app("vscode").is_some());
    assert!(c.settings.len() >= 15);
}

#[test]
fn t24_001_a_row_may_never_write_a_protected_store() {
    for key in [
        "Software\\Microsoft\\Windows\\CurrentVersion\\Run",
        "Software\\Microsoft\\Windows\\CurrentVersion\\RunOnce",
        "Software\\Policies\\Vendor",
        "Software\\Microsoft\\Windows\\CurrentVersion\\Policies\\System",
        "Software\\Classes\\exefile\\shell\\open\\command",
        "Environment",
        "SOFTWARE\\Microsoft\\Windows NT\\CurrentVersion\\Winlogon",
        "Software\\Microsoft\\Windows\\Shell\\Associations\\UrlAssociations\\https\\UserChoice",
    ] {
        let why = refused(reg_row("x.evil", key, "Shell"));
        assert!(why.contains("never be written"), "{key}: {why}");
    }
    // The machine's registry is only read.
    let mut row = reg_row("x.machine", "Software\\Vendor", "A");
    row["places"]["windows"]["hive"] = json!("machine");
    assert!(refused(row).contains("only ever read"));
    // A Mac sign-in hook, a GNOME command shortcut.
    let mac = json!({"id": "x.hook", "value": {"name": {}}, "places": {"mac": {
        "mechanism": "mac-pref", "domain": "com.apple.loginwindow", "fields": [{"name": "LoginHook", "kind": "string"}],
        "codec": {"codec": "same"}}}});
    assert!(refused(mac).contains("never be written"));
    let gnome = json!({"id": "x.keys", "value": {"name": {}}, "places": {"gnome": {
        "mechanism": "gsettings", "schema": "org.gnome.settings-daemon.plugins.media-keys", "fields": [{"name": "custom-keybindings", "kind": "string"}],
        "codec": {"codec": "same"}}}});
    assert!(refused(gnome).contains("never be written"));
    // A handoff setting is never written, even if a row says so.
    let mut handoff = reg_row("x.handoff", "Software\\Vendor", "A");
    handoff["route"] = json!("handoff");
    assert!(refused(handoff).contains("only a native setting"));
    // Paths that climb out, and odd names.
    assert!(refused(reg_row("x.climb", "Software\\..\\Environment", "A")).contains("not allowed"));
    assert!(refused(reg_row("x.slash", "\\Software\\Vendor", "A")).contains("not allowed"));
}

#[test]
fn t24_002_rows_with_unsound_shapes_or_tables_are_refused() {
    let bad_range = json!({"id": "x.range", "value": {"int": {"min": 5, "max": 1}}});
    assert!(refused(bad_range).contains("not sound"));
    let bad_neutral = json!({"id": "x.neutral", "value": {"choice": {"options": ["a", "b"]}}, "places": {"windows": {
        "mechanism": "registry", "key": "Software\\Vendor", "fields": [{"name": "A", "kind": "dword"}],
        "codec": {"codec": "table", "rows": [{"native": [1], "neutral": "c"}]}}}});
    assert!(!refused(bad_neutral).is_empty());
    let any_without_write = json!({"id": "x.any", "value": {"choice": {"options": ["a"]}}, "places": {"windows": {
        "mechanism": "registry", "key": "Software\\Vendor", "fields": [{"name": "A", "kind": "dword"}],
        "codec": {"codec": "table", "rows": [{"native": [{"any": true}], "neutral": "a"}]}}}});
    assert!(refused(any_without_write).contains("what to write"));
    let page = json!({"id": "x.page", "value": "bool", "pages": {"windows": {"uri": "https://example.com/"}}});
    assert!(refused(page).contains("settings pages"));
    let twice = with_row(reg_row("appearance.theme", "Software\\Vendor", "A"));
    assert!(matches!(load(&twice), Err(CatalogueError::Row { .. })));
    let mut extra = starter();
    extra["settings"][0]["run"] = json!("calc.exe");
    assert!(matches!(load(&extra), Err(CatalogueError::Damaged(_))));
}

#[test]
fn a_newer_catalogue_format_is_refused_not_guessed() {
    let mut c = starter();
    c["format"] = json!(2);
    assert!(matches!(load(&c), Err(CatalogueError::Format(_))));
    let mut c = starter();
    c["revision"] = json!(0);
    assert!(matches!(load(&c), Err(CatalogueError::Damaged(_))));
}

/// A setting PCTwin has no code for: Windows' keyboard repeat delay, added only as a row.
fn repeat_delay() -> Value {
    json!({"id": "keyboard.repeat-delay", "value": {"int": {"min": 0, "max": 3}},
        "places": {"windows": {"mechanism": "registry", "key": "Control Panel\\Keyboard",
            "fields": [{"name": "KeyboardDelay", "kind": "sz-int"}], "codec": {"codec": "same"}, "refresh": "sign-in"}}})
}

fn store_on(cat: &Catalogue, hive: Box<dyn Hive>) -> Store {
    Store::new(
        cat.clone(),
        Mechanisms {
            system: System::Windows,
            user: Some(hive),
            machine: None,
            runner: None,
            localtime: None,
        },
    )
}

#[test]
fn a_new_row_with_no_code_is_read_applied_read_back_and_undone() {
    let cat = load(&with_row(repeat_delay())).unwrap();
    let mut old = MemHive::default();
    old.put(
        "Control Panel\\Keyboard",
        "KeyboardDelay",
        RegValue::Sz("1".into()),
    );
    let capture = store_on(&cat, Box::new(old)).capture();
    assert_eq!(
        capture
            .readings
            .get(&cat.row("keyboard.repeat-delay").unwrap().id),
        Some(&Reading::Value(SettingValue::Int(1)))
    );
    let mut new = MemHive::default();
    new.put(
        "Control Panel\\Keyboard",
        "KeyboardDelay",
        RegValue::Sz("3".into()),
    );
    let mut target_store = store_on(&cat, Box::new(new));
    let target = target_store.probe_target();
    let mapped = vec![("a".to_string(), "a".to_string())];
    let people = People {
        old_account: "a",
        new_account: "a",
        mapped: &mapped,
    };
    let files: std::collections::BTreeMap<String, Arrival> = Default::default();
    let p = plan(
        &cat,
        &capture,
        System::Windows,
        &target,
        &files,
        &Choices::new(),
        &people,
    )
    .unwrap();
    let row = p
        .settings
        .iter()
        .find(|s| s.id.as_str() == "keyboard.repeat-delay")
        .unwrap();
    assert_eq!(row.action, Action::Apply(SettingValue::Int(1)));
    let recipe = row.recipe(Os::Windows, cat.revision).unwrap().unwrap();
    let (_d, j) = journal();
    let mut a = SettingsAdapter {
        store: target_store,
        platform: Platform {
            os: Os::Windows,
            arch: Arch::X86_64,
        },
    };
    let recipes = vec![recipe];
    let run = plan_of(&recipes).run(&mut a, &rebuild(&j), SCOPE, &actor());
    assert_eq!(run.results[0].1.status(), ItemStatus::Converted);
    let h = |a: &mut SettingsAdapter| {
        a.store
            .mechanisms()
            .user
            .as_deref()
            .unwrap()
            .get("Control Panel\\Keyboard", "KeyboardDelay")
            .unwrap()
    };
    assert_eq!(h(&mut a), Some(RegValue::Sz("1".into())));
    assert_eq!(
        undo(&j, &mut a, &recipes, SCOPE),
        vec![EffectUndoOutcome::Restored]
    );
    assert_eq!(h(&mut a), Some(RegValue::Sz("3".into())));
}

#[test]
fn without_its_row_the_same_value_still_moves_by_the_generic_rules() {
    use pctwin_settings::classify::{Ctx, FolderMap};
    use pctwin_settings::found::discover_registry;
    use pctwin_settings::generic::plan_found;
    let mut old = MemHive::default();
    old.put(
        "Control Panel\\Keyboard",
        "KeyboardDelay",
        RegValue::Sz("1".into()),
    );
    let d = discover_registry(&old);
    let map = FolderMap::default();
    let ctx = Ctx {
        paths: &map,
        same_screens: false,
    };
    let plain = plan_found(
        &Catalogue::starter().unwrap(),
        System::Windows,
        System::Windows,
        &d,
        &ctx,
    );
    assert_eq!(plain.moves.len(), 1);
    assert_eq!(plain.moves[0].value, SettingValue::Text("1".into()));
    // With the row, the catalogue takes it (and can translate it to other systems).
    let rich = plan_found(
        &load(&with_row(repeat_delay())).unwrap(),
        System::Windows,
        System::Windows,
        &d,
        &ctx,
    );
    assert_eq!((rich.moves.len(), rich.by_catalogue), (0, 1));
}

#[cfg(windows)]
#[test]
fn the_new_row_on_this_laptops_real_registry_in_a_test_key() {
    use pctwin_settings::registry::WinHive;
    let n = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap()
        .as_nanos();
    let base = format!("Software\\PCTwinTest\\row-{}-{n}", std::process::id());
    struct Gone(String);
    impl Drop for Gone {
        fn drop(&mut self) {
            let _ =
                winreg::RegKey::predef(winreg::enums::HKEY_CURRENT_USER).delete_subkey_all(&self.0);
        }
    }
    let _gone = Gone(base.clone());
    let cat = load(&with_row(repeat_delay())).unwrap();
    let mut old = WinHive::user_subkey(&format!("{base}\\old")).unwrap();
    old.set(
        "Control Panel\\Keyboard",
        "KeyboardDelay",
        &RegValue::Sz("2".into()),
    )
    .unwrap();
    let capture = store_on(&cat, Box::new(old)).capture();
    let new = WinHive::user_subkey(&format!("{base}\\new")).unwrap();
    let mut target_store = store_on(&cat, Box::new(new));
    let target = target_store.probe_target();
    let mapped = vec![("a".to_string(), "a".to_string())];
    let people = People {
        old_account: "a",
        new_account: "a",
        mapped: &mapped,
    };
    let files: std::collections::BTreeMap<String, Arrival> = Default::default();
    let p = plan(
        &cat,
        &capture,
        System::Windows,
        &target,
        &files,
        &Choices::new(),
        &people,
    )
    .unwrap();
    let row = p
        .settings
        .iter()
        .find(|s| s.id.as_str() == "keyboard.repeat-delay")
        .unwrap();
    let recipes = vec![row.recipe(Os::Windows, cat.revision).unwrap().unwrap()];
    let (_d, j) = journal();
    let mut a = SettingsAdapter {
        store: target_store,
        platform: Platform {
            os: Os::Windows,
            arch: Arch::X86_64,
        },
    };
    let run = plan_of(&recipes).run(&mut a, &rebuild(&j), SCOPE, &actor());
    assert_eq!(run.results[0].1.status(), ItemStatus::Converted);
    assert_eq!(
        a.store.read("keyboard.repeat-delay").unwrap(),
        Some(SettingValue::Int(2))
    );
    assert_eq!(
        undo(&j, &mut a, &recipes, SCOPE),
        vec![EffectUndoOutcome::Restored]
    );
    assert_eq!(a.store.read("keyboard.repeat-delay").unwrap(), None);
}
