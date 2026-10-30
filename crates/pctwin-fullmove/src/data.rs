//! App data (2.5) moving with the move: captured on the old laptop, carried in the sealed link
//! before the files, checked file by file into PCTwin's own staging folder on the new laptop, and
//! landed there through the safety gate and the imports journal, with undo.
//!
//! - **The choice** ([`Choice`]): the new laptop names the components to bring: every one that
//!   copies (proposed by 2.5), and the secrets the person ticked one by one (D07; none by
//!   default). Nothing else is read on the old laptop.
//! - **The old laptop** ([`Source`]): each chosen component is captured with 2.5's own grant for
//!   exactly this choice (a secret with a secret grant naming exactly it): SQLite through its
//!   online backup from a steady snapshot, other files only while no program holds them and they
//!   stay the same. One that cannot be captured is said, with why, never sent half.
//! - **The stream**: per component, its capture record, then each captured file. The new laptop
//!   ([`Sink`]) keeps a file only if its path is one the record names, inside the component's own
//!   staging folder, and its size and BLAKE3 hash are the record's.
//! - **Landing** ([`land_all`]): 2.5's `apply` lands each complete component under the person's
//!   home, never over an existing folder (beside it, said), secrets owner-only, journaled by the
//!   imports role. Its result word is 2.5's own, from what really happened.

use std::collections::BTreeMap;
use std::io::Write;
use std::path::{Path, PathBuf};
use std::time::Duration;

use pctwin_gate::Destination;
use pctwin_grant::{CaptureEffect, CaptureKey, Now, PlanSeal, RootRef, SecretEffect};
use pctwin_journal::{Actor, Journal, PlannedEffect, Scope};
use pctwin_recipe::{RecipeId, RecipeRef};
use pctwin_record::ItemId;
use pctwin_special::api::{self, Grant, MadeFolders, Planned};
use pctwin_special::capture::{Approved, Capture, CaptureError, Holders, Stop};
use pctwin_special::classify::Placement;
use pctwin_special::component::Component;
use pctwin_special::handoff::Step;
use pctwin_special::land::{Conflict, OwnerOnly};
use pctwin_special::roots::{RootKind, System};
use pctwin_types::{ItemResult, Reason};
use serde::{Deserialize, Serialize};

/// The longest capture record, in bytes.
pub const MAX_RECORD: usize = 16 * 1024 * 1024;
/// How long one grant lasts.
const GRANT: Duration = pctwin_grant::MAX_LIFETIME;

/// A component's ID as it crosses: its 32 bytes in hexadecimal.
pub fn id_hex(c: &Component) -> String {
    c.id.0.iter().map(|b| format!("{b:02x}")).collect()
}

/// What the new laptop chose.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Choice {
    /// Components that copy (never a secret).
    pub copy: Vec<String>,
    /// Secrets the person ticked, one by one.
    pub secrets: Vec<String>,
}

impl Choice {
    pub fn encode(&self) -> Vec<u8> {
        serde_json::to_vec(self).unwrap_or_default()
    }
    pub fn decode(b: &[u8]) -> Self {
        serde_json::from_slice(b).unwrap_or_default()
    }
}

/// Where a component goes, as it crosses.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "kebab-case", deny_unknown_fields)]
pub enum PlacementWire {
    Same { root: RootKind, folder: String },
    Translated { root: RootKind, folder: String },
    PreservedOnly,
}

impl From<&Placement> for PlacementWire {
    fn from(p: &Placement) -> Self {
        match p {
            Placement::Same { root, folder } => PlacementWire::Same {
                root: *root,
                folder: folder.clone(),
            },
            Placement::Translated { root, folder } => PlacementWire::Translated {
                root: *root,
                folder: folder.clone(),
            },
            Placement::PreservedOnly => PlacementWire::PreservedOnly,
        }
    }
}

impl From<&PlacementWire> for Placement {
    fn from(p: &PlacementWire) -> Self {
        match p {
            PlacementWire::Same { root, folder } => Placement::Same {
                root: *root,
                folder: folder.clone(),
            },
            PlacementWire::Translated { root, folder } => Placement::Translated {
                root: *root,
                folder: folder.clone(),
            },
            PlacementWire::PreservedOnly => Placement::PreservedOnly,
        }
    }
}

/// Why a chosen component was not captured, in a word the screens translate.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum NotCaptured {
    /// Its app is still open on the old laptop.
    AppOpen,
    /// It changed after the approval, or kept changing while read.
    Changing,
    /// It could not be read, or did not pass its own check.
    Unreadable,
    /// The move was stopped, or the permission did not fit.
    Stopped,
}

impl From<&CaptureError> for NotCaptured {
    fn from(e: &CaptureError) -> Self {
        match e {
            CaptureError::AwaitingClose { .. } => NotCaptured::AppOpen,
            CaptureError::ChangedSinceApproval | CaptureError::KeptChanging => {
                NotCaptured::Changing
            }
            CaptureError::Unreadable(_) | CaptureError::NotConsistent(_) => NotCaptured::Unreadable,
            CaptureError::Stopped | CaptureError::Denied(_) => NotCaptured::Stopped,
        }
    }
}

