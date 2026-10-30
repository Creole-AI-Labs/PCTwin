//! Keep a local copy too (Task List 2.6, with consent; acceptance T26-013, T26-014).
//!
//! By default a file stored online only stays in its cloud ([`crate::cloud`]). When a person
//! chooses to keep a copy of their own online-only files on the new laptop as well, the old
//! laptop brings them down in bounded batches, checks each really arrived, hands them to the move
//! to send, then asks the cloud app to make each one online only again:
//!
//! - **only with that person's consent**, for that provider; nobody else's files are touched;
//! - **only the person's own synced files**: a file is brought down only if it is inside the
//!   person's synced folder (found on the laptop, for any sync app, known or not) and is not the
//!   app's own account state by its names or by what the system marks it as; once down, what it
//!   holds is checked too, and account state (a data-protection blob, a keychain, token text) is
//!   never sent ([`crate::cloud::account_state_by_name`],
//!   [`crate::cloud::account_state_by_content`]; third security review, finding 4);
//! - **space first**: each batch fits the old laptop's free space with [`KEEP_FREE_OLD`] to spare,
//!   checked again before every batch; a file that can never fit stays safe in its cloud;
//! - **the connection**: on a connection that may charge for data (or one PCTwin cannot tell),
//!   nothing more comes down unless the person allowed that too (T26-013);
//! - **honest**: a download that failed or did not really happen is never sent; a file that was
//!   brought down is always put back, sent or not; a file the cloud app did not make online only
//!   again is said plainly with the space it still takes. The effect on the cloud app (what was
//!   downloaded, what is still down) is recorded in the [`Report`].
//!
//! Reading a file through is what brings it down (any ordinary read of a cloud file does). On
//! Windows the cloud app is asked to make it online only again with the documented `attrib -P +U`
//! ("unpinned"); the cloud app does that in its own time, so PCTwin waits a bounded time and then
//! says what it sees. macOS has no command for that (Apple removed `brctl evict` in macOS 14 and
//! `fileproviderctl evict` in 14.4); its own call, `NSFileManager evictUbiquitousItemAtURL`, is
//! given by the app ([`PutBack::Evict`]) and works only for files macOS itself keeps in a cloud
//! (iCloud Drive), so on a Mac only a cloud folder macOS says it keeps is offered (`keep_local`).
//! Linux sync apps keep whole copies, never files stored online only, so there is nothing to bring
//! down there.

use std::collections::BTreeMap;
use std::io;
use std::path::{Path, PathBuf};
use std::time::{Duration, Instant};

use pctwin_record::{CloudProvider, ItemId};

use crate::cloud::{Os, account_state_by_content, account_state_by_name, provider_name};

/// Space always left free on the old laptop's drive while copies are kept: a drive filled to the
/// last byte stops a laptop working (the same room the move keeps on the new laptop).
pub const KEEP_FREE_OLD: u64 = 1024 * 1024 * 1024;

/// What the old laptop's cloud app can do with one file.
pub trait CloudFiles {
    /// Brings the file's contents down onto the laptop (a read right through it), and only if
    /// its size as the laptop sees it before anything comes down is `size`, the approved size; at
    /// most `size` bytes are read. A different size is refused before anything is downloaded, with
    /// [`io::ErrorKind::InvalidData`] (it changed in the cloud since the plan).
    fn bring_down(&mut self, path: &Path, size: u64) -> io::Result<()>;
    /// Opens the file for reading inside the consented folder, through that folder's handle and
    /// each folder's below it, never through a link: the check and the open are one handle.
    fn open_read(&self, path: &Path) -> io::Result<std::fs::File>;
    /// Whether its contents are only online right now.
    fn is_online_only(&self, path: &Path) -> io::Result<bool>;
    /// Its size as the laptop sees it now.
    fn size_on_laptop(&self, path: &Path) -> io::Result<u64>;
    /// Asks the cloud app to make it online only again. `Ok` only once it is: a request the cloud
    /// app has not acted on yet is reported by [`is_online_only`](Self::is_online_only) afterwards.
    fn put_back(&mut self, path: &Path) -> io::Result<()>;
    /// Free space on the drive the cloud folder is on (`None`: not known).
    fn free_space(&self) -> Option<u64>;
    /// Whether the system marks it as the app's own (Windows: hidden and system).
    fn marked_as_apps_own(&self, path: &Path) -> io::Result<bool>;
    /// Its first `n` bytes, once it is on the laptop.
    fn head(&self, path: &Path, n: usize) -> io::Result<Vec<u8>>;
}

