//! Opening an original on the old laptop, for the check undo asks for. The open lives here, with
//! the rest of the code that touches files, so the move code never uses the system's open flags.

use std::fs::File;
use std::io;
use std::path::Path;

/// Opens a file for reading only, never following a link, never waiting on a pipe and never
/// taking a terminal.
///
/// The open itself refuses a link at the name (`O_NOFOLLOW`), so one swapped in after a check by
/// name is refused here, not followed; a pipe or device swapped in cannot hold the caller up
/// (`O_NONBLOCK`; the caller then finds the handle is not a regular file and lets go); and a
/// terminal device swapped in cannot become this process's controlling terminal (`O_NOCTTY`). The
/// handle is close-on-exec. The flags come from `rustix`, because their numbers differ between
/// systems.
#[cfg(unix)]
pub fn open_original(path: &Path) -> io::Result<File> {
    use rustix::fs::{Mode, OFlags};
    let fd = rustix::fs::open(
        path,
        OFlags::RDONLY | OFlags::NOFOLLOW | OFlags::NONBLOCK | OFlags::NOCTTY | OFlags::CLOEXEC,
        Mode::empty(),
    )?;
    Ok(File::from(fd))
}

/// Opens a file for reading only. Windows lets others read, write and delete it meanwhile (the
/// standard library's default share mode), so this holds nobody up; a link itself is opened, not
/// what it points to, and an online-only file is not woken.
#[cfg(not(unix))]
pub fn open_original(path: &Path) -> io::Result<File> {
    let mut options = std::fs::OpenOptions::new();
    options.read(true);
    #[cfg(windows)]
    {
        use std::os::windows::fs::OpenOptionsExt;
        /// Opens a link itself instead of following it (and never wakes an online-only file).
        const FILE_FLAG_OPEN_REPARSE_POINT: u32 = 0x0020_0000;
        options.custom_flags(FILE_FLAG_OPEN_REPARSE_POINT);
    }
    options.open(path)
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Read;

    #[test]
    fn an_ordinary_file_opens_and_reads() {
        let dir = tempfile::tempdir().unwrap();
        let name = dir.path().join("f");
        std::fs::write(&name, b"hello").unwrap();
        let mut text = String::new();
        open_original(&name)
            .unwrap()
            .read_to_string(&mut text)
            .unwrap();
        assert_eq!(text, "hello");
    }

    #[test]
    fn a_file_that_is_not_there_is_not_found() {
        let dir = tempfile::tempdir().unwrap();
        let e = open_original(&dir.path().join("nope")).unwrap_err();
        assert_eq!(e.kind(), io::ErrorKind::NotFound);
    }

    #[test]
    fn the_handle_is_read_only() {
        use std::io::Write;
        let dir = tempfile::tempdir().unwrap();
        let name = dir.path().join("f");
        std::fs::write(&name, b"hello").unwrap();
        let mut file = open_original(&name).unwrap();
        assert!(file.write_all(b"x").is_err());
        assert_eq!(std::fs::read(&name).unwrap(), b"hello");
    }

    #[cfg(unix)]
    mod unix {
        use super::*;
        use std::os::unix::fs::symlink;
        use std::time::Duration;

        fn mkfifo(path: &Path) {
            // rustix has no mknod on macOS; the mkfifo tool is on every Unix.
            let status = std::process::Command::new("mkfifo")
                .arg(path)
                .status()
                .unwrap();
            assert!(status.success());
        }

        #[test]
        fn a_link_is_refused_not_followed() {
            let dir = tempfile::tempdir().unwrap();
            let target = dir.path().join("t");
            std::fs::write(&target, b"x").unwrap();
            let link = dir.path().join("l");
            symlink(&target, &link).unwrap();
            assert!(open_original(&link).is_err());
            assert!(open_original(&target).is_ok());
        }

        #[test]
        fn a_link_to_nowhere_is_refused_too() {
            let dir = tempfile::tempdir().unwrap();
            let link = dir.path().join("l");
            symlink(dir.path().join("gone"), &link).unwrap();
            assert!(open_original(&link).is_err());
        }

        #[test]
        fn opening_a_pipe_does_not_block() {
            let dir = tempfile::tempdir().unwrap();
            let name = dir.path().join("pipe");
            mkfifo(&name);
            let (tx, rx) = std::sync::mpsc::channel();
            std::thread::spawn(move || {
                let _ = tx.send(open_original(&name).map(|f| f.metadata().unwrap().is_file()));
            });
            let opened = rx
                .recv_timeout(Duration::from_secs(20))
                .expect("opening a pipe waited for a writer");
            // The open may be allowed, but what it hands back is not a regular file.
            assert_eq!(opened.ok(), Some(false));
        }

        #[test]
        fn the_handle_is_close_on_exec() {
            let dir = tempfile::tempdir().unwrap();
            let name = dir.path().join("f");
            std::fs::write(&name, b"x").unwrap();
            let file = open_original(&name).unwrap();
            let flags = rustix::io::fcntl_getfd(&file).unwrap();
            assert!(flags.contains(rustix::io::FdFlags::CLOEXEC));
        }
    }
}
