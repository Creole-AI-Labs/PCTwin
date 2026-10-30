//! App settings files found by shape and moved field by field (Task List 2.4: same-OS app
//! settings, cross-OS translations with VS Code and JetBrains first, recent files rewritten;
//! T24-003, T24-005, T24-017, T24-018).

use std::collections::{BTreeMap, BTreeSet};

use pctwin_recipe::SettingsRoot;
use pctwin_settings::catalogue::{Catalogue, System, Writer};
use pctwin_settings::classify::{Ctx, FolderMap, Why};
use pctwin_settings::files::{
    FileCtx, FileOutcome, FoundFile, Left, OLDER_APP, discover_files, format_of, plan_file,
    rewrite_recent_list, strip_jsonc, translate,
};

fn folders() -> FolderMap {
    FolderMap(vec![
        (
            "C:\\Users\\ada\\Documents".into(),
            "C:\\Users\\ada\\Documents".into(),
        ),
        ("C:\\Users\\ada\\code".into(), "/Users/ada/code".into()),
        ("/home/ada/Projects".into(), "/home/ada/Projects".into()),
    ])
}

struct Env {
    map: FolderMap,
    running: BTreeSet<String>,
    versions: BTreeMap<String, (String, String)>,
    renames: BTreeMap<String, String>,
}

impl Env {
    fn new() -> Self {
        Self {
            map: folders(),
            running: BTreeSet::new(),
            versions: BTreeMap::new(),
            renames: BTreeMap::new(),
        }
    }

    fn ctx(&self, from: System, to: System) -> FileCtx<'_> {
        FileCtx {
            classify: Ctx {
                paths: &self.map,
                same_screens: false,
                hints: pctwin_settings::catalogue::Catalogue::starter_hints(),
                facts: pctwin_settings::judge::Facts::system(),
            },
            from,
            to,
            home_old: "/home/ada",
            home_new: "/home/ada",
            running: &self.running,
            versions: &self.versions,
            folder_renames: &self.renames,
        }
    }
}

fn file(root: SettingsRoot, rel: &str) -> FoundFile {
    FoundFile {
        root,
        rel: rel.into(),
        label: rel.split('/').next().unwrap().into(),
        format: format_of(rel),
    }
}

fn moved(o: FileOutcome) -> (String, String, Vec<Left>, bool) {
    match o {
        FileOutcome::Move {
            rel,
            bytes,
            left,
            equivalent,
            ..
        } => (rel, String::from_utf8(bytes).unwrap(), left, equivalent),
        other => panic!("not moved: {other:?}"),
    }
}

fn why(left: &[Left], field: &str) -> Option<Why> {
    left.iter()
        .find(|l| l.field == field)
        .map(|l| l.why.clone())
}

const VSCODE: &str = r#"{
    // My editor
    "editor.fontSize": 15,
    "workbench.colorTheme": "Solarized Dark",
    "git.defaultCloneDirectory": "C:\\Users\\ada\\code",
    "python.defaultInterpreterPath": "C:\\Python312\\python.exe",
    "terminal.integrated.profiles.windows": { "Git": { "path": "C:\\Git\\bin\\bash.exe" } },
    "telemetry.machineId": "abc",
    "github.copilot.advanced": { "authToken": "ghp_0123456789abcdefghijABCDEFGHIJ012345" },
    "remote.SSH.configFile": "C:\\Users\\ada\\.ssh\\config",
    "files.exclude": { "**/.git": true, },
    /* trailing comma below */
    "window.zoomLevel": 1,
}"#;

#[test]
fn vs_code_settings_move_field_by_field_with_the_catalogues_rules() {
    let cat = Catalogue::starter().unwrap();
    let env = Env::new();
    let f = file(SettingsRoot::Config, "Code/User/settings.json");
    let (rel, out, left, equivalent) = moved(plan_file(
        &cat,
        &f,
        VSCODE.as_bytes(),
        &env.ctx(System::Windows, System::Mac),
    ));
    assert_eq!(rel, "Code/User/settings.json");
    assert!(!equivalent);
    let v: serde_json::Value = serde_json::from_str(&out).unwrap();
    assert_eq!(v["editor.fontSize"], 15);
    assert_eq!(v["workbench.colorTheme"], "Solarized Dark");
    assert_eq!(v["git.defaultCloneDirectory"], "/Users/ada/code");
    assert_eq!(v["files.exclude"]["**/.git"], true);
    assert_eq!(v["window.zoomLevel"], 1);
    for gone in [
        "python.defaultInterpreterPath",
        "terminal.integrated.profiles.windows",
        "telemetry.machineId",
        "remote.SSH.configFile",
    ] {
        assert!(v.get(gone).is_none(), "{gone}");
    }
    assert!(v["github.copilot.advanced"].get("authToken").is_none());
    assert_eq!(
        why(&left, "github.copilot.advanced/authToken"),
        Some(Why::Secret)
    );
    assert_eq!(
        why(&left, "terminal.integrated.profiles.windows"),
        Some(Why::StartsProgram)
    );
    assert!(!out.contains("ghp_"));
}

