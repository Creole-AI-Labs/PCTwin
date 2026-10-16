//! The screens' reach, checked. The screens are an untrusted web view (Security Design: the
//! screens to engine bridge), so what they may call is a closed list, kept in three places that
//! must agree: `invoke_handler` in lib.rs (what exists), `COMMANDS` in build.rs (what gets a
//! permission), and capabilities/ (what the main window is granted). These tests read those files
//! as text; tests/permissions.rs then asks the real permission check, through Tauri's mock
//! runtime, to refuse what is not granted.

use std::collections::BTreeSet;
use std::path::{Path, PathBuf};

use serde_json::Value;

fn here() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
}

fn read(path: &Path) -> String {
    std::fs::read_to_string(path).unwrap_or_else(|e| panic!("cannot read {}: {e}", path.display()))
}

/// The text between `open` and the next `close` after it.
fn between<'a>(text: &'a str, open: &str, close: &str) -> &'a str {
    let start = text
        .find(open)
        .unwrap_or_else(|| panic!("`{open}` not found"))
        + open.len();
    let len = text[start..]
        .find(close)
        .unwrap_or_else(|| panic!("`{close}` not found after `{open}`"));
    &text[start..start + len]
}

/// The quoted names in build.rs's `COMMANDS`.
fn build_rs_commands() -> Vec<String> {
    let text = read(&here().join("build.rs"));
    between(&text, "const COMMANDS: &[&str] = &[", "];")
        .split(',')
        .map(str::trim)
        .filter(|s| !s.is_empty())
        .map(|s| {
            s.strip_prefix('"')
                .and_then(|s| s.strip_suffix('"'))
                .unwrap_or_else(|| panic!("not a plain quoted name in build.rs: {s}"))
                .to_owned()
        })
        .collect()
}

/// The command names in lib.rs's `generate_handler!` (the last part of each path).
fn handler_commands() -> Vec<String> {
    let text = read(&here().join("src/lib.rs"));
    assert_eq!(
        text.matches("generate_handler!").count(),
        1,
        "lib.rs must register commands in one place only"
    );
    between(&text, "tauri::generate_handler![", "]")
        .split(',')
        .map(str::trim)
        .filter(|s| !s.is_empty())
        .map(|s| s.rsplit("::").next().unwrap_or(s).to_owned())
        .collect()
}

fn no_repeats(list: &[String], what: &str) -> BTreeSet<String> {
    let set: BTreeSet<String> = list.iter().cloned().collect();
    assert_eq!(set.len(), list.len(), "a command is listed twice in {what}");
    set
}

#[test]
fn build_rs_lists_exactly_the_registered_commands() {
    let listed = no_repeats(&build_rs_commands(), "build.rs");
    let registered = no_repeats(&handler_commands(), "lib.rs");
    let unlisted: Vec<_> = registered.difference(&listed).collect();
    let unregistered: Vec<_> = listed.difference(&registered).collect();
    assert!(
        unlisted.is_empty(),
        "registered in lib.rs but missing from build.rs COMMANDS: {unlisted:?}"
    );
    assert!(
        unregistered.is_empty(),
        "in build.rs COMMANDS but not registered in lib.rs: {unregistered:?}"
    );
    assert!(registered.len() >= 11, "the parse found too few commands");
}

/// The permissions the screens need besides the app's own commands, and nothing more.
const OTHER_PERMISSIONS: [&str; 3] = [
    "core:event:allow-listen",
    "core:event:allow-unlisten",
    "opener:allow-open-url",
];

/// Permissions that must never reach the screens: files, dialogs, programs, sharing, the
/// network, processes, and making or changing windows and web views. Also any plugin's default
/// set, which grants whatever that plugin's authors chose.
fn never_granted(identifier: &str) -> bool {
    const PREFIXES: [&str; 9] = [
        "fs:",
        "dialog:",
        "shell:",
        "share:",
        "http:",
        "process:",
        "core:webview:",
        "core:window:",
        "core:app:",
    ];
    PREFIXES.iter().any(|p| identifier.starts_with(p))
        || identifier == "core:default"
        || identifier.ends_with(":default")
}

#[test]
fn the_refused_list_catches_what_it_must() {
    for bad in [
        "fs:default",
        "fs:allow-read-file",
        "dialog:allow-open",
        "shell:allow-execute",
        "shell:allow-open",
        "share:allow-share",
        "http:default",
        "process:allow-restart",
        "core:webview:allow-create-webview-window",
        "core:webview:allow-create-webview",
        "core:window:allow-create",
        "core:window:allow-set-size",
        "core:app:allow-app-show",
        "core:default",
        "opener:default",
        "window-state:default",
    ] {
        assert!(never_granted(bad), "{bad} must be refused");
    }
    for ok in OTHER_PERMISSIONS {
        assert!(!never_granted(ok), "{ok} is needed by the screens");
    }
}

