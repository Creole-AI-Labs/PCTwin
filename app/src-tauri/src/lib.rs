//! PCTwin desktop app entry point: the window, the tray icon and the lock-down around the screens.
//!
//! The screens are an untrusted web view. They may call only the commands in [`invoke_handler`],
//! each allowed by name: build.rs lists them for the app's permission manifest, and
//! capabilities/default.json grants each one to the main window only. lockdown.rs checks the
//! inputs Tauri builds its permissions from (and the config it really merged); tests/permissions.rs
//! builds the real app with [`app`] and [`context`] on Tauri's mock runtime and calls every command
//! Tauri knows, so anything granted beyond the closed list fails it.

// The engine-written contract (contract.rs) lists every command in one json! value.
#![recursion_limit = "256"]

#[cfg(test)]
mod contract;
// The Phase 2 deliverables made explicit (Task List 2.9) that need their own commands.
mod deliver;
// The engine's 2.2, 2.3 and 2.6 runners publish into the guard and take its intents as they are
// wired in (Task List 2.8 follow-up); until then the conversions and the publishing side are
// exercised by their tests only.
#[allow(dead_code)]
mod guard;
// Finishing a move (Task List 2.7): the overview, the before-wipe list, handover and clean-up.
mod finish;
mod finish_catchup;
mod finish_cleanup;
mod finish_doctor;
mod finish_store;
// The full move: apps, settings and special categories joined to the move (Phase 2 joining).
mod fullmove;
#[cfg(test)]
mod guard_examples;
#[allow(dead_code)]
mod guard_views;
mod handed;
mod help;
mod history;
#[cfg(test)]
mod info_plist;
mod keep_local;
mod laptop_name;
pub mod launch;
pub mod limits;
#[cfg(test)]
mod lockdown;
mod moving;
mod others;
mod pairing;
mod quicksend;
pub mod rebuild;
mod recovery;
mod remembered;
mod restart;
mod sessions;
mod undo_books;
mod updates;

/// Other apps' data files (their JSON and databases) are read in parser processes too, one per
/// folder read at once; each starts only when first asked.
fn install_content_readers() {
    static CONTENTS: std::sync::OnceLock<pctwin_parse::Pool> = std::sync::OnceLock::new();
    let pool = CONTENTS.get_or_init(|| {
        pctwin_parse::Pool::beside_this_program(pctwin_special::classify::PLAN_THREADS)
    });
    pctwin_special::content::install_reader(pool);
    // Game launchers' records and save lists (2.5 games) go through the same processes.
    pctwin_special::games::install_reader(pool);
}

/// Where the files a parser process reads come from: each source has its own process, so a file
/// that took over one process can never answer for files of another.
#[derive(Clone, Copy)]
pub(crate) enum Files {
    /// The apps installed on this laptop (their programs' and installers' signatures).
    InstalledHere,
    /// Installers found among the person's files on the old laptop (read on Windows only, where
    /// an older copy of the system can hold them; elsewhere nothing makes this source yet).
    #[cfg_attr(not(windows), allow(dead_code))]
    FoundInstallers,
    /// The files a project names, read for its work checks (fonts, archives, databases).
    WorkChecks,
}

/// PCTwin's locked-down parser process (adoption change 1) for files from `from`, beside the
/// app: the insides of untrusted files are read only there.
pub(crate) fn parser(from: Files) -> &'static pctwin_parse::Parser {
    static HERE: std::sync::OnceLock<pctwin_parse::Parser> = std::sync::OnceLock::new();
    static FOUND: std::sync::OnceLock<pctwin_parse::Parser> = std::sync::OnceLock::new();
    static WORK: std::sync::OnceLock<pctwin_parse::Parser> = std::sync::OnceLock::new();
    let slot = match from {
        Files::InstalledHere => &HERE,
        Files::FoundInstallers => &FOUND,
        Files::WorkChecks => &WORK,
    };
    slot.get_or_init(pctwin_parse::Parser::beside_this_program)
}

