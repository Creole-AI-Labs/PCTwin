//! Files in the person's own folders that are live while their app runs (Task List 2.5, decided
//! 29 October 2026), on the old laptop:
//!
//! - [`find_live`]: which of the files that move are such a store, by kind from the catalogue
//!   ([`pctwin_special::live`]): its ending first, then its first and last bytes. A file stored
//!   online only is never opened (opening it would download it).
//! - [`until_closed`]: before the send, the files an app holds open for writing now (the system
//!   says, [`Holders`]); the person is asked to close only the apps holding them, then answers Try
//!   again or Go on without them. PCTwin never closes an app itself. Where the system cannot tell
//!   (a Mac today), nothing is asked and the send's own steady check stands: a file that changes
//!   while it is read is not sent.
//! - [`LiveOpener`]: during the send, each live file is opened so that no program can start
//!   writing to it while it is read (Windows: shared for reading only), or, where that cannot be
//!   done, only after the system says nobody holds it. A file still held is not read at all: it
//!   ends "another program had it open" ([`pctwin_transfer::InUse`]), which the report shows as
//!   needs-action, never as copied; the apps holding it are kept for the old laptop's report.

use std::collections::HashMap;
use std::io::{self, Read, Seek, SeekFrom};
use std::path::{Path, PathBuf};
use std::sync::{Arc, Mutex};

use pctwin_record::ItemId;
use pctwin_special::capture::{Holders, Holding};
use pctwin_special::live::LiveKinds;
use pctwin_transfer::{InUse, OnlineOnly, Opener, Source};

/// One file of the move that is live while its app runs.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct LiveFile {
    pub item: ItemId,
    pub path: PathBuf,
    /// Its name, as the person knows it.
    pub name: String,
    /// Its kind, from the catalogue (`mail-store`, `database`, ...).
    pub kind: String,
}

/// A live file an app holds open for writing now.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct HeldFile {
    pub file: LiveFile,
    /// The apps holding it, by the names the system shows; empty when the system cannot name
    /// them.
    pub by: Vec<String>,
}

/// Opens `path` to look at its first and last bytes, never downloading a file stored online
/// only, and sharing it fully, so its app is never disturbed.
fn head_and_tail(path: &Path) -> io::Result<(Vec<u8>, Vec<u8>, u64)> {
    let meta = std::fs::symlink_metadata(path)?;
    if !meta.is_file() || pctwin_types::cloud::is_cloud_only(&meta) {
        return Err(io::Error::other(OnlineOnly));
    }
    let mut options = std::fs::OpenOptions::new();
    options.read(true);
    #[cfg(windows)]
    {
        use std::os::windows::fs::OpenOptionsExt;
        // FILE_FLAG_OPEN_NO_RECALL: a file that went online only since is not brought back.
        options.custom_flags(0x0010_0000);
    }
    let mut f = options.open(path)?;
    let len = f.metadata()?.len();
    let mut head = Vec::with_capacity(pctwin_special::sniff::HEAD);
    (&mut f)
        .take(pctwin_special::sniff::HEAD as u64)
        .read_to_end(&mut head)?;
    let mut tail = Vec::new();
    let tail_len = pctwin_special::sniff::TAIL as u64;
    if len > tail_len {
        f.seek(SeekFrom::Start(len - tail_len))?;
        f.take(tail_len).read_to_end(&mut tail)?;
    }
    Ok((head, tail, len))
}

/// The files among `files` (each item with where it is on this laptop) that are live while their
/// app runs, by `kinds`. A file that cannot be read is not one here (the send says why it did
/// not move).
pub fn find_live(
    files: impl IntoIterator<Item = (ItemId, PathBuf)>,
    kinds: &LiveKinds,
) -> Vec<LiveFile> {
    let mut found = Vec::new();
    for (item, path) in files {
        let Some(name) = path
            .file_name()
            .and_then(|n| n.to_str())
            .map(str::to_string)
        else {
            continue;
        };
        if !kinds.may_be(&name) {
            continue;
        }
        let Ok((head, tail, len)) = head_and_tail(&path) else {
            continue;
        };
        if let Some(kind) = kinds.kind_of(&name, &head, &tail, len) {
            found.push(LiveFile {
                item,
                path,
                name,
                kind: kind.kind.clone(),
            });
        }
    }
    found
}

