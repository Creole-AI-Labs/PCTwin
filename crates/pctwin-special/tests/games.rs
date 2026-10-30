//! Games and launchers (Task List 2.5; acceptance T25-038 to T25-041), on folders made as the
//! launchers and games leave them. Every game and launcher here is added only as catalogue rows.

use std::collections::BTreeMap;
use std::path::Path;
use std::time::{Duration, Instant};

use pctwin_catalogue::Catalogue;
use pctwin_record::Owner;
use pctwin_special::api::{Plan, Planned};
use pctwin_special::category::{Category, Note};
use pctwin_special::classify::Placement;
use pctwin_special::component::{Component, Custody, Evidence};
use pctwin_special::gamefmt::Tree;
use pctwin_special::games::{
    self, Catalogues, FindInput, Found, GameReport, InThisProcess, Places, Registry, Route, Source,
};
use pctwin_special::handoff::{Opened, Person, Step};
use pctwin_special::roots::{DataRoot, RootKind, System};
use pctwin_types::{ItemResult, ItemStatus, Reason};

static READER: InThisProcess = InThisProcess::new(pctwin_parse_seal::in_child());

fn put(p: &Path, bytes: &[u8]) {
    std::fs::create_dir_all(p.parent().unwrap()).unwrap();
    std::fs::write(p, bytes).unwrap();
}

fn cat<R: pctwin_catalogue::Row>(name: &str, rows: &str) -> Catalogue<R> {
    let c = Catalogue::<R>::shipped(&format!(
        r#"{{"format":1,"catalogue":"{name}","version":1,"rows":[{rows}]}}"#
    ));
    assert!(c.report().is_clean(), "{name}: {:?}", c.report());
    c
}

struct NoRegistry;
impl Registry for NoRegistry {
    fn tree(&self, _: &str) -> Result<Option<Tree>, String> {
        Ok(None)
    }
}

struct OneKey(Tree);
impl Registry for OneKey {
    fn tree(&self, path: &str) -> Result<Option<Tree>, String> {
        Ok((path == "HKEY_LOCAL_MACHINE/SOFTWARE/Kiln/Games").then(|| self.0.clone()))
    }
}

/// A Windows person's folders inside `root` (any host system).
fn places(root: &Path) -> Places {
    let home = root.join("home");
    let mut folders = BTreeMap::new();
    folders.insert("winAppData".into(), home.join("AppData/Roaming"));
    folders.insert("winLocalAppData".into(), home.join("AppData/Local"));
    folders.insert("winDocuments".into(), home.join("Documents"));
    folders.insert("winProgramFilesX86".into(), root.join("pf86"));
    folders.insert("winProgramData".into(), root.join("pd"));
    Places {
        system: System::Windows,
        home,
        user: Some("ada".into()),
        folders,
    }
}

fn roots(p: &Places) -> Vec<DataRoot> {
    vec![
        DataRoot {
            kind: RootKind::Roaming,
            path: p.folders["winAppData"].clone(),
        },
        DataRoot {
            kind: RootKind::Local,
            path: p.folders["winLocalAppData"].clone(),
        },
    ]
}

const DAY: &str = r#""checked_on":"2026-10-30""#;

/// A new launcher, `lantern`, that keeps a list of libraries and one text record per game,
/// and marks a game's saves synced online with a file.
fn lantern_row() -> String {
    format!(
        r#"{{"launcher":"lantern","name":"Lantern","system":"windows","base":["<winProgramFilesX86>/Lantern"],
        "libraries":{{"file":"libs.vdf","format":"vdf","at":["libs"],"each":true,"path":"where"}},
        "games":{{"folder":"records","files":"game_*.txt","format":"vdf","at":["Game"],"name":"title","id":"num","install":"dir","install_in":"installed"}},
        "cloud_marker":"sync/<storeUserId>/<storeGameId>/synced.vdf",
        "verify":"lantern://check/<storeGameId>","page":"https://example.org/lantern-help",{DAY}}}"#
    )
}

