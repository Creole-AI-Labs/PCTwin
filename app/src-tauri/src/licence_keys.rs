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
use zeroize::Zeroizing;

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

/// A key on its way to the screen: wiped from this process's memory once it is sent.
pub struct ShownKey(Zeroizing<String>);

impl Serialize for ShownKey {
    fn serialize<S: serde::Serializer>(&self, s: S) -> Result<S::Ok, S::Error> {
        s.serialize_str(&self.0)
    }
}

impl std::fmt::Debug for ShownKey {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str("ShownKey(hidden)")
    }
}

/// The key at place `id`, read again now, for the person who asked to see it.
#[tauri::command]
pub fn licence_key_show(id: String) -> Option<ShownKey> {
    let laptop = here()?;
    keys::read_one(keys::shipped(), this_system(), &id, &laptop)
        .map(|k| ShownKey(Zeroizing::new(k.reveal().to_owned())))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_shown_key_goes_out_as_its_text_and_never_prints() {
        let k = ShownKey(Zeroizing::new("ABCD-1234".into()));
        assert_eq!(serde_json::to_string(&k).unwrap(), "\"ABCD-1234\"");
        assert_eq!(format!("{k:?}"), "ShownKey(hidden)");
    }
}