#[test]
fn t24_003_a_start_up_task_is_kept_but_switched_off() {
    let cat = Catalogue::starter().unwrap();
    let env = Env::new();
    let tasks = r#"{ "version": "2.0.0", "tasks": [ { "label": "watch", "type": "npm", "script": "watch", "runOptions": { "runOn": "folderOpen" } } ] }"#;
    let f = file(SettingsRoot::Config, "Code/User/tasks.json");
    let (_, out, left, _) = moved(plan_file(
        &cat,
        &f,
        tasks.as_bytes(),
        &env.ctx(System::Windows, System::Windows),
    ));
    let v: serde_json::Value = serde_json::from_str(&out).unwrap();
    assert_eq!(v["tasks"][0]["runOptions"]["runOn"], "default");
    assert_eq!(v["tasks"][0]["label"], "watch");
    assert_eq!(
        why(&left, "tasks/0/runOptions/runOn"),
        Some(Why::StartsProgram)
    );
}

#[test]
fn keyboard_shortcuts_between_a_mac_and_windows_are_an_equivalent() {
    let cat = Catalogue::starter().unwrap();
    let env = Env::new();
    let keys = r#"[ { "key": "ctrl+shift+k", "command": "editor.action.deleteLines" },
                    { "key": "ctrl+k ctrl+c", "command": "editor.action.addCommentLine" },
                    { "key": "ctrl+alt+t", "command": "workbench.action.terminal.sendSequence", "args": { "text": "rm -rf ~\n" } } ]"#;
    let f = file(SettingsRoot::Config, "Code/User/keybindings.json");
    let (_, out, left, equivalent) = moved(plan_file(
        &cat,
        &f,
        keys.as_bytes(),
        &env.ctx(System::Windows, System::Mac),
    ));
    assert!(equivalent);
    let v: serde_json::Value = serde_json::from_str(&out).unwrap();
    assert_eq!(v[0]["key"], "cmd+shift+k");
    assert_eq!(v[1]["key"], "cmd+k cmd+c");
    assert!(v[2].get("args").is_none());
    assert_eq!(why(&left, "2/args"), Some(Why::StartsProgram));
    // Same system: unchanged.
    let (_, out, _, equivalent) = moved(plan_file(
        &cat,
        &f,
        keys.as_bytes(),
        &env.ctx(System::Windows, System::Windows),
    ));
    assert!(!equivalent);
    assert!(out.contains("ctrl+shift+k"));
    let t = &cat.tables["vscode-keys"];
    assert_eq!(
        translate(t, "cmd+s", System::Mac, System::Windows, true),
        "ctrl+s"
    );
}

#[test]
fn t24_017_files_another_role_writes_are_left_to_it() {
    let cat = Catalogue::starter().unwrap();
    let env = Env::new();
    let ctx = env.ctx(System::Windows, System::Windows);
    let o = |p: &str| plan_file(&cat, &file(SettingsRoot::Config, p), b"{}", &ctx);
    assert_eq!(
        o("Code/User/globalStorage/storage.json"),
        FileOutcome::OtherWriter(Writer::Data)
    );
    assert_eq!(
        o("JetBrains/IntelliJIdea2025.2/options/security.xml"),
        FileOutcome::OtherWriter(Writer::Nobody)
    );
    assert_eq!(
        o("JetBrains/IntelliJIdea2025.2/plugins/x/p.xml"),
        FileOutcome::OtherWriter(Writer::Install)
    );
}

#[test]
fn t24_018_a_running_apps_files_wait_until_it_is_closed() {
    let cat = Catalogue::starter().unwrap();
    let mut env = Env::new();
    env.running.insert("vscode".into());
    let f = file(SettingsRoot::Config, "Code/User/settings.json");
    assert_eq!(
        plan_file(&cat, &f, b"{}", &env.ctx(System::Windows, System::Windows)),
        FileOutcome::WaitForClose
    );
}

