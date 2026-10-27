//! Discovery first: an app no catalogue knows moves between two Windows laptops by the generic
//! rules, journaled, read back and undone; secrets, machine identities, window places, programs
//! and paths to things not moved stay behind with their reasons; a path into a moved folder is
//! rewritten (the user's decision of 27 October 2026).

mod common;

use common::{SCOPE, actor, journal, plan_of, rebuild, undo};
use pctwin_journal::EffectUndoOutcome;
use pctwin_recipe::{Arch, Os, Platform, SettingValue};
use pctwin_settings::catalogue::{Catalogue, System};
use pctwin_settings::classify::{Ctx, FolderMap, Why};
use pctwin_settings::found::{Address, RegKind, discover_registry};
use pctwin_settings::generic::{FoundPlan, plan_found};
use pctwin_settings::mechanism::Mechanisms;
use pctwin_settings::registry::{Hive, MemHive, RegValue};
use pctwin_settings::{SettingsAdapter, Store};
use pctwin_types::ItemStatus;

const APP: &str = "Software\\FakeVendor\\FakeApp";
const WIN: Platform = Platform {
    os: Os::Windows,
    arch: Arch::X86_64,
};

fn sz(s: &str) -> RegValue {
    RegValue::Sz(s.into())
}

/// The old laptop's registry: an app nobody catalogued, and the system's own stores.
fn fill_old(h: &mut dyn Hive) {
    let put = |h: &mut dyn Hive, k: &str, n: &str, v: RegValue| h.set(k, n, &v).unwrap();
    put(h, APP, "Theme", sz("ocean"));
    put(h, APP, "FontSize", RegValue::Dword(14));
    put(
        h,
        APP,
        "Plugins",
        RegValue::MultiSz(vec!["spell".into(), "grammar".into()]),
    );
    put(h, APP, "Accent", RegValue::Binary(vec![0x10, 0x20, 0x30]));
    put(
        h,
        APP,
        "LastProject",
        sz("C:\\Users\\ada\\Documents\\proj\\notes.txt"),
    );
    put(h, APP, "ExportFolder", sz("C:\\Users\\ada\\Old stuff"));
    put(
        h,
        APP,
        "MachineGuid",
        sz("3f2a9c1e-5b7d-4e8f-a1b2-c3d4e5f60718"),
    );
    put(h, APP, "ApiToken", sz("plain-looking"));
    put(
        h,
        APP,
        "Session",
        sz("AQAAANCMnd8BFdERjHoAwE/Cl+sBAAAAexampleexampleexample"),
    );
    put(
        h,
        APP,
        "Refresh",
        sz("ghp_0123456789abcdefghijABCDEFGHIJ012345"),
    );
    put(
        h,
        APP,
        "WindowPlacement",
        RegValue::Binary(vec![0x2c, 0, 0, 0]),
    );
    put(
        h,
        APP,
        "Helper",
        sz("\"C:\\Program Files\\Fake\\helper.exe\" --tray"),
    );
    put(
        h,
        APP,
        "Owner",
        sz("S-1-5-21-1111111111-2222222222-3333333333-1001"),
    );
    put(
        h,
        &format!("{APP}\\Recent"),
        "File1",
        sz("C:\\Users\\ada\\Documents\\a.txt"),
    );
    put(h, &format!("{APP}\\Cache"), "Index", RegValue::Dword(9));
    put(
        h,
        "Software\\Microsoft\\Windows\\CurrentVersion\\Explorer",
        "Something",
        RegValue::Dword(1),
    );
    put(h, "Software\\Classes\\.fake", "", sz("FakeApp.Doc"));
    put(h, "Control Panel\\Desktop", "WheelScrollLines", sz("3"));
    put(
        h,
        "Control Panel\\Desktop",
        "Wallpaper",
        sz("C:\\Users\\ada\\Pictures\\sea.jpg"),
    );
}

fn folders() -> FolderMap {
    FolderMap(vec![(
        "C:\\Users\\ada\\Documents".into(),
        "C:\\Users\\ada2\\Documents".into(),
    )])
}

fn planned(old: &dyn Hive) -> FoundPlan {
    let cat = Catalogue::starter().unwrap();
    let d = discover_registry(old);
    let map = folders();
    let ctx = Ctx {
        paths: &map,
        same_screens: false,
    };
    plan_found(&cat, System::Windows, System::Windows, &d, &ctx)
}

