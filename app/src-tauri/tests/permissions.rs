//! The screens' reach, proven on what Tauri really resolves (Security Design, "The web view is
//! untrusted"). The app is built the way it ships: its own builder function (`app`, the one
//! `run()` uses, setup included) and its own generated context (`context`: tauri.conf.json merged
//! with this system's tauri.<system>.conf.json, and the permissions the build resolved from
//! build.rs, capabilities/ and permissions/). It runs on Tauri's mock runtime.
//!
//! Then every command Tauri knows (every command of every manifest the build wrote: core, plugins
//! and the app's own) is called from the main window with a local origin, from the main window
//! with remote origins, and from another window. Everything is refused except exactly
//! [`SCREENS_MAY_CALL`] from the main window's own pages. A command granted anywhere (a capability
//! file, an inline capability in a platform overlay, a permission set in permissions/, a plugin's
//! default set, or a grant made while the app runs) fails this. CI runs it on Windows, macOS and
//! Linux, so each system's overlay is included, in debug and in release.
//!
//! Nothing here pairs, touches the network or opens anything: a refused command never runs, and a
//! granted one is sent arguments of the wrong type, so it stops before doing anything. Two granted
//! commands are checked differently: `set_window_mode` really runs (it resizes the mock window),
//! and `pairing_new_search` (which would search the network) and `quicksend_choose_folder` (which
//! would open the system's folder picker), taking no arguments, are checked on
//! the resolved permissions only. The move's commands that take no arguments (pause, resume, stop,
//! report) do run, and find no move to act on.

use std::collections::BTreeSet;

use serde_json::{Value, json};
use tauri::Manager;
use tauri::ipc::{CallbackFn, InvokeBody, Origin};
use tauri::test::{INVOKE_KEY, MockRuntime, get_ipc_response, mock_builder};
use tauri::webview::{InvokeRequest, WebviewWindowBuilder};
use tauri::{App, WebviewUrl, WebviewWindow};

/// Exactly what the screens may call, 46 names: the app's forty-three commands, listening to and
/// unlistening from its own events (two), and opening our own site in the browser (one). Changing
/// this list is a security decision.
const SCREENS_MAY_CALL: [&str; 84] = [
    "set_window_mode",
    "window_drag",
    "set_tray_labels",
    "pairing_old_start",
    "pairing_old_pick",
    "pairing_old_decline",
    "pairing_old_shuffle",
    "pairing_old_rename",
    "pairing_old_start_again",
    "pairing_old_nearby",
    "pairing_heads_up",
    "pairing_stop",
    "pairing_new_search",
    "pairing_new_connect",
    "move_begin",
    "rescue_drives",
    "rescue_begin",
    "move_approve",
    "move_place_people",
    "move_choose_place",
    "move_usual_place",
    "move_privacy_answer",
    "open_privacy_settings",
    "move_pause",
    "move_resume",
    "move_stop",
    "move_retry",
    "move_report",
    "move_people_here",
    "help_code",
    "recovery_check",
    "recovery_run",
    "things_ready",
    "history_list",
    "history_open",
    "history_export",
    "quicksend_pick",
    "quicksend_accept_drops",
    "quicksend_clear",
    "quicksend_offer",
    "quicksend_offer_missing",
    "quicksend_send",
    "quicksend_receive",
    "quicksend_folder",
    "quicksend_choose_folder",
    "quicksend_save_here",
    "quicksend_open_folder",
    "quicksend_keep_original_dates",
    "quicksend_pause",
    "quicksend_resume",
    "quicksend_stop",
    "quicksend_send_note",
    "quicksend_open_link",
    "quicksend_launch_files",
    "known_list",
    "known_remember",
    "known_forget",
    "known_rename",
    "known_allow_folder",
    "known_revoke_folder",
    "known_renew_folder",
    "known_next_folder",
    "known_clear_next_folder",
    "known_save_incoming",
    "known_decline_incoming",
    "known_send",
    "waiting_prepare",
    "waiting_list",
    "waiting_renew",
    "waiting_cancel",
    "update_check",
    "update_install",
    "laptop_name",
    "laptop_name_shuffle",
    "laptop_name_use",
    "laptop_name_type",
    "guard_view",
    "guard_intent",
    "fullmove_undo",
    "move_appdata_answer",
    "move_choose_what",
    "plugin:event|listen",
    "plugin:event|unlisten",
    "plugin:opener|open_url",
];

