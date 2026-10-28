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

/// The old laptop's app data, given the new laptop's choice (read only).
pub struct Source<'a> {
    planned: Vec<Planned>,
    key: &'a CaptureKey,
    holders: &'a dyn Holders,
    /// PCTwin's own private work folder on the old laptop (removed when done).
    work: PathBuf,
    stop: Stop,
    todo: Vec<(usize, bool)>,
    queue: Vec<(Vec<u8>, PathBuf)>,
    seal: PlanSeal,
}

impl<'a> Source<'a> {
    pub fn new(
        planned: Vec<Planned>,
        key: &'a CaptureKey,
        holders: &'a dyn Holders,
        work: PathBuf,
    ) -> Self {
        Self {
            planned,
            key,
            holders,
            work,
            stop: Stop::new(),
            todo: Vec::new(),
            queue: Vec::new(),
            seal: PlanSeal::new(1, [0; 32]),
        }
    }

    fn header(h: &Header) -> Vec<u8> {
        serde_json::to_vec(h).unwrap_or_default()
    }

    /// Captures the component at `at` (`secret`: with a secret grant naming exactly it) and
    /// queues what it sends.
    fn capture(&mut self, at: usize, secret: bool) {
        let p = &self.planned[at];
        let id = id_hex(&p.component);
        let root = RootRef::new(1);
        let approved = api::approve(p, self.seal, root, at as u64);
        let work = self.work.join(at.to_string());
        let not = |why| {
            (
                Self::header(&Header::NotCaptured {
                    id: id.clone(),
                    why,
                }),
                PathBuf::new(),
            )
        };
        if std::fs::create_dir_all(&work).is_err() {
            self.queue.push(not(NotCaptured::Unreadable));
            return;
        }
        let grant = if secret {
            self.key
                .secret(
                    self.seal,
                    vec![approved.object()],
                    SecretEffect::Read,
                    GRANT,
                    Now::real(),
                )
                .map(Grant::Secret)
        } else {
            self.key
                .grant(
                    self.seal,
                    vec![root],
                    CaptureEffect::ReadPrivate,
                    GRANT,
                    Now::real(),
                )
                .map(Grant::Ordinary)
        };
        let Ok(grant) = grant else {
            self.queue.push(not(NotCaptured::Stopped));
            return;
        };
        match api::capture_planned(
            p,
            grant,
            &approved,
            &work,
            self.holders,
            &self.stop,
            Now::real(),
        ) {
            Ok(capture) => {
                let record = Record {
                    app: p.app.clone(),
                    component: p.component.clone(),
                    placement: (&p.placement).into(),
                    root: p.root,
                    capture: capture.clone(),
                };
                let body = self.work.join(format!("{at}.record"));
                if std::fs::write(&body, serde_json::to_vec(&record).unwrap_or_default()).is_err() {
                    self.queue.push(not(NotCaptured::Unreadable));
                    return;
                }
                self.queue
                    .push((Self::header(&Header::Record { id: id.clone() }), body));
                for f in &capture.files {
                    let path = f
                        .rel
                        .split('/')
                        .fold(work.clone(), |path, part| path.join(part));
                    self.queue.push((
                        Self::header(&Header::File {
                            id: id.clone(),
                            rel: f.rel.clone(),
                        }),
                        path,
                    ));
                }
            }
            Err(e) => self.queue.push(not((&e).into())),
        }
    }
}

impl Drop for Source<'_> {
    fn drop(&mut self) {
        let _ = std::fs::remove_dir_all(&self.work);
    }
}

