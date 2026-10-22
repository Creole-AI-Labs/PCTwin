// Prevents additional console window on Windows in release, DO NOT REMOVE!!
#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

fn main() {
    // Before anything is opened: room for the move's files and the old laptop's listener.
    let _ = pctwin_app_lib::limits::raise_open_files();
    pctwin_app_lib::run()
}