/// The app's own commands among them (build.rs `COMMANDS`).
fn app_commands() -> Vec<&'static str> {
    SCREENS_MAY_CALL
        .iter()
        .copied()
        .filter(|c| !c.starts_with("plugin:"))
        .collect()
}

/// The pages of the app's own screens, as Tauri serves them on this system.
const LOCAL: &str = if cfg!(any(windows, target_os = "android")) {
    "http://tauri.localhost"
} else {
    "tauri://localhost"
};

/// Pages that are not the app's own: anyone's, and our own website (which may link to the app, but
/// is still a remote page and must never reach it).
const REMOTE: [&str; 2] = ["https://evil.example/", "https://getpctwin.com/"];

/// A file the build writes into this crate's OUT_DIR, which `generate_context!` resolves from.
fn generated(name: &str) -> Value {
    let path = std::path::Path::new(env!("OUT_DIR")).join(name);
    let text = std::fs::read_to_string(&path)
        .unwrap_or_else(|e| panic!("cannot read {}: {e}", path.display()));
    serde_json::from_str(&text).unwrap_or_else(|e| panic!("{name} is not JSON: {e}"))
}

/// The name the screens invoke a manifest's command by (as tauri-utils resolves it).
fn invoke_name(manifest: &str, command: &str) -> String {
    if manifest == "__app-acl__" {
        command.to_owned()
    } else if let Some(core) = manifest.strip_prefix("core:") {
        format!("plugin:{core}|{command}")
    } else {
        format!("plugin:{manifest}|{command}")
    }
}

/// Every command named by any permission of any manifest the build knows: Tauri's core, each
/// plugin the app has, and the app itself. A command no permission names cannot be granted.
fn every_known_command() -> BTreeSet<String> {
    let manifests = generated("acl-manifests.json");
    let manifests = manifests.as_object().expect("a map of manifests");
    let mut all = BTreeSet::new();
    for (key, manifest) in manifests {
        let permissions = manifest["permissions"].as_object().expect("permissions");
        for permission in permissions.values() {
            for list in ["allow", "deny"] {
                for c in permission["commands"][list]
                    .as_array()
                    .expect("a command list")
                {
                    all.insert(invoke_name(key, c.as_str().expect("a command name")));
                }
            }
        }
    }
    // The sweep is only as good as its list: it must include the core, both plugins and the app.
    for needed in [
        "plugin:window|create",
        "plugin:webview|create_webview_window",
        "plugin:app|app_show",
        "plugin:path|resolve_directory",
        "plugin:opener|open_path",
        "plugin:opener|reveal_item_in_dir",
        "plugin:window-state|save_window_state",
        "plugin:event|emit",
        "pairing_stop",
    ] {
        assert!(all.contains(needed), "the manifests lack {needed}");
    }
    assert!(all.len() > 150, "too few commands found: {}", all.len());
    // Names no manifest has are refused too.
    for unknown in [
        "not_a_command",
        "plugin:fs|read_file",
        "plugin:shell|execute",
        "plugin:navigation-guard|anything",
    ] {
        all.insert(unknown.to_owned());
    }
    all
}

/// The app as it ships, on the mock runtime, started: building does not run the app's setup;
/// Tauri runs it (after making the config's windows) on the event loop's first turn, as the real
/// app does when it starts. One turn of the mock loop does exactly that and nothing else.
fn app() -> App<MockRuntime> {
    let mut app = pctwin_app_lib::app(mock_builder())
        .build(pctwin_app_lib::context())
        .expect("the app builds on the mock runtime");
    #[allow(deprecated)]
    app.run_iteration(|_, _| {});
    assert!(
        app.get_webview_window("main").is_some(),
        "the start made the main window from the config"
    );
    app
}

fn window(app: &App<MockRuntime>, label: &str) -> WebviewWindow<MockRuntime> {
    app.get_webview_window(label).unwrap_or_else(|| {
        WebviewWindowBuilder::new(app, label, WebviewUrl::default())
            .build()
            .unwrap()
    })
}

/// What a page at `url` in window `w` gets back for `cmd` with `args`: Ok, or the error as text.
fn call(w: &WebviewWindow<MockRuntime>, url: &str, cmd: &str, args: Value) -> Result<(), String> {
    get_ipc_response(
        w,
        InvokeRequest {
            cmd: cmd.into(),
            callback: CallbackFn(0),
            error: CallbackFn(1),
            url: url.parse().unwrap(),
            body: InvokeBody::Json(args),
            headers: Default::default(),
            invoke_key: INVOKE_KEY.to_string(),
        },
    )
    .map(|_| ())
    .map_err(|e| match e {
        Value::String(s) => s,
        other => other.to_string(),
    })
}

