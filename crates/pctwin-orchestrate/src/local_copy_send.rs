//! Keeping local copies during the move itself (Task List 2.6, cloud placeholder hydration with
//! consent; wired into the real move 29 October 2026).
//!
//! The move sends its files one by one, in its own order, through an opener. [`KeepOpener`] is
//! that opener for the online-only files a person chose to keep a copy of: when the send opens
//! one, it makes [`KeepLocal`]'s checks for that one file, brings it down, checks it really came
//! down at the approved size and holds no account state, and only then hands it to the send.
//! Everything else is read through the move's own opener, unchanged.
//!
//! - **Space**: a file comes down only if the old laptop keeps [`KEEP_FREE_OLD`] free after it.
//!   When there is no room, files brought down earlier whose reading has finished are made
//!   online only again first; a file that still does not fit stays safe in its cloud.
//! - **The connection**: on one that may charge for data (or one PCTwin cannot tell), nothing
//!   more comes down unless the person allowed that too.
//! - **Put back**: every file brought down is made online only again: when space is needed, and
//!   all the rest when the send ends ([`KeepOpener::finish`]). What was downloaded and what the
//!   cloud app has not made online only again are in the [`Report`].
//! - **Honest**: a file that could not be brought down, or was not what was approved, is never
//!   read: the send is told it is online only ([`pctwin_transfer::OnlineOnly`]), so it ends safe
//!   in its cloud (needs action), never copied.
//!
//! - **Durable**: with [`KeepOpener::recorded`], the list of files this move may make local, each
//!   with its state before (online only), is written to PCTwin's own folder and flushed before
//!   the first one comes down ([`KeptList`]); each is marked before it comes down and again once
//!   it is online only again. A move that stops part way (the laptop turned off, PCTwin closed)
//!   leaves the list, and the next start of PCTwin, or of a move, puts back whatever it still
//!   names ([`put_back_listed`]), then removes it: the old laptop ends as it was.
//!
//! On Windows and, for what macOS itself keeps in a cloud (iCloud Drive), on a Mac
//! ([`crate::local_copy::SyncedFolderFiles`]); elsewhere the choice is not offered.

use std::collections::{BTreeSet, HashMap};
use std::io::{self, Read, Seek, SeekFrom};
use std::path::{Path, PathBuf};
use std::sync::{Arc, Mutex, MutexGuard};

use pctwin_record::ItemId;
use pctwin_transfer::{OnlineOnly, Opener, Source, Stamp};
use serde::{Deserialize, Serialize};

use crate::local_copy::{
    CloudFiles, FileReport, HEAD, KEEP_FREE_OLD, KeepLocal, Network, Report, Stopped, Wanted,
    WhyNot,
};

/// One online-only file the person chose to keep a copy of, with the cloud app it is in (an index
/// into the clouds given to [`KeepOpener::new`]).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct KeepFile {
    pub wanted: Wanted,
    pub cloud: usize,
}

struct State {
    report: Report,
    /// Files brought down and not yet put back.
    down: BTreeSet<ItemId>,
    /// Readers of each file still open.
    readers: HashMap<ItemId, usize>,
    /// The durable list, when the move keeps one.
    list: Option<KeptList>,
}

/// A file's state before PCTwin made it local.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum Before {
    OnlineOnly,
}

/// One file of the durable list.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ListedFile {
    pub path: PathBuf,
    /// The cloud folder it is in (nothing outside it is ever touched when it is put back).
    pub root: PathBuf,
    pub before: Before,
    /// PCTwin is bringing it down, or brought it down and has not seen it online only again.
    pub down: bool,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
struct ListFile {
    version: u32,
    files: Vec<ListedFile>,
}

const LIST_VERSION: u32 = 1;
/// The most files a list may name when it is read back (more than any move makes local).
const MOST_LISTED: usize = 1_000_000;

