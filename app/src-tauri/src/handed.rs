//! Files the system hands PCTwin to send (Quick Send Stage 1 entry points). Every entry point
//! ends the same way: PCTwin starts (or is opened) with `--send` followed by the chosen paths,
//! and the paths join what is chosen on the pick page. Nothing is sent before the person chooses
//! Send.
//!
//! - **Windows "Send to"**: a shortcut named "PCTwin" in the person's Send to folder
//!   (`shell:sendto`), target `PCTwin.exe --send`; Windows adds the chosen paths after it. Made
//!   by the installer (installer notes for the Chief).
//! - **Linux file managers**: the app's `.desktop` file with `Exec=pctwin --send %F`; Nautilus,
//!   Dolphin and Nemo list it under Open With.
//! - **Mac**: the app declares that it opens any file and folder (`CFBundleDocumentTypes`); Open
//!   With and a drop on the Dock icon hand the files to the running app as an "opened" run
//!   event, which [`opened`] reads. Not wired yet: `run()` is pinned by the lockdown test (a
//!   security decision for the Chief), so the run-event hook waits for that change. A true Share
//!   extension needs its own signed target and is not built.
//! - **The widget**: a drop on the widget joins the chosen list (`quicksend::dropped`); the
//!   tray's Send item is the screens' to add.
//!
//! When PCTwin is already running, a second start with `--send` must hand its paths to the first
//! one (a single-instance plugin, not yet added: noted for the Chief).

use std::path::PathBuf;
use std::sync::Mutex;

/// The flag that starts a list of paths to send.
pub const SEND_FLAG: &str = "--send";
/// Most paths taken from one start.
pub const MOST_PATHS: usize = 1000;

static WAITING: Mutex<Vec<PathBuf>> = Mutex::new(Vec::new());

/// The paths after `--send` in `args` (the program's own name first), each an absolute path
/// that exists; anything else, or anything before the flag, is ignored.
pub fn paths_in(args: impl IntoIterator<Item = std::ffi::OsString>) -> Vec<PathBuf> {
    let mut after = false;
    let mut out = Vec::new();
    for arg in args.into_iter().skip(1) {
        if !after {
            after = arg == SEND_FLAG;
            continue;
        }
        let p = PathBuf::from(arg);
        if p.is_absolute() && std::fs::symlink_metadata(&p).is_ok() && out.len() < MOST_PATHS {
            out.push(p);
        }
    }
    out
}

fn waiting() -> std::sync::MutexGuard<'static, Vec<PathBuf>> {
    WAITING
        .lock()
        .unwrap_or_else(std::sync::PoisonError::into_inner)
}

/// Keeps `paths` for the pick page.
pub fn keep(paths: Vec<PathBuf>) {
    let mut w = waiting();
    w.extend(paths);
    w.truncate(MOST_PATHS);
}

/// Keeps the paths this start was given.
pub fn keep_args() {
    keep(paths_in(std::env::args_os()));
}

/// The Mac's "opened" files (Open With, a drop on the Dock icon).
#[allow(dead_code, reason = "the run-event hook waits for the lockdown change")]
pub fn opened(urls: &[tauri::Url]) -> Vec<PathBuf> {
    urls.iter()
        .filter(|u| u.scheme() == "file")
        .filter_map(|u| u.to_file_path().ok())
        .filter(|p| std::fs::symlink_metadata(p).is_ok())
        .take(MOST_PATHS)
        .collect()
}

/// The paths waiting for the pick page, taken once.
pub fn take() -> Vec<PathBuf> {
    std::mem::take(&mut *waiting())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn only_existing_absolute_paths_after_the_flag_are_taken() {
        let d = tempfile::tempdir().unwrap();
        let a = d.path().join("a.txt");
        std::fs::write(&a, b"x").unwrap();
        let gone = d.path().join("gone");
        let args = |v: &[&std::ffi::OsStr]| {
            v.iter()
                .map(|s| s.to_os_string())
                .collect::<Vec<std::ffi::OsString>>()
        };
        let got = paths_in(args(&[
            "pctwin".as_ref(),
            a.as_os_str(),
            SEND_FLAG.as_ref(),
            a.as_os_str(),
            "relative.txt".as_ref(),
            gone.as_os_str(),
            d.path().as_os_str(),
        ]));
        assert_eq!(got, vec![a.clone(), d.path().to_path_buf()]);
        assert!(paths_in(args(&["pctwin".as_ref(), a.as_os_str()])).is_empty());
        // The program's own name is never taken for the flag.
        assert!(paths_in(args(&[SEND_FLAG.as_ref(), a.as_os_str()])).is_empty());
    }

    #[test]
    fn only_existing_files_the_mac_opened_are_taken() {
        let d = tempfile::tempdir().unwrap();
        let a = d.path().join("a.txt");
        std::fs::write(&a, b"x").unwrap();
        let urls = vec![
            tauri::Url::from_file_path(&a).unwrap(),
            tauri::Url::from_file_path(d.path().join("gone")).unwrap(),
            tauri::Url::parse("https://example.org/a.txt").unwrap(),
        ];
        assert_eq!(opened(&urls), vec![a]);
    }
}
