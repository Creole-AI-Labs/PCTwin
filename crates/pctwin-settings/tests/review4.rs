//! The fourth fresh-context review's proofs (findings F1, F2, F4, F5, F10), kept as regression
//! tests: each failed on the code the review read and passes now.

mod common;

use common::{SCOPE, actor, journal, plan_of, rebuild};
use pctwin_recipe::{Arch, Os, Platform, SettingValue, SettingsRoot};
use pctwin_settings::catalogue::{Catalogue, System};
use pctwin_settings::classify::{Ctx, FolderMap, Why};
use pctwin_settings::files::{FileCtx, FileOutcome, FoundFile, format_of, plan_file};
use pctwin_settings::found::{
    Address, Discovery, FoundSetting, MAX_FOUND, RegKind, discover_registry, parse_dconf_dump,
};
use pctwin_settings::generic::{FoundPlan, Move, plan_found};
use pctwin_settings::mechanism::Mechanisms;
use pctwin_settings::registry::{Hive, MemHive, RegValue};
use pctwin_settings::{SettingsAdapter, Store};
use pctwin_types::ItemStatus;
use std::collections::{BTreeMap, BTreeSet};

const WIN: Platform = Platform {
    os: Os::Windows,
    arch: Arch::X86_64,
};

fn reg(key: &str, name: &str, kind: RegKind, v: &str) -> FoundSetting {
    FoundSetting {
        address: Address::Registry {
            key: key.into(),
            name: name.into(),
            kind,
        },
        label: "x".into(),
        value: SettingValue::Text(v.into()),
    }
}

fn dconf(path: &str, key: &str, v: &str) -> FoundSetting {
    FoundSetting {
        address: Address::Dconf {
            path: path.into(),
            key: key.into(),
        },
        label: "x".into(),
        value: SettingValue::Text(v.into()),
    }
}

/// Every value the review planted on a hostile old laptop that the old planner would have
/// written onto the new one.
fn hostile() -> Vec<FoundSetting> {
    vec![
        reg(
            r"Control Panel\Desktop",
            "SCRNSAVE.EXE",
            RegKind::Sz,
            r"\\evil.example\s\x.scr",
        ),
        reg(
            r"Control Panel\Desktop",
            "SCRNSAVE.EXE",
            RegKind::Sz,
            r"C:/Windows/System32/cmd.exe",
        ),
        reg(
            r"Control Panel\Desktop",
            "SCRNSAVE.EXE",
            RegKind::ExpandSz,
            r"%SystemRoot%\System32\cmd.exe",
        ),
        // A moved file, which the old planner pointed at its new place.
        reg(
            r"Control Panel\Desktop",
            "SCRNSAVE.EXE",
            RegKind::Sz,
            r"C:\Users\Ada\Documents\x.dat",
        ),
        reg(
            r"Control Panel\Desktop",
            "Wallpaper",
            RegKind::Sz,
            r"\\evil.example\s\a.jpg",
        ),
        reg(
            r"Control Panel\Cursors",
            "Arrow",
            RegKind::ExpandSz,
            r"\\evil.example\s\a.cur",
        ),
        reg(
            r"Software\SimonTatham\PuTTY\Sessions\s",
            "ProxyTelnetCommand",
            RegKind::Sz,
            "cmd /c calc",
        ),
        reg(
            r"Software\Vendor\App",
            "Editor",
            RegKind::Sz,
            r"\\?\C:\Windows\System32\cmd.exe",
        ),
        reg(
            r"Software\Vendor\App",
            "Start",
            RegKind::Sz,
            "powershell -nop -w hidden -c x",
        ),
        reg(
            r"Software\Vendor\App",
            "Feed",
            RegKind::Sz,
            "https://evil.example/u.xml",
        ),
        reg(
            r"Software\Vendor\App",
            "Open",
            RegKind::Sz,
            "search-ms:query=x&crumb=location:\\\\evil\\s",
        ),
        reg(r"Software\Vendor\App", "Pipe", RegKind::Sz, "a | b"),
        reg(
            r"Software\Vendor\App",
            "Folders",
            RegKind::MultiSz,
            r#"["ok","\\\\evil\\s"]"#,
        ),
        dconf("/org/vendor/app/", "command", "'sh -c id'"),
        dconf(
            "/org/vendor/app/",
            "places",
            "['/home/ada/x', 'smb://evil/s']",
        ),
    ]
}