fn lantern(root: &Path) {
    let base = root.join("pf86/Lantern");
    let lib2 = root.join("games2");
    put(
        &base.join("libs.vdf"),
        format!(
            "\"libs\" {{ \"0\" {{ \"where\" \"{}\" }} \"1\" {{ \"where\" \"{}\" }} }}",
            base.display().to_string().replace('\\', "\\\\"),
            lib2.display().to_string().replace('\\', "\\\\")
        )
        .as_bytes(),
    );
    put(
        &base.join("records/game_7.txt"),
        b"\"Game\" { \"title\" \"Ember Road\" \"num\" \"7\" \"dir\" \"EmberRoad\" }",
    );
    put(&base.join("installed/EmberRoad/ember.bin"), &[1u8; 300]);
    put(
        &base.join("installed/EmberRoad/saves/slot1.dat"),
        b"save one",
    );
    // A record whose folder is not here: said, never invented.
    put(
        &lib2.join("records/game_8.txt"),
        b"\"Game\" { \"title\" \"Gone Game\" \"num\" \"8\" \"dir\" \"Gone\" }",
    );
    // A record naming a folder outside its library: never followed.
    put(
        &base.join("records/game_9.txt"),
        b"\"Game\" { \"title\" \"Climber\" \"num\" \"9\" \"dir\" \"../../outside\" }",
    );
    put(&root.join("outside/x.bin"), b"x");
    // Lantern syncs Ember Road's saves online.
    put(&base.join("sync/1001/7/synced.vdf"), b"\"x\" \"y\"");
}

fn input<'a>(
    p: &'a Places,
    to: System,
    r: &'a [DataRoot],
    rows: &'a Catalogues,
    registry: &'a dyn Registry,
    moving: &'a dyn Fn(&Path) -> bool,
) -> FindInput<'a> {
    FindInput {
        places: p,
        to,
        roots: r,
        rows,
        reader: &READER,
        registry,
        moving,
        until: Instant::now() + Duration::from_secs(30),
    }
}

fn owner() -> Owner {
    Owner::Person {
        account_id: "S-1-5-21-1".into(),
    }
}

fn save<'r>(r: &'r GameReport, game: &str) -> &'r games::SaveFound {
    r.saves
        .iter()
        .find(|s| s.game == game)
        .unwrap_or_else(|| panic!("{game} not found: {:?}", r.saves))
}

/// T25-040 and adoption change 7: a launcher and a game known only by new rows are found, the
/// game's step is the launcher's own verify, and its saves are found under its install folder.
#[test]
fn a_new_launcher_and_game_added_only_as_rows_work_end_to_end() {
    let dir = tempfile::tempdir().unwrap();
    let p = places(dir.path());
    lantern(dir.path());
    let rows = Catalogues {
        saves: cat(
            "game-saves",
            &format!(
                r#"{{"name":"Ember Road","files":{{"<base>/saves":{{"tags":["save"],"when":[{{"os":"windows","store":"lantern"}}]}}}},"installDir":{{"EmberRoad":{{}}}},{DAY}}}"#
            ),
        ),
        launchers: cat("game-launchers", &lantern_row()),
        places: cat("game-places", ""),
    };
    let r = roots(&p);
    let report = games::find(&input(&p, System::Windows, &r, &rows, &NoRegistry, &|_| {
        false
    }));
    assert_eq!(report.installed.len(), 1, "{:?}", report.installed);
    let g = &report.installed[0];
    assert_eq!(
        (g.name.as_str(), g.id.as_deref(), g.launcher.as_str()),
        ("Ember Road", Some("7"), "lantern")
    );
    assert!(g.folder.ends_with("installed/EmberRoad"));
    assert_eq!((g.files, g.bytes), (2, 308));
    assert!(g.cloud, "the launcher's own mark says it syncs");
    assert_eq!(g.verify.as_deref(), Some("lantern://check/7"));
    assert!(
        !g.copy_files,
        "installed games are not copied unless chosen"
    );
    assert_eq!(g.result, ItemResult::because(Reason::FinishByHand));
    assert!(
        report
            .not_checked
            .iter()
            .any(|(what, why)| what == "Gone Game" && why.contains("not on this laptop"))
    );
    // The launcher's verify is opened as the person, never run by PCTwin.
    struct Asked(std::sync::Mutex<Vec<String>>);
    impl Person for Asked {
        fn open_page(&self, url: &str) -> std::io::Result<()> {
            self.0.lock().unwrap().push(url.into());
            Ok(())
        }
        fn open_app(&self, _: &Path) -> std::io::Result<()> {
            panic!("no app is opened for a launcher's verify")
        }
    }
    let asked = Asked(std::sync::Mutex::new(Vec::new()));
    assert_eq!(games::open_verify(&asked, g), Opened::Opened);
    assert_eq!(*asked.0.lock().unwrap(), ["lantern://check/7"]);

    // Its saves (T25-039): the launcher keeps them online, so they are not proposed, and if
    // chosen they are kept beside, never over the launcher's copy.
    let s = save(&report, "Ember Road");
    assert_eq!(s.members, ["slot1.dat"]);
    assert_eq!(s.route, Route::Own);
    assert_eq!(s.cloud.as_deref(), Some("Lantern"));
    assert!(s.notes.contains(&Note::SavesInLauncherCloud));
    let planned = games::components(&owner(), &report);
    assert_eq!(planned.len(), 1);
    assert!(!planned[0].proposed);
    assert_eq!(planned[0].placement, Placement::PreservedOnly);
    assert_eq!(planned[0].category, Category::Game);
    assert_eq!(planned[0].component.custody, Custody::Portable);
    // Nothing chosen: left out, and the old laptop's copy is as it was.
    assert_eq!(
        pctwin_special::api::result_without_landing(&planned[0], false).result,
        ItemResult::skipped_by_user()
    );
    assert!(
        dir.path()
            .join("pf86/Lantern/installed/EmberRoad/saves/slot1.dat")
            .exists()
    );
    assert!(
        report
            .not_checked
            .iter()
            .any(|(what, why)| what == "Climber" && why.contains("plain name")),
        "{:?}",
        report.not_checked
    );
}

