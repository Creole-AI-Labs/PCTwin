//! Removing PCTwin's own temporary folders on this laptop (Task List 2.7 R07; Security Design A
//! "Clean exit" and C "Removing our own cache or model ... is a separate step the user approves").
//!
//! PCTwin records each temporary folder it makes (`made-temp.json` in its data folder, written
//! before the folder is used). Only a recorded folder, still a real folder (never a link or a
//! junction) under the recorded path, and not in use by a move, is removed, after the person
//! confirmed exactly that list (`pctwin_readiness::cleanup`). The person's files are never in
//! one of these folders: they hold downloaded installers and app data staged while a move runs.

use std::path::{Path, PathBuf};

use tauri::{AppHandle, Manager, Runtime};

use pctwin_readiness::cleanup::{Entry, Ownership};

/// The names of PCTwin's temporary folders, as it makes them.
const PREFIXES: [&str; 2] = ["pctwin-installs-", "pctwin-capture-"];
/// The most entries counted for a folder's size.
const MAX_COUNT: usize = 100_000;

/// Whether a name looks like one of PCTwin's temporary folders (a prefix, then 16 hex digits).
pub fn looks_ours(name: &str) -> bool {
    PREFIXES.iter().any(|p| {
        name.strip_prefix(p)
            .is_some_and(|tag| tag.len() == 16 && tag.bytes().all(|b| b.is_ascii_hexdigit()))
    })
}

fn record_file<R: Runtime>(app: &AppHandle<R>) -> Option<PathBuf> {
    app.path().app_data_dir().ok()
}

/// Records that PCTwin made the temporary folder `path` (before it is used). Failing to record
/// it only means clean-up will not offer it.
pub fn record<R: Runtime>(app: &AppHandle<R>, path: &Path) {
    let Some(dir) = record_file(app) else { return };
    let mut made: Vec<PathBuf> =
        crate::finish_store::load(&dir, "made-temp.json").unwrap_or_default();
    if !made.iter().any(|p| p == path) {
        made.push(path.to_path_buf());
        let _ = crate::finish_store::save(&dir, "made-temp.json", &made);
    }
}

/// Whether a move is running on this laptop now (its folders are in use).
pub fn busy<R: Runtime>(app: &AppHandle<R>) -> bool {
    app.try_state::<crate::moving::AppMoving>()
        .is_some_and(|m| m.is_running())
}

/// The bytes in a folder, without following links (at most [`MAX_COUNT`] entries counted).
pub fn size_of(path: &Path) -> u64 {
    let mut total = 0u64;
    let mut count = 0usize;
    let mut stack = vec![path.to_path_buf()];
    while let Some(dir) = stack.pop() {
        for e in std::fs::read_dir(&dir).into_iter().flatten().flatten() {
            count += 1;
            if count > MAX_COUNT {
                return total;
            }
            let Ok(m) = std::fs::symlink_metadata(e.path()) else {
                continue;
            };
            if m.file_type().is_symlink() {
                continue;
            }
            if m.is_dir() {
                stack.push(e.path());
            } else {
                total = total.saturating_add(m.len());
            }
        }
    }
    total
}

/// Removes each approved folder, checked again just before: recorded, still a real folder, not a
/// link. Returns how many were removed and how many could not be. `remove_dir_all` does not follow
/// links inside the folder. Each one removed comes off the record.
pub fn remove<R: Runtime>(app: &AppHandle<R>, approved: &[Entry]) -> (u64, u64) {
    let Some(dir) = record_file(app) else {
        return (0, approved.len() as u64);
    };
    let mut made: Vec<PathBuf> =
        crate::finish_store::load(&dir, "made-temp.json").unwrap_or_default();
    let temp = std::env::temp_dir();
    let (mut removed, mut failed) = (0, 0);
    for e in approved {
        let path = temp.join(&e.name);
        let still = e.ownership == Ownership::RecordedByPctwin
            && looks_ours(&e.name)
            && made.contains(&path)
            && std::fs::symlink_metadata(&path)
                .is_ok_and(|m| m.is_dir() && !m.file_type().is_symlink());
        if still && std::fs::remove_dir_all(&path).is_ok() {
            made.retain(|p| p != &path);
            removed += 1;
        } else {
            failed += 1;
        }
    }
    let _ = crate::finish_store::save(&dir, "made-temp.json", &made);
    (removed, failed)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn only_pctwin_s_own_temporary_names_look_ours() {
        assert!(looks_ours("pctwin-installs-0011223344556677"));
        assert!(looks_ours("pctwin-capture-a1b2c3d4e5f60718"));
        for not in [
            "pctwin-installs-",
            "pctwin-installs-0011",
            "pctwin-installs-00112233445566778",
            "pctwin-installs-zz11223344556677",
            "pctwin-notes",
            "installs-0011223344556677",
            "pctwin-installs-0011223344556677/../x",
        ] {
            assert!(!looks_ours(not), "{not}");
        }
    }

    #[test]
    fn a_folder_s_size_never_follows_a_link() {
        let d = tempfile::tempdir().unwrap();
        std::fs::write(d.path().join("a"), [0u8; 10]).unwrap();
        std::fs::create_dir(d.path().join("sub")).unwrap();
        std::fs::write(d.path().join("sub").join("b"), [0u8; 5]).unwrap();
        assert_eq!(size_of(d.path()), 15);
    }
}