/// Each capability file, by name.
fn capability_files() -> Vec<(String, Value)> {
    let dir = here().join("capabilities");
    let mut files = Vec::new();
    for entry in std::fs::read_dir(&dir).unwrap() {
        let path = entry.unwrap().path();
        let name = path.file_name().unwrap().to_string_lossy().into_owned();
        // Tauri also reads TOML capabilities; only JSON is checked here, so only JSON may exist.
        assert!(
            name.ends_with(".json"),
            "capabilities/{name}: only .json capability files are allowed (they are checked)"
        );
        let value: Value = serde_json::from_str(&read(&path))
            .unwrap_or_else(|e| panic!("capabilities/{name} is not JSON: {e}"));
        files.push((name, value));
    }
    assert!(!files.is_empty(), "no capability files found");
    files
}

#[test]
fn every_capability_is_for_the_main_window_only_and_grants_only_the_closed_list() {
    let wanted: BTreeSet<String> = build_rs_commands()
        .iter()
        .map(|c| format!("allow-{}", c.replace('_', "-")))
        .chain(OTHER_PERMISSIONS.iter().map(|p| (*p).to_owned()))
        .collect();
    let mut granted = BTreeSet::new();
    for (name, cap) in capability_files() {
        let object = cap.as_object().expect("a capability is an object");
        for key in object.keys() {
            assert!(
                matches!(
                    key.as_str(),
                    "$schema" | "identifier" | "description" | "windows" | "permissions"
                ),
                "capabilities/{name}: `{key}` is not allowed (no webviews, remote URLs, platforms or local switch)"
            );
        }
        assert_eq!(
            cap["windows"],
            serde_json::json!(["main"]),
            "capabilities/{name} must apply to the main window only"
        );
        let permissions = cap["permissions"].as_array().expect("a permissions list");
        for p in permissions {
            let identifier = match p {
                Value::String(s) => s.clone(),
                Value::Object(o) => {
                    let id = o["identifier"].as_str().expect("an identifier").to_owned();
                    // The only scoped permission: opening our own site in the browser.
                    assert_eq!(
                        id, "opener:allow-open-url",
                        "capabilities/{name}: unexpected scoped permission"
                    );
                    assert_eq!(
                        p,
                        &serde_json::json!({
                            "identifier": "opener:allow-open-url",
                            "allow": [{ "url": "https://getpctwin.com/*" }]
                        }),
                        "capabilities/{name}: the browser may open our own site only"
                    );
                    id
                }
                other => panic!("capabilities/{name}: not a permission: {other}"),
            };
            assert!(
                !never_granted(&identifier),
                "capabilities/{name} grants {identifier}, which the screens must never have"
            );
            assert!(
                granted.insert(identifier.clone()),
                "{identifier} granted twice"
            );
        }
    }
    let extra: Vec<_> = granted.difference(&wanted).collect();
    let missing: Vec<_> = wanted.difference(&granted).collect();
    assert!(
        extra.is_empty(),
        "granted but not on the closed list: {extra:?}"
    );
    assert!(
        missing.is_empty(),
        "commands the screens use but are not granted: {missing:?}"
    );
}

/// The content security policy as decided; any change is a decision, made here on purpose.
const CSP: &str = "default-src 'self'; script-src 'self'; style-src 'self'; img-src 'self' data:; font-src 'self'; connect-src ipc: http://ipc.localhost; object-src 'none'; base-uri 'none'; form-action 'none'";

#[test]
fn the_config_keeps_the_lockdown() {
    let config: Value = serde_json::from_str(&read(&here().join("tauri.conf.json"))).unwrap();
    let app = &config["app"];
    assert_ne!(
        app["withGlobalTauri"],
        Value::Bool(true),
        "the screens reach Tauri through bridge.ts only, never a global"
    );
    let security = &app["security"];
    assert_eq!(
        security["csp"],
        Value::String(CSP.into()),
        "the CSP changed"
    );
    assert_eq!(security["freezePrototype"], Value::Bool(true));
    assert_eq!(security["capabilities"], serde_json::json!(["default"]));
    for key in [
        "dangerousDisableAssetCspModification",
        "assetProtocol",
        "devCsp",
        "headers",
    ] {
        assert!(
            security.get(key).is_none(),
            "app.security.{key} must not be set"
        );
    }
    let windows = app["windows"].as_array().unwrap();
    assert_eq!(windows.len(), 1);
    assert_eq!(windows[0]["label"], "main");
    assert!(
        windows[0].get("url").is_none(),
        "the window shows the app's own screens only"
    );
}