/// Whether `result` is Tauri's permission check refusing `cmd`, and nothing else: release builds
/// say "Command <cmd> not allowed by ACL"; debug builds start with the command's name and "not
/// allowed" or "explicitly denied". A command's own error never starts that way.
fn refused(cmd: &str, result: &Result<(), String>) -> bool {
    let Err(error) = result else {
        return false;
    };
    let pretty = match cmd.strip_prefix("plugin:") {
        Some(rest) => rest.replacen('|', ".", 1),
        None => cmd.to_owned(),
    };
    *error == format!("Command {cmd} not allowed by ACL")
        || error.starts_with(&format!("{pretty} not allowed"))
        || error.starts_with(&format!("{pretty} explicitly denied"))
}

/// Arguments of the wrong type for a granted command, so it stops before it does anything.
fn wrong_arguments(cmd: &str) -> Value {
    match cmd {
        "set_tray_labels" => json!({ "show": 1, "quit": 2 }),
        "pairing_old_start" => json!({ "network": 5 }),
        "pairing_old_pick" => json!({ "round": "x", "number": "y" }),
        "pairing_old_decline" => json!({ "round": "x" }),
        "pairing_old_rename" => json!({ "name": 7 }),
        "pairing_new_connect" => json!({ "index": "x", "code": 1 }),
        "move_begin" => json!({ "fallback": 1, "others": 2 }),
        "rescue_begin" => json!({ "id": 1, "fallback": 2 }),
        "move_approve" => json!({ "plan": 1, "approve": "x", "leftOut": 3, "leaveBehind": "x" }),
        "move_place_people" => json!({ "choices": 1, "words": 2 }),
        "move_choose_place" | "move_usual_place" => json!({ "role": 1 }),
        "move_privacy_answer" => json!({ "again": "x" }),
        "help_code" => json!({ "topic": 1, "step": 2 }),
        "quicksend_pick" => json!({ "folders": "x" }),
        "quicksend_accept_drops" => json!({ "on": "x" }),
        "quicksend_offer" => json!({ "ids": 1 }),
        "history_open" => json!({ "id": 1 }),
        "plugin:event|listen" | "plugin:event|unlisten" => json!({ "event": 1 }),
        _ => json!({}),
    }
}

#[test]
fn from_the_main_windows_own_pages_only_the_closed_list_is_let_through() {
    let app = app();
    let main = window(&app, "main");
    let mut let_through = BTreeSet::new();
    for cmd in every_known_command() {
        if matches!(
            cmd.as_str(),
            "pairing_new_search"
                | "set_window_mode"
                | "window_drag"
                | "quicksend_choose_folder"
                | "history_export"
                | "update_check"
                | "update_install"
        ) {
            // Checked below, and in the resolved-permissions test (the folder picker would open
            // the system's window and wait for a person). A command with no arguments runs
            // whatever it is given, so the updater's two are never called: they would ask the
            // real update address and install what it offers (on macOS over the build folder,
            // three folders above the test program; on Windows by running the installer).
            let_through.insert(cmd);
            continue;
        }
        let result = call(&main, LOCAL, &cmd, wrong_arguments(&cmd));
        if !refused(&cmd, &result) {
            let_through.insert(cmd);
        }
    }
    let wanted: BTreeSet<String> = SCREENS_MAY_CALL.iter().map(|c| (*c).to_owned()).collect();
    let extra: Vec<_> = let_through.difference(&wanted).collect();
    let missing: Vec<_> = wanted.difference(&let_through).collect();
    assert!(
        extra.is_empty(),
        "the screens can reach more than the closed list: {extra:?}"
    );
    assert!(
        missing.is_empty(),
        "the screens are refused what they need: {missing:?}"
    );
    // A granted command really runs end to end: asking for a size by name.
    assert_eq!(
        call(&main, LOCAL, "set_window_mode", json!({ "mode": "mini" })),
        Ok(())
    );
}

