//! The Mac and Linux mechanisms, against stand-ins for `defaults`, `dconf` and `gsettings` that
//! behave like the real tools (these tests run on every system; the real tools are proven on
//! the CI runners of each system).

mod common;

use std::collections::BTreeMap;

use common::{SCOPE, actor, journal, plan_of, rebuild, undo};
use pctwin_journal::EffectUndoOutcome;
use pctwin_recipe::{Arch, Os, Platform, SettingValue};
use pctwin_settings::catalogue::{Catalogue, System};
use pctwin_settings::classify::{Ctx, FolderMap, Why};
use pctwin_settings::found::{discover_dconf, discover_mac, parse_dconf_dump};
use pctwin_settings::generic::plan_found;
use pctwin_settings::gvariant::{self, GValue};
use pctwin_settings::mechanism::Mechanisms;
use pctwin_settings::plist::{self, Plist};
use pctwin_settings::runner::{Output, Runner, Tool};
use pctwin_settings::{Reading, SettingsAdapter, Store};
use pctwin_types::ItemStatus;

/// `defaults`, `dconf` and `gsettings` in memory.
#[derive(Default, Clone)]
struct Tools {
    /// domain -> key -> value
    prefs: BTreeMap<String, Vec<(String, Plist)>>,
    /// full dconf key -> GVariant text
    dconf: BTreeMap<String, String>,
    /// (schema, key) -> GVariant text
    gsettings: BTreeMap<(String, String), String>,
    locked: Vec<(String, String)>,
    calls: std::rc::Rc<std::cell::RefCell<Vec<String>>>,
}

fn ok(s: &str) -> Result<Output, String> {
    Ok(Output {
        ok: true,
        stdout: s.to_string(),
    })
}

fn fail() -> Result<Output, String> {
    Ok(Output {
        ok: false,
        stdout: String::new(),
    })
}

impl Runner for Tools {
    fn run(&mut self, tool: Tool, args: &[&str]) -> Result<Output, String> {
        self.calls
            .borrow_mut()
            .push(format!("{tool:?} {}", args.join(" ")));
        match (tool, args) {
            (Tool::Defaults, ["domains"]) => ok(&self
                .prefs
                .keys()
                .filter(|d| *d != "NSGlobalDomain")
                .cloned()
                .collect::<Vec<_>>()
                .join(", ")),
            (Tool::Defaults, ["export", d, "-"]) => {
                let items = self.prefs.get(*d).cloned().unwrap_or_default();
                ok(&format!(
                    "<?xml version=\"1.0\" encoding=\"UTF-8\"?>\n<!DOCTYPE plist PUBLIC \"-//Apple//DTD PLIST 1.0//EN\" \"http://www.apple.com/DTDs/PropertyList-1.0.dtd\">\n<plist version=\"1.0\">\n{}\n</plist>\n",
                    plist::to_xml(&Plist::Dict(items)).unwrap()
                ))
            }
            (Tool::Defaults, ["write", d, k, rest @ ..]) => {
                let v = match rest {
                    ["-bool", v] => Plist::Bool(*v == "true"),
                    ["-int", v] => Plist::Integer(v.parse().map_err(|_| "bad")?),
                    ["-float", v] => Plist::Real(v.parse().map_err(|_| "bad")?),
                    ["-string", v] => Plist::String(v.to_string()),
                    [xml] => plist::from_xml(xml)?,
                    _ => return fail(),
                };
                let dom = self.prefs.entry(d.to_string()).or_default();
                dom.retain(|(key, _)| key != k);
                dom.push((k.to_string(), v));
                ok("")
            }
            (Tool::Defaults, ["delete", d, k]) => {
                let dom = self.prefs.entry(d.to_string()).or_default();
                let before = dom.len();
                dom.retain(|(key, _)| key != k);
                if dom.len() == before { fail() } else { ok("") }
            }
            (Tool::Dconf, ["dump", "/"]) => {
                let mut by_dir: BTreeMap<String, Vec<(String, String)>> = BTreeMap::new();
                for (full, v) in &self.dconf {
                    let (dir, key) = full.rsplit_once('/').unwrap();
                    by_dir
                        .entry(dir.trim_start_matches('/').to_string())
                        .or_default()
                        .push((key.into(), v.clone()));
                }
                let mut s = String::new();
                for (dir, kv) in by_dir {
                    s.push_str(&format!("[{dir}]\n"));
                    for (k, v) in kv {
                        s.push_str(&format!("{k}={v}\n"));
                    }
                    s.push('\n');
                }
                ok(&s)
            }
            (Tool::Dconf, ["read", k]) => ok(&self.dconf.get(*k).cloned().unwrap_or_default()),
            (Tool::Dconf, ["write", k, v]) => {
                gvariant::parse(v)?;
                self.dconf.insert(k.to_string(), v.to_string());
                ok("")
            }
            (Tool::Dconf, ["reset", k]) => {
                self.dconf.remove(*k);
                ok("")
            }
            (Tool::Gsettings, ["get", s, k]) => {
                match self.gsettings.get(&(s.to_string(), k.to_string())) {
                    Some(v) => ok(v),
                    None => fail(),
                }
            }
            (Tool::Gsettings, ["set", s, k, v]) => {
                gvariant::parse(v)?;
                self.gsettings
                    .insert((s.to_string(), k.to_string()), v.to_string());
                ok("")
            }
            (Tool::Gsettings, ["writable", s, k]) => {
                ok(if self.locked.contains(&(s.to_string(), k.to_string())) {
                    "false"
                } else {
                    "true"
                })
            }
            (Tool::Gsettings, ["list-keys", s]) => ok(&self
                .gsettings
                .keys()
                .filter(|(sc, _)| sc == s)
                .map(|(_, k)| k.clone())
                .collect::<Vec<_>>()
                .join("\n")),
            _ => fail(),
        }
    }
}