/// One file header in the stream.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "kebab-case", deny_unknown_fields)]
pub enum Header {
    /// The component's capture record follows as this file's bytes ([`Record`]).
    Record { id: String },
    /// One captured file of the component.
    File { id: String, rel: String },
    /// The component was chosen but not captured.
    NotCaptured { id: String, why: NotCaptured },
}

/// A component's capture record.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Record {
    pub app: String,
    pub component: Component,
    pub placement: PlacementWire,
    pub root: RootKind,
    pub capture: Capture,
}

/* ---------- The old laptop ---------- */

/// What the old laptop says while it captures: how many of the chosen apps are done, and the
/// apps waiting to be closed (their names), if any.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields, rename_all = "camelCase")]
pub struct DataProgress {
    pub done: u64,
    pub total: u64,
    #[serde(default)]
    pub waiting_for: Vec<String>,
}

impl DataProgress {
    pub fn encode(&self) -> Vec<u8> {
        serde_json::to_vec(self).unwrap_or_default()
    }
    /// A note from the other laptop, read strictly and bounded.
    pub fn decode(b: &[u8]) -> Option<Self> {
        let p: DataProgress = serde_json::from_slice(b).ok()?;
        (p.done <= p.total
            && p.waiting_for.len() <= 64
            && p.waiting_for.iter().all(|n| n.len() <= 512))
        .then_some(p)
    }
}

/// Who is told the old laptop's progress with the app data.
pub type Tell = Box<dyn FnMut(&DataProgress) + Send>;

/// The person's answer when apps must be closed first: try those again, or go on without them.
pub type AskClose = Box<dyn FnMut(&[String]) -> bool + Send>;

/// One thing for the stream.
enum Item {
    /// A header, and the file its bytes come from (none: no bytes).
    File(Vec<u8>, Option<PathBuf>),
}

/// The old laptop's app data, given the new laptop's choice (read only). Capturing runs on a
/// thread of its own, so the link carries progress while a big database is backed up.
pub struct Source {
    planned: Vec<Planned>,
    key: &'static CaptureKey,
    holders: std::sync::Arc<dyn Holders + Send + Sync>,
    /// PCTwin's own private work folder on the old laptop (removed when done).
    work: PathBuf,
    stop: Stop,
    ask: Option<AskClose>,
    rx: Option<std::sync::mpsc::Receiver<Item>>,
    progress: std::sync::Arc<std::sync::Mutex<DataProgress>>,
}

impl Source {
    pub fn new(
        planned: Vec<Planned>,
        key: &'static CaptureKey,
        holders: std::sync::Arc<dyn Holders + Send + Sync>,
        work: PathBuf,
    ) -> Self {
        Self {
            planned,
            key,
            holders,
            work,
            stop: Stop::new(),
            ask: None,
            rx: None,
            progress: Default::default(),
        }
    }

    /// Asks the person, when chosen apps are still open, whether to try them again once closed.
    pub fn asking(mut self, ask: AskClose) -> Self {
        self.ask = Some(ask);
        self
    }
}

fn header(h: &Header) -> Vec<u8> {
    serde_json::to_vec(h).unwrap_or_default()
}

/// What capturing one component gave: the items to send, or why not.
enum Captured {
    Items(Vec<Item>),
    Not(NotCaptured),
}

/// Captures the component `p` (number `at`; `secret`: with a secret grant naming exactly it),
/// for the choice sealed as `seal`.
#[allow(clippy::too_many_arguments)]
fn capture_one(
    p: &Planned,
    at: usize,
    secret: bool,
    seal: PlanSeal,
    key: &CaptureKey,
    holders: &dyn Holders,
    work: &Path,
    stop: &Stop,
) -> Captured {
    let id = id_hex(&p.component);
    let root = RootRef::new(1);
    let approved = api::approve(p, seal, root, at as u64);
    let folder = work.join(at.to_string());
    let _ = std::fs::remove_dir_all(&folder);
    if std::fs::create_dir_all(&folder).is_err() {
        return Captured::Not(NotCaptured::Unreadable);
    }
    let grant = if secret {
        key.secret(
            seal,
            vec![approved.object()],
            SecretEffect::Read,
            GRANT,
            Now::real(),
        )
        .map(Grant::Secret)
    } else {
        key.grant(
            seal,
            vec![root],
            CaptureEffect::ReadPrivate,
            GRANT,
            Now::real(),
        )
        .map(Grant::Ordinary)
    };
    let Ok(grant) = grant else {
        return Captured::Not(NotCaptured::Stopped);
    };
    match api::capture_planned(p, grant, &approved, &folder, holders, stop, Now::real()) {
        Ok(capture) => {
            let record = Record {
                app: p.app.clone(),
                component: p.component.clone(),
                placement: (&p.placement).into(),
                root: p.root,
                capture: capture.clone(),
            };
            let body = work.join(format!("{at}.record"));
            if std::fs::write(&body, serde_json::to_vec(&record).unwrap_or_default()).is_err() {
                return Captured::Not(NotCaptured::Unreadable);
            }
            let mut items = vec![Item::File(
                header(&Header::Record { id: id.clone() }),
                Some(body),
            )];
            for f in &capture.files {
                let path = f
                    .rel
                    .split('/')
                    .fold(folder.clone(), |path, part| path.join(part));
                items.push(Item::File(
                    header(&Header::File {
                        id: id.clone(),
                        rel: f.rel.clone(),
                    }),
                    Some(path),
                ));
            }
            Captured::Items(items)
        }
        Err(e) => Captured::Not((&e).into()),
    }
}