impl pctwin_move::extras::DataOut for Source<'_> {
    fn chosen(&mut self, choice: Vec<u8>) {
        self.seal = PlanSeal::new(
            1,
            *blake3::Hasher::new_derive_key("PCTwin app data choice 2026-10-28 v1")
                .update(&choice)
                .finalize()
                .as_bytes(),
        );
        let c = Choice::decode(&choice);
        for (at, p) in self.planned.iter().enumerate() {
            let id = id_hex(&p.component);
            // Only what was chosen is read: a component that copies (never a secret by this
            // list), and each secret the person ticked by itself.
            if !p.component.secret && c.copy.contains(&id) {
                self.todo.push((at, false));
            } else if p.component.secret && c.secrets.contains(&id) {
                self.todo.push((at, true));
            }
        }
        self.todo.reverse();
    }

    fn next(&mut self) -> Option<(Vec<u8>, Box<dyn std::io::Read + Send>)> {
        loop {
            if !self.queue.is_empty() {
                let (header, path) = self.queue.remove(0);
                let reader: Box<dyn std::io::Read + Send> = if path.as_os_str().is_empty() {
                    Box::new(std::io::empty())
                } else {
                    match std::fs::File::open(&path) {
                        Ok(f) => Box::new(f),
                        Err(_) => Box::new(std::io::empty()),
                    }
                };
                return Some((header, reader));
            }
            let (at, secret) = self.todo.pop()?;
            self.capture(at, secret);
        }
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
        }
    }

    fn chosen(&self, id: &str) -> bool {
        self.choice.copy.iter().any(|c| c == id) || self.choice.secrets.iter().any(|c| c == id)
    }

    /// Removes the staging folder (after landing: what landed is in the person's home now).
    pub fn remove_staging(&self) {
        let _ = std::fs::remove_dir_all(&self.staging);
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
}

/// Lands every component that arrived whole, secrets first; anything chosen that did not arrive
/// whole ends needs-action, never landed.
pub fn land_all(sink: &Sink, n: &Landing<'_>, apps: &BTreeMap<String, String>) -> Vec<Landed> {
    let rec = n.journal.imports(n.key);
    let stop = Stop::new();
    let made = MadeFolders::default();
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
        let needs = |words: &str| Landed {
            id: id.clone(),
            app: app.clone(),
            result: ItemResult::because(Reason::FinishByHand),
            effect: None,
            words: words.into(),
        };
        let Some(a) = sink.arrived.get(id) else {
            out.push(needs("it did not arrive"));
            continue;
        };
        if let Some(why) = a.not_captured {
            out.push(needs(match why {
                NotCaptured::AppOpen => "its app was open on the old laptop",
                NotCaptured::Changing => "it kept changing on the old laptop",
                NotCaptured::Unreadable => "it could not be read on the old laptop",
                NotCaptured::Stopped => "it was not read",
            }));
            continue;
        }
        let Some(r) = &a.record else {
            out.push(needs("its record did not arrive"));
            continue;
        };
        let whole = !a.refused
            && r.capture
                .files
                .iter()
                .all(|f| a.files.get(&f.rel).copied() == Some(true));
        if !whole || r.component.secret != secret {
            out.push(needs("not every file arrived checked"));
            continue;
        }
        out.push(laptop.land(id, r, step as u32, secret, &sink.folder(id)));
    }
    out
}

struct NewLaptopOf<'a, 'r> {
    n: &'a Landing<'a>,
    rec: &'r pctwin_journal::Recorder<'r, pctwin_journal::Imports>,
    stop: &'a Stop,
    made: &'a MadeFolders,
    seal: PlanSeal,
    root: RootRef,
}

impl NewLaptopOf<'_, '_> {
    fn land(&self, id: &str, r: &Record, step: u32, secret: bool, work: &Path) -> Landed {
        let p = Planned {
            app: r.app.clone(),
            component: r.component.clone(),
            base: PathBuf::new(),
            step: Step::None,
            why: String::new(),
            proposed: true,
            placement: (&r.placement).into(),
            root: r.root,
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
            return Landed {
                id: id.into(),
                app: r.app.clone(),
                result: ItemResult::because(Reason::FinishByHand),
                effect: None,
                words: "it was not landed".into(),
            };
        };
        let (Some(item), Ok(recipe)) = (
            id.get(..32).and_then(|h| ItemId::from_hex(h).ok()),
            RecipeId::new("pctwin.special.land"),
        ) else {
            return Landed {
                id: id.into(),
                app: r.app.clone(),
                result: ItemResult::because(Reason::FinishByHand),
                effect: None,
                words: "it was not landed".into(),
            };
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
        let applied = api::apply(
            &laptop,
            grant,
            &approved,
            &planned,
            &p,
            &r.capture,
            work,
            Conflict::KeepBoth,
            now,
        );
        Landed {
            id: id.into(),
            app: r.app.clone(),
            result: applied.result,
            effect: applied.landed.as_ref().map(|l| l.effect),
            words: applied.words,
        }
    }
}
