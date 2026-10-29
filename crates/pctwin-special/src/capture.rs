//! Consistent capture on the old laptop (work package B25-03; plan section 12).
//!
//! A database copied while its app writes can miss committed data or mix two moments; a plain
//! copy plus a matching hash proves only that the bytes agree. So:
//!
//! - **Approval binds a generation** ([`Generation`]): each file's size and change time when the
//!   person approved. A source that changed since is not the approved one
//!   ([`CaptureError::ChangedSinceApproval`], acceptance case T25-009).
//! - **Is the app still writing?** ([`Holders`]): on Windows, a file another program holds open
//!   for writing refuses to be opened by anyone who will not share writing with it (a standard
//!   open, no new system call); on Linux, each of the person's processes' open files are listed
//!   in `/proc`; on a Mac this is not built yet ([`Holding::CannotTell`]). A file held for writing
//!   waits for its app to close ([`CaptureError::AwaitingClose`]).
//! - **SQLite**: the database and its write-ahead log or journal are copied together into a
//!   private snapshot folder, checked steady (no file changed while copied), and then SQLite's
//!   own online backup API writes one complete database from the snapshot (SQLite replays the
//!   log as the app would). The result must pass SQLite's own `quick_check`. The old laptop's
//!   files are only ever read.
//! - **Other stores** (LevelDB, LMDB, an Outlook data file) move only while nothing holds them and
//!   they stay steady; where it cannot be told whether their app is closed, they are kept as a
//!   labelled copy, never "copied" ([`Preservation::PreservedOnly`]).
//! - **Plain files** are copied steady.
//! - **Files named by their own hash** are checked against it while they are read.
//! - **Grants**: a capture takes the 2.5 grant (used up by the call) and checks it names this
//!   plan, root and effect and has not ended; a secret component needs a secret grant naming
//!   exactly it ([`capture_secret`]); an ordinary grant is refused at a secret (T25-005).
//! - **Stopping**: a revoked grant or a cancelled move ([`Stop`]) stops before the next file, and
//!   the partial capture (PCTwin's own files) is removed.

use std::io::{Read, Write};
use std::path::{Path, PathBuf};
use std::sync::Arc;
use std::sync::atomic::{AtomicBool, Ordering};
use std::time::{Duration, SystemTime};

use pctwin_grant::{
    CaptureEffect, CaptureGrant, Now, ObjectRef, PlanSeal, RootRef, SecretEffect, SecretGrant,
};
use serde::{Deserialize, Serialize};
use sha2::Digest;

use crate::classify::hash_name;
use crate::component::{Component, ComponentId, Custody, Evidence, Preservation};
use crate::sniff::{self, Shape};

/// One file's state, cheap to read: its size and when it last changed.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct FileState {
    pub rel: String,
    pub len: u64,
    /// Nanoseconds since 1970 (`None`: the system gives no time).
    pub modified: Option<u128>,
}

/// The files of a component as they were at one moment.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Generation {
    pub files: Vec<FileState>,
    pub digest: [u8; 32],
}

fn joined(base: &Path, rel: &str) -> PathBuf {
    if rel.is_empty() {
        return base.to_path_buf();
    }
    rel.split('/')
        .fold(base.to_path_buf(), |p, part| p.join(part))
}

/// The generation of `members` under `base` now (a missing file counts as size 0, no time).
pub fn generation(base: &Path, members: &[String]) -> Generation {
    let mut files: Vec<FileState> = members
        .iter()
        .map(|rel| {
            let m = std::fs::symlink_metadata(joined(base, rel))
                .ok()
                .filter(|m| m.is_file());
            FileState {
                rel: rel.clone(),
                len: m.as_ref().map_or(0, |m| m.len()),
                modified: m
                    .and_then(|m| m.modified().ok())
                    .and_then(|t| t.duration_since(SystemTime::UNIX_EPOCH).ok())
                    .map(|d| d.as_nanos()),
            }
        })
        .collect();
    files.sort_by(|a, b| a.rel.cmp(&b.rel));
    let mut h = blake3::Hasher::new();
    h.update(b"pctwin-special generation 1\0");
    for f in &files {
        h.update(&(f.rel.len() as u64).to_le_bytes());
        h.update(f.rel.as_bytes());
        h.update(&f.len.to_le_bytes());
        h.update(&f.modified.unwrap_or(u128::MAX).to_le_bytes());
    }
    Generation {
        files,
        digest: *h.finalize().as_bytes(),
    }
}