/// T25-039 for saves the app data move already takes: synced online by the launcher the game
/// came from, so not proposed, and if chosen kept beside.
#[test]
fn app_data_saves_the_launcher_keeps_online_are_not_proposed() {
    let dir = tempfile::tempdir().unwrap();
    let p = places(dir.path());
    lantern(dir.path());
    let ember = p.folders["winAppData"].join("EmberRoad");
    put(&ember.join("profile/slot2.dat"), b"two");
    let rows = Catalogues {
        saves: cat(
            "game-saves",
            &format!(
                r#"{{"name":"Ember Road","files":{{"<winAppData>/EmberRoad/profile":{{"tags":["save"]}}}},"installDir":{{"EmberRoad":{{}}}},{DAY}}}"#
            ),
        ),
        launchers: cat("game-launchers", &lantern_row()),
        places: cat("game-places", ""),
    };
    let r = roots(&p);
    let report = games::find(&input(&p, System::Windows, &r, &rows, &NoRegistry, &|_| {
        false
    }));
    let s = save(&report, "Ember Road");
    assert_eq!(s.route, Route::WithAppData);
    assert!(games::components(&owner(), &report).is_empty());
    let mut plan = Plan {
        components: vec![Planned {
            base: ember.clone(),
            placement: Placement::Same {
                root: RootKind::Roaming,
                folder: "EmberRoad".into(),
            },
            ..app_component(&p, &["profile/slot2.dat"])
        }],
        not_moved: Vec::new(),
        coverage: Default::default(),
        reports: Vec::new(),
    };
    games::enrich(&mut plan, &report, System::Windows, System::Windows);
    let c = &plan.components[0];
    assert_eq!(c.category, Category::Game);
    assert!(!c.proposed);
    assert_eq!(c.placement, Placement::PreservedOnly);
    assert!(c.notes.contains(&Note::SavesInLauncherCloud));
}

