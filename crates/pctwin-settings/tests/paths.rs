//! Workspaces and recent files after a move (Task List 2.4: workspace restore and recent-file
//! paths rewritten): old home folders and drive letters become the new laptop's places, only for
//! what the move really brought, never anywhere outside the person's home on the new laptop.

use std::collections::{BTreeMap, BTreeSet};

use pctwin_recipe::SettingsRoot;
use pctwin_settings::catalogue::{Catalogue, System};
use pctwin_settings::classify::{Ctx, FolderMap, Why};
use pctwin_settings::files::{
    FileCtx, FileOutcome, FoundFile, format_of, plan_file, recent_list_for, rewrite_document,
    rewrite_recent_list,
};

fn home(s: System) -> &'static str {
    match s {
        System::Windows => "C:\\Users\\ada",
        System::Mac => "/Users/ada",
        _ => "/home/ada",
    }
}

struct Env {
    map: FolderMap,
    running: BTreeSet<String>,
    versions: BTreeMap<String, (String, String)>,
    renames: BTreeMap<String, String>,
}

impl Env {
    fn new(pairs: &[(&str, &str)]) -> Self {
        Self {
            map: FolderMap(
                pairs
                    .iter()
                    .map(|(a, b)| (a.to_string(), b.to_string()))
                    .collect(),
            ),
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
                hints: Catalogue::starter_hints(),
                facts: pctwin_settings::judge::Facts::system(),
            },
            from,
            to,
            home_old: home(from),
            home_new: home(to),
            running: &self.running,
            versions: &self.versions,
            folder_renames: &self.renames,
        }
    }
}

fn entries(out: &str) -> Vec<String> {
    let v: serde_json::Value = serde_json::from_str(out).unwrap();
    v["entries"]
        .as_array()
        .unwrap()
        .iter()
        .map(|e| {
            e.as_object()
                .unwrap()
                .values()
                .next()
                .unwrap()
                .as_str()
                .unwrap()
                .to_string()
        })
        .collect()
}

#[test]
fn vs_codes_recent_list_moves_from_windows_to_a_mac_and_back() {
    // Ada's code folder on drive D: and her documents both moved into her Mac home.
    let env = Env::new(&[
        ("D:\\code", "/Users/ada/code"),
        ("C:\\Users\\ada\\Documents", "/Users/ada/Documents"),
    ]);
    let list = r#"{"entries":[
        {"folderUri":"file:///d%3A/code/site"},
        {"fileUri":"file:///c%3A/Users/ada/Documents/notes%20one.md"},
        {"folderUri":"file:///c%3A/Users/ada/Downloads/old"},
        {"workspace":{"configPath":"file:///d%3A/code/all.code-workspace"}}
    ]}"#;
    let (out, left) = rewrite_recent_list(list, &env.ctx(System::Windows, System::Mac)).unwrap();
    let v: serde_json::Value = serde_json::from_str(&out).unwrap();
    let e = v["entries"].as_array().unwrap();
    assert_eq!(e.len(), 3, "{out}");
    assert_eq!(e[0]["folderUri"], "file:///Users/ada/code/site");
    assert_eq!(
        e[1]["fileUri"],
        "file:///Users/ada/Documents/notes%20one.md"
    );
    assert_eq!(
        e[2]["workspace"]["configPath"],
        "file:///Users/ada/code/all.code-workspace"
    );
    // Downloads did not move: left out, and said why.
    assert!(left.iter().any(|l| l.why == Why::PathNotMoved), "{left:?}");

    // And from a Mac to Windows: a Windows file address.
    let env = Env::new(&[("/Users/ada/code", "C:\\Users\\ada\\code")]);
    let (out, _) = rewrite_recent_list(
        r#"{"entries":[{"folderUri":"file:///Users/ada/code/site"}]}"#,
        &env.ctx(System::Mac, System::Windows),
    )
    .unwrap();
    assert_eq!(entries(&out), ["file:///C%3A/Users/ada/code/site"]);
}

#[test]
fn a_path_the_move_put_outside_the_persons_home_is_never_pointed_at() {
    // A folder map that would send a folder outside the new home (or to another person's) is
    // not followed: the entry is left out, never pointed there.
    for to in [
        "C:\\Windows\\Temp\\code",
        "C:\\Users\\bob\\code",
        "C:\\Users\\adam\\code",
        "D:\\code",
    ] {
        let env = Env::new(&[("C:\\Users\\ada\\code", to)]);
        let (out, left) = rewrite_recent_list(
            r#"{"entries":[{"folderUri":"file:///c%3A/Users/ada/code/site"}]}"#,
            &env.ctx(System::Windows, System::Windows),
        )
        .unwrap();
        assert!(entries(&out).is_empty(), "{to}: {out}");
        assert!(!left.is_empty(), "{to}");
    }
    // Inside the home: pointed at.
    let env = Env::new(&[("C:\\Users\\ada\\code", "C:\\Users\\ada\\Projects\\code")]);
    let (out, _) = rewrite_recent_list(
        r#"{"entries":[{"folderUri":"file:///c%3A/Users/ada/code/site"}]}"#,
        &env.ctx(System::Windows, System::Windows),
    )
    .unwrap();
    assert_eq!(entries(&out), ["file:///C%3A/Users/ada/Projects/code/site"]);
}

