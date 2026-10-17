//! The screens' reach, checked at the source. The screens are an untrusted web view (Security
//! Design: the screens to engine bridge), so what they may call is a closed list. These tests read
//! the inputs Tauri builds its permissions from, as text: `invoke_handler` in lib.rs (what exists),
//! `COMMANDS` in build.rs (what gets a permission), capabilities/ (what the main window is granted),
//! permissions/ (which must hold only what the build makes), the platform overlays
//! (tauri.<system>.conf.json, which may change only how the window looks), and the app's code
//! (which may not grant anything while the app runs). tests/permissions.rs then checks what Tauri
//! really resolved from all of them, on the merged config, by calling every command it knows.

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

/// Every file in a folder and its subfolders, as paths relative to it with `/` between parts.
fn files_under(dir: &Path) -> Vec<String> {
    let mut found = Vec::new();
    let mut todo = vec![dir.to_path_buf()];
    while let Some(d) = todo.pop() {
        for entry in std::fs::read_dir(&d).unwrap_or_else(|e| panic!("{}: {e}", d.display())) {
            let path = entry.unwrap().path();
            if path.is_dir() {
                todo.push(path);
            } else {
                let rel = path.strip_prefix(dir).unwrap();
                let parts: Vec<String> = rel
                    .components()
                    .map(|c| c.as_os_str().to_string_lossy().into_owned())
                    .collect();
                found.push(parts.join("/"));
            }
        }
    }
    found.sort();
    found
}

/// Tauri reads every permission file in permissions/, and one there can define a set named like an
/// app permission (`allow-pairing-stop`) that grants more. Only the build's own files may be there:
/// one per command, made from build.rs's `COMMANDS` (the folder is not checked in).
#[test]
fn permissions_holds_only_what_the_build_makes() {
    let dir = here().join("permissions");
    let mut want: Vec<String> = build_rs_commands()
        .iter()
        .map(|c| format!("autogenerated/{c}.toml"))
        .collect();
    want.sort();
    assert_eq!(
        files_under(&dir),
        want,
        "permissions/ may hold only autogenerated/<command>.toml for each command in build.rs"
    );
    // Each is exactly what tauri-build writes for one command: its allow and deny permissions.
    for cmd in build_rs_commands() {
        let text = read(&dir.join(format!("autogenerated/{cmd}.toml"))).replace("\r\n", "\n");
        let dashed = cmd.replace('_', "-");
        let want = format!(
            "# Automatically generated - DO NOT EDIT!\n\n\
             [[permission]]\n\
             identifier = \"allow-{dashed}\"\n\
             description = \"Enables the {cmd} command without any pre-configured scope.\"\n\
             commands.allow = [\"{cmd}\"]\n\n\
             [[permission]]\n\
             identifier = \"deny-{dashed}\"\n\
             description = \"Denies the {cmd} command without any pre-configured scope.\"\n\
             commands.deny = [\"{cmd}\"]\n"
        );
        assert_eq!(
            text, want,
            "permissions/autogenerated/{cmd}.toml is not the build's own"
        );
    }
}

/// What a platform overlay may set: how the main window looks on that system (today macOS's
/// see-through title bar). Anything else (permissions, the CSP, the global Tauri object, a remote
/// page, another window) belongs in tauri.conf.json, where it is checked, or nowhere.
const OVERLAY_WINDOW_KEYS: [&str; 10] = [
    "label",
    "title",
    "width",
    "height",
    "minWidth",
    "minHeight",
    "resizable",
    "center",
    "titleBarStyle",
    "hiddenTitle",
];

fn check_overlay(name: &str, overlay: &Value) {
    let object = overlay
        .as_object()
        .unwrap_or_else(|| panic!("{name}: not an object"));
    for key in object.keys() {
        assert!(
            matches!(key.as_str(), "$schema" | "app"),
            "{name}: `{key}` may not be set in a platform overlay"
        );
    }
    let Some(app) = object.get("app") else {
        return;
    };
    for key in app.as_object().expect("app is an object").keys() {
        assert_eq!(
            key, "windows",
            "{name}: `app.{key}` may not be set in a platform overlay"
        );
    }
    let windows = app["windows"].as_array().expect("windows is a list");
    assert_eq!(windows.len(), 1, "{name}: one window only");
    let window = windows[0].as_object().expect("a window is an object");
    for key in window.keys() {
        assert!(
            OVERLAY_WINDOW_KEYS.contains(&key.as_str()),
            "{name}: `app.windows[0].{key}` may not be set in a platform overlay"
        );
    }
    assert_eq!(
        window.get("label"),
        Some(&Value::from("main")),
        "{name}: the window is the main window"
    );
}

#[test]
fn platform_overlays_change_only_how_the_window_looks() {
    let mut overlays = 0;
    for entry in std::fs::read_dir(here()).unwrap() {
        let name = entry.unwrap().file_name().to_string_lossy().into_owned();
        let lower = name.to_ascii_lowercase();
        if !lower.starts_with("tauri") || lower == "tauri.conf.json" {
            continue;
        }
        let platform = lower
            .strip_prefix("tauri.")
            .and_then(|r| r.strip_suffix(".conf.json"))
            .unwrap_or_else(|| {
                panic!(
                    "{name}: Tauri's config is tauri.conf.json and tauri.<system>.conf.json only"
                )
            });
        assert!(
            matches!(platform, "windows" | "macos" | "linux" | "android" | "ios"),
            "{name}: not a system Tauri merges"
        );
        assert_eq!(name, lower, "{name}: Tauri's config names are lower case");
        let overlay: Value = serde_json::from_str(&read(&here().join(&name)))
            .unwrap_or_else(|e| panic!("{name} is not JSON: {e}"));
        check_overlay(&name, &overlay);
        overlays += 1;
    }
    assert!(overlays >= 1, "tauri.macos.conf.json was not found");
}

