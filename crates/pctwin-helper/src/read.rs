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

use std::fs::{File, Metadata};
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

/// Windows: the error for a link or junction on the way to the file (ERROR_CANT_RESOLVE_FILENAME).
#[cfg(windows)]
const LINK_ON_THE_WAY: i32 = 1921;

/// Windows: opens without following a link at that name, sharing it for reading only, so while
/// the handle is held no one can write it, rename it, delete it or turn it into a link. A folder
/// needs the backup flag to be opened at all (and gets no other right from it unless the backup
/// privilege is switched on, which the helper never does).
#[cfg(windows)]
fn open_held(path: &std::path::Path, folder: bool) -> io::Result<File> {
    use std::os::windows::fs::OpenOptionsExt;
    const FILE_SHARE_READ: u32 = 0x1;
    const FILE_FLAG_OPEN_REPARSE_POINT: u32 = 0x0020_0000;
    const FILE_FLAG_BACKUP_SEMANTICS: u32 = 0x0200_0000;
    let flags = if folder {
        FILE_FLAG_OPEN_REPARSE_POINT | FILE_FLAG_BACKUP_SEMANTICS
    } else {
        FILE_FLAG_OPEN_REPARSE_POINT
    };
    std::fs::OpenOptions::new()
        .read(true)
        .share_mode(FILE_SHARE_READ)
        .custom_flags(flags)
        .open(path)
}

/// Windows: opens every folder between the drive and the file, one at a time from the drive's
/// top, each refused if it is a link or junction, and holds them all. Read access shared for
/// reading only takes part in Windows' sharing rules, so while they are held no folder on the way
/// can be renamed, deleted or turned into a link: the path cannot be changed under the open that
/// follows. A cloud folder's placeholder is not a link (its tag names no other place), so it is
/// passed through like any folder.
#[cfg(windows)]
fn hold_folders(path: &std::path::Path) -> io::Result<Vec<File>> {
    use std::path::{Component, Prefix};
    let invalid = || io::Error::from(io::ErrorKind::InvalidInput);
    let mut parts = path.components();
    let mut at = match (parts.next(), parts.next()) {
        (Some(Component::Prefix(p)), Some(Component::RootDir))
            if matches!(p.kind(), Prefix::Disk(_) | Prefix::VerbatimDisk(_)) =>
        {
            PathBuf::from(p.as_os_str()).join(std::path::MAIN_SEPARATOR_STR)
        }
        _ => return Err(invalid()),
    };
    let names = parts
        .map(|c| match c {
            Component::Normal(n) => Ok(n),
            _ => Err(invalid()),
        })
        .collect::<io::Result<Vec<_>>>()?;
    let Some((_, folders)) = names.split_last() else {
        return Err(invalid());
    };
    let mut held = Vec::with_capacity(folders.len());
    for name in folders {
        at.push(name);
        let folder = open_held(&at, true)?;
        let meta = folder.metadata()?;
        if meta.file_type().is_symlink() || !meta.is_dir() {
            return Err(io::Error::from_raw_os_error(LINK_ON_THE_WAY));
        }
        held.push(folder);
    }
    Ok(held)
}

/// Windows: opens `path` (a full path on a drive) read-only, never through a link or junction on
/// the way or at the file, with the folders above held while it opens (see [`hold_folders`]).
/// Shared for reading only, so a program writing it makes the open fail: "in use".
#[cfg(windows)]
fn open(path: &std::path::Path) -> io::Result<File> {
    let held = hold_folders(path)?;
    let file = open_held(path, false);
    drop(held);
    file
}