fn mech(system: System, tools: Tools) -> Mechanisms {
    Mechanisms {
        system,
        user: None,
        machine: None,
        runner: Some(Box::new(tools)),
        localtime: None,
    }
}

#[test]
fn plists_and_gvariants_round_trip() {
    let v = Plist::Dict(vec![
        (
            "a".into(),
            Plist::Array(vec![Plist::Integer(1), Plist::String("x<&>y".into())]),
        ),
        ("b".into(), Plist::Bool(true)),
        ("c".into(), Plist::Real(0.5)),
        ("d".into(), Plist::String("line\nbreak".into())),
    ]);
    let xml = plist::to_xml(&v).unwrap();
    assert!(!xml.contains('\n'));
    assert_eq!(plist::from_xml(&xml).unwrap(), v);
    assert!(plist::parse("<plist><dict><key>a</key></dict>").is_err());
    assert!(plist::parse("<notplist/>").is_err());
    for (text, want) in [
        ("'prefer-dark'", GValue::Str("prefer-dark".into())),
        ("'it\\'s'", GValue::Str("it's".into())),
        ("uint32 2", GValue::Int(2)),
        ("1.25", GValue::Float(1.25)),
        ("true", GValue::Bool(true)),
        (
            "[('xkb', 'us'), ('ibus', 'mozc-jp')]",
            GValue::Pairs(vec![
                ("xkb".into(), "us".into()),
                ("ibus".into(), "mozc-jp".into()),
            ]),
        ),
        ("@a(ss) []", GValue::Pairs(vec![])),
    ] {
        assert_eq!(gvariant::parse(text).unwrap(), want, "{text}");
        assert_eq!(gvariant::parse(&gvariant::format(&want)).unwrap(), want);
    }
    assert!(gvariant::parse("'unterminated").is_err());
    assert!(gvariant::parse("'a' 'b'").is_err());
    assert_eq!(gvariant::quote("a'b\\c"), "'a\\'b\\\\c'");
}

#[test]
fn a_mac_rows_values_are_read_and_written_through_defaults() {
    let mut tools = Tools::default();
    tools.prefs.insert(
        "NSGlobalDomain".into(),
        vec![
            ("AppleInterfaceStyle".into(), Plist::String("Dark".into())),
            ("com.apple.mouse.scaling".into(), Plist::Real(0.6875)),
        ],
    );
    let cat = Catalogue::starter().unwrap();
    let mut store = Store::new(cat.clone(), mech(System::Mac, tools));
    let c = store.capture();
    let r = |k: &str| c.readings.get(&cat.row(k).unwrap().id).cloned();
    assert_eq!(
        r("appearance.theme"),
        Some(Reading::Value(SettingValue::Text("dark".into())))
    );
    assert_eq!(
        r("pointer.speed"),
        Some(Reading::Value(SettingValue::Int(229)))
    );
    assert_eq!(r("pointer.natural-scroll"), Some(Reading::NotSet));
    assert_eq!(r("appearance.taskbar-theme"), Some(Reading::NotApplicable));
    // Automatic, written as the system holds it, and read back.
    let grant_free = store.write_if(
        "appearance.theme",
        &SettingValue::Text("dark".into()),
        Some(&SettingValue::Text("auto".into())),
    );
    assert_eq!(grant_free, Ok(pctwin_executor::SettingWrite::Written));
    assert_eq!(
        store.read("appearance.theme"),
        Ok(Some(SettingValue::Text("auto".into())))
    );
    store
        .write_if(
            "appearance.theme",
            &SettingValue::Text("auto".into()),
            Some(&SettingValue::Text("light".into())),
        )
        .unwrap();
    assert_eq!(
        store.read("appearance.theme"),
        Ok(Some(SettingValue::Text("light".into())))
    );
}