#[test]
fn the_overlay_check_refuses_what_it_must() {
    use serde_json::json;
    let refused = |v: Value| std::panic::catch_unwind(|| check_overlay("test", &v)).is_err();
    let window = |extra: Value| {
        let mut w = json!({ "label": "main", "titleBarStyle": "Transparent" });
        w.as_object_mut()
            .unwrap()
            .extend(extra.as_object().unwrap().clone());
        json!({ "app": { "windows": [w] } })
    };
    assert!(!refused(window(json!({}))));
    for bad in [
        json!({ "app": { "withGlobalTauri": true } }),
        json!({ "app": { "security": { "csp": null } } }),
        json!({ "app": { "security": { "capabilities": ["default", {
            "identifier": "x", "windows": ["main"], "permissions": ["fs:default"]
        }] } } }),
        json!({ "build": { "frontendDist": "https://evil.example/" } }),
        json!({ "plugins": { "opener": {} } }),
        window(json!({ "url": "https://evil.example/" })),
        window(json!({ "proxyUrl": "http://evil.example/" })),
        window(json!({ "label": "other" })),
        json!({ "app": { "windows": [{ "label": "main" }, { "label": "two" }] } }),
    ] {
        assert!(refused(bad.clone()), "{bad} must be refused");
    }
}

/// The app's code (src/ and build.rs).
fn app_code() -> Vec<(String, String)> {
    let src = here().join("src");
    let mut code: Vec<(String, String)> = files_under(&src)
        .into_iter()
        .filter(|f| f.ends_with(".rs") && f != "lockdown.rs")
        .map(|f| (format!("src/{f}"), read(&src.join(&f))))
        .collect();
    code.push(("build.rs".into(), read(&here().join("build.rs"))));
    code
}

/// Grants made while the app runs never reach the files the build resolves permissions from, so
/// the code may not make any: no capability added at run time, no reach into the permission
/// store, and one context, made from the build's files only. (Each name is built from pieces so
/// this file does not trip its own check.)
#[test]
fn the_code_grants_nothing_while_the_app_runs() {
    let banned = [
        ["add", "_capability"].concat(),
        ["runtime", "_authority"].concat(),
        ["__allow", "_command"].concat(),
        ["Capability", "Builder"].concat(),
        ["Runtime", "Capability"].concat(),
        ["Capability", "File"].concat(),
    ];
    let code = app_code();
    for (file, text) in &code {
        for word in &banned {
            assert!(
                !text.contains(word.as_str()),
                "{file} uses `{word}`, which can grant the screens more than the closed list"
            );
        }
    }
    let all: String = code.iter().map(|(_, t)| t.as_str()).collect();
    assert_eq!(
        all.matches("generate_context!").count(),
        1,
        "the context is made in one place (lib.rs `context`)"
    );
    assert!(
        all.contains("tauri::generate_context!()"),
        "the context takes no extra capabilities"
    );
}

/// The text of the function whose signature is `signature`, braces included.
fn fn_body<'a>(text: &'a str, signature: &str) -> &'a str {
    let start = text
        .find(signature)
        .unwrap_or_else(|| panic!("`{signature}` not found"));
    let open = start + text[start..].find('{').unwrap();
    let mut depth = 0;
    for (i, c) in text[open..].char_indices() {
        match c {
            '{' => depth += 1,
            '}' => {
                depth -= 1;
                if depth == 0 {
                    return &text[open..=open + i];
                }
            }
            _ => {}
        }
    }
    panic!("`{signature}` has no end")
}

/// run() and the permission tests share one builder function, so what the tests check is the app.
/// Every plugin, command list, page hook and setup step is added there and nowhere else.
#[test]
fn the_app_is_built_in_one_place() {
    let lib = read(&here().join("src/lib.rs"));
    let app = fn_body(
        &lib,
        "pub fn app<R: Runtime>(builder: Builder<R>) -> Builder<R>",
    );
    let code = app_code();
    let all: String = code.iter().map(|(_, t)| t.as_str()).collect();
    for call in [
        ".plugin(",
        ".invoke_handler(",
        ".setup(",
        ".on_page_load(",
        ".on_navigation(",
        "_uri_scheme_protocol(",
    ] {
        let inside = app.matches(call).count()
            + if call == ".on_navigation(" {
                fn_body(&lib, "fn navigation_guard").matches(call).count()
            } else {
                0
            };
        assert_eq!(
            all.matches(call).count(),
            inside,
            "`{call}` is used outside lib.rs `app`, which the permission tests build"
        );
    }
    let run = fn_body(&lib, "pub fn run()");
    assert_eq!(
        run.split_whitespace().collect::<String>(),
        "{app(Builder::default()).run(context()).expect(\"errorwhilerunningPCTwin\");}",
        "run() is the shared builder and context, and nothing else"
    );
}

/// Tauri's `dynamic-acl` feature is what lets code add a capability while the app runs. It is off,
/// so such code does not compile; this keeps it off.
#[test]
fn tauri_cannot_take_grants_at_run_time() {
    let manifest = read(&here().join("Cargo.toml"));
    let lines: Vec<&str> = manifest
        .lines()
        .filter(|l| l.trim_start().starts_with("tauri = "))
        .collect();
    assert_eq!(
        lines.len(),
        2,
        "tauri is a dependency and a test dependency"
    );
    for line in lines {
        assert!(
            line.contains("default-features = false"),
            "tauri's default features include dynamic-acl: {line}"
        );
    }
    assert!(
        !manifest.contains("\"dynamic-acl\"") && !manifest.contains("tauri/dynamic-acl"),
        "Cargo.toml turns on tauri's dynamic-acl"
    );
}