fn why_of(p: &FoundPlan, name: &str) -> Option<Why> {
    p.not_moved
        .iter()
        .find(|n| n.name == name)
        .map(|n| n.why.clone())
}

fn moved(p: &FoundPlan, name: &str) -> Option<SettingValue> {
    p.moves
        .iter()
        .find(|m| m.address.name() == name)
        .map(|m| m.value.clone())
}

#[test]
fn an_unknown_apps_settings_are_found_and_sorted_by_kind() {
    let mut old = MemHive::default();
    fill_old(&mut old);
    let p = planned(&old);
    // Copied as they are.
    assert_eq!(moved(&p, "Theme"), Some(SettingValue::Text("ocean".into())));
    assert_eq!(moved(&p, "FontSize"), Some(SettingValue::Int(14)));
    assert_eq!(
        moved(&p, "Plugins"),
        Some(SettingValue::Text("[\"spell\",\"grammar\"]".into()))
    );
    assert_eq!(
        moved(&p, "Accent"),
        Some(SettingValue::Text("102030".into()))
    );
    assert_eq!(
        moved(&p, "WheelScrollLines"),
        Some(SettingValue::Text("3".into()))
    );
    // A path into a moved folder is rewritten.
    assert_eq!(
        moved(&p, "LastProject"),
        Some(SettingValue::Text(
            "C:\\Users\\ada2\\Documents\\proj\\notes.txt".into()
        ))
    );
    assert_eq!(
        moved(&p, "File1"),
        Some(SettingValue::Text(
            "C:\\Users\\ada2\\Documents\\a.txt".into()
        ))
    );
    assert!(
        p.moves
            .iter()
            .find(|m| m.address.name() == "LastProject")
            .unwrap()
            .rewritten
    );
    // Left behind, each with its reason.
    assert_eq!(why_of(&p, "ExportFolder"), Some(Why::PathNotMoved));
    assert_eq!(why_of(&p, "MachineGuid"), Some(Why::MachineId));
    assert_eq!(why_of(&p, "ApiToken"), Some(Why::Secret));
    assert_eq!(why_of(&p, "Session"), Some(Why::Secret));
    assert_eq!(why_of(&p, "Refresh"), Some(Why::Secret));
    assert_eq!(why_of(&p, "WindowPlacement"), Some(Why::WindowPlace));
    assert_eq!(why_of(&p, "Helper"), Some(Why::StartsProgram));
    assert_eq!(why_of(&p, "Owner"), Some(Why::AccountBound));
    // The system's own stores and caches are reported as wholes, not walked.
    let store = |s: &str| {
        p.not_moved
            .iter()
            .find(|n| n.store == s)
            .map(|n| n.why.clone())
    };
    assert_eq!(store(&format!("HKCU\\{APP}\\Cache")), Some(Why::Cache));
    assert_eq!(why_of(&p, "Index"), None);
    assert_eq!(store("HKCU\\Software\\Microsoft"), Some(Why::SystemStore));
    assert_eq!(store("HKCU\\Software\\Classes"), Some(Why::StartsProgram));
    // The catalogue's own row handles the desktop picture (it waits for its file).
    assert_eq!(p.by_catalogue, 1);
    assert!(moved(&p, "Wallpaper").is_none() && why_of(&p, "Wallpaper").is_none());
    // Labels: the store's own names, or the catalogue's friendly one.
    let label = |n: &str| {
        p.moves
            .iter()
            .find(|m| m.address.name() == n)
            .unwrap()
            .label
            .clone()
    };
    assert_eq!(label("Theme"), "FakeVendor FakeApp");
    assert_eq!(label("WheelScrollLines"), "Desktop and screen");
    // Nothing is lost silently: everything found either moves, is left with a reason, or is
    // the catalogue's.
    let found = discover_registry(&old).found.len();
    let left: usize = p.not_moved.iter().filter(|n| n.name != "*").count();
    assert_eq!(p.moves.len() + left + p.by_catalogue, found);
}

#[test]
fn another_system_gets_only_what_the_catalogue_translates() {
    let mut old = MemHive::default();
    fill_old(&mut old);
    let cat = Catalogue::starter().unwrap();
    let map = folders();
    let ctx = Ctx {
        paths: &map,
        same_screens: false,
    };
    let p = plan_found(
        &cat,
        System::Windows,
        System::Mac,
        &discover_registry(&old),
        &ctx,
    );
    assert!(p.moves.is_empty());
    assert_eq!(why_of(&p, "Theme"), Some(Why::OtherSystem));
}