/// Saves in app data are left to the app data move and marked; saves outside the home become
/// their own component, kept (needs-action) since nothing can put them back by itself.
#[test]
fn saves_go_where_they_belong() {
    let dir = tempfile::tempdir().unwrap();
    let p = places(dir.path());
    let roaming = p.folders["winAppData"].clone();
    put(&roaming.join("QuestA/Saves/slot1.sav"), b"one");
    put(&roaming.join("QuestA/settings.ini"), b"[x]");
    put(&dir.path().join("pf86/QuestB/save/a.sav"), b"a");
    put(&dir.path().join("pf86/QuestB/save/a.log"), b"not a save");
    put(&p.home.join("Documents/My Games/QuestC/s.dat"), b"c");
    let rows = Catalogues {
        saves: cat(
            "game-saves",
            &format!(
                r#"{{"name":"Quest A","files":{{"<winAppData>/QuestA/Saves":{{"tags":["save"],"when":[{{"os":"windows"}}]}},"<xdgData>/QuestA/Saves":{{"tags":["save"],"when":[{{"os":"linux"}}]}}}},"cloud":{{"steam":true}},{DAY}}},
                {{"name":"Quest B","files":{{"<winProgramFilesX86>/QuestB/save/*.sav":{{"tags":["save"]}}}},{DAY}}},
                {{"name":"Quest C","files":{{"<winDocuments>/My Games/QuestC":{{}}}},{DAY}}}"#
            ),
        ),
        launchers: cat("game-launchers", ""),
        places: cat("game-places", ""),
    };
    let r = roots(&p);
    let docs = p.home.join("Documents");
    let moving = move |x: &Path| x.starts_with(&docs);
    let report = games::find(&input(&p, System::Windows, &r, &rows, &NoRegistry, &moving));
    let a = save(&report, "Quest A");
    assert_eq!(a.route, Route::WithAppData);
    assert_eq!(a.source, Source::Pctwin);
    assert_eq!(a.members, ["slot1.sav"]);
    // Not installed through a launcher here: its row's cloud flag is a caution, not a skip.
    assert_eq!(a.cloud, None);
    assert!(a.notes.contains(&Note::CloudSavesMayDiffer));
    let b = save(&report, "Quest B");
    assert_eq!(b.route, Route::Own);
    assert_eq!(b.members, ["a.sav"]);
    assert!(b.base.ends_with("QuestB/save"));
    assert_eq!(save(&report, "Quest C").route, Route::WithFiles);
    let planned = games::components(&owner(), &report);
    assert_eq!(planned.len(), 1, "only Quest B needs its own component");
    assert_eq!(planned[0].app, "Quest B");
    assert!(planned[0].proposed);
    assert_eq!(planned[0].step, Step::None);
    assert_eq!(planned[0].placement, Placement::PreservedOnly);
    // Kept, not handed to the game: needs-action, never "copied".
    let words = pctwin_special::api::result_without_landing(&planned[0], true);
    assert_eq!(words.result.status(), ItemStatus::NeedsAction);
}

fn quest_rows(same: &str) -> Catalogues {
    Catalogues {
        saves: cat(
            "game-saves",
            &format!(
                r#"{{"name":"Quest A","files":{{"<winAppData>/QuestA/Saves":{{"tags":["save"],"when":[{{"os":"windows"}}]}},"<xdgData>/QuestA/Saves":{{"tags":["save"],"when":[{{"os":"linux"}}]}}}},"same_format":[{same}],{DAY}}}"#
            ),
        ),
        launchers: cat("game-launchers", ""),
        places: cat("game-places", ""),
    }
}

fn app_component(p: &Places, members: &[&str]) -> Planned {
    let base = p.folders["winAppData"].join("QuestA");
    Planned {
        app: "QuestA".into(),
        component: Component::new(
            owner(),
            "roaming/QuestA",
            members.iter().map(|m| (*m).to_string()).collect(),
            Custody::Portable,
            false,
            vec![Evidence::NoSpecialShape],
        ),
        base,
        step: Step::None,
        why: String::new(),
        proposed: true,
        placement: Placement::PreservedOnly,
        root: RootKind::Roaming,
        category: Category::Other,
        notes: Vec::new(),
    }
}