#[test]
fn every_mac_domain_is_found_and_an_unknown_apps_preferences_move_and_undo() {
    let mut tools = Tools::default();
    tools.prefs.insert(
        "NSGlobalDomain".into(),
        vec![(
            "AppleLanguages".into(),
            Plist::Array(vec![Plist::String("en-GB".into())]),
        )],
    );
    tools.prefs.insert(
        "com.fakevendor.fakeapp".into(),
        vec![
            ("Theme".into(), Plist::String("ocean".into())),
            ("ShowRuler".into(), Plist::Bool(true)),
            ("Zoom".into(), Plist::Real(1.5)),
            (
                "Panels".into(),
                Plist::Dict(vec![("left".into(), Plist::Integer(2))]),
            ),
            (
                "LastDocument".into(),
                Plist::String("/Users/ada/Projects/a.txt".into()),
            ),
            (
                "NSWindow Frame Main".into(),
                Plist::String("0 0 800 600".into()),
            ),
            ("AuthToken".into(), Plist::String("abc".into())),
            ("InstallID".into(), Plist::String("123".into())),
        ],
    );
    tools.prefs.insert(
        "com.apple.security.revocation".into(),
        vec![("OCSPStyle".into(), Plist::String("x".into()))],
    );
    let mut old = mech(System::Mac, tools);
    let d = discover_mac(&mut old);
    let map = FolderMap(vec![(
        "/Users/ada/Projects".into(),
        "/Users/ada/Work".into(),
    )]);
    let ctx = Ctx {
        paths: &map,
        same_screens: false,
    };
    let cat = Catalogue::starter().unwrap();
    let p = plan_found(&cat, System::Mac, System::Mac, &d, &ctx);
    let moved = |n: &str| {
        p.moves
            .iter()
            .find(|m| m.address.name() == n)
            .map(|m| m.value.clone())
    };
    let why = |n: &str| {
        p.not_moved
            .iter()
            .find(|m| m.name == n)
            .map(|m| m.why.clone())
    };
    assert_eq!(moved("Theme"), Some(SettingValue::Text("ocean".into())));
    assert_eq!(moved("ShowRuler"), Some(SettingValue::Bool(true)));
    assert_eq!(moved("Zoom"), Some(SettingValue::Text("1.5".into())));
    assert_eq!(
        moved("LastDocument"),
        Some(SettingValue::Text("/Users/ada/Work/a.txt".into()))
    );
    assert!(moved("AppleLanguages").is_some());
    assert_eq!(why("NSWindow Frame Main"), Some(Why::WindowPlace));
    assert_eq!(why("AuthToken"), Some(Why::Secret));
    assert_eq!(why("InstallID"), Some(Why::MachineId));
    assert!(
        p.not_moved
            .iter()
            .any(|n| n.label == "com.apple.security.revocation" && n.why == Why::Security)
    );
    // Onto a second Mac, through the executor, then undone.
    let mut new_tools = Tools::default();
    new_tools.prefs.insert("NSGlobalDomain".into(), vec![]);
    let mut store = Store::new(cat, mech(System::Mac, new_tools));
    store.remember(p.book()).unwrap();
    let recipes: Vec<_> = p
        .recipes(Os::Macos, 1)
        .unwrap()
        .into_iter()
        .map(|(_, r)| r)
        .collect();
    let mut a = SettingsAdapter {
        store,
        platform: Platform {
            os: Os::Macos,
            arch: Arch::Aarch64,
        },
    };
    let (_d, j) = journal();
    let run = plan_of(&recipes).run(&mut a, &rebuild(&j), SCOPE, &actor());
    assert!(
        run.results
            .iter()
            .all(|(_, r)| r.status() == ItemStatus::Converted),
        "{:?}",
        run.results
    );
    let key = p
        .moves
        .iter()
        .find(|m| m.address.name() == "Panels")
        .unwrap()
        .key
        .clone();
    assert!(a.store.read(&key).unwrap().is_some());
    let outcomes = undo(&j, &mut a, &recipes, SCOPE);
    assert!(
        outcomes.iter().all(|o| *o == EffectUndoOutcome::Restored),
        "{outcomes:?}"
    );
    assert_eq!(a.store.read(&key).unwrap(), None);
}