/// How much of a file is read to tell whether it is account state.
pub const HEAD: usize = 4096;

/// One person's choice to keep local copies of their online-only files from one provider.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Consent {
    /// Their account ID on the old laptop.
    pub account_id: String,
    pub provider: CloudProvider,
    /// They also allowed it on a connection that may charge for data.
    pub on_metered: bool,
    /// Their home folder on the old laptop.
    pub home: PathBuf,
    /// Their synced folder of this provider, as found on the laptop: only files inside it are
    /// in scope.
    pub root: PathBuf,
}

/// A path's parts below `base`, every one a plain name that is valid text; `None` otherwise.
fn parts_below<'a>(path: &'a Path, base: &Path) -> Option<Vec<&'a str>> {
    path.strip_prefix(base)
        .ok()?
        .components()
        .map(|c| match c {
            std::path::Component::Normal(n) => n.to_str(),
            _ => None,
        })
        .collect()
}

impl Consent {
    /// Whether `path` may be one of this person's own synced files: inside their synced folder,
    /// named plainly (no `..`, no other drive), and not the app's own account state by its names.
    pub fn covers(&self, os: Os, path: &Path) -> bool {
        let Some(under_root) = parts_below(path, &self.root).filter(|p| !p.is_empty()) else {
            return false;
        };
        let under_home = parts_below(path, &self.home);
        account_state_by_name(os, under_home.as_deref(), &under_root).is_none()
    }
}

/// The connection right now.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Network {
    Unmetered,
    /// It may charge for data (a phone's hotspot, a capped plan).
    Metered,
    /// PCTwin could not tell; treated as metered.
    NotKnown,
}

/// One online-only file a person wants a copy of.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Wanted {
    pub item: ItemId,
    pub account_id: String,
    pub provider: CloudProvider,
    pub path: PathBuf,
    /// Its size in the approved plan.
    pub size: u64,
}

/// One batch: the files brought down together, and their bytes.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Batch {
    pub items: Vec<(ItemId, u64)>,
    pub bytes: u64,
}

/// The batches for some files, and the files too big to bring down at all.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct BatchPlan {
    pub batches: Vec<Batch>,
    pub too_big: Vec<ItemId>,
}

/// Splits `files` (in order) into batches that each fit `free` bytes with [`KEEP_FREE_OLD`] to
/// spare and are at most `batch_max` bytes. A file larger than that room is too big.
pub fn plan_batches(files: &[(ItemId, u64)], free: u64, batch_max: u64) -> BatchPlan {
    let room = free.saturating_sub(KEEP_FREE_OLD).min(batch_max);
    let mut plan = BatchPlan {
        batches: Vec::new(),
        too_big: Vec::new(),
    };
    let mut current = Batch {
        items: Vec::new(),
        bytes: 0,
    };
    for &(item, size) in files {
        if size > room {
            plan.too_big.push(item);
            continue;
        }
        if current.bytes + size > room {
            plan.batches.push(std::mem::replace(
                &mut current,
                Batch {
                    items: Vec::new(),
                    bytes: 0,
                },
            ));
        }
        current.items.push((item, size));
        current.bytes += size;
    }
    if !current.items.is_empty() {
        plan.batches.push(current);
    }
    plan
}

/// Why a file has no copy kept.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum WhyNot {
    /// Its person did not choose this for it (or for its provider).
    NoConsent,
    /// It is not one of the person's own synced files (an app's own sign-in or settings, say):
    /// never brought down.
    OutOfScope,
    /// It is already on the laptop: copied as always, not part of this.
    AlreadyOnLaptop,
    /// Bigger than the old laptop can hold with room to spare.
    TooBig,
    /// It could not be brought down, or the cloud app did not really bring it down.
    DownloadFailed,
    /// What came down is not the size the plan approved: it changed in the cloud since.
    Changed,
    /// Brought down, but the move could not send it.
    NotSent,
    /// Keeping copies stopped before it was reached ([`Stopped`]).
    Paused,
}

/// Why keeping copies stopped before the end.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Stopped {
    /// The connection may charge for data now, and the person did not allow that.
    MeteredNow,
    /// The old laptop has no room for the next batch.
    NoSpace,
}