use serde::Deserialize;
use tauri::ipc::Invoke;
use tauri::menu::{Menu, MenuItem};
use tauri::tray::TrayIconBuilder;
use tauri::{AppHandle, Builder, Context, LogicalSize, Manager, Runtime, Url, WebviewWindow, Wry};

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
            Self::Mini => (340.0, 176.0),
            Self::Expanded => (760.0, 620.0),
        }
    }
}

/// The screens may ask for one of the three sizes by name, never an arbitrary size or position.
#[tauri::command]
fn set_window_mode<R: Runtime>(window: WebviewWindow<R>, mode: WindowMode) -> Result<(), String> {
    let (width, height) = mode.size();
    let e = |e: tauri::Error| e.to_string();
    // Was it the mini box? Then the normal window comes back where the person can see it.
    let was_mini = !window.is_decorated().map_err(e)?;
    let mini = mode == WindowMode::Mini;
    // The mini box floats like a call's small window: no frame, above other windows, in the
    // corner of the screen; the person drags it by its body (data-tauri-drag-region).
    window.set_decorations(!mini).map_err(e)?;
    window.set_always_on_top(mini).map_err(e)?;
    window
        .set_size(LogicalSize::new(width, height))
        .map_err(e)?;
    if mini {
        if let Some(monitor) = window.current_monitor().map_err(e)? {
            let area = monitor.work_area();
            let scale = monitor.scale_factor();
            let margin = (16.0 * scale) as i32;
            let (w, h) = ((width * scale) as i32, (height * scale) as i32);
            let x = area.position.x + area.size.width as i32 - w - margin;
            let y = area.position.y + area.size.height as i32 - h - margin;
            window
                .set_position(tauri::PhysicalPosition::new(x, y))
                .map_err(e)?;
        }
    } else if was_mini {
        window.center().map_err(e)?;
    }
    Ok(())
}

/// The mini box has no frame: the person drags it by its body, and the screen asks for that here
/// (a named command of the closed list, never the window's own permissions).
#[tauri::command]
fn window_drag<R: Runtime>(window: WebviewWindow<R>) -> Result<(), String> {
    window.start_dragging().map_err(|e| e.to_string())
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
    pctwin_types::process::quiet("reg")
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
fn set_tray_labels<R: Runtime>(
    app: AppHandle<R>,
    show: String,
    quit: String,
) -> Result<(), String> {
    let short = |s: &str| s.chars().take(60).collect::<String>();
    let menu = tray_menu(&app, &short(&show), &short(&quit)).map_err(|e| e.to_string())?;
    if let Some(tray) = app.tray_by_id("main") {
        tray.set_menu(Some(menu)).map_err(|e| e.to_string())?;
    }
    Ok(())
}

/// The words of the files PCTwin writes for the person beside a browser's data (its bookmarks
/// file and the list of its add-ons and open pages, 2.5), in the person's language, from the
/// screens' translation files. False when a word is not safe: the English words stay.
#[tauri::command]
fn set_file_words(
    bookmarks_file: String,
    lists_file: String,
    bookmarks_title: String,
    add_ons: String,
    open_pages: String,
) -> bool {
    pctwin_special::browser::set_words(pctwin_special::browser::Words {
        bookmarks_file,
        lists_file,
        bookmarks_title,
        add_ons,
        open_pages,
    })
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
        "http" | "https" => match (url.host_str(), url.port()) {
            (Some("tauri.localhost"), None) => true,
            // The development server: in development builds only, so a release never shows
            // whatever another program serves on that port.
            (Some("localhost"), Some(1420)) => cfg!(debug_assertions),
            _ => false,
        },
        _ => false,
    }
}

fn navigation_guard<R: Runtime>() -> tauri::plugin::TauriPlugin<R> {
    tauri::plugin::Builder::new("navigation-guard")
        .on_navigation(|_webview, url| allowed_navigation(url))
        .build()
}