fn plan(cat: &Catalogue, found: Vec<FoundSetting>, hints: bool) -> FoundPlan {
    let d = Discovery {
        found,
        not_moved: vec![],
    };
    let map = FolderMap(vec![
        (
            r"C:\Users\Ada\Documents".into(),
            r"C:\Users\Ada2\Documents".into(),
        ),
        ("/home/ada".into(), "/home/ada2".into()),
    ]);
    let ctx = Ctx {
        paths: &map,
        same_screens: false,
        hints: if hints { &cat.action_hints } else { &[] },
        facts: pctwin_settings::judge::Facts::system(),
    };
    plan_found(cat, System::Windows, System::Windows, &d, &ctx)
}

#[test]
fn f1_nothing_that_could_run_or_fetch_is_ever_planned_as_a_write() {
    let cat = Catalogue::starter().unwrap();
    let p = plan(&cat, hostile(), true);
    assert!(p.moves.is_empty(), "{:#?}", p.moves);
    // Each is reported with its plain reason, never dropped; the desktop picture is the
    // catalogue row's, whose value check takes only a full local path to an arrived file.
    assert_eq!(p.not_moved.len() + p.by_catalogue, hostile().len());
    assert_eq!(p.by_catalogue, 1);
    assert!(
        pctwin_settings::value::ValueType::FilePath
            .check(
                "wallpaper",
                &SettingValue::Text(r"\\evil.example\s\a.jpg".into())
            )
            .is_err()
    );
    let why = |name: &str| -> Vec<Why> {
        p.not_moved
            .iter()
            .filter(|n| n.name == name)
            .map(|n| n.why.clone())
            .collect()
    };
    // A cursor file is something the system loads, wherever it is.
    assert!(why("Arrow").contains(&Why::StartsProgram));
    assert!(why("ProxyTelnetCommand").contains(&Why::StartsProgram));
    assert!(why("command").contains(&Why::StartsProgram));
    assert!(why("SCRNSAVE.EXE").contains(&Why::StartsProgram));
    for n in &p.not_moved {
        assert!(!n.why.words().is_empty());
    }
}

#[test]
fn f1_with_no_hint_rows_at_all_the_shape_rule_still_keeps_back_every_proof_case() {
    let mut cat = Catalogue::starter().unwrap();
    cat.action_hints.clear();
    let p = plan(&cat, hostile(), false);
    assert!(p.moves.is_empty(), "{:#?}", p.moves);
    assert_eq!(p.not_moved.len() + p.by_catalogue, hostile().len());
}

#[test]
fn f1_plain_data_still_moves_and_a_moved_document_is_pointed_at_its_new_place() {
    let cat = Catalogue::starter().unwrap();
    for hints in [true, false] {
        let p = plan(
            &cat,
            vec![
                reg(r"Software\Vendor\App", "Theme", RegKind::Sz, "ocean"),
                reg(r"Software\Vendor\App", "Zoom", RegKind::Sz, "125"),
                reg(
                    r"Software\Vendor\App",
                    "LastFile",
                    RegKind::Sz,
                    r"C:\Users\Ada\Documents\a b\x.txt",
                ),
                reg(
                    r"Software\Vendor\App",
                    "Export",
                    RegKind::Sz,
                    "C:/Users/Ada/Documents/out",
                ),
                reg(
                    r"Control Panel\Desktop",
                    "WheelScrollLines",
                    RegKind::Sz,
                    "3",
                ),
                dconf("/org/vendor/app/", "theme", "'dark'"),
                dconf("/org/vendor/app/", "sizes", "[1, 2, 3]"),
            ],
            hints,
        );
        let moved = |n: &str| {
            p.moves
                .iter()
                .find(|m| m.address.name() == n)
                .map(|m| m.value.clone())
        };
        assert_eq!(moved("Theme"), Some(SettingValue::Text("ocean".into())));
        assert_eq!(moved("Zoom"), Some(SettingValue::Text("125".into())));
        assert_eq!(
            moved("LastFile"),
            Some(SettingValue::Text(
                r"C:\Users\Ada2\Documents\a b\x.txt".into()
            ))
        );
        assert_eq!(
            moved("Export"),
            Some(SettingValue::Text("C:/Users/Ada2/Documents/out".into()))
        );
        assert!(moved("WheelScrollLines").is_some());
        assert!(
            moved("theme").is_some() && moved("sizes").is_some(),
            "{p:#?}"
        );
    }
}