#[test]
fn gnome_values_are_found_in_dconf_and_move_and_undo() {
    let dump = "[org/gnome/desktop/interface]\ncolor-scheme='prefer-dark'\nfont-name='Cantarell 11'\n\n[org/fakevendor/fakeapp]\nrecent-folder='/home/ada/Projects/site'\nwindow-width=900\napi-key='abc'\nzoom=1.25\n\n[org/gnome/desktop/privacy]\nremember-recent-files=false\n";
    let d = parse_dconf_dump(dump);
    assert_eq!(d.found.len(), 7);
    let map = FolderMap(vec![("/home/ada/Projects".into(), "/home/ada/Work".into())]);
    let ctx = Ctx {
        paths: &map,
        same_screens: false,
    };
    let cat = Catalogue::starter().unwrap();
    let p = plan_found(&cat, System::Gnome, System::Gnome, &d, &ctx);
    let moved = |n: &str| {
        p.moves
            .iter()
            .find(|m| m.address.name() == n)
            .map(|m| m.value.clone())
    };
    let why = |n: &str| {
        p.not_moved
            .iter()
            .find(|m| m.name == n)
            .map(|m| m.why.clone())
    };
    // The catalogue's row takes the colour scheme; the font has no row and moves as found.
    assert_eq!(p.by_catalogue, 1);
    assert_eq!(
        moved("font-name"),
        Some(SettingValue::Text("'Cantarell 11'".into()))
    );
    assert_eq!(
        moved("recent-folder"),
        Some(SettingValue::Text("'/home/ada/Work/site'".into()))
    );
    assert_eq!(moved("zoom"), Some(SettingValue::Text("1.25".into())));
    assert_eq!(why("window-width"), Some(Why::WindowPlace));
    assert_eq!(why("api-key"), Some(Why::Secret));
    assert_eq!(why("remember-recent-files"), Some(Why::Security));
    // Onto another GNOME laptop and back.
    let mut store = Store::new(cat, mech(System::Gnome, Tools::default()));
    store.remember(p.book()).unwrap();
    let recipes: Vec<_> = p
        .recipes(Os::Linux, 1)
        .unwrap()
        .into_iter()
        .map(|(_, r)| r)
        .collect();
    let mut a = SettingsAdapter {
        store,
        platform: Platform {
            os: Os::Linux,
            arch: Arch::X86_64,
        },
    };
    let (_dir, j) = journal();
    let run = plan_of(&recipes).run(&mut a, &rebuild(&j), SCOPE, &actor());
    assert!(
        run.results
            .iter()
            .all(|(_, r)| r.status() == ItemStatus::Converted),
        "{:?}",
        run.results
    );
    let mut again = mech(System::Gnome, Tools::default());
    assert!(discover_dconf(&mut again).found.is_empty());
    let outcomes = undo(&j, &mut a, &recipes, SCOPE);
    assert!(
        outcomes.iter().all(|o| *o == EffectUndoOutcome::Restored),
        "{outcomes:?}"
    );
}

#[test]
fn gnome_rows_keep_input_methods_and_respect_locked_keys() {
    let mut tools = Tools {
        calls: CALLS.with(|c| c.clone()),
        ..Tools::default()
    };
    tools.gsettings.insert(
        ("org.gnome.desktop.input-sources".into(), "sources".into()),
        "[('ibus', 'mozc-jp'), ('xkb', 'us')]".into(),
    );
    tools.gsettings.insert(
        ("org.gnome.desktop.interface".into(), "color-scheme".into()),
        "'default'".into(),
    );
    tools
        .locked
        .push(("org.gnome.desktop.interface".into(), "color-scheme".into()));
    let cat = Catalogue::starter().unwrap();
    let mut store = Store::new(cat.clone(), mech(System::Gnome, tools));
    let t = store.probe_target();
    assert!(t.managed.contains(&cat.row("appearance.theme").unwrap().id));
    assert_eq!(
        store.read("keyboard.layouts"),
        Ok(Some(SettingValue::Text("us".into())))
    );
    // Adding French keeps the Japanese input method first, where the person had it.
    store
        .write_if(
            "keyboard.layouts",
            &SettingValue::Text("us".into()),
            Some(&SettingValue::Text("us,fr".into())),
        )
        .unwrap();
    assert_eq!(
        store.read("keyboard.layouts"),
        Ok(Some(SettingValue::Text("us,fr".into())))
    );
    let calls = store_calls(&mut store);
    assert!(
        calls
            .iter()
            .any(|c| c.contains("[('ibus', 'mozc-jp'), ('xkb', 'us'), ('xkb', 'fr')]")),
        "{calls:?}"
    );
    // Undo takes only French away again.
    store
        .write_if(
            "keyboard.layouts",
            &SettingValue::Text("us,fr".into()),
            Some(&SettingValue::Text("us".into())),
        )
        .unwrap();
    assert!(
        store_calls(&mut store)
            .iter()
            .any(|c| c.ends_with("[('ibus', 'mozc-jp'), ('xkb', 'us')]"))
    );
}

fn store_calls(_s: &mut Store) -> Vec<String> {
    // The stand-in is owned by the store; its calls are shared through the Rc.
    CALLS.with(|c| c.borrow().clone())
}

thread_local! {
    static CALLS: std::rc::Rc<std::cell::RefCell<Vec<String>>> = std::rc::Rc::default();
}