impl Drop for Source {
    fn drop(&mut self) {
        self.stop.stop();
        // The thread may still be reading: its folder goes when it is done, or here.
        let _ = std::fs::remove_dir_all(&self.work);
    }
}

impl pctwin_move::extras::DataOut for Source {
    fn chosen(&mut self, choice: Vec<u8>) {
        let seal = PlanSeal::new(
            1,
            *blake3::Hasher::new_derive_key("PCTwin app data choice 2026-10-28 v1")
                .update(&choice)
                .finalize()
                .as_bytes(),
        );
        let c = Choice::decode(&choice);
        // Only what was chosen is read: a component that copies (never a secret by this list),
        // and each secret the person ticked by itself.
        let todo: Vec<(usize, bool)> = self
            .planned
            .iter()
            .enumerate()
            .filter_map(|(at, p)| {
                let id = id_hex(&p.component);
                if !p.component.secret && c.copy.contains(&id) {
                    Some((at, false))
                } else if p.component.secret && c.secrets.contains(&id) {
                    Some((at, true))
                } else {
                    None
                }
            })
            .collect();
        let apps: std::collections::BTreeSet<&str> = todo
            .iter()
            .map(|(at, _)| self.planned[*at].app.as_str())
            .collect();
        if let Ok(mut p) = self.progress.lock() {
            p.total = apps.len() as u64;
        }
        let (tx, rx) = std::sync::mpsc::channel();
        self.rx = Some(rx);
        let planned = self.planned.clone();
        let (key, holders, work, stop) = (
            self.key,
            self.holders.clone(),
            self.work.clone(),
            self.stop.clone(),
        );
        let progress = self.progress.clone();
        let mut ask = self.ask.take();
        let _ = std::thread::Builder::new()
            .name("pctwin-capture".into())
            .spawn(move || {
                let mut done_apps: std::collections::BTreeSet<String> = Default::default();
                let mut open: Vec<(usize, bool)> = Vec::new();
                let send = |items: Vec<Item>| items.into_iter().all(|i| tx.send(i).is_ok());
                let mark = |app: &str, done_apps: &mut std::collections::BTreeSet<String>| {
                    done_apps.insert(app.to_string());
                    if let Ok(mut p) = progress.lock() {
                        p.done = (done_apps.len() as u64).min(p.total);
                    }
                };
                for (at, secret) in todo {
                    if stop.stopped() {
                        return;
                    }
                    let p = &planned[at];
                    match capture_one(p, at, secret, seal, key, &*holders, &work, &stop) {
                        Captured::Items(items) => {
                            if !send(items) {
                                return;
                            }
                        }
                        Captured::Not(NotCaptured::AppOpen) => {
                            open.push((at, secret));
                            continue;
                        }
                        Captured::Not(why) => {
                            let id = id_hex(&p.component);
                            if !send(vec![Item::File(
                                header(&Header::NotCaptured { id, why }),
                                None,
                            )]) {
                                return;
                            }
                        }
                    }
                    mark(&p.app, &mut done_apps);
                }
                // Apps still open: the person is asked to close them, and they are tried again
                // as often as the person asks; otherwise they are said as not captured.
                while !open.is_empty() && !stop.stopped() {
                    let names: Vec<String> = {
                        let set: std::collections::BTreeSet<&str> = open
                            .iter()
                            .map(|(at, _)| planned[*at].app.as_str())
                            .collect();
                        set.into_iter().map(str::to_string).collect()
                    };
                    if let Ok(mut p) = progress.lock() {
                        p.waiting_for = names.clone();
                    }
                    let again = ask.as_mut().is_some_and(|ask| ask(&names));
                    if let Ok(mut p) = progress.lock() {
                        p.waiting_for.clear();
                    }
                    let mut still = Vec::new();
                    for (at, secret) in std::mem::take(&mut open) {
                        let p = &planned[at];
                        let id = id_hex(&p.component);
                        let got = if again {
                            capture_one(p, at, secret, seal, key, &*holders, &work, &stop)
                        } else {
                            Captured::Not(NotCaptured::AppOpen)
                        };
                        match got {
                            Captured::Items(items) => {
                                if !send(items) {
                                    return;
                                }
                                mark(&p.app, &mut done_apps);
                            }
                            Captured::Not(NotCaptured::AppOpen) if again => {
                                still.push((at, secret))
                            }
                            Captured::Not(why) => {
                                if !send(vec![Item::File(
                                    header(&Header::NotCaptured { id, why }),
                                    None,
                                )]) {
                                    return;
                                }
                                mark(&p.app, &mut done_apps);
                            }
                        }
                    }
                    open = still;
                }
            });
    }

