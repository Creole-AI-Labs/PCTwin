//! The install chain on a Mac (2.1 joined to the move, 30 October 2026): each app from a plain
//! Homebrew cask, downloaded and placed by PCTwin as the person, never through the `brew`
//! program (the reinstall research of 16 October 2026: "do not depend on the brew binary").
//! Homebrew need not be on the Mac at all, so it is never installed, and a cask's own scripts
//! never run: only a plain cask is a route ([`pctwin_apps::routes::brew`]).
//!
//! For each app, the plan's casks in order, until one holds:
//!
//! 1. **Downloaded** by the locked-down fetch process (the 2.1 plan, ADOPTION change 1), never
//!    on a connection that may charge unless the person allowed it; where the cask pins the bytes,
//!    exactly those bytes. The download keeps the Mac's quarantine flag.
//! 2. **Opened read-only** in PCTwin's own folder: a disk image is attached without showing it
//!    in the Finder, never verified twice, never agreeing to a licence for the person (a disk
//!    image that asks is left for the person); a zip is unpacked with `ditto`, which keeps the
//!    app's signature and quarantine flag.
//! 3. **The very app the old laptop had**: the cask's one app (reached without following any
//!    link inside the download) must carry the old laptop's bundle ID, be signed by the old
//!    laptop's developer team (a Team ID on both sides; none on either side never matches) and
//!    pass the Mac's own checks (`codesign`, and Gatekeeper accepting it as notarised by Apple,
//!    [`pctwin_maker::mac::check_app`]; a Developer ID app Apple has not notarised is refused).
//!    It must also run here and not be older than the person's: never older, its
//!    `LSMinimumSystemVersion` no newer than this Mac's macOS, built for this Mac's processor (an
//!    Intel-only app on Apple silicon only when Rosetta is installed). Anything else is never
//!    placed: the next cask is tried, and the last one's reason is the app's.
//! 4. **Placed as the person** with `ditto`, into `/Applications` when the person may write it
//!    (an administrator, which most people moving are) and otherwise into their own
//!    `~/Applications`, where macOS runs apps just the same: no administrator prompt and no
//!    helper (Security Design C: the helper is for other people's things). It is copied first
//!    into a fresh hidden folder of PCTwin's own beside that place, checked again there (so what
//!    is placed is exactly what was checked), and then moved into place by a rename that never
//!    replaces anything (`renameatx_np` with `RENAME_EXCL`): an app already at that place, or
//!    one that appears meanwhile, is never touched. Only PCTwin's own folder is ever removed.
//! 5. **The launch check PCTwin can make without opening the app on the person's screen** is
//!    step 3 on the copy that is moved into place. Opening it is the person's own first step
//!    (macOS asks once for a downloaded app). An Intel-only app on Apple silicon runs through
//!    Rosetta: said on its line. Without Rosetta it is not placed: needs-action, saying it needs
//!    Rosetta (macOS offers to install it).
//!
//! Anything that does not hold ends needs-action or failed with its reason, never "reinstalled".
//! Every tool is given a fixed environment and runs at its fixed path, with fixed arguments and
//! checked names; the tools sit behind [`Runner`] and the fetch process behind a function, so
//! the whole chain is proven here with fakes and on GitHub's Mac runners for real.

use std::ffi::OsString;
use std::path::{Component, Path, PathBuf};
use std::time::Duration;

use pctwin_apps::after::{self, Installed, Todo};
use pctwin_apps::fetch::{Expected, Outcome, Request};
use pctwin_apps::routes::Route;
use pctwin_journal::RestorePoint;
use pctwin_maker::facts::{Malware, Signature};
use pctwin_maker::run::{Call, Ran, Runner};
use pctwin_record::{AppMaker, FoundApp, FoundWhere};
use pctwin_types::{ItemResult, ItemStatus, Reason};

use crate::plan::AppEntry;
use crate::run::{AppResult, Installer};

/// The largest app download.
pub const MAX_DOWNLOAD: u64 = 4 * 1024 * 1024 * 1024;
/// The longest a tool may take (attaching a large disk image, copying a large app).
const TOOL_TIME: Duration = Duration::from_secs(15 * 60);
/// The longest one signature or Gatekeeper check may take (Gatekeeper asks Apple online).
const CHECK_TIME: Duration = Duration::from_secs(120);

