//! The product key finder's two commands (Task List 2.1; `pctwin_apps::keys`): where this
//! laptop's apps keep a licence key in the open, and one key, read again, when the person asks
//! to see it. Both run as the person, on their own laptop, read-only.
//!
//! The list carries the app and a place id only, never a key. A key crosses to the screen only
//! through [`licence_key_show`], for the one row asked, and is never kept, logged, written into
//! the move's record or report, or given to the AI.

use pctwin_apps::keys::{self, ThisLaptop};
use pctwin_record::AppSystem;
use serde::Serialize;

/// One app with a licence key found here.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct KeyPlaceView {
    /// The place's id, to ask for its key.
    pub id: String,
    pub app: String,
}

fn this_system() -> AppSystem {
    if cfg!(windows) {
        AppSystem::Windows
    } else if cfg!(target_os = "macos") {
        AppSystem::Mac
    } else {
        AppSystem::Linux
    }
}

fn here() -> Option<ThisLaptop> {
    // The person's own home, as the system names it for the signed-in account.
    std::env::home_dir().map(|home| ThisLaptop { home })
}

/// The apps that keep a licence key in the open on this laptop, for the person signed in.
#[tauri::command]
pub fn licence_keys_found() -> Vec<KeyPlaceView> {
    let Some(laptop) = here() else {
        return Vec::new();
    };
    keys::find(keys::shipped(), this_system(), &laptop)
        .into_iter()
        .map(|k| KeyPlaceView {
            id: k.id,
            app: k.app,
        })
        .collect()
}

/// The key at place `id`, read again now, for the person who asked to see it.
#[tauri::command]
pub fn licence_key_show(id: String) -> Option<String> {
    let laptop = here()?;
    keys::read_one(keys::shipped(), this_system(), &id, &laptop).map(|k| k.reveal().to_owned())
}
