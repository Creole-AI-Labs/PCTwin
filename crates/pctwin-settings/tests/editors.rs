//! Code editors between systems (Task List 2.4: cross-OS translations, VS Code and JetBrains
//! first): keyboard shortcuts and keymaps through catalogue tables, and add-ons carried as a list
//! with an honest result each. The editors are catalogue rows; a new one is a new row.

use std::collections::{BTreeMap, BTreeSet};

use pctwin_recipe::SettingsRoot;
use pctwin_settings::catalogue::{Catalogue, ListForm, System};
use pctwin_settings::classify::{Ctx, FolderMap};
use pctwin_settings::extensions::{
    self, BY_FOLDER, FROM_A_FILE, NOT_A_NAME, NOT_THERE, Planned, read_folders, read_json,
};
use pctwin_settings::files::{FileCtx, FileOutcome, FoundFile, format_of, plan_file, translate};
use pctwin_types::ItemStatus;

struct Env {
    map: FolderMap,
    running: BTreeSet<String>,
    versions: BTreeMap<String, (String, String)>,
    renames: BTreeMap<String, String>,
}

impl Env {
    fn new() -> Self {
        Self {
            map: FolderMap::default(),
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
            home_old: "/home/ada",
            home_new: "/home/ada",
            running: &self.running,
            versions: &self.versions,
            folder_renames: &self.renames,
        }
    }
}

fn file(rel: &str) -> FoundFile {
    FoundFile {
        root: SettingsRoot::Config,
        rel: rel.into(),
        label: rel.split('/').next().unwrap().into(),
        format: format_of(rel),
    }
}

/// The moved text and whether it is an equivalent the person confirms.
fn moved(o: FileOutcome) -> (String, bool) {
    match o {
        FileOutcome::Move {
            bytes, equivalent, ..
        } => (String::from_utf8(bytes).unwrap(), equivalent),
        other => panic!("not moved: {other:?}"),
    }
}

const KEYS: &str = r#"[
    { "key": "cmd+s", "command": "workbench.action.files.save" },
    { "key": "ctrl+tab", "command": "workbench.action.nextEditor" },
    { "key": "cmd+k cmd+c", "command": "editor.action.addCommentLine" },
    { "key": "shift+alt+f", "command": "editor.action.formatDocument" }
]"#;

fn keys_of(out: &str) -> Vec<String> {
    let v: serde_json::Value = serde_json::from_str(out).unwrap();
    v.as_array()
        .unwrap()
        .iter()
        .map(|e| e["key"].as_str().unwrap().to_string())
        .collect()
}

#[test]
fn vs_code_shortcuts_swap_modifiers_both_ways_and_never_merge_two_into_one() {
    let cat = Catalogue::starter().unwrap();
    let env = Env::new();
    let f = file("Code/User/keybindings.json");
    let run = |from, to| moved(plan_file(&cat, &f, KEYS.as_bytes(), &env.ctx(from, to)));
    // A Mac's Command is Ctrl elsewhere; its Control is the Windows key (`win`) on Windows and
    // `meta` on Linux, so cmd+s and ctrl+tab stay two different shortcuts.
    let (out, eq) = run(System::Mac, System::Windows);
    assert!(eq);
    assert_eq!(
        keys_of(&out),
        ["ctrl+s", "win+tab", "ctrl+k ctrl+c", "shift+alt+f"]
    );
    let (out, _) = run(System::Mac, System::Gnome);
    assert_eq!(
        keys_of(&out),
        ["ctrl+s", "meta+tab", "ctrl+k ctrl+c", "shift+alt+f"]
    );
    // And back: Windows' to a Mac's is the same table read the other way.
    let win = moved(plan_file(
        &cat,
        &f,
        out.replace("meta", "win").as_bytes(),
        &env.ctx(System::Windows, System::Mac),
    ))
    .0;
    assert_eq!(
        keys_of(&win),
        ["cmd+s", "ctrl+tab", "cmd+k cmd+c", "shift+alt+f"]
    );
    // Windows to Linux: only the system key's name changes.
    let (out, eq) = moved(plan_file(
        &cat,
        &f,
        br#"[{"key": "win+e", "command": "a"}, {"key": "ctrl+e", "command": "b"}]"#,
        &env.ctx(System::Windows, System::Gnome),
    ));
    assert!(eq);
    assert_eq!(keys_of(&out), ["meta+e", "ctrl+e"]);
    // Between two Linux desktops nothing is translated, and nothing is an equivalent.
    let (out, eq) = run(System::Gnome, System::Kde);
    assert!(!eq);
    assert_eq!(keys_of(&out)[0], "cmd+s");
}