#[test]
fn remote_pages_and_other_windows_are_refused_every_command() {
    let app = app();
    let main = window(&app, "main");
    let other = window(&app, "other");
    let mut reached = Vec::new();
    for cmd in every_known_command() {
        for (who, w, url) in [
            ("main", &main, REMOTE[0]),
            ("main", &main, REMOTE[1]),
            ("other", &other, LOCAL),
        ] {
            let result = call(w, url, &cmd, json!({}));
            if !refused(&cmd, &result) {
                reached.push(format!("{who} at {url}: {cmd} => {result:?}"));
            }
        }
    }
    assert!(
        reached.is_empty(),
        "reached from outside the main window's own pages: {reached:#?}"
    );
}

#[test]
fn the_browser_opens_our_own_site_only() {
    let app = app();
    let main = window(&app, "main");
    for url in [
        "https://example.com/",
        "https://getpctwin.com.evil.example/",
        "https://evil.example/?https://getpctwin.com/",
        "http://getpctwin.com/",
        "file:///C:/Windows/System32/cmd.exe",
        "mailto:someone@example.com",
    ] {
        let result = call(
            &main,
            LOCAL,
            "plugin:opener|open_url",
            json!({ "url": url }),
        );
        assert_eq!(
            result,
            Err(format!("Not allowed to open url {url}")),
            "{url} must be refused by the opener's scope"
        );
    }
}

/// The same answer straight from the permissions Tauri resolved at build time, with no command
/// run and no error text read: for every command, window and origin.
#[test]
fn the_resolved_permissions_grant_exactly_the_closed_list() {
    let mut context = pctwin_app_lib::context::<MockRuntime>();
    let authority = context.runtime_authority_mut();
    let local = Origin::Local;
    let remote: Vec<Origin> = REMOTE
        .iter()
        .map(|u| Origin::Remote {
            url: u.parse().unwrap(),
        })
        .collect();
    let mut granted = BTreeSet::new();
    for cmd in every_known_command() {
        if authority
            .resolve_access(&cmd, "main", "main", &local)
            .is_some()
        {
            granted.insert(cmd.clone());
        }
        for label in ["other", "main2", "Main", "*"] {
            assert!(
                authority
                    .resolve_access(&cmd, label, label, &local)
                    .is_none(),
                "{cmd} is granted to window {label}"
            );
        }
        for origin in &remote {
            assert!(
                authority
                    .resolve_access(&cmd, "main", "main", origin)
                    .is_none(),
                "{cmd} is granted to a remote page"
            );
        }
    }
    let wanted: BTreeSet<String> = SCREENS_MAY_CALL.iter().map(|c| (*c).to_owned()).collect();
    assert_eq!(granted, wanted);
}

/// The app manifest the build made from build.rs's `COMMANDS`: one allow and one deny permission
/// per command, and nothing else (no permission sets, no default, no scope), so no file in
/// permissions/ can widen a name like `allow-pairing-stop`.
#[test]
fn the_apps_own_permissions_are_one_per_command() {
    let manifests = generated("acl-manifests.json");
    let app = &manifests["__app-acl__"];
    let mut want = serde_json::Map::new();
    for cmd in app_commands() {
        let dashed = cmd.replace('_', "-");
        want.insert(
            format!("allow-{dashed}"),
            json!({ "allow": [cmd], "deny": [] }),
        );
        want.insert(
            format!("deny-{dashed}"),
            json!({ "allow": [], "deny": [cmd] }),
        );
    }
    let permissions = app["permissions"].as_object().expect("permissions");
    let have: serde_json::Map<String, Value> = permissions
        .iter()
        .map(|(id, p)| {
            let keys: BTreeSet<&str> = p.as_object().unwrap().keys().map(String::as_str).collect();
            assert_eq!(
                keys,
                BTreeSet::from(["identifier", "description", "commands"]),
                "{id} has more than commands (a scope or platforms)"
            );
            assert_eq!(p["identifier"], json!(id));
            (id.clone(), p["commands"].clone())
        })
        .collect();
    assert_eq!(Value::Object(have), Value::Object(want));
    assert_eq!(app["default_permission"], Value::Null);
    assert_eq!(app["permission_sets"], json!({}));
    assert_eq!(app["global_scope_schema"], Value::Null);
}

