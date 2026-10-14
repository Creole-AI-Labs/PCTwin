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

/// The approved symbol in one colour (scripts/make-tray-icons.py): black for macOS's menu bar
/// (as a template image the system recolours) and light Windows taskbars, white for dark ones.
/// Raw RGBA pixels, so no image decoder is needed.
const TRAY_BLACK: &[u8] = include_bytes!("../icons/tray/tray-black.rgba");
const TRAY_WHITE: &[u8] = include_bytes!("../icons/tray/tray-white.rgba");
const TRAY_SIZE: u32 = 64;

fn tray_image(rgba: &'static [u8]) -> tauri::image::Image<'static> {
    tauri::image::Image::new(rgba, TRAY_SIZE, TRAY_SIZE)
}

/// Whether a Windows registry answer (`reg query ... /v SystemUsesLightTheme`) says the taskbar is
/// light. Windows 11's taskbar is dark unless the person chose light, so anything unclear is dark.
pub fn taskbar_is_light(reg_output: &str) -> bool {
    reg_output
        .lines()
        .find(|l| l.contains("SystemUsesLightTheme"))
        .and_then(|l| l.split_whitespace().last())
        .is_some_and(|v| v.eq_ignore_ascii_case("0x1"))
}

/// Sets the tray picture that suits the system's bar.
fn refresh_tray_icon<R: Runtime>(app: &AppHandle<R>) {
    let Some(tray) = app.tray_by_id("main") else {
        return;
    };
    if cfg!(target_os = "macos") {
        let _ = tray.set_icon(Some(tray_image(TRAY_BLACK)));
        let _ = tray.set_icon_as_template(true);
    } else if cfg!(windows) {
        let light = windows_taskbar_light();
        let _ = tray.set_icon(Some(tray_image(if light {
            TRAY_BLACK
        } else {
            TRAY_WHITE
        })));
    } else if let Some(icon) = app.default_window_icon() {
        // Linux panels come in every colour and do not say which: the blue mark reads on both.
        let _ = tray.set_icon(Some(icon.clone()));
    }
}

#[cfg(windows)]
fn windows_taskbar_light() -> bool {
    use std::os::windows::process::CommandExt;
    // No console window flashes up while asking.
    const CREATE_NO_WINDOW: u32 = 0x0800_0000;
    std::process::Command::new("reg")
        .args([
            "query",
            r"HKCU\Software\Microsoft\Windows\CurrentVersion\Themes\Personalize",
            "/v",
            "SystemUsesLightTheme",
        ])
        .creation_flags(CREATE_NO_WINDOW)
        .output()
        .map(|o| taskbar_is_light(&String::from_utf8_lossy(&o.stdout)))
        .unwrap_or(false)
}

#[cfg(not(windows))]
fn windows_taskbar_light() -> bool {
    false
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
            refresh_tray_icon(app.handle());
            Ok(())
        })
        // The person can switch the taskbar between light and dark at any time.
        .on_window_event(|window, event| {
            if let tauri::WindowEvent::ThemeChanged(_) = event {
                refresh_tray_icon(window.app_handle());
            }
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
    fn the_tray_picture_follows_the_taskbar() {
        let reg = |v: &str| {
            format!(
                "\r\nHKEY_CURRENT_USER\\Software\\Microsoft\\Windows\\CurrentVersion\\Themes\\Personalize\r\n    SystemUsesLightTheme    REG_DWORD    {v}\r\n"
            )
        };
        assert!(taskbar_is_light(&reg("0x1")));
        assert!(!taskbar_is_light(&reg("0x0")));
        // Unreadable or missing: Windows 11's default dark taskbar.
        assert!(!taskbar_is_light(""));
        assert!(!taskbar_is_light(
            "ERROR: The system was unable to find the specified registry key"
        ));
    }

    #[test]
    fn both_tray_pictures_are_real_images() {
        for bytes in [TRAY_BLACK, TRAY_WHITE] {
            assert_eq!(bytes.len(), (TRAY_SIZE * TRAY_SIZE * 4) as usize);
            // Transparent around the mark, solid in it.
            let alpha: Vec<u8> = bytes.chunks(4).map(|p| p[3]).collect();
            assert!(alpha.contains(&0) && alpha.contains(&255));
        }
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
