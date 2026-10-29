/// Every command the screens may call, as named in `invoke_handler` (src/lib.rs). Listing them
/// here turns on the app's own permission check: a command is refused unless a capability in
/// capabilities/ grants its `allow-<name>` permission (lockdown.rs keeps the lists equal).
const COMMANDS: &[&str] = &[
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
    "quicksend_open_folder",
    "quicksend_keep_original_dates",
    "quicksend_pause",
    "quicksend_resume",
    "quicksend_stop",
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
    "work_checks",
    "work_check_add",
    "work_check_run",
    "work_check_answer",
    "work_check_remove",
    "repair_offer",
    "repair_run",
    "move_changes",
    "move_reapprove",
    "safe_stop",
    "recovery_kit_save",
];

fn main() {
    tauri_build::try_build(
        tauri_build::Attributes::new()
            .app_manifest(tauri_build::AppManifest::new().commands(COMMANDS)),
    )
    .expect("failed to run the Tauri build step");

    // The app's Windows resource holds its manifest, which asks for Common Controls v6 (Tauri
    // needs it). Cargo links it into the app only; the permission tests start Tauri's mock runtime
    // and need it too, or Windows will not start them (STATUS_ENTRYPOINT_NOT_FOUND).
    if std::env::var("CARGO_CFG_TARGET_ENV").as_deref() == Ok("msvc") {
        let out = std::env::var("OUT_DIR").expect("OUT_DIR is set for build scripts");
        let resource = std::path::Path::new(&out).join("resource.lib");
        if resource.exists() {
            println!("cargo:rustc-link-arg-tests={}", resource.display());
        }
    }
}