    fn next_within(&mut self, wait: std::time::Duration) -> pctwin_move::extras::NextData {
        use pctwin_move::extras::NextData;
        let Some(rx) = &self.rx else {
            return NextData::Done;
        };
        match rx.recv_timeout(wait) {
            Ok(Item::File(h, path)) => {
                let reader: Box<dyn std::io::Read + Send> = match path {
                    Some(p) => match std::fs::File::open(&p) {
                        Ok(f) => Box::new(f),
                        Err(_) => Box::new(std::io::empty()),
                    },
                    None => Box::new(std::io::empty()),
                };
                NextData::File(h, reader)
            }
            Err(std::sync::mpsc::RecvTimeoutError::Timeout) => {
                NextData::Wait(self.progress.lock().map(|p| p.encode()).unwrap_or_default())
            }
            Err(std::sync::mpsc::RecvTimeoutError::Disconnected) => NextData::Done,
        }
    }

    fn lost(&mut self) {
        self.stop.stop();
    }
}

/* ---------- The new laptop ---------- */

/// One component as it arrived.
#[derive(Debug, Clone, Default)]
pub struct Arrived {
    pub record: Option<Record>,
    /// Files received whole and matching the record, by path.
    pub files: BTreeMap<String, bool>,
    pub not_captured: Option<NotCaptured>,
    /// Something did not match (a path, a size, a hash): it is not landed.
    pub refused: bool,
}

enum Open {
    Record {
        id: String,
        bytes: Vec<u8>,
    },
    File {
        id: String,
        rel: String,
        path: PathBuf,
        file: Option<std::fs::File>,
        len: u64,
        hash: Box<blake3::Hasher>,
    },
    Dropped,
}

/// The new laptop's side: the choice it sends, and what arrives, in PCTwin's own staging folder.
pub struct Sink {
    pub choice: Choice,
    staging: PathBuf,
    pub arrived: BTreeMap<String, Arrived>,
    open: Option<Open>,
    /// The connection was lost part way through the app data.
    pub lost: bool,
    on_progress: Option<Tell>,
}

/// Whether `rel` is a plain path inside a folder: names separated by `/`, none empty, `.` or
/// `..`, and no drive, colon or backslash.
fn plain_rel(rel: &str) -> bool {
    !rel.is_empty()
        && rel.len() <= 4096
        && !rel.contains(['\\', ':', '\0'])
        && rel
            .split('/')
            .all(|p| !p.is_empty() && p != "." && p != "..")
}

impl Sink {
    pub fn new(choice: Choice, staging: PathBuf) -> Self {
        Self {
            choice,
            staging,
            arrived: BTreeMap::new(),
            open: None,
            lost: false,
            on_progress: None,
        }
    }

    /// Tells `tell` the old laptop's progress while it captures.
    pub fn telling(mut self, tell: Tell) -> Self {
        self.on_progress = Some(tell);
        self
    }

    /// Whether the old laptop refused component `id` because it changed after the approval, or
    /// kept changing while it was read (the capture's own check: `ChangedSinceApproval`,
    /// `KeptChanging`).
    pub fn changed_after_approval(&self, id: &str) -> bool {
        self.arrived
            .get(id)
            .is_some_and(|a| a.not_captured == Some(NotCaptured::Changing))
    }

    fn chosen(&self, id: &str) -> bool {
        self.choice.copy.iter().any(|c| c == id) || self.choice.secrets.iter().any(|c| c == id)
    }

    /// Removes the staging folder (after landing: what landed is in the person's home now).
    pub fn remove_staging(&self) {
        let _ = std::fs::remove_dir_all(&self.staging);
    }

    /// The staging folder itself.
    pub fn staging(&self) -> &Path {
        &self.staging
    }

    /// The staging folder of component `id`.
    pub fn folder(&self, id: &str) -> PathBuf {
        self.staging.join(id)
    }
}

impl pctwin_move::extras::DataIn for Sink {
    fn choice(&mut self) -> Vec<u8> {
        self.choice.encode()
    }

    fn progress(&mut self, note: &[u8]) {
        if let (Some(p), Some(tell)) = (DataProgress::decode(note), self.on_progress.as_mut()) {
            tell(&p);
        }
    }

    fn lost(&mut self) {
        self.lost = true;
        // A file part way is not kept.
        if let Some(Open::File { id, path, .. }) = self.open.take() {
            let _ = std::fs::remove_file(&path);
            self.arrived.entry(id).or_default().refused = true;
        }
    }