/// T25-038: between systems a save is handed to the game only when its row says the format is
/// the same and names the one place there; otherwise it is kept, "may not work".
#[test]
fn saves_between_systems_go_to_the_game_only_when_a_row_says_the_format_is_the_same() {
    let dir = tempfile::tempdir().unwrap();
    let p = places(dir.path());
    put(
        &p.folders["winAppData"].join("QuestA/Saves/slot1.sav"),
        b"one",
    );
    let r = roots(&p);
    let rows = quest_rows(r#""windows","linux""#);
    let report = games::find(&input(&p, System::Linux, &r, &rows, &NoRegistry, &|_| {
        false
    }));
    let a = save(&report, "Quest A");
    assert_eq!(
        a.placement,
        Placement::Translated {
            root: RootKind::XdgData,
            folder: "QuestA/Saves".into()
        }
    );
    assert!(!a.notes.contains(&Note::SaveMayNotWorkHere));
    // The app data component holding them is handed to the game's folder there.
    let mut plan = Plan {
        components: vec![app_component(&p, &["Saves/slot1.sav"])],
        not_moved: Vec::new(),
        coverage: Default::default(),
        reports: Vec::new(),
    };
    games::enrich(&mut plan, &report, System::Windows, System::Linux);
    assert_eq!(plan.components[0].category, Category::Game);
    assert_eq!(
        plan.components[0].placement,
        Placement::Translated {
            root: RootKind::XdgData,
            folder: "QuestA".into()
        }
    );
    // With other files beside the saves, the whole folder is not the game's: kept.
    let mut plan = Plan {
        components: vec![app_component(&p, &["Saves/slot1.sav", "settings.ini"])],
        not_moved: Vec::new(),
        coverage: Default::default(),
        reports: Vec::new(),
    };
    games::enrich(&mut plan, &report, System::Windows, System::Linux);
    assert_eq!(plan.components[0].placement, Placement::PreservedOnly);
    assert!(plan.components[0].notes.contains(&Note::SaveMayNotWorkHere));

    // No row saying the format is the same: kept on the new laptop, "may not work".
    for same in ["", r#""windows","mac""#] {
        let rows = quest_rows(same);
        let report = games::find(&input(&p, System::Linux, &r, &rows, &NoRegistry, &|_| {
            false
        }));
        let a = save(&report, "Quest A");
        assert_eq!(a.placement, Placement::PreservedOnly, "{same}");
        assert!(a.notes.contains(&Note::SaveMayNotWorkHere));
    }
}

/// T25-041 and the store-protected places: programs among saves or add-ons are found by their
/// content, listed apart and never copied; a store's own saves are warned, never copied.
#[test]
fn programs_and_store_protected_saves_are_never_copied() {
    let dir = tempfile::tempdir().unwrap();
    let p = places(dir.path());
    lantern(dir.path());
    let mods = dir.path().join("pf86/Lantern/installed/EmberRoad/Mods");
    put(&mods.join("BetterLight/manifest.json"), br#"{"name":"x"}"#);
    // A program by its content, whatever its name.
    put(
        &mods.join("BetterLight/light.data"),
        b"\x7fELF\x02\x01\x01\0\0\0\0\0\0\0\0\0",
    );
    let wgs = p.folders["winLocalAppData"].join("Packages/Maker.Game_abc/SystemAppData/wgs");
    put(&wgs.join("container.1"), b"store data");
    let rows = Catalogues {
        saves: cat(
            "game-saves",
            &format!(
                r#"{{"name":"Ember Road","files":{{"<base>/saves":{{}}}},"installDir":{{"EmberRoad":{{}}}},{DAY}}},
                {{"name":"Store Game","files":{{"<winLocalAppData>/Packages/Maker.Game_abc/SystemAppData/wgs":{{}}}},{DAY}}}"#
            ),
        ),
        launchers: cat("game-launchers", &lantern_row()),
        places: cat(
            "game-places",
            &format!(
                r#"{{"system":"windows","kind":"store-protected","path":"<winLocalAppData>/Packages/*/SystemAppData/wgs","name":"Store saves",{DAY}}},
                {{"system":"windows","kind":"mods","path":"<base>/Mods","name":"Ember Road add-ons","game":"Ember Road",{DAY}}}"#
            ),
        ),
    };
    let r = roots(&p);
    let report = games::find(&input(&p, System::Windows, &r, &rows, &NoRegistry, &|_| {
        false
    }));
    assert_eq!(report.protected.len(), 1);
    assert_eq!(report.protected[0].files, 1);
    // A row naming the store's own place finds nothing to copy there.
    assert!(report.saves.iter().all(|s| !s.base.starts_with(&wgs)));
    assert!(report.saves.iter().all(|s| s.game != "Store Game"));
    let m = report
        .saves
        .iter()
        .find(|s| s.what == Found::Mods)
        .expect("the add-on folder");
    assert_eq!(m.members, ["BetterLight/manifest.json"]);
    assert_eq!(m.programs, ["BetterLight/light.data"]);
    assert!(m.notes.contains(&Note::ModProgramsHeld));
    let planned = games::components(&owner(), &report);
    let mod_part = planned
        .iter()
        .find(|x| x.base == m.base)
        .expect("add-on data kept");
    assert!(
        !mod_part
            .component
            .members
            .iter()
            .any(|x| x.ends_with("light.data")),
        "a program is never in a component"
    );

    // In the app data move: the store's files and programs are taken out and listed.
    let base = p.folders["winLocalAppData"].join("Packages/Maker.Game_abc");
    let store = Planned {
        app: "Maker.Game".into(),
        component: Component::new(
            owner(),
            "local/Packages/Maker.Game_abc",
            vec![
                "SystemAppData/wgs/container.1".into(),
                "Settings/settings.dat".into(),
            ],
            Custody::Portable,
            false,
            vec![Evidence::NoSpecialShape],
        ),
        base,
        step: Step::None,
        why: String::new(),
        proposed: true,
        placement: Placement::PreservedOnly,
        root: RootKind::Local,
        category: Category::Other,
        notes: Vec::new(),
    };
    let only_store = Planned {
        component: Component::new(
            owner(),
            "local/Packages/Maker.Game_abc",
            vec!["SystemAppData/wgs/container.1".into()],
            Custody::Portable,
            false,
            vec![Evidence::NoSpecialShape],
        ),
        ..store.clone()
    };
    let mut plan = Plan {
        components: vec![store, only_store],
        not_moved: Vec::new(),
        coverage: Default::default(),
        reports: Vec::new(),
    };
    games::enrich(&mut plan, &report, System::Windows, System::Windows);
    assert_eq!(
        plan.components.len(),
        1,
        "a component of store saves only is not moved"
    );
    assert_eq!(
        plan.components[0].component.members,
        ["Settings/settings.dat"]
    );
    assert_eq!(plan.not_moved.len(), 2);
    assert!(plan.not_moved.iter().all(|n| n.why.contains("store")));
}

/// The person's own save list (Ludusavi's copy of its manifest) is read, through the parser's
/// reader, for installed games no PCTwin row knows.
#[test]
fn the_persons_own_save_list_is_read_for_their_installed_games() {
    let dir = tempfile::tempdir().unwrap();
    let p = places(dir.path());
    lantern(dir.path());
    put(
        &p.folders["winAppData"].join("ludusavi/manifest.yaml"),
        b"---\nEmber Road:\n  files:\n    <winDocuments>/Ember/*.sav:\n      when:\n        - os: windows\n  installDir:\n    EmberRoad: {}\nNot Installed:\n  files:\n    <winDocuments>/Other:\n      when:\n        - os: windows\n",
    );
    put(&p.home.join("Documents/Ember/one.sav"), b"1");
    put(&p.home.join("Documents/Other/two.sav"), b"2");
    let rows = Catalogues {
        saves: cat("game-saves", ""),
        launchers: cat("game-launchers", &lantern_row()),
        places: cat(
            "game-places",
            &format!(
                r#"{{"system":"windows","kind":"save-list","path":"<winAppData>/ludusavi/manifest.yaml","name":"Your Ludusavi save list",{DAY}}}"#
            ),
        ),
    };
    let r = roots(&p);
    let report = games::find(&input(&p, System::Windows, &r, &rows, &NoRegistry, &|_| {
        false
    }));
    let list = report.save_list.as_ref().expect("the list was read");
    assert_eq!((list.entries, list.unreadable, list.more), (1, 0, false));
    let s = save(&report, "Ember Road");
    assert_eq!(s.source, Source::PersonsList);
    assert_eq!(s.members, ["one.sav"]);
    assert!(report.saves.iter().all(|s| s.game != "Not Installed"));
}

/// Records in JSON files and in the registry are read by the same rows mechanism.
#[test]
fn json_and_registry_records_list_games() {
    let dir = tempfile::tempdir().unwrap();
    let p = places(dir.path());
    let game = dir.path().join("games/Comet");
    put(&game.join("comet.bin"), &[0u8; 10]);
    put(
        &dir.path().join("pd/Kite/Manifests/a.item"),
        format!(
            r#"{{"DisplayName":"Comet","AppName":"comet01","InstallLocation":"{}"}}"#,
            game.display().to_string().replace('\\', "\\\\")
        )
        .as_bytes(),
    );
    put(
        &dir.path().join("pd/Kite/Manifests/broken.item"),
        b"{ not json",
    );
    let reg_game = dir.path().join("games/Nova");
    put(&reg_game.join("nova.bin"), &[0u8; 4]);
    let reg = OneKey(Tree::Map(vec![(
        "1207".into(),
        Tree::Map(vec![
            ("gameName".into(), Tree::Text("Nova".into())),
            ("gameID".into(), Tree::Text("1207".into())),
            ("path".into(), Tree::Text(reg_game.display().to_string())),
        ]),
    )]));
    let rows = Catalogues {
        saves: cat("game-saves", ""),
        launchers: cat(
            "game-launchers",
            &format!(
                r#"{{"launcher":"kite","name":"Kite","system":"windows","base":["<winProgramData>/Kite"],
                "games":{{"folder":"Manifests","files":"*.item","format":"json","name":"DisplayName","id":"AppName","install":"InstallLocation"}},{DAY}}},
                {{"launcher":"kiln","name":"Kiln","system":"windows",
                "games":{{"registry":"HKEY_LOCAL_MACHINE/SOFTWARE/Kiln/Games","format":"registry","each":true,"name":"gameName","id":"gameID","install":"path"}},{DAY}}}"#
            ),
        ),
        places: cat("game-places", ""),
    };
    let r = roots(&p);
    let report = games::find(&input(&p, System::Windows, &r, &rows, &reg, &|_| false));
    let names: Vec<(&str, &str)> = report
        .installed
        .iter()
        .map(|g| (g.name.as_str(), g.launcher.as_str()))
        .collect();
    assert_eq!(names, [("Comet", "kite"), ("Nova", "kiln")]);
    assert_eq!(report.installed[0].bytes, 10);
    assert!(report.installed.iter().all(|g| g.verify.is_none()));
    assert!(
        report
            .not_checked
            .iter()
            .any(|(what, _)| what.ends_with("broken.item")),
        "a record that cannot be read is said: {:?}",
        report.not_checked
    );
}