impl Stopped {
    pub fn words(&self) -> &'static str {
        match self {
            Stopped::MeteredNow => {
                "Paused: this connection may charge for data now. Keeping copies of online-only files continues on a free connection, or if you allow it."
            }
            Stopped::NoSpace => {
                "Paused: the old laptop is running out of space to download more files. The rest stay safe in their cloud."
            }
        }
    }
}

/// How one file ended.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct FileReport {
    pub provider: CloudProvider,
    /// The move took it.
    pub sent: bool,
    /// After it was brought down: whether it is online only again (`None`: never brought down).
    pub back_online_only: Option<bool>,
    pub why_not: Option<WhyNot>,
    pub size: u64,
}

/// What keeping copies did, file by file, and its effect on the cloud apps.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct Report {
    pub files: BTreeMap<ItemId, FileReport>,
    pub stopped: Option<Stopped>,
    /// Bytes downloaded from the cloud apps.
    pub downloaded_bytes: u64,
    /// Bytes still downloaded on the old laptop because the cloud app has not made them online
    /// only again.
    pub still_on_old_laptop_bytes: u64,
}

const FREE_UP: &str = "To free it, right-click it and choose Free up space.";

impl Report {
    /// The plain words for one file, or `None` when it went as planned (or was not part of this).
    pub fn words_for(&self, item: &ItemId) -> Option<String> {
        let f = self.files.get(item)?;
        let p = provider_name(f.provider).unwrap_or("its cloud");
        let still_down = format!(
            "{p} has not made it online only again on the old laptop, so it still takes space there. {FREE_UP}"
        );
        let back = f.back_online_only == Some(true);
        Some(match f.why_not {
            None if back => return None,
            None => format!("Copied. {still_down}"),
            Some(WhyNot::AlreadyOnLaptop) => return None,
            Some(WhyNot::NoConsent) => format!(
                "Safe in {p}. It was not downloaded to keep a copy, because that was not chosen for it."
            ),
            Some(WhyNot::OutOfScope) => format!(
                "Not copied. PCTwin keeps copies only of the files you keep in {p}, never the app's own sign-in or settings. Sign in to {p} on the new laptop."
            ),
            Some(WhyNot::TooBig) => format!(
                "Safe in {p}. It is too big to download on the old laptop with the space it has."
            ),
            Some(WhyNot::Paused) => format!(
                "Safe in {p}. Keeping a copy of it is paused; it continues when you choose."
            ),
            Some(WhyNot::DownloadFailed) => format!(
                "Safe in {p}. It could not be downloaded to keep a copy, so it stays online only."
            ),
            Some(WhyNot::NotSent) if back => format!(
                "Safe in {p}. It was downloaded but could not be sent, and it is online only again."
            ),
            Some(WhyNot::NotSent) => {
                format!("Safe in {p}. It was downloaded but could not be sent. {still_down}")
            }
            Some(WhyNot::Changed) if back => format!(
                "Safe in {p}. It changed in {p} after the plan was made, so this copy was not sent; it is online only again."
            ),
            Some(WhyNot::Changed) => format!(
                "Safe in {p}. It changed in {p} after the plan was made, so this copy was not sent. {still_down}"
            ),
        })
    }
}

/// One batch handed to the move: each file and where to read it.
pub type ToSend = [(ItemId, PathBuf)];
/// What the move took of a batch: each file, and whether it was taken.
pub type Taken = Vec<(ItemId, bool)>;

/// Keeps local copies for the people who chose it.
#[derive(Debug, Clone)]
pub struct KeepLocal {
    /// The old laptop's system, whose app folders the consents name.
    os: Os,
    consents: Vec<Consent>,
    batch_max: u64,
}

impl KeepLocal {
    /// `batch_max`: most bytes brought down at once.
    pub fn new(os: Os, consents: Vec<Consent>, batch_max: u64) -> Self {
        Self {
            os,
            consents,
            batch_max,
        }
    }

    fn consent(&self, w: &Wanted) -> Option<&Consent> {
        self.consents
            .iter()
            .find(|c| c.account_id == w.account_id && c.provider == w.provider)
    }

    /// Whether `w` may be brought down at all: its person chose this for its provider, it is one
    /// of their own synced files (anything that cannot be told apart from the app's own is not),
    /// and it is online only now. The same checks [`run`](Self::run) makes, for one file.
    pub fn check_one(&self, cloud: &dyn CloudFiles, w: &Wanted) -> Result<(), WhyNot> {
        let consent = self.consent(w).ok_or(WhyNot::NoConsent)?;
        if !consent.covers(self.os, &w.path) || cloud.marked_as_apps_own(&w.path).unwrap_or(true) {
            return Err(WhyNot::OutOfScope);
        }
        if !cloud.is_online_only(&w.path).unwrap_or(true) {
            return Err(WhyNot::AlreadyOnLaptop);
        }
        Ok(())
    }