    fn begin(&mut self, header: &[u8]) -> Result<(), String> {
        let Ok(h) = serde_json::from_slice::<Header>(header) else {
            self.open = Some(Open::Dropped);
            return Ok(());
        };
        self.open = Some(match h {
            Header::Record { id } if self.chosen(&id) => Open::Record {
                id,
                bytes: Vec::new(),
            },
            Header::File { id, rel } if self.chosen(&id) && plain_rel(&rel) => {
                let names = self
                    .arrived
                    .get(&id)
                    .and_then(|a| a.record.as_ref())
                    .is_some_and(|r| r.capture.files.iter().any(|f| f.rel == rel));
                if !names {
                    self.arrived.entry(id).or_default().refused = true;
                    Open::Dropped
                } else {
                    let path = rel
                        .split('/')
                        .fold(self.folder(&id), |path, part| path.join(part));
                    let file = path
                        .parent()
                        .and_then(|d| std::fs::create_dir_all(d).ok())
                        .and_then(|()| {
                            std::fs::OpenOptions::new()
                                .write(true)
                                .create_new(true)
                                .open(&path)
                                .ok()
                        });
                    Open::File {
                        id,
                        rel,
                        path,
                        file,
                        len: 0,
                        hash: Box::new(blake3::Hasher::new()),
                    }
                }
            }
            Header::NotCaptured { id, why } if self.chosen(&id) => {
                self.arrived.entry(id).or_default().not_captured = Some(why);
                Open::Dropped
            }
            _ => Open::Dropped,
        });
        Ok(())
    }

    fn chunk(&mut self, bytes: &[u8]) -> Result<(), String> {
        match &mut self.open {
            Some(Open::Record { bytes: b, .. }) => {
                if b.len() + bytes.len() > MAX_RECORD {
                    self.open = Some(Open::Dropped);
                } else {
                    b.extend_from_slice(bytes);
                }
            }
            Some(Open::File {
                file, len, hash, ..
            }) => {
                *len += bytes.len() as u64;
                hash.update(bytes);
                if let Some(f) = file
                    && f.write_all(bytes).is_err()
                {
                    *file = None;
                }
            }
            _ => {}
        }
        Ok(())
    }

    fn end(&mut self) -> Result<(), String> {
        match self.open.take() {
            Some(Open::Record { id, bytes }) => {
                let a = self.arrived.entry(id.clone()).or_default();
                match serde_json::from_slice::<Record>(&bytes) {
                    // The record must be the component it says it is.
                    Ok(r)
                        if id_hex(&r.component) == id && r.capture.component == r.component.id =>
                    {
                        a.record = Some(r)
                    }
                    _ => a.refused = true,
                }
            }
            Some(Open::File {
                id,
                rel,
                path,
                file,
                len,
                hash,
            }) => {
                let written = file.is_some_and(|f| f.sync_all().is_ok());
                let a = self.arrived.entry(id).or_default();
                let want = a
                    .record
                    .as_ref()
                    .and_then(|r| r.capture.files.iter().find(|f| f.rel == rel));
                let ok = written
                    && want.is_some_and(|w| w.len == len && w.hash == *hash.finalize().as_bytes());
                if !ok {
                    a.refused = true;
                    let _ = std::fs::remove_file(&path);
                }
                a.files.insert(rel, ok);
            }
            _ => {}
        }
        Ok(())
    }
}

/// The staging folder belongs to the sink: whatever becomes of the move (landed, stopped, the link
/// lost, the app closed), nothing staged (a ticked secret least of all) outlives it.
impl Drop for Sink {
    fn drop(&mut self) {
        // A file part way is closed first, so it can be removed.
        self.open = None;
        self.remove_staging();
    }
}

/// The prefix of every staging folder (swept away at start: one a crash left behind).
pub const STAGING_PREFIX: &str = "fullmove-data-";

/// Removes every staging folder left in `dir` (PCTwin's own data folder) by an earlier run that
/// ended without cleaning up (the laptop turned off, the app crashed). Returns how many.
pub fn sweep_staging(dir: &Path) -> usize {
    let mut n = 0;
    for e in std::fs::read_dir(dir).into_iter().flatten().flatten() {
        let is_ours = e
            .file_name()
            .to_str()
            .is_some_and(|name| name.starts_with(STAGING_PREFIX));
        // Never follow a link out of PCTwin's own folder.
        let is_dir = e.file_type().is_ok_and(|t| t.is_dir());
        if is_ours && is_dir && std::fs::remove_dir_all(e.path()).is_ok() {
            n += 1;
        }
    }
    n
}

/// The new laptop, for landing.
pub struct Landing<'a> {
    pub journal: &'a Journal,
    pub key: &'a CaptureKey,
    pub home: &'a Destination,
    pub home_path: &'a Path,
    pub owner_only: &'a dyn OwnerOnly,
    pub scope: Scope,
    pub actor: Actor,
    /// How this laptop lays out a person's home: its own system ([`System::here`]); tests that
    /// build a Windows home name Windows on every system they run on.
    pub system: System,
    /// The old laptop's kind of system, as its extras said (`plan.from`): with this laptop's own,
    /// where each component goes here is decided here, from this laptop's own catalogue
    /// ([`pctwin_special::classify::placement_of`]), never taken from the old laptop.
    pub from: System,
    /// What the settings catalogue's file rules need to put an app's settings files right as
    /// they land ([`crate::fixups`]); `None`: they land exactly as captured.
    pub fix: Option<crate::fixups::FixCtx<'a>>,
}

