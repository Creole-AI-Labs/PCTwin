//! One helper per laptop: one mover's move at a time (Security Design part C, "serve a second
//! session"). The helper holds an exclusive lock on one file for as long as it runs; a second
//! helper finds it held and ends before it makes a channel. The system lets the lock go when the
//! helper ends, however it ends.

use std::fs::{File, OpenOptions};
use std::path::{Path, PathBuf};

/// Where the lock is: a folder only the system and administrators can make things in, so no one
/// else can take the lock first and keep every move from starting. On Windows that is
/// `Program Files\PCTwin` (found by asking Windows, never the environment), not ProgramData,
/// where every account may make folders.
pub fn lock_path() -> Result<PathBuf, String> {
    #[cfg(windows)]
    {
        pctwin_helper_os::program_folder()
            .map(|dir| dir.join("helper.lock"))
            .map_err(|e| format!("the lock's folder: {e}"))
    }
    // The system's own run folder, root's alone: /run on Linux; a Mac has none, and its /var is
    // a link, so the real /private/var/run (found by the two-accounts lab).
    #[cfg(target_os = "macos")]
    {
        Ok(PathBuf::from("/private/var/run/pctwin-helper.lock"))
    }
    #[cfg(all(unix, not(target_os = "macos")))]
    {
        Ok(PathBuf::from("/run/pctwin-helper.lock"))
    }
}

/// Takes the lock at `path` (its folder made if missing), or says another helper holds it.
pub fn one_helper(path: &Path) -> Result<File, String> {
    if let Some(dir) = path.parent() {
        std::fs::create_dir_all(dir).map_err(|e| format!("the lock's folder: {e}"))?;
    }
    let file = OpenOptions::new()
        .create(true)
        .truncate(false)
        .write(true)
        .open(path)
        .map_err(|e| format!("the lock: {e}"))?;
    match file.try_lock() {
        Ok(()) => Ok(file),
        Err(std::fs::TryLockError::WouldBlock) => {
            Err("another PCTwin move is already using the helper on this laptop".into())
        }
        Err(std::fs::TryLockError::Error(e)) => Err(format!("the lock: {e}")),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_second_helper_finds_the_lock_held_until_the_first_ends() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("helper.lock");
        let first = one_helper(&path).unwrap();
        assert!(one_helper(&path).is_err());
        drop(first);
        assert!(one_helper(&path).is_ok());
    }
}
