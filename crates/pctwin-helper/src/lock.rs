//! One helper per laptop: one mover's move at a time (Security Design part C, "serve a second
//! session"). The helper holds an exclusive lock on one file for as long as it runs; a second
//! helper finds it held and ends before it makes a channel. The system lets the lock go when the
//! helper ends, however it ends.

use std::fs::{File, OpenOptions};
use std::path::{Path, PathBuf};

/// Where the lock is: a folder no ordinary account can open anything in, so no one else can hold
/// the lock (even shared, for reading) and keep every move from starting (found by the
/// fresh-context review). On Windows, `Program Files\PCTwin\helper`, administrators-only from its
/// first moment (found by asking Windows, never the environment); the installer's own folder
/// lets every account read. On a Mac, PCTwin's root-owned folder in /Library/Application Support
/// (its /private/var/run is writable by the daemon group without the sticky bit).
pub fn lock_path() -> Result<PathBuf, String> {
    #[cfg(windows)]
    {
        pctwin_helper_os::admin_only_folder("helper")
            .map(|dir| dir.join("helper.lock"))
            .map_err(|e| format!("the lock's folder: {e}"))
    }
    #[cfg(target_os = "macos")]
    {
        Ok(PathBuf::from(
            "/Library/Application Support/PCTwin/helper.lock",
        ))
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
    let mut options = OpenOptions::new();
    options.create(true).truncate(false).write(true);
    // Never through a link planted at the lock's name, and never waiting on something that is
    // not a file planted there (a named pipe would hold the opening for ever).
    #[cfg(unix)]
    std::os::unix::fs::OpenOptionsExt::custom_flags(&mut options, libc_nofollow_nonblock());
    let file = options.open(path).map_err(|e| format!("the lock: {e}"))?;
    if !file
        .metadata()
        .map_err(|e| format!("the lock: {e}"))?
        .is_file()
    {
        return Err("the lock is not a plain file".into());
    }
    match file.try_lock() {
        Ok(()) => Ok(file),
        Err(std::fs::TryLockError::WouldBlock) => {
            Err("another PCTwin move is already using the helper on this laptop".into())
        }
        Err(std::fs::TryLockError::Error(e)) => Err(format!("the lock: {e}")),
    }
}

/// `O_NOFOLLOW | O_NONBLOCK` on this system.
#[cfg(unix)]
fn libc_nofollow_nonblock() -> i32 {
    (rustix::fs::OFlags::NOFOLLOW | rustix::fs::OFlags::NONBLOCK).bits() as i32
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

    /// A named pipe planted at the lock's name never holds the helper: it is refused at once.
    #[cfg(unix)]
    #[test]
    fn a_pipe_planted_at_the_lock_is_refused_without_waiting() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("helper.lock");
        rustix::fs::mknodat(
            rustix::fs::CWD,
            &path,
            rustix::fs::FileType::Fifo,
            rustix::fs::Mode::RUSR | rustix::fs::Mode::WUSR,
            0,
        )
        .unwrap();
        let (tx, rx) = std::sync::mpsc::channel();
        std::thread::spawn(move || {
            let _ = tx.send(one_helper(&path).is_err());
        });
        let refused = rx
            .recv_timeout(std::time::Duration::from_secs(10))
            .expect("the planted pipe held the helper");
        assert!(refused);
    }
}