/// The capability files the build resolved: one, for the main window's own pages, granting the
/// closed list. (Inline capabilities in the config are refused by the merged-config test.)
#[test]
fn the_resolved_capabilities_are_the_one_default() {
    let mut caps = generated("capabilities.json");
    let default = caps
        .as_object_mut()
        .expect("a map of capabilities")
        .get_mut("default")
        .expect("the default capability");
    default
        .as_object_mut()
        .unwrap()
        .remove("description")
        .expect("a description");
    let mut permissions: Vec<Value> = app_commands()
        .iter()
        .map(|c| json!(format!("allow-{}", c.replace('_', "-"))))
        .collect();
    permissions.extend([
        json!("core:event:allow-listen"),
        json!("core:event:allow-unlisten"),
        json!({ "identifier": "opener:allow-open-url", "allow": [{ "url": "https://getpctwin.com/*" }] }),
    ]);
    assert_eq!(
        caps,
        json!({
            "default": {
                "identifier": "default",
                "local": true,
                "windows": ["main"],
                "permissions": permissions,
            }
        })
    );
}

/// The content security policy as decided; any change is a decision, made here on purpose.
const CSP: &str = "default-src 'self'; script-src 'self'; style-src 'self'; img-src 'self' data:; font-src 'self'; connect-src ipc: http://ipc.localhost; object-src 'none'; base-uri 'none'; form-action 'none'";

/// The config the app really runs with (tauri.conf.json merged with this system's overlay, and
/// anything the build was given in TAURI_CONFIG), not the text of one file.
#[test]
fn the_merged_config_keeps_the_lockdown() {
    let context = pctwin_app_lib::context::<MockRuntime>();
    let config = serde_json::to_value(context.config()).expect("the config serialises");
    let app = &config["app"];

    assert_eq!(
        app["withGlobalTauri"],
        json!(false),
        "the screens reach Tauri through bridge.ts only, never a global"
    );
    assert_eq!(
        app["security"],
        json!({
            "csp": CSP,
            "freezePrototype": true,
            "dangerousDisableAssetCspModification": false,
            "assetProtocol": { "scope": [], "enable": false },
            "pattern": { "use": "brownfield" },
            "capabilities": ["default"],
        }),
        "app.security changed (CSP, capabilities, asset protocol, isolation or headers)"
    );

    // Settings that are left out are not there at all (the config skips empty ones), so a new
    // key anywhere here (a tray, a proxy, browser arguments, a parent window, a data folder) is a
    // change to decide on, not something to miss.
    let keys = |v: &Value| -> BTreeSet<String> { v.as_object().unwrap().keys().cloned().collect() };
    let names =
        |list: &[&str]| -> BTreeSet<String> { list.iter().map(|k| (*k).to_owned()).collect() };
    assert_eq!(
        keys(app),
        names(&[
            "enableGTKAppId",
            "macOSPrivateApi",
            "security",
            "windows",
            "withGlobalTauri"
        ]),
        "app settings changed"
    );
    assert_eq!(app["macOSPrivateApi"], json!(false));

    let windows = app["windows"].as_array().expect("windows");
    assert_eq!(windows.len(), 1, "one window only");
    let main = &windows[0];
    assert_eq!(main["label"], json!("main"));
    assert_eq!(
        main["url"],
        json!("index.html"),
        "the window shows the app's own screens only"
    );
    for (key, value) in [
        ("create", json!(true)),
        ("useHttpsScheme", json!(false)),
        ("browserExtensionsEnabled", json!(false)),
        ("javascriptDisabled", json!(false)),
    ] {
        assert_eq!(main[key], value, "windows[0].{key} changed");
    }
    assert_eq!(
        keys(main),
        names(&[
            "acceptFirstMouse",
            "allowLinkPreview",
            "alwaysOnBottom",
            "alwaysOnTop",
            "browserExtensionsEnabled",
            "center",
            "closable",
            "contentProtected",
            "create",
            "decorations",
            "disableInputAccessoryView",
            "dragDropEnabled",
            "focus",
            "focusable",
            "fullscreen",
            "generalAutofillEnabled",
            "height",
            "hiddenTitle",
            "incognito",
            "javascriptDisabled",
            "label",
            "limitNavigationsToAppBoundDomains",
            "maximizable",
            "maximized",
            "minHeight",
            "minWidth",
            "minimizable",
            "noRedirectionBitmap",
            "resizable",
            "scrollBarStyle",
            "shadow",
            "skipTaskbar",
            "title",
            "titleBarStyle",
            "transparent",
            "url",
            "useHttpsScheme",
            "visible",
            "visibleOnAllWorkspaces",
            "width",
            "zoomHotkeysEnabled",
        ]),
        "a window setting was added (a proxy, browser arguments, a parent, a data folder...)"
    );

    // The screens come from the app's own files, or the local dev server while developing.
    assert_eq!(config["build"]["frontendDist"], json!("../dist"));
    assert_eq!(config["build"]["devUrl"], json!("http://localhost:1420/"));
    // The only plugin settings are the updater's (src/updates.rs): its key, one HTTPS address,
    // signatures bound to their version, and nothing that would loosen the check (no plain-HTTP
    // address allowed, no other plugin configured).
    let plugins = config["plugins"].as_object().unwrap();
    assert_eq!(
        plugins.keys().map(String::as_str).collect::<Vec<_>>(),
        ["updater"],
        "only the updater has settings"
    );
    let updater = plugins["updater"].as_object().unwrap();
    assert_eq!(
        updater.keys().map(String::as_str).collect::<BTreeSet<_>>(),
        BTreeSet::from(["endpoints", "pubkey", "requireSignedVersion", "windows"]),
        "an updater setting was added (an insecure address, headers...)"
    );
    assert!(updater["pubkey"].as_str().is_some_and(|k| !k.is_empty()));
    assert_eq!(updater["requireSignedVersion"], json!(true));
    assert_eq!(updater["windows"], json!({ "installMode": "passive" }));
    let endpoints = updater["endpoints"].as_array().unwrap();
    assert!(!endpoints.is_empty());
    assert!(
        endpoints
            .iter()
            .all(|e| e.as_str().is_some_and(|u| u.starts_with("https://"))),
        "updates come over HTTPS only"
    );
}

