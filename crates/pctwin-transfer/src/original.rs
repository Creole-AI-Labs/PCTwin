//! Asking the old laptop whether it still has the original, unchanged, before undo removes the
//! copy on the new laptop.
//!
//! The old laptop is read-only here: it opens the file for reading only, looks at its size,
//! modified time and identity, and lets go. It looks only at files it sent itself (a path the new
//! laptop names is never opened), and an answer it cannot give is "cannot look", never a guess.

use std::collections::HashMap;
use std::fs::File;
use std::io;
use std::path::{Path, PathBuf};

use pctwin_journal::{FileId, PlannedWrite};
use pctwin_record::ItemId;

use crate::Stamp;

/// The most items in one request or answer; a longer one is refused when it is read.
pub const MAX_ORIGINALS_PER_REQUEST: usize = 1024;

/// What the old laptop sees now where an original was.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum OriginalNow {
    /// A regular file is there, with this size, modified time and identity.
    Present {
        size: u64,
        modified_ns: Option<i64>,
        file: Option<FileId>,
    },
    /// Nothing is there any more.
    Missing,
    /// The old laptop could not look (it never sent this item, the place is not an ordinary
    /// file, or reading failed). This confirms nothing.
    CannotLook,
}

/// Which file this open handle is, on its drive: the drive's number and the file's number on it.
#[cfg(unix)]
pub(crate) fn file_identity(file: &File) -> io::Result<FileId> {
    use std::os::unix::fs::MetadataExt;
    let meta = file.metadata()?;
    Ok(FileId {
        volume: meta.dev(),
        index: meta.ino(),
    })
}

/// Which file this open handle is, on its drive: the drive's number and the file's number on it.
#[cfg(windows)]
pub(crate) fn file_identity(file: &File) -> io::Result<FileId> {
    let info = winapi_util::file::information(file)?;
    Ok(FileId {
        volume: info.volume_serial_number(),
        index: info.file_index(),
    })
}

/// Which file this open handle is: not known on this kind of system.
#[cfg(not(any(unix, windows)))]
pub(crate) fn file_identity(_file: &File) -> io::Result<FileId> {
    Err(io::Error::from(io::ErrorKind::Unsupported))
}

/// Opens a file for reading only. Windows lets others read, write and delete it meanwhile (the
/// standard library's default share mode), so this holds nobody up; a link itself is opened, not
/// what it points to.
fn open_read_only(path: &Path) -> io::Result<File> {
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

/// One fresh, read-only look at one path. Nothing is written, no time is set, and the file is
/// not held open after this returns.
fn look(path: &Path) -> OriginalNow {
    // A link, folder or device is not what the scan saw, so it is never followed or opened.
    match std::fs::symlink_metadata(path) {
        Ok(meta) if meta.is_file() => {}
        Ok(_) => return OriginalNow::CannotLook,
        Err(e) if e.kind() == io::ErrorKind::NotFound => return OriginalNow::Missing,
        Err(_) => return OriginalNow::CannotLook,
    }
    let file = match open_read_only(path) {
        Ok(f) => f,
        Err(e) if e.kind() == io::ErrorKind::NotFound => return OriginalNow::Missing,
        Err(_) => return OriginalNow::CannotLook,
    };
    // The handle itself must be a regular file (the name may have changed since the look above).
    let Ok(meta) = file.metadata() else {
        return OriginalNow::CannotLook;
    };
    if !meta.is_file() {
        return OriginalNow::CannotLook;
    }
    let stamp = Stamp::of(&meta);
    OriginalNow::Present {
        size: stamp.size,
        modified_ns: stamp.modified_ns,
        file: file_identity(&file).ok(),
    }
}

/// Old laptop: answers for `items` from its own list of what it sent (item to source path).
/// An item it did not send is `CannotLook`: a path the new laptop names is never looked at.
/// The answers come in the order asked.
pub fn answer_originals(
    sent: &HashMap<ItemId, PathBuf>,
    items: &[ItemId],
) -> Vec<(ItemId, OriginalNow)> {
    items
        .iter()
        .map(|item| {
            let now = match sent.get(item) {
                Some(path) => look(path),
                None => OriginalNow::CannotLook,
            };
            (*item, now)
        })
        .collect()
}

/// New laptop: is the original still exactly the file that was copied? True only if it is
/// present, its identity is known on both sides and equal, and its size and modified time are
/// known and equal to what was read at move time. Anything missing or different is false.
pub fn original_unchanged(write: &PlannedWrite, now: &OriginalNow) -> bool {
    let OriginalNow::Present {
        size,
        modified_ns,
        file,
    } = now
    else {
        return false;
    };
    let (Some(recorded), Some(seen)) = (write.source_file, file) else {
        return false;
    };
    let (Some(recorded_time), Some(seen_time)) = (write.source_modified_ns, modified_ns) else {
        return false;
    };
    recorded == *seen && *size == write.size && recorded_time == *seen_time
}