/// Where the durable list is kept: PCTwin's own data folder, on the laptop's own drive. This crate
/// never writes a file itself (its clippy fence); the app gives the store.
pub trait ListStore: Send {
    /// The list as last written, or `None` when there is none.
    fn read(&self) -> io::Result<Option<Vec<u8>>>;
    /// Writes the whole list in place of the old one, flushed to the disk before it returns, so
    /// a list is never half written.
    fn write(&self, bytes: &[u8]) -> io::Result<()>;
    /// Removes the list (none there is fine).
    fn remove(&self) -> io::Result<()>;
}

fn write_list(store: &dyn ListStore, files: &[ListedFile]) -> io::Result<()> {
    let body = serde_json::to_vec(&ListFile {
        version: LIST_VERSION,
        files: files.to_vec(),
    })
    .map_err(io::Error::other)?;
    store.write(&body)
}

/// The durable list of the files a move may make local on the old laptop (see the module's
/// words).
pub struct KeptList {
    store: Box<dyn ListStore>,
    files: Vec<ListedFile>,
}

impl KeptList {
    /// Writes the list of `files` (each with `roots[file.cloud]`, its cloud folder) to `store`,
    /// flushed, before anything comes down. A list already there (an earlier move's, not yet put
    /// back) is never overwritten: put it back first ([`put_back_listed`]).
    pub fn create(
        store: Box<dyn ListStore>,
        files: &[KeepFile],
        roots: &[PathBuf],
    ) -> io::Result<Self> {
        if store.read()?.is_some() {
            return Err(io::Error::new(
                io::ErrorKind::AlreadyExists,
                "an earlier list is still there",
            ));
        }
        let listed: Vec<ListedFile> = files
            .iter()
            .map(|f| {
                Ok(ListedFile {
                    path: f.wanted.path.clone(),
                    root: roots
                        .get(f.cloud)
                        .cloned()
                        .ok_or_else(|| io::Error::other("a file with no cloud folder"))?,
                    before: Before::OnlineOnly,
                    down: false,
                })
            })
            .collect::<io::Result<_>>()?;
        write_list(store.as_ref(), &listed)?;
        Ok(Self {
            store,
            files: listed,
        })
    }

    /// Marks `file` as coming down (`true`) or online only again (`false`), flushed.
    fn mark(&mut self, file: &Path, down: bool) -> io::Result<()> {
        let Some(f) = self.files.iter_mut().find(|f| f.path == file) else {
            return Err(io::Error::other("not on the list"));
        };
        f.down = down;
        write_list(self.store.as_ref(), &self.files)
    }

    /// Removes the list when nothing on it is still down; otherwise keeps it for the next start.
    fn finish(&self) -> io::Result<()> {
        if self.files.iter().any(|f| f.down) {
            return Ok(());
        }
        self.store.remove()
    }
}

/// What putting back a left list did.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct PutBack {
    /// Files made online only again.
    pub back: Vec<PathBuf>,
    /// Files the cloud app has not made online only again (kept on the list for the next start).
    pub still_down: Vec<PathBuf>,
}