/// Every command the screens may call. A command must also be listed in build.rs and granted in
/// capabilities/default.json, or the screens are refused it.
fn invoke_handler<R: Runtime>() -> impl Fn(Invoke<R>) -> bool + Send + Sync + 'static {
    tauri::generate_handler![
        set_window_mode,
        window_drag,
        set_tray_labels,
        set_file_words,
        pairing::pairing_old_start,
        pairing::pairing_old_pick,
        pairing::pairing_old_decline,
        pairing::pairing_old_shuffle,
        pairing::pairing_old_rename,
        pairing::pairing_old_start_again,
        pairing::pairing_old_nearby,
        pairing::pairing_heads_up,
        pairing::pairing_stop,
        pairing::pairing_new_search,
        pairing::pairing_new_connect,
        moving::move_begin,
        moving::rescue_drives,
        moving::rescue_begin,
        moving::move_approve,
        moving::move_place_people,
        moving::move_choose_place,
        moving::move_usual_place,
        moving::move_privacy_answer,
        moving::open_privacy_settings,
        moving::move_pause,
        moving::move_resume,
        moving::move_stop,
        moving::move_retry,
        moving::move_report,
        others::move_people_here,
        help::help_code,
        recovery::recovery_check,
        recovery::recovery_run,
        recovery::things_ready,
        history::history_list,
        history::history_open,
        history::history_export,
        quicksend::quicksend_pick,
        quicksend::quicksend_accept_drops,
        quicksend::quicksend_clear,
        quicksend::quicksend_offer,
        quicksend::quicksend_offer_missing,
        quicksend::quicksend_send,
        quicksend::quicksend_receive,
        quicksend::quicksend_folder,
        quicksend::quicksend_choose_folder,
        quicksend::quicksend_save_here,
        quicksend::quicksend_decline_here,
        quicksend::quicksend_open_folder,
        quicksend::quicksend_keep_original_dates,
        quicksend::quicksend_pause,
        quicksend::quicksend_resume,
        quicksend::quicksend_stop,
        quicksend::quicksend_send_note,
        quicksend::quicksend_open_link,
        quicksend::quicksend_launch_files,
        remembered::known_list,
        remembered::known_remember,
        remembered::known_forget,
        remembered::known_rename,
        remembered::known_allow_folder,
        remembered::known_revoke_folder,
        remembered::known_renew_folder,
        remembered::known_next_folder,
        remembered::known_clear_next_folder,
        remembered::known_save_incoming,
        remembered::known_decline_incoming,
        remembered::known_send,
        remembered::waiting_prepare,
        remembered::waiting_list,
        remembered::waiting_renew,
        remembered::waiting_cancel,
        sessions::quicksend_session_add,
        sessions::quicksend_session_end,
        sessions::quicksend_sessions,
        sessions::quicksend_sends,
        sessions::quicksend_send_pause,
        sessions::quicksend_send_resume,
        sessions::quicksend_send_cancel,
        updates::update_check,
        updates::update_install,
        laptop_name::laptop_name,
        laptop_name::laptop_name_shuffle,
        laptop_name::laptop_name_use,
        laptop_name::laptop_name_type,
        guard::guard_view,
        guard::guard_intent,
        fullmove::fullmove_undo,
        fullmove::move_appdata_answer,
        moving::move_choose_what,
        finish::finish_overview,
        finish::goodbye_list,
        finish::goodbye_answer,
        finish::goodbye_keep_until,
        finish::catchup_arm,
        finish::catchup_approve,
        finish::handover_preview,
        finish::handover_confirm,
        finish::handover_result,
        finish::doctor_run,
        finish::setup_checklist,
        finish::setup_mark,
        finish::setup_save,
        finish::undo_preview,
        finish::cleanup_preview,
        finish::cleanup_confirm,
        finish::report_preview,
        finish::report_save,
        deliver::work::work_checks,
        deliver::work::work_check_add,
        deliver::work::work_check_run,
        deliver::work::work_check_answer,
        deliver::work::work_check_remove,
        deliver::repair::repair_offer,
        deliver::repair::repair_run,
        deliver::changes::move_changes,
        deliver::changes::move_reapprove,
        deliver::safe_stop::safe_stop,
        deliver::kit::recovery_kit_save,
    ]
}

/// The app's config, assets and permissions as the build made them: tauri.conf.json merged with
/// this system's tauri.<system>.conf.json, and the permissions resolved from build.rs and
/// capabilities/. The one context the app runs with; the permission tests use it too.
pub fn context<R: Runtime>() -> Context<R> {
    tauri::generate_context!()
}