/// Mac and Linux: opens `path` (absolute) one folder at a time from `/`, never through a link
/// anyone but the system's own account (root) made: a folder somewhere above the file swapped for
/// a link (to `/etc`, say) is refused, not followed. Links root made (a Mac's `/var`) are
/// followed, since nobody else can make or change them. The file itself is never a link, and is
/// opened without waiting (a pipe cannot hang the helper).
#[cfg(unix)]
fn open(path: &std::path::Path) -> io::Result<File> {
    use rustix::fs::{AtFlags, FileType, Mode, OFlags};
    use std::path::Component;
    let loop_err = || io::Error::from_raw_os_error(rustix::io::Errno::LOOP.raw_os_error());
    let parts: Vec<&std::ffi::OsStr> = path
        .components()
        .map(|c| match c {
            Component::RootDir => Ok(None),
            Component::Normal(n) => Ok(Some(n)),
            _ => Err(io::Error::from(io::ErrorKind::InvalidInput)),
        })
        .collect::<io::Result<Vec<_>>>()?
        .into_iter()
        .flatten()
        .collect();
    if !path.is_absolute() || parts.is_empty() {
        return Err(io::Error::from(io::ErrorKind::InvalidInput));
    }
    let dir_flags = OFlags::DIRECTORY | OFlags::RDONLY | OFlags::CLOEXEC;
    let mut dir = rustix::fs::open("/", dir_flags | OFlags::NOFOLLOW, Mode::empty())?;
    for (i, name) in parts.iter().enumerate() {
        let last = i + 1 == parts.len();
        let st = rustix::fs::statat(&dir, *name, AtFlags::SYMLINK_NOFOLLOW)?;
        let link = FileType::from_raw_mode(st.st_mode) == FileType::Symlink;
        if link && (last || st.st_uid != 0) {
            return Err(loop_err());
        }
        if last {
            let fd = rustix::fs::openat(
                &dir,
                *name,
                OFlags::RDONLY | OFlags::NOFOLLOW | OFlags::CLOEXEC | OFlags::NONBLOCK,
                Mode::empty(),
            )?;
            return Ok(File::from(fd));
        }
        // A folder that was not a link when looked at is opened refusing one now.
        let flags = if link {
            dir_flags
        } else {
            dir_flags | OFlags::NOFOLLOW
        };
        dir = rustix::fs::openat(&dir, *name, flags, Mode::empty())?;
    }
    Err(loop_err())
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
    #[cfg(windows)]
    if e.raw_os_error() == Some(LINK_ON_THE_WAY) {
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

#[cfg(all(test, unix))]
mod open_tests {
    use super::*;

    #[test]
    fn a_folder_above_the_file_swapped_for_a_link_is_refused() {
        let dir = tempfile::tempdir().unwrap();
        let real = dir.path().join("real");
        std::fs::create_dir(&real).unwrap();
        std::fs::write(real.join("a.txt"), b"hello").unwrap();
        // The plain path opens (the temp folder may sit under a link root made, as a Mac's /var).
        assert!(open(&real.join("a.txt")).is_ok());
        // A folder link made by this account (not root) anywhere above the file is refused.
        std::os::unix::fs::symlink(&real, dir.path().join("via")).unwrap();
        let e = open(&dir.path().join("via").join("a.txt")).unwrap_err();
        assert_eq!(refused_open(&e), Refusal::NotPlain);
        // And the file itself as a link.
        std::os::unix::fs::symlink(real.join("a.txt"), real.join("b.txt")).unwrap();
        assert!(open(&real.join("b.txt")).is_err());
        // A relative path is never opened.
        assert!(open(std::path::Path::new("real/a.txt")).is_err());
    }
}

#[cfg(all(test, windows))]
mod open_tests {
    use super::*;

    fn entry_for(path: &std::path::Path) -> Entry {
        let meta = std::fs::metadata(path).unwrap();
        Entry {
            path: path.to_path_buf(),
            size: meta.len(),
            modified_ns: nanos(&meta),
        }
    }

    /// A junction (a folder link anyone can make, no rights needed) made with the system's own
    /// `mklink`.
    fn junction(link: &std::path::Path, target: &std::path::Path) {
        let ok = std::process::Command::new("cmd")
            .args(["/c", "mklink", "/J"])
            .arg(link)
            .arg(target)
            .stdout(std::process::Stdio::null())
            .status()
            .unwrap()
            .success();
        assert!(ok, "mklink /J");
    }

    #[test]
    fn a_folder_above_the_file_swapped_for_a_junction_is_refused() {
        let dir = tempfile::tempdir().unwrap();
        let mine = dir.path().join("mine");
        let theirs = dir.path().join("theirs");
        std::fs::create_dir_all(mine.join("Docs")).unwrap();
        std::fs::create_dir_all(&theirs).unwrap();
        std::fs::write(mine.join("Docs").join("a.txt"), b"hello").unwrap();
        // What the scan saw.
        let entry = entry_for(&mine.join("Docs").join("a.txt"));
        // A file of the same size and time somewhere else, then the folder swapped for a
        // junction to it: the path now leads there.
        std::fs::write(theirs.join("a.txt"), b"HELLO").unwrap();
        let file = std::fs::File::options()
            .write(true)
            .open(theirs.join("a.txt"))
            .unwrap();
        file.set_modified(std::fs::metadata(&entry.path).unwrap().modified().unwrap())
            .unwrap();
        drop(file);
        std::fs::rename(mine.join("Docs"), mine.join("Docs-was")).unwrap();
        junction(&mine.join("Docs"), &theirs);
        assert_eq!(read_range(&entry, &mut None, 0, 5), Err(Refusal::NotPlain));
        // The plain path still reads.
        let plain = entry_for(&mine.join("Docs-was").join("a.txt"));
        assert_eq!(read_range(&plain, &mut None, 0, 5).unwrap(), b"hello");
        // A relative path is never opened.
        assert!(open(std::path::Path::new(r"mine\Docs-was\a.txt")).is_err());
    }

    /// While the file is being opened, no folder above it can be renamed or replaced.
    #[test]
    fn the_folders_above_are_held_while_the_file_is_opened() {
        let dir = tempfile::tempdir().unwrap();
        let docs = dir.path().join("Docs");
        std::fs::create_dir_all(docs.join("Trips")).unwrap();
        std::fs::write(docs.join("Trips").join("a.txt"), b"hello").unwrap();
        let held = hold_folders(&docs.join("Trips").join("a.txt")).unwrap();
        assert!(std::fs::rename(&docs, dir.path().join("Moved")).is_err());
        assert!(std::fs::rename(docs.join("Trips"), docs.join("Moved")).is_err());
        drop(held);
        assert!(std::fs::rename(docs.join("Trips"), docs.join("Moved")).is_ok());
    }
}