    /// Whether `w` may come down on `network`: a free connection, or its person allowed one that
    /// may charge for data.
    pub fn allowed_on(&self, w: &Wanted, network: Network) -> bool {
        network == Network::Unmetered || self.consent(w).is_some_and(|c| c.on_metered)
    }

    /// Brings `wanted` down batch by batch, hands each batch's good files to `send` (which says
    /// which it took), and puts every file it brought down back online only.
    pub fn run(
        &mut self,
        cloud: &mut dyn CloudFiles,
        wanted: Vec<Wanted>,
        network: &dyn Fn() -> Network,
        send: &mut dyn FnMut(&ToSend) -> Taken,
    ) -> Report {
        let mut report = Report::default();
        let mark = |report: &mut Report, w: &Wanted, why: Option<WhyNot>| {
            report.files.insert(
                w.item,
                FileReport {
                    provider: w.provider,
                    sent: false,
                    back_online_only: None,
                    why_not: why,
                    size: w.size,
                },
            );
        };
        let mut queue = Vec::new();
        for w in wanted {
            match self.check_one(cloud, &w) {
                Ok(()) => queue.push(w),
                Err(why) => mark(&mut report, &w, Some(why)),
            }
        }
        // Files that can never fit, even with nothing else down.
        let sizes: Vec<_> = queue.iter().map(|w| (w.item, w.size)).collect();
        let first = plan_batches(&sizes, cloud.free_space().unwrap_or(0), self.batch_max);
        queue.retain(|w| {
            if first.too_big.contains(&w.item) {
                mark(&mut report, w, Some(WhyNot::TooBig));
                false
            } else {
                true
            }
        });
        let mut queue = std::collections::VecDeque::from(queue);
        while let Some(next) = queue.front() {
            if !self.allowed_on(next, network()) {
                report.stopped = Some(Stopped::MeteredNow);
                break;
            }
            let sizes: Vec<_> = queue.iter().map(|w| (w.item, w.size)).collect();
            let plan = plan_batches(&sizes, cloud.free_space().unwrap_or(0), self.batch_max);
            let Some(batch) = plan
                .batches
                .into_iter()
                .next()
                .filter(|b| !b.items.is_empty())
            else {
                report.stopped = Some(Stopped::NoSpace);
                break;
            };
            let in_batch: Vec<Wanted> = queue
                .iter()
                .filter(|w| batch.items.iter().any(|(i, _)| *i == w.item))
                .cloned()
                .collect();
            queue.retain(|w| !batch.items.iter().any(|(i, _)| *i == w.item));
            // Down, and checked: really on the laptop now, at the approved size.
            let mut down = Vec::new();
            let mut good = Vec::new();
            for w in &in_batch {
                let came = match cloud.bring_down(&w.path, w.size) {
                    Ok(()) => cloud.is_online_only(&w.path).ok() == Some(false),
                    Err(e) if e.kind() == io::ErrorKind::InvalidData => {
                        mark(&mut report, w, Some(WhyNot::Changed));
                        continue;
                    }
                    Err(_) => false,
                };
                if !came {
                    mark(&mut report, w, Some(WhyNot::DownloadFailed));
                    continue;
                }
                report.downloaded_bytes += w.size;
                down.push(w);
                let name = w
                    .path
                    .file_name()
                    .map(|n| n.to_string_lossy().into_owned())
                    .unwrap_or_default();
                let holds_account_state = cloud.head(&w.path, HEAD).map_or(true, |head| {
                    account_state_by_content(&name, &head).is_some()
                });
                if holds_account_state {
                    // Down already, so it goes back below; never sent.
                    mark(&mut report, w, Some(WhyNot::OutOfScope));
                } else if cloud.size_on_laptop(&w.path).ok() == Some(w.size) {
                    mark(&mut report, w, Some(WhyNot::NotSent));
                    good.push((w.item, w.path.clone()));
                } else {
                    mark(&mut report, w, Some(WhyNot::Changed));
                }
            }
            if !good.is_empty() {
                for (item, took) in send(&good) {
                    if took
                        && good.iter().any(|(g, _)| *g == item)
                        && let Some(f) = report.files.get_mut(&item)
                    {
                        f.sent = true;
                        f.why_not = None;
                    }
                }
            }
            // Every file brought down goes back, sent or not.
            for w in down {
                let back = cloud.put_back(&w.path).is_ok()
                    && cloud.is_online_only(&w.path).ok() == Some(true);
                if !back {
                    report.still_on_old_laptop_bytes += w.size;
                }
                if let Some(f) = report.files.get_mut(&w.item) {
                    f.back_online_only = Some(back);
                }
            }
        }
        for w in queue {
            mark(&mut report, &w, Some(WhyNot::Paused));
        }
        report
    }
}