#[test]
fn a_catalogue_exception_keeps_one_apps_value_behind() {
    let mut old = MemHive::default();
    old.put("Software\\Valve\\Steam", "AutoLoginUser", sz("someone"));
    old.put("Software\\Valve\\Steam", "Language", sz("english"));
    let p = planned(&old);
    assert!(matches!(
        why_of(&p, "AutoLoginUser"),
        Some(Why::Exception(_))
    ));
    assert!(moved(&p, "Language").is_some());
}

/// Moves `p` onto `new` through the executor and returns the store.
fn apply(
    p: &FoundPlan,
    new: Box<dyn Hive>,
    j: &pctwin_journal::Journal,
) -> (SettingsAdapter, Vec<pctwin_recipe::Recipe>) {
    let cat = Catalogue::starter().unwrap();
    let mut store = Store::new(
        cat,
        Mechanisms {
            system: System::Windows,
            user: Some(new),
            machine: None,
            runner: None,
            localtime: None,
        },
    );
    store.remember(p.book()).unwrap();
    let recipes: Vec<_> = p
        .recipes(Os::Windows, 1)
        .unwrap()
        .into_iter()
        .map(|(_, r)| r)
        .collect();
    let mut adapter = SettingsAdapter {
        store,
        platform: WIN,
    };
    let run = plan_of(&recipes).run(&mut adapter, &rebuild(j), SCOPE, &actor());
    assert_eq!(run.stopped, None);
    for (_, r) in &run.results {
        assert_eq!(r.status(), ItemStatus::Converted, "{r:?}");
    }
    (adapter, recipes)
}

fn get(a: &mut SettingsAdapter, key: &str, name: &str) -> Option<RegValue> {
    a.store
        .mechanisms()
        .user
        .as_deref()
        .unwrap()
        .get(key, name)
        .unwrap()
}

#[test]
fn an_unknown_app_moves_same_system_and_undoes_cleanly() {
    let mut old = MemHive::default();
    fill_old(&mut old);
    let p = planned(&old);
    let (_d, j) = journal();
    let mut new = MemHive::default();
    new.put("Control Panel\\Desktop", "WheelScrollLines", sz("5"));
    let (mut a, recipes) = apply(&p, Box::new(new), &j);
    assert_eq!(get(&mut a, APP, "Theme"), Some(sz("ocean")));
    assert_eq!(get(&mut a, APP, "FontSize"), Some(RegValue::Dword(14)));
    assert_eq!(
        get(&mut a, APP, "Plugins"),
        Some(RegValue::MultiSz(vec!["spell".into(), "grammar".into()]))
    );
    assert_eq!(
        get(&mut a, APP, "Accent"),
        Some(RegValue::Binary(vec![0x10, 0x20, 0x30]))
    );
    assert_eq!(
        get(&mut a, APP, "LastProject"),
        Some(sz("C:\\Users\\ada2\\Documents\\proj\\notes.txt"))
    );
    for left in [
        "MachineGuid",
        "ApiToken",
        "Session",
        "Refresh",
        "Helper",
        "WindowPlacement",
    ] {
        assert_eq!(get(&mut a, APP, left), None, "{left}");
    }
    assert_eq!(
        get(&mut a, "Control Panel\\Desktop", "WheelScrollLines"),
        Some(sz("3"))
    );
    assert!(a.store.created_keys().iter().any(|k| k == APP));
    // Undo: every value back as it was, and the keys PCTwin made are gone again.
    let outcomes = undo(&j, &mut a, &recipes, SCOPE);
    assert!(
        outcomes.iter().all(|o| *o == EffectUndoOutcome::Restored),
        "{outcomes:?}"
    );
    assert_eq!(get(&mut a, APP, "Theme"), None);
    assert_eq!(
        get(&mut a, "Control Panel\\Desktop", "WheelScrollLines"),
        Some(sz("5"))
    );
    let removed = a.store.prune_created();
    assert!(removed.contains(&APP.to_string()));
    assert!(removed.contains(&"Software\\FakeVendor".to_string()));
    let h = a.store.mechanisms().user.as_deref().unwrap();
    assert!(!h.key_exists("Software\\FakeVendor").unwrap());
    assert!(h.key_exists("Control Panel\\Desktop").unwrap());
}