/// What the fetch process did with one download.
pub type Fetch<'a> = &'a dyn Fn(&Request) -> Outcome;

/// The Mac installer (see the module notes).
pub struct MacInstaller<'a> {
    pub runner: &'a dyn Runner,
    /// The locked-down fetch process.
    pub fetch: Fetch<'a>,
    /// The fetch process's staging folder: downloads are made there, and nowhere else.
    pub staging: PathBuf,
    /// PCTwin's own folder for opened disk images and unpacked zips (removed afterwards).
    pub work: PathBuf,
    /// The person's home (`~/Applications` is the place when `/Applications` is not theirs).
    pub home: PathBuf,
    /// `/Applications` (a test's own folder in tests).
    pub system_apps: PathBuf,
    /// This Mac has an Apple-silicon processor.
    pub arm64: bool,
    /// Rosetta is installed (Apple silicon).
    pub rosetta: bool,
    /// This Mac's macOS version (`sw_vers -productVersion`).
    pub macos: String,
    /// The person allowed downloads on a connection that may charge (a Mac cannot say).
    pub allow_metered: bool,
}

/// A cask's app name as a path inside its disk image or zip: plain relative steps ending in one
/// `.app`, never `..`, never a root.
pub fn app_inside(name: &str) -> Option<PathBuf> {
    if name.is_empty() || name.len() > 1024 || name.contains(['\0', '\\']) {
        return None;
    }
    let p = PathBuf::from(name);
    let plain = p.components().all(|c| match c {
        Component::Normal(s) => s.to_str().is_some_and(|s| !s.starts_with('-')),
        _ => false,
    });
    let ends_app = p
        .file_name()
        .and_then(|n| n.to_str())
        .is_some_and(|n| n.len() > 4 && n.ends_with(".app"));
    (plain && ends_app).then_some(p)
}

/// The app inside an opened download, reached step by step from `root` without following any
/// link: a link at any step (a folder in a disk image pointing at `/Applications`, say) is
/// refused, and the app itself must be a real folder.
pub fn reach_inside(root: &Path, inside: &Path) -> Option<PathBuf> {
    let mut at = root.to_path_buf();
    for c in inside.components() {
        at.push(c);
        let m = std::fs::symlink_metadata(&at).ok()?;
        if m.file_type().is_symlink() || !m.is_dir() {
            return None;
        }
    }
    Some(at)
}

/// Moves `from` to `to` (both in one folder's volume), never replacing anything at `to`.
#[cfg(unix)]
fn move_no_replace(from: &Path, to: &Path) -> std::io::Result<()> {
    use rustix::fs::{CWD, RenameFlags, renameat_with};
    renameat_with(CWD, from, CWD, to, RenameFlags::NOREPLACE).map_err(std::io::Error::from)
}

/// Elsewhere (the tests' own build on Windows; the Mac chain never runs there): the folder is
/// moved only when nothing has that name (Windows never moves a folder over another).
#[cfg(not(unix))]
fn move_no_replace(from: &Path, to: &Path) -> std::io::Result<()> {
    if std::fs::symlink_metadata(to).is_ok() {
        return Err(std::io::ErrorKind::AlreadyExists.into());
    }
    std::fs::rename(from, to)
}

/// The start of the hidden folder PCTwin copies an app into beside its place.
pub const PLACING_PREFIX: &str = ".pctwin-placing-";

/// A cask token as part of a file name: letters, digits, `-`, `_`, `@`, `.`.
fn plain_token(t: &str) -> bool {
    !t.is_empty()
        && t.len() <= 128
        && !t.starts_with(['-', '.'])
        && t.bytes()
            .all(|b| b.is_ascii_alphanumeric() || matches!(b, b'-' | b'_' | b'@' | b'.'))
}

/// What a download is, by its own bytes.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Container {
    DiskImage,
    Zip,
}