/// Why a component of app data needs the person, in a word the screens translate (never the
/// engine's own words shown raw).
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(tag = "why", rename_all = "kebab-case")]
pub enum DataWhy {
    /// The connection was lost before it arrived; it is still on the old laptop.
    Lost,
    /// It did not arrive.
    DidNotArrive,
    /// Its app was open on the old laptop, so it was not read.
    AppOpen,
    /// It kept changing on the old laptop while it was read.
    Changing,
    /// It could not be read on the old laptop.
    Unreadable,
    /// It was not read (the move stopped first).
    NotRead,
    /// Not every file of it arrived checked.
    NotWhole,
    /// It arrived, but PCTwin could not put it in place here.
    NotLanded,
    /// The app's folder was already on this laptop: it landed beside it, as `folder`, and the app
    /// keeps using its own folder until the person swaps them.
    Beside { folder: String },
    /// It landed in `folder`, but the app needs a step before it can use it.
    Placed { folder: String },
    /// Mail kept only on the old Mac: `messages` messages written as mailbox files in `folder`;
    /// the person imports them in their mail app (never claimed imported).
    MailReady { messages: u64, folder: String },
    /// `messages` messages written in `folder`, but `missing` of them had attachments that were
    /// not on the old laptop, `failed` could not be written, or some of the mail did not arrive.
    MailPartsMissing {
        messages: u64,
        missing: u64,
        failed: u64,
        folder: String,
    },
    /// `accounts` mail accounts keep their mail on their server: the person signs in here.
    MailSignIn { accounts: u64 },
    /// PCTwin could not look inside the old Mac's mail (Full Disk Access is off there).
    MailNoAccess,
}

/// How one component ended.
#[derive(Debug, Clone, PartialEq)]
pub struct Landed {
    pub id: String,
    pub app: String,
    pub result: ItemResult,
    /// Its landing in the journal (for undo).
    pub effect: Option<u64>,
    pub words: String,
    /// Why it needs the person, when it does.
    pub why: Option<DataWhy>,
}

/// Lands every component that arrived whole, secrets first; anything chosen that did not arrive
/// whole ends needs-action, never landed. A component in `held` (its app's data was edited on the
/// old laptop after the approval) is not landed either: it waits for the person's answer.
pub fn land_all(
    sink: &Sink,
    n: &Landing<'_>,
    apps: &BTreeMap<String, String>,
    held: &std::collections::BTreeSet<String>,
) -> Vec<Landed> {
    let rec = n.journal.imports(n.key);
    let stop = Stop::new();
    let made = MadeFolders::default();
    let special_rows =
        pctwin_catalogue::Catalogue::<pctwin_special::catalogue::SpecialRow>::shipped(
            pctwin_special::catalogue::SHIPPED,
        );
    let seal = PlanSeal::new(
        2,
        *blake3::Hasher::new_derive_key("PCTwin app data landing 2026-10-28 v1")
            .update(&sink.choice.encode())
            .finalize()
            .as_bytes(),
    );
    let root = RootRef::new(2);
    let laptop = NewLaptopOf {
        n,
        rec: &rec,
        stop: &stop,
        made: &made,
        seal,
        root,
    };
    let mut ids: Vec<(&String, bool)> = sink
        .choice
        .secrets
        .iter()
        .map(|i| (i, true))
        .chain(sink.choice.copy.iter().map(|i| (i, false)))
        .collect();
    ids.dedup();
    let mut out = Vec::new();
    for (step, (id, secret)) in ids.into_iter().enumerate() {
        let app = apps.get(id).cloned().unwrap_or_default();
        let needs = |words: &str, why: DataWhy| Landed {
            id: id.clone(),
            app: app.clone(),
            result: ItemResult::because(Reason::FinishByHand),
            effect: None,
            words: words.into(),
            why: Some(why),
        };
        if held.contains(id) {
            out.push(needs(
                "it was changed on the old laptop after you approved; PCTwin asks you again",
                DataWhy::Changing,
            ));
            continue;
        }
        let Some(a) = sink.arrived.get(id) else {
            out.push(if sink.lost {
                needs(
                    "the connection was lost before it arrived; it is still safe on your old laptop",
                    DataWhy::Lost,
                )
            } else {
                needs("it did not arrive", DataWhy::DidNotArrive)
            });
            continue;
        };
        if let Some(why) = a.not_captured {
            out.push(match why {
                NotCaptured::AppOpen => {
                    needs("its app was open on the old laptop", DataWhy::AppOpen)
                }
                NotCaptured::Changing => {
                    needs("it kept changing on the old laptop", DataWhy::Changing)
                }
                NotCaptured::Unreadable => needs(
                    "it could not be read on the old laptop",
                    DataWhy::Unreadable,
                ),
                NotCaptured::Stopped => needs("it was not read", DataWhy::NotRead),
            });
            continue;
        }
        let Some(r) = &a.record else {
            out.push(needs("its record did not arrive", DataWhy::NotWhole));
            continue;
        };
        // Apple Mail's raw files are never landed as they are: [`convert_mail`] writes them out.
        if r.root == RootKind::MacMail {
            continue;
        }
        let whole = arrived_whole(a, r.capture.files.iter().map(|f| f.rel.as_str()));
        if !whole || r.component.secret != secret {
            out.push(needs("not every file arrived checked", DataWhy::NotWhole));
            continue;
        }
        // Where it goes here, decided here (the old laptop planned for its own kind of system).
        let placed = placed_here(r, n.from, n.system, &special_rows);
        let r = &placed;
        // An app's settings files the catalogue describes are put right in staging first; the
        // record then names exactly the bytes that land.
        let fixed;
        let r = match &n.fix {
            Some(ctx) if !secret => {
                fixed = crate::fixups::fix(r, &sink.folder(id), ctx);
                &fixed
            }
            _ => r,
        };
        out.push(laptop.land(id, r, step as u32, secret, &sink.folder(id)));
    }
    out
}

