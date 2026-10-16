//! The screens' permission check, for real: the app's commands and its generated permissions
//! (build.rs and capabilities/, compiled into this test by `generate_context!`) on Tauri's mock
//! runtime. A command the main window is not granted, or any command from another window, is
//! refused by Tauri itself before it runs.
//!
//! Pairing's state is left out on purpose: a command that gets past the permission check then
//! stops at "state not managed" (or at its missing arguments), so nothing here pairs, touches the
//! network or opens anything.

use tauri::Manager;
use tauri::ipc::{CallbackFn, InvokeBody};
use tauri::test::{INVOKE_KEY, MockRuntime, get_ipc_response, mock_builder};
use tauri::webview::{InvokeRequest, WebviewWindowBuilder};
use tauri::{App, WebviewUrl, WebviewWindow};

/// The quoted names in build.rs's `COMMANDS` (src/lockdown.rs checks they are the registered ones).
fn commands() -> Vec<String> {
    let text = std::fs::read_to_string(concat!(env!("CARGO_MANIFEST_DIR"), "/build.rs")).unwrap();
    let open = "const COMMANDS: &[&str] = &[";
    let rest = &text[text.find(open).unwrap() + open.len()..];
    let list = &rest[..rest.find("];").unwrap()];
    let names: Vec<String> = list
        .split(',')
        .map(|s| s.trim().trim_matches('"').to_owned())
        .filter(|s| !s.is_empty())
        .collect();
    assert!(
        names.len() >= 11,
        "the parse found too few commands: {names:?}"
    );
    names
}

fn app() -> App<MockRuntime> {
    mock_builder()
        .invoke_handler(pctwin_app_lib::invoke_handler())
        .build(tauri::generate_context!(test = true))
        .expect("the mock app builds")
}

fn window(app: &App<MockRuntime>, label: &str) -> WebviewWindow<MockRuntime> {
    app.get_webview_window(label).unwrap_or_else(|| {
        WebviewWindowBuilder::new(app, label, WebviewUrl::default())
            .build()
            .unwrap()
    })
}

/// What the screens get back for `cmd` with `args`, from the window `w`.
fn call(w: &WebviewWindow<MockRuntime>, cmd: &str, args: serde_json::Value) -> Result<(), String> {
    let url = if cfg!(any(windows, target_os = "android")) {
        "http://tauri.localhost"
    } else {
        "tauri://localhost"
    };
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
    .map_err(|e| e.to_string())
}

/// Tauri's permission check said no (debug builds give the reason, release builds the same words).
fn refused(result: &Result<(), String>) -> bool {
    matches!(result, Err(e) if e.contains("not allowed"))
}

#[test]
fn the_main_window_may_call_each_granted_command() {
    let app = app();
    let main = window(&app, "main");
    // Asking for a size by name works end to end.
    assert_eq!(
        call(
            &main,
            "set_window_mode",
            serde_json::json!({ "mode": "mini" })
        ),
        Ok(())
    );
    for cmd in commands() {
        // No arguments: a granted command is let through, then stops at a missing argument or at
        // pairing's unmanaged state. Only the permission check says "not allowed".
        let result = call(&main, &cmd, serde_json::json!({}));
        assert!(result.is_err(), "{cmd} ran without its arguments or state");
        assert!(
            !refused(&result),
            "{cmd} was refused to the main window: {result:?}"
        );
    }
}

#[test]
fn any_other_window_is_refused_every_command() {
    let app = app();
    let other = window(&app, "other");
    for cmd in commands() {
        let result = call(&other, &cmd, serde_json::json!({}));
        assert!(
            refused(&result),
            "{cmd} must be refused to another window: {result:?}"
        );
    }
    let result = call(
        &other,
        "set_window_mode",
        serde_json::json!({ "mode": "mini" }),
    );
    assert!(refused(&result), "{result:?}");
}

#[test]
fn the_main_window_is_refused_what_is_not_granted() {
    let app = app();
    let main = window(&app, "main");
    for (cmd, args) in [
        // Opening a new window or web view (for a remote page, say), or changing this one.
        (
            "plugin:webview|create_webview_window",
            serde_json::json!({ "options": { "label": "x", "url": "https://evil.example/" } }),
        ),
        (
            "plugin:window|create",
            serde_json::json!({ "options": { "label": "x" } }),
        ),
        (
            "plugin:window|set_size",
            serde_json::json!({ "label": "main", "value": { "Logical": { "width": 1, "height": 1 } } }),
        ),
        ("plugin:app|app_show", serde_json::json!({})),
        (
            "plugin:path|resolve_directory",
            serde_json::json!({ "directory": 11 }),
        ),
        // Plugins the app has, whose other commands are not granted.
        (
            "plugin:window-state|save_window_state",
            serde_json::json!({ "flags": 0 }),
        ),
        (
            "plugin:opener|open_path",
            serde_json::json!({ "path": "C:/Windows/System32/cmd.exe" }),
        ),
        // A name the app does not have.
        ("not_a_command", serde_json::json!({})),
    ] {
        let result = call(&main, cmd, args);
        assert!(
            refused(&result),
            "{cmd} must be refused to the main window: {result:?}"
        );
    }
}