/// The container of `file`: a zip by its first bytes, a disk image by the `koly` block in its
/// last 512 (Apple's UDIF trailer); anything else (an installer package, a tarball) is not
/// opened.
pub fn container_of(file: &Path) -> Option<Container> {
    use std::io::{Read, Seek, SeekFrom};
    let mut f = std::fs::File::open(file).ok()?;
    let mut head = [0u8; 4];
    f.read_exact(&mut head).ok()?;
    if head == *b"PK\x03\x04" {
        return Some(Container::Zip);
    }
    let len = f.metadata().ok()?.len();
    if len < 512 {
        return None;
    }
    f.seek(SeekFrom::Start(len - 512)).ok()?;
    let mut tail = [0u8; 4];
    f.read_exact(&mut tail).ok()?;
    (tail == *b"koly").then_some(Container::DiskImage)
}

/// The mount points `hdiutil attach -plist` printed (its plist may follow other text).
pub fn mount_points(stdout: &str) -> Vec<PathBuf> {
    let Some(start) = stdout.find("<?xml") else {
        return Vec::new();
    };
    let Ok(v) = plist::from_bytes::<plist::Value>(&stdout.as_bytes()[start..]) else {
        return Vec::new();
    };
    v.as_dictionary()
        .and_then(|d| d.get("system-entities"))
        .and_then(|e| e.as_array())
        .into_iter()
        .flatten()
        .filter_map(|e| e.as_dictionary()?.get("mount-point")?.as_string())
        .map(PathBuf::from)
        .collect()
}

/// Whether macOS `here` is at least `needs` (both dotted numbers; unreadable: not known, so no).
pub fn macos_at_least(here: &str, needs: &str) -> Option<bool> {
    let parse =
        |s: &str| -> Option<Vec<u32>> { s.trim().split('.').map(|p| p.parse().ok()).collect() };
    let (mut a, mut b) = (parse(here)?, parse(needs)?);
    let n = a.len().max(b.len());
    a.resize(n, 0);
    b.resize(n, 0);
    Some(a >= b)
}

/// Whether Rosetta is installed on this Mac (its runtime, where Apple puts it).
pub fn rosetta_here() -> bool {
    std::fs::symlink_metadata("/Library/Apple/usr/libexec/oah/libRosettaRuntime").is_ok()
}

/// Why one cask did not give the app (the last one tried is the app's result).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Miss {
    /// Waiting for a connection that does not charge: the person installs it later.
    Metered,
    /// Not the app the old laptop had, or not proven to be.
    NotVerified,
    /// The download or the tools did not work.
    Failed,
    /// Something is at the app's place already: never replaced.
    Occupied,
    /// A disk image that asks the person to agree to its licence.
    Licence,
    /// The app, but older than the person's: never placed.
    Older,
    /// The app, but it would not run on this Mac (a newer macOS, another processor).
    WontRun,
    /// The app, built for Intel Macs only, and Rosetta is not installed: not placed.
    NeedsRosetta,
}

impl Miss {
    fn reason(self) -> Reason {
        match self {
            Miss::Metered | Miss::Licence => Reason::ManualInstall,
            Miss::NotVerified => Reason::NotVerified,
            Miss::Failed => Reason::InstallFailed,
            Miss::Occupied | Miss::Older | Miss::NeedsRosetta => Reason::FinishByHand,
            Miss::WontRun => Reason::NoEquivalent,
        }
    }
}

/// The opened download: where its contents are, and how to put it away.
enum Opened {
    Mounted(PathBuf),
    Unpacked(PathBuf),
}