/// The tray needs the system's real window toolkit. Tauri's mock runtime (the permission tests)
/// has none, so there the app is built without its tray and everything else is the same.
fn has_tray<R: Runtime>() -> bool {
    std::any::TypeId::of::<R>() == std::any::TypeId::of::<Wry>()
}

/// Everything the app adds to Tauri: its plugins, state, commands, page-load check, setup and
/// window events. [`run`] and the permission tests (tests/permissions.rs) both build from this one
/// function, so the tests check the app as it really is. Nothing here, or anywhere else, may grant
/// the screens more at run time: Tauri's `dynamic-acl` feature is off (Cargo.toml), and lockdown.rs
/// refuses code that names a capability or the permission store.
pub fn app<R: Runtime>(builder: Builder<R>) -> Builder<R> {
    install_content_readers();
    builder
        .plugin(navigation_guard())
        .plugin(tauri_plugin_opener::init())
        // Remembers the size and position the person chose.
        .plugin(tauri_plugin_window_state::Builder::default().build())
        .plugin(restart::plugin())
        .plugin(updates::plugin())
        .manage(pairing::new_state())
        .manage(moving::new_state())
        .manage(quicksend::new_state())
        .manage(remembered::new_state())
        .manage(sessions::new_state())
        .manage(laptop_name::new_state())
        .manage(guard::new_state())
        // The Phase 2 role keys, made once as the program starts (pctwin-grant).
        .manage(fullmove::new_state().expect("the role keys are made once, as PCTwin starts"))
        .invoke_handler(invoke_handler())
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
            // The change journal asks the system through the helper's probe, before any journal
            // is opened (third review, finding 8).
            pctwin_helper::os_probes::install();
            // App data staged by a run that never finished is removed before anything else.
            fullmove::sweep_staging(app.handle());
            // PCTwin's own update and uninstall wait for its safe stopping points.
            deliver::safe_stop::set_up(app.handle());
            // Files the system handed PCTwin to send (Send to, a file manager entry), and the
            // remembered connections' service when something is remembered or waiting.
            handed::keep_args();
            remembered::start_if_needed(app.handle());
            // Online-only files an earlier move made local and did not put back go back online
            // only (the move stopped part way): the old laptop ends as it was.
            keep_local::put_back_left(app.handle());
            if !has_tray::<R>() {
                return Ok(());
            }
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
        .on_window_event(|window, event| match event {
            tauri::WindowEvent::ThemeChanged(_) => refresh_tray_icon(window.app_handle()),
            // Quick Send's drag and drop: the paths stay here; the screens get names and sizes.
            tauri::WindowEvent::DragDrop(drop) => match drop {
                tauri::DragDropEvent::Enter { .. } | tauri::DragDropEvent::Over { .. } => {
                    quicksend::dragging(window.app_handle(), true);
                }
                tauri::DragDropEvent::Leave => quicksend::dragging(window.app_handle(), false),
                tauri::DragDropEvent::Drop { paths, .. } => {
                    quicksend::dragging(window.app_handle(), false);
                    quicksend::dropped(window.app_handle(), paths.clone());
                }
                _ => {}
            },
            _ => {}
        })
}

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    app(Builder::default())
        .run(context())
        .expect("error while running PCTwin");
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_three_window_sizes_match_the_design_system() {
        assert_eq!(WindowMode::Everyday.size(), (480.0, 620.0));
        // The mini box: smaller than the design system's 360 x 240 panel, floating in the corner
        // (asked for in the first real test, 28 October 2026).
        assert_eq!(WindowMode::Mini.size(), (340.0, 176.0));
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
        assert_eq!(ok("http://localhost:1420/"), cfg!(debug_assertions));
        assert!(!ok("https://getpctwin.com/"));
        assert!(!ok("https://evil.example/"));
        assert!(!ok("http://localhost:8080/"));
        assert!(!ok("file:///C:/Windows/System32/"));
        assert!(!ok("javascript:alert(1)"));
    }
}