/// How the cloud app is asked to make a file online only again.
#[derive(Debug, Clone)]
pub enum PutBack {
    /// Windows: the system's own `attrib -P +U`, from Windows' own `System32` as the kernel names
    /// it (never from the environment); `None` when it is not known, and then nothing is put back.
    Attrib(Option<PathBuf>),
    /// macOS: the system's own call for a file it keeps in a cloud (`NSFileManager
    /// evictUbiquitousItemAtURL`), given by the app; it answers whether macOS took the request.
    Evict(fn(&Path) -> io::Result<()>),
}

/// The cloud apps built on the system's own support for files stored online only: Windows' cloud
/// files (OneDrive and others) and, on a Mac, what macOS keeps in a cloud (iCloud Drive).
#[derive(Debug, Clone)]
pub struct SyncedFolderFiles {
    /// The consented person's cloud folder: nothing outside it is touched.
    root: PathBuf,
    /// How long to wait for the cloud app to make a file online only again.
    wait: Duration,
    put_back: PutBack,
}

/// Why a file was not touched.
fn not_in_scope() -> io::Error {
    io::Error::new(
        io::ErrorKind::PermissionDenied,
        "it is not in the person's own cloud folder",
    )
}

impl SyncedFolderFiles {
    /// `root`: the consented person's cloud folder. `system32`: Windows' own `System32` folder as
    /// the kernel names it (`pctwin_helper_os::system_folder`), where `attrib` is run from;
    /// `None` when it is not known, and then nothing is put back.
    pub fn new(root: &Path, wait: Duration, system32: Option<&Path>) -> Self {
        Self {
            root: root.to_path_buf(),
            wait,
            put_back: PutBack::Attrib(system32.map(Path::to_path_buf)),
        }
    }

    /// On a Mac: `root` as for [`new`](Self::new); `evict` is macOS's own call asking for a file
    /// to be made online only again.
    pub fn evicting(root: &Path, wait: Duration, evict: fn(&Path) -> io::Result<()>) -> Self {
        Self {
            root: root.to_path_buf(),
            wait,
            put_back: PutBack::Evict(evict),
        }
    }

    /// Only a file inside the consented folder (third review, finding 7): named plainly below it
    /// (no `..`), with no link on the way, and really inside it.
    fn in_scope(&self, path: &Path) -> io::Result<()> {
        let below = path.strip_prefix(&self.root).map_err(|_| not_in_scope())?;
        let mut at = self.root.clone();
        for part in below.components() {
            let std::path::Component::Normal(name) = part else {
                return Err(not_in_scope());
            };
            at.push(name);
            if std::fs::symlink_metadata(&at)?.file_type().is_symlink() {
                return Err(not_in_scope());
            }
        }
        let real = std::fs::canonicalize(path)?;
        if !real.starts_with(std::fs::canonicalize(&self.root)?) {
            return Err(not_in_scope());
        }
        Ok(())
    }
}

impl CloudFiles for SyncedFolderFiles {
    fn bring_down(&mut self, path: &Path, size: u64) -> io::Result<()> {
        use std::io::Read;
        let file = self.open_read(path)?;
        // Its size as the laptop sees it, before anything comes down (opening does not download).
        if file.metadata()?.len() != size {
            return Err(io::Error::new(
                io::ErrorKind::InvalidData,
                "it is not the size the plan approved",
            ));
        }
        io::copy(&mut file.take(size), &mut io::sink())?;
        Ok(())
    }