#[test]
fn t24_019_undo_keeps_a_value_the_person_changed_afterwards() {
    let mut old = MemHive::default();
    old.put(APP, "Theme", sz("ocean"));
    old.put(APP, "FontSize", RegValue::Dword(14));
    let p = planned(&old);
    let (_d, j) = journal();
    let (mut a, recipes) = apply(&p, Box::new(MemHive::default()), &j);
    a.store
        .mechanisms()
        .user
        .as_deref_mut()
        .unwrap()
        .set(APP, "Theme", &sz("forest"))
        .unwrap();
    let outcomes = undo(&j, &mut a, &recipes, SCOPE);
    assert!(
        outcomes
            .iter()
            .any(|o| matches!(o, EffectUndoOutcome::Kept { .. }))
    );
    assert_eq!(get(&mut a, APP, "Theme"), Some(sz("forest")));
    assert_eq!(get(&mut a, APP, "FontSize"), None);
    // The key still holds the person's value: it stays.
    assert!(!a.store.prune_created().contains(&APP.to_string()));
}

#[test]
fn t24_025_a_system_that_does_not_keep_the_value_is_a_failure_not_success() {
    let mut old = MemHive::default();
    old.put(APP, "Theme", sz("ocean"));
    let p = planned(&old);
    let (_d, j) = journal();
    let mut new = MemHive::default();
    new.lies = true;
    let cat = Catalogue::starter().unwrap();
    let mut store = Store::new(
        cat,
        Mechanisms {
            system: System::Windows,
            user: Some(Box::new(new)),
            machine: None,
            runner: None,
            localtime: None,
        },
    );
    store.remember(p.book()).unwrap();
    let recipes: Vec<_> = p
        .recipes(Os::Windows, 1)
        .unwrap()
        .into_iter()
        .map(|(_, r)| r)
        .collect();
    let mut a = SettingsAdapter {
        store,
        platform: WIN,
    };
    let run = plan_of(&recipes).run(&mut a, &rebuild(&j), SCOPE, &actor());
    assert_eq!(run.results[0].1.status(), ItemStatus::Failed);
}

#[test]
fn t24_022_a_refused_write_is_never_reported_done() {
    let mut old = MemHive::default();
    old.put(APP, "Theme", sz("ocean"));
    let p = planned(&old);
    let (_d, j) = journal();
    let cat = Catalogue::starter().unwrap();
    let mut new = MemHive::default();
    new.deny_writes = true;
    let mut store = Store::new(
        cat,
        Mechanisms {
            system: System::Windows,
            user: Some(Box::new(new)),
            machine: None,
            runner: None,
            localtime: None,
        },
    );
    store.remember(p.book()).unwrap();
    let recipes: Vec<_> = p
        .recipes(Os::Windows, 1)
        .unwrap()
        .into_iter()
        .map(|(_, r)| r)
        .collect();
    let mut a = SettingsAdapter {
        store,
        platform: WIN,
    };
    let run = plan_of(&recipes).run(&mut a, &rebuild(&j), SCOPE, &actor());
    let r = &run.results[0].1;
    assert_eq!(r.status(), ItemStatus::NeedsAction);
    assert_eq!(r.reason(), Some(pctwin_types::Reason::Permission));
}

#[test]
fn t24_020_a_value_that_cannot_be_read_first_is_never_written() {
    let mut old = MemHive::default();
    old.put(APP, "Theme", sz("ocean"));
    let p = planned(&old);
    let (_d, j) = journal();
    let mut new = MemHive::default();
    new.unreadable = true;
    let cat = Catalogue::starter().unwrap();
    let mut store = Store::new(
        cat,
        Mechanisms {
            system: System::Windows,
            user: Some(Box::new(new)),
            machine: None,
            runner: None,
            localtime: None,
        },
    );
    store.remember(p.book()).unwrap();
    let recipes: Vec<_> = p
        .recipes(Os::Windows, 1)
        .unwrap()
        .into_iter()
        .map(|(_, r)| r)
        .collect();
    let mut a = SettingsAdapter {
        store,
        platform: WIN,
    };
    let run = plan_of(&recipes).run(&mut a, &rebuild(&j), SCOPE, &actor());
    assert_eq!(run.results[0].1.status(), ItemStatus::Failed);
    assert!(j.effects().unwrap().is_empty(), "nothing was even intended");
}