/// At the start of PCTwin, or of a move: puts back every file a list left in `store` still names
/// as down, through `cloud_for(its cloud folder)`, then removes the list (or keeps only what is
/// still down). The cloud folders are `roots`, as found on the laptop again now; the folders the
/// list names are never trusted: a file in none of `roots` is not touched and stays on the list.
/// No list: nothing to do. A list that cannot be read is kept, never acted on.
pub fn put_back_listed(
    store: &dyn ListStore,
    roots: &[PathBuf],
    cloud_for: &mut dyn FnMut(&Path) -> Box<dyn CloudFiles>,
) -> io::Result<PutBack> {
    let Some(bytes) = store.read()? else {
        return Ok(PutBack::default());
    };
    let list: ListFile = serde_json::from_slice(&bytes).map_err(io::Error::other)?;
    if list.version != LIST_VERSION || list.files.len() > MOST_LISTED {
        return Err(io::Error::other("not a list this version reads"));
    }
    let mut done = PutBack::default();
    let mut left = Vec::new();
    for f in list.files.into_iter().filter(|f| f.down) {
        // The deepest cloud folder found now that holds it; none: left alone, kept on the list.
        let Some(root) = roots
            .iter()
            .filter(|r| f.path.starts_with(r))
            .max_by_key(|r| r.as_os_str().len())
        else {
            done.still_down.push(f.path.clone());
            left.push(f);
            continue;
        };
        let mut cloud = cloud_for(root);
        let back = cloud.is_online_only(&f.path).ok() == Some(true)
            || (cloud.put_back(&f.path).is_ok()
                && cloud.is_online_only(&f.path).ok() == Some(true));
        // A file that is gone is not down any more either.
        if back || !f.path.exists() {
            done.back.push(f.path);
        } else {
            done.still_down.push(f.path.clone());
            left.push(f);
        }
    }
    if left.is_empty() {
        store.remove()?;
    } else {
        write_list(store, &left)?;
    }
    Ok(done)
}

/// The send's opener while local copies are kept (see the module's words).
pub struct KeepOpener {
    inner: Arc<dyn Opener>,
    keep: KeepLocal,
    files: HashMap<PathBuf, KeepFile>,
    clouds: Mutex<Vec<Box<dyn CloudFiles + Send>>>,
    network: Box<dyn Fn() -> Network + Send + Sync>,
    state: Arc<Mutex<State>>,
}

fn lock<T>(m: &Mutex<T>) -> MutexGuard<'_, T> {
    m.lock().unwrap_or_else(std::sync::PoisonError::into_inner)
}

fn safe_in_cloud() -> io::Error {
    io::Error::other(OnlineOnly)
}

impl KeepOpener {
    /// `inner`: the move's own opener, for every other file. `keep`: the people's consents.
    /// `files`: the files chosen, each with its cloud in `clouds`. `network`: the connection now.
    pub fn new(
        inner: Arc<dyn Opener>,
        keep: KeepLocal,
        files: Vec<KeepFile>,
        clouds: Vec<Box<dyn CloudFiles + Send>>,
        network: Box<dyn Fn() -> Network + Send + Sync>,
    ) -> Self {
        let mut report = Report::default();
        for f in &files {
            report.files.insert(
                f.wanted.item,
                FileReport {
                    provider: f.wanted.provider,
                    sent: false,
                    back_online_only: None,
                    why_not: Some(WhyNot::Paused),
                    size: f.wanted.size,
                },
            );
        }
        Self {
            inner,
            keep,
            files: files
                .into_iter()
                .map(|f| (f.wanted.path.clone(), f))
                .collect(),
            clouds: Mutex::new(clouds),
            network,
            state: Arc::new(Mutex::new(State {
                report,
                down: BTreeSet::new(),
                readers: HashMap::new(),
                list: None,
            })),
        }
    }

    /// Keeps `list` up to date as files come down and go back (see the module's words). Made
    /// before the first file is opened.
    pub fn recorded(self, list: KeptList) -> Self {
        lock(&self.state).list = Some(list);
        self
    }

    /// What keeping copies has done so far.
    pub fn report(&self) -> Report {
        lock(&self.state).report.clone()
    }

    /// When the send has ended: every file still down is made online only again, and each one
    /// `arrived` says the new laptop has is marked sent. The final report.
    pub fn finish(&self, arrived: &dyn Fn(ItemId) -> bool) -> Report {
        let mut clouds = lock(&self.clouds);
        let mut state = lock(&self.state);
        let down: Vec<ItemId> = state.down.iter().copied().collect();
        for item in down {
            if let Some(f) = self.files.values().find(|f| f.wanted.item == item) {
                put_back(&mut state, clouds[f.cloud].as_mut(), &f.wanted);
            }
        }
        for (item, f) in state.report.files.iter_mut() {
            if f.why_not == Some(WhyNot::NotSent) && arrived(*item) {
                f.sent = true;
                f.why_not = None;
            }
        }
        if let Some(list) = &state.list {
            // Kept while anything is still down: the next start tries again.
            let _ = list.finish();
        }
        state.report.clone()
    }

