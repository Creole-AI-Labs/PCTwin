//! Keep a local copy too (Task List 2.6, with consent; acceptance T26-013, T26-014).
//!
//! By default a file stored online only stays in its cloud ([`crate::cloud`]). When a person
//! chooses to keep a copy of their own online-only files on the new laptop as well, the old
//! laptop brings them down in bounded batches, checks each really arrived, hands them to the move
//! to send, then asks the cloud app to make each one online only again:
//!
//! - **only with that person's consent**, for that provider; nobody else's files are touched;
//! - **only the person's own synced files**: a file is brought down only if it is inside that
//!   app's allowed place in the person's home folder ([`crate::cloud::capture_scope`]); an app's
//!   sign-in tokens, settings and the system's credential stores never are, whatever they are
//!   called (third security review, finding 4);
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
//! says what it sees. macOS has no such command (only `evictUbiquitousItem`, from Objective-C), so
//! the choice is not offered there yet.

use std::collections::BTreeMap;
use std::io;
use std::path::{Path, PathBuf};
use std::time::{Duration, Instant};

use pctwin_record::{CloudProvider, ItemId};

use crate::cloud::{Os, capture_scope, provider_name};

/// Space always left free on the old laptop's drive while copies are kept: a drive filled to the
/// last byte stops a laptop working (the same room the move keeps on the new laptop).
pub const KEEP_FREE_OLD: u64 = 1024 * 1024 * 1024;

/// What the old laptop's cloud app can do with one file.
pub trait CloudFiles {
    /// Brings the file's contents down onto the laptop (a read right through it).
    fn bring_down(&mut self, path: &Path) -> io::Result<()>;
    /// Whether its contents are only online right now.
    fn is_online_only(&self, path: &Path) -> io::Result<bool>;
    /// Its size as the laptop sees it now.
    fn size_on_laptop(&self, path: &Path) -> io::Result<u64>;
    /// Asks the cloud app to make it online only again. `Ok` only once it is: a request the cloud
    /// app has not acted on yet is reported by [`is_online_only`](Self::is_online_only) afterwards.
    fn put_back(&mut self, path: &Path) -> io::Result<()>;
    /// Free space on the drive the cloud folder is on (`None`: not known).
    fn free_space(&self) -> Option<u64>;
}

/// One person's choice to keep local copies of their online-only files from one provider.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Consent {
    /// Their account ID on the old laptop.
    pub account_id: String,
    pub provider: CloudProvider,
    /// They also allowed it on a connection that may charge for data.
    pub on_metered: bool,
    /// Their home folder on the old laptop: only that app's allowed place inside it is in scope.
    pub home: PathBuf,
}

impl Consent {
    /// Whether `path` is one of this person's own synced files of this provider: inside the
    /// app's allowed place in their home folder, named plainly (no `..`, no other drive), every
    /// name valid text.
    pub fn covers(&self, os: Os, path: &Path) -> bool {
        let Ok(below) = path.strip_prefix(&self.home) else {
            return false;
        };
        let parts: Option<Vec<&str>> = below
            .components()
            .map(|c| match c {
                std::path::Component::Normal(n) => n.to_str(),
                _ => None,
            })
            .collect();
        parts.is_some_and(|p| capture_scope(os, &p) == Some(self.provider))
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
            let consent = self.consent(&w);
            if consent.is_none() {
                mark(&mut report, &w, Some(WhyNot::NoConsent));
            } else if !consent.is_some_and(|c| c.covers(self.os, &w.path)) {
                mark(&mut report, &w, Some(WhyNot::OutOfScope));
            } else if !cloud.is_online_only(&w.path).unwrap_or(true) {
                mark(&mut report, &w, Some(WhyNot::AlreadyOnLaptop));
            } else {
                queue.push(w);
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
            let allowed =
                network() == Network::Unmetered || self.consent(next).is_some_and(|c| c.on_metered);
            if !allowed {
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
                let came = cloud.bring_down(&w.path).is_ok()
                    && cloud.is_online_only(&w.path).ok() == Some(false);
                if !came {
                    mark(&mut report, w, Some(WhyNot::DownloadFailed));
                    continue;
                }
                report.downloaded_bytes += w.size;
                down.push(w);
                if cloud.size_on_laptop(&w.path).ok() == Some(w.size) {
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

/// The Windows cloud apps (OneDrive and others built on the system's cloud files support).
#[derive(Debug, Clone)]
pub struct WindowsCloudFiles {
    /// The consented person's cloud folder: nothing outside it is touched.
    root: PathBuf,
    /// How long to wait for the cloud app to make a file online only again.
    wait: Duration,
    /// Windows' own `System32`, as the kernel names it (never from the environment).
    system32: Option<PathBuf>,
}

/// Why a file was not touched.
fn not_in_scope() -> io::Error {
    io::Error::new(
        io::ErrorKind::PermissionDenied,
        "it is not in the person's own cloud folder",
    )
}

impl WindowsCloudFiles {
    /// `root`: the consented person's cloud folder. `system32`: Windows' own `System32` folder as
    /// the kernel names it (`pctwin_helper_os::system_folder`), where `attrib` is run from;
    /// `None` when it is not known, and then nothing is put back.
    pub fn new(root: &Path, wait: Duration, system32: Option<&Path>) -> Self {
        Self {
            root: root.to_path_buf(),
            wait,
            system32: system32.map(Path::to_path_buf),
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

impl CloudFiles for WindowsCloudFiles {
    fn bring_down(&mut self, path: &Path) -> io::Result<()> {
        self.in_scope(path)?;
        let mut file = std::fs::File::open(path)?;
        io::copy(&mut file, &mut io::sink())?;
        Ok(())
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
        // The system's own attrib, by its full path in the folder the kernel names, never one
        // found on PATH or through an environment value; started there, with no environment of
        // this process's.
        let system32 = self
            .system32
            .as_deref()
            .ok_or_else(|| io::Error::other("the system folder is not known"))?;
        let mut command = attrib_command(system32, path);
        if !command.status()?.success() {
            return Err(io::Error::other("the cloud app was not asked"));
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
}

/// `attrib -P +U <path>` ("unpinned": the cloud app may make it online only again): Windows' own
/// `attrib` by its full path in `system32`, started in that folder, with none of this process's
/// environment, no window and nothing in or out.
pub fn attrib_command(system32: &Path, path: &Path) -> std::process::Command {
    let mut command = std::process::Command::new(system32.join("attrib.exe"));
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
