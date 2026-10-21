//! Reading another person's file for the move: read-only, never through a link, never recalling
//! a file stored online only, never a file that changed since the scan, and (on Windows) never
//! one another program has open for writing, so a copy is never torn.
//!
//! - Before opening: the file's own entry must be a plain file (not a link or a special entry),
//!   not online only, and the size and changed time the scan saw.
//! - Opening: read-only, without following a link (Windows: the reparse point itself is opened;
//!   Mac and Linux: `O_NOFOLLOW`). Windows shares the file for reading only, so a program
//!   writing it makes the open fail: "in use".
//! - After opening, on the handle: still a plain file of the scanned size and time, and the very
//!   file read before (its identity is pinned at the first read), so a file swapped in between
//!   reads is caught.

use std::fs::{File, Metadata, OpenOptions};
use std::io::{self, Read, Seek, SeekFrom};
use std::path::PathBuf;
use std::time::UNIX_EPOCH;

use pctwin_gate::FileId;

use crate::protocol::{MAX_READ, Refusal};

/// One file in the helper's plan: where it is, and what the scan saw.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Entry {
    pub path: PathBuf,
    pub size: u64,
    /// When it was last changed at the scan (`None`: the system did not say).
    pub modified_ns: Option<i64>,
}

fn nanos(meta: &Metadata) -> Option<i64> {
    let d = meta.modified().ok()?.duration_since(UNIX_EPOCH).ok()?;
    i64::try_from(d.as_nanos()).ok()
}

/// The refusal for an entry that is not exactly what the scan saw, if any.
fn check(meta: &Metadata, entry: &Entry) -> Result<(), Refusal> {
    if meta.file_type().is_symlink() || !meta.is_file() {
        return Err(Refusal::NotPlain);
    }
    if pctwin_scan::is_cloud_only(meta) {
        return Err(Refusal::OnlineOnly);
    }
    let time_differs = matches!((entry.modified_ns, nanos(meta)), (Some(a), Some(b)) if a != b);
    if meta.len() != entry.size || time_differs {
        return Err(Refusal::Changed);
    }
    Ok(())
}

#[cfg(windows)]
fn open(path: &std::path::Path) -> io::Result<File> {
    use std::os::windows::fs::OpenOptionsExt;
    const FILE_SHARE_READ: u32 = 0x1;
    const FILE_FLAG_OPEN_REPARSE_POINT: u32 = 0x0020_0000;
    OpenOptions::new()
        .read(true)
        .share_mode(FILE_SHARE_READ)
        .custom_flags(FILE_FLAG_OPEN_REPARSE_POINT)
        .open(path)
}

#[cfg(unix)]
fn open(path: &std::path::Path) -> io::Result<File> {
    use std::os::unix::fs::OpenOptionsExt;
    OpenOptions::new()
        .read(true)
        .custom_flags(rustix::fs::OFlags::NOFOLLOW.bits() as i32)
        .open(path)
}

/// The refusal for a failed open.
fn refused_open(e: &io::Error) -> Refusal {
    #[cfg(windows)]
    const IN_USE: [i32; 2] = [32, 33]; // ERROR_SHARING_VIOLATION, ERROR_LOCK_VIOLATION
    #[cfg(unix)]
    const IN_USE: [i32; 0] = [];
    #[cfg(unix)]
    if e.raw_os_error() == Some(rustix::io::Errno::LOOP.raw_os_error()) {
        return Refusal::NotPlain;
    }
    match e.kind() {
        io::ErrorKind::NotFound => Refusal::Gone,
        _ if e.raw_os_error().is_some_and(|c| IN_USE.contains(&c)) => Refusal::InUse,
        _ => Refusal::Unreadable,
    }
}