/// `r` placed for this laptop (`to`), as this laptop's own catalogue says for data from a laptop
/// of `from` ([`pctwin_special::classify::placement_of`]). The old laptop names only where the
/// data was there (its root and folder, sent as `Same`); a component it sent kept aside stays
/// kept aside, and one it sent already placed elsewhere is not trusted: kept aside too.
pub fn placed_here(
    r: &Record,
    from: System,
    to: System,
    rows: &pctwin_catalogue::Catalogue<pctwin_special::catalogue::SpecialRow>,
) -> Record {
    let placement = match &r.placement {
        PlacementWire::Same { root, folder } => {
            (&pctwin_special::classify::placement_of(*root, folder, from, to, rows)).into()
        }
        PlacementWire::Translated { .. } | PlacementWire::PreservedOnly => {
            PlacementWire::PreservedOnly
        }
    };
    Record {
        placement,
        ..r.clone()
    }
}

/// Whether a component arrived whole: nothing about it was refused, and every file its record
/// names (`wanted`) arrived checked. Anything less ends "needs action", never landed as whole.
pub(crate) fn arrived_whole<'w>(a: &Arrived, wanted: impl IntoIterator<Item = &'w str>) -> bool {
    !a.refused
        && wanted
            .into_iter()
            .all(|rel| a.files.get(rel).copied() == Some(true))
}

pub(crate) struct NewLaptopOf<'a, 'r> {
    pub(crate) n: &'a Landing<'a>,
    pub(crate) rec: &'r pctwin_journal::Recorder<'r, pctwin_journal::Imports>,
    pub(crate) stop: &'a Stop,
    pub(crate) made: &'a MadeFolders,
    pub(crate) seal: PlanSeal,
    pub(crate) root: RootRef,
}

impl NewLaptopOf<'_, '_> {
    fn land(&self, id: &str, r: &Record, step: u32, secret: bool, work: &Path) -> Landed {
        match self.applied(id, r, step, secret, work) {
            Some(applied) => {
                let why = why_of(&applied);
                Landed {
                    id: id.into(),
                    app: r.app.clone(),
                    result: applied.result,
                    effect: applied.landed.as_ref().map(|l| l.effect),
                    words: applied.words,
                    why,
                }
            }
            None => Landed {
                id: id.into(),
                app: r.app.clone(),
                result: ItemResult::because(Reason::FinishByHand),
                effect: None,
                words: "it was not landed".into(),
                why: Some(DataWhy::NotLanded),
            },
        }
    }

    /// Lands `r` (its files in `work`) through 2.5's own landing; `None`: it was not tried.
    pub(crate) fn applied(
        &self,
        id: &str,
        r: &Record,
        step: u32,
        secret: bool,
        work: &Path,
    ) -> Option<api::Applied> {
        let p = Planned {
            app: r.app.clone(),
            component: r.component.clone(),
            base: PathBuf::new(),
            step: Step::None,
            why: String::new(),
            proposed: true,
            placement: (&r.placement).into(),
            root: r.root,
            // Landing does not use them: what the person is shown came with the plan.
            category: pctwin_special::category::Category::Other,
            notes: Vec::new(),
        };
        let approved = Approved {
            seal: self.seal,
            component: r.component.id,
            root: self.root,
            generation: r.capture.generation.clone(),
            handle: u64::from(step),
        };
        let now = Now::real();
        let grant = if secret {
            self.n
                .key
                .secret(
                    self.seal,
                    vec![approved.object()],
                    SecretEffect::Land,
                    GRANT,
                    now,
                )
                .map(Grant::Secret)
        } else {
            self.n
                .key
                .grant(
                    self.seal,
                    vec![self.root],
                    CaptureEffect::Import,
                    GRANT,
                    now,
                )
                .map(Grant::Ordinary)
        };
        let Ok(grant) = grant else {
            return None;
        };
        let (Some(item), Ok(recipe)) = (
            id.get(..32).and_then(|h| ItemId::from_hex(h).ok()),
            RecipeId::new("pctwin.special.land"),
        ) else {
            return None;
        };
        let planned = PlannedEffect {
            scope: self.n.scope,
            item,
            recipe: RecipeRef {
                id: recipe,
                version: 1,
            },
            step,
            actor: self.n.actor.clone(),
        };
        let laptop = api::NewLaptop {
            rec: self.rec,
            home: self.n.home,
            home_path: self.n.home_path,
            root: self.root,
            seal: self.seal,
            system: self.n.system,
            owner_only: self.n.owner_only,
            stop: self.stop,
            made: self.made,
        };
        Some(api::apply(
            &laptop,
            grant,
            &approved,
            &planned,
            &p,
            &r.capture,
            work,
            Conflict::KeepBoth,
            now,
        ))
    }
}