#[test]
fn jetbrains_recent_projects_move_from_windows_to_a_mac() {
    let cat = Catalogue::starter().unwrap();
    let env = Env::new(&[
        ("C:\\Users\\ada\\IdeaProjects", "/Users/ada/IdeaProjects"),
        ("D:\\work", "/Users/ada/work"),
    ]);
    // JetBrains writes Windows paths with forward slashes, and the home as `$USER_HOME$`.
    let xml = r#"<application>
  <component name="RecentProjectsManager">
    <option name="additionalInfo">
      <map>
        <entry key="$USER_HOME$/IdeaProjects/shop" />
        <entry key="D:/work/api" />
        <entry key="E:/elsewhere/gone" />
      </map>
    </option>
    <option name="lastProjectLocation" value="$USER_HOME$/IdeaProjects" />
  </component>
</application>
"#;
    let f = FoundFile {
        root: SettingsRoot::Config,
        rel: "JetBrains/GoLand2025.2/options/recentProjects.xml".into(),
        label: "JetBrains".into(),
        format: format_of("recentProjects.xml"),
    };
    match plan_file(
        &cat,
        &f,
        xml.as_bytes(),
        &env.ctx(System::Windows, System::Mac),
    ) {
        FileOutcome::Move { bytes, left, .. } => {
            let out = String::from_utf8(bytes).unwrap();
            assert!(
                out.contains(r#"<entry key="$USER_HOME$/IdeaProjects/shop" />"#),
                "{out}"
            );
            // A full path from another drive becomes the Mac's full path.
            assert!(
                out.contains(r#"<entry key="/Users/ada/work/api" />"#),
                "{out}"
            );
            assert!(!out.contains("elsewhere"), "{out}");
            assert!(out.contains(r#"value="$USER_HOME$/IdeaProjects""#), "{out}");
            assert!(left.iter().any(|l| l.why == Why::PathNotMoved), "{left:?}");
        }
        other => panic!("{other:?}"),
    }
}

#[test]
fn a_workspace_file_points_at_the_moved_folders_and_keeps_everything_else() {
    let cat = Catalogue::starter().unwrap();
    let env = Env::new(&[
        ("C:\\Users\\ada\\code\\api", "/Users/ada/code/api"),
        ("D:\\shared", "/Users/ada/shared"),
    ]);
    let ws = r#"{
	// Ada's main workspace
	"folders": [
		{ "path": "C:\\Users\\ada\\code\\api" },
		{ "path": "web" },
		{ "uri": "file:///d%3A/shared/docs" },
		{ "path": "E:\\gone" },
		{ "uri": "vscode-remote://ssh-remote+box/srv" },
	],
	"settings": { "editor.tabSize": 2 },
}"#;
    let ctx = env.ctx(System::Windows, System::Mac);
    let r = rewrite_document(&cat, "Ada.code-workspace", ws.as_bytes(), &ctx)
        .expect("a workspace file is a document the catalogue names")
        .unwrap();
    let out = String::from_utf8(r.bytes).unwrap();
    assert_eq!(r.rewritten, 2);
    assert!(
        out.contains(r#"{ "path": "/Users/ada/code/api" }"#),
        "{out}"
    );
    assert!(
        out.contains(r#"{ "uri": "file:///Users/ada/shared/docs" }"#),
        "{out}"
    );
    // A relative folder moves with the file; one not moved and a remote one stay as the person
    // wrote them (it is their file), each reported.
    assert!(out.contains(r#"{ "path": "web" }"#), "{out}");
    assert!(out.contains(r#"{ "path": "E:\\gone" }"#), "{out}");
    assert!(out.contains("vscode-remote://ssh-remote+box/srv"), "{out}");
    assert!(
        out.contains("// Ada's main workspace"),
        "comments kept: {out}"
    );
    assert!(out.contains(r#""editor.tabSize": 2"#));
    let whys: Vec<_> = r.left.iter().map(|l| (l.field.as_str(), &l.why)).collect();
    assert!(
        whys.contains(&("folders/3/path", &Why::PathNotMoved)),
        "{whys:?}"
    );
    assert!(whys.contains(&("folders/4/uri", &Why::Fetches)), "{whys:?}");
    // Any other file is not a document the catalogue names.
    assert!(rewrite_document(&cat, "notes.json", b"{}", &ctx).is_none());
}

#[test]
fn a_path_written_twice_in_other_spellings_is_left_with_its_reason() {
    let cat = Catalogue::starter().unwrap();
    let env = Env::new(&[("C:\\Users\\ada\\code", "C:\\Users\\ada\\Projects")]);
    // The same folder written with an escaped slash: the file is not changed blindly.
    let ws =
        r#"{"folders": [{"path": "C:\\Users\\ada\\code"}, {"path": "C:\u005cUsers\\ada\\code"}]}"#;
    let r = rewrite_document(
        &cat,
        "two.code-workspace",
        ws.as_bytes(),
        &env.ctx(System::Windows, System::Windows),
    )
    .unwrap()
    .unwrap();
    assert_eq!(r.rewritten, 0);
    assert_eq!(String::from_utf8(r.bytes).unwrap(), ws);
    assert_eq!(r.left.len(), 2, "{:?}", r.left);
}

#[test]
fn the_catalogue_names_which_stored_values_are_recent_lists() {
    let cat = Catalogue::starter().unwrap();
    assert!(recent_list_for(
        &cat,
        SettingsRoot::Config,
        "Code/User/globalStorage/state.vscdb",
        "history.recentlyOpenedPathsList"
    ));
    assert!(!recent_list_for(
        &cat,
        SettingsRoot::Config,
        "Code/User/globalStorage/state.vscdb",
        "workbench.panel.width"
    ));
    assert!(!recent_list_for(
        &cat,
        SettingsRoot::Home,
        "Code/User/globalStorage/state.vscdb",
        "history.recentlyOpenedPathsList"
    ));
}