    fn open_read(&self, path: &Path) -> io::Result<std::fs::File> {
        use cap_fs_ext::OpenOptionsFollowExt;
        use cap_primitives::fs as cap;
        let below = path.strip_prefix(&self.root).map_err(|_| not_in_scope())?;
        let mut names = Vec::new();
        for part in below.components() {
            let std::path::Component::Normal(name) = part else {
                return Err(not_in_scope());
            };
            names.push(name);
        }
        let (last, folders) = names.split_last().ok_or_else(not_in_scope)?;
        let mut dir = cap::open_ambient_dir(&self.root, cap_primitives::ambient_authority())?;
        for name in folders {
            dir = cap::open_dir_nofollow(&dir, Path::new(name))?;
        }
        let mut options = cap::OpenOptions::new();
        options.read(true).follow(cap::FollowSymlinks::No);
        let file = cap::open(&dir, Path::new(last), &options)?;
        if !file.metadata()?.is_file() {
            return Err(not_in_scope());
        }
        Ok(file)
    }

    fn is_online_only(&self, path: &Path) -> io::Result<bool> {
        Ok(pctwin_scan::is_cloud_only(&std::fs::symlink_metadata(
            path,
        )?))
    }

    fn size_on_laptop(&self, path: &Path) -> io::Result<u64> {
        Ok(std::fs::symlink_metadata(path)?.len())
    }

    fn put_back(&mut self, path: &Path) -> io::Result<()> {
        self.in_scope(path)?;
        match &self.put_back {
            // The system's own attrib, by its full path in the folder the kernel names, never one
            // found on PATH or through an environment value; started there, with no environment
            // of this process's.
            PutBack::Attrib(system32) => {
                let system32 = system32
                    .as_deref()
                    .ok_or_else(|| io::Error::other("the system folder is not known"))?;
                let mut command = attrib_command(system32, path);
                if !command.status()?.success() {
                    return Err(io::Error::other("the cloud app was not asked"));
                }
            }
            PutBack::Evict(evict) => evict(path)?,
        }
        // The cloud app makes it online only in its own time.
        let started = Instant::now();
        loop {
            if self.is_online_only(path)? {
                return Ok(());
            }
            if started.elapsed() >= self.wait {
                return Err(io::Error::other("not online only yet"));
            }
            std::thread::sleep(Duration::from_millis(100));
        }
    }

    fn free_space(&self) -> Option<u64> {
        free_space_of(&self.root, &pctwin_scan::volumes_this_laptop())
    }

    fn marked_as_apps_own(&self, path: &Path) -> io::Result<bool> {
        self.in_scope(path)?;
        let meta = std::fs::symlink_metadata(path)?;
        #[cfg(windows)]
        {
            use std::os::windows::fs::MetadataExt;
            const HIDDEN_AND_SYSTEM: u32 = 0x2 | 0x4;
            Ok(meta.file_attributes() & HIDDEN_AND_SYSTEM == HIDDEN_AND_SYSTEM)
        }
        #[cfg(not(windows))]
        {
            let _ = meta;
            Ok(false)
        }
    }

    fn head(&self, path: &Path, n: usize) -> io::Result<Vec<u8>> {
        use std::io::Read;
        self.in_scope(path)?;
        let mut out = Vec::with_capacity(n);
        std::fs::File::open(path)?
            .take(n as u64)
            .read_to_end(&mut out)?;
        Ok(out)
    }
}

/// `attrib -P +U <path>` ("unpinned": the cloud app may make it online only again): Windows' own
/// `attrib` by its full path in `system32`, started in that folder, with none of this process's
/// environment, no window and nothing in or out.
pub fn attrib_command(system32: &Path, path: &Path) -> std::process::Command {
    let mut command = pctwin_types::process::quiet(system32.join("attrib.exe"));
    command
        .env_clear()
        .current_dir(system32)
        .arg("-P")
        .arg("+U")
        .arg(path)
        .stdin(std::process::Stdio::null())
        .stdout(std::process::Stdio::null())
        .stderr(std::process::Stdio::null());
    #[cfg(windows)]
    {
        use std::os::windows::process::CommandExt;
        const CREATE_NO_WINDOW: u32 = 0x0800_0000;
        command.creation_flags(CREATE_NO_WINDOW);
    }
    command
}

/// Free space on the volume holding `root`: the one mounted deepest above it (a drive mounted in
/// a folder of another counts as itself).
pub fn free_space_of(root: &Path, volumes: &[pctwin_scan::Volume]) -> Option<u64> {
    volumes
        .iter()
        .filter(|v| root.starts_with(&v.mount))
        .max_by_key(|v| v.mount.as_os_str().len())
        .map(|v| v.free_bytes)
}