/// The live files an app holds open for writing now. One the system cannot tell about is not
/// counted (the send's steady check stands).
pub fn held_now(files: &[LiveFile], holders: &dyn Holders) -> Vec<HeldFile> {
    files
        .iter()
        .filter_map(|f| match holders.holding(std::slice::from_ref(&f.path)) {
            Holding::Held { by } => Some(HeldFile {
                file: f.clone(),
                by,
            }),
            Holding::Free | Holding::CannotTell => None,
        })
        .collect()
}

/// Before the send: while an app holds any of `files` open, asks the person (`ask`, with the
/// files held and who holds them) to close it; `ask` answers true for Try again, false for Go on
/// without them. Returns the files still held when the person went on (none once all are closed).
/// PCTwin never closes an app.
pub fn until_closed(
    files: &[LiveFile],
    holders: &dyn Holders,
    ask: &mut dyn FnMut(&[HeldFile]) -> bool,
) -> Vec<HeldFile> {
    loop {
        let held = held_now(files, holders);
        if held.is_empty() || !ask(&held) {
            return held;
        }
    }
}

/// The apps the person is asked to close for `held`: each app once, in the order found.
pub fn apps_to_close(held: &[HeldFile]) -> Vec<String> {
    let mut apps: Vec<String> = Vec::new();
    for h in held {
        for a in &h.by {
            if !apps.contains(a) {
                apps.push(a.clone());
            }
        }
    }
    apps
}

/// Reads the move's files through `inner`, except each live file, which is read only while no
/// program can write to it (see the module's words).
pub struct LiveOpener {
    inner: Arc<dyn Opener>,
    live: HashMap<PathBuf, ItemId>,
    holders: Arc<dyn Holders + Send + Sync>,
    held: Mutex<HashMap<ItemId, Vec<String>>>,
}

impl LiveOpener {
    pub fn new(
        inner: Arc<dyn Opener>,
        files: &[LiveFile],
        holders: Arc<dyn Holders + Send + Sync>,
    ) -> Self {
        Self {
            inner,
            live: files.iter().map(|f| (f.path.clone(), f.item)).collect(),
            holders,
            held: Mutex::new(HashMap::new()),
        }
    }

    /// The live files that were not read because an app held them, with the apps (by the
    /// system's names; empty when it could not name them).
    pub fn held(&self) -> HashMap<ItemId, Vec<String>> {
        self.held
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner)
            .clone()
    }

    fn refuse(&self, item: ItemId, path: &Path, by: Option<Vec<String>>) -> io::Error {
        let by = by.unwrap_or_else(|| match self.holders.holding(&[path.to_path_buf()]) {
            Holding::Held { by } => by,
            Holding::Free | Holding::CannotTell => Vec::new(),
        });
        self.held
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner)
            .insert(item, by);
        io::Error::other(InUse)
    }

    #[cfg(windows)]
    fn open_live(&self, item: ItemId, path: &Path) -> io::Result<Box<dyn Source>> {
        use std::os::windows::fs::OpenOptionsExt;
        const FILE_SHARE_READ: u32 = 1;
        const SHARING_VIOLATION: i32 = 32;
        const LOCK_VIOLATION: i32 = 33;
        if std::fs::symlink_metadata(path).is_ok_and(|m| pctwin_types::cloud::is_cloud_only(&m)) {
            return Err(io::Error::other(OnlineOnly));
        }
        // Shared for reading only: it fails while anyone has it open for writing, and while it
        // is open nobody can start writing to it, so the copy is whole.
        match std::fs::OpenOptions::new()
            .read(true)
            .share_mode(FILE_SHARE_READ)
            .custom_flags(0x0010_0000)
            .open(path)
        {
            Ok(f) => Ok(Box::new(f)),
            Err(e) if matches!(e.raw_os_error(), Some(SHARING_VIOLATION | LOCK_VIOLATION)) => {
                Err(self.refuse(item, path, None))
            }
            Err(e) => Err(e),
        }
    }

    #[cfg(not(windows))]
    fn open_live(&self, item: ItemId, path: &Path) -> io::Result<Box<dyn Source>> {
        // No way here to keep writers out while it is read: it is read only when the system says
        // nobody holds it, and the send's own steady check refuses a copy that changed meanwhile.
        match self.holders.holding(&[path.to_path_buf()]) {
            Holding::Held { by } => Err(self.refuse(item, path, Some(by))),
            Holding::Free | Holding::CannotTell => self.inner.open(path),
        }
    }
}

impl Opener for LiveOpener {
    fn open(&self, path: &Path) -> io::Result<Box<dyn Source>> {
        match self.live.get(path) {
            Some(item) => self.open_live(*item, path),
            None => self.inner.open(path),
        }
    }
}
