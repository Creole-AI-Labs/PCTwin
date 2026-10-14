//! PCTwin desktop app entry point: the window, the tray icon and the lock-down around the screens.

use serde::Deserialize;
use tauri::menu::{Menu, MenuItem};
use tauri::tray::TrayIconBuilder;
use tauri::{AppHandle, LogicalSize, Manager, Runtime, Url, WebviewWindow, Wry};

/// The three window sizes from the design system, in logical pixels.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum WindowMode {
    /// Welcome, roles, pairing, progress and the personal home.
    Everyday,
    /// The small progress panel.
    Mini,
    /// Reviews that need room: inventory, plans, reports, settings.
    Expanded,
}

impl WindowMode {
    pub fn size(self) -> (f64, f64) {
        match self {
            Self::Everyday => (480.0, 620.0),
            Self::Mini => (360.0, 240.0),
            Self::Expanded => (760.0, 620.0),
        }
    }
}

/// The screens may ask for one of the three sizes by name, never an arbitrary size or position.
#[tauri::command]
fn set_window_mode(window: WebviewWindow, mode: WindowMode) -> Result<(), String> {
    let (width, height) = mode.size();
    window
        .set_size(LogicalSize::new(width, height))
        .map_err(|e| e.to_string())
}

const TRAY_SHOW: &str = "show";
const TRAY_QUIT: &str = "quit";

/// Tray menu labels come from the screens' translation files, so the menu speaks the person's
/// language; until the screens send them, the menu has the app's name only.
#[tauri::command]
fn set_tray_labels(app: AppHandle, show: String, quit: String) -> Result<(), String> {
    let short = |s: &str| s.chars().take(60).collect::<String>();
    let menu = tray_menu(&app, &short(&show), &short(&quit)).map_err(|e| e.to_string())?;
    if let Some(tray) = app.tray_by_id("main") {
        tray.set_menu(Some(menu)).map_err(|e| e.to_string())?;
    }
    Ok(())
}

fn tray_menu<R: Runtime>(app: &AppHandle<R>, show: &str, quit: &str) -> tauri::Result<Menu<R>> {
    let show = MenuItem::with_id(app, TRAY_SHOW, show, true, None::<&str>)?;
    let quit = MenuItem::with_id(app, TRAY_QUIT, quit, true, None::<&str>)?;
    Menu::with_items(app, &[&show, &quit])
}

/// The screens are the app's own files. Anything else (a remote page, a file URL) is refused;
/// links go to the person's browser through the opener, limited to our own site.
pub fn allowed_navigation(url: &Url) -> bool {
    match url.scheme() {
        "tauri" => true,
        "http" | "https" => matches!(
            (url.host_str(), url.port()),
            (Some("tauri.localhost"), None) | (Some("localhost"), Some(1420))
        ),
        _ => false,
    }
}

fn navigation_guard() -> tauri::plugin::TauriPlugin<Wry> {
    tauri::plugin::Builder::new("navigation-guard")
        .on_navigation(|_webview, url| allowed_navigation(url))
        .build()
}

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .plugin(navigation_guard())
        .plugin(tauri_plugin_opener::init())
        // Remembers the size and position the person chose.
        .plugin(tauri_plugin_window_state::Builder::default().build())
        .invoke_handler(tauri::generate_handler![set_window_mode, set_tray_labels])
        // Release check (Phase 0 exit gate: "the blank app opens"): with PCTWIN_SMOKE_TEST set to a
        // file path, the app opens as usual, writes which page its window loaded once the screens
        // have finished loading, and quits. Without it, nothing here runs.
        .on_page_load(|webview, payload| {
            if payload.event() != tauri::webview::PageLoadEvent::Finished {
                return;
            }
            let Some(out) = std::env::var_os("PCTWIN_SMOKE_TEST") else {
                return;
            };
            let page = webview.url().map(|u| u.to_string()).unwrap_or_default();
            let title = webview.window().title().unwrap_or_default();
            let _ = std::fs::write(
                out,
                format!(
                    "opened {page} titled {title}
"
                ),
            );
            let app = webview.app_handle().clone();
            std::thread::spawn(move || {
                std::thread::sleep(std::time::Duration::from_secs(2));
                app.exit(0);
            });
        })
        .setup(|app| {
            let menu = tray_menu(app.handle(), "PCTwin", "PCTwin")?;
            let mut tray = TrayIconBuilder::with_id("main")
                .tooltip("PCTwin")
                .menu(&menu)
                .show_menu_on_left_click(true)
                .on_menu_event(|app, event| match event.id().as_ref() {
                    TRAY_SHOW => {
                        if let Some(window) = app.get_webview_window("main") {
                            let _ = window.unminimize();
                            let _ = window.show();
                            let _ = window.set_focus();
                        }
                    }
                    TRAY_QUIT => app.exit(0),
                    _ => {}
                });
            if let Some(icon) = app.default_window_icon() {
                tray = tray.icon(icon.clone());
            }
            tray.build(app)?;
            Ok(())
        })
        .run(tauri::generate_context!())
        .expect("error while running PCTwin");
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_three_window_sizes_match_the_design_system() {
        assert_eq!(WindowMode::Everyday.size(), (480.0, 620.0));
        assert_eq!(WindowMode::Mini.size(), (360.0, 240.0));
        assert_eq!(WindowMode::Expanded.size(), (760.0, 620.0));
    }

    #[test]
    fn only_the_three_sizes_can_be_asked_for() {
        assert!(serde_json::from_str::<WindowMode>("\"mini\"").is_ok());
        assert!(serde_json::from_str::<WindowMode>("\"fullscreen\"").is_err());
        assert!(serde_json::from_str::<WindowMode>("{\"width\":9999}").is_err());
    }

    #[test]
    fn the_window_only_shows_the_apps_own_screens() {
        let ok = |s: &str| allowed_navigation(&Url::parse(s).unwrap());
        assert!(ok("tauri://localhost/index.html"));
        assert!(ok("http://tauri.localhost/"));
        assert!(ok("http://localhost:1420/"));
        assert!(!ok("https://getpctwin.com/"));
        assert!(!ok("https://evil.example/"));
        assert!(!ok("http://localhost:8080/"));
        assert!(!ok("file:///C:/Windows/System32/"));
        assert!(!ok("javascript:alert(1)"));
    }
}