const RECENT: &str = r#"<application>
  <component name="RecentProjectsManager">
    <option name="additionalInfo">
      <map>
        <entry key="$USER_HOME$/Projects/site"><value><RecentProjectMetaInfo opened="true"/></value></entry>
        <entry key="$USER_HOME$/Old/gone"><value><RecentProjectMetaInfo/></value></entry>
      </map>
    </option>
    <option name="lastProjectLocation" value="$USER_HOME$/Projects" />
  </component>
</application>"#;

#[test]
fn jetbrains_recent_projects_are_rewritten_and_missing_ones_left_out() {
    let cat = Catalogue::starter().unwrap();
    let mut env = Env::new();
    env.map = FolderMap(vec![("/home/ada/Projects".into(), "/home/ada/Work".into())]);
    env.renames
        .insert("IntelliJIdea2025.2".into(), "IntelliJIdea2025.3".into());
    let f = file(
        SettingsRoot::Config,
        "JetBrains/IntelliJIdea2025.2/options/recentProjects.xml",
    );
    let (rel, out, left, _) = moved(plan_file(
        &cat,
        &f,
        RECENT.as_bytes(),
        &env.ctx(System::Gnome, System::Gnome),
    ));
    assert_eq!(
        rel,
        "JetBrains/IntelliJIdea2025.3/options/recentProjects.xml"
    );
    assert!(
        out.contains(r#"<entry key="$USER_HOME$/Work/site">"#),
        "{out}"
    );
    assert!(!out.contains("Old/gone"), "{out}");
    assert!(out.contains(r#"value="$USER_HOME$/Work""#), "{out}");
    assert!(roxmltree::Document::parse(&out).is_ok());
    assert!(
        left.iter()
            .any(|l| l.why == Why::PathNotMoved || l.why == Why::RecentMissing)
    );
}

#[test]
fn t24_005_an_older_app_on_the_new_laptop_is_a_guided_step() {
    let cat = Catalogue::starter().unwrap();
    let mut env = Env::new();
    env.renames
        .insert("IntelliJIdea2025.2".into(), "IntelliJIdea2024.3".into());
    let f = file(
        SettingsRoot::Config,
        "JetBrains/IntelliJIdea2025.2/options/editor.xml",
    );
    assert_eq!(
        plan_file(
            &cat,
            &f,
            b"<application/>",
            &env.ctx(System::Windows, System::Windows)
        ),
        FileOutcome::Guided(OLDER_APP)
    );
    let mut env = Env::new();
    env.versions
        .insert("vscode".into(), ("1.99".into(), "1.95".into()));
    let f = file(SettingsRoot::Config, "Code/User/settings.json");
    assert_eq!(
        plan_file(&cat, &f, b"{}", &env.ctx(System::Windows, System::Windows)),
        FileOutcome::Guided(OLDER_APP)
    );
    let mut env = Env::new();
    env.versions
        .insert("vscode".into(), ("1.50".into(), "1.99".into()));
    assert_eq!(
        plan_file(&cat, &f, b"{}", &env.ctx(System::Windows, System::Windows)),
        FileOutcome::Guided(OLDER_APP)
    );
}

#[test]
fn an_unknown_apps_files_move_same_system_by_the_generic_rules() {
    let cat = Catalogue::starter().unwrap();
    let env = Env::new();
    let ctx = env.ctx(System::Gnome, System::Gnome);
    let json = r#"{ "theme": "ocean", "deviceId": "abc-123", "lastFolder": "/home/ada/Projects/x", "exportTo": "/srv/old", "token": "t", "window": { "width": 900 } }"#;
    let (_, out, left, _) = moved(plan_file(
        &cat,
        &file(SettingsRoot::Config, "fakeapp/config.json"),
        json.as_bytes(),
        &ctx,
    ));
    let v: serde_json::Value = serde_json::from_str(&out).unwrap();
    assert_eq!(v["theme"], "ocean");
    assert_eq!(v["lastFolder"], "/home/ada/Projects/x");
    assert!(v.get("deviceId").is_none() && v.get("token").is_none() && v.get("exportTo").is_none());
    assert_eq!(why(&left, "deviceId"), Some(Why::MachineId));
    assert_eq!(why(&left, "exportTo"), Some(Why::PathNotMoved));
    assert_eq!(why(&left, "window/width"), Some(Why::WindowPlace));
    let ini = "; comment\n[general]\ntheme = ocean\npassword = hunter2\nfolder=\"/home/ada/Projects/y\"\n";
    let (_, out, left, _) = moved(plan_file(
        &cat,
        &file(SettingsRoot::Config, "fakeapp/app.ini"),
        ini.as_bytes(),
        &ctx,
    ));
    assert_eq!(
        out,
        "; comment\n[general]\ntheme = ocean\nfolder=\"/home/ada/Projects/y\"\n"
    );
    assert_eq!(why(&left, "general/password"), Some(Why::Secret));
    let toml = "theme = \"ocean\"\n[sync]\napi_key = \"abc\"\n";
    let (_, out, left, _) = moved(plan_file(
        &cat,
        &file(SettingsRoot::Config, "fakeapp/app.toml"),
        toml.as_bytes(),
        &ctx,
    ));
    assert!(out.contains("theme = \"ocean\"") && !out.contains("abc"));
    assert_eq!(why(&left, "sync/api_key"), Some(Why::Secret));
    // Another system: only what a catalogue entry describes.
    assert_eq!(
        plan_file(
            &cat,
            &file(SettingsRoot::Config, "fakeapp/config.json"),
            json.as_bytes(),
            &env.ctx(System::Gnome, System::Mac)
        ),
        FileOutcome::NotMoved(Why::OtherSystem)
    );
    // Something that is not what its name says does not move.
    assert_eq!(
        plan_file(
            &cat,
            &file(SettingsRoot::Config, "fakeapp/broken.json"),
            b"{ nope",
            &ctx
        ),
        FileOutcome::NotMoved(Why::Kind)
    );
}

#[test]
fn vs_codes_recent_list_is_rewritten_for_the_data_role() {
    let env = Env::new();
    let list = r#"{"entries":[{"folderUri":"file:///c%3A/Users/ada/Documents/proj"},{"fileUri":"file:///c%3A/Temp/x.txt"},{"folderUri":"vscode-remote://ssh-remote%2Bbox/home/x"}]}"#;
    let (out, left) =
        rewrite_recent_list(list, &env.ctx(System::Windows, System::Windows)).unwrap();
    let v: serde_json::Value = serde_json::from_str(&out).unwrap();
    let entries = v["entries"].as_array().unwrap();
    assert_eq!(entries.len(), 1, "{out}");
    assert_eq!(
        entries[0]["folderUri"],
        "file:///C%3A/Users/ada/Documents/proj"
    );
    // A remote address is kept back (opening it connects to another computer), and said so.
    assert!(left.iter().any(|l| l.why == Why::Fetches), "{left:?}");
    assert!(left.iter().any(|l| l.why == Why::PathNotMoved), "{left:?}");
}

#[test]
fn jsonc_comments_and_trailing_commas_are_read() {
    assert_eq!(
        strip_jsonc("{\"a\": \"//x\", // c\n \"b\": [1,2,], /* d */}")
            .unwrap()
            .replace(' ', ""),
        "{\"a\":\"//x\",\n\"b\":[1,2]}"
    );
    assert_eq!(
        strip_jsonc("{\"a\": \"\\\"/*\"}").unwrap(),
        "{\"a\": \"\\\"/*\"}"
    );
    assert!(strip_jsonc("{/* open").is_none());
    assert!(strip_jsonc("{\"open").is_none());
}

#[test]
fn settings_files_are_found_by_shape_and_caches_skipped() {
    let dir = tempfile::tempdir().unwrap();
    let mk = |p: &str, body: &str| {
        let path = dir.path().join(p);
        std::fs::create_dir_all(path.parent().unwrap()).unwrap();
        std::fs::write(path, body).unwrap();
    };
    mk("FakeApp/settings.json", "{}");
    mk("FakeApp/sub/prefs.ini", "a=1");
    mk("FakeApp/Cache/data.json", "{}");
    mk("FakeApp/logs/x.json", "{}");
    mk("FakeApp/image.png", "x");
    mk("top-level.json", "{}");
    let found = discover_files(&[(SettingsRoot::Config, dir.path().to_path_buf())]);
    let rels: Vec<&str> = found.iter().map(|f| f.rel.as_str()).collect();
    assert_eq!(rels, ["FakeApp/settings.json", "FakeApp/sub/prefs.ini"]);
    assert!(found.iter().all(|f| f.label == "FakeApp"));
}