impl MacInstaller<'_> {
    fn call(&self, program: &str, args: Vec<OsString>, stdin: Option<&[u8]>) -> Call {
        Call {
            program: PathBuf::from(program),
            args,
            env: vec![
                ("LC_ALL".into(), "C".into()),
                ("PATH".into(), "/usr/bin:/bin:/usr/sbin:/sbin".into()),
                ("HOME".into(), self.home.clone().into_os_string()),
            ],
            timeout: TOOL_TIME,
            stdin: stdin.map(<[u8]>::to_vec),
            cwd: Some(PathBuf::from("/")),
        }
    }

    fn run(&self, program: &str, args: Vec<OsString>, stdin: Option<&[u8]>) -> Option<Ran> {
        self.runner.run(&self.call(program, args, stdin)).ok()
    }

    fn ok(&self, program: &str, args: Vec<OsString>) -> bool {
        self.run(program, args, None)
            .is_some_and(|r| r.code == Some(0))
    }

    /// Where apps go: `/Applications` when the person may write it, otherwise `~/Applications`.
    fn apps_dir(&self) -> Option<PathBuf> {
        if self.ok(
            "/bin/test",
            vec!["-w".into(), self.system_apps.clone().into_os_string()],
        ) {
            return Some(self.system_apps.clone());
        }
        let mine = self.home.join("Applications");
        std::fs::create_dir_all(&mine).ok()?;
        Some(mine)
    }

    /// Downloads one cask build into the staging folder.
    fn download(
        &self,
        n: usize,
        token: &str,
        url: &str,
        sha: Option<&[u8; 32]>,
    ) -> Result<PathBuf, Miss> {
        if !plain_token(token) {
            return Err(Miss::NotVerified);
        }
        let file = self.staging.join(format!("cask-{n}-{token}"));
        let req = Request {
            url: url.to_string(),
            also_hosts: Vec::new(),
            max_bytes: MAX_DOWNLOAD,
            expected: sha.map(|h| Expected::Sha256(h.iter().map(|b| format!("{b:02x}")).collect())),
            file: file.clone(),
            allow_metered: self.allow_metered,
        };
        match (self.fetch)(&req) {
            Outcome::Done { .. } => Ok(file),
            Outcome::MeteredWait => Err(Miss::Metered),
            Outcome::NotExpectedBytes | Outcome::WebPage | Outcome::Refused { .. } => {
                Err(Miss::NotVerified)
            }
            _ => Err(Miss::Failed),
        }
    }

    /// Opens a download read-only in PCTwin's own folder.
    fn open(&self, n: usize, file: &Path) -> Result<Opened, Miss> {
        let place = self.work.join(format!("open-{n}"));
        std::fs::create_dir_all(&place).map_err(|_| Miss::Failed)?;
        match container_of(file).ok_or(Miss::NotVerified)? {
            Container::Zip => {
                let into = place.join("unpacked");
                let ran = self.ok(
                    "/usr/bin/ditto",
                    vec![
                        "-x".into(),
                        "-k".into(),
                        file.as_os_str().to_owned(),
                        into.clone().into_os_string(),
                    ],
                );
                if ran {
                    Ok(Opened::Unpacked(into))
                } else {
                    Err(Miss::Failed)
                }
            }
            Container::DiskImage => {
                // "q" leaves a licence's pager and "n" declines it: PCTwin never agrees for the
                // person, so a disk image that asks does not attach.
                let ran = self
                    .run(
                        "/usr/bin/hdiutil",
                        vec![
                            "attach".into(),
                            "-plist".into(),
                            "-nobrowse".into(),
                            "-readonly".into(),
                            "-noautoopen".into(),
                            "-mountrandom".into(),
                            place.clone().into_os_string(),
                            file.as_os_str().to_owned(),
                        ],
                        Some(b"qn\n"),
                    )
                    .ok_or(Miss::Failed)?;
                let points = mount_points(&ran.stdout);
                if ran.code != Some(0) {
                    for p in points.iter().filter(|p| p.starts_with(&place)) {
                        self.detach(p);
                    }
                    let words = format!("{} {}", ran.stdout, ran.stderr).to_ascii_lowercase();
                    return Err(if words.contains("licen") || words.contains("agree") {
                        Miss::Licence
                    } else {
                        Miss::Failed
                    });
                }
                // Every volume it attached is put away; the one under PCTwin's folder is used.
                let mut ours = None;
                for p in points {
                    if ours.is_none() && p.starts_with(&place) {
                        ours = Some(p);
                    } else {
                        self.detach(&p);
                    }
                }
                ours.map(Opened::Mounted).ok_or(Miss::Failed)
            }
        }
    }

    fn detach(&self, mount: &Path) {
        let args = |force: bool| {
            let mut a: Vec<OsString> = vec!["detach".into(), mount.as_os_str().to_owned()];
            if force {
                a.push("-force".into());
            }
            a
        };
        if !self.ok("/usr/bin/hdiutil", args(false)) {
            let _ = self.ok("/usr/bin/hdiutil", args(true));
        }
    }

    fn close(&self, opened: &Opened) {
        match opened {
            Opened::Mounted(p) => self.detach(p),
            Opened::Unpacked(p) => {
                let _ = std::fs::remove_dir_all(p);
            }
        }
    }

    /// The bundle read as the scan reads one, and whether it is the old laptop's app: the same
    /// bundle ID, a Team ID on both sides and the same one, a signature that holds, and
    /// Gatekeeper accepting it as notarised by Apple.
    fn same_app(&self, bundle: &Path, had: &FoundApp) -> Result<FoundApp, Miss> {
        let AppMaker::MacTeam { team_id: had_team } = &had.maker else {
            return Err(Miss::NotVerified);
        };
        let meta = std::fs::symlink_metadata(bundle).map_err(|_| Miss::NotVerified)?;
        if !meta.is_dir() {
            return Err(Miss::NotVerified);
        }
        let read = pctwin_apps::found::mac::app_of(bundle, had.owner.clone(), self.runner)
            .map_err(|_| Miss::NotVerified)?;
        let same_id = matches!(
            (&read.found, &had.found),
            (FoundWhere::MacBundle { bundle_id: a, .. }, FoundWhere::MacBundle { bundle_id: b, .. })
                if a == b
        );
        if !same_id {
            return Err(Miss::NotVerified);
        }
        // The Team ID is checked once, below, from the same `codesign` reading the scan uses.
        let facts = pctwin_maker::mac::check_app(self.runner, bundle, CHECK_TIME);
        let notarised = matches!(facts.malware, Malware::CoveredBySystemTrust { .. });
        let same_team = facts.team.as_ref().is_some_and(|t| t.team_id == *had_team);
        if facts.signature != Signature::Trusted || !notarised || !same_team {
            return Err(Miss::NotVerified);
        }
        Ok(read)
    }

    /// Whether the checked app `read` (at `bundle`) runs on this Mac and is not older than the
    /// person's (see the module notes, step 3). `Ok(true)`: it runs through Rosetta.
    fn fits_here(&self, read: &FoundApp, bundle: &Path, had: &FoundApp) -> Result<bool, Miss> {
        if !pctwin_apps::version::not_a_downgrade(read.version.as_deref(), had.version.as_deref()) {
            return Err(Miss::Older);
        }
        let plist = bundle.join("Contents").join("Info.plist");
        let needs = std::fs::read(&plist)
            .ok()
            .and_then(|b| pctwin_apps::found::mac::minimum_system(&b));
        if needs
            .as_deref()
            .is_some_and(|n| macos_at_least(&self.macos, n) != Some(true))
        {
            return Err(Miss::WontRun);
        }
        let arch = read.arch;
        if !arch.known {
            return Err(Miss::NotVerified);
        }
        if self.arm64 {
            if arch.arm64 {
                return Ok(false);
            }
            if !arch.x64 {
                return Err(Miss::WontRun);
            }
            return if self.rosetta {
                Ok(true)
            } else {
                Err(Miss::NeedsRosetta)
            };
        }
        if arch.x64 {
            Ok(false)
        } else {
            Err(Miss::WontRun)
        }
    }

    /// The app at `bundle` checked in full: the old laptop's app, and it fits this Mac.
    fn checked(&self, bundle: &Path, had: &FoundApp) -> Result<(FoundApp, bool), Miss> {
        let read = self.same_app(bundle, had)?;
        let rosetta = self.fits_here(&read, bundle, had)?;
        Ok((read, rosetta))
    }

    /// One cask for `e`: downloaded, opened, checked, copied into PCTwin's own folder beside
    /// its place, checked again there, and moved into place without replacing anything. `Ok`
    /// holds whether it runs through Rosetta.
    fn one_cask(
        &self,
        n: usize,
        e: &AppEntry,
        apps: &Path,
        cask: (&str, &str, Option<&[u8; 32]>, &str),
    ) -> Result<bool, Miss> {
        let (token, url, sha, app) = cask;
        let inside = app_inside(app).ok_or(Miss::NotVerified)?;
        let name = inside.file_name().ok_or(Miss::NotVerified)?.to_owned();
        let dest = apps.join(&name);
        if std::fs::symlink_metadata(&dest).is_ok() {
            return Err(Miss::Occupied);
        }
        let file = self.download(n, token, url, sha)?;
        let opened = self.open(n, &file);
        let _ = std::fs::remove_file(&file);
        let opened = opened?;
        let root = match &opened {
            Opened::Mounted(p) | Opened::Unpacked(p) => p.clone(),
        };
        let placed = reach_inside(&root, &inside)
            .ok_or(Miss::NotVerified)
            .and_then(|source| {
                self.checked(&source, &e.app)?;
                self.place(n, &source, &name, &dest, &e.app)
            });
        self.close(&opened);
        placed
    }

    /// Copies the checked app at `source` into a fresh hidden folder of PCTwin's own in the
    /// folder of `dest`, checks that copy, and moves it to `dest` only if nothing is there.
    /// Only PCTwin's own folder is ever removed.
    fn place(
        &self,
        n: usize,
        source: &Path,
        name: &std::ffi::OsStr,
        dest: &Path,
        had: &FoundApp,
    ) -> Result<bool, Miss> {
        let folder = dest.parent().ok_or(Miss::Failed)?;
        let mut b = [0u8; 8];
        getrandom::fill(&mut b).map_err(|_| Miss::Failed)?;
        let tag: String = b.iter().map(|x| format!("{x:02x}")).collect();
        let mine = folder.join(format!("{PLACING_PREFIX}{n}-{tag}"));
        // A new folder, never one that was there: if the name is taken, nothing goes on.
        std::fs::create_dir(&mine).map_err(|_| Miss::Failed)?;
        let staged = mine.join(name);
        let done = if self.ok(
            "/usr/bin/ditto",
            vec![
                source.as_os_str().to_owned(),
                staged.clone().into_os_string(),
            ],
        ) {
            // What is moved into place is exactly what is checked here.
            self.checked(&staged, had).and_then(|(_, rosetta)| {
                match move_no_replace(&staged, dest) {
                    Ok(()) => Ok(rosetta),
                    Err(e) if e.kind() == std::io::ErrorKind::AlreadyExists => Err(Miss::Occupied),
                    Err(_) => Err(Miss::Failed),
                }
            })
        } else {
            Err(Miss::Failed)
        };
        let _ = std::fs::remove_dir_all(&mine);
        done
    }

    /// The result for an app placed after every check held (see the module notes): reporting
    /// only; nothing is checked or removed here.
    fn verdict(&self, e: &AppEntry, rosetta: bool) -> AppResult {
        let mut todo = after::checklist(
            &e.app,
            e.plan.card.sign_in,
            e.plan.card.licence_key,
            Installed::Done {
                restart_needed: false,
            },
            &[],
        );
        if rosetta {
            todo.push(Todo::Rosetta);
        }
        let result = ItemResult::done(ItemStatus::Reinstalled)
            .unwrap_or_else(|_| ItemResult::because(Reason::NotVerified));
        AppResult::installed(result, todo)
    }

    fn install_one(&self, n: usize, e: &AppEntry, apps: &Path) -> AppResult {
        let casks: Vec<(&str, &str, Option<&[u8; 32]>, &str)> = e
            .plan
            .routes
            .iter()
            .filter_map(|r| match r {
                Route::Cask {
                    token,
                    url,
                    sha256,
                    app,
                    ..
                } => Some((token.as_str(), url.as_str(), sha256.as_ref(), app.as_str())),
                _ => None,
            })
            .collect();
        if casks.is_empty() {
            // The App Store, the maker's page: the person's step; their data is ready.
            return AppResult::because(Reason::ManualInstall);
        }
        let mut last = Miss::Failed;
        for (i, c) in casks.into_iter().enumerate() {
            match self.one_cask(n * 4 + i, e, apps, c) {
                Ok(rosetta) => return self.verdict(e, rosetta),
                // Waiting for a free connection, or a place that is taken: no other cask helps.
                Err(m @ (Miss::Metered | Miss::Occupied)) => return AppResult::because(m.reason()),
                Err(m) => last = m,
            }
        }
        let mut r = AppResult::because(last.reason());
        if last == Miss::NeedsRosetta {
            r.todo.push(Todo::Rosetta);
        }
        r
    }
}