#[test]
fn f1_writing_is_scoped_to_what_discovery_reads() {
    let w = |k: &str, n: &str| {
        Address::Registry {
            key: k.into(),
            name: n.into(),
            kind: RegKind::Sz,
        }
        .writable()
    };
    assert!(w(r"Software\Vendor\App", "a"));
    assert!(w(r"Control Panel\Desktop", "a"));
    for (k, n) in [
        (r"Software\WOW6432Node\x", "a"),
        (r"SOFTWARE\CLASSES\x", "a"),
        (r"Software\Policies\x", "a"),
        (r"Software\Microsoft\x", "a"),
        (r"Software\Vendor\..\Classes", "a"),
        (r"Software\Vendor\.\x", "a"),
        (r"Software\Vendor\App", ""),
        (r"Environment", "PATH"),
        (r"Keyboard Layout\Preload", "1"),
        (r"AppEvents\Schemes", "a"),
        (r"Software", "a"),
        (r"Software\Vendor\App\shell\open\command", "a"),
    ] {
        assert!(!w(k, n), "{k} {n}");
    }
    // No deeper than discovery walks.
    let deep = format!(r"Software\Vendor{}", r"\L".repeat(12));
    assert!(w(&deep, "a"));
    assert!(!w(&format!(r"{deep}\L"), "a"));
}

/// A plan the planner could never make, forged straight into the executor's recipes.
fn forged(address: Address, value: &str) -> FoundPlan {
    FoundPlan {
        moves: vec![Move {
            key: address.key(),
            address,
            label: "Forged".into(),
            value: SettingValue::Text(value.into()),
            rewritten: false,
        }],
        ..FoundPlan::default()
    }
}