/// What the person approved for one component: the plan, the component, the approved root it
/// is read from, and its generation then.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Approved {
    pub seal: PlanSeal,
    pub component: ComponentId,
    pub root: RootRef,
    pub generation: Generation,
    /// The component's number in the plan.
    pub handle: u64,
}

impl Approved {
    /// The thing a secret grant must name: this component at this generation.
    pub fn object(&self) -> ObjectRef {
        let mut h = blake3::Hasher::new();
        h.update(b"pctwin-special approved 1\0");
        h.update(&self.component.0);
        h.update(&self.generation.digest);
        ObjectRef::new(self.handle, *h.finalize().as_bytes())
    }
}

/// Whether a program holds files open for writing.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Holding {
    Free,
    /// Held: by these programs, when the system says.
    Held {
        by: Vec<String>,
    },
    /// The system cannot say here.
    CannotTell,
}

/// Asks the system whether files are held open for writing.
pub trait Holders {
    fn holding(&self, files: &[PathBuf]) -> Holding;
}

/// The system's own answer.
#[derive(Debug, Default, Clone, Copy)]
pub struct SystemHolders;

impl Holders for SystemHolders {
    #[cfg(windows)]
    fn holding(&self, files: &[PathBuf]) -> Holding {
        use std::os::windows::fs::OpenOptionsExt;
        const FILE_SHARE_READ: u32 = 1;
        const SHARING_VIOLATION: i32 = 32;
        const LOCK_VIOLATION: i32 = 33;
        for f in files {
            // Opening for reading while refusing to share writing fails if anyone has it open
            // for writing. The handle is closed at once.
            match std::fs::OpenOptions::new()
                .read(true)
                .share_mode(FILE_SHARE_READ)
                .open(f)
            {
                Ok(_) => {}
                Err(e) if matches!(e.raw_os_error(), Some(SHARING_VIOLATION | LOCK_VIOLATION)) => {
                    return Holding::Held { by: Vec::new() };
                }
                Err(e) if e.kind() == std::io::ErrorKind::NotFound => {}
                Err(_) => return Holding::CannotTell,
            }
        }
        Holding::Free
    }

    #[cfg(target_os = "linux")]
    fn holding(&self, files: &[PathBuf]) -> Holding {
        let wanted: Vec<PathBuf> = files
            .iter()
            .filter_map(|f| std::fs::canonicalize(f).ok())
            .collect();
        let Ok(procs) = std::fs::read_dir("/proc") else {
            return Holding::CannotTell;
        };
        let mut by = Vec::new();
        for p in procs.flatten() {
            let name = p.file_name();
            let Some(pid) = name
                .to_str()
                .filter(|n| n.bytes().all(|b| b.is_ascii_digit()))
            else {
                continue;
            };
            if pid == std::process::id().to_string() {
                continue;
            }
            // Only the person's own processes can be read; another account's cannot hold the
            // person's private files open without the person's rights.
            let Ok(fds) = std::fs::read_dir(p.path().join("fd")) else {
                continue;
            };
            for fd in fds.flatten().take(100_000) {
                if std::fs::read_link(fd.path()).is_ok_and(|t| wanted.contains(&t)) {
                    let comm = std::fs::read_to_string(p.path().join("comm")).unwrap_or_default();
                    by.push(comm.trim().to_string());
                    break;
                }
            }
        }
        if by.is_empty() {
            Holding::Free
        } else {
            Holding::Held { by }
        }
    }

    #[cfg(not(any(windows, target_os = "linux")))]
    fn holding(&self, _files: &[PathBuf]) -> Holding {
        Holding::CannotTell
    }
}

/// A move's stop switch: set when the person cancels or a grant is revoked. Work stops before its
/// next file.
#[derive(Debug, Clone, Default)]
pub struct Stop(Arc<AtomicBool>);

impl Stop {
    pub fn new() -> Self {
        Self::default()
    }
    pub fn stop(&self) {
        self.0.store(true, Ordering::SeqCst);
    }
    pub fn stopped(&self) -> bool {
        self.0.load(Ordering::SeqCst)
    }
}