/// Rows that would reach beyond what they may are refused when the catalogue is read.
#[test]
fn rows_that_overreach_are_refused() {
    let bad_launchers = [
        // Verify through another program's scheme.
        r#"{"launcher":"x","name":"X","system":"windows","base":["<home>/X"],"games":{"folder":"r","files":"*.v","format":"vdf","name":"n","install":"i"},"verify":"file:///c/evil","checked_on":"2026-10-30"}"#,
        // Records outside the launcher's folder.
        r#"{"launcher":"x","name":"X","system":"windows","base":["<home>/X"],"games":{"folder":"../r","files":"*.v","format":"vdf","name":"n","install":"i"},"checked_on":"2026-10-30"}"#,
        // The registry read as a file.
        r#"{"launcher":"x","name":"X","system":"windows","games":{"registry":"HKEY_LOCAL_MACHINE/SOFTWARE/X","format":"vdf","name":"n","install":"i"},"checked_on":"2026-10-30"}"#,
        // Another hive.
        r#"{"launcher":"x","name":"X","system":"windows","games":{"registry":"HKEY_USERS/X","format":"registry","name":"n","install":"i"},"checked_on":"2026-10-30"}"#,
    ];
    for row in bad_launchers {
        let c = Catalogue::<games::LauncherRow>::shipped(&format!(
            r#"{{"format":1,"catalogue":"game-launchers","version":1,"rows":[{row}]}}"#
        ));
        assert!(c.rows().is_empty(), "{row}");
    }
    let bad_saves = [
        r#"{"name":"G","files":{"C:/Games/G":{}},"checked_on":"2026-10-30"}"#,
        r#"{"name":"G","files":{"<home>/../G":{}},"checked_on":"2026-10-30"}"#,
        r#"{"name":"G","files":{},"checked_on":"2026-10-30"}"#,
        r#"{"name":"G","alias":"H","files":{"<home>/G":{}},"checked_on":"2026-10-30"}"#,
        r#"{"name":"G","files":{"<home>/G":{}},"checked_on":"soon"}"#,
    ];
    for row in bad_saves {
        let c = Catalogue::<games::SaveRow>::shipped(&format!(
            r#"{{"format":1,"catalogue":"game-saves","version":1,"rows":[{row}]}}"#
        ));
        assert!(c.rows().is_empty(), "{row}");
    }
    let c = Catalogue::<games::GamePlaceRow>::shipped(
        r#"{"format":1,"catalogue":"game-places","version":1,"rows":[{"system":"windows","kind":"mods","path":"<base>/Mods","name":"M","checked_on":"2026-10-30"}]}"#,
    );
    assert!(c.rows().is_empty(), "an add-on folder names its game");
}
