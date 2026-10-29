//! Reading the old laptop without ever waiting on it (the full-move bench, 28 October 2026: a
//! plan of a real home folder blocked on one read for 15 minutes).
//!
//! - **Look before opening** ([`kind_of`]): what an entry is comes from its attributes, read
//!   without opening it. Anything that is not a plain local file or folder is never opened or
//!   listed: a file kept online by a cloud app (offline, recall-on-open, recall-on-data-access:
//!   opening or listing it asks the cloud app to download it), any reparse point (links,
//!   junctions, socket files, cloud placeholders), pipes, sockets and devices.
//! - **Open so nothing is fetched** ([`open`]): on Windows with `FILE_FLAG_OPEN_NO_RECALL` (the
//!   system must not bring the data back from remote storage) and without following a reparse
//!   point; sharing reading, writing and deleting with everyone, so the open never waits for
//!   another program.
//! - **No read can stall the plan** ([`bounded`]): every read runs on a worker thread and is
//!   given [`READ_LIMIT`]; one that does not answer in time is abandoned (its worker is left to
//!   finish on its own, a new one takes over) and the file is reported "did not answer in time:
//!   not fully checked".

use std::path::Path;
use std::sync::mpsc::{self, Sender};
use std::time::Duration;

/// How long one read of the old laptop may take.
pub const READ_LIMIT: Duration = Duration::from_secs(10);

/// What an entry is, from its attributes alone.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Kind {
    File,
    Folder,
    /// Kept online by a cloud app, or offline: reading it would download it.
    OnlineOnly,
    /// A link, junction or other reparse point.
    Link,
    /// A pipe, socket, device or anything else that is not a file.
    NotAFile,
}

#[cfg(windows)]
const REPARSE_POINT: u32 = 0x0000_0400;

/// The kind of the entry with metadata `m` (from `symlink_metadata`, which opens nothing).
pub fn kind_of(m: &std::fs::Metadata) -> Kind {
    // Every system's mark, Windows and macOS alike (a dataless Mac file was opened before, 29
    // October 2026).
    if pctwin_types::cloud::is_cloud_only(m) {
        return Kind::OnlineOnly;
    }
    #[cfg(windows)]
    {
        use std::os::windows::fs::MetadataExt;
        let a = m.file_attributes();
        if a & REPARSE_POINT != 0 {
            return Kind::Link;
        }
    }
    let t = m.file_type();
    if t.is_symlink() {
        Kind::Link
    } else if t.is_dir() {
        Kind::Folder
    } else if t.is_file() {
        Kind::File
    } else {
        Kind::NotAFile
    }
}

/// Opens a plain file for reading without recalling it from remote storage, without following a
/// reparse point, and without ever waiting for another program.
pub fn open(path: &Path) -> std::io::Result<std::fs::File> {
    let mut o = std::fs::OpenOptions::new();
    o.read(true);
    #[cfg(windows)]
    {
        use std::os::windows::fs::OpenOptionsExt;
        const SHARE_ALL: u32 = 0x1 | 0x2 | 0x4;
        const OPEN_NO_RECALL: u32 = 0x0010_0000;
        const OPEN_REPARSE_POINT: u32 = 0x0020_0000;
        o.share_mode(SHARE_ALL)
            .custom_flags(OPEN_NO_RECALL | OPEN_REPARSE_POINT);
    }
    #[cfg(unix)]
    {
        use std::os::unix::fs::OpenOptionsExt;
        // Never follow a link, and never wait on a pipe or device.
        o.custom_flags(libc::O_NOFOLLOW | libc::O_NONBLOCK);
    }
    o.open(path)
}

/// Reads a whole plain file of at most `max` bytes (see [`open`]).
pub fn read_all(path: &Path, max: u64) -> std::io::Result<Vec<u8>> {
    use std::io::Read;
    let f = open(path)?;
    let mut out = Vec::new();
    f.take(max + 1).read_to_end(&mut out)?;
    if out.len() as u64 > max {
        return Err(std::io::Error::other("too large"));
    }
    Ok(out)
}

type Job = Box<dyn FnOnce() + Send>;

thread_local! {
    /// This thread's own reader: a read stuck on it delays no other thread's reads. Replaced when
    /// a read does not answer.
    static WORKER: std::cell::RefCell<Option<Sender<Job>>> = const { std::cell::RefCell::new(None) };
}

fn worker() -> Option<Sender<Job>> {
    WORKER.with(|w| {
        let mut w = w.borrow_mut();
        if w.is_none() {
            let (tx, rx) = mpsc::channel::<Job>();
            std::thread::Builder::new()
                .name("pctwin-special-reader".into())
                .spawn(move || {
                    while let Ok(job) = rx.recv() {
                        job();
                    }
                })
                .ok()?;
            *w = Some(tx);
        }
        w.clone()
    })
}

/// Abandons this thread's reader (stuck on a read): its next read gets a new one. The stuck one
/// ends by itself once its read returns.
fn abandon() {
    WORKER.with(|w| *w.borrow_mut() = None);
}

/// Runs `f` (a read of the old laptop) with time limit `limit`; `None` when it did not answer in
/// time (it is then abandoned, never waited for).
pub fn bounded<T: Send + 'static>(
    limit: Duration,
    f: impl FnOnce() -> T + Send + 'static,
) -> Option<T> {
    let (tx, rx) = mpsc::channel();
    let job: Job = Box::new(move || {
        let _ = tx.send(f());
    });
    let w = worker()?;
    if w.send(job).is_err() {
        abandon();
        return None;
    }
    match rx.recv_timeout(limit) {
        Ok(v) => Some(v),
        Err(_) => {
            abandon();
            None
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_read_that_does_not_answer_is_abandoned_and_the_next_one_works() {
        let started = std::time::Instant::now();
        let stuck = bounded(Duration::from_millis(200), || {
            std::thread::sleep(Duration::from_secs(30));
            1
        });
        assert_eq!(stuck, None);
        assert!(started.elapsed() < Duration::from_secs(5));
        assert_eq!(bounded(Duration::from_secs(5), || 2), Some(2));
        assert_eq!(bounded(Duration::from_secs(5), || 3), Some(3));
    }

    #[test]
    fn plain_files_and_folders_are_told_apart_without_opening_them() {
        let dir = tempfile::tempdir().unwrap();
        let f = dir.path().join("f");
        std::fs::write(&f, b"x").unwrap();
        assert_eq!(kind_of(&std::fs::symlink_metadata(&f).unwrap()), Kind::File);
        assert_eq!(
            kind_of(&std::fs::symlink_metadata(dir.path()).unwrap()),
            Kind::Folder
        );
        assert_eq!(read_all(&f, 1).unwrap(), b"x");
        assert!(read_all(&f, 0).is_err());
    }
}
