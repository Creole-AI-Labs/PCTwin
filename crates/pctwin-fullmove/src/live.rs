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
//!   done, only after the system says nobody holds it. While it is read, an app started then
//!   cannot write to it, and may say the file is in use, until the read ends; the old laptop's
//!   screen says so. A database's own files beside it (its `-wal`, `-shm`, `-journal`, from the
//!   catalogue) are live files too, held while the database is read, so it is copied whole. A file still held is not read at all: it
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
    /// Its own files beside it in the move (a database's `-wal`, `-shm`, `-journal`): held while
    /// it is read.
    pub sidecars: Vec<PathBuf>,
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
/// only, and sharing it fully, so its app can still write while it is looked at.
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
    let files: Vec<(ItemId, PathBuf)> = files.into_iter().collect();
    let by_path: HashMap<&Path, ItemId> = files.iter().map(|(i, p)| (p.as_path(), *i)).collect();
    let mut found: Vec<LiveFile> = Vec::new();
    for (item, path) in files.iter().cloned() {
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
            // Its own files beside it that move too: live files of their own, held with it.
            let mut sidecars = Vec::new();
            for suffix in &kind.sidecars {
                let mut side = path.as_os_str().to_owned();
                side.push(suffix);
                let side = PathBuf::from(side);
                if let Some(&side_item) = by_path.get(side.as_path()) {
                    sidecars.push(side.clone());
                    found.push(LiveFile {
                        item: side_item,
                        name: side
                            .file_name()
                            .map(|n| n.to_string_lossy().into_owned())
                            .unwrap_or_default(),
                        path: side,
                        kind: kind.kind.clone(),
                        sidecars: Vec::new(),
                    });
                }
            }
            found.push(LiveFile {
                item,
                path,
                name,
                kind: kind.kind.clone(),
                sidecars,
            });
        }
    }
    // A sidecar named twice (its own ending is a kind's too) is kept once.
    let mut seen = std::collections::HashSet::new();
    found.retain(|f| seen.insert(f.item));
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
    sidecars: HashMap<PathBuf, Vec<PathBuf>>,
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
            sidecars: files
                .iter()
                .map(|f| (f.path.clone(), f.sidecars.clone()))
                .collect(),
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
        let open = |p: &Path| {
            std::fs::OpenOptions::new()
                .read(true)
                .share_mode(FILE_SHARE_READ)
                .custom_flags(0x0010_0000)
                .open(p)
        };
        let held =
            |e: &io::Error| matches!(e.raw_os_error(), Some(SHARING_VIOLATION | LOCK_VIOLATION));
        let file = match open(path) {
            Ok(f) => f,
            Err(e) if held(&e) => return Err(self.refuse(item, path, None)),
            Err(e) => return Err(e),
        };
        // Its own files beside it are held too while it is read; one held by its app means the
        // database is still open there, so it is not read at all.
        let mut guards = Vec::new();
        for side in self.sidecars.get(path).into_iter().flatten() {
            match open(side) {
                Ok(g) => guards.push(g),
                Err(e) if held(&e) => return Err(self.refuse(item, path, None)),
                Err(_) => {}
            }
        }
        Ok(Box::new(WithSidecars {
            file,
            _guards: guards,
        }))
    }

    #[cfg(not(windows))]
    fn open_live(&self, item: ItemId, path: &Path) -> io::Result<Box<dyn Source>> {
        // No way here to keep writers out while it is read: it is read only when the system says
        // nobody holds it or its own files beside it (a database's journal held means it is still
        // open), and the send's own steady check refuses a copy that changed meanwhile.
        let mut asked = vec![path.to_path_buf()];
        asked.extend(self.sidecars.get(path).into_iter().flatten().cloned());
        match self.holders.holding(&asked) {
            Holding::Held { by } => Err(self.refuse(item, path, Some(by))),
            Holding::Free | Holding::CannotTell => self.inner.open(path),
        }
    }
}

/// A live file being read, with its own files beside it held so nothing writes to them meanwhile.
#[cfg(windows)]
struct WithSidecars {
    file: std::fs::File,
    _guards: Vec<std::fs::File>,
}

#[cfg(windows)]
impl Read for WithSidecars {
    fn read(&mut self, buf: &mut [u8]) -> io::Result<usize> {
        self.file.read(buf)
    }
}

#[cfg(windows)]
impl Seek for WithSidecars {
    fn seek(&mut self, pos: SeekFrom) -> io::Result<u64> {
        self.file.seek(pos)
    }
}

#[cfg(windows)]
impl Source for WithSidecars {
    fn stamp(&mut self) -> io::Result<pctwin_transfer::Stamp> {
        self.file.stamp()
    }

    fn identity(&mut self) -> io::Result<Option<pctwin_journal::FileId>> {
        self.file.identity()
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