#[test]
fn a_book_that_lies_about_an_address_is_refused() {
    let a = Address::Registry {
        key: APP.into(),
        name: "Theme".into(),
        kind: RegKind::Sz,
    };
    let other = Address::Registry {
        key: "Software\\Microsoft\\Windows\\CurrentVersion\\Run".into(),
        name: "Evil".into(),
        kind: RegKind::Sz,
    };
    let cat = Catalogue::starter().unwrap();
    let mut store = Store::new(
        cat,
        Mechanisms {
            system: System::Windows,
            user: Some(Box::new(MemHive::default())),
            machine: None,
            runner: None,
            localtime: None,
        },
    );
    // The key names one place, the address is another.
    assert!(store.remember([(a.key(), other.clone())].into()).is_err());
    // A place PCTwin never writes, even under its own key.
    assert!(store.remember([(other.key(), other)].into()).is_err());
    assert!(store.remember([(a.key(), a)].into()).is_ok());
}

#[test]
fn recipes_group_by_label_and_respect_the_step_limit() {
    let mut old = MemHive::default();
    for n in 0..450 {
        old.put(APP, &format!("Option{n}"), RegValue::Dword(n));
    }
    let p = planned(&old);
    let recipes = p.recipes(Os::Windows, 1).unwrap();
    assert_eq!(recipes.len(), 3);
    assert!(
        recipes
            .iter()
            .all(|(l, r)| l == "FakeVendor FakeApp" && r.steps.len() <= 200)
    );
    assert_eq!(
        recipes.iter().map(|(_, r)| r.steps.len()).sum::<usize>(),
        450
    );
}

#[cfg(windows)]
mod real_registry {
    //! The same move on this laptop's real registry, inside a key of the test's own under
    //! `HKEY_CURRENT_USER\Software\PCTwinTest`: no real setting is read or changed.

    use super::*;
    use pctwin_settings::registry::WinHive;

    struct TestKey(String);

    impl Drop for TestKey {
        fn drop(&mut self) {
            let hkcu = winreg::RegKey::predef(winreg::enums::HKEY_CURRENT_USER);
            let _ = hkcu.delete_subkey_all(&self.0);
        }
    }

    fn test_key() -> TestKey {
        let n = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_nanos();
        TestKey(format!(
            "Software\\PCTwinTest\\found-{}-{n}",
            std::process::id()
        ))
    }

    #[test]
    fn an_unknown_app_moves_on_the_real_registry_and_undoes_cleanly() {
        let base = test_key();
        let mut old = WinHive::user_subkey(&format!("{}\\old", base.0)).unwrap();
        fill_old(&mut old);
        let p = planned(&old);
        assert!(moved(&p, "Theme").is_some());
        assert_eq!(why_of(&p, "Session"), Some(Why::Secret));
        let (_d, j) = journal();
        let new = WinHive::user_subkey(&format!("{}\\new", base.0)).unwrap();
        let (mut a, recipes) = apply(&p, Box::new(new), &j);
        assert_eq!(get(&mut a, APP, "Theme"), Some(sz("ocean")));
        assert_eq!(
            get(&mut a, APP, "Accent"),
            Some(RegValue::Binary(vec![0x10, 0x20, 0x30]))
        );
        assert_eq!(
            get(&mut a, APP, "Plugins"),
            Some(RegValue::MultiSz(vec!["spell".into(), "grammar".into()]))
        );
        assert_eq!(get(&mut a, APP, "Session"), None);
        let outcomes = undo(&j, &mut a, &recipes, SCOPE);
        assert!(
            outcomes.iter().all(|o| *o == EffectUndoOutcome::Restored),
            "{outcomes:?}"
        );
        let removed = a.store.prune_created();
        assert!(
            removed.contains(&"Software\\FakeVendor".to_string()),
            "{removed:?}"
        );
        let h = a.store.mechanisms().user.as_deref().unwrap();
        assert!(!h.key_exists("Software\\FakeVendor").unwrap());
    }

    #[test]
    fn this_laptops_own_registry_is_read_without_changing_anything() {
        // Discovery over the real person's registry: read only. Only counts are looked at, so
        // no value of the person's leaves this test.
        let hive = WinHive::current_user();
        let d = discover_registry(&hive);
        assert!(!d.found.is_empty() || !d.not_moved.is_empty());
        let mut seen = std::collections::BTreeSet::new();
        for f in &d.found {
            assert!(
                f.address.writable()
                    || f.address
                        .store()
                        .to_lowercase()
                        .starts_with("control panel")
            );
            assert!(seen.insert(f.address.key()), "every place has its own key");
        }
    }
}