/// The argument names the screens send (app/src/contract/shell/calls.json, written by the engine's
/// contract test) are each command's own, in its own order: given every earlier argument with a
/// value of the right type, a value of the wrong type under the name is refused as that argument
/// (a renamed or reordered parameter is refused as a missing one instead). Nothing runs: each call
/// stops at the wrong type. Then, with every argument right, the call must get past its arguments,
/// so the command takes nothing the screens do not send; `pairing_old_start`, `pairing_new_search`
/// and `set_tray_labels` are left out of that step because they would pair, search the network or
/// build a native menu off the main thread.
#[test]
fn the_screens_argument_names_are_the_commands_own() {
    let path =
        std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../src/contract/shell/calls.json");
    let calls: Value = serde_json::from_str(&std::fs::read_to_string(path).unwrap()).unwrap();
    let commands = calls["commands"].as_array().unwrap();
    let names: BTreeSet<&str> = commands
        .iter()
        .map(|c| c["name"].as_str().unwrap())
        .collect();
    let wanted: BTreeSet<&str> = app_commands().into_iter().collect();
    assert_eq!(names, wanted, "calls.json lists exactly the app's commands");
    let app = app();
    let main = window(&app, "main");
    let args_error = |cmd: &str, error: &str| error.contains(&format!("for command `{cmd}`"));
    for command in commands {
        let cmd = command["name"].as_str().unwrap();
        let args = command["args"].as_array().unwrap();
        let mut given = serde_json::Map::new();
        for arg in args {
            let key = arg[0].as_str().unwrap();
            let mut body = given.clone();
            body.insert(key.to_owned(), json!({ "not": "a value" }));
            let error = call(&main, LOCAL, cmd, Value::Object(body))
                .expect_err("a value of the wrong type is refused");
            assert!(
                error.contains(&format!("invalid args `{key}` for command `{cmd}`"))
                    && !error.contains("missing required key"),
                "{cmd} has no argument `{key}` where the screens send it: {error}"
            );
            given.insert(key.to_owned(), arg[1].clone());
        }
        // These would pair, search the network, build a native menu (which macOS allows only
        // on the main thread), open the system's file picker and wait for a person, or ask the
        // real update address and install what it offers: on macOS the updater replaces the
        // program's bundle, which for a test program is three folders up (the build folder);
        // on Windows it would run the installer. Their names are proven above all the same.
        if matches!(
            cmd,
            "pairing_old_start"
                | "pairing_new_search"
                | "set_tray_labels"
                | "quicksend_pick"
                | "quicksend_choose_folder"
                | "known_allow_folder"
                | "move_choose_place"
                | "history_export"
                | "update_check"
                | "update_install"
        ) {
            continue;
        }
        if let Err(error) = call(&main, LOCAL, cmd, Value::Object(given)) {
            assert!(
                !args_error(cmd, &error),
                "{cmd} takes an argument the screens do not send: {error}"
            );
        }
    }
}