fn run_forged(p: &FoundPlan) -> (ItemStatus, Box<dyn Hive>) {
    let (_d, j) = journal();
    let mut store = Store::new(
        Catalogue::starter().unwrap(),
        Mechanisms {
            system: System::Windows,
            user: Some(Box::new(MemHive::default())),
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
    let status = run.results[0].1.status();
    (status, a.store.mechanisms().user.take().unwrap())
}

#[test]
fn f1_every_write_is_checked_again_on_the_new_laptop() {
    for (key, name, value) in [
        (r"Software\Vendor\App", "Theme", r"\\evil.example\s\x"),
        (r"Software\Vendor\App", "Theme", "cmd /c calc"),
        (
            r"Software\Vendor\App",
            "Theme",
            "%SystemRoot%\\System32\\cmd",
        ),
        (
            r"Control Panel\Desktop",
            "SCRNSAVE.EXE",
            r"C:\Users\Ada2\Documents\x.dat",
        ),
        (r"Software\Vendor\App", "Editor", "notepad"),
    ] {
        let a = Address::Registry {
            key: key.into(),
            name: name.into(),
            kind: RegKind::Sz,
        };
        let (status, hive) = run_forged(&forged(a, value));
        assert_eq!(status, ItemStatus::Failed, "{name}={value}");
        assert_eq!(hive.get(key, name).unwrap(), None, "{name}={value}");
    }
    // A plain value still goes through the same door.
    let a = Address::Registry {
        key: r"Software\Vendor\App".into(),
        name: "Theme".into(),
        kind: RegKind::Sz,
    };
    let (status, hive) = run_forged(&forged(a, "ocean"));
    assert_eq!(status, ItemStatus::Converted);
    assert_eq!(
        hive.get(r"Software\Vendor\App", "Theme").unwrap(),
        Some(RegValue::Sz("ocean".into()))
    );
}

#[test]
fn f1_a_place_discovery_never_reads_is_never_remembered() {
    let mut store = Store::new(
        Catalogue::starter().unwrap(),
        Mechanisms {
            system: System::Windows,
            user: Some(Box::new(MemHive::default())),
            machine: None,
            runner: None,
            localtime: None,
        },
    );
    let a = Address::Registry {
        key: r"Software\WOW6432Node\Vendor".into(),
        name: "x".into(),
        kind: RegKind::Sz,
    };
    assert!(store.remember(BTreeMap::from([(a.key(), a)])).is_err());
}

struct Env {
    map: FolderMap,
    running: BTreeSet<String>,
    versions: BTreeMap<String, (String, String)>,
    renames: BTreeMap<String, String>,
}

fn env() -> Env {
    Env {
        map: FolderMap(vec![(
            r"C:\Users\ada\Documents".into(),
            r"D:\New\Docs".into(),
        )]),
        running: BTreeSet::new(),
        versions: BTreeMap::new(),
        renames: BTreeMap::new(),
    }
}

fn file_plan(name: &str, bytes: &[u8]) -> FileOutcome {
    let cat = Catalogue::starter().unwrap();
    let e = env();
    let c = FileCtx {
        classify: Ctx {
            paths: &e.map,
            same_screens: false,
            hints: &cat.action_hints,
            facts: pctwin_settings::judge::Facts::system(),
        },
        from: System::Windows,
        to: System::Windows,
        home_old: r"C:\Users\ada",
        home_new: r"C:\Users\ada",
        running: &e.running,
        versions: &e.versions,
        folder_renames: &e.renames,
    };
    let f = FoundFile {
        root: SettingsRoot::Config,
        rel: format!("R4Vendor/{name}"),
        label: "R4Vendor".into(),
        format: format_of(name),
    };
    plan_file(&cat, &f, bytes, &c)
}

#[test]
fn f2_deep_text_is_refused_before_any_parser_on_a_small_stack() {
    // The review's crash: 3,000 levels of XML on a 1 MB stack (the Windows main thread's).
    let t = std::thread::Builder::new()
        .stack_size(1024 * 1024)
        .spawn(|| {
            let mut out = Vec::new();
            for (name, text) in [
                (
                    "d.xml",
                    format!("{}v{}", "<a>".repeat(3000), "</a>".repeat(3000)),
                ),
                (
                    "d.json",
                    format!("{}1{}", "[".repeat(10_000), "]".repeat(10_000)),
                ),
                (
                    "d.toml",
                    format!("a = {}1{}\n", "[".repeat(10_000), "]".repeat(10_000)),
                ),
                ("k.toml", format!("{} = 1\n", vec!["k"; 10_000].join("."))),
                (
                    "d.jsonc",
                    format!("// x\n{}1{}", "[".repeat(10_000), "]".repeat(10_000)),
                ),
            ] {
                out.push((name, file_plan(name, text.as_bytes())));
            }
            out
        })
        .unwrap();
    for (name, o) in t.join().expect("no stack overflow") {
        assert_eq!(o, FileOutcome::NotMoved(Why::TooDeep), "{name}");
    }
    assert_eq!(Why::TooDeep.words(), "found, not read: too deeply nested");
}

#[test]
fn f2_nesting_at_the_limit_is_still_read() {
    let n = pctwin_types::limits::NESTING;
    let t = std::thread::Builder::new()
        .stack_size(1024 * 1024)
        .spawn(move || {
            file_plan(
                "ok.xml",
                format!("{}v{}", "<a>".repeat(n), "</a>".repeat(n)).as_bytes(),
            )
        })
        .unwrap();
    assert!(matches!(t.join().unwrap(), FileOutcome::Move { .. }));
}

#[test]
fn f2_preferences_wifi_and_dconf_go_through_the_same_gate() {
    let deep = format!(
        "<plist version=\"1.0\"><dict><key>k</key>{}<string>v</string>{}</dict></plist>",
        "<array>".repeat(5000),
        "</array>".repeat(5000)
    );
    let t = std::thread::Builder::new()
        .stack_size(1024 * 1024)
        .spawn(move || pctwin_settings::plist::parse(&deep))
        .unwrap();
    assert_eq!(
        t.join().unwrap(),
        Err("found, not read: too deeply nested".to_string())
    );
    // An entity could nest past the gate once expanded: never read.
    let ent = "<!DOCTYPE plist [<!ENTITY e \"<array></array>\">]><plist version=\"1.0\"><array>&e;</array></plist>";
    assert!(pctwin_settings::plist::parse(ent).is_err());
    let wifi = format!(
        "<WLANProfile xmlns=\"http://www.microsoft.com/networking/WLAN/profile/v1\">{}{}</WLANProfile>",
        "<x>".repeat(5000),
        "</x>".repeat(5000)
    );
    let t = std::thread::Builder::new()
        .stack_size(1024 * 1024)
        .spawn(move || pctwin_settings::wifi::parse_profile(&wifi).is_err())
        .unwrap();
    assert!(t.join().unwrap());
    let d = parse_dconf_dump(&format!(
        "[a]\nk={}'x'{}\nok=1\n",
        "[".repeat(300),
        "]".repeat(300)
    ));
    assert_eq!(d.found.len(), 1);
    assert!(
        d.not_moved
            .iter()
            .any(|n| n.name == "k" && n.why == Why::TooDeep)
    );
}

#[test]
fn f4_a_quoted_value_with_a_comment_keeps_its_quotes_and_comment() {
    let ini = "[s]\r\nk1=C:\\Users\\ada\\Documents\\a\r\nk2=\"C:\\Users\\ada\\Documents\\b\" ; x\r\n k5 = \"C:\\Users\\ada\\Documents\\d\"\r\nk6=C:\\Users\\ada\\Documents\\e # note\n";
    let FileOutcome::Move {
        bytes, rewritten, ..
    } = file_plan("inj.ini", ini.as_bytes())
    else {
        panic!("the file moves");
    };
    let out = String::from_utf8(bytes).unwrap();
    assert_eq!(
        out,
        "[s]\r\nk1=D:\\New\\Docs\\a\r\nk2=\"D:\\New\\Docs\\b\" ; x\r\n k5 = \"D:\\New\\Docs\\d\"\r\nk6=D:\\New\\Docs\\e # note\n"
    );
    assert_eq!(rewritten, 4);
}

#[test]
fn f5_nothing_is_dropped_silently_from_the_registry() {
    let mut h = MemHive::default();
    h.put(
        r"Software\R4A\App",
        "",
        RegValue::Sz("default value".into()),
    );
    h.put(r"Software\R4A\App", "named", RegValue::Sz("x".into()));
    let mut deep = String::from(r"Software\R4B");
    for i in 0..15 {
        deep.push_str(&format!(r"\L{i}"));
    }
    h.put(&deep, "bottom", RegValue::Sz("deep".into()));
    for i in 0..(MAX_FOUND + 10) {
        h.put(r"Software\R4C\Big", &format!("v{i}"), RegValue::Dword(1));
    }
    h.put(r"Software\R4D", "after", RegValue::Sz("late".into()));
    let d = discover_registry(&h);
    let note = |store: &str, name: &str| {
        d.not_moved
            .iter()
            .find(|n| n.store.contains(store) && n.name == name)
            .map(|n| n.why.clone())
    };
    assert_eq!(note(r"R4A\App", ""), Some(Why::Unnamed));
    assert_eq!(
        note(r"R4B\L0\L1\L2\L3\L4\L5\L6\L7\L8\L9\L10\L11", "*"),
        Some(Why::TooDeep)
    );
    // The cap is per value, and what it left unread is named.
    assert_eq!(d.found.len(), MAX_FOUND);
    assert_eq!(note(r"R4C\Big", "*"), Some(Why::TooMany));
    assert_eq!(note("Software\\R4D", "*"), Some(Why::TooMany));
}

#[test]
fn f5_nothing_is_dropped_silently_from_dconf() {
    let mut dump = String::from("[a]\n");
    for i in 0..(MAX_FOUND + 5) {
        dump.push_str(&format!("k{i}=1\n"));
    }
    dump.push_str("[b]\nlast='x'\nnoequals\n");
    let d = parse_dconf_dump(&dump);
    assert_eq!(d.found.len(), MAX_FOUND);
    let why = |p: &str| {
        d.not_moved
            .iter()
            .filter(|n| n.store == p)
            .map(|n| n.why.clone())
            .collect::<Vec<_>>()
    };
    assert_eq!(why("/a/"), vec![Why::TooMany]);
    assert!(why("/b/").contains(&Why::TooMany));
    assert!(why("/b/").iter().any(|w| matches!(w, Why::Unreadable(_))));
}

#[test]
fn f5_json_deeper_than_its_parser_reads_is_reported_not_dropped() {
    // Past serde's own limit (128) but inside the gate: a plain reason, not "damaged".
    let j = format!("{}1{}", "[".repeat(200), "]".repeat(200));
    assert_eq!(
        file_plan("d.json", j.as_bytes()),
        FileOutcome::NotMoved(Why::TooDeep)
    );
}

#[test]
fn f10_a_file_too_large_to_plan_is_reported() {
    let big = vec![b' '; (pctwin_settings::files::MAX_FILE + 1) as usize];
    assert_eq!(
        file_plan("big.json", &big),
        FileOutcome::NotMoved(Why::TooLarge)
    );
}

/// Every parser behind the gate, fuzzed here as well as by `fuzz/settings_parsers` (which CI runs
/// on a nightly compiler): seeds mutated with brackets, quotes and comment marks, run on a 1 MB
/// stack (the Windows main thread's). Never a panic, never a stack overflow.
#[test]
fn f2_every_parser_behind_the_gate_survives_mutated_input_on_a_small_stack() {
    let t = std::thread::Builder::new()
        .stack_size(1024 * 1024)
        .spawn(|| {
            let seeds: &[(&str, String)] = &[
                ("f.json", r#"{"a":[1,"C:\\Users\\ada\\Documents\\f.txt",{"b":null}], // c
 "t":true,}"#.to_string()),
                ("f.toml", "a = 1\n[s]\nb = 'C:\\Users\\ada\\Documents\\f.txt'\nc = [1,2]\n[[t]]\nx={y=1}\n".to_string()),
                ("f.ini", "[s]\nk=\"C:\\Users\\ada\\Documents\\f.txt\" ; c\nx=1\n".to_string()),
                ("f.xml", r#"<r a="C:\Users\ada\Documents\f.txt"><o name="n" value="1"/><p>x</p><![CDATA[x]]><!-- c --></r>"#.to_string()),
            ];
            let mut s = 0x9e37_79b9_7f4a_7c15u64;
            let mut rnd = move || {
                s ^= s << 13;
                s ^= s >> 7;
                s ^= s << 17;
                s
            };
            let marks = b"[]{}<>\"'\\=\n\r&;#/*!-<>[[[[{{{{";
            for (name, seed) in seeds {
                for _ in 0..800 {
                    let mut b = seed.as_bytes().to_vec();
                    for _ in 0..(rnd() % 8 + 1) {
                        let p = (rnd() as usize) % (b.len() + 1);
                        match rnd() % 5 {
                            0 if !b.is_empty() => {
                                let q = p.min(b.len() - 1);
                                b[q] = marks[(rnd() as usize) % marks.len()];
                            }
                            1 => b.insert(p, marks[(rnd() as usize) % marks.len()]),
                            2 if p < b.len() => {
                                b.remove(p);
                            }
                            3 => {
                                // A run of openers: deep nesting in the middle of anything.
                                let o = marks[(rnd() as usize) % 6];
                                let n = (rnd() % 600) as usize;
                                b.splice(p..p, std::iter::repeat_n(o, n));
                            }
                            _ => {
                                let q = (rnd() as usize) % (b.len() + 1);
                                let (lo, hi) = (p.min(q), p.max(q));
                                let chunk = b[lo..hi].to_vec();
                                b.splice(p..p, chunk);
                            }
                        }
                    }
                    let _ = file_plan(name, &b);
                    let text = String::from_utf8_lossy(&b);
                    let _ = pctwin_settings::plist::parse(&text);
                    let _ = pctwin_settings::wifi::parse_profile(&text);
                    let _ = parse_dconf_dump(&text);
                    let _ = pctwin_settings::gvariant::strings(&text);
                    let _ = Catalogue::from_json(&b);
                }
            }
        })
        .unwrap();
    t.join().expect("no panic and no stack overflow");
}