    fn mark(state: &mut State, w: &Wanted, why: Option<WhyNot>) {
        if let Some(f) = state.report.files.get_mut(&w.item) {
            f.why_not = why;
        }
    }

    /// Room for `size` more bytes, with [`KEEP_FREE_OLD`] to spare.
    fn fits(cloud: &dyn CloudFiles, size: u64) -> bool {
        cloud
            .free_space()
            .is_some_and(|free| free.saturating_sub(KEEP_FREE_OLD) >= size)
    }

    fn open_kept(&self, f: &KeepFile) -> io::Result<Box<dyn Source>> {
        let w = &f.wanted;
        let mut clouds = lock(&self.clouds);
        let mut state = lock(&self.state);
        // Already brought down for this move (read again, or picked up after a drop).
        if state.down.contains(&w.item) {
            let cloud = clouds[f.cloud].as_mut();
            if cloud.is_online_only(&w.path).ok() == Some(false)
                && cloud.size_on_laptop(&w.path).ok() == Some(w.size)
            {
                return self.reader(&mut state, cloud, w);
            }
            put_back(&mut state, cloud, w);
        }
        match self.keep.check_one(clouds[f.cloud].as_ref(), w) {
            Ok(()) => {}
            // On the laptop already: read as any other file is.
            Err(WhyNot::AlreadyOnLaptop) => {
                Self::mark(&mut state, w, Some(WhyNot::AlreadyOnLaptop));
                drop(state);
                drop(clouds);
                return self.inner.open(&w.path);
            }
            Err(why) => {
                Self::mark(&mut state, w, Some(why));
                return Err(safe_in_cloud());
            }
        }
        if !self.keep.allowed_on(w, (self.network)()) {
            state.report.stopped = Some(Stopped::MeteredNow);
            Self::mark(&mut state, w, Some(WhyNot::Paused));
            return Err(safe_in_cloud());
        }
        if !Self::fits(clouds[f.cloud].as_ref(), w.size) {
            // Room first: files whose reading has finished go back online only.
            let done: Vec<ItemId> = state
                .down
                .iter()
                .copied()
                .filter(|i| state.readers.get(i).copied().unwrap_or(0) == 0)
                .collect();
            for item in done {
                if let Some(other) = self.files.values().find(|o| o.wanted.item == item) {
                    put_back(&mut state, clouds[other.cloud].as_mut(), &other.wanted);
                }
            }
            if !Self::fits(clouds[f.cloud].as_ref(), w.size) {
                if state.down.is_empty() {
                    Self::mark(&mut state, w, Some(WhyNot::TooBig));
                } else {
                    state.report.stopped = Some(Stopped::NoSpace);
                    Self::mark(&mut state, w, Some(WhyNot::Paused));
                }
                return Err(safe_in_cloud());
            }
        }
        // Written down, flushed, before it comes down; if the list cannot be written, it does not.
        if let Some(list) = state.list.as_mut()
            && list.mark(&w.path, true).is_err()
        {
            Self::mark(&mut state, w, Some(WhyNot::DownloadFailed));
            return Err(safe_in_cloud());
        }
        let cloud = clouds[f.cloud].as_mut();
        let came = match cloud.bring_down(&w.path, w.size) {
            Ok(()) => cloud.is_online_only(&w.path).ok() == Some(false),
            // Not the approved size: refused before anything came down.
            Err(e) if e.kind() == io::ErrorKind::InvalidData => {
                if let Some(list) = state.list.as_mut() {
                    let _ = list.mark(&w.path, false);
                }
                Self::mark(&mut state, w, Some(WhyNot::Changed));
                return Err(safe_in_cloud());
            }
            Err(_) => false,
        };
        if !came {
            // Whatever part came down goes back.
            state.down.insert(w.item);
            if cloud.is_online_only(&w.path).ok() != Some(true) {
                put_back(&mut state, cloud, w);
            } else {
                state.down.remove(&w.item);
                if let Some(list) = state.list.as_mut() {
                    let _ = list.mark(&w.path, false);
                }
            }
            Self::mark(&mut state, w, Some(WhyNot::DownloadFailed));
            return Err(safe_in_cloud());
        }
        state.report.downloaded_bytes += w.size;
        state.down.insert(w.item);
        let name = w
            .path
            .file_name()
            .map(|n| n.to_string_lossy().into_owned())
            .unwrap_or_default();
        let holds_account_state = cloud.head(&w.path, HEAD).map_or(true, |head| {
            crate::cloud::account_state_by_content(&name, &head).is_some()
        });
        if holds_account_state {
            Self::mark(&mut state, w, Some(WhyNot::OutOfScope));
            put_back(&mut state, cloud, w);
            return Err(safe_in_cloud());
        }
        if cloud.size_on_laptop(&w.path).ok() != Some(w.size) {
            Self::mark(&mut state, w, Some(WhyNot::Changed));
            put_back(&mut state, cloud, w);
            return Err(safe_in_cloud());
        }
        Self::mark(&mut state, w, Some(WhyNot::NotSent));
        self.reader(&mut state, cloud, w)
    }