/// Why a landed (or not landed) component needs the person, from how it was applied: nothing when
/// it is done or the person left it out.
pub fn why_of(applied: &api::Applied) -> Option<DataWhy> {
    use pctwin_types::ItemStatus;
    if matches!(
        applied.result.status(),
        ItemStatus::Copied | ItemStatus::Converted | ItemStatus::SkippedByUser
    ) {
        return None;
    }
    Some(match &applied.landed {
        Some(l) if l.beside => DataWhy::Beside {
            folder: l.folder.clone(),
        },
        Some(l) if l.verified => DataWhy::Placed {
            folder: l.folder.clone(),
        },
        _ => DataWhy::NotLanded,
    })
}

#[cfg(test)]
mod whole_tests {
    use super::*;

    fn arrived(files: &[(&str, bool)], refused: bool) -> Arrived {
        Arrived {
            files: files.iter().map(|(r, ok)| (r.to_string(), *ok)).collect(),
            refused,
            ..Arrived::default()
        }
    }

    /// One file not checked, one missing, or the set refused: never landed as whole.
    #[test]
    fn a_component_is_whole_only_when_every_file_arrived_checked_and_nothing_was_refused() {
        let wanted = ["a.db", "b.db"];
        assert!(arrived_whole(
            &arrived(&[("a.db", true), ("b.db", true)], false),
            wanted
        ));
        assert!(!arrived_whole(
            &arrived(&[("a.db", true), ("b.db", false)], false),
            wanted
        ));
        assert!(!arrived_whole(
            &arrived(&[("a.db", false), ("b.db", true)], false),
            wanted
        ));
        assert!(
            !arrived_whole(&arrived(&[("a.db", true)], false), wanted),
            "one missing"
        );
        assert!(!arrived_whole(
            &arrived(&[("a.db", true), ("b.db", true)], true),
            wanted
        ));
        assert!(
            arrived_whole(&arrived(&[], false), []),
            "a record naming no files"
        );
        assert!(
            !arrived_whole(&arrived(&[], true), []),
            "refused, even with no files"
        );
    }
}

#[cfg(test)]
mod placed_tests {
    use super::*;
    use pctwin_special::capture::{Capture, Method, generation};
    use pctwin_special::component::{Component, Custody, Evidence, Preservation};

    fn record(placement: PlacementWire, root: RootKind) -> Record {
        let component = Component::new(
            pctwin_record::Owner::Person {
                account_id: "ada".into(),
            },
            "Code",
            vec!["User/keybindings.json".to_string()],
            Custody::Portable,
            false,
            vec![Evidence::Unreadable {
                why: "a test".into(),
            }],
        );
        let t = tempfile::tempdir().unwrap();
        Record {
            app: "Visual Studio Code".into(),
            capture: Capture {
                component: component.id,
                method: Method::SteadyCopy,
                preservation: Preservation::Captured,
                generation: generation(t.path(), &[]),
                files: vec![],
                folded: Vec::new(),
                evidence: Vec::new(),
                closed_unconfirmed: false,
            },
            component,
            placement,
            root,
        }
    }

    /// Engine gap closed (31 October 2026): where app data goes is decided on the new laptop,
    /// from its own catalogue, for the old laptop's kind of system; the old laptop's own word
    /// for another place is never taken.
    #[test]
    fn app_data_is_placed_here_from_this_laptops_catalogue() {
        let rows = pctwin_catalogue::Catalogue::<pctwin_special::catalogue::SpecialRow>::shipped(
            pctwin_special::catalogue::SHIPPED,
        );
        let same = |root| PlacementWire::Same {
            root,
            folder: "Code".into(),
        };
        let r = record(same(RootKind::Roaming), RootKind::Roaming);
        assert_eq!(
            placed_here(&r, System::Windows, System::Mac, &rows).placement,
            PlacementWire::Translated {
                root: RootKind::ApplicationSupport,
                folder: "Code".into()
            }
        );
        assert_eq!(
            placed_here(&r, System::Windows, System::Linux, &rows).placement,
            PlacementWire::Translated {
                root: RootKind::XdgConfig,
                folder: "Code".into()
            }
        );
        // The same kind of laptop: the same place.
        assert_eq!(
            placed_here(&r, System::Windows, System::Windows, &rows).placement,
            same(RootKind::Roaming)
        );
        // A folder no row names for this system: kept aside, never guessed.
        let unknown = record(
            PlacementWire::Same {
                root: RootKind::Roaming,
                folder: "Lab Unknown".into(),
            },
            RootKind::Roaming,
        );
        assert_eq!(
            placed_here(&unknown, System::Windows, System::Mac, &rows).placement,
            PlacementWire::PreservedOnly
        );
        // The old laptop's own word for another place is not taken.
        let claimed = record(
            PlacementWire::Translated {
                root: RootKind::ApplicationSupport,
                folder: "Elsewhere".into(),
            },
            RootKind::Roaming,
        );
        assert_eq!(
            placed_here(&claimed, System::Windows, System::Mac, &rows).placement,
            PlacementWire::PreservedOnly
        );
    }
}