#[test]
fn a_shortcut_with_nothing_to_translate_is_not_called_an_equivalent() {
    let cat = Catalogue::starter().unwrap();
    let env = Env::new();
    let (out, eq) = moved(plan_file(
        &cat,
        &file("Code/User/keybindings.json"),
        br#"[{"key": "shift+alt+f", "command": "x"}, {"key": "f5", "command": "y"}]"#,
        &env.ctx(System::Windows, System::Mac),
    ));
    assert!(!eq);
    assert_eq!(keys_of(&out), ["shift+alt+f", "f5"]);
}

const KEYMAP: &str = r#"<keymap version="1" name="Ada's keys" parent="$default">
  <action id="GotoAction">
    <keyboard-shortcut first-keystroke="ctrl shift A" />
  </action>
  <action id="CommentByLineComment">
    <keyboard-shortcut first-keystroke="ctrl K" second-keystroke="ctrl C" />
  </action>
  <action id="GotoDeclaration">
    <mouse-shortcut keystroke="ctrl button1" />
  </action>
  <action id="ShowDesktop">
    <keyboard-shortcut first-keystroke="meta D" />
  </action>
</keymap>
"#;

#[test]
fn a_jetbrains_keymap_moves_to_a_mac_with_its_parent_and_keys_translated() {
    let cat = Catalogue::starter().unwrap();
    let mut env = Env::new();
    // Any JetBrains product: one row family, the product and version in the folder's name.
    env.renames
        .insert("PyCharm2025.2".into(), "PyCharm2025.2".into());
    let f = file("JetBrains/PyCharm2025.2/keymaps/Ada's keys.xml");
    let (out, eq) = moved(plan_file(
        &cat,
        &f,
        KEYMAP.as_bytes(),
        &env.ctx(System::Windows, System::Mac),
    ));
    assert!(eq);
    assert!(out.contains(r#"parent="Mac OS X 10.5+""#), "{out}");
    assert!(out.contains(r#"first-keystroke="meta shift A""#), "{out}");
    assert!(
        out.contains(r#"first-keystroke="meta K" second-keystroke="meta C""#),
        "{out}"
    );
    assert!(out.contains(r#"keystroke="meta button1""#), "{out}");
    // The Windows key's shortcut becomes Control on the Mac, never a second Command one.
    assert!(out.contains(r#"first-keystroke="ctrl D""#), "{out}");
    // The keymap's own name is left as it is.
    assert!(out.contains(r#"name="Ada's keys""#), "{out}");
    assert!(roxmltree::Document::parse(&out).is_ok());

    // From the Mac to GNOME, the same file comes back.
    let (back, _) = moved(plan_file(
        &cat,
        &file("JetBrains/IntelliJIdea2025.2/keymaps/Ada's keys.xml"),
        out.as_bytes(),
        &env.ctx(System::Mac, System::Gnome),
    ));
    assert_eq!(back, KEYMAP);

    // The same system: the keymap moves as it is (its parent `$default` is a keymap's name,
    // never judged as a place written with a variable).
    let (same, eq) = moved(plan_file(
        &cat,
        &f,
        KEYMAP.as_bytes(),
        &env.ctx(System::Windows, System::Windows),
    ));
    assert!(!eq);
    assert_eq!(same, KEYMAP);
}

#[test]
fn the_chosen_jetbrains_keymap_follows_the_system() {
    let cat = Catalogue::starter().unwrap();
    let env = Env::new();
    let xml = r#"<application>
  <component name="KeymapManager">
    <active_keymap name="Default for GNOME" />
  </component>
</application>
"#;
    let f = file("JetBrains/WebStorm2025.1/options/keymap.xml");
    let (out, eq) = moved(plan_file(
        &cat,
        &f,
        xml.as_bytes(),
        &env.ctx(System::Gnome, System::Mac),
    ));
    assert!(eq);
    assert!(
        out.contains(r#"<active_keymap name="Mac OS X 10.5+" />"#),
        "{out}"
    );
    // A keymap the person made keeps its name: it moves with them.
    let mine = xml.replace("Default for GNOME", "Ada's keys");
    let (out, eq) = moved(plan_file(
        &cat,
        &f,
        mine.as_bytes(),
        &env.ctx(System::Gnome, System::Windows),
    ));
    assert!(!eq);
    assert_eq!(out, mine);
    // The same system: untouched.
    let (out, _) = moved(plan_file(
        &cat,
        &f,
        xml.as_bytes(),
        &env.ctx(System::Gnome, System::Gnome),
    ));
    assert_eq!(out, xml);
}

#[test]
fn a_translation_naming_a_missing_or_partial_table_is_refused() {
    let mut v: serde_json::Value =
        serde_json::from_str(pctwin_settings::catalogue::STARTER).unwrap();
    v["tables"]["vscode-keys"]["columns"] = serde_json::json!(["windows", "mac", "other"]);
    let e = Catalogue::from_json(v.to_string().as_bytes()).unwrap_err();
    assert!(e.to_string().contains("vscode"), "{e}");
    let mut v: serde_json::Value =
        serde_json::from_str(pctwin_settings::catalogue::STARTER).unwrap();
    v["tables"]
        .as_object_mut()
        .unwrap()
        .remove("jetbrains-keys");
    assert!(Catalogue::from_json(v.to_string().as_bytes()).is_err());
    // The table function itself: nothing changes on the same system, and unknown names stay.
    let cat = Catalogue::starter().unwrap();
    let t = &cat.tables["vscode-keys"];
    assert_eq!(
        translate(t, "ctrl+x", System::Windows, System::Windows, true),
        "ctrl+x"
    );
    assert_eq!(
        translate(t, "Ctrl+Hyper+x", System::Windows, System::Mac, true),
        "cmd+Hyper+x"
    );
}

const EXTENSIONS: &str = r#"[
  {"identifier": {"id": "ms-python.python", "uuid": "f1f59ae4"}, "version": "2025.1.0",
   "metadata": {"source": "gallery", "isMachineScoped": false}},
  {"identifier": {"id": "esbenp.prettier-vscode"}, "version": "11.0.0"},
  {"identifier": {"id": "MS-Python.Python"}, "version": "2025.1.0", "metadata": {"source": "gallery"}},
  {"identifier": {"id": "ada.local-tools"}, "version": "0.1.0", "metadata": {"source": "vsix"}},
  {"identifier": {"id": "https://evil.example/x.vsix"}, "metadata": {"source": "gallery"}},
  {"identifier": {"id": "..\\..\\x"}},
  {"version": "1.0.0"}
]"#;

#[test]
fn vs_code_extensions_are_asked_for_by_name_from_the_store_or_left_to_the_person() {
    let cat = Catalogue::starter().unwrap();
    let list = cat.app("vscode").unwrap().extensions.as_ref().unwrap();
    // One place on every system: the home folder's `.vscode/extensions`.
    for s in [System::Windows, System::Mac, System::Gnome] {
        let p = list.place(s).unwrap();
        assert_eq!(
            (p.root, p.path.as_str()),
            (SettingsRoot::Home, ".vscode/extensions/extensions.json")
        );
    }
    let found = read_json(list, EXTENSIONS.as_bytes()).unwrap();
    let names: Vec<_> = found.iter().map(|f| f.name.as_str()).collect();
    assert_eq!(
        names,
        [
            "ms-python.python",
            "esbenp.prettier-vscode",
            "ada.local-tools",
            "https://evil.example/x.vsix",
            "..\\..\\x"
        ]
    );
    let planned = extensions::plan(list, &found);
    assert_eq!(
        planned[..2],
        [
            Planned::Reinstall {
                id: "ms-python.python".into(),
                store: "vscode-marketplace".into()
            },
            Planned::Reinstall {
                id: "esbenp.prettier-vscode".into(),
                store: "vscode-marketplace".into()
            },
        ]
    );
    let why: Vec<_> = planned[2..]
        .iter()
        .map(|p| match p {
            Planned::NeedsAction { why, .. } => *why,
            other => panic!("{other:?}"),
        })
        .collect();
    assert_eq!(why, [FROM_A_FILE, NOT_A_NAME, NOT_A_NAME]);

    // Only what the new laptop's own list names afterwards is reinstalled.
    let after: BTreeSet<String> = ["MS-PYTHON.PYTHON".to_string()].into();
    let results = extensions::results(&planned, &after);
    let statuses: Vec<_> = results.iter().map(|r| (r.status, r.why)).collect();
    assert_eq!(
        statuses,
        [
            (ItemStatus::Reinstalled, None),
            (ItemStatus::NeedsAction, Some(NOT_THERE)),
            (ItemStatus::NeedsAction, Some(FROM_A_FILE)),
            (ItemStatus::NeedsAction, Some(NOT_A_NAME)),
            (ItemStatus::NeedsAction, Some(NOT_A_NAME)),
        ]
    );
    // Only a plain `publisher.name` is ever handed to a store.
    for good in ["ms-python.python", "GitHub.copilot-chat", "a_b.c-1"] {
        assert!(extensions::id_ok(good), "{good}");
    }
    for bad in [
        "",
        ".hidden",
        "a..b",
        "a.",
        "-flag",
        "a b",
        "a/b",
        "a\\b",
        "a:b",
        "https://x.y",
    ] {
        assert!(!extensions::id_ok(bad), "{bad}");
    }
    // A file that is not a list is not read as one.
    assert!(read_json(list, b"{\"a\": 1}").is_none());
    assert!(read_json(list, b"not json").is_none());
}

#[test]
fn jetbrains_plugins_are_listed_by_folder_and_each_is_the_persons_step() {
    let cat = Catalogue::starter().unwrap();
    let list = cat.app("jetbrains").unwrap().extensions.as_ref().unwrap();
    assert_eq!(list.form, ListForm::FolderNames);
    assert_eq!(list.store, None);
    // Linux keeps plugins in the local data folder; Windows and a Mac beside the settings.
    assert_eq!(
        list.place(System::Gnome).unwrap().root,
        SettingsRoot::LocalData
    );
    assert_eq!(list.place(System::Mac).unwrap().root, SettingsRoot::Config);
    assert_eq!(
        list.place(System::Windows).unwrap().path,
        "JetBrains/*/plugins"
    );
    let found = read_folders(&[
        "Rider-Unity".into(),
        "rider-unity".into(),
        ".DS_Store".into(),
        "IdeaVim".into(),
    ]);
    let planned = extensions::plan(list, &found);
    assert_eq!(
        planned,
        [
            Planned::NeedsAction {
                name: "Rider-Unity".into(),
                why: BY_FOLDER
            },
            Planned::NeedsAction {
                name: "IdeaVim".into(),
                why: BY_FOLDER
            },
        ]
    );
    // Even if the new laptop has one, it was not PCTwin's doing: still the person's step.
    let r = extensions::results(&planned, &["ideavim".to_string()].into());
    assert!(r.iter().all(|r| r.status == ItemStatus::NeedsAction));
}

#[test]
fn a_new_editor_is_one_catalogue_row_with_no_code() {
    let mut v: serde_json::Value =
        serde_json::from_str(pctwin_settings::catalogue::STARTER).unwrap();
    v["apps"].as_array_mut().unwrap().push(serde_json::json!({
        "app": "vscodium",
        "min_version": "1.80",
        "files": [{
            "root": "config", "path": "VSCodium/User/keybindings.json", "format": "jsonc",
            "writer": "settings", "default": "copy",
            "cross_os": [{"at": "*/key", "rule": {"translate": {"table": "vscode-keys", "keys": true}}}]
        }],
        "extensions": {
            "places": [{"root": "home", "path": ".vscode-oss/extensions/extensions.json"}],
            "form": {"json": {"id": "identifier/id", "source": "metadata/source", "store_sources": ["gallery"]}},
            "store": "open-vsx"
        }
    }));
    let cat = Catalogue::from_json(v.to_string().as_bytes()).unwrap();
    let env = Env::new();
    let (out, eq) = moved(plan_file(
        &cat,
        &file("VSCodium/User/keybindings.json"),
        KEYS.as_bytes(),
        &env.ctx(System::Mac, System::Windows),
    ));
    assert!(eq);
    assert_eq!(keys_of(&out)[0], "ctrl+s");
    let list = cat.app("vscodium").unwrap().extensions.as_ref().unwrap();
    let planned = extensions::plan(list, &read_json(list, EXTENSIONS.as_bytes()).unwrap());
    assert_eq!(
        planned[0],
        Planned::Reinstall {
            id: "ms-python.python".into(),
            store: "open-vsx".into()
        }
    );
    // A store name that is not plain is refused.
    v["apps"][2]["extensions"]["store"] = serde_json::json!("https://x");
    assert!(Catalogue::from_json(v.to_string().as_bytes()).is_err());
}