/// How a component was captured.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum Method {
    /// SQLite's online backup from a steady snapshot of the database and its log.
    OnlineBackup,
    /// Every file read while nothing held it and it stayed the same.
    SteadyCopy,
    /// Kept byte for byte, its consistency not established: labelled, never "copied".
    PreservedOnly,
}

/// One file captured: its path inside the component's folder, size and BLAKE3 hash.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct CapturedFile {
    pub rel: String,
    pub len: u64,
    pub hash: [u8; 32],
}

/// A captured component, ready to send.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Capture {
    pub component: ComponentId,
    pub method: Method,
    pub preservation: Preservation,
    /// The generation read, which matched the approved one.
    pub generation: Generation,
    pub files: Vec<CapturedFile>,
    /// Files the capture holds within another (a log SQLite folded into its database).
    pub folded: Vec<String>,
    /// Facts found while capturing (a hash that did not match, a part missing).
    pub evidence: Vec<Evidence>,
    /// The program holding it could not be checked (no answer from the system), and the capture
    /// relied on the files staying steady.
    pub closed_unconfirmed: bool,
}

/// Why a capture did not happen.
#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
pub enum CaptureError {
    #[error("this permission is not for this step: {0}")]
    Denied(&'static str),
    #[error("the data changed after you approved it; look at it again")]
    ChangedSinceApproval,
    #[error("the app that uses it is still open; close it to continue")]
    AwaitingClose { by: Vec<String> },
    #[error("the app kept changing it while it was read; close the app to continue")]
    KeptChanging,
    #[error("stopped as asked; nothing partly read was kept")]
    Stopped,
    #[error("it could not be read: {0}")]
    Unreadable(String),
    #[error("the database did not pass its own check after it was read: {0}")]
    NotConsistent(String),
}

/// How many times a copy is tried again when a file changed while it was read.
const TRIES: u32 = 3;
/// How long to wait between tries.
const SETTLE: Duration = Duration::from_millis(200);

fn check_grant(
    grant: &CaptureGrant,
    approved: &Approved,
    effect: CaptureEffect,
    now: Now,
) -> Result<(), CaptureError> {
    if grant.effect() != effect {
        return Err(CaptureError::Denied("it allows another kind of work"));
    }
    if grant.seal() != approved.seal {
        return Err(CaptureError::Denied("it belongs to another plan"));
    }
    if !grant.roots().contains(&approved.root) {
        return Err(CaptureError::Denied("it does not cover this folder"));
    }
    if grant.expiry().is_over(now) {
        return Err(CaptureError::Denied("it has ended"));
    }
    Ok(())
}

/// Captures an ordinary (not secret) component. `base` is its app data folder on the old laptop,
/// `work` a private, empty PCTwin folder the capture is written into.
#[allow(clippy::too_many_arguments)]
pub fn capture(
    grant: CaptureGrant,
    approved: &Approved,
    component: &Component,
    base: &Path,
    work: &Path,
    holders: &dyn Holders,
    stop: &Stop,
    now: Now,
) -> Result<Capture, CaptureError> {
    check_grant(&grant, approved, CaptureEffect::ReadPrivate, now)?;
    if component.secret {
        return Err(CaptureError::Denied(
            "a secret needs its own permission, naming exactly it",
        ));
    }
    run(approved, component, base, work, holders, stop)
}

/// Captures a secret component the person chose, with a secret grant naming exactly it at its
/// approved generation.
#[allow(clippy::too_many_arguments)]
pub fn capture_secret(
    grant: SecretGrant,
    approved: &Approved,
    component: &Component,
    base: &Path,
    work: &Path,
    holders: &dyn Holders,
    stop: &Stop,
    now: Now,
) -> Result<Capture, CaptureError> {
    if grant.effect() != SecretEffect::Read {
        return Err(CaptureError::Denied("it allows another kind of work"));
    }
    if grant.seal() != approved.seal {
        return Err(CaptureError::Denied("it belongs to another plan"));
    }
    if !grant.components().contains(&approved.object()) {
        return Err(CaptureError::Denied(
            "it does not name this secret as approved",
        ));
    }
    if grant.expiry().is_over(now) {
        return Err(CaptureError::Denied("it has ended"));
    }
    if !component.secret {
        return Err(CaptureError::Denied(
            "a secret permission is only for secrets",
        ));
    }
    run(approved, component, base, work, holders, stop)
}

/// Keeps a component that cannot be captured consistently (its app stays open) as a labelled,
/// byte-for-byte copy, when the person asks for that. Never "copied".
#[allow(clippy::too_many_arguments)]
pub fn preserve_as_is(
    grant: CaptureGrant,
    approved: &Approved,
    component: &Component,
    base: &Path,
    work: &Path,
    stop: &Stop,
    now: Now,
) -> Result<Capture, CaptureError> {
    check_grant(&grant, approved, CaptureEffect::ReadPrivate, now)?;
    if component.secret {
        return Err(CaptureError::Denied(
            "a secret needs its own permission, naming exactly it",
        ));
    }
    copyable(component)?;
    let generation = generation(base, &component.members);
    if generation.digest != approved.generation.digest {
        return Err(CaptureError::ChangedSinceApproval);
    }
    let files = copy_all(base, work, &component.members, stop)?;
    Ok(Capture {
        component: component.id,
        method: Method::PreservedOnly,
        preservation: Preservation::PreservedOnly,
        generation,
        files: files.0,
        folded: Vec::new(),
        evidence: files.1,
        closed_unconfirmed: true,
    })
}

/// Only P and E components, and S ones kept as a labelled copy, have bytes to capture.
fn copyable(component: &Component) -> Result<(), CaptureError> {
    match component.custody {
        Custody::Portable | Custody::NeedsExport => Ok(()),
        Custody::ServiceHeld if component.preserve => Ok(()),
        Custody::ServiceHeld => Err(CaptureError::Denied("the service holds it: sign in again")),
        Custody::MachineBound => Err(CaptureError::Denied(
            "it is bound to this laptop or a device and is never copied",
        )),
        Custody::Unresolved => Err(CaptureError::Denied("what it is could not be established")),
    }
}

fn run(
    approved: &Approved,
    component: &Component,
    base: &Path,
    work: &Path,
    holders: &dyn Holders,
    stop: &Stop,
) -> Result<Capture, CaptureError> {
    copyable(component)?;
    if approved.component != component.id {
        return Err(CaptureError::Denied(
            "it was approved for another component",
        ));
    }
    let gen_now = generation(base, &component.members);
    if gen_now.digest != approved.generation.digest {
        return Err(CaptureError::ChangedSinceApproval);
    }
    let paths: Vec<PathBuf> = component.members.iter().map(|m| joined(base, m)).collect();
    let holding = holders.holding(&paths);
    if let Holding::Held { by } = holding {
        return Err(CaptureError::AwaitingClose { by });
    }
    let unconfirmed = holding == Holding::CannotTell;
    let shapes: Vec<Shape> = paths.iter().map(|p| shape_of(p)).collect();
    // Each database, with the log or journal SQLite keeps beside it by its own naming rule.
    let dbs: Vec<String> = component
        .members
        .iter()
        .zip(&shapes)
        .filter(|(_, s)| **s == Shape::Sqlite)
        .map(|(m, _)| m.clone())
        .collect();
    let companions = |db: &str| [format!("{db}-wal"), format!("{db}-journal")];
    let folded: Vec<String> = component
        .members
        .iter()
        .filter(|m| dbs.iter().any(|d| companions(d).contains(m)))
        .cloned()
        .collect();
    let rest: Vec<String> = component
        .members
        .iter()
        .filter(|m| !dbs.contains(m) && !folded.contains(m))
        .cloned()
        .collect();
    let live = shapes
        .iter()
        .any(|s| s.is_live_store() && *s != Shape::Sqlite)
        || shapes
            .iter()
            .any(|s| matches!(s, Shape::OutlookData { .. }));
    if live && unconfirmed {
        // Whether its app is closed cannot be told here: a labelled copy, never "copied".
        let (files, evidence) = steady_copy(base, work, &component.members, stop)?;
        return Ok(Capture {
            component: component.id,
            method: Method::PreservedOnly,
            preservation: Preservation::PreservedOnly,
            generation: gen_now,
            files,
            folded: Vec::new(),
            evidence,
            closed_unconfirmed: true,
        });
    }
    let (mut files, mut evidence) = steady_copy(base, work, &rest, stop)?;
    for db in &dbs {
        let group: Vec<String> = std::iter::once(db.clone())
            .chain(
                companions(db)
                    .into_iter()
                    .filter(|c| component.members.contains(c)),
            )
            .collect();
        match sqlite_one(base, work, db, &group, stop) {
            Ok(f) => files.push(f),
            Err(e) => {
                remove_written(work, &files);
                return Err(e);
            }
        }
    }
    let partial = evidence
        .iter()
        .any(|e| matches!(e, Evidence::HashMismatch | Evidence::MissingPart { .. }));
    evidence.dedup();
    Ok(Capture {
        component: component.id,
        method: if dbs.is_empty() {
            Method::SteadyCopy
        } else {
            Method::OnlineBackup
        },
        preservation: if partial {
            Preservation::Partial
        } else {
            Preservation::Captured
        },
        generation: gen_now,
        files,
        folded,
        evidence,
        closed_unconfirmed: unconfirmed,
    })
}

fn shape_of(p: &Path) -> Shape {
    let Ok(mut f) = std::fs::File::open(p) else {
        return Shape::Empty;
    };
    let mut head = vec![0u8; sniff::HEAD];
    let n = f.read(&mut head).unwrap_or(0);
    head.truncate(n);
    sniff::shape(&head, &[], n as u64)
}

/// Copies `members` from `base` into `work`, hashing; files named by their SHA-256 are checked.
fn copy_all(
    base: &Path,
    work: &Path,
    members: &[String],
    stop: &Stop,
) -> Result<(Vec<CapturedFile>, Vec<Evidence>), CaptureError> {
    let mut out = Vec::new();
    let mut evidence = Vec::new();
    for rel in members {
        if stop.stopped() {
            remove_written(work, &out);
            return Err(CaptureError::Stopped);
        }
        let from = joined(base, rel);
        let to = joined(work, rel);
        let unreadable = |e: std::io::Error| CaptureError::Unreadable(e.kind().to_string());
        if let Some(parent) = to.parent() {
            std::fs::create_dir_all(parent).map_err(unreadable)?;
        }
        let mut src = match std::fs::File::open(&from) {
            Ok(f) => f,
            Err(e) if e.kind() == std::io::ErrorKind::NotFound => {
                evidence.push(Evidence::MissingPart { what: rel.clone() });
                continue;
            }
            Err(e) => return Err(unreadable(e)),
        };
        let mut dst = match std::fs::OpenOptions::new()
            .write(true)
            .create_new(true)
            .open(&to)
        {
            Ok(f) => f,
            Err(e) => {
                remove_written(work, &out);
                return Err(unreadable(e));
            }
        };
        let mut b3 = blake3::Hasher::new();
        let mut s256 = sha2::Sha256::new();
        let mut buf = vec![0u8; 1 << 20];
        let mut len = 0u64;
        loop {
            let n = src.read(&mut buf).map_err(unreadable)?;
            if n == 0 {
                break;
            }
            b3.update(&buf[..n]);
            s256.update(&buf[..n]);
            dst.write_all(&buf[..n]).map_err(unreadable)?;
            len += n as u64;
        }
        dst.sync_all().map_err(unreadable)?;
        let name = rel.rsplit('/').next().unwrap_or(rel);
        if let Some(want) = hash_name(name) {
            let got: String = s256.finalize().iter().map(|b| format!("{b:02x}")).collect();
            if got != want {
                evidence.push(Evidence::HashMismatch);
                evidence.push(Evidence::MissingPart {
                    what: format!("{rel} (its content does not match its name)"),
                });
            }
        }
        out.push(CapturedFile {
            rel: rel.clone(),
            len,
            hash: *b3.finalize().as_bytes(),
        });
    }
    Ok((out, evidence))
}

/// Removes the files a capture wrote into `work` (never the folder itself, which the caller made
/// private).
fn remove_written(work: &Path, files: &[CapturedFile]) {
    for f in files {
        let _ = std::fs::remove_file(joined(work, &f.rel));
    }
}

/// Copies each of `members` into `work`, checking each stayed the same while it was read (tried
/// again a few times). A file that keeps changing is left out and named, so the capture is
/// partial; when every file keeps changing, nothing is captured.
fn steady_copy(
    base: &Path,
    work: &Path,
    members: &[String],
    stop: &Stop,
) -> Result<(Vec<CapturedFile>, Vec<Evidence>), CaptureError> {
    let mut files = Vec::new();
    let mut evidence = Vec::new();
    let mut changing = 0usize;
    for m in members {
        let one = std::slice::from_ref(m);
        let mut done = false;
        for attempt in 0..TRIES {
            let before = generation(base, one);
            let got = match copy_all(base, work, one, stop) {
                Ok(g) => g,
                Err(e) => {
                    remove_written(work, &files);
                    return Err(e);
                }
            };
            let after = generation(base, one);
            if before == after && sizes_match(&before, &got.0) {
                files.extend(got.0);
                evidence.extend(got.1);
                done = true;
                break;
            }
            remove_written(work, &got.0);
            if attempt + 1 < TRIES {
                std::thread::sleep(SETTLE);
            }
        }
        if !done {
            changing += 1;
            evidence.push(Evidence::MissingPart {
                what: format!("{m} (it kept changing while it was read)"),
            });
        }
    }
    if changing > 0 && changing == members.len() {
        remove_written(work, &files);
        return Err(CaptureError::KeptChanging);
    }
    Ok((files, evidence))
}

fn sizes_match(g: &Generation, files: &[CapturedFile]) -> bool {
    files
        .iter()
        .all(|f| g.files.iter().any(|s| s.rel == f.rel && s.len == f.len))
}

/// One database: a steady snapshot of it with its log or journal (`group`) in a private folder,
/// then SQLite's online backup (which replays the log) into one complete database in `work`,
/// which must pass SQLite's own `quick_check`.
fn sqlite_one(
    base: &Path,
    work: &Path,
    db: &str,
    group: &[String],
    stop: &Stop,
) -> Result<CapturedFile, CaptureError> {
    let snap = tempfile::tempdir().map_err(|e| CaptureError::Unreadable(e.kind().to_string()))?;
    // The database and its log must be steady together.
    let mut steady = false;
    for attempt in 0..TRIES {
        let before = generation(base, group);
        let (got, _) = copy_all(base, snap.path(), group, stop)?;
        let after = generation(base, group);
        if before == after && sizes_match(&before, &got) {
            steady = true;
            break;
        }
        remove_written(snap.path(), &got);
        if attempt + 1 < TRIES {
            std::thread::sleep(SETTLE);
        }
    }
    if !steady {
        return Err(CaptureError::KeptChanging);
    }
    if stop.stopped() {
        return Err(CaptureError::Stopped);
    }
    let src = joined(snap.path(), db);
    let out = joined(work, db);
    if let Some(parent) = out.parent() {
        std::fs::create_dir_all(parent)
            .map_err(|e| CaptureError::Unreadable(e.kind().to_string()))?;
    }
    // The steady copies' bytes, bounded, go to the parser process: there the log is laid over
    // the pages in memory and SQLite's own quick_check must pass. SQLite never opens another
    // app's database in this process (adoption change 1).
    let unreadable = |e: std::io::Error| CaptureError::Unreadable(e.kind().to_string());
    let journal = joined(snap.path(), &format!("{db}-journal"));
    if std::fs::metadata(&journal).is_ok_and(|m| m.len() > 0) {
        // A change was being written when it was copied (a rollback journal): not this copy.
        return Err(CaptureError::NotConsistent(
            "a change was being written to it when it was copied".into(),
        ));
    }
    let wal = joined(snap.path(), &format!("{db}-wal"));
    let too_large = |e: std::io::Error| {
        if e.to_string() == "too large" {
            CaptureError::NotConsistent("too large to check".into())
        } else {
            unreadable(e)
        }
    };
    let db_bytes = crate::guard::read_all(&src, crate::content::MAX_DB).map_err(too_large)?;
    let log = if std::fs::metadata(&wal).is_ok_and(|m| m.len() > 0) {
        Some(crate::guard::read_all(&wal, crate::content::MAX_LOG).map_err(too_large)?)
    } else {
        None
    };
    let bytes = crate::content::snapshot_of(&db_bytes, log.as_deref())
        .map_err(|e| CaptureError::NotConsistent(e.to_string()))?;
    std::fs::write(&out, &bytes).map_err(unreadable)?;
    Ok(CapturedFile {
        rel: db.to_string(),
        len: bytes.len() as u64,
        hash: *blake3::hash(&bytes).as_bytes(),
    })
}