    /// The file, opened inside its cloud folder by handle ([`CloudFiles::open_read`]), never by
    /// its path again.
    fn reader(
        &self,
        state: &mut State,
        cloud: &dyn CloudFiles,
        w: &Wanted,
    ) -> io::Result<Box<dyn Source>> {
        let file = cloud.open_read(&w.path)?;
        *state.readers.entry(w.item).or_default() += 1;
        Ok(Box::new(Kept {
            file,
            item: w.item,
            state: Arc::clone(&self.state),
        }))
    }
}

/// Makes `w` online only again, and says how that went.
fn put_back(state: &mut State, cloud: &mut dyn CloudFiles, w: &Wanted) {
    if !state.down.remove(&w.item) {
        return;
    }
    let back = cloud.put_back(&w.path).is_ok() && cloud.is_online_only(&w.path).ok() == Some(true);
    // Off the list once it is online only again; still on it otherwise, for the next start.
    if back && let Some(list) = state.list.as_mut() {
        let _ = list.mark(&w.path, false);
    }
    if let Some(f) = state.report.files.get_mut(&w.item) {
        // Brought down again later: only how it last ended counts.
        f.back_online_only = Some(back);
    }
    state.report.still_on_old_laptop_bytes = state
        .report
        .files
        .values()
        .filter(|f| f.back_online_only == Some(false))
        .map(|f| f.size)
        .sum();
}

impl Opener for KeepOpener {
    fn open(&self, path: &Path) -> io::Result<Box<dyn Source>> {
        match self.files.get(path) {
            Some(f) => self.open_kept(f),
            None => self.inner.open(path),
        }
    }
}

/// A kept file being read: when its reader is done, it may be put back.
struct Kept {
    file: std::fs::File,
    item: ItemId,
    state: Arc<Mutex<State>>,
}

impl Read for Kept {
    fn read(&mut self, buf: &mut [u8]) -> io::Result<usize> {
        self.file.read(buf)
    }
}

impl Seek for Kept {
    fn seek(&mut self, pos: SeekFrom) -> io::Result<u64> {
        self.file.seek(pos)
    }
}

impl Source for Kept {
    fn stamp(&mut self) -> io::Result<Stamp> {
        self.file.stamp()
    }

    fn identity(&mut self) -> io::Result<Option<pctwin_journal::FileId>> {
        self.file.identity()
    }
}

impl Drop for Kept {
    fn drop(&mut self) {
        let mut state = lock(&self.state);
        if let Some(n) = state.readers.get_mut(&self.item) {
            *n = n.saturating_sub(1);
        }
    }
}