impl Installer for MacInstaller<'_> {
    fn restore_point(&mut self) -> RestorePoint {
        RestorePoint::NotWindows
    }

    fn asked(&self) -> Option<RestorePoint> {
        Some(RestorePoint::NotWindows)
    }

    fn install(&mut self, apps: &[&AppEntry], done: &mut dyn FnMut()) -> Vec<AppResult> {
        let place = self.apps_dir();
        let ready = std::fs::create_dir_all(&self.work).is_ok()
            && std::fs::create_dir_all(&self.staging).is_ok();
        let out = apps
            .iter()
            .enumerate()
            .map(|(n, e)| {
                let r = match (&place, ready) {
                    (Some(place), true) => self.install_one(n, e, place),
                    _ => AppResult::because(Reason::ManualInstall),
                };
                done();
                r
            })
            .collect();
        let _ = std::fs::remove_dir_all(&self.work);
        out
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_cask_app_name_is_a_plain_path_to_one_app() {
        for ok in ["Firefox.app", "Visual Studio Code.app", "Tools/Thing.app"] {
            assert!(app_inside(ok).is_some(), "{ok}");
        }
        for bad in [
            "",
            ".app",
            "../Evil.app",
            "/Applications/Evil.app",
            "Thing",
            "a/../b.app",
            "-x.app",
            "a\\b.app",
            "nul\0.app",
        ] {
            assert!(app_inside(bad).is_none(), "{bad:?}");
        }
    }

    #[test]
    fn tokens_are_plain() {
        assert!(plain_token("visual-studio-code"));
        assert!(plain_token("python@3.12"));
        for bad in ["", "-x", ".x", "a/b", "a b"] {
            assert!(!plain_token(bad), "{bad}");
        }
    }

    #[test]
    fn macos_versions_compare_by_number() {
        assert_eq!(macos_at_least("15.1", "14"), Some(true));
        assert_eq!(macos_at_least("14.0", "14"), Some(true));
        assert_eq!(macos_at_least("13.6.1", "14.0"), Some(false));
        assert_eq!(macos_at_least("10.15", "10.9"), Some(true));
        assert_eq!(macos_at_least("15", "x"), None);
    }

    #[test]
    fn a_download_is_opened_by_its_own_bytes() {
        let d = tempfile::tempdir().unwrap();
        let zip = d.path().join("z");
        std::fs::write(&zip, b"PK\x03\x04rest").unwrap();
        assert_eq!(container_of(&zip), Some(Container::Zip));
        let mut dmg = vec![0u8; 2048];
        dmg[2048 - 512..2048 - 508].copy_from_slice(b"koly");
        let img = d.path().join("i");
        std::fs::write(&img, &dmg).unwrap();
        assert_eq!(container_of(&img), Some(Container::DiskImage));
        let pkg = d.path().join("p");
        std::fs::write(&pkg, b"xar!\0\x1crest-of-a-package").unwrap();
        assert_eq!(container_of(&pkg), None);
        // `koly` anywhere but the trailer is not a disk image.
        let mut early = vec![0u8; 2048];
        early[100..104].copy_from_slice(b"koly");
        std::fs::write(&img, &early).unwrap();
        assert_eq!(container_of(&img), None);
    }

    #[test]
    fn mount_points_are_read_from_hdiutil_plist_after_any_text() {
        let out = "Checksumming...\n<?xml version=\"1.0\" encoding=\"UTF-8\"?>\n<!DOCTYPE plist PUBLIC \"-//Apple//DTD PLIST 1.0//EN\" \"http://www.apple.com/DTDs/PropertyList-1.0.dtd\">\n<plist version=\"1.0\"><dict><key>system-entities</key><array>\n<dict><key>content-hint</key><string>GUID_partition_scheme</string><key>dev-entry</key><string>/dev/disk4</string></dict>\n<dict><key>dev-entry</key><string>/dev/disk4s1</string><key>mount-point</key><string>/tmp/w/open-0/dmg.Ab12</string></dict>\n</array></dict></plist>\n";
        assert_eq!(mount_points(out), [PathBuf::from("/tmp/w/open-0/dmg.Ab12")]);
        assert!(mount_points("hdiutil: attach failed").is_empty());
    }
}