/// Reads `len` bytes from `offset` of `entry`'s file. `pinned` is the file's identity from the
/// first read (set by it), so every later read is of the very same file.
pub fn read_range(
    entry: &Entry,
    pinned: &mut Option<FileId>,
    offset: u64,
    len: u32,
) -> Result<Vec<u8>, Refusal> {
    if len > MAX_READ {
        return Err(Refusal::TooMuch);
    }
    if offset
        .checked_add(u64::from(len))
        .is_none_or(|end| end > entry.size)
    {
        return Err(Refusal::PastTheEnd);
    }
    let before = std::fs::symlink_metadata(&entry.path).map_err(|e| match e.kind() {
        io::ErrorKind::NotFound => Refusal::Gone,
        _ => Refusal::Unreadable,
    })?;
    check(&before, entry)?;
    let mut file = open(&entry.path).map_err(|e| refused_open(&e))?;
    // Judged again on the handle: the entry may have been swapped between the look and the open.
    let meta = file.metadata().map_err(|_| Refusal::Unreadable)?;
    check(&meta, entry)?;
    let id = pctwin_gate::file_identity(&file).map_err(|_| Refusal::Unreadable)?;
    match pinned {
        Some(p) if *p != id => return Err(Refusal::Changed),
        Some(_) => {}
        None => *pinned = Some(id),
    }
    file.seek(SeekFrom::Start(offset))
        .map_err(|_| Refusal::Unreadable)?;
    let mut out = vec![0u8; len as usize];
    file.read_exact(&mut out).map_err(|_| Refusal::Unreadable)?;
    // A file changed while it was read is never handed out.
    let after = file.metadata().map_err(|_| Refusal::Unreadable)?;
    check(&after, entry)?;
    Ok(out)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn entry_for(path: &std::path::Path) -> Entry {
        let meta = std::fs::metadata(path).unwrap();
        Entry {
            path: path.to_path_buf(),
            size: meta.len(),
            modified_ns: nanos(&meta),
        }
    }

    #[test]
    fn a_file_as_scanned_is_read_and_its_identity_pinned() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("a.txt");
        std::fs::write(&path, b"hello world").unwrap();
        let entry = entry_for(&path);
        let mut pinned = None;
        assert_eq!(read_range(&entry, &mut pinned, 6, 5).unwrap(), b"world");
        assert!(pinned.is_some());
        assert_eq!(read_range(&entry, &mut pinned, 0, 5).unwrap(), b"hello");
    }

    #[test]
    fn past_the_end_too_much_and_gone_are_refused() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("a.txt");
        std::fs::write(&path, b"hello").unwrap();
        let entry = entry_for(&path);
        let mut pinned = None;
        assert_eq!(
            read_range(&entry, &mut pinned, 1, 5),
            Err(Refusal::PastTheEnd)
        );
        assert_eq!(
            read_range(&entry, &mut pinned, u64::MAX, 1),
            Err(Refusal::PastTheEnd)
        );
        assert_eq!(
            read_range(&entry, &mut pinned, 0, MAX_READ + 1),
            Err(Refusal::TooMuch)
        );
        std::fs::remove_file(&path).unwrap();
        assert_eq!(read_range(&entry, &mut pinned, 0, 1), Err(Refusal::Gone));
    }

    #[test]
    fn a_file_changed_since_the_scan_is_not_read() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("a.txt");
        std::fs::write(&path, b"hello").unwrap();
        let entry = entry_for(&path);
        std::fs::write(&path, b"hello, longer").unwrap();
        assert_eq!(read_range(&entry, &mut None, 0, 1), Err(Refusal::Changed));
    }

    #[test]
    fn another_file_put_in_its_place_is_caught() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("a.txt");
        std::fs::write(&path, b"hello").unwrap();
        let entry = entry_for(&path);
        let mut pinned = None;
        read_range(&entry, &mut pinned, 0, 1).unwrap();
        // Same size, same changed time, another file.
        let other = dir.path().join("b.txt");
        std::fs::write(&other, b"HELLO").unwrap();
        let when = std::fs::metadata(&path).unwrap().modified().unwrap();
        std::fs::File::options()
            .write(true)
            .open(&other)
            .unwrap()
            .set_modified(when)
            .unwrap();
        std::fs::remove_file(&path).unwrap();
        std::fs::rename(&other, &path).unwrap();
        assert_eq!(read_range(&entry, &mut pinned, 0, 1), Err(Refusal::Changed));
    }

    #[test]
    fn a_link_is_never_followed() {
        let dir = tempfile::tempdir().unwrap();
        let target = dir.path().join("secret.txt");
        std::fs::write(&target, b"secret").unwrap();
        let link = dir.path().join("link.txt");
        #[cfg(unix)]
        let made = std::os::unix::fs::symlink(&target, &link).is_ok();
        #[cfg(windows)]
        let made = std::os::windows::fs::symlink_file(&target, &link).is_ok();
        if !made {
            // Windows without the right to make links (not in developer mode): nothing to test.
            return;
        }
        let entry = Entry {
            path: link,
            size: 6,
            modified_ns: None,
        };
        assert_eq!(read_range(&entry, &mut None, 0, 1), Err(Refusal::NotPlain));
    }

    /// Windows: a file another program has open for writing is "in use", never torn.
    #[cfg(windows)]
    #[test]
    fn a_file_open_for_writing_elsewhere_is_in_use() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("a.txt");
        std::fs::write(&path, b"hello").unwrap();
        let entry = entry_for(&path);
        let writer = std::fs::OpenOptions::new().write(true).open(&path).unwrap();
        assert_eq!(read_range(&entry, &mut None, 0, 1), Err(Refusal::InUse));
        drop(writer);
        assert!(read_range(&entry, &mut None, 0, 1).is_ok());
    }
}
